Pinakotheke 1.29.0 release-candidate authority
================================================

``1.29.0-rc.1`` is the first release candidate governed by a single,
machine-checked compatibility record:
``contracts/release/pinakotheke-1.29.0-rc.1.compatibility.json``.  The record
pins the tested Pinakotheke, Monas, DASObjectStore, Mnemosyne design-language,
and Mnemosyne commits; names every required capability; checks the copied wire
contract digests; and records the native-package and Firefox version mappings.
It contains no credential, private URL, media, or unpublished path dependency.

Version mappings
----------------

The Rust workspace remains the editable product-version authority.  RC
prerelease ordering and installer constraints require these derived forms:

.. list-table::
   :header-rows: 1

   * - Surface
     - RC value
   * - Product, Rust, web, API, Monas bootstrap, Synoptikon manifest
     - ``1.29.0-rc.1``
   * - Firefox ``version_name``
     - ``1.29.0-rc.1``
   * - Firefox numeric ``version``
     - ``1.29.0.1``
   * - Debian
     - ``1.29.0~rc.1``
   * - RPM
     - version ``1.29.0``, release ``0.rc.1``
   * - macOS package
     - ``1.29.0.1``

The compatibility checker derives these mappings and rejects disagreement.
It also rejects external Cargo path dependencies, unknown fields, unknown
schema majors, contract checksum drift, falsely verified dependencies, and a
release-ready state with any unresolved gate.

Runtime prerequisites
---------------------

Monas ``0.9.1`` reads Pinakotheke's installed product bootstrap and performs a
bounded loopback runtime preflight before it listens.  It requires the exact
running product version and exact host/product capability sets.  Mismatch,
redirect, timeout, unreachable backend, malformed or oversized response, or a
missing or additional capability fails startup closed.  Host-supported and
product-proven capabilities are reported separately.

DASObjectStore's compatibility minimum is read from the release manifest only
after the application-authorized delete path has released store-global logical
capacity reconciliation.  DEB and RPM package metadata carry that exact
minimum.  The macOS preinstall requires a separately installed
``dasobjectstore`` executable, parses its version, and rejects an older
installation.  No package bundles DASObjectStore.

Checks
------

For ordinary source verification:

.. code-block:: console

   make rc-compatibility-check
   make quality

Release preparation additionally requires clean sibling checkouts at the exact
pinned commits:

.. code-block:: console

   python3 scripts/release/check_rc_compatibility.py \
     --require-ready --require-siblings --verify-product-source

The tested Pinakotheke commit must be the release source or an ancestor followed
only by the documented evidence-closure files.  This prevents source or
packaging changes after testing while avoiding an impossible self-referential
commit hash in the checked-in manifest.

Fresh, exact release output
---------------------------

``make rc-output-prepare`` refuses a dirty source tree, a reused version
directory, a blocked compatibility record, mismatched source version, missing
sibling evidence, symlinks, and path escape.  The release-output manifest binds
the source commit, compatibility digest, derived version mirrors, ordered build
commands, local test evidence, artifact types, architectures, sizes, and
SHA-256 checksums.

Sealing requires exactly:

* x86_64 and arm64 DEB, RPM, and macOS packages;
* one platform-independent Firefox XPI;
* CycloneDX SBOM, source archive, checksum manifest, and machine-produced
  release evidence; and
* one verified x86_64 rollback package from the previous release.

The verifier rejects a missing, duplicate, extra, changed, symlinked, escaped,
or special-file artifact.  The source must still be clean at the same commit,
and compatibility and version evidence are revalidated at seal time.  “Sealed”
is a logical, checksum-verifiable state; filesystem retention policy remains
the release operator's responsibility.

Firefox packaging uses a tracked file allowlist and deliberately excludes local
upload state.  Because a WebExtension is platform-independent, the RC produces
one canonical unsigned XPI.  Mozilla signing is a later publication step, not a
source-build variant.

Rollback and deployment
-----------------------

Do not deploy from the historic mixed ``dist`` tree.  Deploy only a sealed
version directory whose exact inventory verifies.  Keep the manifest-bound
previous x86_64 package available, exercise the documented upgrade/rollback
test, and retain its checksum with the RC evidence.  A deployment must stop if
package metadata, runtime health, or compatibility evidence disagrees.

GitHub Actions is advisory and non-blocking.  Local Rust, schema, packaging,
security, deterministic Firefox, and pinned Sphinx-container checks are the
release authority.
