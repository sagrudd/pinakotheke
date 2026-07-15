Segmented video capability gate
===============================

HLS and DASH are not progressive MP4 files.  Their manifests, segments,
encryption, Media Source Extensions behavior, and changing signed URLs require
an exact site adapter.  x-img therefore leaves segmented video origin-served
unless every capability proof below is present.  The generic adapter has no
segmented capability and cannot acquire one from a successful lookup.

Required adapter evidence
-------------------------

The server-side ``x-img.segmented-video-gate.v1`` decision requires:

* an exact HTTPS origin, adapter ID, and semantic adapter version;
* an explicit HLS or DASH manifest kind;
* separate semantic versions for manifest and segment canonicalization;
* a synthetic, redistributable fixture evidence ID;
* real Firefox evidence for the exact adapter behavior;
* an explicitly opened, user-displayed candidate;
* no DRM or encryption; and
* a ``Ready`` normalized Pinakotheke playback profile matching the evidence.

Missing, malformed, stale, or mismatched evidence produces ``Origin served``.
It never selects another adapter, endpoint, ObjectStore, source rendition, or
credential path.  Approval is only the policy gate for a future exact adapter;
it is not a generic manifest rewriter and does not itself fetch media.

Firefox behavior
----------------

On an enabled video site, a toolbar action identifies visible manifest URLs by
``.m3u8`` or ``.mpd`` and treats blob/MSE sources conservatively.  Because the
current generic adapter declares no segmented proof, the extension does not
query a delivery alias, rewrite a manifest, inspect segments, read cookies, or
change the video element.  Settings show a bounded diagnostic such as
``Origin served — HLS substitution requires a proven site adapter``.  It stores
only the already-enabled origin and coarse reason, never a page/media URL or
signed query.

Progressive normalized MP4 remains supported through :doc:`mp4-substitution`.
DRM/encrypted, unobserved, unopened, source-only, failed, or unverified media
always stays with the origin.

Verification
------------

.. code-block:: console

   cargo +1.97.0 test -p x-img-core segmented_video
   node --check firefox-extension/background.js
   node --check firefox-extension/options.js
   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check

Compatibility-sensitive contracts were inspected at DASObjectStore
``42bf66a7494f4e0aa81f103100b71489b38164dc``, Monas
``3d21b0bc7b83fa8408d01b93347a56f43f3a96b7``, Mnemosyne design language
``5539df8f662a78ebdf7cf4c868d71831380c8cfd``, and future Synoptikon
``52810176bf95a170f93d74a6f5daa94da5c6640e``.  No unpublished path dependency
is used.
