# x-img TODO

Status: dependency-ordered planning backlog

Version: 1.1.0

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

- [x] **XIMG-043 P0 — Retire the dedicated Instagram API connector path.**
  Completed in this roadmap update; Instagram is no longer a required social
  API adapter. Its supported path is an explicitly enabled Firefox site policy
  with observed-media-only capture, no cookie/credential forwarding, no
  automatic traversal, and policy/rights disclosure. Official API research may
  resume later as an optional adapter, never as a fallback requirement.

- [x] **XIMG-044 P0 — Archive fixture-only Instagram media discovery.**
  Completed in `298d2c5`; synthetic page planning covers single-media posts,
  multi-media carousels, reels/videos, cursor pagination, page/candidate
  budgets, supported image/MP4 variant choice, provenance, and XIMG-023
  reconciliation replay. Opaque fixture credential expiry/revocation fails
  closed to reauthorization without accepting a token. No Instagram/Meta API
  request, browser fallback, byte transfer, or policy-gate bypass is present.

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

- [x] **XIMG-053 P1 — Implement quick preview and video playback.**
  Acceptance: original/fit view, metadata, alt text, source link, range playback,
  focus trapping/return, and unavailable-object state pass.
  Completed in `d5652a0`: selected cards open a keyboard-controlled Mnemosyne
  task pane with object/source/type/alt-text evidence, fit/original visual
  control, source metadata link, explicit unavailable state, and no origin
  fallback. Only a ready normalized ObjectStore video renders the existing
  host-authenticated range URL. Synthetic visual proxies contain no media
  payload; see `docs/quick-preview.rst`.

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

- [x] **XIMG-061 P0 — Implement Monas-mediated extension pairing.** Completed
  in `c6f4692`; the host pairing contract binds one profile/origin and
  fixture-tests expiry, rotation, revocation, CSRF, replay, and local-network
  rejection without extension-side credentials.

- [x] **XIMG-062 P1 — Implement trivial website policy UI.** Completed in
  `bb14d50`; options disclose the exact HTTPS origin and consequences before
  permission, select images/videos, independently pause capture/substitution,
  and remove the origin permission. All behavior is visibly per-site opt-in.

- [x] **XIMG-063 P1 — Implement site-adapter registry.** Completed in
  `d5feaba`; versioned explicit adapters canonicalize origins, exclude unsafe
  paths, declare capabilities, and use fixtures. Generic observed-image mode is
  experimental and requires explicit opt-in; registry matching enables nothing.

- [x] **XIMG-064 P0 — Implement viewed-media capture.** Completed in
  `f5ce32d`; local Dockerfile build and container verification passed on
  2026-07-15 after restarting Docker Desktop.
  Stream or re-fetch only through an approved design from XIMG-007, submit to the
  common scheduler, and fail without page disruption. Acceptance: a thumbnail
  is eligible only when actually displayed/observed, an original only after
  explicit user open, no automatic opening/hidden traversal/bulk crawling/
  simulated browsing occurs, and no cookies, authorization headers, form
  bodies, credentials, or general history reach x-img; avoiding a site API does
  not waive platform terms.

  The host-authenticated metadata-only capture-plan endpoint validates the
  paired actor, exact enabled origin, adapter, current-page provenance,
  visible-thumbnail eligibility, redaction, and scheduler admission; the
  Firefox toolbar submits only viewport-displayed images and fails open. Native,
  wasm, quality, contract, and local Docker Sphinx checks pass. A plan is not an
  ObjectStore commit or review item.

- [x] **XIMG-065 P1 — Integrate captures into the common review queue.**
  Completed in `6752ec8`; verified committed website captures retain their
  site/page/media/adapter provenance and enter the shared `New` queue only
  after ObjectStore verification. A matching connector alias reuses its
  canonical media identity, avoiding a duplicate review record.
  Acceptance: site, page, canonical media URL, discovery time, and adapter
  version are retained; committed aliases deduplicate account-connector media.

- [x] **XIMG-066 P0 — Define user-selected video candidate plans.** Completed
  in `1b1835a`; metadata-only candidate plans show the required review details,
  bind the reviewed endpoint/ObjectStore and intended Firefox profile, require
  explicit confirmation, and block unobserved, rights-disallowed, DRM,
  unsupported segmented, and non-video-destination candidates. Aggregate codec
  gaps are deterministic and privacy-preserving; project-level prioritisation
  is tracked in [GitHub issue #1](https://github.com/sagrudd/x-img/issues/1)
  without user URLs, titles, credentials, cookies, or media. Native, wasm,
  quality, contract, and local Docker Sphinx verification passed.

- [x] **XIMG-067 P0 — Define versioned normalized video objects and profiles.**
  Completed in `50c9a00`; immutable WebM VP9/Opus-or-AV1/Opus and MP4 H.264/AAC
  profiles require Firefox, hardware/software, encoder, quality, storage, and
  licensing evidence. Typed checksummed ObjectStore derivatives, source
  retention, provenance, and readiness reject source-only videos. Docker-first
  plans require a pinned image digest, resource bounds, authorized
  DASObjectStore/paired-device/future-Keryx placement, and managed or bounded
  scratch. Native, wasm, quality, contract, and local Docker Sphinx verification
  passed. See `docs/normalized-video-profiles.rst`.

- [x] **XIMG-068 P0 — Implement the containerized video normalization adapter.**
  Completed in `c1346e8` and `6af0ae2`; the first adapter runs only on a paired
  device with a digest-pinned Docker/FFmpeg image, structured network-isolated
  arguments, capability/read-only/resource restrictions, bounded ephemeral
  scratch, direct bounded DASObjectStore ingest, poster/probe/checksum/provenance
  artifacts, cleanup, cancellation, idempotency/crash reconciliation, and host
  phase progress. It never marks a rendition Ready: Firefox proof remains
  XIMG-069. No shell interpolation, durable product-local payload, secret, or
  copyrighted fixture is used. Native, wasm, quality, contract, and local Docker
  Sphinx verification passed. See `docs/video-normalization.rst`.

- [x] **XIMG-069 P0 — Prove normalized Firefox playback and delivery.** Depends
  on XIMG-034/067/068. Direct authenticated playback is independent of later
  Firefox cache substitution (XIMG-072). Acceptance requires DAS commit, checksum, probe, real
  Firefox playback, MIME/ETag/Content-Length, conditional requests, byte
  ranges, seek/pause/resume, authorization, and fail-open tests; blocked or
  failed/DRM media remains explicit and never falls back as ready source-only
  playback.
  Completed in `f49572e`: an actor-bound Axum ``/api/playback/v1/{playback_id}``
  host adapter streams only the scoped DASObjectStore response and preserves
  MIME, ETag, conditional, single-range, and explicit error behavior. The
  route tests reject unauthenticated/multi-range requests and prove a verified
  partial stream. Firefox 152.0.6 played a Docker-generated ephemeral
  normalized MP4 through the checked-in local harness, with metadata, range,
  seek, pause, and resume evidence. No test video, browser profile, source URL,
  or credential is retained. See `docs/direct-playback.rst`.

## 0.7.0 — External cache

- [x] **XIMG-070 P0 — Implement low-latency cache alias lookup.**
  Acceptance: bounded memory, invalidation, immutable hit identity, measured
  p95 budget, offline/object-unavailable state, and no browsing-history leak;
  only previously displayed/observed thumbnails may be cached automatically,
  and every substitution is per-site opt-in, transparent, same-instance, and
  fail-open to the origin.
  Completed in `70cc536`: a capacity-bounded server index rejects signed query
  keys and immutable identity conflicts, separates observed thumbnails from
  explicitly opened originals, supports exact/origin invalidation and
  checksum-bound offline state, and requires host actor/pairing/instance/site/
  adapter/substitution authorization. The Axum endpoint returns bounded hit or
  origin-fallback metadata without echoing aliases; 10,000 synthetic lookups
  over 4,096 entries measured 5.5 microseconds p95 against a 2 ms budget. See
  `docs/cache-alias-lookup.rst`.

- [x] **XIMG-071 P0 — Implement image substitution.**
  Restrict to enabled sites and proven aliases; fail open without loops.
  Acceptance: HTTPS/CSP/CORS/CORP/type/length/ETag behavior passes real Firefox.
  Completed in `9db5944`: opaque stable delivery IDs preserve the exact
  reviewed endpoint/ObjectStore/object identity, and the authenticated Axum
  route revalidates actor, pairing, instance, site, adapter, expiry, policy,
  representation, and availability before streaming through the existing
  DASObjectStore read port. The toolbar path considers only visible images on
  an explicitly enabled site, strips query/fragment aliases, validates the
  bounded response MIME/length/checksum ETag, uses an ephemeral blob URL, and
  restores the original ``src``/``srcset`` once on every failure without a
  loop. API tests prove production HTTPS/CORS/CORP/no-store header semantics;
  installed Firefox 152 passes cross-origin display plus CSP, CORS, MIME,
  length, and ETag fail-open using runtime-only bytes. Rust, wasm, clippy,
  contracts, quality, privacy, and pinned local Docker/Sphinx checks pass. See
  `docs/image-substitution.rst`.

- [x] **XIMG-072 P0 — Implement MP4 and range substitution.**
  Acceptance: seek, pause/resume, concurrent ranges, cancellation, conditional
  requests, and fallback pass real Firefox tests.
  Completed in `13ee87a`: alias lookup returns a video-specific opaque route
  only for explicitly opened normalized MP4 records. The Axum route repeats
  host/pairing/site/adapter/object authorization, streams the exact reviewed
  DASObjectStore object, supports full, conditional, and single-range reads,
  returns exact ``206``/``304``/``416`` metadata, and leaves response bodies
  independently streamable. The opted-in extension replaces only visible,
  metadata-loaded videos through Firefox's native media element, never buffers
  video bytes, and restores the original source/time/play state once on error.
  Firefox 152 passed range, concurrent-range, cancellation, conditional,
  seek, pause/resume, and fallback tests with an ephemeral Docker-generated
  H.264/AAC MP4. Rust, clippy, wasm, JavaScript, contracts, quality/privacy,
  and pinned local Docker/Sphinx checks passed. See
  `docs/mp4-substitution.rst`.

- [x] **XIMG-073 P0 — Gate segmented video substitution by adapter.**
  Implement HLS/DASH only where manifest and segment canonicalization are proven.
  Unsupported streams remain origin-served and visibly diagnosed. Normalized
  Pinakotheke renditions only are eligible for ready/playable substitution.
  Completed in `3b90b5d`: the server-side v1 gate requires exact origin,
  adapter/version, HLS/DASH kind, separately versioned manifest and segment
  canonicalization, synthetic fixture and real-Firefox evidence, explicit
  display/open, no DRM/encryption, and a matching Ready normalized profile.
  Missing or mismatched evidence is worded ``Origin served`` and never selects
  another adapter, endpoint, store, or source. The generic Firefox adapter
  conservatively recognizes manifest URLs and blob/MSE sources, performs no
  lookup/rewrite/traversal, leaves playback untouched, and shows one bounded
  URL-free diagnostic in settings. No segmented adapter is falsely advertised
  as approved. Rust, clippy, wasm, JavaScript, strict JSON/privacy, contracts,
  and pinned Docker/Sphinx checks pass. See
  `docs/segmented-video-gate.rst`.

- [x] **XIMG-074 P1 — Add toolbar cache controls and diagnostics.**
  Per-site pause, hit/miss state, last error, open x-img source view, and clear
  wording about permissions without exposing secrets. Show the words
  `Previously observed` and `Stored in ObjectStore` where applicable, with
  accessible iconography/tooltips and a reversible non-obstructive status
  treatment; never watermark or mutate stored media bytes.
  Completed in `e3dc371`: the action popup shows the active configured origin,
  separate capture/substitution state, explicit visible-media run,
  pause/resume, Settings, and paired x-img Websites-context navigation. One
  replaceable diagnostic per configured origin records only a worded
  hit/miss/error reason and observed/stored booleans; removed origins are
  pruned and page/media URLs, aliases, queries, checksums, cookies, credentials,
  and payloads are excluded. ``◉ Previously observed`` and
  ``✓ Stored in ObjectStore`` use words, icons, and an explanatory tooltip;
  media bytes are never modified. Failures say ``Origin served`` and never
  disrupt the page. Rust, clippy, wasm, JavaScript, least-privilege/privacy,
  contracts, and pinned Docker/Sphinx checks pass. See
  `docs/firefox-toolbar.rst`.

## 0.8.0–1.0.0 — Hardening and release

- [x] **XIMG-080 P0 — Add fault-injection and recovery suite.**
  Delivered in ``401adf6``. The strict synthetic matrix and one local runner
  exercise bounded ingest, authority crash replay, destination revalidation,
  scheduler cancellation, video-normalizer crash/cleanup, cache/capture
  authority loss, and real-Firefox substitution failure. Evidence: workspace
  fmt/check/test/clippy, wasm check, quality and public-contract checks, all
  nine fault cases, prohibited-name scan, and the pinned Sphinx container build
  plus run passed locally on 2026-07-15; hosted CI was not required.
- [x] **XIMG-081 P0 — Add migration/export/restore tests.**
  Delivered in ``a0fd895``. A strict metadata-only snapshot exports canonical
  JSON plus an independent SHA-256, restores only a validated review candidate,
  and performs idempotent copy-on-write legacy migration with an exact backup.
  Native tests preserve configuration, historic labels, canonical media and
  endpoint/ObjectStore/object/checksum identities; require reviewed Firefox
  re-pairing; and reject corruption, unknown fields, unsafe metadata, and
  future majors before mutation. Workspace fmt/check/test/clippy, wasm,
  quality, public-contract, nine-case fault/Firefox, privacy, and pinned local
  Sphinx container build/run checks passed on 2026-07-15. Hosted CI was not
  required.
- [x] **XIMG-082 P0 — Implement approved deletion/compliance reconciliation.**
  Delivered in ``37ae847``. The metadata-only lifecycle requires a bounded host
  policy approval, tombstones catalogue visibility before any deletion, keeps
  catalogue-only and catalogue-plus-object scopes distinct, and binds removal
  to the exact endpoint/ObjectStore/object/checksum. Pending or still-present
  authority evidence remains retryable; only a matching DASObjectStore result
  becomes ``Removal verified`` and mismatches become ``Conflict``. Four native
  tests prove approval, ordering, scope, crash/replay idempotency, exact-object
  verification, and redacted bounded audit events. Workspace fmt/check/test/
  clippy, wasm, quality, public-contract, nine-case fault/Firefox, privacy, and
  pinned local Sphinx container build/run passed on 2026-07-15; hosted CI was
  not required.
- [x] **XIMG-083 P1 — Add redacted telemetry, health, and audit surfaces.**
  Delivered in ``d522e1c``. Public ``/health`` exposes only versioned liveness
  and build identity; the host-authenticated operations snapshot exposes typed
  word-first health for five components, saturating aggregate counters, and a
  128-entry fixed-code audit ring with eviction count. A host-retained shared
  recorder supports live updates, while the schema has no free-form field for
  URLs, browsing/account/actor data, credentials, sessions, object keys,
  checksums, or payloads. Core and Axum tests prove worst-state aggregation,
  bounds, redaction, coarse public output, and authentication. Workspace fmt/
  check/test/clippy, wasm, quality, public-contract, nine-case fault/Firefox,
  privacy, and pinned local Sphinx container build/run passed on 2026-07-15;
  hosted CI was not required.
- [x] **XIMG-084 P1 — Complete privacy, security, accessibility, and license audits.**
  Delivered in ``e23d748``. ``scripts/audit/check.sh`` enforces a strict
  six-category matrix across tracked/untracked candidates: credential/payload
  privacy, unsafe/dynamic-code and extension CSP/permission security, semantic
  HTML/Yew focus accessibility, MPL/SPDX coverage, cargo-deny locked dependency
  advisories/licenses/bans/sources, JavaScript syntax, and version mirrors.
  Findings fixed the missed Firefox manifest version path, explicit extension
  CSP, semantic popup/options documents and live regions, visible Yew keyboard
  focus, and missing script SPDX notices. Two non-exploitable unmaintained
  transitive-Yew advisories have narrow documented ``deny.toml`` exceptions;
  duplicate dependency generations remain visible warnings. Audit, workspace
  fmt/check/test/clippy, wasm, quality, public-contract, nine-case fault/Firefox,
  toolbar, privacy, and pinned local Sphinx container build/run passed on
  2026-07-15; hosted CI was not required.
- [x] **XIMG-085 P1 — Package Monas product and Firefox extension.**
  Completed in ``073c7c7``, ``fb769f3``, and ``a571daa``: the Makefile and
  packaging sources define
  Linux x86_64/arm64 DEB+RPM, macOS x86_64/arm64 PKG, and six deterministic
  host/architecture-labelled Firefox XPIs. All twelve unsigned artifacts built
  locally and passed ``make checksums verify``. Linux uses native-container GNU
  cross-linkers, explicit target libc development packages, and target-aware
  RPM metadata, avoiding emulated ``rustc`` and host/target strip mismatches.
  A deterministic typed release manifest records platform, architecture,
  length, checksum, and explicit unsigned state for every artifact. The native
  package boundary remains the CLI plus the versioned host-composable Monas
  bootstrap; it does not misrepresent x-img as a competing standalone auth
  daemon. Signing/notarization belongs to XIMG-087, and production-like Monas
  install/upgrade/rollback acceptance belongs to XIMG-086.
- [x] **XIMG-086 P1 — Run production-like upgrade/rollback acceptance.**
  Completed in ``18e0c12``, ``97ed392``, and ``1535d60``. The accumulated
  backward-compatible work was released as 0.3.0 so the verified 0.2.0
  artifacts became a genuine prior baseline. ``make upgrade-rollback`` proved
  0.2.0 → 0.3.0 → 0.2.0 for real DEB and RPM packages on x86_64 and arm64 in
  network-isolated digest-pinned Debian/Fedora containers. CLI and Monas
  bootstrap versions changed at every boundary while the synthetic metadata
  SHA-256 and endpoint/ObjectStore/object/checksum/review identities remained
  exact. Strict export/restore/repeat-migration and current pinned Monas plus
  DASObjectStore contract checks passed. Signing/notarization remains XIMG-087.
- [x] **XIMG-087 P1 — Publish 0.9.0 release candidate.** Completed in
  ``267d392``, ``c4b0344``, and ``4916360`` and published as the explicit
  GitHub prerelease ``v0.9.0``. Fifteen uploaded files comprise thirteen
  verified artifacts plus ``SHA256SUMS`` and the typed release manifest:
  dual-architecture DEB/RPM/PKG, six platform-labelled Firefox XPIs, and a
  deterministic CycloneDX 1.6 SBOM. Genuine 0.3.0 → 0.9.0 → 0.3.0 DEB/RPM
  rollback passed on x86_64 and arm64, as did workspace, wasm, audit, fault,
  contract, packaging, and pinned local Sphinx checks. Release notes state that
  artifacts are unsigned evaluation builds, hosted CI was not used, live
  connector/authority boundaries remain limited, and Pinakotheke is the 1.0
  coordinated migration. Published at
  https://github.com/sagrudd/x-img/releases/tag/v0.9.0.
- [x] **XIMG-088 P0 — Close all release blockers and publish 1.0.0.**
  Completed in ``79dbc72`` and ``99108b0`` and published as ``v1.0.0`` at
  https://github.com/sagrudd/pinakotheke/releases/tag/v1.0.0. The coordinated
  identity gate passed across the canonical repository, Rust workspace, Monas,
  DASObjectStore, Firefox, packaging, and documentation surfaces while legacy
  schemas and the ``x-img`` CLI alias remained compatible. Thirteen canonical
  artifacts plus checksums and a typed release manifest were locally built,
  verified, and published; the complete local quality, audit, fault, contract,
  package-transition, Firefox, and containerized Sphinx evidence passed without
  relying on hosted CI.
  Acceptance included the coordinated Pinakotheke rename/rebrand matrix and
  repository migration, compatibility aliases/migrations, and local-container
  Sphinx documentation verification described above.
  The first delivery slice adds ``make v1-preflight`` and the deliberately
  strict ``make v1-cutover`` gate: the former proves the exact surface inventory
  while reporting current blockers, and the latter refuses release until every
  canonical identity and the public GitHub repository are ready together.
  The coordinated identity implementation, compatibility proof, repository
  rename, package/document verification, tag, and release are complete.
  The second delivery slice prepares a canonical ``pinakotheke`` executable and
  a warning-emitting ``x-img`` alias over one clap implementation, with golden
  parser-equivalence tests; 0.9 packages intentionally continue installing only
  ``x-img`` until the coordinated cutover.
  The third delivery slice adds inert, strictly validated Pinakotheke Monas and
  DASObjectStore registration candidates. They preserve authentication, scope,
  destination, quota, and legacy audit/object identities but cannot mount a
  product, activate a principal, or issue a credential before cutover.
  The fourth delivery slice fixes update-time storage reset and adds a
  Pinakotheke Firefox manifest candidate retaining the shipped Gecko ID,
  permissions, CSP, and entry points. Executable synthetic upgrade proof
  preserves pairing, site opt-ins, endpoint, and ObjectStore selection exactly.
  The fifth delivery slice parameterizes every package family, Firefox XPI,
  SBOM, checksum, and artifact manifest behind an explicit product switch.
  ``x-img`` remains the 0.9 default; canonical mode is version-locked, installs
  ``pinakotheke`` plus the ``x-img`` CLI alias, and consumes reviewed candidates.
  The sixth delivery slice adds an executable all-local-surface cutover
  transformer and an isolated rehearsal. The rehearsal renames and compiles the
  Rust workspace, activates copies of reviewed authority/Firefox candidates,
  updates canonical version/repository/documentation identity, passes the
  strict cutover and packaging gates, then deletes the temporary tree. The live
  0.9 repository remains unchanged until the coordinated authority and GitHub
  change window.
  The seventh delivery slice builds real canonical DEB/RPM candidates for
  x86_64 and arm64 and exercises 0.9 → 1.0 → 0.9 transitions in pinned,
  network-isolated Debian/Fedora containers. Both CLI identities and the
  canonical Monas registration are verified, RPM/DEB successor metadata is
  explicit, and the synthetic authority/catalogue snapshot remains byte-exact.
  The eighth delivery slice closes the post-rename tooling gap: the isolated
  cutover now passes the complete quality, dependency/license/security audit,
  nine-case fault-recovery, public-contract, and package-source suites. Release
  helpers use canonical crate paths and active authority/Firefox documents, and
  Pinakotheke becomes the default package identity only inside the coordinated
  cutover transaction.
  The ninth delivery slice performs the coordinated live identity migration:
  workspace crates and version, active Monas and DASObjectStore registrations,
  Firefox identity, packaging defaults, public documentation, and the GitHub
  repository become canonical together. Historic ``x-img`` schemas and the
  warning-emitting CLI alias remain compatible. Publishing verified 1.0.0
  artifacts, tag, and release notes remains before this item can close.

## Post-1.0

- [x] **XIMG-090 P0 — Scaffold the runnable Pinakotheke monolith.** Completed
  in ``2cfa1e1``. ``pinakotheke serve`` now resolves ``$HOME/.x-img`` by
  default, requires an absolute non-symlink root, creates only private
  mode-``0700`` ``config``, ``state``, ``run``, and ``logs`` metadata
  directories, and starts a graceful Axum HTTP/1 listener on
  ``127.0.0.1:8731``. Non-loopback binding fails unless the operator supplies
  the explicit unauthenticated-network acknowledgement, which also emits a
  warning. Public landing, liveness, and bounded readiness routes report
  Pinakotheke ``Ready`` but Monas authentication and DASObjectStore ``Not
  configured``; authenticated/media routes are not falsely mounted. A real
  local smoke run verified HTTP responses, permissions, and Control-C shutdown.
  Workspace tests, strict clippy, wasm, quality/audit, and local Docker Sphinx
  build/run passed. XIMG-091 is the next dependency-ready monolith slice.
  Original acceptance: add
  ``pinakotheke serve`` with a loopback-only default, validated ``~/.x-img``
  root layout, Axum listener, coarse health/readiness, graceful shutdown, and
  tests proving non-loopback binds require an explicit reviewed override. The
  first slice may expose only public health plus unavailable component status;
  it must not fake Monas authentication or DASObjectStore readiness.
- [x] **XIMG-091 P0 — Provision the managed local DASObjectStore profile.** Completed
  in ``30b18d1`` (core implementation ``7b5423a``). Add
  an explicit bounded macOS development profile rooted at
  ``~/.x-img/dasobjectstore`` with private credentials under
  ``~/.config/dasobjectstore``. Provision/discover a named logical ObjectStore
  through DASObjectStore authority, retain stable endpoint/store IDs, and never
  write media by treating the managed root as an ordinary directory. Core
  implementation is pushed in ``7b5423a`` against DASObjectStore ``0.84.0``
  commits ``42463234`` and ``0d71b2a1``: reviewed plan/provision/status/down
  commands delegate to the canonical helper, validate its versioned
  secret-free identity, atomically retain the stable selection, and expose
  honest storage readiness. Unit, workspace, clippy, wasm, quality, audit, and
  pinned sibling checks pass. After an aggressive Docker Desktop restart, a
  clean isolated home proved authority-owned provision and rediscovery, stable
  endpoint ``local-docker-314985151`` plus ObjectStore ``pinakotheke_local``,
  mode-``0600`` selection state, storage ``Ready`` with Monas still honestly
  ``Not configured``, graceful monolith shutdown, and non-destructive profile
  shutdown. The pinned local Sphinx container build and run also passed. The
  final diagnostic fix exposes only one bounded authority-error line instead
  of an opaque exit status.
- [x] **XIMG-092 P0 — Compose Monas authentication into the monolith.** Completed
  in ``74f035e`` with Monas ``0.2.0`` commits ``e0999e3`` and ``6e62943``. Mount
  the Pinakotheke product through Monas/Prosopikon, inject authenticated host
  context into product APIs, and prove login/session/logout and direct-route
  rejection without adding Pinakotheke-owned credentials or cookies. The first
  backend ingress quantum is implemented in ``74f035e``: an optional private
  process-local dispatch credential admits only strict
  ``x-img.host-context.v1`` Monas context, strips both dispatch headers before
  product handling, rejects direct/forged/invalid requests, and reports the
  configured boundary honestly in readiness. Monas now registers the canonical
  product mount, verifies Prosopikon sessions, strips cookies and forged host
  headers, injects actor/authorization/correlation context, streams request and
  response bodies, and accepts only a loopback backend plus private token file.
  Synthetic real-store registration/login/session/logout tests prove admission
  and revocation through an Axum backend; Pinakotheke tests independently prove
  direct-backend and forged dispatch rejection. Neither public repository gains
  an unpublished sibling path dependency.
- [ ] **XIMG-093 P1 — Add macOS per-user service management.** Provide
  non-root ``launchd`` install/status/logs/restart/uninstall commands with
  absolute paths, private permissions, transactional updates, preserved state,
  graceful shutdown, and no destructive uninstall default.
- [ ] **XIMG-094 P0 — Prove clean-home monolith operation.** In an isolated
  temporary home, start the service, complete Monas login, select the managed
  local ObjectStore, commit and read one synthetic object through scoped DAS
  contracts, restart and reconcile exactly once, then shut down cleanly. Record
  bounded local evidence and do not rely on GitHub Actions.

- [x] **XIMG-200 P3 — Add Synoptikon host/catalogue integration.** Completed
  in ``d0005bb``. A public ``mnemosyne.product.manifest.v1`` registration now
  declares dual-host support, Synoptikon entitlement/account/audit ownership,
  and DASObjectStore artifact authority without an unpublished dependency.
  Verified Synoptikon contexts require tenant, account, project, and entitlement
  scope plus ``ximg.catalogue.read``. The host-composed Axum endpoint returns a
  deterministic project-isolated page of review metadata and immutable
  DASObjectStore references, capped at 200 items, with no payload, source URL,
  browser history, or credential fields. Native workspace tests, strict clippy,
  version/privacy/security audits, the pinned Mnemosyne contract check, and the
  local Docker Sphinx build/run passed. Post-1.0 development is now 1.1.0 and
  the quality runner preserves but no longer incorrectly executes the one-time
  1.0 cutover gate against later versions.
- [ ] **XIMG-201 P3 — Add approved site adapters through the registry.**
- [ ] **XIMG-202 P3 — Add perceptual duplicate grouping.**
- [ ] **XIMG-203 P3 — Add collections, tags, and saved searches.**
- [ ] **XIMG-204 P3 — Add provenance-linked derivatives/transcodes.**
