# XIMG-003 compatibility matrix

Status: proposed planning contract
Snapshot: 2026-07-14 UTC
TODO: XIMG-003

This matrix records the sibling revisions inspected for the x-img planning
baseline. A sibling commit is an inspection pin, not a runtime dependency.
x-img must consume published crates or copied, versioned wire contracts and
synthetic fixtures. Unknown future schema majors fail closed.

## Pinned contract surfaces

| Boundary | Inspected commit | Contract documents and source paths | Fixture or identifier anchors | x-img compatibility rule and status |
| --- | --- | --- | --- | --- |
| Monas standalone host, product mount, and authentication | `../monas` `3d21b0bc7b83fa8408d01b93347a56f43f3a96b7` (workspace `0.1.8`) | `README.md`; `crates/monas-core/src/lib.rs` (`ProductMount`, `ProductBootstrapResponse`, `HostInfoResponse`); `crates/monas-core/src/auth.rs`; `crates/monas-server/src/lib.rs` | `monas_standalone`; `/opt/<productName>`; `/api/auth/{register,login,logout,session}`; `monas_session`; tests `host_info_declares_standalone_storage_boundary`, `mnematikon_bootstrap_declares_monas_entrypoint_and_auth`, and `product_shell_requires_cookie_session` | x-img consumes authenticated host context and host-relative mounts; it does not implement login, cookies, or session issuance. The current mount examples (`mnematikon` `0.18.20`, `/products/mnematikon/...`) are anchors, not x-img values. **Status: host adapter not implemented.** |
| DASObjectStore application auth and object authority | `../DASObjectStore` `95cb4229cebec1290b8e0945a468c00d22152b5e` (workspace `0.76.27`) | `docs/application-authentication.md`; `docs/versioning.md`; `docs/format-registry.md`; `docs/user/service-boundary.rst`; `docs/user/object-stores.rst`; `docs/user/programmatic-access.rst`; `docs/user/remote-upload.rst`; `crates/dasobjectstore-core/src/application_auth.rs`, `manifest.rs`, `object_catalogue.rs`, `remote_upload.rs`; `crates/dasobjectstore-daemon/src/api/application_token.rs`, `provider_stream.rs`, `object_browser.rs`, `stores.rs`; `crates/dasobjectstore-gui-api/src/object_stores_aggregator.rs`, `remote_upload_aggregator.rs` | `dasobjectstore.application_auth.v1`; `dasobjectstore.provider_stream.v1`; `dasobjectstore.profile_browser.v1`; `dasobjectstore.profile_s3.v1`; `OBJECT_STORE_MANIFEST_SCHEMA_VERSION = 1`; `PORTABLE_OBJECT_CATALOGUE_SCHEMA_VERSION = 1`; public auth fixtures `application-auth/{identity,key,exchange-request,access-token,renewal-token,completion-capability}.json`; synthetic IDs include `codex`, `codex-fixture`, and `store-1` | x-img writes only through an authorized DAS boundary, with short-lived scoped access or upload completion, exact length/checksum verification, range-capable reads, and idempotent reconciliation. Copy only the named schema IDs and redistributable fixture shapes into x-img contract tests; never copy payloads or secrets. **Status: authority contract pinned; live route adapter remains gated.** |
| Mnemosyne design language and UI interaction contract | `../mnemosyne_design_language` `5539df8f662a78ebdf7cf4c868d71831380c8cfd` (package `@mnemosyne/design-language@0.1.0`) | `docs/brief.md`; `docs/interface-patterns.md`; `docs/authoring.md`; `tokens/core.json`; `assets/css/tokens.css`; `assets/css/brand-footer.css`; `docs/provenance.json` | Semantic tokens such as `color.brand.footer`, `color.surface.raised`, `color.text.primary`, `color.action.primary.default`, and `color.status.*`; acceptance anchors are the required Mnemosyne footer, one partial mark per view, table/structured-list records, transient task panes, keyboard focus, and word-first states | x-img UI must consume semantic tokens and the approved footer/mark contract. Comparable account, endpoint, ObjectStore, and job records use tables; configuration and confirmation use task panes. No sibling path or unpublished package may be required by the public build. **Status: design reference pinned; UI not implemented.** |
| Future Synoptikon product, host, and catalogue adapter | `../mnemosyne` `ee21d98b23dec3caa6926d9d0dcc002989aa465b` (SDK `0.2.1`; API types `0.21.1`) | `mneion-api-types/schemas/CATALOGUE_FORMAT.md`; `mneion-api-types/schemas/HOST_PRODUCT_UI_ADAPTER.md`; `mneion-api-types/schemas/HOST_STORAGE_BOUNDARY.md`; `mneion-api-types/schemas/MANIFEST_COMPATIBILITY.md`; `mnemosyne-product-sdk/Cargo.toml`; `mnemosyne-product-sdk/src/{manifest.rs,product_host.rs,product_api_routes.rs,product_ui.rs,endpoint.rs,conformance.rs}`; `mneion-api-types/src/catalogue.rs` | `mnemosyne.product.manifest.v1`; `mnemosyne.catalogue.v1`; `mnemosyne.product_ui.bootstrap.v1`; `mnemosyne.product.health.v1`; `mnemosyne.product.info.v1`; host modes `synoptikon_integrated` and `monas_standalone`; valid fixtures `mnematikon-dual-host.json`, `phoreus-dual-host.json`, `phoreus-monas-standalone.json`, `hermeneia-dual-host.json`; negative fixtures `invalid-product-id.json`, `missing-support.json` | **Future Synoptikon assumption:** x-img will expose a dual-host product manifest and use host-relative API/bootstrap contracts, with Synoptikon owning tenant/project context, audit, RDBMS state, and durable artefact registration. x-img must not depend on the current `publish = false` SDK or its unpublished path dependency; pin schemas/fixtures until a public package exists. **Status: assumption only; no Synoptikon adapter is authorized.** |

The x-img policy and architecture decisions that consume this matrix are
`docs/adr/0002-platform-policy.md`,
`docs/adr/0003-bioinformatics-resource-commit.md`,
`docs/adr/0004-endpoints-and-objectstores.md`, and
`docs/adr/0005-video-selection-normalization.md`. They remain proposed gates;
they do not make sibling APIs available to a public build.

## Versioning and public-build rule

- The external schema identifiers above are compatibility keys. A changed
  major requires an explicit x-img adapter/fixture review; readers reject an
  unknown future major rather than guessing.
- x-img-owned JSON and HTTP contracts should use stable, namespaced identifiers
  such as `ximg.<surface>.v1`, carry exact endpoint/ObjectStore and provenance
  IDs where applicable, and preserve additive migration rules separately from
  sibling schemas.
- Contract tests must vendor only synthetic or redistributable JSON fixtures
  under x-img. The DAS application-auth fixture names above are shape anchors;
  all credentials, signed material, private URLs, account lists, and media
  bytes remain absent.
- A clean public clone must build without `path = "../monas"`,
  `path = "../DASObjectStore"`, `path = "../mnemosyne"`, or
  `path = "../mnemosyne_design_language"`. Sibling checkouts may be used by
  optional compatibility CI, never by the baseline build.
- Re-pin this matrix whenever a compatibility-sensitive adapter is implemented
  or a sibling contract changes. Record the new full commit and refresh the
  copied fixture/schema evidence in the same change.

## Unresolved risks

1. All four sibling revisions are moving pre-release surfaces. The commits
   provide reproducibility, but they are not a substitute for published
   semantic-version ranges or a compatibility promise.
2. Monas currently declares `object_store_required = false` and local
   filesystem authority for its generic standalone host. x-img simultaneously
   requires DASObjectStore for durable media. A product-specific manifest and
   host capability extension must resolve this difference before Monas writes
   are implemented; x-img must never silently fall back to an unmanaged folder.
3. DAS documentation defines the authority, scoped authentication, streaming,
   range, and verification obligations, but exact public upload/read route
   names and stable wire schemas remain implementation-sensitive at this pin.
   XIMG-032 through XIMG-034 therefore remain blocked on versioned contract
   fixtures and cross-repository tests.
4. The future Synoptikon SDK expects integrated host context and object-store
   artefact authority, while x-img’s current catalogue and acquisition schemas
   do not yet exist. The adapter must be designed after XIMG-005/XIMG-006 and
   must preserve Monas and Synoptikon authority boundaries rather than sharing
   local persistence assumptions.
5. The design-language repository documents a package release workflow, but
   this snapshot is still a source checkout. Pin a published design-language
   package or vendor an explicitly reviewed token/asset contract before a
   reproducible UI release.
6. Monas has no dedicated checked-in JSON contract-fixture directory at this
   pin; x-img must author synthetic host/bootstrap/session fixtures before
   implementing the host adapter.
7. DAS internal provider-stream ranges are not yet the Firefox-facing HTTP
   read contract: the current GUI object-download route does not provide
   `Range`, `Content-Range`, ETag, or conditional-GET behavior. XIMG-034 and
   later Firefox range work remain gated until that boundary is versioned and
   tested.
