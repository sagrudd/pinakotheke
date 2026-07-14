# x-img

`x-img` is a planned personal media-acquisition and review service for a small,
explicitly configured set of X/Twitter and Instagram accounts, together with
websites enabled through a Firefox extension.

All sources resolve to one x-img Web instance. That instance presents a
thumbnail-dense, ThumbsPlus-inspired library and review queue, and offers a
single-click refresh of configured social accounts.

## Non-negotiable boundaries

- Rust implementation with `clap` for CLI surfaces, `axum` for HTTP/API
  adapters, and `yew` for the Web UI.
- Interface hosting and login are owned by sibling `../monas`; x-img must not
  create a competing account or session system.
- Image and video bytes are stored only in a DASObjectStore ObjectStore through
  sibling `../DASObjectStore`; local x-img state may contain configuration,
  identifiers, indexes, and audit records, but never durable media payloads.
- Account and site configuration is local, explicit, versioned JSON.
- Acquisition is idempotent: once a media identity has a verified committed
  object, routine refreshes do not download it again.
- The UI follows sibling `../mnemosyne_design_language` and retains a future
  Synoptikon adapter boundary for `../mnemosyne`.
- The source repository is public and licensed under MPL-2.0. Archived media is
  private user data and is never committed to this repository.

## Current status

Planning only. See [MILESTONES.md](MILESTONES.md) for release gates and
[TODO.md](TODO.md) for dependency-ordered work. Automated contributors must
follow [AGENTS.md](AGENTS.md).

## Key concerns before implementation

1. X and Instagram access, storage, display, and deletion obligations must be
   reviewed against current platform terms and the user's rights to each work.
2. Protected/private account access requires an official user-authorized flow;
   browser cookies and credentials must never be scraped or copied into x-img.
3. “Exactly once” is implemented as idempotent committed acquisition, not as an
   impossible promise that a network request can never be retried after a crash.
4. Video caching requires byte-range and segmented-media behavior to be proven
   in Firefox before the external-cache design is declared viable.
5. The public project must not redistribute downloaded media, tokens, account
   lists, DAS credentials, or Monas sessions.

## Versioning

The project uses Semantic Versioning. Planning starts at `0.1.0`; the stable
connector, storage, Web, and Firefox contracts are the `1.0.0` gate.

## License

Mozilla Public License 2.0. See [LICENSE](LICENSE).
