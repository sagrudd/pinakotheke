# x-img TODO

Status: dependency-ordered planning backlog

Version: 1.22.8

Updated: 2026-07-19

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

- [x] **XIMG-096 P0 — Prove the critical Firefox-capture-to-gallery vertical.**
  This is a normative stable-release correction and cannot be satisfied by
  synthetic cards, proxy artwork, packaging evidence, or published release
  metadata. Through the Monas-authenticated Pinakotheke instance, prove that an
  image thumbnail actually observed by Firefox and an original explicitly
  opened by the user are committed to the selected DASObjectStore, admitted to
  the common review catalogue, rendered as real ThumbsPlus-style cards, and
  opened from authorized ObjectStore delivery without origin fallback. Prove
  the equivalent user-selected video path through verified normalization,
  poster/card rendering, and Firefox play, seek, pause, and resume. Restart the
  local monolith and verify that cards, provenance, review state, endpoint/store
  identity, and object availability survive. Exercise unavailable-object and
  partial-failure states without broken media or silent origin access, plus a
  large mixed catalogue with virtualization, keyboard traversal, filters, and
  responsive desktop/mobile layouts. The evidence must use real Firefox and
  ephemeral redistributable media; stored bytes remain solely in
  DASObjectStore. Delivery slices: (1) a bounded host-authenticated gallery
  catalogue exposing only verified ObjectStore references and explicit
  availability; (2) replace synthetic Yew records and visual proxies with that
  API and authorized image/video routes; (3) persistent capture/review wiring
  and the restart/Firefox acceptance harness. Update the ThumbsPlus-style user
  documentation and record the local container-built Sphinx evidence.
  The production normalization seam is implemented in ``1a15c10``:
  ``pinakotheke video normalize`` consumes one private confirmed plan, runs the structured
  digest-pinned network-isolated Docker adapter, and streams the normalized
  video, poster, and manifest directly to a reviewed helper that owns
  DASObjectStore authentication. Exact authority receipts are mandatory;
  malformed or changed completion fails, unfinished children are killed, and
  pristine bounded scratch is removed on every outcome. A process-fixture test
  exercises all three byte streams and cleanup without retaining repository
  media. Commit ``b46edce`` adds the first-party packaged ``pinakotheke
  ingest-stream-v1`` helper, which validates the exact bounded header, payload,
  and checksum, invokes only the configured native or containerized DAS remote
  client, and emits a receipt only after verified daemon completion. An
  isolated live run against
  DASObjectStore ``093772da79bbb494da070965c7d4f49e5ad83f56`` committed and
  independently inspected a synthetic 33-byte JSON manifest with the expected
  content type. Commit ``c472973`` records the external-host slice, which
  exposed and fixed invalid Linux ``--mount`` syntax and capability-free
  private-scratch ownership. Against
  DASObjectStore ``28e6d82cc8c25dd83838fde8b6de3aa16384eb95`` on the x86_64
  DASServer, the fixed worker normalized a three-second synthetic source and
  committed verified MP4, WebP poster, and JSON manifest objects with exact
  types; independent FFprobe confirmed H.264/AAC, 320x240, and 3.041 seconds.
  DGX Spark separately proved the hardened three-output worker and cleanup on
  GB10 arm64 with a digest-pinned locally registered FFmpeg 8.1.2 image and a
  fixture completion authority. The remaining gate is persistent gallery
  admission plus real Firefox playback, seek, pause/resume, and restart
  acceptance. External assurance also found that the DAS local-profile helper
  creates a root-owned bind-mounted credential registry on Linux; the isolated
  run used a scoped temporary ownership repair, and the sibling helper needs a
  non-root reconciliation fix before this becomes routine Linux provisioning.
  Progress: slice 1 now defines a strict bounded Monas-hosted catalogue page
  whose image/video card and preview representations contain complete verified
  DASObjectStore references, explicit availability, and only host-local
  authorized delivery paths. Core and Axum tests reject origin URLs,
  inconsistent availability, unbounded pages, and unauthenticated access. The
  Yew/API replacement now removes the synthetic card array and preview proxies:
  the browser consumes the canonical Monas-forwarded catalogue path, renders
  ready ObjectStore thumbnails, original images, posters, and normalized video,
  and fails closed with accessible loading/empty/permission/transport/schema/
  unavailable states. Source counts and filtering use returned account/website
  classifications; Instagram remains a website. The authenticated monolith now composes the
  canonical gallery route and returns an honest empty catalogue until a host
  supplies the persistent projection. Persistent gallery metadata now loads on
  monolith startup from a private, strict, bounded, atomically replaced v1 JSON
  document. Missing state is empty; corrupt/future/oversized/symlinked state
  fails closed, and restart tests preserve complete ObjectStore identity,
  review state, and availability without payload bytes.
  Verified Firefox image admission now bridges committed acquisition evidence,
  website provenance, the common ``New`` review queue, and atomic persistent
  gallery replacement. Observed thumbnails create cards; explicitly opened
  originals enrich an existing card only after their own verified commit and
  with the same endpoint/ObjectStore. Original-first, uncommitted, conflicting,
  and destination-changing attempts fail closed; server-generated delivery
  paths and restart tests preserve both immutable objects without payload
  bytes.
  Persisted thumbnail/original paths now resolve to the exact catalogue-bound
  endpoint, ObjectStore, object key, checksum, MIME type, and length and stream
  through the existing authorized read port under the private Monas dispatch.
  Tests prove direct access denial, missing/original-unavailable handling,
  exact backend selection, metadata validation, and payload streaming without
  origin fallback or local persistence. DASObjectStore commit
  ``bdafc51154989db075f241d041d9eab699f4a022`` still exposes no stable public
  application HTTP-read wire, so live CLI backend composition, browser failure
  reconciliation, real Firefox evidence, video persistence/delivery,
  and virtualization remain. Local Sphinx 8.2.3 HTML/dummy builds and the
  pinned Dockerfile build/container run passed with warnings denied on
  2026-07-16 after Docker Desktop recovered.
  Ready normalized-video records with matching versioned Firefox evidence now
  persist as one ``New`` card with separate typed/checksummed poster and
  rendition references. Planned/source-only/unproven/conflicting records fail
  closed. The poster streams through authenticated image delivery and the
  rendition through authenticated single-range delivery with MIME, length,
  checksum, ETag, conditional, and range validation; Axum tests prove poster,
  partial MP4, and multi-range rejection after restart-safe admission. Live CLI
  DASObjectStore transport, real Firefox restart/play/seek/pause/resume,
  browser failure reconciliation, and large-catalogue virtualization remain.
  Persistent catalogue queries now apply bounded source/media/review/
  availability/time/text filters before stable pagination and report matched
  plus unfiltered totals. Yew sends source and text filters to that
  authenticated boundary, restarts at the newest match when they change, and
  incrementally loads the next 100 records without the former silent 200-card
  truncation. The dense grid now renders only a responsive eight-row window
  plus bounded overscan, preserves virtual height, recalculates on resize and
  density changes, and uses a roving tab stop with Arrow/Home/End navigation
  across off-screen loaded cards. A 10,000-record unit fixture bounds the DOM
  slice. The gallery now has a real WebAssembly start entry and reproducible
  ``make web`` Trunk build. A bounded, symlink-free web root is served from the
  canonical app path only behind private Monas dispatch; direct backend access
  is denied. Native tests prove admitted index delivery. Installed Firefox now
  exercises the actual release Yew/WASM bundle against 1,000 mixed synthetic
  records: five pages, a bounded <=120-card DOM, scrolling beyond record 400,
  End-key focus on record 499, server filtering, unavailable preview with no
  non-loopback request, and desktop/390px reflow all pass in private temporary
  profiles. This is browser-component evidence. Native DEB/RPM/PKG assembly now
  builds and installs the real hashed Yew/WASM bundle, and embeds its matching
  platform path in the monolith. Explicit ``--web-root`` and private
  ``ROOT/web`` development overrides remain bounded, symlink-free, and higher
  priority. The packaged monolith now accepts one absolute reviewed
  ``pinakotheke.object-read-helper.v1`` executable and composes the app,
  catalogue, image/poster routes, and normalized-video range route behind the
  same Monas dispatch. Requests contain immutable object identity only;
  responses use bounded metadata plus a four-chunk streaming queue, with exact
  length, process status, and full-response SHA-256 verification and no local
  payload file. DASObjectStore/host authentication remains helper-owned. The
  packaged first-party helper now pins endpoint and ObjectStore-to-bucket
  mappings, uses host-owned AWS credentials, verifies DAS completion checksum
  metadata, supports one range, and deletes private bounded scratch. Live Monas
  session evidence, capture/commit/restart reconciliation, and normalized
  gallery playback remain.
  The helper prerequisite now maps DASObjectStore identity exactly: verified
  acquisitions and managed video objects carry a positive immutable object
  version; catalogue persistence, image/video grants, and helper requests
  preserve it. Legacy catalogue-v1 records default non-destructively to version
  1, while zero and missing new evidence fail closed. A helper can therefore
  construct the authority's ``BackendObjectKey`` without guessing.
  Managed macOS composition now requires the reviewed helper path and endpoint
  identity together, validates the stable identity, and pins it in the backend
  agent environment as non-secret authority scope. This prevents a helper from
  silently following a request-selected endpoint. The live authority proof
  still remains.
  Verification on 2026-07-16: focused CLI tests, the repository quality gate,
  and local Sphinx 8.2.3 warnings-as-errors dummy build passed. The required
  container build could not start because Docker Desktop's local socket did not
  answer its health probe; rerun the documented build when that host service is
  available.
  The runnable foreground and per-user-service monolith now load a bounded,
  strict, private metadata-only pairing/site authority document and mount the
  real capture-plan endpoint only behind Monas dispatch. Native tests prove
  direct denial, admitted actor-bound planning, unsafe configuration rejection,
  and operation with or without object delivery. Persisted plan execution,
  verified DASObjectStore commit, and crash reconciliation remain.
  The packaged extension now targets the canonical Monas product API and app
  mounts, closing the former origin-only pairing plus legacy-path mismatch.
  Accepted capture plans are now committed to a strict private atomic journal
  before success, restored after restart, deduplicated across retries, and
  listed only to their authenticated actor. Daily per-page budgets also survive
  restart. The remaining transition is execution of this explicit pending work
  through verified DASObjectStore ingest and persistent gallery admission.
  Verification on 2026-07-16: 151 focused Rust tests, repository quality and
  privacy/version checks, and local Sphinx 8.2.3 warnings-as-errors passed.
  Docker Desktop again failed to answer its local socket, so the documented
  container replication remains a host-service follow-up rather than a code
  acceptance blocker.
  A private host-worker completion endpoint now accepts strict independently
  verified ObjectStore image evidence, replays acquisition/reconciliation
  gates, atomically admits the common ``New`` gallery card, updates the live
  catalogue, and retains an idempotent settled marker. Monas context alone is
  insufficient: a separate process token is mandatory. The remaining gap is a
  production worker/read-helper that performs the authority transfer and
  verification before calling this boundary, plus real Firefox evidence.
  Verification on 2026-07-16: 153 focused Rust tests, warnings-denied Clippy,
  repository quality/privacy/version checks, and local Sphinx 8.2.3 with
  warnings denied passed. Docker Desktop still did not answer its local socket,
  so container replication remains an explicit host-service follow-up.
  The first production-worker seam now exists as ``pinakotheke capture
  acquire``: one reviewed executable receives only an approved plan plus fixed
  destination, owns permitted retrieval and scoped DASObjectStore streaming,
  and returns a strict verified metadata receipt with empty stdout. A synthetic
  executable proves the protocol and destination enforcement. Continuous
  scheduling and a concrete DASObjectStore-packaged helper remain, but pending
  plans can now be driven end-to-end through the public adapter boundary.
  Verification on 2026-07-16: 155 focused Rust tests, warnings-denied Clippy,
  repository quality/privacy/version checks, and local warnings-denied Sphinx
  8.2.3 passed. Docker Desktop still did not answer its local socket, leaving
  container replication as the recorded host-service follow-up.
  The same reviewed helper can now run continuously inside foreground or
  launchd monolith operation. Firefox admission returns promptly; helper work
  is serialized, repeated actor/plan requests coalesce, verified receipts use
  the existing completion gate, and failures remain pending without a stored
  claim. A concrete DASObjectStore-packaged helper and real Firefox/live
  authority acceptance remain.
  Verification on 2026-07-16: 157 focused Rust tests, warnings-denied Clippy,
  repository quality/privacy/version checks, and local warnings-denied Sphinx
  8.2.3 passed. Docker Desktop again did not answer its local socket, so the
  container replication remains the recorded host-service follow-up.
  Monolith startup now discovers durable unsettled plans and requeues eligible
  work without a Firefox retry. Current actor pairing, expiry/revocation,
  exact-origin policy, adapter version, and capture-kind permission are
  revalidated first; withdrawn authority remains pending and performs no
  helper/network/ObjectStore work. Concrete DASObjectStore helper packaging and
  real Firefox/live-authority acceptance remain.
  Verification on 2026-07-16: 158 focused Rust tests, warnings-denied Clippy,
  repository quality/privacy/version checks, and local warnings-denied Sphinx
  8.2.3 passed. Docker Desktop again did not answer its local socket; container
  replication remains the recorded host-service follow-up.
  Firefox now installs an idempotent trusted-click observer for enabled image
  sites and submits ``explicit_original`` only for an image link or image
  document; synthetic and unlinked-thumbnail clicks are ineligible. The
  experimental generic adapter now applies to arbitrary HTTPS origins only
  after explicit site opt-in and optional-origin permission. Node/extension
  contract evidence proves canonical Monas routing, sender-tab provenance,
  signed-query removal, disabled-policy rejection, and redacted diagnostics.
  The focused Node syntax/event contracts, toolbar contract, repository quality
  checks, release audit, and warnings-denied local Sphinx 8.2.3 build pass.
  The required Docker documentation build was attempted for 20 seconds but the
  local Docker Desktop daemon did not respond; this host replication gap does
  not invalidate the successful local documentation or feature checks.
  Explicit-original observation is now a persistent Firefox MV3 dynamic
  content script registered only for exact opted-in image origins. It is
  restored on browser startup and extension update, excludes adapter-blocked
  paths and subframes, and is unregistered before capture pause/removal revokes
  permission. The background still revalidates live policy, adapter, and
  sender-tab provenance for every trusted click. This removes the repeated
  per-page toolbar prerequisite without adding tabs, history, cookies, or
  ``webRequest`` authority. Real installed-Firefox capture through a concrete
  DASObjectStore helper remains the critical acceptance gap.
  Verification on 2026-07-16: JavaScript syntax, observer/toolbar/identity
  contracts, deterministic XPI packaging, repository quality/privacy/version
  checks, release audits, x-img-owned public contracts, and warnings-denied
  Sphinx 8.2.3 passed. The aggregate sibling check reported only that the local
  Monas checkout is newer than the intentionally pinned compatibility revision;
  this Firefox-only change consumes no Monas contract. Docker Desktop again did
  not answer within the bounded 20-second container-build attempt.
  The packaged Pinakotheke binary now implements the concrete
  ``acquire-image-v1`` worker protocol. A strict private secret-free config pins
  the endpoint, reviewed executables, DAS remote-client configuration, daemon
  socket, and byte cap. The helper permits HTTPS-only bounded retrieval into
  mode-``0700`` ephemeral scratch, validates image type/length, streams SHA-256,
  derives idempotent object identity/version, and invokes
  ``dasobjectstore-remote --submit-to-daemon`` for the selected store. It emits
  a receipt only after the DAS daemon reports verified completion and deletes
  scratch on every outcome; child output is bounded and diagnostics suppressed.
  This was inspected against DASObjectStore commit
  ``5769f27859a58101aedd9de0087fc278fd3e4b16``. Live paired-authority and real
  Firefox acceptance remain before XIMG-096 can close.
  Firefox capture plans now carry an optional canonical presentation URL. A
  displayed linked thumbnail uses its link target; the trusted opened-original
  event reuses that target. The server derives and persists catalogue identity
  from site/page/presentation, ignores helper-selected catalogue IDs, keeps
  unrelated page images separate, and migrates legacy journals by falling back
  to canonical media identity. This closes the distinct thumbnail/original URL
  correlation gap without guessing.
  Installed Firefox 152 now accepts the dual background manifest and an
  isolated WebDriver BiDi harness proves an actually observed linked thumbnail
  plus trusted explicitly opened original through the production capture path.
  Non-default-port sites use Firefox's required port-independent host match
  pattern while exact-origin policy and provenance remain port-bound. This is
  browser-path evidence; the live paired DASObjectStore commit/restart vertical
  remains the final XIMG-096 authority gap.
  Verification on 2026-07-16: all 168 workspace tests, strict workspace Clippy,
  Firefox observation/explicit-open/toolbar contracts, installed Firefox 152
  observed/opened capture, repository quality and release security/privacy
  audits, and warnings-denied Sphinx 8.2.3 passed. Docker Desktop again did not
  answer within the bounded 30-second container build attempt. Capture against
  the paired live DASObjectStore authority remains before XIMG-096 can close.
  Verification on 2026-07-16: all 166 workspace tests, strict CLI Clippy,
  repository quality/privacy/version checks, release security/license audits,
  strict JSON contracts, and warnings-denied Sphinx 8.2.3 passed. Docker
  Desktop again did not answer within the bounded 20-second documentation
  container-build attempt; the local Sphinx authority passed independently.
  Completed by the assembled acceptance gate in ``f7dadb1``. Real installed
  Firefox now covers automatic observed-thumbnail capture, trusted opened
  original and progressive-video capture, verified stored framing, the actual
  Yew/WASM gallery with 1,000 mixed records, bounded virtualization, keyboard
  traversal, filters, responsive layouts, unavailable/no-origin states, and
  normalized H.264/AAC metadata load, play, seek, pause/resume, conditional and
  concurrent ranges, cancellation, and missing-object recovery. Native tests
  cover verified completion, immediate ``New`` admission, complete immutable
  endpoint/ObjectStore/object/provenance/review/poster/video metadata, exact
  authorized delivery, and restart convergence. The clean-home XIMG-094 run
  in ``9e1688c`` supplies daemon-verified DASObjectStore commit, checksum-equal
  authorized read, restart reconciliation, and session recovery; subsequent
  recorded DASServer runs supply automatic X image admission, account-key
  provenance, and normalized MP4/WebP/manifest settlement. The authoritative
  evidence matrix and failure boundaries are in ``docs/critical-vertical.rst``;
  its pinned Sphinx HTML and dummy container builds pass with warnings denied.

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

- [x] **XIMG-097 P1 — Make DASObjectStore an external native-package dependency.**
  DEB and RPM metadata must require the separately published ``dasobjectstore``
  package, while the macOS PKG must refuse installation with an actionable
  prerequisite message unless DASObjectStore is independently installed.
  Pinakotheke packages must not bundle DASObjectStore executables, services,
  configuration, credentials, or object data. Package-source checks and Sphinx
  user documentation must enforce and explain this boundary. Completed in
  ``c32241a``; real x86_64 DEB/RPM metadata and an arm64 macOS PKG prerequisite
  were inspected, with no DASObjectStore payload found.

- [x] **XIMG-098 P0 — Complete the live storage-destination workflow.** The
  first deployed slice renders the real Yew application through Monas and
  discovers the signed-in actor's DASObjectStore inventory in a visible,
  keyboard-accessible selector. Ready stores are selectable and unhealthy,
  read-only, unauthorized, and unavailable states remain explicit. Complete
  this item by persisting the reviewed endpoint/ObjectStore stable IDs in the
  Pinakotheke authority, restoring them after restart, and proving that capture
  planning and pre-commit revalidation consume that exact selection without a
  fallback. Monas forwarding fix: pushed sibling commit ``51d90d2``.
  Pinakotheke 1.13.0 adds the first closure slice: a strict actor-scoped,
  mode-0600, atomically replaced destination store with restart persistence,
  optimistic revision conflicts, migration seeding from the existing reviewed
  capture authority, and no cross-actor/default fallback. A protected GET/PUT
  API and accessible Yew task pane load, distinguish saved/unsaved/error
  states, and explicitly save only a Ready writable row whose endpoint matches
  authenticated onboarding. The server accepts only the exact endpoint/store
  pair already authorized by the host; browser inventory is not authority.
  Completed in ``fb2e467``. Every new capture plan now persists the actor's
  exact endpoint, ObjectStore, and selection revision; legacy unbound plans do
  not enter worker recovery. A strict bounded host callback revalidates the
  same IDs and revision plus presence, TLS trust, pairing expiry, readiness,
  write/type capability, and quota immediately before acquisition and again
  before external completion. Changed or unavailable authority fails closed,
  receipts must match the plan binding, and no path chooses a fallback store.

- [x] **XIMG-099 P0 — Establish trusted local HTTPS for Firefox.** Completed
  in ``61d91d7`` on 2026-07-17. The DASServer deployment now terminates TLS
  1.3 on port 8731,
  forwards to a loopback-only Monas listener, and uses a narrow leaf certificate
  containing ``192.168.1.192`` signed by a Mac-local ``mkcert`` CA. The CA key
  never left the Mac and the leaf key is private below ``~/.x-img/tls``. Curl
  verified the chain without ``-k``; Firefox 152.0.6 with
  ``acceptInsecureCerts=false`` loaded the Monas route and installed the current
  unsigned Pinakotheke 1.2.1 XPI temporarily. The cPanel ``ZoneEdit`` probe
  failed because that module is unavailable, so no non-functional DNS-01 or
  external-service dependency was introduced.

- [x] **XIMG-100 P0 — Obtain and verify Mozilla's unlisted production
  signature.** The repository is AMO-ready: its Firefox manifest uses only
  valid permissions and Firefox background declarations, preserves the stable
  Gecko ID, and accurately declares required browsing activity, website
  content, and website activity transmission. ``make firefox-lint`` runs the
  pinned Mozilla validator with warnings as errors; ``make firefox-sign`` uses
  environment-only AMO credentials, requests the unlisted channel, and verifies
  the returned signature envelope, identity, and workspace version. Mozilla
  approved the first unlisted ``1.2.1`` XPI on 2026-07-17. Its SHA-256 is
  ``1e32a642c576503b89f4e2c2131e1916dfc03cb5561ecf60ffc2e31b6207f229``;
  Firefox ``152.0.6`` accepted it as a permanent add-on in an isolated ordinary
  profile. Stable-identity upgrade, paired opted-in capture, and substitution
  fail-open acceptance passed against the exact signed source. The trusted
  DASServer download uses ``application/x-xpinstall`` and matches the approved
  checksum. The listener redirects nginx's internal plaintext-on-TLS-port
  condition to the identical HTTPS URL, so an accidental ``http://`` install
  link cannot strand Firefox on a 400 page. Completed by ``6406ac2`` and
  ``414ab49``; no developer-mode artifact is release evidence.

- [x] **XIMG-101 P0 — Correct Pinakotheke sign-on branding in Monas.** Monas
  now selects a product-aware Pinakotheke presentation only from the validated
  same-origin return path. The page follows the DASObjectStore reference with a
  restrained Mnemosyne wordmark, archive-purpose context, focused sign-in
  panel, explicit Monas session ownership, and compliant provenance footer;
  Pinakotheke still receives no credentials and implements no session issuer.
  Monas tests, WebAssembly checks, rendered desktop review, trusted-HTTPS
  inspection, and live DASServer deployment passed. Completed in sibling Monas
  commit ``114ef95`` on 2026-07-17.

- [x] **XIMG-102 P1 — Ship the branded signed Firefox extension.** Replace the
  generic puzzle-piece fallback with approved grayscale Mnemosyne icon assets
  at 16, 32, 48, and 96 pixels. Preserve aspect ratio and transparency, bind
  the toolbar and product manifest icon maps, verify packaged XPI contents,
  obtain the Mozilla unlisted ``1.2.2`` signature, prove permanent Firefox
  installation, and update the trusted DASServer download without changing the
  stable Gecko identity.
  Completed in ``67980e2`` on 2026-07-17. Mozilla returned the signed 1.2.2
  XPI, Firefox accepted it as a permanent add-on, and the checksum-identical
  DASServer deployment is available over trusted HTTPS.

- [x] **XIMG-103 P0 — Deliver server-led Firefox onboarding.** Completed in
  ``57d7a02`` and ``77a2c01`` with scoped Monas pairing support in sibling
  commit ``817d956``. The live DASServer proof on 2026-07-17 configured named
  ObjectStore ``pinakotheke_media``, returned authenticated onboarding over
  HTTPS, rejected a changed pairing with HTTP 401, served the Mozilla-signed
  1.3.1 XPI, and passed permanent Firefox installation. The
  Monas-authenticated Pinakotheke web UI must expose the current signed XPI and
  an actor-bound pairing payload only when a reviewed named DASObjectStore is
  configured. Firefox must validate the instance, pairing, and Ready
  endpoint/ObjectStore response against the authenticated server before it
  saves configuration. Site permissions remain exact-origin opt-ins, with an
  explicit X-ingress intent checkbox. Acceptance requires a live DASServer
  named ObjectStore, private capture authority, signed extension, authenticated
  pairing proof, and failed pairing when logged out or when payload values are
  changed.

- [x] **XIMG-104 P0 — Replace capture approval with automatic site caching.**
  Pinakotheke 1.6.0 adds a three-second live, actor-scoped ingress status strip
  with observed thumbnail/opened image/opened video, pending, verified stored,
  and gallery counts. It also corrects X linked-image handling: Firefox now
  submits the rendered image URL as the byte source and retains the enclosing
  X status URL only as presentation provenance. This resolves the live defect
  in which trusted clicks reached the server but remained pending because the
  acquisition helper received HTML rather than image content.
  The first-party DAS helper now uses the account-bearing presentation URL to
  commit keys under ``x.com/<canonical-account>/<capture-kind>/<sha256>``;
  unassignable X CDN media is quarantined under ``_unattributed`` rather than
  misattributed. A synthetic extension contract proves an opted-in X thumbnail
  is submitted from viewport observation without any click.
  Delivered in ``91c96a8`` and deployed to the x86_64 DASServer on 2026-07-17.
  Mozilla signed extension 1.6.0, permanent Firefox installation, package
  upgrade, service restart, HTTPS health, and XPI delivery checks passed. The
  task remains open only for the normalized opened-video completion path.
  Live 1.6.0 evidence then exposed two defects: X CDN query removal produced a
  404 during helper acquisition, and a segmented-video substitution result
  obscured capture status. Patch 1.6.1 preserves only the safe ``format`` and
  ``name`` CDN variants, permits capture independently of legacy
  ``instanceId`` state, and presents capture/substitution results separately.
  Delivered in ``822aa82`` and deployed to the x86_64 DASServer on 2026-07-17.
  Mozilla signing, stable-identity permanent installation, DEB upgrade, service
  health, and live XPI delivery passed for 1.6.1. Fresh browsing is required to
  create query-preserving plans; the four earlier queryless plans remain
  honestly pending and cannot be repaired without their discarded variants.
  Live 1.6.1 testing exposed an observability blocker: the extension produced
  no new plans after installation while the server discarded helper failures.
  Pinakotheke 1.7.0 adds a bounded Firefox event ring and JSON download plus
  structured server admission/acquisition/settlement/gallery logs. Events omit
  media/page URLs, credentials, cookies, and browsing history outside enabled
  origins. This must be used for the next live X and generic-site capture proof.
  Its first deployment exposed non-protocol helper stderr from all four legacy
  plans; the boundary now converts failures to bounded ``policy_blocked``,
  ``unavailable``, or ``rejected`` outcomes without exposing source details.
  Delivered in ``1e16178`` and ``cb3e15d`` and deployed to the x86_64
  DASServer as 1.7.0. Mozilla signature verification, public XPI delivery,
  service restart, and bounded legacy-plan failure logging passed.
  Live 1.7.0 capture then proved download and DASObjectStore commit but exposed
  a settlement mismatch: reconciliation rejected the safe canonical X
  ``format``/``name`` alias. Version 1.7.1 reuses the admission canonicalizer
  for alias validation and adds an exact regression test while retaining
  fail-closed rejection of arbitrary and signed query parameters.
  Live recovery then exposed ``OriginalRequiresThumbnail`` because the opened
  image arrived without an observed-thumbnail plan. Original-first admission
  now creates a reviewable card using the same verified DAS object for its
  initial thumbnail delivery role; a later thumbnail safely replaces that role.
  Delivered in ``b9697ec`` and ``f81235e``. Live DASServer restart recovery
  settled ``capture-plan-5`` without a new browser submission, logged
  ``gallery_admitted``, and produced an original-first gallery card whose
  thumbnail and preview roles reference the same existing DAS object.
  Live gallery requests exposed HTTP 502 because the first-party read-helper
  config was absent and existing DAS S3-export objects lack optional checksum
  metadata. Version 1.7.2 provisions the reviewed store mapping and permits
  full-image delivery only after bounded scratch download and SHA-256
  verification; range reads still require authority checksum metadata.
  Automatic X thumbnails produced no plans while explicit navigator opens did.
  The exact-origin observer now supplies its validated visible-image snapshot
  with each mutation signal, avoiding a second privileged DOM execution while
  retaining viewport, size, HTTPS, origin-policy, and candidate bounds.
  Delivered in ``41d9cc3`` and deployed to the x86_64 DASServer on 2026-07-17.
  A previously blank 59,263-byte JPEG was read from the configured logical
  ObjectStore, independently SHA-256 verified against its catalogue reference,
  and returned with the content helper protocol. The 1.7.2 DEB upgrade,
  service restart, Mozilla signature verification, nginx configuration check,
  and public ``application/x-xpinstall`` delivery all passed. Fresh browsing
  with extension 1.7.2 remains the required live proof for automatic visible
  X-thumbnail admission.
  Image delivery landed in ``dc09fe6``: enabled pages debounce load, mutation,
  and scroll observations; meaningful visible images are submitted without a
  toolbar action; linked originals retain the trusted-click boundary; and an
  actor-scoped status reports ``stored`` only after verified settlement plus
  live gallery admission. Firefox then applies a browser-only two-pixel green
  border. Full workspace tests, strict clippy, wasm, zero-warning extension
  lint, quality checks, and the pinned Sphinx container build/run pass. The
  A live x86_64 DASServer proof on 2026-07-17 settled a restart-recovered plan,
  committed the object through scoped DASObjectStore access, and immediately
  produced one gallery item. X ingress now admits only ``pbs.twimg.com`` image
  media, excluding interface artwork and emoji. The task remains open: trusted
  video play is detected and honestly reported, but must next be composed with
  the normalized-video worker and equivalent verified gallery/status path.
  Mozilla signed extension 1.5.1 on 2026-07-17; permanent-install acceptance
  passed and the authenticated application advertises the deployed LAN XPI.
  Treat an enabled exact-origin site rule as the user's standing consent to
  cache the selected media classes. Cache displayed thumbnails immediately;
  cache an image original only after the user opens it, and cache video only
  after the user opens or plays it. Verified commits enter the common gallery
  immediately with review state ``New`` and require no pre-ingest approval.
  Ignore incidental browser/site chrome when a source adapter can identify
  content media. A cache hit must add a non-obstructing two-pixel green frame
  in Firefox using extension styling only; never mutate stored media bytes.
  X media uses a stable logical DASObjectStore key prefix and gallery grouping
  ``x.com/<canonical-account>/...`` rather than an unmanaged filesystem
  subdirectory. Deduplicate immutable bytes across actors while retaining
  actor-specific observation, provenance, and review state.
  Completed by the assembled gate in ``f7dadb1``: installed Firefox proves an
  automatically observed thumbnail plus trusted opened original and video,
  waits for verified ``stored`` status, and checks the browser-only two-pixel
  frame. XIMG-111 supplies automatic incompatible-video normalization and
  XIMG-113 supplies committed poster/metadata, immediate persistent gallery
  admission, and Firefox playback assurance. The earlier DASServer evidence
  above remains the live authority half; no approval queue or local payload
  store has been reintroduced.

- [x] **XIMG-105 P0 — Persist and reconcile the user site corpus.** Completed
  in ``8bf3530`` with bounded deletion tombstones in ``7a2812d``. The strict
  private store is actor-scoped and restart-safe; authenticated GET/PUT uses
  optimistic revisions and returns HTTP 409 with the current corpus for stale
  writers. Firefox migrates existing rules on first pairing, restores the
  server corpus at startup, persists each settings change, reports conflicts,
  and provides credential-free bounded JSON export/import. Native core/API
  tests, strict clippy, wasm check, zero-warning ``web-ext`` lint, quality
  checks, and the pinned Sphinx container build/run passed. The Monas-compatible
  POST correction is ``18be683``. A live DASServer 1.4.0 deployment then saved
  the user's ``https://x.com`` images/videos, capture/substitution, X-ingress
  rule at revision 1 and returned the identical corpus after a Pinakotheke
  restart. Original scope:
  store the
  strict versioned corpus of exact-origin rules under authenticated
  Pinakotheke authority, scoped to the actor and containing images/videos,
  capture, substitution, and X-ingress intent. Firefox keeps a local working
  copy for light-touch operation, but reconciles with the server after pairing
  and startup. Ordinary signed-extension upgrades must preserve rules through
  the stable Gecko identity and ``browser.storage.local``; uninstall, profile
  replacement, or a second device restores them from Pinakotheke. Use explicit
  revision/conflict handling, reject unknown schema majors, preserve deletions
  with bounded tombstones, and never silently promote actor rules to a shared
  multi-user policy. Add export/import for recovery without credentials.

- [x] **XIMG-106 P0 — Make gallery delivery concurrent and provenance-led.**
  Completed in ``6ae7bd9`` and deployed to the x86_64 DASServer as 1.7.3 on
  2026-07-17. The former single serialized reader is a bounded 128-slot Axum
  pool; blocking helper/provider/download/checksum work runs off async request
  threads with backpressure. Eight-reader regression evidence proves overlap.
  Checksum-versioned responses are private-cacheable for one hour. Cards lead
  with the captured X account and UTC capture time, retain honest generic-site
  fallback labels, and new X admissions use the X-account source class. All
  193 workspace tests, strict Clippy, wasm compilation, quality checks, and the
  pinned Sphinx container build/run passed. On the live server, 15 cold legacy
  images totalling 1,344,774 bytes completed in 1.913 seconds concurrently,
  versus 9.632 seconds sequentially; unchanged repeat browser views use the
  private cache instead of retrieving the objects again.

- [x] **XIMG-107 P1 — Add latest-download and artist-folder browsing.**
  Completed in ``93db65d`` and deployed to the x86_64 DASServer as 1.8.0 on
  2026-07-17. The default graphical gallery requests the newest 20 downloads.
  A strict authenticated folder projection provides root and breadcrumb
  navigation, immediate child counts, latest-capture times, and exact
  object-prefix filtering while preserving folder/source/text context for the
  next 20 cards. It presents ``x.com/<artist>/<capture-class>`` without exposing
  filesystem paths or unrelated DASObjectStore objects. All 194 workspace
  tests, strict Clippy, wasm compilation, quality checks, and the pinned Sphinx
  container build/run passed. Live evidence projected 25 catalogue items, 24
  under ``x.com``, 15 artist folders, and exactly three ``p0ttyprincess`` items
  beneath its ``explicit_original`` child. This was checked against
  DASObjectStore ``2e1e1669ff0bccc05324b3b74785300e00f53d90`` and design
  language ``fbfa28e55d1c8111ef95a139d83927c231534b5f``.

- [x] **XIMG-108 P0 — Capture trusted-click X progressive video.**
  Detect only a recent trusted pointer/keyboard activation followed by playback,
  select a concrete HTTPS MP4 exposed by Firefox from ``video.twimg.com``, and
  submit an ``explicit_video`` plan. The first-party worker must retrieve it
  without cookies, bound its size, require ``video/mp4``, prove H.264 video and
  absent-or-AAC audio with ``ffprobe``, commit it through the reviewed
  DASObjectStore, and admit a playable ``New`` gallery item under the X artist
  folder. Autoplay, synthetic events, non-X media hosts, segmented/MSE-only
  playback, and unsupported codecs must remain origin-served with a redacted
  diagnostic. Acceptance requires native/extension tests, local documentation
  verification, a signed extension, DASServer deployment, and a live X video
  commit/playback proof.
  Completed by the assembled path in ``f7dadb1`` together with XIMG-110,
  XIMG-111, and XIMG-113. The user-approved XIMG-110 site-neutral policy
  deliberately supersedes the original ``video.twimg.com``/non-X-host split:
  eligibility is now exact-origin opt-in plus recent trusted activation, while
  X presentation provenance still produces its account folder. Native tests
  prove credential-free bounded MP4 retrieval, H.264/AAC probing, verified DAS
  completion, and immediate ``New`` admission; signed-extension/DASServer runs
  established the deployed path. The installed-Firefox gate now proves trusted
  progressive submission and stored framing, and the normalized playback gate
  proves metadata load, range seek, pause/resume, concurrent/conditional reads,
  cancellation, and unavailable-object recovery without origin fallback.

- [ ] **XIMG-109 P0 — Require authoritative DASObjectStore capture completion.**
  Remove the transfer-only success path from the first-party helper. A capture
  configuration must set ``submit_to_daemon`` to ``true`` and provide the
  reviewed daemon socket; the paired DASObjectStore remote client must attach
  SHA-256 metadata and report the terminal
  ``remote_s3_transfer_complete`` state only after provider verification,
  two-copy placement settlement, and catalogue publication. The DASServer
  writer was paused on 2026-07-17 and guarded ``--reconcile-s3`` recovered 38
  provider objects (3,793,683 source bytes) through normal SSD-first ingest.
  Follow-up verification scanned 38 catalogue objects and 76 placements with
  zero missing payloads, size mismatches, hash mismatches, or unverified
  placements. The host's broken distro AWS CLI was replaced by the official
  self-contained v2.36.1 executable after its Python 3.14 build failed before
  listing Garage. Complete this item after the fail-closed helper is deployed,
  the writer is restarted, and one fresh browser capture proves catalogue
  growth rather than provider-only growth. Compatibility was checked against
  DASObjectStore ``2e1e1669ff0bccc05324b3b74785300e00f53d90``.
  On 2026-07-17, Pinakotheke ``1.9.0`` and DASObjectStore ``0.114.3`` were
  deployed with distinct logical-store/provider-bucket daemon routing and a
  bounded shared staging directory. A live checksum-bearing capture completed
  through the daemon and appeared in the authoritative
  ``profile_catalogue_objects`` table under ``provider:garage``; three live
  completion records are now present and staging was empty after each run.
  DASObjectStore commit ``6e89fd55`` supplies the reviewed bucket handoff.
  Keep this item open: the current remote-completion contract records verified
  externally replicated provider placement, while this acceptance contract
  additionally requires automatic settlement into the store's two managed HDD
  copies and a fresh installed-Firefox capture proof.

- [x] **XIMG-110 P0 — Generalize trusted-play progressive video capture.**
  Remove the accidental single-source restriction from the first-party capture
  helper. Accept concrete progressive HTTPS video from any explicitly enabled
  origin only after trusted playback, without browser credentials. Preserve
  bounded retrieval, strict provenance, MP4 MIME and H.264/AAC verification,
  authoritative DASObjectStore completion, idempotency, and fail-open behavior.
  Website names must not be compiled into code, manifests, fixtures, or user
  documentation. Acceptance requires generic-origin unit coverage and an
  installed-Firefox proof against one operator-enabled origin.
  Pinakotheke 1.12.0 detects an opaque progressive HTTPS resource exposed in
  the trusted-play window and carries its validated exact retrieval URL
  privately to the isolated acquisition helper. Stable query-free identity
  still provides idempotency, while a rotated capability refreshes an
  unsettled plan. Mozilla-signed 1.12.0 permanently installs with the stable
  identity in Firefox 152.0.6. The real-Firefox harness now creates an
  ephemeral progressive video inside its isolated profile and proves that a
  native trusted pointer/play produces an ``explicit_video`` plan without
  cookies, headers, payload bytes, or an encoded website catalogue. The
  checksum-identical signed XPI and 1.12.0 service are deployed on DASServer.
  Delivered in ``d0bb6cb``.

- [x] **XIMG-111 P0 — Automate container normalization handoff.**
  When progressive media is eligible but not already in the browser playback
  profile, create a redacted codec-gap record and hand the bounded source to
  DAS-managed staging. Select an authorized worker on the DASObjectStore host,
  paired Firefox device, or governed remote worker; run the existing
  digest-pinned network-isolated FFmpeg adapter; commit rendition, poster, and
  provenance manifest as separate objects; delete staging; and admit only
  after checksum, probe, and Firefox playback evidence. No unsupported source
  may be mislabeled as stored or playable. Completed in ``3ce70cb`` with the
  1.14.0 DASObjectStore-host handoff: incompatible bounded progressive video
  records only an aggregate redacted codec gap, enters private DAS-managed
  staging, runs the existing digest-pinned network-isolated normalizer, commits
  normalized MP4, WebP poster, and expanded provenance manifest separately,
  and cleans staging. Gallery settlement receives only the verified MP4 after
  all three commits, output probing, and configured Firefox profile evidence;
  failures leave the capture pending and never admit the source bytes.

- [x] **XIMG-112 P0 — Prove generic segmented-video adapters.**
  Add a site-neutral plan for user-played HLS/DASH or MSE media without hidden
  traversal, playlist crawling, browser cookies, or authorization headers.
  Permit an adapter only after bounded synthetic fixtures prove manifest and
  segment identity, retry/idempotency, policy and DRM blocks, and fail-open
  origin playback. Record only redacted codec/container diagnostics. Completed
  in ``1c26c20`` with the metadata-only HLS/DASH/MSE planner, deterministic
  versioned plan identity, strict observation and proof bounds, redacted
  diagnostics, and a redistributable positive/blocking fixture matrix. Planning
  never fetches or traverses media, and substitution remains origin-served
  until the existing normalized rendition and exact Firefox evidence gate is
  satisfied.

- [x] **XIMG-113 P1 — Complete the playable-video library campaign.**
  Expose a dedicated Videos filter and a keyboard-accessible quick viewer using
  authorized DASObjectStore range delivery, committed posters, native controls,
  and inline playback. Add duration, dimensions, codec/profile, capture time,
  source account/origin label, and normalization state. Prove prompt loading,
  seeking, missing-object behavior, and that the viewer never contacts the
  source website. Completed in ``9279979``: normalization probe metadata and
  the separately committed poster now survive the isolated worker handoff and
  persistent catalogue; the dedicated Playable videos view and quick viewer
  expose the complete evidence. Installed Firefox passed the real Yew filter,
  metadata, keyboard, virtualization, unavailable/no-origin, and responsive
  checks plus ephemeral normalized-MP4 metadata load, range seek, concurrent
  and conditional reads, cancellation, pause/resume, and missing-object proof.

- [x] **XIMG-114 P0 — Terminate trusted HTTPS directly in Axum.** Completed in
  ``e61699f``, ``d2fdfc2``, and ``65ba25b`` with Monas host support in
  ``74aa66a`` and ``799484e``.
  Add paired certificate-chain/private-key CLI arguments and a Rustls listener.
  Reject partial pairs, relative paths, symlinks, empty assets, and group/other
  readable private keys before binding. Preserve HTTP only as an explicit local
  development mode. Document CA trust, SANs, verification, rotation, service
  management, migration, and rollback in the authoritative root guide. Deploy
  to DASServer on port 8731 with nginx absent from the request path and prove
  `/ready`, Monas/Yew application access, and extension download over trusted
  HTTPS.
  DASServer deployment now binds Monas/Rustls directly to ``0.0.0.0:8731`` and
  keeps Pinakotheke's authenticated product backend on loopback ``8732``.
  The Pinakotheke nginx site is disabled. A trusted client proved the Monas
  login page, HTTP/2 XPI delivery with ``application/x-xpinstall``, and both
  services active. A restart proof also confirmed graceful Pinakotheke shutdown
  releases and reacquires the capture-worker lease without manual cleanup.

- [ ] **XIMG-116 P0 — Capture late script-fetched progressive video after
  trusted play.** The progressive-resource implementation landed in
  ``1.17.0``: the exact
  opted-in top-frame observer now polls nine bounded times over two seconds
  after recent trusted activation, considers script/fetch initiated progressive
  resources as well as native video initiators, and retains the strict
  ``video.twimg.com`` gate for X. Synthetic tests prove a ``blob:`` element can
  resolve a recent fetch-initiated X MP4 without reading request headers,
  cookies, credentials, or response bytes. Segmented/MSE-only and unresolved
  playback remains origin-served. Mozilla signed the ``1.17.0`` extension,
  permanent-install verification passed, and DASServer serves the
  checksum-identical XPI with ``application/x-xpinstall`` beside the ready
  ``1.17.0`` backend. Live diagnostics after installation showed no new
  content-observer event and no server plan because Firefox had registered the
  updated script only for future documents in the already-open X
  single-page-application tab. ``1.17.1`` now immediately injects the guarded,
  idempotent observer into every open eligible tab after registration while
  retaining persistent registration for future navigation. Remaining
  acceptance: install the ``1.17.1`` XPI and prove one real
  user-played X video reaches verified DASObjectStore settlement and appears as
  a playable gallery item. Do not mark complete from an admitted or pending
  plan. Mozilla signing, permanent-install verification, and checksum-identical
  DASServer deployment of the ``1.17.1`` backend and XPI are complete in
  ``c010870``. A subsequent live play produced no server plan: X's overlaid
  control was not related to its sibling video by the prior ancestor walk.
  ``1.17.2`` maps a trusted pointer inside a visible video rectangle to that
  exact video and still requires its genuine play event within two seconds;
  synthetic coverage includes a separate overlay control. Live capture proof
  remains. Mozilla signing, permanent-install verification, and
  checksum-identical DASServer deployment of the ``1.17.2`` backend and XPI
  are complete in ``7f145d6``.

- [x] **XIMG-117 P0 — Assemble trusted-play HLS/fMP4 into a committed video.**
  Live ``1.17.2`` plan ``capture-plan-48`` proved end-to-end observer and plan
  admission, then correctly failed because the chosen ``.m4s`` object was a
  48,529-byte fragment without initialization metadata. ``1.18.0`` selects the
  matching observed master manifest by stable media-family identity, excludes
  fragment suffixes from progressive capture, and adds bounded clear-HTTPS
  HLS/DASH assembly before the existing probe and verified DASObjectStore
  completion path. Synthetic tests must prove master-over-track selection,
  fragment rejection, structured FFmpeg invocation and bounds. Completion
  additionally requires one real user-played X video to settle and appear in
  the gallery; do not mark complete from plan admission alone. Implementation
  is pushed in ``8828024``. Mozilla signing, permanent-install verification,
  and checksum-identical DASServer deployment of the ``1.18.0`` backend and XPI
  are complete. A subsequent live play reached the content observer but was
  rejected as ``missing_trusted_activation`` before any server request.
  ``1.19.1`` adds a bounded genuine-page-activation fallback while retaining
  the autoplay block. Implementation is pushed in ``f94a5e3``; Mozilla signing,
  permanent-install verification, and checksum-identical DASServer deployment
  of the matching ``1.19.1`` backend and XPI are complete. Live ``1.19.1`` then
  proved that its injected script exited against the legacy observer marker in
  an already-open X tab. ``1.19.2`` versions that marker so upgraded logic is
  installed immediately. Implementation is pushed in ``25284ef``; Mozilla
  signing, permanent-install verification, and checksum-identical DASServer
  deployment of the matching ``1.19.2`` backend and XPI are complete. The live
  playback then progressed to ``segmented_or_unresolved`` because the worker-
  fetched manifest was absent from page Resource Timing. ``1.20.0`` adds a
  bounded URL-only completed-request handoff for the explicitly permitted X
  media host. Implementation is pushed in ``cc99a20``; Mozilla signing,
  permanent-install verification, and checksum-identical DASServer deployment
  of the matching ``1.20.0`` backend and XPI are complete. The user-played
  request still produced no plan because Firefox attributed the worker-owned
  manifest to no tab. ``1.20.1`` correlates such manifests by stable media
  family under the enabled X-video rule. Implementation is pushed in
  ``03b42e3``; Mozilla signing, permanent-install verification, and checksum-
  identical DASServer deployment of the matching backend and XPI are complete.
  The settlement criterion remains unchecked pending one fresh play with the
  newly installed ``1.20.1`` extension.
  Live ``1.20.1`` then produced no new plan because Firefox had not granted the
  user-controlled ``video.twimg.com`` origin permission. ``1.20.2`` requests
  that exact permission during X-video enablement and from the toolbar repair
  action for existing rules. Implementation is pushed in ``2c6ce92``; Mozilla
  signing, permanent-install verification, and checksum-identical DASServer
  deployment of the matching backend and XPI are complete. Real settlement
  proof after granting the permission remains required.
  Live ``1.20.2`` then exposed a user-action ordering defect: an awaited
  permission check preceded the request, so Firefox could not show approval.
  ``1.20.3`` caches the check during rendering and starts the request directly
  in the toolbar click handler. Implementation is pushed in ``b1a71fe``;
  Mozilla signing, permanent-install verification, and checksum-identical XPI
  deployment are complete. Docker Desktop returned an output-sync I/O error,
  so the same pushed source was built with the DASServer Docker engine; the
  resulting ``1.20.3`` package upgraded the backend and Yew web assets. Live
  readiness reports Monas and DASObjectStore ``Ready``, and the authenticated
  web application now receives the ``1.20.3`` XPI path from onboarding. Live
  ``1.20.3`` then admitted user-played ``capture-plan-52`` from an X HLS master.
  DASServer initially rejected the helper configuration because its timeout
  executable was a symlink; resolving that configured path to its regular
  executable allowed restart reconciliation to assemble, checksum-verify,
  settle, and admit the 12,628,955-byte MP4. The live catalogue exposes it as a
  new ``normalized_video`` card for ``X / @bblinguinii`` with an authorized
  ``/video`` delivery route and an ObjectStore key beneath that account's
  ``explicit_video`` prefix. XIMG-117 is complete.

- [x] **XIMG-118 P0 — Report committed video availability independently of
  optional posters.** Completed in ``383a920``. The gallery now derives a
  normalized video's availability, endpoint, ObjectStore, and object version
  from its committed video representation. An absent poster no longer causes a
  range-readable MP4 to be labelled ``Object unavailable``. A regression test
  covers the ready-video/unavailable-poster record. DASServer was upgraded to
  ``1.20.4`` and live verification returned a 1,024-byte ``206`` MP4 range for
  the affected 12,628,955-byte object; both services remained active.

- [x] **XIMG-119 P1 — Generate and settle representative video posters.**
  Completed in ``df45e38``. Firefox-compatible MP4 acquisition now probes
  duration, dimensions, and codecs, extracts one bounded WebP frame, commits it
  as a distinct checksum-verified DASObjectStore object, and admits its metadata
  with the video card. Synthetic tests prove timeout-bounded extraction and
  daemon-verified poster settlement. DASServer runs ``1.21.0``; the existing X
  video was backfilled with a 33,906-byte poster and its authenticated thumbnail
  route returns ``200 image/webp`` with the matching SHA-256 ETag. Mozilla
  signed the version-synchronized ``1.21.0`` extension, the permanent-install
  check retained its stable identity, and DASServer serves the checksum-identical
  XPI as ``application/x-xpinstall``.

- [x] **XIMG-115 P0 — Make Monas restart invalidate sessions and harden product
  forwarding.** Completed in Monas ``0.8.4`` commits ``624e7b4``,
  ``c91c544``, and ``90ed54a`` and recorded
  in Pinakotheke compatibility documentation. Monas now revokes all existing
  Prosopikon browser sessions before serving after startup, requiring a fresh
  login while retaining user registration and password data. Authenticated
  Pinakotheke GET/HEAD forwarding no longer attaches a synthetic streaming
  request body, strips request framing and hop-by-hop headers, and emits only
  secret-safe upstream failure categories. Unit tests prove restart revocation,
  repeated-start idempotence, protected-route redirects, and safe forwarding.
  Upgraded installations first assign immutable principal/session IDs through
  Prosopikon's digest-guarded migration and retain a private byte-for-byte
  pre-migration backup; this prevents successful legacy-account login from
  looping back to the sign-in page.
  HTTP/2 absolute browser URIs are normalized to path and query before the
  fixed loopback origin is applied, preventing request-builder failure without
  forwarding the public scheme or authority. Live DASServer evidence confirms
  a valid fresh session and no authenticated upstream failure.

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
- [x] **XIMG-093 P1 — Add macOS per-user service management.** Completed in
  ``b948fe2``. Provide
  non-root ``launchd`` install/status/logs/restart/uninstall commands with
  absolute paths, private permissions, transactional updates, preserved state,
  graceful shutdown, and no destructive uninstall default. The new service
  commands manage separate backend and Monas agents, generate a private
  process credential, separate Prosopikon authority, restore prior definitions
  on failed replacement, and preserve every data root on uninstall. A real
  isolated-home lifecycle proved install, status, health, direct-route
  rejection, restart, log discovery, private files, agent removal, stopped
  listeners, and retained state.
- [x] **XIMG-095 P0 — Deliver the Monas-owned login and session screen.**
  Completed in Monas ``0.3.0`` commit ``a0fabe2``. The
  current Monas Yew surface is a placeholder even though XIMG-092 completed the
  authentication APIs, Prosopikon authority, session cookie, product mount,
  forwarding, and host-context injection. Add a user-facing sign-in experience
  to ``../monas`` modelled on the calm two-part DASObjectStore WebUI login shell
  inspected at DASObjectStore commit
  ``a93f0f872152d3790746292de1f3aec5d1a7bdd3`` without copying its product-owned
  authentication boundary. Monas must own username/password submission,
  registration-token onboarding where enabled, session check, expiry,
  logout, and safe same-origin return to the requested Pinakotheke route;
  Pinakotheke must never receive or render the password, Prosopikon token, or
  ``monas_session`` cookie. Use the approved Mnemosyne wordmark, semantic
  tokens, normative footer assets, and one decorative partial mark—never the
  current text/Unicode approximation. Design explicit ``Checking session``,
  ``Signing in``, invalid credentials, expired session, host unavailable,
  signed out, and retry states; use accessible labels/autocomplete, disabled
  busy submission, error alerts, keyboard/focus behavior, responsive layout,
  WCAG 2.2 AA contrast, and no password persistence. Prove in real Firefox that
  an unauthenticated Pinakotheke deep link reaches login, successful login
  returns to that exact allow-listed route, refresh preserves the host session,
  logout and expiry revoke access, direct backend requests still fail, and no
  secret appears in Pinakotheke state, browser local storage, URL, or logs.
  Record the exact Monas, Prosopikon, DASObjectStore design reference, and
  Mnemosyne design-language commits. Monas native/WASM tests prove safe return,
  login, session admission, logout revocation, invalid/external targets, and
  re-gating; a real browser proved the Pinakotheke deep-link redirect and
  accessible branded login DOM. Approved assets are served from the packaged
  Mnemosyne branding root (or an explicit development root), never redrawn.
  Pinakotheke never receives the password or cookie. XIMG-094 is now unblocked.
- [x] **XIMG-094 P0 — Prove clean-home monolith operation.** In an isolated
  temporary home, start the service, complete Monas login, select the managed
  local ObjectStore, commit and read one synthetic object through scoped DAS
  contracts, restart and reconcile exactly once, then shut down cleanly. Record
  bounded local evidence and do not rely on GitHub Actions.
  Completed in ``9e1688c`` against DASObjectStore
  ``f195c4d5a30d1cc34ca61f31a6939edf54db782f``: an isolated macOS home proved
  Monas login and exact return, direct-backend rejection, all-component
  readiness, daemon-verified synthetic commit, checksum-identical scoped
  read-back, restart reconciliation with session continuity, logout revocation,
  and clean shutdown. The new explicit local-profile API port kept the proof
  isolated from an existing default-port authority. All 175 workspace tests,
  strict Clippy, repository quality checks, native warnings-denied Sphinx, and
  both the pinned documentation-container build and run passed locally.
  Progress on 2026-07-16: the packaged binary now implements the production
  ``read-v1`` helper protocol using a strict mode-0600 endpoint/store-to-bucket
  configuration, host-owned scoped AWS credentials, verified DAS checksum
  metadata, conditional reads, bounded ranges, and always-cleaned private
  ephemeral scratch. This was reviewed against DASObjectStore commit
  ``5769f27859a58101aedd9de0087fc278fd3e4b16``. Docker Desktop did not recover
  after a full application restart, so the clean-home live authority run is
  the next executable step rather than simulated evidence.
  Verification: all 172 workspace tests, strict workspace Clippy, repository
  quality/privacy/version checks, release security/license audits, and
  warnings-denied Sphinx 8.2.3 pass locally.
  Further progress on 2026-07-16: after a clean Docker backend restart, a real
  isolated HOME provisioned and strictly rediscovered endpoint
  ``local-docker-2731860728`` plus ObjectStore ``pinakotheke_local`` and wrote a
  mode-0600 secret-free selection. Repeated provisioning now reconciles a
  failed already-running start only through the exact Ready authority identity.
  DASObjectStore commit ``b88eba40`` fixes its local Docker build to use the
  Prosopikon source already copied into the public build context. The next
  blocker is narrower: Docker Desktop exposes the container-created daemon
  Unix-socket path but refuses host connections, so authoritative remote-client
  completion needs a supported host-reachable daemon transport or a packaged
  container-side remote client. Direct S3 smoke/write results are explicitly
  insufficient to close XIMG-094.
  DASObjectStore commit ``01a8c385`` now packages its version-matched
  ``dasobjectstore-remote`` client and digest-pinned AWS CLI beside the daemon
  socket in the local authority image. A real arm64 image build proves Docker
  Compose, AWS CLI 2.27.49, and remote client 0.110.0 are executable together.
  This removes the unsupported host-socket dependency. The next Pinakotheke
  slice must add a narrow container execution adapter with managed-root-only
  source translation and scoped credential handoff; browser/product requests
  must never select a container, host path, or provider credential.
  The narrow adapter is now implemented: native and container execution are
  mutually exclusive, the Docker Compose service and daemon socket are fixed,
  only a canonical DAS-managed scratch descendant is translated, and scoped
  remote/AWS files exist only in the private automatically removed job
  directory. Unit coverage proves the structured invocation and cleanup. The
  first isolated live run exposed two authority-profile defects rather than
  bypassing completion: non-default Garage listeners diverged from Compose,
  and capacity admission lacked a folder-profile binding. DASObjectStore commit
  ``720ae9c1`` aligns the four ports and idempotently provisions the canonical
  binding. Docker Desktop then failed to restore its daemon socket after an
  authorized restart. The remaining gate is to rerun this fixed profile,
  commit/read the synthetic object, restart, and reconcile once.
  Authority-core evidence now passes against DASObjectStore commit
  ``b35ee0b2``: a clean canonical temporary home provisions a bounded local
  profile, two identical HTTPS image acquisitions converge on one checksum key
  and one immutable catalogue version, the scoped Pinakotheke read helper
  verifies all 8,090 bytes, and daemon restart plus reprovision preserves one
  catalogue row and the same checksum. The run also fixed S3 region handoff,
  matching store/binding capacity, durable local catalogue placement, initial
  profile-store registration, and retry-stable provider identity. The remaining
  XIMG-094 step is the same-home Monas login/monolith composition and clean
  shutdown. The capture helper now forwards its bounded verified ``image/*``
  type through both native and container execution using the validated
  ``--content-type`` contract in DASObjectStore commit ``7a3d5578``. The next
  live run must prove provider head/readback returns that exact type rather
  than ``application/octet-stream`` before XIMG-096 gallery admission.
  A fresh isolated rerun on 2026-07-16 restored Docker, provisioned the exact
  ``pinakotheke-clean`` authority at DASObjectStore commit ``26227ca2``, and
  exposed an obsolete ``upload --store`` invocation in the canonical remote
  completion smoke test. DASObjectStore commit ``03f88237`` fixes that Clap
  contract drift. The required exact-revision rebuild then stopped honestly on
  a Docker BuildKit ``metadata_v2.db`` input/output error while copying the
  pinned AWS CLI layer. The isolated containers, networks, and temporary home
  were removed after a hard backend restart. Next execution starts with Docker
  VM storage repair/reset, rebuilds commit ``03f88237`` or later, and resumes
  completion smoke before Monas composition; no direct-S3 substitute is valid.
  Verification: all 175 workspace tests, strict workspace Clippy, repository
  quality/privacy/version checks, release security/license audits, and
  warnings-denied Sphinx 8.2.3 pass locally. The isolated containers and HOME
  were removed after evidence collection; unrelated DAS development containers
  and ObjectStores were not changed. The required pinned documentation image
  build and container dummy verification also pass after Docker recovery.

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
- [ ] **XIMG-120 P0 — Make browser cache evidence and video progress explicit.**
  Query authoritative alias evidence independently of substitution and add a
  browser-only two-pixel green frame to matching settled images and videos.
  Correlate each trusted video selection to its page element, exclude autoplay
  following unrelated page activation, retain a bounded URL-free status list,
  and report selected/downloading/pending/stored/failed states in the toolbar.
  Close only after signed-extension installation and live X/DASObjectStore
  settlement proof on DASServer.
  Implementation is pushed in ``9e874df``. Mozilla approved the unlisted
  ``1.22.0`` XPI; permanent-install verification passed, and DASServer serves
  the checksum-matching artifact while both version-``1.22.0`` services are
  active. One user-driven X browse/play cycle remains before closure.
  That first live cycle exposed an absent production alias route. Hotfix
  ``371ebf0`` mounts persistent journal/gallery evidence at the authenticated
  product boundary; version ``1.22.1`` and its Mozilla-signed matching
  extension are deployed. A repeated browser observation remains the closing
  proof.
  The repeated cycle exposed X rendition drift: stored opened images used
  ``name=900x900`` while page thumbnails used ``small`` or ``medium``. The
  evidence identity now binds the stable ``pbs.twimg.com/media/<asset>`` path
  while acquisition and provenance retain the exact rendition URL.
  The correction is pushed in ``4a16175`` and the matching Mozilla-signed
  ``1.22.2`` server/extension pair is deployed on DASServer. A repeated browse
  remains the final visual acceptance check.
  That check exposed a second namespace mismatch: Firefox adapter ID
  ``generic-observed-image`` was incorrectly compared with server site policy
  ID ``x-web``. Version ``1.22.3`` removes that invalid comparison without
  weakening actor, pairing, origin, version, settlement, or gallery checks.
  Fix ``cbfcf62`` and the matching Mozilla-signed ``1.22.3`` package are now
  deployed; the next browse is the visual acceptance check.
  Because that check still showed no frame, ``1.22.4`` adds redacted server
  hit/miss-stage telemetry and extension lookup/frame-application counts. It
  also reinforces the non-destructive marker against site layout clipping.
  Instrumentation commit ``e83e010`` and the matching signed ``1.22.4`` pair
  are deployed. The next browse must be diagnosed from server telemetry and
  the popup's recent ``cache_evidence``/``stored_frame`` events.
  Installed-Firefox diagnostics then reported ``16 visible image(s); 0
  eligible``: X no longer consistently used the one hard-coded historical CDN
  host. Version ``1.22.5`` accepts every bounded, visibly rendered HTTPS image
  from the explicitly opted-in X page for evidence lookup and plan submission;
  server policy remains authoritative and unknown aliases remain harmless
  misses. Fix ``86a513a`` is pushed; Mozilla signed ``1.22.5``, its permanent
  Firefox installation check passed, and the matching package plus
  checksum-identical XPI are active on DASServer. A repeat browse is the
  remaining visual acceptance evidence.
  That live pass reached the production route but exposed HTTP 500 before the
  handler: the cache-evidence route was added after Axum's gallery extension
  layer and therefore lacked the authoritative catalogue required to prove a
  hit. Version ``1.22.6`` attaches that exact shared catalogue to the route and
  adds a production-monolith regression request; deployment and repeat browse
  remain. Fix ``3873fe8`` is pushed; Mozilla-signed ``1.22.6`` passed permanent
  Firefox installation and is deployed with the matching DASServer package.
  A live request using the installed extension's scoped pairing now returns
  HTTP 200 ``outcome=hit`` with settled checksum and gallery delivery evidence.
  Only the repeat installed-browser visual check remains.
  That check proved frames on unobstructed inline images, while X gallery-grid
  wrappers could visually cover the framed ``img``. Version ``1.22.7`` applies
  the same reversible browser-only state to the image and only its bounded
  same-footprint wrapper chain; it never expands to the tweet or changes stored
  bytes. Fix ``81bc048`` is pushed; Mozilla-signed ``1.22.7`` passed permanent
  installation and is deployed with the matching DASServer package. A repeat
  gallery-grid check remains.
  The repeat check and captured screenshot showed the remaining functional
  cause: the bounded visible-image list admitted avatars/emoji before actual X
  media, and X photo overlays received trusted clicks instead of the underlying
  unlinked ``img``. Version ``1.22.8`` prioritizes and admits only visible
  ``pbs.twimg.com/media`` assets for X evidence/capture, derives status-page
  provenance where available, and resolves a trusted click coordinate to the
  visible X media beneath the overlay. Fix ``ae3a108`` is pushed;
  Mozilla-signed ``1.22.8`` passed permanent-install verification and the
  matching package plus checksum-verified XPI are active on DASServer. The
  installed-browser gallery-grid/open-original check remains the closing
  visual proof.
- [ ] **XIMG-202 P3 — Add perceptual duplicate grouping.**
- [ ] **XIMG-203 P3 — Add collections, tags, and saved searches.**
- [ ] **XIMG-204 P3 — Add provenance-linked derivatives/transcodes.**
- [x] **XIMG-205 P0 — Consume audience-bound Prosopikon host identity.**
  Pinakotheke accepts the additive Monas v1 canonical identity group only when
  authority, principal, session, and exact ``pinakotheke`` audience are all
  present and valid. Legacy ``actor_id`` remains compatibility-only for
  extension pairing; no credential or local identity/session issuer was added.
