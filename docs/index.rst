x-img documentation
====================

x-img is a planning-stage private media acquisition and review service. This
documentation records the authority, policy, storage, scheduling, pairing, and
cache contracts that must be satisfied before implementation.

Architecture decisions
-----------------------

.. toctree::
   :maxdepth: 2

   adr/index
   compatibility-matrix

Configuration
-------------

.. toctree::
   :maxdepth: 1

   configuration
   acquisition
   bioinformatics-commit
   monas-product
   host-context
   das-application-identity
   destinations
   object-ingest
   object-read
   cache-alias-lookup
   image-substitution
   mp4-substitution
   segmented-video-gate
   x-oauth
   x-followed-accounts
   x-media-discovery
   instagram-media-discovery
   account-refresh
   firefox-extension
   firefox-toolbar
   firefox-capture
   website-capture-review
   site-adapters
   review-admission
   quick-preview
   video-candidates
   normalized-video-profiles
   video-normalization
   direct-playback
   identity-migration
   migration-export-restore
   deletion-compliance
   operations
   release-audits
   packaging
   upgrade-rollback-acceptance

Release and quality
-------------------

.. toctree::
   :maxdepth: 1

   release-quality-policy
   fault-injection-recovery

The local documentation container is the reproducible verification authority:

.. code-block:: console

   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check
