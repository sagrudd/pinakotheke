// SPDX-License-Identifier: MPL-2.0
//! Process-isolated host adapter for scoped DASObjectStore object reads.

use std::{
    io::{self, BufRead, BufReader, Read, Write},
    path::Path,
    pin::Pin,
    process::{Child, ChildStdout, Command, Stdio},
    task::{Context, Poll},
};

use axum::body::Body;
use bytes::Bytes;
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;
use x_img_api::HostObjectReadBackend;
use x_img_core::object_read::{
    ByteRange, ObjectContentMetadata, ObjectReadBackendError, ObjectReadRequest, ObjectReadResult,
    ObjectUnavailable,
};

const SCHEMA: &str = "pinakotheke.object-read-helper.v1";
const HEADER_LIMIT: u64 = 8 * 1024;
const CHUNK_BYTES: usize = 64 * 1024;

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct HelperRequest<'a> {
    schema_version: &'static str,
    endpoint_id: &'a str,
    object_store_id: &'a str,
    object_key: &'a str,
    object_version: u64,
    checksum: &'a str,
    range: Option<HelperRange>,
    if_none_match_etag: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HelperRange {
    start: u64,
    end_inclusive: u64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
enum HelperResponse {
    Content {
        schema_version: String,
        content_type: String,
        content_length: u64,
        total_length: u64,
        checksum: String,
        etag: String,
        content_range: Option<HelperRange>,
    },
    NotModified {
        schema_version: String,
        etag: String,
    },
    NotFound {
        schema_version: String,
    },
    AccessDenied {
        schema_version: String,
    },
    Unavailable {
        schema_version: String,
    },
    Rejected {
        schema_version: String,
    },
}

pub(crate) fn backend(path: &Path) -> io::Result<HostObjectReadBackend> {
    validate_helper(path)?;
    let path = path.to_owned();
    Ok(HostObjectReadBackend::new(Box::new(move |request| {
        open(&path, request)
    })))
}

fn validate_helper(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "object read helper path must be absolute",
        ));
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "object read helper must be a regular file, not a symlink",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "object read helper must be executable",
            ));
        }
    }
    Ok(())
}

fn open(
    path: &Path,
    request: &ObjectReadRequest,
) -> Result<ObjectReadResult<Body>, ObjectReadBackendError> {
    let mut child = Command::new(path)
        .arg("read-v1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(unavailable)?;
    let envelope = HelperRequest {
        schema_version: SCHEMA,
        endpoint_id: &request.object.endpoint_id,
        object_store_id: &request.object.object_store_id,
        object_key: &request.object.object_key,
        object_version: request.object.object_version,
        checksum: &request.object.checksum,
        range: request.range.map(|range| HelperRange {
            start: range.start,
            end_inclusive: range.end_inclusive,
        }),
        if_none_match_etag: request.if_none_match_etag.as_deref(),
    };
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| unavailable("missing helper stdin"))?;
    serde_json::to_writer(&mut stdin, &envelope).map_err(rejected)?;
    stdin.write_all(b"\n").map_err(unavailable)?;
    drop(stdin);

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| unavailable("missing helper response"))?;
    let mut header = String::new();
    BufReader::new(stderr)
        .take(HEADER_LIMIT + 1)
        .read_line(&mut header)
        .map_err(unavailable)?;
    if header.len() as u64 > HEADER_LIMIT || !header.ends_with('\n') {
        terminate(&mut child);
        return Err(ObjectReadBackendError::Rejected(
            "helper response header is missing or exceeds 8 KiB".into(),
        ));
    }
    let response: HelperResponse = serde_json::from_str(&header).map_err(|error| {
        terminate(&mut child);
        rejected(error)
    })?;
    if response_schema(&response) != SCHEMA {
        terminate(&mut child);
        return Err(ObjectReadBackendError::Rejected(
            "helper response uses an unsupported schema".into(),
        ));
    }
    match response {
        HelperResponse::Content {
            content_type,
            content_length,
            total_length,
            checksum,
            etag,
            content_range,
            ..
        } => {
            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| unavailable("missing helper payload stream"))?;
            let metadata = ObjectContentMetadata {
                content_type,
                content_length,
                total_length,
                checksum: checksum.clone(),
                etag,
                content_range: content_range.map(|range| ByteRange {
                    start: range.start,
                    end_inclusive: range.end_inclusive,
                }),
            };
            let stream = helper_stream(
                child,
                stdout,
                content_length,
                request.range.is_none().then_some(checksum),
            );
            Ok(ObjectReadResult::Content {
                metadata,
                stream: Body::from_stream(stream),
            })
        }
        HelperResponse::NotModified { etag, .. } => {
            finish_empty(child)?;
            Ok(ObjectReadResult::NotModified { etag })
        }
        HelperResponse::NotFound { .. } => unavailable_result(child, ObjectUnavailable::NotFound),
        HelperResponse::AccessDenied { .. } => {
            unavailable_result(child, ObjectUnavailable::AccessDenied)
        }
        HelperResponse::Unavailable { .. } => {
            unavailable_result(child, ObjectUnavailable::Unavailable)
        }
        HelperResponse::Rejected { .. } => {
            terminate(&mut child);
            Err(ObjectReadBackendError::Rejected(
                "host object read helper rejected the request".into(),
            ))
        }
    }
}

fn response_schema(response: &HelperResponse) -> &str {
    match response {
        HelperResponse::Content { schema_version, .. }
        | HelperResponse::NotModified { schema_version, .. }
        | HelperResponse::NotFound { schema_version }
        | HelperResponse::AccessDenied { schema_version }
        | HelperResponse::Unavailable { schema_version }
        | HelperResponse::Rejected { schema_version } => schema_version,
    }
}

struct HelperStream {
    receiver: mpsc::Receiver<Result<Bytes, io::Error>>,
}

impl Stream for HelperStream {
    type Item = Result<Bytes, io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(context)
    }
}

fn helper_stream(
    mut child: Child,
    mut stdout: ChildStdout,
    expected_length: u64,
    expected_checksum: Option<String>,
) -> HelperStream {
    let (sender, receiver) = mpsc::channel(4);
    std::thread::spawn(move || {
        let mut buffer = vec![0_u8; CHUNK_BYTES];
        let mut total = 0_u64;
        let mut digest = Sha256::new();
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(length) => {
                    total = total.saturating_add(length as u64);
                    if expected_checksum.is_some() {
                        digest.update(&buffer[..length]);
                    }
                    if sender
                        .blocking_send(Ok(Bytes::copy_from_slice(&buffer[..length])))
                        .is_err()
                    {
                        terminate(&mut child);
                        return;
                    }
                }
                Err(error) => {
                    let _ = sender.blocking_send(Err(error));
                    terminate(&mut child);
                    return;
                }
            }
        }
        let status = child.wait();
        let checksum_matches = expected_checksum.is_none_or(|expected| {
            format!("sha256:{:x}", digest.finalize()).eq_ignore_ascii_case(&expected)
        });
        if total != expected_length
            || !checksum_matches
            || !status.is_ok_and(|value| value.success())
        {
            let _ = sender.blocking_send(Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "host object read stream failed length, checksum, or process verification",
            )));
        }
    });
    HelperStream { receiver }
}

fn finish_empty(mut child: Child) -> Result<(), ObjectReadBackendError> {
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| unavailable("missing helper payload stream"))?;
    let mut unexpected = [0_u8; 1];
    if stdout.read(&mut unexpected).map_err(unavailable)? != 0 {
        terminate(&mut child);
        return Err(ObjectReadBackendError::Rejected(
            "not-modified helper response included payload bytes".into(),
        ));
    }
    let status = child.wait().map_err(unavailable)?;
    if status.success() {
        Ok(())
    } else {
        Err(ObjectReadBackendError::Unavailable(
            ObjectUnavailable::Unavailable,
        ))
    }
}

fn unavailable_result(
    mut child: Child,
    outcome: ObjectUnavailable,
) -> Result<ObjectReadResult<Body>, ObjectReadBackendError> {
    terminate(&mut child);
    Err(ObjectReadBackendError::Unavailable(outcome))
}

fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn unavailable(error: impl std::fmt::Display) -> ObjectReadBackendError {
    let _ = error;
    ObjectReadBackendError::Unavailable(ObjectUnavailable::Unavailable)
}

fn rejected(error: impl std::fmt::Display) -> ObjectReadBackendError {
    let _ = error;
    ObjectReadBackendError::Rejected("invalid host object read helper exchange".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use std::{
        fs::File,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temporary_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "pinakotheke-read-helper-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn helper_must_be_absolute_regular_and_executable() {
        assert_eq!(
            validate_helper(Path::new("relative-helper"))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        let path = temporary_path();
        File::create(&path).unwrap();
        #[cfg(unix)]
        assert_eq!(
            validate_helper(&path).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        assert!(validate_helper(&path).is_ok());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn response_schema_is_strict_and_versioned() {
        let response: HelperResponse = serde_json::from_str(
            r#"{"outcome":"not_found","schema_version":"pinakotheke.object-read-helper.v1"}"#,
        )
        .unwrap();
        assert_eq!(response_schema(&response), SCHEMA);
        assert!(serde_json::from_str::<HelperResponse>(
            r#"{"outcome":"not_found","schema_version":"pinakotheke.object-read-helper.v1","extra":true}"#
        )
        .is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn streams_verified_helper_content_without_a_local_payload_file() {
        use std::os::unix::fs::PermissionsExt;

        let path = temporary_path();
        std::fs::write(
            &path,
            r###"#!/bin/sh
test "$1" = read-v1
request=$(cat)
case "$request" in *'"object_key":"media/example.txt"'*) ;; *) exit 9 ;; esac
case "$request" in *'"object_version":7'*) ;; *) exit 10 ;; esac
printf '%s\n' '{"outcome":"content","schema_version":"pinakotheke.object-read-helper.v1","content_type":"application/octet-stream","content_length":3,"total_length":3,"checksum":"sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad","etag":"\"sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\"","content_range":null}' >&2
printf abc
"###,
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let request = ObjectReadRequest {
            object: x_img_core::object_read::AuthorizedObjectReference {
                endpoint_id: "endpoint-1".into(),
                object_store_id: "store-1".into(),
                object_key: "media/example.txt".into(),
                object_version: 7,
                checksum: "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                    .into(),
            },
            range: None,
            if_none_match_etag: None,
        };
        let ObjectReadResult::Content { stream, .. } = open(&path, &request).unwrap() else {
            panic!("expected content");
        };
        assert_eq!(stream.collect().await.unwrap().to_bytes(), "abc");
        std::fs::remove_file(path).unwrap();
    }
}
