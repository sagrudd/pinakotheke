Trusted local HTTPS for Firefox
================================

The Pinakotheke Firefox extension accepts only HTTPS instances. A private LAN
deployment does not need a public DNS name or an external certificate service:
it may use a small local certificate authority that is explicitly trusted by
the Firefox host.

The verified macOS development path uses ``mkcert`` and NSS. ``mkcert`` keeps
the CA private key on the Mac; copy only the issued server leaf certificate and
leaf private key to DASServer. Never copy ``rootCA-key.pem``. The certificate
must contain every address used by the browser, including the DASServer IP.

.. code-block:: console

   brew install mkcert nss
   mkcert -install
   mkcert -cert-file server.crt -key-file server.key \
     192.168.1.192 localhost 127.0.0.1

macOS may request authorization before trusting the local CA. Approve that
local Keychain change, then restart Firefox. For a Firefox profile that does
not inherit macOS trust, import ``$(mkcert -CAROOT)/rootCA.pem`` under
``Settings → Privacy & Security → Certificates → View Certificates →
Authorities`` and select trust for websites.

DASServer terminates TLS in a local reverse proxy on port 8731 and forwards
only to Monas on loopback. Monas continues to own login and forwards the
Pinakotheke application/API to the loopback product backend. The proxy must
preserve ``Host`` and set ``X-Forwarded-Proto: https``. The leaf key is mode
``0600`` below the private Pinakotheke runtime root; neither CA key nor browser
credentials belong on DASServer.

Verify the exact address without disabling certificate checks:

.. code-block:: console

   curl --cacert "$(mkcert -CAROOT)/rootCA.pem" \
     https://192.168.1.192:8731/

Mozilla has signed the authenticated Pinakotheke ``1.3.1`` XPI. Install the
signed artifact from
``https://192.168.1.192:8731/downloads/pinakotheke-1.3.1.xpi``;
unsigned platform builds below ``dist/firefox/`` remain temporary development
artifacts. Pair the extension with ``https://192.168.1.192:8731`` only after
the page loads without a certificate warning.

The deployed listener handles nginx's internal ``497``
plain-request-on-TLS-port condition with a narrow permanent redirect to the
same host, port, path, and query over ``https``. This recovery occurs before
Monas and carries no authentication state; it prevents an accidentally copied
``http://192.168.1.192:8731`` XPI link from ending at nginx's opaque 400 page.
The HTTPS endpoint remains canonical, and HSTS is intentionally deferred until
every local client trusts the private CA.

The 2026-07-17 DASServer proof used TLS 1.3 with certificate verification
enabled in Firefox 152.0.6, installed the Mozilla-signed Pinakotheke 1.2.2 XPI
permanently through WebDriver BiDi, and loaded the Monas login route without an
insecure-certificate override. The sibling patterns inspected were Mnemosyne
``52810176bf95a170f93d74a6f5daa94da5c6640e``, DASObjectStore
``d99e0c98736be9fa9443e78d91a0a2e04488b881``, Mnematikon
``de6f7d7074decc8a2fa7ca449b7fc713d7b33dc7``, and domain-cert
``ae07e31ff814909c966b83dfa8ee7ffe112a5cf6``. The cPanel account did not
expose ``ZoneEdit``, so DNS-01 automation was deliberately not added.
