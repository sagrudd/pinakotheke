// SPDX-License-Identifier: MPL-2.0
//! Bounded process adapter for DASObjectStore catalogue inventory.

use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use x_img_api::HostGalleryInventory;
use x_img_core::gallery_reconciliation::{AuthorityObject, AuthorityObjectIdentity};

const SCHEMA: &str = "pinakotheke.gallery-inventory-helper.v2";
const CAPTURE_AUTHORITY_SCHEMA: &str = "pinakotheke.capture-authority.v1";
const RESPONSE_LIMIT: usize = 64 * 1024 * 1024;
const AUTHORITY_LIMIT: u64 = 256 * 1024;

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
    object_version: u64,
    checksum: String,
    state: String,
    content_length: u64,
}

pub(crate) fn backend(
    path: &Path,
    authority_path: &Path,
    endpoint_id: String,
    object_store_id: String,
) -> io::Result<HostGalleryInventory> {
    validate_helper(path)?;
    let authority = load_reviewed_destination(authority_path)?;
    if authority.endpoint_id != endpoint_id || authority.object_store_id != object_store_id {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "gallery inventory destination does not match reviewed authority",
        ));
    }
    let path = path.to_owned();
    let authority_path = authority_path.to_owned();
    Ok(std::sync::Arc::new(move || {
        invoke(&path, &authority_path, &endpoint_id, &object_store_id)
    }))
}

fn invoke(
    path: &Path,
    authority_path: &Path,
    endpoint_id: &str,
    object_store_id: &str,
) -> Result<Vec<AuthorityObject>, String> {
    let mut child = Command::new(path)
        .arg("gallery-inventory-v2")
        // This is deliberately supplied by the parent from the same private
        // authority document that admitted the destination.  The child never
        // falls back to an ambient ObjectStore selection.
        .env(
            "PINAKOTHEKE_GALLERY_INVENTORY_AUTHORITY_FILE",
            authority_path,
        )
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
                || object.object_version == 0
                || !valid_checksum(&object.checksum)
            {
                return Err("gallery inventory authority returned an invalid object".into());
            }
            Ok(AuthorityObject {
                identity: AuthorityObjectIdentity {
                    endpoint_id: endpoint_id.into(),
                    object_store_id: object_store_id.into(),
                    object_key: object.object_key,
                },
                object_version: object.object_version,
                checksum: object.checksum,
                state: object.state,
                content_length: object.content_length,
            })
        })
        .collect()
}

pub(crate) fn run_protocol() -> Result<(), Box<dyn std::error::Error>> {
    let request: Request = serde_json::from_reader(io::stdin().lock())?;
    if request.schema_version != SCHEMA
        || !safe_identifier(&request.endpoint_id)
        || !safe_identifier(&request.object_store_id)
    {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "invalid inventory request").into(),
        );
    }
    let authority_path = std::env::var_os("PINAKOTHEKE_GALLERY_INVENTORY_AUTHORITY_FILE")
        .map(PathBuf::from)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "gallery inventory requires a reviewed destination authority",
            )
        })?;
    let authority = load_reviewed_destination(&authority_path)?;
    if authority.endpoint_id != request.endpoint_id
        || authority.object_store_id != request.object_store_id
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "gallery inventory request does not match reviewed destination",
        )
        .into());
    }
    let executable = std::env::var_os("PINAKOTHEKE_DASOBJECTSTORE_CLI")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/usr/bin/dasobjectstore"));
    validate_das_cli(&executable)?;
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
                object_version: object.object_version,
                checksum: object.checksum,
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
    object_version: u64,
    checksum: String,
    state: String,
    size_bytes: u64,
}

fn valid_checksum(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureAuthorityDocument {
    schema_version: String,
    endpoint_id: String,
    object_store_id: String,
    pairings: Vec<CapturePairingRecord>,
    // Site policies do not grant inventory authority.  They remain accepted
    // here so one private v1 document is the destination source of truth.
    #[serde(default)]
    sites: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapturePairingRecord {
    pairing_id: String,
    actor_id: String,
    expires_at: u64,
    revoked: bool,
}

#[derive(Debug)]
struct ReviewedDestination {
    endpoint_id: String,
    object_store_id: String,
}

fn load_reviewed_destination(path: &Path) -> io::Result<ReviewedDestination> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "gallery inventory authority path must be absolute",
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > AUTHORITY_LIMIT
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "gallery inventory authority must be a bounded regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "gallery inventory authority file must not be accessible by group or others",
            ));
        }
    }
    let bytes = fs::read(path)?;
    let authority: CaptureAuthorityDocument = serde_json::from_slice(&bytes).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "gallery inventory authority document is invalid",
        )
    })?;
    let mut pairing_ids = std::collections::BTreeSet::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_secs();
    if authority.schema_version != CAPTURE_AUTHORITY_SCHEMA
        || !safe_identifier(&authority.endpoint_id)
        || !safe_identifier(&authority.object_store_id)
        || authority.pairings.is_empty()
        || authority.pairings.len() > 128
        || authority.sites.len() > 256
        || authority.pairings.iter().any(|pairing| {
            !safe_identifier(&pairing.pairing_id)
                || !safe_identifier(&pairing.actor_id)
                || pairing.expires_at == 0
                || !pairing_ids.insert(&pairing.pairing_id)
        })
        || !authority
            .pairings
            .iter()
            .any(|pairing| !pairing.revoked && pairing.expires_at > now)
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "gallery inventory authority has no active reviewed pairing",
        ));
    }
    Ok(ReviewedDestination {
        endpoint_id: authority.endpoint_id,
        object_store_id: authority.object_store_id,
    })
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
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
        let mode = metadata.permissions().mode();
        if mode & 0o111 == 0 || mode & 0o022 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "gallery inventory helper must be executable and not group or world writable",
            ));
        }
    }
    Ok(())
}

fn validate_das_cli(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "DASObjectStore CLI path must be absolute",
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "DASObjectStore CLI must be a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        if mode & 0o111 == 0 || mode & 0o022 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "DASObjectStore CLI must be executable and not group or world writable",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let serial = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "pinakotheke-gallery-inventory-test-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("test clock")
                    .as_nanos(),
                serial
            ));
            fs::create_dir(&path).expect("create test directory");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                    .expect("secure test directory");
            }
            Self(path)
        }

        fn authority(&self, endpoint_id: &str, object_store_id: &str, expires_at: u64) -> PathBuf {
            let path = self.0.join("capture-authority.json");
            fs::write(
                &path,
                format!(
                    r#"{{"schema_version":"pinakotheke.capture-authority.v1","endpoint_id":"{endpoint_id}","object_store_id":"{object_store_id}","pairings":[{{"pairing_id":"pairing-1","actor_id":"actor-1","expires_at":{expires_at},"revoked":false}}],"sites":[]}}"#,
                ),
            )
            .expect("write authority");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
                    .expect("secure authority");
            }
            path
        }

        fn executable(&self) -> PathBuf {
            let path = self.0.join("helper");
            fs::write(&path, "#!/bin/sh\nexit 0\n").expect("write helper");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                    .expect("make helper executable");
            }
            path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn response_rejects_unknown_fields() {
        assert!(serde_json::from_str::<Response>(
            r#"{"schema_version":"pinakotheke.gallery-inventory-helper.v2","objects":[],"extra":true}"#
        ).is_err());
    }

    #[test]
    fn inventory_requires_immutable_sha256_evidence() {
        assert!(valid_checksum(&format!("sha256:{}", "a".repeat(64))));
        assert!(!valid_checksum("sha256:too-short"));
        assert!(!valid_checksum(&format!("sha512:{}", "a".repeat(64))));
    }

    #[test]
    fn reviewed_destination_requires_an_active_private_pairing() {
        let directory = TestDirectory::new();
        let authority = directory.authority("endpoint-1", "store-1", u64::MAX);
        let destination = load_reviewed_destination(&authority).expect("reviewed destination");
        assert_eq!(destination.endpoint_id, "endpoint-1");
        assert_eq!(destination.object_store_id, "store-1");

        let expired = directory.authority("endpoint-1", "store-2", 1);
        assert!(load_reviewed_destination(&expired).is_err());
    }

    #[test]
    fn backend_rejects_a_destination_not_bound_by_private_authority() {
        let directory = TestDirectory::new();
        let authority = directory.authority("endpoint-1", "store-1", u64::MAX);
        let helper = directory.executable();
        assert!(backend(&helper, &authority, "endpoint-2".into(), "store-1".into()).is_err());
        assert!(backend(&helper, &authority, "endpoint-1".into(), "store-2".into()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_group_readable_authority_and_world_writable_helper() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new();
        let authority = directory.authority("endpoint-1", "store-1", u64::MAX);
        fs::set_permissions(&authority, fs::Permissions::from_mode(0o640))
            .expect("relax authority permissions");
        assert!(load_reviewed_destination(&authority).is_err());

        let helper = directory.executable();
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o707))
            .expect("relax helper permissions");
        assert!(validate_helper(&helper).is_err());
    }
}
