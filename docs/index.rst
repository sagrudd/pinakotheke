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
   monas-product
   host-context
   das-application-identity
   identity-migration

Release and quality
-------------------

.. toctree::
   :maxdepth: 1

   release-quality-policy

The local documentation container is the reproducible verification authority:

.. code-block:: console

   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check
