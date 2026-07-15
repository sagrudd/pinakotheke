# ADR 0004: Endpoint/device and logical ObjectStore selection

- Status: Accepted for XIMG-036's metadata contract; endpoint-aware writes
  remain gated by XIMG-037 and live DASObjectStore transport adapters
- Date: 2026-07-14
- Deciders: x-img maintainers
- Scope: Firefox capture, bioinformatics resource plans, and all future writes

## Context

Pinakotheke must support a useful zero-friction local setup and named remote
storage without confusing a physical/service endpoint with a logical
ObjectStore. A folder profile is a managed DASObjectStore deployment, not an
unmanaged directory that the extension may write directly. A remote appliance
may expose several ObjectStores with different purposes, policies, capacities,
and write capabilities. A display name is mutable and cannot be an identity.

Compatibility-sensitive inspection pins:

| Sibling | Commit | Relevant contracts inspected |
| --- | --- | --- |
| `../DASObjectStore` | `76f6411eab1e2c486c0bc1b4695b71f09307d9df` | `docs/application-authentication.md`, endpoint inventory, profile capability, remote EasyConnect, and daemon/API contracts |
| `../monas` | `3d21b0bc7b83fa8408d01b93347a56f43f3a96b7` | `README.md`, standalone authenticated host and product-mount boundary |
| `../mnemosyne_design_language` | `5539df8f662a78ebdf7cf4c868d71831380c8cfd` | `docs/interface-patterns.md`, `docs/brief.md`, endpoint inventory and destination-scoped task-pane patterns |

The public x-img build must not depend on these sibling paths. It will consume
versioned wire contracts or synthetic fixtures and keep host-specific behavior
behind adapters.

## Decisions

### Endpoint and ObjectStore are separate records

- An endpoint/device record identifies a managed local service, remote server,
  or appliance: stable endpoint/appliance ID, display name, deployment class,
  HTTPS/discovery address, TLS trust/onboarding state, pairing state,
  availability/health, and last refresh time. The ID is immutable across a
  rename.
- A logical ObjectStore record is discovered from an authorized endpoint and
  contains a stable ObjectStore ID, display name, class/purpose, writable or
  read-only capability, availability/health, authorized quota/free capacity,
  supported object types, and policy metadata. Store names never replace IDs.
- The default zero-friction endpoint is a local folder-profile DASObjectStore
  provisioned through its authorized daemon/service boundary. x-img or the
  extension may request bootstrap and render progress, but must never create,
  scan, or write an arbitrary unmanaged folder.

### Pairing, discovery, and selection

- Users add named remote endpoints through an HTTPS URL or approved discovery
  flow. Monas supplies the authenticated actor/session context and
  DASObjectStore approves pairing, TLS trust, scoped capability, expiry, and
  revocation. Raw passwords, S3 secrets, and broad or long-lived tokens never
  enter extension site rules or browser storage.
- After pairing, the service discovers and lists every ObjectStore visible to
  that identity. The accessible endpoint/store table and dropdown show display
  name, stable ID, endpoint/device, class/purpose, writable/read-only state,
  availability/health, authorized quota/free capacity, and supported object
  types. Values are refreshed from the authority, not trusted from the browser.
- Each endpoint has an explicit default ObjectStore. A user may override it
  for an enabled site or an explicit resource-commit plan. Every write review,
  button, job, and audit record shows endpoint and ObjectStore together. A
  remote endpoint never silently selects its first store or falls back to a
  different endpoint.
- Capture/resource selection filters to compatible writable stores, but an
  empty result remains an explicit `No compatible writable ObjectStore` state.
  The selected target is retained by stable IDs, not mutable display names.

### Commit-time safety and provenance

Immediately before every commit, the server revalidates the actor/session,
pairing and TLS state, endpoint and ObjectStore existence, current name and
health, write capability, supported object type, policy, quota, reservation,
and reviewed destination. Removed, renamed, unavailable, read-only, expired,
or disconnected targets pause or fail with a repair/reconnect action. Reconnect
must re-discover stores; it must never silently switch the destination. A user
must review a new endpoint/store pair before retrying a write.

Catalogue provenance binds endpoint/appliance ID, ObjectStore ID, immutable
object key and checksum, actor/session reference, and commit time. It does not
bind identity only to names. Cache substitution may read a committed object
from its recorded endpoint/store, subject to current authorization and
fail-open behavior. Writes always use the currently reviewed destination. If
multiple endpoints contain the same canonical alias, the catalogue keeps
endpoint/store-qualified locations and the UI says `Multiple committed
locations`; it never treats an alias as a unique destination.

### UI, transport, and privacy gates

Endpoint management and ObjectStore selection use Mnemosyne inventory-table and
destination-scoped task-pane patterns. The selector is keyboard accessible,
focus-safe, labelled with words such as `Ready`, `Read-only`, `Unavailable`,
and `Needs reconnect`, and uses icons/colour only as supplements. Local HTTPS,
mixed-content blocking, remote TLS trust, certificate rotation, discovery
origins, and pairing revocation are explicit setup and test gates.

## Acceptance tests

- A clean local bootstrap provisions a managed folder-profile endpoint through
  DASObjectStore and proves x-img never writes an unmanaged path.
- Synthetic remote discovery lists all visible stores with stable IDs and
  capabilities, health, quota/free capacity, and object types; pairing uses
  scoped, revocable credentials and browser storage contains no raw secret.
- The endpoint/store task pane and dropdown pass keyboard, focus, status-word,
  responsive, and no-compatible-target accessibility fixtures, including an
  accessible name, native-select or complete combobox semantics, option/state
  announcements, arrow-key navigation, Escape, and focus return.
- Site and resource plans display endpoint plus ObjectStore before confirmation;
  the server rejects removed, renamed, read-only, unavailable, expired, wrong-
  type, over-quota, and policy-invalid destinations immediately before commit.
- Reconnect and retry require a fresh reviewed target and never silently switch
  endpoints. Provenance fixtures bind stable endpoint/store IDs, object key,
  checksum, actor/session reference, and commit time; alias fixtures cover
  identical canonical media on multiple endpoints.
- Async refresh/reconnect races prove a stale stable-ID pair is rejected rather
  than replaced; the same reviewed pair remains visible in the plan,
  confirmation, job, audit record, and committed provenance.
- Local HTTPS/mixed-content and remote TLS trust/rotation/revocation fixtures
  fail closed for writes while cache reads remain safely fail-open to the page.
- A public clone builds without sibling-only path dependencies.

## User-facing documentation

The Sphinx/Read the Docs project must explain local folder-profile bootstrap,
remote pairing and TLS onboarding, store discovery, endpoint/store selection,
capacity and read-only states, reconnect behavior, and why x-img never accepts
an unmanaged folder or silently changes a reviewed destination. The local
`docs/Dockerfile` build remains authoritative.
