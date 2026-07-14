// SPDX-License-Identifier: MPL-2.0
//! Axum composition boundary for a future host-managed API service.
//!
//! A host must validate its session before injecting an authenticated context.
//! This crate never parses cookies, passwords, or session tokens.

use axum::{Extension, Router, http::StatusCode, routing::get};
use x_img_core::host_context::{AuthenticatedHostContext, XIMG_ACCESS};

/// Returns the product router. Health is public; every product API route needs
/// a host-injected, authorized context.
pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/context", get(context))
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

#[cfg(test)]
mod tests {
    use axum::{
        Extension,
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use x_img_core::host_context::{HostContextAdapter, MonasHostContextAdapter};

    use super::router;

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
}
