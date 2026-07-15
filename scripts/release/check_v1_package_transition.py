#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Build and exercise the x-img 0.9 to Pinakotheke 1.0 package transition."""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DEBIAN_IMAGE = "debian@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818"
FEDORA_IMAGE = "fedora@sha256:99e203b80b1c3d8f7e161ec10a68fd02b081ef83a3963553e513c82846b97814"


def run(*command: str, cwd: Path = ROOT) -> None:
    subprocess.run(command, cwd=cwd, check=True)


def checksum(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def lifecycle(
    image: str,
    platform: str,
    baseline: Path,
    candidate: Path,
    package_kind: str,
) -> None:
    with tempfile.TemporaryDirectory(prefix="pinakotheke-package-state-") as temporary:
        state = Path(temporary)
        sentinel = state / "metadata-snapshot.json"
        sentinel.write_text(
            json.dumps(
                {
                    "schema_version": "x-img.release-lifecycle-fixture.v1",
                    "endpoint_id": "endpoint-synthetic-01",
                    "objectstore_id": "store-synthetic-01",
                    "object_checksum": "a" * 64,
                    "review_state": "New",
                },
                sort_keys=True,
            ) + "\n",
            encoding="utf-8",
        )
        before = checksum(sentinel)
        if package_kind == "deb":
            install_old = f"dpkg -i /baseline/{baseline.name}"
            remove_old = "dpkg -r x-img"
            install_new = f"dpkg -i /candidate/{candidate.name}"
            remove_new = "dpkg -r pinakotheke"
        else:
            install_old = f"rpm -Uvh /baseline/{baseline.name}"
            remove_old = "rpm -e x-img"
            install_new = f"rpm -Uvh /candidate/{candidate.name}"
            remove_new = "rpm -e pinakotheke"
        verify_old = (
            'test "$(x-img --version)" = "x-img 0.9.0"; '
            "test ! -e /usr/bin/pinakotheke; "
            "grep -q '\"product_id\": \"x-img\"' /usr/share/x-img/monas/product-bootstrap.json"
        )
        verify_new = (
            'test "$(pinakotheke --version)" = "pinakotheke 1.0.0"; '
            'test "$(x-img --version 2>/dev/null)" = "x-img 1.0.0"; '
            "grep -q '\"product_id\": \"pinakotheke\"' "
            "/usr/share/pinakotheke/monas/product-bootstrap.json"
        )
        script = "; ".join(
            (
                "set -eu",
                install_old,
                verify_old,
                remove_old,
                install_new,
                verify_new,
                remove_new,
                install_old,
                verify_old,
                "test -f /var/lib/x-img/metadata-snapshot.json",
            )
        )
        run(
            "docker", "run", "--rm", "--platform", platform, "--network=none",
            "--tmpfs", "/tmp", "-v", f"{baseline.parent}:/baseline:ro",
            "-v", f"{candidate.parent}:/candidate:ro",
            "-v", f"{state}:/var/lib/x-img", image, "sh", "-c", script,
        )
        if checksum(sentinel) != before:
            raise SystemExit(f"package transition changed metadata: {candidate.name}")


def main() -> int:
    matrix = (
        ("x86_64", "x86_64-unknown-linux-gnu", "amd64", "x86_64", "linux/amd64"),
        ("arm64", "aarch64-unknown-linux-gnu", "arm64", "aarch64", "linux/arm64"),
    )
    with tempfile.TemporaryDirectory(prefix="pinakotheke-v1-packages-") as temporary:
        root = Path(temporary)
        candidate_root = root / "source"
        candidate_dist = root / "dist"
        shutil.copytree(
            ROOT,
            candidate_root,
            ignore=shutil.ignore_patterns(".git", ".codex", "dist", "target", "__pycache__", "_build"),
        )
        run(
            "python3", "scripts/release/prepare_v1_cutover.py", "--root",
            str(candidate_root), "--apply", cwd=candidate_root,
        )
        run("cargo", "+1.97.0", "generate-lockfile", "--offline", cwd=candidate_root)
        for directory, rust_target, deb_arch, rpm_arch, platform in matrix:
            output = candidate_dist / "linux" / directory
            output.mkdir(parents=True)
            run(
                "docker", "buildx", "build", "--build-arg", "VERSION=1.0.0",
                "--build-arg", "PRODUCT_NAME=pinakotheke", "--build-arg",
                f"RUST_TARGET={rust_target}", "--build-arg", f"DEB_ARCH={deb_arch}",
                "--build-arg", f"RPM_ARCH={rpm_arch}", "-f", "packaging/Dockerfile.linux",
                "--output", f"type=local,dest={output}", ".", cwd=candidate_root,
            )
            baseline_deb = ROOT / "dist/linux" / directory / f"x-img-0.9.0-linux-{deb_arch}.deb"
            baseline_rpm = ROOT / "dist/linux" / directory / f"x-img-0.9.0-linux-{rpm_arch}.rpm"
            candidate_deb = output / f"pinakotheke-1.0.0-linux-{deb_arch}.deb"
            candidate_rpm = output / f"pinakotheke-1.0.0-linux-{rpm_arch}.rpm"
            for package in (baseline_deb, baseline_rpm, candidate_deb, candidate_rpm):
                if not package.is_file():
                    raise SystemExit(f"missing package transition input: {package}")
            lifecycle(DEBIAN_IMAGE, platform, baseline_deb, candidate_deb, "deb")
            lifecycle(FEDORA_IMAGE, platform, baseline_rpm, candidate_rpm, "rpm")
    print("Pinakotheke package transition passed on x86_64 and arm64; metadata remained exact")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
