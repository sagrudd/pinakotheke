Install, upgrade, and rollback acceptance
=========================================

XIMG-086 has a local, production-shaped acceptance command:

.. code-block:: console

   make packages
   make upgrade-rollback \
     BASELINE_DIST=/path/to/x-img-0.2.0-artifacts \
     BASELINE_VERSION=0.2.0

The command never requires hosted CI. It verifies the candidate's complete
twelve-artifact manifest, then uses digest-pinned Debian and Fedora containers
for both x86_64 and arm64. Each genuine baseline package is installed and
executed, upgraded to the candidate, downgraded to the baseline, and removed.
The CLI and Monas bootstrap version must change at each boundary. A separately
mounted metadata directory must retain an exact SHA-256 throughout. This proves
that packaging does not claim or rewrite x-img catalogue state.

The same run executes the strict metadata export, restore, repeat-migration,
corruption, future-schema, and Firefox re-pairing tests. Endpoint,
ObjectStore, object, checksum, review, and historic identity evidence must
survive exactly. No media bytes or credentials are used. Finally, the runner
checks the Monas product/auth paths at commit
``3d21b0bc7b83fa8408d01b93347a56f43f3a96b7`` and DASObjectStore authority
paths at commit ``76f6411eab1e2c486c0bc1b4695b71f09307d9df``.

Acceptance evidence
-------------------

The 0.3.0 acceptance used the locally verified 0.2.0 artifact set created by
XIMG-085 as its genuine baseline. The full 0.2.0 → 0.3.0 → 0.2.0 lifecycle
passed for DEB and RPM on x86_64 and arm64. The same run proved logical metadata
rollback and the pinned host/authority contracts. Package bytes remain outside
the repository and must be supplied explicitly; the runner never silently
manufactures a baseline.

macOS PKGs and Firefox XPIs remain structurally verified by ``make verify``;
signing/notarization and publication are XIMG-087 release-candidate gates.
