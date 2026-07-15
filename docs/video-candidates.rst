Video candidate plans and codec gaps
====================================

Video acquisition starts with a metadata-only candidate plan.  It is a
DownloadThemAll-like task-pane record, not a downloader: it never opens a
video, fetches bytes, reads browser cookies, extracts credentials, or starts a
transfer.  The candidate has to originate from a video actually observed on an
enabled page or one the user has explicitly selected.

Reviewing a candidate
---------------------

The task pane shows the title and source context, duration, dimensions,
container, video and audio codecs, estimated size, subtitle languages,
adapter/version, policy/support state, reviewed endpoint plus ObjectStore, and
the intended ``pinakotheke.firefox-h264-aac-mp4.v1`` playback profile.  It
uses the reviewed endpoint/ObjectStore pair exactly as selected; it does not
fall back to another store.

The user must explicitly confirm an eligible candidate before a future worker
may make a transfer or normalization job.  Confirmation still does not commit
anything.  Later work must revalidate policy, rights, destination health,
quota, authorization, and the normalization profile immediately before a
commit.

Candidates are blocked, with an explanation, when they were neither observed
nor selected, policy or rights disallow acquisition, the media is DRM-protected,
the delivery is segmented without a proven adapter, or the reviewed destination
is not a video ObjectStore.  The extension must not automatically open media,
traverse hidden items, crawl a playlist/channel, simulate browsing, bypass DRM,
or collect cookies or credentials.

Codec-gap prioritisation
------------------------

The planner classifies MP4 containing H.264/AVC1 video and AAC audio as
already Firefox-compatible.  Other eligible combinations remain reviewable,
but are marked ``RequiresNormalization`` for a later, versioned conversion
profile.  This is deliberately not a claim that the original is safe to
advertise as playable.

For prioritisation, x-img keeps a deduplicated, deterministic aggregate of the
enabled origin, container, video codec, audio codecs, and occurrence count.
It records no media bytes.  Individual page or media URLs, titles, signed query
parameters, cookies, authorization, credentials, and browsing history must
not be placed in project tickets or diagnostics.  Maintainers review an
aggregate recurring gap, then open or update a redacted GitHub issue and add a
synthetic fixture before implementing an adapter or playback profile.
The project-level tracker is `GitHub issue 1`_; it accepts only this aggregate
triage information.

.. _GitHub issue 1: https://github.com/sagrudd/x-img/issues/1

Verification
------------

Build the user documentation locally in its pinned container:

.. code-block:: console

   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check

Compatibility-sensitive sibling sources used for this boundary are Monas
``3d21b0bc7b83fa8408d01b93347a56f43f3a96b7``, DASObjectStore
``264670540972d8b00c3997cedaa3e86635532cbf``, Mnemosyne design language
``5539df8f662a78ebdf7cf4c868d71831380c8cfd``, and Mnemosyne
``52810176bf95a170f93d74a6f5daa94da5c6640e``.  The public build has no path
dependency on those checkouts.
