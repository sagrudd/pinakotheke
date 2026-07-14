// SPDX-License-Identifier: MPL-2.0
//! Review-queue admission after verified catalogue commit only.

use crate::acquisition::{Acquisition, AcquisitionState, ReviewState, VerifiedObject};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewReviewItem {
    pub media_identity_id: String,
    pub source_id: String,
    pub discovery_time_unix_seconds: u64,
    pub object: VerifiedObject,
    pub review_state: ReviewState,
}
#[derive(Debug, Default)]
pub struct ReviewQueue {
    items: BTreeMap<String, NewReviewItem>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewAdmissionError {
    NotCommitted,
    InvalidSource,
}
impl std::fmt::Display for ReviewAdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "review admission rejected: {self:?}")
    }
}
impl std::error::Error for ReviewAdmissionError {}
impl ReviewQueue {
    /// Admits one committed, verified object as `New`; replays retain the first record.
    pub fn admit_new(
        &mut self,
        acquisition: &Acquisition,
        source_id: impl Into<String>,
        discovery_time_unix_seconds: u64,
    ) -> Result<&NewReviewItem, ReviewAdmissionError> {
        let source_id = source_id.into();
        if !safe(&source_id) {
            return Err(ReviewAdmissionError::InvalidSource);
        }
        if acquisition.state() != AcquisitionState::Committed {
            return Err(ReviewAdmissionError::NotCommitted);
        }
        let object = acquisition
            .verified_object()
            .cloned()
            .ok_or(ReviewAdmissionError::NotCommitted)?;
        let key = acquisition.media_identity_id().to_owned();
        self.items.entry(key.clone()).or_insert(NewReviewItem {
            media_identity_id: key.clone(),
            source_id,
            discovery_time_unix_seconds,
            object,
            review_state: ReviewState::New,
        });
        Ok(self.items.get(&key).expect("inserted"))
    }
    #[must_use]
    pub fn get(&self, media_identity_id: &str) -> Option<&NewReviewItem> {
        self.items.get(media_identity_id)
    }
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }
}
fn safe(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | ':' | '-')
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::acquisition::VerifiedObject;
    const SUM: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    fn committed() -> Acquisition {
        let mut a = Acquisition::discovered("x:account:post:media").unwrap();
        a.claim("worker").unwrap();
        a.start_transfer().unwrap();
        a.record_stored().unwrap();
        a.verify(VerifiedObject::new("endpoint", "store", "object", SUM).unwrap())
            .unwrap();
        a.commit().unwrap();
        a
    }
    #[test]
    fn only_verified_commit_enters_new_queue() {
        let mut q = ReviewQueue::default();
        let a = Acquisition::discovered("x:account:post:media").unwrap();
        assert_eq!(
            q.admit_new(&a, "x:account", 1),
            Err(ReviewAdmissionError::NotCommitted)
        );
        let a = committed();
        let item = q.admit_new(&a, "x:account", 7).unwrap();
        assert_eq!(item.review_state, ReviewState::New);
        assert_eq!(item.discovery_time_unix_seconds, 7);
        assert_eq!(q.len(), 1);
        q.admit_new(&a, "x:account", 8).unwrap();
        assert_eq!(q.len(), 1);
    }
}
