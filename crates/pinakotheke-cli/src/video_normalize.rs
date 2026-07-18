// SPDX-License-Identifier: MPL-2.0
//! Reviewed host-side video normalization and streaming DAS ingest.

use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
};

use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use x_img_core::{
    destination::ReviewedDestination,
    object_ingest::{
        CommitReceipt, IngestBackendError, IngestRequest, ObjectIngestBackend,
        StreamingObjectIngestor,
    },
    video_normalization::{
        CancellationToken, DockerFfmpegAdapter, NORMALIZATION_SCHEMA, SystemDockerRuntime,
    },
    video_profile::{
        AudioCodec, CodecVariant, DockerExecutionPlan, ExecutionPlacement, ScratchAuthority,
        VideoCodec,
    },
};

const PLAN_SCHEMA: &str = "pinakotheke.video-normalize-plan.v1";
const INGEST_SCHEMA: &str = "pinakotheke.object-ingest-stream.v1";
const MAX_PLAN_BYTES: u64 = 32 * 1024;
const MAX_RECEIPT_BYTES: usize = 16 * 1024;

#[derive(Debug, PartialEq, Eq, Subcommand)]
pub(crate) enum VideoCommand {
    /// Run one confirmed normalization plan on this reviewed host.
    Normalize(NormalizeArgs),
}

#[derive(Debug, PartialEq, Eq, Args)]
pub(crate) struct NormalizeArgs {
    /// Private mode-0600 confirmed plan produced by the host.
    #[arg(long)]
    plan: PathBuf,
    /// Reviewed Docker CLI executable (absolute, regular, not a symlink).
    #[arg(long)]
    docker: PathBuf,
    /// Reviewed Pinakotheke executable providing ingest-stream-v1.
    #[arg(long)]
    ingest_helper: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanDocument {
    schema_version: String,
    job_id: String,
    source_identity: String,
    profile_id: String,
    video_codec: String,
    audio_codec: String,
    endpoint_id: String,
    object_store_id: String,
    actor_ref: String,
    reviewed_at_unix_seconds: u64,
    pairing_ref: String,
    device_ref: String,
    image_reference: String,
    image_digest: String,
    cpu_millis_limit: u32,
    memory_bytes_limit: u64,
    scratch_bytes_limit: u64,
    cleanup_ref: String,
    scratch_root: PathBuf,
    normalized_object_key: String,
    poster_object_key: String,
    manifest_object_key: String,
}

#[derive(Serialize)]
struct IngestHeader<'a> {
    schema_version: &'static str,
    ingest_id: &'a str,
    endpoint_id: &'a str,
    object_store_id: &'a str,
    object_key: &'a str,
    expected_size_bytes: u64,
    expected_checksum: &'a str,
    content_type: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HelperReceipt {
    schema_version: String,
    endpoint_id: String,
    object_store_id: String,
    object_key: String,
    size_bytes: u64,
    checksum: String,
    object_reference: String,
}

pub(crate) struct Upload {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
}

impl Drop for Upload {
    fn drop(&mut self) {
        drop(self.stdin.take());
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub(crate) struct ProcessIngestBackend {
    helper: PathBuf,
    content_types: BTreeMap<String, &'static str>,
}

impl ObjectIngestBackend for ProcessIngestBackend {
    type Upload = Upload;

    fn begin(&mut self, request: &IngestRequest) -> Result<Self::Upload, IngestBackendError> {
        let content_type = self
            .content_types
            .get(&request.object_key)
            .copied()
            .ok_or_else(|| rejected("ingest object type was not reviewed"))?;
        let mut child = Command::new(&self.helper)
            .arg("ingest-stream-v1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| rejected("ingest helper is unavailable"))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| rejected("ingest helper has no input"))?;
        let header = IngestHeader {
            schema_version: INGEST_SCHEMA,
            ingest_id: &request.ingest_id,
            endpoint_id: &request.endpoint_id,
            object_store_id: &request.object_store_id,
            object_key: &request.object_key,
            expected_size_bytes: request.expected_size_bytes,
            expected_checksum: &request.expected_checksum,
            content_type,
        };
        if serde_json::to_writer(&mut stdin, &header)
            .and_then(|()| stdin.write_all(b"\n").map_err(serde_json::Error::io))
            .is_err()
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(rejected("ingest helper rejected its request header"));
        }
        Ok(Upload {
            child: Some(child),
            stdin: Some(stdin),
        })
    }

    fn write_chunk(
        &mut self,
        upload: &mut Self::Upload,
        chunk: &[u8],
    ) -> Result<(), IngestBackendError> {
        upload
            .stdin
            .as_mut()
            .ok_or_else(|| rejected("ingest helper input closed early"))?
            .write_all(chunk)
            .map_err(|_| rejected("ingest helper stopped during streaming"))
    }

    fn complete(
        &mut self,
        mut upload: Self::Upload,
        expected: &CommitReceipt,
    ) -> Result<CommitReceipt, IngestBackendError> {
        drop(upload.stdin.take());
        let output = upload
            .child
            .take()
            .ok_or_else(|| rejected("ingest helper process is unavailable"))?
            .wait_with_output()
            .map_err(|_| rejected("ingest helper completion failed"))?;
        if !output.status.success() || output.stdout.len() > MAX_RECEIPT_BYTES {
            return Err(rejected("ingest helper did not verify completion"));
        }
        let receipt: HelperReceipt = serde_json::from_slice(&output.stdout)
            .map_err(|_| rejected("ingest helper returned an invalid receipt"))?;
        if receipt.schema_version != INGEST_SCHEMA {
            return Err(rejected("ingest helper receipt schema changed"));
        }
        let actual = CommitReceipt {
            endpoint_id: receipt.endpoint_id,
            object_store_id: receipt.object_store_id,
            object_key: receipt.object_key,
            size_bytes: receipt.size_bytes,
            checksum: receipt.checksum,
            object_reference: receipt.object_reference,
        };
        if &actual != expected {
            return Err(rejected("ingest helper changed the reviewed authority"));
        }
        Ok(actual)
    }
}

pub(crate) fn run(command: VideoCommand) -> Result<(), Box<dyn std::error::Error>> {
    let VideoCommand::Normalize(arguments) = command;
    require_executable(&arguments.docker)?;
    require_executable(&arguments.ingest_helper)?;
    require_private_regular(&arguments.plan)?;
    let metadata = fs::metadata(&arguments.plan)?;
    if metadata.len() > MAX_PLAN_BYTES {
        return Err("normalization plan exceeds 32 KiB".into());
    }
    let document: PlanDocument = serde_json::from_slice(&fs::read(&arguments.plan)?)?;
    let plan = build_plan(document)?;
    let result = normalize_with_process_ingest(&plan, arguments.docker, arguments.ingest_helper)?;
    println!(
        "normalized {} bytes as {} with poster {} and manifest {}",
        result.normalized_video.size_bytes,
        result.normalized_video.object_reference,
        result.poster.object_reference,
        result.provenance_manifest.object_reference
    );
    Ok(())
}

/// Runs the reviewed Docker adapter and commits all derived objects through
/// the same bounded process-isolated DASObjectStore ingest helper used by the
/// explicit CLI command. The caller owns plan construction and policy gates.
pub(crate) fn normalize_with_process_ingest(
    plan: &x_img_core::video_normalization::PairedDeviceNormalizationPlan,
    docker: PathBuf,
    ingest_helper: PathBuf,
) -> Result<x_img_core::video_normalization::NormalizationResult, Box<dyn std::error::Error>> {
    require_executable(&docker)?;
    require_executable(&ingest_helper)?;
    let profile = x_img_core::video_profile::playback_profile(&plan.profile_id)
        .ok_or("normalization profile is unavailable")?;
    let backend = ProcessIngestBackend {
        helper: ingest_helper,
        content_types: BTreeMap::from([
            (plan.normalized_object_key.clone(), profile.content_type),
            (plan.poster_object_key.clone(), "image/webp"),
            (plan.manifest_object_key.clone(), "application/json"),
        ]),
    };
    let mut ingestor = StreamingObjectIngestor::new(backend);
    let mut adapter = DockerFfmpegAdapter::new(SystemDockerRuntime::new(docker));
    Ok(adapter.normalize_and_ingest(plan, &mut ingestor, &CancellationToken::new())?)
}

fn build_plan(
    document: PlanDocument,
) -> Result<
    x_img_core::video_normalization::PairedDeviceNormalizationPlan,
    Box<dyn std::error::Error>,
> {
    if document.schema_version != PLAN_SCHEMA {
        return Err("normalization plan schema is unsupported".into());
    }
    require_pristine_scratch(&document.scratch_root)?;
    let variant = CodecVariant {
        video: match document.video_codec.as_str() {
            "vp9" => VideoCodec::Vp9,
            "av1" => VideoCodec::Av1,
            "h264" => VideoCodec::H264,
            _ => return Err("normalization video codec is unsupported".into()),
        },
        audio: match document.audio_codec.as_str() {
            "opus" => AudioCodec::Opus,
            "aac" => AudioCodec::Aac,
            _ => return Err("normalization audio codec is unsupported".into()),
        },
    };
    Ok(
        x_img_core::video_normalization::PairedDeviceNormalizationPlan {
            schema_version: NORMALIZATION_SCHEMA,
            job_id: document.job_id,
            source_identity: document.source_identity,
            profile_id: document.profile_id,
            output_variant: variant,
            destination: ReviewedDestination {
                endpoint_id: document.endpoint_id,
                object_store_id: document.object_store_id,
                object_type: "video".into(),
                selection_kind: "site_override".into(),
                reviewed_at_unix_seconds: document.reviewed_at_unix_seconds,
                actor_ref: document.actor_ref,
            },
            executor: DockerExecutionPlan {
                placement: ExecutionPlacement::PairedFirefoxDevice {
                    pairing_ref: document.pairing_ref,
                    device_ref: document.device_ref,
                },
                image_reference: document.image_reference,
                image_digest: document.image_digest,
                cpu_millis_limit: document.cpu_millis_limit,
                memory_bytes_limit: document.memory_bytes_limit,
                scratch_bytes_limit: document.scratch_bytes_limit,
            },
            scratch: ScratchAuthority::BoundedEphemeral {
                cleanup_ref: document.cleanup_ref,
            },
            input_file: document.scratch_root.join("input.media"),
            scratch_root: document.scratch_root,
            normalized_object_key: document.normalized_object_key,
            poster_object_key: document.poster_object_key,
            manifest_object_key: document.manifest_object_key,
        },
    )
}

fn require_private_regular(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "plan path must be absolute",
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "plan must be a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "plan must be mode 0600 or stricter",
            ));
        }
    }
    Ok(())
}

fn require_executable(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "executable must be absolute",
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.permissions().mode() & 0o111 == 0
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "executable is not reviewed",
            ));
        }
    }
    Ok(())
}

fn require_pristine_scratch(path: &Path) -> io::Result<()> {
    if !path.is_absolute() || !path.starts_with(std::env::temp_dir()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "scratch must be below the system temporary root",
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "scratch must be a real directory",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "scratch must be mode 0700 or stricter",
            ));
        }
    }
    let mut entries = fs::read_dir(path)?;
    let input = entries
        .next()
        .transpose()?
        .filter(|entry| entry.file_name() == "input.media")
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "scratch must contain only input.media",
            )
        })?;
    if entries.next().transpose()?.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "scratch must contain only input.media",
        ));
    }
    let input_metadata = fs::symlink_metadata(input.path())?;
    if !input_metadata.is_file()
        || input_metadata.file_type().is_symlink()
        || input_metadata.len() == 0
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "input.media must be a non-empty regular file",
        ));
    }
    Ok(())
}

fn rejected(message: &str) -> IngestBackendError {
    IngestBackendError::Rejected(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn streams_three_verified_objects_and_removes_scratch() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pinakotheke-video-cli-{nonce}"));
        let scratch = root.join("scratch");
        fs::create_dir_all(&scratch).unwrap();
        private(&root, 0o700);
        private(&scratch, 0o700);
        fs::write(scratch.join("input.media"), b"synthetic source").unwrap();

        let docker = root.join("docker-fixture");
        fs::write(
            &docker,
            format!(
                "#!/bin/sh\ncase \"$*\" in\n  *--entrypoint\\ ffprobe*) printf '%s' '{{\"streams\":[{{\"codec_type\":\"video\",\"codec_name\":\"h264\",\"width\":64,\"height\":48}},{{\"codec_type\":\"audio\",\"codec_name\":\"aac\"}}],\"format\":{{\"duration\":\"1.0\"}}}}' ;;\n  *poster.webp*) printf poster > '{}/poster.webp' ;;\n  *) printf normalized > '{}/normalized.mp4' ;;\nesac\n",
                scratch.display(),
                scratch.display()
            ),
        )
        .unwrap();
        private(&docker, 0o700);

        let helper = root.join("ingest-fixture.py");
        fs::write(
            &helper,
            r#"#!/usr/bin/env python3
import hashlib,json,sys
h=json.loads(sys.stdin.buffer.readline())
p=sys.stdin.buffer.read()
assert len(p)==h['expected_size_bytes']
assert 'sha256:'+hashlib.sha256(p).hexdigest()==h['expected_checksum']
print(json.dumps({'schema_version':h['schema_version'],'endpoint_id':h['endpoint_id'],'object_store_id':h['object_store_id'],'object_key':h['object_key'],'size_bytes':len(p),'checksum':h['expected_checksum'],'object_reference':'dasobjectstore:'+h['endpoint_id']+':'+h['object_store_id']+':'+h['object_key']}))
"#,
        )
        .unwrap();
        private(&helper, 0o700);

        let plan = root.join("plan.json");
        fs::write(
            &plan,
            format!(
                r#"{{"schema_version":"{PLAN_SCHEMA}","job_id":"job-1","source_identity":"source-1","profile_id":"pinakotheke-video-mp4-v1","video_codec":"h264","audio_codec":"aac","endpoint_id":"endpoint-1","object_store_id":"store-1","actor_ref":"actor:1","reviewed_at_unix_seconds":1,"pairing_ref":"pairing:1","device_ref":"device:1","image_reference":"registry://fixture/ffmpeg","image_digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","cpu_millis_limit":1000,"memory_bytes_limit":67108864,"scratch_bytes_limit":1048576,"cleanup_ref":"cleanup:1","scratch_root":"{}","normalized_object_key":"video/normalized.mp4","poster_object_key":"video/poster.webp","manifest_object_key":"video/manifest.json"}}"#,
                scratch.display()
            ),
        )
        .unwrap();
        private(&plan, 0o600);

        run(VideoCommand::Normalize(NormalizeArgs {
            plan,
            docker,
            ingest_helper: helper,
        }))
        .unwrap();
        assert!(!scratch.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    fn private(path: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }
}
