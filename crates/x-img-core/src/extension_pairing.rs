// SPDX-License-Identifier: MPL-2.0
//! Host-mediated, metadata-only Firefox pairing contract.
#![allow(missing_docs)]
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pairing {
    pub profile_id: String,
    pub instance_origin: String,
    pub csrf: String,
    pub expires_at: u64,
    pub revoked: bool,
    used: bool,
}
#[derive(Debug, Default)]
pub struct Pairings {
    entries: BTreeMap<String, Pairing>,
    next: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingError {
    UnsafeOrigin,
    Unknown,
    Expired,
    Revoked,
    Csrf,
    Replay,
    Origin,
}
impl Pairings {
    pub fn begin(
        &mut self,
        profile_id: &str,
        origin: &str,
        csrf: &str,
        expires_at: u64,
    ) -> Result<String, PairingError> {
        if !safe_origin(origin) {
            return Err(PairingError::UnsafeOrigin);
        }
        let id = format!("pair-{}", self.next);
        self.next += 1;
        self.entries.insert(
            id.clone(),
            Pairing {
                profile_id: profile_id.into(),
                instance_origin: origin.into(),
                csrf: csrf.into(),
                expires_at,
                revoked: false,
                used: false,
            },
        );
        Ok(id)
    }
    pub fn redeem(
        &mut self,
        id: &str,
        origin: &str,
        csrf: &str,
        now: u64,
    ) -> Result<(), PairingError> {
        let p = self.entries.get_mut(id).ok_or(PairingError::Unknown)?;
        if p.revoked {
            return Err(PairingError::Revoked);
        }
        if p.expires_at <= now {
            return Err(PairingError::Expired);
        }
        if p.instance_origin != origin {
            return Err(PairingError::Origin);
        }
        if p.csrf != csrf {
            return Err(PairingError::Csrf);
        }
        if p.used {
            return Err(PairingError::Replay);
        }
        p.used = true;
        Ok(())
    }
    pub fn revoke(&mut self, id: &str) -> Result<(), PairingError> {
        self.entries
            .get_mut(id)
            .ok_or(PairingError::Unknown)?
            .revoked = true;
        Ok(())
    }
    pub fn rotate(&mut self, id: &str, csrf: &str, expiry: u64) -> Result<(), PairingError> {
        let p = self.entries.get_mut(id).ok_or(PairingError::Unknown)?;
        p.csrf = csrf.into();
        p.expires_at = expiry;
        p.used = false;
        Ok(())
    }
}
fn safe_origin(origin: &str) -> bool {
    origin.starts_with("https://")
        && !origin.contains(['@', '?', '#'])
        && !origin.contains("localhost")
        && !origin.contains("127.")
        && !origin.contains("192.168.")
        && !origin.contains("10.")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pairing_binds_origin_and_rejects_expiry_csrf_replay_revocation_and_local_network() {
        let mut p = Pairings::default();
        assert_eq!(
            p.begin("profile", "https://localhost", "c", 1),
            Err(PairingError::UnsafeOrigin)
        );
        let id = p
            .begin("profile", "https://x-img.example", "c", 10)
            .unwrap();
        assert_eq!(
            p.redeem(&id, "https://other", "c", 1),
            Err(PairingError::Origin)
        );
        assert_eq!(
            p.redeem(&id, "https://x-img.example", "bad", 1),
            Err(PairingError::Csrf)
        );
        p.redeem(&id, "https://x-img.example", "c", 1).unwrap();
        assert_eq!(
            p.redeem(&id, "https://x-img.example", "c", 1),
            Err(PairingError::Replay)
        );
        p.rotate(&id, "next", 20).unwrap();
        p.redeem(&id, "https://x-img.example", "next", 2).unwrap();
        p.revoke(&id).unwrap();
        assert_eq!(
            p.redeem(&id, "https://x-img.example", "next", 3),
            Err(PairingError::Revoked)
        );
    }
}
