// SPDX-License-Identifier: MPL-2.0
//! First-party bounded capture adapter for DASObjectStore's remote completion client.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};
use x_img_core::{
    destination::ReviewedDestination,
    video_normalization::{NORMALIZATION_SCHEMA, PairedDeviceNormalizationPlan},
    video_profile::{
        AudioCodec, CodecVariant, DockerExecutionPlan, ExecutionPlacement,
        PINAKOTHEKE_VIDEO_MP4_V1, ScratchAuthority, VideoCodec,
    },
};

const REQUEST_SCHEMA: &str = "pinakotheke.capture-acquire-helper.v1";
const CONFIG_SCHEMA: &str = "pinakotheke.das-capture-helper.v1";
const DEFAULT_MAX_BYTES: u64 = 64 * 1024 * 1024;
const DEFAULT_MAX_VIDEO_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: String,
    plan_id: String,
    #[serde(default = "legacy_actor_ref")]
    actor_ref: String,
    site_id: String,
    origin: String,
    canonical_page_url: String,
    canonical_media_url: String,
    #[serde(default)]
    retrieval_media_url: String,
    canonical_presentation_url: String,
    capture_kind: String,
    width: u32,
    height: u32,
    adapter_version: String,
    endpoint_id: String,
    object_store_id: String,
}

fn legacy_actor_ref() -> String {
    "legacy-host-actor".into()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: String,
    endpoint_id: String,
    #[serde(default)]
    object_store_bucket: Option<String>,
    curl_executable: PathBuf,
    #[serde(default)]
    ffprobe_executable: Option<PathBuf>,
    #[serde(default)]
    dasobjectstore_remote_executable: Option<PathBuf>,
    #[serde(default)]
    dasobjectstore_remote_config: Option<PathBuf>,
    #[serde(default)]
    daemon_socket: Option<PathBuf>,
    #[serde(default = "default_submit_to_daemon")]
    submit_to_daemon: bool,
    #[serde(default)]
    container_execution: Option<ContainerExecution>,
    max_image_bytes: Option<u64>,
    max_video_bytes: Option<u64>,
    #[serde(default)]
    normalization: Option<NormalizationHandoff>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NormalizationHandoff {
    docker_executable: PathBuf,
    ingest_helper: PathBuf,
    executor_ref: String,
    staging_ref: String,
    staging_root: PathBuf,
    image_reference: String,
    image_digest: String,
    cpu_millis_limit: u32,
    memory_bytes_limit: u64,
    scratch_bytes_limit: u64,
    firefox_playback_evidence_id: String,
    codec_gap_journal: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VideoProbe {
    container: String,
    video_codec: String,
    audio_codec: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodecGapJournal {
    schema_version: String,
    gaps: BTreeMap<String, CodecGapRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodecGapRecord {
    container: String,
    video_codec: String,
    audio_codec: String,
    occurrences: u64,
    last_observed_at_epoch_seconds: u64,
}

const fn default_submit_to_daemon() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContainerExecution {
    docker_executable: PathBuf,
    compose_file: PathBuf,
    managed_scratch_root: PathBuf,
    container_scratch_root: PathBuf,
    remote_config: PathBuf,
    aws_credentials: PathBuf,
    service: String,
    daemon_socket: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[serde(rename = "committed")]
struct Committed {
    schema_version: &'static str,
    catalogue_id: String,
    title: String,
    content_type: String,
    content_length: u64,
    endpoint_id: String,
    object_store_id: String,
    object_key: String,
    object_version: u64,
    checksum_sha256: String,
    verified_at_epoch_seconds: u64,
}

#[derive(Debug, Serialize)]
struct Failed {
    outcome: &'static str,
    schema_version: &'static str,
}

pub(crate) fn run_protocol() -> Result<(), Box<dyn std::error::Error>> {
    match run() {
        Ok(()) => Ok(()),
        Err(error) => {
            let outcome = protocol_failure_outcome(error.as_ref());
            serde_json::to_writer(
                io::stderr().lock(),
                &Failed {
                    outcome,
                    schema_version: REQUEST_SCHEMA,
                },
            )?;
            eprintln!();
            Ok(())
        }
    }
}

fn protocol_failure_outcome(error: &(dyn std::error::Error + 'static)) -> &'static str {
    error
        .downcast_ref::<io::Error>()
        .map_or("rejected", |error| match error.kind() {
            io::ErrorKind::PermissionDenied => "policy_blocked",
            io::ErrorKind::WouldBlock
            | io::ErrorKind::TimedOut
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::NotConnected => "unavailable",
            _ => "rejected",
        })
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = std::env::var_os("PINAKOTHEKE_DAS_HELPER_CONFIG")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".x-img/config/das-capture-helper.json"))
        })
        .ok_or("PINAKOTHEKE_DAS_HELPER_CONFIG or HOME is required")?;
    let request: Request = serde_json::from_reader(io::stdin().lock())?;
    let config = load_config(&config_path)?;
    let receipt = acquire(&request, &config)?;
    serde_json::to_writer(io::stderr().lock(), &receipt)?;
    eprintln!();
    Ok(())
}

fn load_config(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    require_private_regular(path)?;
    let bytes = fs::read(path)?;
    if bytes.len() > 16 * 1024 {
        return Err("DAS capture helper config exceeds 16 KiB".into());
    }
    let config: Config = serde_json::from_slice(&bytes)?;
    if config.schema_version != CONFIG_SCHEMA
        || config.endpoint_id.is_empty()
        || config.endpoint_id.len() > 128
        || !config
            .endpoint_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        || config.max_image_bytes.unwrap_or(DEFAULT_MAX_BYTES) == 0
        || config.max_image_bytes.unwrap_or(DEFAULT_MAX_BYTES) > 1024 * 1024 * 1024
        || config.max_video_bytes.unwrap_or(DEFAULT_MAX_VIDEO_BYTES) == 0
        || config.max_video_bytes.unwrap_or(DEFAULT_MAX_VIDEO_BYTES) > 8 * 1024 * 1024 * 1024
        || !config.submit_to_daemon
        || config.object_store_bucket.as_ref().is_some_and(|bucket| {
            bucket.is_empty()
                || bucket.len() > 63
                || !bucket.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'.' | b'-')
                })
        })
    {
        return Err("DAS capture helper config is invalid".into());
    }
    require_executable(&config.curl_executable)?;
    if let Some(ffprobe) = &config.ffprobe_executable {
        require_executable(ffprobe)?;
    }
    if let Some(normalization) = &config.normalization {
        validate_normalization_handoff(normalization)?;
        if config.ffprobe_executable.is_none() {
            return Err("video normalization requires ffprobe".into());
        }
    }
    match &config.container_execution {
        Some(container) => {
            if config.dasobjectstore_remote_executable.is_some()
                || config.dasobjectstore_remote_config.is_some()
                || config.daemon_socket.is_some()
            {
                return Err("native and container DAS execution cannot be combined".into());
            }
            validate_container_execution(container)?;
            if config
                .normalization
                .as_ref()
                .is_some_and(|normalization| {
                    !normalization
                        .staging_root
                        .starts_with(&container.managed_scratch_root)
                })
            {
                return Err("normalization staging must use the DAS managed root".into());
            }
        }
        None => {
            require_executable(
                config
                    .dasobjectstore_remote_executable
                    .as_deref()
                    .ok_or("native DAS remote executable is required")?,
            )?;
            require_private_regular(
                config
                    .dasobjectstore_remote_config
                    .as_deref()
                    .ok_or("native DAS remote config is required")?,
            )?;
            if !config
                .daemon_socket
                .as_deref()
                .is_some_and(Path::is_absolute)
            {
                return Err("native DAS daemon socket must be absolute".into());
            }
        }
    }
    Ok(config)
}

fn validate_normalization_handoff(
    handoff: &NormalizationHandoff,
) -> Result<(), Box<dyn std::error::Error>> {
    require_executable(&handoff.docker_executable)?;
    require_executable(&handoff.ingest_helper)?;
    let identifier = |value: &str| {
        !value.is_empty()
            && value.len() <= 128
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
            })
    };
    if !identifier(&handoff.executor_ref)
        || !identifier(&handoff.staging_ref)
        || !identifier(&handoff.firefox_playback_evidence_id)
        || !handoff.image_reference.starts_with("registry://")
        || handoff
            .image_reference
            .contains(['@', '?', '#', ' ', '\n', '\r'])
        || handoff.image_digest.len() != 71
        || !handoff.image_digest.starts_with("sha256:")
        || !handoff.image_digest[7..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || handoff.cpu_millis_limit == 0
        || handoff.memory_bytes_limit == 0
        || handoff.scratch_bytes_limit == 0
        || !handoff.staging_root.is_absolute()
        || !handoff.codec_gap_journal.is_absolute()
    {
        return Err("normalization handoff configuration is invalid".into());
    }
    let staging = fs::canonicalize(&handoff.staging_root)?;
    let staging_metadata = fs::symlink_metadata(&staging)?;
    if staging != handoff.staging_root
        || !staging_metadata.is_dir()
        || staging_metadata.file_type().is_symlink()
    {
        return Err("normalization staging root is not DAS-managed".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if staging_metadata.permissions().mode() & 0o007 != 0 {
            return Err("normalization staging root is not private".into());
        }
    }
    if let Some(parent) = handoff.codec_gap_journal.parent() {
        let metadata = fs::symlink_metadata(parent)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("codec-gap journal parent is not trusted".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err("codec-gap journal parent must be private".into());
            }
        }
    }
    if handoff.codec_gap_journal.exists() {
        require_private_regular(&handoff.codec_gap_journal)?;
    }
    Ok(())
}

fn validate_container_execution(
    container: &ContainerExecution,
) -> Result<(), Box<dyn std::error::Error>> {
    require_executable(&container.docker_executable)?;
    require_private_regular(&container.compose_file)?;
    require_private_regular(&container.remote_config)?;
    require_private_regular(&container.aws_credentials)?;
    if container.service != "dasobjectstored"
        || !container.managed_scratch_root.is_absolute()
        || !container.container_scratch_root.is_absolute()
        || !container.daemon_socket.is_absolute()
        || container.managed_scratch_root == Path::new("/")
        || container.container_scratch_root == Path::new("/")
    {
        return Err("container execution boundary is invalid".into());
    }
    let root = fs::canonicalize(&container.managed_scratch_root)?;
    if root != container.managed_scratch_root
        || fs::symlink_metadata(&root)?.file_type().is_symlink()
    {
        return Err("managed scratch root must be a canonical directory".into());
    }
    Ok(())
}

fn acquire(request: &Request, config: &Config) -> Result<Committed, Box<dyn std::error::Error>> {
    validate_request(request, config)?;
    let video = request.capture_kind == "explicit_video";
    let scratch = if video {
        match &config.normalization {
            Some(normalization) => Scratch::create_in(&normalization.staging_root)?,
            None => match &config.container_execution {
                Some(container) => Scratch::create_in(&container.managed_scratch_root)?,
                None => Scratch::create()?,
            },
        }
    } else {
        match &config.container_execution {
            Some(container) => Scratch::create_in(&container.managed_scratch_root)?,
            None => Scratch::create()?,
        }
    };
    let payload = scratch.path.join("payload");
    let max_bytes = if video {
        config.max_video_bytes.unwrap_or(DEFAULT_MAX_VIDEO_BYTES)
    } else {
        config.max_image_bytes.unwrap_or(DEFAULT_MAX_BYTES)
    };
    let max_bytes_text = max_bytes.to_string();
    let mut curl = Command::new(&config.curl_executable);
    let retrieval_media_url = if request.retrieval_media_url.is_empty() {
        &request.canonical_media_url
    } else {
        &request.retrieval_media_url
    };
    curl.args([
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--max-redirs",
        "5",
        "--connect-timeout",
        "15",
        "--max-time",
        "300",
        "--max-filesize",
        &max_bytes_text,
        "--output",
    ])
    .arg(&payload)
    .args(["--write-out", "%{content_type}"])
    .arg(retrieval_media_url)
    .stderr(Stdio::null());
    let (curl_status, curl_stdout) = output_bounded(&mut curl, 128)?;
    if !curl_status.success() {
        return Err("HTTPS media retrieval failed".into());
    }
    let content_type = String::from_utf8(curl_stdout)?.trim().to_ascii_lowercase();
    if (!video && !content_type.starts_with("image/"))
        || (video && !content_type.starts_with("video/"))
        || content_type.len() > 128
    {
        return Err("retrieved object is not an eligible bounded medium".into());
    }
    let metadata = fs::symlink_metadata(&payload)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > max_bytes
    {
        return Err("retrieved media payload is invalid".into());
    }
    #[cfg(unix)]
    if config.container_execution.is_none() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        fs::set_permissions(&payload, fs::Permissions::from_mode(0o640))?;
        let daemon_group = fs::symlink_metadata(&scratch.path)?.gid();
        std::os::unix::fs::chown(&payload, None, Some(daemon_group))?;
    }
    if video {
        let probe = probe_video(
            config
                .ffprobe_executable
                .as_deref()
                .ok_or("video capture requires ffprobe")?,
            &payload,
        )?;
        if !firefox_compatible(&content_type, &probe) {
            return normalize_incompatible_video(request, config, scratch, payload, probe);
        }
    }
    let checksum = sha256_file(&payload)?;
    let object_key = object_key(request, &checksum);
    let version = u64::from_str_radix(&checksum[..16], 16).unwrap_or(1).max(1);
    let mut upload = upload_command(
        config,
        &scratch,
        &payload,
        request,
        &object_key,
        &content_type,
    )?;
    let (upload_status, upload_stdout) = output_bounded(&mut upload, 64 * 1024)?;
    if !upload_status.success() {
        return Err("DASObjectStore remote upload failed".into());
    }
    let report = String::from_utf8(upload_stdout)?;
    let daemon_verified = report.lines().any(|line| {
        line.starts_with("Final:")
            && line.contains("state=Complete")
            && line.contains("stage=remote_s3_transfer_complete")
    });
    if !daemon_verified {
        return Err("DASObjectStore did not report verified completion".into());
    }
    Ok(Committed {
        schema_version: REQUEST_SCHEMA,
        catalogue_id: catalogue_id(request),
        title: format!(
            "Captured {} from {}",
            if video { "video" } else { "image" },
            request.site_id
        ),
        content_type,
        content_length: metadata.len(),
        endpoint_id: request.endpoint_id.clone(),
        object_store_id: request.object_store_id.clone(),
        object_key,
        object_version: version,
        checksum_sha256: checksum,
        verified_at_epoch_seconds: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    })
}

fn upload_command(
    config: &Config,
    scratch: &Scratch,
    payload: &Path,
    request: &Request,
    object_key: &str,
    content_type: &str,
) -> Result<Command, Box<dyn std::error::Error>> {
    if let Some(container) = &config.container_execution {
        let remote_config = scratch.path.join("remote.json");
        let credentials = scratch.path.join("aws-credentials");
        copy_private(&container.remote_config, &remote_config)?;
        copy_private(&container.aws_credentials, &credentials)?;
        let container_job = translated_container_path(container, &scratch.path)?;
        let container_payload = container_job.join("payload");
        let container_config = container_job.join("remote.json");
        let container_credentials = container_job.join("aws-credentials");
        let mut command = Command::new(&container.docker_executable);
        command
            .args(["compose", "-f"])
            .arg(&container.compose_file)
            .args(["exec", "-T", "-e"])
            .arg(format!(
                "AWS_SHARED_CREDENTIALS_FILE={}",
                container_credentials.display()
            ))
            .arg(&container.service)
            .arg("dasobjectstore-remote")
            .arg("--config")
            .arg(container_config)
            .arg("upload")
            // Daemon submission is an authority boundary: pass the reviewed
            // logical ObjectStore identifier and let DASObjectStore resolve
            // its current bucket binding.
            .arg(&request.object_store_id)
            .args(
                config
                    .object_store_bucket
                    .as_deref()
                    .map(|bucket| ["--bucket", bucket])
                    .into_iter()
                    .flatten(),
            )
            .arg("--source")
            .arg(container_payload)
            .arg("--key")
            .arg(object_key)
            .arg("--content-type")
            .arg(content_type)
            .args(["--no-progress", "--submit-to-daemon", "--daemon-socket"])
            .arg(&container.daemon_socket)
            .stderr(Stdio::null());
        return Ok(command);
    }
    let mut command = Command::new(
        config
            .dasobjectstore_remote_executable
            .as_deref()
            .ok_or("native DAS remote executable is required")?,
    );
    command
        .arg("--config")
        .arg(
            config
                .dasobjectstore_remote_config
                .as_deref()
                .ok_or("native DAS remote config is required")?,
        )
        .arg("upload")
        .arg(&request.object_store_id)
        .args(
            config
                .object_store_bucket
                .as_deref()
                .map(|bucket| ["--bucket", bucket])
                .into_iter()
                .flatten(),
        )
        .arg("--source")
        .arg(payload)
        .arg("--key")
        .arg(object_key)
        .arg("--content-type")
        .arg(content_type)
        .arg("--no-progress");
    command.args(["--submit-to-daemon", "--daemon-socket"]).arg(
        config
            .daemon_socket
            .as_deref()
            .ok_or("native DAS daemon socket is required")?,
    );
    command.stderr(Stdio::null());
    Ok(command)
}

fn translated_container_path(
    container: &ContainerExecution,
    host_path: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let relative = host_path
        .strip_prefix(&container.managed_scratch_root)
        .map_err(|_| "scratch path escaped the managed root")?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err("scratch path is not a direct managed descendant".into());
    }
    Ok(container.container_scratch_root.join(relative))
}

fn copy_private(source: &Path, destination: &Path) -> io::Result<()> {
    require_private_regular(source)?;
    fs::copy(source, destination)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(destination, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn catalogue_id(request: &Request) -> String {
    let identity = format!(
        "{}\n{}",
        request.canonical_page_url, request.canonical_media_url
    );
    let digest = format!("{:x}", Sha256::digest(identity.as_bytes()));
    format!("website-{}-{}", request.site_id, &digest[..24])
}

fn object_key(request: &Request, checksum: &str) -> String {
    if request.origin == "https://x.com" {
        let account = x_account(&request.canonical_presentation_url)
            .or_else(|| x_account(&request.canonical_page_url))
            .unwrap_or_else(|| "_unattributed".into());
        return format!(
            "x.com/{}/{}/{checksum}",
            account.to_ascii_lowercase(),
            request.capture_kind
        );
    }
    format!(
        "sites/{}/{}/{checksum}",
        request.site_id, request.capture_kind
    )
}

fn x_account(url: &str) -> Option<String> {
    let uri = url.parse::<axum::http::Uri>().ok()?;
    if uri.scheme_str() != Some("https") || uri.authority()?.host() != "x.com" {
        return None;
    }
    let account = uri.path().split('/').find(|segment| !segment.is_empty())?;
    const RESERVED: &[&str] = &[
        "compose",
        "explore",
        "home",
        "i",
        "intent",
        "messages",
        "notifications",
        "search",
        "settings",
    ];
    if RESERVED.contains(&account)
        || account.len() > 15
        || !account
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return None;
    }
    Some(account.into())
}

fn validate_request(request: &Request, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    if request.schema_version != REQUEST_SCHEMA
        || request.endpoint_id != config.endpoint_id
        || request.plan_id.is_empty()
        || request.actor_ref.is_empty()
        || request.actor_ref.len() > 128
        || !request
            .actor_ref
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
        || request.site_id.is_empty()
        || request.site_id.len() > 64
        || !request
            .site_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        || request.object_store_id.is_empty()
        || request.object_store_id.len() > 128
        || !request
            .object_store_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        || request.width == 0
        || request.height == 0
        || request.adapter_version.is_empty()
        || !matches!(
            request.capture_kind.as_str(),
            "observed_thumbnail" | "explicit_original" | "explicit_video"
        )
    {
        return Err("capture request is invalid or changed destination".into());
    }
    let origin = request.origin.parse::<axum::http::Uri>()?;
    let page = request.canonical_page_url.parse::<axum::http::Uri>()?;
    let media = request.canonical_media_url.parse::<axum::http::Uri>()?;
    let retrieval_media_url = if request.retrieval_media_url.is_empty() {
        &request.canonical_media_url
    } else {
        &request.retrieval_media_url
    };
    let retrieval = retrieval_media_url.parse::<axum::http::Uri>()?;
    let presentation = request
        .canonical_presentation_url
        .parse::<axum::http::Uri>()?;
    if origin.scheme_str() != Some("https")
        || page.scheme_str() != Some("https")
        || media.scheme_str() != Some("https")
        || retrieval.scheme_str() != Some("https")
        || presentation.scheme_str() != Some("https")
        || page.authority() != origin.authority()
        || origin
            .authority()
            .is_some_and(|value| value.as_str().contains('@'))
        || page
            .authority()
            .is_some_and(|value| value.as_str().contains('@'))
        || media
            .authority()
            .is_some_and(|value| value.as_str().contains('@'))
        || retrieval
            .authority()
            .is_some_and(|value| value.as_str().contains('@'))
        || presentation
            .authority()
            .is_some_and(|value| value.as_str().contains('@'))
    {
        return Err("capture request URLs are not eligible HTTPS provenance".into());
    }
    Ok(())
}

fn probe_video(ffprobe: &Path, payload: &Path) -> Result<VideoProbe, Box<dyn std::error::Error>> {
    let mut command = Command::new(ffprobe);
    command
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=format_name:stream=codec_type,codec_name",
            "-of",
            "json",
        ])
        .arg(payload)
        .stderr(Stdio::null());
    let (status, output) = output_bounded(&mut command, 4096)?;
    if !status.success() {
        return Err("video probe failed".into());
    }
    let value: serde_json::Value = serde_json::from_slice(&output)?;
    let streams = value
        .get("streams")
        .and_then(serde_json::Value::as_array)
        .ok_or("video probe omitted streams")?;
    let video_codec = streams
        .iter()
        .find(|stream| {
            stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("video")
        })
        .and_then(|stream| stream.get("codec_name"))
        .and_then(serde_json::Value::as_str)
        .filter(|codec| safe_codec(codec))
        .ok_or("video probe omitted a safe video codec")?;
    let audio_codec = streams
        .iter()
        .find(|stream| {
            stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("audio")
        })
        .and_then(|stream| stream.get("codec_name"))
        .and_then(serde_json::Value::as_str)
        .filter(|codec| safe_codec(codec));
    let container = value
        .get("format")
        .and_then(|format| format.get("format_name"))
        .and_then(serde_json::Value::as_str)
        .and_then(|names| names.split(',').find(|name| safe_codec(name)))
        .ok_or("video probe omitted a safe container")?;
    Ok(VideoProbe {
        container: container.into(),
        video_codec: video_codec.into(),
        audio_codec: audio_codec.map(str::to_owned),
    })
}

fn safe_codec(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn firefox_compatible(content_type: &str, probe: &VideoProbe) -> bool {
    content_type == "video/mp4"
        && matches!(probe.container.as_str(), "mov" | "mp4")
        && matches!(probe.video_codec.as_str(), "h264" | "avc1")
        && probe
            .audio_codec
            .as_deref()
            .is_none_or(|codec| codec == "aac")
}

fn normalize_incompatible_video(
    request: &Request,
    config: &Config,
    scratch: Scratch,
    payload: PathBuf,
    probe: VideoProbe,
) -> Result<Committed, Box<dyn std::error::Error>> {
    let handoff = config
        .normalization
        .as_ref()
        .ok_or("video requires configured container normalization")?;
    record_codec_gap(&handoff.codec_gap_journal, &probe)?;
    let source_checksum = sha256_file(&payload)?;
    let input = scratch.path.join("input.media");
    fs::rename(&payload, &input)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&scratch.path, fs::Permissions::from_mode(0o700))?;
        fs::set_permissions(&input, fs::Permissions::from_mode(0o600))?;
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let key_root = format!("{}/normalized", object_key(request, &source_checksum));
    let plan = PairedDeviceNormalizationPlan {
        schema_version: NORMALIZATION_SCHEMA,
        job_id: format!("normalize-{}", request.plan_id),
        source_identity: format!("source-{}", &source_checksum[..24]),
        profile_id: PINAKOTHEKE_VIDEO_MP4_V1.into(),
        output_variant: CodecVariant {
            video: VideoCodec::H264,
            audio: AudioCodec::Aac,
        },
        destination: ReviewedDestination {
            endpoint_id: request.endpoint_id.clone(),
            object_store_id: request.object_store_id.clone(),
            object_type: "video".into(),
            selection_kind: "site_override".into(),
            reviewed_at_unix_seconds: now,
            actor_ref: request.actor_ref.clone(),
        },
        executor: DockerExecutionPlan {
            placement: ExecutionPlacement::DasObjectStoreHost {
                executor_ref: handoff.executor_ref.clone(),
            },
            image_reference: handoff.image_reference.clone(),
            image_digest: handoff.image_digest.clone(),
            cpu_millis_limit: handoff.cpu_millis_limit,
            memory_bytes_limit: handoff.memory_bytes_limit,
            scratch_bytes_limit: handoff.scratch_bytes_limit,
        },
        scratch: ScratchAuthority::DasObjectStoreManaged {
            staging_ref: handoff.staging_ref.clone(),
        },
        scratch_root: scratch.path.clone(),
        input_file: input,
        normalized_object_key: format!("{key_root}/video.mp4"),
        poster_object_key: format!("{key_root}/poster.webp"),
        manifest_object_key: format!("{key_root}/manifest.json"),
    };
    let result = crate::video_normalize::normalize_with_process_ingest(
        &plan,
        handoff.docker_executable.clone(),
        handoff.ingest_helper.clone(),
    )?;
    if result.phase != x_img_core::video_normalization::NormalizationPhase::AwaitingFirefoxPlayback
        || handoff.firefox_playback_evidence_id.is_empty()
    {
        return Err("normalized video lacks reviewed Firefox playback evidence".into());
    }
    let checksum = result
        .normalized_video
        .checksum
        .strip_prefix("sha256:")
        .ok_or("normalizer returned an invalid checksum")?
        .to_owned();
    let version = u64::from_str_radix(&checksum[..16], 16).unwrap_or(1).max(1);
    Ok(Committed {
        schema_version: REQUEST_SCHEMA,
        catalogue_id: catalogue_id(request),
        title: format!("Normalized video from {}", request.site_id),
        content_type: "video/mp4".into(),
        content_length: result.normalized_video.size_bytes,
        endpoint_id: result.normalized_video.endpoint_id,
        object_store_id: result.normalized_video.object_store_id,
        object_key: result.normalized_video.object_key,
        object_version: version,
        checksum_sha256: checksum,
        verified_at_epoch_seconds: now,
    })
}

fn record_codec_gap(path: &Path, probe: &VideoProbe) -> Result<(), Box<dyn std::error::Error>> {
    let mut journal = if path.exists() {
        let bytes = fs::read(path)?;
        if bytes.len() > 64 * 1024 {
            return Err("codec-gap journal exceeds 64 KiB".into());
        }
        let journal: CodecGapJournal = serde_json::from_slice(&bytes)?;
        if journal.schema_version != "pinakotheke.codec-gap-journal.v1" {
            return Err("codec-gap journal schema is unsupported".into());
        }
        journal
    } else {
        CodecGapJournal {
            schema_version: "pinakotheke.codec-gap-journal.v1".into(),
            gaps: BTreeMap::new(),
        }
    };
    let audio = probe.audio_codec.as_deref().unwrap_or("none");
    let key = format!("{}:{}:{audio}", probe.container, probe.video_codec);
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let gap = journal.gaps.entry(key).or_insert_with(|| CodecGapRecord {
        container: probe.container.clone(),
        video_codec: probe.video_codec.clone(),
        audio_codec: audio.into(),
        occurrences: 0,
        last_observed_at_epoch_seconds: now,
    });
    gap.occurrences = gap.occurrences.saturating_add(1);
    gap.last_observed_at_epoch_seconds = now;
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(&journal)?;
    if bytes.len() > 64 * 1024 {
        return Err("codec-gap journal exceeds 64 KiB".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(&bytes)?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    Ok(())
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn output_bounded(command: &mut Command, limit: u64) -> io::Result<(ExitStatus, Vec<u8>)> {
    let mut child = command.stdout(Stdio::piped()).spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("child stdout unavailable"))?;
    let mut bytes = Vec::new();
    stdout.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        let _ = child.kill();
        let _ = child.wait();
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "child output exceeded the bounded protocol",
        ));
    }
    Ok((child.wait()?, bytes))
}

fn require_private_regular(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path must be absolute",
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path must be a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "file must be mode 0600 or stricter",
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
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "executable must be a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "executable is not executable",
            ));
        }
    }
    Ok(())
}

struct Scratch {
    path: PathBuf,
}
impl Scratch {
    fn create() -> io::Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pinakotheke-das-capture-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Native daemon submission may run under the shared
            // dasobjectstore group. A setgid TMPDIR supplies that group while
            // this mode keeps the bounded payload unavailable to others.
            fs::set_permissions(&path, fs::Permissions::from_mode(0o750))?;
        }
        Ok(Self { path })
    }

    fn create_in(root: &Path) -> io::Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let path = root.join(format!(
            ".pinakotheke-capture-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
        }
        Ok(Self { path })
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn protocol_failures_are_bounded_categories_without_error_text() {
        let permission = io::Error::new(io::ErrorKind::PermissionDenied, "secret path");
        let unavailable = io::Error::new(io::ErrorKind::TimedOut, "private URL");
        let invalid = io::Error::new(io::ErrorKind::InvalidData, "signed query");
        assert_eq!(protocol_failure_outcome(&permission), "policy_blocked");
        assert_eq!(protocol_failure_outcome(&unavailable), "unavailable");
        assert_eq!(protocol_failure_outcome(&invalid), "rejected");
    }

    #[test]
    fn video_probe_accepts_only_firefox_h264_aac_profile() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-video-probe-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let probe = root.join("ffprobe");
        let payload = root.join("video.mp4");
        fs::write(&payload, b"synthetic").unwrap();
        executable(
            &probe,
            "#!/bin/sh\nprintf '%s' '{\"streams\":[{\"codec_type\":\"video\",\"codec_name\":\"h264\"},{\"codec_type\":\"audio\",\"codec_name\":\"aac\"}],\"format\":{\"format_name\":\"mov,mp4\"}}'\n",
        );
        let compatible = probe_video(&probe, &payload).unwrap();
        assert!(firefox_compatible("video/mp4", &compatible));
        executable(
            &probe,
            "#!/bin/sh\nprintf '%s' '{\"streams\":[{\"codec_type\":\"video\",\"codec_name\":\"vp9\"},{\"codec_type\":\"audio\",\"codec_name\":\"opus\"}],\"format\":{\"format_name\":\"matroska,webm\"}}'\n",
        );
        let incompatible = probe_video(&probe, &payload).unwrap();
        assert!(!firefox_compatible("video/webm", &incompatible));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn incompatible_video_is_normalized_committed_and_redacted() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-auto-normalize-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let curl = root.join("curl");
        executable(
            &curl,
            "#!/bin/sh\nout=''\nwhile [ $# -gt 0 ]; do [ \"$1\" = --output ] && { shift; out=$1; }; shift; done\nprintf source-video > \"$out\"\nprintf video/webm\n",
        );
        let ffprobe = root.join("ffprobe");
        executable(
            &ffprobe,
            "#!/bin/sh\nprintf '%s' '{\"streams\":[{\"codec_type\":\"video\",\"codec_name\":\"vp9\"},{\"codec_type\":\"audio\",\"codec_name\":\"opus\"}],\"format\":{\"format_name\":\"matroska,webm\"}}'\n",
        );
        let docker = root.join("docker-fixture.py");
        executable(
            &docker,
            r#"#!/usr/bin/env python3
import json,pathlib,sys
mount=next(a for a in sys.argv if a.startswith('type=bind,src='))
scratch=pathlib.Path(mount.split(',src=',1)[1].split(',dst=',1)[0])
if '--entrypoint' in sys.argv and 'ffprobe' in sys.argv:
 print(json.dumps({'streams':[{'codec_type':'video','codec_name':'h264','width':64,'height':48},{'codec_type':'audio','codec_name':'aac'}],'format':{'duration':'1.0'}}),end='')
elif any('poster.webp' in a for a in sys.argv):
 (scratch/'poster.webp').write_bytes(b'poster')
else:
 (scratch/'normalized.mp4').write_bytes(b'normalized-video')
"#,
        );
        let ingest = root.join("ingest-fixture.py");
        executable(
            &ingest,
            r#"#!/usr/bin/env python3
import hashlib,json,sys
h=json.loads(sys.stdin.buffer.readline())
p=sys.stdin.buffer.read()
assert len(p)==h['expected_size_bytes']
assert 'sha256:'+hashlib.sha256(p).hexdigest()==h['expected_checksum']
print(json.dumps({'schema_version':h['schema_version'],'endpoint_id':h['endpoint_id'],'object_store_id':h['object_store_id'],'object_key':h['object_key'],'size_bytes':len(p),'checksum':h['expected_checksum'],'object_reference':'dasobjectstore:'+h['endpoint_id']+':'+h['object_store_id']+':'+h['object_key']}))
"#,
        );
        let gap_journal = root.join("codec-gaps.json");
        let staging = root.join("video-staging");
        fs::create_dir(&staging).unwrap();
        fs::set_permissions(&staging, fs::Permissions::from_mode(0o700)).unwrap();
        let config = Config {
            schema_version: CONFIG_SCHEMA.into(),
            endpoint_id: "endpoint-1".into(),
            object_store_bucket: None,
            curl_executable: curl,
            ffprobe_executable: Some(ffprobe),
            dasobjectstore_remote_executable: Some(root.join("unused-remote")),
            dasobjectstore_remote_config: Some(root.join("unused-config")),
            daemon_socket: Some(root.join("unused.sock")),
            submit_to_daemon: true,
            container_execution: None,
            max_image_bytes: Some(1024),
            max_video_bytes: Some(1024),
            normalization: Some(NormalizationHandoff {
                docker_executable: docker,
                ingest_helper: ingest,
                executor_ref: "dasobjectstore-worker-1".into(),
                staging_ref: "dasobjectstore-staging-1".into(),
                staging_root: staging.clone(),
                image_reference: "registry://fixture/ffmpeg".into(),
                image_digest: format!("sha256:{}", "a".repeat(64)),
                cpu_millis_limit: 1_000,
                memory_bytes_limit: 64 * 1024 * 1024,
                scratch_bytes_limit: 1024 * 1024,
                firefox_playback_evidence_id: "firefox-profile-evidence-1".into(),
                codec_gap_journal: gap_journal.clone(),
            }),
        };
        let request = Request {
            schema_version: REQUEST_SCHEMA.into(),
            plan_id: "video-plan-1".into(),
            actor_ref: "actor-1".into(),
            site_id: "enabled-site".into(),
            origin: "https://example.invalid".into(),
            canonical_page_url: "https://example.invalid/watch/1".into(),
            canonical_media_url: "https://media.invalid/video.webm".into(),
            retrieval_media_url: "https://media.invalid/video.webm".into(),
            canonical_presentation_url: "https://example.invalid/watch/1".into(),
            capture_kind: "explicit_video".into(),
            width: 64,
            height: 48,
            adapter_version: "1.0.0".into(),
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
        };
        let receipt = acquire(&request, &config).unwrap();
        assert_eq!(receipt.content_type, "video/mp4");
        assert_eq!(receipt.content_length, 16);
        assert!(receipt.object_key.ends_with("/normalized/video.mp4"));
        let journal = fs::read_to_string(gap_journal).unwrap();
        assert!(journal.contains("vp9"));
        assert!(journal.contains("opus"));
        assert!(!journal.contains("example.invalid"));
        assert!(!journal.contains("video.webm"));
        assert_eq!(fs::read_dir(staging).unwrap().count(), 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn transfer_only_configuration_is_rejected() {
        let path = std::env::temp_dir().join(format!(
            "pinakotheke-transfer-only-config-{}",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"{"schema_version":"pinakotheke.das-capture-helper.v1","endpoint_id":"endpoint-1","curl_executable":"/does/not/run","dasobjectstore_remote_executable":"/does/not/run","dasobjectstore_remote_config":"/does/not/read","daemon_socket":"/does/not/connect","submit_to_daemon":false}"#,
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(load_config(&path).is_err());
        fs::remove_file(path).unwrap();
    }

    fn executable(path: &Path, body: &str) {
        fs::write(path, body).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }

    #[test]
    fn commits_bounded_image_through_verified_daemon_upload_and_cleans_scratch() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-das-helper-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let curl = root.join("curl");
        let remote = root.join("remote");
        executable(
            &curl,
            "#!/bin/sh\nout=''\nwhile [ $# -gt 0 ]; do [ \"$1\" = --output ] && { shift; out=$1; }; shift; done\nprintf fixture > \"$out\"\nprintf image/png\n",
        );
        executable(
            &remote,
            "#!/bin/sh\nprintf '%s' \"$*\" | grep -q -- '--config .* upload store-1 --bucket dos-store-1 --source .* --key sites/site-1/observed_thumbnail/.* --content-type image/png --no-progress --submit-to-daemon --daemon-socket' || exit 9\nprintf 'Daemon remote upload job submitted\\nFinal: job state=Complete stage=remote_s3_transfer_complete\\n'\n",
        );
        let remote_config = root.join("remote.json");
        fs::write(&remote_config, "{}").unwrap();
        fs::set_permissions(&remote_config, fs::Permissions::from_mode(0o600)).unwrap();
        let config = Config {
            schema_version: CONFIG_SCHEMA.into(),
            endpoint_id: "endpoint-1".into(),
            object_store_bucket: Some("dos-store-1".into()),
            curl_executable: curl,
            ffprobe_executable: None,
            dasobjectstore_remote_executable: Some(remote),
            dasobjectstore_remote_config: Some(remote_config),
            daemon_socket: Some(root.join("daemon.sock")),
            submit_to_daemon: true,
            container_execution: None,
            max_image_bytes: Some(1024),
            max_video_bytes: None,
            normalization: None,
        };
        let mut request = Request {
            schema_version: REQUEST_SCHEMA.into(),
            plan_id: "plan-1".into(),
            actor_ref: "actor-1".into(),
            site_id: "site-1".into(),
            origin: "https://example.invalid".into(),
            canonical_page_url: "https://example.invalid/gallery".into(),
            canonical_media_url: "https://media.invalid/image.png".into(),
            retrieval_media_url: "https://media.invalid/image.png".into(),
            canonical_presentation_url: "https://example.invalid/artists/example/status/1".into(),
            capture_kind: "observed_thumbnail".into(),
            width: 10,
            height: 10,
            adapter_version: "1.0.0".into(),
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
        };
        let before: Vec<_> = fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("pinakotheke-das-capture-")
            })
            .collect();
        let receipt = acquire(&request, &config).unwrap();
        assert_eq!(receipt.content_length, 7);
        assert_eq!(receipt.content_type, "image/png");
        assert_eq!(receipt.object_store_id, "store-1");
        assert!(
            receipt
                .object_key
                .starts_with("sites/site-1/observed_thumbnail/")
        );
        assert!(receipt.object_version > 0);
        assert_eq!(receipt.catalogue_id, catalogue_id(&request));
        request.canonical_media_url = "https://media.invalid/second.png".into();
        assert_ne!(receipt.catalogue_id, catalogue_id(&request));
        let after: Vec<_> = fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("pinakotheke-das-capture-")
            })
            .collect();
        assert_eq!(before.len(), after.len());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_a_request_that_changes_the_pinned_endpoint_before_retrieval() {
        let request = Request {
            schema_version: REQUEST_SCHEMA.into(),
            plan_id: "plan-1".into(),
            actor_ref: "actor-1".into(),
            site_id: "site-1".into(),
            origin: "https://example.invalid".into(),
            canonical_page_url: "https://example.invalid/gallery".into(),
            canonical_media_url: "https://media.invalid/image.png".into(),
            retrieval_media_url: "https://media.invalid/image.png".into(),
            canonical_presentation_url: "https://example.invalid/artists/example/status/1".into(),
            capture_kind: "explicit_original".into(),
            width: 10,
            height: 10,
            adapter_version: "1.0.0".into(),
            endpoint_id: "changed-endpoint".into(),
            object_store_id: "store-1".into(),
        };
        let config = Config {
            schema_version: CONFIG_SCHEMA.into(),
            endpoint_id: "endpoint-1".into(),
            object_store_bucket: None,
            curl_executable: PathBuf::from("/does/not/run"),
            ffprobe_executable: None,
            dasobjectstore_remote_executable: Some(PathBuf::from("/does/not/run")),
            dasobjectstore_remote_config: Some(PathBuf::from("/does/not/read")),
            daemon_socket: Some(PathBuf::from("/does/not/connect")),
            submit_to_daemon: true,
            container_execution: None,
            max_image_bytes: Some(1024),
            max_video_bytes: None,
            normalization: None,
        };
        assert!(validate_request(&request, &config).is_err());
    }

    #[test]
    fn accepts_explicit_video_from_an_opted_in_https_site() {
        let request = Request {
            schema_version: REQUEST_SCHEMA.into(),
            plan_id: "plan-video-1".into(),
            actor_ref: "actor-1".into(),
            site_id: "example-video".into(),
            origin: "https://media.example.invalid".into(),
            canonical_page_url: "https://media.example.invalid/watch/1".into(),
            canonical_media_url: "https://cdn.example.invalid/media/1.mp4".into(),
            retrieval_media_url: "https://cdn.example.invalid/media/1.mp4?token=test".into(),
            canonical_presentation_url: "https://media.example.invalid/watch/1".into(),
            capture_kind: "explicit_video".into(),
            width: 1280,
            height: 720,
            adapter_version: "1.0.0".into(),
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
        };
        let config = Config {
            schema_version: CONFIG_SCHEMA.into(),
            endpoint_id: "endpoint-1".into(),
            object_store_bucket: None,
            curl_executable: PathBuf::from("/does/not/run"),
            ffprobe_executable: Some(PathBuf::from("/does/not/run")),
            dasobjectstore_remote_executable: Some(PathBuf::from("/does/not/run")),
            dasobjectstore_remote_config: Some(PathBuf::from("/does/not/read")),
            daemon_socket: Some(PathBuf::from("/does/not/connect")),
            submit_to_daemon: true,
            container_execution: None,
            max_image_bytes: Some(1024),
            max_video_bytes: Some(1024),
            normalization: None,
        };
        assert!(validate_request(&request, &config).is_ok());
    }

    #[test]
    fn committed_receipt_uses_the_lowercase_protocol_discriminator() {
        let receipt = Committed {
            schema_version: REQUEST_SCHEMA,
            catalogue_id: "card-1".into(),
            title: "Synthetic".into(),
            content_type: "image/png".into(),
            content_length: 7,
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
            object_key: "media/checksum".into(),
            object_version: 1,
            checksum_sha256: "a".repeat(64),
            verified_at_epoch_seconds: 1,
        };
        let encoded = serde_json::to_value(receipt).unwrap();
        assert_eq!(encoded["outcome"], "committed");
    }

    #[test]
    fn x_object_keys_use_the_post_author_and_capture_class() {
        let request = Request {
            schema_version: REQUEST_SCHEMA.into(),
            plan_id: "plan-1".into(),
            actor_ref: "actor-1".into(),
            site_id: "x-com".into(),
            origin: "https://x.com".into(),
            canonical_page_url: "https://x.com/home".into(),
            canonical_media_url: "https://pbs.twimg.com/media/image".into(),
            retrieval_media_url: "https://pbs.twimg.com/media/image".into(),
            canonical_presentation_url: "https://x.com/Example_Artist/status/42".into(),
            capture_kind: "observed_thumbnail".into(),
            width: 640,
            height: 480,
            adapter_version: "1.0.0".into(),
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
        };
        assert_eq!(
            object_key(&request, "abc123"),
            "x.com/example_artist/observed_thumbnail/abc123"
        );
    }

    #[test]
    fn container_execution_translates_only_managed_scratch_and_cleans_credentials() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-container-helper-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let curl = root.join("curl");
        let docker = root.join("docker");
        let arguments = root.join("arguments");
        executable(
            &curl,
            "#!/bin/sh\nout=''\nwhile [ $# -gt 0 ]; do [ \"$1\" = --output ] && { shift; out=$1; }; shift; done\nprintf fixture > \"$out\"\nprintf image/png\n",
        );
        executable(
            &docker,
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" > '{}'\nprintf 'Final: job state=Complete stage=remote_s3_transfer_complete\\n'\n",
                arguments.display()
            ),
        );
        let compose = root.join("compose.yml");
        let remote_config = root.join("remote.json");
        let credentials = root.join("credentials");
        fs::write(&compose, "services: {}\n").unwrap();
        fs::write(&remote_config, "{}\n").unwrap();
        fs::write(&credentials, "[dasobjectstore]\nsecret_access_key=test\n").unwrap();
        for file in [&compose, &remote_config, &credentials] {
            fs::set_permissions(file, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let managed = root.join("managed");
        fs::create_dir(&managed).unwrap();
        let config = Config {
            schema_version: CONFIG_SCHEMA.into(),
            endpoint_id: "endpoint-1".into(),
            object_store_bucket: None,
            curl_executable: curl,
            ffprobe_executable: None,
            dasobjectstore_remote_executable: None,
            dasobjectstore_remote_config: None,
            daemon_socket: None,
            submit_to_daemon: true,
            container_execution: Some(ContainerExecution {
                docker_executable: docker,
                compose_file: compose,
                managed_scratch_root: managed.clone(),
                container_scratch_root: PathBuf::from("/Volumes/Seagate/DASObjectStore"),
                remote_config,
                aws_credentials: credentials,
                service: "dasobjectstored".into(),
                daemon_socket: PathBuf::from("/run/dasobjectstore/dasobjectstored.sock"),
            }),
            max_image_bytes: Some(1024),
            max_video_bytes: None,
            normalization: None,
        };
        let request = Request {
            schema_version: REQUEST_SCHEMA.into(),
            plan_id: "plan-1".into(),
            actor_ref: "actor-1".into(),
            site_id: "site-1".into(),
            origin: "https://example.invalid".into(),
            canonical_page_url: "https://example.invalid/gallery".into(),
            canonical_media_url: "https://media.invalid/image.png".into(),
            retrieval_media_url: "https://media.invalid/image.png".into(),
            canonical_presentation_url: "https://example.invalid/artists/example/status/1".into(),
            capture_kind: "explicit_original".into(),
            width: 10,
            height: 10,
            adapter_version: "1.0.0".into(),
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
        };
        let receipt = acquire(&request, &config).unwrap();
        assert_eq!(receipt.content_length, 7);
        let arguments = fs::read_to_string(arguments).unwrap();
        assert!(arguments.contains("compose -f"));
        assert!(
            arguments.contains(
                "exec -T -e AWS_SHARED_CREDENTIALS_FILE=/Volumes/Seagate/DASObjectStore/"
            )
        );
        assert!(arguments.contains("dasobjectstored dasobjectstore-remote --config"));
        assert!(arguments.contains("--content-type image/png"));
        assert!(arguments.contains("--daemon-socket /run/dasobjectstore/dasobjectstored.sock"));
        assert_eq!(fs::read_dir(&managed).unwrap().count(), 0);
        let _ = fs::remove_dir_all(root);
    }
}
