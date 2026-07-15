# x-img TODO

Status: dependency-ordered planning backlog

Version: 0.2.0

Updated: 2026-07-14

## How to use this backlog

- Work from the first unchecked item whose dependencies are complete.
- One item is one reviewable change unless its acceptance criteria explicitly
  require a coupled slice.
- Before work, acquire the repository run lease described in `AGENTS.md`.
- Every admitted automation run must delegate bounded, non-overlapping research,
  implementation, test, or review quanta to subagents.
- Mark an item complete only after its code/docs, tests, commit, and push are all
  complete. Add the commit hash beside the item.
- Do not silently reinterpret a blocked requirement. Record the evidence and
  the decision needed, then move only to independent work.

Priority meanings: P0 blocks safe architecture or data integrity; P1 blocks a
milestone; P2 improves a usable milestone; P3 is post-1.0.

## 0.1.0 — Governance and feasibility

- [x] **XIMG-001 P0 — Create and protect the public repository.**
  Create `sagrudd/x-img`, set public visibility, MPL-2.0 license, default branch
  protections where available, description/topics, and verify a clean clone.
  Acceptance: repository URL is recorded in README; no secrets or local media
  are present; planning baseline is pushed. Completed in `78eee75`; the `main`
  branch requires linear history and rejects force-pushes and deletion.

- [x] **XIMG-002 P0 — Record platform-policy and content-rights decisions.**
  Review current primary X and Meta/Instagram developer terms, API terms,
  display/storage rules, deletion/compliance duties, automation restrictions,
  and personal copyright/privacy obligations. Define allowed and prohibited
  connector behavior for public and protected/private sources.
  Acceptance: dated ADR links every conclusion to a primary source; unresolved
  legal/policy questions block live acquisition but not fixture work.
  Completed in [`docs/adr/0002-platform-policy.md`](docs/adr/0002-platform-policy.md), commit `625c833`; X and Instagram remain fixture-only until the listed approval, account-class, rights, and retention gates are answered.

- [x] **XIMG-003 P0 — Inventory sibling contracts.**
  Pin the relevant Monas product mount/session contract, DASObjectStore
  application-auth/upload/read/range contracts, Mnemosyne design-language
  revision, and future Synoptikon product adapter contract.
  Acceptance: compatibility matrix names versions/commits and contract fixtures;
  no public build requires unpublished path dependencies. Completed in
  `b381921`; open Monas fixture and DAS HTTP-range contract risks remain
  explicitly recorded in `docs/compatibility-matrix.md`.

- [x] **XIMG-004 P0 — Write architecture decisions.**
  Create ADRs for authority boundaries, local metadata versus media bytes,
  idempotent acquisition, canonical source identity, review lifecycle, account
  refresh scheduling, extension pairing, and external-cache fail-open behavior.
  Acceptance: each ADR includes alternatives, failure modes, privacy impact,
  compatibility impact, and acceptance tests. Completed in `49af3eb`; the
  Sphinx entry point and pinned local container build the full ADR set with
  warnings treated as errors.

- [x] **XIMG-005 P0 — Define versioned configuration.** Completed in `564ff20`.
  Draft strict JSON schemas for one x-img instance, X accounts, Instagram
  accounts, and website policies. Keep secrets as host-managed references.
  Acceptance: examples cover enabled/disabled sources, per-source media policy,
  refresh budget, review defaults, and schema rejection of unknown fields.

- [x] **XIMG-006 P0 — Define acquisition and catalogue schemas.** Completed in
  `b90ff7f`; strict metadata schema, synthetic fixtures, reconciliation ADR,
  and Sphinx configuration guidance are in place.
  Specify source post/item, media identity, object reference, download attempt,
  job lease, account cursor, review state, tombstone, and audit event.
  Acceptance: state diagrams cover retry before/after DAS commit, duplicate URLs,
  URL rotation, platform ID reuse assumptions, and reconciliation.

- [x] **XIMG-007 P0 — Prove the Firefox architecture on paper and fixtures.**
  Define how the extension detects/captures responses and substitutes cached
  content for images, MP4, range requests, and HLS/DASH without credential
  capture or mixed-content failure.
  Acceptance: spike matrix covers WebRequest/DNR, response filtering, local
  HTTPS, CORS/CSP/CORP, redirects, signed URLs, and fail-open behavior; uncertain
  cases are isolated behind explicit site capabilities. It must also prove no
  automatic opening, hidden traversal, bulk crawling, simulated browsing,
  cookie/credential forwarding, or API-avoidance policy loophole; thumbnails
  are eligible only after actual display/observation and originals only after
  explicit user open. Completed in `1b788bc`; the architecture ADR and
  synthetic fixture matrix keep DNR body replacement, response filtering,
  progressive range delivery, and HLS/DASH behind explicit capability and
  fail-open evidence gates.

- [x] **XIMG-008 P1 — Establish release and quality policy.** Completed in
  `34c7792`; the Sphinx release policy, dependency-free repository checks, and
  manually-dispatched, non-blocking GitHub Actions mirror define SemVer/version
  authority, Rust 1.95.0 MSRV and Firefox Release/ESR support,
  verification/dependency/fixture rules, and the local-container-authoritative
  Definition of Done. Hosted CI will be backfilled or migrated later and never
  blocks progress while funding is unavailable.
  Add changelog, SemVer rules, supported Rust/MSRV and Firefox versions, CI
  matrix, dependency policy, fixture privacy rules, and Definition of Done.
  Acceptance: version sources cannot drift; the Definition of Done requires
  precise Sphinx/Read the Docs user documentation and a reproducible local
  `docs/Dockerfile` container build/verification that is authoritative over
  any GitHub Actions mirror; local checks cover planning links/schemas, with
  hosted CI advisory only until it is backfilled or migrated.

- [x] **XIMG-009 P0 — Plan the Pinakotheke v1 identity migration.** Completed
  in `c37e5d2`; ADR 0011, the Sphinx migration guide, a synthetic identity
  matrix, and an executable coverage check define the coordinated v1 cutover,
  compatibility window, rollback, and required config/catalogue/ObjectStore/
  extension-pairing proof cases.
  Keep `x-img` as the planning/repository name until a coordinated v1.0.0
  migration to Pinakotheke and target GitHub slug `sagrudd/pinakotheke`.
  Inventory documentation, Rust/code identifiers, CLI/package/product
  metadata, Monas/Synoptikon/DASObjectStore adapters, Firefox extension
  identity, repository settings, compatibility aliases, and data/schema
  migrations.
  Acceptance: the 1.0.0 gate has an executable rename matrix, rollback plan,
  alias/deprecation windows, and tests proving existing config, catalogue,
  object references, and extension pairings remain readable or migrate
  explicitly; no partial rename can ship.

## 0.2.0 — Rust core and contracts

- [x] **XIMG-020 P1 — Scaffold the Rust workspace.** Depends on XIMG-003/004/008.
  Completed in `81e359c`; shared model, core, `clap` CLI, Axum composition, and
  Yew crates inherit workspace lint/version/license metadata and carry MPL
  notices. Native checks, WebAssembly client check, tests, clippy, the CLI
  version (`0.2.0`), planning checks, and local Sphinx container verification
  passed; no live source, storage, authentication, or media-payload code exists.

- [x] **XIMG-021 P1 — Implement strict account/site config.** Depends on XIMG-005.
  Completed in `d96922d`; the offline `x-img config validate`, `list`, and
  `replace` commands strictly parse and validate versioned JSON, list only
  source identifiers/origins, and atomically replace a validated complete
  document. Tests cover duplicate handles/origins, invalid handles, unknown
  fields, wildcard origins, missing opaque authorization references, and
  incompatible schema versions; no source, authority, or storage call occurs.

- [x] **XIMG-022 P0 — Implement acquisition state machine.** Depends on XIMG-006.
  Completed in `7fcf9c8`; the platform-neutral in-memory lifecycle permits only
  discovered → claimed → transferring → stored → verified → committed, requires
  immutable ObjectStore evidence before commit, and admits review only after
  commit. Unit tests prove the valid lifecycle and reject double claims,
  out-of-order commits, review-before-commit, terminal re-entry, and malformed
  evidence; it performs no transfer, persistence, or authority call.

- [x] **XIMG-023 P0 — Implement idempotency and reconciliation.**
  Completed in `3b5cb51` and `5343996`; an in-memory, metadata-only reconciliation catalogue
  keys settlement by canonical media identity plus immutable checksum, appends
  safe URL aliases on replay, preserves the first object reference, and records
  checksum disagreement as conflict. Crash/replay tests cover absent authority
  evidence across the discovered/claimed/transferring/stored/verified/commit
  crash boundaries, post-upload retry, repeated settlement, and identity reuse
  without byte, network, persistence, or overwrite behavior.

- [x] **XIMG-024 P1 — Implement job scheduler contracts.**
  Completed in `07d4f22`; the in-memory scheduler coalesces repeated refreshes
  per actor scope, creates explicit child source scopes, gates claims by
  per-source lease and bounded child capacity, enforces request/byte/time
  budgets, and releases leases on cancellation. Tests prove concurrent refresh
  coalescing and no per-source overlap; it executes no connector or storage job.

- [x] **XIMG-025 P1 — Add deterministic connector fixtures.**
  Completed in `942c75f`; the strict synthetic X/Instagram fixture matrix and
  Rust validation contract cover pagination, edits, deleted/inaccessible items,
  duplicate media, variants, rate limits, authorization expiry, malformed
  responses, and cursor reset. Fixtures remain opaque, deterministic, and free
  of real traffic, credentials, account data, or media payloads.

- [x] **XIMG-026 P0 — Define explicit bioinformatics transfer plans.** Depends
  on XIMG-006/008. Completed in `924ad12`; the metadata-only GEO/SRA/ENA/NCBI
  plan accepts one bounded explicit accession/URL, captures reviewable release,
  files/checksums/bytes/transport, rights/policy, and stable destination, and
  requires an allowed-plan confirmation. Bulk input, unsafe destination, and
  blocked-policy confirmation fail closed; no discovery or transfer occurs.

- [x] **XIMG-027 P0 — Add bioinformatics resolution and transport fixtures.**
  Completed in `9732f34`; the strict synthetic matrix covers ENA/SRA manifests
  and multi-run expansion, GEO raw archive review, NCBI routing, checksum/size
  evidence, retry/resume/cancellation, backpressure, checksum mismatch, and
  optional Aspera-to-HTTPS fallback without credentials or payloads.

## 0.3.0 — External authorities

- [x] **XIMG-030 P0 — Define and register the Monas product.** Completed in
  `eada9e8`; the strict public bootstrap registration pins one Monas Web/API
  mount, product root, host-owned Prosopikon authentication, DASObjectStore
  requirement, capability list, and Synoptikon-equivalent bootstrap. Synthetic
  fixtures reject anonymous access and a direct x-img login-route declaration;
  live Axum host-context enforcement remains XIMG-031.

- [x] **XIMG-031 P0 — Implement authenticated host-context adapter.**
  Completed in `4b000f1`; the API accepts only a host-injected, non-secret
  authorized context and direct privileged access returns `401`. Monas and
  Synoptikon adapters use the same strict synthetic contract, reject missing
  `ximg.access`, and retain no cookie, session, password, token, or credential;
  the host remains responsible for validating its session before injection.

- [x] **XIMG-032 P0 — Register scoped DASObjectStore application identity.**
  Completed in `784a3cd`; the public registration binds one endpoint,
  ObjectStore, prefix, narrow read/write/list/verify scope, byte limits, expiry,
  and opaque authority references. Its authorization gate rejects expired,
  replayed, wrong-store, wrong-prefix, and oversized operations without storing
  a credential or issuing a token; live token exchange/ingest remains XIMG-033.

- [x] **XIMG-033 P0 — Implement streaming object ingest port.** Completed in
  `f2c6ef2`; the bounded authority-backend port validates chunk limits, exact
  length, SHA-256, and authority receipt before completion, returns the original
  receipt for an exact replay, and surfaces backpressure without buffering.
  It owns no payload files or durable local media; daemon transport and durable
  crash reconciliation remain future adapter work.

- [x] **XIMG-034 P0 — Implement authorized object read/cache port.** Completed
  in `2d36647`; the authority stream handoff validates media type, content and
  total length, SHA-256, quoted ETag, conditional not-modified, exact byte
  ranges, and explicit unavailable outcomes. It persists no local media cache;
  a future HTTP/browser adapter remains responsible for transport and fail-open
  origin behavior.

- [x] **XIMG-035 P1 — Add cross-repository contract CI.** Completed in
  `3e20812`; `scripts/contracts/check.sh` verifies x-img-owned versioned
  fixtures and forbids unpublished sibling path dependencies in a clean public
  clone. With `--require-siblings`, it verifies the four exact compatibility
  revisions and required contract paths; the manually dispatched public-source
  workflow performs the same inspection. This is deliberately source-contract
  evidence, not a credentialed live-authority test before transport adapters
  exist.

- [x] **XIMG-036 P0 — Define endpoint/device and ObjectStore contracts.**
  Completed in `0594598`; the strict metadata-only inventory separates stable
  endpoint/device and ObjectStore IDs, managed local profile provisioning,
  paired HTTPS appliance references, health, quota, TLS, compatible types, and
  endpoint-qualified reviewed selection. Synthetic fixtures reject unmanaged
  folder fields, mutable IDs, broad secrets, and arbitrary first-store
  selection. It holds no credential and performs no live pairing, discovery,
  write, or transport; XIMG-037 implements reviewed selection/revalidation.

- [x] **XIMG-037 P0 — Implement reviewed endpoint/ObjectStore selection.**
  Completed in `664c27c` (with its direct serde dependency recorded in
  `40d9d23`); the core exposes every validated endpoint/store as structured,
  word-first status rows and retains one explicit stable-ID reviewed pair.
  Commit-time revalidation rejects removed, renamed, unavailable, read-only,
  expired, over-quota, untrusted-TLS, reconnect, and multi-endpoint alias
  states without choosing a fallback. Live discovery/pairing transport and the
  rendered Yew task pane remain future adapters.

- [x] **XIMG-038 P0 — Implement confirmed direct bioinformatics commit.**
  Completed in `f6a07fa`; one allowed, explicitly confirmed plan file must
  retain its exact reviewed endpoint/ObjectStore pair through commit-time
  revalidation before bounded direct ingest. SHA-256, exact length, and the
  authority receipt are verified; accession/file/checksum replays return the
  first metadata provenance record and changed destinations fail. No local
  payload or fallback exists. Durable crash reconciliation and live provider/
  DAS transport remain future adapters.

## 0.4.0 — Social account acquisition

- [x] **XIMG-040 P0 — Implement official X OAuth adapter.** Completed in
  `7c10d9f`; the official Authorization Code + S256 PKCE boundary requests only
  `tweet.read`, `users.read`, `follows.read`, and `offline.access`, enforces
  state/redirect/expiry/replay, and delegates exchange/refresh/revocation to
  opaque Monas-held references. Grant scopes and protected access are bound to
  the viewing X account. It contains no raw token/cookie/secret or live X API
  traffic; ADR 0002's approval, rights, retention, and deletion gates still
  block live acquisition.

- [x] **XIMG-041 P1 — Import/select followed X accounts.** Completed in
  `1d693d9`; the grant-bound, fixture-driven preview accepts only returned
  stable X account IDs and produces explicit `Added`, `Already configured`, and
  `Not selected` rows before a confirmation can yield a candidate local JSON
  configuration. It does not write itself (the existing atomic
  `ConfigStore::replace` boundary persists a confirmed candidate), bulk-enable
  accounts, call X, or relax ADR 0002's live approval/rights gates.

- [x] **XIMG-042 P0 — Implement incremental X media discovery.** Completed in
  `ba2ed76`; synthetic page planning supports photos, videos, animated GIFs,
  cursor pagination, explicit depth/page/item limits, and best supported
  JPEG/PNG/WebP or MP4 variants. It records canonical X identity, source alias,
  account/item/media IDs, checksum, discovery time, adapter/policy result and
  converts a candidate to the XIMG-023 reconciliation request; duplicate page
  entries settle once in the fixture test. It has no live X request, byte
  transfer, or ADR 0002 policy-gate bypass.

- [ ] **XIMG-043 P0 — Research and implement approved Instagram auth/API.**
  Freeze implementation to the capabilities permitted by XIMG-002. Acceptance:
  unsupported account/media classes are explicit and no browser credential or
  scraping fallback is introduced implicitly.

- [x] **XIMG-044 P0 — Implement incremental Instagram media discovery.**
  Completed in `298d2c5`; synthetic page planning covers single-media posts,
  multi-media carousels, reels/videos, cursor pagination, page/candidate
  budgets, supported image/MP4 variant choice, provenance, and XIMG-023
  reconciliation replay. Opaque fixture credential expiry/revocation fails
  closed to reauthorization without accepting a token. No Instagram/Meta API
  request, browser fallback, byte transfer, or XIMG-043/ADR 0002 gate bypass is
  present.

- [x] **XIMG-045 P0 — Implement one-click account refresh.** Completed in
  `63c2672`; one authenticated actor schedules every enabled X/Instagram
  account into a coalesced global job with bounded per-account progress,
  partial failure, cancellation, retry, no-overlap, and final new-item summary
  states. It is metadata-only orchestration: no connector, credential, media,
  or review-admission work is executed.

- [x] **XIMG-046 P1 — Implement new-item review admission.** Completed in
  `3ac77e9`; only verified committed evidence enters the `New` queue with source
  grouping and discovery time; interrupted work is rejected.

## 0.5.0 — Web library and review

- [x] **XIMG-050 P1 — Implement Mnemosyne-compliant Monas shell.** Completed
  in `9e9cabb`; the Yew shell uses canonical semantic-token classes, compact
  product/navigation/Monas-host header, accessible responsive empty state, and
  one required provenance footer mark. The host supplies approved token and
  brand assets; x-img adds no login flow.

- [x] **XIMG-051 P1 — Implement source/account navigation.** Completed in
  `ffdd275`; keyboard-operable source navigation selects All, X, Instagram, or
  Websites with visible selected context and configured-source counts.

- [x] **XIMG-052 P1 — Implement dense virtualized thumbnail browser.** Completed in `312d290`; synthetic dense adaptive grid has stable keyboard selection and lazy-load boundary.
  Include keyboard traversal, adaptive thumbnail size, stable selection,
  thumbnail lazy loading, and fast filters. Acceptance: performance targets are
  documented and met against a large synthetic catalogue.

- [ ] **XIMG-053 P1 — Implement quick preview and video playback.**
  Acceptance: original/fit view, metadata, alt text, source link, range playback,
  focus trapping/return, and unavailable-object state pass.

- [x] **XIMG-054 P1 — Implement review queue and batch review.** Completed in
  `84e11ca`; word-first New/Reviewed/Hidden/Removed queue states, reversible
  batch review/hide actions, and a user-toggleable accessible observed-thumbnail
  versus committed-ObjectStore-original overlay are implemented. Stored bytes
  are never altered; persistence remains the review-admission API follow-up.

- [x] **XIMG-055 P1 — Implement refresh controls and progress.** Completed in
  `a48fc13`; one Refresh accounts control renders word-first per-account
  progress, partial failure, safe retry, and final new-item summary states over
  the XIMG-045 orchestration contract. Connector execution remains host-side.

- [x] **XIMG-056 P2 — Add browse/search metadata filters.** Completed in
  `fd07323`; the browser has keyboard-operable client-side metadata text search
  over its synthetic catalogue. Server-side account/platform/site/media/time/
  dimension/review/object filters remain the catalogue API follow-up.

## 0.6.0 — Firefox capture

- [x] **XIMG-060 P0 — Scaffold least-privilege Firefox extension.** Completed in
  `c7f317c`; Manifest V3 uses optional per-origin HTTPS permissions and a
  minimal toolbar/options surface, with no private browsing default, cookies,
  credential forwarding, automatic opening, hidden traversal, crawling, or
  simulated browsing. The extension identity remains migration-ready.
  Use supported manifest behavior, per-origin optional permissions, no private
  browsing by default, and a minimal toolbar/options surface. Capture and
  substitution are per-site opt-in, transparent, routed through the same x-img
  instance, and fail open.
  Acceptance: the extension identity is migration-ready for Pinakotheke and
  contains no automatic opening, hidden traversal, bulk crawling, simulated
  browsing, cookie extraction, or credential forwarding.

- [ ] **XIMG-061 P0 — Implement Monas-mediated extension pairing.**
  Pair one extension profile with one x-img instance using revocable,
  least-privilege credentials. Acceptance: origin binding, expiry, rotation,
  revocation, CSRF, replay, and local-network threat tests pass.

- [ ] **XIMG-062 P1 — Implement trivial website policy UI.**
  Add current site, enable/disable capture and substitution, choose supported
  media classes, and remove permission. Acceptance: exact requested origin and
  consequences are visible before Firefox permission request; capture and
  substitution are visibly per-site opt-in and can be paused independently.

- [x] **XIMG-063 P1 — Implement site-adapter registry.** Completed in
  `d5feaba`; versioned explicit adapters canonicalize origins, exclude unsafe
  paths, declare capabilities, and use fixtures. Generic observed-image mode is
  experimental and requires explicit opt-in; registry matching enables nothing.

- [ ] **XIMG-064 P0 — Implement viewed-media capture.**
  Stream or re-fetch only through an approved design from XIMG-007, submit to the
  common scheduler, and fail without page disruption. Acceptance: a thumbnail
  is eligible only when actually displayed/observed, an original only after
  explicit user open, no automatic opening/hidden traversal/bulk crawling/
  simulated browsing occurs, and no cookies, authorization headers, form
  bodies, credentials, or general history reach x-img; avoiding a site API does
  not waive platform terms.

- [ ] **XIMG-065 P1 — Integrate captures into the common review queue.**
  Acceptance: site, page, canonical media URL, discovery time, and adapter
  version are retained; committed aliases deduplicate account-connector media.

- [ ] **XIMG-066 P0 — Define user-selected video candidate plans.** Depends on
  XIMG-037/060/062/063/064. Offer a DownloadThemAll-like task pane only for
  observed or explicitly selected candidates; show title/source, duration,
  dimensions, container/codecs, size, audio/subtitles, policy/support,
  endpoint/ObjectStore, and intended profile. Acceptance rejects automatic
  opening, hidden traversal, playlist/channel bulk discovery, DRM bypass, and
  cookie/credential extraction, and requires explicit confirmation.
  Planning evidence: `docs/adr/0005-video-selection-normalization.md` in commit
  `5ad8eee`.

- [ ] **XIMG-067 P0 — Define versioned normalized video objects and profiles.**
  Depends on XIMG-033/034/066. Specify `pinakotheke-video-webm-v1` and
  `pinakotheke-video-mp4-v1` candidate contracts, profile evidence for VP9/Opus,
  AV1/Opus, and H.264/AAC, typed derived objects, readiness states, retention,
  and provenance. Acceptance never marks source-only video ready and documents
  browser, hardware, licensing, quality, encoding, and storage evidence.
  Planning evidence: `docs/adr/0005-video-selection-normalization.md` in commit
  `5ad8eee`.

- [ ] **XIMG-068 P0 — Implement the containerized video normalization adapter.**
  Depends on XIMG-024/033/067. Use a pinned FFmpeg image/tool with structured
  arguments, bounded resources, DAS-managed staging or isolated ephemeral
  scratch, cleanup, probe/checksum manifests, cancellation, retry/resume,
  progress, and crash/idempotency handling. Acceptance has no shell
  interpolation, durable x-img-local payload, secret, or copyrighted fixture.
  Planning evidence: `docs/adr/0005-video-selection-normalization.md` in commit
  `5ad8eee`.

- [ ] **XIMG-069 P0 — Prove normalized Firefox playback and delivery.** Depends
  on XIMG-067/068/072. Acceptance requires DAS commit, checksum, probe, real
  Firefox playback, MIME/ETag/Content-Length, conditional requests, byte
  ranges, seek/pause/resume, authorization, and fail-open tests; blocked or
  failed/DRM media remains explicit and never falls back as ready source-only
  playback.
  Planning evidence: `docs/adr/0005-video-selection-normalization.md` in commit
  `5ad8eee`.

## 0.7.0 — External cache

- [ ] **XIMG-070 P0 — Implement low-latency cache alias lookup.**
  Acceptance: bounded memory, invalidation, immutable hit identity, measured
  p95 budget, offline/object-unavailable state, and no browsing-history leak;
  only previously displayed/observed thumbnails may be cached automatically,
  and every substitution is per-site opt-in, transparent, same-instance, and
  fail-open to the origin.

- [ ] **XIMG-071 P0 — Implement image substitution.**
  Restrict to enabled sites and proven aliases; fail open without loops.
  Acceptance: HTTPS/CSP/CORS/CORP/type/length/ETag behavior passes real Firefox.

- [ ] **XIMG-072 P0 — Implement MP4 and range substitution.**
  Acceptance: seek, pause/resume, concurrent ranges, cancellation, conditional
  requests, and fallback pass real Firefox tests.

- [ ] **XIMG-073 P0 — Gate segmented video substitution by adapter.**
  Implement HLS/DASH only where manifest and segment canonicalization are proven.
  Unsupported streams remain origin-served and visibly diagnosed. Normalized
  Pinakotheke renditions only are eligible for ready/playable substitution.

- [ ] **XIMG-074 P1 — Add toolbar cache controls and diagnostics.**
  Per-site pause, hit/miss state, last error, open x-img source view, and clear
  wording about permissions without exposing secrets. Show the words
  `Previously observed` and `Stored in ObjectStore` where applicable, with
  accessible iconography/tooltips and a reversible non-obstructive status
  treatment; never watermark or mutate stored media bytes.

## 0.8.0–1.0.0 — Hardening and release

- [ ] **XIMG-080 P0 — Add fault-injection and recovery suite.**
- [ ] **XIMG-081 P0 — Add migration/export/restore tests.**
- [ ] **XIMG-082 P0 — Implement approved deletion/compliance reconciliation.**
- [ ] **XIMG-083 P1 — Add redacted telemetry, health, and audit surfaces.**
- [ ] **XIMG-084 P1 — Complete privacy, security, accessibility, and license audits.**
- [ ] **XIMG-085 P1 — Package Monas product and Firefox extension.**
- [ ] **XIMG-086 P1 — Run production-like upgrade/rollback acceptance.**
- [ ] **XIMG-087 P1 — Publish 0.9.0 release candidate.**
- [ ] **XIMG-088 P0 — Close all release blockers and publish 1.0.0.**
  Acceptance includes the coordinated Pinakotheke rename/rebrand matrix and
  repository migration, compatibility aliases/migrations, and local-container
  Sphinx documentation verification described above.

## Post-1.0

- [ ] **XIMG-200 P3 — Add Synoptikon host/catalogue integration.**
- [ ] **XIMG-201 P3 — Add approved site adapters through the registry.**
- [ ] **XIMG-202 P3 — Add perceptual duplicate grouping.**
- [ ] **XIMG-203 P3 — Add collections, tags, and saved searches.**
- [ ] **XIMG-204 P3 — Add provenance-linked derivatives/transcodes.**
