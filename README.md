# x-img

Public repository: [github.com/sagrudd/x-img](https://github.com/sagrudd/x-img)

`x-img` is a planned personal acquisition and review service for a small,
explicitly configured set of X/Twitter and Instagram accounts, user-identified
public GEO/SRA/ENA/NCBI resources, and websites enabled through a Firefox
extension.

All sources resolve to one x-img Web instance. That instance presents a
thumbnail-dense, ThumbsPlus-inspired library and review queue, and offers a
single-click refresh of configured social accounts.

## Non-negotiable boundaries

- Rust implementation with `clap` for CLI surfaces, `axum` for HTTP/API
  adapters, and `yew` for the Web UI.
- Interface hosting and login are owned by sibling `../monas`; x-img must not
  create a competing account or session system.
- Image and video bytes are stored only in a DASObjectStore ObjectStore through
  sibling `../DASObjectStore`; local x-img state may contain configuration,
  identifiers, indexes, and audit records, but never durable media payloads.
- Account and site configuration is local, explicit, versioned JSON.
- Bioinformatics acquisition is explicit and user-initiated: an accession or
  URL becomes a reviewable transfer plan before bounded streaming to the
  configured DASObjectStore ObjectStore. x-img does not bulk-discover or crawl
  public repositories.
- Storage endpoints/devices are separate from logical ObjectStores. The
  default local folder profile is provisioned by the authorized DASObjectStore
  service; remote endpoints are paired through Monas/DASObjectStore, expose all
  stores visible to the user, and require explicit endpoint-plus-store review
  before every write. x-img never writes an unmanaged folder or silently
  changes a reviewed destination.
- Video-focused websites may offer a user-selected, policy-gated review pane.
  Only observed or explicitly selected candidates enter it; normalized,
  checksum-verified, Firefox-tested renditions are stored as typed
  DASObjectStore objects before they are advertised as playable.
  Candidate planning records aggregate unusual codec combinations for
  prioritisation without sending individual browsing URLs, media, cookies, or
  credentials to project diagnostics or tickets.
- Video normalization is Docker-first and may execute only on an authorized
  DASObjectStore host, paired device worker, or future governed worker. Every
  plan pins an image digest and resource bounds; durable source and derived
  bytes remain DASObjectStore objects. See
  [normalized-video profile documentation](docs/normalized-video-profiles.rst).
  The first adapter is an explicitly paired-device worker with network-isolated,
  digest-pinned Docker/FFmpeg execution and bounded ephemeral scratch; it
  streams all derived objects to DASObjectStore before cleanup. See
  [video normalization](docs/video-normalization.rst).
- Acquisition is idempotent: once a media identity has a verified committed
  object, routine refreshes do not download it again.
- The UI follows sibling `../mnemosyne_design_language` and retains a future
  Synoptikon adapter boundary for `../mnemosyne`.
- The source repository is public and licensed under MPL-2.0. Archived media is
  private user data and is never committed to this repository.

## Current status

The `0.2.0` Rust workspace scaffold is available: shared model and core
boundaries, a `clap` CLI, an Axum composition boundary, and a Yew client
boundary compile without enabling any live source, storage, authentication, or
media-payload integration. See [MILESTONES.md](MILESTONES.md) for release gates
and [TODO.md](TODO.md) for dependency-ordered work. Automated contributors must
follow [AGENTS.md](AGENTS.md).

The CLI now strictly validates, lists, and atomically replaces local versioned
account/site configuration; it performs no network access or source refresh.
See [configuration documentation](docs/configuration.rst) for commands and
fail-closed validation rules.

The Rust core also enforces the acquisition lifecycle through verified object
commit and review admission. It is an in-memory policy boundary only; it does
not transfer media, contact an authority, or persist acquisition state. See
[acquisition lifecycle documentation](docs/acquisition.rst).

The core's reconciliation catalogue keys committed metadata by canonical media
identity plus immutable checksum. Replays reuse the first committed object
reference and append safe URL aliases; checksum disagreement becomes an
explicit conflict. It remains in-memory and performs no storage query or write.

The scheduler contract coalesces repeated refreshes, prevents concurrent work
for one source, bounds child capacity and cost usage, and releases leases on
cancellation. It remains an in-memory contract and does not execute any job.

Synthetic fixture coverage for X and Instagram is versioned and deterministic;
it exercises connector edge cases without network access or credentials and
does not authorize a live connector.

The core also validates one explicit bioinformatics transfer plan before any
future job: it requires reviewable source, file/checksum, rights, and stable
ObjectStore destination evidence, with policy blocking and confirmation kept
separate from transfer.

Confirmed bioinformatics files now stream through the existing bounded ingest
port only after plan confirmation and destination revalidation; their verified
receipt produces metadata-only provenance and deterministic in-memory replay.
No provider, authority, or local payload transport is enabled. See
[confirmed bioinformatics commits](docs/bioinformatics-commit.rst).

x-img also has a fixture-tested official X OAuth 2.0 + S256 PKCE host boundary:
Monas owns verifier, secret, token, refresh, and revocation custody while x-img
keeps opaque references only. Live X API/media access remains blocked on ADR
0002 approval and rights gates. See [X OAuth authorization](docs/x-oauth.rst).

Followed X accounts can now enter the local allowlist only through a
fixture-tested review preview: the user selects returned stable account IDs,
reviews added/already-configured/not-selected rows, then explicitly confirms a
complete candidate configuration for atomic save. It contains no live X call or
bulk import, and remains subject to ADR 0002. See
[followed-account selection](docs/x-followed-accounts.rst).

The core also plans incremental X media discovery from synthetic adapter pages:
it bounds cursors, timeline depth, pages, and items; selects the best supported
photo/video/animated-GIF rendition; records complete metadata provenance; and
feeds canonical identity plus checksum into the existing idempotency boundary.
It does not call X or transfer media while ADR 0002 remains open. See
[incremental X media discovery](docs/x-media-discovery.rst).

Instagram is supported first as an explicitly enabled Firefox website policy:
the extension may submit only media actually displayed to the user, never
forwards cookies or credentials, never automates browsing, and fails open to
the origin. A dedicated Instagram API connector is optional future work, not a
requirement for this path.

Firefox observed-image capture is now admitted through a narrow,
host-authenticated metadata-only plan endpoint. The extension submits only
viewport-displayed images after a toolbar click; x-img checks the paired actor,
site policy, adapter, and candidate bound before adding a redacted plan to the
common scheduler. It does not accept browser media bytes or mark anything as
stored. See [Firefox capture plans](docs/firefox-capture.rst).

Verified website captures can now enter the same `New` review queue as account
media. Their site, page, media alias, discovery time, and adapter provenance
are retained; a matching committed connector alias reuses its canonical
identity instead of creating a duplicate. See
[website capture review admission](docs/website-capture-review.rst).

One authenticated refresh action now has a fixture-tested orchestration model:
it selects all enabled social accounts, coalesces repeat presses, exposes
per-account bounded progress and terminal states, supports cancellation/retry,
and produces a final summary. It executes no connector or media work. See
[one-click account refresh](docs/account-refresh.rst).

x-img now carries a strict, versioned Monas product-registration contract for
one authenticated application/API mount, a DASObjectStore requirement, and a
future Synoptikon-equivalent bootstrap. It declares no x-img login/session
route; a later host-context adapter will enforce the registered host context at
the live Axum boundary. See [Monas product registration](docs/monas-product.rst).

Privileged API routes now require a host-injected, non-secret authenticated
context and reject direct requests. Monas and a future Synoptikon adapter share
the same fixture-tested boundary; x-img never parses or retains session cookies
or credentials. See [authenticated host context](docs/host-context.rst).

x-img also carries a strict DASObjectStore application-identity registration:
one endpoint/ObjectStore/prefix, bounded operations and bytes, explicit expiry,
and opaque host/authority references only. It is not a live token exchange or
storage adapter. See [DASObjectStore application identity](docs/das-application-identity.rst).

The core now defines a bounded streaming object-ingest port: chunks are sent
directly to an authority backend, checksum and exact length are verified before
completion, backpressure is surfaced without buffering, and verified completion
is idempotent. It does not store media bytes locally. See
[streaming object ingest](docs/object-ingest.rst).

The core also defines an authorized object-read/cache handoff port. It validates
media type, length, SHA-256/ETag, conditional reads, ranges, and unavailable
states before exposing an authority stream; it never keeps a local media cache.
See [authorized object read](docs/object-read.rst).

The Web library now opens selected cards in a keyboard-controlled quick-preview
task pane with source/type/ObjectStore metadata, visible alt text, fit/original
view, explicit unavailable-object handling, and focus return. Only a ready
normalized ObjectStore video receives the host-authenticated range playback
route; there is no origin fallback. See
[quick preview](docs/quick-preview.rst).

Ready normalized videos now have an actor-bound Axum delivery route at
`/api/playback/v1/{playback_id}`. It accepts only a server-side scoped
DASObjectStore stream and host-injected Monas context, preserves a verified
MIME/length/ETag/single-range response, and has no origin fallback. Firefox
range, seek, pause, and resume evidence is reproducible with an ephemeral
worker output; see [direct normalized playback](docs/direct-playback.rst).

The Firefox cache foundation now has a bounded, host-authenticated alias lookup
endpoint. It admits only canonical query-free aliases for verified committed
objects, structurally separates observed thumbnails from explicitly opened
originals, rejects immutable identity conflicts, revalidates same-instance/site
policy, and returns explicit origin-fallback states for misses, stale records,
or unavailable authority. See [cache alias lookup](docs/cache-alias-lookup.rst).

x-img now validates a versioned metadata-only endpoint/device and ObjectStore
inventory. It distinguishes managed local folder profiles from paired HTTPS
appliances, requires stable endpoint-plus-store IDs and explicit defaults, and
rejects unmanaged folders, credentials, mutable identity, and arbitrary
first-store selection. See [destination contracts](docs/destinations.rst).

Reviewed destination selection now exposes all authority-visible endpoint/store
rows with word-first readiness states, keeps the explicit reviewed pair, and
fails commit-time revalidation on removal, rename, TLS, expiry, reconnect,
read-only, quota, or alias changes rather than switching storage.

The supported-toolchain, browser, Semantic Versioning, dependency, fixture,
CI, documentation, and Definition of Done rules are maintained in the
[release and quality policy](docs/release-quality-policy.rst). Run the
dependency-free planning checks with `scripts/quality/check.sh`; the pinned
local Sphinx container remains the documentation release authority.
GitHub Actions is currently a manually-dispatched advisory mirror only: its
absence or status never blocks development. Record the local checks and
backfill or migrate hosted CI later.

Cross-repository contract evidence is also dependency-free by default:
`scripts/contracts/check.sh` validates x-img-owned fixtures in a public clone.
Where the pinned sibling repositories are available, use
`scripts/contracts/check.sh --require-siblings` to inspect their exact commits
and required contract anchors. This does not use credentials or claim live
authority integration; see the release policy for the boundary.

The v1.0.0 product and brand target is **Pinakotheke**. Until the coordinated
release migration is complete, this public planning repository remains
`sagrudd/x-img`; the target repository slug is `sagrudd/pinakotheke` and all
compatibility aliases and migrations must be documented before that move.
The [Pinakotheke identity migration guide](docs/identity-migration.rst) records
the staged cutover, compatibility window, immutable data identities, and
rollback constraints.

## Key concerns before implementation

1. X and Instagram access, storage, display, and deletion obligations must be
   reviewed against current platform terms and the user's rights to each work.
2. Protected/private account access requires an official user-authorized flow;
   browser cookies and credentials must never be scraped or copied into x-img.
3. “Exactly once” is implemented as idempotent committed acquisition, not as an
   impossible promise that a network request can never be retried after a crash.
4. Video caching requires byte-range and segmented-media behavior to be proven
   in Firefox before the external-cache design is declared viable.
5. The public project must not redistribute downloaded media, tokens, account
   lists, DAS credentials, or Monas sessions.
6. Firefox capture and substitution are per-site opt-in and fail-open: the
   extension never automatically opens pages, traverses hidden content,
   bulk-crawls, simulates browsing, or forwards site cookies/credentials;
   thumbnails are eligible only after display/observation and originals only
   after an explicit user open. Avoiding an API does not exempt behavior from
   platform terms.
7. GEO, SRA, ENA, and NCBI resources require explicit accession/URL selection,
   rights/policy review, destination confirmation, checksum verification, and
   provenance; controlled-access or license-uncertain resources remain blocked.
8. Endpoint/device identity, logical ObjectStore identity, pairing, capability,
   health, quota, and TLS state are authority-owned; extension/browser storage
   never contains raw passwords, S3 secrets, or broad tokens.
9. Video normalization uses versioned, evidence-backed playback profiles and a
   pinned containerized FFmpeg adapter. DRM, unsupported media, rights-
   uncertain sources, and source-only renditions remain explicitly blocked or
   failed; browser capture does not avoid platform terms.
10. Direct normalized-video playback is host-authenticated and ObjectStore-only;
    it preserves verified range and ETag behavior without enabling a website
    cache rule or falling back to an origin URL.

## Versioning

The project uses Semantic Versioning. The workspace package version is the
single product-version authority and is currently `0.2.0`; the stable
connector, storage, Web, and Firefox contracts are the `1.0.0` gate.

## License

Mozilla Public License 2.0. See [LICENSE](LICENSE).
