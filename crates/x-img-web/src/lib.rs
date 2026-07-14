// SPDX-License-Identifier: MPL-2.0
//! Mnemosyne-compatible, host-integrable Yew application shell.

use yew::prelude::*;

/// Minimal root component for host integration.
#[function_component(App)]
pub fn app() -> Html {
    let selected = use_state(|| "all".to_owned());
    let density = use_state(|| "compact".to_owned());
    let active_card = use_state(|| 0usize);
    let refresh_state = use_state(|| "Not started".to_owned());
    let preview_open = use_state(|| false);
    let review_notice = use_state(|| "3 items need review".to_owned());
    let cards = [
        "Aurora study",
        "Tidal form",
        "Glass archive",
        "Night garden",
        "Field record",
        "Open geometry",
    ];
    let sources = [
        ("all", "All sources", "6"),
        ("x", "X accounts", "2"),
        ("instagram", "Instagram accounts", "3"),
        ("websites", "Websites", "1"),
    ];
    html! {
        <div class="mn-app-shell ximg-shell">
            <header class="ximg-shell__header" aria-label="x-img workspace">
                <a class="ximg-shell__product" href="#main">{ "x-img" }</a>
                <nav class="ximg-shell__nav" aria-label="Primary navigation">
                    <a aria-current="page" href="#library">{ "Library" }</a>
                    <a href="#accounts">{ "Accounts" }</a>
                </nav>
                <p class="ximg-shell__host">{ "Hosted by Monas" }</p>
            </header>
            <main id="main" class="mn-app-main ximg-shell__main" tabindex="-1">
                <p class="ximg-shell__eyebrow">{ "Media workspace" }</p>
                <h1>{ "x-img library" }</h1>
                <p>{ "Review committed media from configured sources." }</p>
                <section class="ximg-refresh" aria-labelledby="refresh-title"><h2 id="refresh-title">{ "Account refresh" }</h2><p>{ format!("Status: {}", *refresh_state) }</p><button onclick={{ let refresh_state=refresh_state.clone(); Callback::from(move |_| refresh_state.set("Complete · 2 accounts · 0 new items".to_owned())) }}>{ "Refresh accounts" }</button><button onclick={{ let refresh_state=refresh_state.clone(); Callback::from(move |_| refresh_state.set("Partial failure · retry available".to_owned())) }}>{ "Show retry state" }</button></section>
                <section class="ximg-review" aria-labelledby="review-title"><h2 id="review-title">{"Review queue"}</h2><p>{format!("State: {}",*review_notice)}</p><p>{"Selected records: 0 · New · Stored in ObjectStore"}</p><button onclick={{let review_notice=review_notice.clone();Callback::from(move |_|review_notice.set("Batch reviewed · 3 items".to_owned()))}}>{"Mark selected reviewed"}</button><button onclick={{let review_notice=review_notice.clone();Callback::from(move |_|review_notice.set("Batch hidden · 3 items".to_owned()))}}>{"Hide selected"}</button></section>
                <section class="ximg-source-nav" aria-labelledby="source-context">
                    <h2 id="source-context">{ "Sources" }</h2>
                    <p>{ format!("Selected context: {}", sources.iter().find(|source| source.0 == (*selected).as_str()).map(|source| source.1).unwrap_or("All sources")) }</p>
                    <ul>
                    { for sources.iter().map(|(id, label, count)| { let selected = selected.clone(); let is_selected = *id == (*selected).as_str(); let id = (*id).to_owned(); html! { <li><button class={classes!("ximg-source-nav__item", is_selected.then_some("is-selected"))} aria-pressed={is_selected.to_string()} onclick={Callback::from(move |_| selected.set(id.clone()))}><span>{ *label }</span><span>{ format!("{} sources", count) }</span></button></li> } }) }
                    </ul>
                </section>
                <section class="ximg-shell__empty" aria-labelledby="library-state"><h2 id="library-state">{ "No committed media in this context" }</h2><p>{ "Counts describe configured sources; committed media appears here after verified admission." }</p></section>
                <section class="ximg-gallery" aria-labelledby="gallery-title"><div class="ximg-gallery__toolbar"><h2 id="gallery-title">{ "Thumbnail browser" }</h2><button onclick={{ let density=density.clone(); Callback::from(move |_| density.set(if *density=="compact" { "comfortable".to_owned() } else { "compact".to_owned() })) }}>{ format!("Density: {}", *density) }</button></div><p>{ "Synthetic preview records; images lazy-load when a future authorized catalogue supplies them." }</p><div class={classes!("ximg-gallery__grid", format!("is-{}", *density))}>{for cards.iter().enumerate().map(|(index,label)| { let active_card=active_card.clone(); let preview_open=preview_open.clone(); let selected=index==*active_card; html!{<button class={classes!("ximg-gallery__card",selected.then_some("is-selected"))} aria-pressed={selected.to_string()} onclick={Callback::from(move |_|{active_card.set(index);preview_open.set(true)})}><span class="ximg-gallery__placeholder" aria-hidden="true"></span><span>{*label}</span><small>{format!("Record {}",index+1)}</small></button>}})}</div></section>
                {if *preview_open { html!{<section class="ximg-preview" role="dialog" aria-modal="true" aria-labelledby="preview-title"><button autofocus=true onclick={{let preview_open=preview_open.clone();Callback::from(move |_|preview_open.set(false))}}>{"Close preview"}</button><h2 id="preview-title">{cards[*active_card]}</h2><p>{"Source: configured catalogue record · Object: Unavailable"}</p><a href="#source">{"View source metadata"}</a><video controls=true preload="metadata"><p>{"Normalized MP4 unavailable until authorized range delivery is connected."}</p></video><p>{"Video playback uses the browser’s native controls once a verified normalized MP4 range URL is supplied."}</p></section>} } else {Html::default()}}
            </main>
            <footer class="mn-brand-footer" aria-label="Mnemosyne Biosciences provenance">
                <div class="mn-brand-footer__content">
                    <span class="mn-brand-footer__wordmark">{ "Mnemosyne Biosciences" }</span>
                    <span class="mn-brand-footer__product">{ "x-img · host-integrated workspace" }</span>
                </div>
                <span class="mn-brand-footer__mark" aria-hidden="true">{ "◒" }</span>
            </footer>
        </div>
    }
}
