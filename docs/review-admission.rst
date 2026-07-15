New-item review admission
=========================

XIMG-046 admits a review card only when the acquisition lifecycle is
``Committed`` and carries verified ObjectStore evidence. The queue retains the
canonical media identity, source/account grouping, discovery time, verified
object reference, and ``New`` state. Interrupted, failed, policy-blocked, or
unverified work is rejected and never appears as a broken new card. Replays
retain the first queue record.

Website captures use the same rule through :doc:`website-capture-review`.
Their plan and adapter provenance is retained separately, but the shared queue
still admits only a verified committed DASObjectStore object.
