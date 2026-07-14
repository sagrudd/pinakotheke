# ADR 0001: Authority boundaries, identity, and catalogue lifecycle

- Status: Proposed; implementation remains behind the XIMG-002 policy and
  XIMG-003 external-contract gates
- Date: 2026-07-14
- Deciders: x-img maintainers
- Scope: all source adapters, catalogue records, acquisition jobs, review
  state, host integrations, and ObjectStore writes

## Context

x-img combines user-selected social, website, and bioinformatics sources in
one private catalogue. The system must remain useful when a source URL rotates,
a worker crashes around an upload, an object store is unavailable, or a host
session is renewed. The repository is public, while catalogue configuration,
provenance, and acquired media are private user data.

Compatibility-sensitive contract pins used for this decision are:

| Sibling | Commit | Boundary inspected |
| --- | --- | --- |
| `../mnemosyne_design_language` | `5539df8f662a78ebdf7cf4c868d71831380c8cfd` | product context, task panes, status words, and footer contract in `docs/brief.md` and `docs/interface-patterns.md` |
| `../monas` | `3d21b0bc7b83fa8408d01b93347a56f43f3a96b7` | authenticated standalone product mount and host-owned session boundary in `README.md` |
| `../DASObjectStore` | `e4997eeb4a682effb705a091faf67c99e560a5b3` | daemon-owned storage, scoped application authentication, verification, and metadata recovery in `docs/architecture.md`, `docs/application-authentication.md`, and `docs/metadata-compatibility.md` |
| `../mnemosyne` | `ee21d98b23dec3caa6926d9d0dcc002989aa465b` | host product UI adapter and catalogue/host boundary in `mneion-api-types/schemas/HOST_PRODUCT_UI_ADAPTER.md` and `CATALOGUE_FORMAT.md` |

The public x-img build must consume versioned wire contracts or synthetic
fixtures; it must not require any sibling checkout or unpublished path
dependency.

## Decisions

### 1. Authorities and durable data

- Monas owns the standalone application shell, login, session cookie, and
  authenticated host context. x-img consumes that context and never issues a
  competing user account or session.
- A future Synoptikon adapter may supply the same host-context port. Host mode
  and capabilities are explicit adapter inputs; domain and connector logic do
  not probe host URLs or depend on a particular host implementation.
- DASObjectStore is the sole durable authority for image and video bytes. The
  x-img product root, database, browser storage, repository, logs, and fixtures
  may contain only metadata, references, bounded job state, or synthetic data.
- x-img owns source identity, account/site configuration, acquisition state,
  job leases, review state, tombstones, and audit/provenance records. Every
  object reference records the endpoint/ObjectStore identity supplied by the
  authority; display names are labels only.

### 2. Local metadata versus media bytes

Local records use explicit, versioned schemas and contain configuration,
canonical identifiers, source URL aliases, checksums, object references,
policy results, timestamps, state, and audit evidence. A transfer may use
bounded ephemeral buffers or authority-managed staging, but no x-img-local
payload file may become durable. Unknown future schema majors are rejected;
non-destructive migrations are explicit and tested.

### 3. Canonical source identity and idempotent settlement

Each source adapter produces a canonical source identity from its platform/site,
account or origin, source item identity, and media identity. A source URL or
signed URL is an alias and never the identity by itself. The catalogue keeps
aliases for URL rotation and duplicate discovery.

The settlement key is the canonical media identity plus the immutable verified
object checksum. A retry may repeat network work, but it must not overwrite a
different checksum or create a second committed record for the same key. If a
platform reuses an identifier with different content, the checksum mismatch
creates an explicit conflict requiring reconciliation rather than silent
replacement. If two canonical identities resolve to the same verified object,
the records may share a reference only through an explicit alias relation that
retains both provenance histories.

### 4. Acquisition and review lifecycle

The common state machine is:

`discovered -> claimed -> transferring -> stored -> verified -> committed`.

`failed`, `policy-blocked`, `cancelled`, `tombstoned`, and `conflict` are
explicit terminal or repairable outcomes as defined by the versioned contract.
Only `verified` may advance to `committed`; only a committed record with a
verified ObjectStore reference may enter `new`, `reviewed`, or retained review
states. A crash before or after DASObjectStore completion is reconciled from
the authority and the idempotency key before the catalogue reports success.

Deletion, permission loss, inaccessible sources, and policy changes stop new
review admission and produce an audit/tombstone state before any object-store
removal request. A source cursor is advisory; committed identity is the final
deduplication authority.

## Alternatives considered

- **Store payloads under the x-img product root:** rejected because it creates a
  second storage authority and makes quota, deletion, recovery, and access
  control inconsistent with DASObjectStore.
- **Let Monas or x-img own connector credentials and sessions:** rejected
  because host authentication and secret handling are external authorities;
  x-img must receive only an authenticated context or host-managed reference.
- **Use source URLs or cursors as deduplication keys:** rejected because URLs
  rotate and cursors are advisory; canonical identity plus checksum is stable.
- **Mark an item new when discovered or after upload starts:** rejected because
  a crash or partial object would expose an unusable review record.
- **Replace a committed object when an identifier changes content:** rejected
  because immutable bytes and provenance must remain auditable; reconciliation
  must create a conflict or a new identity.
- **Make the UI decide policy or destination:** rejected because browser input
  is untrusted; the server and authority revalidate policy, capability, quota,
  and the reviewed endpoint/ObjectStore immediately before commit.

## Failure modes and required behavior

| Condition | Required result |
| --- | --- |
| Missing or invalid Monas/Synoptikon host context | reject privileged request; do not create a local login |
| DASObjectStore unavailable or capability expired | pause/fail the job; retain retry evidence; never use local durable fallback |
| Upload completes but catalogue commit is interrupted | reconcile by upload identity, checksum, and authority verification; converge to one terminal result |
| Checksum or exact length differs | quarantine/abort through DASObjectStore; no `committed` or review state |
| Duplicate URL or source item | attach an alias/provenance event to the existing identity; do not transfer again after verified settlement |
| Canonical identity collision with a different checksum | create `conflict`; preserve both evidence sets; require explicit reconciliation |
| Policy, permission, deletion, or rights state changes | stop admission; record `policy-blocked` or tombstone; do not silently present current media |
| Unknown schema or contract major | reject before mutation and report an upgrade/migration requirement |
| Host or browser is unavailable during a cache read | fail open to the origin and do not expose credentials or browsing history |

## Privacy impact

The design minimizes x-img-local data to explicit configuration, source and
media identifiers, source URL aliases, checksums, object references, policy
results, job/review state, and audit evidence. Monas/Synoptikon host context,
tokens, signed queries, cookies, passwords, and private keys remain outside
normal x-img records and logs. Logs redact authorization, signed parameters,
session identifiers, private URLs, and browsing history. The user remains
responsible for rights, access control, retention, export, and deletion of the
private archive.

## Compatibility impact

The host-context, ObjectStore reference, identity, acquisition, review, and
audit shapes are versioned public contracts. This decision preserves a narrow
adapter boundary for both Monas and a future Synoptikon host, and preserves
endpoint/ObjectStore-qualified locations for reconnects and multi-store
catalogues. The `x-img` planning name remains unchanged until the coordinated
Pinakotheke v1.0.0 migration; no identifier or schema rename is implied here.

Sibling behavior recorded above is evidence for contract design, not a runtime
dependency. Any incompatible change requires a new schema major, migration,
and compatibility fixtures.

## Acceptance tests

- Static/privacy checks prove no durable media path, browser cookie, password,
  host session, signed query, or storage secret is stored in x-img state/logs.
- Contract fixtures reject anonymous or wrong-host privileged requests and
  accept Monas and Synoptikon-shaped host contexts through the same port.
- State-machine tests reject double claims, commit-before-verification,
  review-before-commit, invalid transitions, and unknown future majors.
- Crash-injection fixtures before upload, after upload, and around catalogue
  commit converge to one verified committed record or one explicit failure.
- Duplicate URLs, URL rotation, cursor reset, and platform-ID reuse fixtures
  preserve aliases and provenance without overwriting a different checksum.
- Deletion, policy-block, unavailable-object, and permission-loss fixtures
  prevent new/review admission and produce explicit audit/tombstone states.
- A public clone/build passes using versioned contracts or synthetic fixtures
  without any sibling-only path dependency or real private data.

## User-facing documentation

The Sphinx/Read the Docs documentation must explain the object authority,
metadata-only local state, verified settlement, identity aliases, review
admission, crash recovery, policy blocks, and the difference between an
unavailable object and a deleted source. The local `docs/Dockerfile` build is
the release authority when the documentation project is added.
