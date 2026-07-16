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

Packaged host command
---------------------

The first runnable host boundary is:

.. code-block:: console

   pinakotheke video normalize \
     --plan /absolute/private/confirmed-plan.json \
     --docker /absolute/reviewed/docker \
     --ingest-helper /absolute/reviewed/das-stream-helper

The plan must be a strict mode-``0600``
``pinakotheke.video-normalize-plan.v1`` document. It fixes the confirmed job,
source identity, playback profile and codec variant, endpoint plus ObjectStore,
actor, paired device, immutable container digest, resource bounds, three
derived object keys, and one mode-``0700`` scratch directory below the system
temporary root. That directory must contain exactly one non-empty regular
``input.media`` file. Pre-existing outputs, extra files, and symlinks are
rejected before Docker runs.
The public JSON shapes are
``contracts/dasobjectstore/pinakotheke-video-normalize-plan.v1.schema.json``
and
``contracts/dasobjectstore/pinakotheke-object-ingest-stream.v1.schema.json``.

The Docker and ingest-helper paths must be absolute executable regular files,
not symlinks. Pinakotheke invokes Docker with structured, network-isolated
arguments. For each normalized video, poster, and provenance manifest, it
starts the reviewed ingest helper and writes one bounded JSON header followed
by the declared payload bytes on stdin. The helper owns DASObjectStore
authentication and must return one strict
``pinakotheke.object-ingest-stream.v1`` verified receipt. A changed endpoint,
ObjectStore, key, length, checksum, object reference, schema, failed process,
or response over 16 KiB fails the job. Helper stderr is suppressed, unfinished
children are killed, and the scratch tree is removed on every outcome.
The helper boundary was reviewed against DASObjectStore commit
``093772da79bbb494da070965c7d4f49e5ad83f56``: the daemon remains authoritative
for scoped application identity, quota, provider verification, catalogue
publication, capability replay, and the final completion decision.

This command makes the normalization adapter deployable, but does not itself
admit a gallery card. The host must still record successful Firefox playback
evidence and pass the normalized-video admission boundary. The live XIMG-096
run must use the selected DASObjectStore rather than a fixture helper.

States and recovery
-------------------

The user sees explicit states: ``Planned``, ``Normalizing``, ``Probing``,
``Ingesting``, ``Awaiting Firefox playback``, ``Cancelled``, ``Failed``, or
``Reconciliation required``.  Cancellation kills the running container and
then cleans scratch.  After a crash, an unfinished job is not assumed committed
and is never resumed from stale local bytes; it moves to ``Reconciliation
required``.  A retry must create a new authorized attempt after revalidating
policy, destination, quota, pairing, image digest, and source identity.
The worker emits the Normalizing, Probing, Ingesting, and terminal state events
through a host progress sink so the Monas task pane can report progress without
receiving payload bytes or container logs.

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

The repository contains no live website adapter, real user media fixture,
production DASObjectStore credential, or default container image. Deployments
must review their stream helper, registered image digest, pairing, and granted
ObjectStore scope before use.
