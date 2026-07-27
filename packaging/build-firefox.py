#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Build the deterministic, platform-independent Firefox XPI."""

from __future__ import annotations

import argparse
import json
import pathlib
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "firefox-extension"
PINAKOTHEKE_MANIFEST = ROOT / "firefox-extension/manifest.json"
SOURCE_FILES = (
    "adapters.json",
    "background.js",
    "content-explicit-open.js",
    "icon-16.png",
    "icon-32.png",
    "icon-48.png",
    "icon-96.png",
    "manifest.json",
    "options.html",
    "options.js",
    "popup.html",
    "popup.js",
    "registry.js",
)


def expected_firefox_version(product_version: str) -> str:
    """Map product rc.N SemVer to Firefox's numeric four-component version."""
    stable, separator, prerelease = product_version.partition("-")
    parts = stable.split(".")
    if len(parts) != 3 or any(not part.isdigit() for part in parts):
        raise SystemExit("product version is not supported Semantic Versioning")
    if not separator:
        return stable
    label, dot, sequence = prerelease.partition(".")
    if label != "rc" or dot != "." or not sequence.isdigit() or sequence == "0":
        raise SystemExit("Firefox prerelease mapping supports only rc.N")
    return ".".join((*parts, sequence))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--dist", required=True, type=pathlib.Path)
    parser.add_argument("--product", choices=("x-img", "pinakotheke"), default="x-img")
    args = parser.parse_args()
    manifest_path = PINAKOTHEKE_MANIFEST if args.product == "pinakotheke" else SOURCE / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    if manifest.get("version") != expected_firefox_version(args.version):
        raise SystemExit("Firefox manifest and product compatibility versions differ")
    if manifest.get("version_name") != args.version:
        raise SystemExit("Firefox version_name and product version differ")
    destination = args.dist / "firefox"
    destination.mkdir(parents=True, exist_ok=True)
    output = destination / f"{args.product}-{args.version}.xpi"
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name in SOURCE_FILES:
            source = SOURCE / name
            if not source.is_file() or source.is_symlink():
                raise SystemExit(f"required Firefox source is unavailable: {name}")
            info = zipfile.ZipInfo(source.name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            content = json.dumps(manifest, separators=(",", ":")).encode() if source.name == "manifest.json" else source.read_bytes()
            archive.writestr(info, content, compresslevel=9)
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
