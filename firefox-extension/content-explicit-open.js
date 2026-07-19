// SPDX-License-Identifier: MPL-2.0
// Runs only on exact origins dynamically registered from explicit site policy.
(() => {
  const pinakothekeObserverVersion = browser.runtime.getManifest().version;
  if (globalThis.__pinakothekeExplicitOpenObserverVersion !== pinakothekeObserverVersion) {
  globalThis.__pinakothekeExplicitOpenObserverVersion = pinakothekeObserverVersion;
  // Retain the historic marker for compatibility while using the versioned
  // marker as the upgrade/idempotency authority.
  globalThis.__pinakothekeExplicitOpenObserver = true;
  const style = document.createElement("style");
  style.textContent = [
    ".pinakotheke-capture-selected { box-sizing: border-box !important; outline: 2px dashed #1769aa !important; outline-offset: -2px !important; }",
    ".pinakotheke-stored-object { box-sizing: border-box !important; border: 2px solid #238636 !important; outline: 2px solid #238636 !important; outline-offset: -2px !important; box-shadow: inset 0 0 0 2px #238636 !important; }",
  ].join("\n");
  (document.head || document.documentElement).append(style);
  for (const stale of document.querySelectorAll(
    ".pinakotheke-capture-selected, .pinakotheke-stored-object",
  )) {
    stale.classList.remove("pinakotheke-capture-selected", "pinakotheke-stored-object");
    if (stale.dataset) delete stale.dataset.pinakothekeCaptureState;
  }
  for (const staleOverlay of document.querySelectorAll("[data-pinakotheke-stored-overlay]")) {
    staleOverlay.remove();
  }
  const canonical = raw => { const url = new URL(raw); url.search = ""; url.hash = ""; return url.href; };
  const mediaTokens = new WeakMap();
  const storedOverlays = new WeakMap();
  const storedMedia = new Set();
  const storedImageIdentities = new Set();
  const storedImageIdentityOrder = [];
  const framedTargets = new WeakMap();
  const isXMediaUrl = raw => {
    try {
      const url = new URL(raw);
      return url.protocol === "https:" && url.hostname === "pbs.twimg.com"
        && url.pathname.startsWith("/media/");
    } catch (_) {
      return false;
    }
  };
  const presentationUrlFor = image => image.closest("a[href]")?.href
    || image.closest("article")?.querySelector?.('a[href*="/status/"]')?.href
    || location.href;
  const xOriginalUrl = raw => {
    const url = new URL(raw);
    if (!isXMediaUrl(url.href)) return url.href;
    url.searchParams.set("name", "orig");
    return url.href;
  };
  const mediaIdentityFor = media => {
    const kind = media instanceof HTMLVideoElement ? "video" : "image";
    const rendered = media.currentSrc || media.src || "";
    try {
      const source = rendered.startsWith("blob:") ? rendered : canonical(rendered);
      return kind === "video"
        ? `${kind}|${canonical(presentationUrlFor(media))}|${source}`
        : `${kind}|${source}`;
    } catch (_) {
      return `${kind}|unavailable`;
    }
  };
  const newMediaToken = () => globalThis.crypto?.randomUUID?.()
    || `media-${Date.now()}-${Math.random().toString(36).slice(2)}`;
  const mediaTokenFor = media => {
    const identity = mediaIdentityFor(media);
    let record = mediaTokens.get(media);
    const datasetMatches = !media.dataset
      || media.dataset.pinakothekeMediaIdentity === identity;
    if (!record || record.identity !== identity || !datasetMatches) {
      clearMediaFrame(media);
      record = { identity, token: newMediaToken() };
      mediaTokens.set(media, record);
      if (media.dataset) {
        media.dataset.pinakothekeMediaIdentity = identity;
        media.dataset.pinakothekeMediaToken = record.token;
      }
    }
    return record.token;
  };
  const framingTargets = media => {
    const targets = [media];
    const mediaRect = media.getBoundingClientRect();
    let parent = media.parentElement;
    for (let depth = 0; parent && depth < 4; depth += 1, parent = parent.parentElement) {
      const rect = parent.getBoundingClientRect();
      const sameFootprint = Math.abs(rect.left - mediaRect.left) <= 3
        && Math.abs(rect.top - mediaRect.top) <= 3
        && Math.abs(rect.right - mediaRect.right) <= 3
        && Math.abs(rect.bottom - mediaRect.bottom) <= 3;
      if (!sameFootprint) break;
      targets.push(parent);
    }
    return targets;
  };
  const positionStoredOverlay = media => {
    let overlay = storedOverlays.get(media);
    if (!overlay) {
      overlay = document.createElement("span");
      overlay.setAttribute("aria-hidden", "true");
      overlay.dataset.pinakothekeStoredOverlay = "true";
      Object.assign(overlay.style, {
        position: "fixed",
        pointerEvents: "none",
        boxSizing: "border-box",
        border: "2px solid #238636",
        zIndex: "2147483647",
      });
      (document.body || document.documentElement).append(overlay);
      storedOverlays.set(media, overlay);
      storedMedia.add(media);
    }
    const rect = media.getBoundingClientRect();
    if (!media.isConnected || rect.width <= 0 || rect.height <= 0) {
      overlay.remove();
      storedOverlays.delete(media);
      storedMedia.delete(media);
      return;
    }
    overlay.style.left = `${rect.left}px`;
    overlay.style.top = `${rect.top}px`;
    overlay.style.width = `${rect.width}px`;
    overlay.style.height = `${rect.height}px`;
    overlay.style.borderRadius = getComputedStyle(media).borderRadius;
  };
  const clearMediaFrame = media => {
    const targets = new Set([media, ...(framedTargets.get(media) || [])]);
    for (const target of targets) {
      target.classList?.remove("pinakotheke-capture-selected");
      target.classList?.remove("pinakotheke-stored-object");
    }
    framedTargets.delete(media);
    const overlay = storedOverlays.get(media);
    overlay?.remove();
    storedOverlays.delete(media);
    storedMedia.delete(media);
    if (media.dataset) delete media.dataset.pinakothekeCaptureState;
  };
  const refreshStoredOverlays = () => {
    for (const media of [...storedMedia]) positionStoredOverlay(media);
  };
  const applyStoredFrame = media => {
    const targets = framingTargets(media);
    framedTargets.set(media, targets);
    for (const target of targets) {
      target.classList.remove("pinakotheke-capture-selected");
      target.classList.add("pinakotheke-stored-object");
    }
    media.dataset.pinakothekeCaptureState = "Stored in ObjectStore";
    positionStoredOverlay(media);
  };
  const rememberStoredImageIdentity = raw => {
    const identity = canonical(raw);
    if (storedImageIdentities.has(identity)) return;
    storedImageIdentities.add(identity);
    storedImageIdentityOrder.push(identity);
    while (storedImageIdentityOrder.length > 4096) {
      storedImageIdentities.delete(storedImageIdentityOrder.shift());
    }
  };
  const repairKnownStoredFrame = media => {
    if (!media.currentSrc || !storedImageIdentities.has(canonical(media.currentSrc))) return false;
    applyStoredFrame(media);
    return true;
  };
  document.addEventListener("scroll", refreshStoredOverlays, { passive: true, capture: true });
  globalThis.addEventListener?.("resize", refreshStoredOverlays, { passive: true });
  const visibleImages = () => [...document.images]
    .filter(image => {
      const style = getComputedStyle(image);
      const rect = image.getBoundingClientRect();
      const inViewport = rect.width >= 32 && rect.height >= 32
        && rect.bottom > 0 && rect.right > 0
        && rect.top < innerHeight && rect.left < innerWidth;
      if (image.currentSrc && isXMediaUrl(image.currentSrc)) return inViewport;
      return image.complete && image.currentSrc
        && image.naturalWidth >= 64 && image.naturalHeight >= 64
        && style.display !== "none" && style.visibility !== "hidden"
        && Number(style.opacity) > 0 && inViewport;
    })
    .map(image => {
      const mediaToken = mediaTokenFor(image);
      repairKnownStoredFrame(image);
      return {
        url: image.currentSrc,
        presentationUrl: presentationUrlFor(image),
        width: image.naturalWidth || Math.round(image.getBoundingClientRect().width),
        height: image.naturalHeight || Math.round(image.getBoundingClientRect().height),
        mediaToken,
      };
    })
    .sort((left, right) => Number(isXMediaUrl(right.url)) - Number(isXMediaUrl(left.url)))
    .slice(0, 64);
  const visibleVideos = () => [...document.querySelectorAll("video")]
    .filter(video => isVisibleVideo(video))
    .map(video => ({
      presentationUrl: presentationUrlFor(video),
      width: video.videoWidth || Math.round(video.getBoundingClientRect().width),
      height: video.videoHeight || Math.round(video.getBoundingClientRect().height),
      mediaToken: mediaTokenFor(video),
    }))
    .filter(video => video.width > 0 && video.height > 0)
    .slice(0, 8);
  let observationTimer;
  let lastVisibleFingerprint = "";
  const observed = () => {
    clearTimeout(observationTimer);
    observationTimer = setTimeout(() => {
      refreshStoredOverlays();
      const images = visibleImages();
      const videos = visibleVideos();
      const fingerprint = [
        ...images.map(image => `image|${canonical(image.url)}|${image.mediaToken}`),
        ...videos.map(video => `video|${canonical(video.presentationUrl)}|${video.mediaToken}`),
      ].join("\n");
      if (fingerprint === lastVisibleFingerprint) return;
      void browser.runtime.sendMessage({ command: "visible-media-changed", images, videos })
        .then(result => {
          if (result?.completed) lastVisibleFingerprint = fingerprint;
        })
        .catch(() => {
          // Keep the fingerprint retryable after extension reload or a
          // temporarily unavailable background/host connection.
        });
    }, 250);
  };
  new MutationObserver(observed).observe(document.documentElement, { childList: true, subtree: true, attributes: true, attributeFilter: ["src", "srcset"] });
  document.addEventListener("scroll", observed, { passive: true, capture: true });
  document.addEventListener("load", observed, true);
  // X virtualizes and reuses gallery nodes in ways that do not always produce
  // a useful src/srcset mutation. A bounded safety scan repairs missed
  // observer delivery; fingerprinting above prevents repeated server work.
  setInterval(observed, 2000);
  observed();

  browser.runtime.onMessage.addListener(message => {
    if (!["frame-stored", "media-capture-state"].includes(message?.command)) return;
    const wanted = message.mediaUrl ? canonical(message.mediaUrl) : null;
    let matched = 0;
    for (const media of document.querySelectorAll("img,video")) {
      try {
        const tokenMatches = Boolean(message.mediaToken
          && mediaTokenFor(media) === message.mediaToken);
        const urlMatches = Boolean(wanted && media.currentSrc
          && canonical(media.currentSrc) === wanted);
        // X replaces a clicked thumbnail node with a new modal/gallery node.
        // A stable canonical media URL is therefore authoritative when one is
        // supplied; the element token is only a fallback for URL-less state
        // messages. A different rendered media identity still cannot inherit
        // the frame from a recycled node.
        const matches = wanted ? urlMatches : tokenMatches;
        if (!matches) continue;
        matched += 1;
        if (message.command === "frame-stored" || message.state === "stored") {
          if (wanted && !(media instanceof HTMLVideoElement)) rememberStoredImageIdentity(wanted);
          applyStoredFrame(media);
        } else {
          const targets = framingTargets(media);
          framedTargets.set(media, targets);
          for (const target of targets) target.classList.add("pinakotheke-capture-selected");
          media.dataset.pinakothekeCaptureState = message.label || "Selected for download";
        }
      } catch (_) { /* ignore malformed page media */ }
    }
    return Promise.resolve({ matched });
  });

  const videoActivations = new WeakMap();
  let recentVisibleVideoActivation = null;
  const trustedPlayWindowMilliseconds = 8000;
  const isVisibleVideo = video => {
    const rect = video.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0
      && rect.bottom > 0 && rect.right > 0
      && rect.top < innerHeight && rect.left < innerWidth;
  };
  const visibleVideoAtPoint = event => [...document.querySelectorAll("video")].find(video => {
    const rect = video.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0
      && Number.isFinite(event.clientX) && Number.isFinite(event.clientY)
      && event.clientX >= rect.left && event.clientX <= rect.right
      && event.clientY >= rect.top && event.clientY <= rect.bottom;
  }) || null;
  const rememberVideoActivation = event => {
    if (!event.isTrusted) return;
    const activation = {
      epochMilliseconds: Date.now(),
      performanceMilliseconds: performance.now(),
    };
    let element = event.target instanceof Element ? event.target : null;
    let video = element?.closest("video") || null;
    for (let depth = 0; !video && element && depth < 5; depth += 1, element = element.parentElement) {
      video = element.querySelector?.("video") || null;
    }
    if (!video && event.type === "pointerdown") video = visibleVideoAtPoint(event);
    if (video) {
      videoActivations.set(video, activation);
      recentVisibleVideoActivation = { video, activation };
      setTimeout(() => {
        if (video.isConnected && isVisibleVideo(video) && !video.paused && video.currentSrc) {
          void resolvePlayedVideo(video, activation);
        }
      }, 150);
    }
  };
  document.addEventListener("pointerdown", rememberVideoActivation, true);
  document.addEventListener("keydown", event => {
    if (event.key === "Enter" || event.key === " ") rememberVideoActivation(event);
  }, true);
  const videoResolutions = new WeakSet();
  const likelyProgressiveVideo = entry => {
    try {
      const url = new URL(entry.name);
      if (/\.(?:m4s|m3u8|mpd|cmfv|cmfa)$/i.test(url.pathname)) return false;
      return url.protocol === "https:" && (
        entry.initiatorType === "video"
        || url.hostname === "video.twimg.com"
        || /\.(?:mp4|m4v|webm|mov)$/i.test(url.pathname)
      );
    } catch (_) {
      return false;
    }
  };
  const likelySegmentedManifest = entry => {
    try {
      const url = new URL(entry.name);
      return url.protocol === "https:"
        && url.hostname === "video.twimg.com"
        && /\.(?:m3u8|mpd)$/i.test(url.pathname);
    } catch (_) {
      return false;
    }
  };
  const preferMasterManifest = entries => entries
    .filter(likelySegmentedManifest)
    .sort((left, right) => {
      const leftDepth = new URL(left.name).pathname.split("/").length;
      const rightDepth = new URL(right.name).pathname.split("/").length;
      return leftDepth - rightDepth || right.startTime - left.startTime;
    })[0]?.name || null;
  const mediaFamily = raw => {
    try {
      const url = new URL(raw);
      const match = url.pathname.match(/^\/(amplify_video|ext_tw_video)\/([^/]+)\//);
      return match ? `${url.hostname}/${match[1]}/${match[2]}` : null;
    } catch (_) {
      return null;
    }
  };
  const resolvePlayedVideo = async (video, activation) => {
    if (videoResolutions.has(video)) return;
    videoResolutions.add(video);
    if (!activation || Date.now() - activation.epochMilliseconds > trustedPlayWindowMilliseconds) {
      videoResolutions.delete(video);
      return;
    }
    // X frequently presents a blob: URL to the element while Firefox has
    // already fetched a concrete, independently retrievable MP4 resource.
    // Resource timing exposes URLs, not request headers or cookies. X fetches
    // progressive MP4 through script/fetch, so initiatorType cannot be the
    // sole signal. Poll briefly after play because the resource often appears
    // after the play event. Segmented/MSE playback remains origin-served.
    for (let attempt = 0; attempt < 9; attempt += 1) {
      const current = (() => {
        try {
          const url = new URL(video.currentSrc);
          return url.protocol === "https:" ? video.currentSrc : null;
        } catch (_) {
          return null;
        }
      })();
      const allResources = performance.getEntriesByType("resource").slice(-512);
      const recentResources = allResources
        .filter(entry => entry.startTime >= activation.performanceMilliseconds - 1000)
        .slice(-256);
      const timed = recentResources.filter(likelyProgressiveVideo)
        .sort((left, right) => right.startTime - left.startTime)
        .slice(0, 24)
        .map(entry => entry.name);
      const recentFamilies = new Set(recentResources.map(entry => mediaFamily(entry.name)).filter(Boolean));
      const manifest = preferMasterManifest(allResources.filter(entry => {
        const family = mediaFamily(entry.name);
        return family && recentFamilies.has(family);
      }));
      let backgroundManifest = null;
      try {
        const resolved = await browser.runtime.sendMessage({
          command: "resolve-segmented-video",
          mediaFamilies: [...recentFamilies].slice(0, 8),
        });
        backgroundManifest = resolved?.mediaUrl || null;
      } catch (_) {
        // URL-only request observation is optional and capture remains fail-open.
      }
      const mediaUrl = manifest || backgroundManifest || [current, ...timed].find(candidate => {
        if (!candidate) return false;
        try { return !/\.(?:m4s|cmfv|cmfa)$/i.test(new URL(candidate).pathname); } catch (_) { return false; }
      });
      if (mediaUrl) {
        const mediaToken = mediaTokenFor(video);
        void browser.runtime.sendMessage({
          command: "explicit-video-opened",
          mediaUrl,
          presentationUrl: location.href,
          width: video.videoWidth || video.clientWidth,
          height: video.videoHeight || video.clientHeight,
          mediaToken,
        });
        videoResolutions.delete(video);
        return;
      }
      if (attempt < 8) await new Promise(resolve => setTimeout(resolve, 250));
    }
    videoResolutions.delete(video);
    void browser.runtime.sendMessage({ command: "explicit-video-unresolved" });
  };
  document.addEventListener("play", event => {
    if (!(event.target instanceof HTMLVideoElement)) return;
    if (!event.target.currentSrc) {
      void browser.runtime.sendMessage({ command: "explicit-video-observer", outcome: "missing_current_src" });
      return;
    }
    const video = event.target;
    const activation = videoActivations.get(video)
      || (recentVisibleVideoActivation?.video === video
        ? recentVisibleVideoActivation.activation : null);
    if (!activation || Date.now() - activation.epochMilliseconds > trustedPlayWindowMilliseconds) {
      void browser.runtime.sendMessage({ command: "explicit-video-observer", outcome: "missing_trusted_activation" });
      return;
    }
    void resolvePlayedVideo(video, activation);
  }, true);
  document.addEventListener("click", event => {
    if (!event.isTrusted || event.button !== 0) return;
    const directImage = event.target instanceof Element ? event.target.closest("img") : null;
    const image = directImage || [...document.images].reverse().find(candidate => {
      if (location.origin !== "https://x.com" || !isXMediaUrl(candidate.currentSrc)) return false;
      const rect = candidate.getBoundingClientRect();
      return Number.isFinite(event.clientX) && Number.isFinite(event.clientY)
        && event.clientX >= rect.left && event.clientX <= rect.right
        && event.clientY >= rect.top && event.clientY <= rect.bottom;
    });
    if (!image || !image.currentSrc || image.naturalWidth < 1 || image.naturalHeight < 1) return;
    const link = image.closest("a[href]");
    const xMedia = location.origin === "https://x.com" && isXMediaUrl(image.currentSrc);
    if (!link && !xMedia && !document.contentType?.startsWith("image/")) return;
    // The enclosing link is presentation provenance (for example an X status
    // page), not necessarily the image payload. Always submit the bytes that
    // Firefox actually rendered as the media candidate.
    // The trusted click is the user's explicit-open action. X thumbnails are
    // rendition aliases of a stable public media object; request its original
    // rendition instead of permanently settling the small grid rendition.
    const mediaUrl = xMedia ? xOriginalUrl(image.currentSrc) : image.currentSrc;
    const presentationUrl = presentationUrlFor(image);
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
      mediaToken: mediaTokenFor(image),
    });
  }, true);
  }
})();
