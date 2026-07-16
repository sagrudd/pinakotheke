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

The current endpoint intentionally admits only ``observed_thumbnail`` events.
An ``explicit_original`` event is rejected unless a site policy explicitly
allows it; a later extension slice must record a real user-open event before
that policy can be enabled. Video acquisition is not implemented by this
endpoint.

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
context. The document contains opaque pairing references and explicit enabled
site rules; it contains no browser cookies, site credentials, media bytes, or
DASObjectStore secrets.

.. code-block:: json

   {
     "schema_version": "pinakotheke.capture-authority.v1",
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

Compatibility evidence
----------------------

This metadata-only boundary was inspected against the following sibling source
revisions; they are compatibility pins, not dependencies of the public build:

* Monas ``3d21b0bc7b83fa8408d01b93347a56f43f3a96b7`` for host-owned session
  admission and host-relative product APIs;
* DASObjectStore ``b8e1fb9c6059497b981140fcd6e2818d23fc972f`` for scoped,
  short-lived application authority and verified completion; and
* Mnemosyne design language ``5539df8f662a78ebdf7cf4c868d71831380c8cfd`` and
  Mnemosyne ``52810176bf95a170f93d74a6f5daa94da5c6640e`` for host-relative
  product/API and task-pane boundaries.

Verification
------------

The core and Axum tests prove actor binding, pairing expiry/revocation,
site/adapter eligibility, candidate bounds, redaction of a query-bearing media
URL, scheduler admission, missing host-context rejection, and fail-open
unconfigured host behavior. Build the user documentation locally with:

.. code-block:: console

   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check
