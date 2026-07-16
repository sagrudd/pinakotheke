// SPDX-License-Identifier: MPL-2.0
//! Direct, authorized normalized-video delivery independent of site caching.
//!
//! This boundary maps one host-authorized playback grant to the existing
//! DASObjectStore read port. It does not fetch an origin URL, retain bytes, or
//! make a source-only object playable.

#![allow(missing_docs)]

use std::collections::BTreeMap;

use crate::{
    object_read::{
        AuthorizedObjectReader, AuthorizedObjectReference, ByteRange, ObjectReadBackend,
        ObjectReadError, ObjectReadRequest, ValidatedObjectRead,
    },
    video_profile::NormalizedVideoState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectPlaybackGrant {
    pub playback_id: String,
    pub actor_id: String,
    pub object: AuthorizedObjectReference,
    pub total_length: u64,
    pub state: NormalizedVideoState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackHeaders {
    pub content_type: String,
    pub content_length: u64,
    pub total_length: u64,
    pub etag: String,
    pub accept_ranges: bool,
    pub content_range: Option<ByteRange>,
}

pub enum DirectPlaybackResponse<S> {
    Content {
        partial: bool,
        headers: PlaybackHeaders,
        stream: S,
    },
    NotModified {
        etag: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectPlaybackError {
    InvalidRange,
    UnknownPlayback,
    Forbidden,
    NotReady,
    Read(ObjectReadError),
}

impl std::fmt::Display for DirectPlaybackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRange => "requested range is invalid or unsupported",
            Self::UnknownPlayback => "playback object is not known",
            Self::Forbidden => "playback actor is not authorized for this object",
            Self::NotReady => "normalized video is not ready for direct playback",
            Self::Read(error) => {
                return write!(formatter, "authorized object read failed: {error}");
            }
        })
    }
}

impl std::error::Error for DirectPlaybackError {}

pub struct DirectPlaybackService<B: ObjectReadBackend> {
    reader: AuthorizedObjectReader<B>,
    grants: BTreeMap<String, DirectPlaybackGrant>,
}

impl<B: ObjectReadBackend> DirectPlaybackService<B> {
    pub fn new(
        reader: AuthorizedObjectReader<B>,
        grants: impl IntoIterator<Item = DirectPlaybackGrant>,
    ) -> Self {
        Self {
            reader,
            grants: grants
                .into_iter()
                .map(|grant| (grant.playback_id.clone(), grant))
                .collect(),
        }
    }

    pub fn deliver(
        &mut self,
        actor_id: &str,
        playback_id: &str,
        range_header: Option<&str>,
        if_none_match: Option<&str>,
    ) -> Result<DirectPlaybackResponse<B::Stream>, DirectPlaybackError> {
        let grant = self
            .grants
            .get(playback_id)
            .ok_or(DirectPlaybackError::UnknownPlayback)?;
        if grant.actor_id != actor_id {
            return Err(DirectPlaybackError::Forbidden);
        }
        if grant.state != NormalizedVideoState::Ready {
            return Err(DirectPlaybackError::NotReady);
        }
        let range = range_header
            .map(|header| parse_single_range(header, grant.total_length))
            .transpose()?;
        match self
            .reader
            .open(&ObjectReadRequest {
                object: grant.object.clone(),
                range,
                if_none_match_etag: if_none_match.map(str::to_owned),
            })
            .map_err(DirectPlaybackError::Read)?
        {
            ValidatedObjectRead::Content { metadata, stream } => {
                Ok(DirectPlaybackResponse::Content {
                    partial: range.is_some(),
                    headers: PlaybackHeaders {
                        content_type: metadata.content_type,
                        content_length: metadata.content_length,
                        total_length: metadata.total_length,
                        etag: metadata.etag,
                        accept_ranges: true,
                        content_range: metadata.content_range,
                    },
                    stream,
                })
            }
            ValidatedObjectRead::NotModified { etag } => {
                Ok(DirectPlaybackResponse::NotModified { etag })
            }
        }
    }

    pub fn into_reader(self) -> AuthorizedObjectReader<B> {
        self.reader
    }
}

/// Parses exactly one RFC 7233 byte range. Multiple ranges are deliberately
/// rejected: media playback needs one predictable bounded authority stream,
/// not multipart response assembly.
pub fn parse_single_range(
    value: &str,
    total_length: u64,
) -> Result<ByteRange, DirectPlaybackError> {
    let value = value
        .strip_prefix("bytes=")
        .ok_or(DirectPlaybackError::InvalidRange)?;
    if total_length == 0 || value.contains(',') || value.is_empty() {
        return Err(DirectPlaybackError::InvalidRange);
    }
    let (start, end) = value
        .split_once('-')
        .ok_or(DirectPlaybackError::InvalidRange)?;
    if start.is_empty() {
        return Err(DirectPlaybackError::InvalidRange);
    }
    let start = start
        .parse::<u64>()
        .map_err(|_| DirectPlaybackError::InvalidRange)?;
    if start >= total_length {
        return Err(DirectPlaybackError::InvalidRange);
    }
    let end_inclusive = if end.is_empty() {
        total_length.saturating_sub(1)
    } else {
        end.parse::<u64>()
            .map_err(|_| DirectPlaybackError::InvalidRange)?
            .min(total_length.saturating_sub(1))
    };
    (start <= end_inclusive)
        .then_some(ByteRange {
            start,
            end_inclusive,
        })
        .ok_or(DirectPlaybackError::InvalidRange)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object_read::{ObjectContentMetadata, ObjectReadBackendError, ObjectReadResult};

    const CHECKSUM: &str =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    struct Backend {
        response: Option<Result<ObjectReadResult<()>, ObjectReadBackendError>>,
    }

    impl ObjectReadBackend for Backend {
        type Stream = ();

        fn open(
            &mut self,
            _: &ObjectReadRequest,
        ) -> Result<ObjectReadResult<Self::Stream>, ObjectReadBackendError> {
            self.response.take().expect("one fixture response")
        }
    }

    fn grant(state: NormalizedVideoState) -> DirectPlaybackGrant {
        DirectPlaybackGrant {
            playback_id: "playback-1".into(),
            actor_id: "actor-1".into(),
            object: AuthorizedObjectReference {
                endpoint_id: "endpoint".into(),
                object_store_id: "store".into(),
                object_key: "video/normalized.mp4".into(),
                object_version: 1,
                checksum: CHECKSUM.into(),
            },
            total_length: 10,
            state,
        }
    }

    fn metadata(range: Option<ByteRange>) -> ObjectContentMetadata {
        let length = range.map_or(10, |range| range.end_inclusive - range.start + 1);
        ObjectContentMetadata {
            content_type: "video/mp4".into(),
            content_length: length,
            total_length: 10,
            checksum: CHECKSUM.into(),
            etag: format!("\"{CHECKSUM}\""),
            content_range: range,
        }
    }

    #[test]
    fn resolves_open_ended_ranges_to_one_authorized_partial_response() {
        let range = ByteRange {
            start: 3,
            end_inclusive: 9,
        };
        let reader = AuthorizedObjectReader::new(Backend {
            response: Some(Ok(ObjectReadResult::Content {
                metadata: metadata(Some(range)),
                stream: (),
            })),
        });
        let mut service = DirectPlaybackService::new(reader, [grant(NormalizedVideoState::Ready)]);
        let response = service
            .deliver("actor-1", "playback-1", Some("bytes=3-"), None)
            .expect("partial delivery");
        assert!(matches!(
            response,
            DirectPlaybackResponse::Content {
                partial: true,
                headers: PlaybackHeaders { accept_ranges: true, content_range: Some(actual), .. },
                ..
            } if actual == range
        ));
    }

    #[test]
    fn denies_source_only_wrong_actor_and_multi_range_requests() {
        let reader = AuthorizedObjectReader::new(Backend { response: None });
        let mut service = DirectPlaybackService::new(
            reader,
            [grant(NormalizedVideoState::AwaitingFirefoxPlayback)],
        );
        assert!(matches!(
            service.deliver("actor-1", "playback-1", None, None),
            Err(DirectPlaybackError::NotReady)
        ));

        let reader = AuthorizedObjectReader::new(Backend { response: None });
        let mut service = DirectPlaybackService::new(reader, [grant(NormalizedVideoState::Ready)]);
        assert!(matches!(
            service.deliver("other", "playback-1", None, None),
            Err(DirectPlaybackError::Forbidden)
        ));
        assert!(matches!(
            service.deliver("actor-1", "playback-1", Some("bytes=0-1,3-4"), None),
            Err(DirectPlaybackError::InvalidRange)
        ));
    }

    #[test]
    fn parses_bounded_and_open_ended_ranges() {
        assert_eq!(
            parse_single_range("bytes=0-3", 10),
            Ok(ByteRange {
                start: 0,
                end_inclusive: 3
            })
        );
        assert_eq!(
            parse_single_range("bytes=8-", 10),
            Ok(ByteRange {
                start: 8,
                end_inclusive: 9
            })
        );
        assert_eq!(
            parse_single_range("bytes=-2", 10),
            Err(DirectPlaybackError::InvalidRange)
        );
    }
}
