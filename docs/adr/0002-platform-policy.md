# ADR 0002: Platform policy and content-rights gate

- Status: Proposed; live acquisition blocked
- Date: 2026-07-14
- Deciders: x-img maintainers
- Scope: X and Instagram acquisition, display, storage, deletion, and
  automation

## Context

x-img is a private, single-user media archive. It is not an X or Instagram
client, public mirror, scraper, recommendation service, advertising service, or
model-training data set. The repository is public, while acquired media and
account configuration are private user data. This distinction does not by
itself grant a right to copy or retain another person's work.

This decision records a policy baseline, not legal advice. Platform terms and
API documentation can change; the implementation must re-check the linked
primary sources and obtain any required platform approval before live use.

## Decisions

### 1. Official APIs and explicit authorization only

- X acquisition may use only the currently documented X API and an approved,
  disclosed use case. Access credentials are host-managed and remain private;
  x-img must never collect browser cookies, passwords, copied session data, or
  another person's token. The X Developer Policy requires a written use-case
  description, approval for substantive changes, private credentials, and
  compliance with all incorporated policies. See [X Developer Policy](https://docs.x.com/developer-terms/policy),
  especially the access and credential requirements.
- A dedicated Instagram connector may use only a current, approved Meta API
  product and an official user-authorized flow. It is no longer required for
  the first product path: Instagram may be configured as an explicit Firefox
  website policy subject to the extension's observed-media-only, no-cookie,
  no-automation, fail-open rules. The initial implementation must not assume
  that a consumer or private account is addressable. The official Meta API
  collection documents Facebook Login access through a managed Page and an
  Instagram professional account, while the Instagram Login surface has its own
  product and permission requirements. See [Meta's Instagram API collection](https://www.postman.com/meta/instagram/documentation/6yqw8pt/instagram-api)
  and [Instagram API with Instagram Login](https://developers.facebook.com/docs/instagram-platform/instagram-api-with-instagram-login/).
- Browser automation, HTML scraping, reverse-engineered endpoints, copied
  cookies, password ingestion, and bypasses of permission or access controls
  are prohibited. X expressly prohibits non-API website automation and
  scraping; Meta prohibits automated collection without express permission.
  See [X automation rules](https://help.x.com/en/rules-and-policies/x-automation)
  and [Meta automated data collection terms](https://www.facebook.com/legal/automated_data_collection_terms).

### 2. Scope of accounts and media

- A source is eligible only when the authenticated account is entitled to view
  it and the selected API product explicitly permits the requested account and
  media class. Protected/private X content therefore requires the viewing
  account's official authorization; it is never obtained by scraping a logged-in
  browser. Instagram consumer, private, or unsupported account/media classes
  remain policy-blocked until the approved product, permission, and terms are
  identified in a later decision.
- The first live connector is read-only. It does not post, like, follow,
  unfollow, comment, message, or otherwise automate platform actions. Refreshes
  are bounded and user-initiated or explicitly scheduled within the approved
  API rate and cost limits. x-img must not circumvent rate limits or create a
  substitute for the platform application. See [X Developer Agreement](https://docs.x.com/developer-terms/agreement)
  and [X Developer Policy](https://docs.x.com/developer-terms/policy).

### 3. Rights, display, and redistribution

- Before enabling a source, the user must attest that they own or have the
  necessary rights and lawful basis to archive the requested works and any
  personal data in them. A private archive is not a license to copy, publish,
  sell, sublicense, or redistribute third-party works.
- X-derived presentation must retain source attribution and current display
  requirements. x-img may not imply X sponsorship, alter post text, place
  unrelated advertising in the X-content presentation, or use X data for
  model training or advertising outside X. See [X display requirements](https://docs.x.com/developer-terms/display-requirements)
  and [X Developer Agreement](https://docs.x.com/developer-terms/agreement).
- x-img will expose source, account, canonical item identity, provenance, and
  policy state in its local catalogue, but will not make the catalogue or
  media public. All durable media bytes remain in DASObjectStore under its
  authority; x-img stores only permitted metadata, references, and audit state.
  This is an x-img architecture boundary, not a claim that DASObjectStore
  changes a platform's retention or display terms.
- Meta's Terms require users not to share content they do not own or lack the
  necessary rights to share, and restrict automated collection and data sale or
  licensing outside the Platform Terms. See [Meta Terms of Service](https://www.facebook.com/legal/terms).

### 4. Deletion, policy changes, and compliance

- Every live source must have a reconciliation path for deletion, suspension,
  protection, permission loss, and URL or media changes. A deletion or policy
  event must prevent new review admission and transition existing records to an
  explicit audit/tombstone state before removal from the object store is
  requested.
- For X, the connector must be able to remove or modify affected X Content as
  soon as possible and no later than 24 hours after a written request from X or
  the X user, unless a legally permitted exception and express written X
  permission apply. See [X Developer Agreement, updates and removals](https://docs.x.com/developer-terms/agreement).
- Meta deletion, retention, user-rights, and app-review obligations must be
  implemented from the approved API product's current terms and callbacks;
  x-img must not infer a universal retention period from a different Meta
  product. See [Meta Platform Terms](https://developers.facebook.com/terms),
  [Meta Terms of Service](https://www.facebook.com/terms), and the applicable
  [Meta data-protection terms](https://www.facebook.com/legal/terms/Meta-Global-Processor-Terms/).
- Policy changes, revoked permissions, or a missing compliance mechanism fail
  closed for live acquisition. Previously committed records remain visible as
  policy or object-unavailable states until the reconciliation decision is
  complete; they are never silently presented as current.

## Prohibited behavior

The following are hard failures, not degraded modes:

1. scraping or scripting X/Instagram web pages, including with a logged-in
   browser, to avoid an API restriction;
2. collecting or storing passwords, browser cookies, session cookies, access
   tokens in local JSON, or authorization headers in logs;
3. acquiring content the authorized account cannot view or the approved API
   product does not permit;
4. exceeding or circumventing API rate, cost, pagination, or access limits;
5. public redistribution, resale, sublicensing, advertising use, model
   training, or presenting the archive as endorsed by a platform; and
6. marking an item `new`, `reviewed`, or retained before the policy and
   DASObjectStore commit checks have succeeded.

## Alternatives considered

- **Scrape the website with the user's browser session:** rejected because it
  captures or depends on credentials and conflicts with the X and Meta
  automation restrictions.
- **Use an unofficial downloader or third-party mirror:** rejected because it
  weakens authorization, provenance, deletion handling, and platform-policy
  compliance.
- **Archive every public URL and resolve legality later:** rejected because
  public visibility does not establish copying rights, API permission, or a
  durable retention basis.
- **Permit live connectors now and add policy later:** rejected because the
  unresolved gates are P0 and could make already-acquired data non-compliant.
- **Fixture-only implementation until approval is complete:** accepted. It
  permits schema, state-machine, idempotency, and failure-path work without
  accessing real platform data.

## Failure modes and required behavior

| Condition | Required result |
| --- | --- |
| API approval or required scope absent | `policy-blocked`; no request for media bytes |
| Account class unsupported or private access not authorized | `policy-blocked`; retain reason and source identity only |
| API returns deletion, suspension, protection, or permission loss | stop new admission; reconcile to tombstone/audit state |
| Rate/cost budget exhausted | bounded retry or explicit `rate-limited`; never circumvent |
| Source URL rotates or returns an unexpected host | reject until canonical identity and policy are revalidated |
| DASObjectStore commit or verification fails | no `new`/`reviewed` state; reconcile on retry |
| Platform terms or API contract changes | disable affected live adapter; fixture tests may continue |

## Privacy impact

The design minimizes data by storing only the configured account/site identity,
source and media identifiers, provenance, policy result, audit state, and
DASObjectStore references in x-img. Tokens and authenticated host context stay
with Monas or another approved host adapter. Logs redact authorization,
signed-query parameters, browsing history, and private URLs. A user-controlled
archive can still contain personal data and sensitive imagery, so authorization,
access control, export, deletion, and backup handling remain required in later
milestones.

## Compatibility impact

The policy result is part of the versioned source and acquisition contracts.
Unknown future policy/schema majors fail closed. The connector boundary accepts
only an approved platform adapter, so a later Instagram product or Synoptikon
host adapter can replace the current one without adding a scraping fallback or
changing domain state transitions. No public build may require unpublished
platform or sibling path dependencies.

## Unresolved gates

These questions block live acquisition and must be answered in a dated review
before XIMG-040 or XIMG-043 implementation:

1. Which current X API access tier and exact read scopes will X approve for a
   single-user private archive that stores original media in an external object
   store and displays it in a Monas-hosted UI?
2. Which current Meta/Instagram API product, account classes, permissions,
   app-review requirements, media URL lifetime rules, and deletion callbacks
   cover the intended personal archive? In particular, are consumer or private
   accounts supported at all?
3. What copyright, personality-rights, privacy, and data-protection basis
   applies to each intended source and to people depicted in archived media?
4. What retention, deletion, export, backup, and jurisdiction requirements
   apply to the DASObjectStore copy after a platform deletion or user request?
5. Has the intended use case been accepted by each platform in writing where
   the terms or API review require approval?

Until these are answered, X remains fixture-only and a dedicated Instagram API
connector remains disabled. The explicit Firefox site-policy path remains
subject to platform terms and user rights; it is not an API bypass. Synthetic
or redistributable fixtures may cover pagination, duplicates, URL rotation,
deletion, rate limits, token expiry, malformed responses, and crash recovery;
they must not contain real media, account lists, tokens, cookies, or private
URLs.

XIMG-040 supplies a fixture-tested official OAuth 2.0 Authorization Code +
S256 PKCE host boundary only. It uses the documented ``tweet.read``,
``users.read``, ``follows.read``, and ``offline.access`` baseline and exact
redirect/state requirements, but does not change this live-acquisition block.
Its 2026-07-14 source review is the `X OAuth 2.0 Authorization Code flow with
PKCE <https://docs.x.com/fundamentals/authentication/oauth-2-0/authorization-code>`_.

## Acceptance tests

- Static checks reject platform hosts, browser-cookie APIs, password fields,
  authorization headers in logs, and hard-coded tokens outside test fixtures.
- Fixture adapters refuse unsupported account/media classes and unknown policy
  versions, while preserving an explicit reason and provenance.
- A deletion/protection/permission-loss fixture prevents review admission and
  converges to one tombstone without duplicating or overwriting object bytes.
- Rate and cost budget fixtures prove bounded retries and no overlap or
  circumvention.
- Rights/policy acknowledgement is required before enabling a live source; a
  missing acknowledgement leaves it disabled.
- Contract tests prove that a verified DASObjectStore commit is required before
  `new` or `reviewed` state and that no durable media is written to x-img.

## Source snapshot

The sources above were checked on 2026-07-14 UTC. Their terms and documentation
are mutable; implementation work must record the exact source revision or
retrieval date again when the live connector gate is revisited.
