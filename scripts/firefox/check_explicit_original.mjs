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
  tabs: { async query() { return []; } },
  scripting: {
    async executeScript() { return []; },
    async getRegisteredContentScripts() { return registeredScripts; },
    async unregisterContentScripts() { registeredScripts = []; },
    async registerContentScripts(scripts) { registeredScripts = scripts; },
  },
};
const fetchFixture = async (url, options) => {
  if (String(url).startsWith("moz-extension://")) {
    return { async json() { return registry; } };
  }
  captures.push({ url: String(url), options });
  return { ok: true };
};
const source = fs.readFileSync("firefox-extension/background.js", "utf8");
const backgroundContext = vm.createContext({
  browser,
  fetch: fetchFixture,
  URL,
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
  browser: { runtime: { sendMessage(message) { contentMessages.push(message); } } },
  document: {
    contentType: "text/html",
    addEventListener(kind, callback, capture) {
      assert.equal(kind, "click");
      assert.equal(capture, true);
      clickListener = callback;
    },
  },
  Element: FixtureElement,
  URL,
});
assert.equal(typeof clickListener, "function");
clickListener({ isTrusted: false, button: 0, target: openedImage });
assert.equal(contentMessages.length, 0, "synthetic clicks must be ignored");
clickListener({ isTrusted: true, button: 0, target: openedImage });
assert.equal(contentMessages.length, 1);
assert.equal(contentMessages[0].command, "explicit-original-opened");
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
assert.equal(storage.siteDiagnostics[body.origin].state, "Original queued");

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
