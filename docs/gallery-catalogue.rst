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
first with a stable catalogue-ID tie break. Every page reports both the number
of records matching the complete server query and the unfiltered catalogue
total, so the interface never presents a truncated page as the complete
library.

Bounded filtering and incremental browsing
------------------------------------------

The authenticated catalogue accepts exact ``source_kind``, ``media_kind``,
``review_state``, and representation ``availability`` filters, inclusive
``discovered_from_epoch_seconds`` and ``discovered_to_epoch_seconds`` bounds,
and a case-insensitive metadata ``text`` search over title, source label, and
catalogue ID. Filtering occurs before offset pagination. An inverted time
range, control characters, text longer than 128 characters, a zero page size,
or a page larger than 200 records is rejected instead of broadening the query.

The Yew library sends its selected All/X/Website context and metadata text to
this server boundary. It initially requests 100 records and exposes an
explicit ``Load next 100 records`` action while ``next_offset`` is present.
The source and text query are preserved across subsequent pages, filter changes
restart at the newest matching record, and the page states exactly how many
matching and total catalogue records exist. This removes the former silent
200-card truncation without putting the entire persistent catalogue in browser
memory.

Loaded pages are rendered through a fixed-row, responsive viewport window. The
window derives its column count from the current container width and selected
density, renders eight visible rows plus two overscan rows on either side, and
represents off-screen rows with non-interactive vertical space. Scrolling,
window resize, and density changes recalculate the slice; a 10,000-record unit
fixture proves that the DOM-bound slice remains bounded while the virtual
height and final record remain reachable.

Cards use one roving tab stop. Arrow keys move by card or visual row, while
Home and End reach the first and last currently loaded record. Moving to an
off-screen record updates the scroll position, renders the new window, and
then restores focus to the selected card. If pointer scrolling moves the
selected record outside the window, the first visible card remains a keyboard
entry point. Exercise the compiled application in installed Firefox with:

.. code-block:: console

   make firefox-gallery-check

The check builds the actual release Yew/WASM bundle, launches Firefox with a
private temporary profile, and presents 1,000 synthetic mixed image/video
records through a loopback surface representing an already-authenticated Monas
forward. It loads five 100-record pages, verifies that no more than 120 cards
enter the DOM, reaches records above index 400 by scrolling, moves focus to
record 499 with End, applies a server metadata filter, opens an unavailable
object, and proves that no resource request leaves the loopback origin. A
second pass constrains the real app to a 390-pixel content viewport and proves
two-column reflow. Browser profiles and fixture bytes are deleted afterwards.

This is browser-component evidence, not a substitute for a genuine Monas
session or DASObjectStore transport. The full capture/commit/restart vertical
remains required by XIMG-096.

Object authority and availability
---------------------------------

Every card representation carries the stable endpoint ID, ObjectStore ID,
object ID, positive immutable object version, SHA-256 checksum, media type, and length of its verified
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
render in the keyboard-accessible preview pane. The web client requests
source-filtered pages using the returned X-account or website classification.
Instagram capture remains part of the normal website class.

The crate has a WebAssembly start entry and a checked-in Trunk document. A
release build uses the canonical app mount as its public URL, and the monolith
serves the resulting bounded static tree only after the same private Monas
dispatch admission used by the catalogue. Direct backend access is denied.

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

Historic ``x-img.gallery-catalogue.v1`` records written before object versions
were explicit are read as version ``1``. Every newly written record includes
``object_version``; zero is invalid. The migration is non-destructive and makes
the DASObjectStore ``BackendObjectKey`` mapping exact instead of guessing a
version from an object string or checksum.

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
positive object version, and checksum evidence and the capture plan passes
website review admission.
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
an endpoint, ObjectStore, object ID, object version, checksum, MIME type, or source URL.

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

Normalized-video cards and playback
-----------------------------------

``admit_ready_normalized_video`` creates one persistent ``New`` card only from
a ``Ready`` normalized-video record that passes the versioned profile validator
with matching Firefox playback evidence. The record must already contain a
typed, checksummed normalized rendition, poster, and provenance manifest in the
same reviewed endpoint/ObjectStore. Planned, normalizing, source-only,
unproven, malformed, or conflicting records are rejected.

The poster becomes the card's thumbnail representation and is served through
the same authenticated image route. The normalized rendition becomes the
``.../objects/{catalogue_id}/video`` preview. Both delivery paths are generated
server-side and survive metadata-store restart; video and poster remain
separate DASObjectStore objects and no bytes enter the metadata document.

The video route resolves the exact persisted rendition and supports one bounded
HTTP byte range, open-ended ranges, checksum ETags, conditional responses, and
``416`` with the complete object length for invalid or multiple ranges. It
revalidates content type, total length, checksum, ETag, and returned range
before streaming. Responses are authenticated, same-origin, ``nosniff``, and
``private, no-store`` with no origin fallback. Native Axum tests prove poster
delivery plus MP4 range playback; real Firefox restart/play/seek/pause/resume
acceptance remains required by XIMG-096.
