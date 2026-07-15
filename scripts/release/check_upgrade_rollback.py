#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Exercise package lifecycle and metadata rollback without local media bytes."""

from __future__ import annotations

import hashlib
import json
import pathlib
import platform
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
DEBIAN_IMAGE = "debian@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818"
FEDORA_IMAGE = "fedora@sha256:99e203b80b1c3d8f7e161ec10a68fd02b081ef83a3963553e513c82846b97814"


def run(*command: str) -> None:
    subprocess.run(command, cwd=ROOT, check=True)


def digest(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def docker_lifecycle(image: str, package: pathlib.Path, install: str, remove: str) -> None:
    with tempfile.TemporaryDirectory(prefix="x-img-release-state-") as temporary:
        state = pathlib.Path(temporary)
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
            )
            + "\n"
        )
        before = digest(sentinel)
        script = (
            "set -eu; "
            f"{install} /packages/{package.name}; "
            "test \"$(x-img --version)\" = \"x-img 0.2.0\"; "
            "grep -q '\"product_version\": \"0.2.0\"' /usr/share/x-img/monas/product-bootstrap.json; "
            f"{install} /packages/{package.name}; "
            f"{remove}; "
            "test -f /var/lib/x-img/metadata-snapshot.json"
        )
        run(
            "docker",
            "run",
            "--rm",
            "--network=none",
            "--tmpfs",
            "/tmp",
            "-v",
            f"{package.parent}:/packages:ro",
            "-v",
            f"{state}:/var/lib/x-img",
            image,
            "sh",
            "-c",
            script,
        )
        if digest(sentinel) != before:
            raise SystemExit(f"package lifecycle changed x-img metadata: {package.name}")


def main() -> int:
    version = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--format-version", "1", "--no-deps"], cwd=ROOT
        )
    )["packages"][0]["version"]
    run("python3", "packaging/check.py", "--dist", str(ROOT / "dist"), "--version", version)

    machine = platform.machine().lower()
    if machine in {"arm64", "aarch64"}:
        directory, deb_arch, rpm_arch = "arm64", "arm64", "aarch64"
    elif machine in {"x86_64", "amd64"}:
        directory, deb_arch, rpm_arch = "x86_64", "amd64", "x86_64"
    else:
        raise SystemExit(f"unsupported lifecycle-test architecture: {machine}")

    deb = ROOT / "dist/linux" / directory / f"x-img-{version}-linux-{deb_arch}.deb"
    rpm = ROOT / "dist/linux" / directory / f"x-img-{version}-linux-{rpm_arch}.rpm"
    docker_lifecycle(DEBIAN_IMAGE, deb, "dpkg -i", "dpkg -r x-img")
    docker_lifecycle(FEDORA_IMAGE, rpm, "rpm -Uvh --replacepkgs", "rpm -e x-img")

    run("cargo", "+1.97.0", "test", "-p", "x-img-core", "migration_backup")
    run(
        "scripts/contracts/check.sh",
        "--sibling-root",
        str(ROOT.parent),
        "--sibling",
        "monas",
        "--sibling",
        "DASObjectStore",
    )
    print("upgrade/rollback acceptance passed: packages, metadata, Monas, DASObjectStore")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
