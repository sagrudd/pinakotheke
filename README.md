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

The supported-toolchain, browser, Semantic Versioning, dependency, fixture,
CI, documentation, and Definition of Done rules are maintained in the
[release and quality policy](docs/release-quality-policy.rst). Run the
dependency-free planning checks with `scripts/quality/check.sh`; the pinned
local Sphinx container remains the documentation release authority.

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

## Versioning

The project uses Semantic Versioning. The workspace package version is the
single product-version authority and is currently `0.2.0`; the stable
connector, storage, Web, and Firefox contracts are the `1.0.0` gate.

## License

Mozilla Public License 2.0. See [LICENSE](LICENSE).
