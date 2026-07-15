#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Validate packaging sources and, when present, built artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
HOSTS = [(os_name, arch) for os_name in ("macos", "windows", "linux") for arch in ("x86_64", "arm64")]


def digest(path: pathlib.Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def check_sources(version: str) -> None:
    manifest = json.loads((ROOT / "firefox-extension/manifest.json").read_text())
    assert manifest["version"] == version
    bootstrap = json.loads((ROOT / "contracts/monas/x-img-product-bootstrap.v1.json").read_text())
    assert bootstrap["product_version"] == version
    assert (ROOT / "LICENSE").is_file()
    makefile = (ROOT / "Makefile").read_text()
    for target in ["linux-x86_64", "linux-arm64", "macos-pkg-x86_64", "macos-pkg-arm64", "firefox"]:
        assert f"{target}:" in makefile


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist", type=pathlib.Path, default=ROOT / "dist")
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-only", action="store_true")
    parser.add_argument("--write-checksums", action="store_true")
    args = parser.parse_args()
    check_sources(args.version)
    if args.source_only:
        print("packaging sources passed")
        return 0
    expected = [
        args.dist / "linux/x86_64" / f"x-img-{args.version}-linux-amd64.deb",
        args.dist / "linux/x86_64" / f"x-img-{args.version}-linux-x86_64.rpm",
        args.dist / "linux/arm64" / f"x-img-{args.version}-linux-arm64.deb",
        args.dist / "linux/arm64" / f"x-img-{args.version}-linux-aarch64.rpm",
        args.dist / "macos/x86_64" / f"x-img-{args.version}-macos-x86_64.pkg",
        args.dist / "macos/arm64" / f"x-img-{args.version}-macos-arm64.pkg",
    ]
    expected.extend(
        args.dist / "firefox" / os_name / arch / f"x-img-{args.version}-firefox-{os_name}-{arch}.xpi"
        for os_name, arch in HOSTS
    )
    missing = [path.relative_to(args.dist) for path in expected if not path.is_file()]
    if missing:
        raise SystemExit("missing required package artifacts: " + ", ".join(map(str, missing)))
    artifacts = sorted(path for path in args.dist.rglob("*") if path.is_file() and path.name != "SHA256SUMS")
    if not artifacts:
        raise SystemExit("no package artifacts found; build a package target first")
    for os_name, arch in HOSTS:
        xpi = args.dist / "firefox" / os_name / arch / f"x-img-{args.version}-firefox-{os_name}-{arch}.xpi"
        if xpi.exists():
            with zipfile.ZipFile(xpi) as archive:
                assert "manifest.json" in archive.namelist()
                assert json.loads(archive.read("manifest.json"))["version"] == args.version
    checksums = "".join(f"{digest(path)}  {path.relative_to(args.dist)}\n" for path in artifacts)
    checksum_path = args.dist / "SHA256SUMS"
    if args.write_checksums:
        checksum_path.write_text(checksums)
    elif not checksum_path.is_file() or checksum_path.read_text() != checksums:
        raise SystemExit("SHA256SUMS is missing or stale; run make checksums")
    print(f"package verification passed: {len(artifacts)} artifact(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
