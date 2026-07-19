// SPDX-License-Identifier: MPL-2.0
//! Axum composition boundary for a future host-managed API service.
//!
//! A host must validate its session before injecting an authenticated context.
//! This crate never parses cookies, passwords, or session tokens.

use std::{
    collections::BTreeSet,
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
use tokio::sync::Semaphore;
use tower_http::services::ServeDir;
use x_img_core::{
    cache_alias::{
        CacheBypassReason, CacheLookupOutcome, CacheLookupRequest, CacheLookupService,
        CacheRepresentation,
    },
    capture_completion::{
        CaptureCompletionError, CaptureCompletionOutcome, VerifiedCaptureCompletion,
        complete_verified_image,
    },
    gallery_catalogue::{
        GalleryCatalogue, GalleryCatalogueError, GalleryCatalogueFilter, GalleryCatalogueStore,
        GalleryFolderPage, GalleryImageResolveError, GalleryImageRole, GalleryMediaKind,
        GalleryObjectAvailability, GalleryPage, GalleryReviewState, GallerySourceKind,
    },
    host_context::{
        AuthenticatedHostContext, HostContextAdapter, MonasHostContextAdapter, XIMG_ACCESS,
    },
    object_read::{
        AuthorizedObjectReader, ObjectReadBackend, ObjectReadBackendError, ObjectReadError,
        ObjectReadRequest, ObjectReadResult, ValidatedObjectRead,
    },
    operations::{OperationalTelemetry, OperationsSnapshot},
    playback_delivery::{
        DirectPlaybackError, DirectPlaybackResponse, DirectPlaybackService, parse_single_range,
    },
    reviewed_destination::{
        ReplaceReviewedDestination, ReviewedDestinationError, ReviewedDestinationStore,
    },
    site_corpus::{ReplaceSiteCorpus, SiteCorpusError, SiteCorpusStore},
    synoptikon_catalogue::{
        SynoptikonCatalogueError, SynoptikonCataloguePage, SynoptikonCatalogueProjection,
    },
    viewed_media::{
        CaptureDestinationSnapshot, CaptureKind, CapturePlan, CapturePlanError, CapturePlanRequest,
        CapturePlanService,
    },
};

type CapturePlans = Arc<Mutex<CapturePlanService>>;
type PlaybackDelivery = Arc<Mutex<DirectPlaybackService<HostObjectReadBackend>>>;
type CacheAliases = Arc<Mutex<CacheLookupService>>;
type ImageDelivery = Arc<ObjectDeliveryPool>;
type Operations = Arc<Mutex<OperationalTelemetry>>;
type SynoptikonCatalogue = Arc<SynoptikonCatalogueProjection>;
type MonasGalleryCatalogue = Arc<Mutex<GalleryCatalogue>>;
type SiteCorpora = Arc<Mutex<SiteCorpusStore>>;
type ReviewedDestinations = Arc<Mutex<ReviewedDestinationStore>>;

/// Private host-worker authority used only to report independently verified
/// DASObjectStore image commits.
#[derive(Clone)]
pub struct CaptureCompletionAuthority {
    token: String,
    gallery_store: GalleryCatalogueStore,
}

/// Capture planning plus its optional separately credentialled completion port.
pub struct CapturePlanComposition {
    plans: CapturePlanService,
    completion: Option<CaptureCompletionAuthority>,
    acquire: Option<HostCaptureAcquireBackend>,
    onboarding: Option<ExtensionOnboardingAuthority>,
    site_corpus: Option<SiteCorpusStore>,
    reviewed_destinations: Option<ReviewedDestinationStore>,
    destination_revalidator: Option<HostCaptureDestinationRevalidateBackend>,
}

/// Reviewed server identity and DASObjectStore destination exposed only to an
/// authenticated Monas actor while pairing Firefox.
#[derive(Clone)]
pub struct ExtensionOnboardingAuthority {
    instance_id: String,
    endpoint_id: String,
    object_store_id: String,
    download_path: String,
}

/// Process-isolated callback that returns verified authority metadata only.
pub type HostCaptureAcquire =
    Box<dyn FnMut(&str, &CapturePlan) -> Result<VerifiedCaptureCompletion, String> + Send>;

/// Current host-authority facts for the exact destination named by a capture
/// plan. The browser inventory is never accepted as authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureDestinationAuthorityState {
    /// Stable endpoint identity returned by the host authority.
    pub endpoint_id: String,
    /// Stable logical ObjectStore identity returned by the host authority.
    pub object_store_id: String,
    /// Whether the selected endpoint still exists.
    pub endpoint_present: bool,
    /// Whether the selected ObjectStore still exists on that endpoint.
    pub object_store_present: bool,
    /// Whether the endpoint's TLS identity is currently trusted.
    pub tls_trusted: bool,
    /// Whether the actor still has a valid scoped pairing.
    pub paired: bool,
    /// Absolute expiry of that pairing.
    pub pairing_expires_at_epoch_seconds: u64,
    /// Whether the endpoint and store are operationally ready.
    pub ready: bool,
    /// Whether the selected store currently accepts this object type.
    pub writable: bool,
    /// Remaining authority-reported quota; zero is rejected.
    pub quota_available_bytes: u64,
}

/// Host callback which queries live authority for one actor and one exact
/// plan-bound destination. It must not choose or return a fallback store.
pub type HostCaptureDestinationRevalidate = Box<
    dyn FnMut(
            &str,
            &CaptureDestinationSnapshot,
            CaptureKind,
        ) -> Result<CaptureDestinationAuthorityState, String>
        + Send,
>;

/// Process-isolated live destination-authority adapter.
pub struct HostCaptureDestinationRevalidateBackend {
    revalidate: HostCaptureDestinationRevalidate,
}

/// Host adapter wrapping one process-isolated acquisition callback.
pub struct HostCaptureAcquireBackend {
    acquire: HostCaptureAcquire,
}

struct CaptureCompletionRuntime {
    authority: CaptureCompletionAuthority,
    plans: CapturePlans,
    gallery: MonasGalleryCatalogue,
    acquire: Option<Mutex<HostCaptureAcquireBackend>>,
    reviewed_destinations: Option<ReviewedDestinations>,
    destination_revalidator: Option<Mutex<HostCaptureDestinationRevalidateBackend>>,
    in_flight: Mutex<BTreeSet<String>>,
}

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
    #[serde(default)]
    canonical_presentation: Option<String>,
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
struct ExtensionOnboardingResponse {
    schema_version: &'static str,
    instance_id: String,
    pairing_reference: String,
    dasobjectstore_status: &'static str,
    endpoint_id: String,
    object_store_id: String,
    extension_download_path: String,
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

impl CaptureCompletionAuthority {
    /// Creates a private worker authority bound to the persistent gallery.
    pub fn new(
        token: String,
        gallery_store: GalleryCatalogueStore,
        endpoint_id: String,
        object_store_id: String,
    ) -> io::Result<Self> {
        if token.len() < 32 || token.len() > 256 || token.chars().any(char::is_whitespace) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "capture completion token must be a bounded non-whitespace secret",
            ));
        }
        if !safe_authority_identifier(&endpoint_id) || !safe_authority_identifier(&object_store_id)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "capture completion destination identity is invalid",
            ));
        }
        Ok(Self {
            token,
            gallery_store,
        })
    }
}

fn safe_authority_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

impl CapturePlanComposition {
    /// Creates the capture composition used by the complete monolith router.
    #[must_use]
    pub fn new(plans: CapturePlanService, completion: Option<CaptureCompletionAuthority>) -> Self {
        Self {
            plans,
            completion,
            acquire: None,
            onboarding: None,
            site_corpus: None,
            reviewed_destinations: None,
            destination_revalidator: None,
        }
    }

    /// Adds a reviewed asynchronous acquisition adapter.
    #[must_use]
    pub fn with_acquire(mut self, acquire: HostCaptureAcquireBackend) -> Self {
        self.acquire = Some(acquire);
        self
    }

    /// Adds the authenticated Firefox download and pairing presentation.
    #[must_use]
    pub fn with_onboarding(mut self, authority: ExtensionOnboardingAuthority) -> Self {
        self.onboarding = Some(authority);
        self
    }

    /// Adds actor-scoped persistent website import definitions.
    #[must_use]
    pub fn with_site_corpus(mut self, store: SiteCorpusStore) -> Self {
        self.site_corpus = Some(store);
        self
    }

    /// Adds actor-scoped persistent reviewed destination selections.
    #[must_use]
    pub fn with_reviewed_destinations(mut self, store: ReviewedDestinationStore) -> Self {
        self.reviewed_destinations = Some(store);
        self
    }

    /// Adds the host adapter which revalidates live destination authority
    /// immediately before a capture helper may run.
    #[must_use]
    pub fn with_destination_revalidator(
        mut self,
        backend: HostCaptureDestinationRevalidateBackend,
    ) -> Self {
        self.destination_revalidator = Some(backend);
        self
    }
}

impl ExtensionOnboardingAuthority {
    /// Creates a reviewed onboarding authority. No credential is stored here.
    pub fn new(
        instance_id: String,
        endpoint_id: String,
        object_store_id: String,
        download_path: String,
    ) -> io::Result<Self> {
        if !safe_authority_identifier(&instance_id)
            || !safe_authority_identifier(&endpoint_id)
            || !safe_authority_identifier(&object_store_id)
            || !download_path.starts_with("/downloads/pinakotheke-")
            || !download_path.ends_with(".xpi")
            || download_path.contains(['?', '#'])
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Firefox onboarding authority is invalid",
            ));
        }
        Ok(Self {
            instance_id,
            endpoint_id,
            object_store_id,
            download_path,
        })
    }
}

impl HostCaptureAcquireBackend {
    /// Creates the adapter from a host-owned metadata-only acquisition callback.
    pub fn new(acquire: HostCaptureAcquire) -> Self {
        Self { acquire }
    }

    fn acquire(
        &mut self,
        actor_ref: &str,
        plan: &CapturePlan,
    ) -> Result<VerifiedCaptureCompletion, String> {
        (self.acquire)(actor_ref, plan)
    }
}

impl HostCaptureDestinationRevalidateBackend {
    /// Creates an adapter from a host-owned live authority callback.
    pub fn new(revalidate: HostCaptureDestinationRevalidate) -> Self {
        Self { revalidate }
    }

    fn revalidate(
        &mut self,
        actor_id: &str,
        destination: &CaptureDestinationSnapshot,
        capture_kind: CaptureKind,
    ) -> Result<CaptureDestinationAuthorityState, String> {
        (self.revalidate)(actor_id, destination, capture_kind)
    }
}

#[derive(Debug, Serialize)]
struct OperationsResponse {
    schema_version: &'static str,
    snapshot: OperationsSnapshot,
}

#[derive(Debug, Serialize)]
struct PendingCapturePlansResponse {
    schema_version: &'static str,
    plans: Vec<CapturePlan>,
}

#[derive(Debug, Serialize)]
struct CapturePlanStatusResponse {
    schema_version: &'static str,
    plan_id: String,
    catalogue_id: String,
    state: &'static str,
}

#[derive(Debug, Serialize)]
struct IngestionStatusResponse {
    schema_version: &'static str,
    observed_assets: usize,
    observed_thumbnails: usize,
    opened_originals: usize,
    opened_videos: usize,
    pending: usize,
    stored: usize,
    gallery_items: usize,
    last_observed_at_epoch_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureCompletionRequest {
    schema_version: String,
    catalogue_id: String,
    title: String,
    content_type: String,
    content_length: u64,
    endpoint_id: String,
    object_store_id: String,
    object_key: String,
    object_version: u64,
    checksum_sha256: String,
    verified_at_epoch_seconds: u64,
    #[serde(default)]
    video: Option<x_img_core::persistent_gallery_admission::GalleryVideoCompletion>,
}

#[derive(Debug, Serialize)]
struct CaptureCompletionResponse {
    schema_version: &'static str,
    outcome: &'static str,
}

/// Server-side callback used to bridge a host's scoped DASObjectStore read
/// client to Axum. The callback returns a body stream and never exposes a
/// filesystem location or browser credential to x-img.
pub type HostObjectOpen = Box<
    dyn Fn(&ObjectReadRequest) -> Result<ObjectReadResult<Body>, ObjectReadBackendError>
        + Send
        + Sync,
>;
type SharedHostObjectOpen = Arc<
    dyn Fn(&ObjectReadRequest) -> Result<ObjectReadResult<Body>, ObjectReadBackendError>
        + Send
        + Sync,
>;

/// Concrete host adapter for direct playback routes.
///
/// The surrounding host is responsible for authenticated DASObjectStore
/// transport and TLS. x-img validates the returned object metadata and makes
/// the stream available only after its injected Monas context is authorized.
#[derive(Clone)]
pub struct HostObjectReadBackend {
    open: SharedHostObjectOpen,
}

const DEFAULT_OBJECT_READ_CONCURRENCY: usize = 128;

/// Bounded concurrent bridge from Axum to blocking host object-read helpers.
///
/// Each admitted read receives an independent reader clone. Process launch,
/// provider lookup, download, and checksum verification run on Tokio's
/// blocking pool so slow storage cannot occupy an async request worker.
struct ObjectDeliveryPool {
    reader: AuthorizedObjectReader<HostObjectReadBackend>,
    permits: Arc<Semaphore>,
}

impl ObjectDeliveryPool {
    fn new(backend: HostObjectReadBackend) -> Self {
        Self::with_concurrency(backend, DEFAULT_OBJECT_READ_CONCURRENCY)
    }

    fn with_concurrency(backend: HostObjectReadBackend, concurrency: usize) -> Self {
        Self {
            reader: AuthorizedObjectReader::new(backend),
            permits: Arc::new(Semaphore::new(concurrency.max(1))),
        }
    }

    async fn open(
        &self,
        request: ObjectReadRequest,
    ) -> Result<ValidatedObjectRead<Body>, ObjectReadError> {
        let permit = Arc::clone(&self.permits)
            .acquire_owned()
            .await
            .map_err(|_| {
                ObjectReadError::Backend(ObjectReadBackendError::Unavailable(
                    x_img_core::object_read::ObjectUnavailable::Unavailable,
                ))
            })?;
        let mut reader = self.reader.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            reader.open(&request)
        })
        .await
        .map_err(|_| {
            ObjectReadError::Backend(ObjectReadBackendError::Unavailable(
                x_img_core::object_read::ObjectUnavailable::Unavailable,
            ))
        })?
    }
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

/// Serves persistent gallery metadata and the reviewed capture-plan boundary.
pub async fn serve_monolith_with_gallery_web_and_capture(
    listener: tokio::net::TcpListener,
    dasobjectstore_ready: bool,
    monas_dispatch: Option<MonasDispatchVerifier>,
    gallery: GalleryCatalogue,
    web_root: Option<PathBuf>,
    capture_plans: CapturePlanService,
) -> io::Result<()> {
    axum::serve(
        listener,
        monolith_router_with_gallery_web_and_capture_authority(
            dasobjectstore_ready,
            monas_dispatch,
            gallery,
            web_root,
            capture_plans,
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
    serve_monolith_with_gallery_web_delivery_and_capture(
        listener,
        dasobjectstore_ready,
        monas_dispatch,
        gallery,
        web_root,
        backend,
        None,
    )
    .await
}

/// Serves the complete gallery plus the reviewed Firefox capture-plan boundary.
pub async fn serve_monolith_with_gallery_web_delivery_and_capture(
    listener: tokio::net::TcpListener,
    dasobjectstore_ready: bool,
    monas_dispatch: Option<MonasDispatchVerifier>,
    gallery: GalleryCatalogue,
    web_root: Option<PathBuf>,
    backend: HostObjectReadBackend,
    capture: Option<CapturePlanComposition>,
) -> io::Result<()> {
    axum::serve(
        listener,
        monolith_router_with_gallery_web_delivery_and_capture_authority(
            dasobjectstore_ready,
            monas_dispatch,
            gallery,
            web_root,
            backend,
            capture,
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
        Self {
            open: Arc::from(open),
        }
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

/// Mounts reviewed Firefox packages on the same direct application listener.
pub fn with_firefox_downloads(router: Router, downloads_root: PathBuf) -> Router {
    router.nest_service("/downloads", ServeDir::new(downloads_root))
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
        .route(
            "/products/pinakotheke/api/gallery/v1/folders",
            get(gallery_folders),
        )
        .layer(Extension(Arc::new(Mutex::new(gallery))));
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

/// Returns the Monas-admitted gallery plus capture planning without delivery.
pub fn monolith_router_with_gallery_web_and_capture_authority(
    dasobjectstore_ready: bool,
    monas_dispatch: Option<MonasDispatchVerifier>,
    gallery: GalleryCatalogue,
    web_root: Option<PathBuf>,
    capture_plans: CapturePlanService,
) -> Router {
    let authentication_ready = monas_dispatch.is_some();
    let mut protected = Router::new()
        .route("/products/pinakotheke/api/context", get(context))
        .route(
            "/products/pinakotheke/api/gallery/v1/catalogue",
            get(gallery_catalogue),
        )
        .route(
            "/products/pinakotheke/api/gallery/v1/folders",
            get(gallery_folders),
        )
        .route(
            "/products/pinakotheke/api/extension/v1/capture-plans",
            get(capture_plans_pending).post(capture_plan),
        )
        .layer(Extension(Arc::new(Mutex::new(gallery))))
        .layer(Extension(Arc::new(Mutex::new(capture_plans))));
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
    monolith_router_with_gallery_web_delivery_and_capture_authority(
        dasobjectstore_ready,
        monas_dispatch,
        gallery,
        web_root,
        backend,
        None,
    )
}

/// Returns the complete Monas-admitted gallery and capture-plan boundary.
pub fn monolith_router_with_gallery_web_delivery_and_capture_authority(
    dasobjectstore_ready: bool,
    monas_dispatch: Option<MonasDispatchVerifier>,
    gallery: GalleryCatalogue,
    web_root: Option<PathBuf>,
    backend: HostObjectReadBackend,
    capture: Option<CapturePlanComposition>,
) -> Router {
    let authentication_ready = monas_dispatch.is_some();
    let gallery = Arc::new(Mutex::new(gallery));
    let mut protected = Router::new()
        .route("/products/pinakotheke/api/context", get(context))
        .route(
            "/products/pinakotheke/api/gallery/v1/catalogue",
            get(gallery_catalogue),
        )
        .route(
            "/products/pinakotheke/api/gallery/v1/folders",
            get(gallery_folders),
        )
        .route(
            "/products/pinakotheke/api/gallery/v1/objects/{catalogue_id}/{role}",
            get(deliver_gallery_image),
        )
        .route(
            "/products/pinakotheke/api/gallery/v1/objects/{catalogue_id}/video",
            get(deliver_gallery_video),
        )
        .layer(Extension(Arc::clone(&gallery)))
        .layer(Extension(Arc::new(ObjectDeliveryPool::new(backend))));
    if let Some(capture) = capture {
        let CapturePlanComposition {
            plans: capture_plans,
            completion: completion_authority,
            acquire,
            onboarding,
            site_corpus,
            reviewed_destinations,
            destination_revalidator,
        } = capture;
        let capture_plans = Arc::new(Mutex::new(capture_plans));
        let reviewed_destinations = reviewed_destinations.map(|store| Arc::new(Mutex::new(store)));
        protected = protected
            .route(
                "/products/pinakotheke/api/extension/v1/capture-plans",
                get(capture_plans_pending).post(capture_plan),
            )
            .route(
                "/products/pinakotheke/api/extension/v1/cache-aliases/lookup",
                post(capture_alias_evidence).layer(Extension(Arc::clone(&gallery))),
            );
        if let Some(onboarding) = onboarding.clone() {
            protected = protected
                .route(
                    "/products/pinakotheke/api/extension/v1/onboarding",
                    get(extension_onboarding),
                )
                .layer(Extension(onboarding));
        }
        if let (Some(store), Some(authority)) = (reviewed_destinations.as_ref(), onboarding) {
            protected = protected
                .route(
                    "/products/pinakotheke/api/destinations/v1/reviewed",
                    get(get_reviewed_destination).put(put_reviewed_destination),
                )
                .layer(Extension(authority))
                .layer(Extension(Arc::clone(store)));
        }
        if let Some(store) = reviewed_destinations.as_ref() {
            // Capture admission consumes this same actor-scoped authority;
            // adding it independently of the settings route avoids any route
            // ordering or onboarding presentation becoming a fallback.
            protected = protected.layer(Extension(Arc::clone(store)));
        }
        if let Some(site_corpus) = site_corpus {
            protected = protected
                .route(
                    "/products/pinakotheke/api/extension/v1/site-corpus",
                    get(get_site_corpus).post(put_site_corpus),
                )
                .layer(Extension(Arc::new(Mutex::new(site_corpus))));
        }
        protected = protected.layer(Extension(Arc::clone(&capture_plans)));
        if let Some(authority) = completion_authority {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs());
            let recoverable = capture_plans
                .lock()
                .map(|plans| plans.recoverable_pending(now))
                .unwrap_or_default();
            let runtime = Arc::new(CaptureCompletionRuntime {
                authority,
                plans: capture_plans,
                gallery,
                acquire: acquire.map(Mutex::new),
                reviewed_destinations,
                destination_revalidator: destination_revalidator.map(Mutex::new),
                in_flight: Mutex::new(BTreeSet::new()),
            });
            for (actor_id, plan) in recoverable {
                let _ = schedule_capture_runtime(&runtime, actor_id, plan);
            }
            protected = protected
                .route(
                    "/products/pinakotheke/api/internal/v1/capture-plans/{plan_id}/complete",
                    post(complete_capture_plan),
                )
                .route(
                    "/products/pinakotheke/api/extension/v1/capture-plans/{plan_id}",
                    get(capture_plan_state),
                )
                .route(
                    "/products/pinakotheke/api/ingestion/v1/status",
                    get(ingestion_status),
                )
                .layer(Extension(runtime));
        }
    }
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

async fn get_site_corpus(
    Extension(context): Extension<AuthenticatedHostContext>,
    Extension(store): Extension<SiteCorpora>,
) -> Response {
    if !context.permits(XIMG_ACCESS) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match store
        .lock()
        .map_err(|_| ())
        .and_then(|store| store.get(context.actor_id()).map_err(|_| ()))
    {
        Ok(corpus) => Json(corpus).into_response(),
        Err(()) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn put_site_corpus(
    Extension(context): Extension<AuthenticatedHostContext>,
    Extension(store): Extension<SiteCorpora>,
    Json(request): Json<ReplaceSiteCorpus>,
) -> Response {
    if !context.permits(XIMG_ACCESS) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(store) = store.lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    match store.replace(context.actor_id(), request) {
        Ok(corpus) => Json(corpus).into_response(),
        Err(SiteCorpusError::Conflict(current)) => {
            (StatusCode::CONFLICT, Json(current)).into_response()
        }
        Err(
            SiteCorpusError::Invalid
            | SiteCorpusError::UnsupportedSchema
            | SiteCorpusError::TooLarge,
        ) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        Err(SiteCorpusError::Io(_) | SiteCorpusError::Json(_)) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn get_reviewed_destination(
    Extension(context): Extension<AuthenticatedHostContext>,
    Extension(store): Extension<ReviewedDestinations>,
) -> Response {
    if !context.permits(XIMG_ACCESS) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(store) = store.lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    match store.get(context.actor_id()) {
        Ok(selection) => Json(selection).into_response(),
        Err(ReviewedDestinationError::NotSelected) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn put_reviewed_destination(
    Extension(context): Extension<AuthenticatedHostContext>,
    Extension(store): Extension<ReviewedDestinations>,
    Extension(authority): Extension<ExtensionOnboardingAuthority>,
    Json(request): Json<ReplaceReviewedDestination>,
) -> Response {
    if !context.permits(XIMG_ACCESS) {
        return StatusCode::FORBIDDEN.into_response();
    }
    // This first persistence slice accepts only the exact destination already
    // reviewed by the host capture authority. Browser dashboard rows are
    // presentation data and never grant storage authority.
    if request.endpoint_id != authority.endpoint_id
        || request.object_store_id != authority.object_store_id
    {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    }
    let Ok(store) = store.lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    match store.replace(context.actor_id(), request) {
        Ok(selection) => Json(selection).into_response(),
        Err(ReviewedDestinationError::Conflict(current)) => {
            (StatusCode::CONFLICT, Json(current)).into_response()
        }
        Err(
            ReviewedDestinationError::Invalid
            | ReviewedDestinationError::UnsupportedSchema
            | ReviewedDestinationError::TooLarge
            | ReviewedDestinationError::NotSelected,
        ) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        Err(ReviewedDestinationError::Io(_) | ReviewedDestinationError::Json(_)) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn extension_onboarding(
    Extension(context): Extension<AuthenticatedHostContext>,
    Extension(plans): Extension<CapturePlans>,
    Extension(authority): Extension<ExtensionOnboardingAuthority>,
) -> Result<Json<ExtensionOnboardingResponse>, StatusCode> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let pairing_reference = plans
        .lock()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .active_pairing_id(context.actor_id(), now)
        .ok_or(StatusCode::FORBIDDEN)?;
    Ok(Json(ExtensionOnboardingResponse {
        schema_version: "pinakotheke.extension-onboarding.v1",
        instance_id: authority.instance_id,
        pairing_reference,
        dasobjectstore_status: "Ready",
        endpoint_id: authority.endpoint_id,
        object_store_id: authority.object_store_id,
        extension_download_path: authority.download_path,
    }))
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
        status: if dasobjectstore_ready && authentication_ready {
            "ready"
        } else {
            "not_ready"
        },
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
        .route(
            "/api/extension/v1/capture-plans",
            get(capture_plans_pending).post(capture_plan),
        )
        .route(
            "/api/extension/v1/cache-aliases/lookup",
            post(cache_alias_lookup),
        )
        .route("/api/playback/v1/{playback_id}", get(deliver_playback))
        .layer(Extension(Arc::new(Mutex::new(capture_plans))))
}

/// Returns a host composition with authenticated redacted operational details.
/// Public health remains a coarse process-liveness response.
pub fn router_with_operations(operations: Arc<Mutex<OperationalTelemetry>>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/context", get(context))
        .route("/api/operations/v1/snapshot", get(operations_snapshot))
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
        .route("/api/gallery/v1/folders", get(gallery_folders))
        .route(
            "/products/pinakotheke/api/gallery/v1/catalogue",
            get(gallery_catalogue),
        )
        .route(
            "/products/pinakotheke/api/gallery/v1/folders",
            get(gallery_folders),
        )
        .layer(Extension(Arc::new(Mutex::new(catalogue))))
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
            "/products/pinakotheke/api/gallery/v1/folders",
            get(gallery_folders),
        )
        .route(
            "/products/pinakotheke/api/gallery/v1/objects/{catalogue_id}/{role}",
            get(deliver_gallery_image),
        )
        .route(
            "/products/pinakotheke/api/gallery/v1/objects/{catalogue_id}/video",
            get(deliver_gallery_video),
        )
        .layer(Extension(Arc::new(Mutex::new(catalogue))))
        .layer(Extension(Arc::new(ObjectDeliveryPool::new(backend))))
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
    object_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GalleryFoldersQuery {
    prefix: Option<String>,
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
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
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
                object_prefix: query.object_prefix,
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

async fn gallery_folders(
    context: Option<Extension<AuthenticatedHostContext>>,
    catalogue: Option<Extension<MonasGalleryCatalogue>>,
    Query(query): Query<GalleryFoldersQuery>,
) -> Result<Json<GalleryFolderPage>, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    let catalogue = catalogue.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    catalogue
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .folder_page(&context, query.prefix.as_deref())
        .map(Json)
        .map_err(|error| match error {
            GalleryCatalogueError::Unauthorized => StatusCode::FORBIDDEN,
            GalleryCatalogueError::InvalidFilter | GalleryCatalogueError::InvalidPageSize => {
                StatusCode::BAD_REQUEST
            }
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
    let catalogue = catalogue.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let grant = catalogue
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
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
    match delivery
        .open(request)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
    {
        ValidatedObjectRead::NotModified { etag } => Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, etag)
            .header(
                header::CACHE_CONTROL,
                "private, max-age=3600, must-revalidate",
            )
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
                .header(
                    header::CACHE_CONTROL,
                    "private, max-age=3600, must-revalidate",
                )
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
    let catalogue = catalogue.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let grant = catalogue
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
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
        delivery
            .open(request)
            .await
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
        .layer(Extension(Arc::new(Mutex::new(cache_aliases))))
        .layer(Extension(Arc::new(ObjectDeliveryPool::new(backend))))
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
    capture_plans: Option<Extension<CapturePlans>>,
    runtime: Option<Extension<Arc<CaptureCompletionRuntime>>>,
    reviewed_destinations: Option<Extension<ReviewedDestinations>>,
    context: Option<Extension<AuthenticatedHostContext>>,
    Json(request): Json<CapturePlanRequest>,
) -> Result<Json<CapturePlan>, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    if !context.permits(XIMG_ACCESS) {
        return Err(StatusCode::FORBIDDEN);
    }
    let capture_plans = capture_plans.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_secs();
    let destination = if let Some(store) = reviewed_destinations {
        let selection = store
            .0
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .get(context.actor_id())
            .map_err(|error| match error {
                ReviewedDestinationError::NotSelected => StatusCode::CONFLICT,
                ReviewedDestinationError::Invalid
                | ReviewedDestinationError::UnsupportedSchema
                | ReviewedDestinationError::TooLarge
                | ReviewedDestinationError::Io(_)
                | ReviewedDestinationError::Json(_) => StatusCode::SERVICE_UNAVAILABLE,
                ReviewedDestinationError::Conflict(_) => StatusCode::CONFLICT,
            })?;
        Some(CaptureDestinationSnapshot {
            endpoint_id: selection.endpoint_id,
            object_store_id: selection.object_store_id,
            selection_revision: selection.revision,
        })
    } else if runtime.is_some() {
        // A production capture worker without the actor-scoped destination
        // authority must fail closed. The planning-only contract router may
        // remain unbound and can never execute a helper.
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    } else {
        None
    };
    let mut capture_plans = capture_plans
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let plan = match destination {
        Some(destination) => {
            capture_plans.plan_with_destination(context.actor_id(), now, request, destination)
        }
        None => capture_plans.plan(context.actor_id(), now, request),
    }
    .map_err(capture_plan_status)?;
    eprintln!(
        "pinakotheke_ingress event=plan_admitted plan_id={} kind={:?} origin={} site_id={}",
        plan.plan_id, plan.capture_kind, plan.origin, plan.site_id
    );
    drop(capture_plans);
    if let Some(runtime) = runtime.map(|runtime| runtime.0) {
        schedule_capture_runtime(&runtime, context.actor_id().to_owned(), plan.clone())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(Json(plan))
}

async fn capture_plan_state(
    Path(plan_id): Path<String>,
    Extension(context): Extension<AuthenticatedHostContext>,
    Extension(runtime): Extension<Arc<CaptureCompletionRuntime>>,
) -> Result<Json<CapturePlanStatusResponse>, StatusCode> {
    if !context.permits(XIMG_ACCESS) {
        return Err(StatusCode::FORBIDDEN);
    }
    let plans = runtime
        .plans
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let plan = plans
        .pending(context.actor_id(), &plan_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    let settled = plans.is_settled(context.actor_id(), &plan_id);
    drop(plans);
    let stored = settled
        && runtime
            .gallery
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .items()
            .iter()
            .any(|item| item.catalogue_id == plan.catalogue_id);
    Ok(Json(CapturePlanStatusResponse {
        schema_version: "pinakotheke.capture-plan-status.v1",
        plan_id,
        catalogue_id: plan.catalogue_id,
        state: if stored { "stored" } else { "pending" },
    }))
}

async fn ingestion_status(
    Extension(context): Extension<AuthenticatedHostContext>,
    Extension(runtime): Extension<Arc<CaptureCompletionRuntime>>,
) -> Result<Json<IngestionStatusResponse>, StatusCode> {
    if !context.permits(XIMG_ACCESS) {
        return Err(StatusCode::FORBIDDEN);
    }
    let activity = runtime
        .plans
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .activity_for_actor(context.actor_id());
    let gallery_items = runtime
        .gallery
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .items()
        .len();
    Ok(Json(IngestionStatusResponse {
        schema_version: "pinakotheke.ingestion-status.v1",
        observed_assets: activity.observed_thumbnails
            + activity.opened_originals
            + activity.opened_videos,
        observed_thumbnails: activity.observed_thumbnails,
        opened_originals: activity.opened_originals,
        opened_videos: activity.opened_videos,
        pending: activity.pending,
        stored: activity.stored,
        gallery_items,
        last_observed_at_epoch_seconds: activity.last_observed_at_epoch_seconds,
    }))
}

fn schedule_capture_runtime(
    runtime: &Arc<CaptureCompletionRuntime>,
    actor_id: String,
    plan: CapturePlan,
) -> Result<bool, ()> {
    if runtime.acquire.is_none() {
        return Ok(false);
    }
    let key = format!("{actor_id}:{}", plan.plan_id);
    if !runtime
        .in_flight
        .lock()
        .map_err(|_| ())?
        .insert(key.clone())
    {
        return Ok(false);
    }
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        runtime.in_flight.lock().map_err(|_| ())?.remove(&key);
        return Ok(false);
    };
    let runtime = Arc::clone(runtime);
    handle.spawn_blocking(move || {
        let acquired = acquire_capture_with_retry(&runtime, &actor_id, &plan);
        match acquired {
            Ok(evidence) => match settle_capture_runtime(&runtime, &actor_id, evidence) {
                Ok(_) => eprintln!(
                    "pinakotheke_ingress event=gallery_admitted plan_id={} kind={:?}",
                    plan.plan_id, plan.capture_kind
                ),
                Err(error) => eprintln!(
                    "pinakotheke_ingress event=settlement_failed plan_id={} kind={:?} reason={error:?}",
                    plan.plan_id, plan.capture_kind
                ),
            },
            Err(reason) => eprintln!(
                "pinakotheke_ingress event=acquisition_failed plan_id={} kind={:?} reason={}",
                plan.plan_id, plan.capture_kind, reason
            ),
        }
        if let Ok(mut in_flight) = runtime.in_flight.lock() {
            in_flight.remove(&key);
        }
    });
    Ok(true)
}

fn acquire_capture_with_retry(
    runtime: &CaptureCompletionRuntime,
    actor_id: &str,
    plan: &CapturePlan,
) -> Result<VerifiedCaptureCompletion, String> {
    const MAX_ATTEMPTS: usize = 3;
    for attempt in 0..MAX_ATTEMPTS {
        revalidate_capture_destination(runtime, actor_id, plan)?;
        let result = runtime
            .acquire
            .as_ref()
            .ok_or_else(|| String::from("acquire backend unavailable"))?
            .lock()
            .map_err(|_| String::from("acquire backend lock poisoned"))?
            .acquire(actor_id, plan);
        match result {
            Ok(evidence) => return Ok(evidence),
            Err(reason) if transient_capture_failure(&reason) && attempt + 1 < MAX_ATTEMPTS => {
                std::thread::sleep(std::time::Duration::from_millis(200 * (attempt as u64 + 1)));
            }
            Err(reason) => return Err(reason),
        }
    }
    unreachable!("bounded retry loop always returns")
}

fn transient_capture_failure(reason: &str) -> bool {
    reason.ends_with(": object_upload")
        || reason.ends_with(": daemon_verification")
        || reason.contains("temporarily unavailable")
}

fn revalidate_capture_destination(
    runtime: &CaptureCompletionRuntime,
    actor_id: &str,
    plan: &CapturePlan,
) -> Result<(), String> {
    let snapshot = plan
        .destination
        .as_ref()
        .ok_or_else(|| String::from("capture plan has no reviewed destination"))?;
    let current = runtime
        .reviewed_destinations
        .as_ref()
        .ok_or_else(|| String::from("reviewed destination authority unavailable"))?
        .lock()
        .map_err(|_| String::from("reviewed destination authority unavailable"))?
        .get(actor_id)
        .map_err(|_| String::from("reviewed destination is unavailable"))?;
    if current.revision != snapshot.selection_revision
        || current.endpoint_id != snapshot.endpoint_id
        || current.object_store_id != snapshot.object_store_id
    {
        return Err(String::from("reviewed destination changed after admission"));
    }
    let state = runtime
        .destination_revalidator
        .as_ref()
        .ok_or_else(|| String::from("live destination revalidator unavailable"))?
        .lock()
        .map_err(|_| String::from("live destination revalidator unavailable"))?
        .revalidate(actor_id, snapshot, plan.capture_kind)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| String::from("system time unavailable"))?
        .as_secs();
    validate_capture_destination_authority(snapshot, &state, now)
}

fn validate_capture_destination_authority(
    snapshot: &CaptureDestinationSnapshot,
    state: &CaptureDestinationAuthorityState,
    now_epoch_seconds: u64,
) -> Result<(), String> {
    if !state.endpoint_present
        || !state.object_store_present
        || state.endpoint_id != snapshot.endpoint_id
        || state.object_store_id != snapshot.object_store_id
        || !state.tls_trusted
        || !state.paired
        || now_epoch_seconds >= state.pairing_expires_at_epoch_seconds
        || !state.ready
        || !state.writable
        || state.quota_available_bytes == 0
    {
        return Err(String::from("live destination authority rejected capture"));
    }
    Ok(())
}

async fn capture_plans_pending(
    capture_plans: Option<Extension<CapturePlans>>,
    context: Option<Extension<AuthenticatedHostContext>>,
) -> Result<Json<PendingCapturePlansResponse>, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    if !context.permits(XIMG_ACCESS) {
        return Err(StatusCode::FORBIDDEN);
    }
    let capture_plans = capture_plans.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let plans = capture_plans
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .pending_for_actor(context.actor_id());
    Ok(Json(PendingCapturePlansResponse {
        schema_version: "pinakotheke.pending-capture-plans.v1",
        plans,
    }))
}

async fn complete_capture_plan(
    Path(plan_id): Path<String>,
    headers: HeaderMap,
    context: Option<Extension<AuthenticatedHostContext>>,
    runtime: Option<Extension<Arc<CaptureCompletionRuntime>>>,
    Json(request): Json<CaptureCompletionRequest>,
) -> Result<Json<CaptureCompletionResponse>, StatusCode> {
    let context = context.ok_or(StatusCode::UNAUTHORIZED)?.0;
    if !context.permits(XIMG_ACCESS) {
        return Err(StatusCode::FORBIDDEN);
    }
    let runtime = runtime.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let supplied = headers
        .get("x-pinakotheke-capture-worker-token")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if !constant_time_equal(supplied.as_bytes(), runtime.authority.token.as_bytes()) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if request.schema_version != "pinakotheke.capture-completion.v1" {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let pending = runtime
        .plans
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .pending(context.actor_id(), &plan_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    let destination = pending.destination.as_ref().ok_or(StatusCode::CONFLICT)?;
    if request.endpoint_id != destination.endpoint_id
        || request.object_store_id != destination.object_store_id
    {
        return Err(StatusCode::CONFLICT);
    }
    revalidate_capture_destination(&runtime, context.actor_id(), &pending)
        .map_err(|_| StatusCode::CONFLICT)?;
    let outcome = settle_capture_runtime(
        &runtime,
        context.actor_id(),
        VerifiedCaptureCompletion {
            plan_id,
            catalogue_id: request.catalogue_id,
            title: request.title,
            content_type: request.content_type,
            content_length: request.content_length,
            endpoint_id: request.endpoint_id,
            object_store_id: request.object_store_id,
            object_key: request.object_key,
            object_version: request.object_version,
            checksum_sha256: request.checksum_sha256,
            verified_at_epoch_seconds: request.verified_at_epoch_seconds,
            video: request.video,
        },
    )
    .map_err(|error| match error {
        CaptureCompletionError::UnknownPlan => StatusCode::NOT_FOUND,
        CaptureCompletionError::InvalidEvidence
        | CaptureCompletionError::Acquisition(_)
        | CaptureCompletionError::Reconciliation(_)
        | CaptureCompletionError::ReconciliationConflict
        | CaptureCompletionError::Gallery(_) => StatusCode::UNPROCESSABLE_ENTITY,
        CaptureCompletionError::Journal(_) => StatusCode::SERVICE_UNAVAILABLE,
    })?;
    Ok(Json(CaptureCompletionResponse {
        schema_version: "pinakotheke.capture-completion-result.v1",
        outcome: match outcome {
            CaptureCompletionOutcome::ThumbnailInserted => "thumbnail_inserted",
            CaptureCompletionOutcome::OriginalAttached => "original_attached",
            CaptureCompletionOutcome::VideoInserted => "video_inserted",
            CaptureCompletionOutcome::AlreadyPresent => "already_present",
        },
    }))
}

fn settle_capture_runtime(
    runtime: &CaptureCompletionRuntime,
    actor_id: &str,
    evidence: VerifiedCaptureCompletion,
) -> Result<CaptureCompletionOutcome, CaptureCompletionError> {
    let mut plans = runtime
        .plans
        .lock()
        .map_err(|_| CaptureCompletionError::InvalidEvidence)?;
    let plan = plans
        .pending(actor_id, &evidence.plan_id)
        .ok_or(CaptureCompletionError::UnknownPlan)?;
    let destination = plan
        .destination
        .as_ref()
        .ok_or(CaptureCompletionError::InvalidEvidence)?;
    if evidence.endpoint_id != destination.endpoint_id
        || evidence.object_store_id != destination.object_store_id
    {
        return Err(CaptureCompletionError::InvalidEvidence);
    }
    let outcome = complete_verified_image(
        &mut plans,
        runtime.authority.gallery_store.clone(),
        actor_id,
        evidence,
    )?;
    drop(plans);
    let refreshed = runtime
        .authority
        .gallery_store
        .load_or_empty()
        .map_err(|_| CaptureCompletionError::InvalidEvidence)?;
    *runtime
        .gallery
        .lock()
        .map_err(|_| CaptureCompletionError::InvalidEvidence)? = refreshed;
    Ok(outcome)
}

fn capture_plan_status(error: CapturePlanError) -> StatusCode {
    match error {
        CapturePlanError::PairingActorMismatch
        | CapturePlanError::UnknownPairing
        | CapturePlanError::PairingExpired
        | CapturePlanError::PairingRevoked => StatusCode::FORBIDDEN,
        CapturePlanError::Scheduler | CapturePlanError::Persistence => {
            StatusCode::SERVICE_UNAVAILABLE
        }
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

async fn capture_alias_evidence(
    Extension(context): Extension<AuthenticatedHostContext>,
    Extension(capture_plans): Extension<CapturePlans>,
    Extension(gallery): Extension<MonasGalleryCatalogue>,
    Json(envelope): Json<CacheAliasLookupEnvelope>,
) -> Result<Json<CacheAliasLookupResponse>, StatusCode> {
    if !context.permits(XIMG_ACCESS) || envelope.schema_version != CACHE_LOOKUP_SCHEMA_VERSION {
        return Err(StatusCode::FORBIDDEN);
    }
    if envelope.adapter_id.is_empty() || envelope.adapter_id.len() > 128 {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    if envelope
        .canonical_presentation
        .as_deref()
        .is_some_and(|value| {
            value.len() > 2_048
                || !value.starts_with("https://")
                || value.contains([' ', '\n', '\r', '@', '?', '#'])
        })
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_secs();
    let plan = capture_plans
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .settled_for_alias_or_presentation(
            context.actor_id(),
            &envelope.pairing_id,
            now,
            &envelope.origin,
            &envelope.adapter_version,
            &envelope.canonical_alias,
            envelope.canonical_presentation.as_deref(),
        );
    let Some(plan) = plan else {
        eprintln!("pinakotheke_cache_evidence outcome=miss stage=settled_alias");
        return Ok(Json(cache_fallback("miss", None)));
    };
    let gallery = gallery
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(item) = gallery
        .items()
        .iter()
        .find(|item| item.catalogue_id == plan.catalogue_id)
    else {
        eprintln!("pinakotheke_cache_evidence outcome=miss stage=gallery");
        return Ok(Json(cache_fallback("miss", None)));
    };
    let representation = match plan.capture_kind {
        CaptureKind::ObservedThumbnail => Some(&item.thumbnail),
        CaptureKind::ExplicitOriginal => item.preview.as_ref(),
        CaptureKind::ExplicitVideo => item.preview.as_ref().or(Some(&item.thumbnail)),
    };
    let Some(representation) = representation else {
        eprintln!("pinakotheke_cache_evidence outcome=miss stage=representation");
        return Ok(Json(cache_fallback("miss", None)));
    };
    eprintln!("pinakotheke_cache_evidence outcome=hit stage=ready");
    Ok(Json(CacheAliasLookupResponse {
        schema_version: CACHE_RESULT_SCHEMA_VERSION,
        outcome: "hit",
        reason: None,
        media_class: Some(match plan.capture_kind {
            CaptureKind::ObservedThumbnail => "thumbnail_image",
            CaptureKind::ExplicitOriginal => "original_image",
            CaptureKind::ExplicitVideo => "normalized_mp4",
        }),
        content_type: Some(representation.content_type.clone()),
        content_length: Some(representation.content_length),
        object_checksum: Some(representation.checksum.clone()),
        delivery_path: representation.delivery_path.clone(),
    }))
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
        delivery
            .open(request)
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?
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
    match image_delivery
        .open(request)
        .await
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
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn object_delivery_pool_runs_independent_reads_concurrently() {
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let active_for_open = Arc::clone(&active);
        let maximum_for_open = Arc::clone(&maximum);
        let backend = HostObjectReadBackend::new(Box::new(move |request| {
            let current = active_for_open.fetch_add(1, Ordering::SeqCst) + 1;
            maximum_for_open.fetch_max(current, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(75));
            active_for_open.fetch_sub(1, Ordering::SeqCst);
            Ok(ObjectReadResult::Content {
                metadata: ObjectContentMetadata {
                    content_type: "image/jpeg".into(),
                    content_length: 1,
                    total_length: 1,
                    checksum: request.object.checksum.clone(),
                    etag: format!("\"{}\"", request.object.checksum),
                    content_range: None,
                },
                stream: Body::from(vec![1_u8]),
            })
        }));
        let pool = Arc::new(ObjectDeliveryPool::with_concurrency(backend, 8));
        let request = ObjectReadRequest {
            object: AuthorizedObjectReference {
                endpoint_id: "endpoint-1".into(),
                object_store_id: "store-1".into(),
                object_key: "objects/image.jpg".into(),
                object_version: 1,
                checksum: CHECKSUM.into(),
            },
            range: None,
            if_none_match_etag: None,
        };
        let reads = (0..8)
            .map(|_| {
                let pool = Arc::clone(&pool);
                let request = request.clone();
                tokio::spawn(async move { pool.open(request).await })
            })
            .collect::<Vec<_>>();
        for read in reads {
            read.await.unwrap().unwrap();
        }
        assert!(maximum.load(Ordering::SeqCst) >= 4);
    }

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
            GalleryCatalogue, GalleryCatalogueStore, GalleryItem, GalleryMediaKind,
            GalleryObjectAvailability, GalleryRepresentation, GalleryRepresentationKind,
            GalleryReviewState, GallerySourceKind,
        },
        host_context::{HostContextAdapter, MonasHostContextAdapter},
        object_read::{
            AuthorizedObjectReader, AuthorizedObjectReference, ObjectContentMetadata,
            ObjectReadRequest, ObjectReadResult,
        },
        operations::{Component, EventCode, EventOutcome, HealthState, OperationalTelemetry},
        playback_delivery::{DirectPlaybackGrant, DirectPlaybackService},
        reviewed_destination::{
            AuthoritySelectionSeed, ReplaceReviewedDestination, ReviewedDestinationStore,
        },
        synoptikon_catalogue::{
            CatalogueMediaKind, CatalogueReviewState, SynoptikonCatalogueItem,
            SynoptikonCatalogueProjection,
        },
        video_profile::NormalizedVideoState,
        viewed_media::{
            AdapterKind, CAPTURE_REQUEST_SCHEMA_VERSION, CaptureDestinationSnapshot, CaptureKind,
            CapturePairing, CapturePlanRequest, CapturePlanService, SiteCapturePolicy,
        },
    };

    use super::{
        CaptureCompletionAuthority, CaptureCompletionRuntime, CaptureDestinationAuthorityState,
        CapturePlanComposition, ExtensionOnboardingAuthority, HostCaptureAcquireBackend,
        HostCaptureDestinationRevalidateBackend, HostObjectReadBackend, MonasDispatchVerifier,
        ObjectDeliveryPool, VerifiedCaptureCompletion, monolith_router,
        monolith_router_with_authorities, monolith_router_with_gallery_authority,
        monolith_router_with_gallery_delivery_authority,
        monolith_router_with_gallery_web_delivery_and_capture_authority,
        monolith_router_with_gallery_web_delivery_authority, monolith_router_with_storage,
        revalidate_capture_destination, router, router_with_cache_aliases,
        router_with_cache_substitution, router_with_capture_plans, router_with_direct_playback,
        router_with_gallery_catalogue, router_with_gallery_delivery,
        router_with_image_substitution, router_with_operations, router_with_synoptikon_catalogue,
        validate_capture_destination_authority,
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
            source_page_url: Some("https://example.invalid/gallery".into()),
            source_kind: GallerySourceKind::Website,
            media_kind: GalleryMediaKind::Image,
            review_state: GalleryReviewState::New,
            discovered_at_epoch_seconds: 1,
            width: 320,
            height: 200,
            video: None,
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
            source_page_url: Some("https://example.invalid/video".into()),
            source_kind: GallerySourceKind::Website,
            media_kind: GalleryMediaKind::NormalizedVideo,
            review_state: GalleryReviewState::New,
            discovered_at_epoch_seconds: 1,
            width: 1920,
            height: 1080,
            video: Some(x_img_core::gallery_catalogue::GalleryVideoMetadata {
                duration_millis: 12_345,
                video_codec: "h264".into(),
                audio_codec: "aac".into(),
                profile_id: "pinakotheke-video-mp4-v1".into(),
                normalization_state: "ready".into(),
                firefox_playback_evidence_id: "firefox-140".into(),
            }),
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

    fn with_ready_capture_destination(
        composition: CapturePlanComposition,
        path: impl Into<std::path::PathBuf>,
    ) -> CapturePlanComposition {
        let store = ReviewedDestinationStore::new(path);
        store
            .seed_from_authority_if_absent(
                "synthetic-monas-user",
                &AuthoritySelectionSeed {
                    endpoint_id: "synthetic-endpoint".into(),
                    object_store_id: "synthetic-store".into(),
                },
            )
            .unwrap();
        composition
            .with_reviewed_destinations(store)
            .with_destination_revalidator(HostCaptureDestinationRevalidateBackend::new(Box::new(
                |_, snapshot, _| {
                    Ok(CaptureDestinationAuthorityState {
                        endpoint_id: snapshot.endpoint_id.clone(),
                        object_store_id: snapshot.object_store_id.clone(),
                        endpoint_present: true,
                        object_store_present: true,
                        tls_trusted: true,
                        paired: true,
                        pairing_expires_at_epoch_seconds: u64::MAX,
                        ready: true,
                        writable: true,
                        quota_available_bytes: 1024,
                    })
                },
            )))
    }

    fn ready_destination_state() -> CaptureDestinationAuthorityState {
        CaptureDestinationAuthorityState {
            endpoint_id: "synthetic-endpoint".into(),
            object_store_id: "synthetic-store".into(),
            endpoint_present: true,
            object_store_present: true,
            tls_trusted: true,
            paired: true,
            pairing_expires_at_epoch_seconds: u64::MAX,
            ready: true,
            writable: true,
            quota_available_bytes: 1024,
        }
    }

    #[test]
    fn live_destination_authority_rejects_every_write_gate_without_fallback() {
        let snapshot = CaptureDestinationSnapshot {
            endpoint_id: "synthetic-endpoint".into(),
            object_store_id: "synthetic-store".into(),
            selection_revision: 1,
        };
        assert!(
            validate_capture_destination_authority(&snapshot, &ready_destination_state(), 1)
                .is_ok()
        );
        let rejected = [
            CaptureDestinationAuthorityState {
                endpoint_present: false,
                ..ready_destination_state()
            },
            CaptureDestinationAuthorityState {
                object_store_present: false,
                ..ready_destination_state()
            },
            CaptureDestinationAuthorityState {
                endpoint_id: "fallback-endpoint".into(),
                ..ready_destination_state()
            },
            CaptureDestinationAuthorityState {
                object_store_id: "fallback-store".into(),
                ..ready_destination_state()
            },
            CaptureDestinationAuthorityState {
                tls_trusted: false,
                ..ready_destination_state()
            },
            CaptureDestinationAuthorityState {
                paired: false,
                ..ready_destination_state()
            },
            CaptureDestinationAuthorityState {
                pairing_expires_at_epoch_seconds: 1,
                ..ready_destination_state()
            },
            CaptureDestinationAuthorityState {
                ready: false,
                ..ready_destination_state()
            },
            CaptureDestinationAuthorityState {
                writable: false,
                ..ready_destination_state()
            },
            CaptureDestinationAuthorityState {
                quota_available_bytes: 0,
                ..ready_destination_state()
            },
        ];
        for state in rejected {
            assert!(validate_capture_destination_authority(&snapshot, &state, 1).is_err());
        }
    }

    #[test]
    fn worker_revalidation_rejects_selection_revision_change_and_missing_adapter() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-api-destination-revalidation-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let store = ReviewedDestinationStore::new(root.join("destinations.json"));
        let selected = store
            .seed_from_authority_if_absent(
                "synthetic-monas-user",
                &AuthoritySelectionSeed {
                    endpoint_id: "synthetic-endpoint".into(),
                    object_store_id: "synthetic-store".into(),
                },
            )
            .unwrap();
        let snapshot = CaptureDestinationSnapshot {
            endpoint_id: selected.endpoint_id,
            object_store_id: selected.object_store_id,
            selection_revision: selected.revision,
        };
        let mut plans = capture_plans();
        let request = CapturePlanRequest {
            schema_version: CAPTURE_REQUEST_SCHEMA_VERSION.into(),
            pairing_id: "pair-0".into(),
            origin: "https://example.invalid".into(),
            page_url: "https://example.invalid/gallery".into(),
            adapter_kind: AdapterKind::ExperimentalGeneric,
            adapter_version: "1.0.0".into(),
            capture_kind: CaptureKind::ObservedThumbnail,
            media_url: "https://example.invalid/thumbnail.webp".into(),
            presentation_url: None,
            width: 320,
            height: 200,
        };
        let plan = plans
            .plan_with_destination("synthetic-monas-user", 1, request, snapshot)
            .unwrap();
        let stores = Arc::new(Mutex::new(store.clone()));
        let runtime = CaptureCompletionRuntime {
            authority: CaptureCompletionAuthority::new(
                "synthetic-capture-worker-token-0001".into(),
                GalleryCatalogueStore::new(root.join("gallery.json")),
                "synthetic-endpoint".into(),
                "synthetic-store".into(),
            )
            .unwrap(),
            plans: Arc::new(Mutex::new(plans)),
            gallery: Arc::new(Mutex::new(GalleryCatalogue::default())),
            acquire: None,
            reviewed_destinations: Some(Arc::clone(&stores)),
            destination_revalidator: None,
            in_flight: Mutex::new(std::collections::BTreeSet::new()),
        };
        assert!(revalidate_capture_destination(&runtime, "synthetic-monas-user", &plan).is_err());
        store
            .replace(
                "synthetic-monas-user",
                ReplaceReviewedDestination {
                    schema_version: x_img_core::reviewed_destination::REVIEWED_DESTINATION_SCHEMA
                        .into(),
                    expected_revision: 1,
                    endpoint_id: "synthetic-endpoint".into(),
                    object_store_id: "synthetic-store".into(),
                },
            )
            .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_callback = Arc::clone(&calls);
        let runtime = CaptureCompletionRuntime {
            destination_revalidator: Some(Mutex::new(
                HostCaptureDestinationRevalidateBackend::new(Box::new(move |_, _, _| {
                    calls_for_callback.fetch_add(1, Ordering::SeqCst);
                    Ok(ready_destination_state())
                })),
            )),
            ..runtime
        };
        assert!(revalidate_capture_destination(&runtime, "synthetic-monas-user", &plan).is_err());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "changed selection must fail before host/helper work"
        );
        assert!(revalidate_capture_destination(&runtime, "another-actor", &plan).is_err());
        let _ = std::fs::remove_dir_all(root);
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
                presentation_url: None,
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
        let folders = router_with_gallery_catalogue(gallery_catalogue())
            .layer(Extension(context))
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/gallery/v1/folders")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(folders.status(), StatusCode::OK);
        let body = to_bytes(folders.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["schema_version"], "pinakotheke.gallery-folders.v1");
        assert_eq!(json["folders"][0]["name"], "objects");
        assert_eq!(json["folders"][0]["item_count"], 1);

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
        assert_eq!(
            response.headers()["cache-control"],
            "private, max-age=3600, must-revalidate"
        );
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
    async fn monolith_reports_ready_when_storage_and_authentication_are_composed() {
        let response = monolith_router_with_authorities(
            true,
            Some(
                MonasDispatchVerifier::new("synthetic-monas-dispatch-token-ready-0001".into())
                    .unwrap(),
            ),
        )
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
        assert_eq!(json["status"], "ready");
        assert_eq!(json["components"][1]["status"], "Ready");
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
    async fn onboarding_requires_monas_and_returns_only_the_actors_ready_pairing() {
        let token = "synthetic-monas-dispatch-token-0001";
        let composition = CapturePlanComposition::new(capture_plans(), None).with_onboarding(
            ExtensionOnboardingAuthority::new(
                "pinakotheke-test".into(),
                "endpoint-1".into(),
                "store-1".into(),
                "/downloads/pinakotheke-1.3.0.xpi".into(),
            )
            .unwrap(),
        );
        let router = monolith_router_with_gallery_web_delivery_and_capture_authority(
            true,
            Some(MonasDispatchVerifier::new(token.into()).unwrap()),
            gallery_catalogue(),
            None,
            gallery_image_backend(),
            Some(composition),
        );
        let direct = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/extension/v1/onboarding")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(direct.status(), StatusCode::UNAUTHORIZED);
        let admitted = router
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/extension/v1/onboarding")
                    .header("x-monas-dispatch-token", token)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(admitted.status(), StatusCode::OK);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(admitted.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(body["pairing_reference"], "pair-0");
        assert_eq!(body["object_store_id"], "store-1");
        assert!(body.get("actor_id").is_none());
    }

    #[tokio::test]
    async fn reviewed_destination_is_actor_scoped_persistent_and_host_bounded() {
        let token = "synthetic-monas-dispatch-token-0001";
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-api-reviewed-destination-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let composition = CapturePlanComposition::new(capture_plans(), None)
            .with_onboarding(
                ExtensionOnboardingAuthority::new(
                    "pinakotheke-test".into(),
                    "endpoint-1".into(),
                    "store-1".into(),
                    "/downloads/pinakotheke-1.12.0.xpi".into(),
                )
                .unwrap(),
            )
            .with_reviewed_destinations(ReviewedDestinationStore::new(root.join("reviewed.json")));
        let app = monolith_router_with_gallery_web_delivery_and_capture_authority(
            true,
            Some(MonasDispatchVerifier::new(token.into()).unwrap()),
            gallery_catalogue(),
            None,
            gallery_image_backend(),
            Some(composition),
        );
        let path = "/products/pinakotheke/api/destinations/v1/reviewed";
        let missing = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("x-monas-dispatch-token", token)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        let saved = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(path)
                    .header("content-type", "application/json")
                    .header("x-monas-dispatch-token", token)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::from(r#"{"schema_version":"pinakotheke.reviewed-destination.v1","expected_revision":0,"endpoint_id":"endpoint-1","object_store_id":"store-1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(saved.status(), StatusCode::OK);
        let restored = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("x-monas-dispatch-token", token)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(restored.status(), StatusCode::OK);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(restored.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(body["revision"], 1);
        assert_eq!(body["object_store_id"], "store-1");
        let changed = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(path)
                    .header("content-type", "application/json")
                    .header("x-monas-dispatch-token", token)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::from(r#"{"schema_version":"pinakotheke.reviewed-destination.v1","expected_revision":1,"endpoint_id":"endpoint-1","object_store_id":"unreviewed-store"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(changed.status(), StatusCode::UNPROCESSABLE_ENTITY);
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn site_corpus_is_monas_scoped_persistent_and_conflict_safe() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-site-corpus-api-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let token = "synthetic-monas-dispatch-token-0001";
        let composition = CapturePlanComposition::new(capture_plans(), None).with_site_corpus(
            x_img_core::site_corpus::SiteCorpusStore::new(root.join("site-corpus.json")),
        );
        let app = monolith_router_with_gallery_web_delivery_and_capture_authority(
            true,
            Some(MonasDispatchVerifier::new(token.into()).unwrap()),
            gallery_catalogue(),
            None,
            gallery_image_backend(),
            Some(composition),
        );
        let direct = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/extension/v1/site-corpus")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(direct.status(), StatusCode::UNAUTHORIZED);
        let body = Body::from(
            r#"{"schema_version":"pinakotheke.site-corpus.v1","expected_revision":0,"rules":[{"origin":"https://x.com","images":true,"videos":true,"capture":true,"substitution":true,"x_ingress":true}]}"#,
        );
        let saved = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/products/pinakotheke/api/extension/v1/site-corpus")
                    .header("content-type", "application/json")
                    .header("x-monas-dispatch-token", token)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(saved.status(), StatusCode::OK);
        let saved_json: serde_json::Value =
            serde_json::from_slice(&to_bytes(saved.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(saved_json["revision"], 1);
        let stale = app.clone().oneshot(Request::builder().method("POST").uri("/products/pinakotheke/api/extension/v1/site-corpus").header("content-type", "application/json").header("x-monas-dispatch-token", token).header("x-monas-host-context", MONAS_CONTEXT).body(Body::from(r#"{"schema_version":"pinakotheke.site-corpus.v1","expected_revision":0,"rules":[]}"#)).unwrap()).await.unwrap();
        assert_eq!(stale.status(), StatusCode::CONFLICT);
        let current: serde_json::Value =
            serde_json::from_slice(&to_bytes(stale.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(current["revision"], 1);
        let reloaded = app
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/extension/v1/site-corpus")
                    .header("x-monas-dispatch-token", token)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let reloaded_json: serde_json::Value =
            serde_json::from_slice(&to_bytes(reloaded.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(reloaded_json["rules"][0]["origin"], "https://x.com");
        std::fs::remove_dir_all(root).unwrap();
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
        let admitted_router = router_with_capture_plans(capture_plans()).layer(Extension(context));
        let admitted = admitted_router
            .clone()
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
        let pending = admitted_router
            .oneshot(
                Request::builder()
                    .uri("/api/extension/v1/capture-plans")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pending.status(), StatusCode::OK);
        let pending_json: serde_json::Value =
            serde_json::from_slice(&to_bytes(pending.into_body(), 16 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(
            pending_json["schema_version"],
            "pinakotheke.pending-capture-plans.v1"
        );
        assert_eq!(pending_json["plans"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn production_monolith_mounts_capture_and_evidence_behind_monas_dispatch() {
        let token = "synthetic-monas-dispatch-token-0001";
        let router = || {
            monolith_router_with_gallery_web_delivery_and_capture_authority(
                true,
                Some(MonasDispatchVerifier::new(token.into()).unwrap()),
                GalleryCatalogue::default(),
                None,
                gallery_image_backend(),
                Some(CapturePlanComposition::new(capture_plans(), None)),
            )
        };
        let direct = router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/products/pinakotheke/api/extension/v1/capture-plans")
                    .header("content-type", "application/json")
                    .body(request_body())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(direct.status(), StatusCode::UNAUTHORIZED);
        let admitted = router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/products/pinakotheke/api/extension/v1/capture-plans")
                    .header("content-type", "application/json")
                    .header("x-monas-dispatch-token", token)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(request_body())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(admitted.status(), StatusCode::OK);

        let evidence = router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/products/pinakotheke/api/extension/v1/cache-aliases/lookup")
                    .header("content-type", "application/json")
                    .header("x-monas-dispatch-token", token)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({
                            "schema_version": "x-img.cache-alias-lookup.v1",
                            "pairing_id": "pair-0",
                            "instance_id": "pinakotheke-monolith",
                            "origin": "https://example.invalid",
                            "canonical_alias": "https://example.invalid/not-stored.jpg",
                            "adapter_id": "generic-observed-image",
                            "adapter_version": "1.0.0"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(evidence.status(), StatusCode::OK);
        let evidence: serde_json::Value =
            serde_json::from_slice(&to_bytes(evidence.into_body(), 4_096).await.unwrap()).unwrap();
        assert_eq!(evidence["outcome"], "miss");
    }

    #[tokio::test]
    async fn verified_worker_completion_updates_live_gallery_and_settles_pending_plan() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-api-capture-completion-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let store = GalleryCatalogueStore::new(root.join("gallery.json"));
        let dispatch = "synthetic-monas-dispatch-token-0001";
        let worker = "synthetic-capture-worker-token-0001";
        let app = monolith_router_with_gallery_web_delivery_and_capture_authority(
            true,
            Some(MonasDispatchVerifier::new(dispatch.into()).unwrap()),
            GalleryCatalogue::default(),
            None,
            gallery_image_backend(),
            Some(with_ready_capture_destination(
                CapturePlanComposition::new(
                    capture_plans(),
                    Some(
                        CaptureCompletionAuthority::new(
                            worker.into(),
                            store,
                            "synthetic-endpoint".into(),
                            "synthetic-store".into(),
                        )
                        .unwrap(),
                    ),
                ),
                root.join("destinations.json"),
            )),
        );
        let planned = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/products/pinakotheke/api/extension/v1/capture-plans")
                    .header("content-type", "application/json")
                    .header("x-monas-dispatch-token", dispatch)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(request_body())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(planned.status(), StatusCode::OK);
        let plan: serde_json::Value =
            serde_json::from_slice(&to_bytes(planned.into_body(), 16 * 1024).await.unwrap())
                .unwrap();
        let plan_id = plan["plan_id"].as_str().unwrap();
        let pending_status = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/ingestion/v1/status")
                    .header("x-monas-dispatch-token", dispatch)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pending_status.status(), StatusCode::OK);
        let pending_status: serde_json::Value =
            serde_json::from_slice(&to_bytes(pending_status.into_body(), 4096).await.unwrap())
                .unwrap();
        assert_eq!(pending_status["observed_thumbnails"], 1);
        assert_eq!(pending_status["pending"], 1);
        assert_eq!(pending_status["stored"], 0);
        let completion = serde_json::json!({
            "schema_version": "pinakotheke.capture-completion.v1",
            "catalogue_id": "website-card-1",
            "title": "Synthetic artwork",
            "content_type": "image/jpeg",
            "content_length": 12,
            "endpoint_id": "synthetic-endpoint",
            "object_store_id": "synthetic-store",
            "object_key": "x.com/fixtureartist/observed_thumbnail/image-1",
            "object_version": 1,
            "checksum_sha256": CHECKSUM.strip_prefix("sha256:").unwrap(),
            "verified_at_epoch_seconds": 42
        });
        let denied = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/products/pinakotheke/api/internal/v1/capture-plans/{plan_id}/complete"
                    ))
                    .header("content-type", "application/json")
                    .header("x-monas-dispatch-token", dispatch)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::from(serde_json::to_vec(&completion).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
        let mut wrong_destination = completion.clone();
        wrong_destination["object_store_id"] = serde_json::json!("other-store");
        let wrong_destination = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/products/pinakotheke/api/internal/v1/capture-plans/{plan_id}/complete"
                    ))
                    .header("content-type", "application/json")
                    .header("x-monas-dispatch-token", dispatch)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .header("x-pinakotheke-capture-worker-token", worker)
                    .body(Body::from(serde_json::to_vec(&wrong_destination).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(wrong_destination.status(), StatusCode::CONFLICT);
        let completed = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/products/pinakotheke/api/internal/v1/capture-plans/{plan_id}/complete"
                    ))
                    .header("content-type", "application/json")
                    .header("x-monas-dispatch-token", dispatch)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .header("x-pinakotheke-capture-worker-token", worker)
                    .body(Body::from(serde_json::to_vec(&completion).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(completed.status(), StatusCode::OK);
        let stored = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/products/pinakotheke/api/extension/v1/capture-plans/{plan_id}"
                    ))
                    .header("x-monas-dispatch-token", dispatch)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stored.status(), StatusCode::OK);
        let stored: serde_json::Value =
            serde_json::from_slice(&to_bytes(stored.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(stored["state"], "stored");
        let catalogue = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/gallery/v1/catalogue")
                    .header("x-monas-dispatch-token", dispatch)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let catalogue: serde_json::Value =
            serde_json::from_slice(&to_bytes(catalogue.into_body(), 16 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(catalogue["items"].as_array().unwrap().len(), 1);
        let stored_status = app
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/ingestion/v1/status")
                    .header("x-monas-dispatch-token", dispatch)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let stored_status: serde_json::Value =
            serde_json::from_slice(&to_bytes(stored_status.into_body(), 4096).await.unwrap())
                .unwrap();
        assert_eq!(stored_status["pending"], 0);
        assert_eq!(stored_status["stored"], 1);
        assert_eq!(stored_status["gallery_items"], 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn admitted_plan_retries_transient_object_upload_and_becomes_live() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-api-background-capture-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let store = GalleryCatalogueStore::new(root.join("gallery.json"));
        let dispatch = "synthetic-monas-dispatch-token-0001";
        let worker = "synthetic-capture-worker-token-0001";
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_acquire = Arc::clone(&attempts);
        let acquire = HostCaptureAcquireBackend::new(Box::new(move |_, plan| {
            if attempts_for_acquire.fetch_add(1, Ordering::SeqCst) == 0 {
                return Err("capture helper rejected the plan: object_upload".into());
            }
            Ok(VerifiedCaptureCompletion {
                plan_id: plan.plan_id.clone(),
                catalogue_id: "background-card-1".into(),
                title: "Background synthetic image".into(),
                content_type: "image/jpeg".into(),
                content_length: 12,
                endpoint_id: "synthetic-endpoint".into(),
                object_store_id: "synthetic-store".into(),
                object_key: "background-object-1".into(),
                object_version: 1,
                checksum_sha256: CHECKSUM.strip_prefix("sha256:").unwrap().into(),
                verified_at_epoch_seconds: 42,
                video: None,
            })
        }));
        let app = monolith_router_with_gallery_web_delivery_and_capture_authority(
            true,
            Some(MonasDispatchVerifier::new(dispatch.into()).unwrap()),
            GalleryCatalogue::default(),
            None,
            gallery_image_backend(),
            Some(with_ready_capture_destination(
                CapturePlanComposition::new(
                    capture_plans(),
                    Some(
                        CaptureCompletionAuthority::new(
                            worker.into(),
                            store,
                            "synthetic-endpoint".into(),
                            "synthetic-store".into(),
                        )
                        .unwrap(),
                    ),
                )
                .with_acquire(acquire),
                root.join("destinations.json"),
            )),
        );
        let planned = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/products/pinakotheke/api/extension/v1/capture-plans")
                    .header("content-type", "application/json")
                    .header("x-monas-dispatch-token", dispatch)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(request_body())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(planned.status(), StatusCode::OK);
        let mut visible = false;
        for _ in 0..50 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/products/pinakotheke/api/gallery/v1/catalogue")
                        .header("x-monas-dispatch-token", dispatch)
                        .header("x-monas-host-context", MONAS_CONTEXT)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let json: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 16 * 1024).await.unwrap())
                    .unwrap();
            if json["items"]
                .as_array()
                .is_some_and(|items| items.len() == 1)
            {
                visible = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(visible, "background capture should update the live gallery");
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn background_helper_failure_keeps_plan_pending_without_gallery_claim() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-api-failed-background-capture-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let dispatch = "synthetic-monas-dispatch-token-0001";
        let app = monolith_router_with_gallery_web_delivery_and_capture_authority(
            true,
            Some(MonasDispatchVerifier::new(dispatch.into()).unwrap()),
            GalleryCatalogue::default(),
            None,
            gallery_image_backend(),
            Some(with_ready_capture_destination(
                CapturePlanComposition::new(
                    capture_plans(),
                    Some(
                        CaptureCompletionAuthority::new(
                            "synthetic-capture-worker-token-0001".into(),
                            GalleryCatalogueStore::new(root.join("gallery.json")),
                            "synthetic-endpoint".into(),
                            "synthetic-store".into(),
                        )
                        .unwrap(),
                    ),
                )
                .with_acquire(HostCaptureAcquireBackend::new(Box::new(|_, _| {
                    Err("synthetic helper failure".into())
                }))),
                root.join("destinations.json"),
            )),
        );
        let planned = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/products/pinakotheke/api/extension/v1/capture-plans")
                    .header("content-type", "application/json")
                    .header("x-monas-dispatch-token", dispatch)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(request_body())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(planned.status(), StatusCode::OK);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        let pending = app
            .oneshot(
                Request::builder()
                    .uri("/products/pinakotheke/api/extension/v1/capture-plans")
                    .header("x-monas-dispatch-token", dispatch)
                    .header("x-monas-host-context", MONAS_CONTEXT)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let pending: serde_json::Value =
            serde_json::from_slice(&to_bytes(pending.into_body(), 16 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(pending["plans"].as_array().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn startup_requeues_journalled_pending_plan_without_browser_retry() {
        let root = std::env::temp_dir().join(format!(
            "pinakotheke-api-startup-recovery-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let pairings = || {
            [CapturePairing {
                pairing_id: "pair-0".into(),
                actor_id: "synthetic-monas-user".into(),
                expires_at: u64::MAX,
                revoked: false,
            }]
        };
        let sites = || {
            [SiteCapturePolicy {
                site_id: "synthetic-site".into(),
                origin: "https://example.invalid".into(),
                capture_enabled: true,
                adapter_kind: AdapterKind::ExperimentalGeneric,
                adapter_version: "1.0.0".into(),
                allow_observed_thumbnails: true,
                allow_explicit_originals: false,
                max_candidates_per_page: 2,
            }]
        };
        let journal = root.join("capture-plans.json");
        let mut before_restart =
            CapturePlanService::with_journal(pairings(), sites(), &journal).unwrap();
        before_restart
            .plan_with_destination(
                "synthetic-monas-user",
                1,
                CapturePlanRequest {
                    schema_version: CAPTURE_REQUEST_SCHEMA_VERSION.into(),
                    pairing_id: "pair-0".into(),
                    origin: "https://example.invalid".into(),
                    page_url: "https://example.invalid/gallery".into(),
                    adapter_kind: AdapterKind::ExperimentalGeneric,
                    adapter_version: "1.0.0".into(),
                    capture_kind: CaptureKind::ObservedThumbnail,
                    media_url: "https://example.invalid/startup.jpg".into(),
                    presentation_url: None,
                    width: 320,
                    height: 200,
                },
                x_img_core::viewed_media::CaptureDestinationSnapshot {
                    endpoint_id: "synthetic-endpoint".into(),
                    object_store_id: "synthetic-store".into(),
                    selection_revision: 1,
                },
            )
            .unwrap();
        drop(before_restart);
        let restarted = CapturePlanService::with_journal(pairings(), sites(), &journal).unwrap();
        let store = GalleryCatalogueStore::new(root.join("gallery.json"));
        let dispatch = "synthetic-monas-dispatch-token-0001";
        let acquire = HostCaptureAcquireBackend::new(Box::new(|_, plan| {
            Ok(VerifiedCaptureCompletion {
                plan_id: plan.plan_id.clone(),
                catalogue_id: "startup-card-1".into(),
                title: "Recovered image".into(),
                content_type: "image/jpeg".into(),
                content_length: 12,
                endpoint_id: "synthetic-endpoint".into(),
                object_store_id: "synthetic-store".into(),
                object_key: "startup-object-1".into(),
                object_version: 1,
                checksum_sha256: CHECKSUM.strip_prefix("sha256:").unwrap().into(),
                verified_at_epoch_seconds: 42,
                video: None,
            })
        }));
        let app = monolith_router_with_gallery_web_delivery_and_capture_authority(
            true,
            Some(MonasDispatchVerifier::new(dispatch.into()).unwrap()),
            GalleryCatalogue::default(),
            None,
            gallery_image_backend(),
            Some(with_ready_capture_destination(
                CapturePlanComposition::new(
                    restarted,
                    Some(
                        CaptureCompletionAuthority::new(
                            "synthetic-capture-worker-token-0001".into(),
                            store,
                            "synthetic-endpoint".into(),
                            "synthetic-store".into(),
                        )
                        .unwrap(),
                    ),
                )
                .with_acquire(acquire),
                root.join("destinations.json"),
            )),
        );
        let mut visible = false;
        for _ in 0..50 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/products/pinakotheke/api/gallery/v1/catalogue")
                        .header("x-monas-dispatch-token", dispatch)
                        .header("x-monas-host-context", MONAS_CONTEXT)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let json: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 16 * 1024).await.unwrap())
                    .unwrap();
            if json["items"]
                .as_array()
                .is_some_and(|items| items.len() == 1)
            {
                visible = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(visible, "startup should recover durable pending capture");
        let recovered = CapturePlanService::with_journal(pairings(), sites(), &journal).unwrap();
        assert!(
            recovered
                .pending_for_actor("synthetic-monas-user")
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(root);
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
