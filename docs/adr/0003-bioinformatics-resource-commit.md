# ADR 0003: User-initiated bioinformatics resource commit

- Status: Accepted for XIMG-038's confirmed streaming contract; live provider
  and DASObjectStore transport remain gated by XIMG-002 policy and adapters
- Date: 2026-07-14
- Deciders: x-img maintainers
- Scope: GEO, SRA, ENA, and NCBI public-resource acquisition

## Context

Pinakotheke's v1 scope includes trivial direct commit of user-identified public
bioinformatics resources alongside X and Instagram media. This is a distinct
resource-adapter family, not a generic crawler and not a Firefox site adapter.
The user supplies an accession or URL; the service resolves a bounded,
reviewable plan and writes only after confirmation.

Compatibility-sensitive inspection pins:

| Sibling | Commit | Relevant contracts inspected |
| --- | --- | --- |
| `../epithema` | `619b7700536a93aacbd928e82bd6bfa9d8070877` | `crates/epithema-providers/src/ena_ngs.rs`, `sra_ngs.rs`, `ena.rs`, `sra_archive.rs`; `crates/epithema-diagnostics/src/provenance.rs`; NGS fixtures under `crates/epithema-providers/tests/fixtures/ngs/` and `crates/epithema-testkit/tests/fixtures/acceptance_anchors/` |
| `../epic_collection` | `07ade3c4e9a7630dc2e444f5e181fe3bc121fa21` | `scripts/discover_geo.py`; `catalog/human_methylation_geo_raw.toml`, `catalog/epic_seed.toml`; `docs/literature-review.md`; `docs/TODO_DOWNLOAD.md` |
| `../DASObjectStore` | `76f6411eab1e2c486c0bc1b4695b71f09307d9df` | `docs/application-authentication.md`, storage/read/upload contracts, and `crates/dasobjectstore-core/src/application_auth.rs`, `object_catalogue.rs`; `crates/dasobjectstore-daemon/src/api/provider_stream.rs` |

The public x-img build must not depend on these sibling paths. Their behavior
is input to versioned x-img wire contracts, copied synthetic fixtures, and
narrow adapters only.

## Decisions

### User intent and plan confirmation

- The authenticated `clap` CLI and Monas-hosted Axum/Yew task pane accept an
  explicit accession or URL, such as `GEO:GSE110544`, `ENA:ERR400001`,
  `SRA:SRR123456`, or an NCBI accession/URL. Input count is bounded and there
  is no bulk discovery, broad accession search, repository crawl, or implicit
  catalog trawl.
- Resolution produces a plan before any payload transfer. The plan includes
  source authority/database and release, canonical accession/URL, selected
  files and filenames, expected sizes and checksums when available, transport
  (`HTTPS`, `FTP`, or optional `Aspera`), target endpoint and ObjectStore,
  prefix and object type, estimated bytes, licensing/usage metadata, policy
  result, and an explicit warning for missing evidence.
- The user reviews and confirms the plan. The server revalidates source policy,
  destination authorization, quotas, and capability immediately before each
  commit; the browser never supplies the policy decision or an arbitrary
  destination.

### Separate adapters, shared safety ports

GEO, SRA, ENA, and NCBI adapters own accession resolution, provider metadata,
file-list normalization, transport selection, and provider-specific retry or
resume behavior. They do not share Firefox/site-detection code. Both families
use the same x-img ports for authenticated actor context, bounded job leases,
backpressure, provenance, idempotency, review/audit state, and
DASObjectStore ingest/read.

### Direct DASObjectStore settlement

- Resource bytes stream through a scoped DASObjectStore application identity or
  upload capability directly to the reviewed ObjectStore. x-img may use only
  tightly bounded ephemeral worker buffers/staging required by the authority;
  it must not retain a durable payload file in the product root, database,
  browser storage, logs, fixtures, or repository.
- The transfer computes and verifies the provider checksum (MD5/SHA where
  supplied, plus the x-img canonical checksum) and exact length. Catalogue
  admission occurs only after DASObjectStore verification and idempotent
  completion. A crash before or after completion reconciles to one committed
  object or one explicit failure; retries never overwrite a different checksum.
- Each committed record preserves source database/accession/release, canonical
  URLs, filename, source checksum, transport and tool version, discovery and
  commit timestamps, licensing/usage metadata, destination endpoint/ObjectStore
  identity, object key/checksum/reference, and policy result.
- Stable accession plus file identity is the primary deduplication key; the
  immutable checksum is the content guard. URL changes are aliases, not new
  resources unless the resolved file identity or checksum changes.

### Transport policy

- HTTPS is the baseline transport. FTP is permitted only where the source
  authority documents it and the plan records the risk and URL.
- Aspera is an optional optimized transport, never a requirement for the public
  build. It needs a separately approved adapter, pinned tool/container image,
  explicit binary/licensing availability, bounded resource limits, and HTTPS
  fallback. Aspera keys, credentials, and command lines never appear in config
  or logs. Synthetic manifests and transfer fixtures cover retry, resume,
  fallback, checksum mismatch, cancellation, and missing-tool behavior.
- Untrusted accession, filename, URL, or metadata is passed as structured
  arguments; no shell interpolation is permitted.

### Rights and source policy

Only public, redistributable, or user-authorized resources may be committed.
Controlled-access, embargoed, or license-uncertain records resolve to an
explicit `policy-blocked` plan and remain metadata-only until authorization and
terms are recorded. Public visibility alone is not a retention or redistribution
license. User-facing Sphinx documentation must explain accession examples,
rights confirmation, plan review, and the local container verification command.

## Failure modes

| Condition | Required result |
| --- | --- |
| Accession resolves to multiple authorities/files | stop at plan review; require explicit selection |
| Provider manifest lacks checksum or size | show missing evidence; apply bounded transfer and post-transfer verification |
| Source is restricted, embargoed, or rights-uncertain | `policy-blocked`; no payload transfer |
| HTTP/FTP/Aspera interruption | bounded retry/resume when supported; otherwise explicit failed state |
| Checksum or exact-length mismatch | quarantine/abort through DASObjectStore; never commit catalogue record |
| Crash around completion | reconcile upload capability and catalogue to one terminal result |
| DASObjectStore unavailable or destination policy changes | pause/fail safely; never write to local fallback storage |

## Acceptance tests

- CLI and task-pane fixtures accept one explicit accession/URL and reject bulk
  discovery, unbounded lists, wildcard repository scans, and implicit URLs.
- Plan fixtures render source authority, release, files, sizes/checksums,
  transport, target ObjectStore/prefix/type, estimated bytes, rights/policy,
  and confirmation state before any transfer.
- ENA/SRA fixtures prove accession classification, multi-run expansion,
  provider URLs, byte counts, MD5 fields, and provenance; GEO fixtures prove
  accession metadata, raw archive/file-list selection, sidecar manifest, and
  checksum behavior; NCBI fixtures prove explicit database/accession routing.
- The shared job fixture proves bounded streaming, cancellation, retry/resume,
  backpressure, checksum verification, crash reconciliation, and idempotent
  deduplication keyed by accession/file identity plus checksum.
- Aspera is optional and fixture-tested with HTTPS fallback; no test requires a
  proprietary binary or real credential.
- Static/privacy checks prove that no durable payload path is under x-img and
  no token, password, Aspera credential, or private URL is logged or persisted.
- A public-clone build passes without sibling checkout or unpublished path
  dependency; copied fixtures are synthetic or redistributable.

## User-facing documentation

The Sphinx/Read the Docs project must include reproducible examples for GEO,
SRA, ENA, and NCBI accession planning, rights/policy blocking, confirmation,
destination selection, retry/resume, and checksum failure. Its local
container build remains the release authority as required by `AGENTS.md`.
