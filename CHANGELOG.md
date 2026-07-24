# Changelog

## 1.28.0 - 2026-07-24

- Add a first-party, bounded Unix-socket adapter for DASObjectStore's
  application-authorized exact-object deletion contract.
- Include exact content length in the deletion helper request and derive the
  DASObjectStore object ID from the upload-authoritative logical object key.
- Keep paired session material in a separate private host file and reject
  symlinks, permissive permissions, changed destination evidence, unknown
  schemas, and mismatched daemon request identities.

## 1.27.7 - 2026-07-23

- Record DASObjectStore 0.124.0's new application-authorized exact-object
  deletion boundary as the concrete XIMG-130 dependency.
- Keep authoritative deletion unavailable until the Pinakotheke helper,
  capacity reconciliation, deployment, and live synthetic evidence are
  complete; raw S3 deletion remains prohibited.

## 1.27.6 - 2026-07-23

- Makes ``submit_to_daemon: true`` an explicit mandatory capture-helper
  configuration field and removes the documented provider-only compatibility
  path.
- Accepts a DASObjectStore upload only when one authoritative completion event
  is the final non-empty remote-client result; transfer-only, ambiguous, and
  contradictory terminal output fails closed.

## 1.27.5 - 2026-07-19

- Supplies clear HLS/DASH assembly with the credential-free canonical page
  referrer and exact enabled origin already bound to the capture plan.
- Continues to exclude cookies, authorization headers, browser credentials,
  and arbitrary caller-supplied request headers from server-side retrieval.
- Reports bounded segmented-assembly failures accurately instead of collapsing
  them into the generic capture-rejected category.

## 1.27.4 - 2026-07-19

- Makes the authenticated actor's persisted exact-origin site corpus the sole
  site-enablement authority for executable Firefox capture plans.
- Retains independent pairing validation and reviewed endpoint/ObjectStore
  destination binding for every admitted capture.
- Accepts legacy static site records only for configuration compatibility;
  they no longer grant capture authority or participate in restart recovery.

## 1.27.3 - 2026-07-19

- Scopes explicit user-opened image and user-played video candidate budgets to
  the selected presentation/post instead of an entire scrolling profile page.
- Retains the configured per-page daily limit for automatic or observed
  candidates and the configured per-presentation limit for explicit media.
- Adds redacted categorical server diagnostics for capture-plan rejection.

## 1.27.2 - 2026-07-19

- Admits a visible video's trusted native-control play when Firefox reports an
  active transient user gesture, even if the page observer did not receive the
  consumed pointer event.
- Keeps autoplay, synthetic playback, hidden media, and expired activation out
  of the explicit-capture path.
- Pins admitted native-control and rejected no-activation behavior in the
  deterministic Firefox extension fixture.

## 1.27.1 - 2026-07-19

- Fixes capture-helper progress serialization so admitted explicit images no
  longer abort before DASObjectStore settlement with an unknown ``Progress``
  outcome.
- Pins the lower-snake-case capture-helper v1 discriminator with regression
  coverage at the producer and strict consumer boundaries.
- Retains the bounded trusted-selection snapshot used when a deep virtualized
  timeline replaces the selected image node between pointerdown and click.
- Requires DASObjectStore 0.121.1's unified local/provider inventory view so a
  verified Garage commit remains available to gallery reconciliation.

## 1.27.0 - 2026-07-19

- Reconciles gallery object availability from the authoritative DASObjectStore
  catalogue at startup and every ten seconds.
- Atomically tombstones representations removed out of band and restores them
  only when the protected object returns to the same endpoint and ObjectStore.
- Adds authenticated aggregate diagnostics for authoritative, projected,
  orphan, stale, and changed object counts.
- Defines a bounded inventory-helper contract backed by DASObjectStore's
  catalogue rather than raw provider bucket listings.

## 1.26.0 - 2026-07-19

- Adds an authenticated deletion review and confirmation action to image and
  normalized-video quick preview.
- Deletes every exact DASObjectStore representation before removing the
  persistent Pinakotheke projection; failed authority deletion keeps the
  gallery record visible and retryable.
- Removes duplicate catalogue rows sharing the selected asset's immutable
  object references without issuing duplicate ObjectStore mutations.
- Defines a bounded host deletion-helper protocol so DASObjectStore retains
  authority over policy, retention, catalogue mutation, and audit.

## 1.25.0 - 2026-07-19

- Adds generic trusted-play discovery for progressive and already-observed
  segmented video on explicitly enabled HTTPS origins.
- Shows a browser-only video download graphic with measured transfer progress,
  normalization/ObjectStore phases, and verified committed state.
- Frames navigation thumbnails for previously committed videos through the
  existing bounded in-memory cache-evidence batch.
- Preserves a bounded semantic creator/uploader hint and groups generic-site
  objects by that identity, with an explicit unattributed fallback.

## 1.24.0 - 2026-07-19

- Makes Firefox acquisition explicit-selection-only: displayed thumbnails are
  lookup candidates but never create capture plans or stored objects.
- Captures image bytes only after the user opens an image and video bytes only
  after the user starts playback.
- Fingerprints visible media by canonical identity rather than recycled DOM
  token, preventing long timelines from repeating unchanged viewport work.
- Makes settled capture-plan replay a read-only idempotent response that never
  requeues acquisition or ObjectStore settlement.

## 1.23.3 - 2026-07-19

- Retains an identity-bound image capture snapshot from a trusted pointerdown
  for two seconds so X cannot lose an explicit original request by replacing a
  deeply virtualized media node before the click event reaches the extension.
- Resolves image targets through the composed event path and records a redacted
  diagnostic when a trusted X click still has no eligible image.

## 1.23.2 - 2026-07-19

- Repairs stored-image frames locally when X disconnects and reuses a known
  media node, without waiting for or repeating a server lookup.
- Raises the bounded visible-image window from 16 to 64 while retaining one
  batch evidence request.
- Adds hit counts to redacted client/server batch diagnostics so large-page
  misses can be distinguished from DOM framing failures.

## 1.23.1 - 2026-07-19

- Removed the global capture-worker mutex and introduced separate bounded
  four-worker lanes for observed thumbnails and explicitly opened media, so
  background thumbnail work cannot queue an opened original or video.
- Reduced initial Firefox settlement polling from one second to 100 ms, then
  backs off to 250 ms and one second while preserving verified-commit-only
  green framing.
- Added concurrency and adaptive-polling regressions.

## 1.23.0 - 2026-07-19

- Replaced sequential per-image cache-evidence requests with one bounded batch
  request for up to 256 canonical media identities. Framing and substitution
  reuse the same response and never query DASObjectStore for existence.
- Added large-viewport regressions proving one network request covers sixteen
  visible images and every settled original receives stored-object framing.
- Added an authenticated server regression proving 256 in-memory evidence
  lookups complete as one bounded request.

## 1.22.21 - 2026-07-19

- Apply a stored frame to an X modal/gallery node that replaces the clicked
  thumbnail when both nodes display the same stable media identity; a recycled
  node displaying different media remains unframed.
- Track explicitly selected image downloads alongside videos in the extension
  popup, including selected, downloading, pending, failed, and stored states.
- Extend transient DASObjectStore acquisition retries to cover the observed
  multi-second settlement contention while revalidating destination authority
  before every attempt.

## 1.22.20 - 2026-07-19

- Preserve a safe source-presentation link for new captures and backfill
  historic X cards from the private capture-plan journal, allowing a user to
  explicitly reopen a thumbnail-only image and trigger original capture.
- Retry transient DASObjectStore upload and verification handoff failures with
  bounded backoff and fresh destination revalidation before every attempt.
- Extend deterministic browser fixtures to cover X-style overlay clicks and
  retain the restart-safe thumbnail-to-original single-card regression proof.

## 1.22.19 - 2026-07-19

- Derive X image gallery identity from the immutable media path rather than a
  transient page, gallery, presentation, or rendition URL.
- Persist each plan's resolved catalogue identity and preserve historic video
  suffixes when loading older capture-plan journals.
- Add guarded dry-run/apply reconciliation for historic duplicate X cards. It
  keeps the best verified representations, updates delivery paths and plan
  bindings together, creates private metadata backups, and never deletes
  DASObjectStore objects.

## 1.22.18 - 2026-07-19

- Render the verified DASObjectStore thumbnail in quick preview when an image
  was observed but its original was not captured.
- Distinguish an uncaptured or unavailable original from a wholly unavailable
  image record without ever falling back to the source website.

## 1.22.17 - 2026-07-19

- Reserve the two-pixel green browser frame for verified user-opened originals
  and user-played videos; an automatically observed thumbnail no longer claims
  that its original was imported.
- Bind page-element tokens and overlays to the rendered media identity, clear
  them when X recycles an element for different media, and require both token
  and URL to match when both are supplied by a settlement receipt.
- Prefer verified explicit-original evidence over thumbnail-only evidence for
  the same canonical alias.

## 1.22.16 - 2026-07-19

- Capture X playback initiated by site JavaScript when it follows the recorded
  trusted user activation, without trusting an uncorrelated programmatic play.
- Include visible video cards in bounded viewport evidence scans and recover
  their stored green frames using pairing-scoped X presentation identity when
  the live media element exposes only a transient ``blob:`` URL.

## 1.22.15 - 2026-07-19

- Preserve bounded, redacted acquisition failure categories across the capture
  helper boundary so operators can distinguish retrieval, video assembly,
  ObjectStore verification, and poster failures.
- Retry the checksum-addressed video poster commit across the short catalogue
  transaction handoff after its parent MP4 has settled.
- Store each poster beside its parent MP4 instead of beneath that object key,
  avoiding an invalid file-versus-folder collision in DASObjectStore.
- Grant the native DAS daemon the same narrow group-read access to the derived
  poster scratch payload as the already-working parent video upload.

## 1.22.14 - 2026-07-19

- Treat a genuine X media image with positive on-screen geometry as visible
  without requiring generic intrinsic-load, opacity, and style heuristics that
  incorrectly rejected X's virtualized gallery elements.

## 1.22.13 - 2026-07-19

- Make visible-media fingerprint delivery acknowledgement-driven so an early
  skipped or failed background scan remains retryable instead of suppressing
  all subsequent historical-evidence checks for that viewport.

## 1.22.12 - 2026-07-19

- Add a bounded, fingerprint-deduplicated visible-media safety scan to recover
  X virtualized gallery updates that emit neither a useful source mutation nor
  a reliable observer message during an active browsing session.

## 1.22.11 - 2026-07-19

- Add bounded tab-completion and tab-activation cache scans so historical
  ObjectStore evidence is refreshed even when a retained X tab does not emit
  the dynamically registered content-observer signal.

## 1.22.10 - 2026-07-19

- Restore authoritative prior-session cache-evidence lookups for paired
  extensions whose legacy display-only instance identifier is absent; Monas
  host context and the scoped pairing remain mandatory server-side.

## 1.22.9 - 2026-07-19

- Fix X gallery evidence framing with a browser-only, pointer-transparent
  top-layer rectangle that follows the settled media element despite X's
  click overlays and clipping containers.
- Resolve a trusted X gallery open to the stable original media rendition so
  explicit-original acquisition does not settle only the small grid image.

## 1.22.8 - 2026-07-19

- Prioritize genuine X media before applying the visible-candidate bound and
  exclude avatars, emoji, and interface imagery from capture/evidence frames.
- Resolve trusted clicks on X photo overlays to the visible underlying media
  so opening an unlinked gallery tile submits the explicit original again.

## 1.22.7 - 2026-07-19

- Make stored framing visible through X gallery overlays by marking the media
  and only its same-footprint visual wrapper chain.

## 1.22.6 - 2026-07-19

- Attach the authoritative gallery catalogue to the production monolith's
  extension cache-evidence route so settled aliases return evidence instead
  of failing Axum extraction before lookup.

## 1.22.5 - 2026-07-19

- Restore authoritative evidence lookup and observed-image capture when X
  serves visibly rendered media from a delivery host other than its historical
  image CDN; server-side policy remains authoritative for every acquisition.

## 1.22.4 - 2026-07-19

- Add redacted end-to-end cache-evidence and DOM-frame outcome diagnostics.
- Reinforce the browser-only stored marker with border, outline, and inset
  frame so site layout clipping cannot hide authoritative evidence.

## 1.22.3 - 2026-07-19

- Correct cache evidence matching across the extension-adapter and server-site
  identifier namespaces while retaining actor, pairing, origin, version,
  stable-media, settlement, and gallery authority checks.

## 1.22.2 - 2026-07-19

- Treat X ``name=small``, ``name=medium``, and opened-size renditions as the
  same stable media asset for stored-evidence framing.

## 1.22.1 - 2026-07-19

- Mount authoritative cache-alias evidence in the production monolith and
  resolve settled aliases from the persistent capture journal and gallery.
- Route Firefox evidence checks through the Monas-authenticated Pinakotheke
  product boundary instead of the absent standalone cache route.

## 1.22.0 - 2026-07-19

- Query authoritative cache-alias evidence for displayed media independently of
  substitution, and apply the non-destructive two-pixel green stored frame.
- Correlate user-selected videos with their page elements and expose bounded
  selected, downloading, pending, stored, and failed status in the toolbar.
- Remove the page-wide activation fallback that could admit autoplay after an
  unrelated click, and follow ObjectStore settlement for up to three minutes.

## 1.21.0 - 2026-07-19

- Extract a bounded WebP poster from every Firefox-compatible committed video
  and settle it as a separate checksum-verified DASObjectStore object.
- Populate video cards with the captured frame plus duration, dimensions,
  codecs, playback profile, and Firefox playback evidence.

## 1.20.4 - 2026-07-19

- Derive normalized-video availability and ObjectStore provenance from the
  committed video representation rather than an optional poster thumbnail.
- Stop reporting a range-readable committed MP4 as unavailable merely because
  no separate poster object was generated.

## 1.20.3 - 2026-07-19

- Preserve Firefox's user-action eligibility by invoking the X media-origin
  permission request synchronously from the toolbar button handler.
- Cache the asynchronously inspected permission state while rendering, before
  the user clicks, so no awaited operation precedes the request.

## 1.20.2 - 2026-07-19

- Treat the exact X video CDN origin as an explicit Firefox runtime permission.
- Request that permission when X video ingress is enabled and let the toolbar
  repair existing installations before another user-played capture attempt.
- Show a clear permission-required state instead of silently missing worker
  manifests.

## 1.20.1 - 2026-07-19

- Retain worker-owned X manifests even when Firefox reports no owning tab, then
  correlate them to trusted playback by the stable X media-family identity.
- Add bounded URL-free diagnostics for manifest observation and trusted-play
  resolution without recording browsing or media URLs.

## 1.20.0 - 2026-07-19

- Observe only completed X media-CDN HLS/DASH manifest URLs for explicitly
  enabled X video capture, retaining at most 32 URLs per tab for two minutes.
- Resolve worker-fetched playlists that are absent from page Resource Timing
  without reading headers, cookies, credentials, or response bodies and without
  blocking or modifying origin traffic.

## 1.19.2 - 2026-07-19

- Version the injected Firefox page-observer marker so an extension upgrade
  activates the new observer in already-open eligible tabs instead of exiting
  because a prior release left its legacy marker behind.

## 1.19.1 - 2026-07-19

- Admit a visible video play for capture after a recent genuine page pointer or
  keyboard activation when X's overlay controls cannot be mapped to the exact
  ``video`` element. Autoplay without a trusted activation remains ineligible.

## 1.19.0 - 2026-07-18

- Accept Monas host context bound to the verified Prosopikon authority,
  immutable principal, immutable session, and exact ``pinakotheke`` audience.
- Require canonical identity fields as an all-or-none group and reject wrong
  audiences, nil/non-canonical UUIDs, partial assertions, and unknown fields.
- Preserve the legacy ``actor_id`` and original v1 context shape only as an
  explicit extension-pairing compatibility path pending service-session
  migration; canonical UUIDs are assertions, never bearer credentials.

## 1.18.0 - 2026-07-18

- Resolve the master HLS/DASH manifest belonging to the fragmented media family
  observed after a trusted play, while refusing to treat ``.m4s`` fragments as
  standalone progressive video.
- Assemble clear HTTPS segmented audio/video through a five-minute, one-GiB,
  two-hour-duration bounded FFmpeg process before the existing probe,
  DASObjectStore completion, and gallery admission path.

## 1.17.2 - 2026-07-18

- Associate a trusted click on an overlaid video control with the visible video
  beneath its pointer coordinates, then require that exact video to emit a real
  play event within two seconds before capture.
- Record bounded observer diagnostics when a play event lacks a usable source
  or recent trusted activation.

## 1.17.1 - 2026-07-18

- Inject the persistent capture observer into eligible tabs that are already
  open when the extension is installed, updated, started, or resynchronised.
  This restores trusted play capture in long-lived single-page applications
  without requiring a page reload.

## 1.17.0 - 2026-07-18

- Detect user-played progressive video fetched by page script as well as a
  native video element, including recent ``video.twimg.com`` MP4 resources.
- Poll bounded resource timing for two seconds after the trusted play event so
  late-starting progressive media can enter the existing capture, settlement,
  normalization, and gallery path without observing cookies or headers.

## 1.16.1 - 2026-07-18

- Report the composed remote DASObjectStore read authority as ready and make
  aggregate monolith readiness become ready when both Monas authentication and
  DASObjectStore authority are configured, preventing a healthy backend from
  being rejected by Monas after upgrade.

## 1.16.0 - 2026-07-18

- Carry normalization probe metadata and the committed poster receipt through
  the isolated capture worker into persistent video catalogue records.
- Show duration, dimensions, codecs, playback profile, capture/source context,
  and worded Firefox-ready state in the dedicated video cards and quick viewer.
- Extend installed-Firefox assurance across the Videos filter, metadata UI,
  responsive/keyboard behavior, range seeking, concurrent reads, conditional
  delivery, cancellation, pause/resume, and missing-object handling without a
  source-site request.
- Assemble the automatic thumbnail, trusted opened image/video, verified
  settlement, restart-safe gallery, and Firefox playback checks into one
  repeatable critical-vertical release gate.

## 1.15.0 - 2026-07-18

- Add a site-neutral bounded planning contract for user-played HLS, DASH, and
  MSE media using only canonical manifest and already-observed segment hashes.
- Prove deterministic retry identity, ordered segment bounds, aggregate byte
  limits, policy/DRM/encryption/authorization blocks, redacted codec/container
  diagnostics, and fail-open origin playback with redistributable fixtures.

## 1.14.0 - 2026-07-18

- Hand incompatible progressive video from the bounded capture helper directly
  to the existing DASObjectStore-host, digest-pinned, network-isolated normalizer.
- Commit the normalized MP4, WebP poster, and expanded provenance manifest as
  separate verified DASObjectStore objects before gallery settlement.
- Persist only aggregate container/codec/count/time gap metadata, with no site,
  page, media URL, title, browser credential, or payload data.
- Bind capture-helper provenance to the authenticated host actor while retaining
  compatibility for legacy v1 helper requests.

## 1.13.0 - 2026-07-18

- Persist reviewed endpoint/ObjectStore selections per authenticated actor in
  strict private metadata with optimistic revisions and restart-safe migration.
- Add a Monas-authenticated destination GET/PUT API that accepts only the exact
  pair already authorized by the host and never trusts browser inventory as
  storage authority.
- Replace session-only Yew selection with explicit accessible Save, saved,
  unsaved, conflict, permission, invalid-response, and unavailable states.
- Bind every new capture plan to the actor's saved endpoint, ObjectStore, and
  selection revision; legacy unbound plans remain visible but cannot execute.
- Require a strict host destination callback before acquisition and external
  completion, rejecting changed identity, missing authority, untrusted TLS,
  expired pairing, unavailable/read-only stores, unsupported media, and empty
  quota without choosing a fallback.

## 1.12.0 - 2026-07-17

- Detect user-played opaque progressive HTTPS video resources within a bounded
  trusted-activation window on any explicitly enabled origin.
- Preserve a validated exact retrieval capability privately through the
  isolated acquisition helper while retaining query-free canonical identity
  for idempotency and catalogue provenance.
- Refresh rotating retrieval capabilities on unsettled plans without creating
  duplicate media plans or exposing signed queries through APIs or logs.

## 1.11.0 - 2026-07-17

- Terminate HTTPS directly in the Pinakotheke Axum service with Rustls using an
  explicit certificate-chain/private-key pair; nginx is no longer required.
- Add an authoritative root-level deployment guide for trusted local and public
  certificates, direct Axum HTTPS, rotation, verification, and proxy removal.
- Serve reviewed signed Firefox packages from the direct Axum listener so
  removing the proxy does not remove extension installation downloads.
- Preserve graceful SIGTERM shutdown for direct TLS and local HTTP listeners so
  service restarts release the capture-worker lease cleanly.
- Deploy the no-nginx topology with Monas/Rustls owning public HTTPS and
  Pinakotheke remaining the authenticated loopback product backend.

## 1.10.0 - 2026-07-17

- Permit trusted-play progressive video plans from any explicitly enabled
  HTTPS origin while retaining bounded credential-free retrieval, provenance
  validation, codec checks, and DASObjectStore authority.
- Add a dedicated playable-video library filter and improve the quick viewer
  with inline playback and a committed poster.
- Define the site-neutral video campaign without embedding a website catalogue
  in code, manifests, fixtures, or documentation.

## 1.9.0 - 2026-07-17

- Capture a concrete progressive X MP4 only after a trusted user activation
  starts playback, with explicit diagnostics for segmented or unresolved media.
- Require the first-party acquisition helper to verify the bounded MP4 as the
  Firefox H.264/AAC playback profile before DASObjectStore commit and immediate
  gallery admission.
- Report opened-video activity separately from opened images.
- Remove transfer-only capture success: every image/video upload must complete
  through DASObjectStore daemon verification and authoritative catalogue
  publication.
- Submit the reviewed logical ObjectStore ID and provider binding to the
  DASObjectStore daemon, leaving verification and catalogue settlement to the
  storage authority.
- Permit the DASObjectStore service group to read bounded native staging files
  from a dedicated setgid ``TMPDIR`` until verified daemon completion.

## Unreleased

- Add an authenticated, prefix-bounded gallery-folder projection with
  breadcrumbs, immediate child folders, item counts, and latest-capture times.
- Make the library landing view a graphical list of the latest 20 downloads;
  folder selection filters cards through the server across
  ``x.com/<artist>/<capture-class>`` without exposing host filesystem paths.
- Replace the serialized gallery read boundary with a 128-slot, backpressured
  Axum delivery pool; provider lookup, object retrieval, and checksum
  verification run on Tokio's blocking workers rather than request threads.
- Version gallery image URLs by committed checksum and permit private browser
  reuse for one hour, avoiding repeat DASObjectStore retrieval when revisiting
  an unchanged library.
- Lead thumbnail cards with the captured X account and UTC capture time;
  newly admitted X records also carry an explicit X-account source identity.
- Report bounded visible-image snapshots from the exact-origin Firefox content
  observer so X mutations and image-navigator changes trigger automatic
  thumbnail planning without a second privileged DOM execution.
- Verify complete legacy DAS S3-export objects against their committed SHA-256
  before gallery delivery when optional authority checksum metadata is absent;
  ranged reads remain fail-closed without that metadata.
- Accept the already-canonical X image ``format``/``name`` variant as a safe
  reconciliation alias, allowing a verified DASObjectStore commit to settle
  into the gallery while continuing to reject arbitrary or signed queries.
- Apply that same canonical rule when reopening the capture-plan journal so a
  safe X variant remains restart-recoverable.
- Admit a verified explicitly opened image as an original-first gallery card;
  its existing DAS object supplies the initial thumbnail delivery role until a
  separately observed thumbnail is available, without duplicating bytes.
- Add a bounded, credential-free Firefox diagnostic event ring with visible
  recent events and JSON export, covering observer registration, viewport
  scans, eligibility counts, plan HTTP outcomes, and stored/pending status.
- Emit structured Pinakotheke service logs when capture plans are admitted,
  acquisition helpers fail, settlement fails, or gallery admission completes;
  retain the helper's bounded error class instead of silently discarding it.
- Normalize acquisition-helper failures to bounded protocol outcomes so the
  journal reports policy-blocked, unavailable, or rejected instead of a JSON
  parsing error.
- Preserve X CDN ``format`` and ``name`` variants while continuing to discard
  arbitrary query parameters, so DASObjectStore acquisition fetches the image
  Firefox actually displayed instead of a queryless 404 URL.
- Run automatic thumbnail capture even when legacy pairing state lacks an
  instance identifier; only substitution needs that identifier. Separate
  capture and substitution diagnostics so a segmented-video fallback cannot
  hide image-ingress progress in the toolbar.
- Add an authenticated, live ingress-status strip to the Pinakotheke library
  and distinguish observed thumbnails, explicitly opened images/videos,
  pending acquisitions, verified stores, and gallery admission.
- Submit the image bytes rendered by Firefox when a user opens an X image,
  retaining the enclosing status link only as presentation provenance instead
  of incorrectly attempting to ingest the HTML page as image content.
- Store captured images in logical DASObjectStore namespaces grouped as
  ``x.com/<account>/<capture-kind>/<sha256>`` (or an honest unattributed
  quarantine) and prove that visible, unclicked X thumbnails submit automatic
  capture plans with their account-bearing presentation provenance.
- Show the manifest-derived Firefox extension version in the toolbar popup and
  use the canonical Pinakotheke product name there.
- Automatically observe eligible visible images on enabled pages, poll an
  actor-scoped verified-capture status, and add a two-pixel green browser-only
  frame only after the object is admitted to the live gallery.
- Exclude X interface and emoji hosts from X-ingress thumbnail observation,
  preserve logical ObjectStore provenance while routing to its S3 export
  bucket, and provide an explicit scoped DAS remote-client compatibility mode
  for appliances whose daemon cannot yet resolve that mapping.
- Added an authenticated actor-scoped site corpus with persistent revisions,
  stale-write conflicts, first-pair migration, and Firefox startup/settings
  synchronization across extension upgrades and browser profiles.
- Added Monas-authenticated, DASObjectStore-gated Firefox download and pairing
  onboarding with live server verification and explicit X-ingress intent.
- Use a narrow revocable Monas pairing credential for extension requests so
  browser SameSite cookie protections remain intact.

- Replace Firefox's generic puzzle-piece fallback with correctly scaled,
  black-and-white Mnemosyne Biosciences icons for toolbar and add-on surfaces.
- Adopt the product-aware, Mnemosyne-compliant Pinakotheke sign-on delivered by
  Monas 0.6.2 while preserving Monas/Prosopikon authentication ownership.
- Add an AMO-compatible unlisted Firefox signing workflow with pinned Mozilla
  validation, environment-only credentials, and signed-XPI identity checks.
- Add isolated Firefox acceptance that requires a Mozilla-signed XPI to install
  permanently with the stable Pinakotheke extension identity.
- Recover accidental plaintext requests to the private-LAN TLS installation
  port by redirecting to the identical canonical HTTPS URL.
- Correct the Firefox manifest's background and permission declarations and
  disclose the user-directed browsing, website-content, and save-action data
  transmitted to the configured Pinakotheke service.

All notable changes to Pinakotheke will be documented in this file. The project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed

- Declare DASObjectStore as an external DEB/RPM dependency and macOS PKG
  prerequisite while explicitly keeping its executables, configuration,
  credentials, services, and object data out of Pinakotheke package payloads.

- Make the hardened FFmpeg bind mount valid on Linux Docker Engine by using
  the ``--mount`` key/value form and run the capability-free container as the
  private scratch directory's numeric owner, preserving mode-0700/0600 access
  without granting DAC override capability or widening host permissions.

### Added

- Document and prove a private-LAN HTTPS deployment using a locally trusted CA,
  loopback Monas proxying, TLS 1.3, and a real Firefox temporary-XPI install.

- Load the Monas-authenticated DASObjectStore inventory in the Pinakotheke
  library and present a keyboard-accessible endpoint/ObjectStore selector with
  explicit Ready, read-only, and unavailable states.

- Implement the packaged ``ingest-stream-v1`` DASObjectStore helper used by
  video normalization, with exact-length stdin framing, streaming checksum,
  native or fixed-container daemon completion, bounded private scratch, strict
  typed receipts, and cleanup on every outcome.
- Add a packaged ``pinakotheke video normalize`` host command that accepts one
  private confirmed plan, runs the digest-pinned network-isolated Docker/FFmpeg
  adapter, streams video/poster/manifest bytes to a reviewed DASObjectStore
  helper over stdin, verifies exact receipts, kills unfinished children, and
  removes bounded scratch on every outcome.
- Allow each managed local profile lifecycle command to select an explicit
  loopback DASObjectStore API port while retaining port 3900 as the default,
  and strictly bind discovery to the requested authority.
- Complete the isolated XIMG-094 clean-home proof across Monas login, exact
  return, direct-backend rejection, DASObjectStore readiness, verified commit
  and scoped read-back, restart reconciliation, logout, and clean shutdown at
  DASObjectStore ``f195c4d5``.
- Add a strict local-Docker capture transport that maps only private
  DAS-managed scratch into the fixed authority service, supplies scoped
  credentials by private file path, and removes the job directory on every
  outcome.
- Record the live authority defects resolved by DASObjectStore ``720ae9c1``:
  isolated port alignment and canonical folder-profile capacity binding.
- Prove duplicate container-side captures, scoped checksum-verified readback,
  and restart persistence against DASObjectStore ``b35ee0b2``; record the
  remaining same-home Monas composition and media-type preservation gates.
- Preserve the capture helper's verified image MIME type through native and
  container uploads using DASObjectStore's validated single-object content-type
  contract at sibling commit ``7a3d5578``.
- Resume the isolated XIMG-094 authority proof, fix DASObjectStore's obsolete
  completion-smoke upload syntax at sibling commit ``03f88237``, and retain the
  exact-revision rerun gate after Docker BuildKit reported a metadata I/O fault.

- Record and validate the DASObjectStore container-side remote-completion
  prerequisite that removes Docker Desktop bind-mounted Unix sockets from the
  clean-home authority path.

- Add the packaged first-party DASObjectStore read helper with explicit
  endpoint/ObjectStore-to-bucket authority, host-owned AWS credentials,
  completion-metadata verification, conditional and ranged reads, and bounded
  automatically removed scratch.

- Add isolated installed-Firefox acceptance for an observed linked thumbnail
  and trusted explicitly opened original through the production capture path.

- Define XIMG-096 as the non-waivable Firefox-to-DASObjectStore-to-gallery
  stable-release gate and add its first bounded, Monas-authenticated gallery
  catalogue boundary with explicit object availability and no origin fallback.

- Replace the Yew gallery's synthetic cards and proxy visuals with the canonical
  Monas-forwarded catalogue, real authorized media elements, source-derived
  filters, and explicit safe failure states.
- Persist the Monas gallery's metadata-only ObjectStore references in a strict,
  bounded, private, atomically replaced document loaded by the local monolith,
  with fail-closed schema/corruption handling and restart coverage.
- Bridge verified Firefox image acquisitions through website provenance and the
  common review queue into persistent gallery cards, with idempotent thumbnail
  admission and independently committed explicit-original enrichment.
- Resolve persistent gallery image paths under private Monas dispatch and
  stream only the exact catalogue-authorized DASObjectStore object through a
  host backend with strict MIME, length, checksum, and ETag validation.
- Persist only Firefox-proven ready normalized videos as poster-backed gallery
  cards and deliver their exact rendition through authenticated, validated
  single-range playback without local bytes or origin fallback.
- Add bounded server-side persistent-gallery filters, honest matched/total
  counts, and query-preserving incremental Yew pagination instead of silently
  truncating the real library at 200 cards.
- Window the responsive persistent gallery to visible plus overscan rows while
  retaining virtual scroll height and roving Arrow/Home/End keyboard traversal
  across off-screen loaded records.
- Add the runnable Trunk/WebAssembly gallery build and serve its bounded,
  symlink-free output from the canonical application mount only through private
  Monas dispatch admission.
- Add installed-Firefox acceptance for the compiled Yew gallery with 1,000
  mixed records, bounded windowing, off-screen keyboard focus, server filters,
  unavailable/no-origin behavior, and desktop/narrow responsive reflow.
- Install the compiled Yew/WASM gallery in Linux and macOS native packages and
  embed its platform path in the monolith, retaining validated per-user and
  explicit development overrides.
- Compose a strict process-isolated host object-reader into the packaged
  monolith, with bounded streaming, exact metadata/length/full-read checksum
  verification, range support, and no local payload or credential handling.
- Bind a managed macOS object-read helper to one explicit reviewed endpoint
  identity, reject half-configured installs, and pass the fixed non-secret scope
  through the backend agent environment instead of deriving it from requests.
- Mount the Firefox capture-plan API in the runnable Monas-authenticated
  monolith from a strict private metadata-only pairing/site authority document,
  including per-user launchd composition and fail-closed validation.
- Route Firefox capture planning and gallery navigation through the canonical
  Monas Pinakotheke product mounts instead of obsolete standalone paths.
- Persist accepted Firefox capture plans in a bounded private atomic journal,
  restore actor-scoped pending work and daily budgets after restart, return
  idempotent retries, and expose an authenticated pending-plan reconciliation
  view without media bytes or credentials.
- Add a separately credentialled host-worker completion boundary that admits
  exact verified DASObjectStore image evidence through acquisition,
  reconciliation, persistent review, and the live gallery, with restart-safe
  idempotent settled markers and no payload bytes.
- Add a run-one capture acquisition command and strict executable protocol for
  host-owned permitted retrieval, direct DASObjectStore streaming/verification,
  and metadata-only settlement without shell execution or local payload files.
  Foreground/service and offline workers share an exclusive local mutation
  lease so long-running acquisition cannot overlap journal writes.
- Compose the reviewed acquisition helper into foreground and launchd monolith
  operation with prompt browser admission, serialized background execution,
  per-plan retry coalescing, live verified settlement, and pending-on-failure.
- Recover durable unsettled captures automatically on monolith startup after
  revalidating current actor pairing, expiry/revocation, site policy, adapter,
  and capture-kind authority; withdrawn work remains pending and untouched.
- Enable the experimental generic Firefox adapter for explicitly opted-in HTTPS
  origins and admit originals only from trusted image-link/image-document
  clicks, using sender-tab provenance and no automatic opening or traversal.
- Persist the trusted explicit-original observer as a Firefox dynamic content
  script scoped to enabled image origins, restoring it on startup/update and
  unregistering it immediately when capture is paused or permission removed.
- Add a first-party packaged capture helper that bounds HTTPS image retrieval,
  uses private ephemeral scratch, submits checksum-addressed uploads through
  DASObjectStore's daemon completion path, and emits verified metadata only.
- Correlate linked observed thumbnails with distinctly addressed opened
  originals through a canonical presentation identity owned by the capture
  plan, with non-destructive legacy-journal fallback.
- Carry an explicit positive immutable object version from verified acquisition
  and normalized-video evidence through persistent gallery records, authorized
  read grants, and host-helper requests, with legacy catalogue-v1 records
  defaulting non-destructively to version 1.
- Add the XIMG-200 Synoptikon product manifest, strict tenant/account/project
  host scope, and bounded authenticated catalogue projection over immutable
  DASObjectStore references.
- Add the first XIMG-090 ``pinakotheke serve`` monolith slice with a private
  per-user metadata root, loopback-safe Axum listener, graceful shutdown, and
  honest component readiness.
- Add XIMG-091's reviewed local DASObjectStore profile plan, authority-owned
  provisioning and discovery, stable secret-free destination selection, and
  honest monolith storage readiness, with bounded actionable authority failure
  diagnostics and an isolated-home Docker acceptance proof.
- Add XIMG-092's first fail-closed Monas dispatch ingress: a private
  process-local credential admits only strict non-secret host context, strips
  dispatch headers before product handling, and leaves browser sessions wholly
  owned by Monas/Prosopikon.
- Complete XIMG-092 against Monas ``0.2.0`` with the authenticated canonical
  product mount, streaming loopback forwarding, cookie stripping, strict host
  context injection, and session revocation proof.
- Add XIMG-093's non-root macOS launchd lifecycle for the coordinated
  Pinakotheke backend and Monas host, including private credential generation,
  guarded transactional replacement, status/restart/log access, and
  data-preserving uninstall.
- Complete XIMG-095 with the Monas ``0.3.0`` host-owned login/Yew shell,
  same-origin Pinakotheke deep-link return, Prosopikon session lifecycle, and
  approved Mnemosyne branding asset boundary.

## [1.0.0] - 2026-07-15

- Add an executable Pinakotheke v1 preflight and no-partial-cutover release
  gate that reports canonical identity blockers without exposing private data.
- Prepare one shared clap implementation for the canonical ``pinakotheke``
  command and behavior-preserving, warning-emitting ``x-img`` legacy alias;
  0.9 packages continue to install only the legacy command.
- Add validated, inert Pinakotheke 1.0 Monas and DASObjectStore registration
  candidates while preserving the active x-img registrations and historical
  authority identity.
- Preserve Firefox pairing, site policy, endpoint, and ObjectStore state across
  extension updates, and add a canonical manifest candidate retaining the
  shipped Gecko update identity and least-privilege permission surface.
- Parameterize native, Firefox, SBOM, checksum, and artifact-manifest packaging
  for a version-locked Pinakotheke cutover while retaining x-img as the 0.9
  default and future CLI compatibility alias.
- Add an isolated, compile-tested rehearsal of the coordinated Pinakotheke 1.0
  identity cutover while leaving the live 0.9 repository unchanged.
- Prove dual-architecture DEB/RPM transition from x-img 0.9.0 to canonical
  Pinakotheke 1.0.0 and rollback with exact metadata preservation.
- Extend the isolated v1 cutover rehearsal through the complete renamed
  quality, audit, fault-recovery, contract, and canonical packaging toolchain.
- Complete the coordinated Pinakotheke identity migration across the Rust
  workspace, active authority contracts, Firefox identity, packaging defaults,
  documentation, and canonical public repository while retaining compatibility.

### Fixed

- Reconcile a repeated local-profile start only when DASObjectStore can still
  rediscover the exact reviewed Ready endpoint/ObjectStore identity, allowing
  safe recovery from an already-running nested provider without accepting a
  changed or unavailable authority.

- Declare both Firefox ``background.scripts`` and cross-browser
  ``background.service_worker`` in extension manifests, and generate valid
  port-independent WebExtension match patterns for non-default-port HTTPS
  origins while retaining the exact origin in policy and provenance.

## [0.9.0] - 2026-07-15

### Added

- Added a deterministic CycloneDX 1.6 release SBOM covering locked Rust
  dependencies and the Firefox extension component.
- Published 0.9.0 as an explicitly unsigned evaluation release candidate with
  local package, rollback, audit, contract, and documentation evidence.

## [0.3.0] - 2026-07-15

### Planning

- Added XIMG-086's local package lifecycle and metadata rollback acceptance
  across genuine 0.2.0/0.3.0 DEB and RPM transitions on x86_64 and arm64 using
  digest-pinned Debian/Fedora containers and pinned Monas plus DASObjectStore
  contracts.
- Added the XIMG-085 packaging foundation: a Makefile for cross-linked Linux
  DEB/RPM, dual-architecture macOS PKG, deterministic six-label Firefox XPI,
  SHA-256 manifests, a deterministic typed release-artifact inventory, and
  strict source/artifact verification.
- Added XIMG-084's single-command local release audit covering privacy,
  security, accessibility, extension permissions/CSP, licenses, locked Rust
  dependencies/advisories/sources, JavaScript syntax, and version mirrors.
- Added XIMG-083's bounded redacted operations model and Axum surfaces. Public
  health is coarse; host-authenticated snapshots expose only typed component
  state, saturating aggregate counters, fixed audit codes, and eviction count.
- Added XIMG-082's approved deletion/compliance reconciliation contract.
  Catalogue tombstoning is separate from exact DASObjectStore removal,
  approval scope is mandatory, pending retries converge, mismatched authority
  evidence conflicts, and only verified removal becomes terminal.
- Added XIMG-081's strict metadata snapshot boundary and native migration,
  export, and restore proofs. Checksummed copy-on-write artifacts preserve
  catalogue/ObjectStore identities, reject corruption and future schemas, and
  require explicit reviewed Firefox re-pairing without containing secrets or
  media bytes.
- Added XIMG-080's deterministic fault/recovery acceptance suite. Nine
  synthetic cases prove bounded ingest, idempotent crash reconciliation,
  destination stability, scheduler cancellation, normalizer cleanup, cache and
  capture authority failures, and real-Firefox substitution fail-open behavior.
- Added XIMG-074's Firefox cache popup with per-site pause/resume, explicit run,
  current hit/miss/error reason, host source-view link, clear permission text,
  and accessible ``Previously observed`` versus ``Stored in ObjectStore``
  evidence. Diagnostics are one bounded URL-free record per configured origin.
- Added XIMG-073's fail-closed segmented-video adapter gate. HLS/DASH requires
  exact versioned manifest/segment canonicalization, synthetic fixtures, real
  Firefox evidence, explicit-open evidence, no DRM/encryption, and a matching
  Ready normalized profile. Generic manifest/MSE sources stay origin-served
  with a bounded URL-free Firefox diagnostic.
- Added XIMG-072 normalized MP4 external-cache delivery. Exact reviewed video
  records support authenticated full, conditional, and single-range streams;
  the opted-in Firefox path uses native video playback and restores the origin
  once on failure. Real Firefox proves concurrent ranges, cancellation, seek,
  pause/resume, conditional requests, and fallback with ephemeral media.
- Added XIMG-071's host-authenticated image substitution path. Stable delivery
  IDs preserve the exact reviewed ObjectStore identity; the Firefox client
  validates MIME, length, checksum ETag, size, CORS/CORP delivery, and restores
  the origin once on any failure. A real Firefox loopback harness proves
  replacement and CSP/CORS/type/length/ETag fail-open without retained media.
- Added XIMG-070's bounded cache-alias index and host-authenticated lookup
  endpoint. Immutable ObjectStore hits require server-owned same-instance,
  origin, adapter, pairing, expiry, substitution, and observation/open gates;
  signed queries are rejected without echo, authority failures explicitly fall
  back to origin, and the 4,096-entry synthetic p95 is measured below 2 ms.
- Added XIMG-053's Mnemosyne-aligned quick preview task pane. It provides
  selected-record context, alt text, fit/original visual state, keyboard focus
  handling, explicit unavailable-object behavior, and only attaches native
  video controls to a ready normalized ObjectStore playback route—never an
  origin fallback or browser-retained payload.
- Completed XIMG-069's direct authenticated normalized-video delivery route
  and Firefox proof. Ready, actor-bound ObjectStore grants preserve
  single-range, MIME, length, ETag, conditional, and unavailable semantics
  without origin fallback or a dependency on later Firefox cache substitution.
  The local Firefox harness uses only an ephemeral normalized file and cleans
  its browser profile after verifying metadata, range, seek, pause, and resume.
- Added XIMG-068's paired-device Docker FFmpeg adapter. It requires an
  approved pairing, digest-pinned image, resource limits, isolated scratch,
  structured network-disabled invocation, bounded ObjectStore ingest, probe
  and checksum provenance, cleanup, cancellation, and crash-reconciliation
  states. It leaves every rendition Awaiting Firefox playback rather than Ready.
- Added XIMG-067's two immutable normalized-video candidate profiles: WebM
  VP9/Opus or AV1/Opus and MP4 H.264/AAC. Docker-first plans pin an image digest,
  resource limits, authorized DASObjectStore/paired-device/future-Keryx worker
  placement, and managed-or-bounded scratch. A rendition cannot become Ready
  without typed checksummed ObjectStore derivatives and matching Firefox
  profile evidence; source-only objects remain explicitly non-ready.
- Added XIMG-066's explicit, metadata-only video candidate planner. Observed
  or user-selected, policy-eligible candidates expose review details and need
  confirmation before later work; DRM, unsupported segmented delivery,
  unobserved candidates, and non-video destinations are blocked. It records
  aggregate origin/container/codec occurrence gaps for normalization priority
  without publishing media, URLs, cookies, credentials, or browsing history.
- Added XIMG-065's verified website-capture provenance handoff to the shared
  review queue. It records site/page/media/adapter evidence only after a
  verified ObjectStore commit, rejects premature or mismatched acquisitions,
  and reuses a matching committed connector identity rather than creating a
  duplicate review item.
- Added XIMG-064's host-authenticated Firefox viewed-media capture-plan
  boundary. It admits bounded, actually viewport-displayed thumbnail metadata
  only after paired actor, exact-origin policy, adapter, and scheduler checks;
  query components are redacted and no browser payload, source cookie,
  authorization header, form data, or credential is accepted. The extension
  remains fail-open and does not automatically open originals.
- Reframed Instagram as an explicit Firefox site-policy path; the dedicated
  Instagram API connector is now optional future work.
- Added XIMG-061's host-mediated extension pairing contract.
- Added XIMG-062's explicit per-site Firefox policy controls.
- Added XIMG-063's explicit, versioned Firefox site-adapter registry.
- Added XIMG-054's review states, reversible batch actions, and ObjectStore
  original versus observed-thumbnail status treatment.
- Added XIMG-055's per-account refresh progress and retry presentation.
- Added XIMG-056's keyboard-operable client-side metadata search.
- Added XIMG-060's least-privilege Firefox Manifest V3 scaffold.
- Added XIMG-051's selected-context X/Instagram/website source navigation.
- Added XIMG-050's Mnemosyne-compatible Yew/Monas shell.
- Added XIMG-046's verified-commit-only new-item review admission queue.
- Added XIMG-045's one-click social-account refresh orchestration contract:
  enabled X/Instagram accounts become coalesced global jobs with bounded
  per-account progress, partial failure, cancellation, retry, no-overlap, and
  final new-item summary states, without connector execution or media transfer.
- Added XIMG-044's fixture-only incremental Instagram discovery planner. It
  handles posts, carousels, reels/videos, pagination and budgets; exposes
  opaque credential expiry/revocation; selects supported renditions; preserves
  provenance; and feeds reconciliation idempotency without an API call, media
  transfer, or XIMG-043/ADR 0002 gate bypass.
- Added XIMG-042's fixture-driven, incremental X media-discovery planner. It
  enforces cursor/page/item/depth budgets, selects supported best photo/video/
  animated-GIF variants, preserves metadata provenance, and produces existing
  idempotency requests without a live X call, media transfer, or ADR 0002
  policy bypass.
- Added XIMG-041's explicit followed-X-account selection boundary. A
  grant-bound returned candidate list becomes a reviewable added/existing/not-
  selected configuration diff and requires confirmation before the existing
  atomic configuration save; it neither bulk-enables accounts nor makes live X
  traffic while ADR 0002 remains open.
- Added XIMG-040's official X OAuth 2.0 Authorization Code + S256 PKCE host
  boundary. State replay, denial, expiry, required scopes, viewing-account
  binding, opaque refresh, and revocation are test-covered without raw tokens,
  cookies, secrets, or live X API/media traffic.
- Added XIMG-038's confirmed direct bioinformatics commit orchestrator. It
  requires allowed explicit plan confirmation and exact destination
  revalidation, streams bounded chunks through the DAS ingest port, verifies
  SHA-256/length/authority receipt, and records idempotent metadata provenance
  without durable local payloads or live transport.
- Added XIMG-037's reviewed destination row and commit-time revalidation
  contract. It exposes all discovered stores in structured status-word rows and
  refuses removed, renamed, unavailable, read-only, expired, over-quota,
  untrusted-TLS, reconnect, and cross-endpoint alias states without fallback.
- Added XIMG-036's strict endpoint/device and logical ObjectStore inventory
  contract. Synthetic fixtures prove managed local profile and paired remote
  discovery selection while rejecting unmanaged folders, mutable identities,
  broad secrets, and arbitrary first-store selection; no live pairing,
  credential, or transport integration is enabled.
- Made hosted CI non-blocking while GitHub Actions funding is unavailable.
  GitHub workflows are manually dispatched advisory mirrors; recorded local
  verification remains the delivery authority until CI is backfilled or moved.
- Added XIMG-035's dependency-free cross-repository contract check and pinned
  optional sibling-source workflow. A public clone validates x-img-owned
  fixtures without sibling dependencies; an explicit four-repository checkout
  verifies exact compatibility revisions and contract anchors without
  credentials or a claim of live authority integration.
- Added XIMG-034's authorized DASObjectStore object-read/cache handoff port.
  It validates content type, lengths, SHA-256, ETag, conditional and byte-range
  metadata, and explicit unavailable states before returning a stream, without
  persisting media locally.
- Added XIMG-033's bounded streaming object-ingest port. It forwards chunks to
  an authority backend without local payload staging, enforces chunk/length and
  SHA-256 verification, surfaces backpressure, checks authority completion, and
  returns only an idempotent verified receipt for a repeated ingest ID.
- Added XIMG-032's scoped DASObjectStore application-identity contract and
  authorization gate. It binds one endpoint, ObjectStore, prefix, narrow
  operation set, byte limits, expiry, and opaque authority references; tests
  fail closed for expired, replayed, wrong-store, wrong-prefix, and oversized
  operations without storing credentials or issuing tokens.
- Added XIMG-031's authenticated host-context adapter. Privileged Axum routes
  require a host-injected, authorized non-secret context and reject direct
  access; fixture-tested Monas and Synoptikon adapters share the same boundary
  without x-img accepting, logging, configuring, or issuing sessions.
- Added XIMG-030's versioned Monas product registration and synthetic strict
  fixtures. It requires host-owned Prosopikon authentication, one x-img
  application/API mount, a DASObjectStore requirement, capability disclosure,
  and a future Synoptikon-equivalent bootstrap; it rejects anonymous access and
  direct x-img login-route declarations without adding a live host dependency.
- Added the XIMG-020 Rust 2024 workspace scaffold at product version 0.2.0:
  shared model and core crates, a `clap` CLI reporting the workspace version,
  an Axum composition boundary, and a Yew WebAssembly boundary. The pinned
  Rust 1.97.0 toolchain, Rust 1.95 MSRV metadata, lint policy, MPL notices, and
  lockfile are in place; no live source, storage, authentication, or media
  payload integration is enabled.
- Added XIMG-021 strict local configuration parsing, validation, safe atomic
  replacement, and identifier-only listing for versioned X, Instagram, and
  website rules. It rejects unknown fields and schema majors, duplicate source
  identities, invalid account names, unsafe wildcard/non-origin websites, and
  missing/incompatible opaque authority references without contacting any
  source or authority.
- Added XIMG-022's platform-neutral acquisition state machine. It accepts only
  the verified settlement path, makes failure/policy/cancellation/conflict
  terminal, requires bounded immutable ObjectStore evidence before commit, and
  prevents every review state from being assigned before a verified commit.
- Added XIMG-023 deterministic idempotency and reconciliation: canonical media
  identity plus immutable checksum settles one metadata record, replay appends
  safe aliases without replacing an object reference, and absent/mismatched
  authority observations remain pending or conflict without any byte, network,
  or persistent-storage operation.
- Added XIMG-024 in-memory scheduler contracts for coalesced global refreshes,
  mutually exclusive source leases, bounded child/request/byte/time budgets,
  and cooperative cancellation. No connector, queue, persistence, or network
  execution is enabled.
- Added XIMG-025's strict synthetic X/Instagram connector-fixture matrix and
  test contract, covering pagination, edits, deletions, duplicates, variants,
  rate limits, authorization expiry, malformed responses, and cursor reset
  without live traffic, account data, credentials, or media payloads.
- Added XIMG-026 metadata-only GEO/SRA/ENA/NCBI transfer plans with one
  explicit accession/URL, bounded resolved file evidence, transport, rights,
  endpoint/ObjectStore destination, policy blocking, and explicit confirmation;
  no repository discovery or byte transfer is enabled.
- Added XIMG-027 synthetic bioinformatics resolution/transport fixtures for
  ENA/SRA, GEO, NCBI, checksum/size evidence, retry/resume/cancellation,
  backpressure, and optional Aspera-to-HTTPS fallback without payloads or
  credentials.
- Added the XIMG-009 executable Pinakotheke v1 identity-migration plan,
  including a complete rename surface matrix, a minimum compatibility window,
  migration and rollback rules, retained schema/object/extension identities,
  and required configuration, catalogue, ObjectStore, and pairing proof cases.
- Established the XIMG-008 release and quality policy: Semantic Versioning and
  version authority, Rust/MSRV and Firefox Release/ESR support, the required CI
  matrix, dependency/security/licence governance, fixture privacy, release and
  exception procedures, and a precise Definition of Done. Added dependency-
  free local link/JSON/schema-major/privacy/version checks plus a SHA-pinned
  GitHub Actions mirror; local containerized Sphinx verification remains the
  documentation authority.
- Added the XIMG-007 Firefox architecture spike ADR and synthetic fixture
  matrix, covering WebRequest/DNR limits, bounded response filtering, exact
  origin permissions, HTTPS and response contracts, signed-URL redaction,
  observed-thumbnail versus explicit-original eligibility, segmented-video
  capability gates, and fail-open behavior.
- Added the XIMG-006 versioned acquisition/catalogue metadata contract for
  source items, canonical media identities, DASObjectStore references,
  attempts, leases, cursors, review state, tombstones, audit events, and
  crash/idempotency reconciliation, with strict synthetic fixtures.
- Added XIMG-005 versioned draft-2020-12 configuration schemas and synthetic
  positive/negative fixtures for the instance, X, Instagram, and website
  policy contracts, with strict fields and host-managed references documented
  in the Sphinx configuration guide.
- Added the XIMG-004 architecture ADR set for authority boundaries,
  metadata-only local state, canonical identity and idempotent settlement,
  review lifecycle, bounded account refresh scheduling, Firefox pairing, and
  external-cache fail-open behavior, with a pinned local Sphinx verification
  container.
- Added the XIMG-003 sibling compatibility matrix with pinned Monas,
  DASObjectStore, Mnemosyne design-language, and future Synoptikon revisions,
  fixture anchors, public-build independence rules, and explicit host/range
  contract risks.
- Added the Pinakotheke v1.0.0 coordinated rename/rebrand gate, including
  compatibility aliases and repository migration planning.
- Added the Sphinx/Read the Docs local-container documentation authority and
  stricter Firefox observation, opt-in, fail-open, and status-display rules.
- Added the planned explicit GEO/SRA/ENA/NCBI resource workflow: reviewable
  accession plans, rights gates, direct DASObjectStore streaming, checksum and
  provenance requirements, optional Aspera with HTTPS fallback, and no bulk
  discovery or durable x-img-local payloads.
- Added endpoint/device versus logical ObjectStore planning: managed local
  folder-profile bootstrap, remote HTTPS pairing and discovery, explicit
  endpoint-plus-store selection, commit-time capability checks, stable-ID
  provenance, and safe reconnect behavior without silent destination changes.
- Added user-selected video planning: policy-gated candidate review,
  versioned Firefox playback profiles, normalized DASObjectStore renditions,
  pinned containerized FFmpeg, bounded scratch/cleanup, explicit failure states,
  and no automatic crawling, DRM circumvention, or source-only readiness.

## [0.1.0] - 2026-07-14

### Added

- Initial public planning baseline.
- Dependency-ordered milestones and TODO backlog for X, Instagram, Firefox,
  Monas, DASObjectStore, and future Synoptikon integration.
- Contributor rules for single-run automation, subagent delegation, focused
  commits, immediate pushes, data authority, privacy, and design conformance.
- Mozilla Public License 2.0.

[Unreleased]: https://github.com/sagrudd/x-img/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/sagrudd/x-img/releases/tag/v0.1.0
