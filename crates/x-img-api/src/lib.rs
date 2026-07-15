// SPDX-License-Identifier: MPL-2.0
//! Axum composition boundary for a future host-managed API service.
//!
//! A host must validate its session before injecting an authenticated context.
//! This crate never parses cookies, passwords, or session tokens.

use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
    routing::{get, post},
};
use x_img_core::{
    host_context::{AuthenticatedHostContext, XIMG_ACCESS},
    object_read::{ObjectReadBackend, ObjectReadBackendError, ObjectReadRequest, ObjectReadResult},
    playback_delivery::{DirectPlaybackError, DirectPlaybackResponse, DirectPlaybackService},
    viewed_media::{CapturePlan, CapturePlanError, CapturePlanRequest, CapturePlanService},
};

type CapturePlans = Arc<Mutex<CapturePlanService>>;
type PlaybackDelivery = Arc<Mutex<DirectPlaybackService<HostObjectReadBackend>>>;

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
        .route("/api/playback/v1/{playback_id}", get(deliver_playback))
        .with_state(None::<CapturePlans>)
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
        .route("/api/playback/v1/{playback_id}", get(deliver_playback))
        .with_state(Some(Arc::new(Mutex::new(capture_plans))))
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
        .route("/api/playback/v1/{playback_id}", get(deliver_playback))
        .with_state(None::<CapturePlans>)
        .layer(Extension(Arc::new(Mutex::new(playback))))
}

async fn health() -> &'static str {
    "x-img API scaffold"
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
    use axum::{
        Extension,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use x_img_core::{
        host_context::{HostContextAdapter, MonasHostContextAdapter},
        object_read::{
            AuthorizedObjectReader, AuthorizedObjectReference, ObjectContentMetadata,
            ObjectReadResult,
        },
        playback_delivery::{DirectPlaybackGrant, DirectPlaybackService},
        video_profile::NormalizedVideoState,
        viewed_media::{
            AdapterKind, CAPTURE_REQUEST_SCHEMA_VERSION, CaptureKind, CapturePairing,
            CapturePlanRequest, CapturePlanService, SiteCapturePolicy,
        },
    };

    use super::{
        HostObjectReadBackend, router, router_with_capture_plans, router_with_direct_playback,
    };

    const CHECKSUM: &str =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const ETAG: &str =
        "\"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"";
    const PLAYBACK_BYTES: &[u8] = b"synthetic-firefox-playback";

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
                    checksum: CHECKSUM.into(),
                },
                total_length: PLAYBACK_BYTES.len() as u64,
                state: NormalizedVideoState::Ready,
            }],
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

    #[test]
    fn creates_a_router_without_starting_a_listener() {
        let _router = router();
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
}
