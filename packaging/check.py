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
RELEASE_MANIFEST = "release-manifest.v1.json"


def digest(path: pathlib.Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def check_sources(version: str, product: str) -> None:
    # The v1 identity cutover is complete: both the canonical package and the
    # compatibility-labelled build consume the shipped manifest version.
    manifest_path = ROOT / "firefox-extension/manifest.json"
    manifest = json.loads(manifest_path.read_text())
    assert manifest["version"] == version
    bootstrap_path = (ROOT / "contracts/monas/x-img-product-bootstrap.v1.json" if product == "x-img" else
                      ROOT / "contracts/monas/x-img-product-bootstrap.v1.json")
    bootstrap = json.loads(bootstrap_path.read_text())
    assert bootstrap["product_version"] == version
    assert (ROOT / "LICENSE").is_file()
    makefile = (ROOT / "Makefile").read_text()
    for target in ["linux-x86_64", "linux-arm64", "macos-pkg-x86_64", "macos-pkg-arm64", "firefox"]:
        assert f"{target}:" in makefile
    assert '--build-context web-assets="$(DIST)/web"' in makefile
    assert "linux-x86_64: web" in makefile
    assert "macos-pkg-x86_64: web" in makefile
    dockerfile = (ROOT / "packaging/Dockerfile.linux").read_text()
    assert "COPY --from=web-assets" in dockerfile
    assert "PINAKOTHEKE_DEFAULT_WEB_ROOT" in dockerfile
    macos_builder = (ROOT / "packaging/build-macos-pkg.sh").read_text()
    assert "PINAKOTHEKE_DEFAULT_WEB_ROOT" in macos_builder
    assert 'cp -a "$web/."' in macos_builder


def artifact_record(path: pathlib.Path, dist: pathlib.Path) -> dict[str, object]:
    relative = path.relative_to(dist).as_posix()
    parts = relative.split("/")
    if parts[0] == "firefox":
        kind, os_name, arch = "firefox-xpi", parts[1], parts[2]
    elif parts[0] == "linux":
        kind = "linux-deb" if path.suffix == ".deb" else "linux-rpm"
        os_name, arch = "linux", parts[1]
    elif parts[0] == "macos":
        kind, os_name, arch = "macos-pkg", "macos", parts[1]
    else:
        kind, os_name, arch = "cyclonedx-sbom", "all", "all"
    return {
        "path": relative,
        "kind": kind,
        "os": os_name,
        "architecture": arch,
        "bytes": path.stat().st_size,
        "sha256": digest(path),
        "signed": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist", type=pathlib.Path, default=ROOT / "dist")
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-only", action="store_true")
    parser.add_argument("--write-checksums", action="store_true")
    parser.add_argument("--product", choices=("x-img", "pinakotheke"), default="x-img")
    args = parser.parse_args()
    check_sources(args.version, args.product)
    if args.source_only:
        print("packaging sources passed")
        return 0
    expected = [
        args.dist / f"{args.product}-{args.version}.cdx.json",
        args.dist / "linux/x86_64" / f"{args.product}-{args.version}-linux-amd64.deb",
        args.dist / "linux/x86_64" / f"{args.product}-{args.version}-linux-x86_64.rpm",
        args.dist / "linux/arm64" / f"{args.product}-{args.version}-linux-arm64.deb",
        args.dist / "linux/arm64" / f"{args.product}-{args.version}-linux-aarch64.rpm",
        args.dist / "macos/x86_64" / f"{args.product}-{args.version}-macos-x86_64.pkg",
        args.dist / "macos/arm64" / f"{args.product}-{args.version}-macos-arm64.pkg",
    ]
    expected.extend(
        args.dist / "firefox" / os_name / arch / f"{args.product}-{args.version}-firefox-{os_name}-{arch}.xpi"
        for os_name, arch in HOSTS
    )
    missing = [path.relative_to(args.dist) for path in expected if not path.is_file()]
    if missing:
        raise SystemExit("missing required package artifacts: " + ", ".join(map(str, missing)))
    ignored = {"SHA256SUMS", RELEASE_MANIFEST}
    artifacts = sorted(path for path in args.dist.rglob("*") if path.is_file() and path.name not in ignored)
    if not artifacts:
        raise SystemExit("no package artifacts found; build a package target first")
    for os_name, arch in HOSTS:
        xpi = args.dist / "firefox" / os_name / arch / f"{args.product}-{args.version}-firefox-{os_name}-{arch}.xpi"
        if xpi.exists():
            with zipfile.ZipFile(xpi) as archive:
                assert "manifest.json" in archive.namelist()
                assert json.loads(archive.read("manifest.json"))["version"] == args.version
    sbom = json.loads((args.dist / f"{args.product}-{args.version}.cdx.json").read_text())
    assert sbom["bomFormat"] == "CycloneDX"
    assert sbom["specVersion"] == "1.6"
    assert sbom["metadata"]["component"]["version"] == args.version
    checksums = "".join(f"{digest(path)}  {path.relative_to(args.dist)}\n" for path in artifacts)
    checksum_path = args.dist / "SHA256SUMS"
    release_manifest = {
        "schema_version": "x-img.release-artifacts.v1",
        "product": args.product,
        "version": args.version,
        "artifacts": [artifact_record(path, args.dist) for path in artifacts],
    }
    release_manifest_path = args.dist / RELEASE_MANIFEST
    release_manifest_text = json.dumps(release_manifest, indent=2, sort_keys=True) + "\n"
    if args.write_checksums:
        checksum_path.write_text(checksums)
        release_manifest_path.write_text(release_manifest_text)
    else:
        if not checksum_path.is_file() or checksum_path.read_text() != checksums:
            raise SystemExit("SHA256SUMS is missing or stale; run make checksums")
        if not release_manifest_path.is_file() or release_manifest_path.read_text() != release_manifest_text:
            raise SystemExit(f"{RELEASE_MANIFEST} is missing or stale; run make checksums")
    print(f"package verification passed: {len(artifacts)} artifact(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
