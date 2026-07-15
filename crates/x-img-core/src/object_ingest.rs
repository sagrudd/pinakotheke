// SPDX-License-Identifier: MPL-2.0
//! Bounded streaming ingest contract for DASObjectStore-backed media bytes.
//!
//! This module never opens a local payload file or retains chunk bytes. Each
//! caller-provided bounded chunk is immediately offered to the authority
//! backend, while x-img retains only verification and idempotency metadata.

#![allow(missing_docs)]

use std::{collections::BTreeMap, fs::File, io::Read, path::Path};

use sha2::{Digest, Sha256};

pub const MAX_INGEST_CHUNK_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestRequest {
    pub ingest_id: String,
    pub endpoint_id: String,
    pub object_store_id: String,
    pub object_key: String,
    pub expected_size_bytes: u64,
    pub expected_checksum: String,
    pub max_chunk_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitReceipt {
    pub endpoint_id: String,
    pub object_store_id: String,
    pub object_key: String,
    pub size_bytes: u64,
    pub checksum: String,
    pub object_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestBackendError {
    Backpressure,
    Rejected(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectIngestError {
    Invalid(String),
    Backend(IngestBackendError),
    ChunkTooLarge { max_bytes: usize },
    SizeExceeded,
    SizeMismatch { expected: u64, actual: u64 },
    ChecksumMismatch { expected: String, actual: String },
    AuthorityMismatch,
    IdempotencyConflict,
    EphemeralFileIo,
}

impl std::fmt::Display for ObjectIngestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "invalid ingest request: {message}"),
            Self::Backend(IngestBackendError::Backpressure) => {
                formatter.write_str("DASObjectStore ingest backend applied backpressure")
            }
            Self::Backend(IngestBackendError::Rejected(message)) => {
                write!(
                    formatter,
                    "DASObjectStore ingest backend rejected the stream: {message}"
                )
            }
            Self::ChunkTooLarge { max_bytes } => {
                write!(formatter, "stream chunk exceeds the {max_bytes}-byte limit")
            }
            Self::SizeExceeded => formatter.write_str("stream exceeds the declared byte length"),
            Self::SizeMismatch { expected, actual } => {
                write!(
                    formatter,
                    "stream length {actual} differs from expected {expected}"
                )
            }
            Self::ChecksumMismatch { expected, actual } => {
                write!(
                    formatter,
                    "stream checksum {actual} differs from expected {expected}"
                )
            }
            Self::AuthorityMismatch => {
                formatter.write_str("authority completion differs from ingest target")
            }
            Self::IdempotencyConflict => {
                formatter.write_str("ingest ID was already committed differently")
            }
            Self::EphemeralFileIo => {
                formatter.write_str("bounded ephemeral ingest file could not be read")
            }
        }
    }
}

impl std::error::Error for ObjectIngestError {}

/// The DASObjectStore-facing boundary. Implementations may stream over an
/// authorized transport, but may not turn x-img's product root into staging.
pub trait ObjectIngestBackend {
    type Upload;

    fn begin(&mut self, request: &IngestRequest) -> Result<Self::Upload, IngestBackendError>;
    fn write_chunk(
        &mut self,
        upload: &mut Self::Upload,
        chunk: &[u8],
    ) -> Result<(), IngestBackendError>;
    fn complete(
        &mut self,
        upload: Self::Upload,
        expected: &CommitReceipt,
    ) -> Result<CommitReceipt, IngestBackendError>;
}

pub enum BeginIngest<U> {
    Active(IngestSession<U>),
    AlreadyCommitted(CommitReceipt),
}

pub struct IngestSession<U> {
    request: IngestRequest,
    upload: U,
    bytes_written: u64,
    hasher: Sha256,
}

pub struct StreamingObjectIngestor<B: ObjectIngestBackend> {
    backend: B,
    committed: BTreeMap<String, CommitReceipt>,
}

impl<B: ObjectIngestBackend> StreamingObjectIngestor<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            committed: BTreeMap::new(),
        }
    }

    pub fn begin(
        &mut self,
        request: IngestRequest,
    ) -> Result<BeginIngest<B::Upload>, ObjectIngestError> {
        validate_request(&request)?;
        if let Some(receipt) = self.committed.get(&request.ingest_id) {
            if receipt_matches_request(receipt, &request) {
                return Ok(BeginIngest::AlreadyCommitted(receipt.clone()));
            }
            return Err(ObjectIngestError::IdempotencyConflict);
        }
        let upload = self
            .backend
            .begin(&request)
            .map_err(ObjectIngestError::Backend)?;
        Ok(BeginIngest::Active(IngestSession {
            request,
            upload,
            bytes_written: 0,
            hasher: Sha256::new(),
        }))
    }

    pub fn write_chunk(
        &mut self,
        session: &mut IngestSession<B::Upload>,
        chunk: &[u8],
    ) -> Result<(), ObjectIngestError> {
        if chunk.len() > session.request.max_chunk_bytes {
            return Err(ObjectIngestError::ChunkTooLarge {
                max_bytes: session.request.max_chunk_bytes,
            });
        }
        let chunk_size = u64::try_from(chunk.len()).map_err(|_| ObjectIngestError::SizeExceeded)?;
        if session.bytes_written.saturating_add(chunk_size) > session.request.expected_size_bytes {
            return Err(ObjectIngestError::SizeExceeded);
        }
        self.backend
            .write_chunk(&mut session.upload, chunk)
            .map_err(ObjectIngestError::Backend)?;
        session.hasher.update(chunk);
        session.bytes_written = session.bytes_written.saturating_add(chunk_size);
        Ok(())
    }

    pub fn finish(
        &mut self,
        session: IngestSession<B::Upload>,
    ) -> Result<CommitReceipt, ObjectIngestError> {
        if session.bytes_written != session.request.expected_size_bytes {
            return Err(ObjectIngestError::SizeMismatch {
                expected: session.request.expected_size_bytes,
                actual: session.bytes_written,
            });
        }
        let actual_checksum = format!("sha256:{:x}", session.hasher.finalize());
        if actual_checksum != session.request.expected_checksum {
            return Err(ObjectIngestError::ChecksumMismatch {
                expected: session.request.expected_checksum,
                actual: actual_checksum,
            });
        }
        let expected = CommitReceipt {
            endpoint_id: session.request.endpoint_id.clone(),
            object_store_id: session.request.object_store_id.clone(),
            object_key: session.request.object_key.clone(),
            size_bytes: session.request.expected_size_bytes,
            checksum: session.request.expected_checksum.clone(),
            object_reference: format!(
                "dasobjectstore:{}:{}:{}",
                session.request.endpoint_id,
                session.request.object_store_id,
                session.request.object_key
            ),
        };
        let receipt = self
            .backend
            .complete(session.upload, &expected)
            .map_err(ObjectIngestError::Backend)?;
        if receipt != expected {
            return Err(ObjectIngestError::AuthorityMismatch);
        }
        self.committed
            .insert(session.request.ingest_id, receipt.clone());
        Ok(receipt)
    }

    pub fn into_backend(self) -> B {
        self.backend
    }

    /// Streams one bounded ephemeral worker file directly to DASObjectStore.
    ///
    /// The caller owns the file's isolated scratch lifecycle. x-img retains no
    /// copy after this method returns, and the method does not reveal its path
    /// in errors or receipts.
    pub fn stream_ephemeral_file(
        &mut self,
        request: IngestRequest,
        path: &Path,
    ) -> Result<CommitReceipt, ObjectIngestError> {
        let mut session = match self.begin(request)? {
            BeginIngest::AlreadyCommitted(receipt) => return Ok(receipt),
            BeginIngest::Active(session) => session,
        };
        let mut file = File::open(path).map_err(|_| ObjectIngestError::EphemeralFileIo)?;
        let mut buffer = vec![0_u8; session.request.max_chunk_bytes];
        loop {
            let bytes_read = file
                .read(&mut buffer)
                .map_err(|_| ObjectIngestError::EphemeralFileIo)?;
            if bytes_read == 0 {
                break;
            }
            self.write_chunk(&mut session, &buffer[..bytes_read])?;
        }
        self.finish(session)
    }
}

fn validate_request(request: &IngestRequest) -> Result<(), ObjectIngestError> {
    for (name, value) in [
        ("ingest_id", request.ingest_id.as_str()),
        ("endpoint_id", request.endpoint_id.as_str()),
        ("object_store_id", request.object_store_id.as_str()),
    ] {
        if !is_identifier(value) {
            return Err(ObjectIngestError::Invalid(format!(
                "`{name}` must be an identifier"
            )));
        }
    }
    if !is_safe_object_key(&request.object_key) {
        return Err(ObjectIngestError::Invalid(
            "`object_key` must be safe".to_owned(),
        ));
    }
    if request.expected_size_bytes == 0
        || request.max_chunk_bytes == 0
        || request.max_chunk_bytes > MAX_INGEST_CHUNK_BYTES
    {
        return Err(ObjectIngestError::Invalid(
            "expected size must be positive and chunk limit must be bounded".to_owned(),
        ));
    }
    if !is_sha256_checksum(&request.expected_checksum) {
        return Err(ObjectIngestError::Invalid(
            "expected checksum must be a sha256 digest".to_owned(),
        ));
    }
    Ok(())
}

fn receipt_matches_request(receipt: &CommitReceipt, request: &IngestRequest) -> bool {
    receipt.endpoint_id == request.endpoint_id
        && receipt.object_store_id == request.object_store_id
        && receipt.object_key == request.object_key
        && receipt.size_bytes == request.expected_size_bytes
        && receipt.checksum == request.expected_checksum
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
    use std::{
        env, fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[derive(Debug, Default)]
    struct CountingBackend {
        chunks: usize,
        bytes: u64,
        backpressure: bool,
    }

    impl ObjectIngestBackend for CountingBackend {
        type Upload = ();

        fn begin(&mut self, _request: &IngestRequest) -> Result<Self::Upload, IngestBackendError> {
            Ok(())
        }

        fn write_chunk(
            &mut self,
            _upload: &mut Self::Upload,
            chunk: &[u8],
        ) -> Result<(), IngestBackendError> {
            if self.backpressure {
                return Err(IngestBackendError::Backpressure);
            }
            self.chunks += 1;
            self.bytes = self
                .bytes
                .saturating_add(u64::try_from(chunk.len()).expect("usize fits u64"));
            Ok(())
        }

        fn complete(
            &mut self,
            _upload: Self::Upload,
            expected: &CommitReceipt,
        ) -> Result<CommitReceipt, IngestBackendError> {
            Ok(expected.clone())
        }
    }

    fn request() -> IngestRequest {
        IngestRequest {
            ingest_id: "ingest-1".to_owned(),
            endpoint_id: "endpoint-1".to_owned(),
            object_store_id: "store-1".to_owned(),
            object_key: "x-img/fixture.bin".to_owned(),
            expected_size_bytes: 3,
            expected_checksum:
                "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_owned(),
            max_chunk_bytes: 2,
        }
    }

    #[test]
    fn streams_bounded_chunks_and_completion_is_idempotent() {
        let mut ingestor = StreamingObjectIngestor::new(CountingBackend::default());
        let mut session = match ingestor.begin(request()).expect("begin succeeds") {
            BeginIngest::Active(session) => session,
            BeginIngest::AlreadyCommitted(_) => panic!("first ingest is active"),
        };
        ingestor
            .write_chunk(&mut session, b"ab")
            .expect("first chunk");
        ingestor
            .write_chunk(&mut session, b"c")
            .expect("second chunk");
        let receipt = ingestor.finish(session).expect("verified completion");
        assert_eq!(receipt.size_bytes, 3);
        let backend = ingestor.into_backend();
        assert_eq!(backend.chunks, 2);
        assert_eq!(backend.bytes, 3);
    }

    #[test]
    fn rejects_length_checksum_and_backpressure_without_local_buffering() {
        let mut ingestor = StreamingObjectIngestor::new(CountingBackend {
            backpressure: true,
            ..CountingBackend::default()
        });
        let mut session = match ingestor.begin(request()).expect("begin succeeds") {
            BeginIngest::Active(session) => session,
            BeginIngest::AlreadyCommitted(_) => panic!("first ingest is active"),
        };
        assert_eq!(
            ingestor.write_chunk(&mut session, b"ab"),
            Err(ObjectIngestError::Backend(IngestBackendError::Backpressure))
        );
        assert_eq!(session.bytes_written, 0);
        assert_eq!(
            ingestor.write_chunk(&mut session, b"too"),
            Err(ObjectIngestError::ChunkTooLarge { max_bytes: 2 })
        );

        let mut mismatch = StreamingObjectIngestor::new(CountingBackend::default());
        let mut mismatch_session = match mismatch.begin(request()).expect("begin succeeds") {
            BeginIngest::Active(session) => session,
            BeginIngest::AlreadyCommitted(_) => panic!("first ingest is active"),
        };
        mismatch
            .write_chunk(&mut mismatch_session, b"ab")
            .expect("first mismatch chunk");
        mismatch
            .write_chunk(&mut mismatch_session, b"d")
            .expect("second mismatch chunk");
        assert!(matches!(
            mismatch.finish(mismatch_session),
            Err(ObjectIngestError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn returns_existing_commit_and_rejects_a_conflicting_replay() {
        let mut ingestor = StreamingObjectIngestor::new(CountingBackend::default());
        let mut session = match ingestor.begin(request()).expect("begin succeeds") {
            BeginIngest::Active(session) => session,
            BeginIngest::AlreadyCommitted(_) => panic!("first ingest is active"),
        };
        ingestor
            .write_chunk(&mut session, b"abc")
            .expect_err("chunk limit applies");
        ingestor
            .write_chunk(&mut session, b"ab")
            .expect("first chunk");
        ingestor
            .write_chunk(&mut session, b"c")
            .expect("second chunk");
        let receipt = ingestor.finish(session).expect("finish succeeds");
        match ingestor.begin(request()).expect("replay succeeds") {
            BeginIngest::AlreadyCommitted(replayed) => assert_eq!(replayed, receipt),
            BeginIngest::Active(_) => panic!("completed ingest must not reopen an upload"),
        }
        let mut conflicting = request();
        conflicting.object_key = "x-img/other.bin".to_owned();
        assert!(matches!(
            ingestor.begin(conflicting),
            Err(ObjectIngestError::IdempotencyConflict)
        ));
    }

    #[test]
    fn streams_an_ephemeral_file_in_bounded_chunks_without_retaining_it() {
        let path = env::temp_dir().join(format!(
            "x-img-ingest-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::write(&path, b"abc").expect("synthetic ephemeral file");
        let mut ingestor = StreamingObjectIngestor::new(CountingBackend::default());
        let receipt = ingestor
            .stream_ephemeral_file(request(), &path)
            .expect("stream completes");
        fs::remove_file(path).expect("caller cleans ephemeral file");
        assert_eq!(receipt.size_bytes, 3);
        let backend = ingestor.into_backend();
        assert_eq!(backend.chunks, 2);
        assert_eq!(backend.bytes, 3);
    }
}
