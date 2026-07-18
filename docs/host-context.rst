Authenticated host context
==========================

x-img does not authenticate browser sessions. Monas validates its session
cookie before dispatching to the x-img mount; Synoptikon will validate its own
request context before dispatching. The x-img API accepts only a host-injected,
non-secret ``AuthenticatedHostContext`` extension.

The versioned fixture contract is ``x-img.host-context.v1``. The additive
canonical form carries the Prosopikon ``authority_id`` (the issuer identity),
``principal_id``, ``session_id``, and exact ``pinakotheke`` audience as one
all-or-none group, alongside the compatibility actor identifier,
authorizations, and correlation identifier. UUID assertions are not bearer
credentials and must only arrive through authenticated host dispatch. The
document intentionally has no cookie, password, token, credential, or storage
secret field. Pinakotheke neither logs nor persists the context or session ID.

Admission
---------

The host adapter requires the product authorization ``ximg.access``. A direct
request to a privileged x-img API route has no injected context and receives
``401 Unauthorized``. A context that reaches the route without the required
authorization is rejected with ``403 Forbidden``. The ``/health`` route is the
sole public scaffold probe and is not a product action.

Host replacement
----------------

``MonasHostContextAdapter`` and ``SynoptikonHostContextAdapter`` implement the
same narrow adapter contract. The former is shaped against ``../monas`` commit
``dac0e113c8b197cb06abc38187d72f27e562ad63`` and Prosopikon commit
``0ca5326498050c0719fe241bd12b1186e4658ce3``; the latter remains shaped against
``../mnemosyne`` commit ``9877017e3139711ed6313c53603409c53020541d``. These are
compatibility inspection pins, not path dependencies. Synthetic fixtures prove
that either host can supply a valid, authorized context and that missing product
authorization fails closed.

Host integrators must authenticate before constructing this extension. They
must never forward raw session cookies or credentials to x-img, and x-img must
never create login, logout, registration, or session-validation routes.
The original context shape remains accepted only for extension-pairing v1
compatibility. Any partial canonical group, wrong audience, or malformed or
nil UUID fails closed. Compatibility retirement remains gated on inventory,
rollback, mapping, and canary acceptance.
