Firefox extension scaffold
==========================

Brand icon
----------

Firefox uses the approved black-and-white Mnemosyne Biosciences icon rather
than its generic puzzle-piece fallback. The 16, 32, 48, and 96 pixel PNGs are
aspect-preserving derivatives of
``mnemosyne-biosciences-logo-icon-black.png`` from
``mnemosyne_design_language`` commit
``5539df8f662a78ebdf7cf4c868d71831380c8cfd``. Each derivative is centred on
a transparent square canvas; the artwork is neither stretched nor recoloured.
The 16/32 sizes serve the toolbar action and all four sizes serve Firefox's
extension-management and installation surfaces.

Server-led setup
----------------

Set up Firefox from the authenticated Pinakotheke web application, not by
inventing values in the extension settings. The ordered prerequisites are:

#. the DASObjectStore service is available;
#. Pinakotheke is installed and the user signs in through Monas;
#. a reviewed named writable ObjectStore is configured for Pinakotheke;
#. the **Connect Firefox** task pane reports ``DASObjectStore: Ready`` and
   presents the signed extension download, instance identifier, and
   actor-bound pairing reference; and
#. Firefox presents the actor-bound, scoped pairing to Monas and verifies the
   returned server/destination identity before saving it.

In the extension settings, enter the exact Pinakotheke origin (for example
``https://192.168.1.192:8731``) and the two values shown by **Connect
Firefox**, then choose **Pair**. A logged-out session, unavailable
DASObjectStore, expired/revoked pairing, or changed value fails without saving
the relationship. The pairing is a narrow revocable product credential: Monas
maps it to the original actor, strips it before forwarding, and injects host
context. Pinakotheke never receives the user's Monas cookie. The record
contains stable endpoint and ObjectStore identifiers but no DAS credentials,
site cookies, or passwords.

Enable each website separately using its exact HTTPS origin. Select Images
and/or Videos and explicitly check **This site is intended for X ingress** when
the rule is intended to collect user-observed X content. That checkbox records
intent; it does not enable crawling, hidden traversal, automatic opening, or
credential access.

Persistent site corpus
----------------------

Site definitions are authenticated actor data, not disposable extension
preferences. Pinakotheke persists a strict ``pinakotheke.site-corpus.v1``
document below its private metadata root. Each successful replacement advances
an actor-specific revision. Firefox retains a local working copy for ordinary
browsing and synchronizes it after pairing and browser startup.

On first pairing, existing local rules are uploaded when the server corpus is
empty. On later starts, the server revision restores rules to a replacement
profile or additional paired device. Every options-page POST supplies the
last observed revision. HTTP 409 means another device changed the corpus;
Firefox restores that newer server value and asks the user to review it rather
than overwriting it. A server or network failure retains the previous corpus
and reports that the requested change was not saved.

Removed origins leave bounded revisioned tombstones, preventing an older
paired device from resurrecting a deleted rule. Settings can export or import
a bounded strict JSON rule document; this recovery document deliberately omits
the server URL, pairing reference, endpoint/store authority, and diagnostics.

The corpus contains exact origins, image/video selection, capture,
substitution, and X-ingress intent. It contains no pairing secret, site
credential, cookie, browsing history, or downloaded payload. Rules remain
actor-scoped even when immutable DASObjectStore payloads deduplicate across
users.

XIMG-060 supplies a Manifest V3 Firefox extension scaffold for one configured
x-img instance. Its baseline permissions are only ``storage``, ``activeTab``,
``scripting``, and ``permissions``. Site access is an optional exact HTTPS
origin permission requested only from the options page; private browsing is not
enabled by default.

The options page shows the exact origin and consequences before permission,
chooses image/video media classes, and provides independent capture and
substitution toggles plus permission removal for each site.

The extension has no cookie, history, password, response-body interception, or
credential API; it never opens pages, traverses hidden content, crawls, or
simulates browsing. For explicitly enabled X video capture only, it observes a
bounded list of completed ``video.twimg.com`` HLS/DASH request URLs. It never
reads request headers, response headers, bodies, cookies, or authorization and
never blocks or modifies the request.
The toolbar has no effect unless a paired instance and an explicitly enabled
site exist, and failures are silent so ordinary page use continues.

On an enabled image site, images displayed in the viewport are eligible only
for cache-evidence lookup and stored-state framing. They never create an
``observed_thumbnail`` capture plan. A trusted user open may submit the image's
original, and trusted user-started playback may submit a video. The paired host
must authenticate and authorize either plan before it enters the shared
scheduler; this is never a direct browser payload upload. See
:doc:`firefox-capture` for the endpoint, policy, and fail-open contract.

For video, the enabled page may expose a progressive HTTPS resource or an
already-observed HLS/DASH manifest after the trusted play. Firefox sends only
that bounded retrieval candidate and semantic creator/uploader metadata to the
paired Pinakotheke instance. An in-page graphic communicates download,
normalization, ObjectStore commit, and final availability without reading site
cookies or changing the origin's stored media. Previously committed
presentation pages frame their visible poster thumbnails through one bounded
Pinakotheke evidence lookup.

The compatibility baseline reviewed for this slice is Mnemosyne design
language ``fbfa28e55d1c8111ef95a139d83927c231534b5f``, Monas
``dac0e113c8b197cb06abc38187d72f27e562ad63``, DASObjectStore
``89347a18d53da56d8a3e7da87b31e9f86aaddb66``, and the future Mnemosyne host
contract ``2244a49f5057ef6251b2760bd0729de8e2207f56``. Pinakotheke continues to
consume only the repository's versioned wire contracts and fixtures.

The toolbar now opens the explicit per-site cache control and diagnostic
surface described in :doc:`firefox-toolbar`. Image, normalized-MP4, and
fail-closed segmented behavior remain governed by their adapter/authority gates.

Pinakotheke upgrade identity
----------------------------

The Gecko extension ID ``x-img@example.invalid`` shipped in the 0.9 release and
is therefore a durable update identity, despite containing the legacy planning
name. Pinakotheke 1.0 retains it. Changing the ID would create a different
extension, strand existing installations, and lose Firefox-managed upgrade
continuity.

The inert canonical candidate is
``packaging/firefox/pinakotheke-manifest.v1.candidate.json``. It changes the
listing name and description while retaining the exact Gecko ID, permissions,
optional origins, CSP, minimum Firefox version, and extension entry points. It
is not included in 0.9 XPIs. The executable upgrade contract confirms that an
update preserves pairing, explicitly enabled site rules, endpoint ID, and
ObjectStore ID exactly. Fresh-install defaults are written only when their keys
are absent; an extension update never resets stored configuration.

Run the focused check locally with:

.. code-block:: console

   node scripts/firefox/check_identity_upgrade.mjs
