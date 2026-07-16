Authorized object read and cache handoff
========================================

x-img reads image and video bytes only from an authorized DASObjectStore
authority. ``AuthorizedObjectReader`` returns a validated stream handoff and
never stores the stream in the x-img product root, database, browser storage,
logs, or a local disk cache. Browser/HTTP caching policy belongs to a future
host adapter and must remain transparent and fail-open.

Request and response contract
-----------------------------

A read request identifies the stable endpoint, ObjectStore, logical object ID,
positive immutable object version, and SHA-256 checksum established at verified commit. It may request one
inclusive byte range and an ``If-None-Match`` ETag. The ETag is the quoted
SHA-256 checksum, so a conditional request cannot accidentally validate a
different object.

For content, the authority must provide an accepted image/video (or opaque
binary) MIME type, content length, total length, checksum, quoted ETag, and,
for a range request, a matching ``Content-Range`` equivalent. x-img rejects
wrong metadata, a mismatched checksum/ETag, invalid range size, a range outside
the declared total, or a full response whose length differs from total length.
For a matching conditional read, the authority returns ``NotModified`` with the
matching ETag and no payload stream.

Unavailable states
------------------

The port keeps ``NotFound``, ``AccessDenied``, and temporary ``Unavailable``
as explicit authority outcomes. It does not turn any of them into an empty
payload or a stale local substitution. A Firefox/site cache adapter must fail
open to the origin when it cannot obtain a valid authority response.

Host helper adapter
-------------------

The local monolith can now compose a host-owned scoped reader with
``--object-read-helper /absolute/path/to/helper``. The executable must be a
regular, non-symlinked executable and implement
``pinakotheke.object-read-helper.v1``. Pinakotheke invokes it directly with the
single argument ``read-v1`` (never through a shell), writes one strict JSON
request line to standard input, reads one response JSON line of at most 8 KiB
from standard error, and streams standard output as the payload. The JSON never
contains payload bytes, credentials, cookies, bearer tokens, backend paths, or
origin URLs.

The host helper resolves the supplied stable endpoint/ObjectStore/object/version/checksum
through its own DASObjectStore authentication. Pinakotheke bounds streaming with
a four-chunk queue and 64 KiB chunks, checks the process result and exact byte
length, and verifies SHA-256 cumulatively for full responses. Range responses
retain the authority's full-object checksum and are length/range validated.
Unknown fields, future schemas, malformed metadata, a non-zero helper exit, or
a truncated/mismatched stream fail closed. No payload file is created beneath
the product root.

The checked-in JSON Schema is
``contracts/dasobjectstore/pinakotheke-object-read-helper.v1.schema.json``.
This is a narrow host adapter, not a new authentication system: DASObjectStore
or the composing host must supply the helper and retain all secret material.

Service endpoint binding
------------------------

A per-user macOS service requires the helper and its reviewed endpoint identity
as a pair. ``pinakotheke service install`` rejects either value on its own and
rejects path-like or unbounded endpoint identities. The backend agent exposes
the fixed value to the helper as
``PINAKOTHEKE_OBJECT_READ_ENDPOINT_ID``. A production helper must compare each
request's ``endpoint_id`` with that fixed value before contacting its own
authenticated DASObjectStore boundary. The value is authority scope, not a
password or token; secrets still belong to DASObjectStore or the host.

Foreground operators must configure the equivalent fixed endpoint scope in the
reviewed helper's execution environment. Pinakotheke deliberately does not set
it from request data, silently select the first endpoint, or fall back to an
origin URL.

The adapter was reviewed against ``../DASObjectStore`` commit
``8b6d94a284bca636525f4b3d56ad2bfc4fe864a1`` and its application-auth plus
provider-stream range/checksum model. No sibling path dependency, browser
credential, or backend path enters the public build.
