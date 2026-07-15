#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Exercise a normalized MP4 through real Firefox without retaining media.

The supplied file is expected to be an ephemeral output from the approved
normalization worker or a DASObjectStore-managed staging mount.  It is only
read while this process runs.  The script exposes an in-memory-equivalent
single-range HTTP presentation, drives a local Firefox profile, and verifies
metadata, seek, pause/resume, and range delivery before deleting its profile.
"""

from __future__ import annotations

import argparse
import http.server
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from typing import ClassVar


class PlaybackHandler(http.server.BaseHTTPRequestHandler):
    """Minimal local range presenter used only for the browser evidence run."""

    video: ClassVar[pathlib.Path]
    events: ClassVar[set[str]] = set()
    ranges: ClassVar[list[str]] = []

    def log_message(self, _format: str, *_args: object) -> None:
        """Keep browser diagnostics free of request URLs and media names."""

    def do_POST(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
        if self.path != "/event":
            self.send_error(404)
            return
        size = int(self.headers.get("content-length", "0"))
        event = self.rfile.read(size).decode("ascii", "ignore").strip()
        if event:
            self.events.add(event)
        self.send_response(204)
        self.end_headers()

    def do_GET(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
        if self.path == "/probe":
            self._page()
            return
        if self.path == "/video":
            self._video()
            return
        self.send_error(404)

    def _page(self) -> None:
        page = b"""<!doctype html><meta charset=utf-8><title>starting</title>
<video id=v muted playsinline preload=auto src=/video></video>
<script>
const v = document.getElementById('v');
let resumed = false;
function event(name) { fetch('/event', {method: 'POST', body: name}); }
v.addEventListener('error', () => event('error'));
v.addEventListener('loadedmetadata', () => {
  event('loadedmetadata');
  v.currentTime = Math.min(0.25, Math.max(0, v.duration / 2));
});
v.addEventListener('seeked', () => { event('seeked'); v.play().catch(() => event('error')); });
v.addEventListener('play', () => {
  event(resumed ? 'resume' : 'play');
  if (!resumed) { resumed = true; setTimeout(() => v.pause(), 150); }
});
v.addEventListener('pause', () => { event('pause'); setTimeout(() => v.play().catch(() => event('error')), 50); });
fetch('/video', {headers: {Range: 'bytes=0-511'}})
  .then(r => r.status === 206 ? event('range') : event('error'))
  .catch(() => event('error'));
</script>"""
        self.send_response(200)
        self.send_header("content-type", "text/html; charset=utf-8")
        self.send_header("content-length", str(len(page)))
        self.end_headers()
        self.wfile.write(page)

    def _video(self) -> None:
        total = self.video.stat().st_size
        requested = self.headers.get("range")
        start, end = 0, total - 1
        if requested:
            try:
                unit, spec = requested.split("=", 1)
                start_text, end_text = spec.split("-", 1)
                if unit != "bytes" or not start_text or "," in spec:
                    raise ValueError
                start = int(start_text)
                end = min(int(end_text) if end_text else total - 1, total - 1)
                if start > end or start >= total:
                    raise ValueError
            except ValueError:
                self.send_response(416)
                self.send_header("content-range", f"bytes */{total}")
                self.end_headers()
                return
            self.ranges.append(requested)
            self.send_response(206)
            self.send_header("content-range", f"bytes {start}-{end}/{total}")
        else:
            self.send_response(200)
        length = end - start + 1
        self.send_header("content-type", "video/mp4")
        self.send_header("content-length", str(length))
        self.send_header("accept-ranges", "bytes")
        self.send_header("etag", '"firefox-normalized-playback-fixture"')
        self.end_headers()
        with self.video.open("rb") as source:
            source.seek(start)
            remaining = length
            while remaining:
                chunk = source.read(min(64 * 1024, remaining))
                if not chunk:
                    break
                self.wfile.write(chunk)
                remaining -= len(chunk)


def firefox_binary(value: str | None) -> str:
    if value:
        return value
    macos = "/Applications/Firefox.app/Contents/MacOS/firefox"
    if os.path.exists(macos):
        return macos
    discovered = shutil.which("firefox")
    if discovered:
        return discovered
    raise FileNotFoundError("Firefox was not found; pass --firefox")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--video", required=True, type=pathlib.Path)
    parser.add_argument("--firefox", help="Firefox binary; autodetected when omitted")
    parser.add_argument("--timeout", type=float, default=30.0)
    args = parser.parse_args()
    if not args.video.is_file() or args.video.stat().st_size == 0:
        parser.error("--video must name a non-empty normalized MP4")

    PlaybackHandler.video = args.video
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), PlaybackHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    profile = tempfile.mkdtemp(prefix="x-img-firefox-playback-")
    browser: subprocess.Popen[str] | None = None
    required = {"loadedmetadata", "seeked", "play", "pause", "resume", "range"}
    try:
        browser = subprocess.Popen(
            [
                firefox_binary(args.firefox),
                "--headless",
                "--no-remote",
                "--profile",
                profile,
                f"http://127.0.0.1:{server.server_port}/probe",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            text=True,
        )
        deadline = time.monotonic() + args.timeout
        while time.monotonic() < deadline:
            if "error" in PlaybackHandler.events:
                raise RuntimeError("Firefox reported a media error")
            if required.issubset(PlaybackHandler.events) and PlaybackHandler.ranges:
                print("Firefox normalized playback passed: metadata, range, seek, pause/resume")
                return 0
            time.sleep(0.1)
        missing = ", ".join(sorted(required - PlaybackHandler.events))
        raise RuntimeError(f"Firefox playback evidence timed out; missing: {missing or 'range'}")
    finally:
        if browser and browser.poll() is None:
            browser.terminate()
            try:
                browser.wait(timeout=5)
            except subprocess.TimeoutExpired:
                browser.kill()
        server.shutdown()
        server.server_close()
        shutil.rmtree(profile, ignore_errors=True)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (FileNotFoundError, RuntimeError) as error:
        print(f"Firefox normalized playback failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
