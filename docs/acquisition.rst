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
committed record. Future XIMG-023 reconciliation and persistence work will
create a fresh, explicitly reconciled lifecycle where a retry is permitted;
this state machine does not silently reopen terminal records.

Boundaries
----------

The core does not prove that supplied metadata came from DASObjectStore; the
future authorized storage adapter must do that. It merely prevents a caller
from treating absent or malformed evidence as verified. It also does not
implement idempotency, crash reconciliation, persistence, object upload,
account refresh, or review UI behavior. Those remain separate release gates.
