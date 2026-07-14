# x-img milestones

Status: planning baseline

Version: 0.2.0

Updated: 2026-07-14

Product identity: `x-img` is the planning/repository name until the coordinated
v1.0.0 migration to the Pinakotheke brand and target repository slug
`sagrudd/pinakotheke`.

## Product outcome

x-img provides one private, authenticated Web library for media acquired from:

- a small JSON allowlist of X/Twitter accounts;
- a small JSON allowlist of Instagram accounts; and
- websites enabled by the user through a Firefox extension.
- user-identified public GEO, SRA, ENA, and NCBI resources through an explicit
  review-and-confirm transfer plan.
- endpoint/device inventory and logical ObjectStore selection through the
  authorized DASObjectStore boundary, with useful local and remote operation.

A user can press one `Refresh accounts` action to schedule all enabled social
connectors. Newly committed items enter a visible review queue and are
distinguishable from reviewed items. Website captures and cache hits appear in
the same catalogue, with their originating site and canonical source URL.

## System boundary

```text
X connector -----------\
Instagram connector ----+--> x-img acquisition/catalogue --> DASObjectStore
Firefox extension ------/              |                       media bytes
                                        v
                              Monas-hosted Axum/Yew UI
                              login and session authority
                                        |
                              future Synoptikon adapter
```

- x-img owns source adapters, canonical media identity, acquisition jobs,
  review state, provenance, and cache lookup policy.
- Bioinformatics adapters are separate from Firefox/site adapters and accept
  only explicit accessions/URLs; both families share bounded jobs, review,
  provenance, idempotency, Monas identity, and DASObjectStore ports.
- DASObjectStore owns all durable image/video bytes, verification, object
  commit, read service, and storage credentials/capabilities.
- Monas owns the standalone application shell, login, session, and product
  mount. x-img consumes authenticated host context.
- The Firefox extension is a client of the same x-img instance. It owns neither
  the catalogue nor a second local cache.

## Cross-cutting release rules

Every milestone must:

- preserve MPL-2.0 licensing and public-repository hygiene;
- maintain Semantic Versioning and update the changelog on version changes;
- author precise user-facing documentation in Sphinx/Read the Docs format and
  build/verify it in the reproducible local `docs/Dockerfile` container; GitHub
  Actions may mirror the check but is not its authority;
- use versioned JSON/API/persistent-state schemas;
- pass formatting, lint, unit, contract, and relevant integration checks;
- update TODO status and user/operator documentation;
- be committed as focused changes and pushed immediately after each commit;
- maintain idempotent acquisition and never place durable media on the local
  filesystem; and
- meet the Mnemosyne design-language and WCAG 2.2 AA requirements.

## 0.1.0 — Governance and feasibility

Goal: establish a safe, implementable contract before code is written.

Exit criteria:

- public `sagrudd/x-img` repository exists with MPL-2.0 licensing;
- `README.md`, `AGENTS.md`, `MILESTONES.md`, and `TODO.md` agree on scope;
- current X, Meta/Instagram, Firefox, Monas, and DASObjectStore constraints are
  recorded using primary sources and sibling contract fixtures;
- content-rights, platform-policy, deletion/compliance, and private-account
  decisions are explicit;
- the durable object, local catalogue, account JSON, job, and review schemas are
  drafted with compatibility rules; and
- architecture ADRs define authority boundaries, metadata-only local state,
  canonical identity/idempotency, review admission, bounded refresh jobs,
  extension pairing, and external-cache fail-open behavior; and
- a Firefox spike plan identifies how redirects, range requests, HLS/DASH,
  CORS, CSP, signed URLs, and local HTTPS will be tested.
- Firefox policy explicitly forbids automatic opening, hidden traversal, bulk
  crawling, simulated browsing, and cookie/credential forwarding; it records
  observed-thumbnail versus explicitly-opened-original semantics and keeps
  capture/substitution per-site opt-in, transparent, same-instance, and
  fail-open.
- XIMG-007 evidence is recorded in `docs/adr/0010-firefox-architecture-spike.md`
  and `docs/fixtures/firefox-architecture-matrix.json` (commit `1b788bc`),
  with DNR/body-filtering limits, HTTPS and response-contract gates, and
  explicit segmented-video capability requirements.
- XIMG-008 release and quality policy is recorded in
  `docs/release-quality-policy.rst` (commit `34c7792`), with a Rust 1.95.0 MSRV
  for the initial 0.2.x line, current Firefox Release/ESR channel support,
  version-drift and planning/schema/privacy checks, a manually dispatched,
  non-blocking CI mirror, and the local Sphinx container retained as the
  documentation authority. Hosted CI will be backfilled or migrated later and
  does not block progress while funding is unavailable.
- XIMG-009 records the executable Pinakotheke v1 identity-migration plan in
  `docs/adr/0011-pinakotheke-v1-identity-migration.md` and its synthetic
  coverage matrix (commit `c37e5d2`). It preserves data-bearing identity,
  requires explicit aliases/migrations and proof fixtures, and blocks partial
  rename/rebrand releases.

## 0.2.0 — Rust core and versioned contracts

Goal: implement platform-neutral domain logic without live platform access.

Exit criteria:

- Rust workspace and `clap` CLI skeleton compile on supported targets;
- XIMG-020 provides the pinned Rust 1.97.0 workspace scaffold (commit
  `81e359c`): shared model/core, `clap` CLI, Axum composition boundary, and
  Yew WebAssembly boundary. It passes native and WebAssembly compilation,
  tests, clippy, CLI-version, planning, and local Sphinx-container checks;
  no live source, storage, authentication, or media-payload integration is
  present.
- versioned JSON config supports X accounts, Instagram accounts, and website
  policies with strict validation and safe atomic writes;
- XIMG-021 implements that contract (commit `d96922d`) with deny-unknown-field
  parsing, exact schema-version checks, strict account/origin/reference rules,
  duplicate rejection, synchronized same-directory atomic replacement, and a
  no-network CLI for validation and safe identifier-only listing.
- acquisition state machine covers discovered, claimed, transferring, stored,
  verified, committed, failed, and policy-blocked states;
- XIMG-022 implements the state-machine contract (commit `7fcf9c8`): bounded
  verified ObjectStore metadata is required before commit, review is permitted
  only after commit, and double claims, terminal re-entry, and out-of-order
  settlement transitions fail closed without live authority calls.
- canonical identity supports platform media IDs and content-hash fallback;
- XIMG-023 implements deterministic metadata-only idempotency and reconciliation
  (commits `3b5cb51`, `5343996`): canonical identity plus immutable checksum
  settles one record, replay merges safe aliases without object replacement,
  and all crash boundaries plus absent/conflicting authority evidence fail
  closed as pending/conflict.
- review state distinguishes new, reviewed, retained, hidden, and removed;
- XIMG-024 implements scheduler contracts (commit `07d4f22`) for coalesced
  global refreshes, explicit child scopes, mutually exclusive source leases,
  bounded capacity/cost usage, and cooperative cancellation without executing
  a connector, queue, or storage operation.
- fixture adapters prove pagination, duplicate discovery, crash reconciliation,
  and retry behavior without network access; and
- XIMG-025 provides the synthetic X/Instagram connector-fixture matrix (commit
  `942c75f`) for pagination, edits/deletions, duplicate/variant media,
  rate/authorization failures, malformed responses, and cursor reset, with a
  strict completeness test and no live platform interaction.
- explicit GEO/SRA/ENA/NCBI plan and resolution fixtures prove source authority,
  release, files, checksums, transport, rights/policy, destination, and no
  bulk discovery; and
- XIMG-026 implements the explicit metadata-only plan contract (commit
  `924ad12`) with bounded source input, reviewable file/checksum/transport and
  destination evidence, policy blocking, and confirmation before any future
  transfer or DASObjectStore write.
- XIMG-027 provides the synthetic resolution/transport matrix (commit
  `9732f34`) for ENA/SRA, GEO, NCBI, retry/resume/cancellation, backpressure,
  checksum mismatch, and optional Aspera fallback without live providers,
  credentials, or payloads.
- no code path can persist media bytes outside the storage port.

## 0.3.0 — DASObjectStore and Monas integration contracts

Goal: make external authorities real before adding collectors.

Exit criteria:

- x-img is declared as a Monas product mount with mandatory authentication and
  object-store requirement;
- XIMG-030 records the versioned Monas product bootstrap (commit `eada9e8`):
  one ``/products/x-img/app/`` and ``/products/x-img/api/`` mount, host-owned
  Prosopikon authentication, ``/opt/x-img`` product root, DASObjectStore
  requirement, declared capabilities, and a Synoptikon-equivalent bootstrap.
  Its synthetic fixtures reject anonymous access and direct x-img login-route
  declarations.
- Axum routes consume Monas-authenticated context and reject direct/anonymous
  privileged access;
- XIMG-031 implements that boundary (commit `4b000f1`): a direct privileged
  route returns `401`, while a host-injected authorized context is admitted.
  The strict Monas and Synoptikon adapters consume only non-secret identity,
  authorization, and correlation fields, reject missing `ximg.access`, and do
  not parse or retain cookies, passwords, session tokens, or credentials.
- a scoped DASObjectStore application identity obtains short-lived capability
  or S3 credentials without persisting secrets in x-img state;
- XIMG-032 records and validates the scoped application identity (commit
  `784a3cd`): one endpoint/ObjectStore/prefix, bounded read/write/list/verify
  operations and bytes, expiry, and opaque owner/authority references. Expired,
  replayed, wrong-store, wrong-prefix, and oversized operations fail closed;
  a future daemon adapter remains responsible for proof, token/capability
  issuance, quota, health, and final commit verification.
- object keys, metadata, checksums, media types, provenance, and range-readable
  responses have contract tests against sibling fixtures;
- XIMG-034 implements the authorized object-read/cache handoff (commit
  `2d36647`): media type, length, SHA-256/ETag, conditional, range, and
  unavailable outcomes are validated before an authority stream is exposed. It
  has no local byte cache; future HTTP/browser adapters own transport and
  fail-open origin substitution behavior.
- XIMG-033 implements the bounded streaming ingest port (commit `f2c6ef2`):
  chunks flow directly to an authority backend with no local payload staging;
  exact length and SHA-256, backend receipt identity, backpressure, and
  idempotent completion are fail-closed. A future DAS transport adapter owns
  credential exchange, durable authority commit, and cross-restart recovery.
- XIMG-035 adds cross-repository contract CI (commit `3e20812`): the required
  public-clone check validates x-img-owned fixture anchors and rejects sibling
  path dependencies; an explicit pinned-sibling inspection verifies the exact
  Monas, DASObjectStore, Synoptikon/Mnemosyne, and design-language revisions
  and contract paths. It does not claim live authority integration before
  credentialed transport adapters exist.
- XIMG-036 defines the endpoint/device and logical ObjectStore contract (commit
  `0594598`): stable IDs, managed local profile or paired remote appliance
  references, TLS/health/quota/type evidence, explicit default/override
  selection, and endpoint-qualified review metadata are strict and synthetic
  fixture tested. It rejects unmanaged folders, mutable identities, broad
  secrets, and arbitrary first-store selection without live credentials or
  transport.
- XIMG-037 implements reviewed endpoint/ObjectStore selection (commits
  `664c27c`, `40d9d23`): all validated authority-visible stores are exposed as
  endpoint-plus-store rows with word-first status; the exact reviewed stable-ID
  pair is revalidated immediately before commit and never falls back on
  removal, rename, unavailable/read-only, expiry, quota, TLS, reconnect, or
  cross-endpoint alias changes. Live transport and rendered task panes remain
  gated.
- XIMG-038 implements confirmed direct bioinformatics commit (commit
  `f6a07fa`): allowed explicit plans are revalidated against the exact reviewed
  endpoint/ObjectStore pair, then bounded chunks flow only through the existing
  DAS ingest port. SHA-256, length, authority receipt, accession/file/checksum
  replay, and metadata-only source/transport/rights/destination provenance are
  tested; live provider/DAS transport and durable crash recovery remain gated.
- upload completion is reconciled idempotently after crash boundaries; and
- a future Synoptikon host adapter can replace Monas without changing domain or
  connector logic.
- the resource ingest contract streams bounded, confirmed GEO/SRA/ENA/NCBI
  transfers to DASObjectStore, verifies checksums before catalogue admission,
  and records endpoint/ObjectStore and source provenance.
- endpoint/device and ObjectStore contracts distinguish managed local
  folder-profile provisioning from remote appliances, discover all visible
  stores, and require explicit endpoint-plus-store selection for writes.

## 0.4.0 — Account connectors and one-click refresh

Goal: acquire new media from configured social accounts into one review queue.

Exit criteria:

- official, user-authorized X connector supports permitted public and protected
  access, pagination, photos, videos, GIFs, and highest-quality allowed variant;
- XIMG-040 implements the official X OAuth Authorization Code + S256 PKCE host
  boundary (commit `7c10d9f`): exact redirect, state, expiry, replay, required
  read/follow/refresh scopes, opaque host exchange/refresh/revocation, and
  viewing-account binding are test-covered without raw credentials, cookies,
  tokens, live API traffic, or a scraping fallback. ADR 0002 still blocks live
  acquisition until X approval, rights, retention, and deletion gates close.
- XIMG-041 implements the followed-account review boundary (commit `1d693d9`):
  only stable account IDs returned for the grant-bound viewing account may be
  selected; the task-pane contract provides word-first added/existing/not-
  selected diff rows and requires confirmation before the existing atomic JSON
  save boundary. It provides no live X call, bulk import, or policy-gate bypass.
- XIMG-042 implements fixture-driven incremental X discovery (commit `ba2ed76`):
  cursor-chain, page/item/depth budgets; supported photo/video/animated-GIF
  rendition selection; canonical source provenance; and reconciliation-request
  idempotency are test-covered. It makes no X request or transfer while ADR
  0002 remains open.
- XIMG-044 implements fixture-only incremental Instagram discovery (commit
  `298d2c5`): posts, carousels, reels/videos, cursor/page/candidate budgets,
  opaque credential expiry/revocation, provenance, and reconciliation replay
  are test-covered. No Meta/Instagram request, browser fallback, transfer, or
  XIMG-043/ADR 0002 approval bypass is enabled.
- XIMG-045 implements one-click account-refresh orchestration (commit
  `63c2672`): all enabled X/Instagram accounts are coalesced into one global
  job with per-account bounded progress, partial failure, cancellation, retry,
  no overlap, and final new-item summary states; it executes no connector or
  media transfer.
- XIMG-046 implements verified-commit-only new-item admission (commit
  `3ac77e9`), retaining source grouping and discovery time while excluding
  interrupted work from review cards.
- official, user-authorized Instagram connector supports the account/media
  types permitted by the approved API and records unsupported cases clearly;
- `Refresh accounts` schedules all enabled connectors once, reports per-account
  progress, respects rate/cost budgets, supports cancellation, and prevents
  overlapping refreshes for the same account;
- incremental cursors are advisory while committed media identity is the final
  deduplication authority;
- new items are atomically marked for review only after DASObjectStore commit;
- removed, inaccessible, rate-limited, and policy-blocked sources remain
  explicit audit states; and
- tokens are encrypted/host-managed and never appear in logs or JSON account
  configuration.

## 0.5.0 — Monas-hosted media browser and review workflow

Goal: deliver the dense, fast visual browser.

- XIMG-050 implements the Mnemosyne-compatible Monas shell (commit `9e9cabb`):
  semantic-token CSS, compact header, responsive accessible empty state, and
  one mandatory footer provenance mark; host authentication remains Monas-owned.

Exit criteria:

- Yew UI is mounted in Monas and follows the central Mnemosyne tokens, shell,
  footer, keyboard, focus, and state patterns;
- account/source navigation, dense thumbnail grid, virtual scrolling, keyboard
  traversal, quick preview, metadata detail, and video playback are usable;
- new/unreviewed media is visibly filterable and can be marked reviewed in
  batches without using colour alone;
- gallery status distinguishes `Previously observed` thumbnails from `Stored
  in ObjectStore` committed originals using accessible, reversible,
  non-obstructive words/iconography and colour, with tooltips and a user toggle;
  stored bytes are never watermarked or mutated;
- the single `Refresh accounts` action exposes progress and safe retry;
- loading, empty, stale, permission, transport, and object-unavailable states
  are designed explicitly;
- thumbnails and originals are served from DASObjectStore through authorized,
  range-capable URLs; and
- endpoint/device inventory and ObjectStore selection use accessible
  Mnemosyne tables/task panes, show endpoint and store together, and handle
  writable/read-only, health, capacity, pairing, TLS, and reconnect states
  without silently changing a reviewed destination; and
- no full administration form is permanently embedded in the browsing view.

## 0.6.0 — Firefox site capture

Goal: allow users to add supported websites and capture viewed media into the
same x-img instance.

Exit criteria:

- signed Firefox Manifest V3-compatible extension has a trivial options UI for
  one x-img instance and add/remove/enable website policies;
- host permissions are requested per site at user action time, with clear scope;
- site adapters canonicalize supported image/video URLs and ignore pixels,
  avatars, ads, previews, and unsupported streams according to explicit policy;
- viewed supported media is offered once to x-img, stored through
  DASObjectStore, and appears in the common review queue;
- extension-to-x-img requests use a revocable, narrowly scoped pairing/session
  issued through Monas and contain no browser password or copied site cookie;
- private browsing is disabled by default and browsing history is minimized;
- errors are visible but never break the source page; and
- capture is per-site opt-in and transparent: thumbnails are cached only after
  actual display/observation, originals only after an explicit user open, and
  the extension never automatically opens, traverses hidden content,
  bulk-crawls, simulates browsing, or forwards cookies/credentials; and
- initial adapters are fixture-tested before any generic-site mode is enabled.
- video-focused adapters offer only observed or explicitly selected candidates
  in a review task pane with source details, tracks, policy/support, reviewed
  endpoint/ObjectStore, and normalization profile; no automatic opening,
  hidden traversal, playlist/channel bulk acquisition, DRM circumvention, or
  cookie/credential extraction is permitted.

## 0.7.0 — Firefox external-cache substitution

Goal: serve previously committed media from the object store on enabled sites.

Exit criteria:

- cache lookup meets a measured latency budget and fails open to the origin;
- canonical source aliases resolve to immutable committed object versions;
- image, MP4, byte-range, and approved segmented-video cases are proven in real
  Firefox tests;
- redirect/proxy delivery satisfies HTTPS, CORS, CSP, CORP, content type,
  content length, range, ETag, and conditional-request behavior;
- stale signed URLs do not prevent canonical hits and do not leak credentials;
- cache substitution is limited to user-enabled origins and can be paused from
  the toolbar;
- thumbnails are cached only after actual display/observation and originals
  only after an explicit user open; automatic opening, hidden traversal, bulk
  crawling, simulated browsing, and cookie/credential forwarding are forbidden;
- object unavailability falls back safely without redirect loops; and
- hit/miss/substitution diagnostics are inspectable without retaining general
  browsing history.
- normalized video is served only after a versioned Pinakotheke playback
  profile has passed transcode, checksum, probe, and real Firefox playback
  checks; source-only or failed video is visibly blocked/failed, never ready.

## 0.8.0 — Reliability, policy, and operations

Goal: make sustained personal operation predictable.

Exit criteria:

- bounded workers, backpressure, rate/cost budgets, retries, reconciliation,
  graceful shutdown, and job leases pass fault-injection tests;
- account refresh, extension capture, and cache serve share one scheduler and
  cannot create conflicting claims;
- schema migration, export, restore, and DASObjectStore loss/unavailability
  runbooks are tested;
- platform deletion/compliance behavior matches the approved policy decision;
- structured logs, metrics, health, and audit events contain no secrets;
- dependency, license, vulnerability, and extension-permission audits pass; and
- package/install documentation covers Monas, DASObjectStore, Firefox, and
  upgrades.
- video jobs enforce bounded streaming, cancellation, resumable transfer where
  possible, quotas, backpressure, pinned containerized FFmpeg, scratch cleanup,
  crash reconciliation, and profile-version idempotency.

## 0.9.0 — Release candidate

Goal: freeze contracts and validate end-to-end behavior with user-owned data.

Exit criteria:

- all supported connectors and site adapters pass end-to-end acceptance tests;
- performance targets are met for refresh, initial gallery load, scrolling,
  preview, range playback, and cache lookup;
- accessibility, security, privacy, and platform-policy reviews have no open
  release blockers;
- upgrade from the previous minor version preserves configuration, catalogue,
  object aliases, and review state; and
- public documentation clearly distinguishes supported behavior, known limits,
  and non-goals, including the evidence-backed Firefox playback profile choice,
  rights gates, and local Sphinx container verification.

## 1.0.0 — Stable personal archive

Goal: stable public interfaces and dependable day-to-day operation.

Exit criteria:

- no unresolved P0/P1 TODOs;
- the coordinated Pinakotheke rename/rebrand is complete across user-facing
  documentation, Rust/code identifiers, CLI/package/product metadata,
  Monas/Synoptikon/DASObjectStore adapters, Firefox extension identity, and
  the GitHub repository migrated from `sagrudd/x-img` to the chosen
  `sagrudd/pinakotheke` slug; compatibility aliases and migrations are
  documented and tested where needed;
- the Sphinx/Read the Docs user documentation builds and verifies successfully
  in the reproducible local container, with the local container check treated
  as authoritative over any GitHub Actions mirror;
- stable CLI, JSON schemas, API, object metadata, Monas mount, and extension
  pairing contracts are documented;
- release artifacts, Firefox package, checksums, SBOM, and upgrade notes are
  published; and
- rollback and recovery procedures have been exercised on a production-like
  Monas plus DASObjectStore deployment.

## Post-1.0 candidates

- Synoptikon catalogue/plugin integration through the preserved host adapter;
- additional explicitly approved site adapters;
- perceptual duplicate grouping without weakening exact source provenance;
- user-defined collections, tags, and saved searches; and
- derivative thumbnails or transcoding, stored as separate DASObjectStore
  objects with immutable parent provenance.
