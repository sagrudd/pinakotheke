#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Regression tests for RC-native version and DAS prerequisite wiring."""

from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class RcPackagingTest(unittest.TestCase):
    def rendered_preinstall(self, executable: Path) -> Path:
        source = (ROOT / "packaging/macos/preinstall").read_text(encoding="utf-8")
        source = source.replace("@DASOBJECTSTORE_MIN_VERSION@", "0.145.4")
        start = (
            "for executable in /usr/local/bin/dasobjectstore "
            "/opt/homebrew/bin/dasobjectstore /usr/bin/dasobjectstore; do"
        )
        source = source.replace(start, f"for executable in {executable}; do")
        path = executable.parent / "preinstall"
        path.write_text(source, encoding="utf-8")
        path.chmod(0o755)
        return path

    def fake_das(self, directory: Path, version: str) -> Path:
        executable = directory / "dasobjectstore"
        executable.write_text(
            f"#!/bin/sh\nprintf '%s\\n' 'dasobjectstore {version}'\n",
            encoding="utf-8",
        )
        executable.chmod(0o755)
        return executable

    def test_macos_preinstall_accepts_exact_or_newer_release(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            executable = self.fake_das(root, "0.145.4")
            script = self.rendered_preinstall(executable)
            subprocess.run([os.fspath(script)], check=True)
            executable = self.fake_das(root, "0.146.0")
            subprocess.run([os.fspath(script)], check=True)

    def test_macos_preinstall_rejects_older_or_unparseable_release(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            executable = self.fake_das(root, "0.145.3")
            script = self.rendered_preinstall(executable)
            result = subprocess.run(
                [os.fspath(script)], check=False, capture_output=True, text=True
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("0.145.4 or newer", result.stderr)
            executable.write_text(
                "#!/bin/sh\nprintf '%s\\n' unknown\n", encoding="utf-8"
            )
            result = subprocess.run(
                [os.fspath(script)], check=False, capture_output=True, text=True
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("Cannot determine", result.stderr)

    def test_package_sources_bind_manifest_versions_and_minimum(self) -> None:
        makefile = (ROOT / "Makefile").read_text(encoding="utf-8")
        dockerfile = (ROOT / "packaging/Dockerfile.linux").read_text(encoding="utf-8")
        builder = (ROOT / "packaging/build-macos-pkg.sh").read_text(encoding="utf-8")
        rpm = (ROOT / "packaging/x-img.spec").read_text(encoding="utf-8")
        self.assertIn("--print-das-minimum", makefile)
        self.assertIn("--print-release-field deb_version", makefile)
        self.assertIn("Depends: dasobjectstore (>= %s)", dockerfile)
        self.assertIn("Requires: dasobjectstore >= %{dasobjectstore_min_version}", rpm)
        self.assertIn('--version "$package_version"', builder)
        self.assertIn("s/@DASOBJECTSTORE_MIN_VERSION@/$das_minimum/g", builder)


if __name__ == "__main__":
    unittest.main()
