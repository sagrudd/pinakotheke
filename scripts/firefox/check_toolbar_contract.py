#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Validate the least-privilege, redacted Firefox toolbar contract."""

from __future__ import annotations

import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[2]
EXTENSION = ROOT / "firefox-extension"


def main() -> int:
    manifest = json.loads((EXTENSION / "manifest.json").read_text())
    assert manifest["action"]["default_popup"] == "popup.html"
    forbidden = {"cookies", "history", "webRequestBlocking"}
    assert forbidden.isdisjoint(manifest["permissions"])
    assert "webRequest" in manifest["permissions"]
    assert manifest["host_permissions"] == []
    assert "https://video.twimg.com/*" in manifest["optional_host_permissions"]
    popup = (EXTENSION / "popup.html").read_text()
    popup_script = (EXTENSION / "popup.js").read_text()
    options_script = (EXTENSION / "options.js").read_text()
    background = (EXTENSION / "background.js").read_text()
    assert "Pinakotheke cache" in popup
    assert 'id="extension-version"' in popup
    assert "browser.runtime.getManifest()" in popup_script
    assert manifest["version"] in (ROOT / "Cargo.toml").read_text()
    for phrase in (
        "Previously observed",
        "Stored in ObjectStore",
        "Pause substitution",
        "Open Pinakotheke source view",
    ):
        assert phrase in popup + popup_script
    for forbidden_text in ("requestHeaders", "requestBody", "onAuthRequired"):
        assert forbidden_text not in background + popup_script
    assert 'browser.webRequest.onCompleted.addListener' in background
    assert 'https://video.twimg.com/*' in background + options_script
    assert "permissionOrigins(value, videos.checked, xIngress.checked)" in options_script
    assert "needsXMediaPermission" in popup_script
    assert "browser.permissions.request({origins:[X_MEDIA_PERMISSION]})" in popup_script
    assert 'id="video-downloads"' in popup
    assert "mediaCaptureStates" in popup_script
    assert "Available in DASObjectStore" in background
    assert 'command: "media-capture-state"' in background
    content = (EXTENSION / "content-explicit-open.js").read_text()
    assert "border: 2px solid #238636" in content
    assert "framingTargets(media)" in content
    assert "sameFootprint" in content
    assert 'zIndex: "2147483647"' in content
    assert 'pointerEvents: "none"' in content
    assert 'dataset.pinakothekeStoredOverlay = "true"' in content
    assert "isXMediaUrl" in content
    assert 'url.searchParams.set("name", "orig")' in content
    assert "[...document.images].reverse().find" in content
    assert 'url.pathname.startsWith("/media/")' in content + background
    assert "mediaToken" in content
    assert 'lookupAlias(instanceUrl, instanceId || "", pairId' in background
    assert "browser.tabs.onUpdated.addListener" in background
    assert "browser.tabs.onActivated.addListener" in background
    assert "scheduleTabScan(tabId)" in background
    assert "recentPageActivation" not in content
    click_handler = popup_script[popup_script.index("run.onclick="):popup_script.index("toggle.onclick=")]
    assert "await needsXMediaPermission" not in click_handler
    assert click_handler.index("browser.permissions.request") < click_handler.index(".then(async granted")
    diagnostic_block = background[background.index("async function recordSiteDiagnostic"):]
    diagnostic_block = diagnostic_block[: diagnostic_block.index("async function recordSegmentedOriginFallback")]
    for forbidden_field in ("pageUrl", "mediaUrl", "canonicalAlias", "checksum", "cookie"):
        assert forbidden_field not in diagnostic_block
    assert "siteDiagnostics" in diagnostic_block
    assert "some(site => site.origin === origin)" in diagnostic_block
    assert options_script.index("sync-capture-observers") < options_script.index("permissions.remove")
    subprocess.run(["node", "scripts/firefox/check_identity_upgrade.mjs"], cwd=ROOT, check=True)
    subprocess.run(["node", "scripts/firefox/check_explicit_original.mjs"], cwd=ROOT, check=True)
    print("Firefox toolbar contract passed: popup, controls, labels, bounded redacted diagnostics")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
