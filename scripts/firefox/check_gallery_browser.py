#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Exercise the real Yew gallery bundle in installed Firefox with synthetic metadata.

This is browser-component evidence.  The loopback server represents an already
authenticated Monas forwarding surface; it is not a DASObjectStore substitute
and never writes media or browser data to the repository.
"""

from __future__ import annotations

import argparse
import http.server
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import urllib.parse
from typing import ClassVar


CHECKSUM = "sha256:" + "a" * 64
PNG = bytes.fromhex("89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000d49444154789c63606060f80f0001040100d0d42a5d0000000049454e44ae426082")
INJECT = r"""
<script>
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const waitFor = async (test, name) => {
  for (let i=0; i<200; i++) { const value=test(); if (value) return value; await sleep(50); }
  throw new Error('timeout:'+name);
};
const event = name => fetch('/event', {method:'POST', body:name});
addEventListener('TrunkApplicationStarted', async () => {
 try {
 await waitFor(() => document.querySelector('.ximg-gallery__card'), 'cards');
  const mobileMode = new URLSearchParams(location.search).get('mode') === 'mobile';
  if (mobileMode) {
    const main = document.querySelector('.ximg-shell__main'); main.style.width='390px'; main.style.maxWidth='390px';
    const viewport = document.getElementById('gallery-scroll'); viewport.dispatchEvent(new Event('scroll', {bubbles:true}));
    await waitFor(() => Number(getComputedStyle(document.querySelector('.ximg-gallery__grid')).getPropertyValue('--gallery-columns')) <= 2, 'mobile-reflow');
    await event('mobile-responsive'); return;
  }
  const columns = Number(getComputedStyle(document.querySelector('.ximg-gallery__grid')).getPropertyValue('--gallery-columns'));
  if (columns < 3) throw new Error('desktop-columns');
  await event('desktop-responsive');
  for (let i=0; i<4; i++) {
    const more = await waitFor(() => document.querySelector('.ximg-gallery__more'), 'load-more');
    more.click();
    await waitFor(() => document.querySelector('[role=status]')?.textContent.includes(`Loaded ${(i+2)*100} `), 'page');
  }
  const viewport = document.getElementById('gallery-scroll');
  if (document.querySelectorAll('.ximg-gallery__card').length > 120) throw new Error('unbounded-dom');
  for (let attempt=0; attempt<3; attempt++) {
    viewport.scrollTop = viewport.scrollHeight;
    viewport.dispatchEvent(new Event('scroll', {bubbles:true}));
    await sleep(150);
  }
  const visible = [...document.querySelectorAll('.ximg-gallery__card')];
  if (!visible.some(card => Number(card.id.split('-').pop()) > 400)) {
    throw new Error('window-end:' + viewport.scrollTop + ':' + viewport.scrollHeight + ':' + visible.map(card => card.id).join(','));
  }
  if (visible.length > 120) throw new Error('unbounded-window-end');
  visible[0].focus();
  visible[0].dispatchEvent(new KeyboardEvent('keydown', {key:'End', bubbles:true}));
  await waitFor(() => document.activeElement?.id === 'preview-trigger-499', 'keyboard-end');
  await event('bounded-window-keyboard');
  const input = document.querySelector('.ximg-filters input');
  input.value = 'needle';
  input.dispatchEvent(new InputEvent('input', {bubbles:true, data:'needle', inputType:'insertText'}));
  await waitFor(() => document.querySelector('[role=status]')?.textContent.includes('of 20 catalogue'), 'filter');
  await event('server-filter');
  input.value = 'unavailable';
  input.dispatchEvent(new InputEvent('input', {bubbles:true, data:'unavailable', inputType:'insertText'}));
  await waitFor(() => document.querySelector('[role=status]')?.textContent.includes('of 28 catalogue'), 'unavailable-filter');
  const card = await waitFor(() => [...document.querySelectorAll('.ximg-gallery__card')].find(node => node.textContent.includes('Object unavailable')), 'unavailable');
  card.click();
  await waitFor(() => document.querySelector('.ximg-preview__unavailable'), 'unavailable-preview');
  if (performance.getEntriesByType('resource').some(entry => !entry.name.startsWith(location.origin))) throw new Error('origin-request');
  await event('unavailable-no-origin');
 } catch (error) { await event('error:' + String(error.message || error)); }
});
</script>
"""


def item(index: int) -> dict[str, object]:
    unavailable = index % 37 == 0
    needle = index % 50 == 0
    title = f"{'Needle ' if needle else ''}{'Unavailable ' if unavailable else ''}synthetic media {index:04d}"
    source = "x_account" if index % 2 == 0 else "website"
    video = index % 5 == 0
    thumbnail = {
        "kind": "video_poster" if video else "thumbnail",
        "availability": "unavailable" if unavailable else "ready",
        "endpoint_id": "firefox-fixture-endpoint",
        "object_store_id": "firefox-fixture-store",
        "object_key": f"fixture/{index}/thumbnail.png",
        "checksum": CHECKSUM,
        "content_type": "image/png",
        "content_length": len(PNG),
        "delivery_path": None if unavailable else f"/products/pinakotheke/api/gallery/v1/objects/{index}/thumbnail",
    }
    return {
        "catalogue_id": f"fixture-{index:04d}", "title": title,
        "source_label": "Synthetic X account" if source == "x_account" else "Synthetic website",
        "source_kind": source, "media_kind": "normalized_video" if video else "image",
        "review_state": ["new", "reviewed", "hidden"][index % 3],
        "discovered_at_epoch_seconds": 2_000_000_000 - index,
        "width": 640, "height": 480, "thumbnail": thumbnail, "preview": None,
    }


ITEMS = [item(index) for index in range(1000)]


class Handler(http.server.BaseHTTPRequestHandler):
    root: ClassVar[pathlib.Path]
    events: ClassVar[set[str]] = set()

    def log_message(self, _format: str, *_args: object) -> None:
        pass

    def do_POST(self) -> None:  # noqa: N802
        if self.path != "/event": self.send_error(404); return
        size = int(self.headers.get("content-length", "0"))
        self.events.add(self.rfile.read(size).decode("utf-8", "replace"))
        self.send_response(204); self.end_headers()

    def do_GET(self) -> None:  # noqa: N802
        parsed = urllib.parse.urlparse(self.path)
        if parsed.path == "/products/pinakotheke/api/gallery/v1/catalogue": self._catalogue(parsed.query); return
        if parsed.path.startswith("/products/pinakotheke/api/gallery/v1/objects/"): self._png(); return
        relative = parsed.path.removeprefix("/products/pinakotheke/app/") or "index.html"
        target = self.root / relative
        if target.name == "index.html": self._index(target); return
        if target.is_file() and target.parent == self.root: self._file(target); return
        self.send_error(404)

    def _catalogue(self, query: str) -> None:
        values = urllib.parse.parse_qs(query)
        selected = ITEMS
        source = values.get("source_kind", [None])[0]
        text = values.get("text", [""])[0].lower()
        if source: selected = [entry for entry in selected if entry["source_kind"] == source]
        if text: selected = [entry for entry in selected if text in str(entry["title"]).lower() or text in str(entry["source_label"]).lower()]
        offset = int(values.get("offset", [0])[0]); limit = min(int(values.get("limit", [100])[0]), 200)
        page = selected[offset:offset+limit]
        payload = json.dumps({"schema_version":"pinakotheke.gallery-catalogue.v1", "items":page,
            "next_offset": offset+len(page) if offset+len(page) < len(selected) else None,
            "matched_items":len(selected), "total_items":len(ITEMS)}).encode()
        self.send_response(200); self.send_header("content-type", "application/json"); self.send_header("content-length", str(len(payload))); self.end_headers(); self.wfile.write(payload)

    def _png(self) -> None:
        self.send_response(200); self.send_header("content-type", "image/png"); self.send_header("content-length", str(len(PNG))); self.end_headers(); self.wfile.write(PNG)

    def _index(self, target: pathlib.Path) -> None:
        payload = target.read_text().replace("</body>", INJECT + "</body>").encode()
        self.send_response(200); self.send_header("content-type", "text/html; charset=utf-8"); self.send_header("content-length", str(len(payload))); self.end_headers(); self.wfile.write(payload)

    def _file(self, target: pathlib.Path) -> None:
        payload = target.read_bytes(); content = "application/wasm" if target.suffix == ".wasm" else "text/javascript" if target.suffix == ".js" else "text/css"
        self.send_response(200); self.send_header("content-type", content); self.send_header("content-length", str(len(payload))); self.end_headers(); self.wfile.write(payload)


def firefox_binary(value: str | None) -> str:
    if value: return value
    macos = "/Applications/Firefox.app/Contents/MacOS/firefox"
    if os.path.exists(macos): return macos
    found = shutil.which("firefox")
    if found: return found
    raise FileNotFoundError("Firefox was not found; pass --firefox")


def run_browser(binary: str, url: str, width: int, required: set[str], timeout: float) -> None:
    profile = tempfile.mkdtemp(prefix="pinakotheke-gallery-firefox-")
    environment = os.environ.copy(); environment["MOZ_HEADLESS_WIDTH"] = str(width); environment["MOZ_HEADLESS_HEIGHT"] = "900"
    process = subprocess.Popen([binary, "--headless", "--no-remote", "--new-instance", "--profile", profile, url], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=environment)
    try:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            errors = [event for event in Handler.events if event.startswith("error:")]
            if errors: raise RuntimeError(errors[0])
            if required.issubset(Handler.events): return
            time.sleep(0.1)
        raise RuntimeError("Firefox gallery evidence timed out: " + ", ".join(sorted(required - Handler.events)))
    finally:
        process.terminate()
        try: process.wait(timeout=5)
        except subprocess.TimeoutExpired: process.kill()
        shutil.rmtree(profile, ignore_errors=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument("--dist", type=pathlib.Path, required=True); parser.add_argument("--firefox"); parser.add_argument("--timeout", type=float, default=30)
    args = parser.parse_args()
    if not (args.dist / "index.html").is_file(): parser.error("--dist must contain a Trunk build")
    Handler.root = args.dist.resolve(); server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler); thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
    try:
        url = f"http://127.0.0.1:{server.server_port}/products/pinakotheke/app/"
        run_browser(firefox_binary(args.firefox), url, 1280, {"desktop-responsive", "bounded-window-keyboard", "server-filter", "unavailable-no-origin"}, args.timeout)
        run_browser(firefox_binary(args.firefox), url + "?mode=mobile", 390, {"mobile-responsive"}, args.timeout)
        print("Firefox Yew gallery passed: 1000 records, bounded window, keyboard, filters, unavailable, desktop/mobile")
        return 0
    finally: server.shutdown(); server.server_close()


if __name__ == "__main__":
    try: raise SystemExit(main())
    except (FileNotFoundError, RuntimeError) as error: print(f"Firefox gallery failed: {error}", file=sys.stderr); raise SystemExit(1) from error
