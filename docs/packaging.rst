Native packages and Firefox bundles
====================================

The repository ``Makefile`` builds local release artifacts under ``dist/``.
The current native packages honestly contain the ``x-img`` metadata CLI, the
versioned Monas product-bootstrap contract, and MPL-2.0 license. They do not
claim to install a standalone API daemon, Monas, DASObjectStore, user media, or
credentials; those runtime/package boundaries remain separate work.

The packaging contract was checked against sibling Monas commit
``3d21b0bc7b83fa8408d01b93347a56f43f3a96b7``. Public builds consume only the
checked-in bootstrap contract and have no sibling path dependency.

Targets
-------

.. code-block:: console

   make help
   make linux
   make macos-pkg
   make firefox
   make packages
   make checksums
   make verify

``make linux`` uses Docker Buildx and a digest-pinned Rust 1.97 Bookworm image.
The compiler runs on the container's native architecture and GNU cross-linkers
produce ``x86_64-unknown-linux-gnu`` and ``aarch64-unknown-linux-gnu`` binaries,
avoiding unreliable QEMU execution of ``rustc``. Each architecture yields one
DEB and one RPM. Docker Desktop or an equivalent BuildKit daemon must be healthy
and have enough local image space.

``make macos-pkg`` requires macOS, Rustup, and Apple's ``pkgbuild`` from the
Xcode command-line tools. It produces x86_64 and arm64 PKGs. These development
packages are unsigned; release signing/notarization identities must be supplied
by the release operator and are not stored in this repository.

``make firefox`` creates deterministic XPIs labelled for macOS, Windows, and
Linux on x86_64 and arm64. WebExtension source is platform-independent, so the
six packages intentionally contain identical extension files; explicit names
make the requested distribution matrix and checksums reviewable. Public Firefox
distribution still requires the applicable Mozilla signing/listing process.

Artifacts and verification
--------------------------

Expected outputs are:

* Linux x86_64/arm64: four DEB/RPM files;
* macOS x86_64/arm64: two PKG files; and
* Firefox macOS/Windows/Linux x86_64/arm64: six XPI files; and
* one deterministic CycloneDX 1.6 software bill of materials.

``make sbom`` inventories locked third-party Rust packages and the Firefox
application component without contacting a hosted service. ``make checksums``
writes ``dist/SHA256SUMS`` and a deterministic
``dist/release-manifest.v1.json``. The manifest identifies each artefact's
kind, operating system, architecture, byte length, SHA-256, and signing state;
development outputs explicitly say ``signed: false``. ``make verify`` requires
all thirteen artifacts, validates the SBOM, six XPI manifests and product version, and
rejects missing or stale checksum and release manifests. ``make quality``
checks packaging sources alongside the existing local quality and release
audits without requiring hosted CI.

Pinakotheke cutover mode
------------------------

Packaging has one strict product switch. ``PRODUCT=x-img`` is the default and
keeps 0.9 artifact names, installation paths, bootstrap, SBOM identity, and
Firefox manifest unchanged. After the workspace version and coordinated
identity have moved to 1.0.0, the release operator uses:

.. code-block:: console

   make packages PRODUCT=pinakotheke

Canonical mode produces ``pinakotheke-*`` DEB, RPM, PKG, XPI, checksum,
manifest, and SBOM identities. Native packages install ``pinakotheke`` as the
canonical command and retain ``x-img`` as the compatibility alias. They consume
the reviewed Pinakotheke Monas bootstrap candidate; Firefox consumes the
canonical manifest candidate while retaining its shipped Gecko ID. Linux
package metadata explicitly replaces/conflicts with the old package rather than
allowing two installations to own ``/usr/bin/x-img``.

The 0.9 workspace cannot build a falsely labelled 1.0 native release because
the container verifies the Rust workspace version. The safe preparation check
builds a temporary canonical XPI, checks every candidate source and alias, and
retains no output:

.. code-block:: console

   python3 packaging/check_v1_plan.py

Before the live cutover, the production package transition is exercised with:

.. code-block:: console

   make v1-package-transition

This builds temporary canonical 1.0.0 DEB and RPM packages for x86_64 and
arm64, then uses pinned, network-isolated Debian and Fedora containers to move
from the published x-img 0.9.0 package to Pinakotheke 1.0.0 and back. The check
requires the canonical command and the warning-emitting legacy command at 1.0,
verifies the active canonical Monas product identity, and proves a synthetic
metadata snapshot remains byte-exact. Temporary packages and state are removed
after the check. RPM declares the canonical package as the provider and
successor of x-img; DEB declares the equivalent provides/conflicts/replaces
relationship.

Troubleshooting
---------------

An unsupported host fails with an explicit prerequisite message. If Docker
reports storage or content-database errors, free generated build cache and
restart Docker before retrying; the Makefile does not silently fall back to an
artifact for the wrong architecture. Use ``make clean`` only to remove generated
``dist/`` and packaging scratch.
