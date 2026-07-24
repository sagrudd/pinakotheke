Deletion and compliance reconciliation
=======================================

x-img treats catalogue visibility and durable ObjectStore removal as separate
approved actions. A source deletion, access loss, policy change, rights request,
or user request may require prompt catalogue tombstoning, but none is inferred
to authorize deletion of an immutable DASObjectStore object.

User deletion from the library
------------------------------

Open an image or normalized video, choose ``Review deletion``, read the exact
number of affected catalogue records and DASObjectStore objects, then choose
``Delete from Pinakotheke and DASObjectStore``. This is an authenticated,
irreversible action. It covers the card thumbnail or video poster and the
stored original image or normalized video rendition. Exact duplicate cards
that share those immutable object references are disclosed and removed as one
asset; unrelated records are not changed.

The browser never deletes bytes itself. Pinakotheke sends only endpoint,
ObjectStore, key/object ID, positive version, exact byte length, and SHA-256
evidence through the reviewed ``pinakotheke.object-delete-helper.v1`` host
adapter. DASObjectStore remains responsible for current actor/application
authorization, retention policy, provider deletion, authoritative catalogue
mutation, capacity reconciliation, and audit. A raw S3 delete is not a valid
adapter implementation because it can leave the DASObjectStore catalogue
inconsistent.

Pinakotheke removes its persistent projection only after every exact object is
reported ``deleted`` or ``already_absent``. If the helper is missing, rejects
the operation, times out, or the gallery changes during deletion, the record
remains visible with a retryable failure. A retry is safe after a partial
authority operation because already-absent objects are idempotent success.

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

DASObjectStore authority status
-------------------------------

DASObjectStore ``0.124.0`` commit
``7cf31c3cb1582ad9817dc98b7db57fd335007735`` defines the required
``dasobjectstore.application_object_delete.v1`` authority. It separately scopes
the ``delete`` operation, checks exact ObjectStore, object ID/version, size,
SHA-256, Garage bucket/key, and provider metadata before mutation, verifies
provider absence, then atomically withdraws the matching catalogue row and
records a redacted audit event. Exact absence is idempotent success.

Pinakotheke ``1.28.0`` provides the first-party bounded Unix-socket transport.
Configure ``--object-delete-helper`` with the installed ``pinakotheke`` binary
and set ``PINAKOTHEKE_DAS_DELETE_HELPER_CONFIG`` to a mode-``0600`` regular
file such as:

.. code-block:: json

   {
     "schema_version": "pinakotheke.das-object-delete-helper.v1",
     "daemon_socket": "/run/dasobjectstore/dasobjectstored.sock",
     "endpoint_id": "reviewed-endpoint-id",
     "application_id": "pinakotheke",
     "session_file": "/private/runtime/pinakotheke-delete-session.json",
     "provider": "garage",
     "endpoint_url": "http://127.0.0.1:3900",
     "stores": [
       {"object_store_id": "reviewed-store-id", "bucket": "reviewed-bucket"}
     ]
   }

The separately mode-``0600`` session file contains the paired session ID and
renewal token using schema ``pinakotheke.das-object-delete-session.v1``. It is a
host secret, must never be committed, logged, placed in browser storage, or
included in the general site corpus, and should be replaced when the pairing
rotates. The helper rejects symlinks, permissive files, changed endpoint/store
evidence, unknown response schemas, and responses whose deterministic request
identity does not match.

Until both files and a delete-scoped application identity are configured, the
deletion pane retains the record and reports the authority as unavailable. A
raw S3 operation remains invalid even though the provider supports it.

Local proof
-----------

Run the focused contract tests with:

.. code-block:: console

   cargo +1.97.0 test -p pinakotheke-core compliance_reconciliation

They prove catalogue-only scope, required approval, tombstone-before-delete,
pending/retry behavior, exact-object verification, replay idempotency, shared
duplicate expansion, projection-after-authority ordering, conflict on changed
authority identity, and the strict first-party daemon exchange. Live synthetic
evidence still has to prove authorization, policy, retention, provider absence,
catalogue withdrawal, capacity reconciliation, and audit immediately before
Pinakotheke projection removal.
