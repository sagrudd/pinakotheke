Exact X image deduplication
===========================

Pinakotheke treats the immutable X media path as the identity of an image. The
page currently displaying it, the status/gallery route, the user-defined site
rule name, and the ``small``, ``900x900``, or ``orig`` rendition query do not
create separate gallery cards. An observed thumbnail and a subsequently opened
original therefore enrich one card.

This exact identity rule is separate from future perceptual duplicate
grouping. Two different media identifiers remain separate even when their
pixels look alike. Pinakotheke does not compare or mutate image payloads to
make this decision.

Historic catalogue reconciliation
-----------------------------------

Releases before 1.22.19 could derive a card identity from transient page and
presentation routes. Operators can inspect affected private metadata without
making changes:

.. code-block:: console

   pinakotheke catalogue reconcile-x-images --root ~/.x-img

The report contains counts only; it does not print source URLs, media
identifiers, account names, ObjectStore keys, or checksums. To apply the result,
first stop Pinakotheke and its capture worker, then run:

.. code-block:: console

   pinakotheke catalogue reconcile-x-images \
     --root ~/.x-img \
     --apply \
     --confirm-service-stopped

The command creates private timestamped backups of the gallery and capture-plan
metadata. It then binds all eligible plans and cards to the stable identity,
collapses exact duplicate cards, keeps the smallest independent stored
thumbnail, and keeps the largest ready stored original. Existing ``Removed``
or ``Hidden`` review intent is preserved conservatively. Endpoint and logical
ObjectStore identities must match exactly; ambiguous or cross-store groups fail
closed.

The same guarded pass backfills a safe X presentation link from a settled
capture-plan record when the gallery card does not already have one. This lets
the quick preview offer an explicit source-open action for a thumbnail-only
record without making an origin request itself. Query-bearing media retrieval
capabilities are never used as source links.

No DASObjectStore object is deleted. Redundant immutable representations remain
available to DASObjectStore retention and administration policy even when they
are no longer repeated in the Pinakotheke gallery projection.

Recovery
--------

Do not apply reconciliation while Pinakotheke is running. If a process or host
failure interrupts the operation, keep the service stopped and restore both
matching ``pre-x-image-reconcile`` metadata backups before retrying. A
successful second dry run reports zero duplicate groups, redundant cards,
renames, and plan updates.
