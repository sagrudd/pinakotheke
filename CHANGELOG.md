# Changelog

All notable changes to x-img will be documented in this file. The project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Planning

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
