Website capture review admission
================================

XIMG-065 connects a verified website-capture acquisition to the common review
queue. It does not upload browser bytes, create a second catalogue, or admit a
capture plan by itself.

Admission rule
--------------

A Firefox capture plan first waits for a future approved acquisition worker to
create a normal acquisition lifecycle. Only an acquisition in ``Committed``
state with verified DASObjectStore evidence may enter the shared ``New`` queue.
Interrupted, failed, policy-blocked, unverified, or merely scheduled capture
plans are rejected.

The retained metadata-only provenance is:

* capture-plan identifier and enabled site identifier;
* exact origin plus redacted canonical page and media URLs;
* adapter kind and version; and
* discovery time.

This provenance is keyed by the canonical media identity and does not change
the verified ObjectStore object, its bytes, or its review state.

Deduplication
-------------

When a committed connector record already has the same canonical media URL as
a source alias, the browser capture reuses that connector's canonical identity.
It therefore produces one shared queue record rather than a website duplicate.
For a previously unknown site resource, x-img uses a domain-separated SHA-256
identity derived from the already redacted canonical media URL. The URL itself
remains provenance, not the stored identity.

Both paths still require the verified object checksum and normal reconciliation
rules; an observation never becomes a deduplication or commit proof alone.

Compatibility and verification
------------------------------

This metadata-only handoff was reviewed against Monas
``3d21b0bc7b83fa8408d01b93347a56f43f3a96b7``, DASObjectStore
``264670540972d8b00c3997cedaa3e86635532cbf``, Mnemosyne design language
``5539df8f662a78ebdf7cf4c868d71831380c8cfd``, and Mnemosyne
``52810176bf95a170f93d74a6f5daa94da5c6640e``. They are inspection pins, not
public-build dependencies.

The core tests prove verified-only admission, retained site/page/media/adapter
provenance, identity mismatch rejection, and reuse of a connector identity for
a matching committed alias. Build the documentation locally with:

.. code-block:: console

   docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
   docker run --rm x-img-docs:check
