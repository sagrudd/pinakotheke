Versioned configuration
=======================

x-img configuration is explicit, local metadata. It contains source
identifiers, policy choices, budgets, review defaults, and opaque references to
host-managed authority records. It never contains passwords, browser cookies,
Monas sessions, access tokens, DASObjectStore secrets, signed URLs, or media
bytes.

Schema set
----------

The checked-in schemas use JSON Schema draft 2020-12 and versioned identifiers:

* ``x-img.instance.v1`` is the top-level instance and destination selection;
* ``x-img.x-account.v1`` describes one X account;
* ``x-img.instagram-account.v1`` describes one Instagram account; and
* ``x-img.website-policy.v1`` describes one explicitly enabled website rule.

``x-img-common.v1.schema.json`` contains the shared definitions for media
policy, refresh budgets, review defaults, origins, and host-managed references.
The top-level schema references the account and website schemas by their local
versioned filenames, so a validator should load the complete ``schemas/``
directory rather than fetching a network URL.

The acquisition contract is separate from source configuration:
``x-img.acquisition.v1`` defines the metadata envelopes for source items,
canonical media identities, verified ObjectStore references, download attempts,
job leases, account cursors, review state, tombstones, and audit events. It
does not contain media bytes, credentials, cookies, sessions, signed URLs, or
unbounded transfer buffers. Its state transitions and crash-reconciliation
rules are documented in :doc:`adr/0009-acquisition-catalogue-schemas`.

Every object rejects unknown properties. ``schema_version`` is a constant, not
an open-ended string: an unknown future major must be rejected before a config
write or job snapshot. A future version requires an explicit migration and
compatibility fixtures; it must not be silently downgraded.

Host and storage references
---------------------------

``host_context_ref`` identifies the Monas host context (or a future compatible
host adapter). ``object_store_ref`` records the stable endpoint/device ID,
logical ObjectStore ID, managed prefix, and a DASObjectStore application
reference. These are references and identities only. Display names are not
authority keys, and a reconnect must never silently select a different store.

Source policy
-------------

Each source has an independent ``enabled`` flag and media policy. The policy
must state whether images, videos, and animated images are allowed; thumbnails
are limited to ``observed_only``; and originals are limited to an explicit user
open unless a separately approved policy permits them. Disabled examples remain
in the configuration so a reviewable diff can show what will change when a
source is enabled.

Account authorization is represented by an opaque
``monas.connector-authorization`` reference. The schema does not accept a raw
credential. Enabled accounts and protected/authorized-viewer account classes
must include that reference, while the source adapter and server-side policy
remain responsible for validating its authority.

Budgets and review defaults
---------------------------

Refresh budgets bound requests, pages, items, bytes, duration, and the minimum
interval between refreshes. Website policies additionally bound observed
candidates per page/day and bytes per candidate. Review defaults choose only
the initial state (``new`` or ``hidden``), whether automatic review is allowed,
and whether items are grouped by source. An item is not eligible for ``new`` or
``reviewed`` admission until its ObjectStore commit is verified.

Synthetic example and validation checklist
------------------------------------------

The complete synthetic example is
``examples/config/instance.v1.json``. It includes enabled and disabled X,
Instagram, and website entries. The negative fixtures
``invalid-unknown-field.v1.json`` and ``invalid-future-major.json`` must fail
against ``schemas/x-img-instance.v1.schema.json``. A focused implementation
test should also assert that:

* an enabled account without an authorization reference is rejected;
* an unauthorized wildcard or non-origin website value is rejected;
* negative or unbounded budget values are rejected;
* raw token, cookie, password, and session-shaped fields are rejected as
  unknown properties; and
* disabled sources remain parseable without an authorization reference but are
  never scheduled by the refresh planner.

The reproducible documentation check is authoritative:

.. code-block:: console

   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check

Rust configuration commands
---------------------------

XIMG-021 implements the strict configuration layer in the local ``x-img``
CLI. These commands are offline: they validate local metadata only and never
contact X, Instagram, a website, Monas, or DASObjectStore.

.. code-block:: console

   cargo run -p x-img-cli -- config validate --path instance.json
   cargo run -p x-img-cli -- config list --path instance.json
   cargo run -p x-img-cli -- config replace --path instance.json --input candidate.json

``validate`` parses the complete document and fails closed on unknown fields,
unknown schema versions, invalid opaque-reference kinds, invalid account names,
unsafe origins, missing required authorizations, duplicate account IDs/handles,
or duplicate website IDs/origins. ``list`` prints only source-kind, stable
rule ID, and handle/username/origin; it deliberately does not print host or
authorization references.

``replace`` first parses and validates the complete candidate. Only then does
it write a synchronized temporary file beside the destination and rename it
into place. The destination directory must already exist, and a failed parse or
validation leaves the existing configuration unchanged. This protects local
configuration metadata; it does not claim to authenticate authority references
or schedule an acquisition.

The implemented XIMG-022 lifecycle layer admits review state only after a
verified ObjectStore evidence record is committed. The separate XIMG-023
contract will add canonical-identity-plus-immutable-checksum idempotency,
crash reconciliation, and persisted URL-alias handling without treating URLs
as identity. See :doc:`acquisition` for the current lifecycle boundary.
