Firefox extension scaffold
==========================

XIMG-060 supplies a Manifest V3 Firefox extension scaffold for one configured
x-img instance. Its baseline permissions are only ``storage``, ``activeTab``,
``scripting``, and ``permissions``. Site access is an optional exact HTTPS
origin permission requested only from the options page; private browsing is not
enabled by default.

The options page shows the exact origin and consequences before permission,
chooses image/video media classes, and provides independent capture and
substitution toggles plus permission removal for each site.

The extension has no cookie, webRequest, history, password, or credential API;
it never opens pages, traverses hidden content, crawls, or simulates browsing.
The toolbar has no effect unless a paired instance and an explicitly enabled
site exist, and failures are silent so ordinary page use continues.

When the user clicks the toolbar on an enabled image site, only images actually
displayed in the current viewport are eligible for an ``observed_thumbnail``
capture plan. The extension does not automatically open an original. The
paired host must authenticate and authorize the plan before it enters the
shared scheduler; this is never a direct browser payload upload. See
:doc:`firefox-capture` for the endpoint, policy, and fail-open contract.

The toolbar now opens the explicit per-site cache control and diagnostic
surface described in :doc:`firefox-toolbar`. Image, normalized-MP4, and
fail-closed segmented behavior remain governed by their adapter/authority gates.
