Direct normalized-video playback
================================

Direct playback is a host-authenticated x-img delivery path for a verified,
normalized ObjectStore rendition.  It is separate from Firefox website-cache
substitution: a user may play a committed video in the x-img/Monas workspace
without enabling any third-party-site interception.  Later cache work may reuse
this delivery contract, but cannot gate it or change its authority checks.

Authorization and readiness
---------------------------

The host supplies the authenticated actor context.  A playback grant binds that
actor to one stable endpoint, ObjectStore, object key, checksum, and total
length.  The grant is rejected if the actor differs, the playback ID is
unknown, or the rendition is not ``Ready``.  ``Source selected``,
``Normalizing``, ``Awaiting Firefox playback``, ``Blocked``, and ``Failed``
video records have no direct playback path.  x-img never substitutes an origin
URL when an ObjectStore object is unavailable.

HTTP delivery contract
----------------------

The delivery boundary maps a grant to the existing authorized DASObjectStore
read port.  It preserves the verified media content type, exact length, quoted
checksum ETag, ``Accept-Ranges: bytes``, and a single valid byte range.  It
supports conditional requests using the same ETag.  Multiple ranges and invalid
or unsatisfiable ranges are rejected rather than assembled into an unbounded
multipart response.

The Axum host adapter accepts only a server-side scoped DASObjectStore read
callback and a host-injected Monas context.  It emits the authorized object
stream at ``GET /api/playback/v1/{playback_id}``; it has no origin URL or
fallback behaviour.  The host terminates HTTPS and owns the authenticated DAS
transport.  The authoritative DASObjectStore read wire route remains a
separate versioned integration concern, so the public x-img build carries no
sibling path dependency.

Real Firefox evidence
---------------------

Use an ephemeral normalized MP4 from the approved worker or a DAS-managed
staging mount; do not copy it into this repository or a browser profile:

.. code-block:: console

   python3 scripts/firefox/check_normalized_playback.py --video /ephemeral/normalized.mp4

The local harness starts a temporary loopback presenter, makes Firefox request
the same MIME/range shape as the delivery route, and proves metadata, a byte
range, seek, pause, and resume.  It deletes its temporary profile when it
finishes.  HTTP authorization is proven independently by the Axum route test;
the browser never receives a Monas or DAS credential.

Verification
------------

.. code-block:: console

   cargo +1.97.0 test -p x-img-core playback_delivery
   cargo +1.97.0 test -p x-img-api direct_playback
   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check

Compatibility-sensitive sources reviewed for this boundary: Monas
``3d21b0bc7b83fa8408d01b93347a56f43f3a96b7``, DASObjectStore
``13a893d52556520dc61ebb800a39a971058f6d66``, Mnemosyne design language
``5539df8f662a78ebdf7cf4c868d71831380c8cfd``, and Mnemosyne
``52810176bf95a170f93d74a6f5daa94da5c6640e``.
