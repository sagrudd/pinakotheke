Critical Firefox capture-to-gallery acceptance
==============================================

XIMG-104, XIMG-108, and XIMG-096 are one assembled product path rather than
three independent demonstrations. The repeatable local gate is:

.. code-block:: console

   make critical-vertical-check VIDEO=/ephemeral/normalized.mp4

``VIDEO`` must be a temporary redistributable H.264/AAC MP4 produced by the
reviewed normalization worker or an equivalent isolated fixture. The command
does not copy it into the repository or Firefox profile.

What the gate proves
--------------------

Installed Firefox uses the production extension scripts on a temporary HTTPS
origin. An opted-in visible thumbnail is submitted automatically. A real
pointer action opens the image and a separate trusted play action submits the
progressive video. The isolated server emulates the production ``stored``
response; the native and live-authority checks below prove the settlement and
gallery-admission conditions that authorize that response in production.
Firefox then applies the extension-only two-pixel green frame to the thumbnail
and video. No cookie, authorization header, credential, or payload is sent in a
capture plan, and stored media bytes are never modified.

The native portion proves that independently verified completion traverses the
acquisition and reconciliation state machines, enters the common ``New``
catalogue, persists endpoint, ObjectStore, immutable object version, checksum,
source/account provenance, review state, poster, and complete video metadata,
and converges after restart. Gallery image, poster, and video routes remain
Monas-authenticated and never contain an origin fallback.

The real Yew application is then exercised in installed Firefox with 1,000
mixed records. The run covers the Playable videos context, metadata, committed
poster/unavailable states, bounded virtualization, keyboard traversal, server
filters, and desktop/mobile layout. Finally Firefox loads the ephemeral MP4
through the single-range presentation and proves metadata load, play, seek,
pause/resume, concurrent ranges, conditionals, cancellation, and missing-object
recovery.

Live authority evidence
-----------------------

The local gate intentionally does not embed DAS credentials or retain media.
Its authority half is backed by the clean-home XIMG-094 run against
DASObjectStore ``f195c4d5a30d1cc34ca61f31a6939edf54db782f``: a daemon-verified
commit and checksum-identical scoped read survived restart and reconciliation.
Subsequent DASServer runs proved automatic X thumbnail/original admission,
``x.com/<account>/<capture-kind>`` keys, authoritative catalogue completion,
and separately committed normalized MP4, WebP poster, and JSON provenance
objects. XIMG-110 replaced the old X-only progressive restriction with the
stronger site-neutral rule: the exact HTTPS origin must be explicitly enabled,
the play must follow a recent trusted activation, and no browser credential is
forwarded. X continues to receive its account folder provenance.

Failure behavior
----------------

Autoplay, synthetic activation, unobserved media, DRM, segmented media without
a proven adapter, unsupported or unnormalized codecs, changed destinations,
unverified commits, unavailable objects, and partial worker failure never
produce a Stored claim. The page remains origin-served during capture failure;
the Pinakotheke viewer itself never contacts the source site as a fallback.

Documentation authority
-----------------------

Build and verify this guide with the repository's pinned local container:

.. code-block:: console

   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check

Compatibility-sensitive sources were reviewed at Mnemosyne design language
``fbfa28e55d1c8111ef95a139d83927c231534b5f``, Monas
``799484eeb1f6d324500f8ed59bed8e43deed7be5``, DASObjectStore
``8afcfb487120f5fa9d0431b3ae8ce0fc4a42af37``, and Mnemosyne
``52810176bf95a170f93d74a6f5daa94da5c6640e``.
