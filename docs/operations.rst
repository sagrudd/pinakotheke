Health, metrics, and audit
==========================

Capture helper executables
--------------------------

Every executable configured for the DASObjectStore capture helper must be an
absolute executable regular file. Symlinks are rejected before acquisition so
an administrator cannot replace a reviewed tool target indirectly. Resolve a
distribution alias such as ``/usr/bin/timeout`` with ``readlink -f`` and store
the resulting regular-file path in ``timeout_executable``. A rejected helper
configuration leaves the plan unsettled; after correcting the private config,
restart Pinakotheke and its journal reconciliation retries the plan without a
duplicate committed object.

x-img exposes coarse public liveness and a separate host-authenticated
operations snapshot. Diagnostics are intended to answer whether the service and
its authority boundaries are usable without revealing what a user browsed,
captured, reviewed, or stored.

Public health
-------------

``GET /health`` returns only the versioned schema, ``alive`` process status,
product compatibility name, and package version. It contains no component
inventory, endpoint, ObjectStore, account, origin, job, audit, or request data.
This route proves process liveness, not dependency readiness.

Authenticated operations
------------------------

``GET /api/operations/v1/snapshot`` requires the same host-injected Monas or
approved future host context as other product APIs. It returns:

* word-first ``Ready``, ``Degraded``, or ``Unavailable`` state for host context,
  ObjectStore, scheduler, normalizer, and Firefox boundaries;
* saturating aggregate counters keyed only by fixed component and event enums;
* at most 128 recent fixed-code audit facts with an in-process sequence; and
* a count of older audit facts discarded by the capacity bound.

There are no free-form diagnostic messages. The schema cannot represent URLs,
origins, account or actor identity, source aliases, object keys, checksums,
credentials, cookies, authorization headers, Monas sessions, DAS capabilities,
signed queries, browsing history, media metadata, or payload bytes.

Gallery convergence has a separate authenticated aggregate at
``/products/pinakotheke/api/operations/v1/gallery-convergence``. It reports
only authoritative, projected, orphan, stale, and changed counts. See
:doc:`gallery-convergence` for its authority and failure semantics.

Host composition
----------------

The host retains a shared ``Arc<Mutex<OperationalTelemetry>>`` and passes it to
``router_with_operations``. Domain and adapter boundaries record only typed
component, event, and outcome values. Durable audit export remains host-owned
and must preserve the same redacted shape and access control.

An unavailable component determines overall ``Unavailable``; otherwise a
degraded component determines ``Degraded``. Operators should use the detailed
authenticated snapshot for readiness and keep public load-balancer checks on
the coarse liveness response.

Local proof
-----------

.. code-block:: console

   cargo +1.97.0 test -p pinakotheke-core operations
   cargo +1.97.0 test -p pinakotheke-api public_health_is_coarse_and_operations_require_host_context

Tests prove worst-state aggregation, fixed-capacity eviction, aggregate counts,
absence of prohibited diagnostic fields, coarse public output, and rejection of
unauthenticated operational detail.
