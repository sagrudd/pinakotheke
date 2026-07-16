Local Pinakotheke monolith
==========================

``pinakotheke-monolith`` is the local-first distribution framework for a
coherent Pinakotheke web service. It does not merge security authorities:
Monas/Prosopikon continues to own login and sessions, and DASObjectStore
continues to own all durable media bytes. The foreground service now recognizes
a reviewed DASObjectStore-managed local profile while Monas authentication
remains visibly unavailable.

Start on macOS
--------------

Run as the ordinary local user, never with ``sudo``:

.. code-block:: console

   cargo run -p pinakotheke-cli --bin pinakotheke -- serve

The default listener is ``http://127.0.0.1:8731`` and the default product root
is ``~/.x-img``. An explicit alternative must be absolute:

.. code-block:: console

   pinakotheke serve --root "$HOME/.x-img" --port 8731

Startup creates only private mode-``0700`` metadata directories:

.. code-block:: text

   ~/.x-img/
     config/
     state/
     run/
     logs/

The root and its required children must be real directories, not symlinks.
Provision local ObjectStore authority
-------------------------------------

Docker Desktop must be running. Review the plan, then ask DASObjectStore's
canonical helper to provision the bounded profile:

.. code-block:: console

   pinakotheke storage local-profile plan \
     --provisioner /absolute/path/to/DASObjectStore/deploy/local-docker/local.sh
   pinakotheke storage local-profile provision \
     --provisioner /absolute/path/to/DASObjectStore/deploy/local-docker/local.sh

The authority owns ``~/.x-img/dasobjectstore`` and the named
``pinakotheke_local`` logical ObjectStore. Private Garage configuration and
keys remain under ``~/.config/dasobjectstore``. Pinakotheke stores only the
secret-free stable endpoint/ObjectStore selection in its metadata state; it
never writes media by treating the managed root as an ordinary folder.

Use ``status`` to re-discover and validate the authority identity, or ``down``
to stop the containers without deleting state. Both accept the same required
``--provisioner`` argument. An alternate ``--root`` must be absolute.

Readiness
---------

``GET /health`` reports coarse process liveness. ``GET /ready`` reports three
worded component states. In the first slice, ``pinakotheke`` is ``Ready`` while
``monas_authentication`` is ``Not configured``. ``dasobjectstore`` becomes
``Ready`` only when the exact secret-free managed selection has been persisted;
the overall state remains ``not_ready`` until Monas is composed. Authenticated
product and media routes are not mounted yet.

Trusted Monas dispatch boundary
-------------------------------

The XIMG-092 ingress slice accepts authenticated product requests only after a
Monas-owned dispatcher supplies both a validated, non-secret
``x-img.host-context.v1`` document and a process-local dispatch credential.
The credential is not a browser session and must never be placed in a browser,
URL, configuration JSON, or log. Pass its private mode-``0600`` file to the
backend listener:

.. code-block:: console

   pinakotheke serve \
     --monas-dispatch-token-file "$HOME/.x-img/run/monas-dispatch.token"

The protected proof route is
``/products/pinakotheke/api/context``. A direct request, a forged credential,
an invalid context, or a context lacking ``ximg.access`` is rejected. The two
dispatch headers are removed before product code runs. When configured,
readiness reports the trusted dispatch boundary as ``Ready``; this means only
that the backend is prepared for Monas dispatch, not that a login has occurred.

Monas ``0.3.0`` provides the matching authenticated forwarding mount and login
screen. Create
one private credential and start the backend on a separate loopback port, then
start Monas as the only user-facing listener:

.. code-block:: console

   install -d -m 700 "$HOME/.x-img/run"
   (umask 077; openssl rand -hex 32 > "$HOME/.x-img/run/monas-dispatch.token")
   pinakotheke serve --port 8732 \
     --monas-dispatch-token-file "$HOME/.x-img/run/monas-dispatch.token"

.. code-block:: console

   MONAS_BIND_ADDR=127.0.0.1:8731 \
   PINAKOTHEKE_UPSTREAM=http://127.0.0.1:8732 \
   PINAKOTHEKE_DISPATCH_TOKEN_FILE="$HOME/.x-img/run/monas-dispatch.token" \
   monas-server

Open ``http://127.0.0.1:8731/products/pinakotheke/app/``. Monas verifies its
Prosopikon cookie, generates the correlation identifier, strips the cookie and
any caller-supplied dispatch headers, and streams to the backend. Pinakotheke
never parses the cookie or issues login, session, or logout state. Keep port
8732 loopback-only; direct protected requests remain rejected.

Build and mount the Yew application
-----------------------------------

Build the browser application locally with the checked-in Trunk document and
WebAssembly start entry:

.. code-block:: console

   make web

This writes hashed HTML, CSS, JavaScript, and WebAssembly files to
``dist/web`` with the canonical ``/products/pinakotheke/app/`` public URL. No
sibling source tree or unpublished path dependency is used. The semantic-token
mirror records ``mnemosyne_design_language`` commit
``5539df8f662a78ebdf7cf4c868d71831380c8cfd``; Monas remains responsible for
the approved host branding and login assets.

Point the backend at the reviewed build directory:

.. code-block:: console

   pinakotheke serve --port 8732 \
     --web-root "$PWD/dist/web" \
     --monas-dispatch-token-file "$HOME/.x-img/run/monas-dispatch.token"

Alternatively, copy the complete build into ``~/.x-img/web`` and omit
``--web-root``. Pinakotheke accepts only an absolute, symlink-free tree with a
regular ``index.html``, at most 128 files, and at most 32 MiB. Missing default
assets leave the app mount unconfigured rather than serving a placeholder as
the gallery.

The entire app mount is protected by Monas dispatch. A direct request to the
backend app path returns ``401``; after Monas authenticates the user, the same
path serves the Yew document and its hashed assets. Catalogue and object routes
retain their independent host-context checks. Native tests prove both direct
denial and admitted static delivery. Native packages install this web build at
the platform path embedded in the monolith.

Compose ObjectStore delivery
----------------------------

Supply a reviewed host/DASObjectStore adapter to make persisted gallery objects
readable through the same authenticated monolith:

.. code-block:: console

   pinakotheke serve --port 8732 \
     --object-read-helper /absolute/path/to/reviewed-helper \
     --monas-dispatch-token-file "$HOME/.x-img/run/monas-dispatch.token"

The helper uses the strict ``pinakotheke.object-read-helper.v1`` protocol
documented in :doc:`object-read`. It receives object identity metadata only and
streams bytes directly from DASObjectStore. It must own scoped authentication;
do not wrap an arbitrary download command or expose credentials through its
arguments. With the helper configured, the compiled Yew app, catalogue, image
preview, poster, and normalized-video range routes are composed together behind
Monas dispatch. Without it, object delivery remains unmounted and the UI shows
the explicit unavailable state rather than consulting an origin URL.

``make firefox-gallery-check`` independently exercises the compiled bundle in
installed Firefox. It uses an ephemeral loopback catalogue and private Firefox
profiles; it proves the browser component, not the local Monas and
DASObjectStore authorities.

Stop the foreground process with ``Control-C``. Axum stops accepting new work
and completes graceful shutdown.

Per-user macOS service
----------------------

Review and install the two coordinated user agents:

.. code-block:: console

   pinakotheke service plan
   pinakotheke service install \
     --pinakotheke-binary /absolute/path/to/pinakotheke \
     --monas-binary /absolute/path/to/monas-server \
     --object-read-helper /absolute/path/to/reviewed-helper \
     --object-read-endpoint-id endpoint-local

Installation requires absolute executable regular files, generates a private
dispatch credential, keeps the backend on port 8732, and exposes Monas on port
8731. The helper is optional until a reviewed DASObjectStore implementation is
installed. Its path and stable reviewed endpoint identity must be supplied
together; the backend agent retains both and exposes the identity to the helper
as ``PINAKOTHEKE_OBJECT_READ_ENDPOINT_ID``. The identity is scope, not a
credential, and the helper must authenticate independently. Prosopikon remains
under ``~/.config/monas/prosopikon``; Pinakotheke
metadata and logs remain under ``~/.x-img``; DASObjectStore retains its own
authority roots.

.. code-block:: console

   pinakotheke service status
   pinakotheke service logs
   pinakotheke service logs --follow
   pinakotheke service restart
   pinakotheke service uninstall

Existing definitions are never overwritten implicitly. After review, use
``install --replace``; replacement restores the prior pair if launchd admission
fails. Uninstall removes only the service definitions. Configuration,
credentials, logs, catalogue state, Prosopikon users, and ObjectStore data are
preserved.

Network safety
--------------

Loopback is the default and recommended binding. A non-loopback address is
refused unless the operator supplies the deliberately explicit
``--allow-non-loopback-without-authentication`` acknowledgement. The option is
for controlled development only: it prints a warning and does not create TLS or
authentication. Do not expose this first slice to an untrusted network.

Next slices
-----------

XIMG-094 still proves a clean-home authenticated ingest/read/restart flow end
to end. The host read adapter is now available; a matching DASObjectStore helper
implementation and live commit/reconciliation path remain.
