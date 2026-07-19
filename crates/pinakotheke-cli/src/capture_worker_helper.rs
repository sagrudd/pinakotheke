// SPDX-License-Identifier: MPL-2.0
//! Process-isolated host worker for one approved capture acquisition.

use std::{
    io::{self, BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use x_img_api::HostCaptureAcquireBackend;
use x_img_core::{
    capture_completion::VerifiedCaptureCompletion,
    persistent_gallery_admission::GalleryVideoCompletion,
    viewed_media::{CaptureKind, CapturePlan},
};

const SCHEMA: &str = "pinakotheke.capture-acquire-helper.v1";
const RESPONSE_LIMIT: u64 = 16 * 1024;

pub(crate) fn backend(
    helper: &Path,
    endpoint_id: String,
    object_store_id: String,
) -> io::Result<HostCaptureAcquireBackend> {
    validate_helper(helper)?;
    let helper = helper.to_owned();
    Ok(HostCaptureAcquireBackend::new(std::sync::Arc::new(
        move |actor_ref, plan| {
            acquire(&helper, actor_ref, plan, &endpoint_id, &object_store_id)
                .map_err(|error| error.to_string())
        },
    )))
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct AcquireRequest<'a> {
    schema_version: &'static str,
    plan_id: &'a str,
    actor_ref: &'a str,
    site_id: &'a str,
    origin: &'a str,
    canonical_page_url: &'a str,
    canonical_media_url: &'a str,
    retrieval_media_url: &'a str,
    canonical_presentation_url: &'a str,
    capture_kind: CaptureKind,
    width: u32,
    height: u32,
    adapter_version: &'a str,
    endpoint_id: &'a str,
    object_store_id: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
enum AcquireResponse {
    Committed {
        schema_version: String,
        #[serde(rename = "catalogue_id")]
        _catalogue_id: String,
        title: String,
        content_type: String,
        content_length: u64,
        endpoint_id: String,
        object_store_id: String,
        object_key: String,
        object_version: u64,
        checksum_sha256: String,
        verified_at_epoch_seconds: u64,
        #[serde(default)]
        video: Option<Box<GalleryVideoCompletion>>,
    },
    PolicyBlocked {
        schema_version: String,
        #[serde(default)]
        reason_code: Option<String>,
    },
    Unavailable {
        schema_version: String,
        #[serde(default)]
        reason_code: Option<String>,
    },
    Rejected {
        schema_version: String,
        #[serde(default)]
        reason_code: Option<String>,
    },
}

pub(crate) fn acquire(
    helper: &Path,
    actor_ref: &str,
    plan: &CapturePlan,
    endpoint_id: &str,
    object_store_id: &str,
) -> io::Result<VerifiedCaptureCompletion> {
    validate_helper(helper)?;
    let mut child = Command::new(helper)
        .arg("acquire-image-v1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let request = AcquireRequest {
        schema_version: SCHEMA,
        plan_id: &plan.plan_id,
        actor_ref,
        site_id: &plan.site_id,
        origin: &plan.origin,
        canonical_page_url: &plan.canonical_page_url,
        canonical_media_url: &plan.canonical_media_url,
        retrieval_media_url: &plan.retrieval_media_url,
        canonical_presentation_url: &plan.canonical_presentation_url,
        capture_kind: plan.capture_kind,
        width: plan.width,
        height: plan.height,
        adapter_version: &plan.adapter_version,
        endpoint_id,
        object_store_id,
    };
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("capture helper stdin unavailable"))?;
    serde_json::to_writer(&mut stdin, &request).map_err(io::Error::other)?;
    stdin.write_all(b"\n")?;
    drop(stdin);
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("capture helper response unavailable"))?;
    let mut response = String::new();
    BufReader::new(stderr)
        .take(RESPONSE_LIMIT + 1)
        .read_line(&mut response)?;
    if response.len() as u64 > RESPONSE_LIMIT || !response.ends_with('\n') {
        terminate(&mut child);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "capture helper response is missing or oversized",
        ));
    }
    let parsed: AcquireResponse = serde_json::from_str(&response).map_err(|error| {
        terminate(&mut child);
        io::Error::new(io::ErrorKind::InvalidData, error)
    })?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("capture helper stdout unavailable"))?;
    let mut unexpected = [0_u8; 1];
    if stdout.read(&mut unexpected)? != 0 {
        terminate(&mut child);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "capture helper must stream payload directly to DASObjectStore",
        ));
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(io::Error::other("capture helper failed"));
    }
    let schema = match &parsed {
        AcquireResponse::Committed { schema_version, .. }
        | AcquireResponse::PolicyBlocked { schema_version, .. }
        | AcquireResponse::Unavailable { schema_version, .. }
        | AcquireResponse::Rejected { schema_version, .. } => schema_version,
    };
    if schema != SCHEMA {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "capture helper schema is unsupported",
        ));
    }
    match parsed {
        AcquireResponse::Committed {
            _catalogue_id: _,
            title,
            content_type,
            content_length,
            endpoint_id: actual_endpoint,
            object_store_id: actual_store,
            object_key,
            object_version,
            checksum_sha256,
            verified_at_epoch_seconds,
            video,
            ..
        } if actual_endpoint == endpoint_id && actual_store == object_store_id => {
            Ok(VerifiedCaptureCompletion {
                plan_id: plan.plan_id.clone(),
                catalogue_id: plan.catalogue_id.clone(),
                title,
                content_type,
                content_length,
                endpoint_id: actual_endpoint,
                object_store_id: actual_store,
                object_key,
                object_version,
                checksum_sha256,
                verified_at_epoch_seconds,
                video: video.map(|video| *video),
            })
        }
        AcquireResponse::Committed { .. } => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "capture helper changed the reviewed destination",
        )),
        AcquireResponse::PolicyBlocked { reason_code, .. } => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            helper_failure("capture helper blocked acquisition by policy", reason_code),
        )),
        AcquireResponse::Unavailable { reason_code, .. } => Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            helper_failure("capture helper is temporarily unavailable", reason_code),
        )),
        AcquireResponse::Rejected { reason_code, .. } => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            helper_failure("capture helper rejected the plan", reason_code),
        )),
    }
}

fn helper_failure(message: &str, reason_code: Option<String>) -> String {
    let reason_code = reason_code.filter(|code| {
        !code.is_empty()
            && code.len() <= 64
            && code
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
    });
    reason_code.map_or_else(|| message.to_owned(), |code| format!("{message}: {code}"))
}

fn validate_helper(path: &Path) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !path.is_absolute() || metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "capture helper must be an absolute regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "capture helper must be executable",
            ));
        }
    }
    Ok(())
}

fn terminate(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use x_img_core::viewed_media::{AdapterKind, CAPTURE_PLAN_SCHEMA_VERSION, CapturePlanState};

    fn plan() -> CapturePlan {
        CapturePlan {
            schema_version: CAPTURE_PLAN_SCHEMA_VERSION,
            plan_id: "capture-plan-0".into(),
            scheduler_job_id: "refresh-0".into(),
            site_id: "site-1".into(),
            origin: "https://example.invalid".into(),
            canonical_page_url: "https://example.invalid/gallery".into(),
            canonical_media_url: "https://example.invalid/thumb.jpg".into(),
            retrieval_media_url: "https://example.invalid/thumb.jpg".into(),
            destination: None,
            canonical_presentation_url: "https://example.invalid/thumb.jpg".into(),
            catalogue_id: "website-card-1".into(),
            adapter_kind: AdapterKind::ExperimentalGeneric,
            adapter_version: "1.0.0".into(),
            capture_kind: CaptureKind::ObservedThumbnail,
            width: 320,
            height: 200,
            state: CapturePlanState::AwaitingApprovedAcquisition,
        }
    }

    #[cfg(unix)]
    #[test]
    fn helper_returns_only_verified_metadata_for_the_fixed_destination() {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!(
            "pinakotheke-capture-helper-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(
            &path,
            r##"#!/bin/sh
test "$1" = acquire-image-v1 || exit 2
read request
printf '%s' "$request" | grep -q '"canonical_media_url":"https://example.invalid/thumb.jpg"' || exit 3
printf '%s\n' '{"outcome":"committed","schema_version":"pinakotheke.capture-acquire-helper.v1","catalogue_id":"card-1","title":"Synthetic image","content_type":"image/jpeg","content_length":42,"endpoint_id":"endpoint-1","object_store_id":"store-1","object_key":"object-1","object_version":2,"checksum_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","verified_at_epoch_seconds":42}' >&2
"##,
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let receipt = acquire(&path, "actor-1", &plan(), "endpoint-1", "store-1").unwrap();
        assert_eq!(receipt.object_version, 2);
        assert_eq!(receipt.content_length, 42);
        let _ = std::fs::remove_file(path);
    }
}
