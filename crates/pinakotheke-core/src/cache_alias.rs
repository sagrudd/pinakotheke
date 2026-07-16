// SPDX-License-Identifier: MPL-2.0
//! Bounded, privacy-preserving cache-alias lookup for Firefox substitution.
//!
//! The index contains metadata for media already admitted and committed to
//! DASObjectStore. It owns no payload bytes, credentials, page URLs, browsing
//! history, or origin fallback. A caller treats every non-hit as "serve origin".

#![allow(missing_docs)]

use std::collections::{BTreeMap, VecDeque};

use crate::object_read::AuthorizedObjectReference;

pub const MAX_ALIAS_INDEX_CAPACITY: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheRepresentation {
    ThumbnailImage,
    OriginalImage,
    NormalizedMp4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheEligibility {
    ObservedThumbnail,
    ExplicitlyOpenedOriginal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheObjectAvailability {
    Ready,
    EndpointOffline,
    ObjectUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheAliasRecord {
    pub delivery_id: String,
    pub instance_id: String,
    pub site_origin: String,
    pub canonical_alias: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub representation: CacheRepresentation,
    pub eligibility: CacheEligibility,
    pub object: AuthorizedObjectReference,
    pub content_type: String,
    pub content_length: u64,
    pub valid_until_epoch_seconds: u64,
    pub availability: CacheObjectAvailability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheLookupRequest {
    pub pairing_id: String,
    pub instance_id: String,
    pub site_origin: String,
    pub canonical_alias: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub now_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheLookupAuthorization {
    pub pairing_id: String,
    pub actor_id: String,
    pub instance_id: String,
    pub site_origin: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub substitution_enabled: bool,
    pub expires_at_epoch_seconds: u64,
    pub revoked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheBypassReason {
    InvalidRequest,
    SubstitutionPaused,
    PairingInvalid,
    WrongInstance,
    AdapterMismatch,
    Stale,
    EndpointOffline,
    ObjectUnavailable,
    NotAnImage,
    NotNormalizedMp4,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CacheLookupOutcome<'a> {
    Hit(&'a CacheAliasRecord),
    Miss,
    OriginFallback(CacheBypassReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheAliasError {
    InvalidCapacity,
    InvalidRecord(&'static str),
    ImmutableAliasConflict,
}

impl std::fmt::Display for CacheAliasError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCapacity => formatter.write_str("cache alias capacity is invalid"),
            Self::InvalidRecord(field) => write!(formatter, "invalid cache alias field: {field}"),
            Self::ImmutableAliasConflict => {
                formatter.write_str("canonical alias already identifies another immutable object")
            }
        }
    }
}

impl std::error::Error for CacheAliasError {}

#[derive(Debug, Clone)]
pub struct CacheAliasIndex {
    capacity: usize,
    len: usize,
    by_origin: BTreeMap<String, BTreeMap<String, CacheAliasRecord>>,
    by_delivery: BTreeMap<String, (String, String)>,
    insertion_order: VecDeque<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct CacheLookupService {
    index: CacheAliasIndex,
    authorizations: BTreeMap<String, CacheLookupAuthorization>,
}

impl CacheLookupService {
    pub fn new(
        index: CacheAliasIndex,
        authorizations: impl IntoIterator<Item = CacheLookupAuthorization>,
    ) -> Result<Self, CacheAliasError> {
        let mut by_pairing = BTreeMap::new();
        for authorization in authorizations {
            validate_authorization(&authorization)?;
            if by_pairing
                .insert(authorization.pairing_id.clone(), authorization)
                .is_some()
            {
                return Err(CacheAliasError::InvalidRecord("pairing_id"));
            }
        }
        Ok(Self {
            index,
            authorizations: by_pairing,
        })
    }

    #[must_use]
    pub fn lookup(&self, actor_id: &str, request: &CacheLookupRequest) -> CacheLookupOutcome<'_> {
        let Some(authorization) = self.authorizations.get(&request.pairing_id) else {
            return CacheLookupOutcome::OriginFallback(CacheBypassReason::PairingInvalid);
        };
        self.index.lookup(actor_id, request, authorization)
    }

    /// Resolves a delivery grant back to the exact reviewed object and repeats
    /// the pairing/policy checks immediately before a DASObjectStore read.
    #[must_use]
    pub fn authorize_image_delivery(
        &self,
        actor_id: &str,
        pairing_id: &str,
        delivery_id: &str,
        now_epoch_seconds: u64,
    ) -> CacheLookupOutcome<'_> {
        let outcome = self.authorize_delivery(actor_id, pairing_id, delivery_id, now_epoch_seconds);
        match outcome {
            CacheLookupOutcome::Hit(record)
                if matches!(
                    record.representation,
                    CacheRepresentation::ThumbnailImage | CacheRepresentation::OriginalImage
                ) =>
            {
                CacheLookupOutcome::Hit(record)
            }
            CacheLookupOutcome::Hit(_) => {
                CacheLookupOutcome::OriginFallback(CacheBypassReason::NotAnImage)
            }
            other => other,
        }
    }

    /// Resolves only a verified normalized MP4 for native Firefox range
    /// playback. Source video and other representations remain origin-served.
    #[must_use]
    pub fn authorize_video_delivery(
        &self,
        actor_id: &str,
        pairing_id: &str,
        delivery_id: &str,
        now_epoch_seconds: u64,
    ) -> CacheLookupOutcome<'_> {
        let outcome = self.authorize_delivery(actor_id, pairing_id, delivery_id, now_epoch_seconds);
        match outcome {
            CacheLookupOutcome::Hit(record)
                if record.representation == CacheRepresentation::NormalizedMp4 =>
            {
                CacheLookupOutcome::Hit(record)
            }
            CacheLookupOutcome::Hit(_) => {
                CacheLookupOutcome::OriginFallback(CacheBypassReason::NotNormalizedMp4)
            }
            other => other,
        }
    }

    fn authorize_delivery(
        &self,
        actor_id: &str,
        pairing_id: &str,
        delivery_id: &str,
        now_epoch_seconds: u64,
    ) -> CacheLookupOutcome<'_> {
        let Some(authorization) = self.authorizations.get(pairing_id) else {
            return CacheLookupOutcome::OriginFallback(CacheBypassReason::PairingInvalid);
        };
        if !identifier(actor_id)
            || !identifier(delivery_id)
            || authorization.revoked
            || now_epoch_seconds >= authorization.expires_at_epoch_seconds
            || authorization.actor_id != actor_id
            || authorization.pairing_id != pairing_id
        {
            return CacheLookupOutcome::OriginFallback(CacheBypassReason::PairingInvalid);
        }
        if !authorization.substitution_enabled {
            return CacheLookupOutcome::OriginFallback(CacheBypassReason::SubstitutionPaused);
        }
        let Some(record) = self.index.record_by_delivery(delivery_id) else {
            return CacheLookupOutcome::Miss;
        };
        if record.instance_id != authorization.instance_id {
            return CacheLookupOutcome::OriginFallback(CacheBypassReason::WrongInstance);
        }
        if record.site_origin != authorization.site_origin
            || record.adapter_id != authorization.adapter_id
            || record.adapter_version != authorization.adapter_version
        {
            return CacheLookupOutcome::OriginFallback(CacheBypassReason::AdapterMismatch);
        }
        if now_epoch_seconds > record.valid_until_epoch_seconds {
            return CacheLookupOutcome::OriginFallback(CacheBypassReason::Stale);
        }
        match record.availability {
            CacheObjectAvailability::Ready => CacheLookupOutcome::Hit(record),
            CacheObjectAvailability::EndpointOffline => {
                CacheLookupOutcome::OriginFallback(CacheBypassReason::EndpointOffline)
            }
            CacheObjectAvailability::ObjectUnavailable => {
                CacheLookupOutcome::OriginFallback(CacheBypassReason::ObjectUnavailable)
            }
        }
    }

    pub fn index_mut(&mut self) -> &mut CacheAliasIndex {
        &mut self.index
    }
}

impl CacheAliasIndex {
    pub fn new(capacity: usize) -> Result<Self, CacheAliasError> {
        if capacity == 0 || capacity > MAX_ALIAS_INDEX_CAPACITY {
            return Err(CacheAliasError::InvalidCapacity);
        }
        Ok(Self {
            capacity,
            len: 0,
            by_origin: BTreeMap::new(),
            by_delivery: BTreeMap::new(),
            insertion_order: VecDeque::with_capacity(capacity),
        })
    }

    pub fn admit(&mut self, record: CacheAliasRecord) -> Result<bool, CacheAliasError> {
        validate_record(&record)?;
        if let Some(existing) = self
            .by_origin
            .get(&record.site_origin)
            .and_then(|aliases| aliases.get(&record.canonical_alias))
        {
            if immutable_identity(existing) == immutable_identity(&record) {
                return Ok(false);
            }
            return Err(CacheAliasError::ImmutableAliasConflict);
        }
        if self.by_delivery.contains_key(&record.delivery_id) {
            return Err(CacheAliasError::ImmutableAliasConflict);
        }

        while self.len >= self.capacity {
            let Some((origin, alias)) = self.insertion_order.pop_front() else {
                break;
            };
            self.remove(&origin, &alias);
        }
        self.insertion_order
            .push_back((record.site_origin.clone(), record.canonical_alias.clone()));
        self.by_delivery.insert(
            record.delivery_id.clone(),
            (record.site_origin.clone(), record.canonical_alias.clone()),
        );
        self.by_origin
            .entry(record.site_origin.clone())
            .or_default()
            .insert(record.canonical_alias.clone(), record);
        self.len += 1;
        Ok(true)
    }

    #[must_use]
    pub fn lookup(
        &self,
        actor_id: &str,
        request: &CacheLookupRequest,
        authorization: &CacheLookupAuthorization,
    ) -> CacheLookupOutcome<'_> {
        if validate_lookup(request).is_err() {
            return CacheLookupOutcome::OriginFallback(CacheBypassReason::InvalidRequest);
        }
        if !identifier(actor_id)
            || authorization.revoked
            || request.now_epoch_seconds >= authorization.expires_at_epoch_seconds
            || authorization.actor_id != actor_id
            || authorization.pairing_id != request.pairing_id
            || authorization.instance_id != request.instance_id
            || authorization.site_origin != request.site_origin
            || authorization.adapter_id != request.adapter_id
            || authorization.adapter_version != request.adapter_version
        {
            return CacheLookupOutcome::OriginFallback(CacheBypassReason::PairingInvalid);
        }
        if !authorization.substitution_enabled {
            return CacheLookupOutcome::OriginFallback(CacheBypassReason::SubstitutionPaused);
        }
        let Some(record) = self
            .by_origin
            .get(&request.site_origin)
            .and_then(|aliases| aliases.get(&request.canonical_alias))
        else {
            return CacheLookupOutcome::Miss;
        };
        if record.instance_id != request.instance_id {
            return CacheLookupOutcome::OriginFallback(CacheBypassReason::WrongInstance);
        }
        if record.adapter_id != request.adapter_id
            || record.adapter_version != request.adapter_version
        {
            return CacheLookupOutcome::OriginFallback(CacheBypassReason::AdapterMismatch);
        }
        if request.now_epoch_seconds > record.valid_until_epoch_seconds {
            return CacheLookupOutcome::OriginFallback(CacheBypassReason::Stale);
        }
        match record.availability {
            CacheObjectAvailability::Ready => CacheLookupOutcome::Hit(record),
            CacheObjectAvailability::EndpointOffline => {
                CacheLookupOutcome::OriginFallback(CacheBypassReason::EndpointOffline)
            }
            CacheObjectAvailability::ObjectUnavailable => {
                CacheLookupOutcome::OriginFallback(CacheBypassReason::ObjectUnavailable)
            }
        }
    }

    pub fn set_availability(
        &mut self,
        site_origin: &str,
        canonical_alias: &str,
        expected_checksum: &str,
        availability: CacheObjectAvailability,
    ) -> bool {
        let Some(record) = self
            .by_origin
            .get_mut(site_origin)
            .and_then(|aliases| aliases.get_mut(canonical_alias))
        else {
            return false;
        };
        if record.object.checksum != expected_checksum {
            return false;
        }
        record.availability = availability;
        true
    }

    pub fn invalidate(&mut self, site_origin: &str, canonical_alias: &str) -> bool {
        let removed = self.remove(site_origin, canonical_alias);
        if removed {
            self.insertion_order
                .retain(|(origin, alias)| origin != site_origin || alias != canonical_alias);
        }
        removed
    }

    pub fn invalidate_origin(&mut self, site_origin: &str) -> usize {
        let removed_records = self.by_origin.remove(site_origin).unwrap_or_default();
        let removed = removed_records.len();
        for record in removed_records.values() {
            self.by_delivery.remove(&record.delivery_id);
        }
        self.len = self.len.saturating_sub(removed);
        self.insertion_order
            .retain(|(origin, _)| origin != site_origin);
        removed
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn record_by_delivery(&self, delivery_id: &str) -> Option<&CacheAliasRecord> {
        let (origin, alias) = self.by_delivery.get(delivery_id)?;
        self.by_origin.get(origin)?.get(alias)
    }

    fn remove(&mut self, site_origin: &str, canonical_alias: &str) -> bool {
        let Some(aliases) = self.by_origin.get_mut(site_origin) else {
            return false;
        };
        let removed = aliases.remove(canonical_alias);
        if let Some(record) = &removed {
            self.by_delivery.remove(&record.delivery_id);
        }
        if removed.is_some() {
            self.len = self.len.saturating_sub(1);
        }
        if aliases.is_empty() {
            self.by_origin.remove(site_origin);
        }
        removed.is_some()
    }
}

fn immutable_identity(record: &CacheAliasRecord) -> (&str, &str, &AuthorizedObjectReference) {
    (&record.delivery_id, &record.instance_id, &record.object)
}

fn validate_record(record: &CacheAliasRecord) -> Result<(), CacheAliasError> {
    for (field, value) in [
        ("delivery_id", record.delivery_id.as_str()),
        ("instance_id", record.instance_id.as_str()),
        ("adapter_id", record.adapter_id.as_str()),
        ("endpoint_id", record.object.endpoint_id.as_str()),
        ("object_store_id", record.object.object_store_id.as_str()),
    ] {
        if !identifier(value) {
            return Err(CacheAliasError::InvalidRecord(field));
        }
    }
    if !https_origin(&record.site_origin) {
        return Err(CacheAliasError::InvalidRecord("site_origin"));
    }
    if !canonical_alias(&record.canonical_alias) {
        return Err(CacheAliasError::InvalidRecord("canonical_alias"));
    }
    if !semver(&record.adapter_version) {
        return Err(CacheAliasError::InvalidRecord("adapter_version"));
    }
    if !safe_object_key(&record.object.object_key) {
        return Err(CacheAliasError::InvalidRecord("object_key"));
    }
    if !sha256(&record.object.checksum) {
        return Err(CacheAliasError::InvalidRecord("checksum"));
    }
    if record.content_length == 0 {
        return Err(CacheAliasError::InvalidRecord("content_length"));
    }
    if record.valid_until_epoch_seconds == 0 {
        return Err(CacheAliasError::InvalidRecord("valid_until_epoch_seconds"));
    }
    match (record.representation, record.eligibility) {
        (CacheRepresentation::ThumbnailImage, CacheEligibility::ObservedThumbnail)
        | (
            CacheRepresentation::OriginalImage | CacheRepresentation::NormalizedMp4,
            CacheEligibility::ExplicitlyOpenedOriginal,
        ) => {}
        _ => return Err(CacheAliasError::InvalidRecord("eligibility")),
    }
    match record.representation {
        CacheRepresentation::ThumbnailImage | CacheRepresentation::OriginalImage
            if record.content_type.starts_with("image/") => {}
        CacheRepresentation::NormalizedMp4 if record.content_type == "video/mp4" => {}
        _ => return Err(CacheAliasError::InvalidRecord("content_type")),
    }
    Ok(())
}

fn validate_lookup(request: &CacheLookupRequest) -> Result<(), CacheAliasError> {
    if !identifier(&request.pairing_id)
        || !identifier(&request.instance_id)
        || !https_origin(&request.site_origin)
        || !canonical_alias(&request.canonical_alias)
        || !identifier(&request.adapter_id)
        || !semver(&request.adapter_version)
    {
        return Err(CacheAliasError::InvalidRecord("lookup"));
    }
    Ok(())
}

fn validate_authorization(authorization: &CacheLookupAuthorization) -> Result<(), CacheAliasError> {
    if !identifier(&authorization.pairing_id)
        || !identifier(&authorization.actor_id)
        || !identifier(&authorization.instance_id)
        || !https_origin(&authorization.site_origin)
        || !identifier(&authorization.adapter_id)
        || !semver(&authorization.adapter_version)
        || authorization.expires_at_epoch_seconds == 0
    {
        return Err(CacheAliasError::InvalidRecord("authorization"));
    }
    Ok(())
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn https_origin(value: &str) -> bool {
    let Some(host) = value.strip_prefix("https://") else {
        return false;
    };
    !host.is_empty()
        && value.len() <= 512
        && !host.contains(['/', '@', '?', '#', '*', ' ', '\n', '\r'])
}

fn canonical_alias(value: &str) -> bool {
    let Some(host_and_path) = value.strip_prefix("https://") else {
        return false;
    };
    !host_and_path.is_empty()
        && value.len() <= 2_048
        && !host_and_path.contains(['@', '?', '#', ' ', '\n', '\r'])
        && !host_and_path.starts_with('/')
}

fn semver(value: &str) -> bool {
    let mut parts = value.split('.');
    matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(major), Some(minor), Some(patch), None)
            if [major, minor, patch].into_iter().all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    )
}

fn safe_object_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1_024
        && !value.starts_with('/')
        && !value.contains("..")
        && !value.contains("//")
        && !value.contains(['\\', '\0', '\n', '\r'])
}

fn sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    })
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    const CHECKSUM: &str =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn record(index: usize) -> CacheAliasRecord {
        CacheAliasRecord {
            delivery_id: format!("delivery-{index}"),
            instance_id: "ximg-instance-1".into(),
            site_origin: "https://site.example.invalid".into(),
            canonical_alias: format!("https://cdn.example.invalid/media/{index}.jpg"),
            adapter_id: "synthetic-images".into(),
            adapter_version: "1.0.0".into(),
            representation: CacheRepresentation::ThumbnailImage,
            eligibility: CacheEligibility::ObservedThumbnail,
            object: AuthorizedObjectReference {
                endpoint_id: "endpoint-1".into(),
                object_store_id: "store-1".into(),
                object_key: format!("images/{index}.jpg"),
                object_version: 1,
                checksum: CHECKSUM.into(),
            },
            content_type: "image/jpeg".into(),
            content_length: 1024,
            valid_until_epoch_seconds: 2_000,
            availability: CacheObjectAvailability::Ready,
        }
    }

    fn request(index: usize) -> CacheLookupRequest {
        CacheLookupRequest {
            pairing_id: "pair-1".into(),
            instance_id: "ximg-instance-1".into(),
            site_origin: "https://site.example.invalid".into(),
            canonical_alias: format!("https://cdn.example.invalid/media/{index}.jpg"),
            adapter_id: "synthetic-images".into(),
            adapter_version: "1.0.0".into(),
            now_epoch_seconds: 1_000,
        }
    }

    fn authorization() -> CacheLookupAuthorization {
        CacheLookupAuthorization {
            pairing_id: "pair-1".into(),
            actor_id: "actor-1".into(),
            instance_id: "ximg-instance-1".into(),
            site_origin: "https://site.example.invalid".into(),
            adapter_id: "synthetic-images".into(),
            adapter_version: "1.0.0".into(),
            substitution_enabled: true,
            expires_at_epoch_seconds: 2_000,
            revoked: false,
        }
    }

    #[test]
    fn resolves_only_opted_in_same_instance_eligible_immutable_hits() {
        let mut index = CacheAliasIndex::new(4).unwrap();
        assert!(index.admit(record(1)).unwrap());
        let outcome = index.lookup("actor-1", &request(1), &authorization());
        let CacheLookupOutcome::Hit(hit) = outcome else {
            panic!("expected immutable hit");
        };
        assert_eq!(hit.object.object_key, "images/1.jpg");

        let mut paused = authorization();
        paused.substitution_enabled = false;
        assert_eq!(
            index.lookup("actor-1", &request(1), &paused),
            CacheLookupOutcome::OriginFallback(CacheBypassReason::SubstitutionPaused)
        );
        let mut other_instance = request(1);
        other_instance.instance_id = "other-instance".into();
        assert_eq!(
            index.lookup("actor-1", &other_instance, &authorization()),
            CacheLookupOutcome::OriginFallback(CacheBypassReason::PairingInvalid)
        );
    }

    #[test]
    fn rejects_signed_queries_and_conflicting_alias_identity() {
        let mut index = CacheAliasIndex::new(4).unwrap();
        index.admit(record(1)).unwrap();
        let mut signed = record(2);
        signed
            .canonical_alias
            .push_str("?token=must-not-be-retained");
        assert_eq!(
            index.admit(signed),
            Err(CacheAliasError::InvalidRecord("canonical_alias"))
        );

        let mut conflict = record(1);
        conflict.object.object_key = "images/different.jpg".into();
        assert_eq!(
            index.admit(conflict),
            Err(CacheAliasError::ImmutableAliasConflict)
        );

        let mut unopened_original = record(2);
        unopened_original.representation = CacheRepresentation::OriginalImage;
        assert_eq!(
            index.admit(unopened_original),
            Err(CacheAliasError::InvalidRecord("eligibility"))
        );
    }

    #[test]
    fn bounds_memory_invalidates_and_surfaces_authority_unavailability() {
        let mut index = CacheAliasIndex::new(2).unwrap();
        index.admit(record(1)).unwrap();
        index.admit(record(2)).unwrap();
        index.admit(record(3)).unwrap();
        assert_eq!(index.len(), 2);
        assert_eq!(
            index.lookup("actor-1", &request(1), &authorization()),
            CacheLookupOutcome::Miss
        );

        assert!(index.set_availability(
            "https://site.example.invalid",
            "https://cdn.example.invalid/media/2.jpg",
            CHECKSUM,
            CacheObjectAvailability::EndpointOffline,
        ));
        assert_eq!(
            index.lookup("actor-1", &request(2), &authorization()),
            CacheLookupOutcome::OriginFallback(CacheBypassReason::EndpointOffline)
        );
        assert!(index.invalidate(
            "https://site.example.invalid",
            "https://cdn.example.invalid/media/2.jpg"
        ));
        assert_eq!(
            index.lookup("actor-1", &request(2), &authorization()),
            CacheLookupOutcome::Miss
        );
        assert_eq!(index.invalidate_origin("https://site.example.invalid"), 1);
        assert!(index.is_empty());
    }

    #[test]
    fn measured_p95_lookup_stays_within_two_milliseconds() {
        let mut index = CacheAliasIndex::new(4_096).unwrap();
        for item in 0..4_096 {
            index.admit(record(item)).unwrap();
        }
        let requests: Vec<_> = (0..10_000).map(|item| request(item % 4_096)).collect();
        let mut samples = Vec::with_capacity(requests.len());
        for request in &requests {
            let started = Instant::now();
            assert!(matches!(
                index.lookup("actor-1", request, &authorization()),
                CacheLookupOutcome::Hit(_)
            ));
            samples.push(started.elapsed());
        }
        samples.sort_unstable();
        let p95 = samples[samples.len() * 95 / 100];
        eprintln!("4,096-entry/10,000-query cache alias p95: {p95:?}");
        assert!(p95.as_millis() < 2, "p95 alias lookup was {p95:?}");
    }

    #[test]
    fn delivery_revalidates_pairing_and_resolves_the_exact_image_object() {
        let mut index = CacheAliasIndex::new(4).unwrap();
        index.admit(record(1)).unwrap();
        let service = CacheLookupService::new(index, [authorization()]).unwrap();
        let CacheLookupOutcome::Hit(hit) =
            service.authorize_image_delivery("actor-1", "pair-1", "delivery-1", 1_000)
        else {
            panic!("expected authorized image delivery");
        };
        assert_eq!(hit.object.object_key, "images/1.jpg");
        assert_eq!(
            service.authorize_image_delivery("other-actor", "pair-1", "delivery-1", 1_000),
            CacheLookupOutcome::OriginFallback(CacheBypassReason::PairingInvalid)
        );
        assert_eq!(
            service.authorize_image_delivery("actor-1", "pair-1", "missing", 1_000),
            CacheLookupOutcome::Miss
        );
    }

    #[test]
    fn video_delivery_accepts_only_explicitly_opened_normalized_mp4() {
        let mut video = record(2);
        video.canonical_alias = "https://cdn.example.invalid/media/2.mp4".into();
        video.object.object_key = "video/2.mp4".into();
        video.representation = CacheRepresentation::NormalizedMp4;
        video.eligibility = CacheEligibility::ExplicitlyOpenedOriginal;
        video.content_type = "video/mp4".into();
        let mut index = CacheAliasIndex::new(4).unwrap();
        index.admit(video).unwrap();
        index.admit(record(1)).unwrap();
        let service = CacheLookupService::new(index, [authorization()]).unwrap();
        assert!(matches!(
            service.authorize_video_delivery("actor-1", "pair-1", "delivery-2", 1_000),
            CacheLookupOutcome::Hit(_)
        ));
        assert_eq!(
            service.authorize_video_delivery("actor-1", "pair-1", "delivery-1", 1_000),
            CacheLookupOutcome::OriginFallback(CacheBypassReason::NotNormalizedMp4)
        );
    }
}
