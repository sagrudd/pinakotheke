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

First-party DAS helper
----------------------

The packaged ``pinakotheke`` binary now implements the helper's hidden
``read-v1`` command itself. Set ``PINAKOTHEKE_DAS_READ_HELPER_CONFIG`` to an
absolute mode-``0600`` configuration file (the default is
``$HOME/.x-img/config/das-object-read-helper.json``), then pass the same
``pinakotheke`` executable as ``--object-read-helper``. A minimal configuration
is:

.. code-block:: json

   {
     "schema_version": "pinakotheke.das-object-read-helper.v1",
     "endpoint_id": "local-docker-314985151",
     "endpoint_url": "http://127.0.0.1:3900",
     "region": "garage",
     "profile": "pinakotheke",
     "aws_executable": "/usr/local/bin/aws",
     "stores": [
       {"object_store_id": "pinakotheke_local", "bucket": "dos-pinakotheke-local"}
     ],
     "max_object_bytes": 1073741824
   }

The endpoint and every ObjectStore-to-bucket mapping are explicit; the helper
never selects the first bucket or changes destination after reconnect. AWS
credentials remain in the host-owned named profile or its credential process
and never enter Pinakotheke configuration, requests, logs, or browser state.
Only HTTPS endpoints are accepted, except explicit loopback HTTP development
endpoints.

The helper performs a bounded ``head-object`` before every read. When the
object has ``dasobjectstore-sha256`` metadata it must exactly match the reviewed
commit. A complete legacy S3-export object without that optional metadata is
downloaded into private bounded scratch and its bytes are SHA-256 verified
before any response header or payload is emitted. A ranged read remains
fail-closed unless the authority metadata contains the exact checksum, because
a partial byte range cannot independently prove the full-object digest. The
helper uses structured ``s3api get-object`` arguments and verifies lengths;
the supervisor additionally verifies complete streamed checksums. Scratch is
deleted on every outcome and is never placed under the Pinakotheke root.
Conditional checksum ETags avoid a payload read only when authority checksum
metadata already proves the object; legacy objects are verified first.

The strict configuration schema is
``contracts/dasobjectstore/pinakotheke-das-object-read-helper.v1.schema.json``.
This first version uses the scoped S3 read granted by DASObjectStore because
the current public ``dasobjectstore-remote`` client has no download command.
Replacing it with a daemon-native read transport later does not change the
``pinakotheke.object-read-helper.v1`` boundary.

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
``5769f27859a58101aedd9de0087fc278fd3e4b16`` and its application-auth plus
provider-stream range/checksum model. No sibling path dependency, browser
credential, or backend path enters the public build.
