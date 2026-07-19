Gallery and DASObjectStore convergence
======================================

DASObjectStore is authoritative for media existence. The Pinakotheke gallery
is a restart-safe metadata projection and never infers existence from Garage,
S3, a source website, or a successful browser request.

Authority reconciliation
------------------------

Configure the monolith with the reviewed executable used for its local
DASObjectStore integration:

.. code-block:: console

   pinakotheke serve \
     --capture-authority-file /private/capture-authority.json \
     --gallery-inventory-helper /usr/bin/pinakotheke

At startup, and every ten seconds thereafter, Pinakotheke asks the helper for
one complete bounded inventory of ``Protected`` objects in the configured
logical ObjectStore. The packaged helper calls ``dasobjectstore store contents
STORE --json``. It does not list a provider bucket and does not receive storage
credentials.

Each gallery thumbnail, original, poster, or normalized video is compared by
stable endpoint ID, ObjectStore ID, and immutable DAS object ID. An absent
object is atomically persisted as ``Unavailable`` before the in-memory gallery
is replaced. Its delivery route is removed. If authority later returns, the
route and ``Ready`` state are reconstructed. Provenance metadata remains so the
user can understand the missing object; source websites are never used as a
read fallback.

Diagnostics
-----------

An authenticated Monas request to
``GET /products/pinakotheke/api/operations/v1/gallery-convergence`` returns:

* ``authoritative_count`` — unique protected DAS objects;
* ``projected_count`` — unique gallery object references currently present in
  that authority inventory;
* ``orphan_count`` — authoritative objects without a gallery projection;
* ``stale_count`` — projected references absent from authority; and
* ``changed_representations`` — representations changed by the most recent
  pass.

Counts contain no object keys, account names, URLs, checksums, or actor data.
If the helper is unavailable, the last verified report remains visible and no
gallery state is changed. Startup fails when its initial configured authority
query cannot complete; this prevents a restarted service from advertising
unverified availability.

Verification
------------

.. code-block:: console

   cargo test -p pinakotheke-core gallery_reconciliation
   cargo test -p pinakotheke-api out_of_band_deletion_is_persisted_and_survives_restart

The tests cover committed presence, out-of-band deletion, atomic persistence,
restart convergence, returned authority, duplicate projection references, and
orphan counts without storing media payloads.
