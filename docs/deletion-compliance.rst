Deletion and compliance reconciliation
=======================================

x-img treats catalogue visibility and durable ObjectStore removal as separate
approved actions. A source deletion, access loss, policy change, rights request,
or user request may require prompt catalogue tombstoning, but none is inferred
to authorize deletion of an immutable DASObjectStore object.

Approved request
----------------

Every action is bound to a stable request ID, canonical media identity, reason,
scope, exact endpoint/ObjectStore/object/checksum evidence, opaque actor
reference, policy-decision reference, and approval time. The request contains no
session, token, cookie, signed URL, source payload, browsing history, or media
bytes.

Two scopes are supported:

``Catalogue only``
   Hide the item from normal catalogue and cache presentation while retaining
   provenance and the authority object. An ObjectStore removal request is
   rejected for this approval.

``Catalogue and object``
   Tombstone first, then permit an authorized adapter to submit removal of the
   exact reviewed object. This does not permit endpoint, ObjectStore, object
   reference, or checksum substitution.

State and recovery
------------------

The word-first lifecycle is:

.. code-block:: text

   Active -> Tombstoned -> Removal requested -> Removal verified
                                      \-------> Conflict

``Tombstoned`` immediately removes normal visibility without claiming that
bytes were deleted. ``Pending`` or ``Still present`` authority observations
remain ``Removal requested`` and are safe to retry after a crash. Only a
matching DASObjectStore observation reaches ``Removal verified``. Mismatched
authority evidence becomes ``Conflict`` and never silently selects another
store or object.

Replaying tombstone, request, pending, or verified evidence converges without
duplicating audit events. Audit entries are bounded event codes and states;
free-form provider responses and secrets are not retained. Provenance remains
available for accountable compliance evidence even after normal presentation
is removed.

Local proof
-----------

Run the focused contract tests with:

.. code-block:: console

   cargo +1.97.0 test -p x-img-core compliance_reconciliation

They prove catalogue-only scope, required approval, tombstone-before-delete,
pending/retry behavior, exact-object verification, replay idempotency, and
conflict on changed authority identity. A live DASObjectStore deletion adapter
must still enforce its own current authorization, policy, retention, and audit
requirements immediately before mutation.
