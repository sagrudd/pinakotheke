DASObjectStore application identity
===================================

x-img uses a daemon-owned DASObjectStore application identity; it does not
hold a storage password, S3 secret, bearer token, private key, or host path.
The public registration shape is
``contracts/dasobjectstore/x-img-application-identity.v1.json``. It was
reviewed against ``../DASObjectStore`` commit
``76f6411eab1e2c486c0bc1b4695b71f09307d9df`` and its
``dasobjectstore.application_auth.v1`` contract. That pin is not a path or
runtime dependency.

Scope
-----

The registration names one stable endpoint and one logical ObjectStore, one
logical ``x-img/`` prefix, and the narrow ``read``, ``write``, ``list``, and
``verify`` operation set. It carries both per-object and total byte limits,
explicit issue/expiry times, an opaque Monas owner reference, and an opaque
DASObjectStore application reference. It never contains credential material.

Before a future storage adapter requests a short-lived DAS access token or
upload-completion capability, x-img authorizes a single scoped operation. The
contract rejects an expired identity, replayed operation ID, endpoint/store
mismatch, object key outside the prefix, unauthorized operation, and either
byte-limit violation. The daemon remains authoritative and must independently
verify proof, identity status, scope, quota, destination health, and any
completion capability immediately before a real commit.

Fixtures and deployment
-----------------------

The synthetic authorization matrix covers accepted, expired, wrong-store,
wrong-prefix, and oversized requests; the native tests also prove replay
rejection. It contains no token, certificate, proof, credential, media byte, or
private endpoint.

An operator must register the corresponding service principal with the
DASObjectStore daemon through its approved administrative process, keep the
long-lived identity key or certificate in the daemon/approved secret boundary,
and supply only the opaque ``credential_ref`` to x-img. XIMG-033 will consume
short-lived, server-side authority through the storage-ingest port; this task
does not contact a daemon or issue any access token.
