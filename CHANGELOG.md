# Changelog

All notable changes to x-img will be documented in this file. The project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Planning

- Added XIMG-033's bounded streaming object-ingest port. It forwards chunks to
  an authority backend without local payload staging, enforces chunk/length and
  SHA-256 verification, surfaces backpressure, checks authority completion, and
  returns only an idempotent verified receipt for a repeated ingest ID.
- Added XIMG-032's scoped DASObjectStore application-identity contract and
  authorization gate. It binds one endpoint, ObjectStore, prefix, narrow
  operation set, byte limits, expiry, and opaque authority references; tests
  fail closed for expired, replayed, wrong-store, wrong-prefix, and oversized
  operations without storing credentials or issuing tokens.
- Added XIMG-031's authenticated host-context adapter. Privileged Axum routes
  require a host-injected, authorized non-secret context and reject direct
  access; fixture-tested Monas and Synoptikon adapters share the same boundary
  without x-img accepting, logging, configuring, or issuing sessions.
- Added XIMG-030's versioned Monas product registration and synthetic strict
  fixtures. It requires host-owned Prosopikon authentication, one x-img
  application/API mount, a DASObjectStore requirement, capability disclosure,
  and a future Synoptikon-equivalent bootstrap; it rejects anonymous access and
  direct x-img login-route declarations without adding a live host dependency.
- Added the XIMG-020 Rust 2024 workspace scaffold at product version 0.2.0:
  shared model and core crates, a `clap` CLI reporting the workspace version,
  an Axum composition boundary, and a Yew WebAssembly boundary. The pinned
  Rust 1.97.0 toolchain, Rust 1.95 MSRV metadata, lint policy, MPL notices, and
  lockfile are in place; no live source, storage, authentication, or media
  payload integration is enabled.
- Added XIMG-021 strict local configuration parsing, validation, safe atomic
  replacement, and identifier-only listing for versioned X, Instagram, and
  website rules. It rejects unknown fields and schema majors, duplicate source
  identities, invalid account names, unsafe wildcard/non-origin websites, and
  missing/incompatible opaque authority references without contacting any
  source or authority.
- Added XIMG-022's platform-neutral acquisition state machine. It accepts only
  the verified settlement path, makes failure/policy/cancellation/conflict
  terminal, requires bounded immutable ObjectStore evidence before commit, and
  prevents every review state from being assigned before a verified commit.
- Added XIMG-023 deterministic idempotency and reconciliation: canonical media
  identity plus immutable checksum settles one metadata record, replay appends
  safe aliases without replacing an object reference, and absent/mismatched
  authority observations remain pending or conflict without any byte, network,
  or persistent-storage operation.
- Added XIMG-024 in-memory scheduler contracts for coalesced global refreshes,
  mutually exclusive source leases, bounded child/request/byte/time budgets,
  and cooperative cancellation. No connector, queue, persistence, or network
  execution is enabled.
- Added XIMG-025's strict synthetic X/Instagram connector-fixture matrix and
  test contract, covering pagination, edits, deletions, duplicates, variants,
  rate limits, authorization expiry, malformed responses, and cursor reset
  without live traffic, account data, credentials, or media payloads.
- Added XIMG-026 metadata-only GEO/SRA/ENA/NCBI transfer plans with one
  explicit accession/URL, bounded resolved file evidence, transport, rights,
  endpoint/ObjectStore destination, policy blocking, and explicit confirmation;
  no repository discovery or byte transfer is enabled.
- Added XIMG-027 synthetic bioinformatics resolution/transport fixtures for
  ENA/SRA, GEO, NCBI, checksum/size evidence, retry/resume/cancellation,
  backpressure, and optional Aspera-to-HTTPS fallback without payloads or
  credentials.
- Added the XIMG-009 executable Pinakotheke v1 identity-migration plan,
  including a complete rename surface matrix, a minimum compatibility window,
  migration and rollback rules, retained schema/object/extension identities,
  and required configuration, catalogue, ObjectStore, and pairing proof cases.
- Established the XIMG-008 release and quality policy: Semantic Versioning and
  version authority, Rust/MSRV and Firefox Release/ESR support, the required CI
  matrix, dependency/security/licence governance, fixture privacy, release and
  exception procedures, and a precise Definition of Done. Added dependency-
  free local link/JSON/schema-major/privacy/version checks plus a SHA-pinned
  GitHub Actions mirror; local containerized Sphinx verification remains the
  documentation authority.
- Added the XIMG-007 Firefox architecture spike ADR and synthetic fixture
  matrix, covering WebRequest/DNR limits, bounded response filtering, exact
  origin permissions, HTTPS and response contracts, signed-URL redaction,
  observed-thumbnail versus explicit-original eligibility, segmented-video
  capability gates, and fail-open behavior.
- Added the XIMG-006 versioned acquisition/catalogue metadata contract for
  source items, canonical media identities, DASObjectStore references,
  attempts, leases, cursors, review state, tombstones, audit events, and
  crash/idempotency reconciliation, with strict synthetic fixtures.
- Added XIMG-005 versioned draft-2020-12 configuration schemas and synthetic
  positive/negative fixtures for the instance, X, Instagram, and website
  policy contracts, with strict fields and host-managed references documented
  in the Sphinx configuration guide.
- Added the XIMG-004 architecture ADR set for authority boundaries,
  metadata-only local state, canonical identity and idempotent settlement,
  review lifecycle, bounded account refresh scheduling, Firefox pairing, and
  external-cache fail-open behavior, with a pinned local Sphinx verification
  container.
- Added the XIMG-003 sibling compatibility matrix with pinned Monas,
  DASObjectStore, Mnemosyne design-language, and future Synoptikon revisions,
  fixture anchors, public-build independence rules, and explicit host/range
  contract risks.
- Added the Pinakotheke v1.0.0 coordinated rename/rebrand gate, including
  compatibility aliases and repository migration planning.
- Added the Sphinx/Read the Docs local-container documentation authority and
  stricter Firefox observation, opt-in, fail-open, and status-display rules.
- Added the planned explicit GEO/SRA/ENA/NCBI resource workflow: reviewable
  accession plans, rights gates, direct DASObjectStore streaming, checksum and
  provenance requirements, optional Aspera with HTTPS fallback, and no bulk
  discovery or durable x-img-local payloads.
- Added endpoint/device versus logical ObjectStore planning: managed local
  folder-profile bootstrap, remote HTTPS pairing and discovery, explicit
  endpoint-plus-store selection, commit-time capability checks, stable-ID
  provenance, and safe reconnect behavior without silent destination changes.
- Added user-selected video planning: policy-gated candidate review,
  versioned Firefox playback profiles, normalized DASObjectStore renditions,
  pinned containerized FFmpeg, bounded scratch/cleanup, explicit failure states,
  and no automatic crawling, DRM circumvention, or source-only readiness.

## [0.1.0] - 2026-07-14

### Added

- Initial public planning baseline.
- Dependency-ordered milestones and TODO backlog for X, Instagram, Firefox,
  Monas, DASObjectStore, and future Synoptikon integration.
- Contributor rules for single-run automation, subagent delegation, focused
  commits, immediate pushes, data authority, privacy, and design conformance.
- Mozilla Public License 2.0.

[Unreleased]: https://github.com/sagrudd/x-img/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/sagrudd/x-img/releases/tag/v0.1.0
