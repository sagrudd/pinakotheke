# XIMG-003 compatibility matrix

Current XIMG-200 Synoptikon integration baseline: ``../mnemosyne`` commit
``52810176bf95a170f93d74a6f5daa94da5c6640e``. This supersedes the earlier
moving-source snapshot retained in the historical table below.

Current XIMG-091 managed local-profile baseline: ``../DASObjectStore`` commit
``0d71b2a197a310004b686bc2a4bff3e8fd9c6463`` (workspace ``0.84.0``). The
canonical helper admits only the exact Pinakotheke home root, accepts reviewed
logical-store parameters, and exposes a secret-free versioned identity.

Current XIMG-092 authenticated forwarding baseline: ``../monas`` commit
``6e62943dedbe21f0f7551d5fd1371f61f26fa42b`` (workspace ``0.2.0``). Monas
owns the Pinakotheke mount, Prosopikon cookie verification, strict context
injection, streaming loopback forwarding, and private dispatch configuration.

Current XIMG-095 login baseline: ``../monas`` commit
``a0fabe2d250f2d217765ee59a95cc2a04610bedc`` (workspace ``0.3.0``). Monas
owns the responsive login/Yew shell, strict same-origin deep-link return,
Prosopikon session establishment and revocation, approved-brand asset boundary,
and Pinakotheke redirect gate.

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
| Monas standalone host, product mount, and authentication | `../monas` `114ef9588b8906287381ec7708fb12c3d03efd7b` (workspace `0.6.2`) | `README.md`; `crates/monas-core/src/lib.rs` (`ProductMount`, `ProductBootstrapResponse`, `HostInfoResponse`); `crates/monas-core/src/auth.rs`; `crates/monas-server/src/lib.rs` | `monas_standalone`; `/opt/<productName>`; `/api/auth/{register,login,logout,session}`; `monas_session`; validated product-aware sign-on presentation; x-img `contracts/monas/x-img-product-bootstrap.v1.json`; x-img `fixtures/host-context/v1/monas-valid.json`; tests `host_info_declares_standalone_storage_boundary`, `login_page_uses_validated_pinakotheke_destination_branding`, and `product_shell_requires_cookie_session` | x-img consumes authenticated host context and host-relative mounts; it does not implement login, cookies, or session issuance. Monas owns the Pinakotheke-labelled, Mnemosyne-compliant sign-on and selects it only from the validated same-origin product return path. **Status: registration, host-context, and sign-on branding contracts complete.** |
| DASObjectStore application auth and object authority | `../DASObjectStore` `76f6411eab1e2c486c0bc1b4695b71f09307d9df` (moving source snapshot) | `docs/application-authentication.md`; `docs/versioning.md`; `docs/format-registry.md`; `docs/user/service-boundary.rst`; `docs/user/object-stores.rst`; `docs/user/programmatic-access.rst`; `docs/user/remote-upload.rst`; `crates/dasobjectstore-core/src/application_auth.rs`, `manifest.rs`, `object_catalogue.rs`, `remote_upload.rs`; `crates/dasobjectstore-daemon/src/api/application_token.rs`, `provider_stream.rs`, `object_browser.rs`, `stores.rs`; `crates/dasobjectstore-gui-api/src/profile_download.rs`, `remote_upload_aggregator.rs` | `dasobjectstore.application_auth.v1`; `dasobjectstore.provider_stream.v1`; `dasobjectstore.profile_browser.v1`; `dasobjectstore.profile_s3.v1`; `OBJECT_STORE_MANIFEST_SCHEMA_VERSION = 1`; `PORTABLE_OBJECT_CATALOGUE_SCHEMA_VERSION = 1`; public auth fixtures `application-auth/{identity,key,exchange-request,access-token,renewal-token,completion-capability}.json`; x-img `contracts/dasobjectstore/x-img-application-identity.v1.json`; x-img `contracts/dasobjectstore/x-img-destination-inventory.v1.json`; x-img `object_ingest::ObjectIngestBackend`; x-img `object_read::ObjectReadBackend`; x-img `bioinformatics_commit::ConfirmedBioinformaticsCommitter`; synthetic IDs include `codex`, `codex-fixture`, and `store-1` | x-img writes only through an authorized DAS boundary, with short-lived scoped access or upload completion, exact length/checksum verification, range-capable reads, and idempotent reconciliation. XIMG-032 adds strict non-secret registration/preflight scope; XIMG-033 adds bounded authority-stream and receipt verification; XIMG-034 adds stream-handoff validation for type/length/checksum/ETag/conditional/range/unavailable behavior without local payload caching; XIMG-036 adds stable endpoint/ObjectStore inventory and reviewed-destination validation; XIMG-038 composes confirmed bioinformatics plan metadata, exact destination revalidation, and verified streaming ingest without payload retention. Daemon proof/token issuance and live transport adapters remain gated. Copy only named schema IDs and redistributable fixture shapes; never copy payloads or secrets. **Status: identity, destination, ingest, read-port, and confirmed bioinformatics commit contracts complete; live transports remain gated.** |
| Mnemosyne design language and UI interaction contract | `../mnemosyne_design_language` `5539df8f662a78ebdf7cf4c868d71831380c8cfd` (package `@mnemosyne/design-language@0.1.0`) | `docs/brief.md`; `docs/interface-patterns.md`; `docs/authoring.md`; `tokens/core.json`; `assets/css/tokens.css`; `assets/css/brand-footer.css`; `docs/provenance.json` | Semantic tokens such as `color.brand.footer`, `color.surface.raised`, `color.text.primary`, `color.action.primary.default`, and `color.status.*`; acceptance anchors are the required Mnemosyne footer, one partial mark per view, table/structured-list records, transient task panes, keyboard focus, and word-first states | x-img UI must consume semantic tokens and the approved footer/mark contract. Comparable account, endpoint, ObjectStore, and job records use tables; configuration and confirmation use task panes. No sibling path or unpublished package may be required by the public build. **Status: design reference pinned; UI not implemented.** |
| Future Synoptikon product, host, and catalogue adapter | `../mnemosyne` `9877017e3139711ed6313c53603409c53020541d` (SDK/API types subject to this moving source snapshot) | `mneion-api-types/schemas/CATALOGUE_FORMAT.md`; `mneion-api-types/schemas/HOST_PRODUCT_UI_ADAPTER.md`; `mneion-api-types/schemas/HOST_STORAGE_BOUNDARY.md`; `mneion-api-types/schemas/MANIFEST_COMPATIBILITY.md`; `mnemosyne-product-sdk/Cargo.toml`; `mnemosyne-product-sdk/src/{manifest.rs,product_host.rs,product_api_routes.rs,product_ui.rs,endpoint.rs,conformance.rs}`; `mneion-api-types/src/catalogue.rs` | `mnemosyne.product.manifest.v1`; `mnemosyne.catalogue.v1`; `mnemosyne.product_ui.bootstrap.v1`; `mnemosyne.product.health.v1`; `mnemosyne.product.info.v1`; host modes `synoptikon_integrated` and `monas_standalone`; x-img `fixtures/host-context/v1/synoptikon-valid.json`; valid fixtures `mnematikon-dual-host.json`, `phoreus-dual-host.json`, `phoreus-monas-standalone.json`, `hermeneia-dual-host.json`; negative fixtures `invalid-product-id.json`, `missing-support.json` | XIMG-031 proves a future Synoptikon adapter can replace Monas at the authenticated host-context boundary. Synoptikon remains the authority for tenant/project context, audit, RDBMS state, and durable artefact registration. x-img must not depend on the current `publish = false` SDK or its unpublished path dependency; pin schemas/fixtures until a public package exists. **Status: host-context fixture adapter complete; live Synoptikon integration is not authorized.** |

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
   XIMG-032 through XIMG-034 use x-img-owned versioned fixtures and are covered
   by XIMG-035's pinned source-contract inspection. Live transport integration
   remains gated until those public route contracts are versioned and testable.
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
