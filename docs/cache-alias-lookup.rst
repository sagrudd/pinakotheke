Cache alias lookup
==================

x-img resolves an eligible canonical media alias to one immutable
DASObjectStore object before the Firefox extension attempts substitution.  The
lookup is metadata-only: it neither reads media bytes nor records general page
history, source credentials, cookies, authorization headers, or signed query
parameters.

Eligibility and admission
-------------------------

Only verified committed objects enter the bounded server-side index.  Each
record binds all of the following:

* the paired x-img instance and explicitly enabled HTTPS site origin;
* the versioned site adapter and query-free canonical media alias;
* ``Observed thumbnail`` or ``Explicitly opened original`` evidence;
* a thumbnail image, original image, or Firefox-verified normalized MP4
  representation; and
* stable endpoint, ObjectStore, object key, checksum, MIME type, length,
  validity, and availability metadata.

An observed thumbnail cannot be relabelled as an original.  Original images
and normalized video require explicit-open evidence.  Re-admitting an alias
with a different immutable object is a conflict, never an overwrite.

Lookup and fail-open behavior
-----------------------------

The host-authenticated endpoint is:

.. code-block:: text

   POST /api/extension/v1/cache-aliases/lookup

The host supplies the Monas actor context.  The server revalidates the pairing,
actor, instance, origin, adapter version, expiry/revocation, and server-owned
substitution policy.  The extension cannot enable substitution by setting a
request flag.

A hit returns only the media class, verified MIME/length/checksum, and an x-img
delivery path.  It does not echo the alias or disclose a page URL.  A miss,
paused policy, invalid pairing, stale entry, offline endpoint, unavailable
object, malformed canonical alias, or adapter mismatch returns a bounded
``origin_fallback`` result.  XIMG-071 and XIMG-072 consume this result for image
and normalized-MP4 delivery; until then the delivery path is not intercepted.

Signed URL handling
-------------------

A versioned adapter must remove short-lived query and fragment components when
it produces the canonical alias.  A lookup containing ``?`` or ``#`` is
rejected and the response does not repeat it.  This prevents signed query data
from entering index keys, diagnostics, or browser storage.

Capacity, invalidation, and latency
-----------------------------------

The index has an explicit capacity from 1 to 65,536 entries.  Oldest admission
is evicted when full.  A single alias or an entire site origin can be
invalidated; authority health can mark an exact checksum offline or unavailable
without selecting another ObjectStore.

The acceptance budget is p95 below 2 ms for a 4,096-entry index.  The checked
Rust test performs 10,000 mixed hits; the 2026-07-15 local debug run measured
5.5 microseconds p95.  This is contract evidence rather than a production
capacity promise, so deployment telemetry must remeasure it without retaining
aliases.

Verification
------------

.. code-block:: console

   cargo +1.97.0 test -p x-img-core cache_alias -- --nocapture
   cargo +1.97.0 test -p x-img-api cache_alias
   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check
