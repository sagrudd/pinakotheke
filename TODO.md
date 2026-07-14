# x-img TODO

Status: dependency-ordered planning backlog

Version: 0.1.0

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

- [ ] **XIMG-003 P0 — Inventory sibling contracts.**
  Pin the relevant Monas product mount/session contract, DASObjectStore
  application-auth/upload/read/range contracts, Mnemosyne design-language
  revision, and future Synoptikon product adapter contract.
  Acceptance: compatibility matrix names versions/commits and contract fixtures;
  no public build requires unpublished path dependencies.

- [ ] **XIMG-004 P0 — Write architecture decisions.**
  Create ADRs for authority boundaries, local metadata versus media bytes,
  idempotent acquisition, canonical source identity, review lifecycle, account
  refresh scheduling, extension pairing, and external-cache fail-open behavior.
  Acceptance: each ADR includes alternatives, failure modes, privacy impact,
  compatibility impact, and acceptance tests.

- [ ] **XIMG-005 P0 — Define versioned configuration.**
  Draft strict JSON schemas for one x-img instance, X accounts, Instagram
  accounts, and website policies. Keep secrets as host-managed references.
  Acceptance: examples cover enabled/disabled sources, per-source media policy,
  refresh budget, review defaults, and schema rejection of unknown fields.

- [ ] **XIMG-006 P0 — Define acquisition and catalogue schemas.**
  Specify source post/item, media identity, object reference, download attempt,
  job lease, account cursor, review state, tombstone, and audit event.
  Acceptance: state diagrams cover retry before/after DAS commit, duplicate URLs,
  URL rotation, platform ID reuse assumptions, and reconciliation.

- [ ] **XIMG-007 P0 — Prove the Firefox architecture on paper and fixtures.**
  Define how the extension detects/captures responses and substitutes cached
  content for images, MP4, range requests, and HLS/DASH without credential
  capture or mixed-content failure.
  Acceptance: spike matrix covers WebRequest/DNR, response filtering, local
  HTTPS, CORS/CSP/CORP, redirects, signed URLs, and fail-open behavior; uncertain
  cases are isolated behind explicit site capabilities. It must also prove no
  automatic opening, hidden traversal, bulk crawling, simulated browsing,
  cookie/credential forwarding, or API-avoidance policy loophole; thumbnails
  are eligible only after actual display/observation and originals only after
  explicit user open.

- [ ] **XIMG-008 P1 — Establish release and quality policy.**
  Add changelog, SemVer rules, supported Rust/MSRV and Firefox versions, CI
  matrix, dependency policy, fixture privacy rules, and Definition of Done.
  Acceptance: version sources cannot drift; the Definition of Done requires
  precise Sphinx/Read the Docs user documentation and a reproducible local
  `docs/Dockerfile` container build/verification that is authoritative over
  any GitHub Actions mirror; and CI checks planning links/schemas.

- [ ] **XIMG-009 P0 — Plan the Pinakotheke v1 identity migration.**
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

- [ ] **XIMG-020 P1 — Scaffold the Rust workspace.** Depends on XIMG-003/004/008.
  Add shared model, core, CLI, API adapter, and Yew crates with workspace-linted
  dependencies and MPL notices. Acceptance: native and wasm checks pass; CLI
  reports `0.2.0`; no live source or storage code exists yet.

- [ ] **XIMG-021 P1 — Implement strict account/site config.** Depends on XIMG-005.
  Parse, validate, atomically update, and list versioned JSON without secrets.
  Acceptance: duplicate accounts/origins, invalid handles, unknown fields,
  unsafe wildcard defaults, and incompatible schema majors are tested.

- [ ] **XIMG-022 P0 — Implement acquisition state machine.** Depends on XIMG-006.
  Acceptance: property/unit tests prove valid transitions and reject double
  claims, commit-before-verification, and review-before-object-commit.

- [ ] **XIMG-023 P0 — Implement idempotency and reconciliation.**
  Use platform media identity plus immutable object checksum; keep aliases from
  canonical source identities. Acceptance: crash-injection fixtures at every
  state converge to one committed catalogue record and never overwrite bytes.

- [ ] **XIMG-024 P1 — Implement job scheduler contracts.**
  Model global refresh, per-account jobs, extension capture, leases, cancellation,
  cost/rate budgets, and bounded concurrency. Acceptance: concurrent refresh
  presses coalesce safely and per-source work never overlaps.

- [ ] **XIMG-025 P1 — Add deterministic connector fixtures.**
  Cover pagination, edits, deleted/inaccessible items, duplicate media, multiple
  variants, rate limits, token expiry, malformed responses, and cursor reset for
  X and Instagram.

- [ ] **XIMG-026 P0 — Define explicit bioinformatics transfer plans.** Depends
  on XIMG-006/008. Accept one bounded, user-identified GEO, SRA, ENA, or NCBI
  accession/URL and render a reviewable plan containing authority/release,
  files, sizes/checksums, transport, endpoint/ObjectStore/prefix/type,
  estimated bytes, licensing/usage, and policy result before any transfer;
  reject bulk discovery and implicit repository crawling.

- [ ] **XIMG-027 P0 — Add bioinformatics resolution and transport fixtures.**
  Depends on XIMG-026. Cover ENA/SRA manifests and multi-run expansion, GEO
  metadata/raw archive/file lists, NCBI routing, checksum/size evidence,
  retries/resume/cancellation, bounded backpressure, and optional Aspera with
  HTTPS fallback, without real credentials or copyrighted payloads.

## 0.3.0 — External authorities

- [ ] **XIMG-030 P0 — Define and register the Monas product.**
  Set mandatory authentication, single Web/API mount, object-store requirement,
  product root, capability list, and future Synoptikon-equivalent bootstrap.
  Acceptance: unauthenticated app/API access fails; direct x-img login routes do
  not exist.

- [ ] **XIMG-031 P0 — Implement authenticated host-context adapter.**
  Acceptance: Monas session identity and authorization are validated at the host
  boundary, never copied into logs/config, and can be replaced by a Synoptikon
  adapter in contract tests.

- [ ] **XIMG-032 P0 — Register scoped DASObjectStore application identity.**
  Request only read/write/list/verify access to one configured ObjectStore/prefix
  with short-lived credentials or capabilities. Acceptance: expired, replayed,
  wrong-store, wrong-prefix, and oversized operations fail closed.

- [ ] **XIMG-033 P0 — Implement streaming object ingest port.**
  Acceptance: checksum and exact length are verified; completion is idempotent;
  backpressure is honored; local temp files cannot become durable media stores.

- [ ] **XIMG-034 P0 — Implement authorized object read/cache port.**
  Acceptance: content type, length, checksum, ETag, conditional GET, byte ranges,
  and object-unavailable errors match DASObjectStore contracts.

- [ ] **XIMG-035 P1 — Add cross-repository contract CI.**
  Test pinned fixtures without requiring sibling checkouts; optionally run live
  integration when sibling repositories are available.

- [ ] **XIMG-036 P0 — Implement confirmed direct bioinformatics commit.** Depends
  on XIMG-023/024/026/027/033. After user confirmation and server-side policy
  and capability revalidation, stream directly to DASObjectStore, verify
  checksum and exact length before catalogue commit, reconcile crashes
  idempotently, deduplicate by accession/file identity plus checksum, and
  preserve source, transport, rights, and destination provenance. No durable
  x-img-local payload or silent local fallback is allowed.

## 0.4.0 — Social account acquisition

- [ ] **XIMG-040 P0 — Implement official X OAuth adapter.**
  Request only required read/offline/follow scopes through a Monas-managed
  integration flow. Acceptance: state/PKCE/token refresh/revocation tests pass;
  protected access is used only when authorized by the viewing account.

- [ ] **XIMG-041 P1 — Import/select followed X accounts.**
  Let the user explicitly select from permitted followed accounts into the local
  JSON allowlist; never silently follow every account. Acceptance: import is a
  task pane and config diff is reviewable before save.

- [ ] **XIMG-042 P0 — Implement incremental X media discovery.**
  Support photos, videos, GIFs, pagination, permitted timeline depth, and best
  supported variant. Acceptance: fixture parity, budget enforcement, complete
  provenance, and XIMG-023 idempotency pass.

- [ ] **XIMG-043 P0 — Research and implement approved Instagram auth/API.**
  Freeze implementation to the capabilities permitted by XIMG-002. Acceptance:
  unsupported account/media classes are explicit and no browser credential or
  scraping fallback is introduced implicitly.

- [ ] **XIMG-044 P0 — Implement incremental Instagram media discovery.**
  Acceptance: supported posts/carousels/videos, pagination, provenance, budgets,
  token lifecycle, and XIMG-023 idempotency pass.

- [ ] **XIMG-045 P0 — Implement one-click account refresh.**
  One authenticated action schedules all enabled X and Instagram accounts,
  coalesces repeated clicks, and returns a global job with per-account children.
  Acceptance: live progress, partial failure, cancellation, retry, new-item
  count, and no-overlap behavior are tested.

- [ ] **XIMG-046 P1 — Implement new-item review admission.**
  Mark media `new` only after verified object commit; keep source/account grouping
  and discovery time. Acceptance: interrupted jobs never expose broken new cards.

## 0.5.0 — Web library and review

- [ ] **XIMG-050 P1 — Implement Mnemosyne-compliant Monas shell.**
  Import the canonical semantic tokens and approved brand assets; add compact
  header and mandatory footer. Acceptance: no literal component palette values,
  one decorative mark, responsive and AA-compliant shell.

- [ ] **XIMG-051 P1 — Implement source/account navigation.**
  Show X, Instagram, and website sources resolving to the same catalogue, with
  visible selected context and counts.

- [ ] **XIMG-052 P1 — Implement dense virtualized thumbnail browser.**
  Include keyboard traversal, adaptive thumbnail size, stable selection,
  thumbnail lazy loading, and fast filters. Acceptance: performance targets are
  documented and met against a large synthetic catalogue.

- [ ] **XIMG-053 P1 — Implement quick preview and video playback.**
  Acceptance: original/fit view, metadata, alt text, source link, range playback,
  focus trapping/return, and unavailable-object state pass.

- [ ] **XIMG-054 P1 — Implement review queue and batch review.**
  Filters and words distinguish new/reviewed/hidden/removed; actions are scoped
  and undoable where feasible. Acceptance: colour is never the sole signal;
  `Previously observed` thumbnail status is distinct from `Stored in ObjectStore`
  committed-original status using accessible words/iconography, colour,
  tooltip, a reversible non-obstructive frame/badge/overlay, and a user toggle;
  stored bytes are never watermarked or mutated.

- [ ] **XIMG-055 P1 — Implement refresh controls and progress.**
  Surface XIMG-045 as one `Refresh accounts` action with per-account progress,
  partial-failure details, safe retry, and final new-item summary.

- [ ] **XIMG-056 P2 — Add browse/search metadata filters.**
  Account, platform, website, media type, discovered/posted time, dimensions,
  review state, and object availability.

## 0.6.0 — Firefox capture

- [ ] **XIMG-060 P0 — Scaffold least-privilege Firefox extension.**
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

- [ ] **XIMG-063 P1 — Implement site-adapter registry.**
  Version matching, canonicalization, exclusions, capabilities, and fixtures.
  Start with explicit adapters; generic mode remains experimental and opt-in.

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
  Unsupported streams remain origin-served and visibly diagnosed.

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
