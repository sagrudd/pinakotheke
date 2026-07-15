#!/usr/bin/env node
// SPDX-License-Identifier: MPL-2.0
// Synthetic Firefox WebExtension upgrade contract; no browser profile or payload is retained.

import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const current = JSON.parse(fs.readFileSync("firefox-extension/manifest.json", "utf8"));
const candidate = JSON.parse(
  fs.readFileSync("packaging/firefox/pinakotheke-manifest.v1.candidate.json", "utf8"),
);
assert.equal(candidate.name, "Pinakotheke");
assert.equal(candidate.version, "1.0.0");
assert.equal(
  candidate.browser_specific_settings.gecko.id,
  current.browser_specific_settings.gecko.id,
  "the published Gecko ID must remain stable",
);
assert.deepEqual(candidate.permissions, current.permissions);
assert.deepEqual(candidate.optional_host_permissions, current.optional_host_permissions);
assert.deepEqual(candidate.content_security_policy, current.content_security_policy);

let installed;
const storage = {
  instanceUrl: "https://pinakotheke.example.invalid",
  instanceId: "instance-stable",
  pairId: "pairing-stable",
  sites: [{
    origin: "https://art.example.invalid",
    capture: true,
    substitution: false,
    media: ["images"],
  }],
  endpointId: "endpoint-stable",
  objectStoreId: "store-stable",
};
const browser = {
  runtime: {
    onInstalled: { addListener(callback) { installed = callback; } },
    onMessage: { addListener() {} },
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
vm.runInNewContext(fs.readFileSync("firefox-extension/background.js", "utf8"), {
  browser, fetch, URL, AbortController, Blob, setTimeout, clearTimeout,
});
assert.equal(typeof installed, "function");

const before = structuredClone(storage);
await installed({ reason: "update", previousVersion: "0.9.0" });
assert.deepEqual(storage, before, "upgrade must preserve pairing, rules, endpoint, and ObjectStore");

for (const key of Object.keys(storage)) delete storage[key];
await installed({ reason: "install" });
assert.equal(
  JSON.stringify(storage),
  JSON.stringify({ instanceUrl: "", instanceId: "", pairId: "", sites: [] }),
);

console.log("Firefox identity upgrade passed: stable Gecko ID and preserved extension state");
