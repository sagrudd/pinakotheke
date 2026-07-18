Endpoint and ObjectStore destinations
=====================================

x-img distinguishes a storage endpoint/device from a logical ObjectStore. The
stable endpoint ID identifies a managed local DASObjectStore service or a remote
appliance; the stable ObjectStore ID identifies one store discovered through
that endpoint. Display names are labels only and are never used as a write or
provenance identity.

The strict public contract is ``x-img.das-destination-inventory.v1``. Its
synthetic inventory fixtures cover a managed local folder profile, a remote
appliance with multiple stores, an explicit site override, and fail-closed
unmanaged-folder, mutable-ID, broad-secret, and arbitrary-first-store cases.
It was reviewed against DASObjectStore commit
``76f6411eab1e2c486c0bc1b4695b71f09307d9df``. This is a copied x-img
contract, not a sibling path dependency or a live pairing client.

Local and remote setup
----------------------

The default local destination is a DASObjectStore-managed folder profile. The
x-img service may request and show its provisioning state, but it never accepts
an arbitrary filesystem path or writes a browser-selected folder directly.

Remote destinations begin with an HTTPS endpoint and a Monas-authenticated,
DASObjectStore-approved pairing. Browser and x-img configuration retain only
opaque host, provisioning, and pairing references. They never contain a raw
password, S3 secret, certificate, bearer token, or long-lived broad credential.

Selection and commit safety
---------------------------

After pairing, the authority supplies every visible ObjectStore. XIMG-037
exposes each one as a structured destination row. The Pinakotheke library now
loads the signed-in actor's inventory through Monas and shows endpoint and
store together, including stable IDs, labels, health, write
capability, quota, and compatible object types. It must show
``Ready``, ``Read-only``, ``Unavailable``, or ``Needs reconnect`` in words;
colour is only supplementary.

An endpoint with multiple stores requires an explicit default. A site or
resource plan may override that default, but every reviewed write remains bound
to one endpoint ID and one ObjectStore ID. x-img never chooses the first remote
store, silently falls back to another store, or treats a renamed label as a new
destination.

Immediately before a real commit, the server adapter must revalidate the
host context, pairing/TLS state, endpoint/store existence, health, writable
capability, object type, policy, quota, and reviewed stable-ID pair. The
catalogue provenance records endpoint ID, ObjectStore ID, object key, checksum,
actor/session reference, and commit time. XIMG-037 validates reviewed rows and
revalidates the exact stable-ID pair against authority state. The current Yew
selector now reads and explicitly saves the actor's reviewed stable IDs through
``GET``/``PUT /products/pinakotheke/api/destinations/v1/reviewed``. The private
``state/reviewed-destinations.v1.json`` authority uses optimistic revisions,
atomic replacement, mode ``0600``, strict schema validation, and actor scope;
it survives restart and never chooses another actor or the first visible store.
On migration, the configured capture-authority pair seeds each known active
actor only when no persisted selection exists.

The first 1.13 slice still treats the host capture authority as the selection
allowlist: a PUT naming any other pair is rejected even if the browser's DAS
dashboard displayed it. This deliberately avoids converting presentation data
into authority. Plan-bound snapshots and live DAS authority revalidation
immediately before worker execution remain the XIMG-098 completion gate. Until
then the running helper continues to use the configured reviewed authority pair.
