#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Verify the identity, version, and Mozilla signature envelope of an XPI."""

from __future__ import annotations

import argparse
import json
import pathlib
import zipfile


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", required=True, type=pathlib.Path)
    parser.add_argument("--extension-id", required=True)
    parser.add_argument("--version", required=True)
    args = parser.parse_args()

    candidates = sorted(args.directory.glob("*.xpi"), key=lambda path: path.stat().st_mtime)
    if not candidates:
        raise SystemExit("Mozilla did not produce a signed XPI")
    xpi = candidates[-1]
    with zipfile.ZipFile(xpi) as archive:
        names = {name.lower() for name in archive.namelist()}
        required = {"meta-inf/manifest.mf", "meta-inf/mozilla.sf", "meta-inf/mozilla.rsa"}
        missing = required - names
        if missing:
            raise SystemExit(f"XPI lacks Mozilla signature entries: {sorted(missing)}")
        manifest = json.loads(archive.read("manifest.json"))
    gecko = manifest.get("browser_specific_settings", {}).get("gecko", {})
    if gecko.get("id") != args.extension_id:
        raise SystemExit("signed XPI extension identity differs from the release identity")
    if manifest.get("version") != args.version:
        raise SystemExit("signed XPI version differs from the workspace version")
    print(f"verified Mozilla signature envelope, identity, and version: {xpi}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
