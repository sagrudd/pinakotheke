# ADR 0009: Acquisition and catalogue metadata schemas

- Status: Proposed; implementation remains behind the XIMG-002 policy and
  external-authority gates
- Date: 2026-07-14
- Deciders: x-img maintainers
- Scope: source items, media identity, ObjectStore references, download
  attempts, job leases, account cursors, review state, tombstones, and audit
  events

## Context

x-img needs one versioned metadata contract for social posts/items, explicitly
selected website candidates, and user-identified bioinformatics resources. The
contract must survive duplicate discovery, URL rotation, provider cursor reset,
worker crashes around a DASObjectStore upload, and platform identifiers being
reused for different content. It must also keep the public repository free of
media bytes, credentials, cookies, sessions, signed delivery URLs, and private
account data.

The schema is an aggregate metadata envelope in
`schemas/x-img-acquisition.v1.schema.json`. Its arrays are a portable contract
shape for fixtures and future persistence; referential uniqueness, transition
ordering, and crash reconciliation remain domain rules for XIMG-022/XIMG-023.
Unknown properties and unknown future schema majors fail before a record is
accepted or mutated.

Compatibility-sensitive sibling revisions inspected for this contract are:

| Sibling | Revision | Relevant boundary |
| --- | --- | --- |
| `../monas` | `3d21b0bc7b83fa8408d01b93347a56f43f3a96b7` | host-owned authenticated context and product mount |
| `../DASObjectStore` | `0e1cd2299d09ca95d6e2e0e3e2b86cdc92085c9a` | verified object metadata, authority-owned identity, and scoped upload completion |
| `../mnemosyne_design_language` | `5539df8f662a78ebdf7cf4c868d71831380c8cfd` | status words and task-pane vocabulary used by future review UI |
| `../mnemosyne` | `ee21d98b23dec3caa6926d9d0dcc002989aa465b` | future host/catalogue adapter boundary; not a runtime dependency |

Sibling behavior is evidence for the public contract, not an unpublished path
dependency. x-img stores only stable endpoint/ObjectStore IDs and immutable
object evidence supplied by the storage authority.

## Decisions

### 1. Source item and media identity

- A `source_item` records the platform/site/resource family, account or origin,
  source item kind, provider item ID when available, safe source URL aliases,
  discovery time, and policy result.
- A `media_identity` is the canonical acquisition identity for one media member
  of a source item. Its `canonical_media_key` is not a URL. A platform media ID
  is retained only with its source scope; provider IDs are not globally unique.
- URL aliases are append-only provenance. Duplicate URLs attach to the same
  identity when canonical identity and content evidence agree. A rotated URL
  does not create a new identity or overwrite the old alias history.
- If a provider reuses an item/media ID for different content, the immutable
  checksum evidence cannot be silently replaced. The record enters `conflict`
  and reconciliation preserves both provenance sets.

### 2. Object reference and settlement

- An `object_reference` contains endpoint ID, logical ObjectStore ID, object
  key, typed content metadata, exact byte length, immutable SHA-256, verified
  and committed times, and a provenance reference. Display names are not
  authority keys.
- x-img never places media bytes in this envelope, a local fixture, a log, or
  browser storage. Upload IDs are opaque metadata; completion capabilities,
  bearer tokens, signed queries, and host sessions are not persisted here.
- The settlement key is canonical media identity plus immutable verified
  checksum. A verified committed object is idempotent: a retry may repeat
  network work, but it cannot overwrite a different object or create a second
  committed catalogue record for the same key.

### 3. Attempts, leases, and cursors

- A `download_attempt` records bounded transfer evidence, expected/received
  lengths, checksum evidence, an opaque authority upload ID, timing, and an
  explicit state. It does not store payloads or response bodies.
- A `job_lease` records the job scope, owner, nonce, expiry, heartbeat, and
  reclaim state. Atomic claim and expiry behavior are implementation rules;
  double claims and stale-owner writes must be rejected.
- An `account_cursor` is advisory checkpoint metadata. A connector may replay
  or reset it after provider edits, deletions, invalidation, or pagination
  changes. Cursor values never replace canonical identity deduplication.

### 4. Review state, tombstones, and audit

- `new`, `reviewed`, and `retained` review states require a committed verified
  ObjectStore reference. Discovery, upload start, or an unverified provider
  response is never enough to admit a review card.
- `hidden` and `removed` are explicit review outcomes. Deletion, permission,
  rights, and policy changes create a `tombstone` with a reason and separate
  object-removal request/completion flags; removal is not silently inferred.
- Append-only `audit_event` records capture state transitions and safe
  references, not secrets or provider payloads. Reconciliation emits an event
  and converges to the same terminal result when run more than once.

## State and reconciliation rules

The normal media acquisition path is:

```text
discovered -> claimed -> transferring -> stored -> verified -> committed
```

The following outcomes are explicit and cannot be used as a shortcut to
`committed`:

```text
failed | policy-blocked | cancelled | tombstoned | conflict
```

- Before the authority accepts an upload, a retry may create another attempt,
  but the prior attempt remains evidence and no catalogue object exists.
- After authority upload completion but before catalogue commit, reconciliation
  queries the authority using the scoped upload/object identity and checksum.
  A verified matching object settles exactly once; an absent or mismatched
  object remains failed/conflicted and is never treated as committed.
- After catalogue commit, a replay finds the canonical identity and checksum,
  attaches any new URL alias, and does not transfer or overwrite bytes.
- A cursor replay, duplicate page, duplicate URL, or edit is processed through
  the same canonical identity path. A platform-ID collision with a different
  checksum produces `conflict`, not replacement.
- A lease expiry permits reclaim only after the prior owner is stale and
  in-flight authority state has been reconciled. A stale owner cannot advance
  state after reclaim.

## Alternatives considered

- **Use source URLs as identity:** rejected because URLs rotate, may be signed,
  and can duplicate the same media.
- **Use provider cursors as exactly-once state:** rejected because cursors can
  reset, expire, reorder, or replay provider results.
- **Mark review state at discovery:** rejected because a crash can expose an
  unverified or unavailable object.
- **Store a local payload or upload capability in the record:** rejected because
  DASObjectStore is the only durable byte authority and capabilities are
  short-lived secrets owned by the authority.
- **Overwrite a committed object on platform-ID reuse:** rejected because it
  destroys provenance and makes immutable object verification meaningless.

## Failure modes and required behavior

| Condition | Required result |
| --- | --- |
| Unknown field or future schema major | Reject before mutation; require explicit migration |
| Duplicate URL or rotated source URL | Attach alias/provenance; do not create a second identity |
| Same platform ID, different checksum | Enter `conflict`; preserve both evidence sets |
| Crash before upload | Keep attempt/lease evidence; retry safely |
| Crash after authority upload, before catalogue commit | Reconcile by upload identity and checksum; settle once or fail explicitly |
| Checksum/length mismatch | Abort or quarantine through DASObjectStore; never commit/review |
| Lease owner expiry | Reclaim only after expiry and authority reconciliation; reject stale writes |
| Cursor invalidation/reset | Replay with cursor event and canonical identity deduplication |
| Policy/rights/permission change | Stop admission; create policy-block or tombstone audit state |
| Object unavailable | Show unavailable state; never substitute a local durable payload |

## Privacy and security impact

The schema permits identifiers, timestamps, checksums, typed object references,
bounded cursor/attempt metadata, and safe audit codes only. It deliberately
does not define fields for passwords, cookies, authorization headers, host
sessions, raw access tokens, signed URLs, provider response bodies, browsing
history, or media bytes. Logs and diagnostics must apply the same redaction
rules. A future persistence implementation must reject unknown fields before
deserialization has any side effect.

## Compatibility impact

The schema ID `x-img.acquisition.v1` is a public contract. Additive fields
require an explicit compatible schema revision and fixtures; incompatible
changes require a new major and a non-destructive migration. Referential and
state-machine rules are intentionally tested in the future Rust contract
layer, not inferred from JSON Schema alone. This preserves a stable adapter
boundary for Monas today and Synoptikon later without requiring either sibling
checkout in a public x-img build.

## Acceptance tests

- `valid-minimal.v1.json` validates against the draft-2020-12 schema and
  demonstrates all required entity families, a duplicate/rotated URL alias,
  verified immutable object evidence, and review admission after commit.
- `invalid-unknown-field.v1.json` is rejected at the aggregate envelope.
- Fixtures reject unknown fields in every entity, unknown future majors,
  malformed IDs/checksums/timestamps, negative lengths, unsafe URL aliases,
  and invalid enum values.
- State tests accept only the documented lifecycle and reject double claims,
  transfer/commit before verification, and `new`/`reviewed`/`retained` without
  a committed ObjectStore reference.
- Crash fixtures cover before upload, after upload completion, after
  verification, and around catalogue commit; repeated reconciliation converges
  to one terminal result and never overwrites bytes.
- Identity fixtures cover duplicate URLs, URL rotation, duplicate pages,
  cursor reset/replay, and platform-ID reuse with a different checksum.
- Lease fixtures cover atomic claim, heartbeat, expiry, stale-owner rejection,
  bounded reclaim, cancellation, and release.
- Privacy/static checks reject media payloads, cookies, credentials, sessions,
  signed query values, and private URLs in records, fixtures, and logs.

## User-facing documentation

The Sphinx configuration guide explains the separation between source
configuration and acquisition metadata. Future UI documentation must expose
state words, verified ObjectStore status, policy blocks, tombstones, conflicts,
and object-unavailable states without using colour alone.
