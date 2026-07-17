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
