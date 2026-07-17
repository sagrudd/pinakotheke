#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Run deterministic privacy, security, accessibility, and license audits."""

from __future__ import annotations

import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
MATRIX = ROOT / "docs/fixtures/release-audit-matrix.json"
EXPECTED = {"privacy", "security", "accessibility", "licenses", "dependencies", "versions"}
PAYLOAD_SUFFIXES = {".jpg", ".jpeg", ".png", ".gif", ".webp", ".mp4", ".mov", ".mkv", ".webm", ".bam", ".cram", ".sra", ".fastq"}


def fail(message: str) -> None:
    raise AssertionError(message)


def tracked_files() -> list[pathlib.Path]:
    output = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=ROOT
    )
    return [ROOT / item.decode() for item in output.split(b"\0") if item]


def audit_matrix() -> None:
    document = json.loads(MATRIX.read_text())
    assert document.get("schema") == "x-img.release-audit-matrix"
    assert document.get("schema_major") == 1
    assert document.get("fixture_kind") == "synthetic"
    audits = document.get("audits")
    assert isinstance(audits, list)
    assert {item.get("id") for item in audits} == EXPECTED
    for item in audits:
        assert set(item) == {"id", "gate"} and isinstance(item["gate"], str) and item["gate"]


def privacy(files: list[pathlib.Path]) -> None:
    for path in files:
        if path.suffix.lower() in PAYLOAD_SUFFIXES:
            fail(f"tracked payload-like file is forbidden: {path.relative_to(ROOT)}")
        if path.suffix.lower() not in {".rs", ".py", ".sh", ".js", ".json", ".toml", ".md", ".rst", ".html", ".css", ".yml", ".yaml"}:
            continue
        text = path.read_text(errors="replace")
        patterns = [
            r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
            r"\bAKIA[0-9A-Z]{16}\b",
            r"\bgh[opusr]_[A-Za-z0-9]{20,}\b",
            r"\bBearer\s+[A-Za-z0-9._~+/=-]{20,}",
        ]
        if any(re.search(pattern, text) for pattern in patterns):
            fail(f"credential signature found: {path.relative_to(ROOT)}")


def security() -> None:
    manifest = json.loads((ROOT / "firefox-extension/manifest.json").read_text())
    allowed = {"storage", "activeTab", "scripting"}
    if set(manifest.get("permissions", [])) != allowed:
        fail("Firefox required permissions differ from the reviewed least-privilege set")
    if manifest.get("optional_host_permissions") != ["https://*/*"]:
        fail("Firefox origins must remain optional HTTPS permissions")
    collected = manifest.get("browser_specific_settings", {}).get("gecko", {}).get(
        "data_collection_permissions", {}
    ).get("required")
    if collected != ["browsingActivity", "websiteContent", "websiteActivity"]:
        fail("Firefox data transmission declaration differs from reviewed behavior")
    csp = manifest.get("content_security_policy", {}).get("extension_pages", "")
    if "script-src 'self'" not in csp or "object-src 'none'" not in csp:
        fail("Firefox extension CSP must restrict scripts and objects")
    source = "\n".join(path.read_text() for path in (ROOT / "firefox-extension").glob("*.js"))
    for pattern in [r"\beval\s*\(", r"\bnew\s+Function\b", r"\.innerHTML\s*=", r"document\.write\s*\("]:
        if re.search(pattern, source):
            fail(f"dynamic or unsafe DOM execution found: {pattern}")
    rust = "\n".join(path.read_text() for path in (ROOT / "crates").rglob("*.rs"))
    if re.search(r"\bunsafe\s*\{", rust):
        fail("unsafe Rust block found")


def accessibility() -> None:
    css = (ROOT / "crates/pinakotheke-web/assets/mnemosyne-shell.css").read_text()
    if ":focus-visible" not in css:
        fail("Yew shell has no visible keyboard focus rule")
    rust = (ROOT / "crates/pinakotheke-web/src/lib.rs").read_text()
    for required in ['aria-label=', 'aria-current=', 'aria-pressed=', 'role="dialog"']:
        if required not in rust:
            fail(f"Yew semantic requirement missing: {required}")
    for name in ["popup.html", "options.html"]:
        text = (ROOT / "firefox-extension" / name).read_text()
        for required in ['<html lang="en">', "<main>", "<h1", 'role="status"', 'aria-live="polite"']:
            if required not in text:
                fail(f"{name} accessibility requirement missing: {required}")
        for button in re.findall(r"<button\b[^>]*>", text):
            if 'type="button"' not in button:
                fail(f"{name} button lacks explicit type")


def licenses(files: list[pathlib.Path]) -> None:
    if "Mozilla Public License Version 2.0" not in (ROOT / "LICENSE").read_text():
        fail("LICENSE is not MPL-2.0 text")
    for path in files:
        if path.suffix.lower() not in {".rs", ".py", ".sh", ".js", ".css"}:
            continue
        first = "\n".join(path.read_text(errors="replace").splitlines()[:3])
        if "SPDX-License-Identifier: MPL-2.0" not in first:
            fail(f"source lacks MPL-2.0 SPDX header: {path.relative_to(ROOT)}")


def main() -> int:
    audit_matrix()
    files = tracked_files()
    privacy(files)
    security()
    accessibility()
    licenses(files)
    subprocess.run(["cargo", "deny", "check", "advisories", "licenses", "bans", "sources"], cwd=ROOT, check=True)
    subprocess.run([sys.executable, "scripts/quality/check.py", "versions"], cwd=ROOT, check=True)
    for script in sorted((ROOT / "firefox-extension").glob("*.js")):
        subprocess.run(["node", "--check", script], cwd=ROOT, check=True)
    print("release audits passed: accessibility, dependencies, licenses, privacy, security, versions")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
