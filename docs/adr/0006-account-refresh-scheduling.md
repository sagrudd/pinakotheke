# ADR 0006: Account refresh scheduling and bounded jobs

- Status: Proposed; live connector execution remains blocked by XIMG-002
- Date: 2026-07-14
- Deciders: x-img maintainers
- Scope: the authenticated `Refresh accounts` action, per-account work,
  extension/resource jobs, leases, budgets, cancellation, and reconciliation

## Context

x-img must offer one user-visible refresh action for enabled X and Instagram
accounts without overlapping work, bypassing platform budgets, or turning a
retry into a duplicate acquisition. The same bounded scheduler will later
coordinate account refresh, Firefox capture, and explicit bioinformatics or
video jobs, while DASObjectStore remains the byte authority and the common
catalogue state machine remains the identity authority.

The scheduler is a domain contract, not a connector implementation. X and
Instagram remain fixture-only until the policy and approval gates in ADR 0002
are answered.

## Decisions

### Global refresh and child jobs

- A user-authenticated `Refresh accounts` command creates at most one active
  global refresh per x-img instance and actor scope. Repeated presses while it
  is active return the existing global job reference and do not enqueue a
  second pass.
- The global job snapshots the enabled account configuration and creates
  independently observable child jobs for each eligible account. The snapshot
  makes a running job reproducible when configuration changes mid-run; a later
  refresh observes the new configuration.
- Child jobs carry source identity, connector/adapter version, cursor input,
  policy result, budget reservation, lease, progress phase, cancellation state,
  retry metadata, and a link to the global job. A partial failure does not hide
  successful siblings; the global terminal result reports complete, partial,
  cancelled, failed, or policy-blocked children explicitly.

### Leases, coalescing, and bounded execution

- A scheduler lease is uniquely keyed by actor scope, source kind, and account
  identity. It has an owner nonce, issued/expiry times, heartbeat, attempt, and
  bounded concurrency class. A worker may claim only an unleased or expired
  job with an atomic compare-and-set operation.
- The scheduler enforces global worker limits, per-source limits, per-account
  non-overlap, request/page/byte/time budgets, and connector-declared rate or
  cost reservations. It never increases concurrency or retries to circumvent a
  platform response.
- Cancellation is cooperative at page, item, transfer, and DASObjectStore
  settlement boundaries. A cancelled job releases its lease, records the last
  safe checkpoint, and remains resumable only when the connector and authority
  can prove that retrying is idempotent.
- Queue backpressure is explicit. A full or unavailable queue returns a
  retryable `capacity-limited` result rather than accepting unbounded work or
  buffering media payloads in scheduler memory.

### Crash recovery and shared acquisition

- A worker crash or lease expiry never marks an item committed, new, or
  reviewed. Reconciliation consults the upload/object authority and the
  canonical identity/checksum before re-claiming the item.
- Cursor progress is advisory. The scheduler may resume from a stored cursor,
  reset it after a provider error, or replay a page; XIMG-0001's canonical
  identity and verified ObjectStore commit prevent duplicate catalogue records.
- The scheduler shares the acquisition port with extension and resource jobs,
  but source-specific leases prevent account work from overlapping itself.
  Different source families may run concurrently only within their budgets and
  shared worker/resource limits.
- A global job is terminal only after all child jobs are terminal or explicitly
  cancelled and leases are released. Reconciliation is safe to run again and
  converges to the same child/global result.

## Alternatives considered

- **Enqueue one independent job per button press:** rejected because repeated
  presses overlap account work, waste rate budget, and complicate user-visible
  progress.
- **Use one global lock for all sources:** rejected because one slow or blocked
  account would prevent safe progress for unrelated enabled accounts.
- **Trust cursors as exactly-once progress:** rejected because providers may
  edit, delete, reorder, repeat, or invalidate cursors; committed identity is
  the final authority.
- **Let workers retry until a source succeeds:** rejected because it can exceed
  rate/cost budgets and create an unbounded queue during an outage.
- **Mark items new when discovered before settlement:** rejected because a
  crash can expose a broken review card or a payload not yet verified by
  DASObjectStore.
- **Make connectors own separate schedulers:** rejected because account,
  extension, and resource jobs would not share leases, budgets, cancellation,
  or crash reconciliation.

## Failure modes and required behavior

| Condition | Required result |
| --- | --- |
| Repeated refresh while active | return the active global job; no duplicate child jobs |
| Account disabled after snapshot | finish the already-authorized snapshot or stop safely per policy; do not silently add newly enabled accounts |
| Lease owner crashes or heartbeat expires | reclaim only after expiry; reconcile in-flight ObjectStore state first |
| Provider rate/cost budget exhausted | pause or terminal `rate-limited`; bounded retry only within policy |
| Cursor invalid or reset requested | replay/reset with identity deduplication; record cursor event |
| Cancellation during transfer or settlement | stop at a safe boundary, release lease, retain resumable evidence, never mark committed prematurely |
| DASObjectStore commit/verification unavailable | leave item transferring/failed pending reconciliation; never mark new/reviewed |
| One child fails | retain failure reason and continue independent eligible children within global limits |
| Queue or worker capacity exhausted | return `capacity-limited`; reject unbounded enqueue |
| Configuration/schema/host/policy invalid | reject or `policy-blocked` before source requests; do not attempt browser or credential fallback |

## Privacy impact

Job records retain only source/account identifiers, policy and budget results,
cursor/checkpoint metadata, state, timing, adapter version, and provenance
references. They never retain passwords, cookies, host sessions, raw tokens,
authorization headers, signed URLs, private source payloads, or browsing
history. Progress and audit output redact source URLs when they contain secrets
and expose no media bytes. A user-visible job view shows state words and safe
retry/cancel actions rather than credential or provider-response dumps.

## Compatibility impact

Global-job, child-job, lease, budget, progress, cancellation, and reconciliation
records are versioned contracts. Unknown future majors fail closed. The
scheduler depends on the common acquisition and ObjectStore ports, so a Monas
host, Synoptikon host, X/Instagram adapter, Firefox adapter, or
bioinformatics adapter can change behind its boundary without changing lease
semantics. Snapshotting configuration prevents a mid-run schema update from
changing the meaning of an active job; explicit migration is required for
stored jobs.

## Acceptance tests

- Concurrent refresh fixtures prove one active global job, coalesced repeated
  presses, one child per eligible account, and no per-account overlap.
- Lease fixtures prove atomic claim, heartbeat expiry, stale-owner rejection,
  bounded reclaim, cancellation, release, and crash recovery.
- Budget fixtures prove global/per-account concurrency, page/request/byte/time
  limits, rate-limit backoff, no circumvention, and capacity backpressure.
- Partial-result fixtures prove independent child progress, explicit global
  partial/cancelled/failed states, and no loss of successful siblings.
- Cursor fixtures prove replay/reset, edits, deletions, duplicate pages, and
  provider cursor invalidation converge through canonical identity without
  duplicate committed records.
- Crash-injection fixtures around claim, transfer, DASObjectStore completion,
  verification, and catalogue commit never expose unverified `new` or
  `reviewed` items and reconcile idempotently.
- Privacy/static checks reject secrets, cookies, authorization headers, media
  payloads, and private browsing history in job state, logs, and fixtures.
- A public clone/build uses synthetic connector fixtures only and no live
  platform or sibling path dependency.

## User-facing documentation

The Sphinx/Read the Docs documentation must describe the global refresh action,
per-account progress, coalescing, budgets, cancellation, partial failure,
retry/resume, lease recovery, and why an item is not reviewable until its
ObjectStore commit is verified. The local `docs/Dockerfile` build remains the
documentation release authority.
