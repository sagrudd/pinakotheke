Site adapter registry
=====================

XIMG-063 defines a versioned explicit adapter registry. Matching canonicalizes
an HTTPS origin, applies path exclusions, and requires experimental mode for
the generic observed-image adapter. Each adapter declares its capture and
substitution capabilities; no capability is inferred and the registry alone
does not enable capture or substitution.

The experimental generic adapter may match an arbitrary HTTPS origin only
after that exact origin is enabled in the site policy UI and granted as an
optional Firefox permission. This is not a wildcard site policy: excluded
paths, media classes, and capability gates still apply. Its explicit-original
capability accepts only a trusted image-link or image-document click.

Segmented HLS/DASH capability is not a boolean shortcut.  It requires the
versioned evidence contract in :doc:`segmented-video-gate`; the generic adapter
remains explicitly disabled and origin-served.
