# ADR 0010: Firefox interception architecture spike

- Status: Accepted; progressive image/MP4 paths and the fail-closed segmented
  adapter evidence gate are implemented, while exact HLS/DASH adapters remain
  individually gated by the browser and host-contract checks below
- Date: 2026-07-14
- Deciders: x-img maintainers
- Scope: XIMG-007 response observation, capture, and cache substitution for
  explicitly enabled Firefox origins

## Context

XIMG-007 must establish a safe Firefox architecture before the extension is
implemented. The extension may observe or substitute only media on an origin
that the user enabled, through the paired x-img instance. It must preserve
ordinary page loading when the extension, x-img, Monas, or DASObjectStore is
unavailable. It must not turn response observation into hidden traversal,
credential capture, or an implicit API-avoidance loophole.

This spike uses Mozilla's current WebExtensions documentation as the browser
authority:

- [`webRequest`](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/webRequest)
  describes request lifecycle events, redirects, response headers, and the
  requirement for host access.
- [`webRequest.filterResponseData()`](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/webRequest/filterResponseData)
  gives an extension control of a response stream, requires `webRequest`,
  `webRequestBlocking`, host permission, and (for MV3) `webRequestFilterResponse`.
  A filter must write and close/disconnect or it can keep the request open.
- [`declarativeNetRequest`](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/declarativeNetRequest)
  supports block, redirect, and header actions; it does not replace response
  bodies. Redirect/header use still needs the relevant host access, and
  response-header matching is feature-sensitive.
- [`optional_host_permissions`](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/optional_host_permissions)
  and [`permissions.request()`](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/permissions)
  support an exact, user-action-gated origin request.
- [`MediaSource`](https://developer.mozilla.org/en-US/docs/Web/API/MediaSource)
  and [Mozilla's media format guidance](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Formats/Containers)
  show why segmented media and codec/container readiness require explicit
  browser tests rather than generic URL rewriting.

The compatibility-sensitive sibling pins used for this planning decision are:

| Contract | Commit | Relevance |
| --- | --- | --- |
| Mnemosyne design language | `5539df8f662a78ebdf7cf4c868d71831380c8cfd` | task-pane, state-word, and accessibility patterns |
| Monas | `3d21b0bc7b83fa8408d01b93347a56f43f3a96b7` | host-owned authentication and host-relative product boundary |
| DASObjectStore | `7f97d66117060aace68612dae4d5578221da59db` | authorized object read, range, and storage authority |
| Mnemosyne/Synoptikon contracts | `ee21d98b23dec3caa6926d9d0dcc002989aa465b` | future host adapter; no unpublished dependency |

## Decisions

### Isolate browser behavior behind explicit capabilities

The extension uses a versioned site-adapter capability registry. A capability
names the exact origin, resource type, observation rule, interception mode,
required Firefox permissions, response contract, and fail-open behavior. A
capability is not inferred from a matching URL, a redirect, or a successful
network request. Unknown or unsupported capabilities remain origin-served.

The first implementation spike has three modes:

1. **Observed progressive image:** eligible after a displayed/observed image
   event; response observation may use a bounded `filterResponseData` stream or
   a separately authorized source fetch with credentials omitted.
2. **Explicitly opened progressive video:** eligible only after a user-open
   event; substitution requires an HTTPS x-img delivery contract with correct
   MIME, length, ETag, conditional requests, and byte ranges.
3. **Segmented HLS/DASH:** experimental and site-specific only. A generic
   extension must not discover manifests or segments, rewrite playlists, or
   traverse hidden media. It remains origin-served until a fixture and real
   Firefox test prove manifest/segment identity, authorization, ranges, and
   fail-open behavior.

`declarativeNetRequest` is limited to deterministic, precomputed redirect or
header rules where a site capability proves the complete response contract.
It is not a body-capture or body-substitution mechanism. Programmatic
`webRequest.filterResponseData` is the only planned response-body interception
path, and it must be feature-detected and bounded.

### Keep permissions and request data least-privilege

- The options/task pane shows the exact origin, capture/substitution effects,
  media classes, and pause/remove controls before requesting
  `optional_host_permissions` from a direct user action.
- The manifest does not request `cookies`, `history`, password access, private
  browsing access, or unrestricted host access. Capture code never registers
  `requestBody`, `requestHeaders`, `onAuthRequired`, or a credential relay.
- x-img receives a bounded observation/open event and approved source metadata,
  not cookies, authorization headers, form bodies, credentials, or general
  browsing history. Signed query parameters are removed before diagnostics or
  alias lookup; a rotated signed URL is only a source alias.
- The extension may automatically cache only a thumbnail that was actually
  displayed/observed. An original requires a separate explicit user-open event.
  Background, hidden, speculative, playlist, channel, and bulk-discovered
  resources are ineligible.

### Preserve HTTP and page behavior

The x-img delivery boundary is HTTPS and host-relative to the paired instance.
An HTTP loopback endpoint is not a valid substitute for an HTTPS page because
it risks mixed-content failure. The response must be authorized for the
reviewed endpoint plus ObjectStore and must preserve the media content type,
exact content length, ETag/checksum identity, conditional-request behavior,
`Accept-Ranges`, and valid `Content-Range` responses.

The extension does not silently strip CSP, CORP, CORS, TLS, or authorization
failures. Redirects and response-header changes are capability-specific and
must be tested against the page's initiator and target origin. If lookup,
pairing, permission, TLS, mixed-content, CORS/CSP/CORP, range, ObjectStore, or
filter-stream handling fails, the extension disconnects or bypasses the
interception and lets the source request continue. It must never redirect to a
different store or retry through a different x-img instance.

`filterResponseData` has a dangerous default: a filter that neither writes nor
closes/disconnects can leave the browser request open. The implementation must
write through bounded chunks immediately, enforce a timeout/byte budget, and
call `disconnect()` on any uncertain path. A source-page disruption is a test
failure even if the x-img request itself is correct.

### Keep segmented video behind a proven adapter

HLS/DASH support is not part of generic capture or substitution. A site adapter
may expose a segmented capability only after it proves, with synthetic fixtures
and real Firefox coverage, that the user-visible page caused the request, the
manifest and segments are authorized and canonicalizable, the selected
rendition is a committed normalized Pinakotheke object, and failure returns to
the origin. DRM, encrypted/unsupported streams, hidden manifests, and
unproven MSE behavior remain explicitly blocked or origin-served.

## Alternatives considered

- **Use DNR as the universal interceptor:** rejected because DNR does not
  replace response bodies and its header/redirect behavior cannot prove the
  required per-site response contract by itself.
- **Read and replay page cookies or authorization headers:** rejected because
  it violates the Monas/extension boundary and expands the credential surface.
- **Fetch every observed URL in the background:** rejected because observation
  is not permission to traverse hidden or speculative media and it could
  capture credentials or general browsing history.
- **Use an HTTP localhost proxy from HTTPS pages:** rejected because mixed
  content and local-network trust are not safe defaults.
- **Treat all HLS/DASH manifests as ordinary MP4:** rejected because
  manifests, segments, MSE, codecs, DRM, and authorization have different
  contracts and must be proven by adapter.
- **Fail closed when substitution is unavailable:** rejected because ordinary
  page loading must continue from the origin.

## Failure modes and required behavior

| Condition | Required result |
| --- | --- |
| Site is not explicitly enabled or permission was revoked | Do not intercept or query x-img; serve origin |
| Candidate was not displayed/observed, or original was not explicitly opened | Do not admit capture or substitution |
| Cookies, auth headers, form bodies, credentials, or history would be read | Discard/block the path; never forward the data |
| Pairing, origin, audience, scope, expiry, or endpoint/store check fails | Bypass; never select another instance or store |
| Signed URL is stale or rotated | Resolve only through a redacted canonical alias; never log the query |
| Filter stream errors, timeout, or budget exhaustion | Disconnect and fail open without a redirect loop |
| MIME, length, ETag, conditional, range, CORS/CSP/CORP, TLS, or mixed-content check fails | Bypass and preserve the source request |
| HLS/DASH/DRM capability is not proven for the exact site | Keep origin-served and report an explicit unsupported state |
| x-img, Monas, DASObjectStore, or network is unavailable | Keep ordinary page loading from the origin |

## Privacy and compatibility impact

The browser stores only the paired instance metadata, explicit site rules,
capability versions, and bounded redacted diagnostics. Durable media bytes
remain in DASObjectStore. No browser storage, log, fixture, or repository file
may contain source payloads, cookies, tokens, credentials, signed query values,
or general browsing history.

The JSON fixture at
[`docs/fixtures/firefox-architecture-matrix.json`](../fixtures/firefox-architecture-matrix.json)
is synthetic and records the browser evidence, capability gates, negative
privacy cases, and expected fail-open outcomes. It is not a claim that all
listed modes are currently implemented. Unknown future fixture majors must be
rejected. The extension-facing wire contract remains independent of browser
API names, so Firefox API changes can be handled by a capability adapter.

## Acceptance tests

- Validate the fixture as strict JSON and reject an unknown future schema major.
- Verify the permission flow shows the exact origin and consequences before a
  direct user-action request; removal pauses capture and substitution
  independently.
- Static/privacy checks reject cookies, history, `requestBody`, request-header
  capture, auth handlers, password fields, signed-query logging, and durable
  payload writes.
- Browser fixtures prove image observation, explicit video open, redirects,
  HTTPS/mixed-content, CORS/CSP/CORP, MIME/length/ETag/conditional requests,
  byte ranges, concurrent ranges, cancellation, and no redirect loops.
- Stream-failure fixtures prove `filterResponseData` writes through or
  disconnects and that every timeout/error returns to the origin.
- Negative fixtures prove no automatic opening, hidden traversal, bulk crawl,
  playlist/channel discovery, simulated browsing, DRM bypass, cookie/credential
  forwarding, or API-avoidance loophole.
- Segmented-video cases remain capability-gated until manifest, segment, MSE,
  authorization, normalization, and real Firefox playback evidence exists.
- A public clone/build uses the fixture and docs without any sibling-only path
  dependency or real account, URL, media, token, cookie, or credential.

## User-facing documentation

The Sphinx documentation must explain origin permissions, independent capture
and substitution controls, observed-thumbnail versus explicit-original
eligibility, unsupported/DRM states, segmented-video capability gates, and
fail-open behavior. The local authority remains:

```console
docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
docker run --rm x-img-docs:check
```
