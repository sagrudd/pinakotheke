Paired-device Docker video normalization
========================================

The first implemented normalization worker runs only on an explicitly paired
device that the user has approved for x-img.  It is a worker on the computer
that may run Firefox, not Firefox itself: the browser never stores video
payloads, provides cookies, or starts a container.  DASObjectStore remains the
durable authority for all source and derived objects.

Before a job starts, x-img requires a confirmed candidate, a writable reviewed
endpoint plus ObjectStore, a selected versioned playback profile, a current
paired-device reference, and a Docker image reference with an immutable
``sha256`` digest.  A mutable tag, an arbitrary host, a direct local folder,
or an unpaired device is rejected.  The installation/operator must register an
approved FFmpeg image with its licence and profile evidence; this public
repository deliberately does not treat a floating public image tag as a
production default.

Worker behavior
---------------

The worker creates one bounded isolated ephemeral scratch directory outside the
x-img product root.  An authorized transfer worker may place the selected
source there, then x-img invokes Docker directly with an argument vector—never
a shell command.  The container has no network, a read-only root filesystem,
all Linux capabilities dropped, ``no-new-privileges``, process/CPU/memory
limits, a temporary filesystem, and only the isolated scratch directory
mounted at ``/work``.

The pinned FFmpeg container produces a normalized rendition and a poster.  A
pinned containerized FFprobe invocation validates projected codec, dimensions,
and duration metadata.  x-img calculates the output SHA-256 using bounded
reads, then streams the rendition, poster, and a small provenance manifest
through the authorized DASObjectStore ingest port.  It does not retain a copy
after the verified ingest.  Scratch is deleted after both success and failure.

States and recovery
-------------------

The user sees explicit states: ``Planned``, ``Normalizing``, ``Probing``,
``Ingesting``, ``Awaiting Firefox playback``, ``Cancelled``, ``Failed``, or
``Reconciliation required``.  Cancellation kills the running container and
then cleans scratch.  After a crash, an unfinished job is not assumed committed
and is never resumed from stale local bytes; it moves to ``Reconciliation
required``.  A retry must create a new authorized attempt after revalidating
policy, destination, quota, pairing, image digest, and source identity.

Successful normalization stops at ``Awaiting Firefox playback``.  It is not
catalogue-ready until XIMG-069 proves authorized Firefox MIME/range playback.
DRM-protected, blocked, failed, and unsupported media remain explicit states;
the worker does not fall back to provider playback or circumvent protection.

Verification
------------

Native tests use synthetic byte fixtures and a fixture Docker runtime to prove
the structured invocation, bounded streaming, provenance, cancellation, crash
reconciliation, idempotency, and cleanup boundaries.  The local documentation
authority remains:

.. code-block:: console

   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check

The implementation has no live website adapter, no real user media fixture,
and no production DASObjectStore or pairing credential.  Deployments must test
their registered image digest and granted ObjectStore scope before use.
