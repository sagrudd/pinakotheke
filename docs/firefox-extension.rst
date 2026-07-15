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
