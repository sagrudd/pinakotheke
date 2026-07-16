# Changelog

All notable changes to Pinakotheke will be documented in this file. The project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Add a packaged ``pinakotheke video normalize`` host command that accepts one
  private confirmed plan, runs the digest-pinned network-isolated Docker/FFmpeg
  adapter, streams video/poster/manifest bytes to a reviewed DASObjectStore
  helper over stdin, verifies exact receipts, kills unfinished children, and
  removes bounded scratch on every outcome.
- Allow each managed local profile lifecycle command to select an explicit
  loopback DASObjectStore API port while retaining port 3900 as the default,
  and strictly bind discovery to the requested authority.
- Complete the isolated XIMG-094 clean-home proof across Monas login, exact
  return, direct-backend rejection, DASObjectStore readiness, verified commit
  and scoped read-back, restart reconciliation, logout, and clean shutdown at
  DASObjectStore ``f195c4d5``.
- Add a strict local-Docker capture transport that maps only private
  DAS-managed scratch into the fixed authority service, supplies scoped
  credentials by private file path, and removes the job directory on every
  outcome.
- Record the live authority defects resolved by DASObjectStore ``720ae9c1``:
  isolated port alignment and canonical folder-profile capacity binding.
- Prove duplicate container-side captures, scoped checksum-verified readback,
  and restart persistence against DASObjectStore ``b35ee0b2``; record the
  remaining same-home Monas composition and media-type preservation gates.
- Preserve the capture helper's verified image MIME type through native and
  container uploads using DASObjectStore's validated single-object content-type
  contract at sibling commit ``7a3d5578``.
- Resume the isolated XIMG-094 authority proof, fix DASObjectStore's obsolete
  completion-smoke upload syntax at sibling commit ``03f88237``, and retain the
  exact-revision rerun gate after Docker BuildKit reported a metadata I/O fault.

- Record and validate the DASObjectStore container-side remote-completion
  prerequisite that removes Docker Desktop bind-mounted Unix sockets from the
  clean-home authority path.

- Add the packaged first-party DASObjectStore read helper with explicit
  endpoint/ObjectStore-to-bucket authority, host-owned AWS credentials,
  completion-metadata verification, conditional and ranged reads, and bounded
  automatically removed scratch.

- Add isolated installed-Firefox acceptance for an observed linked thumbnail
  and trusted explicitly opened original through the production capture path.

- Define XIMG-096 as the non-waivable Firefox-to-DASObjectStore-to-gallery
  stable-release gate and add its first bounded, Monas-authenticated gallery
  catalogue boundary with explicit object availability and no origin fallback.

- Replace the Yew gallery's synthetic cards and proxy visuals with the canonical
  Monas-forwarded catalogue, real authorized media elements, source-derived
  filters, and explicit safe failure states.
- Persist the Monas gallery's metadata-only ObjectStore references in a strict,
  bounded, private, atomically replaced document loaded by the local monolith,
  with fail-closed schema/corruption handling and restart coverage.
- Bridge verified Firefox image acquisitions through website provenance and the
  common review queue into persistent gallery cards, with idempotent thumbnail
  admission and independently committed explicit-original enrichment.
- Resolve persistent gallery image paths under private Monas dispatch and
  stream only the exact catalogue-authorized DASObjectStore object through a
  host backend with strict MIME, length, checksum, and ETag validation.
- Persist only Firefox-proven ready normalized videos as poster-backed gallery
  cards and deliver their exact rendition through authenticated, validated
  single-range playback without local bytes or origin fallback.
- Add bounded server-side persistent-gallery filters, honest matched/total
  counts, and query-preserving incremental Yew pagination instead of silently
  truncating the real library at 200 cards.
- Window the responsive persistent gallery to visible plus overscan rows while
  retaining virtual scroll height and roving Arrow/Home/End keyboard traversal
  across off-screen loaded records.
- Add the runnable Trunk/WebAssembly gallery build and serve its bounded,
  symlink-free output from the canonical application mount only through private
  Monas dispatch admission.
- Add installed-Firefox acceptance for the compiled Yew gallery with 1,000
  mixed records, bounded windowing, off-screen keyboard focus, server filters,
  unavailable/no-origin behavior, and desktop/narrow responsive reflow.
- Install the compiled Yew/WASM gallery in Linux and macOS native packages and
  embed its platform path in the monolith, retaining validated per-user and
  explicit development overrides.
- Compose a strict process-isolated host object-reader into the packaged
  monolith, with bounded streaming, exact metadata/length/full-read checksum
  verification, range support, and no local payload or credential handling.
- Bind a managed macOS object-read helper to one explicit reviewed endpoint
  identity, reject half-configured installs, and pass the fixed non-secret scope
  through the backend agent environment instead of deriving it from requests.
- Mount the Firefox capture-plan API in the runnable Monas-authenticated
  monolith from a strict private metadata-only pairing/site authority document,
  including per-user launchd composition and fail-closed validation.
- Route Firefox capture planning and gallery navigation through the canonical
  Monas Pinakotheke product mounts instead of obsolete standalone paths.
- Persist accepted Firefox capture plans in a bounded private atomic journal,
  restore actor-scoped pending work and daily budgets after restart, return
  idempotent retries, and expose an authenticated pending-plan reconciliation
  view without media bytes or credentials.
- Add a separately credentialled host-worker completion boundary that admits
  exact verified DASObjectStore image evidence through acquisition,
  reconciliation, persistent review, and the live gallery, with restart-safe
  idempotent settled markers and no payload bytes.
- Add a run-one capture acquisition command and strict executable protocol for
  host-owned permitted retrieval, direct DASObjectStore streaming/verification,
  and metadata-only settlement without shell execution or local payload files.
  Foreground/service and offline workers share an exclusive local mutation
  lease so long-running acquisition cannot overlap journal writes.
- Compose the reviewed acquisition helper into foreground and launchd monolith
  operation with prompt browser admission, serialized background execution,
  per-plan retry coalescing, live verified settlement, and pending-on-failure.
- Recover durable unsettled captures automatically on monolith startup after
  revalidating current actor pairing, expiry/revocation, site policy, adapter,
  and capture-kind authority; withdrawn work remains pending and untouched.
- Enable the experimental generic Firefox adapter for explicitly opted-in HTTPS
  origins and admit originals only from trusted image-link/image-document
  clicks, using sender-tab provenance and no automatic opening or traversal.
- Persist the trusted explicit-original observer as a Firefox dynamic content
  script scoped to enabled image origins, restoring it on startup/update and
  unregistering it immediately when capture is paused or permission removed.
- Add a first-party packaged capture helper that bounds HTTPS image retrieval,
  uses private ephemeral scratch, submits checksum-addressed uploads through
  DASObjectStore's daemon completion path, and emits verified metadata only.
- Correlate linked observed thumbnails with distinctly addressed opened
  originals through a canonical presentation identity owned by the capture
  plan, with non-destructive legacy-journal fallback.
- Carry an explicit positive immutable object version from verified acquisition
  and normalized-video evidence through persistent gallery records, authorized
  read grants, and host-helper requests, with legacy catalogue-v1 records
  defaulting non-destructively to version 1.
- Add the XIMG-200 Synoptikon product manifest, strict tenant/account/project
  host scope, and bounded authenticated catalogue projection over immutable
  DASObjectStore references.
- Add the first XIMG-090 ``pinakotheke serve`` monolith slice with a private
  per-user metadata root, loopback-safe Axum listener, graceful shutdown, and
  honest component readiness.
- Add XIMG-091's reviewed local DASObjectStore profile plan, authority-owned
  provisioning and discovery, stable secret-free destination selection, and
  honest monolith storage readiness, with bounded actionable authority failure
  diagnostics and an isolated-home Docker acceptance proof.
- Add XIMG-092's first fail-closed Monas dispatch ingress: a private
  process-local credential admits only strict non-secret host context, strips
  dispatch headers before product handling, and leaves browser sessions wholly
  owned by Monas/Prosopikon.
- Complete XIMG-092 against Monas ``0.2.0`` with the authenticated canonical
  product mount, streaming loopback forwarding, cookie stripping, strict host
  context injection, and session revocation proof.
- Add XIMG-093's non-root macOS launchd lifecycle for the coordinated
  Pinakotheke backend and Monas host, including private credential generation,
  guarded transactional replacement, status/restart/log access, and
  data-preserving uninstall.
- Complete XIMG-095 with the Monas ``0.3.0`` host-owned login/Yew shell,
  same-origin Pinakotheke deep-link return, Prosopikon session lifecycle, and
  approved Mnemosyne branding asset boundary.

## [1.0.0] - 2026-07-15

- Add an executable Pinakotheke v1 preflight and no-partial-cutover release
  gate that reports canonical identity blockers without exposing private data.
- Prepare one shared clap implementation for the canonical ``pinakotheke``
  command and behavior-preserving, warning-emitting ``x-img`` legacy alias;
  0.9 packages continue to install only the legacy command.
- Add validated, inert Pinakotheke 1.0 Monas and DASObjectStore registration
  candidates while preserving the active x-img registrations and historical
  authority identity.
- Preserve Firefox pairing, site policy, endpoint, and ObjectStore state across
  extension updates, and add a canonical manifest candidate retaining the
  shipped Gecko update identity and least-privilege permission surface.
- Parameterize native, Firefox, SBOM, checksum, and artifact-manifest packaging
  for a version-locked Pinakotheke cutover while retaining x-img as the 0.9
  default and future CLI compatibility alias.
- Add an isolated, compile-tested rehearsal of the coordinated Pinakotheke 1.0
  identity cutover while leaving the live 0.9 repository unchanged.
- Prove dual-architecture DEB/RPM transition from x-img 0.9.0 to canonical
  Pinakotheke 1.0.0 and rollback with exact metadata preservation.
- Extend the isolated v1 cutover rehearsal through the complete renamed
  quality, audit, fault-recovery, contract, and canonical packaging toolchain.
- Complete the coordinated Pinakotheke identity migration across the Rust
  workspace, active authority contracts, Firefox identity, packaging defaults,
  documentation, and canonical public repository while retaining compatibility.

### Fixed

- Reconcile a repeated local-profile start only when DASObjectStore can still
  rediscover the exact reviewed Ready endpoint/ObjectStore identity, allowing
  safe recovery from an already-running nested provider without accepting a
  changed or unavailable authority.

- Declare both Firefox ``background.scripts`` and cross-browser
  ``background.service_worker`` in extension manifests, and generate valid
  port-independent WebExtension match patterns for non-default-port HTTPS
  origins while retaining the exact origin in policy and provenance.

## [0.9.0] - 2026-07-15

### Added

- Added a deterministic CycloneDX 1.6 release SBOM covering locked Rust
  dependencies and the Firefox extension component.
- Published 0.9.0 as an explicitly unsigned evaluation release candidate with
  local package, rollback, audit, contract, and documentation evidence.

## [0.3.0] - 2026-07-15

### Planning

- Added XIMG-086's local package lifecycle and metadata rollback acceptance
  across genuine 0.2.0/0.3.0 DEB and RPM transitions on x86_64 and arm64 using
  digest-pinned Debian/Fedora containers and pinned Monas plus DASObjectStore
  contracts.
- Added the XIMG-085 packaging foundation: a Makefile for cross-linked Linux
  DEB/RPM, dual-architecture macOS PKG, deterministic six-label Firefox XPI,
  SHA-256 manifests, a deterministic typed release-artifact inventory, and
  strict source/artifact verification.
- Added XIMG-084's single-command local release audit covering privacy,
  security, accessibility, extension permissions/CSP, licenses, locked Rust
  dependencies/advisories/sources, JavaScript syntax, and version mirrors.
- Added XIMG-083's bounded redacted operations model and Axum surfaces. Public
  health is coarse; host-authenticated snapshots expose only typed component
  state, saturating aggregate counters, fixed audit codes, and eviction count.
- Added XIMG-082's approved deletion/compliance reconciliation contract.
  Catalogue tombstoning is separate from exact DASObjectStore removal,
  approval scope is mandatory, pending retries converge, mismatched authority
  evidence conflicts, and only verified removal becomes terminal.
- Added XIMG-081's strict metadata snapshot boundary and native migration,
  export, and restore proofs. Checksummed copy-on-write artifacts preserve
  catalogue/ObjectStore identities, reject corruption and future schemas, and
  require explicit reviewed Firefox re-pairing without containing secrets or
  media bytes.
- Added XIMG-080's deterministic fault/recovery acceptance suite. Nine
  synthetic cases prove bounded ingest, idempotent crash reconciliation,
  destination stability, scheduler cancellation, normalizer cleanup, cache and
  capture authority failures, and real-Firefox substitution fail-open behavior.
- Added XIMG-074's Firefox cache popup with per-site pause/resume, explicit run,
  current hit/miss/error reason, host source-view link, clear permission text,
  and accessible ``Previously observed`` versus ``Stored in ObjectStore``
  evidence. Diagnostics are one bounded URL-free record per configured origin.
- Added XIMG-073's fail-closed segmented-video adapter gate. HLS/DASH requires
  exact versioned manifest/segment canonicalization, synthetic fixtures, real
  Firefox evidence, explicit-open evidence, no DRM/encryption, and a matching
  Ready normalized profile. Generic manifest/MSE sources stay origin-served
  with a bounded URL-free Firefox diagnostic.
- Added XIMG-072 normalized MP4 external-cache delivery. Exact reviewed video
  records support authenticated full, conditional, and single-range streams;
  the opted-in Firefox path uses native video playback and restores the origin
  once on failure. Real Firefox proves concurrent ranges, cancellation, seek,
  pause/resume, conditional requests, and fallback with ephemeral media.
- Added XIMG-071's host-authenticated image substitution path. Stable delivery
  IDs preserve the exact reviewed ObjectStore identity; the Firefox client
  validates MIME, length, checksum ETag, size, CORS/CORP delivery, and restores
  the origin once on any failure. A real Firefox loopback harness proves
  replacement and CSP/CORS/type/length/ETag fail-open without retained media.
- Added XIMG-070's bounded cache-alias index and host-authenticated lookup
  endpoint. Immutable ObjectStore hits require server-owned same-instance,
  origin, adapter, pairing, expiry, substitution, and observation/open gates;
  signed queries are rejected without echo, authority failures explicitly fall
  back to origin, and the 4,096-entry synthetic p95 is measured below 2 ms.
- Added XIMG-053's Mnemosyne-aligned quick preview task pane. It provides
  selected-record context, alt text, fit/original visual state, keyboard focus
  handling, explicit unavailable-object behavior, and only attaches native
  video controls to a ready normalized ObjectStore playback route—never an
  origin fallback or browser-retained payload.
- Completed XIMG-069's direct authenticated normalized-video delivery route
  and Firefox proof. Ready, actor-bound ObjectStore grants preserve
  single-range, MIME, length, ETag, conditional, and unavailable semantics
  without origin fallback or a dependency on later Firefox cache substitution.
  The local Firefox harness uses only an ephemeral normalized file and cleans
  its browser profile after verifying metadata, range, seek, pause, and resume.
- Added XIMG-068's paired-device Docker FFmpeg adapter. It requires an
  approved pairing, digest-pinned image, resource limits, isolated scratch,
  structured network-disabled invocation, bounded ObjectStore ingest, probe
  and checksum provenance, cleanup, cancellation, and crash-reconciliation
  states. It leaves every rendition Awaiting Firefox playback rather than Ready.
- Added XIMG-067's two immutable normalized-video candidate profiles: WebM
  VP9/Opus or AV1/Opus and MP4 H.264/AAC. Docker-first plans pin an image digest,
  resource limits, authorized DASObjectStore/paired-device/future-Keryx worker
  placement, and managed-or-bounded scratch. A rendition cannot become Ready
  without typed checksummed ObjectStore derivatives and matching Firefox
  profile evidence; source-only objects remain explicitly non-ready.
- Added XIMG-066's explicit, metadata-only video candidate planner. Observed
  or user-selected, policy-eligible candidates expose review details and need
  confirmation before later work; DRM, unsupported segmented delivery,
  unobserved candidates, and non-video destinations are blocked. It records
  aggregate origin/container/codec occurrence gaps for normalization priority
  without publishing media, URLs, cookies, credentials, or browsing history.
- Added XIMG-065's verified website-capture provenance handoff to the shared
  review queue. It records site/page/media/adapter evidence only after a
  verified ObjectStore commit, rejects premature or mismatched acquisitions,
  and reuses a matching committed connector identity rather than creating a
  duplicate review item.
- Added XIMG-064's host-authenticated Firefox viewed-media capture-plan
  boundary. It admits bounded, actually viewport-displayed thumbnail metadata
  only after paired actor, exact-origin policy, adapter, and scheduler checks;
  query components are redacted and no browser payload, source cookie,
  authorization header, form data, or credential is accepted. The extension
  remains fail-open and does not automatically open originals.
- Reframed Instagram as an explicit Firefox site-policy path; the dedicated
  Instagram API connector is now optional future work.
- Added XIMG-061's host-mediated extension pairing contract.
- Added XIMG-062's explicit per-site Firefox policy controls.
- Added XIMG-063's explicit, versioned Firefox site-adapter registry.
- Added XIMG-054's review states, reversible batch actions, and ObjectStore
  original versus observed-thumbnail status treatment.
- Added XIMG-055's per-account refresh progress and retry presentation.
- Added XIMG-056's keyboard-operable client-side metadata search.
- Added XIMG-060's least-privilege Firefox Manifest V3 scaffold.
- Added XIMG-051's selected-context X/Instagram/website source navigation.
- Added XIMG-050's Mnemosyne-compatible Yew/Monas shell.
- Added XIMG-046's verified-commit-only new-item review admission queue.
- Added XIMG-045's one-click social-account refresh orchestration contract:
  enabled X/Instagram accounts become coalesced global jobs with bounded
  per-account progress, partial failure, cancellation, retry, no-overlap, and
  final new-item summary states, without connector execution or media transfer.
- Added XIMG-044's fixture-only incremental Instagram discovery planner. It
  handles posts, carousels, reels/videos, pagination and budgets; exposes
  opaque credential expiry/revocation; selects supported renditions; preserves
  provenance; and feeds reconciliation idempotency without an API call, media
  transfer, or XIMG-043/ADR 0002 gate bypass.
- Added XIMG-042's fixture-driven, incremental X media-discovery planner. It
  enforces cursor/page/item/depth budgets, selects supported best photo/video/
  animated-GIF variants, preserves metadata provenance, and produces existing
  idempotency requests without a live X call, media transfer, or ADR 0002
  policy bypass.
- Added XIMG-041's explicit followed-X-account selection boundary. A
  grant-bound returned candidate list becomes a reviewable added/existing/not-
  selected configuration diff and requires confirmation before the existing
  atomic configuration save; it neither bulk-enables accounts nor makes live X
  traffic while ADR 0002 remains open.
- Added XIMG-040's official X OAuth 2.0 Authorization Code + S256 PKCE host
  boundary. State replay, denial, expiry, required scopes, viewing-account
  binding, opaque refresh, and revocation are test-covered without raw tokens,
  cookies, secrets, or live X API/media traffic.
- Added XIMG-038's confirmed direct bioinformatics commit orchestrator. It
  requires allowed explicit plan confirmation and exact destination
  revalidation, streams bounded chunks through the DAS ingest port, verifies
  SHA-256/length/authority receipt, and records idempotent metadata provenance
  without durable local payloads or live transport.
- Added XIMG-037's reviewed destination row and commit-time revalidation
  contract. It exposes all discovered stores in structured status-word rows and
  refuses removed, renamed, unavailable, read-only, expired, over-quota,
  untrusted-TLS, reconnect, and cross-endpoint alias states without fallback.
- Added XIMG-036's strict endpoint/device and logical ObjectStore inventory
  contract. Synthetic fixtures prove managed local profile and paired remote
  discovery selection while rejecting unmanaged folders, mutable identities,
  broad secrets, and arbitrary first-store selection; no live pairing,
  credential, or transport integration is enabled.
- Made hosted CI non-blocking while GitHub Actions funding is unavailable.
  GitHub workflows are manually dispatched advisory mirrors; recorded local
  verification remains the delivery authority until CI is backfilled or moved.
- Added XIMG-035's dependency-free cross-repository contract check and pinned
  optional sibling-source workflow. A public clone validates x-img-owned
  fixtures without sibling dependencies; an explicit four-repository checkout
  verifies exact compatibility revisions and contract anchors without
  credentials or a claim of live authority integration.
- Added XIMG-034's authorized DASObjectStore object-read/cache handoff port.
  It validates content type, lengths, SHA-256, ETag, conditional and byte-range
  metadata, and explicit unavailable states before returning a stream, without
  persisting media locally.
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
