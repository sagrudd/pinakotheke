# ADR 0007: Firefox pairing and per-origin opt-in

- Status: Proposed; extension work remains gated by XIMG-007 and the policy
  and host-contract decisions
- Date: 2026-07-14
- Deciders: x-img maintainers
- Scope: Firefox extension pairing, site permissions, capture, and requests to
  one configured x-img instance

## Context

The Firefox extension is a least-privilege client of one x-img instance. It
must be useful for explicitly enabled sites without becoming a crawler,
credential relay, second catalogue, or source-page disruption. Firefox's
current WebExtensions contract supports runtime optional permissions and
user-action-gated permission requests; response filtering has additional API
permissions and requires the extension to pass through or close the stream
correctly. These details are compatibility-sensitive and must be fixture-tested
against the supported Firefox release range.

Primary browser references checked on 2026-07-14 are Mozilla's [optional
permissions](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/optional_permissions),
[`permissions.request()`](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/permissions/request),
[`webRequest`](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/webRequest),
and [`webRequest.filterResponseData()`](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/webRequest/filterResponseData)
documentation. The exact supported manifest/API matrix remains an explicit
spike output rather than an assumption.

## Decisions

### Pair one extension profile to one instance

- Pairing is initiated from a visible user action and binds one extension
  profile to one explicit x-img origin/instance. The pairing record contains a
  stable instance identifier, origin, extension profile identifier, issued and
  expiry times, scope, and revocation status; it never contains a browser
  cookie, password, raw storage secret, or broad bearer token.
- Monas mediates host authentication and issues or brokers a narrowly scoped,
  revocable extension capability. x-img validates instance binding, audience,
  expiry, nonce/replay state, CSRF protection where a browser session is
  involved, and requested operation before accepting a request.
- Rotation replaces the old capability and revocation invalidates it. Expired,
  revoked, wrong-origin, wrong-instance, replayed, or over-scoped requests fail
  closed without retrying against another instance.

### Request permissions only for an enabled origin

- The options/task surface displays the exact origin, capture/substitution
  consequence, media classes, and pause/remove controls before requesting a
  host permission. Permissions are requested only from a direct user action.
- Site rules are explicit, versioned, and independently opt in capture and
  substitution. The extension may inspect or submit only media permitted by
  the active rule and supported adapter capability.
- The extension does not request the `cookies`, history, password, private
  browsing, or unrestricted host permissions. It does not forward cookies,
  authorization headers, form bodies, credentials, or general browsing
  history to x-img.

### Preserve the source page and platform boundaries

- No background task opens pages, traverses hidden DOM or network state, bulk
  crawls playlists/channels, simulates browsing, or discovers media not
  observed on an enabled page or explicitly selected by the user.
- A thumbnail is eligible only after actual display/observation. An original
  is eligible only after the user explicitly opens it. The extension records
  the observation/open event and adapter version as provenance.
- Capture and substitution errors are visible in extension diagnostics but
  fail open to the source page. The extension never claims that avoiding an
  API avoids a site's terms or rights restrictions.

## Alternatives considered

- **Bind the extension to any reachable local x-img instance:** rejected
  because a local network is not an identity boundary and reconnects could
  disclose data to the wrong service.
- **Copy the Monas session cookie into extension storage or x-img requests:**
  rejected because it expands the host credential surface and violates the
  host-owned session boundary.
- **Request all host permissions at install time:** rejected because it is
  broader than a site opt-in and obscures the consequence of enabling a site.
- **Use cookies or browser headers to refetch source media:** rejected because
  it forwards credentials and makes capture dependent on private session state.
- **Treat page disruption as an acceptable capture failure:** rejected because
  ordinary browsing must continue when x-img, Monas, storage, policy, or the
  extension is unavailable.
- **Keep pairing in browser local storage without server-side revocation:**
  rejected because expiry, replay, and instance revocation need an authority
  that can be enforced at the x-img boundary.

## Failure modes and required behavior

| Condition | Required result |
| --- | --- |
| Permission request is not caused by a user action or exact origin review | do not request; keep the site disabled |
| Pairing origin, instance, audience, scope, or expiry mismatch | reject; show `Pairing needs repair`; never fall back to another instance |
| Capability is revoked, rotated, replayed, or CSRF validation fails | reject and require a fresh Monas-mediated pairing |
| x-img, Monas, or DASObjectStore is unavailable | page continues from the origin; diagnostics may record a redacted reason |
| Candidate was not displayed/observed or explicitly opened | do not submit a capture |
| Adapter sees cookies, auth headers, form data, DRM, or unsupported media | discard/block with an explicit reason; never forward the data |
| Response filtering cannot safely pass through, range, or close a stream | disconnect the filter and fail open to the origin |
| Site permission is removed or site policy is paused | stop capture/substitution immediately; retain no new payload |

## Privacy impact

The extension stores only the paired instance identifier/origin, non-secret
capability metadata needed for expiry/revocation, explicit site rules, and
minimal adapter/capture audit state. It must not store site cookies, passwords,
authorization headers, page history, hidden URLs, or durable media bytes. x-img
receives only the bounded observation/open event, approved source metadata, and
the normal acquisition request through the shared server-side policy and
ObjectStore ports. Diagnostics redact signed queries, credentials, session
values, and private source URLs.

## Compatibility impact

The extension-facing pairing envelope, site-rule schema, observation/open
provenance, and failure states are versioned and reject unknown future majors.
The browser adapter is isolated behind a capability registry so Firefox API or
Manifest behavior can change without changing x-img identity, job, or storage
contracts. The current Mozilla references establish the test questions, not a
promise that every Firefox version supports every response-filtering mode.

## Acceptance tests

- Pairing fixtures cover origin/instance binding, expiry, rotation, revocation,
  CSRF, nonce replay, wrong audience, overbroad scope, and reconnect without
  silent instance switching.
- Permission fixtures prove the exact origin and consequences are shown before
  a user-action permission request, and removal/pausing stops both capture and
  substitution independently.
- Static/privacy checks reject cookies, password fields, history APIs, raw
  authorization headers, and secret persistence in extension or x-img state.
- Browser fixtures prove no automatic opening, hidden traversal, bulk crawl,
  simulated browsing, DRM bypass, or unobserved/unopened capture path exists.
- Capture fixtures distinguish displayed thumbnails from explicitly opened
  originals and retain only the required observation/open provenance.
- Response and page-disruption fixtures prove redirect, range, CORS/CSP/CORP,
  mixed-content, permission, extension, host, and ObjectStore failures fail
  open without loops or credential disclosure.
- A public build uses no unpublished sibling dependency, real account, cookie,
  token, credential, or downloaded media.

## User-facing documentation

The Sphinx/Read the Docs documentation must explain pairing, origin binding,
permission review, independent capture/substitution controls, observation
versus explicit-open semantics, revocation/repair, unsupported/DRM blocks, and
fail-open behavior. It must include the local container verification command.
