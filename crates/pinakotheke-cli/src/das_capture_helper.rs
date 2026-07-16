// SPDX-License-Identifier: MPL-2.0
//! First-party bounded capture adapter for DASObjectStore's remote completion client.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const REQUEST_SCHEMA: &str = "pinakotheke.capture-acquire-helper.v1";
const CONFIG_SCHEMA: &str = "pinakotheke.das-capture-helper.v1";
const DEFAULT_MAX_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: String,
    plan_id: String,
    site_id: String,
    origin: String,
    canonical_page_url: String,
    canonical_media_url: String,
    capture_kind: String,
    width: u32,
    height: u32,
    adapter_version: String,
    endpoint_id: String,
    object_store_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: String,
    endpoint_id: String,
    curl_executable: PathBuf,
    #[serde(default)]
    dasobjectstore_remote_executable: Option<PathBuf>,
    #[serde(default)]
    dasobjectstore_remote_config: Option<PathBuf>,
    #[serde(default)]
    daemon_socket: Option<PathBuf>,
    #[serde(default)]
    container_execution: Option<ContainerExecution>,
    max_image_bytes: Option<u64>,
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

pub(crate) fn run() -> Result<(), Box<dyn std::error::Error>> {
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
    {
        return Err("DAS capture helper config is invalid".into());
    }
    require_executable(&config.curl_executable)?;
    match &config.container_execution {
        Some(container) => {
            if config.dasobjectstore_remote_executable.is_some()
                || config.dasobjectstore_remote_config.is_some()
                || config.daemon_socket.is_some()
            {
                return Err("native and container DAS execution cannot be combined".into());
            }
            validate_container_execution(container)?;
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
    let scratch = match &config.container_execution {
        Some(container) => Scratch::create_in(&container.managed_scratch_root)?,
        None => Scratch::create()?,
    };
    let payload = scratch.path.join("payload");
    let max_bytes = config.max_image_bytes.unwrap_or(DEFAULT_MAX_BYTES);
    let max_bytes_text = max_bytes.to_string();
    let mut curl = Command::new(&config.curl_executable);
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
    .arg(&request.canonical_media_url)
    .stderr(Stdio::null());
    let (curl_status, curl_stdout) = output_bounded(&mut curl, 128)?;
    if !curl_status.success() {
        return Err("HTTPS image retrieval failed".into());
    }
    let content_type = String::from_utf8(curl_stdout)?.trim().to_ascii_lowercase();
    if !content_type.starts_with("image/") || content_type.len() > 128 {
        return Err("retrieved object is not a bounded image".into());
    }
    let metadata = fs::symlink_metadata(&payload)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > max_bytes
    {
        return Err("retrieved image payload is invalid".into());
    }
    let checksum = sha256_file(&payload)?;
    let object_key = format!("media/{checksum}");
    let version = u64::from_str_radix(&checksum[..16], 16).unwrap_or(1).max(1);
    let mut upload = upload_command(config, &scratch, &payload, request, &object_key)?;
    let (upload_status, upload_stdout) = output_bounded(&mut upload, 64 * 1024)?;
    if !upload_status.success() {
        return Err("DASObjectStore remote upload failed".into());
    }
    let report = String::from_utf8(upload_stdout)?;
    if !report.lines().any(|line| {
        line.starts_with("Final:")
            && line.contains("state=Complete")
            && line.contains("stage=remote_s3_transfer_complete")
    }) {
        return Err("DASObjectStore did not report verified completion".into());
    }
    Ok(Committed {
        schema_version: REQUEST_SCHEMA,
        catalogue_id: catalogue_id(request),
        title: format!("Captured image from {}", request.site_id),
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
            .arg(&request.object_store_id)
            .arg("--source")
            .arg(container_payload)
            .arg("--key")
            .arg(object_key)
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
        .arg("--source")
        .arg(payload)
        .arg("--key")
        .arg(object_key)
        .args(["--no-progress", "--submit-to-daemon", "--daemon-socket"])
        .arg(
            config
                .daemon_socket
                .as_deref()
                .ok_or("native DAS daemon socket is required")?,
        )
        .stderr(Stdio::null());
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

fn validate_request(request: &Request, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    if request.schema_version != REQUEST_SCHEMA
        || request.endpoint_id != config.endpoint_id
        || request.plan_id.is_empty()
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
            "observed_thumbnail" | "explicit_original"
        )
    {
        return Err("capture request is invalid or changed destination".into());
    }
    let origin = request.origin.parse::<axum::http::Uri>()?;
    let page = request.canonical_page_url.parse::<axum::http::Uri>()?;
    let media = request.canonical_media_url.parse::<axum::http::Uri>()?;
    if origin.scheme_str() != Some("https")
        || page.scheme_str() != Some("https")
        || media.scheme_str() != Some("https")
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
    {
        return Err("capture request URLs are not eligible HTTPS provenance".into());
    }
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
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
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
            "#!/bin/sh\nprintf '%s' \"$*\" | grep -q -- '--config .* upload store-1 --source .* --key media/.* --no-progress --submit-to-daemon --daemon-socket' || exit 9\nprintf 'Daemon remote upload job submitted\\nFinal: job state=Complete stage=remote_s3_transfer_complete\\n'\n",
        );
        let remote_config = root.join("remote.json");
        fs::write(&remote_config, "{}").unwrap();
        fs::set_permissions(&remote_config, fs::Permissions::from_mode(0o600)).unwrap();
        let config = Config {
            schema_version: CONFIG_SCHEMA.into(),
            endpoint_id: "endpoint-1".into(),
            curl_executable: curl,
            dasobjectstore_remote_executable: Some(remote),
            dasobjectstore_remote_config: Some(remote_config),
            daemon_socket: Some(root.join("daemon.sock")),
            container_execution: None,
            max_image_bytes: Some(1024),
        };
        let mut request = Request {
            schema_version: REQUEST_SCHEMA.into(),
            plan_id: "plan-1".into(),
            site_id: "site-1".into(),
            origin: "https://example.invalid".into(),
            canonical_page_url: "https://example.invalid/gallery".into(),
            canonical_media_url: "https://media.invalid/image.png".into(),
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
        assert!(receipt.object_key.starts_with("media/"));
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
            site_id: "site-1".into(),
            origin: "https://example.invalid".into(),
            canonical_page_url: "https://example.invalid/gallery".into(),
            canonical_media_url: "https://media.invalid/image.png".into(),
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
            curl_executable: PathBuf::from("/does/not/run"),
            dasobjectstore_remote_executable: Some(PathBuf::from("/does/not/run")),
            dasobjectstore_remote_config: Some(PathBuf::from("/does/not/read")),
            daemon_socket: Some(PathBuf::from("/does/not/connect")),
            container_execution: None,
            max_image_bytes: Some(1024),
        };
        assert!(validate_request(&request, &config).is_err());
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
            curl_executable: curl,
            dasobjectstore_remote_executable: None,
            dasobjectstore_remote_config: None,
            daemon_socket: None,
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
        };
        let request = Request {
            schema_version: REQUEST_SCHEMA.into(),
            plan_id: "plan-1".into(),
            site_id: "site-1".into(),
            origin: "https://example.invalid".into(),
            canonical_page_url: "https://example.invalid/gallery".into(),
            canonical_media_url: "https://media.invalid/image.png".into(),
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
        assert!(arguments.contains("--daemon-socket /run/dasobjectstore/dasobjectstored.sock"));
        assert_eq!(fs::read_dir(&managed).unwrap().count(), 0);
        let _ = fs::remove_dir_all(root);
    }
}
