Authenticated host context
==========================

x-img does not authenticate browser sessions. Monas validates its session
cookie before dispatching to the x-img mount; Synoptikon will validate its own
request context before dispatching. The x-img API accepts only a host-injected,
non-secret ``AuthenticatedHostContext`` extension.

The versioned fixture contract is ``x-img.host-context.v1``. It contains only a
host name/mode, stable actor identifier, authorization identifiers, and a
correlation identifier. It intentionally has no cookie, password, session
token, bearer token, credential, or storage secret field. x-img neither logs
nor persists the context.

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
``6e62943dedbe21f0f7551d5fd1371f61f26fa42b`` and the latter against
``../mnemosyne`` commit ``9877017e3139711ed6313c53603409c53020541d``. These are
compatibility inspection pins, not path dependencies. Synthetic fixtures prove
that either host can supply a valid, authorized context and that missing product
authorization fails closed.

Host integrators must authenticate before constructing this extension. They
must never forward raw session cookies or credentials to x-img, and x-img must
never create login, logout, registration, or session-validation routes.
