One-click account refresh
=========================

XIMG-045 provides a metadata-only orchestration contract for the single
``Refresh accounts`` action. One authenticated host action selects every
enabled X and Instagram account from strict local configuration and creates one
global job with visible per-account children. It does not call either platform,
read credentials, transfer media, or admit review cards.

Repeated clicks by the same actor coalesce to the active job. A child moves
through ``Pending``, ``Running``, and an explicit terminal state. Progress is
bounded against that account's configured request, page, item, and byte limits;
it reports pages, items, requests, bytes, attempts, and the provisional new
item count. The final summary gives word-visible completed, failed,
policy-blocked, cancelled, and new-item totals.

Failure can be explicitly retried; policy-blocked work cannot be retried as a
transport failure. Cancellation turns pending and running children into
``Cancelled``. A child cannot be claimed twice, so one account never overlaps
within the global refresh. The state model can finish ``Complete``, ``Partial``,
``Failed``, ``Policy blocked``, or ``Cancelled``.

These are scheduling signals, not proof of a media commit. A future approved
connector must still obey its policy gate, use the bounded discovery planners,
and require verified DASObjectStore evidence before review admission.

Verify locally:

.. code-block:: console

   cargo +1.97.0 test -p x-img-core account_refresh
   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check
