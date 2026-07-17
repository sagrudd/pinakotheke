// SPDX-License-Identifier: MPL-2.0
//! First-party bounded DASObjectStore object-read helper.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const REQUEST_SCHEMA: &str = "pinakotheke.object-read-helper.v1";
const CONFIG_SCHEMA: &str = "pinakotheke.das-object-read-helper.v1";
const MAX_CONFIG_BYTES: usize = 32 * 1024;
const MAX_HEAD_BYTES: usize = 32 * 1024;
const DEFAULT_MAX_OBJECT_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: String,
    endpoint_id: String,
    object_store_id: String,
    object_key: String,
    object_version: u64,
    checksum: String,
    range: Option<Range>,
    if_none_match_etag: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Range {
    start: u64,
    end_inclusive: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: String,
    endpoint_id: String,
    endpoint_url: String,
    region: String,
    profile: String,
    aws_executable: PathBuf,
    stores: Vec<Store>,
    max_object_bytes: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Store {
    object_store_id: String,
    bucket: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HeadObject {
    content_length: u64,
    content_type: Option<String>,
    metadata: std::collections::BTreeMap<String, String>,
}

#[derive(Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
enum Response<'a> {
    Content {
        schema_version: &'static str,
        content_type: &'a str,
        content_length: u64,
        total_length: u64,
        checksum: &'a str,
        etag: &'a str,
        content_range: Option<Range>,
    },
    NotModified {
        schema_version: &'static str,
        etag: &'a str,
    },
}

pub(crate) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = std::env::var_os("PINAKOTHEKE_DAS_READ_HELPER_CONFIG")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".x-img/config/das-object-read-helper.json"))
        })
        .ok_or("PINAKOTHEKE_DAS_READ_HELPER_CONFIG or HOME is required")?;
    let request: Request = serde_json::from_reader(io::stdin().lock())?;
    let config = load_config(&config_path)?;
    serve(&request, &config, io::stdout().lock(), io::stderr().lock())
}

fn load_config(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    require_private_regular(path)?;
    let bytes = fs::read(path)?;
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err("DAS read helper config exceeds 32 KiB".into());
    }
    let config: Config = serde_json::from_slice(&bytes)?;
    validate_config(&config)?;
    Ok(config)
}

fn validate_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let max = config.max_object_bytes.unwrap_or(DEFAULT_MAX_OBJECT_BYTES);
    if config.schema_version != CONFIG_SCHEMA
        || !safe_id(&config.endpoint_id)
        || !eligible_endpoint(&config.endpoint_url)
        || !safe_token(&config.region, 64)
        || !safe_token(&config.profile, 128)
        || config.stores.is_empty()
        || config.stores.len() > 128
        || max == 0
        || max > 16 * 1024 * 1024 * 1024
    {
        return Err("DAS read helper config is invalid".into());
    }
    require_executable(&config.aws_executable)?;
    let mut ids = std::collections::BTreeSet::new();
    for store in &config.stores {
        if !safe_id(&store.object_store_id)
            || !safe_bucket(&store.bucket)
            || !ids.insert(&store.object_store_id)
        {
            return Err("DAS read helper store mapping is invalid".into());
        }
    }
    Ok(())
}

fn serve(
    request: &Request,
    config: &Config,
    mut payload_out: impl Write,
    mut protocol_out: impl Write,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_request(request, config)?;
    let store = config
        .stores
        .iter()
        .find(|item| item.object_store_id == request.object_store_id)
        .ok_or("ObjectStore is not authorized")?;
    let head = head_object(config, &store.bucket, &request.object_key)?;
    let expected = request
        .checksum
        .strip_prefix("sha256:")
        .ok_or("unsupported checksum")?;
    let authority_checksum = head
        .metadata
        .get("dasobjectstore-sha256")
        .map(String::as_str);
    if head.content_length == 0
        || head.content_length > config.max_object_bytes.unwrap_or(DEFAULT_MAX_OBJECT_BYTES)
        || authority_checksum.is_some_and(|checksum| checksum != expected)
        || (request.range.is_some() && authority_checksum != Some(expected))
    {
        return Err("DAS authority metadata did not verify".into());
    }
    let etag = format!("\"sha256:{expected}\"");
    let conditional_match =
        request.if_none_match_etag.as_deref() == Some(&etag) && request.range.is_none();
    if conditional_match && authority_checksum == Some(expected) {
        serde_json::to_writer(
            &mut protocol_out,
            &Response::NotModified {
                schema_version: REQUEST_SCHEMA,
                etag: &etag,
            },
        )?;
        writeln!(protocol_out)?;
        return Ok(());
    }
    let range = request.range;
    if range.is_some_and(|value| {
        value.start > value.end_inclusive || value.end_inclusive >= head.content_length
    }) {
        return Err("requested range is outside the verified object".into());
    }
    let scratch = Scratch::create()?;
    let object = scratch.path.join("object");
    get_object(config, &store.bucket, &request.object_key, range, &object)?;
    let length = fs::metadata(&object)?.len();
    let expected_length = range.map_or(head.content_length, |value| {
        value.end_inclusive - value.start + 1
    });
    if length != expected_length {
        return Err("DAS object length did not verify".into());
    }
    if range.is_none() && sha256_file(&object)? != expected {
        return Err("DAS object checksum did not verify".into());
    }
    if conditional_match {
        serde_json::to_writer(
            &mut protocol_out,
            &Response::NotModified {
                schema_version: REQUEST_SCHEMA,
                etag: &etag,
            },
        )?;
        writeln!(protocol_out)?;
        return Ok(());
    }
    let content_type = head
        .content_type
        .as_deref()
        .unwrap_or("application/octet-stream");
    if content_type.len() > 128 || content_type.contains(['\r', '\n']) {
        return Err("DAS content type is invalid".into());
    }
    serde_json::to_writer(
        &mut protocol_out,
        &Response::Content {
            schema_version: REQUEST_SCHEMA,
            content_type,
            content_length: length,
            total_length: head.content_length,
            checksum: &request.checksum,
            etag: &etag,
            content_range: range,
        },
    )?;
    writeln!(protocol_out)?;
    io::copy(
        &mut File::open(object)?.take(expected_length),
        &mut payload_out,
    )?;
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

fn aws_base(config: &Config) -> Command {
    let mut command = Command::new(&config.aws_executable);
    command.args([
        "--endpoint-url",
        &config.endpoint_url,
        "--region",
        &config.region,
        "--profile",
        &config.profile,
    ]);
    command
}

fn head_object(
    config: &Config,
    bucket: &str,
    key: &str,
) -> Result<HeadObject, Box<dyn std::error::Error>> {
    let mut command = aws_base(config);
    command.args([
        "s3api",
        "head-object",
        "--bucket",
        bucket,
        "--key",
        key,
        "--output",
        "json",
    ]);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut output = Vec::new();
    child
        .stdout
        .take()
        .ok_or("AWS metadata output is unavailable")?
        .take((MAX_HEAD_BYTES + 1) as u64)
        .read_to_end(&mut output)?;
    if output.len() > MAX_HEAD_BYTES {
        let _ = child.kill();
        let _ = child.wait();
        return Err("DAS object metadata lookup exceeded its bound".into());
    }
    if !child.wait()?.success() {
        return Err("DAS object metadata lookup failed".into());
    }
    Ok(serde_json::from_slice(&output)?)
}

fn get_object(
    config: &Config,
    bucket: &str,
    key: &str,
    range: Option<Range>,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = aws_base(config);
    command.args(["s3api", "get-object", "--bucket", bucket, "--key", key]);
    if let Some(value) = range {
        command.args([
            "--range",
            &format!("bytes={}-{}", value.start, value.end_inclusive),
        ]);
    }
    let status = command
        .arg(output)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !status.success() {
        return Err("DAS object read failed".into());
    }
    Ok(())
}

fn validate_request(request: &Request, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    if request.schema_version != REQUEST_SCHEMA
        || request.endpoint_id != config.endpoint_id
        || request.object_version == 0
        || !safe_id(&request.object_store_id)
        || request.object_key.is_empty()
        || request.object_key.len() > 1024
        || request.object_key.starts_with('/')
        || request.object_key.contains("..")
        || request.object_key.chars().any(char::is_control)
        || request.checksum.len() != 71
        || !request.checksum.starts_with("sha256:")
        || !request.checksum[7..]
            .chars()
            .all(|value| value.is_ascii_hexdigit() && !value.is_ascii_uppercase())
    {
        return Err("object read request is invalid or changed authority".into());
    }
    Ok(())
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}
fn safe_bucket(value: &str) -> bool {
    value.len() >= 3
        && value.len() <= 63
        && value.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && value.ends_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.'))
}

fn safe_token(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.starts_with(|c: char| c.is_ascii_alphanumeric())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn eligible_endpoint(value: &str) -> bool {
    let Ok(uri) = value.parse::<axum::http::Uri>() else {
        return false;
    };
    let Some(authority) = uri.authority() else {
        return false;
    };
    if authority.as_str().contains('@') || !matches!(uri.path(), "" | "/") || uri.query().is_some()
    {
        return false;
    }
    match uri.scheme_str() {
        Some("https") => true,
        Some("http") => matches!(authority.host(), "127.0.0.1" | "localhost"),
        _ => false,
    }
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.permissions().mode() & 0o111 == 0
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "executable is not a reviewed executable",
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
            "pinakotheke-das-read-{}-{nonce}",
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
    use std::{
        os::unix::fs::PermissionsExt,
        sync::atomic::{AtomicU64, Ordering},
    };

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

    fn fixture() -> (PathBuf, Config, Request) {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-das-read-test-{}-{}",
            std::process::id(),
            FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let aws = root.join("aws");
        fs::write(&aws, "#!/bin/sh\ncase \"$*\" in *head-object*) printf '%s' '{\"ContentLength\":3,\"ContentType\":\"image/png\",\"Metadata\":{\"dasobjectstore-sha256\":\"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\"}}';; *get-object*) for arg do out=$arg; done; case \"$*\" in *'--range bytes=1-2'*) printf bc > \"$out\";; *) printf abc > \"$out\";; esac;; *) exit 9;; esac\n").unwrap();
        fs::set_permissions(&aws, fs::Permissions::from_mode(0o700)).unwrap();
        let config = Config {
            schema_version: CONFIG_SCHEMA.into(),
            endpoint_id: "endpoint-1".into(),
            endpoint_url: "http://127.0.0.1:3900".into(),
            region: "garage".into(),
            profile: "pinakotheke".into(),
            aws_executable: aws,
            stores: vec![Store {
                object_store_id: "store-1".into(),
                bucket: "dos-store-1".into(),
            }],
            max_object_bytes: Some(1024),
        };
        let request = Request {
            schema_version: REQUEST_SCHEMA.into(),
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
            object_key: "media/fixture".into(),
            object_version: 7,
            checksum: "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                .into(),
            range: None,
            if_none_match_etag: None,
        };
        (root, config, request)
    }

    #[test]
    fn streams_only_verified_authority_object_and_cleans_scratch() {
        let (root, config, request) = fixture();
        let scratch = Scratch::create().unwrap();
        let scratch_path = scratch.path.clone();
        drop(scratch);
        assert!(!scratch_path.exists());
        let mut payload = Vec::new();
        let mut protocol = Vec::new();
        serve(&request, &config, &mut payload, &mut protocol).unwrap();
        assert_eq!(payload, b"abc");
        assert!(
            String::from_utf8(protocol)
                .unwrap()
                .contains("\"content_length\":3")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verifies_full_legacy_object_bytes_when_authority_checksum_metadata_is_absent() {
        let (root, config, mut request) = fixture();
        fs::write(&config.aws_executable, "#!/bin/sh\ncase \"$*\" in *head-object*) printf '%s' '{\"ContentLength\":3,\"ContentType\":\"image/png\",\"Metadata\":{}}';; *get-object*) for arg do out=$arg; done; printf abc > \"$out\";; *) exit 9;; esac\n").unwrap();
        let mut payload = Vec::new();
        serve(&request, &config, &mut payload, Vec::new()).unwrap();
        assert_eq!(payload, b"abc");

        request.range = Some(Range {
            start: 0,
            end_inclusive: 1,
        });
        assert!(serve(&request, &config, Vec::new(), Vec::new()).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_changed_authority_and_bad_checksum_metadata() {
        let (root, mut config, mut request) = fixture();
        request.endpoint_id = "other".into();
        assert!(serve(&request, &config, Vec::new(), Vec::new()).is_err());
        request.endpoint_id = "endpoint-1".into();
        request.checksum = format!("sha256:{}", "0".repeat(64));
        assert!(serve(&request, &config, Vec::new(), Vec::new()).is_err());
        config.endpoint_url = "http://127.0.0.1:3900@remote.invalid".into();
        assert!(validate_config(&config).is_err());
        config.endpoint_url = "http://127.0.0.1:3900".into();
        config.profile = "--inject".into();
        assert!(validate_config(&config).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn supports_verified_ranges_and_checksum_etag_conditionals() {
        let (root, config, mut request) = fixture();
        request.range = Some(Range {
            start: 1,
            end_inclusive: 2,
        });
        let mut payload = Vec::new();
        let mut protocol = Vec::new();
        serve(&request, &config, &mut payload, &mut protocol).unwrap();
        assert_eq!(payload, b"bc");
        assert!(
            std::str::from_utf8(&protocol)
                .unwrap()
                .contains("\"start\":1")
        );

        request.range = None;
        request.if_none_match_etag = Some(format!("\"{}\"", request.checksum));
        payload.clear();
        protocol.clear();
        serve(&request, &config, &mut payload, &mut protocol).unwrap();
        assert!(payload.is_empty());
        assert!(
            String::from_utf8(protocol)
                .unwrap()
                .contains("not_modified")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
