// SPDX-License-Identifier: MPL-2.0
// No page cookies, response interception, credentials, or automatic navigation.
async function initializeStorage(details) {
  if (details.reason === "install") {
    const existing = await browser.storage.local.get(["instanceUrl", "instanceId", "pairId", "sites"]);
    const defaults = { instanceUrl: "", instanceId: "", pairId: "", sites: [] };
    const missing = Object.fromEntries(
      Object.entries(defaults).filter(([key]) => existing[key] === undefined),
    );
    if (Object.keys(missing).length) await browser.storage.local.set(missing);
  }
  await syncExplicitOpenObservers();
  await syncSiteCorpusFromServer();
}

browser.runtime.onInstalled.addListener(initializeStorage);
browser.runtime.onStartup.addListener(syncExplicitOpenObservers);

const EXPLICIT_OPEN_SCRIPT_ID = "pinakotheke-explicit-open-v1";
const captureInFlight = new Set();
let segmentedManifests = [];

async function recordMediaCapture(origin, planId, kind, state, detail) {
  if (kind !== "explicit_video") return;
  const stored = await browser.storage.local.get(["mediaCaptureStates"]);
  const entries = Array.isArray(stored.mediaCaptureStates) ? stored.mediaCaptureStates : [];
  const next = entries.filter(entry => entry.planId !== planId);
  next.push({ planId, origin, kind: "Video", state, detail, updatedAt: new Date().toISOString() });
  await browser.storage.local.set({ mediaCaptureStates: next.slice(-24) });
}

async function notifyCaptureState(tabId, media, state, label) {
  try {
    await browser.tabs.sendMessage(tabId, {
      command: "media-capture-state", mediaUrl: media.url,
      mediaToken: media.mediaToken, state, label,
    });
  } catch (_) { /* the originating tab may have closed */ }
}

async function refreshMediaCaptureStates() {
  const stored = await browser.storage.local.get(["instanceUrl", "pairId", "mediaCaptureStates"]);
  if (!stored.instanceUrl || !stored.pairId) return;
  const entries = Array.isArray(stored.mediaCaptureStates) ? stored.mediaCaptureStates : [];
  await Promise.all(entries.filter(entry => !["stored", "failed"].includes(entry.state)).map(async entry => {
    try {
      const response = await fetch(`${stored.instanceUrl}/products/pinakotheke/api/extension/v1/capture-plans/${encodeURIComponent(entry.planId)}`, {
        cache: "no-store", headers: { "x-pinakotheke-pairing": stored.pairId },
      });
      if (!response.ok) return;
      const status = await response.json();
      if (status.schema_version === "pinakotheke.capture-plan-status.v1" && status.state === "stored") {
        await recordMediaCapture(entry.origin, entry.planId, "explicit_video", "stored", "Available in DASObjectStore");
      }
    } catch (_) { /* popup refresh is advisory and fails open */ }
  }));
}

function segmentedMediaFamily(raw) {
  try {
    const url = new URL(raw);
    const match = url.pathname.match(/^\/(amplify_video|ext_tw_video)\/([^/]+)\//);
    return match ? `${url.hostname}/${match[1]}/${match[2]}` : null;
  } catch (_) {
    return null;
  }
}

async function observeSegmentedRequest(details) {
  let candidate;
  try {
    candidate = new URL(details.url);
  } catch (_) {
    return;
  }
  if (candidate.protocol !== "https:" || candidate.hostname !== "video.twimg.com"
    || !/\.(?:m3u8|mpd)$/i.test(candidate.pathname)) return;
  const { sites = [] } = await browser.storage.local.get(["sites"]);
  const rule = sites.find(site => site.origin === "https://x.com");
  if (!rule?.capture || !rule.media.includes("videos")) return;
  if (Number.isInteger(details.tabId) && details.tabId >= 0) {
    let tab;
    try {
      tab = await browser.tabs.get(details.tabId);
    } catch (_) {
      return;
    }
    if (!tab?.url || new URL(tab.url).origin !== "https://x.com") return;
  }
  const now = Date.now();
  segmentedManifests = segmentedManifests
    .filter(item => now - item.observedAt < 120000);
  segmentedManifests.push({
    url: candidate.href,
    family: segmentedMediaFamily(candidate.href),
    tabId: Number.isInteger(details.tabId) ? details.tabId : -1,
    observedAt: now,
  });
  segmentedManifests = segmentedManifests.slice(-32);
  await traceEvent("segmented_manifest", "observed", "completed manifest URL retained", "https://x.com");
}

browser.webRequest.onCompleted.addListener(
  details => void observeSegmentedRequest(details),
  { urls: ["https://video.twimg.com/*"] },
);

function recentSegmentedManifest(tabId, mediaFamilies) {
  const now = Date.now();
  const allowedFamilies = new Set(Array.isArray(mediaFamilies) ? mediaFamilies.slice(0, 8) : []);
  segmentedManifests = segmentedManifests
    .filter(item => now - item.observedAt < 120000);
  const recent = segmentedManifests.filter(item =>
    (item.tabId < 0 || item.tabId === tabId)
      && (!allowedFamilies.size || (item.family && allowedFamilies.has(item.family))));
  return recent.sort((left, right) => {
    const leftDepth = new URL(left.url).pathname.split("/").length;
    const rightDepth = new URL(right.url).pathname.split("/").length;
    return leftDepth - rightDepth || right.observedAt - left.observedAt;
  })[0]?.url || null;
}

async function traceEvent(stage, outcome, detail = "", origin = "") {
  try {
    const stored = await browser.storage.local.get(["diagnosticEvents"]);
    const events = Array.isArray(stored.diagnosticEvents) ? stored.diagnosticEvents : [];
    events.push({ at: new Date().toISOString(), stage, outcome, detail: String(detail).slice(0, 160), origin });
    await browser.storage.local.set({ diagnosticEvents: events.slice(-100) });
  } catch (_) {
    // Diagnostics must never interfere with browsing or capture.
  }
}

function localRuleFromServer(rule) {
  return {
    origin: rule.origin,
    capture: rule.capture,
    substitution: rule.substitution,
    xIngress: rule.x_ingress,
    media: [...(rule.images ? ["images"] : []), ...(rule.videos ? ["videos"] : [])],
  };
}

async function syncSiteCorpusFromServer() {
  const stored = await browser.storage.local.get(["instanceUrl", "pairId", "siteCorpusRevision", "sites"]);
  if (!stored.instanceUrl || !stored.pairId) return { outcome: "not_paired" };
  try {
    const response = await fetch(`${stored.instanceUrl}/products/pinakotheke/api/extension/v1/site-corpus`, {
      cache: "no-store",
      headers: { "x-pinakotheke-pairing": stored.pairId },
    });
    if (!response.ok) return { outcome: "unavailable" };
    const corpus = await response.json();
    if (corpus.schema_version !== "pinakotheke.site-corpus.v1"
      || !Number.isSafeInteger(corpus.revision) || !Array.isArray(corpus.rules)) return { outcome: "invalid" };
    if (corpus.revision === 0 && corpus.rules.length === 0 && (stored.sites || []).length > 0) {
      const upload = await fetch(`${stored.instanceUrl}/products/pinakotheke/api/extension/v1/site-corpus`, {
        method: "POST",
        cache: "no-store",
        headers: { "content-type": "application/json", "x-pinakotheke-pairing": stored.pairId },
        body: JSON.stringify({
          schema_version: "pinakotheke.site-corpus.v1",
          expected_revision: 0,
          rules: stored.sites.map(rule => ({
            origin: rule.origin,
            images: rule.media.includes("images"),
            videos: rule.media.includes("videos"),
            capture: rule.capture,
            substitution: rule.substitution,
            x_ingress: Boolean(rule.xIngress),
          })),
        }),
      });
      if (upload.ok) {
        const saved = await upload.json();
        await browser.storage.local.set({ siteCorpusRevision: saved.revision });
        return { outcome: "uploaded", revision: saved.revision };
      }
      if (upload.status === 409) return syncSiteCorpusFromServer();
      return { outcome: "unavailable" };
    }
    await browser.storage.local.set({
      sites: corpus.rules.map(localRuleFromServer),
      siteCorpusRevision: corpus.revision,
    });
    await syncExplicitOpenObservers();
    return { outcome: "synchronized", revision: corpus.revision };
  } catch (_) {
    return { outcome: "unavailable" };
  }
}

function originMatchPattern(rawOrigin) {
  const origin = new URL(rawOrigin);
  if (origin.protocol !== "https:") throw new Error("only HTTPS origins are supported");
  return `https://${origin.hostname}/*`;
}

async function matchingAdapter(url) {
  const registry = await fetch(browser.runtime.getURL("adapters.json")).then(response => response.json());
  const target = new URL(url);
  return registry.adapters.find(adapter =>
    (adapter.origins.includes(target.origin) || adapter.kind === "experimental_generic")
      && !adapter.exclude_paths.some(path => target.pathname.startsWith(path))
      && (adapter.capabilities.observed_thumbnail
        || adapter.capabilities.explicit_original
        || adapter.capabilities.image_substitution
        || adapter.capabilities.mp4_substitution),
  ) || null;
}

async function syncExplicitOpenObservers() {
  const registered = await browser.scripting.getRegisteredContentScripts({
    ids: [EXPLICIT_OPEN_SCRIPT_ID],
  });
  if (registered.length) {
    await browser.scripting.unregisterContentScripts({ ids: [EXPLICIT_OPEN_SCRIPT_ID] });
  }
  const { sites = [] } = await browser.storage.local.get(["sites"]);
  const eligible = [];
  for (const site of sites) {
    if (!site.capture || !site.media?.some(kind => kind === "images" || kind === "videos")) continue;
    const adapter = await matchingAdapter(site.origin);
    if (adapter?.capabilities.explicit_original) eligible.push({ site, adapter });
  }
  if (!eligible.length) {
    await traceEvent("observer_registration", "none", "no enabled image/video capture rules");
    return { registered: 0 };
  }
  await browser.scripting.registerContentScripts([{
    id: EXPLICIT_OPEN_SCRIPT_ID,
    js: ["content-explicit-open.js"],
    matches: eligible.map(({ site }) => originMatchPattern(site.origin)),
    excludeMatches: eligible.flatMap(({ site, adapter }) =>
      adapter.exclude_paths.map(path => `${originMatchPattern(site.origin).slice(0, -2)}${path}*`)),
    allFrames: false,
    persistAcrossSessions: true,
    runAt: "document_idle",
  }]);
  let injected = 0;
  for (const tab of await browser.tabs.query({})) {
    if (!tab.id || !tab.url) continue;
    let target;
    try {
      target = new URL(tab.url);
    } catch (_) {
      continue;
    }
    const match = eligible.find(({ site, adapter }) =>
      target.origin === site.origin
        && !adapter.exclude_paths.some(path => target.pathname.startsWith(path)));
    if (!match) continue;
    try {
      await browser.scripting.executeScript({
        target: { tabId: tab.id },
        files: ["content-explicit-open.js"],
      });
      injected += 1;
    } catch (_) {
      // A tab can close or become restricted between query and injection.
    }
  }
  await traceEvent(
    "observer_registration",
    "registered",
    `${eligible.length} exact-origin rule(s); injected ${injected} open tab(s)`,
  );
  return { registered: eligible.length, injected };
}

async function submitCapture(instanceUrl, pairId, origin, pageUrl, adapter, captureKind, media) {
  return fetch(`${instanceUrl}/products/pinakotheke/api/extension/v1/capture-plans`, {
    method: "POST",
    credentials: "include",
    headers: { "content-type": "application/json", "x-pinakotheke-pairing": pairId },
    body: JSON.stringify({
      schema_version: "x-img.capture-request.v1",
      pairing_id: pairId,
      origin,
      page_url: pageUrl,
      adapter_kind: adapter.kind,
      adapter_version: adapter.version,
      capture_kind: captureKind,
      media_url: media.url,
      presentation_url: media.presentationUrl || media.url,
      width: media.width,
      height: media.height,
    }),
  });
}

async function captureAndFrame(tabId, instanceUrl, pairId, origin, pageUrl, adapter, captureKind, media) {
  const key = `${origin}|${captureKind}|${canonicalAlias(media.url)}`;
  if (captureInFlight.has(key)) {
    await traceEvent("capture_plan", "coalesced", captureKind, origin);
    return { outcome: "in_flight" };
  }
  captureInFlight.add(key);
  try {
    const response = await submitCapture(instanceUrl, pairId, origin, pageUrl, adapter, captureKind, media);
    if (!response.ok) {
      await traceEvent("capture_plan", "rejected", `${captureKind}; HTTP ${response.status}`, origin);
      return { outcome: "rejected" };
    }
    const plan = await response.json();
    if (!plan.plan_id) {
      await traceEvent("capture_plan", "invalid", `${captureKind}; missing plan id`, origin);
      return { outcome: "invalid" };
    }
    await traceEvent("capture_plan", "admitted", `${captureKind}; ${plan.plan_id}`, origin);
    await recordMediaCapture(origin, plan.plan_id, captureKind, "selected", "Selected by your video play action");
    await notifyCaptureState(tabId, media, "selected", "Selected for download");
    for (let attempt = 0; attempt < 180; attempt += 1) {
      const status = await fetch(`${instanceUrl}/products/pinakotheke/api/extension/v1/capture-plans/${encodeURIComponent(plan.plan_id)}`, { cache: "no-store", headers: { "x-pinakotheke-pairing": pairId } });
      if (status.ok) {
        const state = await status.json();
        if (state.schema_version === "pinakotheke.capture-plan-status.v1" && state.state === "stored") {
          await browser.tabs.sendMessage(tabId, { command: "frame-stored", mediaUrl: media.url, mediaToken: media.mediaToken });
          await recordMediaCapture(origin, plan.plan_id, captureKind, "stored", "Available in DASObjectStore");
          await traceEvent("capture_status", "stored", `${captureKind}; ${plan.plan_id}`, origin);
          return { outcome: "stored" };
        }
      }
      if (attempt === 1) {
        await recordMediaCapture(origin, plan.plan_id, captureKind, "downloading", "Acquisition and ObjectStore settlement are in progress");
        await notifyCaptureState(tabId, media, "downloading", "Download in progress");
      }
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
    await traceEvent("capture_status", "pending", `${captureKind}; ${plan.plan_id}`, origin);
    await recordMediaCapture(origin, plan.plan_id, captureKind, "pending", "Server is still processing; status will reconcile on the next check");
    return { outcome: "pending" };
  } catch (error) {
    await traceEvent("capture_plan", "error", `${captureKind}; ${error?.message || "request failed"}`, origin);
    await recordMediaCapture(origin, `failed-${Date.now()}`, captureKind, "failed", "Capture request failed; origin playback was not interrupted");
    return { outcome: "error" };
  } finally {
    captureInFlight.delete(key);
  }
}

function canonicalAlias(rawUrl) {
  const url = new URL(rawUrl);
  if (url.protocol !== "https:") throw new Error("only HTTPS aliases are eligible");
  if (url.hostname === "pbs.twimg.com") {
    const safe = new URLSearchParams();
    for (const key of ["format", "name"]) {
      const value = url.searchParams.get(key);
      if (value && /^[A-Za-z0-9_-]{1,32}$/.test(value)) safe.set(key, value);
    }
    url.search = safe.toString();
  } else {
    url.search = "";
  }
  url.hash = "";
  return url.href;
}

function exactHttpsRetrievalUrl(rawUrl) {
  const parsed = new URL(rawUrl);
  if (parsed.protocol !== "https:" || parsed.username || parsed.password || rawUrl.length > 4096) {
    throw new Error("media retrieval URL is not eligible");
  }
  parsed.hash = "";
  return parsed.href;
}

async function substituteDisplayedImage(candidate) {
  const canonical = rawUrl => {
    const url = new URL(rawUrl);
    url.search = "";
    url.hash = "";
    return url.href;
  };
  const image = [...document.images].find(item => {
    const rect = item.getBoundingClientRect();
    return item.currentSrc
      && canonical(item.currentSrc) === candidate.canonicalAlias
      && rect.width > 0 && rect.height > 0
      && rect.bottom > 0 && rect.right > 0
      && rect.top < window.innerHeight && rect.left < window.innerWidth;
  });
  if (!image) return { outcome: "origin", reason: "image_not_visible" };

  const originalSrc = image.getAttribute("src");
  const originalSrcset = image.getAttribute("srcset");
  let blobUrl;
  const restore = () => {
    if (originalSrc === null) image.removeAttribute("src"); else image.setAttribute("src", originalSrc);
    if (originalSrcset === null) image.removeAttribute("srcset"); else image.setAttribute("srcset", originalSrcset);
    if (blobUrl) URL.revokeObjectURL(blobUrl);
  };
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 5000);
    let response;
    try {
      response = await fetch(candidate.deliveryUrl, {
        credentials: "include",
        cache: "no-store",
        redirect: "error",
        signal: controller.signal,
      });
    } finally {
      clearTimeout(timer);
    }
    const contentType = response.headers.get("content-type");
    const contentLength = Number(response.headers.get("content-length"));
    const etag = response.headers.get("etag");
    if (!response.ok
      || contentType !== candidate.contentType
      || contentLength !== candidate.contentLength
      || contentLength < 1 || contentLength > 32 * 1024 * 1024
      || etag !== `\"${candidate.objectChecksum}\"`) {
      throw new Error("delivery metadata mismatch");
    }
    const bytes = await response.arrayBuffer();
    if (bytes.byteLength !== contentLength) throw new Error("delivery length mismatch");
    blobUrl = URL.createObjectURL(new Blob([bytes], { type: contentType }));
    image.removeAttribute("srcset");
    await new Promise((resolve, reject) => {
      const loaded = () => { cleanup(); resolve(); };
      const failed = () => { cleanup(); reject(new Error("replacement rejected")); };
      const cleanup = () => {
        image.removeEventListener("load", loaded);
        image.removeEventListener("error", failed);
      };
      image.addEventListener("load", loaded, { once: true });
      image.addEventListener("error", failed, { once: true });
      image.setAttribute("src", blobUrl);
    });
    URL.revokeObjectURL(blobUrl);
    blobUrl = undefined;
    return { outcome: "objectstore" };
  } catch (_) {
    restore();
    return { outcome: "origin", reason: "delivery_failed" };
  }
}

function displayedImages() {
  return [...document.images]
    .filter(image => {
      const style = window.getComputedStyle(image);
      const rect = image.getBoundingClientRect();
      return image.complete
        && image.currentSrc
        && image.naturalWidth >= 64
        && image.naturalHeight >= 64
        && style.display !== "none"
        && style.visibility !== "hidden"
        && Number(style.opacity) > 0
        && rect.width > 0
        && rect.height > 0
        && rect.bottom > 0
        && rect.right > 0
        && rect.top < window.innerHeight
        && rect.left < window.innerWidth;
    })
    .map(image => {
      const linked = image.closest("a[href]")?.href;
      let presentationUrl = image.currentSrc;
      try {
        if (linked && new URL(linked).protocol === "https:") presentationUrl = linked;
      } catch (_) {
        // A malformed link cannot prevent other visible images from being observed.
      }
      return { url: image.currentSrc, presentationUrl, width: image.naturalWidth, height: image.naturalHeight };
    })
    .slice(0, 16);
}

function eligibleObservedImages(origin, rule, images) {
  if (origin !== "https://x.com" || !rule.xIngress) return images;
  return images.filter(observed => {
    try {
      return new URL(observed.url).hostname === "pbs.twimg.com";
    } catch (_) {
      return false;
    }
  });
}

function displayedVideos() {
  return [...document.querySelectorAll("video")]
    .filter(video => {
      const style = window.getComputedStyle(video);
      const rect = video.getBoundingClientRect();
      return video.currentSrc
        && video.readyState >= HTMLMediaElement.HAVE_METADATA
        && style.display !== "none"
        && style.visibility !== "hidden"
        && Number(style.opacity) > 0
        && rect.width > 0 && rect.height > 0
        && rect.bottom > 0 && rect.right > 0
        && rect.top < window.innerHeight && rect.left < window.innerWidth;
    })
    .map(video => ({ url: video.currentSrc }))
    .slice(0, 8);
}

async function substituteDisplayedVideo(candidate) {
  const canonical = rawUrl => {
    const url = new URL(rawUrl);
    url.search = "";
    url.hash = "";
    return url.href;
  };
  const video = [...document.querySelectorAll("video")].find(item => {
    const rect = item.getBoundingClientRect();
    return item.currentSrc
      && canonical(item.currentSrc) === candidate.canonicalAlias
      && rect.width > 0 && rect.height > 0
      && rect.bottom > 0 && rect.right > 0
      && rect.top < window.innerHeight && rect.left < window.innerWidth;
  });
  if (!video) return { outcome: "origin", reason: "video_not_visible" };

  const original = {
    src: video.getAttribute("src"),
    crossOrigin: video.getAttribute("crossorigin"),
    sources: [...video.querySelectorAll("source")].map(source => source.getAttribute("src")),
    time: video.currentTime,
    paused: video.paused,
  };
  let settled = false;
  const restore = () => {
    if (settled) return;
    settled = true;
    if (original.src === null) video.removeAttribute("src"); else video.setAttribute("src", original.src);
    if (original.crossOrigin === null) video.removeAttribute("crossorigin"); else video.setAttribute("crossorigin", original.crossOrigin);
    [...video.querySelectorAll("source")].forEach((source, index) => {
      const sourceUrl = original.sources[index];
      if (sourceUrl === null || sourceUrl === undefined) source.removeAttribute("src"); else source.setAttribute("src", sourceUrl);
    });
    video.load();
  };
  try {
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("replacement timeout")), 8000);
      const loaded = () => { cleanup(); resolve(); };
      const failed = () => { cleanup(); reject(new Error("replacement rejected")); };
      const cleanup = () => {
        clearTimeout(timer);
        video.removeEventListener("loadedmetadata", loaded);
        video.removeEventListener("error", failed);
      };
      video.addEventListener("loadedmetadata", loaded, { once: true });
      video.addEventListener("error", failed, { once: true });
      video.setAttribute("crossorigin", "use-credentials");
      video.querySelectorAll("source").forEach(source => source.removeAttribute("src"));
      video.setAttribute("src", candidate.deliveryUrl);
      video.load();
    });
    if (video.currentSrc !== candidate.deliveryUrl || video.duration <= 0) {
      throw new Error("replacement metadata mismatch");
    }
    if (Number.isFinite(original.time) && original.time > 0) {
      video.currentTime = Math.min(original.time, Math.max(0, video.duration - 0.01));
    }
    if (!original.paused) await video.play();
    settled = true;
    return { outcome: "objectstore" };
  } catch (_) {
    restore();
    return { outcome: "origin", reason: "delivery_failed" };
  }
}

async function lookupAlias(instanceUrl, instanceId, pairId, origin, adapter, alias) {
  const response = await fetch(`${instanceUrl}/products/pinakotheke/api/extension/v1/cache-aliases/lookup`, {
    method: "POST",
    credentials: "include",
    cache: "no-store",
    headers: { "content-type": "application/json", "x-pinakotheke-pairing": pairId },
    body: JSON.stringify({
      schema_version: "x-img.cache-alias-lookup.v1",
      pairing_id: pairId,
      instance_id: instanceId,
      origin,
      canonical_alias: alias,
      adapter_id: adapter.id,
      adapter_version: adapter.version,
    }),
  });
  if (!response.ok) throw new Error("cache lookup unavailable");
  const result = await response.json();
  return result.schema_version === "x-img.cache-alias-result.v1" ? result : null;
}

async function recordSiteDiagnostic(origin, update) {
  const stored = await browser.storage.local.get(["sites", "siteDiagnostics"]);
  if (!(stored.sites || []).some(site => site.origin === origin)) return;
  const diagnostics = stored.siteDiagnostics || {};
  const existing = diagnostics[origin];
  diagnostics[origin] = {
    state: update.state,
    reason: update.reason,
    captureState: update.channel === "capture" ? update.state : existing?.captureState,
    captureReason: update.channel === "capture" ? update.reason : existing?.captureReason,
    substitutionState: update.channel === "substitution" ? update.state : existing?.substitutionState,
    substitutionReason: update.channel === "substitution" ? update.reason : existing?.substitutionReason,
    previouslyObserved: Boolean(existing?.previouslyObserved || update.previouslyObserved),
    storedInObjectStore: Boolean(existing?.storedInObjectStore || update.storedInObjectStore),
  };
  for (const key of Object.keys(diagnostics)) {
    if (!(stored.sites || []).some(site => site.origin === key)) delete diagnostics[key];
  }
  await browser.storage.local.set({ siteDiagnostics: diagnostics });
}

function segmentedMediaKind(rawUrl) {
  const url = new URL(rawUrl);
  const path = url.pathname.toLowerCase();
  if (path.endsWith(".m3u8")) return "HLS";
  if (path.endsWith(".mpd")) return "DASH";
  if (url.protocol === "blob:") return "MSE or segmented";
  return null;
}

async function recordSegmentedOriginFallback(origin, kind) {
  await recordSiteDiagnostic(origin, {
    channel: "substitution",
    state: "Origin served",
    reason: `${kind} substitution requires a proven site adapter`,
    previouslyObserved: true,
    storedInObjectStore: false,
  });
}

function validatedObservedImages(images) {
  if (!Array.isArray(images)) return null;
  return images.slice(0, 16).flatMap(image => {
    try {
      const url = new URL(image.url);
      const presentation = new URL(image.presentationUrl || image.url);
      if (url.protocol !== "https:" || presentation.protocol !== "https:"
        || !Number.isInteger(image.width) || !Number.isInteger(image.height)
        || image.width < 1 || image.height < 1 || image.width > 32768 || image.height > 32768) return [];
      return [{ url: url.href, presentationUrl: presentation.href, width: image.width, height: image.height, mediaToken: String(image.mediaToken || "").slice(0, 80) }];
    } catch (_) {
      return [];
    }
  });
}

async function runCacheForTab(tab, contentImages = null) {
  try {
    if (!tab.id || !tab.url) {
      await traceEvent("viewport_scan", "skipped", "tab identity unavailable");
      return;
    }
    const origin = new URL(tab.url).origin;
    const { instanceUrl, instanceId, pairId, sites = [] } = await browser.storage.local.get(["instanceUrl", "instanceId", "pairId", "sites"]);
    const rule = sites.find(site => site.origin === origin);
    if (!rule) {
      await traceEvent("viewport_scan", "skipped", "origin is not enabled", origin);
      return;
    }
    if (!rule.capture && !rule.substitution) {
      await traceEvent("viewport_scan", "skipped", "capture and substitution are paused", origin);
      return;
    }
    if (!instanceUrl || !pairId) {
      await traceEvent("viewport_scan", "skipped", "Pinakotheke pairing is incomplete", origin);
      return;
    }
    const adapter = await matchingAdapter(tab.url);
    if (!adapter) {
      await traceEvent("viewport_scan", "skipped", "no eligible adapter", origin);
      return;
    }
    const reported = validatedObservedImages(contentImages);
    const displayed = rule.media.includes("images")
      ? reported || (await browser.scripting.executeScript({ target: { tabId: tab.id }, func: displayedImages }))[0].result || []
      : [];
    const images = eligibleObservedImages(origin, rule, displayed);
    await traceEvent("viewport_scan", "complete", `${displayed.length} visible image(s); ${images.length} eligible`, origin);
    for (const observed of images) {
      if (instanceId) {
        try {
          const alias = canonicalAlias(observed.url);
          const evidence = await lookupAlias(instanceUrl, instanceId, pairId, origin, adapter, alias);
          if (evidence?.outcome === "hit") {
            await browser.tabs.sendMessage(tab.id, { command: "frame-stored", mediaUrl: observed.url, mediaToken: observed.mediaToken });
          }
        } catch (_) { /* authoritative evidence lookup fails open */ }
      }
      if (rule.capture && adapter.capabilities.observed_thumbnail) {
        void captureAndFrame(
          tab.id, instanceUrl, pairId, origin, tab.url, adapter, "observed_thumbnail", observed,
        ).then(async capture => {
          if (capture.outcome === "stored") await recordSiteDiagnostic(origin, { channel: "capture", state: "Stored in ObjectStore", reason: "Visible thumbnail committed and admitted to the gallery", previouslyObserved: true, storedInObjectStore: true });
          else if (capture.outcome === "pending") await recordSiteDiagnostic(origin, { channel: "capture", state: "Capture pending", reason: "Visible thumbnail is awaiting ObjectStore completion", previouslyObserved: true, storedInObjectStore: false });
        });
      }
      if (rule.substitution && instanceId && adapter.capabilities.image_substitution) {
        const alias = canonicalAlias(observed.url);
        const hit = await lookupAlias(instanceUrl, instanceId, pairId, origin, adapter, alias);
        if (!hit || hit.outcome !== "hit" || !hit.media_class?.endsWith("_image") || !hit.delivery_path) {
          await recordSiteDiagnostic(origin, {
            channel: "substitution",
            state: "Origin served",
            reason: hit?.reason || hit?.outcome || "Cache lookup unavailable",
            previouslyObserved: true,
            storedInObjectStore: false,
          });
          continue;
        }
        const deliveryUrl = new URL(hit.delivery_path, instanceUrl);
        if (deliveryUrl.origin !== instanceUrl) continue;
        const substitution = await browser.scripting.executeScript({
          target: { tabId: tab.id },
          func: substituteDisplayedImage,
          args: [{
            canonicalAlias: alias,
            deliveryUrl: deliveryUrl.href,
            contentType: hit.content_type,
            contentLength: hit.content_length,
            objectChecksum: hit.object_checksum,
          }],
        });
        const outcome = substitution[0]?.result;
        await recordSiteDiagnostic(origin, {
          channel: "substitution",
          state: outcome?.outcome === "objectstore" ? "Cache hit" : "Origin served",
          reason: outcome?.outcome === "objectstore" ? "Image delivered from the reviewed ObjectStore" : "Image substitution failed open",
          previouslyObserved: hit.media_class === "thumbnail_image",
          storedInObjectStore: outcome?.outcome === "objectstore",
        });
      }
    }
    if (rule.substitution && instanceId && rule.media.includes("videos") && adapter.capabilities.mp4_substitution) {
      const videos = (await browser.scripting.executeScript({
        target: { tabId: tab.id }, func: displayedVideos,
      }))[0].result || [];
      for (const observed of videos) {
        const segmentedKind = segmentedMediaKind(observed.url);
        if (segmentedKind) {
          await recordSegmentedOriginFallback(origin, segmentedKind);
          continue;
        }
        const alias = canonicalAlias(observed.url);
        const hit = await lookupAlias(instanceUrl, instanceId, pairId, origin, adapter, alias);
        if (!hit || hit.outcome !== "hit" || hit.media_class !== "normalized_mp4"
          || hit.content_type !== "video/mp4" || !hit.delivery_path) {
          await recordSiteDiagnostic(origin, {
            channel: "substitution",
            state: "Origin served",
            reason: hit?.reason || hit?.outcome || "Normalized video cache miss",
            previouslyObserved: true,
            storedInObjectStore: false,
          });
          continue;
        }
        const deliveryUrl = new URL(hit.delivery_path, instanceUrl);
        if (deliveryUrl.origin !== instanceUrl) continue;
        const substitution = await browser.scripting.executeScript({
          target: { tabId: tab.id },
          func: substituteDisplayedVideo,
          args: [{ canonicalAlias: alias, deliveryUrl: deliveryUrl.href }],
        });
        const outcome = substitution[0]?.result;
        await recordSiteDiagnostic(origin, {
          channel: "substitution",
          state: outcome?.outcome === "objectstore" ? "Cache hit" : "Origin served",
          reason: outcome?.outcome === "objectstore" ? "Normalized video delivered from the reviewed ObjectStore" : "Video substitution failed open",
          previouslyObserved: true,
          storedInObjectStore: outcome?.outcome === "objectstore",
        });
      }
    }
  } catch (_) {
    if (tab?.url) {
      try {
        const origin = new URL(tab.url).origin;
        await recordSiteDiagnostic(origin, {
          state: "Origin served",
          reason: "x-img cache operation unavailable",
          previouslyObserved: false,
          storedInObjectStore: false,
        });
        await traceEvent("viewport_scan", "error", "cache operation unavailable", origin);
      } catch (_) {
        // Non-Web tabs have no site policy or diagnostic record.
      }
    }
  }
}

browser.runtime.onMessage.addListener(async (message, sender) => {
  if (message?.command === "refresh-media-captures") {
    await refreshMediaCaptureStates();
    return { completed: true };
  }
  if (message?.command === "sync-site-corpus") return syncSiteCorpusFromServer();
  if (message?.command === "sync-capture-observers") {
    return syncExplicitOpenObservers();
  }
  if (message?.command === "run-cache") {
    const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
    await runCacheForTab(tab);
    return { completed: true };
  }
  if (message?.command === "resolve-segmented-video" && sender?.tab?.id && sender.tab.url) {
    const origin = new URL(sender.tab.url).origin;
    const { sites = [] } = await browser.storage.local.get(["sites"]);
    const rule = sites.find(site => site.origin === origin);
    if (origin !== "https://x.com" || !rule?.capture || !rule.media.includes("videos")) {
      return { mediaUrl: null };
    }
    const mediaUrl = recentSegmentedManifest(sender.tab.id, message.mediaFamilies);
    await traceEvent(
      "segmented_manifest",
      mediaUrl ? "resolved" : "missing",
      mediaUrl ? "matching manifest supplied after trusted play" : "no matching recent manifest",
      origin,
    );
    return { mediaUrl };
  }
  if (message?.command === "visible-media-changed" && sender?.tab) {
    await traceEvent("content_observer", "signal", "visible media changed", new URL(sender.tab.url).origin);
    await runCacheForTab(sender.tab, message.images);
    return { completed: true };
  }
  if (message?.command === "explicit-video-unresolved" && sender?.tab?.url) {
    const origin = new URL(sender.tab.url).origin;
    await recordSiteDiagnostic(origin, { channel: "capture", state: "Video unavailable", reason: "No progressive HTTPS video resource was exposed; segmented playback remains origin-served", previouslyObserved: true, storedInObjectStore: false });
    await traceEvent("video_candidate", "segmented_or_unresolved", "no progressive HTTPS MP4 resource", origin);
    return { completed: false };
  }
  if (message?.command === "explicit-video-observer" && sender?.tab?.url
    && ["missing_current_src", "missing_trusted_activation"].includes(message.outcome)) {
    const origin = new URL(sender.tab.url).origin;
    await traceEvent("video_observer", message.outcome, "trusted play was not capture-eligible", origin);
    return { completed: false };
  }
  if (!["explicit-original-opened", "explicit-video-opened"].includes(message?.command) || !sender?.tab?.id || !sender.tab.url) {
    return undefined;
  }
  try {
    const origin = new URL(sender.tab.url).origin;
    const { instanceUrl, pairId, sites = [] } = await browser.storage.local.get([
      "instanceUrl", "pairId", "sites",
    ]);
    const rule = sites.find(site => site.origin === origin);
    const video = message.command === "explicit-video-opened";
    if (!instanceUrl || !pairId || !rule?.capture || !rule.media.includes(video ? "videos" : "images")) return undefined;
    const adapter = await matchingAdapter(sender.tab.url);
    if (!adapter?.capabilities.explicit_original) {
      return { completed: false };
    }
    const width = Number(message.width);
    const height = Number(message.height);
    if (!Number.isInteger(width) || !Number.isInteger(height)
      || width < 1 || height < 1 || width > 32768 || height > 32768) return undefined;
    const mediaUrl = video
      ? exactHttpsRetrievalUrl(String(message.mediaUrl))
      : canonicalAlias(String(message.mediaUrl));
    const presentationUrl = canonicalAlias(String(message.presentationUrl || message.mediaUrl));
    if (video && origin === "https://x.com" && new URL(mediaUrl).hostname !== "video.twimg.com") {
      await recordSiteDiagnostic(origin, { channel: "capture", state: "Video unavailable", reason: "The opened X video did not expose an eligible X media resource", previouslyObserved: true, storedInObjectStore: false });
      return { completed: false };
    }
    const capture = await captureAndFrame(
      sender.tab.id, instanceUrl, pairId, origin, sender.tab.url, adapter, video ? "explicit_video" : "explicit_original",
      { url: mediaUrl, presentationUrl, width, height, mediaToken: String(message.mediaToken || "").slice(0, 80) },
    );
    if (capture.outcome === "stored") {
      await recordSiteDiagnostic(origin, {
        channel: "capture",
        state: "Stored in ObjectStore",
        reason: video ? "User-played video committed and admitted to the gallery" : "User-opened original committed and admitted to the gallery",
        previouslyObserved: true,
        storedInObjectStore: true,
      });
    } else if (capture.outcome === "pending") {
      await recordSiteDiagnostic(origin, {
        channel: "capture",
        state: "Capture pending",
        reason: "User-opened original is awaiting ObjectStore completion",
        previouslyObserved: true,
        storedInObjectStore: false,
      });
    }
    return { completed: capture.outcome === "stored" };
  } catch (_) {
    return { completed: false };
  }
});
