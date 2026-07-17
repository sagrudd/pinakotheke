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

XIMG-060 supplies a Manifest V3 Firefox extension scaffold for one configured
x-img instance. Its baseline permissions are only ``storage``, ``activeTab``,
``scripting``, and ``permissions``. Site access is an optional exact HTTPS
origin permission requested only from the options page; private browsing is not
enabled by default.

The options page shows the exact origin and consequences before permission,
chooses image/video media classes, and provides independent capture and
substitution toggles plus permission removal for each site.

The extension has no cookie, webRequest, history, password, or credential API;
it never opens pages, traverses hidden content, crawls, or simulates browsing.
The toolbar has no effect unless a paired instance and an explicitly enabled
site exist, and failures are silent so ordinary page use continues.

When the user clicks the toolbar on an enabled image site, only images actually
displayed in the current viewport are eligible for an ``observed_thumbnail``
capture plan. The extension does not automatically open an original. The
paired host must authenticate and authorize the plan before it enters the
shared scheduler; this is never a direct browser payload upload. See
:doc:`firefox-capture` for the endpoint, policy, and fail-open contract.

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
