Streaming object ingest
=======================

x-img streams image and video bytes only through a DASObjectStore ingest
backend. The ``StreamingObjectIngestor`` owns no payload directory, temporary
file, database blob, browser-storage copy, or durable local staging area. It
holds only an incremental SHA-256 state, byte count, bounded request metadata,
and idempotency receipt.

Protocol
--------

The caller begins an ingest with a stable ingest ID, endpoint ID, ObjectStore
ID, safe object key, exact expected length, SHA-256 checksum, and maximum chunk
size. Each caller-provided chunk is rejected before forwarding if it exceeds the
configured bound or would exceed the expected total. Otherwise it is sent once
to the authority backend and then included in the local incremental digest.

The backend can return explicit backpressure. x-img does not queue, retry, or
buffer the rejected chunk; the caller must wait and retry according to the
future job scheduler policy. Completion occurs only after exact length and
checksum verification. The backend must return the exact endpoint, ObjectStore,
key, size, checksum, and object reference expected by the request; disagreement
fails closed.

Idempotency and authority
-------------------------

After a verified completion, the same ingest ID and exact metadata returns the
original receipt without opening another upload. A changed target or verification
claim for that ID is an idempotency conflict. This in-memory contract is a
domain boundary only; XIMG-023 crash reconciliation and the future durable
catalogue/authority adapters remain responsible for recovery across process
restarts.

The backend must be authorized through the XIMG-032 scoped application identity
and the DASObjectStore daemon. It remains responsible for token/capability
proof, quota and health checks, authoritative stream persistence, and final
commit. The protocol was reviewed against ``../DASObjectStore`` commit
``8368d34a365689e19321ecd6a35aab7c819268f6``; no sibling path dependency or
live daemon exchange is used here.

Ephemeral worker files
----------------------

The normalization worker may use ``stream_ephemeral_file`` only for an
isolated, bounded worker scratch file.  It opens that file once and forwards it
in the request's bounded chunk size; it does not copy the bytes into x-img
state.  The worker owns deletion of the entire scratch directory after verified
completion or any failure.  Paths are deliberately not recorded in receipts or
errors.
