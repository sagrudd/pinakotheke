# AGENTS.md

This file defines mandatory working rules for human and automated contributors
to x-img.

## Start-of-run reading

Before changing the repository, read this file, `README.md`, `MILESTONES.md`,
and `TODO.md`. For UI or integration work also read the current relevant files
in these sibling repositories when present:

- `../mnemosyne_design_language/docs/brief.md` and
  `docs/interface-patterns.md`;
- `../monas/README.md` and its product/auth contracts;
- `../DASObjectStore/docs/application-authentication.md`, storage/read/upload
  contracts, and versioned fixtures; and
- `../mnemosyne` product SDK/catalogue contracts for future Synoptikon work.

Record the sibling commit used by any compatibility-sensitive change. The
public x-img build must remain usable without unpublished path dependencies;
consume versioned wire contracts or fixtures until dependencies are published.

## Strict single-run lease

Only one top-level x-img work run may mutate, test, commit, or push at a time.
Subagents delegated by that admitted coordinator are part of the same run.

1. The coordinator's first stateful action must atomically create the directory
   `.codex/x-img-run.lock` using `mkdir` without a preceding check.
2. If creation fails, do not edit, test, commit, push, clear the lock, or wait in
   the workspace. Report that another run owns the lease and stop.
3. Write a small owner note inside the lock containing the run/thread identity
   and UTC start time. Do not put secrets in it.
4. Hold the directory for the entire run, including all subagent work, tests,
   commits, and pushes.
5. Remove the directory only after every delegated agent is stopped and the
   final push/handoff is complete.
6. Never steal or automatically expire a lock. After a crash, a human must
   verify that no run or subagent remains before removing it.

`.codex/` is ignored and must never be committed.

## Automation and subagents

Every admitted scheduled run must use subagents to deliver clear,
non-overlapping quanta. The coordinator remains responsible for architecture,
integration, review, tests, commits, and the final handoff.

- Delegate at least one bounded research/review quantum and one bounded
  implementation/test/documentation quantum when the selected TODO supports
  parallel work.
- Give every subagent explicit file ownership or a read-only task. Tell it that
  other agents share the worktree and it must not revert unrelated changes.
- Do not let two subagents edit the same file or modify shared manifests in
  parallel.
- Subagents do not commit or push. The coordinator inspects and integrates their
  output, runs verification, then commits and pushes.
- If safe non-overlapping quanta cannot be defined, subagents perform read-only
  review, test selection, threat analysis, or acceptance-criteria validation.
- Before releasing the lease, ensure all subagents have completed or been
  explicitly stopped.

## Backlog discipline

- Work from the first dependency-ready unchecked TODO item.
- Do not write live connector code until its P0 policy and contract gates are
  complete.
- Treat every TODO item as an acceptance contract, not a suggestion.
- Mark an item complete only when implementation/docs, relevant tests, commit,
  and push are complete. Add the commit hash.
- Update `TODO.md` and `MILESTONES.md` when evidence changes scope or ordering.
- End every run with a concise handoff in `TODO.md` if work remains partially
  complete; do not mark partial work complete.

## Commit, push, and version policy

- Commit each coherent modification separately and push immediately after that
  commit. Do not accumulate unrelated completed changes.
- Keep the tree clean between quanta when practical. Never use destructive
  reset/checkout commands against user work.
- Stage explicit paths; inspect `git diff --check`, staged diff, and status
  before commit.
- Use imperative, focused commit subjects. Include the TODO ID when applicable.
- Use Semantic Versioning. Workspace/package version is the Rust source of
  truth; extension and product metadata must be checked against it.
- Update `CHANGELOG.md` for released behavior. Compatible fixes use patch,
  backward-compatible features use minor, and breaking public/schema changes
  require an agreed major bump.
- Never rewrite public history or force-push unless the user explicitly asks.

## Authority boundaries

- DASObjectStore is the only durable authority for image/video bytes. x-img
  must not retain payload files in its product root, database, browser storage,
  logs, test fixtures, or repository.
- x-img may own versioned account/site JSON, catalogue identifiers, object
  references, acquisition/audit state, thumbnails only if they are separate
  DASObjectStore objects, and review state.
- Monas owns the standalone app shell, login, session cookies, and authenticated
  host context. Do not add x-img user/password/session issuance.
- X/Instagram credentials use official user authorization and host-managed
  secret references. Never ingest browser cookies or passwords.
- The Firefox extension is a least-privilege client of one configured x-img
  instance. Sites require explicit user enablement and origin permission.
- Firefox capture and substitution are per-site opt-in, transparent, and
  routed through that same x-img instance and DASObjectStore authority. The
  extension must never automatically open pages, traverse hidden content,
  bulk-crawl, or simulate browsing. It may cache a thumbnail only when that
  thumbnail was actually displayed or observed; it may capture/cache an
  original only after the user explicitly opens it. It must never extract or
  forward site cookies or credentials, and avoiding an API does not exempt any
  behavior from platform terms.
- The browser cache must fail open to the origin and must never break ordinary
  page loading when x-img, Monas, or DASObjectStore is unavailable.
- Bioinformatics resources are a separate adapter family from Firefox/site
  media. Accept only a bounded, explicit user accession or URL for GEO, SRA,
  ENA, or NCBI; never add bulk discovery, repository crawling, or an implicit
  catalog sweep. Resolve a reviewable plan and obtain user confirmation before
  transfer, then revalidate policy, destination, quota, and write capability
  server-side immediately before commit.
- Bioinformatics bytes stream directly through the authorized DASObjectStore
  ingest boundary. Only bounded ephemeral worker buffers are permitted; no
  durable x-img-local payload or browser-storage copy may be used. Preserve
  accession/file identity, source release and URLs, checksums, transport/tool
  versions, rights metadata, endpoint/ObjectStore identity, object reference,
  and discovery/commit times. Aspera is optional, explicitly licensed and
  binary-bounded, HTTPS fallback is mandatory, and secrets never enter config,
  logs, or fixtures.

## Data integrity and privacy

- “Only once” means idempotent verified commit keyed by canonical platform media
  identity and checksum. Network retries may occur; duplicate committed objects
  must not.
- Use explicit state machines and crash reconciliation. Never mark media new or
  reviewed before the DASObjectStore commit is verified.
- Preserve provenance: platform/site, account/origin, source item, canonical
  media identity, source URL alias, discovery time, object checksum/reference,
  connector/adapter version, and policy result.
- Reject unknown future schema majors. Migrations must be explicit, tested, and
  non-destructive by default.
- Fixtures must be synthetic or redistributable. Never commit real account
  lists, downloaded media, tokens, cookies, credentials, or private URLs.
- Logs and diagnostics must redact authorization, signed-query parameters,
  Monas sessions, DAS credentials, and user browsing history.

## Product identity and documentation

- `x-img` is the planning/repository name until the coordinated v1.0.0
  rebrand. The v1.0.0 product and brand target is **Pinakotheke**, with the
  target GitHub repository slug `sagrudd/pinakotheke`. Do not perform a partial
  rename. The release gate covers documentation, Rust/code identifiers,
  CLI/package/product metadata, Monas/Synoptikon/DASObjectStore adapters,
  Firefox extension identity, and repository migration, with documented
  compatibility aliases and migrations where existing names or schemas must
  remain readable.
- Precise user-facing documentation is authored as a Sphinx project in `docs/`
  using Read the Docs-compatible configuration and reStructuredText entry
  points. The reproducible local authority is the pinned `docs/Dockerfile`.
  Build and verify it locally with:

  ```sh
  docker build --pull --progress=plain -f docs/Dockerfile -t x-img-docs:check .
  ```

  Run the documented container check as well. GitHub Actions may mirror these
  checks but never replaces local container verification as the release
  authority.
- User-facing resource documentation must include explicit GEO, SRA, ENA, and
  NCBI accession/URL examples, plan confirmation, rights/policy blocking,
  destination selection, checksum failure, retry/resume, and local container
  verification.

## Architecture and code quality

- Rust first; use `clap`, `axum`, and `yew` for their intended boundaries.
- Keep domain, ports, connectors, host adapters, persistence, scheduling, and UI
  modular with narrow interfaces and no circular dependencies.
- Prefer shared/generated versioned types over duplicated request structures.
- Keep policy server-side; the UI and extension request plans and render results.
- Use bounded streaming and backpressure for media. Never buffer unbounded video
  payloads in memory.
- Use the smallest permissions and dependencies possible. Pin compatibility-
  sensitive browser and wire behavior with tests.

## Interface requirements

- Follow `../mnemosyne_design_language`; semantic tokens only in components.
- The application shell includes the required Mnemosyne footer and one partial
  decorative mark per view.
- Product/data context leads; actions are scoped and forms live in accessible
  task panes rather than permanent administration cards.
- The media gallery may use cards because the records are inherently visual,
  but comparable account/job/setting records use tables or structured lists.
- State uses words, not colour alone. Design loading, empty, stale, permission,
  transport, partial-failure, and object-unavailable states.
- The gallery distinguishes `Previously observed` thumbnails from `Stored in
  ObjectStore` committed originals. The distinction uses words and iconography
  as well as colour, has an accessible reversible frame/badge/overlay with a
  tooltip, does not obstruct media, and can be toggled by the user. It must
  never watermark or mutate stored media bytes.
- Preserve keyboard navigation, focus entry/trap/return, responsive behavior,
  and WCAG 2.2 AA contrast.

## Verification and Definition of Done

Select checks proportionate to the change. The eventual baseline includes:

- Rust format, lint, native tests, and wasm checks;
- JSON schema and compatibility fixtures;
- connector fixture and crash/idempotency tests;
- Monas and DASObjectStore contract tests;
- real Firefox tests for permissions, capture, redirect, range, and fail-open;
- Sphinx/Read the Docs documentation built and verified in the reproducible
  local container, independent of GitHub Actions;
- accessibility, privacy, secret-scan, dependency, license, and vulnerability
  checks; and
- a clean public clone/build without sibling-only files.

Work is done only when acceptance criteria pass, user-facing Sphinx docs are
updated and locally container-verified, docs/backlog/changelog are aligned,
the focused commit is pushed, and no run-owned lock or subagent remains.
