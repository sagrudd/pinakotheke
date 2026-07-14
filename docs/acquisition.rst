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

Scheduling contracts
--------------------

XIMG-024 provides an in-memory scheduling contract for future account refresh,
extension capture, resource, and video jobs. A refresh request is coalesced per
actor scope: repeated presses return the already active global job rather than
enqueueing another pass. Each child has an explicit source scope, so one source
cannot have two unexpired leases. Different sources are admitted only within
the global child-capacity limit.

The contract tracks request, byte, and elapsed-time budget usage. Attempts that
would exceed a budget return ``BudgetExceeded``; work that would exceed active
child capacity returns ``CapacityLimited``. Cancellation moves pending or
claimed children to ``Cancelled`` and clears their opaque lease owner. It does
not transfer content, contact a connector, create a retry, persist a job, or
claim that a source or ObjectStore operation succeeded.

Synthetic connector fixtures
----------------------------

XIMG-025 provides a versioned synthetic matrix at
``fixtures/connectors/v1/scenarios.json``. It covers pagination, edits,
deletions, duplicate media, multiple variants, rate limiting, authorization
expiry, malformed responses, and cursor reset for both X and Instagram. The
Rust test contract requires every source/scenario pair and rejects an
incompatible fixture schema.

These are opaque deterministic adapter inputs and expected outcomes only. They
contain no actual account names, media payloads, browser data, credentials,
cookies, tokens, signed URLs, or platform traffic. Passing them does not
authorize a connector or weaken the XIMG-002 platform-policy gate.

Bioinformatics transfer plans
-----------------------------

XIMG-026 accepts one explicit GEO, SRA, ENA, or NCBI accession/URL as a
metadata-only transfer plan. Before any future transfer, the plan presents the
authority, release, selected files, announced bytes and checksum, transport,
endpoint/ObjectStore/prefix/object type, rights note, and policy result.
Wildcard/bulk inputs and arbitrary root destinations are rejected.

Only an ``Allowed`` plan can be explicitly confirmed. ``ReviewRequired`` and
``Blocked`` plans remain metadata-only. Confirmation does not transfer bytes:
the future server-side adapter must revalidate policy, destination, quota, and
write capability immediately before DASObjectStore ingest. HTTPS remains the
baseline transport; optional Aspera and any fallback/resume work remain later
fixture and adapter gates.

XIMG-027 supplies the synthetic resolution/transport matrix at
``fixtures/bioinformatics/v1/resolution-transport.json``. It covers ENA/SRA
manifests and multi-run expansion, GEO raw archive review, NCBI routing,
checksums/sizes, retry/resume/cancellation, backpressure, checksum failure, and
optional Aspera fallback to HTTPS. It contains no payload, credential, or live
provider interaction.
