Official X OAuth authorization
==============================

XIMG-040 implements only the protocol boundary for the official X OAuth 2.0
Authorization Code flow with S256 PKCE. It is not a live connector, scraper,
cookie bridge, or token store. X documents this flow, exact redirect matching,
granular scopes, and ``offline.access`` refresh tokens in its `OAuth 2.0 PKCE
guide <https://docs.x.com/fundamentals/authentication/oauth-2-0/authorization-code>`_.

Host-owned secrets and tokens
-----------------------------

Monas supplies random state and PKCE verifier material, keeps the verifier
under an opaque transaction reference, and owns any client secret, token
exchange, refresh, revocation, and credential custody. x-img receives only an
opaque ``monas.x-oauth:`` credential reference, an opaque host actor reference,
the viewing X user ID, granted scopes, and expiry. It never stores a password,
browser cookie, authorization code, access token, refresh token, or client
secret.

The fixed least-privilege baseline is ``tweet.read``, ``users.read``,
``follows.read``, and ``offline.access``. The flow constructs only the official
authorize URL with ``response_type=code`` and ``code_challenge_method=S256``;
its host port uses the official token endpoint. A callback must match a pending
state once, before its short expiry, or it is rejected. Denial, replay, missing
scope, expired grant, and host failure all fail closed.

Protected content and live gate
-------------------------------

A protected-content request is admissible only when its viewing X user ID
matches the user bound to the host-held grant. This is authorization evidence,
not permission to acquire any content. The unresolved X approval, use-case,
rights, retention, and deletion gates in ADR 0002 still block live API calls
and media acquisition. The current implementation is synthetic protocol
coverage for state, PKCE, token refresh, revocation, and account binding.

Verify this documentation locally:

.. code-block:: console

   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check
