Incremental X media discovery
=============================

XIMG-042 supplies the pure, fixture-driven discovery planner for a future
official X adapter. It does not contact X, download media, retain media bytes,
or make a catalogue item reviewable. Live X use remains blocked by the approval,
rights, retention, and deletion gates in :doc:`adr/0002-platform-policy`.

Bounded discovery contract
--------------------------

An approved future adapter may pass pages to ``plan_incremental_discovery``.
Each page declares the cursor that requested it, its next cursor, its permitted
timeline depth, source item ID, and the photos, videos, or animated GIFs found
in that item. The planner rejects a broken cursor chain or a depth beyond the
configured maximum. It stops at the configured page or item limit and reports
the next cursor together with ``truncated_by_budget``; it does not fetch another
page or retry around the limit.

For every media entry, the planner selects the largest supported rendition:

* photos accept JPEG, PNG, or WebP;
* video and animated GIF candidates accept MP4; and
* unsupported HLS or other rendition types are not selected.

The selected metadata records ``x:<account>:<item>:<media>`` as the canonical
identity, the source URL alias, account/item/media IDs, media kind, candidate
checksum, discovery time, adapter version, and policy result. These fields are
metadata only. A source URL is never used as identity, and opaque authority
credentials, browser cookies, signed URL parameters, and media payloads are
not accepted.

Idempotency and later commit
----------------------------

Duplicate media entries across pages resolve to one canonical candidate. A
candidate can produce the existing ``ReconciliationRequest`` keyed by canonical
identity plus immutable SHA-256 evidence. The reconciliation catalogue therefore
settles a verified repeated attempt once, appending safe aliases rather than
overwriting an ObjectStore reference. This is not a byte transfer: a future
authorized ingest adapter must still verify DASObjectStore evidence before the
acquisition lifecycle can commit or enter review.

The synthetic fixture covers pagination, a duplicate page entry, photos,
video, animated GIFs, unsupported variants, depth limits, page limits, selected
best variants, and replay idempotency. It contains no real account, private URL,
credential, token, cookie, or media content.

Verify locally
--------------

.. code-block:: console

   cargo +1.97.0 test -p x-img-core x_discovery
   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check
