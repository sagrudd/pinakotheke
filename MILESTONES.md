# x-img milestones

Status: planning baseline  
Version: 0.1.0  
Updated: 2026-07-14

## Product outcome

x-img provides one private, authenticated Web library for media acquired from:

- a small JSON allowlist of X/Twitter accounts;
- a small JSON allowlist of Instagram accounts; and
- websites enabled by the user through a Firefox extension.

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
- a Firefox spike plan identifies how redirects, range requests, HLS/DASH,
  CORS, CSP, signed URLs, and local HTTPS will be tested.

## 0.2.0 — Rust core and versioned contracts

Goal: implement platform-neutral domain logic without live platform access.

Exit criteria:

- Rust workspace and `clap` CLI skeleton compile on supported targets;
- versioned JSON config supports X accounts, Instagram accounts, and website
  policies with strict validation and safe atomic writes;
- acquisition state machine covers discovered, claimed, transferring, stored,
  verified, committed, failed, and policy-blocked states;
- canonical identity supports platform media IDs and content-hash fallback;
- review state distinguishes new, reviewed, retained, hidden, and removed;
- fixture adapters prove pagination, duplicate discovery, crash reconciliation,
  and retry behavior without network access; and
- no code path can persist media bytes outside the storage port.

## 0.3.0 — DASObjectStore and Monas integration contracts

Goal: make external authorities real before adding collectors.

Exit criteria:

- x-img is declared as a Monas product mount with mandatory authentication and
  object-store requirement;
- Axum routes consume Monas-authenticated context and reject direct/anonymous
  privileged access;
- a scoped DASObjectStore application identity obtains short-lived capability
  or S3 credentials without persisting secrets in x-img state;
- object keys, metadata, checksums, media types, provenance, and range-readable
  responses have contract tests against sibling fixtures;
- upload completion is reconciled idempotently after crash boundaries; and
- a future Synoptikon host adapter can replace Monas without changing domain or
  connector logic.

## 0.4.0 — Account connectors and one-click refresh

Goal: acquire new media from configured social accounts into one review queue.

Exit criteria:

- official, user-authorized X connector supports permitted public and protected
  access, pagination, photos, videos, GIFs, and highest-quality allowed variant;
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

Exit criteria:

- Yew UI is mounted in Monas and follows the central Mnemosyne tokens, shell,
  footer, keyboard, focus, and state patterns;
- account/source navigation, dense thumbnail grid, virtual scrolling, keyboard
  traversal, quick preview, metadata detail, and video playback are usable;
- new/unreviewed media is visibly filterable and can be marked reviewed in
  batches without using colour alone;
- the single `Refresh accounts` action exposes progress and safe retry;
- loading, empty, stale, permission, transport, and object-unavailable states
  are designed explicitly;
- thumbnails and originals are served from DASObjectStore through authorized,
  range-capable URLs; and
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
- initial adapters are fixture-tested before any generic-site mode is enabled.

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
- object unavailability falls back safely without redirect loops; and
- hit/miss/substitution diagnostics are inspectable without retaining general
  browsing history.

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
  and non-goals.

## 1.0.0 — Stable personal archive

Goal: stable public interfaces and dependable day-to-day operation.

Exit criteria:

- no unresolved P0/P1 TODOs;
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
