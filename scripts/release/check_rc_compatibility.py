#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Validate the Pinakotheke RC compatibility manifest without third-party modules."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MANIFEST = (
    ROOT / "contracts/release/pinakotheke-1.29.0-rc.1.compatibility.json"
)
SCHEMA = ROOT / "contracts/release/pinakotheke-rc-compatibility.v1.schema.json"
SCHEMA_VERSION = "x-img.rc-compatibility.v1"
DEPENDENCIES = {
    "monas": "monas",
    "dasobjectstore": "DASObjectStore",
    "mnemosyne": "mnemosyne",
    "mnemosyne_design_language": "mnemosyne_design_language",
}
POST_TEST_EVIDENCE_PATHS = {
    "CHANGELOG.md",
    "MILESTONES.md",
    "README.md",
    "TODO.md",
    "contracts/release/pinakotheke-1.29.0-rc.1.compatibility.json",
    "docs/compatibility-matrix.md",
    "docs/index.rst",
    "docs/packaging.rst",
    "docs/release-candidate-1-29-0.rst",
}
SEMVER = re.compile(
    r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$"
)
SHA = re.compile(r"^[0-9a-f]{40}$")
CAPABILITY = re.compile(r"^[a-z0-9][a-z0-9._-]+\.v[0-9]+$")


class CompatibilityError(ValueError):
    """The compatibility evidence is incomplete or inconsistent."""


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise CompatibilityError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    try:
        return json.loads(
            path.read_text(encoding="utf-8"), object_pairs_hook=reject_duplicate_keys
        )
    except (OSError, json.JSONDecodeError) as error:
        raise CompatibilityError(f"cannot read JSON {path}: {error}") from error


def object_with_keys(
    value: Any, location: str, required: set[str], optional: set[str] | None = None
) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise CompatibilityError(f"{location} must be an object")
    allowed = required | (optional or set())
    missing = required - value.keys()
    extra = value.keys() - allowed
    if missing:
        raise CompatibilityError(f"{location} misses {', '.join(sorted(missing))}")
    if extra:
        raise CompatibilityError(f"{location} has unknown {', '.join(sorted(extra))}")
    return value


def require_string(value: Any, location: str, pattern: re.Pattern[str] | None = None) -> str:
    if not isinstance(value, str) or not value:
        raise CompatibilityError(f"{location} must be a non-empty string")
    if pattern is not None and not pattern.fullmatch(value):
        raise CompatibilityError(f"{location} has invalid value {value!r}")
    return value


def require_semver(value: Any, location: str) -> str:
    return require_string(value, location, SEMVER)


def require_string_list(
    value: Any, location: str, pattern: re.Pattern[str] | None = None
) -> list[str]:
    if not isinstance(value, list) or any(not isinstance(item, str) for item in value):
        raise CompatibilityError(f"{location} must be a string array")
    if len(value) != len(set(value)):
        raise CompatibilityError(f"{location} contains duplicates")
    if pattern is not None:
        for item in value:
            require_string(item, location, pattern)
    return value


def safe_file(root: Path, relative: Any, location: str) -> Path:
    text = require_string(relative, location)
    path = (root / text).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError as error:
        raise CompatibilityError(f"{location} escapes the repository") from error
    if not path.is_file():
        raise CompatibilityError(f"{location} does not exist: {text}")
    return path


def json_pointer(value: Any, pointer: str) -> Any:
    if not pointer.startswith("/"):
        raise CompatibilityError(f"JSON pointer must start with '/': {pointer}")
    current = value
    for raw in pointer[1:].split("/"):
        token = raw.replace("~1", "/").replace("~0", "~")
        if not isinstance(current, dict) or token not in current:
            raise CompatibilityError(f"JSON pointer is absent: {pointer}")
        current = current[token]
    return current


def cargo_workspace_version(path: Path) -> str:
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
        version = data["workspace"]["package"]["version"]
    except (OSError, tomllib.TOMLDecodeError, KeyError, TypeError) as error:
        raise CompatibilityError(f"cannot read workspace version from {path}: {error}") from error
    return require_semver(version, f"{path}:workspace.package.version")


def expected_firefox_version(target: str) -> str:
    match = SEMVER.fullmatch(target)
    if match is None:
        raise CompatibilityError("release.target_version is not semantic versioning")
    prerelease = match.group(4)
    if prerelease is None:
        return ".".join(match.group(index) for index in (1, 2, 3))
    rc = re.fullmatch(r"rc\.([1-9][0-9]*)", prerelease)
    if rc is None:
        raise CompatibilityError(
            "Firefox mapping is defined only for an rc.N target version"
        )
    return ".".join((*[match.group(index) for index in (1, 2, 3)], rc.group(1)))


def expected_package_versions(target: str) -> dict[str, str]:
    """Map product SemVer into native package-manager prerelease ordering."""
    match = SEMVER.fullmatch(target)
    if match is None:
        raise CompatibilityError("release.target_version is not semantic versioning")
    major, minor, patch, prerelease = (match.group(index) for index in (1, 2, 3, 4))
    stable = f"{major}.{minor}.{patch}"
    if prerelease is None:
        return {
            "deb_version": stable,
            "rpm_version": stable,
            "rpm_release": "1",
            "macos_pkg_version": stable,
        }
    parsed = re.fullmatch(r"(alpha|beta|rc)\.([1-9][0-9]*)", prerelease)
    if parsed is None:
        raise CompatibilityError(
            "native package mapping supports only alpha.N, beta.N, or rc.N"
        )
    label, sequence = parsed.groups()
    return {
        "deb_version": f"{stable}~{label}.{sequence}",
        "rpm_version": stable,
        "rpm_release": f"0.{label}.{sequence}",
        "macos_pkg_version": f"{stable}.{sequence}",
    }


def validate_version_sources(root: Path, product: dict[str, Any]) -> None:
    baseline = require_semver(product["baseline_version"], "pinakotheke.baseline_version")
    sources = product["version_sources"]
    if not isinstance(sources, list) or len(sources) < 4:
        raise CompatibilityError("pinakotheke.version_sources must contain at least four sources")
    seen: set[tuple[str, str]] = set()
    for index, raw in enumerate(sources):
        location = f"pinakotheke.version_sources[{index}]"
        source = object_with_keys(raw, location, {"kind", "path", "selector"})
        kind = require_string(source["kind"], f"{location}.kind")
        path = safe_file(root, source["path"], f"{location}.path")
        selector = require_string(source["selector"], f"{location}.selector")
        identity = (str(path), selector)
        if identity in seen:
            raise CompatibilityError(f"{location} duplicates a version source")
        seen.add(identity)
        if kind == "cargo_workspace":
            if selector != "workspace.package.version":
                raise CompatibilityError(f"{location} has unsupported Cargo selector")
            actual = cargo_workspace_version(path)
        elif kind in {"json_pointer", "firefox_json_pointer"}:
            actual = json_pointer(load_json(path), selector)
            require_string(actual, f"{location} resolved value")
            if kind == "firefox_json_pointer":
                actual = baseline if actual == expected_firefox_version(baseline) else actual
            else:
                require_semver(actual, f"{location} resolved value")
        else:
            raise CompatibilityError(f"{location} has unsupported kind {kind!r}")
        if actual != baseline:
            raise CompatibilityError(
                f"{location} reports {actual}, expected baseline {baseline}"
            )


def validate_wire_contract(
    root: Path, raw: Any, location: str
) -> None:
    contract = object_with_keys(
        raw, location, {"contract_version", "path", "sha256"}
    )
    schema_version = require_string(
        contract["contract_version"], f"{location}.contract_version", CAPABILITY
    )
    path = safe_file(root, contract["path"], f"{location}.path")
    digest = require_string(
        contract["sha256"], f"{location}.sha256", re.compile(r"^[0-9a-f]{64}$")
    )
    actual_digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual_digest != digest:
        raise CompatibilityError(
            f"{location} checksum is {actual_digest}, expected {digest}"
        )
    document = load_json(path)
    encoded = json.dumps(document, sort_keys=True)
    if schema_version not in encoded:
        raise CompatibilityError(
            f"{location} schema version {schema_version!r} is absent from {path}"
        )


def validate_dependency(
    root: Path, name: str, raw: Any, gates: dict[str, dict[str, Any]]
) -> None:
    location = f"dependencies.{name}"
    dependency = object_with_keys(
        raw,
        location,
        {
            "repository",
            "tested_commit",
            "tested_version",
            "tested_components",
            "minimum_version",
            "evidence_status",
            "working_tree",
            "required_capabilities",
            "missing_required_capabilities",
            "wire_contracts",
        },
    )
    require_string(dependency["repository"], f"{location}.repository")
    require_string(dependency["tested_commit"], f"{location}.tested_commit", SHA)
    for key in ("tested_version", "minimum_version"):
        if dependency[key] is not None:
            require_semver(dependency[key], f"{location}.{key}")
    components = dependency["tested_components"]
    if not isinstance(components, list) or not components:
        raise CompatibilityError(f"{location}.tested_components must be a non-empty array")
    component_names: set[str] = set()
    for index, raw_component in enumerate(components):
        component_location = f"{location}.tested_components[{index}]"
        component = object_with_keys(
            raw_component, component_location, {"name", "version"}
        )
        component_name = require_string(
            component["name"],
            f"{component_location}.name",
            re.compile(r"^[A-Za-z0-9@][A-Za-z0-9@/._-]+$"),
        )
        if component_name in component_names:
            raise CompatibilityError(f"{location}.tested_components repeats {component_name}")
        component_names.add(component_name)
        require_semver(component["version"], f"{component_location}.version")
    status = require_string(dependency["evidence_status"], f"{location}.evidence_status")
    if status not in {"verified", "blocked_behavioral_preflight", "blocked_unreleased"}:
        raise CompatibilityError(f"{location}.evidence_status is unsupported")
    tree = require_string(dependency["working_tree"], f"{location}.working_tree")
    if tree not in {"clean", "dirty", "unknown"}:
        raise CompatibilityError(f"{location}.working_tree is unsupported")
    required = require_string_list(
        dependency["required_capabilities"],
        f"{location}.required_capabilities",
        CAPABILITY,
    )
    if not required:
        raise CompatibilityError(f"{location}.required_capabilities is empty")
    missing = require_string_list(
        dependency["missing_required_capabilities"],
        f"{location}.missing_required_capabilities",
        CAPABILITY,
    )
    if not set(missing).issubset(required):
        raise CompatibilityError(f"{location} misses a capability it does not require")
    if status == "verified":
        if tree != "clean" or missing:
            raise CompatibilityError(f"{location} verified evidence must be clean and complete")
        if dependency["minimum_version"] is None and name in {"monas", "dasobjectstore"}:
            raise CompatibilityError(f"{location} verified dependency lacks a minimum version")
    else:
        if not missing:
            raise CompatibilityError(f"{location} blocked evidence has no missing capability")
        gated = {
            gate["required_capability"]
            for gate in gates.values()
            if gate["owner"] == name
        }
        if not set(missing).issubset(gated):
            raise CompatibilityError(f"{location} missing capability lacks an unresolved gate")
    contracts = dependency["wire_contracts"]
    if not isinstance(contracts, list):
        raise CompatibilityError(f"{location}.wire_contracts must be an array")
    for index, contract in enumerate(contracts):
        validate_wire_contract(root, contract, f"{location}.wire_contracts[{index}]")


def validate_path_dependencies(root: Path) -> None:
    for manifest in root.rglob("Cargo.toml"):
        if any(part in {".git", ".codex", "target"} for part in manifest.parts):
            continue
        try:
            parsed = tomllib.loads(manifest.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as error:
            raise CompatibilityError(f"cannot inspect {manifest}: {error}") from error
        stack: list[Any] = [parsed]
        while stack:
            item = stack.pop()
            if isinstance(item, dict):
                for key, value in item.items():
                    if key == "path" and isinstance(value, str):
                        resolved = (manifest.parent / value).resolve()
                        try:
                            resolved.relative_to(root.resolve())
                        except ValueError as error:
                            raise CompatibilityError(
                                f"unpublished external path dependency in {manifest}: {value}"
                            ) from error
                    stack.append(value)
            elif isinstance(item, list):
                stack.extend(item)


def validate_manifest(root: Path, manifest_path: Path, require_ready: bool = False) -> dict[str, Any]:
    schema = load_json(SCHEMA if root == ROOT else root / "contracts/release/pinakotheke-rc-compatibility.v1.schema.json")
    if schema.get("properties", {}).get("schema_version", {}).get("const") != SCHEMA_VERSION:
        raise CompatibilityError("compatibility schema does not declare the supported major")
    manifest = object_with_keys(
        load_json(manifest_path),
        "manifest",
        {
            "schema_version",
            "manifest_id",
            "release",
            "pinakotheke",
            "dependencies",
            "blocking_gates",
        },
    )
    if manifest["schema_version"] != SCHEMA_VERSION:
        raise CompatibilityError(
            f"unsupported compatibility schema {manifest['schema_version']!r}"
        )
    require_string(manifest["manifest_id"], "manifest.manifest_id")
    release = object_with_keys(
        manifest["release"],
        "release",
        {
            "target_version",
            "deb_version",
            "rpm_version",
            "rpm_release",
            "macos_pkg_version",
            "firefox_manifest_version",
            "firefox_version_name",
            "status",
        },
    )
    target = require_semver(release["target_version"], "release.target_version")
    for field, expected in expected_package_versions(target).items():
        if release[field] != expected:
            raise CompatibilityError(
                f"release.{field} is {release[field]!r}, expected {expected!r}"
            )
    if release["firefox_manifest_version"] != expected_firefox_version(target):
        raise CompatibilityError("release.firefox_manifest_version is not the deterministic RC mapping")
    if release["firefox_version_name"] != target:
        raise CompatibilityError("release.firefox_version_name must preserve the semantic version")
    status = require_string(release["status"], "release.status")
    if status not in {"blocked", "release_ready"}:
        raise CompatibilityError("release.status is unsupported")
    product = object_with_keys(
        manifest["pinakotheke"],
        "pinakotheke",
        {
            "repository",
            "tested_commit",
            "baseline_version",
            "working_tree",
            "version_sources",
        },
    )
    if product["repository"] != "https://github.com/sagrudd/pinakotheke":
        raise CompatibilityError("pinakotheke.repository is not canonical")
    require_string(product["tested_commit"], "pinakotheke.tested_commit", SHA)
    if product["working_tree"] != "clean":
        raise CompatibilityError("Pinakotheke tested evidence is not clean")
    validate_version_sources(root, product)
    raw_gates = manifest["blocking_gates"]
    if not isinstance(raw_gates, list):
        raise CompatibilityError("blocking_gates must be an array")
    gates: dict[str, dict[str, Any]] = {}
    for index, raw in enumerate(raw_gates):
        location = f"blocking_gates[{index}]"
        gate = object_with_keys(
            raw,
            location,
            {"id", "owner", "state", "required_capability", "resolution"},
        )
        gate_id = require_string(gate["id"], f"{location}.id")
        if gate_id in gates:
            raise CompatibilityError(f"duplicate gate id {gate_id}")
        if gate["owner"] not in DEPENDENCIES and gate["owner"] != "pinakotheke":
            raise CompatibilityError(f"{location}.owner is unsupported")
        if gate["state"] != "unresolved":
            raise CompatibilityError(f"{location}.state must be unresolved")
        require_string(gate["required_capability"], f"{location}.required_capability", CAPABILITY)
        if not isinstance(gate["resolution"], str) or len(gate["resolution"]) < 16:
            raise CompatibilityError(f"{location}.resolution is not actionable")
        gates[gate_id] = gate
    dependencies = object_with_keys(
        manifest["dependencies"], "dependencies", set(DEPENDENCIES)
    )
    for name in DEPENDENCIES:
        validate_dependency(root, name, dependencies[name], gates)
    if status == "blocked" and not gates:
        raise CompatibilityError("blocked release has no unresolved gate")
    if status == "release_ready":
        if gates:
            raise CompatibilityError("release-ready manifest has unresolved gates")
        for name, dependency in dependencies.items():
            if (
                dependency["evidence_status"] != "verified"
                or dependency["working_tree"] != "clean"
                or dependency["missing_required_capabilities"]
            ):
                raise CompatibilityError(f"release-ready dependency {name} is not verified")
        if dependencies["dasobjectstore"]["minimum_version"] is None:
            raise CompatibilityError("release-ready DASObjectStore minimum is unresolved")
    if require_ready and status != "release_ready":
        raise CompatibilityError("manifest is intentionally blocked and cannot be promoted")
    validate_path_dependencies(root)
    return manifest


def verify_siblings(
    manifest: dict[str, Any], sibling_root: Path, require_siblings: bool
) -> None:
    for name, directory in DEPENDENCIES.items():
        path = sibling_root / directory
        if not path.is_dir():
            if require_siblings:
                raise CompatibilityError(f"missing sibling checkout: {path}")
            continue
        try:
            commit = subprocess.check_output(
                ["git", "-C", str(path), "rev-parse", "HEAD"], text=True
            ).strip()
            dirty = bool(
                subprocess.check_output(
                    ["git", "-C", str(path), "status", "--porcelain"], text=True
                ).strip()
            )
        except (OSError, subprocess.CalledProcessError) as error:
            raise CompatibilityError(f"cannot inspect sibling {path}: {error}") from error
        expected = manifest["dependencies"][name]
        if commit != expected["tested_commit"]:
            raise CompatibilityError(
                f"{name} sibling is {commit}, expected {expected['tested_commit']}"
            )
        actual_tree = "dirty" if dirty else "clean"
        if actual_tree != expected["working_tree"]:
            raise CompatibilityError(
                f"{name} sibling is {actual_tree}, manifest says {expected['working_tree']}"
            )


def verify_product_source(manifest: dict[str, Any], root: Path) -> None:
    """Bind tested product evidence to HEAD, allowing only evidence-only closure."""
    expected = manifest["pinakotheke"]["tested_commit"]
    try:
        head = subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
        ).strip()
        subprocess.run(
            ["git", "-C", str(root), "merge-base", "--is-ancestor", expected, head],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
        )
        changed = subprocess.check_output(
            ["git", "-C", str(root), "diff", "--name-only", f"{expected}..{head}"],
            text=True,
        ).splitlines()
    except (OSError, subprocess.CalledProcessError) as error:
        raise CompatibilityError(
            "Pinakotheke tested commit is not an ancestor of the release source"
        ) from error
    unexpected = sorted(set(changed) - POST_TEST_EVIDENCE_PATHS)
    if unexpected:
        raise CompatibilityError(
            "release source changed after tested commit outside evidence files: "
            + ", ".join(unexpected)
        )


def verified_das_minimum(manifest: dict[str, Any]) -> str:
    """Return the package-safe DAS minimum or reject incomplete evidence."""
    dependency = manifest["dependencies"]["dasobjectstore"]
    minimum = dependency["minimum_version"]
    if (
        dependency["evidence_status"] != "verified"
        or dependency["working_tree"] != "clean"
        or dependency["missing_required_capabilities"]
        or minimum is None
    ):
        raise CompatibilityError(
            "DASObjectStore minimum is unresolved or lacks clean verified evidence"
        )
    return require_semver(minimum, "dependencies.dasobjectstore.minimum_version")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--sibling-root", type=Path, default=ROOT.parent)
    parser.add_argument("--verify-siblings", action="store_true")
    parser.add_argument("--require-siblings", action="store_true")
    parser.add_argument("--require-ready", action="store_true")
    parser.add_argument("--verify-product-source", action="store_true")
    output = parser.add_mutually_exclusive_group()
    output.add_argument("--print-das-minimum", action="store_true")
    output.add_argument("--print-firefox-version", action="store_true")
    output.add_argument("--print-firefox-version-name", action="store_true")
    output.add_argument(
        "--print-release-field",
        choices=(
            "target_version",
            "deb_version",
            "rpm_version",
            "rpm_release",
            "macos_pkg_version",
        ),
    )
    args = parser.parse_args()
    try:
        manifest = validate_manifest(
            args.root.resolve(), args.manifest.resolve(), args.require_ready
        )
        if args.verify_siblings or args.require_siblings:
            verify_siblings(manifest, args.sibling_root.resolve(), args.require_siblings)
        if args.verify_product_source:
            verify_product_source(manifest, args.root.resolve())
    except CompatibilityError as error:
        print(f"RC compatibility check failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
    if args.print_das_minimum:
        try:
            print(verified_das_minimum(manifest))
        except CompatibilityError as error:
            print(f"RC compatibility check failed: {error}", file=sys.stderr)
            raise SystemExit(1) from error
    elif args.print_firefox_version:
        print(manifest["release"]["firefox_manifest_version"])
    elif args.print_firefox_version_name:
        print(manifest["release"]["firefox_version_name"])
    elif args.print_release_field:
        print(manifest["release"][args.print_release_field])
    else:
        print(
            "RC compatibility manifest verified: "
            f"{manifest['release']['target_version']} ({manifest['release']['status']})"
        )


if __name__ == "__main__":
    main()
