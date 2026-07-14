Release and quality policy
==========================

This policy defines the evidence required to merge, release, and support
``x-img``. It applies to the Rust workspace, command-line interface, Axum
service, Yew application, Firefox extension, schemas, fixtures, containers,
and user documentation. A release is a reproducible set of those surfaces,
not merely a tag or a compiled binary.

Version authority and Semantic Versioning
------------------------------------------

Releases follow Semantic Versioning. Before 1.0.0, incompatible planning or
contract changes may advance the minor version, but they must still be called
out as breaking changes with a migration path. From 1.0.0 onward:

* a patch release fixes compatible behavior without changing a public
  contract;
* a minor release adds backward-compatible behavior or additive contract
  fields; and
* a major release may remove or incompatibly change CLI, HTTP, configuration,
  catalogue, pairing, extension, or host-adapter contracts.

The Rust workspace package version is the single editable product-version
source. CLI output, crate packages, product and host
manifests, Firefox extension metadata, Sphinx ``release``, generated examples,
and release artefact names must be derived from it. They must not carry
independently maintained release numbers. A repository check must parse every
derived surface and fail on drift before merge or release. ``MILESTONES.md``
and ``TODO.md`` record the current release line and ``docs/conf.py`` mirrors
the workspace package version for Sphinx.

Schema and adapter versions are compatibility versions, not product versions.
They change only when their own compatibility rules require it. Readers must
reject unknown future schema majors, and releases must include explicit,
non-destructive migration fixtures whenever a stored or exchanged contract
changes. The coordinated 1.0.0 rename to Pinakotheke is governed separately by
XIMG-009 and cannot be performed as a partial version-only change.

Supported toolchains and browsers
---------------------------------

Every release records its exact Rust toolchain, minimum supported Rust version
(MSRV), Firefox release range, Firefox ESR range, WebAssembly target, and
container platforms in the compatibility matrix. The planning baseline was
reviewed on 2026-07-14 against Rust stable 1.97.0, Firefox Release 152.0.5, and
Firefox ESR 140.12.0esr. These are evidence timestamps, not promises that an
unimplemented surface has passed acceptance testing.

The initial 0.2.x implementation line targets Rust 2024, pins Rust 1.97.0 for
development and release, and declares Rust 1.95.0 as its MSRV. This follows a
stable-minus-two-release-trains policy and freezes that MSRV for the complete
0.2.x line. The initial Firefox implementation targets the current Release and
current ESR channels, with Firefox 140 as the proposed oldest major. It may set
``browser_specific_settings.gecko.strict_min_version`` to ``140.0`` only after
the complete real-browser suite passes on that ESR. During an ESR transition,
both supported ESR majors are required until the older line is formally
retired. Exact tested browser versions are recorded for every extension
release.

The evidence sources are the `Rust release manifest
<https://static.rust-lang.org/dist/channel-rust-stable.toml>`_, Cargo's
`rust-version guidance
<https://doc.rust-lang.org/stable/cargo/reference/rust-version.html>`_, the
`Mozilla product-details feed
<https://product-details.mozilla.org/1.0/firefox_versions.json>`_, and
Mozilla's `Firefox channel policy
<https://firefox-admin-docs.mozilla.org/guides/firefox-channels/>`_. A release
must not claim a floor that has not run the required checks.

The Rust policy is:

* XIMG-020 provides ``rust-toolchain.toml`` pinned initially to 1.97.0;
* ``rust-version`` in workspace package metadata declares the initial 1.95.0
  MSRV and every crate inherits it;
* the lock file is tested both with the pinned toolchain and the MSRV; and
* raising the MSRV occurs only at a product minor boundary (unless an urgent
  security or correctness fix requires a documented exception), is announced
  in the changelog, and includes dependency-resolution evidence.

The Firefox policy is:

* support the current stable Firefox and the current Firefox ESR at release
  time, plus any transition ESR or older floor explicitly named in the
  compatibility matrix;
* test manifest parsing, permissions, pairing, observation/open eligibility,
  redirect and range behavior, cache substitution, and origin fail-open in a
  real Firefox instance;
* derive extension compatibility metadata from the tested range and never
  promise a browser version based only on successful packaging; and
* treat an increase to the minimum Firefox version as a deprecation-governed
  compatibility change.

Release verification matrix
---------------------------

The required local verification matrix grows with implemented surfaces. Checks
that do not yet have code must be tracked as release gates rather than reported
as passing. At minimum the completed product runs:

* Rust formatting, workspace linting with warnings denied, native unit and
  integration tests on Linux, macOS, and Windows, the pinned Rust toolchain,
  and MSRV; stable and MSRV cells are required while beta is advisory;
* WebAssembly build, lint, and tests for the Yew target;
* schema validation, compatibility and migration fixtures, connector fixtures,
  state-machine/idempotency tests, and crash reconciliation;
* Monas and DASObjectStore versioned contract tests without unpublished sibling
  path dependencies;
* Firefox packaging, static validation, and real-browser tests on stable and
  ESR, including permission, capture, redirect, conditional request, byte
  range, segmented-media capability, and fail-open cases;
* accessibility checks, including keyboard/focus behavior and WCAG 2.2 AA
  contrast for implemented views;
* dependency, vulnerability, secret, licence, and repository payload scans;
* a clean public-clone build proving that sibling repositories and private
  fixtures are not required; and
* planning-link, Sphinx reference, JSON schema, and checked-in fixture checks.

Operating-system or browser exclusions require a documented exception. A green
subset never substitutes for a failing or skipped required local matrix cell.
Each command records its own result so one failure does not hide other local
evidence.

Hosted CI is deliberately non-blocking while GitHub Actions funding is
unavailable. Work is verified and delivered from recorded local checks; no
pull-request, push, workflow, or status check may prevent progress, commits,
or pushes. The checked-in GitHub workflows are manually dispatched advisory
mirrors only. Their results may be backfilled or the work migrated to another
CI system later, but they never replace the local verification matrix.

Dependencies, security, and licensing
--------------------------------------

Dependencies must be necessary, maintained, compatible with MPL-2.0
distribution, and pinned or locked sufficiently for reproducible builds.
Compatibility-sensitive wire, browser, container, FFmpeg, and toolchain
dependencies require an exact reviewed version and upgrade evidence. Public
builds may consume published packages or vendored, versioned, redistributable
contracts; they must not require ``../monas``, ``../DASObjectStore``,
``../mnemosyne``, or ``../mnemosyne_design_language`` path dependencies.

Every dependency change records purpose, licence, source, version, security
status, and affected compatibility evidence. The release gate scans Rust,
Python documentation, JavaScript/extension, and container dependencies for
known vulnerabilities and unacceptable licences. Unresolved findings block a
release unless the exception process below records bounded exposure,
mitigation, expiry, and ownership. Lock files and pinned container digests are
reviewed artefacts and must be updated intentionally.

Security reports must use the repository's private reporting path once it is
published; secrets, private URLs, browsing history, account lists, or user
media must never be pasted into a public issue. A confirmed vulnerability is
triaged for a compatible patch where possible. Revoked credentials, deleted or
protected source content, and compromised release artefacts use an expedited
response and removal process without waiting for a routine release train.

Fixture privacy and provenance
------------------------------

Fixtures must be synthetic or demonstrably redistributable. They must not
contain real user account lists, downloaded media, private accessions, browser
history, tokens, cookies, passwords, sessions, signing material, signed query
parameters, private URLs, Monas credentials, DASObjectStore credentials, or
durable payload copies. Media-like test payloads must be minimal, licence-
documented test assets and must never be captured user content.

Contract fixtures identify their schema, generating source or procedure,
licence, expected result, and compatibility purpose. Secret and payload scans
run on the repository and packaged artefacts. A fixture derived from a sibling
contract records the inspected sibling commit but copies only the versioned,
redistributable shape. Test failures and logs must apply the production
redaction rules.

Documentation authority
-----------------------

Precise user-facing documentation is maintained in ``docs/`` as a Sphinx
project with Read the Docs-compatible configuration and reStructuredText entry
points. The pinned local container is the authoritative build environment:

.. code-block:: console

   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check

Both commands must succeed with warnings treated as errors. GitHub Actions may
repeat this build and publish its output, but a remote check does not replace
the locally reproduced container verification and its recorded result.
Documentation changes must accompany behavior, workflow, configuration,
compatibility, failure-state, security, and migration changes in the same
release quantum.

Definition of Done
------------------

A change is done only when all applicable acceptance criteria are implemented;
tests cover success, denial, failure, and recovery paths; versioned contracts
and fixtures are compatible; privacy and authority boundaries remain intact;
and applicable required local matrix commands are green. User-facing Sphinx documentation,
``TODO.md``, ``MILESTONES.md``, and ``CHANGELOG.md`` must agree with the
delivered behavior. The local documentation container must build and verify,
the version-drift check must pass, and the public clone must not rely on
unpublished sibling files.

The coordinator reviews staged paths and diffs, runs ``git diff --check``,
commits one coherent modification with its TODO identifier, pushes it, records
the commit as evidence, stops every delegated agent, and releases the
single-run lease. Partial work remains unchecked and receives a concise backlog
handoff rather than a completion claim.

The planning-era repository check is run locally with:

.. code-block:: console

   scripts/quality/check.sh

It checks local Markdown/reStructuredText and Sphinx targets, strict JSON and
schema-major structure, fixture privacy, and current version mirrors. Full
runtime JSON Schema validation and Rust/Firefox matrix cells become mandatory
as their implementation tasks introduce those surfaces; a structural planning
check must never be presented as runtime contract validation.

Cross-repository contract evidence
----------------------------------

``scripts/contracts/check.sh`` is the dependency-free baseline contract
check. It verifies x-img-owned, versioned Monas and DASObjectStore contract
fixtures and rejects unpublished sibling path dependencies, so it must pass in
a clean public clone. It is run locally for each applicable change; the GitHub
quality workflow is a manually dispatched, non-blocking mirror only.

When the four compatibility siblings are checked out at the revisions recorded
in ``docs/compatibility-matrix.md``, run the stronger source-anchor inspection:

.. code-block:: console

   scripts/contracts/check.sh --require-siblings

It verifies the exact Monas, DASObjectStore, Synoptikon/Mnemosyne, and design
language commits plus the contract paths x-img relies on. ``XIMG_SIBLING_ROOT``
may point to a directory containing those four checkouts when they are not
adjacent to x-img. The manually dispatched ``sibling-contracts`` GitHub
workflow performs the same pinned inspection from public sources.

This is intentionally a compatibility check, not a substitute for a live
authority integration test: x-img has no live Monas or DASObjectStore transport
adapter yet, and no workflow receives production credentials. Once such
adapters exist, their opt-in integration environment must exercise the
published wire contracts with scoped test identities while retaining this
sibling-free baseline.

Release checklist
-----------------

#. Confirm the target version and classify every change under Semantic
   Versioning; document breaking changes, migrations, and deprecations.
#. Run the version synchronization check and verify package, CLI, service,
   extension, product, documentation, and artefact metadata.
#. Freeze and review dependency locks, pinned toolchains and containers,
   licences, advisories, and the supported compatibility matrix.
#. Run all required native, WebAssembly, schema, contract, Firefox,
   accessibility, security, privacy, and public-clone checks.
#. Build and verify Sphinx locally using ``docs/Dockerfile`` and inspect the
   user workflows and failure guidance affected by the release.
#. Confirm that fixtures and packaged artefacts contain no secrets, private
   source data, browsing history, user media, or x-img-local durable payloads.
#. Update the changelog, milestone evidence, migration and rollback guidance,
   and release notes; verify links and schema identifiers.
#. Create the signed or otherwise project-approved tag from a clean reviewed
   commit, build artefacts from that tag, verify checksums and provenance, and
   publish without rewriting history.
#. Perform post-release installation, Monas/DASObjectStore contract, Firefox
   pairing/playback/fail-open, and documentation checks; record rollback or
   incident action if any check fails.

Deprecations and exceptions
---------------------------

A deprecation names the affected public surface, replacement, first warning
release, earliest removal release, migration steps, compatibility fixtures,
and owner. Runtime or validation warnings must not expose private data. After
1.0.0, a public surface is not removed before a major release unless continued
support would create an urgent security, legal, privacy, or data-integrity
risk. Emergency removal is documented with the reason and safest available
migration or rollback.

An exception is a checked-in, time-bounded decision, not an informal skipped
check. It records scope, rationale, risk, compensating controls, owner, approval
date, expiry or review milestone, and the issue/TODO that removes it. Exceptions
cannot authorize credential capture, DRM circumvention, durable local media,
silent ObjectStore switching, unknown-schema acceptance, or bypass of Monas or
DASObjectStore authority. An expired exception blocks release. Release notes
identify any user-visible limitation without disclosing exploitable detail.
