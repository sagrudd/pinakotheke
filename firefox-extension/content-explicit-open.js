// SPDX-License-Identifier: MPL-2.0
// Runs only on exact origins dynamically registered from explicit site policy.
if (!globalThis.__pinakothekeExplicitOpenObserver) {
  globalThis.__pinakothekeExplicitOpenObserver = true;
  const style = document.createElement("style");
  style.textContent = ".pinakotheke-stored-object { box-sizing: border-box !important; border: 2px solid #238636 !important; }";
  (document.head || document.documentElement).append(style);
  const canonical = raw => { const url = new URL(raw); url.search = ""; url.hash = ""; return url.href; };
  const visibleImages = () => [...document.images]
    .filter(image => {
      const style = getComputedStyle(image);
      const rect = image.getBoundingClientRect();
      return image.complete && image.currentSrc
        && image.naturalWidth >= 64 && image.naturalHeight >= 64
        && style.display !== "none" && style.visibility !== "hidden"
        && Number(style.opacity) > 0
        && rect.width > 0 && rect.height > 0
        && rect.bottom > 0 && rect.right > 0
        && rect.top < innerHeight && rect.left < innerWidth;
    })
    .map(image => ({
      url: image.currentSrc,
      presentationUrl: image.closest("a[href]")?.href || image.currentSrc,
      width: image.naturalWidth,
      height: image.naturalHeight,
    }))
    .slice(0, 16);
  let observationTimer;
  const observed = () => {
    clearTimeout(observationTimer);
    observationTimer = setTimeout(() => void browser.runtime.sendMessage({
      command: "visible-media-changed",
      images: visibleImages(),
    }), 250);
  };
  new MutationObserver(observed).observe(document.documentElement, { childList: true, subtree: true, attributes: true, attributeFilter: ["src", "srcset"] });
  document.addEventListener("scroll", observed, { passive: true, capture: true });
  document.addEventListener("load", observed, true);
  observed();

  browser.runtime.onMessage.addListener(message => {
    if (message?.command !== "frame-stored" || !message.mediaUrl) return;
    const wanted = canonical(message.mediaUrl);
    for (const media of document.querySelectorAll("img,video")) {
      try { if (media.currentSrc && canonical(media.currentSrc) === wanted) media.classList.add("pinakotheke-stored-object"); } catch (_) { /* ignore malformed page media */ }
    }
  });

  const videoActivations = new WeakMap();
  const rememberVideoActivation = event => {
    if (!event.isTrusted) return;
    let element = event.target instanceof Element ? event.target : null;
    let video = element?.closest("video") || null;
    for (let depth = 0; !video && element && depth < 5; depth += 1, element = element.parentElement) {
      video = element.querySelector?.("video") || null;
    }
    if (video) videoActivations.set(video, {
      epochMilliseconds: Date.now(),
      performanceMilliseconds: performance.now(),
    });
  };
  document.addEventListener("pointerdown", rememberVideoActivation, true);
  document.addEventListener("keydown", event => {
    if (event.key === "Enter" || event.key === " ") rememberVideoActivation(event);
  }, true);
  document.addEventListener("play", event => {
    if (!event.isTrusted || !(event.target instanceof HTMLVideoElement) || !event.target.currentSrc) return;
    const video = event.target;
    const activation = videoActivations.get(video);
    if (!activation || Date.now() - activation.epochMilliseconds > 2000) return;
    // X frequently presents a blob: URL to the element while Firefox has
    // already fetched a concrete, independently retrievable MP4 resource.
    // Resource timing exposes URLs, not request headers or cookies. Prefer the
    // newest HTTPS MP4 associated with the played element and retain the page
    // URL only as provenance. Segmented/MSE playback remains origin-served.
    const candidates = [video.currentSrc, ...performance.getEntriesByType("resource")
      .filter(entry => entry.initiatorType === "video"
        && entry.startTime >= activation.performanceMilliseconds - 1000)
      .map(entry => entry.name)
      .filter(url => /^https:\/\//.test(url))
      .slice(-12)
      .reverse()];
    const mediaUrl = candidates.find(url => {
      try { return new URL(url).protocol === "https:"; } catch (_) { return false; }
    });
    if (!mediaUrl) {
      void browser.runtime.sendMessage({ command: "explicit-video-unresolved" });
      return;
    }
    void browser.runtime.sendMessage({
      command: "explicit-video-opened",
      mediaUrl,
      presentationUrl: location.href,
      width: video.videoWidth || video.clientWidth,
      height: video.videoHeight || video.clientHeight,
    });
  }, true);
  document.addEventListener("click", event => {
    if (!event.isTrusted || event.button !== 0) return;
    const image = event.target instanceof Element ? event.target.closest("img") : null;
    if (!image || !image.currentSrc || image.naturalWidth < 1 || image.naturalHeight < 1) return;
    const link = image.closest("a[href]");
    if (!link && !document.contentType?.startsWith("image/")) return;
    // The enclosing link is presentation provenance (for example an X status
    // page), not necessarily the image payload. Always submit the bytes that
    // Firefox actually rendered as the media candidate.
    const mediaUrl = image.currentSrc;
    const presentationUrl = link?.href || mediaUrl;
    try {
      if (new URL(mediaUrl).protocol !== "https:") return;
    } catch (_) {
      return;
    }
    void browser.runtime.sendMessage({
      command: "explicit-original-opened",
      mediaUrl,
      presentationUrl,
      width: image.naturalWidth,
      height: image.naturalHeight,
    });
  }, true);
}
