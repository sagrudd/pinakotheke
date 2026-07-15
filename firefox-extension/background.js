// SPDX-License-Identifier: MPL-2.0
// No page cookies, webRequest interception, credentials, or automatic navigation.
browser.runtime.onInstalled.addListener(() =>
  browser.storage.local.set({ instanceUrl: "", instanceId: "", pairId: "", sites: [] }),
);

async function matchingAdapter(url) {
  const registry = await fetch(browser.runtime.getURL("adapters.json")).then(response => response.json());
  const target = new URL(url);
  return registry.adapters.find(adapter =>
    adapter.origins.includes(target.origin)
      && !adapter.exclude_paths.some(path => target.pathname.startsWith(path))
      && (adapter.capabilities.observed_thumbnail
        || adapter.capabilities.image_substitution
        || adapter.capabilities.mp4_substitution),
  ) || null;
}

function canonicalAlias(rawUrl) {
  const url = new URL(rawUrl);
  if (url.protocol !== "https:") throw new Error("only HTTPS aliases are eligible");
  url.search = "";
  url.hash = "";
  return url.href;
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
        && image.naturalWidth > 0
        && image.naturalHeight > 0
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
    .map(image => ({ url: image.currentSrc, width: image.naturalWidth, height: image.naturalHeight }))
    .slice(0, 32);
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
  const response = await fetch(`${instanceUrl}/api/extension/v1/cache-aliases/lookup`, {
    method: "POST",
    credentials: "include",
    cache: "no-store",
    headers: { "content-type": "application/json" },
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
    state: "Origin served",
    reason: `${kind} substitution requires a proven site adapter`,
    previouslyObserved: true,
    storedInObjectStore: false,
  });
}

async function runCacheForTab(tab) {
  try {
    if (!tab.id || !tab.url) return;
    const origin = new URL(tab.url).origin;
    const { instanceUrl, instanceId, pairId, sites = [] } = await browser.storage.local.get(["instanceUrl", "instanceId", "pairId", "sites"]);
    const rule = sites.find(site => site.origin === origin);
    if (!rule || (!rule.capture && !rule.substitution)
      || !instanceUrl || !instanceId || !pairId) return;
    const adapter = await matchingAdapter(tab.url);
    if (!adapter) return;
    const images = rule.media.includes("images")
      ? (await browser.scripting.executeScript({ target: { tabId: tab.id }, func: displayedImages }))[0].result || []
      : [];
    for (const observed of images) {
      if (rule.capture && adapter.capabilities.observed_thumbnail) {
        const capture = await fetch(`${instanceUrl}/api/extension/v1/capture-plans`, {
          method: "POST",
          credentials: "include",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({
            schema_version: "x-img.capture-request.v1",
            pairing_id: pairId,
            origin,
            page_url: tab.url,
            adapter_kind: adapter.kind,
            adapter_version: adapter.version,
            capture_kind: "observed_thumbnail",
            media_url: observed.url,
            width: observed.width,
            height: observed.height,
          }),
        });
        if (capture.ok) {
          await recordSiteDiagnostic(origin, {
            state: "Previously observed",
            reason: "Visible thumbnail accepted for review",
            previouslyObserved: true,
            storedInObjectStore: false,
          });
        }
      }
      if (rule.substitution && adapter.capabilities.image_substitution) {
        const alias = canonicalAlias(observed.url);
        const hit = await lookupAlias(instanceUrl, instanceId, pairId, origin, adapter, alias);
        if (!hit || hit.outcome !== "hit" || !hit.media_class?.endsWith("_image") || !hit.delivery_path) {
          await recordSiteDiagnostic(origin, {
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
          state: outcome?.outcome === "objectstore" ? "Cache hit" : "Origin served",
          reason: outcome?.outcome === "objectstore" ? "Image delivered from the reviewed ObjectStore" : "Image substitution failed open",
          previouslyObserved: hit.media_class === "thumbnail_image",
          storedInObjectStore: outcome?.outcome === "objectstore",
        });
      }
    }
    if (rule.substitution && rule.media.includes("videos") && adapter.capabilities.mp4_substitution) {
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
      } catch (_) {
        // Non-Web tabs have no site policy or diagnostic record.
      }
    }
  }
}

browser.runtime.onMessage.addListener(async message => {
  if (message?.command !== "run-cache") return undefined;
  const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
  await runCacheForTab(tab);
  return { completed: true };
});
