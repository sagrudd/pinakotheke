#!/usr/bin/env node
// SPDX-License-Identifier: MPL-2.0
// Synthetic WebExtension event contract for explicit user-opened originals.

import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

let messageListener;
let startupListener;
const captures = [];
let registeredScripts = [];
const fileInjections = [];
const storage = {
  instanceUrl: "https://pinakotheke.example.invalid",
  instanceId: "instance-1",
  pairId: "pair-1",
  sites: [{
    origin: "https://art.example.invalid:8443",
    capture: true,
    substitution: false,
    media: ["images"],
  }],
};
const registry = {
  adapters: [{
    id: "generic",
    kind: "experimental_generic",
    version: "1.0.0",
    origins: ["https://example.invalid"],
    exclude_paths: ["/login", "/settings"],
    capabilities: {
      observed_thumbnail: true,
      explicit_original: true,
      image_substitution: false,
      mp4_substitution: false,
    },
  }],
};
const browser = {
  runtime: {
    onInstalled: { addListener() {} },
    onStartup: { addListener(callback) { startupListener = callback; } },
    onMessage: { addListener(callback) { messageListener = callback; } },
    getURL(path) { return `moz-extension://fixture/${path}`; },
  },
  storage: {
    local: {
      async get(keys) {
        return Object.fromEntries(keys.filter(key => storage[key] !== undefined).map(key => [key, storage[key]]));
      },
      async set(values) { Object.assign(storage, values); },
    },
  },
  tabs: {
    async query() { return [{ id: 73, url: "https://art.example.invalid:8443/gallery" }]; },
    async sendMessage() {},
  },
  scripting: {
    async executeScript(details) {
      if (details.files) {
        fileInjections.push(details);
        return [];
      }
      return [{ result: details.func() }];
    },
    async getRegisteredContentScripts() { return registeredScripts; },
    async unregisterContentScripts() { registeredScripts = []; },
    async registerContentScripts(scripts) { registeredScripts = scripts; },
  },
};
const fetchFixture = async (url, options) => {
  if (String(url).startsWith("moz-extension://")) {
    return { async json() { return registry; } };
  }
  if (String(url).includes("/capture-plans/")) {
    return { ok: true, async json() { return { schema_version: "pinakotheke.capture-plan-status.v1", state: "stored" }; } };
  }
  captures.push({ url: String(url), options });
  return { ok: true, async json() { return { plan_id: "capture-plan-fixture" }; } };
};
const source = fs.readFileSync("firefox-extension/background.js", "utf8");
const backgroundContext = vm.createContext({
  browser,
  fetch: fetchFixture,
  URL,
  URLSearchParams,
  AbortController,
  Blob,
  setTimeout,
  clearTimeout,
  document: { images: [] },
  window: { innerHeight: 800, innerWidth: 1200, getComputedStyle() { return { display: "block", visibility: "visible", opacity: "1" }; } },
});
vm.runInContext(source, backgroundContext);
assert.equal(typeof messageListener, "function");
assert.equal(typeof startupListener, "function");

backgroundContext.document.images = [{
  complete: true,
  currentSrc: "https://media.example.invalid/thumb.jpg?signed=drop",
  naturalWidth: 320,
  naturalHeight: 200,
  closest(selector) { return selector === "a[href]" ? { href: "https://media.example.invalid/open.jpg?signed=drop" } : null; },
  getBoundingClientRect() { return { width: 320, height: 200, top: 0, left: 0, bottom: 200, right: 320 }; },
}];
const observed = vm.runInContext("displayedImages()", backgroundContext);
assert.equal(observed.length, 1);
assert.equal(observed[0].url, "https://media.example.invalid/thumb.jpg?signed=drop");
assert.equal(observed[0].presentationUrl, "https://media.example.invalid/open.jpg?signed=drop");

const xRule = { xIngress: true };
const xObserved = vm.runInContext(`eligibleObservedImages("https://x.com", ${JSON.stringify(xRule)}, [
  { url: "https://pbs.twimg.com/media/synthetic.jpg" },
  { url: "https://abs.twimg.com/emoji/v2/svg/1f380.svg" },
  { url: "not-a-url" }
])`, backgroundContext);
assert.deepEqual(Array.from(xObserved, item => item.url), ["https://pbs.twimg.com/media/synthetic.jpg"]);
const genericObserved = vm.runInContext(`eligibleObservedImages("https://art.example.invalid", {}, [
  { url: "https://media.example.invalid/synthetic.jpg" }
])`, backgroundContext);
assert.equal(genericObserved.length, 1);

storage.sites.push({
  origin: "https://x.com",
  capture: true,
  substitution: false,
  xIngress: true,
  media: ["images"],
});
storage.instanceId = "";
backgroundContext.document.images = [];
await messageListener(
  { command: "visible-media-changed", images: [{
    url: "https://pbs.twimg.com/media/visible-thumbnail.jpg?format=jpg&name=small",
    presentationUrl: "https://x.com/FixtureArtist/status/42",
    width: 640,
    height: 480,
  }] },
  { tab: { id: 8, url: "https://x.com/home" } },
);
await new Promise(resolve => setImmediate(resolve));
assert.equal(captures.length, 1, "a merely visible opted-in X thumbnail must be submitted");
const observedBody = JSON.parse(captures[0].options.body);
assert.equal(observedBody.capture_kind, "observed_thumbnail");
assert.equal(observedBody.media_url, "https://pbs.twimg.com/media/visible-thumbnail.jpg?format=jpg&name=small");
assert.equal(observedBody.presentation_url, "https://x.com/FixtureArtist/status/42");
captures.length = 0;
storage.sites.pop();
storage.instanceId = "instance-1";

assert.equal(
  vm.runInContext('canonicalAlias("https://pbs.twimg.com/media/fixture?name=small&format=jpg&token=drop")', backgroundContext),
  "https://pbs.twimg.com/media/fixture?format=jpg&name=small",
);

const sync = await messageListener({ command: "sync-capture-observers" }, {});
assert.equal(sync.registered, 1);
assert.equal(sync.injected, 1);
assert.equal(registeredScripts.length, 1);
assert.equal(fileInjections.length, 1);
assert.equal(fileInjections[0].target.tabId, 73);
assert.deepEqual(Array.from(fileInjections[0].files), ["content-explicit-open.js"]);
assert.deepEqual(Array.from(registeredScripts[0].matches), ["https://art.example.invalid/*"]);
assert.deepEqual(Array.from(registeredScripts[0].excludeMatches), [
  "https://art.example.invalid/login*",
  "https://art.example.invalid/settings*",
]);
assert.equal(registeredScripts[0].persistAcrossSessions, true);
assert.equal(registeredScripts[0].allFrames, false);

const contentListeners = new Map();
const contentMessages = [];
class FixtureElement {
  closest(selector) {
    if (selector === "img") return this;
    if (selector === "video" && this instanceof FixtureVideo) return this;
    if (selector === "a[href]") return { href: "https://media.example.invalid/open.jpg?signed=drop" };
    return null;
  }
}
class FixtureVideo extends FixtureElement {}
const openedImage = new FixtureElement();
openedImage.currentSrc = "https://media.example.invalid/thumb.jpg";
openedImage.naturalWidth = 2048;
openedImage.naturalHeight = 1365;
const contentSource = fs.readFileSync("firefox-extension/content-explicit-open.js", "utf8");
const contentDocument = {
  contentType: "text/html",
  documentElement: {},
  head: { append() {} },
  createElement() { return { textContent: "" }; },
  querySelectorAll() { return []; },
  addEventListener(kind, callback, capture) {
    contentListeners.set(kind, callback);
    if (["click", "pointerdown", "play"].includes(kind)) assert.equal(capture, true);
  },
};
vm.runInNewContext(contentSource, {
  browser: { runtime: {
    sendMessage(message) { contentMessages.push(message); },
    onMessage: { addListener() {} },
  } },
  document: contentDocument,
  Element: FixtureElement,
  HTMLVideoElement: FixtureVideo,
  MutationObserver: class { observe() {} },
  setTimeout() { return 1; },
  clearTimeout() {},
  performance: {
    now() { return 5000; },
    getEntriesByType(type) {
      assert.equal(type, "resource");
      return [
        { name: "https://cdn.example.invalid/unrelated.mp4", initiatorType: "video", startTime: 100 },
        { name: "https://video.twimg.com/amplify_video/42/pl/avc1/1280x720/video.m3u8", initiatorType: "fetch", startTime: 5070 },
        { name: "https://video.twimg.com/amplify_video/42/pl/master.m3u8", initiatorType: "fetch", startTime: 5060 },
        { name: "https://video.twimg.com/amplify_video/42/vid/avc1/1280x720/segment.m4s", initiatorType: "fetch", startTime: 5120 },
        { name: "https://video.twimg.com/amplify_video/fixture/vid/avc1/1280x720/asset.mp4?token=ephemeral", initiatorType: "fetch", startTime: 5100 },
        { name: "https://cdn.example.invalid/page-script", initiatorType: "script", startTime: 5150 },
      ];
    },
  },
  location: { href: "https://art.example.invalid:8443/watch/fixture" },
  URL,
});
const clickListener = contentListeners.get("click");
assert.equal(typeof clickListener, "function");
clickListener({ isTrusted: false, button: 0, target: openedImage });
assert.equal(contentMessages.length, 0, "synthetic clicks must be ignored");
clickListener({ isTrusted: true, button: 0, target: openedImage });
assert.equal(contentMessages.length, 1);
assert.equal(contentMessages[0].command, "explicit-original-opened");
assert.equal(contentMessages[0].mediaUrl, "https://media.example.invalid/thumb.jpg");
assert.equal(contentMessages[0].presentationUrl, "https://media.example.invalid/open.jpg?signed=drop");
assert.equal(contentMessages[0].width, 2048);

const playedVideo = new FixtureVideo();
playedVideo.currentSrc = "blob:https://art.example.invalid/fixture";
playedVideo.videoWidth = 1280;
playedVideo.videoHeight = 720;
playedVideo.clientWidth = 640;
playedVideo.clientHeight = 360;
contentListeners.get("pointerdown")({ isTrusted: true, target: playedVideo });
contentListeners.get("play")({ isTrusted: true, target: playedVideo });
assert.equal(contentMessages.length, 2);
assert.equal(contentMessages[1].command, "explicit-video-opened");
assert.equal(contentMessages[1].mediaUrl, "https://video.twimg.com/amplify_video/42/pl/master.m3u8");
assert.equal(contentMessages[1].width, 1280);

const overlayPlayedVideo = new FixtureVideo();
overlayPlayedVideo.currentSrc = "blob:https://art.example.invalid/overlay";
overlayPlayedVideo.videoWidth = 854;
overlayPlayedVideo.videoHeight = 480;
overlayPlayedVideo.clientWidth = 427;
overlayPlayedVideo.clientHeight = 240;
overlayPlayedVideo.getBoundingClientRect = () => ({ left: 100, top: 100, right: 527, bottom: 340, width: 427, height: 240 });
const overlayControl = new FixtureElement();
contentDocument.querySelectorAll = selector => selector === "video" ? [overlayPlayedVideo] : [];
contentListeners.get("pointerdown")({ isTrusted: true, type: "pointerdown", target: overlayControl, clientX: 320, clientY: 220 });
contentListeners.get("play")({ isTrusted: true, target: overlayPlayedVideo });
assert.equal(contentMessages.length, 3);
assert.equal(contentMessages[2].command, "explicit-video-opened");
assert.equal(contentMessages[2].width, 854);

const sender = { tab: { id: 7, url: "https://art.example.invalid:8443/gallery?private=drop" } };
const result = await messageListener({
  command: "explicit-original-opened",
  mediaUrl: "https://media.example.invalid/original.jpg?signed=drop#fragment",
  presentationUrl: "https://media.example.invalid/original.jpg?signed=drop#fragment",
  width: 1920,
  height: 1080,
}, sender);
assert.equal(result.completed, true);
assert.equal(captures.length, 1);
assert.equal(
  captures[0].url,
  "https://pinakotheke.example.invalid/products/pinakotheke/api/extension/v1/capture-plans",
);
const body = JSON.parse(captures[0].options.body);
assert.equal(body.capture_kind, "explicit_original");
assert.equal(body.origin, "https://art.example.invalid:8443");
assert.equal(body.page_url, sender.tab.url, "page provenance must come from Firefox sender state");
assert.equal(body.media_url, "https://media.example.invalid/original.jpg");
assert.equal(body.presentation_url, "https://media.example.invalid/original.jpg");
assert.equal(body.width, 1920);
assert.equal(storage.siteDiagnostics[body.origin].state, "Stored in ObjectStore");
assert.equal(storage.siteDiagnostics[body.origin].storedInObjectStore, true);

captures.length = 0;
storage.sites[0].media.push("videos");
const videoResult = await messageListener({
  command: "explicit-video-opened",
  mediaUrl: "https://cdn.example.invalid/progressive/opaque-asset?token=ephemeral#fragment",
  presentationUrl: "https://art.example.invalid:8443/watch/fixture",
  width: 1280,
  height: 720,
}, sender);
assert.equal(videoResult.completed, true);
assert.equal(captures.length, 1);
const videoBody = JSON.parse(captures[0].options.body);
assert.equal(videoBody.capture_kind, "explicit_video");
assert.equal(videoBody.media_url, "https://cdn.example.invalid/progressive/opaque-asset?token=ephemeral");
assert.equal(videoBody.origin, "https://art.example.invalid:8443");

storage.sites[0].capture = false;
await messageListener({
  command: "explicit-original-opened",
  mediaUrl: "https://media.example.invalid/blocked.jpg",
  width: 10,
  height: 10,
}, sender);
assert.equal(captures.length, 1, "paused site policy must block explicit capture");
await messageListener({ command: "sync-capture-observers" }, {});
assert.equal(registeredScripts.length, 0, "pausing capture removes the persistent observer");

console.log("Firefox explicit-media contract passed: persistent observer, trusted image/video activation, generic progressive request");
