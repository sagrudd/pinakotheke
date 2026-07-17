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
  tabs: { async query() { return []; }, async sendMessage() {} },
  scripting: {
    async executeScript({ func }) { return [{ result: func() }]; },
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
assert.equal(registeredScripts.length, 1);
assert.deepEqual(Array.from(registeredScripts[0].matches), ["https://art.example.invalid/*"]);
assert.deepEqual(Array.from(registeredScripts[0].excludeMatches), [
  "https://art.example.invalid/login*",
  "https://art.example.invalid/settings*",
]);
assert.equal(registeredScripts[0].persistAcrossSessions, true);
assert.equal(registeredScripts[0].allFrames, false);

let clickListener;
const contentMessages = [];
class FixtureElement {
  closest(selector) {
    if (selector === "img") return this;
    if (selector === "a[href]") return { href: "https://media.example.invalid/open.jpg?signed=drop" };
    return null;
  }
}
const openedImage = new FixtureElement();
openedImage.currentSrc = "https://media.example.invalid/thumb.jpg";
openedImage.naturalWidth = 2048;
openedImage.naturalHeight = 1365;
const contentSource = fs.readFileSync("firefox-extension/content-explicit-open.js", "utf8");
vm.runInNewContext(contentSource, {
  browser: { runtime: {
    sendMessage(message) { contentMessages.push(message); },
    onMessage: { addListener() {} },
  } },
  document: {
    contentType: "text/html",
    documentElement: {},
    head: { append() {} },
    createElement() { return { textContent: "" }; },
    querySelectorAll() { return []; },
    addEventListener(kind, callback, capture) {
      if (kind === "click") {
        assert.equal(capture, true);
        clickListener = callback;
      }
    },
  },
  Element: FixtureElement,
  HTMLVideoElement: class {},
  MutationObserver: class { observe() {} },
  setTimeout() { return 1; },
  clearTimeout() {},
  URL,
});
assert.equal(typeof clickListener, "function");
clickListener({ isTrusted: false, button: 0, target: openedImage });
assert.equal(contentMessages.length, 0, "synthetic clicks must be ignored");
clickListener({ isTrusted: true, button: 0, target: openedImage });
assert.equal(contentMessages.length, 1);
assert.equal(contentMessages[0].command, "explicit-original-opened");
assert.equal(contentMessages[0].mediaUrl, "https://media.example.invalid/thumb.jpg");
assert.equal(contentMessages[0].presentationUrl, "https://media.example.invalid/open.jpg?signed=drop");
assert.equal(contentMessages[0].width, 2048);

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

console.log("Firefox explicit-original contract passed: persistent exact-origin observer, trusted click, canonical request");
