#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Rehearse the local Pinakotheke cutover in an isolated repository copy."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def run(
    *command: str,
    cwd: Path,
    env: dict[str, str] | None = None,
    quiet: bool = False,
) -> None:
    subprocess.run(
        command,
        cwd=cwd,
        env=env,
        check=True,
        stdout=subprocess.DEVNULL if quiet else None,
    )


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="pinakotheke-v1-rehearsal-") as temporary:
        candidate = Path(temporary) / "pinakotheke"
        shutil.copytree(
            ROOT,
            candidate,
            ignore=shutil.ignore_patterns(".git", ".codex", "dist", "target", "__pycache__", "_build"),
        )
        run(
            "python3", "scripts/release/prepare_v1_cutover.py",
            "--root", str(candidate), "--apply", cwd=candidate,
        )
        run("cargo", "+1.97.0", "generate-lockfile", "--offline", cwd=candidate)
        run(
            "cargo", "+1.97.0", "metadata", "--format-version", "1",
            "--no-deps", "--locked", cwd=candidate, quiet=True,
        )
        run("cargo", "+1.97.0", "test", "--workspace", "--locked", cwd=candidate,
            env={**os.environ, "CARGO_TARGET_DIR": str(ROOT / "target/v1-rehearsal")})
        run("python3", "scripts/release/check_v1_cutover.py", "--phase", "cutover", cwd=candidate)
        run("python3", "packaging/check.py", "--source-only", "--product", "pinakotheke",
            "--version", "1.0.0", cwd=candidate)
    print("Pinakotheke v1 local cutover rehearsal passed; live 0.9 tree was not mutated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
