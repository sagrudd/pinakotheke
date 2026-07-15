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

The current Rust core proves these rules with synthetic authority responses;
the host HTTPS route and real Firefox range playback are the remaining XIMG-069
proof.  No real video, user URL, cookie, credential, or source fallback is in
the fixtures.  The authoritative DASObjectStore read wire route remains a
separate versioned integration concern, so the public x-img build carries no
sibling path dependency.

Verification
------------

.. code-block:: console

   cargo +1.97.0 test -p x-img-core playback_delivery
   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check

Compatibility-sensitive sources reviewed for this boundary: Monas
``3d21b0bc7b83fa8408d01b93347a56f43f3a96b7``, DASObjectStore
``13a893d52556520dc61ebb800a39a971058f6d66``, Mnemosyne design language
``5539df8f662a78ebdf7cf4c868d71831380c8cfd``, and Mnemosyne
``52810176bf95a170f93d74a6f5daa94da5c6640e``.
