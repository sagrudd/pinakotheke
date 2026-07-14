// SPDX-License-Identifier: MPL-2.0
//! Yew application boundary for the future Mnemosyne-designed web interface.
//!
//! Visual implementation begins only after the related interface contracts are
//! complete; this component provides a compile-checked client boundary.

use yew::prelude::*;

/// Minimal root component for host integration.
#[function_component(App)]
pub fn app() -> Html {
    html! {
        <main>
            <h1>{ "x-img" }</h1>
            <p>{ "Web workspace scaffold; no live integrations are enabled." }</p>
        </main>
    }
}
