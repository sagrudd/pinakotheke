// SPDX-License-Identifier: MPL-2.0
//! Shared, storage-free product model boundaries.
//!
//! This crate intentionally contains no media payloads, source connectors, or
//! authentication material. Those integrations are introduced only after their
//! policy and contract gates are complete.

/// The repository identity retained until the coordinated v1.0.0 rebrand.
pub const REPOSITORY_NAME: &str = "x-img";

/// A minimal product identity suitable for UI and host adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductIdentity {
    /// The current repository and compatibility name.
    pub name: &'static str,
    /// The semantic version supplied by the workspace package authority.
    pub version: &'static str,
}

/// Returns the current build's public identity.
#[must_use]
pub const fn product_identity() -> ProductIdentity {
    ProductIdentity {
        name: REPOSITORY_NAME,
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[cfg(test)]
mod tests {
    use super::{REPOSITORY_NAME, product_identity};

    #[test]
    fn identity_uses_the_current_repository_name() {
        assert_eq!(product_identity().name, REPOSITORY_NAME);
    }
}
