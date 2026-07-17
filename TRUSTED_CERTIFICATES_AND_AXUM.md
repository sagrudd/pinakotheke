# Trusted certificates and direct Axum HTTPS

This document is the authoritative deployment pattern for TLS termination in
Pinakotheke and a reference pattern for other Mnemosyne web products.

## Decision

The product's Rust process terminates HTTPS directly. Axum supplies the router;
`axum-server` and Rustls own the listener and TLS handshake. A reverse proxy is
not required for TLS, routing, downloads, or application authentication.

Pinakotheke accepts a PEM certificate chain and matching PEM private key:

```console
pinakotheke serve \
  --root /home/example/.x-img \
  --bind 0.0.0.0 \
  --port 8731 \
  --allow-non-loopback-without-authentication \
  --tls-certificate-chain /home/example/.x-img/tls/server.crt \
  --tls-private-key /home/example/.x-img/tls/server.key \
  --firefox-downloads-root /usr/local/share/pinakotheke/firefox
```

Both TLS arguments are required together. Paths must be absolute, non-empty
regular files rather than symlinks, and the private key must have no group or
other access (mode `0600` is recommended). Invalid PEM, a mismatched key, or an
incomplete pair fails startup. When the TLS arguments are present the port is
HTTPS only; sending plain HTTP to it is correctly rejected.

## What “trusted” means

TLS encryption and browser trust are separate properties. Rustls can serve any
valid certificate chain, but Firefox trusts it only when the issuing root is in
Firefox's trust store or the operating-system trust store that Firefox is
configured to use.

For a private IP or `.local` deployment, use a private development/household CA:

1. Create or select one stable local CA. `mkcert` is suitable for a single
   operator; the Mnemosyne `domain-cert` project is the preferred shared
   issuance boundary as it gains local-CA support.
2. Issue a server certificate whose Subject Alternative Name contains every
   address users actually enter, for example `192.168.1.192`, the server DNS
   name, and `localhost` only if local access is needed. A Common Name alone is
   insufficient.
3. Install the CA certificate—not the server private key—into each client's
   trusted authorities. In Firefox, use **Settings → Privacy & Security →
   Certificates → View Certificates → Authorities → Import**, then trust the CA
   for identifying websites.
4. Keep the CA private key off the web server when practical. Copy only the
   issued server chain and server private key to the service account.
5. Verify hostname/IP identity and the complete chain before deployment.

For a public DNS name, an ACME-issued certificate from a publicly trusted CA is
preferred. Public CAs generally do not issue certificates for RFC1918 private
IP addresses.

## Certificate preparation and verification

The chain file must place the leaf certificate first, followed by intermediate
certificates. The root certificate normally remains in the client trust store
and need not be sent by the server.

```console
chmod 600 /home/example/.x-img/tls/server.key
openssl x509 -in /home/example/.x-img/tls/server.crt \
  -noout -subject -issuer -dates -ext subjectAltName
openssl pkey -in /home/example/.x-img/tls/server.key -check -noout
```

After starting the service:

```console
curl --cacert local-ca.pem https://192.168.1.192:8731/ready
openssl s_client -connect 192.168.1.192:8731 \
  -servername server.example.internal -showcerts </dev/null
```

An IP URL is validated against an IP Subject Alternative Name. `--insecure`, a
Firefox certificate exception, or disabling verification is not release
evidence.

## Rotation and service operation

Certificate files are loaded at process startup. Rotate them atomically as a
matched pair, validate them, and restart the service. Keep the previous pair
only in a protected rollback location. Monitor expiry and renew before the
certificate reaches its operational threshold.

Use a service manager (`systemd` for system installations or `launchd` for
per-user macOS installations) to restart Pinakotheke and report failures. It
must invoke the Rust binary directly, bind the intended HTTPS port, and pass the
certificate paths. Do not configure a second process to own the same port.

Application authentication is still owned by Monas. Direct TLS changes the
transport boundary only; it does not authorize anonymous access, expose DAS
credentials, or move session issuance into Pinakotheke.

## Migration from a reverse proxy

1. Issue or retain a certificate with the correct SANs and make the chain/key
   readable only by the Pinakotheke service account.
2. Stop the reverse proxy listener on the selected port.
3. Change the Pinakotheke service from an HTTP loopback port to the intended
   network address and HTTPS port, adding both TLS arguments and the reviewed
   signed-XPI directory when extension downloads are offered.
4. Start Pinakotheke and verify `/ready`, the Monas login flow, the Yew app,
   object range delivery, and Firefox extension downloads over HTTPS.
5. Disable the obsolete proxy site permanently. The proxy package may remain
   for unrelated services, but it is no longer in Pinakotheke's request path.

Rollback means stopping Pinakotheke, restoring its prior bind arguments, and
re-enabling the reviewed proxy configuration. Never leave both listeners
competing for the same port.

## Reuse across Mnemosyne products

Other Rust/Axum products should reuse these invariants:

- direct Rustls termination with TLS 1.2/1.3 defaults;
- explicit certificate-chain and private-key paths;
- fail-closed paired configuration and startup validation;
- SAN-based identity and a deliberately installed trust root;
- private key permissions and no key material in repositories or logs;
- host-owned authentication kept distinct from transport security; and
- service-manager health, rotation, expiry, and rollback procedures.

Certificate generation may be shared as a library or operator tool, but web
products should not each invent a new CA during routine startup. Stable trust
roots are an installation concern; direct HTTPS serving is an application
concern.
