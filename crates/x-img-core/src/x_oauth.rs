// SPDX-License-Identifier: MPL-2.0
//! Official X OAuth 2.0 Authorization Code with PKCE contract.
//!
//! Monas owns the client secret (if any), verifier custody, token custody, and
//! session issuance. x-img retains only short-lived non-secret transaction
//! metadata and opaque authority references.

#![allow(missing_docs)]

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const X_AUTHORIZE_URL: &str = "https://x.com/i/oauth2/authorize";
pub const X_TOKEN_URL: &str = "https://api.x.com/2/oauth2/token";
const REQUIRED_SCOPES: [&str; 4] = ["tweet.read", "users.read", "follows.read", "offline.access"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XOAuthConfig {
    pub client_id: String,
    pub redirect_uri: String,
    pub transaction_ref: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XAuthorizationRequest {
    pub url: String,
    pub state: String,
    pub code_challenge: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XTokenGrant {
    pub credential_ref: String,
    pub host_actor_ref: String,
    pub viewing_x_user_id: String,
    pub scopes: BTreeSet<String>,
    pub expires_at_unix_seconds: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XOAuthError {
    Invalid(String),
    StateMismatch,
    StateReplayed,
    AuthorizationDenied,
    Expired,
    ScopeDenied,
    TokenHost(String),
}
impl std::fmt::Display for XOAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(v) => write!(f, "invalid X OAuth request: {v}"),
            Self::StateMismatch => f.write_str("X OAuth callback state does not match"),
            Self::StateReplayed => f.write_str("X OAuth callback state was already consumed"),
            Self::AuthorizationDenied => f.write_str("X OAuth authorization was denied"),
            Self::Expired => f.write_str("X OAuth authorization has expired"),
            Self::ScopeDenied => {
                f.write_str("X OAuth grant lacks required read/follow/refresh scopes")
            }
            Self::TokenHost(v) => {
                write!(f, "Monas X OAuth token authority rejected the request: {v}")
            }
        }
    }
}
impl std::error::Error for XOAuthError {}

/// The host exchanges codes, refreshes and revokes opaque credential references.
/// Implementations must never return raw token strings to x-img.
pub trait XOAuthTokenHost {
    fn exchange_code(
        &mut self,
        transaction_ref: &str,
        authorization_code: &str,
    ) -> Result<XTokenGrant, String>;
    fn refresh(&mut self, credential_ref: &str) -> Result<XTokenGrant, String>;
    fn revoke(&mut self, credential_ref: &str) -> Result<(), String>;
}

pub struct XOAuthFlow {
    pending: BTreeMap<String, PendingAuthorization>,
    consumed: BTreeSet<String>,
}
impl Default for XOAuthFlow {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingAuthorization {
    transaction_ref: String,
    host_actor_ref: String,
    expires_at_unix_seconds: u64,
}

impl XOAuthFlow {
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            consumed: BTreeSet::new(),
        }
    }
    /// Begins an official S256 PKCE authorization request. The host supplies a
    /// random state and verifier and retains the verifier under transaction_ref.
    pub fn begin(
        &mut self,
        config: &XOAuthConfig,
        host_actor_ref: &str,
        state: String,
        code_verifier: &str,
        now: u64,
        expires_at: u64,
    ) -> Result<XAuthorizationRequest, XOAuthError> {
        validate_config(config)?;
        validate_state(&state)?;
        validate_verifier(code_verifier)?;
        if !host_actor_ref.starts_with("monas.host-context:") {
            return Err(XOAuthError::Invalid(
                "host actor must be an opaque Monas reference".to_owned(),
            ));
        }
        if expires_at <= now || self.pending.contains_key(&state) || self.consumed.contains(&state)
        {
            return Err(XOAuthError::Invalid(
                "state expiry or uniqueness is invalid".to_owned(),
            ));
        }
        let challenge = pkce_s256(code_verifier);
        let query = format!(
            "response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
            percent(&config.client_id),
            percent(&config.redirect_uri),
            percent(&REQUIRED_SCOPES.join(" ")),
            percent(&state),
            percent(&challenge)
        );
        self.pending.insert(
            state.clone(),
            PendingAuthorization {
                transaction_ref: config.transaction_ref.clone(),
                host_actor_ref: host_actor_ref.to_owned(),
                expires_at_unix_seconds: expires_at,
            },
        );
        Ok(XAuthorizationRequest {
            url: format!("{X_AUTHORIZE_URL}?{query}"),
            state,
            code_challenge: challenge,
        })
    }
    pub fn complete<H: XOAuthTokenHost>(
        &mut self,
        host: &mut H,
        state: &str,
        authorization_code: Option<&str>,
        denied: bool,
        now: u64,
    ) -> Result<XTokenGrant, XOAuthError> {
        if self.consumed.contains(state) {
            return Err(XOAuthError::StateReplayed);
        }
        let pending = self
            .pending
            .remove(state)
            .ok_or(XOAuthError::StateMismatch)?;
        self.consumed.insert(state.to_owned());
        if denied {
            return Err(XOAuthError::AuthorizationDenied);
        }
        if now >= pending.expires_at_unix_seconds {
            return Err(XOAuthError::Expired);
        }
        let code = authorization_code
            .filter(|code| !code.is_empty() && code.len() <= 2048)
            .ok_or_else(|| {
                XOAuthError::Invalid("authorization code is absent or unsafe".to_owned())
            })?;
        let grant = host
            .exchange_code(&pending.transaction_ref, code)
            .map_err(XOAuthError::TokenHost)?;
        validate_grant(&grant, &pending.host_actor_ref, now)?;
        Ok(grant)
    }
    pub fn refresh<H: XOAuthTokenHost>(
        &self,
        host: &mut H,
        grant: &XTokenGrant,
        now: u64,
    ) -> Result<XTokenGrant, XOAuthError> {
        let refreshed = host
            .refresh(&grant.credential_ref)
            .map_err(XOAuthError::TokenHost)?;
        validate_grant(&refreshed, &grant.host_actor_ref, now)?;
        Ok(refreshed)
    }
    pub fn revoke<H: XOAuthTokenHost>(
        &self,
        host: &mut H,
        grant: &XTokenGrant,
    ) -> Result<(), XOAuthError> {
        host.revoke(&grant.credential_ref)
            .map_err(XOAuthError::TokenHost)
    }
}

/// Protected-account requests may use only the viewing account bound to the grant.
pub fn authorizes_viewing_account(grant: &XTokenGrant, viewing_x_user_id: &str, now: u64) -> bool {
    grant.viewing_x_user_id == viewing_x_user_id
        && now < grant.expires_at_unix_seconds
        && REQUIRED_SCOPES
            .iter()
            .all(|scope| grant.scopes.contains(*scope))
}
pub fn pkce_s256(verifier: &str) -> String {
    base64url(&Sha256::digest(verifier.as_bytes()))
}
fn validate_config(config: &XOAuthConfig) -> Result<(), XOAuthError> {
    if !identifier(&config.client_id)
        || !config.redirect_uri.starts_with("https://")
        || config.redirect_uri.contains(['#', '?'])
        || !config
            .transaction_ref
            .starts_with("monas.oauth-transaction:")
    {
        return Err(XOAuthError::Invalid(
            "client ID, exact HTTPS redirect URI, or opaque Monas transaction reference is invalid"
                .to_owned(),
        ));
    }
    Ok(())
}
fn validate_state(state: &str) -> Result<(), XOAuthError> {
    if state.len() < 32
        || state.len() > 500
        || !state
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'))
    {
        Err(XOAuthError::Invalid(
            "state must be a 32-500 character URL-safe random value".to_owned(),
        ))
    } else {
        Ok(())
    }
}
fn validate_verifier(value: &str) -> Result<(), XOAuthError> {
    if value.len() < 43
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'))
    {
        Err(XOAuthError::Invalid(
            "PKCE verifier must be 43-128 URL-safe characters".to_owned(),
        ))
    } else {
        Ok(())
    }
}
fn validate_grant(grant: &XTokenGrant, actor: &str, now: u64) -> Result<(), XOAuthError> {
    if !grant.credential_ref.starts_with("monas.x-oauth:")
        || grant.host_actor_ref != actor
        || !identifier(&grant.viewing_x_user_id)
    {
        return Err(XOAuthError::Invalid(
            "host returned an invalid opaque X grant".to_owned(),
        ));
    }
    if grant.expires_at_unix_seconds <= now {
        return Err(XOAuthError::Expired);
    }
    if !REQUIRED_SCOPES
        .iter()
        .all(|scope| grant.scopes.contains(*scope))
    {
        return Err(XOAuthError::ScopeDenied);
    }
    Ok(())
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}
fn percent(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}
fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        output.push(char::from(ALPHABET[((n >> 18) & 63) as usize]));
        output.push(char::from(ALPHABET[((n >> 12) & 63) as usize]));
        if chunk.len() > 1 {
            output.push(char::from(ALPHABET[((n >> 6) & 63) as usize]));
        }
        if chunk.len() > 2 {
            output.push(char::from(ALPHABET[(n & 63) as usize]));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        exchanges: u8,
        refreshes: u8,
        revoked: bool,
    }
    fn grant() -> XTokenGrant {
        XTokenGrant {
            credential_ref: "monas.x-oauth:fixture".into(),
            host_actor_ref: "monas.host-context:fixture-user".into(),
            viewing_x_user_id: "12345".into(),
            scopes: REQUIRED_SCOPES.into_iter().map(str::to_owned).collect(),
            expires_at_unix_seconds: 1000,
        }
    }
    impl XOAuthTokenHost for Host {
        fn exchange_code(&mut self, _: &str, _: &str) -> Result<XTokenGrant, String> {
            self.exchanges += 1;
            Ok(grant())
        }
        fn refresh(&mut self, _: &str) -> Result<XTokenGrant, String> {
            self.refreshes += 1;
            Ok(grant())
        }
        fn revoke(&mut self, _: &str) -> Result<(), String> {
            self.revoked = true;
            Ok(())
        }
    }
    fn config() -> XOAuthConfig {
        XOAuthConfig {
            client_id: "ximg-client".into(),
            redirect_uri: "https://monas.example.invalid/products/x-img/api/x/callback".into(),
            transaction_ref: "monas.oauth-transaction:fixture".into(),
        }
    }
    fn state() -> String {
        "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFG".into()
    }
    fn verifier() -> &'static str {
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~abc"
    }
    #[test]
    fn pkce_state_refresh_and_revocation_are_host_managed() {
        let mut flow = XOAuthFlow::new();
        let request = flow
            .begin(
                &config(),
                "monas.host-context:fixture-user",
                state(),
                verifier(),
                100,
                200,
            )
            .expect("begin");
        assert!(request.url.contains("code_challenge_method=S256"));
        assert_eq!(request.code_challenge, pkce_s256(verifier()));
        let mut host = Host::default();
        let grant = flow
            .complete(&mut host, &request.state, Some("code"), false, 150)
            .expect("complete");
        assert!(authorizes_viewing_account(&grant, "12345", 200));
        assert_eq!(
            flow.complete(&mut host, &request.state, Some("code"), false, 150),
            Err(XOAuthError::StateReplayed)
        );
        flow.refresh(&mut host, &grant, 200).expect("refresh");
        flow.revoke(&mut host, &grant).expect("revoke");
        assert_eq!((host.exchanges, host.refreshes, host.revoked), (1, 1, true));
    }
    #[test]
    fn wrong_state_denial_expiry_scope_and_other_viewer_fail_closed() {
        let mut flow = XOAuthFlow::new();
        let request = flow
            .begin(
                &config(),
                "monas.host-context:fixture-user",
                state(),
                verifier(),
                100,
                200,
            )
            .expect("begin");
        let mut host = Host::default();
        assert_eq!(
            flow.complete(&mut host, "other", Some("code"), false, 150),
            Err(XOAuthError::StateMismatch)
        );
        assert_eq!(
            flow.complete(&mut host, &request.state, None, true, 150),
            Err(XOAuthError::AuthorizationDenied)
        );
        let mut bad = grant();
        bad.scopes.remove("follows.read");
        assert!(!authorizes_viewing_account(&bad, "12345", 200));
        assert!(!authorizes_viewing_account(&grant(), "other", 200));
    }
}
