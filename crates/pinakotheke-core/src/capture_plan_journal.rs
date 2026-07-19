// SPDX-License-Identifier: MPL-2.0
//! Private, bounded, restart-safe journal for accepted Firefox capture plans.

#![allow(missing_docs)]

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

use crate::viewed_media::{
    AdapterKind, CAPTURE_PLAN_SCHEMA_VERSION, CaptureDestinationSnapshot, CaptureKind, CapturePlan,
    CapturePlanState, canonical_media_url, capture_catalogue_id_for_request,
};

const JOURNAL_SCHEMA: &str = "pinakotheke.capture-plan-journal.v1";
const MAX_JOURNAL_BYTES: u64 = 4 * 1024 * 1024;
const MAX_PLANS: usize = 10_000;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCapturePlan {
    pub actor_id: String,
    pub admitted_at_epoch_seconds: u64,
    pub settled: bool,
    pub plan: CapturePlan,
}

#[derive(Debug)]
pub enum CapturePlanJournalError {
    Io(io::Error),
    Json(serde_json::Error),
    UnsupportedSchema,
    InvalidRecord,
    TooLarge,
}

impl std::fmt::Display for CapturePlanJournalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "capture-plan journal rejected: {self:?}")
    }
}

impl std::error::Error for CapturePlanJournalError {}

impl From<io::Error> for CapturePlanJournalError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for CapturePlanJournalError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Debug, Clone)]
pub struct CapturePlanJournal {
    path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalDocument {
    schema_version: String,
    plans: Vec<StoredPendingPlan>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPendingPlan {
    actor_id: String,
    admitted_at_epoch_seconds: u64,
    #[serde(default)]
    settled: bool,
    plan: StoredCapturePlan,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredCapturePlan {
    schema_version: String,
    plan_id: String,
    scheduler_job_id: String,
    site_id: String,
    origin: String,
    canonical_page_url: String,
    canonical_media_url: String,
    #[serde(default)]
    retrieval_media_url: Option<String>,
    #[serde(default)]
    destination: Option<CaptureDestinationSnapshot>,
    #[serde(default)]
    canonical_presentation_url: Option<String>,
    #[serde(default)]
    creator_hint: Option<String>,
    #[serde(default)]
    catalogue_id: Option<String>,
    adapter_kind: AdapterKind,
    adapter_version: String,
    capture_kind: CaptureKind,
    width: u32,
    height: u32,
    state: CapturePlanState,
}

impl CapturePlanJournal {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Vec<PendingCapturePlan>, CapturePlanJournalError> {
        let metadata = match fs::symlink_metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "capture-plan journal must be a regular file",
            )
            .into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "capture-plan journal must be private",
                )
                .into());
            }
        }
        if metadata.len() > MAX_JOURNAL_BYTES {
            return Err(CapturePlanJournalError::TooLarge);
        }
        let document: JournalDocument = serde_json::from_slice(&fs::read(&self.path)?)?;
        if document.schema_version != JOURNAL_SCHEMA {
            return Err(CapturePlanJournalError::UnsupportedSchema);
        }
        if document.plans.len() > MAX_PLANS {
            return Err(CapturePlanJournalError::TooLarge);
        }
        let mut identities = std::collections::BTreeSet::new();
        document
            .plans
            .into_iter()
            .map(|stored| {
                let pending = stored.into_pending()?;
                if !identities.insert(pending.plan.plan_id.clone()) {
                    return Err(CapturePlanJournalError::InvalidRecord);
                }
                Ok(pending)
            })
            .collect()
    }

    pub fn replace(&self, plans: &[PendingCapturePlan]) -> Result<(), CapturePlanJournalError> {
        if plans.len() > MAX_PLANS {
            return Err(CapturePlanJournalError::TooLarge);
        }
        let document = JournalDocument {
            schema_version: JOURNAL_SCHEMA.into(),
            plans: plans.iter().map(StoredPendingPlan::from).collect(),
        };
        let mut bytes = serde_json::to_vec_pretty(&document)?;
        bytes.push(b'\n');
        if bytes.len() as u64 > MAX_JOURNAL_BYTES {
            return Err(CapturePlanJournalError::TooLarge);
        }
        let parent = self.path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "journal path requires a parent",
            )
        })?;
        fs::create_dir_all(parent)?;
        if let Ok(metadata) = fs::symlink_metadata(&self.path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "capture-plan journal target must be a regular file",
            )
            .into());
        }
        let temporary = parent.join(format!(
            ".capture-plan-journal.{}.{}.tmp",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| -> io::Result<()> {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temporary, &self.path)?;
            #[cfg(unix)]
            fs::File::open(parent)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.map_err(Into::into)
    }
}

impl StoredPendingPlan {
    fn into_pending(self) -> Result<PendingCapturePlan, CapturePlanJournalError> {
        if !identifier(&self.actor_id) || self.admitted_at_epoch_seconds == 0 {
            return Err(CapturePlanJournalError::InvalidRecord);
        }
        Ok(PendingCapturePlan {
            actor_id: self.actor_id,
            admitted_at_epoch_seconds: self.admitted_at_epoch_seconds,
            settled: self.settled,
            plan: self.plan.into_plan()?,
        })
    }
}

impl StoredCapturePlan {
    fn into_plan(self) -> Result<CapturePlan, CapturePlanJournalError> {
        let retrieval_media_url = self
            .retrieval_media_url
            .unwrap_or_else(|| self.canonical_media_url.clone());
        let canonical_presentation_url = self
            .canonical_presentation_url
            .unwrap_or_else(|| self.canonical_media_url.clone());
        if self.schema_version != CAPTURE_PLAN_SCHEMA_VERSION
            || !identifier(&self.plan_id)
            || self
                .plan_id
                .strip_prefix("capture-plan-")
                .and_then(|suffix| suffix.parse::<u64>().ok())
                .is_none()
            || !identifier(&self.scheduler_job_id)
            || !identifier(&self.site_id)
            || self.width == 0
            || self.height == 0
            || self.width > 32_768
            || self.height > 32_768
            || self.state != CapturePlanState::AwaitingApprovedAcquisition
            || !https_origin(&self.origin)
            || !https_url(&self.canonical_page_url)
            || canonical_media_url(&self.canonical_media_url).as_deref()
                != Some(self.canonical_media_url.as_str())
            || super::viewed_media::retrieval_media_url(&retrieval_media_url).as_deref()
                != Some(retrieval_media_url.as_str())
            || canonical_media_url(&canonical_presentation_url).as_deref()
                != Some(canonical_presentation_url.as_str())
            || !(self.canonical_page_url == self.origin
                || self
                    .canonical_page_url
                    .starts_with(&format!("{}/", self.origin)))
            || !semver(&self.adapter_version)
            || self
                .destination
                .as_ref()
                .is_some_and(|destination| !destination.is_valid())
        {
            return Err(CapturePlanJournalError::InvalidRecord);
        }
        let catalogue_id = self.catalogue_id.unwrap_or_else(|| {
            // Historic image records must retain their page/presentation ID
            // until the guarded gallery reconciliation updates both metadata
            // documents together. Historic video records regain the suffix
            // that was present at their original admission.
            if self.capture_kind == CaptureKind::ExplicitVideo {
                capture_catalogue_id_for_request(
                    &self.site_id,
                    &self.canonical_page_url,
                    &canonical_presentation_url,
                    &self.canonical_media_url,
                    self.capture_kind,
                )
            } else {
                crate::viewed_media::capture_catalogue_id(
                    &self.site_id,
                    &self.canonical_page_url,
                    &canonical_presentation_url,
                )
            }
        });
        if !identifier(&catalogue_id) {
            return Err(CapturePlanJournalError::InvalidRecord);
        }
        Ok(CapturePlan {
            schema_version: CAPTURE_PLAN_SCHEMA_VERSION,
            plan_id: self.plan_id,
            scheduler_job_id: self.scheduler_job_id,
            site_id: self.site_id,
            origin: self.origin,
            canonical_page_url: self.canonical_page_url,
            canonical_media_url: self.canonical_media_url,
            retrieval_media_url,
            destination: self.destination,
            canonical_presentation_url,
            creator_hint: self.creator_hint,
            catalogue_id,
            adapter_kind: self.adapter_kind,
            adapter_version: self.adapter_version,
            capture_kind: self.capture_kind,
            width: self.width,
            height: self.height,
            state: self.state,
        })
    }
}

impl From<&PendingCapturePlan> for StoredPendingPlan {
    fn from(pending: &PendingCapturePlan) -> Self {
        let plan = &pending.plan;
        Self {
            actor_id: pending.actor_id.clone(),
            admitted_at_epoch_seconds: pending.admitted_at_epoch_seconds,
            settled: pending.settled,
            plan: StoredCapturePlan {
                schema_version: plan.schema_version.into(),
                plan_id: plan.plan_id.clone(),
                scheduler_job_id: plan.scheduler_job_id.clone(),
                site_id: plan.site_id.clone(),
                origin: plan.origin.clone(),
                canonical_page_url: plan.canonical_page_url.clone(),
                canonical_media_url: plan.canonical_media_url.clone(),
                retrieval_media_url: Some(plan.retrieval_media_url.clone()),
                destination: plan.destination.clone(),
                canonical_presentation_url: Some(plan.canonical_presentation_url.clone()),
                creator_hint: plan.creator_hint.clone(),
                catalogue_id: Some(plan.catalogue_id.clone()),
                adapter_kind: plan.adapter_kind,
                adapter_version: plan.adapter_version.clone(),
                capture_kind: plan.capture_kind,
                width: plan.width,
                height: plan.height,
                state: plan.state,
            },
        }
    }
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn https_url(value: &str) -> bool {
    value.len() <= 2_048
        && value.starts_with("https://")
        && !value.contains([' ', '\n', '\r', '@', '#', '?'])
}

fn https_origin(value: &str) -> bool {
    https_url(value) && !value[8..].contains('/')
}

fn semver(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending() -> PendingCapturePlan {
        let catalogue_id = crate::viewed_media::capture_catalogue_id(
            "site-1",
            "https://example.invalid/page",
            "https://media.example.invalid/thumb.jpg",
        );
        PendingCapturePlan {
            actor_id: "actor-1".into(),
            admitted_at_epoch_seconds: 42,
            settled: false,
            plan: CapturePlan {
                schema_version: CAPTURE_PLAN_SCHEMA_VERSION,
                plan_id: "capture-plan-0".into(),
                scheduler_job_id: "refresh-0".into(),
                site_id: "site-1".into(),
                origin: "https://example.invalid".into(),
                canonical_page_url: "https://example.invalid/page".into(),
                canonical_media_url: "https://media.example.invalid/thumb.jpg".into(),
                retrieval_media_url: "https://media.example.invalid/thumb.jpg".into(),
                destination: Some(CaptureDestinationSnapshot {
                    endpoint_id: "endpoint-1".into(),
                    object_store_id: "store-1".into(),
                    selection_revision: 7,
                }),
                canonical_presentation_url: "https://media.example.invalid/thumb.jpg".into(),
                creator_hint: None,
                catalogue_id,
                adapter_kind: AdapterKind::ExperimentalGeneric,
                adapter_version: "1.0.0".into(),
                capture_kind: CaptureKind::ObservedThumbnail,
                width: 320,
                height: 200,
                state: CapturePlanState::AwaitingApprovedAcquisition,
            },
        }
    }

    #[test]
    fn atomically_round_trips_metadata_without_payload_fields() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-capture-journal-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let journal = CapturePlanJournal::new(root.join("journal.json"));
        journal.replace(&[pending()]).unwrap();
        assert_eq!(journal.load().unwrap(), vec![pending()]);
        let text = fs::read_to_string(journal.path()).unwrap();
        for prohibited in ["cookie", "authorization", "payload", "password"] {
            assert!(!text.contains(prohibited));
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_journal_without_presentation_identity_migrates_non_destructively() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-capture-journal-legacy-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let journal = CapturePlanJournal::new(root.join("journal.json"));
        journal.replace(&[pending()]).unwrap();
        let mut document: serde_json::Value =
            serde_json::from_slice(&fs::read(journal.path()).unwrap()).unwrap();
        document["plans"][0]["plan"]
            .as_object_mut()
            .unwrap()
            .remove("canonical_presentation_url");
        document["plans"][0]["plan"]
            .as_object_mut()
            .unwrap()
            .remove("catalogue_id");
        fs::write(
            journal.path(),
            serde_json::to_vec_pretty(&document).unwrap(),
        )
        .unwrap();
        let loaded = journal.load().unwrap();
        assert_eq!(
            loaded[0].plan.canonical_presentation_url,
            loaded[0].plan.canonical_media_url
        );
        assert_eq!(
            loaded[0].plan.catalogue_id,
            crate::viewed_media::capture_catalogue_id(
                &loaded[0].plan.site_id,
                &loaded[0].plan.canonical_page_url,
                &loaded[0].plan.canonical_media_url,
            )
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_journal_without_destination_is_loaded_explicitly_unbound() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-capture-journal-unbound-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let journal = CapturePlanJournal::new(root.join("journal.json"));
        journal.replace(&[pending()]).unwrap();
        let mut document: serde_json::Value =
            serde_json::from_slice(&fs::read(journal.path()).unwrap()).unwrap();
        document["plans"][0]["plan"]
            .as_object_mut()
            .unwrap()
            .remove("destination");
        fs::write(
            journal.path(),
            serde_json::to_vec_pretty(&document).unwrap(),
        )
        .unwrap();

        assert_eq!(journal.load().unwrap()[0].plan.destination, None);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_video_journal_reconstructs_its_original_suffixed_card_identity() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-capture-journal-video-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let journal = CapturePlanJournal::new(root.join("journal.json"));
        journal.replace(&[pending()]).unwrap();
        let mut document: serde_json::Value =
            serde_json::from_slice(&fs::read(journal.path()).unwrap()).unwrap();
        let plan = document["plans"][0]["plan"].as_object_mut().unwrap();
        plan.remove("catalogue_id");
        plan.insert("capture_kind".into(), serde_json::json!("explicit_video"));
        plan.insert(
            "canonical_media_url".into(),
            serde_json::json!("https://video.example.invalid/fixture.mp4"),
        );
        plan.insert(
            "retrieval_media_url".into(),
            serde_json::json!("https://video.example.invalid/fixture.mp4"),
        );
        fs::write(
            journal.path(),
            serde_json::to_vec_pretty(&document).unwrap(),
        )
        .unwrap();

        let loaded = journal.load().unwrap();
        let expected = capture_catalogue_id_for_request(
            &loaded[0].plan.site_id,
            &loaded[0].plan.canonical_page_url,
            &loaded[0].plan.canonical_presentation_url,
            &loaded[0].plan.canonical_media_url,
            CaptureKind::ExplicitVideo,
        );
        assert_eq!(loaded[0].plan.catalogue_id, expected);
        assert!(loaded[0].plan.catalogue_id.contains("-video-"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn journal_rejects_invalid_destination_identity_or_revision() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-capture-journal-destination-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let journal = CapturePlanJournal::new(root.join("journal.json"));
        journal.replace(&[pending()]).unwrap();
        let original = fs::read(journal.path()).unwrap();
        for (field, value) in [
            ("endpoint_id", serde_json::json!("unsafe/id")),
            ("object_store_id", serde_json::json!("")),
            ("selection_revision", serde_json::json!(0)),
        ] {
            let mut document: serde_json::Value = serde_json::from_slice(&original).unwrap();
            document["plans"][0]["plan"]["destination"][field] = value;
            fs::write(
                journal.path(),
                serde_json::to_vec_pretty(&document).unwrap(),
            )
            .unwrap();
            assert!(matches!(
                journal.load(),
                Err(CapturePlanJournalError::InvalidRecord)
            ));
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn future_corrupt_and_symlinked_journals_fail_closed() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-capture-journal-invalid-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("journal.json");
        fs::write(&path, r#"{"schema_version":"future.v2","plans":[]}"#).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert!(matches!(
            CapturePlanJournal::new(&path).load(),
            Err(CapturePlanJournalError::UnsupportedSchema)
        ));
        fs::write(&path, b"not-json").unwrap();
        assert!(matches!(
            CapturePlanJournal::new(&path).load(),
            Err(CapturePlanJournalError::Json(_))
        ));
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let target = root.join("target.json");
            fs::rename(&path, &target).unwrap();
            symlink(&target, &path).unwrap();
            assert!(matches!(
                CapturePlanJournal::new(&path).load(),
                Err(CapturePlanJournalError::Io(_))
            ));
        }
        let _ = fs::remove_dir_all(root);
    }
}
