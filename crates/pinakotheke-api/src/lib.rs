// SPDX-License-Identifier: MPL-2.0
//! Axum composition boundary for a future host-managed API service.
//!
//! A host must validate its session before injecting an authenticated context.
//! This crate never parses cookies, passwords, or session tokens.

use std::{
    io,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, Request, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;
use x_img_core::{
    cache_alias::{
        CacheBypassReason, CacheLookupOutcome, CacheLookupRequest, CacheLookupService,
        CacheRepresentation,
    },
    gallery_catalogue::{
        GalleryCatalogue, GalleryCatalogueError, GalleryCatalogueFilter, GalleryImageResolveError,
        GalleryImageRole, GalleryMediaKind, GalleryObjectAvailability, GalleryPage,
        GalleryReviewState, GallerySourceKind,
    },
    host_context::{
        AuthenticatedHostContext, HostContextAdapter, MonasHostContextAdapter, XIMG_ACCESS,
    },
    object_read::{
        AuthorizedObjectReader, ObjectReadBackend, ObjectReadBackendError, ObjectReadRequest,
        ObjectReadResult, ValidatedObjectRead,
    },
    operations::{OperationalTelemetry, OperationsSnapshot},
    playback_delivery::{
        DirectPlaybackError, DirectPlaybackResponse, DirectPlaybackService, parse_single_range,
    },
    synoptikon_catalogue::{
        SynoptikonCatalogueError, SynoptikonCataloguePage, SynoptikonCatalogueProjection,
    },
    viewed_media::{CapturePlan, CapturePlanError, CapturePlanRequest, CapturePlanService},
};

type CapturePlans = Arc<Mutex<CapturePlanService>>;
type PlaybackDelivery = Arc<Mutex<DirectPlaybackService<HostObjectReadBackend>>>;
type CacheAliases = Arc<Mutex<CacheLookupService>>;
type ImageDelivery = Arc<Mutex<AuthorizedObjectReader<HostObjectReadBackend>>>;
type Operations = Arc<Mutex<OperationalTelemetry>>;
type SynoptikonCatalogue = Arc<SynoptikonCatalogueProjection>;
type MonasGalleryCatalogue = Arc<GalleryCatalogue>;

const CACHE_LOOKUP_SCHEMA_VERSION: &str = "x-img.cache-alias-lookup.v1";
const CACHE_RESULT_SCHEMA_VERSION: &str = "x-img.cache-alias-result.v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheAliasLookupEnvelope {
    schema_version: String,
    pairing_id: String,
    instance_id: String,
    origin: String,
    canonical_alias: String,
    adapter_id: String,
    adapter_version: String,
}

#[derive(Debug, Serialize)]
struct CacheAliasLookupResponse {
    schema_version: &'static str,
    outcome: &'static str,
    reason: Option<&'static str>,
    media_class: Option<&'static str>,
    content_type: Option<String>,
    content_length: Option<u64>,
    object_checksum: Option<String>,
    delivery_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct PublicHealthResponse {
    schema_version: &'static str,
    status: &'static str,
    product: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize)]
struct MonolithReadinessResponse {
    schema_version: &'static str,
    status: &'static str,
    root: &'static str,
    components: [MonolithComponentReadiness; 3],
}

#[derive(Debug, Serialize)]
struct MonolithComponentReadiness {
    component: &'static str,
    status: &'static str,
    detail: &'static str,
}

/// Process-local credential used only to authenticate Monas-to-product dispatch.
/// It is never a browser session and must be supplied from private runtime state.
#[derive(Clone)]
pub struct MonasDispatchVerifier {
    token: Arc<str>,
}

impl MonasDispatchVerifier {
    /// Creates a verifier for a high-entropy token supplied independently to Monas.
    pub fn new(token: String) -> io::Result<Self> {
        if !(32..=512).contains(&token.len()) || token.chars().any(char::is_whitespace) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Monas dispatch token must be 32-512 non-whitespace characters",
            ));
        }
        Ok(Self {
            token: token.into(),
        })
    }
}

#[derive(Debug, Serialize)]
struct OperationsResponse {
    schema_version: &'static str,
    snapshot: OperationsSnapshot,
}

/// Server-side callback used to bridge a host's scoped DASObjectStore read
/// client to Axum. The callback returns a body stream and never exposes a
/// filesystem location or browser credential to x-img.
pub type HostObjectOpen = Box<
    dyn FnMut(&ObjectReadRequest) -> Result<ObjectReadResult<Body>, ObjectReadBackendError> + Send,
>;

/// Concrete host adapter for direct playback routes.
///
/// The surrounding host is responsible for authenticated DASObjectStore
/// transport and TLS. x-img validates the returned object metadata and makes
/// the stream available only after its injected Monas context is authorized.
pub struct HostObjectReadBackend {
    open: HostObjectOpen,
}

/// Serves the initial local monolith until interrupted.
pub async fn serve(listener: tokio::net::TcpListener) -> io::Result<()> {
    serve_monolith(listener, false, None).await
}

/// Serves the monolith with the caller's verified local storage readiness.
pub async fn serve_monolith(
    listener: tokio::net::TcpListener,
    dasobjectstore_ready: bool,
    monas_dispatch: Option<MonasDispatchVerifier>,
) -> io::Result<()> {
    serve_monolith_with_gallery(
        listener,
        dasobjectstore_ready,
        monas_dispatch,
        GalleryCatalogue::default(),
    )
    .await
}

/// Serves the monolith with a validated persistent gallery projection.
pub async fn serve_monolith_with_gallery(
    listener: tokio::net::TcpListener,
    dasobjectstore_ready: bool,
    monas_dispatch: Option<MonasDispatchVerifier>,
    gallery: GalleryCatalogue,
) -> io::Result<()> {
    serve_monolith_with_gallery_and_web(
        listener,
        dasobjectstore_ready,
        monas_dispatch,
        gallery,
        None,
    )
    .await
}

/// Serves the monolith with persistent gallery metadata and reviewed web assets.
pub async fn serve_monolith_with_gallery_and_web(
    listener: tokio::net::TcpListener,
    dasobjectstore_ready: bool,
    monas_dispatch: Option<MonasDispatchVerifier>,
    gallery: GalleryCatalogue,
    web_root: Option<PathBuf>,
) -> io::Result<()> {
    axum::serve(
        listener,
        monolith_router_with_gallery_and_web_authority(
            dasobjectstore_ready,
            monas_dispatch,
            gallery,
            web_root,
        ),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
}

/// Serves the complete authenticated gallery with a host-owned object stream.
pub async fn serve_monolith_with_gallery_web_and_delivery(
    listener: tokio::net::TcpListener,
    dasobjectstore_ready: bool,
    monas_dispatch: Option<MonasDispatchVerifier>,
    gallery: GalleryCatalogue,
    web_root: Option<PathBuf>,
    backend: HostObjectReadBackend,
) -> io::Result<()> {
    axum::serve(
        listener,
        monolith_router_with_gallery_web_delivery_authority(
            dasobjectstore_ready,
            monas_dispatch,
            gallery,
            web_root,
            backend,
        ),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        if let Ok(mut terminate) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = terminate.recv() => {},
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

impl HostObjectReadBackend {
    /// Creates the adapter from a scoped, server-side DASObjectStore opener.
    pub fn new(open: HostObjectOpen) -> Self {
        Self { open }
    }
}

impl ObjectReadBackend for HostObjectReadBackend {
    type Stream = Body;

    fn open(
        &mut self,
        request: &ObjectReadRequest,
    ) -> Result<ObjectReadResult<Self::Stream>, ObjectReadBackendError> {
        (self.open)(request)
    }
}

/// Returns the product router. Health is public; every product API route needs
/// a host-injected, authorized context.
pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/context", get(context))
        .route("/api/extension/v1/capture-plans", post(capture_plan))
        .route(
            "/api/extension/v1/cache-aliases/lookup",
            post(cache_alias_lookup),
        )
        .route("/api/playback/v1/{playback_id}", get(deliver_playback))
        .with_state(None::<CapturePlans>)
}

/// Returns the initial locally runnable monolith surface.
///
/// This first slice deliberately exposes only public orientation, liveness,
/// and honest dependency readiness. Authenticated product APIs are added only
/// when Monas can inject a verified host context.
pub fn monolith_router() -> Router {
    monolith_router_with_storage(false)
}

/// Returns the local surface with a previously verified DASObjectStore state.
pub fn monolith_router_with_storage(dasobjectstore_ready: bool) -> Router {
    monolith_router_with_authorities(dasobjectstore_ready, None)
}

/// Returns a monolith surface with optional trusted Monas dispatch admission.
pub fn monolith_router_with_authorities(
    dasobjectstore_ready: bool,
    monas_dispatch: Option<MonasDispatchVerifier>,
) -> Router {
    monolith_router_with_gallery_authority(
        dasobjectstore_ready,
        monas_dispatch,
        GalleryCatalogue::default(),
    )
}

/// Returns the Monas-admitted monolith with a host-supplied gallery projection.
pub fn monolith_router_with_gallery_authority(
    dasobjectstore_ready: bool,
    monas_dispatch: Option<MonasDispatchVerifier>,
    gallery: GalleryCatalogue,
) -> Router {
    monolith_router_with_gallery_and_web_authority(
        dasobjectstore_ready,
        monas_dispatch,
        gallery,
        None,
    )
}

/// Returns the Monas-admitted gallery plus an optional built Yew application.
pub fn monolith_router_with_gallery_and_web_authority(
    dasobjectstore_ready: bool,
    monas_dispatch: Option<MonasDispatchVerifier>,
    gallery: GalleryCatalogue,
    web_root: Option<PathBuf>,
) -> Router {
    let authentication_ready = monas_dispatch.is_some();
    let mut protected = Router::new()
        .route("/products/pinakotheke/api/context", get(context))
        .route(
            "/products/pinakotheke/api/gallery/v1/catalogue",
            get(gallery_catalogue),
        )
        .layer(Extension(Arc::new(gallery)));
    if let Some(web_root) = web_root {
        protected = protected.nest_service(
            "/products/pinakotheke/app",
            ServeDir::new(web_root).append_index_html_on_directories(true),
        );
    }
    let protected = protected.layer(middleware::from_fn_with_state(
        monas_dispatch,
        admit_monas_dispatch,
    ));
    Router::new()
        .route("/", get(monolith_landing))
        .route("/health", get(health))
        .route(
            "/ready",
            get(move || async move {
                monolith_readiness(dasobjectstore_ready, authentication_ready).await
            }),
        )
        .merge(protected)
}

/// Returns the Monas-admitted monolith with exact gallery image streaming.
pub fn monolith_router_with_gallery_delivery_authority(
    dasobjectstore_ready: bool,
    monas_dispatch: Option<MonasDispatchVerifier>,
    gallery: GalleryCatalogue,
    backend: HostObjectReadBackend,
) -> Router {
    monolith_router_with_gallery_web_delivery_authority(
        dasobjectstore_ready,
        monas_dispatch,
        gallery,
        None,
        backend,
    )
}

/// Returns the Monas-admitted web gallery with exact object streaming.
pub fn monolith_router_with_gallery_web_delivery_authority(
    dasobjectstore_ready: bool,
    monas_dispatch: Option<MonasDispatchVerifier>,
    gallery: GalleryCatalogue,
    web_root: Option<PathBuf>,
    backend: HostObjectReadBackend,
) -> Router {
    let authentication_ready = monas_dispatch.is_some();
    let mut protected = Router::new()
        .route("/products/pinakotheke/api/context", get(context))
        .route(
            "/products/pinakotheke/api/gallery/v1/catalogue",
            get(gallery_catalogue),
        )
        .route(
            "/products/pinakotheke/api/gallery/v1/objects/{catalogue_id}/{role}",
            get(deliver_gallery_image),
        )
        .route(
            "/products/pinakotheke/api/gallery/v1/objects/{catalogue_id}/video",
            get(deliver_gallery_video),
        )
        .layer(Extension(Arc::new(gallery)))
        .layer(Extension(Arc::new(Mutex::new(
            AuthorizedObjectReader::new(backend),
        ))));
    if let Some(web_root) = web_root {
        protected = protected.nest_service(
            "/products/pinakotheke/app",
            ServeDir::new(web_root).append_index_html_on_directories(true),
        );
    }
    let protected = protected.layer(middleware::from_fn_with_state(
        monas_dispatch,
        admit_monas_dispatch,
    ));
    Router::new()
        .route("/", get(monolith_landing))
        .route("/health", get(health))
        .route(
            "/ready",
            get(move || async move {
                monolith_readiness(dasobjectstore_ready, authentication_ready).await
            }),
        )
        .merge(protected)
}

async fn admit_monas_dispatch(
    State(verifier): State<Option<MonasDispatchVerifier>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let Some(verifier) = verifier else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let supplied = request
        .headers_mut()
        .remove("x-monas-dispatch-token")
        .and_then(|value| value.to_str().ok().map(ToOwned::to_owned));
    let context_json = request
        .headers_mut()
        .remove("x-monas-host-context")
        .and_then(|value| value.to_str().ok().map(ToOwned::to_owned));
    if !supplied
        .as_deref()
        .is_some_and(|token| constant_time_equal(token.as_bytes(), verifier.token.as_bytes()))
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Some(context_json) = context_json else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(context) = MonasHostContextAdapter.authenticate(context_json.as_bytes()) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    request.extensions_mut().insert(context);
    next.run(request).await
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

async fn monolith_landing() -> Html<&'static str> {
    Html(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>Pinakotheke</title><main><h1>Pinakotheke</h1><p>Local service is running.</p><p><a href=\"/ready\">Readiness</a></p></main></html>",
    )
}

async fn monolith_readiness(
    dasobjectstore_ready: bool,
    authentication_ready: bool,
) -> Json<MonolithReadinessResponse> {
    Json(MonolithReadinessResponse {
        schema_version: "pinakotheke.monolith-readiness.v1",
        status: "not_ready",
        root: "Ready",
        components: [
            MonolithComponentReadiness {
                component: "pinakotheke",
                status: "Ready",
                detail: "Axum listener and local metadata root are available",
            },
            MonolithComponentReadiness {
                component: "monas_authentication",
                status: if authentication_ready {
                    "Ready"
                } else {
                    "Not configured"
                },
                detail: if authentication_ready {
                    "Trusted Monas host dispatch is configured"
                } else {
                    "Authenticated product routes are unavailable"
                },
            },
            MonolithComponentReadiness {
                component: "dasobjectstore",
                status: if dasobjectstore_ready {
                    "Ready"
                } else {
                    "Not configured"
                },
                detail: if dasobjectstore_ready {
                    "Managed endpoint and ObjectStore identity are selected"
                } else {
                    "Media ingest and object reads are unavailable"
                },
            },
        ],
    })
}

/// Returns a host composition with a configured, server-side capture policy.
///
/// The browser still needs an injected, authorized Monas/Synoptikon host
/// context.  An unconfigured router deliberately refuses capture requests.
pub fn router_with_capture_plans(capture_plans: CapturePlanService) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/context", get(context))
        .route("/api/extension/v1/capture-plans", post(capture_plan))
        .route(
            "/api/extension/v1/cache-aliases/lookup",
            post(cache_alias_lookup),
        )
        .route("/api/playback/v1/{playback_id}", get(deliver_playback))
        .with_state(Some(Arc::new(Mutex::new(capture_plans))))
}

/// Returns a host composition with authenticated redacted operational details.
/// Public health remains a coarse process-liveness response.
pub fn router_with_operations(operations: Arc<Mutex<OperationalTelemetry>>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/context", get(context))
        .route("/api/operations/v1/snapshot", get(operations_snapshot))
        .with_state(None::<CapturePlans>)
        .layer(Extension(operations))
}

/// Returns the Synoptikon-integrated, project-scoped catalogue projection.
///
/// The host must inject a verified Synoptikon context containing tenant,
/// account, project, entitlement, and catalogue-read authorization. This
/// endpoint exposes metadata and immutable DASObjectStore references only.
pub fn router_with_synoptikon_catalogue(catalogue: SynoptikonCatalogueProjection) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/synoptikon/v1/catalogue", get(synoptikon_catalogue))
        .layer(Extension(Arc::new(catalogue)))
}

/// Returns a bounded Monas-hosted gallery catalogue.
///
/// Items contain verified DASObjectStore references and host-local authorized
/// delivery paths only. There is deliberately no source URL or origin fallback.
pub fn router_with_gallery_catalogue(catalogue: GalleryCatalogue) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/gallery/v1/catalogue", get(gallery_catalogue))
        .route(
            "/products/pinakotheke/api/gallery/v1/catalogue",
            get(gallery_catalogue),
        )
        .layer(Extension(Arc::new(catalogue)))
}

/// Returns an authenticated gallery with exact DASObjectStore image streaming.
pub fn router_with_gallery_delivery(
    catalogue: GalleryCatalogue,
    backend: HostObjectReadBackend,
) -> Router {
    Router::new()
        .route("/health", get(health))
        .route(
            "/products/pinakotheke/api/gallery/v1/catalogue",
            get(gallery_catalogue),
        )
        .route(
            "/products/pinakotheke/api/gallery/v1/objects/{catalogue_id}/{role}",
            get(deliver_gallery_image),
        )
        .route(
            "/products/pinakotheke/api/gallery/v1/objects/{catalogue_id}/video",
            get(deliver_gallery_video),
        )
        .layer(Extension(Arc::new(catalogue)))
        .layer(Extension(Arc::new(Mutex::new(
            AuthorizedObjectReader::new(backend),
        ))))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CataloguePageQuery {
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_catalogue_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GalleryCatalogueQuery {
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_catalogue_limit")]
    limit: usize,
    source_kind: Option<GallerySourceKind>,
    media_kind: Option<GalleryMediaKind>,
    review_state: Option<GalleryReviewState>,
    availability: Option<GalleryObjectAvailability>,
    discovered_from_epoch_seconds: Option<u64>,
    discovered_to_epoch_seconds: Option<u64>,
    text: Option<String>,
}

const fn default_catalogue_limit() -> usize {
    100
}

async fn synoptikon_catalogue(
    context: Option<Extension<AuthenticatedHostContext>>,
    catalogue: Option<Extension<SynoptikonCatalogue>>,
    Query(query): Query<CataloguePageQuery>,
) -> Result<Json<SynoptikonCataloguePage>, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    let catalogue = catalogue.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    catalogue
        .page(&context, query.offset, query.limit)
        .map(Json)
        .map_err(|error| match error {
            SynoptikonCatalogueError::Unauthorized => StatusCode::FORBIDDEN,
            SynoptikonCatalogueError::InvalidScope => StatusCode::UNAUTHORIZED,
            SynoptikonCatalogueError::InvalidPageSize => StatusCode::BAD_REQUEST,
        })
}

async fn gallery_catalogue(
    context: Option<Extension<AuthenticatedHostContext>>,
    catalogue: Option<Extension<MonasGalleryCatalogue>>,
    Query(query): Query<GalleryCatalogueQuery>,
) -> Result<Json<GalleryPage>, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    let catalogue = catalogue.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    catalogue
        .filtered_page(
            &context,
            query.offset,
            query.limit,
            &GalleryCatalogueFilter {
                source_kind: query.source_kind,
                media_kind: query.media_kind,
                review_state: query.review_state,
                availability: query.availability,
                discovered_from_epoch_seconds: query.discovered_from_epoch_seconds,
                discovered_to_epoch_seconds: query.discovered_to_epoch_seconds,
                text: query.text,
            },
        )
        .map(Json)
        .map_err(|error| match error {
            GalleryCatalogueError::Unauthorized => StatusCode::FORBIDDEN,
            GalleryCatalogueError::InvalidPageSize => StatusCode::BAD_REQUEST,
            GalleryCatalogueError::InvalidFilter => StatusCode::BAD_REQUEST,
            GalleryCatalogueError::InvalidItem(_) => StatusCode::INTERNAL_SERVER_ERROR,
        })
}

async fn deliver_gallery_image(
    Path((catalogue_id, role)): Path<(String, String)>,
    headers: HeaderMap,
    context: Option<Extension<AuthenticatedHostContext>>,
    catalogue: Option<Extension<MonasGalleryCatalogue>>,
    delivery: Option<Extension<ImageDelivery>>,
) -> Result<Response, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    let role = match role.as_str() {
        "thumbnail" => GalleryImageRole::Thumbnail,
        "original" => GalleryImageRole::Original,
        _ => return Err(StatusCode::NOT_FOUND),
    };
    let grant = catalogue
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .0
        .resolve_image(&context, &catalogue_id, role)
        .map_err(|error| match error {
            GalleryImageResolveError::Unauthorized => StatusCode::FORBIDDEN,
            GalleryImageResolveError::NotFound | GalleryImageResolveError::NotAnImage => {
                StatusCode::NOT_FOUND
            }
            GalleryImageResolveError::Unavailable => StatusCode::GONE,
        })?;
    let request = ObjectReadRequest {
        object: grant.object,
        range: None,
        if_none_match_etag: headers
            .get(header::IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
    };
    let delivery = delivery.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let mut delivery = delivery
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match delivery
        .open(&request)
        .map_err(|_| StatusCode::BAD_GATEWAY)?
    {
        ValidatedObjectRead::NotModified { etag } => Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, etag)
            .header(header::CACHE_CONTROL, "private, no-store")
            .header("cross-origin-resource-policy", "same-origin")
            .header("x-content-type-options", "nosniff")
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
        ValidatedObjectRead::Content { metadata, stream } => {
            if metadata.content_type != grant.content_type
                || metadata.content_length != grant.content_length
                || metadata.total_length != grant.content_length
            {
                return Err(StatusCode::BAD_GATEWAY);
            }
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, metadata.content_type)
                .header(header::CONTENT_LENGTH, metadata.content_length)
                .header(header::ETAG, metadata.etag)
                .header(header::CACHE_CONTROL, "private, no-store")
                .header("cross-origin-resource-policy", "same-origin")
                .header("x-content-type-options", "nosniff")
                .body(stream)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn deliver_gallery_video(
    Path(catalogue_id): Path<String>,
    headers: HeaderMap,
    context: Option<Extension<AuthenticatedHostContext>>,
    catalogue: Option<Extension<MonasGalleryCatalogue>>,
    delivery: Option<Extension<ImageDelivery>>,
) -> Result<Response, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    let grant = catalogue
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .0
        .resolve_video(&context, &catalogue_id)
        .map_err(|error| match error {
            GalleryImageResolveError::Unauthorized => StatusCode::FORBIDDEN,
            GalleryImageResolveError::NotFound | GalleryImageResolveError::NotAnImage => {
                StatusCode::NOT_FOUND
            }
            GalleryImageResolveError::Unavailable => StatusCode::GONE,
        })?;
    let range = match headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
    {
        Some(value) => match parse_single_range(value, grant.content_length) {
            Ok(range) => Some(range),
            Err(_) => {
                return Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(
                        header::CONTENT_RANGE,
                        format!("bytes */{}", grant.content_length),
                    )
                    .header(header::ACCEPT_RANGES, "bytes")
                    .header(header::CACHE_CONTROL, "private, no-store")
                    .body(Body::empty())
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
            }
        },
        None => None,
    };
    let request = ObjectReadRequest {
        object: grant.object,
        range,
        if_none_match_etag: headers
            .get(header::IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
    };
    let delivery = delivery.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let result = {
        let mut delivery = delivery
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        delivery
            .open(&request)
            .map_err(|_| StatusCode::BAD_GATEWAY)?
    };
    match result {
        ValidatedObjectRead::NotModified { etag } => Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, etag)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CACHE_CONTROL, "private, no-store")
            .header("cross-origin-resource-policy", "same-origin")
            .header("x-content-type-options", "nosniff")
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
        ValidatedObjectRead::Content { metadata, stream } => {
            if metadata.content_type != grant.content_type
                || metadata.total_length != grant.content_length
                || metadata.content_range != range
            {
                return Err(StatusCode::BAD_GATEWAY);
            }
            let mut response = Response::builder()
                .status(if range.is_some() {
                    StatusCode::PARTIAL_CONTENT
                } else {
                    StatusCode::OK
                })
                .header(header::CONTENT_TYPE, metadata.content_type)
                .header(header::CONTENT_LENGTH, metadata.content_length)
                .header(header::ETAG, metadata.etag)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CACHE_CONTROL, "private, no-store")
                .header("cross-origin-resource-policy", "same-origin")
                .header("x-content-type-options", "nosniff");
            if let Some(range) = metadata.content_range {
                response = response.header(
                    header::CONTENT_RANGE,
                    format!(
                        "bytes {}-{}/{}",
                        range.start, range.end_inclusive, metadata.total_length
                    ),
                );
            }
            response
                .body(stream)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Returns a host composition with a direct, authorized normalized-video
/// delivery service. This route is intentionally distinct from Firefox site
/// cache substitution: it has no source URL or origin fallback.
pub fn router_with_direct_playback(
    playback: DirectPlaybackService<HostObjectReadBackend>,
) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/context", get(context))
        .route("/api/extension/v1/capture-plans", post(capture_plan))
        .route(
            "/api/extension/v1/cache-aliases/lookup",
            post(cache_alias_lookup),
        )
        .route("/api/playback/v1/{playback_id}", get(deliver_playback))
        .with_state(None::<CapturePlans>)
        .layer(Extension(Arc::new(Mutex::new(playback))))
}

/// Returns a host composition with a bounded, server-authorized cache-alias
/// lookup. A miss or policy/authority failure is an explicit origin fallback.
pub fn router_with_cache_aliases(cache_aliases: CacheLookupService) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/context", get(context))
        .route("/api/extension/v1/capture-plans", post(capture_plan))
        .route(
            "/api/extension/v1/cache-aliases/lookup",
            post(cache_alias_lookup),
        )
        .route("/api/playback/v1/{playback_id}", get(deliver_playback))
        .with_state(None::<CapturePlans>)
        .layer(Extension(Arc::new(Mutex::new(cache_aliases))))
}

/// Returns a host composition capable of lookup and exact, authenticated image
/// delivery. Payloads stream from DASObjectStore and are never persisted by
/// x-img; every delivery repeats pairing and object metadata validation.
pub fn router_with_image_substitution(
    cache_aliases: CacheLookupService,
    backend: HostObjectReadBackend,
) -> Router {
    router_with_cache_substitution(cache_aliases, backend)
}

/// Returns the external-cache composition for progressive images and verified
/// normalized MP4 renditions. Segmented media remains outside this router.
pub fn router_with_cache_substitution(
    cache_aliases: CacheLookupService,
    backend: HostObjectReadBackend,
) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/context", get(context))
        .route(
            "/api/extension/v1/cache-aliases/lookup",
            post(cache_alias_lookup),
        )
        .route(
            "/api/cache/v1/images/{pairing_id}/{delivery_id}",
            get(deliver_cached_image),
        )
        .route(
            "/api/cache/v1/videos/{pairing_id}/{delivery_id}",
            get(deliver_cached_video),
        )
        .with_state(None::<CapturePlans>)
        .layer(Extension(Arc::new(Mutex::new(cache_aliases))))
        .layer(Extension(Arc::new(Mutex::new(
            AuthorizedObjectReader::new(backend),
        ))))
}

async fn health() -> Json<PublicHealthResponse> {
    Json(PublicHealthResponse {
        schema_version: "x-img.public-health.v1",
        status: "alive",
        product: x_img_core::build_info().product.name,
        version: x_img_core::build_info().product.version,
    })
}

async fn operations_snapshot(
    context: Option<Extension<AuthenticatedHostContext>>,
    operations: Option<Extension<Operations>>,
) -> Result<Json<OperationsResponse>, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    if !context.permits(XIMG_ACCESS) {
        return Err(StatusCode::FORBIDDEN);
    }
    let operations = operations.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let snapshot = operations
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .snapshot();
    Ok(Json(OperationsResponse {
        schema_version: "x-img.operations-snapshot.v1",
        snapshot,
    }))
}

async fn context(
    context: Option<Extension<AuthenticatedHostContext>>,
) -> Result<StatusCode, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    if !context.permits(XIMG_ACCESS) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn capture_plan(
    State(capture_plans): State<Option<CapturePlans>>,
    context: Option<Extension<AuthenticatedHostContext>>,
    Json(request): Json<CapturePlanRequest>,
) -> Result<Json<CapturePlan>, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    if !context.permits(XIMG_ACCESS) {
        return Err(StatusCode::FORBIDDEN);
    }
    let capture_plans = capture_plans.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_secs();
    let mut capture_plans = capture_plans
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    capture_plans
        .plan(context.actor_id(), now, request)
        .map(Json)
        .map_err(capture_plan_status)
}

fn capture_plan_status(error: CapturePlanError) -> StatusCode {
    match error {
        CapturePlanError::PairingActorMismatch
        | CapturePlanError::UnknownPairing
        | CapturePlanError::PairingExpired
        | CapturePlanError::PairingRevoked => StatusCode::FORBIDDEN,
        CapturePlanError::Scheduler => StatusCode::SERVICE_UNAVAILABLE,
        CapturePlanError::InvalidRequest
        | CapturePlanError::SiteNotEnabled
        | CapturePlanError::AdapterMismatch
        | CapturePlanError::CaptureNotEligible
        | CapturePlanError::CandidateBudgetExceeded => StatusCode::UNPROCESSABLE_ENTITY,
    }
}

async fn cache_alias_lookup(
    context: Option<Extension<AuthenticatedHostContext>>,
    cache_aliases: Option<Extension<CacheAliases>>,
    Json(envelope): Json<CacheAliasLookupEnvelope>,
) -> Result<Json<CacheAliasLookupResponse>, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    if !context.permits(XIMG_ACCESS) {
        return Err(StatusCode::FORBIDDEN);
    }
    if envelope.schema_version != CACHE_LOOKUP_SCHEMA_VERSION {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let now_epoch_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_secs();
    let request = CacheLookupRequest {
        pairing_id: envelope.pairing_id,
        instance_id: envelope.instance_id,
        site_origin: envelope.origin,
        canonical_alias: envelope.canonical_alias,
        adapter_id: envelope.adapter_id,
        adapter_version: envelope.adapter_version,
        now_epoch_seconds,
    };
    let cache_aliases = cache_aliases.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let cache_aliases = cache_aliases
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        match cache_aliases.lookup(context.actor_id(), &request) {
            CacheLookupOutcome::Hit(hit) => CacheAliasLookupResponse {
                schema_version: CACHE_RESULT_SCHEMA_VERSION,
                outcome: "hit",
                reason: None,
                media_class: Some(match hit.representation {
                    CacheRepresentation::ThumbnailImage => "thumbnail_image",
                    CacheRepresentation::OriginalImage => "original_image",
                    CacheRepresentation::NormalizedMp4 => "normalized_mp4",
                }),
                content_type: Some(hit.content_type.clone()),
                content_length: Some(hit.content_length),
                object_checksum: Some(hit.object.checksum.clone()),
                delivery_path: Some(format!(
                    "/api/cache/v1/{}/{}/{}",
                    if hit.representation == CacheRepresentation::NormalizedMp4 {
                        "videos"
                    } else {
                        "images"
                    },
                    request.pairing_id,
                    hit.delivery_id
                )),
            },
            CacheLookupOutcome::Miss => cache_fallback("miss", None),
            CacheLookupOutcome::OriginFallback(reason) => {
                cache_fallback("origin_fallback", Some(cache_bypass_reason(reason)))
            }
        },
    ))
}

fn cache_fallback(outcome: &'static str, reason: Option<&'static str>) -> CacheAliasLookupResponse {
    CacheAliasLookupResponse {
        schema_version: CACHE_RESULT_SCHEMA_VERSION,
        outcome,
        reason,
        media_class: None,
        content_type: None,
        content_length: None,
        object_checksum: None,
        delivery_path: None,
    }
}

fn cache_bypass_reason(reason: CacheBypassReason) -> &'static str {
    match reason {
        CacheBypassReason::InvalidRequest => "invalid_request",
        CacheBypassReason::SubstitutionPaused => "substitution_paused",
        CacheBypassReason::PairingInvalid => "pairing_invalid",
        CacheBypassReason::WrongInstance => "wrong_instance",
        CacheBypassReason::AdapterMismatch => "adapter_mismatch",
        CacheBypassReason::Stale => "stale",
        CacheBypassReason::EndpointOffline => "endpoint_offline",
        CacheBypassReason::ObjectUnavailable => "object_unavailable",
        CacheBypassReason::NotAnImage => "not_an_image",
        CacheBypassReason::NotNormalizedMp4 => "not_normalized_mp4",
    }
}

async fn deliver_cached_video(
    Path((pairing_id, delivery_id)): Path<(String, String)>,
    headers: HeaderMap,
    context: Option<Extension<AuthenticatedHostContext>>,
    cache_aliases: Option<Extension<CacheAliases>>,
    delivery: Option<Extension<ImageDelivery>>,
) -> Result<Response, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    if !context.permits(XIMG_ACCESS) {
        return Err(StatusCode::FORBIDDEN);
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_secs();
    let record = {
        let service = cache_aliases.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
        let service = service
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        match service.authorize_video_delivery(context.actor_id(), &pairing_id, &delivery_id, now) {
            CacheLookupOutcome::Hit(record) => record.clone(),
            CacheLookupOutcome::Miss => return Err(StatusCode::NOT_FOUND),
            CacheLookupOutcome::OriginFallback(_) => return Err(StatusCode::FORBIDDEN),
        }
    };
    let range = match headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
    {
        Some(value) => match parse_single_range(value, record.content_length) {
            Ok(range) => Some(range),
            Err(_) => {
                return Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(
                        header::CONTENT_RANGE,
                        format!("bytes */{}", record.content_length),
                    )
                    .header(header::ACCEPT_RANGES, "bytes")
                    .header(header::CACHE_CONTROL, "private, no-store")
                    .header("access-control-allow-origin", &record.site_origin)
                    .header("access-control-allow-credentials", "true")
                    .header("cross-origin-resource-policy", "cross-origin")
                    .header(header::VARY, "origin")
                    .body(Body::empty())
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
            }
        },
        None => None,
    };
    let if_none_match = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let request = ObjectReadRequest {
        object: record.object,
        range,
        if_none_match_etag: if_none_match,
    };
    let delivery = delivery.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let result = {
        let mut reader = delivery
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        reader.open(&request).map_err(|_| StatusCode::BAD_GATEWAY)?
    };
    match result {
        ValidatedObjectRead::NotModified { etag } => Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, etag)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CACHE_CONTROL, "private, no-store")
            .header("access-control-allow-origin", &record.site_origin)
            .header("access-control-allow-credentials", "true")
            .header(
                "access-control-expose-headers",
                "etag, content-length, accept-ranges, content-range",
            )
            .header("cross-origin-resource-policy", "cross-origin")
            .header("x-content-type-options", "nosniff")
            .header(header::VARY, "origin")
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
        ValidatedObjectRead::Content { metadata, stream } => {
            if metadata.content_type != "video/mp4"
                || metadata.total_length != record.content_length
                || metadata.content_range != range
            {
                return Err(StatusCode::BAD_GATEWAY);
            }
            let mut response = Response::builder()
                .status(if range.is_some() {
                    StatusCode::PARTIAL_CONTENT
                } else {
                    StatusCode::OK
                })
                .header(header::CONTENT_TYPE, metadata.content_type)
                .header(header::CONTENT_LENGTH, metadata.content_length)
                .header(header::ETAG, metadata.etag)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CACHE_CONTROL, "private, no-store")
                .header("access-control-allow-origin", &record.site_origin)
                .header("access-control-allow-credentials", "true")
                .header(
                    "access-control-expose-headers",
                    "etag, content-length, accept-ranges, content-range",
                )
                .header("cross-origin-resource-policy", "cross-origin")
                .header("x-content-type-options", "nosniff")
                .header(header::VARY, "origin");
            if let Some(range) = metadata.content_range {
                response = response.header(
                    header::CONTENT_RANGE,
                    format!(
                        "bytes {}-{}/{}",
                        range.start, range.end_inclusive, metadata.total_length
                    ),
                );
            }
            response
                .body(stream)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn deliver_cached_image(
    Path((pairing_id, delivery_id)): Path<(String, String)>,
    headers: HeaderMap,
    context: Option<Extension<AuthenticatedHostContext>>,
    cache_aliases: Option<Extension<CacheAliases>>,
    image_delivery: Option<Extension<ImageDelivery>>,
) -> Result<Response, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    if !context.permits(XIMG_ACCESS) {
        return Err(StatusCode::FORBIDDEN);
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_secs();
    let record = {
        let cache_aliases = cache_aliases.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
        let cache_aliases = cache_aliases
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        match cache_aliases.authorize_image_delivery(
            context.actor_id(),
            &pairing_id,
            &delivery_id,
            now,
        ) {
            CacheLookupOutcome::Hit(record) => record.clone(),
            CacheLookupOutcome::Miss => return Err(StatusCode::NOT_FOUND),
            CacheLookupOutcome::OriginFallback(_) => return Err(StatusCode::FORBIDDEN),
        }
    };
    let if_none_match = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let request = ObjectReadRequest {
        object: record.object,
        range: None,
        if_none_match_etag: if_none_match,
    };
    let image_delivery = image_delivery.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let mut image_delivery = image_delivery
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match image_delivery
        .open(&request)
        .map_err(|_| StatusCode::BAD_GATEWAY)?
    {
        ValidatedObjectRead::NotModified { etag } => Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, etag)
            .header(header::CACHE_CONTROL, "private, no-store")
            .header("access-control-allow-origin", &record.site_origin)
            .header("access-control-allow-credentials", "true")
            .header("access-control-expose-headers", "etag, content-length")
            .header("cross-origin-resource-policy", "cross-origin")
            .header("x-content-type-options", "nosniff")
            .header(header::VARY, "origin")
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
        ValidatedObjectRead::Content { metadata, stream } => {
            if metadata.content_type != record.content_type
                || metadata.content_length != record.content_length
            {
                return Err(StatusCode::BAD_GATEWAY);
            }
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, metadata.content_type)
                .header(header::CONTENT_LENGTH, metadata.content_length)
                .header(header::ETAG, metadata.etag)
                .header(header::CACHE_CONTROL, "private, no-store")
                .header("access-control-allow-origin", &record.site_origin)
                .header("access-control-allow-credentials", "true")
                .header("access-control-expose-headers", "etag, content-length")
                .header("cross-origin-resource-policy", "cross-origin")
                .header("x-content-type-options", "nosniff")
                .header(header::VARY, "origin")
                .body(stream)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn deliver_playback(
    Path(playback_id): Path<String>,
    headers: HeaderMap,
    context: Option<Extension<AuthenticatedHostContext>>,
    playback: Option<Extension<PlaybackDelivery>>,
) -> Result<Response, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    if !context.permits(XIMG_ACCESS) {
        return Err(StatusCode::FORBIDDEN);
    }
    let playback = playback.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let range = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok());
    let if_none_match = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok());
    let mut playback = playback
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match playback.deliver(context.actor_id(), &playback_id, range, if_none_match) {
        Ok(DirectPlaybackResponse::NotModified { etag }) => Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, etag)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
        Ok(DirectPlaybackResponse::Content {
            partial,
            headers,
            stream,
        }) => {
            let mut response = Response::builder()
                .status(if partial {
                    StatusCode::PARTIAL_CONTENT
                } else {
                    StatusCode::OK
                })
                .header(header::CONTENT_TYPE, headers.content_type)
                .header(header::CONTENT_LENGTH, headers.content_length)
                .header(header::ETAG, headers.etag);
            if headers.accept_ranges {
                response = response.header(header::ACCEPT_RANGES, "bytes");
            }
            if let Some(range) = headers.content_range {
                response = response.header(
                    header::CONTENT_RANGE,
                    format!(
                        "bytes {}-{}/{}",
                        range.start, range.end_inclusive, headers.total_length
                    ),
                );
            }
            response
                .body(stream)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
        Err(error) => Err(playback_status(error)),
    }
}

fn playback_status(error: DirectPlaybackError) -> StatusCode {
    match error {
        DirectPlaybackError::InvalidRange => StatusCode::RANGE_NOT_SATISFIABLE,
        DirectPlaybackError::UnknownPlayback => StatusCode::NOT_FOUND,
        DirectPlaybackError::Forbidden => StatusCode::FORBIDDEN,
        DirectPlaybackError::NotReady => StatusCode::CONFLICT,
        DirectPlaybackError::Read(_) => StatusCode::BAD_GATEWAY,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::{Arc, Mutex},
    };

    use axum::{
        Extension,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use x_img_core::{
        cache_alias::{
            CacheAliasIndex, CacheAliasRecord, CacheEligibility, CacheLookupAuthorization,
            CacheLookupService, CacheObjectAvailability, CacheRepresentation,
        },
        gallery_catalogue::{
            GalleryCatalogue, GalleryItem, GalleryMediaKind, GalleryObjectAvailability,
            GalleryRepresentation, GalleryRepresentationKind, GalleryReviewState,
            GallerySourceKind,
        },
        host_context::{HostContextAdapter, MonasHostContextAdapter},
        object_read::{
            AuthorizedObjectReader, AuthorizedObjectReference, ObjectContentMetadata,
            ObjectReadResult,
        },
        operations::{Component, EventCode, EventOutcome, HealthState, OperationalTelemetry},
        playback_delivery::{DirectPlaybackGrant, DirectPlaybackService},
        synoptikon_catalogue::{
            CatalogueMediaKind, CatalogueReviewState, SynoptikonCatalogueItem,
            SynoptikonCatalogueProjection,
        },
        video_profile::NormalizedVideoState,
        viewed_media::{
            AdapterKind, CAPTURE_REQUEST_SCHEMA_VERSION, CaptureKind, CapturePairing,
            CapturePlanRequest, CapturePlanService, SiteCapturePolicy,
        },
    };

    use super::{
        HostObjectReadBackend, MonasDispatchVerifier, monolith_router,
        monolith_router_with_authorities, monolith_router_with_gallery_authority,
        monolith_router_with_gallery_delivery_authority,
        monolith_router_with_gallery_web_delivery_authority, monolith_router_with_storage, router,
        router_with_cache_aliases, router_with_cache_substitution, router_with_capture_plans,
        router_with_direct_playback, router_with_gallery_catalogue, router_with_gallery_delivery,
        router_with_image_substitution, router_with_operations, router_with_synoptikon_catalogue,
    };

    const CHECKSUM: &str =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const ETAG: &str =
        "\"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"";
    const PLAYBACK_BYTES: &[u8] = b"synthetic-firefox-playback";
    const MONAS_CONTEXT: &str = r#"{"schema_version":"x-img.host-context.v1","host":"monas","host_mode":"monas_standalone","actor_id":"synthetic-monas-user","authorizations":["ximg.access"],"correlation_id":"fixture-monas-correlation"}"#;

    fn gallery_catalogue() -> GalleryCatalogue {
        let object = |kind, path: Option<&str>, availability| GalleryRepresentation {
            kind,
            availability,
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
            object_key: "objects/media-1".into(),
            object_version: 1,
            checksum: CHECKSUM.into(),
            content_type: "image/jpeg".into(),
            content_length: 12,
            delivery_path: path.map(Into::into),
        };
        GalleryCatalogue::new(vec![GalleryItem {
            catalogue_id: "media-1".into(),
            title: "Synthetic redistributable image".into(),
            source_label: "Example website".into(),
            source_kind: GallerySourceKind::Website,
            media_kind: GalleryMediaKind::Image,
            review_state: GalleryReviewState::New,
            discovered_at_epoch_seconds: 1,
            width: 320,
            height: 200,
            thumbnail: object(
                GalleryRepresentationKind::Thumbnail,
                Some("/api/gallery/v1/objects/thumbnail-1"),
                GalleryObjectAvailability::Ready,
            ),
            preview: Some(object(
                GalleryRepresentationKind::OriginalImage,
                None,
                GalleryObjectAvailability::Unavailable,
            )),
        }])
        .unwrap()
    }

    fn gallery_image_backend() -> HostObjectReadBackend {
        HostObjectReadBackend::new(Box::new(|request| {
            assert_eq!(request.object.endpoint_id, "endpoint-1");
            assert_eq!(request.object.object_store_id, "store-1");
            assert_eq!(request.object.object_key, "objects/media-1");
            if request.if_none_match_etag.as_deref() == Some(ETAG) {
                return Ok(ObjectReadResult::NotModified { etag: ETAG.into() });
            }
            Ok(ObjectReadResult::Content {
                metadata: ObjectContentMetadata {
                    content_type: "image/jpeg".into(),
                    content_length: 12,
                    total_length: 12,
                    checksum: CHECKSUM.into(),
                    etag: ETAG.into(),
                    content_range: None,
                },
                stream: Body::from(b"image-bytes!".to_vec()),
            })
        }))
    }

    fn video_gallery_catalogue() -> GalleryCatalogue {
        let representation = |kind, object_key: &str, content_type: &str, content_length, role| {
            GalleryRepresentation {
                kind,
                availability: GalleryObjectAvailability::Ready,
                endpoint_id: "endpoint-1".into(),
                object_store_id: "store-1".into(),
                object_key: object_key.into(),
                object_version: 1,
                checksum: CHECKSUM.into(),
                content_type: content_type.into(),
                content_length,
                delivery_path: Some(format!(
                    "/products/pinakotheke/api/gallery/v1/objects/video-1/{role}"
                )),
            }
        };
        GalleryCatalogue::new(vec![GalleryItem {
            catalogue_id: "video-1".into(),
            title: "Synthetic normalized video".into(),
            source_label: "Example website".into(),
            source_kind: GallerySourceKind::Website,
            media_kind: GalleryMediaKind::NormalizedVideo,
            review_state: GalleryReviewState::New,
            discovered_at_epoch_seconds: 1,
            width: 1920,
            height: 1080,
            thumbnail: representation(
                GalleryRepresentationKind::VideoPoster,
                "video/poster.webp",
                "image/webp",
                12,
                "thumbnail",
            ),
            preview: Some(representation(
                GalleryRepresentationKind::NormalizedVideo,
                "video/normalized.mp4",
                "video/mp4",
                PLAYBACK_BYTES.len() as u64,
                "video",
            )),
        }])
        .unwrap()
    }

    fn gallery_video_backend() -> HostObjectReadBackend {
        HostObjectReadBackend::new(Box::new(|request| {
            assert_eq!(request.object.object_key, "video/normalized.mp4");
            let range = request.range;
            let (start, end) = range.map_or((0, PLAYBACK_BYTES.len() - 1), |range| {
                (range.start as usize, range.end_inclusive as usize)
            });
            let bytes = PLAYBACK_BYTES[start..=end].to_vec();
            Ok(ObjectReadResult::Content {
                metadata: ObjectContentMetadata {
                    content_type: "video/mp4".into(),
                    content_length: bytes.len() as u64,
                    total_length: PLAYBACK_BYTES.len() as u64,
                    checksum: CHECKSUM.into(),
                    etag: ETAG.into(),
                    content_range: range,
                },
                stream: Body::from(bytes),
            })
        }))
    }

    fn gallery_poster_backend() -> HostObjectReadBackend {
        HostObjectReadBackend::new(Box::new(|request| {
            assert_eq!(request.object.object_key, "video/poster.webp");
            Ok(ObjectReadResult::Content {
                metadata: ObjectContentMetadata {
                    content_type: "image/webp".into(),
                    content_length: 12,
                    total_length: 12,
                    checksum: CHECKSUM.into(),
                    etag: ETAG.into(),
                    content_range: None,
                },
                stream: Body::from(b"poster-bytes".to_vec()),
            })
        }))
    }

    fn direct_playback() -> DirectPlaybackService<HostObjectReadBackend> {
        let backend = HostObjectReadBackend::new(Box::new(|request| {
            if request.if_none_match_etag.as_deref() == Some(ETAG) {
                return Ok(ObjectReadResult::NotModified { etag: ETAG.into() });
            }
            let range = request.range;
            let (start, end_inclusive) = range
                .map_or((0, PLAYBACK_BYTES.len() as u64 - 1), |range| {
                    (range.start, range.end_inclusive)
                });
            let bytes = PLAYBACK_BYTES[start as usize..=end_inclusive as usize].to_vec();
            Ok(ObjectReadResult::Content {
                metadata: ObjectContentMetadata {
                    content_type: "video/mp4".into(),
                    content_length: bytes.len() as u64,
                    total_length: PLAYBACK_BYTES.len() as u64,
                    checksum: CHECKSUM.into(),
                    etag: ETAG.into(),
                    content_range: range,
                },
                stream: Body::from(bytes),
            })
        }));
        DirectPlaybackService::new(
            AuthorizedObjectReader::new(backend),
            [DirectPlaybackGrant {
                playback_id: "normalized-video-1".into(),
                actor_id: "synthetic-monas-user".into(),
                object: AuthorizedObjectReference {
                    endpoint_id: "synthetic-endpoint".into(),
                    object_store_id: "synthetic-store".into(),
                    object_key: "normalized/video.mp4".into(),
                    object_version: 1,
                    checksum: CHECKSUM.into(),
                },
                total_length: PLAYBACK_BYTES.len() as u64,
                state: NormalizedVideoState::Ready,
            }],
        )
    }

    fn cache_aliases() -> CacheLookupService {
        let mut index = CacheAliasIndex::new(8).expect("bounded index");
        index
            .admit(CacheAliasRecord {
                delivery_id: "image-delivery-1".into(),
                instance_id: "ximg-instance-1".into(),
                site_origin: "https://example.invalid".into(),
                canonical_alias: "https://media.example.invalid/image-1.jpg".into(),
                adapter_id: "generic-observed-image".into(),
                adapter_version: "1.0.0".into(),
                representation: CacheRepresentation::ThumbnailImage,
                eligibility: CacheEligibility::ObservedThumbnail,
                object: AuthorizedObjectReference {
                    endpoint_id: "synthetic-endpoint".into(),
                    object_store_id: "synthetic-store".into(),
                    object_key: "images/image-1.jpg".into(),
                    object_version: 1,
                    checksum: CHECKSUM.into(),
                },
                content_type: "image/jpeg".into(),
                content_length: 15,
                valid_until_epoch_seconds: u64::MAX,
                availability: CacheObjectAvailability::Ready,
            })
            .expect("synthetic alias is eligible");
        index
            .admit(CacheAliasRecord {
                delivery_id: "video-delivery-1".into(),
                instance_id: "ximg-instance-1".into(),
                site_origin: "https://example.invalid".into(),
                canonical_alias: "https://media.example.invalid/video-1.mp4".into(),
                adapter_id: "generic-observed-image".into(),
                adapter_version: "1.0.0".into(),
                representation: CacheRepresentation::NormalizedMp4,
                eligibility: CacheEligibility::ExplicitlyOpenedOriginal,
                object: AuthorizedObjectReference {
                    endpoint_id: "synthetic-endpoint".into(),
                    object_store_id: "synthetic-store".into(),
                    object_key: "videos/video-1.mp4".into(),
                    object_version: 1,
                    checksum: CHECKSUM.into(),
                },
                content_type: "video/mp4".into(),
                content_length: PLAYBACK_BYTES.len() as u64,
                valid_until_epoch_seconds: u64::MAX,
                availability: CacheObjectAvailability::Ready,
            })
            .expect("normalized video alias is eligible");
        CacheLookupService::new(
            index,
            [CacheLookupAuthorization {
                pairing_id: "pair-cache-1".into(),
                actor_id: "synthetic-monas-user".into(),
                instance_id: "ximg-instance-1".into(),
                site_origin: "https://example.invalid".into(),
                adapter_id: "generic-observed-image".into(),
                adapter_version: "1.0.0".into(),
                substitution_enabled: true,
                expires_at_epoch_seconds: u64::MAX,
                revoked: false,
            }],
        )
        .expect("synthetic lookup authorization")
    }

    fn cache_lookup_body(alias: &str) -> Body {
        Body::from(
            serde_json::to_vec(&serde_json::json!({
                "schema_version": "x-img.cache-alias-lookup.v1",
                "pairing_id": "pair-cache-1",
                "instance_id": "ximg-instance-1",
                "origin": "https://example.invalid",
                "canonical_alias": alias,
                "adapter_id": "generic-observed-image",
                "adapter_version": "1.0.0"
            }))
            .expect("synthetic cache lookup serializes"),
        )
    }

    fn capture_plans() -> CapturePlanService {
        CapturePlanService::new(
            [CapturePairing {
                pairing_id: "pair-0".into(),
                actor_id: "synthetic-monas-user".into(),
                expires_at: u64::MAX,
                revoked: false,
            }],
            [SiteCapturePolicy {
                site_id: "synthetic-site".into(),
                origin: "https://example.invalid".into(),
                capture_enabled: true,
                adapter_kind: AdapterKind::ExperimentalGeneric,
                adapter_version: "1.0.0".into(),
                allow_observed_thumbnails: true,
                allow_explicit_originals: false,
                max_candidates_per_page: 2,
            }],
        )
    }

    fn request_body() -> Body {
        Body::from(
            serde_json::to_vec(&CapturePlanRequest {
                schema_version: CAPTURE_REQUEST_SCHEMA_VERSION.into(),
                pairing_id: "pair-0".into(),
                origin: "https://example.invalid".into(),
                page_url: "https://example.invalid/gallery?private=redacted".into(),
                adapter_kind: AdapterKind::ExperimentalGeneric,
                adapter_version: "1.0.0".into(),
                capture_kind: CaptureKind::ObservedThumbnail,
                media_url: "https://example.invalid/thumbnail.webp?signature=redacted".into(),
                width: 320,
                height: 200,
            })
            .expect("synthetic request serializes"),
        )
    }

    fn synoptikon_catalogue() -> SynoptikonCatalogueProjection {
        SynoptikonCatalogueProjection::new(vec![SynoptikonCatalogueItem {
            catalogue_id: "media-1".into(),
            project_id: "synthetic-project".into(),
            media_kind: CatalogueMediaKind::Image,
            review_state: CatalogueReviewState::Accepted,
            source_label: "Synthetic fixture".into(),
            discovered_at_epoch_seconds: 1,
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
            object_key: "objects/media-1".into(),
            checksum: CHECKSUM.into(),
            content_type: "image/png".into(),
            content_length: 12,
        }])
    }

    #[tokio::test]
    async fn synoptikon_catalogue_is_project_scoped_and_host_authenticated() {
        let context = x_img_core::host_context::SynoptikonHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/synoptikon-valid.json"
            ))
            .expect("valid Synoptikon context");
        let response = router_with_synoptikon_catalogue(synoptikon_catalogue())
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .uri("/api/synoptikon/v1/catalogue?limit=100")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["project_id"], "synthetic-project");
        assert_eq!(json["items"][0]["catalogue_id"], "media-1");
        assert!(json["items"][0].get("source_url").is_none());

        let denied = router_with_synoptikon_catalogue(synoptikon_catalogue())
            .oneshot(
                Request::builder()
                    .uri("/api/synoptikon/v1/catalogue")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn monas_gallery_exposes_only_authorized_object_delivery_paths() {
        let context = MonasHostContextAdapter
            .authenticate(MONAS_CONTEXT.as_bytes())
            .unwrap();
        let response = router_with_gallery_catalogue(gallery_catalogue())
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/gallery/v1/catalogue?limit=100")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["schema_version"], "pinakotheke.gallery-catalogue.v1");
        assert_eq!(json["matched_items"], 1);
        assert_eq!(json["total_items"], 1);
        assert_eq!(
            json["items"][0]["thumbnail"]["delivery_path"],
            "/api/gallery/v1/objects/thumbnail-1"
        );
        assert_eq!(json["items"][0]["preview"]["availability"], "unavailable");
        assert!(json["items"][0].get("source_url").is_none());
        assert!(json["items"][0]["preview"]["delivery_path"].is_null());

        let denied = router_with_gallery_catalogue(gallery_catalogue())
            .oneshot(
                Request::builder()
                    .uri("/api/gallery/v1/catalogue")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

        let context = MonasHostContextAdapter
            .authenticate(MONAS_CONTEXT.as_bytes())
            .unwrap();
        let filtered = router_with_gallery_catalogue(gallery_catalogue())
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/gallery/v1/catalogue?source_kind=website&media_kind=image&review_state=new&availability=ready&text=redistributable&limit=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(filtered.status(), StatusCode::OK);
        let body = to_bytes(filtered.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["matched_items"], 1);
        assert_eq!(json["items"][0]["catalogue_id"], "media-1");

        let context = MonasHostContextAdapter
            .authenticate(MONAS_CONTEXT.as_bytes())
            .unwrap();
        let invalid = router_with_gallery_catalogue(gallery_catalogue())
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/gallery/v1/catalogue?discovered_from_epoch_seconds=2&discovered_to_epoch_seconds=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn gallery_image_streams_only_the_persisted_authorized_object() {
        let context = MonasHostContextAdapter
            .authenticate(MONAS_CONTEXT.as_bytes())
            .unwrap();
        let response = router_with_gallery_delivery(gallery_catalogue(), gallery_image_backend())
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/gallery/v1/objects/media-1/thumbnail")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "image/jpeg");
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        assert_eq!(
            to_bytes(response.into_body(), 64).await.unwrap().as_ref(),
            b"image-bytes!"
        );

        let unavailable =
            router_with_gallery_delivery(gallery_catalogue(), gallery_image_backend())
                .layer(Extension(
                    MonasHostContextAdapter
                        .authenticate(MONAS_CONTEXT.as_bytes())
                        .unwrap(),
                ))
                .oneshot(
                    Request::builder()
                        .uri("/products/pinakotheke/api/gallery/v1/objects/media-1/original")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
        assert_eq!(unavailable.status(), StatusCode::GONE);

        let unauthorized =
            router_with_gallery_delivery(gallery_catalogue(), gallery_image_backend())
                .oneshot(
                    Request::builder()
                        .uri("/products/pinakotheke/api/gallery/v1/objects/media-1/thumbnail")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn monolith_gallery_delivery_requires_the_private_monas_dispatch() {
        let token = "synthetic-monas-dispatch-token-0001";
        let router = || {
            monolith_router_with_gallery_delivery_authority(
                true,
                Some(MonasDispatchVerifier::new(token.into()).unwrap()),
                gallery_catalogue(),
                gallery_image_backend(),
            )
        };
        let direct = router()
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/gallery/v1/objects/media-1/thumbnail")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(direct.status(), StatusCode::UNAUTHORIZED);

        let admitted = router()
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/gallery/v1/objects/media-1/thumbnail")
                    .header("x-monas-dispatch-token", token)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(admitted.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn gallery_video_preserves_authenticated_single_range_playback() {
        let context = MonasHostContextAdapter
            .authenticate(MONAS_CONTEXT.as_bytes())
            .unwrap();
        let poster =
            router_with_gallery_delivery(video_gallery_catalogue(), gallery_poster_backend())
                .layer(Extension(context.clone()))
                .oneshot(
                    Request::builder()
                        .uri("/products/pinakotheke/api/gallery/v1/objects/video-1/thumbnail")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
        assert_eq!(poster.status(), StatusCode::OK);
        assert_eq!(poster.headers()["content-type"], "image/webp");

        let response =
            router_with_gallery_delivery(video_gallery_catalogue(), gallery_video_backend())
                .layer(Extension(context))
                .oneshot(
                    Request::builder()
                        .uri("/products/pinakotheke/api/gallery/v1/objects/video-1/video")
                        .header("range", "bytes=2-10")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()["content-type"], "video/mp4");
        assert_eq!(response.headers()["accept-ranges"], "bytes");
        assert_eq!(response.headers()["content-range"], "bytes 2-10/26");
        assert_eq!(
            to_bytes(response.into_body(), 64).await.unwrap().as_ref(),
            &PLAYBACK_BYTES[2..=10]
        );

        let invalid =
            router_with_gallery_delivery(video_gallery_catalogue(), gallery_video_backend())
                .layer(Extension(
                    MonasHostContextAdapter
                        .authenticate(MONAS_CONTEXT.as_bytes())
                        .unwrap(),
                ))
                .oneshot(
                    Request::builder()
                        .uri("/products/pinakotheke/api/gallery/v1/objects/video-1/video")
                        .header("range", "bytes=0-1,3-4")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
        assert_eq!(invalid.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    }

    #[test]
    fn creates_a_router_without_starting_a_listener() {
        let _router = router();
    }

    #[tokio::test]
    async fn monolith_readiness_is_honest_until_authorities_are_composed() {
        let response = monolith_router()
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "not_ready");
        assert_eq!(json["components"][0]["status"], "Ready");
        assert_eq!(json["components"][1]["status"], "Not configured");
        assert_eq!(json["components"][2]["status"], "Not configured");
    }

    #[tokio::test]
    async fn monolith_reports_a_verified_local_objectstore_without_claiming_authentication() {
        let response = monolith_router_with_storage(true)
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = to_bytes(response.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "not_ready");
        assert_eq!(json["components"][1]["status"], "Not configured");
        assert_eq!(json["components"][2]["status"], "Ready");
    }

    #[tokio::test]
    async fn monas_dispatch_requires_both_process_credential_and_valid_host_context() {
        let token = "synthetic-monas-dispatch-token-0001";
        let router = || {
            monolith_router_with_authorities(
                true,
                Some(MonasDispatchVerifier::new(token.into()).unwrap()),
            )
        };
        let direct = router()
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/context")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(direct.status(), StatusCode::UNAUTHORIZED);

        let invalid = router()
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/context")
                    .header(
                        "x-monas-dispatch-token",
                        "wrong-token-value-with-enough-length",
                    )
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);

        let admitted = router()
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/context")
                    .header("x-monas-dispatch-token", token)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(admitted.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn built_yew_application_is_available_only_through_monas_dispatch() {
        let root =
            std::env::temp_dir().join(format!("pinakotheke-web-root-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("index.html"),
            "<!doctype html><title>Pinakotheke gallery</title>",
        )
        .unwrap();
        fs::write(root.join("pinakotheke.js"), "export {};").unwrap();
        let token = "synthetic-monas-dispatch-token-0001";
        let router = || {
            monolith_router_with_gallery_web_delivery_authority(
                true,
                Some(MonasDispatchVerifier::new(token.into()).unwrap()),
                gallery_catalogue(),
                Some(root.clone()),
                gallery_image_backend(),
            )
        };

        let direct = router()
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/app/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(direct.status(), StatusCode::UNAUTHORIZED);

        let admitted = router()
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/app/")
                    .header("x-monas-dispatch-token", token)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(admitted.status(), StatusCode::OK);
        let body = to_bytes(admitted.into_body(), 1024).await.unwrap();
        assert!(String::from_utf8_lossy(&body).contains("Pinakotheke gallery"));

        let image = router()
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/gallery/v1/objects/media-1/thumbnail")
                    .header("x-monas-dispatch-token", token)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(image.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(image.into_body(), 1024).await.unwrap(),
            b"image-bytes!".as_slice()
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn monolith_composes_the_authenticated_gallery_on_the_monas_mount() {
        let token = "synthetic-monas-dispatch-token-0001";
        let response = monolith_router_with_gallery_authority(
            true,
            Some(MonasDispatchVerifier::new(token.into()).unwrap()),
            gallery_catalogue(),
        )
        .oneshot(
            Request::builder()
                .uri("/products/pinakotheke/api/gallery/v1/catalogue")
                .header("x-monas-dispatch-token", token)
                .header("x-monas-host-context", MONAS_CONTEXT)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["items"][0]["catalogue_id"], "media-1");
    }

    #[tokio::test]
    async fn public_health_is_coarse_and_operations_require_host_context() {
        let health = router()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);
        let health_body = to_bytes(health.into_body(), 4096).await.unwrap();
        let health_json: serde_json::Value = serde_json::from_slice(&health_body).unwrap();
        assert_eq!(health_json["status"], "alive");
        assert!(health_json.get("components").is_none());
        assert!(health_json.get("audit").is_none());

        let mut telemetry = OperationalTelemetry::default();
        telemetry.set_health(Component::HostContext, HealthState::Ready);
        telemetry.record(
            Component::ObjectStore,
            EventCode::ObjectUnavailable,
            EventOutcome::Pending,
        );
        let operations = Arc::new(Mutex::new(telemetry));
        let direct = router_with_operations(Arc::clone(&operations))
            .oneshot(
                Request::builder()
                    .uri("/api/operations/v1/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(direct.status(), StatusCode::UNAUTHORIZED);

        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .unwrap();
        let admitted = router_with_operations(operations)
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .uri("/api/operations/v1/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(admitted.status(), StatusCode::OK);
        let body = to_bytes(admitted.into_body(), 16 * 1024).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("object_unavailable"));
        for prohibited in ["http://", "https://", "cookie", "authorization", "session"] {
            assert!(!text.contains(prohibited));
        }
    }

    #[tokio::test]
    async fn privileged_route_rejects_direct_access_and_accepts_host_context() {
        let direct = router()
            .oneshot(
                Request::builder()
                    .uri("/context")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(direct.status(), StatusCode::UNAUTHORIZED);

        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .expect("synthetic host context is valid");
        let admitted = router()
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .uri("/context")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(admitted.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn capture_plan_requires_host_context_and_never_receives_payload_bytes() {
        let direct = router_with_capture_plans(capture_plans())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/extension/v1/capture-plans")
                    .header("content-type", "application/json")
                    .body(request_body())
                    .expect("request must build"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(direct.status(), StatusCode::UNAUTHORIZED);

        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .expect("synthetic host context is valid");
        let admitted = router_with_capture_plans(capture_plans())
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/extension/v1/capture-plans")
                    .header("content-type", "application/json")
                    .body(request_body())
                    .expect("request must build"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(admitted.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn default_router_fails_open_for_unconfigured_capture_policy() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .expect("synthetic host context is valid");
        let response = router()
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/extension/v1/capture-plans")
                    .header("content-type", "application/json")
                    .body(request_body())
                    .expect("request must build"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn direct_playback_is_host_authorized_and_preserves_a_single_range_stream() {
        let unauthorized = router_with_direct_playback(direct_playback())
            .oneshot(
                Request::builder()
                    .uri("/api/playback/v1/normalized-video-1")
                    .header("range", "bytes=2-10")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .expect("synthetic host context is valid");
        let response = router_with_direct_playback(direct_playback())
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .uri("/api/playback/v1/normalized-video-1")
                    .header("range", "bytes=2-10")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()["accept-ranges"], "bytes");
        assert_eq!(response.headers()["content-range"], "bytes 2-10/26");
        assert_eq!(
            to_bytes(response.into_body(), 1024)
                .await
                .expect("body streams")
                .as_ref(),
            &PLAYBACK_BYTES[2..=10]
        );
    }

    #[tokio::test]
    async fn direct_playback_rejects_multi_ranges_without_an_origin_fallback() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .expect("synthetic host context is valid");
        let response = router_with_direct_playback(direct_playback())
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .uri("/api/playback/v1/normalized-video-1")
                    .header("range", "bytes=0-1,3-4")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    }

    #[tokio::test]
    async fn direct_playback_preserves_checksum_etags_for_conditional_requests() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .expect("synthetic host context is valid");
        let response = router_with_direct_playback(direct_playback())
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .uri("/api/playback/v1/normalized-video-1")
                    .header("if-none-match", ETAG)
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(response.headers()["etag"], ETAG);
    }

    #[tokio::test]
    async fn cache_alias_lookup_requires_host_context_and_returns_bounded_hit_metadata() {
        let unauthorized = router_with_cache_aliases(cache_aliases())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/extension/v1/cache-aliases/lookup")
                    .header("content-type", "application/json")
                    .body(cache_lookup_body(
                        "https://media.example.invalid/image-1.jpg",
                    ))
                    .expect("request must build"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .expect("synthetic host context is valid");
        let response = router_with_cache_aliases(cache_aliases())
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/extension/v1/cache-aliases/lookup")
                    .header("content-type", "application/json")
                    .body(cache_lookup_body(
                        "https://media.example.invalid/image-1.jpg",
                    ))
                    .expect("request must build"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value = serde_json::from_slice(
            &to_bytes(response.into_body(), 4_096)
                .await
                .expect("bounded response"),
        )
        .expect("lookup response is JSON");
        assert_eq!(body["outcome"], "hit");
        assert_eq!(body["media_class"], "thumbnail_image");
        assert_eq!(
            body["delivery_path"],
            "/api/cache/v1/images/pair-cache-1/image-delivery-1"
        );
        assert!(body.get("canonical_alias").is_none());
        assert!(body.get("endpoint_id").is_none());
    }

    #[tokio::test]
    async fn cache_alias_signed_query_returns_origin_fallback_without_echoing_the_alias() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .expect("synthetic host context is valid");
        let response = router_with_cache_aliases(cache_aliases())
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/extension/v1/cache-aliases/lookup")
                    .header("content-type", "application/json")
                    .body(cache_lookup_body(
                        "https://media.example.invalid/image-1.jpg?signature=secret",
                    ))
                    .expect("request must build"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 4_096)
            .await
            .expect("bounded response");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON response");
        assert_eq!(body["outcome"], "origin_fallback");
        assert_eq!(body["reason"], "invalid_request");
        assert!(!String::from_utf8_lossy(&bytes).contains("signature"));
    }

    #[tokio::test]
    async fn cached_image_delivery_streams_exact_authorized_object_with_browser_headers() {
        const IMAGE: &[u8] = b"synthetic-image";
        let backend = HostObjectReadBackend::new(Box::new(|request| {
            assert_eq!(request.object.object_key, "images/image-1.jpg");
            Ok(ObjectReadResult::Content {
                metadata: ObjectContentMetadata {
                    content_type: "image/jpeg".into(),
                    content_length: IMAGE.len() as u64,
                    total_length: IMAGE.len() as u64,
                    checksum: CHECKSUM.into(),
                    etag: ETAG.into(),
                    content_range: None,
                },
                stream: Body::from(IMAGE),
            })
        }));
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .expect("synthetic host context is valid");
        let response = router_with_image_substitution(cache_aliases(), backend)
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .uri("/api/cache/v1/images/pair-cache-1/image-delivery-1")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "image/jpeg");
        assert_eq!(response.headers()["content-length"], "15");
        assert_eq!(response.headers()["etag"], ETAG);
        assert_eq!(
            response.headers()["access-control-allow-origin"],
            "https://example.invalid"
        );
        assert_eq!(
            response.headers()["cross-origin-resource-policy"],
            "cross-origin"
        );
        assert_eq!(
            response.headers()["access-control-expose-headers"],
            "etag, content-length"
        );
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        assert_eq!(
            to_bytes(response.into_body(), 128)
                .await
                .expect("image streams")
                .as_ref(),
            IMAGE
        );
    }

    fn cache_video_backend() -> HostObjectReadBackend {
        HostObjectReadBackend::new(Box::new(|request| {
            if request.if_none_match_etag.as_deref() == Some(ETAG) {
                return Ok(ObjectReadResult::NotModified { etag: ETAG.into() });
            }
            let range = request.range;
            let (start, end) = range.map_or((0, PLAYBACK_BYTES.len() as u64 - 1), |range| {
                (range.start, range.end_inclusive)
            });
            let bytes = PLAYBACK_BYTES[start as usize..=end as usize].to_vec();
            Ok(ObjectReadResult::Content {
                metadata: ObjectContentMetadata {
                    content_type: "video/mp4".into(),
                    content_length: bytes.len() as u64,
                    total_length: PLAYBACK_BYTES.len() as u64,
                    checksum: CHECKSUM.into(),
                    etag: ETAG.into(),
                    content_range: range,
                },
                stream: Body::from(bytes),
            })
        }))
    }

    #[tokio::test]
    async fn cached_video_preserves_ranges_conditionals_and_concurrent_streams() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .expect("synthetic host context is valid");
        let app = router_with_cache_substitution(cache_aliases(), cache_video_backend())
            .layer(Extension(context));
        let request = |range: &'static str| {
            Request::builder()
                .uri("/api/cache/v1/videos/pair-cache-1/video-delivery-1")
                .header("range", range)
                .body(Body::empty())
                .expect("request builds")
        };
        let (first, second) = tokio::join!(
            app.clone().oneshot(request("bytes=0-7")),
            app.clone().oneshot(request("bytes=8-15"))
        );
        let first = first.expect("router is infallible");
        let second = second.expect("router is infallible");
        assert_eq!(first.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(second.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(first.headers()["content-range"], "bytes 0-7/26");
        assert_eq!(second.headers()["content-range"], "bytes 8-15/26");
        assert_eq!(first.headers()["accept-ranges"], "bytes");
        assert_eq!(first.headers()["content-type"], "video/mp4");
        assert_eq!(
            to_bytes(first.into_body(), 64).await.expect("first stream"),
            &PLAYBACK_BYTES[0..=7]
        );
        assert_eq!(
            to_bytes(second.into_body(), 64)
                .await
                .expect("second stream"),
            &PLAYBACK_BYTES[8..=15]
        );

        let conditional = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/cache/v1/videos/pair-cache-1/video-delivery-1")
                    .header("if-none-match", ETAG)
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router is infallible");
        assert_eq!(conditional.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(conditional.headers()["etag"], ETAG);

        let invalid = app
            .oneshot(request("bytes=0-1,4-5"))
            .await
            .expect("router is infallible");
        assert_eq!(invalid.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(invalid.headers()["content-range"], "bytes */26");
    }

    #[tokio::test]
    async fn cache_lookup_returns_a_video_specific_delivery_path() {
        let context = MonasHostContextAdapter
            .authenticate(include_bytes!(
                "../../../fixtures/host-context/v1/monas-valid.json"
            ))
            .expect("synthetic host context is valid");
        let response = router_with_cache_aliases(cache_aliases())
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/extension/v1/cache-aliases/lookup")
                    .header("content-type", "application/json")
                    .body(cache_lookup_body(
                        "https://media.example.invalid/video-1.mp4",
                    ))
                    .expect("request builds"),
            )
            .await
            .expect("router is infallible");
        let body: serde_json::Value = serde_json::from_slice(
            &to_bytes(response.into_body(), 4_096)
                .await
                .expect("bounded response"),
        )
        .expect("JSON response");
        assert_eq!(body["media_class"], "normalized_mp4");
        assert_eq!(
            body["delivery_path"],
            "/api/cache/v1/videos/pair-cache-1/video-delivery-1"
        );
    }
}
