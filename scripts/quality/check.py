#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Dependency-free planning quality checks for x-img.

These checks intentionally validate repository invariants, not full JSON Schema
semantics. Runtime schema validation remains a Rust implementation task.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any, Iterable
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[2]
EXCLUDED_PARTS = {".git", ".codex", "_build", "target", "node_modules"}
SUPPORTED_SCHEMA_MAJOR = 1
SEMVER = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$")
SCHEMA_VERSION = re.compile(r"^x-img\.[a-z0-9-]+\.v(\d+)$")
COPIED_WIRE_SCHEMA_VERSIONS = {"mnemosyne.product.manifest.v1": 1}


class Findings:
    def __init__(self) -> None:
        self.errors: list[str] = []

    def add(self, path: Path, message: str) -> None:
        try:
            display = path.relative_to(ROOT)
        except ValueError:
            display = path
        self.errors.append(f"{display}: {message}")


def files_with_suffixes(*suffixes: str) -> Iterable[Path]:
    for path in ROOT.rglob("*"):
        if path.is_file() and path.suffix.lower() in suffixes and not EXCLUDED_PARTS.intersection(path.parts):
            yield path


def local_target_exists(source: Path, raw_target: str) -> bool:
    target = raw_target.strip().strip("<>")
    if not target or target.startswith("#"):
        return True
    parsed = urlsplit(target)
    if parsed.scheme or parsed.netloc or target.startswith(("mailto:", "tel:")):
        return True
    relative = Path(unquote(parsed.path))
    if relative.is_absolute():
        return False
    candidate = (source.parent / relative).resolve()
    try:
        candidate.relative_to(ROOT.resolve())
    except ValueError:
        return False
    return candidate.exists()


def sphinx_target_exists(source: Path, raw_target: str) -> bool:
    target = raw_target.strip()
    if target.startswith("/"):
        base = ROOT / "docs" / target.lstrip("/")
    else:
        base = source.parent / target
    candidates = [base, base.with_suffix(".rst"), base.with_suffix(".md"), base / "index.rst"]
    return any(candidate.resolve().is_relative_to(ROOT.resolve()) and candidate.exists() for candidate in candidates)


def check_links(findings: Findings) -> None:
    markdown_link = re.compile(r"!?\[[^\]]*\]\(([^)\s]+)(?:\s+['\"][^'\"]*['\"])?\)")
    doc_role = re.compile(r":doc:`(?:[^`<>]+<)?([^`<>]+)>?`")
    external_rst = re.compile(r"`[^`]+\s+<([^>]+)>`_")

    for path in files_with_suffixes(".md", ".rst"):
        text = path.read_text(encoding="utf-8")
        for match in markdown_link.finditer(text):
            if not local_target_exists(path, match.group(1)):
                findings.add(path, f"broken or escaping local link: {match.group(1)}")
        for match in external_rst.finditer(text):
            if not local_target_exists(path, match.group(1)):
                findings.add(path, f"broken or escaping reStructuredText link: {match.group(1)}")
        for match in doc_role.finditer(text):
            if not sphinx_target_exists(path, match.group(1)):
                findings.add(path, f"broken Sphinx :doc: target: {match.group(1)}")

        if path.suffix == ".rst":
            lines = text.splitlines()
            in_toctree = False
            for line in lines:
                if line.strip() == ".. toctree::":
                    in_toctree = True
                    continue
                if not in_toctree:
                    continue
                if not line.strip() or line.lstrip().startswith(":"):
                    continue
                if not line.startswith((" ", "\t")):
                    in_toctree = False
                    continue
                target = line.strip().split("<")[-1].rstrip(">")
                if not sphinx_target_exists(path, target):
                    findings.add(path, f"broken toctree target: {target}")


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate key {key!r}")
        result[key] = value
    return result


def json_documents(findings: Findings) -> Iterable[tuple[Path, Any]]:
    for path in files_with_suffixes(".json"):
        try:
            yield path, json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=reject_duplicate_keys)
        except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
            findings.add(path, f"invalid strict JSON: {error}")


def walk_values(value: Any) -> Iterable[tuple[str | None, Any]]:
    if isinstance(value, dict):
        for key, child in value.items():
            yield key, child
            yield from walk_values(child)
    elif isinstance(value, list):
        for child in value:
            yield None, child
            yield from walk_values(child)


def check_json(findings: Findings) -> None:
    for path, document in json_documents(findings):
        if path.parent == ROOT / "schemas" and ".v" in path.name:
            match = re.search(r"\.v(\d+)\.schema\.json$", path.name)
            if not match:
                findings.add(path, "versioned schema filename does not expose a major")
                continue
            file_major = int(match.group(1))
            schema_id = document.get("$id", "") if isinstance(document, dict) else ""
            if not schema_id.endswith(path.name):
                findings.add(path, "$id must end with the schema filename")
            const = document.get("properties", {}).get("schema_version", {}).get("const")
            if const:
                version_match = SCHEMA_VERSION.fullmatch(const)
                if not version_match or int(version_match.group(1)) != file_major:
                    findings.add(path, "schema_version const and filename major disagree")

            if document.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
                findings.add(path, "schema must declare JSON Schema draft 2020-12")
            if document.get("type") != "object":
                findings.add(path, "top-level schema type must be object")

        for key, value in walk_values(document):
            if key != "$ref" or not isinstance(value, str):
                continue
            parsed = urlsplit(value)
            if parsed.scheme or parsed.netloc or not parsed.path:
                continue
            target = (path.parent / unquote(parsed.path)).resolve()
            try:
                target.relative_to(ROOT.resolve())
            except ValueError:
                findings.add(path, f"schema reference escapes repository: {value}")
                continue
            if not target.is_file():
                findings.add(path, f"missing local schema reference: {value}")

        for key, value in walk_values(document):
            if key == "schema_major":
                if not isinstance(value, int) or isinstance(value, bool) or value < 1:
                    findings.add(path, f"schema_major must be a positive integer, got {value!r}")
                elif value > SUPPORTED_SCHEMA_MAJOR and path.name != "invalid-future-major.json":
                    findings.add(path, f"unsupported schema_major {value} outside a rejection fixture")
                elif path.name == "invalid-future-major.json" and value <= SUPPORTED_SCHEMA_MAJOR:
                    findings.add(path, "future-major rejection fixture must use an unsupported schema_major")
            if key != "schema_version" or not isinstance(value, str):
                continue
            if value in COPIED_WIRE_SCHEMA_VERSIONS:
                continue
            match = SCHEMA_VERSION.fullmatch(value)
            if not match:
                findings.add(path, f"malformed schema_version: {value!r}")
                continue
            major = int(match.group(1))
            expected_future_fixture = path.name == "invalid-future-major.json"
            if expected_future_fixture and major <= SUPPORTED_SCHEMA_MAJOR:
                findings.add(path, "future-major rejection fixture must use an unsupported major")
            elif not expected_future_fixture and major > SUPPORTED_SCHEMA_MAJOR:
                findings.add(path, f"unsupported schema major v{major} outside a rejection fixture")


def check_privacy(findings: Findings) -> None:
    roots = [ROOT / "examples", ROOT / "docs" / "fixtures"]
    payload_extensions = {".jpg", ".jpeg", ".png", ".gif", ".webp", ".mp4", ".mov", ".mkv", ".webm", ".bam", ".cram", ".sra", ".fastq", ".gz"}
    secret_key = re.compile(r"^(password|passwd|cookie|set_cookie|access_token|refresh_token|session|client_secret|private_key)$", re.I)
    secret_value = re.compile(r"(?:-----BEGIN [A-Z ]*PRIVATE KEY-----|\bBearer\s+[A-Za-z0-9._~+/=-]{8,}|\bAKIA[0-9A-Z]{16}\b|\bgh[opusr]_[A-Za-z0-9]{20,}\b)")

    for root in roots:
        if not root.exists():
            continue
        for path in root.rglob("*"):
            if path.is_file() and path.suffix.lower() in payload_extensions:
                findings.add(path, "payload-like media or bioinformatics file is forbidden in fixtures")

    for path, document in json_documents(findings):
        if not any(root in path.parents for root in roots):
            continue
        for key, value in walk_values(document):
            if key and secret_key.fullmatch(key):
                findings.add(path, f"credential-bearing fixture key is forbidden: {key}")
            if isinstance(value, str) and secret_value.search(value):
                findings.add(path, "possible credential or private key in fixture data")
            if isinstance(value, str) and value.startswith(("http://", "https://")):
                parsed = urlsplit(value)
                if parsed.query and "redacted" not in value.lower():
                    findings.add(path, "fixture URL contains a non-redacted query string")


def read_milestone_version(findings: Findings) -> str | None:
    path = ROOT / "MILESTONES.md"
    match = re.search(r"(?m)^Version:\s*([^\s]+)\s*$", path.read_text(encoding="utf-8"))
    if not match or not SEMVER.fullmatch(match.group(1)):
        findings.add(path, "missing or invalid SemVer 'Version:' source")
        return None
    return match.group(1)


def check_versions(findings: Findings) -> None:
    expected = read_milestone_version(findings)
    if expected is None:
        return

    todo = ROOT / "TODO.md"
    todo_match = re.search(r"(?m)^Version:\s*([^\s]+)\s*$", todo.read_text(encoding="utf-8"))
    if not todo_match or todo_match.group(1) != expected:
        findings.add(todo, f"planning version must match canonical version {expected}")

    conf = ROOT / "docs" / "conf.py"
    if conf.exists():
        match = re.search(r"(?m)^release\s*=\s*['\"]([^'\"]+)['\"]", conf.read_text(encoding="utf-8"))
        if not match or match.group(1) != expected:
            findings.add(conf, f"Sphinx release must match canonical version {expected}")

    for manifest in (
        list(ROOT.glob("extension/**/manifest.json"))
        + list(ROOT.glob("firefox/**/manifest.json"))
        + list(ROOT.glob("firefox-extension/manifest.json"))
    ):
        try:
            value = json.loads(manifest.read_text(encoding="utf-8"), object_pairs_hook=reject_duplicate_keys).get("version")
        except (ValueError, json.JSONDecodeError):
            continue  # strict JSON check reports this separately
        prerelease = re.fullmatch(
            r"([0-9]+\.[0-9]+\.[0-9]+)-rc\.([1-9][0-9]*)", expected
        )
        firefox_expected = (
            f"{prerelease.group(1)}.{prerelease.group(2)}"
            if prerelease
            else expected
        )
        if value != firefox_expected:
            findings.add(
                manifest,
                f"Firefox version must map canonical version {expected} to {firefox_expected}",
            )
        try:
            version_name = json.loads(
                manifest.read_text(encoding="utf-8"),
                object_pairs_hook=reject_duplicate_keys,
            ).get("version_name")
        except (ValueError, json.JSONDecodeError):
            continue
        if prerelease and version_name != expected:
            findings.add(
                manifest, f"Firefox version_name must preserve {expected}"
            )

    for cargo in ROOT.rglob("Cargo.toml"):
        if EXCLUDED_PARTS.intersection(cargo.parts):
            continue
        text = cargo.read_text(encoding="utf-8")
        workspace = re.search(r"(?ms)^\[workspace\.package\].*?^version\s*=\s*['\"]([^'\"]+)['\"]", text)
        package = re.search(r"(?ms)^\[package\].*?^version\s*=\s*['\"]([^'\"]+)['\"]", text)
        for match in (workspace, package):
            if match and match.group(1) != expected:
                findings.add(cargo, f"Rust version {match.group(1)} must match canonical version {expected}")


def check_identity_migration(findings: Findings) -> None:
    path = ROOT / "docs" / "fixtures" / "pinakotheke-identity-migration-matrix.json"
    if not path.is_file():
        findings.add(path, "Pinakotheke identity migration matrix is missing")
        return
    try:
        document = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=reject_duplicate_keys)
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        findings.add(path, f"invalid migration matrix JSON: {error}")
        return
    if document.get("schema") != "x-img.identity-migration-matrix" or document.get("schema_major") != 1:
        findings.add(path, "migration matrix must declare x-img.identity-migration-matrix major 1")
    if document.get("canonical", {}).get("repository") != "sagrudd/pinakotheke":
        findings.add(path, "migration matrix must name canonical repository sagrudd/pinakotheke")
    if document.get("legacy", {}).get("repository") != "sagrudd/x-img":
        findings.add(path, "migration matrix must preserve legacy repository sagrudd/x-img")
    if document.get("no_partial_release") is not True:
        findings.add(path, "migration matrix must prohibit partial release")

    required_surfaces = {
        "documentation", "github-repository", "rust-workspace-package", "cli",
        "monas-product", "synoptikon-adapter", "das-application",
        "catalogue-config-schema", "objectstore-references", "firefox-extension",
        "ci-release-artifacts",
    }
    surface_ids = {entry.get("id") for entry in document.get("surfaces", []) if isinstance(entry, dict)}
    missing_surfaces = required_surfaces - surface_ids
    if missing_surfaces:
        findings.add(path, f"migration matrix missing identity surfaces: {', '.join(sorted(missing_surfaces))}")

    required_proofs = {"existing-config", "existing-catalogue", "existing-object-reference", "existing-extension-pairing"}
    proof_ids = {entry.get("id") for entry in document.get("proof_cases", []) if isinstance(entry, dict)}
    missing_proofs = required_proofs - proof_ids
    if missing_proofs:
        findings.add(path, f"migration matrix missing proof cases: {', '.join(sorted(missing_proofs))}")


CHECKS = {
    "identity-migration": check_identity_migration,
    "json": check_json,
    "links": check_links,
    "privacy": check_privacy,
    "versions": check_versions,
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("checks", nargs="*", choices=sorted(CHECKS), help="checks to run (default: all)")
    args = parser.parse_args()
    selected = args.checks or list(CHECKS)
    findings = Findings()
    for name in selected:
        CHECKS[name](findings)
    if findings.errors:
        for error in sorted(set(findings.errors)):
            print(f"error: {error}", file=sys.stderr)
        print(f"quality checks failed with {len(set(findings.errors))} finding(s)", file=sys.stderr)
        return 1
    print(f"quality checks passed: {', '.join(selected)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
