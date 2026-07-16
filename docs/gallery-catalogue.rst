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

This boundary alone does not claim the full vertical. The web integration below
consumes it; persistent capture/review population, monolith delivery
composition, and the real-Firefox restart proof remain.

Web gallery integration
-----------------------

The Yew gallery requests the same endpoint through Monas at
``/products/pinakotheke/api/gallery/v1/catalogue``. It does not contain a
synthetic fallback catalogue or proxy artwork. Ready thumbnail and poster paths
render directly in dense cards; ready original-image and normalized-video paths
render in the keyboard-accessible preview pane. The web client derives source
counts and filters from the returned X-account or website classification.
Instagram capture remains part of the normal website class.

Loading, empty, permission-denied, transport-error, unsupported-schema, and
object-unavailable states are expressed in words. In all failure states the web
client leaves media absent and never contacts a source website. This slice was
checked against ``mnemosyne_design_language`` commit
``5539df8f662a78ebdf7cf4c868d71831380c8cfd``, Monas commit
``a0fabe2d250f2d217765ee59a95cc2a04610bedc``, and DASObjectStore commit
``cd6617cdbfc0d8587b3b51b9925a378b3aacaab9``. These are compatibility evidence,
not unpublished path dependencies.

Persistent population of this endpoint and the real-Firefox restart proof
remain the next XIMG-096 slice. The authenticated monolith already composes the
canonical route and returns an honest empty catalogue when no persistent
projection has been supplied.
