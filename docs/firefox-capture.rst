Viewed-media capture plans
==========================

X video permission
------------------

X video ingress needs Firefox access to the exact
``https://video.twimg.com/*`` media origin so the extension can observe a
worker-fetched manifest URL. Pinakotheke requests this permission only when the
user enables X ingress with videos selected. Existing installations show
``Permission required`` in the toolbar; choose ``Check visible media`` and
approve the Firefox prompt, reload X, and play the video again. Denial is
fail-open: playback remains served by X and no capture is claimed.
The extension determines whether this permission is missing while rendering
the toolbar. The subsequent request starts directly in the button handler,
before any awaited work, because Firefox preserves approval eligibility only
for the immediate user action.

Lookup and explicit acquisition
-------------------------------

An enabled exact-origin rule permits lookup for its selected media classes.
Firefox observes meaningful images currently displayed in the viewport after
page load, scrolling, or DOM changes only to ask whether their canonical media
identities are already stored. Observation never creates a capture plan,
gallery record, or ObjectStore payload. Image acquisition requires a trusted
user open, and video acquisition requires trusted user-started playback.

The plan response is not storage evidence. Firefox polls the actor-scoped
``GET /products/pinakotheke/api/extension/v1/capture-plans/{plan_id}`` status.
Only ``state: stored``—which requires a settled plan and matching live gallery
record—adds the browser-only ``pinakotheke-stored-object`` class. That class
draws a two-pixel green border with words retained in toolbar diagnostics; it
never alters the downloaded or DASObjectStore-managed bytes. Pending,
unavailable, rejected, and timeout states remain unframed and origin-served.

Evidence lookup is independent of substitution. A user may leave origin
delivery enabled while Pinakotheke asks its paired server whether a displayed
alias is already settled. A positive authoritative answer adds the same
two-pixel green frame. User-selected videos receive a browser-only status
graphic over the playing element while acquisition runs and the green stored
frame after settlement. The graphic says ``Video is downloading``, shows
measured progress when the origin exposes a reliable response length,
identifies normalization and ObjectStore commit phases, and ends with ``Video
committed to DASObjectStore``. Segmented media without a trustworthy total
uses honest phase progress rather than a fabricated byte percentage. These
overlays and frames remain browser-only.

A navigation-page poster may be an ``img`` even though its presentation page
identifies a previously committed video. The same bounded evidence batch sends
both the image alias and presentation identity; a settled normalized video
therefore adds the two-pixel green frame to that poster on later visits without
reading or listing DASObjectStore.

Cache evidence never performs an ObjectStore read or listing. DASObjectStore
remains authoritative for committed bytes, while Pinakotheke keeps settled
capture and catalogue metadata in process memory. Each viewport scan submits
one authenticated batch of up to 256 canonical media identities. Its single
response drives both stored framing and optional substitution; a miss or an
unavailable batch fails open to the website. The legacy single-identity route
remains available only for extension-version compatibility during rollout.

The extension never submits an observed-thumbnail acquisition plan. Viewport equality uses
canonical media identities rather than X's disposable DOM tokens, so recycling
an unchanged element cannot create background work. If a compatible client
still repeats a settled capture request, the server returns the stable plan and
stored status without requeueing acquisition or ObjectStore settlement.

Long X timelines reuse and temporarily disconnect image elements. The
extension therefore remembers up to 4,096 server-confirmed canonical image
identities for the current browser process and repairs the green frame when the
same identity reappears. A different identity clears the frame. This metadata
cache contains no image bytes and does not survive as an independent storage
authority. One viewport batch covers up to 64 visible images, and redacted
diagnostics report only the number of stored hits versus requested identities.

Deep X timelines may replace an image element between pointerdown and click.
Pinakotheke snapshots the exact eligible image identity, original-rendition
URL, dimensions, presentation reference, and ephemeral media token on trusted
pointerdown. Only the corresponding click within two seconds may consume that
snapshot. It contains no bytes, cookies, or authorization headers and cannot
make synthetic or unrelated input eligible. An unresolved trusted click is
recorded as a redacted diagnostic rather than being mistaken for an
ObjectStore failure.

Clicking an eligible linked image retains the stricter explicit-original path
and uses the same verified status before framing. Firefox sends the rendered
image URL as the byte source and records an enclosing link, such as an X status
page, only as presentation provenance. This prevents an HTML page from being
mistaken for image content.

The authenticated library page polls
``GET /products/pinakotheke/api/ingestion/v1/status`` every three seconds. Its
Ingress status strip reports actor-scoped observed thumbnails, explicitly
opened images and videos, pending acquisitions, verified stores, and the live
gallery size. ``Pending`` means that a plan has been durably accepted but has
not yet passed ObjectStore commit verification and gallery admission; it is
not evidence that bytes were stored.

Capture latency
---------------

Pinakotheke reserves separate bounded acquisition capacity for observed
thumbnails and explicitly opened images or videos. A busy scrolling page must
therefore not queue an intentional original behind its thumbnail backlog.
Firefox checks the verified plan state every 100 milliseconds during the normal
two-second settlement window, backs off to 250 milliseconds for the following
five seconds, and then checks once per second. The selected/downloading frame
may appear immediately, but the green frame is still applied only after the
DASObjectStore commit is verified and the gallery record is live.

ObjectStore layout
------------------

The acquisition helper commits X image objects under the logical key
``x.com/<account>/<capture-kind>/<sha256>``. The account is taken from the
canonical X status-page presentation URL, not guessed from the image CDN URL.
Handles are normalized to lowercase. If an observed CDN image has no account
link, Pinakotheke uses the explicit ``x.com/_unattributed/...`` quarantine
prefix rather than assigning it to the wrong creator. Generic enabled sites
use ``sites/<site-id>/<creator-or-_unattributed>/<capture-kind>/<sha256>``.
Firefox supplies only a bounded semantic author/uploader label exposed by the
enabled page; the server validates it and the helper converts it to a safe
folder segment. It never guesses an identity from an unrelated CDN hostname.
These are DASObjectStore keys,
not unmanaged local directories, and the checksum suffix preserves idempotent
byte identity.

X image CDN URLs require their public ``format`` and ``name`` variant
parameters to resolve. Pinakotheke preserves only those two bounded
alphanumeric parameters and discards every other query parameter. Generic
media URLs continue to lose their complete query and fragment. Capture does
not depend on the substitution-specific instance identifier, and Firefox
reports capture and substitution results separately so an origin-served video
does not overwrite image-ingress evidence.

Ingress diagnostics
-------------------

The Firefox toolbar exposes the twelve newest diagnostic events and can
download a bounded ``pinakotheke.extension-diagnostics.v1`` JSON document of at
most one hundred events. Events identify observer registration, viewport scan
counts, rule/adapter/pairing skips, capture-plan HTTP outcomes, and
stored/pending polling outcomes. They contain the enabled origin but never a
page URL, media URL, cookie, authorization header, pairing reference, password,
or site browsing history. Diagnostic failure never blocks browsing.

The Pinakotheke service emits single-line ``pinakotheke_ingress`` journal
records for ``plan_admitted``, ``acquisition_failed``, ``settlement_failed``,
and ``gallery_admitted``. Identifiers, capture class, configured origin, and a
bounded helper error class are retained; source URLs and credentials are not.
Helper failures use only ``policy-blocked``, ``unavailable``, or ``rejected``;
raw downloader messages are excluded because they can contain signed URLs or
local paths.
On the DASServer inspect them with:

.. code-block:: console

   journalctl -u pinakotheke-preview.service --since "10 minutes ago" \
     --no-pager | grep pinakotheke_ingress

A trusted video ``play`` gesture is detected. A trusted pointer or keyboard
activation must be followed by playback, and Firefox must have exposed an
HTTPS progressive response or an already-observed HLS/DASH manifest in that
activation window. The URL may have an opaque path and a short-lived query; no
website catalogue is traversed. That candidate is routed through the capture
worker, profile-verified, normalized when required, committed, admitted, and
reported through verified progress status. Blob-only playback with no bounded
retrieval candidate remains origin-served and produces a redacted diagnostic;
it is not silently described as captured.

XIMG-064 adds the first server-side admission boundary for Firefox observed
media. It is deliberately a **capture plan**, not a browser upload or a
committed catalogue item. Durable bytes remain with DASObjectStore, and a plan
can proceed only through a future approved acquisition worker, ObjectStore
verification, reconciliation, and review admission.

Installed Firefox acceptance
----------------------------

On macOS, ``make firefox-capture-check`` starts the installed Firefox binary
with an isolated temporary profile and an ephemeral HTTPS gallery. It installs
an unsigned temporary test copy of the extension and uses the production
background and content scripts. The check requires a displayed thumbnail to
remain lookup-only, a trusted opened original, and a trusted-play progressive video. It
returns verified stored status and proves that Firefox applies the two-pixel
frame to the matching image and video. Capture requests contain no payload,
cookie, or header fields. The certificate, profile, add-on, and synthetic media
are removed at the end of the run.

The test copy is instrumented only to pre-authorize its ephemeral loopback
origin and invoke the same action used by the toolbar; it is not release-XPI
signature evidence. Firefox temporary extension installation is provided by
the `WebDriver BiDi extension commands
<https://firefox-source-docs.mozilla.org/remote/webdriver-bidi/Extensions.html>`_.
The shipped manifest declares both ``background.scripts`` and
``background.scripts`` because Firefox uses that Firefox-compatible form; see the
`Firefox background manifest reference
<https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/background>`_.

Firefox match patterns do not encode a port. Pinakotheke therefore requests
the HTTPS host pattern needed by Firefox while retaining and comparing the
complete origin, including its port, in site policy, capture requests, and
provenance. Enabling one origin does not create a policy entry for another
port.

Eligibility
-----------

The Firefox extension considers only complete, visible ``img`` elements that
intersect the current viewport when building a bounded cache-evidence batch.
It does not submit those elements for acquisition. A trusted user open submits
one eligible image; trusted user-started playback submits one eligible video.
It does not inspect off-screen images, traverse hidden DOM content, crawl a
page, or simulate browsing.

After a site is explicitly enabled for image capture, Firefox dynamically
registers one persistent, top-frame content script for that exact origin. The
registration is restored after extension updates and browser startup, and is
removed before capture is paused or the origin permission is revoked. It does
not require the user to press the toolbar cache control on every page and it
does not add tab, history, or cookie permission.

The signed extension declares only the exact ``https://video.twimg.com/*``
media-host permission for its obligate X-video adapter. A non-blocking
``webRequest.onCompleted`` listener retains at most 32 completed HLS/DASH
manifest URLs for two minutes. It records no headers or bodies and discards
them unless the ``https://x.com`` video rule is enabled; when Firefox supplies
an owning tab, that tab must also be on X. The list is consulted only after a
trusted visible play. This supplies
manifests fetched by workers that are absent from page
Resource Timing while preserving the existing server-side policy and bounded
assembly gates.

Firefox can report ``tabId = -1`` for a worker-owned request. Such a manifest is
retained only while the X-video rule is enabled and is correlated after trusted
play by its stable X media-family path. URL-free diagnostics distinguish
``observed``, ``resolved``, and ``missing`` without recording the manifest or
page URL.

After registration, the extension also injects the same idempotent observer
into eligible tabs that are already open. This matters for long-lived
single-page applications: an extension update activates the new observer
without requiring a page reload. Tabs that close or become restricted during
injection are skipped and normal origin loading remains fail-open.

The content script submits an ``explicit_original`` only for a trusted
primary-button click on an image link, or an image already displayed as the
document itself. Unlinked thumbnail clicks, synthetic events, hidden traversal,
subframes, and automatic opening are not eligible. Login and settings paths are
excluded from registration. The background process revalidates current site
policy, adapter capability, and sender-tab provenance for every message, so a
stale page script cannot bypass a newly paused policy. Video acquisition is not
triggered by observation alone. A trusted pointer may target the video, an
enclosing control, or an overlaid control inside the visible video's on-screen
rectangle. That exact video must then emit a genuine play event within two
seconds; unrelated page clicks and autoplay remain ineligible. After a trusted
pointer or keyboard activation causes a visible video to play, the content
script searches only the element's
HTTPS source and recent Resource Timing entries. It polls nine bounded times
over two seconds because script-fetched progressive media may appear after the
``play`` event. A resource is eligible only when Firefox identifies the video
initiator, its path has a progressive video suffix, or the exact X media host
is ``video.twimg.com``. This reads no request headers, cookies, credentials, or
response bytes. A concrete candidate enters the existing ``explicit_video``
plan; blob-only, segmented, encrypted, unresolved, synthetic, stale, autoplay,
or hidden playback remains origin-served with a redacted diagnostic. Bounded
observer outcomes distinguish a missing source from missing trusted activation
without recording the page or media URL.

For clear segmented playback, the observer groups the recent audio/video
fragments by their stable media-family path and selects only an already-observed
matching master ``.m3u8`` or ``.mpd`` manifest. It never submits ``.m4s`` or
CMAF fragments as complete video and never reads cookies, request headers, or
authorization state. The server uses structured arguments to a configured
FFmpeg executable behind a configured process-timeout executable. Acquisition
is bounded to five minutes of wall time, 15 seconds per network read, two hours
of media duration, and the configured video byte cap (one GiB by default).
FFmpeg copies compatible streams into a standalone fast-start MP4; the result
must pass the existing probe and verified DASObjectStore completion before it
can enter the gallery. Encrypted, DRM-protected, authorization-dependent,
unresolved, oversized, or timed-out media remains origin-served or failed with
a bounded diagnostic.

The experimental generic adapter is available for any exact HTTPS origin only
after that origin has been added through the site policy UI and Firefox has
granted its optional permission. Explicit adapters may remain origin-limited.
Login and settings paths remain excluded. This makes user-added websites useful
without granting a wildcard site policy or initiating background crawling.

Host-authenticated endpoint
---------------------------

The standalone test router uses ``/api/extension/v1/capture-plans``. The
runnable Monas host mounts the endpoint at::

   POST /products/pinakotheke/api/extension/v1/capture-plans

The endpoint accepts strict JSON with schema version
``x-img.capture-request.v1``. Its required metadata is an opaque pairing
reference, exact site origin, the current page URL, adapter kind and version,
capture kind, source media URL, and positive dimensions. The stable canonical
identity removes generic query and fragment components. A separately validated
exact HTTPS retrieval URL preserves a short-lived CDN query only inside the
private plan journal and isolated acquisition-helper request; it is omitted
from API responses, catalogue metadata, diagnostics, and logs. A repeated
unsettled identity may refresh this capability without creating another plan.
A linked image may add a presentation URL
that correlates the displayed thumbnail with the original the link opens; it
does not authorize acquisition of anything the user did not observe or open.
It has no payload field, headers field,
cookie field, browser-history list, form field, credential field, or storage
authority field.

Monas or a future Synoptikon adapter must authenticate the request first and
inject an ``AuthenticatedHostContext`` with ``ximg.access``. The server binds
the opaque pairing reference to that host actor, checks expiry/revocation,
requires an enabled exact-origin site policy and matching pinned adapter, and
applies the policy's candidate limit. A pairing reference alone is not an
authentication credential. An unconfigured host returns ``503``; missing host
context returns ``401``; bad pairing/policy returns a non-success response.
The browser ignores all such errors, leaving the source page unchanged.

The extension reads neither Firefox history nor hidden page state: it submits
only the current toolbar-clicked page as capture provenance. The browser asks
Firefox for the exact HTTPS permission of the paired x-img
instance during pairing. Its request may use the browser's normal target-host
session handling so Monas can create host context, but the extension never
reads, extracts, copies, or forwards cookies from the viewed website. It never
adds source authorization headers, form bodies, passwords, or credentials to
the capture request.

Accepted plans are added to the common ``ExtensionCapture`` scheduler lane and
return ``x-img.capture-plan.v1`` metadata with a plan and job identifier. The
returned URL has query and fragment components removed, so signed-query values
cannot enter plan metadata or diagnostics. A plan state of
``awaiting_approved_acquisition`` is explicitly not an ObjectStore commit and
must not be shown as a stored original or admitted to review.

After a future acquisition has verified and committed an ObjectStore object,
the metadata can enter the common review queue through
:doc:`website-capture-review`. A capture plan never bypasses that boundary.

Runnable monolith configuration
-------------------------------

The local monolith mounts capture planning at
``/products/pinakotheke/api/extension/v1/capture-plans`` only when supplied a
private metadata-only authority document. The endpoint remains behind Monas
dispatch and requires the pairing actor to match the authenticated host
context. The document binds every completion to one reviewed endpoint and
logical ObjectStore and contains opaque pairing references plus explicit
enabled site rules. It contains no browser cookies, site credentials, media
bytes, or DASObjectStore secrets.

.. code-block:: json

   {
     "schema_version": "pinakotheke.capture-authority.v1",
     "endpoint_id": "endpoint-local",
     "object_store_id": "pinakotheke-local",
     "pairings": [{
       "pairing_id": "pair-firefox-1",
       "actor_id": "local-user",
       "expires_at": 4102444800,
       "revoked": false
     }],
     "sites": [{
       "site_id": "art-site",
       "origin": "https://art.example",
       "capture_enabled": true,
       "adapter_kind": "experimental_generic",
       "adapter_version": "1.0.0",
       "allow_observed_thumbnails": true,
       "allow_explicit_originals": true,
       "max_candidates_per_page": 32
     }]
   }

Save the reviewed document as a mode-``0600`` regular file and start with
``--capture-authority-file /absolute/path/to/capture-authority.json``. The same
option is accepted by ``pinakotheke service install``. Unknown fields, future
schemas, duplicate pairings/origins, unsafe origins, excessive records, and
non-private or symlinked files fail closed. The wire schema is
``contracts/monas/pinakotheke-capture-authority.v1.schema.json``.

Restart-safe pending plans
--------------------------

Accepted plans are atomically journalled as private metadata beneath
``ROOT/state/capture-plans.v1.json`` before the API reports success. The
journal records the authenticated actor reference, admission time, canonical
page/media identities, adapter, capture kind, dimensions, and scheduler
identity. It never records response bodies, media bytes, cookies,
authorization headers, signed query values, or credentials. A corrupt,
oversized, non-private, symlinked, future-schema, duplicate-ID, or cross-origin
record prevents startup rather than being ignored.

An identical actor/media/capture-kind retry returns the existing plan, including
after restart. Candidate budgets are reconstructed for the actor, canonical
page, and UTC day, and active scheduler lanes are rehydrated before new work is
admitted so job identities are not silently reused. A Monas-authenticated actor
can inspect only their own pending plans with:

.. code-block:: text

   GET /products/pinakotheke/api/extension/v1/capture-plans

Pending means ``awaiting_approved_acquisition``. It is explicit reconciliation
work, not evidence that DASObjectStore contains the object and not permission
to display a ``Stored in ObjectStore`` badge.

Verified worker completion
--------------------------

An accepted plan becomes a gallery card only through the narrow host-worker
completion route. Configure it only alongside Monas dispatch, capture
authority, and the reviewed ObjectStore read helper by supplying a separate
mode-``0600`` ``--capture-completion-token-file``. The worker token is a local
process credential and must never enter Firefox storage, site rules, URLs,
logs, or documentation examples with a real value.

After independently verifying the exact DASObjectStore endpoint, logical store,
object key/version, checksum, media type, and length, the host worker submits
strict ``pinakotheke.capture-completion.v1`` metadata to:

.. code-block:: text

   POST /products/pinakotheke/api/internal/v1/capture-plans/{plan_id}/complete

The request needs both Monas actor context and
``X-Pinakotheke-Capture-Worker-Token``. A browser session without the worker
credential is denied. Pinakotheke replays the acquisition and reconciliation
state gates, writes the common ``New`` gallery record atomically, refreshes the
live ThumbsPlus-style catalogue, and only then marks the plan settled. Exact
retries return ``already_present``; settled markers survive restart and stay
out of the pending list. No payload bytes cross this endpoint. The request
schema is
``contracts/monas/pinakotheke-capture-completion.v1.schema.json``.

Run-one acquisition helper
--------------------------

For X ingress, thumbnail observation performs bounded cache-evidence lookup
only for media delivered by X's dedicated image-media host. Interface artwork,
emoji, and other decorative assets from application hosts are excluded.
Neither X nor generic opted-in websites admit observed-thumbnail acquisition;
only trusted image opens and trusted user-started video playback do so.

The first production-worker boundary is an offline, run-one CLI operation. A
reviewed host executable receives one approved canonical plan and the fixed
endpoint/ObjectStore identity, performs any permitted public retrieval and the
authorized DASObjectStore streaming ingest, verifies the committed object, and
returns metadata only. Pinakotheke invokes it directly as
``HELPER acquire-image-v1`` without a shell.

Stop the foreground or launchd Pinakotheke backend before an offline run so two
processes never mutate the private journals concurrently, then execute. The
backend and run-one command contend for the same private capture-worker lease
and refuse concurrent mutation:

.. code-block:: console

   pinakotheke capture acquire \
     --root "$HOME/.x-img" \
     --capture-authority-file "$HOME/.x-img/config/capture-authority.json" \
     --helper /absolute/path/to/reviewed-acquire-helper \
     --actor-id local-user \
     --plan-id capture-plan-0

The helper receives no site cookie, browser credential, Monas session,
DASObjectStore secret, or caller-selected local payload path. It must use only
bounded isolated ephemeral scratch or bounded streaming through its own scoped
DASObjectStore authority and write one strict JSON receipt to standard error.
Standard output must remain empty: returning payload bytes to Pinakotheke is
rejected. The process must preserve the reviewed destination;
non-zero exit, unknown fields/schema, oversized output, destination changes,
and malformed receipts fail before settlement. ``policy_blocked``,
``unavailable``, and ``rejected`` remain explicit retry/stop outcomes.

The executable exchange is defined by
``contracts/dasobjectstore/pinakotheke-capture-acquire-helper.v1.schema.json``.

The native helper prefers daemon-submitted uploads. Where a DASObjectStore
release cannot yet map a logical ObjectStore identifier to its S3 export
bucket during daemon admission, an administrator may set
``submit_to_daemon`` to ``false`` in the private helper configuration. The
helper then uses the same scoped ``dasobjectstore-remote`` client and store
credential directly, requires its successful completion acknowledgement, and
still records the logical ObjectStore identity in provenance. This is an
explicit compatibility mode, not browser-side storage or a payload copy in
Pinakotheke.
This run-one interface is testable now and is the adapter seam for a later
continuously scheduled host worker; it is not permission to scrape, traverse,
open hidden media, ingest DRM-protected material, or forward browser cookies.

Continuous monolith worker
--------------------------

Supplying the same reviewed executable with ``--capture-acquire-helper`` and a
reviewed host callback with ``--destination-revalidation-helper`` turns the
run-one seam into the normal monolith worker. These options must be supplied
together and are accepted only when capture authority, ObjectStore delivery,
Monas dispatch, and the private completion authority are all configured. Each
newly admitted plan is bound to the saved endpoint, ObjectStore, and selection
revision before it is placed on a bounded background task, while the Firefox
request returns promptly with its durable pending identity.

The monolith permits only one helper process at a time and coalesces concurrent
retries of the same actor/plan. A verified receipt passes through the existing
destination-bound completion gate and refreshes the live gallery. Helper,
policy, transport, validation, or authority failure leaves the plan visibly
pending and makes no ``Stored in ObjectStore`` claim; a later explicit retry can
resume it. No failure triggers origin traversal, a browser credential retry, or
a fallback ObjectStore. Immediately before acquisition starts, the host
callback must return fresh exact authority for the plan binding, including
presence, trusted TLS, unexpired pairing, readiness, write and media-type
capability, and non-zero quota. External completion repeats the check before
catalogue admission; DASObjectStore remains responsible for the final atomic
write authorization and reservation.

At startup the monolith loads every durable unsettled plan and revalidates the
current pairing actor, expiry/revocation, enabled exact-origin policy, pinned
adapter version, and capture-kind permission. Eligible work is requeued through
the same one-helper-at-a-time path without waiting for Firefox to repeat the
request. Withdrawn or expired authority leaves the record pending and performs
no network or ObjectStore operation. Settled markers are never requeued.

First-party DASObjectStore helper
---------------------------------

The packaged ``pinakotheke`` binary implements its own hidden
``acquire-image-v1`` helper mode (the protocol name is retained for
compatibility and now accepts image and explicit-video plans). Point
``--capture-acquire-helper`` at the
absolute ``pinakotheke`` executable and provide a private helper configuration
at ``$HOME/.x-img/config/das-capture-helper.json``. Alternatively set
``PINAKOTHEKE_DAS_HELPER_CONFIG`` to an absolute private configuration path.
The configuration is strict, limited to 16 KiB, and must be a mode-``0600``
regular file rather than a symlink.

.. code-block:: json

   {
     "schema_version": "pinakotheke.das-capture-helper.v1",
     "endpoint_id": "local-docker-example",
     "object_store_bucket": "dos-pinakotheke-media",
     "curl_executable": "/usr/bin/curl",
     "ffprobe_executable": "/usr/bin/ffprobe",
     "dasobjectstore_remote_executable": "/usr/local/bin/dasobjectstore-remote",
     "dasobjectstore_remote_config": "/Users/example/.config/dasobjectstore/remote.json",
     "daemon_socket": "/Users/example/.x-img/dasobjectstore/run/dasobjectstored.sock",
     "submit_to_daemon": true,
     "max_image_bytes": 67108864,
     "max_video_bytes": 1073741824,
     "normalization": {
       "docker_executable": "/usr/bin/docker",
       "ingest_helper": "/usr/local/bin/pinakotheke",
       "executor_ref": "dasobjectstore-video-worker-1",
       "staging_ref": "dasobjectstore-staging-1",
       "staging_root": "/Users/example/.x-img/dasobjectstore/video-staging",
       "image_reference": "registry://approved/ffmpeg",
       "image_digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
       "cpu_millis_limit": 2000,
       "memory_bytes_limit": 1073741824,
       "scratch_bytes_limit": 4294967296,
       "firefox_playback_evidence_id": "firefox-h264-aac-profile-1",
       "codec_gap_journal": "/Users/example/.x-img/state/codec-gaps.v1.json"
     }
   }

This document contains no DAS credential. The referenced DASObjectStore remote
configuration or its site-owned credential helper remains the authority for a
scoped, expiring ObjectStore session and must itself be mode ``0600``. Pair and
select the exact ObjectStore with DASObjectStore before starting Pinakotheke;
the helper cannot infer a first store, change the endpoint selected by the
capture authority, or prompt for a password in the background.

``object_store_bucket`` is retained only for compatibility with older private
configurations. Authoritative daemon submission passes both the reviewed
logical ``object_store_id`` and this reviewed provider binding; DASObjectStore
validates and settles the upload under the logical store authority.

For native daemon submission, configure ``TMPDIR`` as a dedicated ephemeral
staging directory shared only by the Pinakotheke actor and the
``dasobjectstore`` group. Make the directory setgid (mode ``2770``); each
bounded capture directory is group-readable during submission and removed
immediately afterwards. This avoids private-``/tmp`` namespace mismatches
without creating a durable Pinakotheke payload store.

On macOS Docker Desktop, use the authority container rather than a
container-created socket path on the host. DASObjectStore commit ``01a8c385``
packages the matching remote client and AWS CLI. Select this transport only in
the private host configuration:

.. code-block:: json

   {
     "schema_version": "pinakotheke.das-capture-helper.v1",
     "endpoint_id": "local-docker-example",
     "curl_executable": "/usr/bin/curl",
     "ffprobe_executable": "/usr/bin/ffprobe",
     "submit_to_daemon": true,
     "container_execution": {
       "docker_executable": "/Applications/Docker.app/Contents/Resources/bin/docker",
       "compose_file": "/Users/example/.x-img/dasobjectstore/pinakotheke-local/compose.yml",
       "managed_scratch_root": "/Users/example/.x-img/dasobjectstore",
       "container_scratch_root": "/Volumes/Seagate/DASObjectStore",
       "remote_config": "/Users/example/.config/dasobjectstore/remote.json",
       "aws_credentials": "/Users/example/.config/dasobjectstore/scoped.credentials",
       "service": "dasobjectstored",
       "daemon_socket": "/run/dasobjectstore/dasobjectstored.sock"
     },
     "max_image_bytes": 67108864,
     "max_video_bytes": 1073741824,
     "normalization": {
       "docker_executable": "/Applications/Docker.app/Contents/Resources/bin/docker",
       "ingest_helper": "/usr/local/bin/pinakotheke",
       "executor_ref": "dasobjectstore-video-worker-1",
       "staging_ref": "dasobjectstore-staging-1",
       "staging_root": "/Users/example/.x-img/dasobjectstore/video-staging",
       "image_reference": "registry://approved/ffmpeg",
       "image_digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
       "cpu_millis_limit": 2000,
       "memory_bytes_limit": 1073741824,
       "scratch_bytes_limit": 4294967296,
       "firefox_playback_evidence_id": "firefox-h264-aac-profile-1",
       "codec_gap_journal": "/Users/example/.x-img/state/codec-gaps.v1.json"
     }
   }

Native and container fields are mutually exclusive. The container service and
daemon socket are fixed by the schema. The helper accepts only canonical
managed scratch roots, creates a private direct child, and translates that one
descendant to the reviewed container mount. It copies the already-scoped remote
configuration and AWS credential file into that job directory, passes only the
credential *path* to Docker, and deletes the whole directory before returning.

``submit_to_daemon`` is required and must be ``true``. Pinakotheke never accepts
the legacy provider-only ``Upload complete`` response as a durable commit. The
paired DASObjectStore remote client hashes each single-file source, attaches
``dasobjectstore-sha256`` provider metadata, obtains the upload-scoped
completion capability, and reports success only after the daemon independently
verifies size/checksum, settles placement, and publishes the authoritative
catalogue row.
Neither the capture plan nor any browser request can select Docker, a compose
file, a host/container path, or credentials.

For each approved plan the helper permits only HTTPS retrieval and HTTPS
redirects, caps redirects and bytes, writes to a fresh mode-``0700`` ephemeral
directory, validates a non-empty ``image/*`` response, and computes SHA-256 by
bounded streaming. It invokes ``dasobjectstore-remote upload`` with an exact
checksum-derived key and ``--submit-to-daemon``. A zero process exit is not
enough: the helper requires the daemon response to say both ``Complete`` and
``remote_s3_transfer_complete`` before emitting a verified receipt. The daemon
therefore owns provider verification and catalogue completion. Scratch is
deleted on success and every error; no payload is written beneath the
Pinakotheke product root.

For a linked thumbnail, Firefox submits the visible media URL for acquisition
and the link target as a separate presentation URL. A later trusted click on
that link submits the same presentation URL with the opened original. Query and
fragment data are removed server-side. Pinakotheke derives the catalogue ID
from the site, page, and canonical presentation URL, so distinct thumbnail and
original URLs converge without collapsing several images on one page. The
helper's compatibility ``catalogue_id`` response cannot override this identity.
Legacy requests and journals without a presentation URL use their canonical
media URL and remain readable. Object keys and positive immutable versions
derive from the payload checksum, making exact retries idempotent.
Standard output remains empty and all child diagnostics are suppressed; only
the strict metadata receipt is written to standard error. The configuration
schema is
``contracts/dasobjectstore/pinakotheke-das-capture-helper.v1.schema.json``.

Compatibility evidence
----------------------

This metadata-only boundary was inspected against the following sibling source
revisions; they are compatibility pins, not dependencies of the public build:

* Monas ``3d21b0bc7b83fa8408d01b93347a56f43f3a96b7`` for host-owned session
  admission and host-relative product APIs;
* DASObjectStore ``01a8c385330b284492fa729055176db54b9ecf1f`` for the
  ``dasobjectstore-remote`` daemon-submitted upload, checksum metadata,
  immutable object version, provider verification, catalogue completion, and
  container-packaged remote client;
* DASObjectStore ``720ae9c1`` for aligned isolated-profile ports and the
  idempotent canonical folder binding required by daemon capacity admission;
* DASObjectStore ``b35ee0b2`` for region-consistent provider verification,
  matched finite capacity, durable local catalogue registration, and
  retry-stable immutable remote-upload identity; and
* Mnemosyne design language ``5539df8f662a78ebdf7cf4c868d71831380c8cfd`` and
  Mnemosyne ``52810176bf95a170f93d74a6f5daa94da5c6640e`` for host-relative
  product/API and task-pane boundaries.

The current first live authority proof commits and reads the exact image bytes
and converges duplicate requests. Garage still reports the uploaded object as
``application/octet-stream`` because the remote upload command does not yet
carry the helper's verified ``image/*`` media type. Treat this as an explicit
gallery-admission blocker: do not relabel it in Pinakotheke or infer MIME from a
filename. The next adapter slice must preserve the reviewed type at upload and
verify it through ``head-object``.

The persistent observer uses Firefox Manifest V3
``scripting.registerContentScripts`` with the already granted exact-origin
permission. Firefox 101 or newer supports this API; the extension requires
Firefox 128 or newer. Dynamic registrations are explicitly reconstructed on
extension update as required by Firefox's registration lifecycle. The same
observer is immediately injected into already-open eligible tabs so update and
startup do not leave an existing single-page application unobserved.

X video capture and stored framing
----------------------------------

X commonly starts a clicked video through page JavaScript and exposes a
transient ``blob:`` URL on the visible ``video`` element. Pinakotheke therefore
records the genuine trusted pointer or keyboard activation first; a subsequent
playing state is eligible only while it remains bound to that visible element
and short activation window. The extension resolves the bounded X manifest
candidate without forwarding cookies or request headers and submits one
explicit-video plan. Autoplay without that activation remains ineligible.

After verified DASObjectStore settlement, the same video receives the
browser-only two-pixel green frame and the written ``Stored in ObjectStore``
state. On a later page load the transient ``blob:`` URL cannot identify the
stored object, so the observer sends the canonical X status/presentation URL
through the existing pairing-scoped evidence endpoint. Only a settled
explicit-video plan for the same actor, pairing, origin, adapter version, and
presentation may produce a hit. The frame is an overlay; it neither changes
the source page nor mutates stored media bytes.

The green frame has deliberately narrow meaning: it is shown only for a
verified user-opened original image or user-played normalized video. A
thumbnail that was merely visible may still be cached and admitted to the
catalogue as ``Previously observed``, but it does not receive the green frame
and does not claim that its original was imported. X may recycle one media DOM
element for several gallery items; Pinakotheke binds the element token and
overlay to the current rendered-media identity and clears them whenever that
identity changes, so a delayed receipt cannot mark unrelated replacement
media. X also replaces a clicked thumbnail node with a distinct modal/gallery
node for the *same* immutable image. In that case the stable canonical media
URL, rather than the obsolete element token, authorizes the new node's green
frame after settlement.

The extension popup lists explicitly selected images and videos with worded
``selected``, ``downloading``, ``pending``, ``failed``, or ``stored`` state.
Colour is never the only evidence of progress. A stored claim still requires
the server plan to report verified DASObjectStore settlement.

Verification
------------

The core and Axum tests prove actor binding, pairing expiry/revocation,
site/adapter eligibility, candidate bounds, redaction of a query-bearing media
URL, scheduler admission, missing host-context rejection, and fail-open
unconfigured host behavior. Build the user documentation locally with:

.. code-block:: console

   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check
