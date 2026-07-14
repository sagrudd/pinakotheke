Acquisition lifecycle
=====================

The Rust core has an explicit in-memory acquisition state machine. It is a
local domain rule, not a connector or storage implementation: it performs no
network request, does not carry media bytes, and does not authenticate a Monas
or DASObjectStore reference.

Normal path
-----------

The only normal settlement sequence is:

.. code-block:: text

   discovered -> claimed -> transferring -> stored -> verified -> committed

``claimed`` requires one stable lease identifier. ``stored`` means an external
authority has accepted an object, but it is not yet catalogue-ready.
``verified`` requires bounded metadata for the stable endpoint, logical
ObjectStore, object-reference ID, and immutable lowercase SHA-256 checksum.
Only ``committed`` may become visible in the catalogue.

Review and explicit outcomes
----------------------------

The review states ``New``, ``Reviewed``, ``Retained``, ``Hidden``, and
``Removed`` can be assigned only after a verified object is committed. This
prevents discovery or a partially uploaded object from appearing as a review
card.

``Failed``, ``PolicyBlocked``, ``Cancelled``, and ``Conflict`` are terminal
outcomes before settlement. They cannot be claimed, transferred, verified, or
committed again by this instance. ``Tombstoned`` is allowed only from a
committed record. A future persistence adapter must create a fresh, explicitly
reconciled lifecycle where a retry is permitted; this state machine does not
silently reopen terminal records.

Boundaries
----------

The core does not prove that supplied metadata came from DASObjectStore; the
future authorized storage adapter must do that. It merely prevents a caller
from treating absent or malformed evidence as verified. It does not implement
persistence, object upload, account refresh, or review UI behavior. Those
remain separate release gates.

Idempotency and crash reconciliation
------------------------------------

XIMG-023 adds an in-memory metadata catalogue for deterministic settlement.
Its key is the canonical media identity plus the verified immutable SHA-256;
a source URL is never an identity. A reconciliation request carries only that
bounded key expectation and safe HTTPS aliases. A future authorized adapter
supplies one observation:

* ``Absent`` leaves the catalogue unchanged and reports that authority evidence
  is still required.
* ``Verified`` with the expected checksum creates one committed record. A crash
  replay with the same key reuses that record, appends any new safe aliases, and
  never replaces the first object reference.
* A mismatch or canonical-identity reuse with a different checksum records a
  ``Conflict`` outcome and retains the competing checksum evidence. It never
  overwrites bytes or silently selects a replacement object.

The module does not call DASObjectStore, persist its in-memory metadata, or
turn a supplied observation into proof of authorization. XIMG-030 and later
storage/persistence contracts must obtain and durably record verified authority
evidence. The present boundary exists so retries and crash recovery have one
deterministic, testable settlement rule.
