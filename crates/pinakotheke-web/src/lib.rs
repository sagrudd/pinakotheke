// SPDX-License-Identifier: MPL-2.0
//! Mnemosyne-compatible, host-integrable Yew application shell.

use gloo_net::http::Request;
use serde::Deserialize;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlElement, HtmlInputElement, HtmlSelectElement, KeyboardEvent};
use x_img_core::gallery_catalogue::{
    GALLERY_CATALOGUE_SCHEMA, GalleryItem, GalleryMediaKind, GalleryObjectAvailability,
    GalleryRepresentation, GalleryReviewState, GallerySourceKind,
};
use yew::prelude::*;

/// Starts the browser application when loaded from the Trunk-built module.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run() {
    yew::Renderer::<App>::new().render();
}

const GALLERY_API: &str = "/products/pinakotheke/api/gallery/v1/catalogue";
const OBJECT_STORE_API: &str = "/products/dasobjectstore/api/v1/dashboard/object-stores";
const EXTENSION_ONBOARDING_API: &str = "/products/pinakotheke/api/extension/v1/onboarding";
const INGESTION_STATUS_API: &str = "/products/pinakotheke/api/ingestion/v1/status";
const GALLERY_PAGE_SIZE: usize = 100;
const GALLERY_WINDOW_ROWS: usize = 8;
const GALLERY_OVERSCAN_ROWS: usize = 2;
const COMPACT_ROW_HEIGHT: usize = 224;
const COMFORTABLE_ROW_HEIGHT: usize = 304;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GalleryWindow {
    start: usize,
    end: usize,
    top_padding: usize,
    bottom_padding: usize,
    columns: usize,
    row_height: usize,
}

fn gallery_window(
    total: usize,
    scroll_top: usize,
    viewport_width: usize,
    density: &str,
) -> GalleryWindow {
    let (minimum_card_width, row_height) = if density == "comfortable" {
        (224, COMFORTABLE_ROW_HEIGHT)
    } else {
        (144, COMPACT_ROW_HEIGHT)
    };
    let columns = (viewport_width / (minimum_card_width + 12)).max(1);
    let total_rows = total.div_ceil(columns);
    let first_visible_row = (scroll_top / row_height).min(total_rows.saturating_sub(1));
    let start_row = first_visible_row.saturating_sub(GALLERY_OVERSCAN_ROWS);
    let end_row = (first_visible_row + GALLERY_WINDOW_ROWS + GALLERY_OVERSCAN_ROWS).min(total_rows);
    GalleryWindow {
        start: (start_row * columns).min(total),
        end: (end_row * columns).min(total),
        top_padding: start_row * row_height,
        bottom_padding: total_rows.saturating_sub(end_row) * row_height,
        columns,
        row_height,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct GalleryPageResponse {
    schema_version: String,
    items: Vec<GalleryItem>,
    next_offset: Option<usize>,
    matched_items: usize,
    total_items: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ObjectStoreDashboard {
    stores: Vec<ObjectStoreRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ObjectStoreRow {
    store_id: String,
    display_name: String,
    health: String,
    writeable: bool,
    writer_policy: ObjectStoreWriterPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ObjectStoreWriterPolicy {
    writeable_by_current_user: bool,
    state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ObjectStoreLoadState {
    Loading,
    Ready(Vec<ObjectStoreRow>),
    PermissionDenied,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtensionOnboarding {
    schema_version: String,
    instance_id: String,
    pairing_reference: String,
    dasobjectstore_status: String,
    endpoint_id: String,
    object_store_id: String,
    extension_download_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExtensionOnboardingState {
    Loading,
    Ready(ExtensionOnboarding),
    PrerequisiteMissing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GalleryLoadState {
    Loading,
    Ready {
        items: Vec<GalleryItem>,
        next_offset: Option<usize>,
        matched_items: usize,
        total_items: usize,
    },
    PermissionDenied,
    TransportError,
    InvalidResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct IngestionStatus {
    schema_version: String,
    observed_assets: usize,
    observed_thumbnails: usize,
    opened_originals: usize,
    opened_videos: usize,
    pending: usize,
    stored: usize,
    gallery_items: usize,
    last_observed_at_epoch_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum IngestionStatusState {
    Loading,
    Ready(IngestionStatus),
    Unavailable,
}

async fn fetch_ingestion_status() -> IngestionStatusState {
    match Request::get(INGESTION_STATUS_API).send().await {
        Ok(response) if response.ok() => response
            .json::<IngestionStatus>()
            .await
            .ok()
            .filter(|status| status.schema_version == "pinakotheke.ingestion-status.v1")
            .map(IngestionStatusState::Ready)
            .unwrap_or(IngestionStatusState::Unavailable),
        Ok(_) | Err(_) => IngestionStatusState::Unavailable,
    }
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

fn encode_query(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(*byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn gallery_url(offset: usize, selected: &str, text: &str) -> String {
    let mut url = format!("{GALLERY_API}?offset={offset}&limit={GALLERY_PAGE_SIZE}");
    match selected {
        "x" => url.push_str("&source_kind=x_account"),
        "websites" => url.push_str("&source_kind=website"),
        _ => {}
    }
    if !text.trim().is_empty() {
        url.push_str("&text=");
        url.push_str(&encode_query(text.trim()));
    }
    url
}

fn initial_viewport_width() -> usize {
    web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|width| width.as_f64())
        .map_or(1024, |width| width.max(1.0) as usize)
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
    let request_generation = use_mut_ref(|| 0_u64);
    let gallery_scroll_top = use_state(|| 0_usize);
    let gallery_viewport_width = use_state(initial_viewport_width);
    let keyboard_focus_pending = use_state(|| false);
    let object_stores = use_state(|| ObjectStoreLoadState::Loading);
    let selected_object_store = use_state(String::new);
    let extension_onboarding = use_state(|| ExtensionOnboardingState::Loading);
    let ingestion_status = use_state(|| IngestionStatusState::Loading);

    {
        let ingestion_status = ingestion_status.clone();
        use_effect_with((), move |()| {
            let refresh = {
                let ingestion_status = ingestion_status.clone();
                move || {
                    let ingestion_status = ingestion_status.clone();
                    spawn_local(async move {
                        ingestion_status.set(fetch_ingestion_status().await);
                    });
                }
            };
            refresh();
            let callback = Closure::<dyn FnMut()>::new(refresh);
            let interval = web_sys::window().and_then(|window| {
                window
                    .set_interval_with_callback_and_timeout_and_arguments_0(
                        callback.as_ref().unchecked_ref(),
                        3_000,
                    )
                    .ok()
                    .map(|id| (window, id))
            });
            move || {
                if let Some((window, id)) = interval {
                    window.clear_interval_with_handle(id);
                }
                drop(callback);
            }
        });
    }

    {
        let extension_onboarding = extension_onboarding.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                let state = match Request::get(EXTENSION_ONBOARDING_API).send().await {
                    Ok(response) if response.ok() => response
                        .json::<ExtensionOnboarding>()
                        .await
                        .ok()
                        .filter(|payload| {
                            payload.schema_version == "pinakotheke.extension-onboarding.v1"
                                && payload.dasobjectstore_status == "Ready"
                        })
                        .map(ExtensionOnboardingState::Ready)
                        .unwrap_or(ExtensionOnboardingState::PrerequisiteMissing),
                    _ => ExtensionOnboardingState::PrerequisiteMissing,
                };
                extension_onboarding.set(state);
            });
            || ()
        });
    }

    {
        let object_stores = object_stores.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                let state = match Request::get(OBJECT_STORE_API).send().await {
                    Ok(response) if matches!(response.status(), 401 | 403) => {
                        ObjectStoreLoadState::PermissionDenied
                    }
                    Ok(response) if response.ok() => response
                        .json::<ObjectStoreDashboard>()
                        .await
                        .map(|dashboard| ObjectStoreLoadState::Ready(dashboard.stores))
                        .unwrap_or(ObjectStoreLoadState::Unavailable),
                    Ok(_) | Err(_) => ObjectStoreLoadState::Unavailable,
                };
                object_stores.set(state);
            });
            || ()
        });
    }

    {
        let gallery_viewport_width = gallery_viewport_width.clone();
        use_effect_with((), move |()| {
            let callback = Closure::<dyn FnMut(Event)>::new(move |_| {
                gallery_viewport_width.set(initial_viewport_width());
            });
            let window = web_sys::window();
            if let Some(window) = &window {
                let _ = window
                    .add_event_listener_with_callback("resize", callback.as_ref().unchecked_ref());
            }
            move || {
                if let Some(window) = window {
                    let _ = window.remove_event_listener_with_callback(
                        "resize",
                        callback.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }

    {
        let gallery = gallery.clone();
        let active_card = active_card.clone();
        let request_generation = request_generation.clone();
        let gallery_scroll_top_effect = gallery_scroll_top.clone();
        use_effect_with(
            ((*selected).clone(), (*filter).clone()),
            move |(selected, filter)| {
                let url = gallery_url(0, selected, filter);
                let generation = {
                    let mut current = request_generation.borrow_mut();
                    *current += 1;
                    *current
                };
                gallery.set(GalleryLoadState::Loading);
                active_card.set(0);
                gallery_scroll_top_effect.set(0);
                spawn_local(async move {
                    let state = match Request::get(&url).send().await {
                        Ok(response) if matches!(response.status(), 401 | 403) => {
                            GalleryLoadState::PermissionDenied
                        }
                        Ok(response) if response.ok() => {
                            match response.json::<GalleryPageResponse>().await {
                                Ok(page) if page.schema_version == GALLERY_CATALOGUE_SCHEMA => {
                                    GalleryLoadState::Ready {
                                        items: page.items,
                                        next_offset: page.next_offset,
                                        matched_items: page.matched_items,
                                        total_items: page.total_items,
                                    }
                                }
                                _ => GalleryLoadState::InvalidResponse,
                            }
                        }
                        Ok(_) | Err(_) => GalleryLoadState::TransportError,
                    };
                    if *request_generation.borrow() == generation {
                        gallery.set(state);
                    }
                });
                || ()
            },
        );
    }

    let items = match &*gallery {
        GalleryLoadState::Ready { items, .. } => items.as_slice(),
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
    {
        let keyboard_focus_pending = keyboard_focus_pending.clone();
        use_effect_with(*active_card, move |index| {
            if *keyboard_focus_pending {
                focus_by_id(&format!("preview-trigger-{index}"));
                keyboard_focus_pending.set(false);
            }
            || ()
        });
    }
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
                <h1>{ "Pinakotheke library" }</h1>
                <p>{ "Review committed media from configured sources." }</p>

                <section class="ximg-ingestion-status" aria-labelledby="ingestion-status-title" aria-live="polite">
                    <h2 id="ingestion-status-title">{ "Ingress status" }</h2>
                    { match &*ingestion_status {
                        IngestionStatusState::Loading => html! { <p role="status">{ "Loading observed-media activity…" }</p> },
                        IngestionStatusState::Unavailable => html! { <p role="alert">{ "Ingress activity is unavailable. Captures may continue; reload or check the Pinakotheke service." }</p> },
                        IngestionStatusState::Ready(status) => html! {
                            <>
                                <dl class="ximg-ingestion-status__metrics">
                                    <div><dt>{ "Observed" }</dt><dd>{ status.observed_assets }</dd></div>
                                    <div><dt>{ "Thumbnails" }</dt><dd>{ status.observed_thumbnails }</dd></div>
                                    <div><dt>{ "Opened images" }</dt><dd>{ status.opened_originals }</dd></div>
                                    <div><dt>{ "Opened videos" }</dt><dd>{ status.opened_videos }</dd></div>
                                    <div><dt>{ "Pending" }</dt><dd>{ status.pending }</dd></div>
                                    <div><dt>{ "Stored" }</dt><dd>{ status.stored }</dd></div>
                                    <div><dt>{ "Gallery items" }</dt><dd>{ status.gallery_items }</dd></div>
                                </dl>
                                <p>{
                                    if status.observed_assets == 0 {
                                        "No media has been observed for this user.".to_owned()
                                    } else if status.pending > 0 {
                                        format!("{} asset(s) are awaiting acquisition or ObjectStore verification.", status.pending)
                                    } else {
                                        "All observed assets have reached a terminal stored state.".to_owned()
                                    }
                                }</p>
                            </>
                        },
                    }}
                </section>

                <section class="ximg-destination" aria-labelledby="firefox-setup-title">
                    <h2 id="firefox-setup-title">{ "Connect Firefox" }</h2>
                    <p>{ "Pinakotheke exposes its signed extension only after Monas authentication and a named DASObjectStore destination are ready." }</p>
                    { match &*extension_onboarding {
                        ExtensionOnboardingState::Loading => html! { <p role="status">{ "Checking Pinakotheke and DASObjectStore prerequisites…" }</p> },
                        ExtensionOnboardingState::PrerequisiteMissing => html! { <p role="alert">{ "Firefox setup is unavailable. Confirm DASObjectStore availability, select a named writable ObjectStore, and configure capture authority." }</p> },
                        ExtensionOnboardingState::Ready(setup) => html! {
                            <div class="ximg-task-pane">
                                <p><strong>{ "DASObjectStore: Ready" }</strong></p>
                                <dl>
                                    <div><dt>{ "Endpoint" }</dt><dd>{ setup.endpoint_id.clone() }</dd></div>
                                    <div><dt>{ "Named ObjectStore" }</dt><dd>{ setup.object_store_id.clone() }</dd></div>
                                </dl>
                                <p><a href={setup.extension_download_path.clone()}>{ "Download signed Pinakotheke extension" }</a></p>
                                <label>{ "Instance identifier" }<input readonly=true value={setup.instance_id.clone()} /></label>
                                <label>{ "Pairing reference" }<input readonly=true value={setup.pairing_reference.clone()} /></label>
                                <p>{ "Open the extension settings, enter this page's HTTPS origin and the two values above, then choose Pair. Firefox verifies them against this authenticated Monas session before saving them." }</p>
                            </div>
                        },
                    }}
                </section>

                <section class="ximg-destination" aria-labelledby="destination-title">
                    <h2 id="destination-title">{ "Storage destination" }</h2>
                    <p>{ "Choose the DASObjectStore that Pinakotheke should present for reviewed capture plans. The server revalidates the destination before any commit." }</p>
                    { match &*object_stores {
                        ObjectStoreLoadState::Loading => html! { <p role="status">{ "Loading ObjectStores…" }</p> },
                        ObjectStoreLoadState::PermissionDenied => html! { <p role="alert">{ "Monas did not authorize ObjectStore discovery." }</p> },
                        ObjectStoreLoadState::Unavailable => html! { <p role="alert">{ "DASObjectStore inventory is unavailable. Media browsing remains available." }</p> },
                        ObjectStoreLoadState::Ready(stores) => html! {
                            <>
                                <label for="object-store-select">{ "DASServer · ObjectStore" }</label>
                                <select
                                    id="object-store-select"
                                    value={(*selected_object_store).clone()}
                                    onchange={{
                                        let selected_object_store = selected_object_store.clone();
                                        Callback::from(move |event: Event| {
                                            selected_object_store.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                        })
                                    }}
                                >
                                    <option value="">{ "Select an ObjectStore…" }</option>
                                    { for stores.iter().map(|store| {
                                        let ready = store.health == "healthy" && store.writeable && store.writer_policy.writeable_by_current_user;
                                        html! {
                                            <option value={store.store_id.clone()} disabled={!ready}>
                                                { format!("{} · {}", store.display_name, if ready { "Ready" } else { "Read-only or unavailable" }) }
                                            </option>
                                        }
                                    }) }
                                </select>
                                <p role="status" aria-live="polite">{
                                    if selected_object_store.is_empty() {
                                        "No ObjectStore selected for this review session.".to_owned()
                                    } else {
                                        format!("Selected: DASServer · {}. This choice will be shown during capture review.", *selected_object_store)
                                    }
                                }</p>
                            </>
                        },
                    }}
                </section>

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
                    <p>{if filter.is_empty() { "Filters: all catalogue records" } else { "Filter active: server-matched catalogue metadata" }}</p>
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
                        GalleryLoadState::Ready { items: records, .. } if records.is_empty() => html! {
                            <div class="ximg-gallery__state" role="status">
                                <h3>{ "No committed media" }</h3>
                                <p>{ "Media appears after Firefox capture and verified DASObjectStore admission." }</p>
                            </div>
                        },
                        GalleryLoadState::Ready { items: records, next_offset, matched_items, total_items } => html! {
                            <>
                            <p role="status">{ format!("Loaded {} of {} catalogue records ({} total before server filters)", records.len(), matched_items, total_items) }</p>
                            {{
                                let window = gallery_window(records.len(), *gallery_scroll_top, *gallery_viewport_width, &density);
                                let window_contains_active = (window.start..window.end).contains(&*active_card);
                                let onscroll = {
                                    let gallery_scroll_top = gallery_scroll_top.clone();
                                    let gallery_viewport_width = gallery_viewport_width.clone();
                                    Callback::from(move |event: Event| {
                                        let element = event.target_unchecked_into::<HtmlElement>();
                                        gallery_scroll_top.set(element.scroll_top().max(0) as usize);
                                        gallery_viewport_width.set(element.client_width().max(1) as usize);
                                    })
                                };
                                html! { <div id="gallery-scroll" class="ximg-gallery__viewport" {onscroll} tabindex="-1">
                            <div
                                class={classes!("ximg-gallery__grid", format!("is-{}", *density))}
                                style={format!("--gallery-columns:{}", window.columns)}
                            >
                                <div class="ximg-gallery__spacer" style={format!("height:{}px", window.top_padding)} aria-hidden="true"></div>
                                { for records[window.start..window.end].iter().enumerate().map(|(relative_index, item)| {
                                    let index = window.start + relative_index;
                                    let active_card = active_card.clone();
                                    let click_active_card = active_card.clone();
                                    let focus_active_card = active_card.clone();
                                    let preview_open = preview_open.clone();
                                    let preview_mode = preview_mode.clone();
                                    let gallery_scroll_top = gallery_scroll_top.clone();
                                    let keyboard_focus_pending = keyboard_focus_pending.clone();
                                    let is_selected = index == *active_card;
                                    let thumbnail_path = ready_path(&item.thumbnail).map(ToOwned::to_owned);
                                    let record_count = records.len();
                                    let columns = window.columns;
                                    let row_height = window.row_height;
                                    html! {
                                        <button
                                            id={format!("preview-trigger-{index}")}
                                            class={classes!("ximg-gallery__card", is_selected.then_some("is-selected"))}
                                            aria-haspopup="dialog"
                                            aria-pressed={is_selected.to_string()}
                                            tabindex={if is_selected || (!window_contains_active && index == window.start) { "0" } else { "-1" }}
                                            onfocus={Callback::from(move |_| focus_active_card.set(index))}
                                            onkeydown={Callback::from(move |event: KeyboardEvent| {
                                                let target = match event.key().as_str() {
                                                    "ArrowRight" => (index + 1).min(record_count - 1),
                                                    "ArrowLeft" => index.saturating_sub(1),
                                                    "ArrowDown" => (index + columns).min(record_count - 1),
                                                    "ArrowUp" => index.saturating_sub(columns),
                                                    "Home" => 0,
                                                    "End" => record_count - 1,
                                                    _ => return,
                                                };
                                                event.prevent_default();
                                                let target_top = (target / columns) * row_height;
                                                if let Some(element) = web_sys::window()
                                                    .and_then(|window| window.document())
                                                    .and_then(|document| document.get_element_by_id("gallery-scroll"))
                                                    .and_then(|element| element.dyn_into::<HtmlElement>().ok())
                                                {
                                                    element.set_scroll_top(target_top as i32);
                                                }
                                                gallery_scroll_top.set(target_top);
                                                keyboard_focus_pending.set(true);
                                                active_card.set(target);
                                            })}
                                            onclick={Callback::from(move |_| {
                                                click_active_card.set(index);
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
                                <div class="ximg-gallery__spacer" style={format!("height:{}px", window.bottom_padding)} aria-hidden="true"></div>
                            </div>
                            </div> }
                            }}
                            { if let Some(offset) = *next_offset {
                                let gallery = gallery.clone();
                                let selected = (*selected).clone();
                                let filter = (*filter).clone();
                                let request_generation = request_generation.clone();
                                html! {
                                    <button class="ximg-gallery__more" onclick={Callback::from(move |_| {
                                        let gallery = gallery.clone();
                                        let url = gallery_url(offset, &selected, &filter);
                                        let request_generation = request_generation.clone();
                                        let generation = *request_generation.borrow();
                                        spawn_local(async move {
                                            let Ok(response) = Request::get(&url).send().await else { return; };
                                            let Ok(page) = response.json::<GalleryPageResponse>().await else { return; };
                                            if page.schema_version != GALLERY_CATALOGUE_SCHEMA { return; }
                                            if *request_generation.borrow() != generation { return; }
                                            if let GalleryLoadState::Ready { items, .. } = &*gallery {
                                                let mut combined = items.clone();
                                                combined.extend(page.items);
                                                gallery.set(GalleryLoadState::Ready {
                                                    items: combined,
                                                    next_offset: page.next_offset,
                                                    matched_items: page.matched_items,
                                                    total_items: page.total_items,
                                                });
                                            }
                                        });
                                    })}>{ "Load next 100 records" }</button>
                                }
                            } else { Html::default() } }
                            </>
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
                                            <div><dt>{ "Object version" }</dt><dd>{ selected_card.thumbnail.object_version }</dd></div>
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
            object_version: 1,
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
    }

    #[test]
    fn catalogue_queries_preserve_bounded_server_filters_across_pages() {
        assert_eq!(
            gallery_url(100, "websites", " calm ocean & film "),
            "/products/pinakotheke/api/gallery/v1/catalogue?offset=100&limit=100&source_kind=website&text=calm%20ocean%20%26%20film"
        );
        assert_eq!(
            gallery_url(0, "x", ""),
            "/products/pinakotheke/api/gallery/v1/catalogue?offset=0&limit=100&source_kind=x_account"
        );
    }

    #[test]
    fn gallery_window_bounds_a_large_catalogue_and_preserves_virtual_height() {
        let first = gallery_window(10_000, 0, 1_024, "compact");
        assert_eq!(first.start, 0);
        assert!(first.end <= 60);
        assert_eq!(first.top_padding, 0);
        assert!(first.bottom_padding > 300_000);

        let middle = gallery_window(10_000, 100_000, 1_024, "compact");
        assert!(middle.start > 0);
        assert!(middle.end - middle.start <= 72);
        assert!(middle.top_padding > 0);
        assert!(middle.bottom_padding > 0);

        let end = gallery_window(10_000, usize::MAX, 1_024, "compact");
        assert_eq!(end.end, 10_000);
        assert_eq!(end.bottom_padding, 0);
    }

    #[test]
    fn gallery_window_reflows_for_mobile_and_density() {
        let mobile = gallery_window(500, 0, 320, "compact");
        assert_eq!(mobile.columns, 2);
        assert_eq!(mobile.row_height, COMPACT_ROW_HEIGHT);

        let comfortable = gallery_window(500, 0, 1_024, "comfortable");
        assert_eq!(comfortable.columns, 4);
        assert_eq!(comfortable.row_height, COMFORTABLE_ROW_HEIGHT);
    }
}
