Storage-authority acceptance (generated data only)
===================================================

This is the acceptance procedure for the RC-B storage-authority work.  It is
deliberately isolated from a normal Pinakotheke installation: it uses a fresh,
generated prefix and synthetic immutable metadata records.  It must never be
pointed at a personal gallery, a
production ObjectStore, an existing media prefix, a browser profile, or any
file containing credentials.

The procedure tests the authority relationship, not a provider shortcut.
DASObjectStore is the durable byte and catalogue authority; Pinakotheke is a
rebuildable projection that may advertise an object only after an authority
completion receipt has been verified.  A direct provider/S3 write, bucket
listing, or raw provider delete is not acceptance evidence.

Scope and safety boundary
-------------------------

``make storage-authority-check`` executes the deterministic metadata-only
proof from a fresh system temporary prefix; it does not accept a caller path or
destination.  The installed-host runner remains a separate RC-E obligation.
The synthetic proof refuses fixture input that names any of the following:

* an empty, relative, symlinked, non-owned, or non-empty prefix;
* a prefix that is a Pinakotheke data root, DASObjectStore data root, home
  directory, repository, browser profile, or parent of any of those paths;
* an ObjectStore, endpoint, or prefix that is not explicitly identified as the
  disposable synthetic acceptance destination; and
* a configuration or environment value containing a session, renewal token,
  password, provider credential, signed URL, site cookie, or real source URL.

The generated metadata records are fixed synthetic test data.  The command
may retain only bounded, redacted identifiers and aggregate outcomes in its
evidence output.  It must remove its generated prefix only after the run has
written a successful or failed redacted summary; it must never clean a path it
did not create during that invocation.

Authority receipt and admission
-------------------------------

The deterministic proof models one synthetic image and one short synthetic
video through the same authority/admission state transitions used by the
product.  For each object, it records only the stable endpoint ID, ObjectStore
ID, immutable object reference, byte length, SHA-256, media type,
request/correlation reference, and terminal state.  A real installed-host
API/UI exercise remains an RC-E obligation.

Before a gallery projection becomes ``Ready``, the run must prove all of these
conditions:

1. DASObjectStore accepted the authorized request and returned a matching
   completion receipt.
2. The receipt binds the reviewed endpoint and ObjectStore, object reference,
   exact length, media type, and SHA-256.
3. DASObjectStore has verified and published the object through its
   authoritative catalogue boundary, including the configured logical-capacity
   accounting.
4. Pinakotheke admits exactly one matching projection record only after that
   verification.  A retry must not create a second durable object or gallery
   record.

The image and video are separate cases: the video is accepted only as a short
synthetic rendition with its typed media metadata.  This check does not claim
that arbitrary source video, a source original, a poster, or a remote provider
inventory is supported.

Deletion, retries, and capacity
-------------------------------

The run opens the admitted synthetic image and video through the ordinary
deletion review flow and confirms the exact objects.  It then requires the
application-authorized DASObjectStore delete contract to report ``deleted`` or
``already_absent`` for each reviewed immutable reference.  Pinakotheke removes
the corresponding projection only after that response is verified.

The acceptance record must show, for each synthetic object:

* provider absence as reported by the DASObjectStore authority;
* authoritative catalogue withdrawal;
* one logical-capacity debit from the configured ObjectStore; and
* a redacted audit outcome, with no object key, source alias, actor, token, or
  provider response body.

Repeat the identical deletion request after a successful delete.  The
``already_absent`` result is success, capacity is not debited twice, and no
projection reappears.  A deliberately injected authorization, transport, or
receipt mismatch failure leaves the projection visibly retryable and must not
claim that bytes were removed.

Crash, restart, and dry-run outcomes
------------------------------------

The runner injects a bounded stop at each supported boundary: before authority
completion, after completion before projection admission, during deletion, and
after provider absence before local projection removal.  After each restart it
reconciles from the applicable stable receipt and DASObjectStore catalogue
evidence to one terminal state.  It must neither duplicate an object nor
silently discard an unresolved projection.

An out-of-band DASObjectStore deletion is tested only as a scoped dry run until
the published DAS catalogue/reconciliation contract explicitly supports the
needed inventory query.  The dry run may report that a supplied synthetic
immutable reference would be marked unavailable or repaired; it must not list
a provider bucket, discover other objects, mutate a catalogue, or change a
Pinakotheke gallery record.  A future authoritative reconciliation pass must
remain bounded to the reviewed generated ObjectStore and must record whether
it repaired missing, orphaned, or stale projection state.

Run and evaluate
----------------

Run the deterministic local proof from the repository root.  It has no
destination, provider, credential, or user-data options; do not substitute a
local filesystem path or provider credential for destination authorization.

.. code-block:: console

   make storage-authority-check

The command is successful only when its redacted synthetic summary states that
both objects were admitted once, deleted or idempotently confirmed absent,
withdrawn from the simulated authoritative catalogue, debited once, and
reconciled across every injected restart.  Any invalid fixture, disagreement,
or unredacted synthetic output is a failure, not a skipped success.

Limitations and non-claims
--------------------------

This isolated acceptance does not validate a user's existing gallery, import
real images or videos, discover a remote ObjectStore inventory, or grant
Pinakotheke provider credentials.  It does not establish remote catalogue
inventory support before a versioned DASObjectStore contract publishes that
capability.  It also does not replace the independent Firefox, video
normalization, Monas, packaging, or installed-host acceptance gates.

For the existing user-facing deletion semantics, see :doc:`deletion-compliance`.
For the projection/authority distinction and its current operational contract,
see :doc:`gallery-convergence`.
