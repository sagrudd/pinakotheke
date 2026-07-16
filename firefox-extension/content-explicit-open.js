// SPDX-License-Identifier: MPL-2.0
// Runs only on exact origins dynamically registered from explicit site policy.
if (!globalThis.__pinakothekeExplicitOpenObserver) {
  globalThis.__pinakothekeExplicitOpenObserver = true;
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
      width: image.naturalWidth,
      height: image.naturalHeight,
    });
  }, true);
}
