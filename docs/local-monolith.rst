Local Pinakotheke monolith
==========================

``pinakotheke-monolith`` is the local-first distribution framework for a
coherent Pinakotheke web service. It does not merge security authorities:
Monas/Prosopikon continues to own login and sessions, and DASObjectStore
continues to own all durable media bytes. The first XIMG-090 slice starts the
foreground HTTP service while those integrations remain visibly unavailable.

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
This slice deliberately does not create ``~/.x-img/dasobjectstore``; XIMG-091
will provision that location through DASObjectStore rather than treating it as
an unmanaged media folder. Credentials and private DAS configuration will
remain outside the product root under an OS-private configuration directory.

Readiness
---------

``GET /health`` reports coarse process liveness. ``GET /ready`` reports three
worded component states. In the first slice, ``pinakotheke`` is ``Ready`` while
``monas_authentication`` and ``dasobjectstore`` are ``Not configured``; the
overall state is therefore ``not_ready``. Authenticated product and media routes
are not mounted yet. This is intentional and prevents a scaffold from claiming
authority it does not possess.

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

XIMG-091 provisions the managed local DASObjectStore profile and named logical
ObjectStore. XIMG-092 composes Monas/Prosopikon authentication and host context.
XIMG-093 adds per-user macOS ``launchd`` management, and XIMG-094 proves a
clean-home authenticated ingest/read/restart flow end to end.
