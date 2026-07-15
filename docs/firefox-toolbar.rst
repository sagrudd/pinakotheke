Firefox cache toolbar
=====================

The x-img toolbar popup is the per-site control and diagnostic surface for the
external cache. It operates only on the active tab's exact, explicitly enabled
HTTPS origin. Opening the popup does not scan media or contact x-img; the user
selects ``Run cache for visible media`` to invoke bounded capture/substitution.

Controls
--------

The popup shows ``Active``, ``Paused``, or ``Not enabled`` separately for
capture and substitution. ``Pause substitution`` and ``Resume substitution``
change only the current site's rule; origin browsing continues normally.
``Settings`` opens site policy, while ``Open x-img source view`` opens the
paired host's Websites context for the already-configured origin.

The permission explanation states that only user-enabled HTTPS origins are in
scope and that cookies, passwords, history, and authorization headers are not
read. Removing a site removes its optional permission and diagnostic entry.

Status and diagnostics
----------------------

Each configured origin has at most one current record. New results replace the
prior result; removed-origin records are deleted. It contains only a worded
state, one coarse reason, and two booleans rendering ``◉ Previously observed``
and ``✓ Stored in ObjectStore``. It contains no page/media URL, alias, signed
query, checksum, object key, cookie, credential, payload, or general history.

The evidence labels use words and iconography rather than colour alone. Their
tooltip explains that this is reversible status framing: stored bytes are never
watermarked or modified. Capture-plan acceptance says ``Previously observed``;
it does not claim ``Stored in ObjectStore`` until reviewed ObjectStore delivery
actually succeeds.

Fail-open behavior
------------------

A miss, host/session failure, unsupported adapter, object outage, or page-side
replacement error becomes ``Origin served``. The toolbar never retries through
another endpoint or ObjectStore and never converts a diagnostic into a page
failure.

Verification
------------

.. code-block:: console

   node --check firefox-extension/background.js
   node --check firefox-extension/popup.js
   node --check firefox-extension/options.js
   python3 scripts/firefox/check_toolbar_contract.py
   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check

Compatibility-sensitive contracts were inspected at DASObjectStore
``42bf66a7494f4e0aa81f103100b71489b38164dc``, Monas
``3d21b0bc7b83fa8408d01b93347a56f43f3a96b7``, Mnemosyne design language
``5539df8f662a78ebdf7cf4c868d71831380c8cfd``, and future Synoptikon
``52810176bf95a170f93d74a6f5daa94da5c6640e``. No unpublished path dependency
is used.
