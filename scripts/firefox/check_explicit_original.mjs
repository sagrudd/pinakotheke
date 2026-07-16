#!/usr/bin/env node
// SPDX-License-Identifier: MPL-2.0
// Synthetic WebExtension event contract for explicit user-opened originals.

import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

let messageListener;
const captures = [];
const storage = {
  instanceUrl: "https://pinakotheke.example.invalid",
  pairId: "pair-1",
  sites: [{
    origin: "https://art.example.invalid",
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
  scripting: { async executeScript() { return []; } },
};
const fetchFixture = async (url, options) => {
  if (String(url).startsWith("moz-extension://")) {
    return { async json() { return registry; } };
  }
  captures.push({ url: String(url), options });
  return { ok: true };
};
const source = fs.readFileSync("firefox-extension/background.js", "utf8");
assert.match(source, /event\.isTrusted/);
assert.match(source, /event\.button !== 0/);
assert.match(source, /closest\("img"\)/);
assert.match(source, /document\.contentType\?\.startsWith\("image\/"\)/);
vm.runInNewContext(source, {
  browser,
  fetch: fetchFixture,
  URL,
  AbortController,
  Blob,
  setTimeout,
  clearTimeout,
});
assert.equal(typeof messageListener, "function");

const sender = { tab: { id: 7, url: "https://art.example.invalid/gallery?private=drop" } };
const result = await messageListener({
  command: "explicit-original-opened",
  mediaUrl: "https://media.example.invalid/original.jpg?signed=drop#fragment",
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
assert.equal(body.origin, "https://art.example.invalid");
assert.equal(body.page_url, sender.tab.url, "page provenance must come from Firefox sender state");
assert.equal(body.media_url, "https://media.example.invalid/original.jpg");
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

console.log("Firefox explicit-original contract passed: trusted click, opt-in, generic adapter, canonical request");
