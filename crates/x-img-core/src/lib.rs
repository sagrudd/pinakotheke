// SPDX-License-Identifier: MPL-2.0
//! Application-core boundaries with no live source or storage integration.

use x_img_model::{ProductIdentity, product_identity};

/// Build information exposed uniformly to the CLI, API host, and web client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildInfo {
    /// Product identity for this build.
    pub product: ProductIdentity,
}

impl BuildInfo {
    /// Returns a concise human-readable summary.
    #[must_use]
    pub fn summary(self) -> String {
        format!("{} {}", self.product.name, self.product.version)
    }
}

/// Returns the current build information.
#[must_use]
pub const fn build_info() -> BuildInfo {
    BuildInfo {
        product: product_identity(),
    }
}

#[cfg(test)]
mod tests {
    use super::build_info;

    #[test]
    fn summary_contains_the_workspace_version() {
        assert_eq!(build_info().summary(), "x-img 0.2.0");
    }
}
