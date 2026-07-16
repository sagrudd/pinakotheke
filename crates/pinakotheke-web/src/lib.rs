// SPDX-License-Identifier: MPL-2.0
//! Mnemosyne-compatible, host-integrable Yew application shell.

use gloo_net::http::Request;
use serde::Deserialize;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlElement, HtmlInputElement, KeyboardEvent};
use x_img_core::gallery_catalogue::{
    GALLERY_CATALOGUE_SCHEMA, GalleryItem, GalleryMediaKind, GalleryObjectAvailability,
    GalleryRepresentation, GalleryReviewState, GallerySourceKind,
};
use yew::prelude::*;

const GALLERY_API: &str = "/products/pinakotheke/api/gallery/v1/catalogue?limit=200";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct GalleryPageResponse {
    schema_version: String,
    items: Vec<GalleryItem>,
    next_offset: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GalleryLoadState {
    Loading,
    Ready(Vec<GalleryItem>),
    PermissionDenied,
    TransportError,
    InvalidResponse,
}

fn media_label(item: &GalleryItem) -> &'static str {
    match item.media_kind {
        GalleryMediaKind::Image => "Image",
        GalleryMediaKind::NormalizedVideo => "Video · normalized",
    }
}

fn review_label(item: &GalleryItem) -> &'static str {
    match item.review_state {
        GalleryReviewState::New => "New",
        GalleryReviewState::Reviewed => "Reviewed",
        GalleryReviewState::Hidden => "Hidden",
        GalleryReviewState::Removed => "Removed",
    }
}

fn object_label(item: &GalleryItem) -> &'static str {
    if item.thumbnail.availability == GalleryObjectAvailability::Unavailable {
        "Object unavailable"
    } else if item
        .preview
        .as_ref()
        .is_some_and(|preview| preview.availability == GalleryObjectAvailability::Ready)
    {
        "Stored in ObjectStore"
    } else {
        "Previously observed"
    }
}

fn ready_path(representation: &GalleryRepresentation) -> Option<&str> {
    (representation.availability == GalleryObjectAvailability::Ready)
        .then_some(representation.delivery_path.as_deref())
        .flatten()
}

fn source_matches(selected: &str, item: &GalleryItem) -> bool {
    match selected {
        "x" => item.source_kind == GallerySourceKind::XAccount,
        "websites" => item.source_kind == GallerySourceKind::Website,
        _ => true,
    }
}

fn focus_by_id(id: &str) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(element) = document.get_element_by_id(id) else {
        return;
    };
    if let Ok(element) = element.dyn_into::<HtmlElement>() {
        let _ = element.focus();
    }
}

fn focus_preview_control(current: Option<String>, reverse: bool) {
    const CONTROLS: [&str; 3] = ["preview-close", "preview-view-mode", "preview-source-link"];
    let current = current
        .as_deref()
        .and_then(|id| CONTROLS.iter().position(|candidate| candidate == &id));
    let next = match (current, reverse) {
        (Some(0), true) | (None, true) => CONTROLS.len() - 1,
        (Some(index), true) => index - 1,
        (Some(index), false) if index + 1 < CONTROLS.len() => index + 1,
        _ => 0,
    };
    focus_by_id(CONTROLS[next]);
}

/// Minimal root component for host integration.
#[function_component(App)]
pub fn app() -> Html {
    let selected = use_state(|| "all".to_owned());
    let density = use_state(|| "compact".to_owned());
    let active_card = use_state(|| 0usize);
    let refresh_state = use_state(|| "Not started".to_owned());
    let refresh_detail = use_state(|| {
        "X / SelectedArtist — Pending; Instagram / SampleCreator — Pending".to_owned()
    });
    let preview_open = use_state(|| false);
    let preview_mode = use_state(|| "Fit to pane".to_owned());
    let preview_ref = use_node_ref();
    let review_notice = use_state(|| "3 items need review".to_owned());
    let object_view = use_state(|| true);
    let filter = use_state(String::new);
    let gallery = use_state(|| GalleryLoadState::Loading);

    {
        let gallery = gallery.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                let state = match Request::get(GALLERY_API).send().await {
                    Ok(response) if matches!(response.status(), 401 | 403) => {
                        GalleryLoadState::PermissionDenied
                    }
                    Ok(response) if response.ok() => {
                        match response.json::<GalleryPageResponse>().await {
                            Ok(page) if page.schema_version == GALLERY_CATALOGUE_SCHEMA => {
                                GalleryLoadState::Ready(page.items)
                            }
                            _ => GalleryLoadState::InvalidResponse,
                        }
                    }
                    Ok(_) | Err(_) => GalleryLoadState::TransportError,
                };
                gallery.set(state);
            });
            || ()
        });
    }

    let items = match &*gallery {
        GalleryLoadState::Ready(items) => items.as_slice(),
        _ => &[],
    };
    let sources = [
        ("all", "All sources", items.len()),
        (
            "x",
            "X accounts",
            items
                .iter()
                .filter(|item| item.source_kind == GallerySourceKind::XAccount)
                .count(),
        ),
        (
            "websites",
            "Websites",
            items
                .iter()
                .filter(|item| item.source_kind == GallerySourceKind::Website)
                .count(),
        ),
    ];

    {
        let preview_ref = preview_ref.clone();
        use_effect_with(*preview_open, move |is_open| {
            if *is_open && let Some(dialog) = preview_ref.cast::<HtmlElement>() {
                let _ = dialog.focus();
            }
            || ()
        });
    }

    let selected_card = items.get(*active_card).cloned();
    let filter_text = (*filter).to_ascii_lowercase();
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

                <section class="ximg-filters" aria-label="Browse metadata filters">
                    <label>
                        {"Search metadata "}
                        <input
                            value={(*filter).clone()}
                            oninput={{
                                let filter = filter.clone();
                                Callback::from(move |event: InputEvent| {
                                    filter.set(event.target_unchecked_into::<HtmlInputElement>().value())
                                })
                            }}
                            placeholder="Account, media type, date"
                        />
                    </label>
                    <p>{if filter.is_empty() { "Filters: all records" } else { "Filter active: matching synthetic records" }}</p>
                </section>

                <section class="ximg-refresh" aria-labelledby="refresh-title">
                    <h2 id="refresh-title">{ "Account refresh" }</h2>
                    <p>{ format!("Status: {}", *refresh_state) }</p>
                    <p>{ format!("Per-account progress: {}", *refresh_detail) }</p>
                    <button onclick={{
                        let refresh_state = refresh_state.clone();
                        let refresh_detail = refresh_detail.clone();
                        Callback::from(move |_| {
                            refresh_state.set("Complete · 2 accounts · 3 new items".to_owned());
                            refresh_detail.set("X / SelectedArtist — Complete (2 new); Instagram / SampleCreator — Complete (1 new)".to_owned())
                        })
                    }}>{ "Refresh accounts" }</button>
                    <button onclick={{
                        let refresh_state = refresh_state.clone();
                        let refresh_detail = refresh_detail.clone();
                        Callback::from(move |_| {
                            refresh_state.set("Partial failure · retry available".to_owned());
                            refresh_detail.set("X / SelectedArtist — Complete (2 new); Instagram / SampleCreator — Failed: retry available".to_owned())
                        })
                    }}>{ "Show retry state" }</button>
                    <button onclick={{
                        let refresh_state = refresh_state.clone();
                        Callback::from(move |_| refresh_state.set("Retry scheduled safely for Instagram / SampleCreator".to_owned()))
                    }}>{ "Retry failed account" }</button>
                </section>

                <section class="ximg-review" aria-labelledby="review-title">
                    <h2 id="review-title">{ "Review queue" }</h2>
                    <p>{ format!("State: {}", *review_notice) }</p>
                    <p>{ "Selected records: 0 · New · Reviewed · Hidden · Removed" }</p>
                    <button
                        aria-pressed={object_view.to_string()}
                        onclick={{
                            let object_view = object_view.clone();
                            Callback::from(move |_| object_view.set(!*object_view))
                        }}
                    >{if *object_view { "Showing: Stored in ObjectStore originals" } else { "Showing: Previously observed thumbnails" }}</button>
                    <p title="This label is a reversible overlay; stored bytes are never changed.">
                        {if *object_view { "● Stored in ObjectStore — committed original" } else { "◌ Previously observed — thumbnail only" }}
                    </p>
                    <button onclick={{
                        let review_notice = review_notice.clone();
                        Callback::from(move |_| review_notice.set("Batch reviewed · 3 items · Undo available".to_owned()))
                    }}>{ "Mark selected reviewed" }</button>
                    <button onclick={{
                        let review_notice = review_notice.clone();
                        Callback::from(move |_| review_notice.set("Batch hidden · 3 items · Undo available".to_owned()))
                    }}>{ "Hide selected" }</button>
                    <button onclick={{
                        let review_notice = review_notice.clone();
                        Callback::from(move |_| review_notice.set("Batch action undone".to_owned()))
                    }}>{ "Undo batch action" }</button>
                </section>

                <section class="ximg-source-nav" aria-labelledby="source-context">
                    <h2 id="source-context">{ "Sources" }</h2>
                    <p>{ format!("Selected context: {}", sources.iter().find(|source| source.0 == (*selected).as_str()).map(|source| source.1).unwrap_or("All sources")) }</p>
                    <ul>
                        { for sources.iter().map(|(id, label, count)| {
                            let selected = selected.clone();
                            let is_selected = *id == (*selected).as_str();
                            let id = (*id).to_owned();
                            html! {
                                <li>
                                    <button
                                        class={classes!("ximg-source-nav__item", is_selected.then_some("is-selected"))}
                                        aria-pressed={is_selected.to_string()}
                                        onclick={Callback::from(move |_| selected.set(id.clone()))}
                                    ><span>{ *label }</span><span>{ format!("{} sources", count) }</span></button>
                                </li>
                            }
                        }) }
                    </ul>
                </section>

                <section class="ximg-gallery" aria-labelledby="gallery-title">
                    <div class="ximg-gallery__toolbar">
                        <h2 id="gallery-title">{ "Thumbnail browser" }</h2>
                        <button onclick={{
                            let density = density.clone();
                            Callback::from(move |_| density.set(if *density == "compact" { "comfortable".to_owned() } else { "compact".to_owned() }))
                        }}>{ format!("Density: {}", *density) }</button>
                    </div>
                    <p>{ "Verified media references are loaded through the Monas-authenticated Pinakotheke catalogue." }</p>
                    { match &*gallery {
                        GalleryLoadState::Loading => html! {
                            <div class="ximg-gallery__state" role="status" aria-live="polite">
                                <h3>{ "Loading media library" }</h3>
                                <p>{ "Waiting for the authenticated catalogue." }</p>
                            </div>
                        },
                        GalleryLoadState::PermissionDenied => html! {
                            <div class="ximg-gallery__state" role="alert">
                                <h3>{ "Permission required" }</h3>
                                <p>{ "Monas did not authorize catalogue access. Sign in again from the Monas host." }</p>
                            </div>
                        },
                        GalleryLoadState::TransportError => html! {
                            <div class="ximg-gallery__state" role="alert">
                                <h3>{ "Catalogue unavailable" }</h3>
                                <p>{ "Pinakotheke could not load the catalogue. No source website was contacted." }</p>
                            </div>
                        },
                        GalleryLoadState::InvalidResponse => html! {
                            <div class="ximg-gallery__state" role="alert">
                                <h3>{ "Catalogue response unsupported" }</h3>
                                <p>{ "The response schema was not accepted. Update the host and Pinakotheke together." }</p>
                            </div>
                        },
                        GalleryLoadState::Ready(records) if records.is_empty() => html! {
                            <div class="ximg-gallery__state" role="status">
                                <h3>{ "No committed media" }</h3>
                                <p>{ "Media appears after Firefox capture and verified DASObjectStore admission." }</p>
                            </div>
                        },
                        GalleryLoadState::Ready(records) => html! {
                            <div class={classes!("ximg-gallery__grid", format!("is-{}", *density))}>
                                { for records.iter().enumerate().filter(|(_, item)| {
                                    source_matches(&selected, item)
                                        && (item.title.to_ascii_lowercase().contains(&filter_text)
                                            || item.source_label.to_ascii_lowercase().contains(&filter_text)
                                            || media_label(item).to_ascii_lowercase().contains(&filter_text))
                                }).map(|(index, item)| {
                                    let active_card = active_card.clone();
                                    let preview_open = preview_open.clone();
                                    let preview_mode = preview_mode.clone();
                                    let is_selected = index == *active_card;
                                    let thumbnail_path = ready_path(&item.thumbnail).map(ToOwned::to_owned);
                                    html! {
                                        <button
                                            id={format!("preview-trigger-{index}")}
                                            class={classes!("ximg-gallery__card", is_selected.then_some("is-selected"))}
                                            aria-haspopup="dialog"
                                            aria-pressed={is_selected.to_string()}
                                            onclick={Callback::from(move |_| {
                                                active_card.set(index);
                                                preview_mode.set("Fit to pane".to_owned());
                                                preview_open.set(true)
                                            })}
                                        >
                                            { if let Some(path) = thumbnail_path {
                                                html! { <img class="ximg-gallery__thumbnail" src={path} alt="" loading="lazy" /> }
                                            } else {
                                                html! { <span class="ximg-gallery__placeholder" aria-hidden="true">{ "Unavailable" }</span> }
                                            }}
                                            <span>{ item.title.clone() }</span>
                                            <small>{ format!("{} · {} · {}", media_label(item), object_label(item), review_label(item)) }</small>
                                        </button>
                                    }
                                }) }
                            </div>
                        },
                    }}
                </section>

                { if *preview_open && selected_card.is_some() {
                    let selected_card = selected_card.expect("checked selected card");
                    let close_preview_state = preview_open.clone();
                    let close_preview_card = active_card.clone();
                    let close_preview = Callback::from(move |_| {
                        close_preview_state.set(false);
                        focus_by_id(&format!("preview-trigger-{}", *close_preview_card));
                    });
                    let keyboard_preview_state = preview_open.clone();
                    let keyboard_active_card = active_card.clone();
                    let on_keydown = Callback::from(move |event: KeyboardEvent| match event.key().as_str() {
                        "Escape" => {
                            event.prevent_default();
                            keyboard_preview_state.set(false);
                            focus_by_id(&format!("preview-trigger-{}", *keyboard_active_card));
                        }
                        "Tab" => {
                            event.prevent_default();
                            let active_id = web_sys::window()
                                .and_then(|window| window.document())
                                .and_then(|document| document.active_element())
                                .and_then(|element| element.get_attribute("id"));
                            focus_preview_control(active_id, event.shift_key());
                        }
                        _ => {}
                    });
                    let preview_mode = preview_mode.clone();
                    let view_mode = (*preview_mode).clone();
                    let toggle_view = Callback::from(move |_| {
                        preview_mode.set(if *preview_mode == "Fit to pane" { "Original size".to_owned() } else { "Fit to pane".to_owned() })
                    });
                    html! {
                        <section
                            ref={preview_ref.clone()}
                            class="ximg-preview"
                            role="dialog"
                            aria-modal="true"
                            aria-labelledby="preview-title"
                            aria-describedby="preview-summary"
                            tabindex="-1"
                            onkeydown={on_keydown}
                        >
                            <div class="ximg-preview__pane">
                                <div class="ximg-preview__heading">
                                    <div><p class="ximg-shell__eyebrow">{ "Selected record" }</p><h2 id="preview-title">{ selected_card.title.clone() }</h2></div>
                                    <button id="preview-close" autofocus=true onclick={close_preview}>{ "Close preview" }</button>
                                </div>
                                <p id="preview-summary">{ format!("{} · {} · {}", media_label(&selected_card), object_label(&selected_card), review_label(&selected_card)) }</p>
                                <div class="ximg-preview__layout">
                                    <section class={classes!("ximg-preview__visual", (view_mode == "Original size").then_some("is-original"))} aria-label="Media visual">
                                        { if selected_card.media_kind == GalleryMediaKind::Image {
                                            if let Some(path) = selected_card.preview.as_ref().and_then(ready_path) {
                                                html! { <img class="ximg-preview__image" src={path.to_owned()} alt={selected_card.title.clone()} /> }
                                            } else {
                                                html! { <div class="ximg-preview__unavailable" role="status"><strong>{ "Original image unavailable" }</strong><p>{ "Pinakotheke does not fall back to the source website." }</p></div> }
                                            }
                                        } else if let Some(path) = selected_card.preview.as_ref().and_then(ready_path) {
                                            html! {
                                                <video controls=true preload="metadata" src={path.to_owned()} aria-label={format!("Play {}", selected_card.title)}>
                                                    { "Your browser cannot play the verified normalized video." }
                                                </video>
                                            }
                                        } else {
                                            html! { <div class="ximg-preview__unavailable" role="status"><strong>{ "Normalized video unavailable" }</strong><p>{ "Pinakotheke does not fall back to the source website." }</p></div> }
                                        }}
                                        <button id="preview-view-mode" aria-pressed={(view_mode == "Original size").to_string()} onclick={toggle_view}>
                                            { if view_mode == "Fit to pane" { "View original size" } else { "Fit to pane" } }
                                        </button>
                                    </section>
                                    <aside class="ximg-preview__details" aria-label="Selected media details">
                                        <dl>
                                            <div><dt>{ "Source" }</dt><dd>{ selected_card.source_label.clone() }</dd></div>
                                            <div><dt>{ "Media type" }</dt><dd>{ media_label(&selected_card) }</dd></div>
                                            <div><dt>{ "Object state" }</dt><dd>{ object_label(&selected_card) }</dd></div>
                                            <div><dt>{ "Dimensions" }</dt><dd>{ format!("{} × {}", selected_card.width, selected_card.height) }</dd></div>
                                            <div><dt>{ "Endpoint / ObjectStore" }</dt><dd>{ format!("{} / {}", selected_card.thumbnail.endpoint_id, selected_card.thumbnail.object_store_id) }</dd></div>
                                        </dl>
                                        <a id="preview-source-link" href={format!("#catalogue-{}", selected_card.catalogue_id)}>{ "View catalogue metadata" }</a>
                                        { if object_label(&selected_card) == "Object unavailable" {
                                            html! {
                                                <section class="ximg-preview__unavailable" role="status" aria-live="polite">
                                                    <h3>{ "Object unavailable" }</h3>
                                                    <p>{ "The committed object cannot be read. Pinakotheke does not fall back to the source URL." }</p>
                                                </section>
                                            }
                                        } else {
                                            html! {
                                                <section class="ximg-preview__playback" role="status">
                                                    <h3>{ "Authorized ObjectStore delivery" }</h3>
                                                    <p>{ "The displayed media uses the verified host-local delivery path above." }</p>
                                                </section>
                                            }
                                        }}
                                    </aside>
                                </div>
                            </div>
                        </section>
                    }
                } else { Html::default() } }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn representation(
        availability: GalleryObjectAvailability,
        path: Option<&str>,
    ) -> GalleryRepresentation {
        GalleryRepresentation {
            kind: x_img_core::gallery_catalogue::GalleryRepresentationKind::Thumbnail,
            availability,
            endpoint_id: "endpoint-1".into(),
            object_store_id: "store-1".into(),
            object_key: "objects/media-1".into(),
            checksum: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .into(),
            content_type: "image/jpeg".into(),
            content_length: 12,
            delivery_path: path.map(Into::into),
        }
    }

    fn item() -> GalleryItem {
        GalleryItem {
            catalogue_id: "media-1".into(),
            title: "Synthetic redistributable image".into(),
            source_label: "Example website".into(),
            source_kind: GallerySourceKind::Website,
            media_kind: GalleryMediaKind::Image,
            review_state: GalleryReviewState::New,
            discovered_at_epoch_seconds: 1,
            width: 320,
            height: 200,
            thumbnail: representation(
                GalleryObjectAvailability::Ready,
                Some("/products/pinakotheke/api/gallery/v1/objects/thumbnail-1"),
            ),
            preview: None,
        }
    }

    #[test]
    fn gallery_helpers_never_make_an_unavailable_object_renderable() {
        let mut media = item();
        assert_eq!(object_label(&media), "Previously observed");
        assert_eq!(
            ready_path(&media.thumbnail),
            Some("/products/pinakotheke/api/gallery/v1/objects/thumbnail-1")
        );
        media.thumbnail = representation(GalleryObjectAvailability::Unavailable, None);
        assert_eq!(object_label(&media), "Object unavailable");
        assert_eq!(ready_path(&media.thumbnail), None);
        assert!(source_matches("websites", &media));
        assert!(!source_matches("x", &media));
    }
}
