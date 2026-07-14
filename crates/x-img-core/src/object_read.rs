// SPDX-License-Identifier: MPL-2.0
//! Authorized DASObjectStore object-read and cache-handoff contract.
//!
//! x-img validates authority metadata then hands a stream back to its caller.
//! It never writes the stream to a local cache, product root, database, or log.

#![allow(missing_docs)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedObjectReference {
    pub endpoint_id: String,
    pub object_store_id: String,
    pub object_key: String,
    pub checksum: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub start: u64,
    pub end_inclusive: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectReadRequest {
    pub object: AuthorizedObjectReference,
    pub range: Option<ByteRange>,
    pub if_none_match_etag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectContentMetadata {
    pub content_type: String,
    pub content_length: u64,
    pub total_length: u64,
    pub checksum: String,
    pub etag: String,
    pub content_range: Option<ByteRange>,
}

pub enum ObjectReadResult<S> {
    Content {
        metadata: ObjectContentMetadata,
        stream: S,
    },
    NotModified {
        etag: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectUnavailable {
    NotFound,
    AccessDenied,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectReadBackendError {
    Unavailable(ObjectUnavailable),
    Rejected(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectReadError {
    InvalidRequest(String),
    Backend(ObjectReadBackendError),
    MetadataMismatch(String),
}

impl std::fmt::Display for ObjectReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(message) => {
                write!(formatter, "invalid object read request: {message}")
            }
            Self::Backend(ObjectReadBackendError::Unavailable(ObjectUnavailable::NotFound)) => {
                formatter.write_str("DASObjectStore object is unavailable: not found")
            }
            Self::Backend(ObjectReadBackendError::Unavailable(ObjectUnavailable::AccessDenied)) => {
                formatter.write_str("DASObjectStore object is unavailable: access denied")
            }
            Self::Backend(ObjectReadBackendError::Unavailable(ObjectUnavailable::Unavailable)) => {
                formatter.write_str("DASObjectStore object is unavailable")
            }
            Self::Backend(ObjectReadBackendError::Rejected(message)) => {
                write!(formatter, "DASObjectStore object read rejected: {message}")
            }
            Self::MetadataMismatch(message) => write!(
                formatter,
                "invalid DASObjectStore response metadata: {message}"
            ),
        }
    }
}

impl std::error::Error for ObjectReadError {}

/// A scoped DASObjectStore read authority. Implementations return a stream,
/// not buffered bytes or backend filesystem locations.
pub trait ObjectReadBackend {
    type Stream;

    fn open(
        &mut self,
        request: &ObjectReadRequest,
    ) -> Result<ObjectReadResult<Self::Stream>, ObjectReadBackendError>;
}

/// Validated stream handoff. Its cache directive forbids x-img-local payload
/// persistence; browser/HTTP cache policy is a future host adapter concern.
pub enum ValidatedObjectRead<S> {
    Content {
        metadata: ObjectContentMetadata,
        stream: S,
    },
    NotModified {
        etag: String,
    },
}

pub struct AuthorizedObjectReader<B: ObjectReadBackend> {
    backend: B,
}

impl<B: ObjectReadBackend> AuthorizedObjectReader<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    pub fn open(
        &mut self,
        request: &ObjectReadRequest,
    ) -> Result<ValidatedObjectRead<B::Stream>, ObjectReadError> {
        validate_request(request)?;
        match self
            .backend
            .open(request)
            .map_err(ObjectReadError::Backend)?
        {
            ObjectReadResult::Content { metadata, stream } => {
                validate_content_metadata(request, &metadata)?;
                Ok(ValidatedObjectRead::Content { metadata, stream })
            }
            ObjectReadResult::NotModified { etag } => {
                let expected = request.if_none_match_etag.as_deref().ok_or_else(|| {
                    ObjectReadError::MetadataMismatch(
                        "not-modified response requires an if-none-match ETag".to_owned(),
                    )
                })?;
                if etag != expected || etag != checksum_etag(&request.object.checksum) {
                    return Err(ObjectReadError::MetadataMismatch(
                        "not-modified ETag does not match the requested object".to_owned(),
                    ));
                }
                Ok(ValidatedObjectRead::NotModified { etag })
            }
        }
    }

    pub fn into_backend(self) -> B {
        self.backend
    }
}

fn validate_request(request: &ObjectReadRequest) -> Result<(), ObjectReadError> {
    for (field, value) in [
        ("endpoint_id", request.object.endpoint_id.as_str()),
        ("object_store_id", request.object.object_store_id.as_str()),
    ] {
        if !is_identifier(value) {
            return Err(ObjectReadError::InvalidRequest(format!(
                "`{field}` must be an identifier"
            )));
        }
    }
    if !is_safe_object_key(&request.object.object_key) {
        return Err(ObjectReadError::InvalidRequest(
            "`object_key` must be safe".to_owned(),
        ));
    }
    if !is_sha256_checksum(&request.object.checksum) {
        return Err(ObjectReadError::InvalidRequest(
            "`checksum` must be a SHA-256 digest".to_owned(),
        ));
    }
    if let Some(range) = request.range
        && range.start > range.end_inclusive
    {
        return Err(ObjectReadError::InvalidRequest(
            "byte range start must not exceed its end".to_owned(),
        ));
    }
    if let Some(etag) = &request.if_none_match_etag
        && etag != &checksum_etag(&request.object.checksum)
    {
        return Err(ObjectReadError::InvalidRequest(
            "if-none-match ETag must identify the requested checksum".to_owned(),
        ));
    }
    Ok(())
}

fn validate_content_metadata(
    request: &ObjectReadRequest,
    metadata: &ObjectContentMetadata,
) -> Result<(), ObjectReadError> {
    if !is_media_content_type(&metadata.content_type) {
        return Err(ObjectReadError::MetadataMismatch(
            "content type is not an accepted media type".to_owned(),
        ));
    }
    if metadata.checksum != request.object.checksum {
        return Err(ObjectReadError::MetadataMismatch(
            "checksum differs from the authorized object reference".to_owned(),
        ));
    }
    if metadata.etag != checksum_etag(&metadata.checksum) {
        return Err(ObjectReadError::MetadataMismatch(
            "ETag must be the quoted object checksum".to_owned(),
        ));
    }
    if metadata.total_length == 0 {
        return Err(ObjectReadError::MetadataMismatch(
            "total length must be positive".to_owned(),
        ));
    }
    match (request.range, metadata.content_range) {
        (None, None) if metadata.content_length == metadata.total_length => Ok(()),
        (Some(requested), Some(actual))
            if requested == actual
                && actual.end_inclusive < metadata.total_length
                && metadata.content_length == actual.end_inclusive - actual.start + 1 =>
        {
            Ok(())
        }
        (None, None) => Err(ObjectReadError::MetadataMismatch(
            "full response length must equal total length".to_owned(),
        )),
        (Some(_), Some(_)) => Err(ObjectReadError::MetadataMismatch(
            "range metadata does not match the requested interval".to_owned(),
        )),
        _ => Err(ObjectReadError::MetadataMismatch(
            "response range presence does not match the request".to_owned(),
        )),
    }
}

fn checksum_etag(checksum: &str) -> String {
    format!("\"{checksum}\"")
}

fn is_media_content_type(value: &str) -> bool {
    let base = value.split(';').next().unwrap_or_default().trim();
    (base.starts_with("image/") || base.starts_with("video/") || base == "application/octet-stream")
        && base
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b'/')
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn is_safe_object_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && !value.starts_with('/')
        && !value.contains("//")
        && value
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn is_sha256_checksum(value: &str) -> bool {
    value.len() == "sha256:".len() + 64
        && value.starts_with("sha256:")
        && value["sha256:".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FixtureBackend {
        response: Option<Result<ObjectReadResult<()>, ObjectReadBackendError>>,
    }

    impl ObjectReadBackend for FixtureBackend {
        type Stream = ();

        fn open(
            &mut self,
            _request: &ObjectReadRequest,
        ) -> Result<ObjectReadResult<Self::Stream>, ObjectReadBackendError> {
            self.response
                .take()
                .expect("fixture supplies exactly one response")
        }
    }

    fn object() -> AuthorizedObjectReference {
        AuthorizedObjectReference {
            endpoint_id: "endpoint-1".to_owned(),
            object_store_id: "store-1".to_owned(),
            object_key: "x-img/fixture.jpg".to_owned(),
            checksum: "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                .to_owned(),
        }
    }

    fn full_metadata() -> ObjectContentMetadata {
        ObjectContentMetadata {
            content_type: "image/jpeg".to_owned(),
            content_length: 3,
            total_length: 3,
            checksum: object().checksum.clone(),
            etag: checksum_etag(&object().checksum),
            content_range: None,
        }
    }

    #[test]
    fn validates_full_and_ranged_authority_responses_without_caching_bytes() {
        let request = ObjectReadRequest {
            object: object(),
            range: None,
            if_none_match_etag: None,
        };
        let mut reader = AuthorizedObjectReader::new(FixtureBackend {
            response: Some(Ok(ObjectReadResult::Content {
                metadata: full_metadata(),
                stream: (),
            })),
        });
        assert!(matches!(
            reader.open(&request),
            Ok(ValidatedObjectRead::Content { .. })
        ));

        let range = ByteRange {
            start: 1,
            end_inclusive: 2,
        };
        let ranged_request = ObjectReadRequest {
            object: object(),
            range: Some(range),
            if_none_match_etag: None,
        };
        let mut reader = AuthorizedObjectReader::new(FixtureBackend {
            response: Some(Ok(ObjectReadResult::Content {
                metadata: ObjectContentMetadata {
                    content_length: 2,
                    content_range: Some(range),
                    ..full_metadata()
                },
                stream: (),
            })),
        });
        assert!(matches!(
            reader.open(&ranged_request),
            Ok(ValidatedObjectRead::Content { .. })
        ));
    }

    #[test]
    fn validates_conditional_and_unavailable_outcomes() {
        let etag = checksum_etag(&object().checksum);
        let request = ObjectReadRequest {
            object: object(),
            range: None,
            if_none_match_etag: Some(etag.clone()),
        };
        let mut reader = AuthorizedObjectReader::new(FixtureBackend {
            response: Some(Ok(ObjectReadResult::NotModified { etag })),
        });
        assert!(matches!(
            reader.open(&request),
            Ok(ValidatedObjectRead::NotModified { .. })
        ));

        let mut reader = AuthorizedObjectReader::new(FixtureBackend {
            response: Some(Err(ObjectReadBackendError::Unavailable(
                ObjectUnavailable::NotFound,
            ))),
        });
        assert!(matches!(
            reader.open(&ObjectReadRequest {
                object: object(),
                range: None,
                if_none_match_etag: None,
            }),
            Err(ObjectReadError::Backend(
                ObjectReadBackendError::Unavailable(ObjectUnavailable::NotFound)
            ))
        ));
    }

    #[test]
    fn rejects_bad_metadata_and_out_of_range_responses() {
        let request = ObjectReadRequest {
            object: object(),
            range: Some(ByteRange {
                start: 1,
                end_inclusive: 2,
            }),
            if_none_match_etag: None,
        };
        let mut reader = AuthorizedObjectReader::new(FixtureBackend {
            response: Some(Ok(ObjectReadResult::Content {
                metadata: ObjectContentMetadata {
                    content_length: 2,
                    content_range: Some(ByteRange {
                        start: 1,
                        end_inclusive: 3,
                    }),
                    ..full_metadata()
                },
                stream: (),
            })),
        });
        assert!(matches!(
            reader.open(&request),
            Err(ObjectReadError::MetadataMismatch(_))
        ));

        let mut invalid = ObjectReadRequest {
            object: object(),
            range: None,
            if_none_match_etag: Some("\"sha256:wrong\"".to_owned()),
        };
        let mut reader = AuthorizedObjectReader::new(FixtureBackend::default());
        assert!(matches!(
            reader.open(&invalid),
            Err(ObjectReadError::InvalidRequest(_))
        ));
        invalid.range = Some(ByteRange {
            start: 3,
            end_inclusive: 2,
        });
        invalid.if_none_match_etag = None;
        assert!(matches!(
            reader.open(&invalid),
            Err(ObjectReadError::InvalidRequest(_))
        ));
    }
}
