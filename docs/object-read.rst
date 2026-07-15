Authorized object read and cache handoff
========================================

x-img reads image and video bytes only from an authorized DASObjectStore
authority. ``AuthorizedObjectReader`` returns a validated stream handoff and
never stores the stream in the x-img product root, database, browser storage,
logs, or a local disk cache. Browser/HTTP caching policy belongs to a future
host adapter and must remain transparent and fail-open.

Request and response contract
-----------------------------

A read request identifies the stable endpoint, ObjectStore, logical object key,
and SHA-256 object checksum established at verified commit. It may request one
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

The contract was reviewed against ``../DASObjectStore`` commit
``76f6411eab1e2c486c0bc1b4695b71f09307d9df`` and its provider-stream range and
checksum model. No sibling path dependency, bearer credential, backend path,
or live authority call is added by this task.
