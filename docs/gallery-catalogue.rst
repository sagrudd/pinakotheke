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

Persistent metadata
-------------------

The local monolith loads ``state/gallery-catalogue.v1.json`` below its private
metadata root at startup. A missing document is an honest empty catalogue. The
strict versioned document contains gallery metadata and immutable
DASObjectStore references only; it cannot contain image or video payloads.
The store rejects unknown fields, future schemas, invalid ObjectStore evidence,
origin delivery URLs, symlinks, non-regular files, more than 100,000 records,
and documents larger than 64 MiB.

Replacement validates the complete candidate before writing a private temporary
file, synchronizes it, atomically renames it, and synchronizes the containing
directory. A malformed or unsupported existing document fails monolith startup
instead of silently presenting an empty library. Restart tests load a written
record through a new store instance and preserve its catalogue ID,
endpoint/ObjectStore identity, checksum, review state, and availability without
retaining payload bytes.

The verified image admission boundary below now populates this store. Live
worker composition, authorized image/video delivery, and the real-Firefox
restart proof remain the next XIMG-096 slices.

Verified Firefox image admission
--------------------------------

``PersistentWebsiteGalleryAdmission`` joins the existing website-capture plan,
acquisition state machine, common review queue, and persistent gallery store.
It is an internal worker boundary, not a browser endpoint. Firefox cannot send
an ObjectStore reference or delivery path and cannot mark a record committed.

An observed thumbnail creates a ``New`` image card only when the acquisition is
already ``Committed`` with verified endpoint, ObjectStore, object-reference,
and checksum evidence and the capture plan passes website review admission.
The server derives the source classification and a host-local thumbnail route.
Replaying the same immutable object is idempotent; changing the object for the
same card is an explicit conflict.

An explicitly opened original may attach to an existing observed-thumbnail
card only after its own independent verified commit. Original-first admission
is rejected. The endpoint and logical ObjectStore must match the reviewed
thumbnail destination; an original cannot silently move the card to another
store. The server generates the original delivery path and atomically replaces
the complete metadata document. A restart test proves that one card retains
both object references, dimensions, ``New`` review state, and ready
availability without retaining image bytes.

Authorized gallery image delivery
---------------------------------

The generated ``.../objects/{catalogue_id}/thumbnail`` and
``.../objects/{catalogue_id}/original`` paths now resolve through the persisted
catalogue. The resolver requires a Monas standalone actor with Pinakotheke
access, an image card, the exact requested representation role, and ``Ready``
availability. Unknown records and non-image roles return not found; a known but
unavailable representation returns gone. Request parameters can never supply
an endpoint, ObjectStore, object key, checksum, MIME type, or source URL.

The authenticated monolith composition passes the resolved immutable reference
to its host-supplied DASObjectStore streaming-read backend. Pinakotheke validates
the returned MIME type, total length, checksum, and checksum ETag before handing
the stream to the browser. Conditional ``If-None-Match`` reads are retained.
Responses are same-origin, ``nosniff``, and ``private, no-store``; Pinakotheke
does not persist the bytes and has no origin fallback.

This code consumes the existing versioned ObjectStore read port and was checked
against DASObjectStore commit ``bdafc51154989db075f241d041d9eab699f4a022``.
DASObjectStore does not yet publish a stable application HTTP-read wire, so the
local CLI does not invent one or consume an unpublished sibling path. Live host
backend composition remains required before real stored images can be rendered.
