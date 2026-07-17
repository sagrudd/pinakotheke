# Pinakotheke milestones

Status: 1.0 stable release

Version: 1.7.1

Updated: 2026-07-17

Product identity: Pinakotheke is canonical from v1.0.0 at
`sagrudd/pinakotheke`; `x-img` remains only where documented compatibility or
historic schema identity requires it.

## Product outcome

Pinakotheke provides one private, authenticated Web library for media acquired from:

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
- Instagram is an explicitly enabled Firefox site policy for the first product
  path; a future official API connector is optional and records unsupported
  account/media classes clearly;
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

Critical product intent: Pinakotheke is a Monas-authenticated, ThumbsPlus-like
personal media library for the images and videos that the user captures with
Firefox. The useful product is the complete path from an observed thumbnail or
explicitly opened/selected original, through the chosen DASObjectStore and the
common review catalogue, to a real gallery card and authorized image preview or
normalized-video playback. Synthetic cards and proxy visuals are development
scaffolding only and are not release evidence. XIMG-096 is the normative
end-to-end gate for this intent.

Installed Firefox 152 now accepts the dual background manifest, and an isolated
WebDriver BiDi harness proves an observed linked thumbnail plus trusted opened
original through the production capture path. Non-default-port sites retain
exact-origin policy/provenance while using Firefox's required port-independent
host match pattern. Live paired DASObjectStore commit and restart evidence is
still required to close XIMG-096.

Commit ``1a15c10`` exposes the former in-process-only video normalizer as a
packaged reviewed host command. A strict private confirmed plan drives the immutable
container, profile, and resource contract, while normalized video, poster, and
manifest bytes stream to a helper-owned DASObjectStore authority over stdin;
exact receipts are required and scratch is always removed. This closes the
production worker-seam gap. Commit ``b46edce`` adds the first-party packaged
stream helper that carries the protocol to native or containerized
``dasobjectstore-remote`` execution. An isolated live run against DASObjectStore
``093772da79bbb494da070965c7d4f49e5ad83f56`` verified a synthetic manifest
commit, exact length, and content type in the selected ObjectStore. Commit
``c472973`` records the external-host Linux slice, which fixed Docker
``--mount`` syntax and runs the hardened, capability-free worker as the private
scratch owner. An x86_64 DASServer run
against DASObjectStore ``28e6d82cc8c25dd83838fde8b6de3aa16384eb95`` then
normalized and committed the complete MP4/poster/manifest triplet with
independent type and FFprobe verification. DGX Spark proved the same hardened
three-output worker and cleanup on GB10 arm64 with fixture authority completion.
XIMG-096 now requires Firefox playback, persistent gallery admission, and
restart evidence; the DAS helper's root-owned Linux bind output is tracked as
non-root provisioning debt.

The clean-home authority run now provisions and strictly rediscovers a real
isolated DASObjectStore endpoint/ObjectStore, including a mode-0600 persisted
selection. Repeated starts reconcile only through the exact Ready identity.
DASObjectStore commit ``b88eba40`` fixes the local image's public Prosopikon
build context. Docker Desktop still refuses host connections to the daemon's
container-created bind-mounted Unix socket, so a supported host-reachable
daemon transport (or container-packaged remote client) remains before the
commit/read/restart gate can close; a direct S3 write is not substitute proof.

DASObjectStore commit ``01a8c385`` supplies the container-packaged option: its
local authority image now contains the version-matched remote completion client
and digest-pinned AWS CLI beside the native daemon socket. The remaining
Pinakotheke work is a narrow host adapter that maps only DAS-managed scratch
into that container execution context and supplies scoped authority credentials
without exposing Docker, paths, or secrets to browser requests.

That adapter now exists behind the unchanged host-authenticated acquisition
protocol. Native and container transports are mutually exclusive; local Docker
uses the fixed authority service/socket, translates only a canonical managed
scratch descendant, hands off scoped files through the private job directory,
and deletes that directory on every outcome. Live isolated commit/read/restart
evidence remains before XIMG-094 and the Firefox vertical can close. The first
live run found and fixed mismatched non-default Garage listener ports and a
missing capacity profile binding in DASObjectStore commit ``720ae9c1``; Docker
Desktop did not restore its daemon socket after the subsequent authorized
restart, so the corrected authority rerun remains explicit rather than being
replaced with direct-S3 evidence.

DASObjectStore commit ``b35ee0b2`` and the Pinakotheke container helper now
prove the missing authority core with real generated data: duplicate capture
converges to one immutable object version, scoped readback verifies the exact
checksum, and restart retains exactly one catalogue row. The final clean-home
gate is narrowed to composing that authority evidence with the already-proven
Monas login/monolith lifecycle in one run. Gallery admission additionally
required the capture upload to preserve its verified image media type rather
than the provider's generic binary default. Pinakotheke now passes its bounded
HTTPS ``image/*`` result through the native and container upload paths using
the validated DASObjectStore remote-client contract introduced at sibling
commit ``7a3d5578``. Live same-home readback still must prove that provider
metadata and authorized delivery return that exact type.

The next isolated composition run also removed a real acceptance-harness
blocker: DASObjectStore's local completion smoke still used the retired
``upload --store`` spelling. Sibling commit ``03f88237`` now uses the positional
Clap store argument. An exact-revision rebuild remains required because Docker
BuildKit subsequently reported an internal metadata database I/O error; that
infrastructure failure is not accepted as release evidence or bypassed with a
direct provider write.

- XIMG-050 implements the Mnemosyne-compatible Monas shell (commit `9e9cabb`):
  semantic-token CSS, compact header, responsive accessible empty state, and
  one mandatory footer provenance mark; host authentication remains Monas-owned.
- XIMG-051 implements source/account navigation (commit `ffdd275`): All, X,
  Instagram, and website contexts share one catalogue view with explicit
  selection and source counts.
- XIMG-056 began with client-side metadata search (commit `fd07323`). The
  XIMG-096 gallery now filters its persistent catalogue server-side by source,
  media, review, availability, time, and bounded text before stable pagination;
  Yew preserves the selected query while incrementally loading honest
  100-record pages. Its responsive overscanned viewport window bounds rendered
  cards and preserves roving Arrow/Home/End keyboard navigation across
  off-screen loaded records. The Yew crate now produces a runnable Trunk/WASM
  application served from the canonical app mount only after private Monas
  dispatch admission. Installed Firefox now passes a 1,000-record compiled-WASM
  component run covering bounded windowing, off-screen keyboard focus, server
  filtering, unavailable/no-origin behavior, and desktop/390px reflow. Native
  DEB/RPM/PKG targets now install that compiled application and embed their
  platform asset location in the monolith, eliminating a manual
  ``--web-root`` for package users while preserving validated development
  overrides. The monolith now also composes a strict host-owned object-reader
  executable with the authenticated web, catalogue, image/poster, and video
  range routes. Its bounded process protocol streams without a local payload
  file and verifies response metadata, length, process status, and full-read
  checksum; host/DAS authentication remains outside Pinakotheke. The packaged
  first-party helper now pins endpoint and ObjectStore-to-bucket mappings, uses
  host-owned AWS credentials, verifies completion checksum metadata, supports
  ranges, and deletes bounded private scratch. The live
  capture/commit/read/restart proof remains.
  Object identity now includes the explicit positive immutable version required
  by DASObjectStore provider streams. It survives verified acquisition,
  normalized-video evidence, catalogue restart, authorized read resolution, and
  helper dispatch; historic catalogue-v1 records migrate as version 1 without
  rewriting. The production helper no longer needs an unsafe version guess.
- XIMG-055 implements visible refresh progress (commit `a48fc13`) with a single
  action, per-account state, partial failure, retry, and new-item summary.
- XIMG-054 implements word-first review states and reversible batch actions
  (commit `84e11ca`), including a toggleable observed-thumbnail versus
  committed-ObjectStore-original distinction without mutating bytes.
- XIMG-053 implements the selected-record quick-preview task pane (this run):
  source/type/ObjectStore metadata, descriptive alt text, fit/original visual
  state, keyboard focus trap/return, explicit unavailable-object state, and a
  native video control only for a verified normalized ObjectStore range route.
  Synthetic proxy visuals retain no media payload and unavailable records never
  fall back to an origin URL.
- XIMG-069 implements direct normalized-video delivery (this run): a
  host-authenticated Axum route maps an actor-bound ready rendition only to a
  scoped DASObjectStore stream, retaining MIME/length/ETag/single-range and
  conditional behavior without an origin fallback. Real Firefox evidence uses
  a Docker-generated ephemeral normalized MP4 and proves metadata, range,
  seek, pause, and resume without retaining a fixture or browser profile.

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
- direct normalized-video playback is proven through the authenticated x-img
  delivery boundary before, and independently from, any optional Firefox cache
  substitution; and
- endpoint/device inventory and ObjectStore selection use accessible
  Mnemosyne tables/task panes, show endpoint and store together, and handle
  writable/read-only, health, capacity, pairing, TLS, and reconnect states
  without silently changing a reviewed destination; and
- no full administration form is permanently embedded in the browsing view.

The exit criteria above are component criteria. Stable-release acceptance also
requires XIMG-096 to prove the assembled Firefox-to-DASObjectStore-to-gallery
vertical with real ephemeral media, persistence across restart, explicit
unavailable states, and no origin fallback.

## 0.6.0 — Firefox site capture

Goal: allow users to add supported websites and capture viewed media into the
same x-img instance.

- XIMG-060 establishes the least-privilege Manifest V3 extension scaffold
  (commit `c7f317c`) with optional origin permissions and no credential/cookie
  access, automatic navigation, hidden traversal, or bulk crawling.
- XIMG-063 establishes explicit versioned site-adapter matching (commit
  `d5feaba`) with canonical origins, exclusions, capabilities, fixtures, and
  an opt-in experimental generic mode.
- XIMG-062 implements per-site Firefox policy controls (commit `bb14d50`) with
  exact origin disclosure, media classes, independent capture/substitution
  pause, and permission removal.

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

- XIMG-070 implements the bounded cache-alias lookup foundation (this run):
  immutable verified ObjectStore identity, observed-thumbnail versus
  explicitly-opened-original eligibility, server-owned same-instance/site/
  adapter/pairing policy, eviction/invalidation, stale/offline/unavailable
  origin-fallback states, query-free privacy, and a measured 4,096-entry p95
  below the 2 ms budget. XIMG-071/XIMG-072 consume its delivery metadata.
- XIMG-071 connects a reviewed image hit to one exact, reauthorized
  DASObjectStore stream and an explicit-site Firefox replacement. Production
  HTTPS/CORS/CORP/type/length/ETag headers are contract-tested; installed
  Firefox proves ephemeral display and CSP/CORS/metadata fail-open behavior.
- XIMG-072 extends that exact reviewed-object path to normalized MP4. Native
  Firefox owns streaming and decoding while Axum preserves authorization,
  conditional and single-range semantics; installed Firefox proves concurrent
  range, cancellation, seek, pause/resume, and origin fallback behavior.
- XIMG-073 makes segmented delivery fail closed by adapter evidence. The
  generic extension leaves HLS/DASH/MSE origin-served and reports a bounded
  reason; an exact adapter must prove canonicalization, Firefox behavior, no
  DRM/encryption, explicit open, and a matching Ready normalized profile.
- XIMG-074 adds the active-site Firefox control surface: explicit run,
  substitution pause/resume, source-view navigation, worded hit/miss/error,
  and accessible observed-versus-stored evidence backed by one bounded,
  URL-free diagnostic per configured origin.

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
- the versioned synthetic fault matrix and ``scripts/faults/check.sh`` provide
  the local release evidence for ingest, crash replay, destination stability,
  scheduler cancellation, normalization cleanup, authority loss, and Firefox
  substitution fail-open behavior;
- account refresh, extension capture, and cache serve share one scheduler and
  cannot create conflicting claims;
- schema migration, export, restore, and DASObjectStore loss/unavailability
  runbooks are tested;
- checksummed metadata snapshots round-trip without rewriting historic labels,
  endpoint/ObjectStore/object identities, or checksums; corruption, future
  majors, unknown fields, and silent Firefox re-pairing fail before mutation;
- platform deletion/compliance behavior matches the approved policy decision;
- catalogue-only and catalogue-plus-object compliance scopes require explicit
  policy approval; exact authority removal is separately requested, retryable,
  checksum-bound, and never reported complete before DASObjectStore verifies it;
- structured logs, metrics, health, and audit events contain no secrets;
- public liveness is coarse while authenticated operations expose bounded typed
  component states, aggregate counts, and fixed audit codes with no free-form
  request, browsing, credential, session, ObjectStore-key, or payload fields;
- dependency, license, vulnerability, and extension-permission audits pass; and
- ``scripts/audit/check.sh`` and its strict six-category matrix provide local
  audit evidence, with narrow reasoned transitive-Yew advisory exceptions and
  duplicate-generation warnings remaining explicit in ``deny.toml``;
- package/install documentation covers Monas, DASObjectStore, Firefox, and
  upgrades.
- the XIMG-085 packaging foundation builds the current CLI plus checked-in Monas
  bootstrap/license as native packages and deterministic Firefox bundles, with
  twelve-artifact checksum and typed release-manifest verification; Monas owns
  host composition/auth rather than x-img shipping a competing daemon, while
  signing/notarization and production-like install acceptance remain explicit
  XIMG-087 and XIMG-086 gates respectively;
- video jobs enforce bounded streaming, cancellation, resumable transfer where
  possible, quotas, backpressure, pinned containerized FFmpeg, scratch cleanup,
  crash reconciliation, and profile-version idempotency.

## 0.9.0 — Release candidate

Goal: freeze contracts and validate end-to-end behavior with user-owned data.

Published: ``v0.9.0`` is an explicitly unsigned GitHub prerelease with thirteen
verified artifacts, checksums, typed manifest, CycloneDX SBOM, local quality
evidence, and 0.3.0↔0.9.0 DEB/RPM rollback proof on both architectures. Its
release notes distinguish fixture/host boundaries and unsupported behavior from
evaluation-ready functionality.

Exit criteria:

- all supported connectors and site adapters pass end-to-end acceptance tests;
- performance targets are met for refresh, initial gallery load, scrolling,
  preview, range playback, and cache lookup;
- accessibility, security, privacy, and platform-policy reviews have no open
  release blockers;
- upgrade from the previous minor version preserves configuration, catalogue,
  object aliases, and review state; XIMG-086 proves genuine 0.2.0 → 0.3.0 →
  0.2.0 DEB/RPM transitions on x86_64 and arm64 with exact metadata and
  authority-identity preservation; and
- public documentation clearly distinguishes supported behavior, known limits,
  and non-goals, including the evidence-backed Firefox playback profile choice,
  rights gates, and local Sphinx container verification.

## 1.0.0 — Stable personal archive

Goal: stable public interfaces and dependable day-to-day operation.

Normative correction: the critical Firefox-capture-to-ThumbsPlus-gallery intent
defined in 0.5.0 and XIMG-096 is a mandatory v1 product criterion. Existing
published v1.0.0 packaging and identity evidence does not prove this functional
vertical and must not be used to waive it. Until XIMG-096 passes, the historical
tag remains published but the complete v1 product claim is not functionally
closed; the next stable release must carry the backfilled evidence.

Published: ``v1.0.0`` is the canonical Pinakotheke release at
https://github.com/sagrudd/pinakotheke/releases/tag/v1.0.0 with thirteen
verified artifacts, checksums, typed manifest, CycloneDX SBOM, compatibility
aliases, and complete local release evidence.

Cutover control: ``make v1-preflight`` inventories the exact coordinated
identity surface and names blockers safely during 0.9 development;
``make v1-cutover`` is the mandatory fail-closed gate and additionally verifies
the canonical public GitHub repository. It must pass before the v1 tag.
The canonical Monas product and scoped DASObjectStore principal now have
validated inert candidates; activation remains an authority-owned cutover step,
and legacy registrations plus historical object/audit identities stay intact.
The Firefox cutover candidate retains the shipped Gecko ID and least-privilege
surface; its executable upgrade proof preserves pairing, site opt-ins,
endpoint, and ObjectStore selection instead of resetting extension storage.
All package families, Firefox XPIs, SBOM, checksums, and typed artifact manifests
now support a version-locked canonical product mode while retaining x-img as the
0.9 default and v1 CLI compatibility alias.
The complete local identity transition is now an executable transaction that is
rehearsed in an isolated copy: the canonical Rust workspace compiles and tests,
reviewed authority and Firefox candidates activate, and strict cutover/package
gates pass without partially renaming the live 0.9 repository.
Real x86_64 and arm64 DEB/RPM transition rehearsals now move from the published
x-img 0.9 packages to temporary canonical Pinakotheke 1.0 packages and back,
while preserving the legacy CLI alias, canonical host identity, and byte-exact
metadata state.
The isolated cutover now passes the complete renamed local quality, security,
dependency/license audit, fault-recovery, public-contract, and packaging suites;
canonical builders use active authority/Firefox documents and default to the
Pinakotheke product rather than leaving release operations on candidate paths.

Exit criteria:

- no unresolved P0/P1 TODOs;
- XIMG-096 proves real Firefox image and normalized-video capture through the
  selected DASObjectStore into persistent, responsive ThumbsPlus-style cards,
  authorized preview/playback, review state, and restart recovery without
  synthetic media or origin fallback;
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

## 1.1.0 — Pinakotheke monolith and local-first service

Goal: make Pinakotheke useful as one coherent, locally runnable macOS service
without weakening the Monas authentication or DASObjectStore byte-authority
boundaries. ``pinakotheke-monolith`` is the distribution and process-composition
framework: users install and operate one product experience, while its embedded
or supervised components retain narrow authority interfaces.

The default development and per-user installation root is ``~/.x-img`` with
separate ``config``, metadata-only ``state``, ``run``, and ``logs`` directories.
A dedicated ``~/.x-img/dasobjectstore`` subtree may hold a DASObjectStore-managed
local development profile, but Pinakotheke must never treat that subtree as an
ordinary writable media folder. Secret DASObjectStore configuration and keys
remain in an OS-private configuration root such as
``~/.config/dasobjectstore``.

XIMG-091 is complete in ``30b18d1`` (core implementation ``7b5423a``) and consumes the
DASObjectStore ``0.84.0`` local-profile authority at ``0d71b2a1``. It provides
reviewed CLI orchestration, strict secret-free identity validation, stable
selection persistence, bounded authority diagnostics, and storage-aware
readiness. An isolated-home Docker proof exercised provision, rediscovery,
mode-``0600`` selection, monolith readiness, graceful shutdown, and retained
state after profile shutdown. The pinned local Sphinx container build and run
passed independently of hosted CI.

XIMG-092 is complete with a fail-closed backend ingress for Monas-authenticated dispatch:
a private process-local credential gates strict non-secret host context, direct
requests fail, and Pinakotheke never handles the browser cookie. Monas ``0.2.0``
at ``6e62943`` provides the canonical product registration, authenticated
streaming loopback mount, private process-credential configuration, strict
context injection, and registration/login/session/logout admission/revocation
proof.

XIMG-093 adds a tested non-root macOS launchd lifecycle for the separate
Pinakotheke backend and Monas host agents. Private credential creation,
transactional guarded replacement, status/restart/log access, and
data-preserving uninstall were exercised in an isolated home.

XIMG-094 is complete. A fresh isolated macOS home composed the packaged Monas
login with the Pinakotheke backend and a managed ``pinakotheke_local`` store at
DASObjectStore commit ``f195c4d5a30d1cc34ca61f31a6939edf54db782f``. The run
proved exact deep-link return, direct-backend rejection, component readiness,
daemon-verified synthetic commit, checksum-identical scoped read-back, restart
reconciliation and session continuity, logout revocation, and clean shutdown.
Managed profiles now accept an explicit loopback API port so this bounded proof
does not disturb an existing default-port authority.

XIMG-095 is complete in Monas ``0.3.0`` commit ``a0fabe2``. Its host-owned
login/Yew shell is informed by the DASObjectStore WebUI inspected at commit
``a93f0f872152d3790746292de1f3aec5d1a7bdd3`` while preserving the different
authority boundary: Monas, not Pinakotheke, owns all password and session
interaction. Strict same-origin return, login, admission, logout revocation,
invalid-target rejection, WASM compilation, and real-browser deep-link/login
rendering are proven.

Exit criteria:

- ``pinakotheke serve`` starts a coherent Axum/Yew service as the invoking user,
  binds loopback by default, refuses an unreviewed public bind, and uses a
  validated per-user root;
- startup reports component readiness for Pinakotheke, Monas authentication,
  and the selected endpoint plus logical ObjectStore without exposing secrets;
- local media writes and reads use scoped DASObjectStore application contracts,
  never direct filesystem access, even when the managed store is physically
  below ``~/.x-img/dasobjectstore``;
- the first-run flow provisions or discovers a named local ObjectStore, records
  stable endpoint/store IDs, and keeps configuration, catalogue metadata,
  runtime files, logs, credentials, and durable media in their correct
  authority domains;
- macOS supports foreground use and an optional per-user ``launchd`` service
  with status, log, upgrade, rollback, and non-destructive uninstall behavior;
- a managed object-read helper is bound to one reviewed stable endpoint
  identity in the backend service environment; path and identity are installed
  together and the identity is never treated as a credential or inferred from
  a browser request;
- foreground and managed-service modes can mount the metadata-only Firefox
  capture-plan endpoint behind Monas from a strict private pairing/site
  authority document, without accepting payload bytes or browser credentials;
- accepted Firefox plans survive restart in a bounded private atomic journal,
  retain actor scope and daily page budgets, and reconcile idempotent retries
  without claiming that pending metadata is stored media;
- only a separately credentialled host worker may convert a pending plan and
  independently verified DASObjectStore image reference into the persistent
  review queue and live gallery; browser sessions cannot assert completion;
- a run-one host worker can execute one pending image through a reviewed,
  shell-free, metadata-only helper protocol whose implementation owns permitted
  retrieval and direct DASObjectStore streaming and may not return payloads;
- foreground and managed-service modes can continuously schedule that helper
  with one process at a time, coalesced retries, prompt Firefox admission, and
  pending-on-failure semantics before verified live-gallery settlement;
- restart automatically requeues only still-authorized unsettled captures after
  pairing, expiry/revocation, site, adapter, and capture-kind revalidation,
  without requiring Firefox to repeat an observed-media request;
- the Firefox generic adapter works for explicitly opted-in HTTPS origins and
  records an original only from a trusted image-link or image-document click,
  never from synthetic events, unlinked thumbnails, or automatic navigation;
- exact-origin explicit-original observation remains active after Firefox
  restart without a repeated toolbar action and is removed immediately when
  capture is paused or the site permission is withdrawn;
- the packaged capture worker performs bounded HTTPS image acquisition through
  a scoped DASObjectStore remote-client session, accepts only daemon-verified
  completion, and leaves no durable Pinakotheke-local payload;
- a linked observed thumbnail and its distinctly addressed opened original
  share one server-derived catalogue identity while unrelated page images
  remain separate, including after journal restart;
- local authentication remains Monas/Prosopikon-owned; the monolith does not
  invent Pinakotheke passwords, cookies, or a parallel session issuer; and
- unauthenticated navigation presents a polished Monas-owned, Mnemosyne-design-
  compliant login/session screen with safe deep-link return, explicit
  loading/error/expiry/logout states, responsive keyboard-accessible behavior,
  approved brand assets, and no credential exposure to Pinakotheke; and
- a clean-home end-to-end test proves first start, login, ObjectStore selection,
  one synthetic ingest/read/restart reconciliation, and clean shutdown without
  root privileges or hosted CI.

## 1.2.1 — Runnable normalized-video authority path and package dependency

Goal: close the remaining XIMG-096 production-video gap by carrying one
confirmed user-selected video through a reviewed normalization worker, the
selected DASObjectStore, persistent gallery admission, and real Firefox
playback. The first slice packages the strict host normalization command and
stdin-streaming authority helper. The remaining release evidence must use a
registered digest-pinned image and live selected ObjectStore, then prove poster
rendering, play, seek, pause/resume, restart persistence, and unavailable or
partial-failure behavior without origin fallback.

Pinakotheke native packages require a separately installed DASObjectStore.
DEB/RPM express that relationship through native dependency metadata and macOS
PKG through an explicit installation prerequisite. No Pinakotheke artifact may
embed DASObjectStore binaries, services, configuration, credentials, or object
data; DASObjectStore retains its independent lifecycle and storage authority.
XIMG-097 completed this boundary in ``c32241a`` with inspected DEB, RPM, and
macOS PKG evidence.

## 1.2.2 — Branded signed Firefox patch

Goal: replace Firefox's generic puzzle-piece fallback with the approved
black-and-white Mnemosyne Biosciences mark across the toolbar, Add-ons Manager,
and installation prompt. The patch retains the signed extension identity,
uses aspect-preserving transparent icon derivatives at Firefox-native sizes,
and requires a new Mozilla unlisted signature before deployment.

XIMG-102 completed in ``67980e2`` on 2026-07-17. The signed artifact passed
Mozilla envelope, stable-identity, version, archive-icon, and permanent Firefox
installation checks. The checksum-identical DASServer copy is deployed over
trusted HTTPS with the required XPI media type and no-store headers.

The deployed application itself is also a release gate: Monas must forward the
built Pinakotheke HTML, JavaScript, CSS, and WASM rather than an empty product
shell. The library must visibly discover the authenticated DASObjectStore
inventory and offer an endpoint/ObjectStore selector before capture review.
Selection is not commit authority; the backend revalidates the exact stable
endpoint/store pair immediately before every write. XIMG-098 tracks persistent
selection and capture-plan consumption after the first visible deployed slice.

Private-LAN Firefox deployments may use a locally trusted CA without any public
DNS or external certificate service. The v1 operational gate requires TLS on
the configured instance URL, a leaf certificate containing the exact LAN IP or
hostname, a loopback-only Monas upstream, no CA private key on the server, and
real Firefox verification with insecure-certificate overrides disabled.

Legitimate Firefox distribution uses Mozilla's unlisted signing channel. The
stable Gecko ID, accurate built-in data-transmission consent, pinned local
``web-ext`` validation, environment-only publisher credentials, returned-XPI
signature verification, and ordinary-profile installation are release gates.
Unsigned XPIs remain development artifacts and temporary installation is not
production acceptance. XIMG-100 tracks the one-time AMO publisher submission
and installed signed-build proof.

XIMG-100 completed on 2026-07-17: Mozilla approved the unlisted ``1.2.1`` XPI,
the returned signature, identity, and version verified, and Firefox accepted
it as a permanent extension in an isolated ordinary profile. The live
trusted-HTTPS DASServer download is checksum-identical and uses the required
XPI media type.

The Monas-owned sign-on is part of the Pinakotheke user experience and must be
product-aware without transferring authentication authority. A validated
Pinakotheke return path selects the archive purpose, product sign-in label, and
product-plus-Monas footer identity inside the common Mnemosyne shell. Unknown
or unsafe destinations retain generic Monas presentation. XIMG-101 completed
this gate in Monas ``0.6.2`` (``114ef95``) and the DASServer deployment.

## 1.3.0 — Authenticated Firefox onboarding

Goal: make the Pinakotheke web application the sole onboarding authority for
the Firefox extension. DASObjectStore availability, a reviewed named writable
ObjectStore, Monas authentication, and an actor-bound pairing are ordered
prerequisites. The authenticated page distributes the current Mozilla-signed
XPI and pairing payload; Firefox verifies that live payload before saving the
server relationship. Exact website origins and X-ingress intent remain
separate explicit browser choices. XIMG-103 tracks the live DASServer proof.

Compatibility-sensitive references: Mnemosyne design language
``5539df8f662a78ebdf7cf4c868d71831380c8cfd``, Monas
``114ef95b6c0e001c0c167dcea324674c57ae6197``, DASObjectStore
``df452a5535f378ccf2b856d8d040b0c2659559a7``, and Mnemosyne
``52810176bf95a170f93d74a6f5daa94da5c6640e``.

## 1.4.0 — Persistent site corpus

Goal: make exact-origin import definitions durable actor data. XIMG-105
delivered authenticated revisioned server persistence, conflict-safe Firefox
synchronization, bounded deletion tombstones, and credential-free recovery.

## 1.5.0 — Light-touch automatic cache

Goal: reduce the Firefox interaction to defining an exact-origin import rule
and browsing normally. The rule is standing capture consent: visible
thumbnails are cached automatically, opened images acquire their originals,
and opened/played videos are acquired when selected by the rule. Successful
verified DASObjectStore commits appear immediately as ``New`` gallery cards;
review is catalogue organisation after capture, never an ingest gate. Firefox
marks material served by or verified in ObjectStore with a two-pixel green
frame without modifying media bytes.

The authenticated Pinakotheke service owns a versioned actor-scoped corpus of
site definitions so signed extension updates, replacement Firefox profiles,
and additional paired devices do not discard user configuration. Local rules
remain available for low-latency browsing and reconcile explicitly with the
server. Shared immutable payloads may deduplicate across users, while rule,
observation, provenance, and review state remain actor-specific. X objects use
the logical namespace and gallery grouping ``x.com/<canonical-account>/...``;
this is not permission to bypass DASObjectStore with filesystem subfolders.
XIMG-104 and XIMG-105 are release gates for this simplified metaphor.

## 1.6.0 — Observable automatic ingress

Goal: make ordinary browsing capture immediately understandable and repair the
linked-image acquisition boundary. The authenticated Pinakotheke library shows
live counts for observed thumbnails, explicitly opened images/videos, pending
plans, verified ObjectStore commits, and admitted gallery cards. Linked X
images use the bytes Firefox rendered while retaining the status-page URL only
as provenance, so successful image commits can flow directly into the gallery.
XIMG-104 remains open until opened video completes the equivalent normalized
ObjectStore and gallery path.

## 1.7.0 — Diagnosable ingress

Goal: make every enabled-page capture explainable across Firefox, Pinakotheke,
the acquisition helper, DASObjectStore verification, and gallery admission.
The extension provides a bounded redacted event log and JSON export; the
service journal records plan admission and terminal worker outcomes without
source URLs or secrets. A live failure must identify its exact stage rather
than remaining indefinitely and opaquely ``pending``.

## 1.7.1 — Safe X variant reconciliation

Goal: settle an acquired X image whose canonical ``pbs.twimg.com`` alias keeps
the strictly validated ``format`` and ``name`` variant. Reconciliation reuses
the admission canonicalizer and continues to reject arbitrary queries, signed
parameters, fragments, credentials, and non-canonical parameter ordering.

## Post-1.0 candidates

- Synoptikon catalogue/plugin integration through the preserved host adapter
  is delivered for 1.1 development by XIMG-200 as a project-scoped read-only
  projection; deployment catalogue enrollment remains host-owned;
- additional explicitly approved site adapters;
- perceptual duplicate grouping without weakening exact source provenance;
- user-defined collections, tags, and saved searches; and
- derivative thumbnails or transcoding, stored as separate DASObjectStore
  objects with immutable parent provenance.
