Native packages and Firefox bundles
====================================

The repository ``Makefile`` builds local release artifacts under ``dist/``.
The native packages contain the Pinakotheke monolith/CLI, the compiled Yew/WASM
application, the versioned Monas product-bootstrap contract, and MPL-2.0
license. They do not install Monas, DASObjectStore, user media, or credentials;
those authority and payload boundaries remain separate.

DASObjectStore is nevertheless a required, separately installed runtime
product. The verified minimum is supplied only by the current RC compatibility
manifest. The DEB declares ``Depends: dasobjectstore (>= <minimum>)`` and the
RPM declares ``Requires: dasobjectstore >= <minimum>`` so native package
managers can resolve the published dependency. The macOS PKG checks for an
independently installed ``dasobjectstore`` executable, parses its version, and
stops with an actionable prerequisite message when it is absent or older than
the verified minimum. Install or upgrade DASObjectStore independently before
Pinakotheke; removing Pinakotheke does not remove ObjectStore data.

The RC packaging contract was checked against Monas ``0.9.1`` commit
``dbcc70577d28f2c4a619eafeaee9e399798bec5c`` and DASObjectStore ``0.145.3``
commit ``f8ac52f930a440ab725b6ecb61ef6c8fac8d535e``. Public builds consume only
checked-in contracts and published package versions; they have no sibling path
dependency.

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

Every native package target depends on ``make web``. Linux BuildKit receives
``dist/web`` as a named, read-only build context and installs it under
``/usr/share/<product>/web``. The macOS builder installs the same output under
``/usr/local/share/<product>/web``. The corresponding absolute location is
compiled into the packaged monolith, so an installed ``pinakotheke serve`` does
not require ``--web-root``. At runtime a valid ``ROOT/web`` directory takes
precedence, and an explicit ``--web-root`` takes precedence over both. All
three paths receive the same file-count, byte-size, regular-file, index, and
no-symlink validation. A missing packaged tree is reported as ``Not installed``
rather than falling back to an origin website.

``make macos-pkg`` requires macOS, Rustup, and Apple's ``pkgbuild`` from the
Xcode command-line tools. It produces x86_64 and arm64 PKGs. These development
packages are unsigned; release signing/notarization identities must be supplied
by the release operator and are not stored in this repository.

``make firefox`` creates one deterministic, platform-independent XPI from an
explicit tracked source allowlist. Local Mozilla upload state is excluded.
Public Firefox distribution still requires the applicable Mozilla
signing/listing process.

Artifacts and verification
--------------------------

Expected outputs are:

* Linux x86_64/arm64: four DEB/RPM files;
* macOS x86_64/arm64: two PKG files; and
* one platform-independent Firefox XPI; and
* one deterministic CycloneDX 1.6 software bill of materials.

``make sbom`` inventories locked third-party Rust packages and the Firefox
application component without contacting a hosted service. ``make checksums``
writes ``dist/SHA256SUMS`` and a deterministic
``dist/release-manifest.v1.json``. The manifest identifies each artefact's
kind, operating system, architecture, byte length, SHA-256, and signing state;
development outputs explicitly say ``signed: false``. ``make verify`` requires
the exact artifact set, validates the SBOM, XPI manifest and product version, and
rejects mixed or unlisted historic files as well as
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
