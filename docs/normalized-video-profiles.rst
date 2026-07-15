Normalized video profiles and execution placement
=================================================

x-img has two immutable candidate playback profiles.  Neither is a default
merely because it is declared: a profile becomes usable for a rendition only
after the required Firefox, hardware/software, encoder, quality, storage, and
licensing evidence is recorded.

.. list-table:: Candidate profiles
   :header-rows: 1

   * - Profile ID
     - Container and codec variants
     - Advertised MIME type
   * - ``pinakotheke-video-webm-v1``
     - WebM VP9/Opus or AV1/Opus
     - ``video/webm``
   * - ``pinakotheke-video-mp4-v1``
     - MP4 H.264/AAC
     - ``video/mp4``

The variants are codec contracts rather than website-specific claims.  A new
combination first becomes a redacted aggregate codec-gap record; it is not
silently coerced into an existing profile.  This lets prioritisation respond to
unusual real-world codecs without adding website names, user URLs, titles,
media bytes, cookies, credentials, or browsing history to documentation,
fixtures, or tickets.

Docker-first normalization workers
----------------------------------

Normalization is a separate, pinned Docker-container job.  The plan names an
approved image reference and an immutable ``sha256`` image digest, plus strict
CPU, memory, and scratch limits.  A later adapter must pass structured FFmpeg
arguments to that container; it must not interpolate a shell command or accept
an image tag such as ``latest``.

The plan also names one authorized execution placement:

* a DASObjectStore-hosted executor, using DAS-managed staging;
* a paired Firefox-device worker, using only bounded ephemeral scratch with a
  cleanup record; or
* a future Keryx-dispatched worker with an approved dispatch and worker
  reference, also using only bounded ephemeral scratch or DAS-managed staging.

The execution location is not a storage authority.  All durable source and
derived bytes remain DASObjectStore objects.  Ephemeral worker scratch is
deleted after success or failure, and a device must never treat browser storage
or an arbitrary local folder as staging.  A Keryx reference is intentionally
opaque in x-img: it preserves a future governed-worker integration without a
Keryx path dependency or a claim that x-img schedules Keryx workloads today.

Readiness and provenance
------------------------

Each normalized-video record binds its stable source identity, profile ID,
reviewed endpoint and ObjectStore, executor image digest and placement, scratch
authority, source-retention decision, and separately typed DASObjectStore
objects.  Those objects are the normalized video, poster, optional subtitles,
optional storyboard, and provenance manifest.

``Ready`` requires a checksummed normalized video, poster, provenance manifest,
matching profile evidence, and a Firefox-playback evidence reference.  A
source-only object cannot be marked ready.  The actual containerized conversion
is XIMG-068; its real Firefox range/playback proof is XIMG-069.  Until those
steps complete, the UI must use explicit ``Source selected``, ``Normalizing``,
``Blocked``, ``Failed``, or ``Awaiting Firefox playback`` state words.

Verification
------------

Build the user documentation locally in its pinned container:

.. code-block:: console

   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check

Compatibility-sensitive sources reviewed: Monas
``3d21b0bc7b83fa8408d01b93347a56f43f3a96b7``, DASObjectStore
``7fec43a2df0c080f063a6702d5f900d7bc440491``, Mnemosyne design language
``5539df8f662a78ebdf7cf4c868d71831380c8cfd``, Mnemosyne
``52810176bf95a170f93d74a6f5daa94da5c6640e``, and Keryx
``9506acb72f2cca795ae11b9c90cc3cacd96244a6``.  The public build has no path
dependency on those checkouts.
