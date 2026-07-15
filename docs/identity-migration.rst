Pinakotheke identity migration
==============================

Pinakotheke is the v1.0.0 product name. Until the coordinated v1.0.0 release
is complete, this public planning repository and its technical identifiers
remain ``x-img``. An individual documentation, package, or URL update is not
the product migration.

What changes at v1.0.0
----------------------

The user-facing name, documentation, executable, package/product metadata,
Monas and future Synoptikon label, Firefox listing, and canonical GitHub
repository become Pinakotheke and ``sagrudd/pinakotheke``. Controlled clones
are updated after the cutover:

.. code-block:: console

   git remote set-url origin https://github.com/sagrudd/pinakotheke.git

GitHub redirects the old repository URL, but the redirect is not a permanent
interface and the old ``sagrudd/x-img`` name must not be recreated.

What stays stable
-----------------

Existing schemas, catalogue identities, ObjectStore references, checksums,
provenance/audit events, and a signed Firefox extension ID are data-bearing
identities. They are not rewritten just to match the new brand. Configuration
and pairings are read unchanged or migrated through a reviewable, backed-up,
idempotent operation. A migration never moves media out of DASObjectStore or
silently selects a different endpoint or ObjectStore.

Compatibility and recovery
--------------------------

The legacy ``x-img`` aliases remain for the later of twelve months after v1.0.0
or two subsequent minor releases. They show a clear migration notice. Removing
them requires a major release unless an urgent security, privacy, legal, or
data-integrity issue requires earlier removal.

Before cutover, operators receive a migration report, backup/export reference,
compatibility result, and rollback instructions. After a GitHub rename,
recovery keeps the canonical repository: recreating the old repository removes
GitHub's redirect. For data, recovery uses a compatible reader or backup;
committed ObjectStore bytes are never rewritten or deleted for branding.

The detailed sequence, surface inventory, and required proof cases are in
:doc:`adr/0011-pinakotheke-v1-identity-migration`.

Executable cutover gate
-----------------------

The migration is enforced rather than inferred from documentation. Run the
inventory-safe preflight at any time:

.. code-block:: console

   make v1-preflight

It validates the exact migration surface inventory and reports every identity
that is not yet canonical. Blockers are expected in a 0.9 release and do not
make preflight fail. The actual release command is intentionally strict:

.. code-block:: console

   make v1-cutover

It also queries the public canonical GitHub repository and exits unsuccessfully
unless version, repository, Rust packages, CLI plus legacy wrapper, Monas and
DASObjectStore registrations, Firefox identity, documentation, legacy schema
reader, migration proof, and GitHub state are all ready together. A failed
cutover check is a release refusal, not a checklist warning. The report contains
surface names only and never reads credentials, media, browsing history, or
ObjectStore records.
