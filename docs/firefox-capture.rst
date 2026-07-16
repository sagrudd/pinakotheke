Viewed-media capture plans
==========================

XIMG-064 adds the first server-side admission boundary for Firefox observed
media. It is deliberately a **capture plan**, not a browser upload or a
committed catalogue item. Durable bytes remain with DASObjectStore, and a plan
can proceed only through a future approved acquisition worker, ObjectStore
verification, reconciliation, and review admission.

Eligibility
-----------

On a direct toolbar click, the Firefox extension considers at most 32 ``img``
elements that are complete, have natural dimensions, are not hidden, and
intersect the current viewport. It submits each eligible item separately to
the paired instance. It does not open an image, inspect off-screen images,
traverse hidden DOM content, crawl a page, or simulate browsing.

After a site is explicitly enabled for image capture, Firefox dynamically
registers one persistent, top-frame content script for that exact origin. The
registration is restored after extension updates and browser startup, and is
removed before capture is paused or the origin permission is revoked. It does
not require the user to press the toolbar cache control on every page and it
does not add tab, history, cookie, or ``webRequest`` permission.

The content script submits an ``explicit_original`` only for a trusted
primary-button click on an image link, or an image already displayed as the
document itself. Unlinked thumbnail clicks, synthetic events, hidden traversal,
subframes, and automatic opening are not eligible. Login and settings paths are
excluded from registration. The background process revalidates current site
policy, adapter capability, and sender-tab provenance for every message, so a
stale page script cannot bypass a newly paused policy. Video acquisition is not
implemented by this endpoint.

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
capture kind, source media URL, and positive dimensions. Query and fragment
components are removed immediately. It has no payload field, headers field,
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
This run-one interface is testable now and is the adapter seam for a later
continuously scheduled host worker; it is not permission to scrape, traverse,
open hidden media, ingest DRM-protected material, or forward browser cookies.

Continuous monolith worker
--------------------------

Supplying the same reviewed executable with ``--capture-acquire-helper`` turns
the run-one seam into the normal monolith worker. This option is accepted only
when capture authority, ObjectStore delivery, Monas dispatch, and the private
completion authority are all configured. Each newly admitted plan is placed on
a bounded background task, while the Firefox request returns promptly with its
durable pending identity.

The monolith permits only one helper process at a time and coalesces concurrent
retries of the same actor/plan. A verified receipt passes through the existing
destination-bound completion gate and refreshes the live gallery. Helper,
policy, transport, validation, or authority failure leaves the plan visibly
pending and makes no ``Stored in ObjectStore`` claim; a later explicit retry can
resume it. No failure triggers origin traversal or a browser credential retry.

At startup the monolith loads every durable unsettled plan and revalidates the
current pairing actor, expiry/revocation, enabled exact-origin policy, pinned
adapter version, and capture-kind permission. Eligible work is requeued through
the same one-helper-at-a-time path without waiting for Firefox to repeat the
request. Withdrawn or expired authority leaves the record pending and performs
no network or ObjectStore operation. Settled markers are never requeued.

First-party DASObjectStore helper
---------------------------------

The packaged ``pinakotheke`` binary implements its own hidden
``acquire-image-v1`` helper mode. Point ``--capture-acquire-helper`` at the
absolute ``pinakotheke`` executable and provide a private helper configuration
at ``$HOME/.x-img/config/das-capture-helper.json``. Alternatively set
``PINAKOTHEKE_DAS_HELPER_CONFIG`` to an absolute private configuration path.
The configuration is strict, limited to 16 KiB, and must be a mode-``0600``
regular file rather than a symlink.

.. code-block:: json

   {
     "schema_version": "pinakotheke.das-capture-helper.v1",
     "endpoint_id": "local-docker-example",
     "curl_executable": "/usr/bin/curl",
     "dasobjectstore_remote_executable": "/usr/local/bin/dasobjectstore-remote",
     "dasobjectstore_remote_config": "/Users/example/.config/dasobjectstore/remote.json",
     "daemon_socket": "/Users/example/.x-img/dasobjectstore/run/dasobjectstored.sock",
     "max_image_bytes": 67108864
   }

This document contains no DAS credential. The referenced DASObjectStore remote
configuration or its site-owned credential helper remains the authority for a
scoped, expiring ObjectStore session and must itself be mode ``0600``. Pair and
select the exact ObjectStore with DASObjectStore before starting Pinakotheke;
the helper cannot infer a first store, change the endpoint selected by the
capture authority, or prompt for a password in the background.

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

The page plus canonical media URL produces a stable gallery identity, so a page
with several images retains several cards rather than collapsing them. An
original with the same canonical URL enriches its observed card. Sites whose
thumbnail and opened original use different URLs still require a future
explicit browser correlation field; until then the original remains pending
rather than being guessed onto another card. Object keys and positive immutable
versions derive from the payload checksum, making exact retries idempotent.
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
* DASObjectStore ``5769f27859a58101aedd9de0087fc278fd3e4b16`` for the
  ``dasobjectstore-remote`` daemon-submitted upload, checksum metadata,
  immutable object version, provider verification, and catalogue completion;
  and
* Mnemosyne design language ``5539df8f662a78ebdf7cf4c868d71831380c8cfd`` and
  Mnemosyne ``52810176bf95a170f93d74a6f5daa94da5c6640e`` for host-relative
  product/API and task-pane boundaries.

The persistent observer uses Firefox Manifest V3
``scripting.registerContentScripts`` with the already granted exact-origin
permission. Firefox 101 or newer supports this API; the extension requires
Firefox 128 or newer. Dynamic registrations are explicitly reconstructed on
extension update as required by Firefox's registration lifecycle.

Verification
------------

The core and Axum tests prove actor binding, pairing expiry/revocation,
site/adapter eligibility, candidate bounds, redaction of a query-bearing media
URL, scheduler admission, missing host-context rejection, and fail-open
unconfigured host behavior. Build the user documentation locally with:

.. code-block:: console

   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check
