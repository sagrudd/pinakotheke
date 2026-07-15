#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Validate the inert Pinakotheke 1.0 package plan without changing v0.9 artifacts."""

from __future__ import annotations

import json
import subprocess
import tempfile
import zipfile
from pathlib import Path

from check import check_sources


ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    check_sources("1.0.0", "pinakotheke")
    makefile = (ROOT / "Makefile").read_text()
    dockerfile = (ROOT / "packaging/Dockerfile.linux").read_text()
    macos = (ROOT / "packaging/build-macos-pkg.sh").read_text()
    rpm = (ROOT / "packaging/x-img.spec").read_text()
    assert "PRODUCT ?= x-img" in makefile
    assert "--build-arg PRODUCT_NAME=$(PRODUCT)" in makefile
    for source in (dockerfile, macos, rpm):
        assert "pinakotheke" in source
        assert "x-img" in source
    assert "Conflicts: x-img" in dockerfile and "Replaces: x-img" in dockerfile
    assert "Provides: x-img" in rpm and "Obsoletes: x-img" in rpm
    assert "release/$binary" in dockerfile and "release/x-img" in dockerfile
    assert "release/pinakotheke" in macos and "release/x-img" in macos

    with tempfile.TemporaryDirectory(prefix="pinakotheke-firefox-plan-") as temporary:
        subprocess.run(
            ["python3", "packaging/build-firefox.py", "--product", "pinakotheke",
             "--os", "linux", "--arch", "x86_64", "--version", "1.0.0",
             "--dist", temporary],
            cwd=ROOT, check=True, capture_output=True, text=True,
        )
        xpi = Path(temporary) / "firefox/linux/x86_64/pinakotheke-1.0.0-firefox-linux-x86_64.xpi"
        with zipfile.ZipFile(xpi) as archive:
            manifest = json.loads(archive.read("manifest.json"))
        assert manifest["name"] == "Pinakotheke"
        assert manifest["version"] == "1.0.0"
        assert manifest["browser_specific_settings"]["gecko"]["id"] == "x-img@example.invalid"
    print("Pinakotheke v1 package plan passed: canonical artifacts with retained compatibility aliases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
