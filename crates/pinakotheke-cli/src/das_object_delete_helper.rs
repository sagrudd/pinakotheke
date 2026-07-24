// SPDX-License-Identifier: MPL-2.0
//! First-party DASObjectStore exact-object deletion helper.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, BufRead, BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    time::Duration,
};

const HELPER_SCHEMA: &str = "pinakotheke.object-delete-helper.v1";
const CONFIG_SCHEMA: &str = "pinakotheke.das-object-delete-helper.v1";
const SESSION_SCHEMA: &str = "pinakotheke.das-object-delete-session.v1";
const DAS_SCHEMA: &str = "dasobjectstore.application_object_delete.v1";
const MAX_DOCUMENT_BYTES: u64 = 32 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HelperRequest {
    schema_version: String,
    endpoint_id: String,
    object_store_id: String,
    object_key: String,
    object_version: u64,
    checksum: String,
    content_length: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: String,
    daemon_socket: PathBuf,
    endpoint_id: String,
    application_id: String,
    session_file: PathBuf,
    provider: String,
    endpoint_url: String,
    stores: Vec<Store>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Store {
    object_store_id: String,
    bucket: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Session {
    schema_version: String,
    session_id: String,
    renewal_token: String,
}

#[derive(Serialize)]
struct DaemonEnvelope<'a> {
    command: &'static str,
    payload: DaemonDeleteRequest<'a>,
}

#[derive(Serialize)]
struct DaemonDeleteRequest<'a> {
    schema_version: &'static str,
    request_id: String,
    session_id: &'a str,
    renewal_token: &'a str,
    application_id: &'a str,
    object_store: &'a str,
    object_id: &'a str,
    object_version: u64,
    object_key: &'a str,
    expected_size_bytes: u64,
    expected_checksum: &'a str,
    provider: &'a str,
    bucket: &'a str,
    endpoint_url: &'a str,
    reason: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
enum DaemonResponse {
    ApplicationObjectDeleted(DaemonDeleteResponse),
    Error(DaemonError),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DaemonDeleteResponse {
    schema_version: String,
    request_id: String,
    outcome: DeleteOutcome,
    audit_event_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DeleteOutcome {
    Deleted,
    AlreadyAbsent,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DaemonError {
    code: String,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
enum HelperResponse {
    Deleted { schema_version: &'static str },
    AlreadyAbsent { schema_version: &'static str },
    Rejected { schema_version: &'static str },
    Unavailable { schema_version: &'static str },
}

pub(crate) fn run_protocol() -> Result<(), Box<dyn std::error::Error>> {
    let request = serde_json::from_reader::<_, HelperRequest>(io::stdin().lock());
    let outcome = match request {
        Ok(request) => execute(&request).unwrap_or_else(|kind| kind),
        Err(_) => HelperResponse::Rejected {
            schema_version: HELPER_SCHEMA,
        },
    };
    serde_json::to_writer(io::stderr().lock(), &outcome)?;
    writeln!(io::stderr().lock())?;
    Ok(())
}

fn execute(request: &HelperRequest) -> Result<HelperResponse, HelperResponse> {
    validate_request(request).map_err(|_| rejected())?;
    let config_path = std::env::var_os("PINAKOTHEKE_DAS_DELETE_HELPER_CONFIG")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".x-img/config/das-object-delete-helper.json"))
        })
        .ok_or_else(unavailable)?;
    execute_with_config(request, &config_path)
}

fn execute_with_config(
    request: &HelperRequest,
    config_path: &Path,
) -> Result<HelperResponse, HelperResponse> {
    let config: Config = load_private_json(config_path).map_err(|_| unavailable())?;
    validate_config(&config).map_err(|_| rejected())?;
    if request.endpoint_id != config.endpoint_id {
        return Err(rejected());
    }
    let store = config
        .stores
        .iter()
        .find(|store| store.object_store_id == request.object_store_id)
        .ok_or_else(rejected)?;
    let session: Session = load_private_json(&config.session_file).map_err(|_| unavailable())?;
    validate_session(&session).map_err(|_| rejected())?;
    let request_id = request_id(request);
    let envelope = DaemonEnvelope {
        command: "delete_application_object",
        payload: DaemonDeleteRequest {
            schema_version: DAS_SCHEMA,
            request_id: request_id.clone(),
            session_id: &session.session_id,
            renewal_token: &session.renewal_token,
            application_id: &config.application_id,
            object_store: &request.object_store_id,
            object_id: &request.object_key,
            object_version: request.object_version,
            object_key: &request.object_key,
            expected_size_bytes: request.content_length,
            expected_checksum: &request.checksum,
            provider: &config.provider,
            bucket: &store.bucket,
            endpoint_url: &config.endpoint_url,
            reason: "user_requested",
        },
    };
    let response = send(&config.daemon_socket, &envelope).map_err(|_| unavailable())?;
    match response {
        DaemonResponse::ApplicationObjectDeleted(response)
            if response.schema_version == DAS_SCHEMA
                && response.request_id == request_id
                && !response.audit_event_id.trim().is_empty() =>
        {
            Ok(match response.outcome {
                DeleteOutcome::Deleted => HelperResponse::Deleted {
                    schema_version: HELPER_SCHEMA,
                },
                DeleteOutcome::AlreadyAbsent => HelperResponse::AlreadyAbsent {
                    schema_version: HELPER_SCHEMA,
                },
            })
        }
        DaemonResponse::ApplicationObjectDeleted(_) => Err(rejected()),
        DaemonResponse::Error(error) => {
            let _redacted_category = (error.code, error.message.is_empty());
            Err(rejected())
        }
    }
}

fn send(
    socket: &Path,
    request: &DaemonEnvelope<'_>,
) -> Result<DaemonResponse, Box<dyn std::error::Error>> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    serde_json::to_writer(&mut stream, request)?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader
        .by_ref()
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_line(&mut response)?;
    if response.is_empty() || response.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err("DASObjectStore returned an invalid response".into());
    }
    Ok(serde_json::from_str(&response)?)
}

fn load_private_json<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<T, Box<dyn std::error::Error>> {
    require_private_regular(path)?;
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 || metadata.len() > MAX_DOCUMENT_BYTES {
        return Err("private helper document has an invalid size".into());
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn require_private_regular(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "private helper path must be absolute",
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "private helper path must name a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "private helper file must not be group or other accessible",
            ));
        }
    }
    Ok(())
}

fn validate_request(request: &HelperRequest) -> Result<(), ()> {
    if request.schema_version != HELPER_SCHEMA
        || !safe_id(&request.endpoint_id)
        || !safe_id(&request.object_store_id)
        || !safe_key(&request.object_key)
        || request.object_version == 0
        || request.content_length == 0
        || !sha256(&request.checksum)
    {
        return Err(());
    }
    Ok(())
}

fn validate_config(config: &Config) -> Result<(), ()> {
    if config.schema_version != CONFIG_SCHEMA
        || !config.daemon_socket.is_absolute()
        || !safe_id(&config.endpoint_id)
        || !safe_id(&config.application_id)
        || !config.session_file.is_absolute()
        || config.provider != "garage"
        || !(config.endpoint_url.starts_with("http://")
            || config.endpoint_url.starts_with("https://"))
        || config.stores.is_empty()
        || config.stores.len() > 128
    {
        return Err(());
    }
    let mut ids = std::collections::BTreeSet::new();
    if config.stores.iter().any(|store| {
        !safe_id(&store.object_store_id)
            || !safe_bucket(&store.bucket)
            || !ids.insert(&store.object_store_id)
    }) {
        return Err(());
    }
    Ok(())
}

fn validate_session(session: &Session) -> Result<(), ()> {
    if session.schema_version != SESSION_SCHEMA
        || !safe_token(&session.session_id, 512)
        || !safe_token(&session.renewal_token, 2048)
    {
        return Err(());
    }
    Ok(())
}

fn request_id(request: &HelperRequest) -> String {
    let identity = format!(
        "{}\n{}\n{}\n{}\n{}",
        request.object_store_id,
        request.object_key,
        request.object_version,
        request.content_length,
        request.checksum
    );
    let digest = format!("{:x}", Sha256::digest(identity.as_bytes()));
    format!("pinakotheke-delete-{}", &digest[..24])
}

fn safe_id(value: &str) -> bool {
    safe_token(value, 256)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn safe_token(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && !value.chars().any(char::is_whitespace)
        && !value.chars().any(char::is_control)
}

fn safe_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 2048
        && !value.starts_with('/')
        && !value.contains('\\')
        && value
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn safe_bucket(value: &str) -> bool {
    safe_id(value) && value.len() >= 3 && value.len() <= 63
}

fn sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

const fn rejected() -> HelperResponse {
    HelperResponse::Rejected {
        schema_version: HELPER_SCHEMA,
    }
}

const fn unavailable() -> HelperResponse {
    HelperResponse::Unavailable {
        schema_version: HELPER_SCHEMA,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::{fs::PermissionsExt, net::UnixListener},
        thread,
    };

    fn request() -> HelperRequest {
        HelperRequest {
            schema_version: HELPER_SCHEMA.into(),
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
            object_key: "sites/example/asset".into(),
            object_version: 7,
            checksum: format!("sha256:{}", "a".repeat(64)),
            content_length: 42,
        }
    }

    #[test]
    fn exact_daemon_exchange_returns_deleted_without_exposing_secrets() {
        let root =
            std::env::temp_dir().join(format!("pinakotheke-delete-helper-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let socket = root.join("daemon.sock");
        let session = root.join("session.json");
        let config = root.join("config.json");
        fs::write(
            &session,
            r#"{"schema_version":"pinakotheke.das-object-delete-session.v1","session_id":"session-secret","renewal_token":"renewal-secret"}"#,
        )
        .unwrap();
        fs::write(
            &config,
            format!(
                r#"{{"schema_version":"pinakotheke.das-object-delete-helper.v1","daemon_socket":"{}","endpoint_id":"endpoint-1","application_id":"pinakotheke","session_file":"{}","provider":"garage","endpoint_url":"http://127.0.0.1:3900","stores":[{{"object_store_id":"store-1","bucket":"dos-store-1"}}]}}"#,
                socket.display(),
                session.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&session, fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
        let listener = UnixListener::bind(&socket).unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(line.contains("\"command\":\"delete_application_object\""));
            assert!(line.contains("\"expected_size_bytes\":42"));
            assert!(line.contains("\"object_id\":\"sites/example/asset\""));
            assert!(line.contains("\"renewal_token\":\"renewal-secret\""));
            let request_id = request_id(&request());
            writeln!(
                reader.get_mut(),
                "{{\"kind\":\"application_object_deleted\",\"payload\":{{\"schema_version\":\"{DAS_SCHEMA}\",\"request_id\":\"{request_id}\",\"outcome\":\"deleted\",\"audit_event_id\":\"audit-1\"}}}}"
            )
            .unwrap();
        });
        let response = execute_with_config(&request(), &config).unwrap();
        server.join().unwrap();
        assert!(matches!(response, HelperResponse::Deleted { .. }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_changed_evidence_before_contacting_the_daemon() {
        let mut invalid = request();
        invalid.content_length = 0;
        assert!(matches!(
            execute(&invalid),
            Err(HelperResponse::Rejected { .. })
        ));
    }

    #[test]
    fn request_identity_is_stable_and_changes_with_exact_evidence() {
        let first = request_id(&request());
        assert_eq!(first, request_id(&request()));
        let mut changed = request();
        changed.object_version += 1;
        assert_ne!(first, request_id(&changed));
    }
}
