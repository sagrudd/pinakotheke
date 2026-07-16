Gallery catalogue boundary
==========================

The Pinakotheke gallery is intended to be a dense, ThumbsPlus-like browser for
media captured through Firefox. Synthetic cards and proxy artwork are useful
development scaffolding, but they are not evidence that the product works.
XIMG-096 tracks the required end-to-end proof.

The first XIMG-096 slice defines ``GET /api/gallery/v1/catalogue``. Monas must
authenticate the browser session and inject a validated standalone host context
before this endpoint is reachable. Direct unauthenticated access is rejected.
The endpoint is bounded to 200 records per page and returns newest records
first with a stable catalogue-ID tie break.

Object authority and availability
---------------------------------

Every card representation carries the stable endpoint ID, ObjectStore ID,
object key, SHA-256 checksum, media type, and length of its verified
DASObjectStore object. A ready representation also carries a host-local
authorized delivery path beginning with ``/``. Source and origin URLs are not
part of this response and can never be used as a media fallback.

An unavailable representation explicitly says ``unavailable`` and has no
delivery path. The web client must render its accessible unavailable-object
state; it must not request the source website. A card representation is either
an observed thumbnail or a normalized-video poster. A preview is either an
explicitly opened original image or a verified normalized-video rendition.
The schema rejects mismatched media and representation types.

This boundary does not yet claim the full vertical. The next slices replace the
Yew synthetic catalogue with this response, connect its paths to authorized
image and video delivery, persist capture and review admission, and run the
real-Firefox restart acceptance proof described by XIMG-096.
