Install, upgrade, and rollback acceptance
=========================================

XIMG-086 has a local, production-shaped acceptance command:

.. code-block:: console

   make packages
   make upgrade-rollback

The command never requires hosted CI. It verifies the complete twelve-artifact
manifest, then uses digest-pinned Debian and Fedora containers matching the
current host architecture. Each real package is installed, executed, checked
against the Monas bootstrap, reinstalled through the package manager, and
removed. A separately mounted metadata directory must retain an exact SHA-256
before and after the lifecycle. This proves that packaging does not claim or
rewrite x-img catalogue state.

The same run executes the strict metadata export, restore, repeat-migration,
corruption, future-schema, and Firefox re-pairing tests. Endpoint,
ObjectStore, object, checksum, review, and historic identity evidence must
survive exactly. No media bytes or credentials are used. Finally, the runner
checks the Monas product/auth paths at commit
``3d21b0bc7b83fa8408d01b93347a56f43f3a96b7`` and DASObjectStore authority
paths at commit ``73d3e6398cbfb8f7ac53b8040cea7c5b718ac140``.

Current limit
-------------

This is an acceptance foundation, not yet the final release-candidate rollback
claim. The repository has no earlier packaged x-img release, so the current
runner proves install/reinstall/remove plus logical metadata rollback rather
than a genuine cross-version package downgrade. XIMG-086 remains open until a
0.9 release candidate is exercised against its immediately preceding signed or
explicitly accepted development package on a production-like Monas plus
DASObjectStore deployment. The runner makes that remaining step repeatable and
keeps the limitation visible.

The current host architecture is executed through both Linux package managers;
the opposite architecture remains structurally verified by ``make verify``.
Run this command once on x86_64 and once on arm64 for final release evidence.
