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

Monas ``0.2.0`` provides the matching authenticated forwarding mount. Create
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

Stop the foreground process with ``Control-C``. Axum stops accepting new work
and completes graceful shutdown.

Network safety
--------------

Loopback is the default and recommended binding. A non-loopback address is
refused unless the operator supplies the deliberately explicit
``--allow-non-loopback-without-authentication`` acknowledgement. The option is
for controlled development only: it prints a warning and does not create TLS or
authentication. Do not expose this first slice to an untrusted network.

Next slices
-----------

XIMG-092 composes Monas/Prosopikon authentication and host context.
XIMG-093 adds per-user macOS ``launchd`` management, and XIMG-094 proves a
clean-home authenticated ingest/read/restart flow end to end.
