#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Apply the coordinated Pinakotheke identity transition to a repository root."""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path


CRATES = ("api", "cli", "core", "model", "web")


def replace(path: Path, old: str, new: str) -> None:
    source = path.read_text(encoding="utf-8")
    if old not in source:
        raise SystemExit(f"cutover prerequisite missing in {path}: {old!r}")
    path.write_text(source.replace(old, new), encoding="utf-8")


def activate_candidate(candidate: Path, destination: Path, changes: dict[str, object]) -> None:
    document = json.loads(candidate.read_text(encoding="utf-8"))
    document.update(changes)
    destination.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")


def apply(root: Path) -> None:
    cargo = root / "Cargo.toml"
    replace(cargo, 'version = "0.9.0"', 'version = "1.0.0"')
    replace(cargo, "https://github.com/sagrudd/x-img", "https://github.com/sagrudd/pinakotheke")
    replace(cargo, 'authors = ["x-img maintainers"]', 'authors = ["Pinakotheke maintainers"]')
    for crate in CRATES:
        replace(cargo, f'"crates/x-img-{crate}"', f'"crates/pinakotheke-{crate}"')

    for crate in CRATES:
        old = root / f"crates/x-img-{crate}"
        new = root / f"crates/pinakotheke-{crate}"
        if not old.is_dir() or new.exists():
            raise SystemExit(f"crate move prerequisite failed: {old} -> {new}")
        old.rename(new)
        manifest = new / "Cargo.toml"
        replace(manifest, f'name = "x-img-{crate}"', f'name = "pinakotheke-{crate}"')

    cli_manifest = root / "crates/pinakotheke-cli/Cargo.toml"
    replace(
        cli_manifest,
        "[lints]\nworkspace = true\n",
        "[lints]\nworkspace = true\n\n[lib]\n# Preserve the internal crate name used by both compatibility binaries.\nname = \"x_img_cli\"\n",
    )

    dependency_owners = {"api": ("core",), "cli": ("core",), "core": ("model",)}
    for owner, dependencies in dependency_owners.items():
        manifest = root / f"crates/pinakotheke-{owner}/Cargo.toml"
        for dependency in dependencies:
            replace(
                manifest,
                f'x-img-{dependency} = {{ path = "../x-img-{dependency}" }}',
                f'x-img-{dependency} = {{ package = "pinakotheke-{dependency}", path = "../pinakotheke-{dependency}" }}',
            )

    activate_candidate(
        root / "contracts/monas/pinakotheke-product-bootstrap.v1.candidate.json",
        root / "contracts/monas/x-img-product-bootstrap.v1.json",
        {"visibility": "registered"},
    )
    activate_candidate(
        root / "contracts/dasobjectstore/pinakotheke-application-identity.v1.candidate.json",
        root / "contracts/dasobjectstore/x-img-application-identity.v1.json",
        {"active": True},
    )
    shutil.copyfile(
        root / "packaging/firefox/pinakotheke-manifest.v1.candidate.json",
        root / "firefox-extension/manifest.json",
    )

    core = root / "crates/pinakotheke-core/src"
    replace(core / "application_identity.rs", 'Self::parse_for(bytes, "x-img", true)', 'Self::parse_for(bytes, "pinakotheke", true)')
    replace(core / "application_identity.rs", 'assert_eq!(legacy.prefix, "x-img/");', 'assert_eq!(legacy.prefix, "pinakotheke/");')
    for old, new in (
        ('product_id: "x-img"', 'product_id: "pinakotheke"'),
        ('product_root: "/opt/x-img"', 'product_root: "/opt/pinakotheke"'),
        ('web_mount: "/products/x-img/app/"', 'web_mount: "/products/pinakotheke/app/"'),
        ('api_mount: "/products/x-img/api/"', 'api_mount: "/products/pinakotheke/api/"'),
        ('bootstrap_path: "/products/x-img/.well-known/mnemosyne/product-bootstrap.json"',
         'bootstrap_path: "/products/pinakotheke/.well-known/mnemosyne/product-bootstrap.json"'),
        ('visibility: "local_profile_enabled"', 'visibility: "registered"'),
    ):
        replace(core / "host_product.rs", old, new)
    replace(core / "lib.rs", '"x-img 0.9.0"', '"Pinakotheke 1.0.0"')
    replace(root / "crates/pinakotheke-model/src/lib.rs", 'REPOSITORY_NAME: &str = "x-img"', 'REPOSITORY_NAME: &str = "Pinakotheke"')

    for fixture in (root / "fixtures/monas/v1").glob("*.json"):
        source = fixture.read_text(encoding="utf-8")
        source = source.replace('"product_id": "x-img"', '"product_id": "pinakotheke"')
        source = source.replace('"product_version": "0.9.0"', '"product_version": "1.0.0"')
        source = source.replace('/opt/x-img', '/opt/pinakotheke').replace('/products/x-img/', '/products/pinakotheke/')
        source = source.replace('"visibility": "local_profile_enabled"', '"visibility": "registered"')
        fixture.write_text(source, encoding="utf-8")
    for fixture in (root / "fixtures/das-application/v1").glob("*.json"):
        source = fixture.read_text(encoding="utf-8")
        source = source.replace('"application_id": "x-img"', '"application_id": "pinakotheke"')
        source = source.replace('monas.host-context:x-img', 'monas.host-context:pinakotheke')
        source = source.replace('dasobjectstore.application:x-img', 'dasobjectstore.application:pinakotheke')
        source = source.replace('"prefix": "x-img/', '"prefix": "pinakotheke/')
        source = source.replace('"object_key": "x-img/', '"object_key": "pinakotheke/')
        fixture.write_text(source, encoding="utf-8")

    replace(root / "README.md", "# x-img", "# Pinakotheke",)
    replace(
        root / "README.md",
        "github.com/sagrudd/x-img](https://github.com/sagrudd/x-img)",
        "github.com/sagrudd/pinakotheke](https://github.com/sagrudd/pinakotheke)",
    )
    replace(root / "docs/index.rst", "x-img documentation\n====================", "Pinakotheke documentation\n==========================")
    replace(root / "docs/conf.py", 'project = "x-img"', 'project = "Pinakotheke"')
    replace(root / "docs/conf.py", 'release = "0.9.0"', 'release = "1.0.0"')
    replace(root / "docs/conf.py", 'html_title = "x-img documentation"', 'html_title = "Pinakotheke documentation"')


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    if not args.apply:
        raise SystemExit("refusing mutation without --apply")
    apply(root)
    print(f"prepared coordinated Pinakotheke cutover in {root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
