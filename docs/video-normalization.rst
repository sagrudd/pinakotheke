Paired-device Docker video normalization
========================================

Site-neutral trusted-play ingress
---------------------------------

Firefox site rules are intentionally origin-based rather than a compiled
catalogue of services. A user must explicitly enable video capture for an HTTPS
origin. Pinakotheke considers a candidate only when a recent trusted pointer or
keyboard activation is followed by playback; merely loading, autoplaying, or
scrolling past a video does not acquire it. The extension never forwards site
cookies, authorization headers, passwords, or browser history.

A concrete progressive HTTPS MP4 can enter the bounded acquisition worker. If
FFprobe proves the browser profile (H.264 video with absent or AAC audio), the
object is committed through the authoritative DASObjectStore completion path.
An unfamiliar container or codec is a normalization requirement, not a failed
permission check and never a reason to store an unverified local copy. The UI
reports the codec/container tuple without retaining the source URL so operators
can prioritize profiles without collecting browsing history.

Segmented HLS/DASH, Media Source Extensions, encrypted media, and blob-only
playback remain origin-served until a generic adapter proves a bounded,
user-initiated candidate plan. DRM or access-control circumvention is never an
adapter capability.

For progressive video, the acquisition helper now performs the automatic
handoff. It probes a bounded ``video/*`` response and uses the direct commit
path only for MP4 H.264 with absent or AAC audio. Any other safe codec tuple is
written to a private aggregate codec-gap journal containing only container,
video codec, audio codec, count, and time—never the site, page, media URL,
title, cookie, or credential. The source is renamed into one mode-``0700``
bounded DAS-managed staging directory, and the configured DASObjectStore-host worker
invokes the existing digest-pinned, network-isolated adapter. The original is
not committed by this path.

The adapter commits the normalized MP4, WebP poster, and JSON provenance
manifest as three independently checksummed DASObjectStore objects. Its
manifest records the playback profile, pinned image digest, output codecs,
dimensions, duration, resource settings, and video/poster checksums. The
capture result can enter the gallery only when all three commits verify, the
container probe matches the H.264/AAC profile, and the operator configuration
names reviewed Firefox playback evidence for that exact profile. Failure keeps
the capture pending, removes scratch, and never labels the source as stored or
playable.

The first implemented normalization worker runs only on an explicitly paired
device that the user has approved for x-img.  It is a worker on the computer
that may run Firefox, not Firefox itself: the browser never stores video
payloads, provides cookies, or starts a container.  DASObjectStore remains the
durable authority for all source and derived objects.

Before a job starts, x-img requires an explicitly played candidate, a writable reviewed
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

On Linux the structured ``--mount`` bind is writable by default and does not
use the volume-only ``,rw`` token. The container runs as the scratch
directory's numeric UID:GID so it can read mode-``0600`` input and create
outputs after all capabilities have been dropped. Pinakotheke does not widen
the mode-``0700`` directory, add DAC capabilities, or run a container entrypoint
that requires root-time user or group mutation.

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
     --ingest-helper /absolute/path/to/pinakotheke

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

First-party DASObjectStore stream helper
----------------------------------------

The packaged ``pinakotheke`` executable also implements the hidden
``ingest-stream-v1`` protocol consumed by ``video normalize``. Point
``--ingest-helper`` at the same reviewed canonical executable and set
``PINAKOTHEKE_DAS_STREAM_HELPER_CONFIG`` to an absolute private mode-``0600``
``pinakotheke.das-stream-ingest-helper.v1`` configuration. Its public schema is
``contracts/dasobjectstore/pinakotheke-das-stream-ingest-helper.v1.schema.json``.

The configuration chooses exactly one authority transport:

* native ``dasobjectstore-remote`` executable, private remote configuration,
  and absolute daemon socket; or
* the fixed ``dasobjectstored`` Compose service with reviewed Docker/Compose,
  canonical DAS-managed scratch roots, private remote/AWS files, and the fixed
  container daemon socket.

The helper reads a bounded JSON header and exactly the declared payload bytes
from stdin, computes SHA-256 while writing only private ephemeral scratch,
rejects early EOF, trailing bytes, changed authority, unsupported MIME, unsafe
keys, and objects over the configured cap, then invokes the existing daemon
completion path. It emits a receipt only after DASObjectStore reports
``Complete`` at ``remote_s3_transfer_complete``. Scratch, copied scoped
credentials, and payload are removed on success or failure. Supported derived
types are normalized MP4/WebM, WebP posters, and JSON provenance manifests.

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

The packaged ``pinakotheke ingest-stream-v1`` command is the first-party
implementation of that helper boundary. Its private configuration uses schema
``pinakotheke.das-stream-ingest-helper.v1`` and chooses exactly one execution
mode:

* native mode pins the ``dasobjectstore-remote`` executable, its current JSON
  remote-client grant, and the daemon socket; or
* container mode pins the Docker executable, private Compose file, matching
  host/container scratch roots, current JSON remote-client grant, AWS shared
  credentials file, service name, and container daemon socket.

The remote-client grant is not the daemon's legacy TOML storage-backend file.
For container execution its endpoint must resolve inside the selected Compose
network, and its writable grant must map the stable ObjectStore ID to the DAS
bucket. The credentials and grant are copied mode-``0600`` into one job scratch
directory and removed on every result. The helper accepts only MP4, WebM, WebP,
or JSON, reads exactly the declared byte count, rejects trailing or
checksum-mismatched input before authority invocation, and emits a receipt only
after ``dasobjectstored`` reports ``remote_s3_transfer_complete``.

This command makes the normalization adapter deployable, but does not itself
admit a gallery card. The host must still record successful Firefox playback
evidence and pass the normalized-video admission boundary. An isolated live
XIMG-096 proof against DASObjectStore commit
``28e6d82cc8c25dd83838fde8b6de3aa16384eb95`` on an x86_64 Linux DASServer
normalized a redistributable three-second test pattern, committed its 129,594
byte MP4, 4,086 byte WebP poster, and 428 byte JSON manifest through the
packaged helper, then independently confirmed all three ObjectStore content
types. FFprobe confirmed H.264/AAC, 320 by 240 pixels, and 3.041 seconds. A DGX
Spark GB10 separately ran the same hardened worker and all three bounded helper
streams on arm64, using a locally registered digest-pinned wrapper around
static FFmpeg 8.1.2; scratch cleanup was verified. The DGX run used a fixture
completion authority, not DASObjectStore, because the current helper requires
a local daemon socket. Persistent gallery admission and real Firefox playback
remain required.

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
