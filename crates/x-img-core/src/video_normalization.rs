// SPDX-License-Identifier: MPL-2.0
//! Paired-device, Docker-pinned FFmpeg normalization adapter.
//!
//! It uses an isolated ephemeral worker directory and streams resulting files
//! through the DASObjectStore ingest port before explicit cleanup. It never
//! opens a shell, browser storage, a user-selected arbitrary directory, or a
//! network connection inside the conversion container.

#![allow(missing_docs)]

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    destination::ReviewedDestination,
    object_ingest::{CommitReceipt, IngestRequest, ObjectIngestBackend, StreamingObjectIngestor},
    video_profile::{
        AudioCodec, CodecVariant, DockerExecutionPlan, ExecutionPlacement, ScratchAuthority,
        VideoCodec, playback_profile,
    },
};

pub const NORMALIZATION_CHUNK_BYTES: usize = 65_536;
pub const NORMALIZATION_SCHEMA: &str = "x-img.video-normalization.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationPhase {
    Planned,
    Normalizing,
    Probing,
    Ingesting,
    AwaitingFirefoxPlayback,
    Completed,
    Cancelled,
    Failed,
    ReconciliationRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizationCheckpoint {
    pub job_id: String,
    pub source_identity: String,
    pub profile_id: String,
    pub image_digest: String,
    pub phase: NormalizationPhase,
    pub reason: Option<String>,
}

#[derive(Debug, Default)]
pub struct NormalizationLedger {
    checkpoints: BTreeMap<String, NormalizationCheckpoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartDecision {
    Started,
    AlreadyCompleted,
    ReconciliationRequired,
}

impl NormalizationLedger {
    pub fn start(
        &mut self,
        job_id: &str,
        source_identity: &str,
        profile_id: &str,
        image_digest: &str,
    ) -> Result<StartDecision, NormalizationError> {
        if !identifier(job_id) || !identifier(source_identity) || !sha256(image_digest) {
            return Err(NormalizationError::InvalidPlan);
        }
        if let Some(checkpoint) = self.checkpoints.get(job_id) {
            if checkpoint.source_identity != source_identity
                || checkpoint.profile_id != profile_id
                || checkpoint.image_digest != image_digest
            {
                return Err(NormalizationError::IdempotencyConflict);
            }
            return Ok(match checkpoint.phase {
                NormalizationPhase::Completed => StartDecision::AlreadyCompleted,
                NormalizationPhase::Normalizing
                | NormalizationPhase::Probing
                | NormalizationPhase::Ingesting => StartDecision::ReconciliationRequired,
                _ => StartDecision::Started,
            });
        }
        self.checkpoints.insert(
            job_id.to_owned(),
            NormalizationCheckpoint {
                job_id: job_id.to_owned(),
                source_identity: source_identity.to_owned(),
                profile_id: profile_id.to_owned(),
                image_digest: image_digest.to_owned(),
                phase: NormalizationPhase::Planned,
                reason: None,
            },
        );
        Ok(StartDecision::Started)
    }

    pub fn mark(
        &mut self,
        job_id: &str,
        phase: NormalizationPhase,
        reason: Option<&str>,
    ) -> Result<(), NormalizationError> {
        let checkpoint = self
            .checkpoints
            .get_mut(job_id)
            .ok_or(NormalizationError::UnknownJob)?;
        checkpoint.phase = phase;
        checkpoint.reason = reason.map(str::to_owned);
        Ok(())
    }

    /// Fails unfinished work after a process crash; it never assumes an
    /// unverified output is committed or silently resumes from local scratch.
    pub fn reconcile_after_crash(&mut self) -> Vec<NormalizationCheckpoint> {
        let mut reconciled = Vec::new();
        for checkpoint in self.checkpoints.values_mut() {
            if matches!(
                checkpoint.phase,
                NormalizationPhase::Normalizing
                    | NormalizationPhase::Probing
                    | NormalizationPhase::Ingesting
            ) {
                checkpoint.phase = NormalizationPhase::ReconciliationRequired;
                checkpoint.reason =
                    Some("worker ended before verified DASObjectStore commit".into());
                reconciled.push(checkpoint.clone());
            }
        }
        reconciled
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedDeviceNormalizationPlan {
    pub schema_version: &'static str,
    pub job_id: String,
    pub source_identity: String,
    pub profile_id: String,
    pub output_variant: CodecVariant,
    pub destination: ReviewedDestination,
    pub executor: DockerExecutionPlan,
    pub scratch: ScratchAuthority,
    pub scratch_root: PathBuf,
    pub input_file: PathBuf,
    pub normalized_object_key: String,
    pub poster_object_key: String,
    pub manifest_object_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerInvocation {
    pub arguments: Vec<String>,
    pub capture_stdout: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeManifest {
    pub profile_id: String,
    pub container: String,
    pub video_codec: String,
    pub audio_codec: String,
    pub width: u32,
    pub height: u32,
    pub duration_millis: u64,
    pub output_checksum: String,
    pub output_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizationResult {
    pub normalized_video: CommitReceipt,
    pub poster: CommitReceipt,
    pub provenance_manifest: CommitReceipt,
    pub probe: ProbeManifest,
    pub phase: NormalizationPhase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizationProgressEvent {
    pub job_id: String,
    pub phase: NormalizationPhase,
    pub detail: Option<String>,
}

pub trait NormalizationProgressSink {
    fn report(&mut self, event: NormalizationProgressEvent);
}

struct NoProgress;

impl NormalizationProgressSink for NoProgress {
    fn report(&mut self, _: NormalizationProgressEvent) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockerRuntimeError {
    Unavailable,
    Failed,
    Cancelled,
    ProbeOutputInvalid,
}

pub trait DockerRuntime {
    fn run(
        &mut self,
        invocation: &DockerInvocation,
        cancellation: &CancellationToken,
    ) -> Result<String, DockerRuntimeError>;
}

/// Docker CLI runtime. `Command` receives an argument vector directly; there
/// is no shell, script, interpolation, network access, or captured stderr.
pub struct SystemDockerRuntime;

impl DockerRuntime for SystemDockerRuntime {
    fn run(
        &mut self,
        invocation: &DockerInvocation,
        cancellation: &CancellationToken,
    ) -> Result<String, DockerRuntimeError> {
        if cancellation.is_cancelled() {
            return Err(DockerRuntimeError::Cancelled);
        }
        let mut command = Command::new("docker");
        command.args(&invocation.arguments).stderr(Stdio::null());
        if invocation.capture_stdout {
            command.stdout(Stdio::piped());
            let output = command
                .output()
                .map_err(|_| DockerRuntimeError::Unavailable)?;
            if cancellation.is_cancelled() {
                return Err(DockerRuntimeError::Cancelled);
            }
            if !output.status.success() || output.stdout.len() > 65_536 {
                return Err(DockerRuntimeError::Failed);
            }
            return String::from_utf8(output.stdout)
                .map_err(|_| DockerRuntimeError::ProbeOutputInvalid);
        }
        command.stdout(Stdio::null());
        let mut child = command
            .spawn()
            .map_err(|_| DockerRuntimeError::Unavailable)?;
        loop {
            if cancellation.is_cancelled() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(DockerRuntimeError::Cancelled);
            }
            if let Some(status) = child.try_wait().map_err(|_| DockerRuntimeError::Failed)? {
                return status
                    .success()
                    .then_some(String::new())
                    .ok_or(DockerRuntimeError::Failed);
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizationError {
    InvalidPlan,
    UnknownJob,
    IdempotencyConflict,
    Runtime(DockerRuntimeError),
    Probe,
    FileIo,
    ScratchLimit,
    Ingest,
}

impl std::fmt::Display for NormalizationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPlan => "normalization plan is invalid",
            Self::UnknownJob => "normalization job is unknown",
            Self::IdempotencyConflict => "normalization job ID was reused with different evidence",
            Self::Runtime(DockerRuntimeError::Unavailable) => "Docker runtime is unavailable",
            Self::Runtime(DockerRuntimeError::Failed) => "Docker normalization failed",
            Self::Runtime(DockerRuntimeError::Cancelled) => "Docker normalization was cancelled",
            Self::Runtime(DockerRuntimeError::ProbeOutputInvalid) => {
                "Docker probe output was invalid"
            }
            Self::Probe => "normalized output does not match the selected playback profile",
            Self::FileIo => "bounded ephemeral normalization file could not be read",
            Self::ScratchLimit => "normalization output exceeds the approved scratch bound",
            Self::Ingest => "DASObjectStore ingest did not verify the normalized artifact",
        })
    }
}

impl std::error::Error for NormalizationError {}

pub struct DockerFfmpegAdapter<R> {
    runtime: R,
}

impl<R: DockerRuntime> DockerFfmpegAdapter<R> {
    pub fn new(runtime: R) -> Self {
        Self { runtime }
    }

    pub fn build_invocations(
        plan: &PairedDeviceNormalizationPlan,
    ) -> Result<Vec<DockerInvocation>, NormalizationError> {
        validate_plan(plan)?;
        let image = pinned_image(&plan.executor)?;
        let common = docker_prefix(plan, &image)?;
        let profile = playback_profile(&plan.profile_id).ok_or(NormalizationError::InvalidPlan)?;
        let mut normalize = common.clone();
        normalize.extend([
            image.clone(),
            "-nostdin".into(),
            "-v".into(),
            "error".into(),
            "-y".into(),
            "-i".into(),
            "/work/input.media".into(),
            "-map".into(),
            "0:v:0".into(),
            "-map".into(),
            "0:a:0?".into(),
        ]);
        match plan.output_variant.video {
            VideoCodec::Vp9 => normalize.extend([
                "-c:v".into(),
                "libvpx-vp9".into(),
                "-row-mt".into(),
                "1".into(),
            ]),
            VideoCodec::Av1 => normalize.extend(["-c:v".into(), "libaom-av1".into()]),
            VideoCodec::H264 => normalize.extend([
                "-c:v".into(),
                "libx264".into(),
                "-movflags".into(),
                "+faststart".into(),
            ]),
        }
        normalize.extend(match plan.output_variant.audio {
            AudioCodec::Opus => ["-c:a".into(), "libopus".into()],
            AudioCodec::Aac => ["-c:a".into(), "aac".into()],
        });
        normalize.push(format!(
            "/work/normalized.{}",
            extension(profile.content_type)
        ));

        let mut poster = common.clone();
        poster.extend([
            image.clone(),
            "-nostdin".into(),
            "-v".into(),
            "error".into(),
            "-y".into(),
            "-i".into(),
            format!("/work/normalized.{}", extension(profile.content_type)),
            "-frames:v".into(),
            "1".into(),
            "/work/poster.webp".into(),
        ]);

        let mut probe = common;
        probe.extend([
            "--entrypoint".into(),
            "ffprobe".into(),
            image,
            "-v".into(),
            "error".into(),
            "-show_entries".into(),
            "format=duration,size:stream=codec_name,codec_type,width,height".into(),
            "-of".into(),
            "json".into(),
            format!("/work/normalized.{}", extension(profile.content_type)),
        ]);
        Ok(vec![
            DockerInvocation {
                arguments: normalize,
                capture_stdout: false,
            },
            DockerInvocation {
                arguments: poster,
                capture_stdout: false,
            },
            DockerInvocation {
                arguments: probe,
                capture_stdout: true,
            },
        ])
    }

    pub fn normalize_and_ingest<B: ObjectIngestBackend>(
        &mut self,
        plan: &PairedDeviceNormalizationPlan,
        ingestor: &mut StreamingObjectIngestor<B>,
        cancellation: &CancellationToken,
    ) -> Result<NormalizationResult, NormalizationError> {
        self.normalize_and_ingest_with_progress(plan, ingestor, cancellation, &mut NoProgress)
    }

    pub fn normalize_and_ingest_with_progress<
        B: ObjectIngestBackend,
        P: NormalizationProgressSink,
    >(
        &mut self,
        plan: &PairedDeviceNormalizationPlan,
        ingestor: &mut StreamingObjectIngestor<B>,
        cancellation: &CancellationToken,
        progress: &mut P,
    ) -> Result<NormalizationResult, NormalizationError> {
        let invocations = match Self::build_invocations(plan) {
            Ok(invocations) => invocations,
            Err(error) => {
                let _ = cleanup_scratch(plan);
                return Err(error);
            }
        };
        let result: Result<NormalizationResult, NormalizationError> = (|| {
            report_progress(progress, plan, NormalizationPhase::Normalizing, None);
            self.runtime
                .run(&invocations[0], cancellation)
                .map_err(NormalizationError::Runtime)?;
            self.runtime
                .run(&invocations[1], cancellation)
                .map_err(NormalizationError::Runtime)?;
            report_progress(progress, plan, NormalizationPhase::Probing, None);
            let probe_json = self
                .runtime
                .run(&invocations[2], cancellation)
                .map_err(NormalizationError::Runtime)?;
            let profile =
                playback_profile(&plan.profile_id).ok_or(NormalizationError::InvalidPlan)?;
            let normalized_path = plan
                .scratch_root
                .join(format!("normalized.{}", extension(profile.content_type)));
            let poster_path = plan.scratch_root.join("poster.webp");
            let (checksum, size) = checksum_and_size(&normalized_path)?;
            let probe = parse_probe(&probe_json, plan, profile.content_type, checksum, size)?;
            enforce_scratch_limit(plan, &[&normalized_path, &poster_path])?;
            report_progress(progress, plan, NormalizationPhase::Ingesting, None);
            let normalized_video = ingest_file(
                ingestor,
                plan,
                "normalized",
                &plan.normalized_object_key,
                &normalized_path,
            )?;
            let poster = ingest_file(
                ingestor,
                plan,
                "poster",
                &plan.poster_object_key,
                &poster_path,
            )?;
            let manifest_path = plan.scratch_root.join("provenance.json");
            let manifest = manifest_json(plan, &probe, &normalized_video, &poster);
            fs::write(&manifest_path, manifest).map_err(|_| NormalizationError::FileIo)?;
            let provenance_manifest = ingest_file(
                ingestor,
                plan,
                "manifest",
                &plan.manifest_object_key,
                &manifest_path,
            )?;
            Ok(NormalizationResult {
                normalized_video,
                poster,
                provenance_manifest,
                probe,
                phase: NormalizationPhase::AwaitingFirefoxPlayback,
            })
        })();
        let cleanup = cleanup_scratch(plan);
        match (result, cleanup) {
            (Ok(result), Ok(())) => {
                report_progress(
                    progress,
                    plan,
                    NormalizationPhase::AwaitingFirefoxPlayback,
                    None,
                );
                Ok(result)
            }
            (Ok(_), Err(error)) => Err(error),
            (Err(error), _) => {
                let phase = if error == NormalizationError::Runtime(DockerRuntimeError::Cancelled) {
                    NormalizationPhase::Cancelled
                } else {
                    NormalizationPhase::Failed
                };
                report_progress(progress, plan, phase, Some(error.to_string()));
                Err(error)
            }
        }
    }

    pub fn into_runtime(self) -> R {
        self.runtime
    }
}

fn report_progress<P: NormalizationProgressSink>(
    sink: &mut P,
    plan: &PairedDeviceNormalizationPlan,
    phase: NormalizationPhase,
    detail: Option<String>,
) {
    sink.report(NormalizationProgressEvent {
        job_id: plan.job_id.clone(),
        phase,
        detail,
    });
}

fn validate_plan(plan: &PairedDeviceNormalizationPlan) -> Result<(), NormalizationError> {
    if plan.schema_version != NORMALIZATION_SCHEMA
        || !identifier(&plan.job_id)
        || !identifier(&plan.source_identity)
        || !identifier(&plan.destination.endpoint_id)
        || !identifier(&plan.destination.object_store_id)
        || plan.destination.object_type != "video"
        || !matches!(
            plan.executor.placement,
            ExecutionPlacement::PairedFirefoxDevice { .. }
        )
        || !matches!(plan.scratch, ScratchAuthority::BoundedEphemeral { .. })
        || !plan.scratch_root.is_absolute()
        || !plan.scratch_root.starts_with(std::env::temp_dir())
        || plan.input_file != plan.scratch_root.join("input.media")
        || !plan.input_file.exists()
        || playback_profile(&plan.profile_id).is_none()
    {
        return Err(NormalizationError::InvalidPlan);
    }
    let profile = playback_profile(&plan.profile_id).expect("checked profile");
    if !profile.variants.contains(&plan.output_variant) {
        return Err(NormalizationError::InvalidPlan);
    }
    if plan.executor.cpu_millis_limit == 0
        || plan.executor.memory_bytes_limit == 0
        || plan.executor.scratch_bytes_limit == 0
        || !plan.executor.image_reference.starts_with("registry://")
        || !sha256(&plan.executor.image_digest)
    {
        return Err(NormalizationError::InvalidPlan);
    }
    Ok(())
}

fn pinned_image(executor: &DockerExecutionPlan) -> Result<String, NormalizationError> {
    let repository = executor
        .image_reference
        .strip_prefix("registry://")
        .filter(|value| !value.is_empty() && !value.contains(['@', '?', '#', ' ']))
        .ok_or(NormalizationError::InvalidPlan)?;
    Ok(format!("{repository}@{}", executor.image_digest))
}

fn docker_prefix(
    plan: &PairedDeviceNormalizationPlan,
    _image: &str,
) -> Result<Vec<String>, NormalizationError> {
    let scratch = plan
        .scratch_root
        .to_str()
        .filter(|value| !value.contains(['\n', '\r']))
        .ok_or(NormalizationError::InvalidPlan)?;
    Ok(vec![
        "run".into(),
        "--rm".into(),
        "--network".into(),
        "none".into(),
        "--read-only".into(),
        "--cap-drop".into(),
        "ALL".into(),
        "--security-opt".into(),
        "no-new-privileges".into(),
        "--pids-limit".into(),
        "128".into(),
        "--cpus".into(),
        format!("{}", f64::from(plan.executor.cpu_millis_limit) / 1000.0),
        "--memory".into(),
        plan.executor.memory_bytes_limit.to_string(),
        "--tmpfs".into(),
        format!(
            "/tmp:rw,noexec,nosuid,size={}",
            plan.executor.scratch_bytes_limit
        ),
        "--mount".into(),
        format!("type=bind,src={scratch},dst=/work,rw"),
        "--workdir".into(),
        "/work".into(),
    ])
}

fn extension(content_type: &str) -> &'static str {
    match content_type {
        "video/webm" => "webm",
        "video/mp4" => "mp4",
        _ => "bin",
    }
}

fn checksum_and_size(path: &Path) -> Result<(String, u64), NormalizationError> {
    let mut file = File::open(path).map_err(|_| NormalizationError::FileIo)?;
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; NORMALIZATION_CHUNK_BYTES];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| NormalizationError::FileIo)?;
        if count == 0 {
            break;
        }
        size = size.saturating_add(u64::try_from(count).map_err(|_| NormalizationError::FileIo)?);
        hasher.update(&buffer[..count]);
    }
    (size > 0)
        .then_some((format!("sha256:{:x}", hasher.finalize()), size))
        .ok_or(NormalizationError::Probe)
}

fn parse_probe(
    json: &str,
    plan: &PairedDeviceNormalizationPlan,
    content_type: &str,
    output_checksum: String,
    output_size_bytes: u64,
) -> Result<ProbeManifest, NormalizationError> {
    let root: Value = serde_json::from_str(json).map_err(|_| NormalizationError::Probe)?;
    let streams = root
        .get("streams")
        .and_then(Value::as_array)
        .ok_or(NormalizationError::Probe)?;
    let video = streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("video"))
        .ok_or(NormalizationError::Probe)?;
    let audio = streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("audio"))
        .ok_or(NormalizationError::Probe)?;
    let video_codec = video
        .get("codec_name")
        .and_then(Value::as_str)
        .ok_or(NormalizationError::Probe)?;
    let audio_codec = audio
        .get("codec_name")
        .and_then(Value::as_str)
        .ok_or(NormalizationError::Probe)?;
    if !matches_variant(video_codec, audio_codec, plan.output_variant) {
        return Err(NormalizationError::Probe);
    }
    let duration_seconds = root
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or(NormalizationError::Probe)?;
    Ok(ProbeManifest {
        profile_id: plan.profile_id.clone(),
        container: extension(content_type).into(),
        video_codec: video_codec.into(),
        audio_codec: audio_codec.into(),
        width: video
            .get("width")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(NormalizationError::Probe)?,
        height: video
            .get("height")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(NormalizationError::Probe)?,
        duration_millis: (duration_seconds * 1000.0) as u64,
        output_checksum,
        output_size_bytes,
    })
}

fn matches_variant(video: &str, audio: &str, variant: CodecVariant) -> bool {
    matches!(
        (video, audio, variant),
        (
            "vp9",
            "opus",
            CodecVariant {
                video: VideoCodec::Vp9,
                audio: AudioCodec::Opus
            }
        ) | (
            "av1",
            "opus",
            CodecVariant {
                video: VideoCodec::Av1,
                audio: AudioCodec::Opus
            }
        ) | (
            "h264",
            "aac",
            CodecVariant {
                video: VideoCodec::H264,
                audio: AudioCodec::Aac
            }
        )
    )
}

fn enforce_scratch_limit(
    plan: &PairedDeviceNormalizationPlan,
    paths: &[&Path],
) -> Result<(), NormalizationError> {
    let total = paths.iter().try_fold(0_u64, |total, path| {
        fs::metadata(path)
            .map_err(|_| NormalizationError::FileIo)
            .map(|metadata| total.saturating_add(metadata.len()))
    })?;
    (total <= plan.executor.scratch_bytes_limit)
        .then_some(())
        .ok_or(NormalizationError::ScratchLimit)
}

fn ingest_file<B: ObjectIngestBackend>(
    ingestor: &mut StreamingObjectIngestor<B>,
    plan: &PairedDeviceNormalizationPlan,
    suffix: &str,
    object_key: &str,
    path: &Path,
) -> Result<CommitReceipt, NormalizationError> {
    let (checksum, size) = checksum_and_size(path)?;
    ingestor
        .stream_ephemeral_file(
            IngestRequest {
                ingest_id: format!("{}:{suffix}", plan.job_id),
                endpoint_id: plan.destination.endpoint_id.clone(),
                object_store_id: plan.destination.object_store_id.clone(),
                object_key: object_key.to_owned(),
                expected_size_bytes: size,
                expected_checksum: checksum,
                max_chunk_bytes: NORMALIZATION_CHUNK_BYTES,
            },
            path,
        )
        .map_err(|_| NormalizationError::Ingest)
}

fn manifest_json(
    plan: &PairedDeviceNormalizationPlan,
    probe: &ProbeManifest,
    normalized: &CommitReceipt,
    poster: &CommitReceipt,
) -> String {
    format!(
        concat!(
            "{{\"schema_version\":\"x-img.normalization-manifest.v1\",",
            "\"source_identity\":\"{}\",\"profile_id\":\"{}\",",
            "\"image_digest\":\"{}\",\"video_checksum\":\"{}\",",
            "\"poster_checksum\":\"{}\",\"duration_millis\":{}}}"
        ),
        plan.source_identity,
        probe.profile_id,
        plan.executor.image_digest,
        normalized.checksum,
        poster.checksum,
        probe.duration_millis
    )
}

fn cleanup_scratch(plan: &PairedDeviceNormalizationPlan) -> Result<(), NormalizationError> {
    fs::remove_dir_all(&plan.scratch_root).map_err(|_| NormalizationError::FileIo)
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::object_ingest::IngestBackendError;

    const DIGEST: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[derive(Default)]
    struct FixtureRuntime {
        calls: Vec<DockerInvocation>,
    }

    impl DockerRuntime for FixtureRuntime {
        fn run(
            &mut self,
            invocation: &DockerInvocation,
            cancellation: &CancellationToken,
        ) -> Result<String, DockerRuntimeError> {
            if cancellation.is_cancelled() {
                return Err(DockerRuntimeError::Cancelled);
            }
            self.calls.push(invocation.clone());
            if invocation.capture_stdout {
                Ok("{\"format\":{\"duration\":\"1.5\",\"size\":\"3\"},\"streams\":[{\"codec_type\":\"video\",\"codec_name\":\"h264\",\"width\":16,\"height\":9},{\"codec_type\":\"audio\",\"codec_name\":\"aac\"}]}".into())
            } else {
                Ok(String::new())
            }
        }
    }

    struct FailingRuntime;

    impl DockerRuntime for FailingRuntime {
        fn run(
            &mut self,
            _: &DockerInvocation,
            _: &CancellationToken,
        ) -> Result<String, DockerRuntimeError> {
            Err(DockerRuntimeError::Failed)
        }
    }

    #[derive(Default)]
    struct Progress(Vec<NormalizationProgressEvent>);

    impl NormalizationProgressSink for Progress {
        fn report(&mut self, event: NormalizationProgressEvent) {
            self.0.push(event);
        }
    }

    #[derive(Default)]
    struct Backend;

    impl ObjectIngestBackend for Backend {
        type Upload = ();

        fn begin(&mut self, _: &IngestRequest) -> Result<Self::Upload, IngestBackendError> {
            Ok(())
        }
        fn write_chunk(
            &mut self,
            _: &mut Self::Upload,
            _: &[u8],
        ) -> Result<(), IngestBackendError> {
            Ok(())
        }
        fn complete(
            &mut self,
            _: Self::Upload,
            expected: &CommitReceipt,
        ) -> Result<CommitReceipt, IngestBackendError> {
            Ok(expected.clone())
        }
    }

    fn plan() -> PairedDeviceNormalizationPlan {
        let root = env::temp_dir().join(format!(
            "x-img-normalization-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir(&root).expect("scratch root");
        let input = root.join("input.media");
        fs::write(&input, b"source").expect("synthetic input");
        PairedDeviceNormalizationPlan {
            schema_version: NORMALIZATION_SCHEMA,
            job_id: "job-1".into(),
            source_identity: "source-1".into(),
            profile_id: "pinakotheke-video-mp4-v1".into(),
            output_variant: CodecVariant {
                video: VideoCodec::H264,
                audio: AudioCodec::Aac,
            },
            destination: ReviewedDestination {
                endpoint_id: "endpoint".into(),
                object_store_id: "store".into(),
                object_type: "video".into(),
                selection_kind: "site_override".into(),
                reviewed_at_unix_seconds: 1,
                actor_ref: "actor".into(),
            },
            executor: DockerExecutionPlan {
                placement: ExecutionPlacement::PairedFirefoxDevice {
                    pairing_ref: "x-img.pairing:device".into(),
                    device_ref: "device:worker".into(),
                },
                image_reference: "registry://ffmpeg/approved".into(),
                image_digest: DIGEST.into(),
                cpu_millis_limit: 1000,
                memory_bytes_limit: 512 * 1024 * 1024,
                scratch_bytes_limit: 8 * 1024 * 1024,
            },
            scratch: ScratchAuthority::BoundedEphemeral {
                cleanup_ref: "x-img.scratch-cleanup:job".into(),
            },
            scratch_root: root,
            input_file: input,
            normalized_object_key: "video/normalized.mp4".into(),
            poster_object_key: "video/poster.webp".into(),
            manifest_object_key: "video/provenance.json".into(),
        }
    }

    #[test]
    fn builds_structured_network_isolated_docker_commands() {
        let plan = plan();
        let commands =
            DockerFfmpegAdapter::<FixtureRuntime>::build_invocations(&plan).expect("commands");
        fs::remove_dir_all(&plan.scratch_root).expect("cleanup");
        assert_eq!(commands.len(), 3);
        assert!(
            commands[0]
                .arguments
                .windows(2)
                .any(|item| item == ["--network", "none"])
        );
        assert!(
            commands[0]
                .arguments
                .iter()
                .any(|item| item == "--read-only")
        );
        assert!(
            commands[0]
                .arguments
                .iter()
                .any(|item| item.contains("@sha256:"))
        );
        assert!(commands[2].capture_stdout);
    }

    #[test]
    fn normalizes_ingests_manifest_and_removes_ephemeral_outputs() {
        let plan = plan();
        fs::write(plan.scratch_root.join("normalized.mp4"), b"video").expect("normalized fixture");
        fs::write(plan.scratch_root.join("poster.webp"), b"poster").expect("poster fixture");
        let mut adapter = DockerFfmpegAdapter::new(FixtureRuntime::default());
        let mut ingestor = StreamingObjectIngestor::new(Backend);
        let mut progress = Progress::default();
        let result = adapter
            .normalize_and_ingest_with_progress(
                &plan,
                &mut ingestor,
                &CancellationToken::new(),
                &mut progress,
            )
            .expect("normalization result");
        assert_eq!(result.phase, NormalizationPhase::AwaitingFirefoxPlayback);
        assert_eq!(
            progress
                .0
                .iter()
                .map(|event| event.phase)
                .collect::<Vec<_>>(),
            vec![
                NormalizationPhase::Normalizing,
                NormalizationPhase::Probing,
                NormalizationPhase::Ingesting,
                NormalizationPhase::AwaitingFirefoxPlayback,
            ]
        );
        assert!(!plan.scratch_root.exists());
    }

    #[test]
    fn failure_removes_the_entire_ephemeral_scratch_directory() {
        let plan = plan();
        let mut adapter = DockerFfmpegAdapter::new(FailingRuntime);
        let mut ingestor = StreamingObjectIngestor::new(Backend);
        assert_eq!(
            adapter.normalize_and_ingest(&plan, &mut ingestor, &CancellationToken::new()),
            Err(NormalizationError::Runtime(DockerRuntimeError::Failed))
        );
        assert!(!plan.scratch_root.exists());
    }

    #[test]
    fn ledger_requires_reconciliation_after_a_crash_and_prevents_conflicting_replay() {
        let mut ledger = NormalizationLedger::default();
        assert_eq!(
            ledger.start("job-1", "source-1", "pinakotheke-video-mp4-v1", DIGEST),
            Ok(StartDecision::Started)
        );
        ledger
            .mark("job-1", NormalizationPhase::Normalizing, None)
            .expect("mark");
        assert_eq!(
            ledger.reconcile_after_crash()[0].phase,
            NormalizationPhase::ReconciliationRequired
        );
        assert_eq!(
            ledger.start("job-1", "source-1", "pinakotheke-video-mp4-v1", DIGEST),
            Ok(StartDecision::Started)
        );
        assert_eq!(
            ledger.start("job-1", "other", "pinakotheke-video-mp4-v1", DIGEST),
            Err(NormalizationError::IdempotencyConflict)
        );
    }
}
