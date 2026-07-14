# ADR 0008: External-cache aliases and fail-open delivery

- Status: Proposed; cache implementation remains gated by XIMG-007 and the
  DASObjectStore read/range contracts
- Date: 2026-07-14
- Deciders: x-img maintainers
- Scope: Firefox image/video substitution, alias lookup, range delivery, and
  cache diagnostics

## Context

x-img may substitute previously committed media on an explicitly enabled site,
but ordinary browsing must remain reliable when the service, host session,
ObjectStore, network, or browser interception path is unavailable. Cache
substitution is not a second archive: it is a bounded lookup from a canonical
source alias to an immutable, authorized DASObjectStore object. The gallery and
extension must distinguish `Previously observed` thumbnails from `Stored in
ObjectStore` originals without mutating or watermarking stored bytes.

## Decisions

### Alias and eligibility model

- The lookup key is a canonical site/source alias produced by an enabled,
  versioned adapter. Signed or rotated source URLs are aliases, not durable
  identity. A hit resolves to an immutable object reference and its recorded
  endpoint/ObjectStore identity, subject to current authorization.
- Automatic thumbnail caching is permitted only after the thumbnail was
  actually displayed or observed. An original is eligible only after the user
  explicitly opened it. Unobserved, hidden, background, playlist, channel, or
  bulk-discovered media is never eligible.
- Substitution is independently opt-in per origin, transparent in diagnostics,
  and routed through the same paired x-img instance. It is never enabled by a
  cache hit, a first-store heuristic, a redirect, or a reconnect.

### Delivery and fail-open behavior

- The extension asks x-img for a bounded alias result containing only the
  immutable object identity, supported media class, authorization requirements,
  and delivery metadata needed for the request. The server revalidates the
  pairing, origin, policy, object availability, and read capability.
- Authorized delivery preserves the recorded media type, length, ETag/checksum,
  conditional-request behavior, redirects, byte ranges, cancellation, and
  seek/pause/resume semantics required by the supported browser profile.
- Any lookup, pairing, policy, TLS, mixed-content, CORS/CSP/CORP, authorization,
  range, MIME, length, ObjectStore, or extension error bypasses substitution
  and lets the original request continue. The extension must not rewrite the
  request repeatedly, redirect to a different endpoint, or expose a cache
  failure as a page failure. A bounded redacted diagnostic may record hit,
  miss, bypass reason, and latency without retaining general browsing history.
- A cache read never writes a new payload. Source-only, blocked, failed, DRM,
  unsupported, or unverified video is origin-served or visibly diagnosed; it is
  never advertised as a ready normalized rendition.

## Alternatives considered

- **Fail closed when x-img is unavailable:** rejected because the extension
  must never break ordinary page loading or make a private service a browsing
  prerequisite.
- **Cache every observed network URL:** rejected because it would capture
  hidden/background media, leak browsing history, and create unstable signed
  URL keys.
- **Use a browser-local durable payload cache:** rejected because DASObjectStore
  is the only durable media authority and local copies are difficult to revoke,
  audit, or reconcile.
- **Redirect to whichever endpoint responds first:** rejected because endpoint
  and ObjectStore are separate stable authorities and a reconnect must not
  silently change a reviewed or committed destination.
- **Rewrite bytes or add status watermarks:** rejected because stored media
  bytes are immutable and gallery state must be represented by reversible UI
  words/iconography, not media mutation.
- **Serve source-only video as a cache hit:** rejected because normalized,
  checksum-verified, probed, Firefox-tested renditions are the only ready
  playback objects.

## Failure modes and required behavior

| Condition | Required result |
| --- | --- |
| No explicit site substitution opt-in | serve origin; do not query x-img |
| Alias miss, stale signed URL, or unsupported adapter capability | serve origin; record bounded miss reason |
| Pairing/session/object authorization failure | serve origin; redact credentials and signed parameters |
| Object unavailable, endpoint reconnect, or ObjectStore health/quota issue | serve origin; do not select another store |
| Wrong MIME, length, ETag, conditional, or range response | abort substitution and serve origin without a loop |
| CORS/CSP/CORP, TLS, mixed-content, or redirect failure | serve origin; show a repairable diagnostic |
| x-img timeout or extension exception | fail open within a bounded timeout; preserve page load |
| Source media was not displayed/observed or original was not explicitly opened | no cache admission and no substitution |
| Normalization not verified or profile unsupported | source remains `Blocked`/`Failed`/origin-served, never `Ready` |

## Privacy impact

The cache retains only bounded alias metadata, immutable object references,
minimal hit/miss diagnostics, and the observation/open provenance required by
the acquisition contract. It must not retain general browsing history,
unobserved source URLs, cookies, credentials, authorization headers, signed
query strings, or durable media bytes in browser storage. Diagnostics use
coarse, redacted reasons and bounded retention. Endpoint/ObjectStore IDs are
displayed as authority metadata, never as secrets.

## Compatibility impact

Alias, object-reference, delivery, and diagnostic envelopes are versioned and
reject unknown future majors. The cache adapter remains separate from account,
bioinformatics, and video normalization adapters. The read contract must be
compatible with DASObjectStore's authorized content type, length, checksum,
ETag, conditional request, and byte-range behavior; unsupported cases remain
origin-served. Firefox implementation choices are pinned by fixture and real
browser tests rather than assumed from a single API mode.

## Acceptance tests

- Alias fixtures cover canonicalization, signed-URL rotation, immutable hit
  identity, duplicate aliases, invalidation, bounded memory, and endpoint/store
  qualification without first-store selection.
- Eligibility fixtures reject hidden/unobserved thumbnails, unopened originals,
  automatic opening, hidden traversal, bulk crawling, and simulated browsing.
- Site-rule fixtures prove capture and substitution are independently opt-in,
  same-instance, transparent, and pauseable.
- HTTP/browser fixtures prove MIME, Content-Length, ETag, conditional requests,
  redirects, CORS/CSP/CORP, HTTPS/mixed-content, authorization, byte ranges,
  concurrent ranges, cancellation, seek, pause/resume, and no redirect loops.
- Fault fixtures prove lookup timeout, expired pairing, ObjectStore loss,
  endpoint reconnect, unsupported media, and normalization failure all preserve
  origin loading and redact diagnostics.
- Video fixtures prove only a committed, checksum-verified, probed, real
  Firefox-tested versioned rendition can be a ready/playable substitution.
- A public clone/build contains no durable media, real browsing history,
  cookies, credentials, tokens, or private URLs.

## User-facing documentation

The Sphinx/Read the Docs documentation must explain aliases, eligibility,
per-site opt-in, endpoint/ObjectStore identity, hit/miss states, object
unavailability, range playback, normalized-video readiness, and fail-open
behavior. It must include the local container verification command.
