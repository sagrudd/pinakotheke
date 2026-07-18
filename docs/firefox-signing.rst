Mozilla-signed Firefox installation
===================================

Pinakotheke uses Mozilla's **unlisted** signing channel. Mozilla validates and
signs the extension, while the project distributes the resulting XPI directly;
it is not listed publicly in the Firefox Add-ons catalogue. Standard Firefox
can install that signed XPI without developer mode or signature overrides.

The extension keeps the stable ``x-img@example.invalid`` Gecko identity so an
installed copy can be upgraded without losing pairing and site rules. Changing
that identity creates a different add-on and is not a branding operation.

One-time publisher setup
------------------------

#. Sign in to the `Mozilla Add-ons developer hub
   <https://addons.mozilla.org/developers/>`_ and create API credentials.
#. Export the credentials only in the private signing shell. Do not put them in
   a repository file, Make variable, command argument, browser storage, log, or
   package::

      export WEB_EXT_API_KEY='user:...'
      export WEB_EXT_API_SECRET='...'

#. Run the pinned local validator and request an unlisted signature::

      make firefox-lint
      make firefox-sign

The signed XPI is written below ``dist/firefox/signed``. The signing target
also refuses an artifact whose Mozilla signature envelope, Gecko identity, or
version does not match the workspace. Mozilla may place a first submission in
manual review; that is an external review state, not a reason to weaken the
manifest or disable signing.

Install and distribute
----------------------

In Firefox, open **Add-ons and themes**, choose **Install Add-on From File**,
and select the signed XPI. A web download endpoint may also distribute it when
served over HTTPS with media type ``application/x-xpinstall``. Never publish
the unsigned files under ``dist/firefox/<platform>/<architecture>`` as an
ordinary user installation.

The install prompt accurately declares browsing activity, website content,
and website activity because an opted-in capture transmits page/media URLs,
selected content metadata, and the save action outside the extension to the
user's configured Pinakotheke service. It does not claim ``none`` merely
because that service is local or user-owned. Site access remains optional and
is requested only for an explicitly enabled HTTPS origin.

The signed extension requires Firefox 142 or later. That floor is intentional:
it is the first cross-desktop/Android baseline for which Mozilla's validator
accepts the built-in data-consent declaration used by this manifest.

Credential and release checks
-----------------------------

``make firefox-lint`` does not need publisher credentials. ``make
firefox-sign`` obtains credentials from the standard ``WEB_EXT_API_KEY`` and
``WEB_EXT_API_SECRET`` environment variables without placing their values in
the process arguments. Before distribution, install the signed XPI in an
ordinary supported Firefox profile and repeat the pairing, opted-in capture,
fail-open, and upgrade checks described in :doc:`firefox-extension` and
:doc:`firefox-capture`.

First signed release evidence
-----------------------------

Mozilla approved the first unlisted Pinakotheke ``1.2.1`` submission on
2026-07-17. The returned XPI has SHA-256
``1e32a642c576503b89f4e2c2131e1916dfc03cb5561ecf60ffc2e31b6207f229``.
The checked-in ``.amo-upload-uuid`` is non-secret AMO submission-continuity
metadata; it contains no API issuer or secret and remains aligned with the
stable manifest ID for later version submissions.

Run permanent-install acceptance in a disposable Firefox profile with::

   make firefox-signed-install-check XPI=dist/firefox/signed/d9ed59c61b424a64a821-1.2.1.xpi

Firefox ``152.0.6`` accepted that artifact with ``moz:permanent=true`` and
reported the stable ``x-img@example.invalid`` identity. This mode requires a
signed extension; the test deletes its isolated profile afterward. The live
DASServer copy is served from
``https://192.168.1.192:8731/downloads/pinakotheke-1.2.1.xpi`` with
``application/x-xpinstall``, ``nosniff``, and ``private, no-store`` headers;
the downloaded checksum matches the approved artifact.

Branded signed release evidence
-------------------------------

Mozilla approved the branded unlisted Pinakotheke ``1.2.2`` submission on
2026-07-17. Its SHA-256 is
``ba1f518a50027bd5941f8868f9f80b2ccbc98c9706b5dfd622593a652be922fc``.
Firefox accepted it as a permanent add-on with the unchanged
``x-img@example.invalid`` identity, and the signed archive contains the
approved 16, 32, 48, and 96 pixel Mnemosyne icon derivatives.

The verified DASServer copy is available at
``https://192.168.1.192:8731/downloads/pinakotheke-1.2.2.xpi`` with the XPI
media type and no-store headers. Its downloaded checksum is identical to the
Mozilla-returned artifact.

Authenticated onboarding release evidence
-----------------------------------------

Mozilla approved Pinakotheke ``1.3.1`` on 2026-07-17. Its SHA-256 is
``0190e24319bd0e6d8e755b04b55aebf0b622ff120a241db16f50840177e10393``.
Firefox accepted it as a permanent add-on. The authenticated Connect Firefox
panel binds its scoped pairing payload to named ObjectStore
``pinakotheke_media``; the live endpoint accepted the exact credential and
rejected a changed credential with HTTP 401. The checksum-identical XPI is at
``https://192.168.1.192:8731/downloads/pinakotheke-1.3.1.xpi``.

Automatic-cache release evidence
--------------------------------

Mozilla approved Pinakotheke ``1.5.1`` as an unlisted signed extension on
2026-07-17. Its SHA-256 is
``7493ab445af0e3afadb760f2ee66ad27b783d470b01292c38d66bd160e91e45d``.
The permanent-install acceptance passed with the stable
``x-img@example.invalid`` identity. Pinakotheke 1.5.1 now advertises the same
versioned path from authenticated onboarding, and the signature-identical live
copy is served at
``https://192.168.1.192:8731/downloads/pinakotheke-1.5.1.xpi`` with
``application/x-xpinstall``, ``nosniff``, and ``private, no-store`` headers.

Live X CDN repair evidence
--------------------------

Mozilla approved Pinakotheke ``1.6.1`` as an unlisted signed extension on
2026-07-17. Its SHA-256 is
``41a71d65e3fa8d8f3ef08274791773617c249cb247d9a305216a6e8f02586ee6``.
Permanent installation retained the stable ``x-img@example.invalid`` identity.
The x86_64 DASServer package and service report 1.6.1, and the checksum-identical
XPI is served from
``https://192.168.1.192:8731/downloads/pinakotheke-1.6.1.xpi`` with the required
Firefox installation content type and private no-cache headers.
The AMO issuer and secret are held in access-controlled macOS Keychain entries
for signing only; they are not present in the repository, artefact, config,
documentation, command history, or service host.

Observable-ingress release evidence
-----------------------------------

Mozilla approved Pinakotheke ``1.6.0`` as an unlisted signed extension on
2026-07-17. Its SHA-256 is
``c24fd4ac7425e8c1bebf5f0ec2c4971f93819f188508376714a5f031af68d0da``.
The stable-identity permanent-install check passed. The x86_64 DASServer was
upgraded to Pinakotheke 1.6.0 and serves the signed XPI at
``https://192.168.1.192:8731/downloads/pinakotheke-1.6.0.xpi`` with the expected
``application/x-xpinstall``, ``nosniff``, and ``private, no-store`` headers.

Generic progressive-video release evidence
------------------------------------------

Mozilla approved Pinakotheke ``1.12.0`` as an unlisted signed extension on
2026-07-18. Its SHA-256 is
``a198e774803e6d4d66ebf2873946d26f20f9a1d58eb64d5401643be97f433102``.
Firefox ``152.0.6`` accepted it as a permanent add-on with the unchanged
``x-img@example.invalid`` identity. The installed-Firefox capture harness
generates an ephemeral synthetic progressive video inside its disposable
profile, serves it from an exact opted-in HTTPS origin, and proves that native
trusted pointer/play input produces an ``explicit_video`` capture plan. The
fixture bytes are deleted with the profile and never enter the repository.

The x86_64 DASServer runs Pinakotheke ``1.12.0`` and serves the
checksum-identical artifact at
``https://192.168.1.192:8731/downloads/pinakotheke-1.12.0.xpi`` with
``application/x-xpinstall``. A trusted TLS download returned the same SHA-256.
