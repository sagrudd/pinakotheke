Migration, export, and restore
==============================

x-img provides a strict metadata snapshot boundary for release recovery and the
future Pinakotheke identity migration. A snapshot contains validated instance
configuration, immutable catalogue references, and non-secret Firefox pairing
identity only. It cannot represent media bytes, cookies, credentials, sessions,
CSRF values, access tokens, signed URLs, or DASObjectStore capabilities.

Export and verification
-----------------------

``migration_backup::export`` validates the complete snapshot, serializes a
deterministic JSON document, and returns an independent SHA-256 checksum. The
authenticated host is responsible for placing both in approved backup storage.
x-img does not write media or backup payloads into its product root.

An operator must retain the checksum separately and verify it before cutover.
Corrupt bytes, unknown fields, unsafe records, excessive record counts, and an
unknown or future schema major fail before any state is returned.

Restore is deliberately two-step. ``migration_backup::restore`` only verifies
and returns a candidate snapshot; it never overwrites live state. The operator
reviews the target instance, endpoint, and logical ObjectStore before a future
host adapter atomically applies that candidate. Object references and checksums
are preserved exactly; restore never copies, moves, or rekeys a media object.

Legacy identity migration
-------------------------

The v1 migration proof is copy-on-write and idempotent. It first creates a
verified export artifact, then returns a migrated candidate and a bounded
result code. The current compatibility operation leaves ``x-img.*.v1`` data
unchanged: historic labels, canonical media identities, endpoint IDs,
ObjectStore IDs, object references, and checksums are not rewritten for
branding.

Firefox pairing records contain no credential. Every restored legacy pairing
is marked ``requires_reviewed_repair``; restoring a snapshot cannot silently
authorize a browser profile or change its destination.

Local proof
-----------

Run the focused native tests with:

.. code-block:: console

   cargo +1.97.0 test -p x-img-core migration_backup

They prove round-trip export/restore, repeat migration idempotency, stable
authority identities, checksum-corruption rejection, future-major rejection,
unknown-field rejection, and mandatory reviewed Firefox re-pairing. The full
release check additionally requires ``scripts/quality/check.sh`` and the pinned
documentation container described on the documentation home page.
