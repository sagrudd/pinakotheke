#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Build a deterministic Firefox XPI with an explicit host/architecture label."""

from __future__ import annotations

import argparse
import json
import pathlib
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "firefox-extension"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--os", required=True, choices=["macos", "windows", "linux"])
    parser.add_argument("--arch", required=True, choices=["x86_64", "arm64"])
    parser.add_argument("--version", required=True)
    parser.add_argument("--dist", required=True, type=pathlib.Path)
    args = parser.parse_args()
    manifest = json.loads((SOURCE / "manifest.json").read_text())
    if manifest.get("version") != args.version:
        raise SystemExit("Firefox manifest and workspace versions differ")
    destination = args.dist / "firefox" / args.os / args.arch
    destination.mkdir(parents=True, exist_ok=True)
    output = destination / f"x-img-{args.version}-firefox-{args.os}-{args.arch}.xpi"
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for source in sorted(path for path in SOURCE.iterdir() if path.is_file()):
            info = zipfile.ZipInfo(source.name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, source.read_bytes(), compresslevel=9)
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
