Quick preview and normalized video playback
===========================================

The library opens a selected media card in a focused preview task pane. Media
is always rendered from DASObjectStore and never falls back to an origin URL.
For a thumbnail-only image, an explicit external link may reopen the recorded
source presentation in a new tab so the user's normal Firefox interaction can
request original capture. Pinakotheke never follows that link itself and never
receives site credentials.

Using the preview
-----------------

Each preview names the selected record and keeps the following evidence visible:

* source account/origin label, capture time, media type, ObjectStore state, and
  descriptive alt text;
* a ``Fit to pane`` / ``View original size`` control for the visual area;
* a visible ``Open source image to capture original`` link for a thumbnail-only
  record when safe presentation provenance is available; and
* an explicit ``Object unavailable`` state when an authorized object cannot be
  read.  That state never falls back to the source URL.

An image record that contains a ready DASObjectStore thumbnail but no original
remains viewable: the preview renders that stored thumbnail and states
``Stored thumbnail shown · Original not captured``. If an original reference
exists but is unavailable, the same stored-only fallback is labelled
``Original representation unavailable``. Only when neither representation is
ready does the pane show ``Image object unavailable``. None of these states
request media from the source website.

Opening the source link is an explicit user action. It does not claim capture
success: the card changes to ``Stored in ObjectStore`` only after the Firefox
extension emits an opened-original plan and DASObjectStore verifies settlement.
Historic cards without recoverable presentation provenance state that the
source link is unavailable instead of inventing one.

The dedicated **Playable videos** browse context filters at the server before
pagination. New normalized video cards show duration and codec families at a
glance. The preview adds exact dimensions, versioned playback profile, and the
worded ``Ready · Firefox verified`` normalization state. Legacy video records
without the additive metadata remain readable but do not invent evidence.

Normalized video
----------------

Only a record marked ``Stored in ObjectStore`` with a verified normalized
playback identifier receives a native HTML video control.  Its URL is the
host-authenticated x-img route:

.. code-block:: text

   /products/pinakotheke/api/gallery/v1/objects/{catalogue_id}/video

A WebP poster is extracted from each newly acquired video under a bounded
FFmpeg timeout and committed as a separate checksum-verified DASObjectStore
object delivered through the corresponding ``thumbnail`` role. Poster availability never
determines video availability: the status and provenance shown for a normalized
video come from its committed video representation. The video route preserves
the verified MIME type, ETag, and a single byte range; see
:doc:`direct-playback`. A video without a ready ObjectStore rendition remains
visibly unavailable or non-playable. The Web client does not supply an origin
URL as a backup.

Keyboard behavior
-----------------

Opening a card moves focus to the preview.  ``Tab`` and ``Shift+Tab`` cycle
through preview actions, ``Escape`` closes the pane, and closing returns focus
to the selected card.  The pane uses words, not colour alone, for object and
delivery state.

Verification
------------

The installed-Firefox gallery harness builds and opens the real Yew bundle. It
proves the dedicated video filter and metadata, bounded virtual window,
keyboard focus, unavailable state, responsive layout, and that unavailable
media causes no request to an origin website. The playback harness accepts only
an ephemeral normalized MP4 and proves metadata load, seeking, concurrent and
conditional ranges, cancellation, pause/resume, and missing-object recovery.
It removes its Firefox profile and retains no media.

.. code-block:: console

   cargo +1.97.0 test -p pinakotheke-web
   cargo +1.97.0 check -p pinakotheke-web --target wasm32-unknown-unknown
   make firefox-gallery-check
   make firefox-playback-check VIDEO=/ephemeral/normalized.mp4
   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check

Compatibility-sensitive sources reviewed: Mnemosyne design language
``fbfa28e55d1c8111ef95a139d83927c231534b5f``, Monas
``dac0e113c8b197cb06abc38187d72f27e562ad63``, DASObjectStore
``27ae0d9e936a68b5cd5783b44725d709e1ba665e``, and Mnemosyne
``2244a49f5057ef6251b2760bd0729de8e2207f56``.
