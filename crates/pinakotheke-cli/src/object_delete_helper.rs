// SPDX-License-Identifier: MPL-2.0
//! Process-isolated host adapter for authoritative DASObjectStore deletion.

use serde::{Deserialize, Serialize};
use std::{
    io::{self, Write},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use x_img_api::{HostObjectDeleteBackend, HostObjectDeleteOutcome};
use x_img_core::gallery_catalogue::GalleryDeleteObject;

const SCHEMA: &str = "pinakotheke.object-delete-helper.v1";
const RESPONSE_LIMIT: usize = 8 * 1024;

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct Request<'a> {
    schema_version: &'static str,
    endpoint_id: &'a str,
    object_store_id: &'a str,
    object_key: &'a str,
    object_version: u64,
    checksum: &'a str,
    content_length: u64,
}

#[derive(Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
enum Response {
    Deleted { schema_version: String },
    AlreadyAbsent { schema_version: String },
    Rejected { schema_version: String },
    Unavailable { schema_version: String },
}

pub(crate) fn backend(path: &Path) -> io::Result<HostObjectDeleteBackend> {
    validate_helper(path)?;
    let path = path.to_owned();
    Ok(HostObjectDeleteBackend::new(std::sync::Arc::new(
        move |object| invoke(&path, object),
    )))
}

fn invoke(path: &Path, deletion: &GalleryDeleteObject) -> Result<HostObjectDeleteOutcome, String> {
    let object = &deletion.object;
    let mut child = Command::new(path)
        .arg("delete-v1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "object deletion authority is unavailable".to_owned())?;
    let request = Request {
        schema_version: SCHEMA,
        endpoint_id: &object.endpoint_id,
        object_store_id: &object.object_store_id,
        object_key: &object.object_key,
        object_version: object.object_version,
        checksum: &object.checksum,
        content_length: deletion.content_length,
    };
    let mut stdin = child
        .stdin
        .take()
        .ok_or("object deletion helper has no input")?;
    serde_json::to_writer(&mut stdin, &request)
        .map_err(|_| "object deletion request could not be encoded")?;
    stdin
        .write_all(b"\n")
        .map_err(|_| "object deletion helper input failed")?;
    drop(stdin);

    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("object deletion authority timed out".into());
            }
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|_| "object deletion authority failed")?;
    if !output.status.success()
        || !output.stdout.is_empty()
        || output.stderr.len() > RESPONSE_LIMIT
        || !output.stderr.ends_with(b"\n")
    {
        return Err("object deletion authority returned an invalid response".into());
    }
    let response: Response = serde_json::from_slice(&output.stderr)
        .map_err(|_| "object deletion authority returned invalid JSON")?;
    let schema = match &response {
        Response::Deleted { schema_version }
        | Response::AlreadyAbsent { schema_version }
        | Response::Rejected { schema_version }
        | Response::Unavailable { schema_version } => schema_version,
    };
    if schema != SCHEMA {
        return Err("object deletion authority returned an unsupported schema".into());
    }
    match response {
        Response::Deleted { .. } => Ok(HostObjectDeleteOutcome::Deleted),
        Response::AlreadyAbsent { .. } => Ok(HostObjectDeleteOutcome::AlreadyAbsent),
        Response::Rejected { .. } => Err("DASObjectStore rejected exact-object deletion".into()),
        Response::Unavailable { .. } => Err("DASObjectStore deletion is unavailable".into()),
    }
}

fn validate_helper(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "object delete helper path must be absolute",
        ));
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "object delete helper must be a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "object delete helper must be executable",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_responses_are_strict_and_versioned() {
        let deleted: Response = serde_json::from_str(
            r#"{"outcome":"deleted","schema_version":"pinakotheke.object-delete-helper.v1"}"#,
        )
        .unwrap();
        assert!(matches!(deleted, Response::Deleted { .. }));
        assert!(serde_json::from_str::<Response>(
            r#"{"outcome":"deleted","schema_version":"pinakotheke.object-delete-helper.v1","extra":true}"#
        )
        .is_err());
    }
}
