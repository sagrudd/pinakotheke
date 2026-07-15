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
* Firefox macOS/Windows/Linux x86_64/arm64: six XPI files.

``make checksums`` writes ``dist/SHA256SUMS`` and a deterministic
``dist/release-manifest.v1.json``. The manifest identifies each artefact's
kind, operating system, architecture, byte length, SHA-256, and signing state;
development outputs explicitly say ``signed: false``. ``make verify`` requires
all twelve artifacts, validates the six XPI manifests and product version, and
rejects missing or stale checksum and release manifests. ``make quality``
checks packaging sources alongside the existing local quality and release
audits without requiring hosted CI.

Troubleshooting
---------------

An unsupported host fails with an explicit prerequisite message. If Docker
reports storage or content-database errors, free generated build cache and
restart Docker before retrying; the Makefile does not silently fall back to an
artifact for the wrong architecture. Use ``make clean`` only to remove generated
``dist/`` and packaging scratch.
