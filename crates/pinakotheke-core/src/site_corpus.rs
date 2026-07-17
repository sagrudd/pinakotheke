// SPDX-License-Identifier: MPL-2.0
//! Strict actor-scoped website import definitions persisted by Pinakotheke.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
use url::Url;

pub const SITE_CORPUS_SCHEMA: &str = "pinakotheke.site-corpus.v1";
const DOCUMENT_SCHEMA: &str = "pinakotheke.site-corpus-store.v1";
const MAX_BYTES: u64 = 1024 * 1024;
const MAX_ACTORS: usize = 256;
const MAX_RULES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteRule {
    pub origin: String,
    pub images: bool,
    pub videos: bool,
    pub capture: bool,
    pub substitution: bool,
    pub x_ingress: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorSiteCorpus {
    pub schema_version: String,
    pub revision: u64,
    pub rules: Vec<SiteRule>,
    pub tombstones: Vec<SiteTombstone>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteTombstone {
    pub origin: String,
    pub deleted_at_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaceSiteCorpus {
    pub schema_version: String,
    pub expected_revision: u64,
    pub rules: Vec<SiteRule>,
}

#[derive(Debug)]
pub enum SiteCorpusError {
    Io(io::Error),
    Json(serde_json::Error),
    UnsupportedSchema,
    Invalid,
    TooLarge,
    Conflict(ActorSiteCorpus),
}
impl std::fmt::Display for SiteCorpusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "site corpus rejected: {self:?}")
    }
}
impl std::error::Error for SiteCorpusError {}
impl From<io::Error> for SiteCorpusError {
    fn from(v: io::Error) -> Self {
        Self::Io(v)
    }
}
impl From<serde_json::Error> for SiteCorpusError {
    fn from(v: serde_json::Error) -> Self {
        Self::Json(v)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: String,
    actors: BTreeMap<String, ActorSiteCorpus>,
}

#[derive(Debug, Clone)]
pub struct SiteCorpusStore {
    path: PathBuf,
}

impl SiteCorpusStore {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get(&self, actor: &str) -> Result<ActorSiteCorpus, SiteCorpusError> {
        validate_actor(actor)?;
        Ok(self
            .load()?
            .actors
            .remove(actor)
            .unwrap_or_else(empty_corpus))
    }

    pub fn replace(
        &self,
        actor: &str,
        requested: ReplaceSiteCorpus,
    ) -> Result<ActorSiteCorpus, SiteCorpusError> {
        validate_actor(actor)?;
        if requested.schema_version != SITE_CORPUS_SCHEMA {
            return Err(SiteCorpusError::UnsupportedSchema);
        }
        validate_rules(&requested.rules)?;
        let mut document = self.load()?;
        let current = document
            .actors
            .get(actor)
            .cloned()
            .unwrap_or_else(empty_corpus);
        if current.revision != requested.expected_revision {
            return Err(SiteCorpusError::Conflict(current));
        }
        let next_revision = current
            .revision
            .checked_add(1)
            .ok_or(SiteCorpusError::Invalid)?;
        let requested_origins: BTreeSet<_> = requested
            .rules
            .iter()
            .map(|rule| rule.origin.as_str())
            .collect();
        let current_origins: BTreeSet<_> = current
            .rules
            .iter()
            .map(|rule| rule.origin.as_str())
            .collect();
        let mut tombstones: Vec<_> = current
            .tombstones
            .into_iter()
            .filter(|item| !requested_origins.contains(item.origin.as_str()))
            .collect();
        for origin in current_origins.difference(&requested_origins) {
            tombstones.retain(|item| item.origin != **origin);
            tombstones.push(SiteTombstone {
                origin: (*origin).to_owned(),
                deleted_at_revision: next_revision,
            });
        }
        tombstones.sort_by_key(|item| item.deleted_at_revision);
        if tombstones.len() > MAX_RULES {
            tombstones.drain(..tombstones.len() - MAX_RULES);
        }
        let next = ActorSiteCorpus {
            schema_version: SITE_CORPUS_SCHEMA.into(),
            revision: next_revision,
            rules: requested.rules,
            tombstones,
        };
        document.actors.insert(actor.into(), next.clone());
        if document.actors.len() > MAX_ACTORS {
            return Err(SiteCorpusError::TooLarge);
        }
        self.save(&document)?;
        Ok(next)
    }

    fn load(&self) -> Result<Document, SiteCorpusError> {
        let metadata = match fs::symlink_metadata(&self.path) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Document {
                    schema_version: DOCUMENT_SCHEMA.into(),
                    actors: BTreeMap::new(),
                });
            }
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(SiteCorpusError::Invalid);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(SiteCorpusError::Invalid);
            }
        }
        if metadata.len() > MAX_BYTES {
            return Err(SiteCorpusError::TooLarge);
        }
        let document: Document = serde_json::from_slice(&fs::read(&self.path)?)?;
        if document.schema_version != DOCUMENT_SCHEMA {
            return Err(SiteCorpusError::UnsupportedSchema);
        }
        if document.actors.len() > MAX_ACTORS {
            return Err(SiteCorpusError::TooLarge);
        }
        for (actor, corpus) in &document.actors {
            validate_actor(actor)?;
            if corpus.schema_version != SITE_CORPUS_SCHEMA {
                return Err(SiteCorpusError::UnsupportedSchema);
            }
            validate_rules(&corpus.rules)?;
            if corpus.tombstones.len() > MAX_RULES
                || corpus.tombstones.iter().any(|item| {
                    item.deleted_at_revision == 0
                        || item.deleted_at_revision > corpus.revision
                        || Url::parse(&item.origin).map_or(true, |url| {
                            url.origin().ascii_serialization() != item.origin
                        })
                })
            {
                return Err(SiteCorpusError::Invalid);
            }
        }
        Ok(document)
    }

    fn save(&self, document: &Document) -> Result<(), SiteCorpusError> {
        let mut bytes = serde_json::to_vec_pretty(document)?;
        bytes.push(b'\n');
        if bytes.len() as u64 > MAX_BYTES {
            return Err(SiteCorpusError::TooLarge);
        }
        let parent = self.path.parent().ok_or(SiteCorpusError::Invalid)?;
        fs::create_dir_all(parent)?;
        let temp = parent.join(format!(".site-corpus.{}.tmp", std::process::id()));
        let mut options = OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temp, &self.path)?;
        #[cfg(unix)]
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    }
}

fn empty_corpus() -> ActorSiteCorpus {
    ActorSiteCorpus {
        schema_version: SITE_CORPUS_SCHEMA.into(),
        revision: 0,
        rules: Vec::new(),
        tombstones: Vec::new(),
    }
}
fn validate_actor(actor: &str) -> Result<(), SiteCorpusError> {
    if actor.is_empty()
        || actor.len() > 128
        || !actor
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '@' | ':'))
    {
        Err(SiteCorpusError::Invalid)
    } else {
        Ok(())
    }
}
fn validate_rules(rules: &[SiteRule]) -> Result<(), SiteCorpusError> {
    if rules.len() > MAX_RULES {
        return Err(SiteCorpusError::TooLarge);
    }
    let mut origins = BTreeSet::new();
    for rule in rules {
        let url = Url::parse(&rule.origin).map_err(|_| SiteCorpusError::Invalid)?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || url.username() != ""
            || url.password().is_some()
            || !origins.insert(rule.origin.clone())
            || (!rule.images && !rule.videos)
        {
            return Err(SiteCorpusError::Invalid);
        }
        if url.origin().ascii_serialization() != rule.origin {
            return Err(SiteCorpusError::Invalid);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persists_per_actor_and_rejects_stale_write() {
        let root = std::env::temp_dir().join(format!("pinakotheke-corpus-{}", std::process::id()));
        let path = root.join("corpus.json");
        let store = SiteCorpusStore::new(&path);
        let request = ReplaceSiteCorpus {
            schema_version: SITE_CORPUS_SCHEMA.into(),
            expected_revision: 0,
            rules: vec![SiteRule {
                origin: "https://x.com".into(),
                images: true,
                videos: true,
                capture: true,
                substitution: true,
                x_ingress: true,
            }],
        };
        let saved = store.replace("actor-1", request.clone()).unwrap();
        assert_eq!(saved.revision, 1);
        assert_eq!(store.get("actor-1").unwrap(), saved);
        assert_eq!(store.get("actor-2").unwrap().revision, 0);
        assert!(
            matches!(store.replace("actor-1", request), Err(SiteCorpusError::Conflict(current)) if current.revision == 1)
        );
        let deleted = store
            .replace(
                "actor-1",
                ReplaceSiteCorpus {
                    schema_version: SITE_CORPUS_SCHEMA.into(),
                    expected_revision: 1,
                    rules: Vec::new(),
                },
            )
            .unwrap();
        assert_eq!(deleted.tombstones[0].origin, "https://x.com");
        assert_eq!(deleted.tombstones[0].deleted_at_revision, 2);
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn rejects_non_origin_and_duplicate_rules() {
        let rules = vec![SiteRule {
            origin: "https://x.com/home".into(),
            images: true,
            videos: false,
            capture: true,
            substitution: false,
            x_ingress: true,
        }];
        assert!(validate_rules(&rules).is_err());
    }
}
