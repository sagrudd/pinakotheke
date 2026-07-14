#!/usr/bin/env python3
"""Validate x-img-owned contracts, optionally against pinned sibling sources."""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PINS = {
    "monas": "3d21b0bc7b83fa8408d01b93347a56f43f3a96b7",
    "DASObjectStore": "8368d34a365689e19321ecd6a35aab7c819268f6",
    "mnemosyne": "9877017e3139711ed6313c53603409c53020541d",
    "mnemosyne_design_language": "5539df8f662a78ebdf7cf4c868d71831380c8cfd",
}
VENDORED_REQUIRED = (
    "contracts/monas/x-img-product-bootstrap.v1.json",
    "contracts/dasobjectstore/x-img-application-identity.v1.json",
    "fixtures/monas/v1/invalid-anonymous-api.json",
    "fixtures/host-context/v1/monas-valid.json",
    "fixtures/host-context/v1/synoptikon-valid.json",
    "fixtures/das-application/v1/authorization-cases.json",
)
SIBLING_REQUIRED = {
    "monas": ("README.md", "crates/monas-core/src/lib.rs", "crates/monas-server/src/lib.rs"),
    "DASObjectStore": (
        "docs/application-authentication.md",
        "crates/dasobjectstore-core/src/application_auth.rs",
        "crates/dasobjectstore-daemon/src/api/provider_stream.rs",
    ),
    "mnemosyne": ("mneion-api-types/schemas/HOST_PRODUCT_UI_ADAPTER.md",),
    "mnemosyne_design_language": ("docs/brief.md", "docs/interface-patterns.md"),
}


def fail(message: str) -> None:
    print(f"contract check failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def git_head(path: Path) -> str:
    return subprocess.check_output(
        ["git", "-C", str(path), "rev-parse", "HEAD"], text=True
    ).strip()


def check_vendored() -> None:
    for relative in VENDORED_REQUIRED:
        if not (ROOT / relative).is_file():
            fail(f"missing x-img-owned contract fixture: {relative}")
    manifests = list(ROOT.rglob("Cargo.toml"))
    forbidden = re.compile(r'path\s*=\s*"\.\./(?:monas|DASObjectStore|mnemosyne|mnemosyne_design_language)"')
    for manifest in manifests:
        if any(part in {".git", ".codex", "target"} for part in manifest.parts):
            continue
        if forbidden.search(manifest.read_text(encoding="utf-8")):
            fail(f"public build has forbidden sibling path dependency: {manifest.relative_to(ROOT)}")
    matrix = (ROOT / "docs/compatibility-matrix.md").read_text(encoding="utf-8")
    for sibling, pin in PINS.items():
        if pin not in matrix:
            fail(f"compatibility matrix does not pin {sibling} at {pin}")
    print("x-img-owned contract fixtures and public-build independence: verified")


def check_siblings(root: Path, require: bool) -> None:
    paths = {name: root / name for name in PINS}
    present = {name: path.is_dir() for name, path in paths.items()}
    if not any(present.values()):
        if require:
            fail(f"required sibling root has no checked-out siblings: {root}")
        print("sibling contract inspection: skipped (no sibling checkouts available)")
        return
    missing = [name for name, is_present in present.items() if not is_present]
    if missing:
        fail(f"sibling root is partial; missing {', '.join(missing)}")
    for name, path in paths.items():
        try:
            actual = git_head(path)
        except (OSError, subprocess.CalledProcessError) as error:
            fail(f"cannot read {name} git revision: {error}")
        if actual != PINS[name]:
            fail(f"{name} revision is {actual}, expected pinned {PINS[name]}")
        for relative in SIBLING_REQUIRED[name]:
            if not (path / relative).is_file():
                fail(f"{name} lacks pinned contract path {relative}")
    print("pinned sibling contract paths and revisions: verified")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--sibling-root",
        type=Path,
        default=Path(os.environ.get("XIMG_SIBLING_ROOT", ROOT.parent)),
        help="directory containing monas, DASObjectStore, mnemosyne, and mnemosyne_design_language",
    )
    parser.add_argument(
        "--require-siblings",
        action="store_true",
        help="fail rather than skip when sibling checkouts are absent",
    )
    args = parser.parse_args()
    check_vendored()
    check_siblings(args.sibling_root, args.require_siblings)


if __name__ == "__main__":
    main()
