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

Configuration validation belongs in the future Rust contract layer (XIMG-021)
and must preserve these strict, fail-closed semantics.
