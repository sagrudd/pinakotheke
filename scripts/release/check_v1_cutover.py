#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Refuse a Pinakotheke v1 cutover until every coordinated identity is ready."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MATRIX = ROOT / "docs/fixtures/pinakotheke-identity-migration-matrix.json"


@dataclass(frozen=True)
class Check:
    surface: str
    ready: bool
    detail: str


def load_json(path: Path) -> dict[str, object]:
    return json.loads(path.read_text(encoding="utf-8"))


def contains(path: str, value: str) -> bool:
    return value in (ROOT / path).read_text(encoding="utf-8")


def current_checks(*, github: bool) -> list[Check]:
    cargo = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    cli_manifest = (ROOT / "crates/x-img-cli/Cargo.toml").read_text(encoding="utf-8")
    cli_legacy = (ROOT / "crates/x-img-cli/src/main.rs").read_text(encoding="utf-8")
    monas = load_json(ROOT / "contracts/monas/x-img-product-bootstrap.v1.json")
    das = load_json(ROOT / "contracts/dasobjectstore/x-img-application-identity.v1.json")
    extension = load_json(ROOT / "firefox-extension/manifest.json")
    extension_candidate = load_json(ROOT / "packaging/firefox/pinakotheke-manifest.v1.candidate.json")
    monas_candidate = load_json(ROOT / "contracts/monas/pinakotheke-product-bootstrap.v1.candidate.json")
    das_candidate = load_json(
        ROOT / "contracts/dasobjectstore/pinakotheke-application-identity.v1.candidate.json"
    )

    checks = [
        Check("version", 'version = "1.0.0"' in cargo, "workspace version is exactly 1.0.0"),
        Check("repository", 'repository = "https://github.com/sagrudd/pinakotheke"' in cargo,
              "Cargo repository is canonical"),
        Check("rust-workspace", '"crates/pinakotheke-' in cargo and '"crates/x-img-' not in cargo,
              "workspace packages use canonical names"),
        Check("cli", 'name = "pinakotheke"' in cli_manifest and
              'name = "x-img"' in cli_manifest and "Invocation::Legacy" in cli_legacy,
              "canonical CLI and documented legacy wrapper are present"),
        Check("monas-product", monas.get("product_id") == "pinakotheke",
              "canonical Monas product registration is active"),
        Check("das-application", das.get("application_id") == "pinakotheke",
              "canonical scoped DASObjectStore application is active"),
        Check("firefox-extension", extension.get("name") == "Pinakotheke" and
              "browser_specific_settings" in extension,
              "canonical listing retains an explicit Gecko identity"),
        Check("documentation", contains("docs/index.rst", "Pinakotheke documentation") and
              contains("README.md", "github.com/sagrudd/pinakotheke"),
              "public documentation leads with the canonical identity"),
        Check("legacy-schemas", contains("crates/x-img-model/src/lib.rs", "x-img.instance.v1"),
              "legacy schema readers remain present"),
        Check("migration-proof", contains("docs/fixtures/pinakotheke-identity-migration-matrix.json",
                                          '"no_partial_release": true'),
              "no-partial migration proof remains enabled"),
        Check("authority-candidates", monas_candidate.get("product_id") == "pinakotheke" and
              monas_candidate.get("visibility") == "cutover_candidate" and
              das_candidate.get("application_id") == "pinakotheke" and
              das_candidate.get("active") is False,
              "inert canonical Monas and DASObjectStore candidates are validated"),
        Check("firefox-candidate", extension_candidate.get("name") == "Pinakotheke" and
              extension_candidate.get("version") == "1.0.0" and
              extension_candidate.get("browser_specific_settings", {}).get("gecko", {}).get("id") ==
              extension.get("browser_specific_settings", {}).get("gecko", {}).get("id"),
              "canonical Firefox candidate retains the published Gecko identity"),
    ]
    if github:
        checks.append(github_check())
    return checks


def github_check() -> Check:
    command = ["gh", "repo", "view", "sagrudd/pinakotheke", "--json", "nameWithOwner", "--jq", ".nameWithOwner"]
    try:
        result = subprocess.run(command, cwd=ROOT, check=False, capture_output=True, text=True, timeout=20)
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return Check("github-repository", False, "canonical GitHub repository could not be verified")
    return Check("github-repository", result.returncode == 0 and result.stdout.strip() == "sagrudd/pinakotheke",
                 "canonical GitHub repository exists and is readable")


def validate_matrix() -> list[str]:
    document = load_json(MATRIX)
    errors: list[str] = []
    expected = {"documentation", "github-repository", "rust-workspace-package", "cli",
                "monas-product", "synoptikon-adapter", "das-application",
                "catalogue-config-schema", "objectstore-references", "firefox-extension",
                "ci-release-artifacts"}
    actual = {entry["id"] for entry in document.get("surfaces", []) if isinstance(entry, dict) and "id" in entry}
    if expected != actual:
        errors.append("migration surface inventory differs from the required exact set")
    if document.get("planned_release") != "1.0.0" or document.get("no_partial_release") is not True:
        errors.append("matrix must target 1.0.0 and prohibit partial release")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--phase", choices=("preflight", "cutover"), default="preflight")
    parser.add_argument("--github", action="store_true", help="also verify the canonical GitHub repository")
    parser.add_argument("--json", action="store_true", help="emit a machine-readable report")
    args = parser.parse_args()

    matrix_errors = validate_matrix()
    checks = current_checks(github=args.github)
    blockers = [check for check in checks if not check.ready]
    report = {
        "schema_version": "x-img.pinakotheke-cutover-report.v1",
        "phase": args.phase,
        "ready": not matrix_errors and (args.phase == "preflight" or not blockers),
        "matrix_errors": matrix_errors,
        "checks": [check.__dict__ for check in checks],
        "blockers": [check.surface for check in blockers],
    }
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        for check in checks:
            print(f"{'READY' if check.ready else 'BLOCKED'} {check.surface}: {check.detail}")
        for error in matrix_errors:
            print(f"BLOCKED matrix: {error}", file=sys.stderr)

    if matrix_errors:
        return 1
    if args.phase == "preflight":
        print(f"preflight inventory passed; {len(blockers)} coordinated cutover blocker(s) remain")
        return 0
    if blockers:
        print("v1 cutover refused: " + ", ".join(check.surface for check in blockers), file=sys.stderr)
        return 1
    print("v1 cutover identity gate passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
