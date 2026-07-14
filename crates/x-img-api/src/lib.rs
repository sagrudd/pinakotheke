// SPDX-License-Identifier: MPL-2.0
//! Axum composition boundary for a future host-managed API service.
//!
//! This crate constructs no listener and implements no authentication, source,
//! or DASObjectStore operation. The host integration is intentionally deferred.

use axum::{Router, routing::get};

/// Returns the minimal, unauthenticated health-only router for host wiring.
pub fn router() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> &'static str {
    "x-img API scaffold"
}

#[cfg(test)]
mod tests {
    use super::router;

    #[test]
    fn creates_a_router_without_starting_a_listener() {
        let _router = router();
    }
}
