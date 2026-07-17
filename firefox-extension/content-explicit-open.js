// SPDX-License-Identifier: MPL-2.0
// Runs only on exact origins dynamically registered from explicit site policy.
if (!globalThis.__pinakothekeExplicitOpenObserver) {
  globalThis.__pinakothekeExplicitOpenObserver = true;
  const style = document.createElement("style");
  style.textContent = ".pinakotheke-stored-object { box-sizing: border-box !important; border: 2px solid #238636 !important; }";
  (document.head || document.documentElement).append(style);
  const canonical = raw => { const url = new URL(raw); url.search = ""; url.hash = ""; return url.href; };
  let observationTimer;
  const observed = () => {
    clearTimeout(observationTimer);
    observationTimer = setTimeout(() => void browser.runtime.sendMessage({ command: "visible-media-changed" }), 250);
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

  document.addEventListener("play", event => {
    if (!event.isTrusted || !(event.target instanceof HTMLVideoElement) || !event.target.currentSrc) return;
    const video = event.target;
    void browser.runtime.sendMessage({ command: "explicit-video-opened", mediaUrl: video.currentSrc, width: video.videoWidth || video.clientWidth, height: video.videoHeight || video.clientHeight });
  }, true);
  document.addEventListener("click", event => {
    if (!event.isTrusted || event.button !== 0) return;
    const image = event.target instanceof Element ? event.target.closest("img") : null;
    if (!image || !image.currentSrc || image.naturalWidth < 1 || image.naturalHeight < 1) return;
    const link = image.closest("a[href]");
    if (!link && !document.contentType?.startsWith("image/")) return;
    const mediaUrl = link?.href || image.currentSrc;
    try {
      if (new URL(mediaUrl).protocol !== "https:") return;
    } catch (_) {
      return;
    }
    void browser.runtime.sendMessage({
      command: "explicit-original-opened",
      mediaUrl,
      presentationUrl: mediaUrl,
      width: image.naturalWidth,
      height: image.naturalHeight,
    });
  }, true);
}
