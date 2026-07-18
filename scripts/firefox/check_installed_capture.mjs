#!/usr/bin/env node
// SPDX-License-Identifier: MPL-2.0
// Real-Firefox synthetic capture evidence using an isolated temporary profile/add-on.

import assert from "node:assert/strict";
import childProcess from "node:child_process";
import fs from "node:fs";
import https from "node:https";
import net from "node:net";
import os from "node:os";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "../..");
const FIREFOX = process.env.FIREFOX || "/Applications/Firefox.app/Contents/MacOS/firefox";
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "pinakotheke-firefox-capture-"));
const profile = path.join(temporary, "profile");
const extension = path.join(temporary, "extension");
const xpi = path.join(temporary, "pinakotheke-test.xpi");
fs.mkdirSync(profile, { mode: 0o700 });
fs.cpSync(path.join(ROOT, "firefox-extension"), extension, { recursive: true });

function run(program, args, options = {}) {
  const result = childProcess.spawnSync(program, args, { encoding: "utf8", ...options });
  if (result.status !== 0) throw new Error(`${path.basename(program)} failed`);
  return result.stdout;
}

run("openssl", ["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
  "-subj", "/CN=127.0.0.1", "-addext", "subjectAltName=IP:127.0.0.1",
  "-keyout", path.join(temporary, "key.pem"), "-out", path.join(temporary, "cert.pem")],
{ stdio: "ignore" });

const captures = [];
let syntheticVideo = Buffer.alloc(0);
const svg = colour => Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="96" height="72"><rect width="96" height="72" fill="${colour}"/></svg>`);
const server = https.createServer({
  key: fs.readFileSync(path.join(temporary, "key.pem")),
  cert: fs.readFileSync(path.join(temporary, "cert.pem")),
}, (request, response) => {
  if (request.method === "GET" && /^\/products\/pinakotheke\/api\/extension\/v1\/capture-plans\/plan-\d+$/.test(request.url)) {
    const body = Buffer.from(JSON.stringify({
      schema_version: "pinakotheke.capture-plan-status.v1",
      plan_id: request.url.split("/").pop(),
      catalogue_id: "firefox-fixture-card",
      state: "stored",
    }));
    response.writeHead(200, { "content-type": "application/json", "content-length": body.length });
    response.end(body);
    return;
  }
  if (request.method === "POST" && request.url === "/products/pinakotheke/api/extension/v1/capture-plans") {
    const chunks = [];
    request.on("data", chunk => chunks.push(chunk));
    request.on("end", () => {
      captures.push(JSON.parse(Buffer.concat(chunks).toString("utf8")));
      const body = Buffer.from(JSON.stringify({ schema_version: "x-img.capture-plan.v1", plan_id: `plan-${captures.length}`, scheduler_job_id: "job-1" }));
      response.writeHead(200, { "content-type": "application/json", "content-length": body.length });
      response.end(body);
    });
    return;
  }
  if (request.method === "POST" && request.url === "/synthetic-video") {
    const chunks = [];
    request.on("data", chunk => chunks.push(chunk));
    request.on("end", () => {
      syntheticVideo = Buffer.concat(chunks);
      response.writeHead(syntheticVideo.length > 0 ? 204 : 400);
      response.end();
    });
    return;
  }
  if (request.url === "/synthetic-video.webm" && syntheticVideo.length > 0) {
    response.writeHead(200, {
      "content-type": "video/webm",
      "content-length": syntheticVideo.length,
      "accept-ranges": "bytes",
    });
    response.end(syntheticVideo);
    return;
  }
  if (request.url === "/video") {
    const body = Buffer.from(`<!doctype html><meta charset=utf-8><title>Synthetic video</title>
      <video id=clip width=160 height=120 muted playsinline></video>
      <script>
      (async () => {
        const canvas = document.createElement('canvas'); canvas.width = 160; canvas.height = 120;
        const context = canvas.getContext('2d'); context.fillStyle = '#176b87'; context.fillRect(0, 0, 160, 120);
        const stream = canvas.captureStream(10);
        const recorder = new MediaRecorder(stream, {mimeType: 'video/webm'}); const chunks = [];
        recorder.ondataavailable = event => { if (event.data.size) chunks.push(event.data); };
        const stopped = new Promise(resolve => recorder.onstop = resolve);
        recorder.start(); await new Promise(resolve => setTimeout(resolve, 250)); recorder.stop(); await stopped;
        stream.getTracks().forEach(track => track.stop());
        await fetch('/synthetic-video', {method: 'POST', body: new Blob(chunks, {type: 'video/webm'})});
        const video = document.querySelector('#clip'); video.src = '/synthetic-video.webm'; video.load();
        video.addEventListener('click', () => void video.play(), {once: true});
      })();
      </script>`);
    response.writeHead(200, { "content-type": "text/html; charset=utf-8", "content-length": body.length });
    response.end(body);
    return;
  }
  if (request.url === "/gallery") {
    const body = Buffer.from("<!doctype html><meta charset=utf-8><title>Synthetic gallery</title><a id=open href=/original.svg><img id=art src=/thumb.svg width=96 height=72 alt='Synthetic artwork'></a>");
    response.writeHead(200, { "content-type": "text/html; charset=utf-8", "content-length": body.length });
    response.end(body);
    return;
  }
  if (request.url === "/thumb.svg" || request.url === "/original.svg") {
    const body = svg(request.url === "/thumb.svg" ? "teal" : "green");
    response.writeHead(200, { "content-type": "image/svg+xml", "content-length": body.length });
    response.end(body);
    return;
  }
  response.writeHead(404); response.end();
});

await new Promise((resolve, reject) => {
  server.once("error", reject);
  server.listen(0, "127.0.0.1", resolve);
});
const origin = `https://127.0.0.1:${server.address().port}`;

const manifestPath = path.join(extension, "manifest.json");
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
manifest.host_permissions = ["https://127.0.0.1/*"];
manifest.content_scripts = [{
  matches: ["https://127.0.0.1/*"],
  js: ["content-explicit-open.js"],
  run_at: "document_idle",
}];
fs.writeFileSync(manifestPath, JSON.stringify(manifest));
const backgroundPath = path.join(extension, "background.js");
const background = fs.readFileSync(backgroundPath, "utf8");
const initializePoint = "async function initializeStorage(details) {";
assert.ok(background.includes(initializePoint));
fs.writeFileSync(backgroundPath, background.replace(
  initializePoint,
  `${initializePoint}\n  await browser.storage.local.set({instanceUrl:${JSON.stringify(origin)},instanceId:"test-instance",pairId:"test-pair",sites:[{origin:${JSON.stringify(origin)},capture:true,substitution:false,media:["images","videos"]}]});`,
));
fs.appendFileSync(path.join(extension, "content-explicit-open.js"), "\nvoid browser.runtime.sendMessage({command: \"run-cache\"});\n");
run("zip", ["-q", "-r", xpi, "."], { cwd: extension });

const remotePort = await new Promise((resolve, reject) => {
  const probe = net.createServer();
  probe.once("error", reject);
  probe.listen(0, "127.0.0.1", () => {
    const port = probe.address().port;
    probe.close(error => error ? reject(error) : resolve(port));
  });
});
const firefox = childProcess.spawn(FIREFOX, ["--headless", "--no-remote", "--profile", profile,
  "--remote-debugging-port", String(remotePort), "about:blank"], { stdio: "ignore" });

let socket;
let nextId = 1;
const pending = new Map();
function command(method, params) {
  return new Promise((resolve, reject) => {
    const id = nextId++;
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params }));
  });
}
async function connect() {
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    try {
      socket = await new Promise((resolve, reject) => {
        const candidate = new WebSocket(`ws://127.0.0.1:${remotePort}/session`);
        candidate.onopen = () => resolve(candidate);
        candidate.onerror = () => reject(new Error("Firefox BiDi unavailable"));
      });
      socket.onmessage = event => {
        const message = JSON.parse(event.data);
        const waiter = pending.get(message.id);
        if (!waiter) return;
        pending.delete(message.id);
        if (message.type === "success") waiter.resolve(message.result); else waiter.reject(new Error(message.message));
      };
      return;
    } catch (_) {
      await new Promise(resolve => setTimeout(resolve, 100));
    }
  }
  throw new Error("Firefox BiDi did not start");
}
async function waitFor(predicate, label) {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    if (await predicate()) return;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(`timed out waiting for ${label}`);
}

try {
  await connect();
  await command("session.new", { capabilities: { alwaysMatch: { acceptInsecureCerts: true } } });
  await command("webExtension.install", { extensionData: { type: "base64", value: fs.readFileSync(xpi).toString("base64") } });
  await new Promise(resolve => setTimeout(resolve, 750));
  const created = await command("browsingContext.create", { type: "tab" });
  const context = created.context;
  await command("browsingContext.activate", { context });
  await command("browsingContext.navigate", { context, url: `${origin}/gallery`, wait: "complete" });
  await waitFor(() => captures.some(item => item.capture_kind === "observed_thumbnail"), "observed thumbnail");
  await waitFor(async () => {
    const framed = await command("script.evaluate", {
      expression: "document.querySelector('#art')?.classList.contains('pinakotheke-stored-object') === true",
      target: { context }, awaitPromise: false,
    });
    return framed.result.type === "boolean" && framed.result.value;
  }, "stored thumbnail frame");
  const evaluated = await command("script.evaluate", {
    expression: "document.querySelector('#art')", target: { context }, awaitPromise: false,
  });
  assert.equal(evaluated.result.type, "node");
  await command("input.performActions", { context, actions: [{
    type: "pointer", id: "mouse", parameters: { pointerType: "mouse" }, actions: [
      { type: "pointerMove", x: 1, y: 1, duration: 0, origin: { type: "element", element: { sharedId: evaluated.result.sharedId } } },
      { type: "pointerDown", button: 0 }, { type: "pointerUp", button: 0 },
    ],
  }] });
  await waitFor(() => captures.some(item => item.capture_kind === "explicit_original"), "explicit original");
  const observed = captures.find(item => item.capture_kind === "observed_thumbnail");
  const original = captures.find(item => item.capture_kind === "explicit_original");
  assert.equal(observed.media_url, `${origin}/thumb.svg`);
  assert.equal(observed.presentation_url, `${origin}/original.svg`);
  assert.equal(original.media_url, `${origin}/thumb.svg`);
  assert.equal(original.presentation_url, `${origin}/original.svg`);
  assert.equal(observed.page_url, `${origin}/gallery`);
  assert.equal(original.page_url, `${origin}/gallery`);
  assert.equal(captures.some(item => item.cookie || item.headers || item.payload), false);
  await command("browsingContext.navigate", { context, url: `${origin}/video`, wait: "complete" });
  await new Promise(resolve => setTimeout(resolve, 1500));
  const playable = await command("script.evaluate", {
    expression: "document.querySelector('#clip')?.readyState >= 1 && document.querySelector('#clip')?.currentSrc.endsWith('/synthetic-video.webm')",
    target: { context }, awaitPromise: false,
  });
  assert.equal(playable.result.type, "boolean");
  assert.equal(playable.result.value, true, "Firefox must load the synthetic progressive video");
  const videoNode = await command("script.evaluate", {
    expression: "document.querySelector('#clip')", target: { context }, awaitPromise: false,
  });
  assert.equal(videoNode.result.type, "node");
  await command("input.performActions", { context, actions: [{
    type: "pointer", id: "video-mouse", parameters: { pointerType: "mouse" }, actions: [
      { type: "pointerMove", x: 10, y: 10, duration: 0, origin: { type: "element", element: { sharedId: videoNode.result.sharedId } } },
      { type: "pointerDown", button: 0 }, { type: "pointerUp", button: 0 },
    ],
  }] });
  await waitFor(() => captures.some(item => item.capture_kind === "explicit_video"), "trusted-play progressive video");
  await waitFor(async () => {
    const framed = await command("script.evaluate", {
      expression: "document.querySelector('#clip')?.classList.contains('pinakotheke-stored-object') === true && getComputedStyle(document.querySelector('#clip')).borderTopWidth === '2px'",
      target: { context }, awaitPromise: false,
    });
    return framed.result.type === "boolean" && framed.result.value;
  }, "stored video frame");
  const video = captures.find(item => item.capture_kind === "explicit_video");
  assert.equal(video.media_url, `${origin}/synthetic-video.webm`);
  assert.equal(video.page_url, `${origin}/video`);
  assert.equal(captures.some(item => item.cookie || item.headers || item.payload), false);
  console.log("Installed Firefox capture passed: automatic thumbnail, trusted opened image/video, and verified 2px stored frames");
  await command("session.end", {});
} finally {
  if (socket?.readyState === WebSocket.OPEN) socket.close();
  if (firefox.exitCode === null) {
    firefox.kill("SIGTERM");
    await Promise.race([
      new Promise(resolve => firefox.once("exit", resolve)),
      new Promise(resolve => setTimeout(resolve, 5_000)),
    ]);
  }
  await new Promise(resolve => server.close(resolve));
  fs.rmSync(temporary, { recursive: true, force: true });
}
