// SPDX-License-Identifier: MPL-2.0
//! Bounded process adapter for DASObjectStore catalogue inventory.

use serde::{Deserialize, Serialize};
use std::{
    io::{self, Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use x_img_api::HostGalleryInventory;
use x_img_core::gallery_reconciliation::{AuthorityObject, AuthorityObjectIdentity};

const SCHEMA: &str = "pinakotheke.gallery-inventory-helper.v1";
const RESPONSE_LIMIT: usize = 64 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: String,
    endpoint_id: String,
    object_store_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    schema_version: String,
    objects: Vec<InventoryObject>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryObject {
    object_key: String,
    state: String,
    content_length: u64,
}

pub(crate) fn backend(
    path: &Path,
    endpoint_id: String,
    object_store_id: String,
) -> io::Result<HostGalleryInventory> {
    validate_helper(path)?;
    let path = path.to_owned();
    Ok(std::sync::Arc::new(move || {
        invoke(&path, &endpoint_id, &object_store_id)
    }))
}

fn invoke(
    path: &Path,
    endpoint_id: &str,
    object_store_id: &str,
) -> Result<Vec<AuthorityObject>, String> {
    let mut child = Command::new(path)
        .arg("gallery-inventory-v1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "gallery inventory authority is unavailable".to_owned())?;
    let request = Request {
        schema_version: SCHEMA.into(),
        endpoint_id: endpoint_id.into(),
        object_store_id: object_store_id.into(),
    };
    serde_json::to_writer(
        child
            .stdin
            .as_mut()
            .ok_or("inventory helper has no input")?,
        &request,
    )
    .map_err(|_| "gallery inventory request could not be encoded")?;
    child
        .stdin
        .as_mut()
        .ok_or("inventory helper has no input")?
        .write_all(b"\n")
        .map_err(|_| "gallery inventory helper input failed")?;
    drop(child.stdin.take());

    let stdout = child
        .stdout
        .take()
        .ok_or("gallery inventory helper has no output")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("gallery inventory helper has no diagnostics")?;
    let stdout_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take((RESPONSE_LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr
            .take((RESPONSE_LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });

    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("gallery inventory authority timed out".into());
            }
        }
    }
    let status = child
        .wait()
        .map_err(|_| "gallery inventory authority failed")?;
    let stdout = stdout_reader
        .join()
        .map_err(|_| "gallery inventory authority output failed")?
        .map_err(|_| "gallery inventory authority output failed")?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| "gallery inventory authority output failed")?
        .map_err(|_| "gallery inventory authority output failed")?;
    if !status.success()
        || !stdout.is_empty()
        || stderr.len() > RESPONSE_LIMIT
        || !stderr.ends_with(b"\n")
    {
        return Err("gallery inventory authority returned an invalid response".into());
    }
    let response: Response = serde_json::from_slice(&stderr)
        .map_err(|_| "gallery inventory authority returned invalid JSON")?;
    if response.schema_version != SCHEMA || response.objects.len() > 100_000 {
        return Err("gallery inventory authority returned an unsupported response".into());
    }
    response
        .objects
        .into_iter()
        .map(|object| {
            if object.object_key.is_empty()
                || object.object_key.len() > 2_048
                || object.state != "Protected"
            {
                return Err("gallery inventory authority returned an invalid object".into());
            }
            Ok(AuthorityObject {
                identity: AuthorityObjectIdentity {
                    endpoint_id: endpoint_id.into(),
                    object_store_id: object_store_id.into(),
                    object_key: object.object_key,
                },
                state: object.state,
                content_length: object.content_length,
            })
        })
        .collect()
}

pub(crate) fn run_protocol() -> Result<(), Box<dyn std::error::Error>> {
    let request: Request = serde_json::from_reader(io::stdin().lock())?;
    if request.schema_version != SCHEMA
        || request.endpoint_id.is_empty()
        || request.object_store_id.is_empty()
        || request.object_store_id.len() > 128
    {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "invalid inventory request").into(),
        );
    }
    let executable = std::env::var_os("PINAKOTHEKE_DASOBJECTSTORE_CLI")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/usr/bin/dasobjectstore"));
    let output = Command::new(executable)
        .args(["store", "contents", &request.object_store_id, "--json"])
        .output()?;
    if !output.status.success() || output.stdout.len() > RESPONSE_LIMIT {
        return Err(io::Error::other("DASObjectStore inventory failed").into());
    }
    let snapshot: DasSnapshot = serde_json::from_slice(&output.stdout)?;
    if snapshot.store_id != request.object_store_id || snapshot.objects.len() > 100_000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DASObjectStore inventory mismatch",
        )
        .into());
    }
    let response = Response {
        schema_version: SCHEMA.into(),
        objects: snapshot
            .objects
            .into_iter()
            .filter(|object| object.state == "Protected")
            .map(|object| InventoryObject {
                object_key: object.object_id,
                state: object.state,
                content_length: object.size_bytes,
            })
            .collect(),
    };
    serde_json::to_writer(io::stderr().lock(), &response)?;
    eprintln!();
    Ok(())
}

#[derive(Deserialize)]
struct DasSnapshot {
    store_id: String,
    objects: Vec<DasObject>,
}

#[derive(Deserialize)]
struct DasObject {
    object_id: String,
    state: String,
    size_bytes: u64,
}

fn validate_helper(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "gallery inventory helper path must be absolute",
        ));
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "gallery inventory helper must be a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "gallery inventory helper must be executable",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_rejects_unknown_fields() {
        assert!(serde_json::from_str::<Response>(
            r#"{"schema_version":"pinakotheke.gallery-inventory-helper.v1","objects":[],"extra":true}"#
        ).is_err());
    }
}
