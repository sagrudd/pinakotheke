#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Prepare, logically seal, and verify a version-specific release output.

The tool deliberately does not build, deploy, copy, or delete artifacts.
Instead it creates a fresh output directory, records artifacts produced there
by an external build, and seals a deterministic manifest once the output is
complete.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
import tomllib
from pathlib import Path, PurePosixPath
from typing import Any, NoReturn, Sequence


SCHEMA_VERSION = "pinakotheke.release-output.v1"
MANIFEST_NAME = "release-manifest.json"
PRODUCT = "Pinakotheke"
SEMVER = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)
SHA256 = re.compile(r"^[0-9a-f]{64}$")
COMMIT = re.compile(r"^[0-9a-f]{40,64}$")
ALLOWED_STATES = {"prepared", "sealed"}
ALLOWED_ARTIFACT_KINDS = {
    "checksum-manifest",
    "deb",
    "firefox-xpi",
    "macos-pkg",
    "release-evidence",
    "rollback-package",
    "rpm",
    "sbom",
    "source-archive",
}
REQUIRED_ARTIFACT_MATRIX = {
    ("checksum-manifest", "portable"),
    ("deb", "arm64"),
    ("deb", "x86_64"),
    ("firefox-xpi", "portable"),
    ("macos-pkg", "arm64"),
    ("macos-pkg", "x86_64"),
    ("release-evidence", "portable"),
    ("rollback-package", "x86_64"),
    ("rpm", "arm64"),
    ("rpm", "x86_64"),
    ("sbom", "portable"),
    ("source-archive", "source"),
}
ALLOWED_ARCHITECTURES = {
    "arm64",
    "multi",
    "portable",
    "source",
    "x86_64",
}
ALLOWED_ARTIFACT_KEYS = {
    "architecture",
    "kind",
    "path",
    "sha256",
    "size_bytes",
}
ALLOWED_MANIFEST_KEYS = {
    "artifacts",
    "build_commands",
    "compatibility_manifest",
    "dependencies",
    "local_test_evidence",
    "product",
    "release_version",
    "schema_version",
    "source",
    "state",
    "version_mirrors",
}
ALLOWED_COMPATIBILITY_KEYS = {"path", "sha256"}
ALLOWED_TEST_EVIDENCE_KEYS = {"command", "result"}


class ReleaseOutputError(RuntimeError):
    """A safe, user-facing release-output validation failure."""


def _fail(message: str) -> NoReturn:
    raise ReleaseOutputError(message)


def _run_git(repository: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", os.fspath(repository), *arguments],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        _fail(f"git {' '.join(arguments)} failed: {detail}")
    return result.stdout.strip()


def _repository_details(repository: Path) -> tuple[Path, str]:
    repository = repository.resolve()
    top_level = Path(
        _run_git(repository, "rev-parse", "--show-toplevel")
    ).resolve()
    if top_level != repository:
        _fail(
            f"repository must be the Git worktree root ({top_level}), "
            f"not {repository}"
        )
    if _run_git(repository, "status", "--porcelain=v1", "--untracked-files=all"):
        _fail("source tree is dirty; commit or remove every change first")
    commit = _run_git(repository, "rev-parse", "HEAD")
    if not COMMIT.fullmatch(commit):
        _fail("Git returned an invalid source commit")
    return repository, commit


def _workspace_version(repository: Path) -> str:
    cargo_path = repository / "Cargo.toml"
    try:
        with cargo_path.open("rb") as cargo_file:
            cargo = tomllib.load(cargo_file)
    except (OSError, tomllib.TOMLDecodeError) as error:
        _fail(f"cannot read workspace Cargo.toml: {error}")
    workspace = cargo.get("workspace")
    if not isinstance(workspace, dict):
        _fail("Cargo.toml does not define [workspace]")
    package = workspace.get("package")
    if not isinstance(package, dict) or not isinstance(package.get("version"), str):
        _fail("Cargo.toml does not define [workspace.package].version")
    version = package["version"]
    if not SEMVER.fullmatch(version):
        _fail("workspace package version is not valid Semantic Versioning")
    return version


def _validate_release_version(repository: Path, requested: str) -> None:
    if not SEMVER.fullmatch(requested):
        _fail("release version is not valid Semantic Versioning")
    workspace_version = _workspace_version(repository)
    if requested != workspace_version:
        _fail(
            f"release version {requested} does not match workspace version "
            f"{workspace_version}"
        )


def _reject_symlink_ancestors(path: Path, purpose: str) -> None:
    absolute = path.absolute()
    current = Path(absolute.anchor)
    for part in absolute.parts[1:]:
        current = current / part
        if current.exists() or current.is_symlink():
            if current.is_symlink():
                _fail(f"{purpose} traverses symbolic link: {current}")
        else:
            break


def _repository_file(repository: Path, raw_path: Any) -> tuple[str, Path]:
    relative = _relative_artifact_path(raw_path)
    candidate = repository.joinpath(*relative.parts)
    _reject_symlink_ancestors(candidate, "compatibility manifest")
    try:
        resolved = candidate.resolve(strict=True)
    except OSError as error:
        _fail(f"compatibility manifest is unavailable: {raw_path}: {error}")
    try:
        resolved.relative_to(repository)
    except ValueError:
        _fail("compatibility manifest escapes the source repository")
    if not resolved.is_file():
        _fail("compatibility manifest must be a regular file")
    return str(relative), resolved


def _require_release_ready_compatibility(path: Path) -> None:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
        schema_version = document["schema_version"]
        release_status = document["release"]["status"]
        blocking_gates = document["blocking_gates"]
    except (OSError, json.JSONDecodeError, KeyError, TypeError) as error:
        _fail(f"compatibility manifest is not a readable RC contract: {error}")
    if schema_version != "pinakotheke.rc-compatibility.v1":
        _fail("compatibility manifest has an unsupported schema")
    if release_status != "release_ready" or blocking_gates != []:
        _fail("compatibility manifest is blocked and cannot seed release output")


def _version_mirrors_from_compatibility(
    compatibility_file: Path, product_version: str
) -> dict[str, str]:
    try:
        release = json.loads(compatibility_file.read_text(encoding="utf-8"))["release"]
        mirrors = {
            "deb_version": release["deb_version"],
            "firefox_manifest_version": release["firefox_manifest_version"],
            "firefox_version_name": release["firefox_version_name"],
            "macos_pkg_version": release["macos_pkg_version"],
            "product_semver": release["target_version"],
            "rpm_release": release["rpm_release"],
            "rpm_version": release["rpm_version"],
        }
    except (OSError, json.JSONDecodeError, KeyError, TypeError) as error:
        _fail(f"compatibility manifest lacks version mappings: {error}")
    if any(not isinstance(value, str) or not value for value in mirrors.values()):
        _fail("compatibility manifest contains an invalid version mapping")
    if mirrors["product_semver"] != product_version:
        _fail("compatibility target and requested release version differ")
    return mirrors


def _dependencies_from_compatibility(
    compatibility_file: Path,
) -> dict[str, dict[str, Any]]:
    try:
        raw_dependencies = json.loads(
            compatibility_file.read_text(encoding="utf-8")
        )["dependencies"]
    except (OSError, json.JSONDecodeError, KeyError, TypeError) as error:
        _fail(f"compatibility manifest lacks dependency evidence: {error}")
    if not isinstance(raw_dependencies, dict) or not raw_dependencies:
        _fail("compatibility dependencies must be a non-empty object")
    dependencies: dict[str, dict[str, Any]] = {}
    for name, raw in sorted(raw_dependencies.items()):
        if not isinstance(name, str) or not isinstance(raw, dict):
            _fail("compatibility dependency evidence is invalid")
        try:
            dependency = {
                "minimum_version": raw["minimum_version"],
                "required_capabilities": raw["required_capabilities"],
                "tested_commit": raw["tested_commit"],
                "tested_version": raw["tested_version"],
            }
        except KeyError as error:
            _fail(f"compatibility dependency {name} lacks {error.args[0]}")
        if not COMMIT.fullmatch(str(dependency["tested_commit"])):
            _fail(f"compatibility dependency {name} has invalid tested commit")
        for field in ("minimum_version", "tested_version"):
            value = dependency[field]
            if value is not None and (
                not isinstance(value, str) or not SEMVER.fullmatch(value)
            ):
                _fail(f"compatibility dependency {name} has invalid {field}")
        capabilities = dependency["required_capabilities"]
        if (
            not isinstance(capabilities, list)
            or not capabilities
            or len(capabilities) != len(set(capabilities))
            or any(not isinstance(value, str) or not value for value in capabilities)
        ):
            _fail(
                f"compatibility dependency {name} has invalid required capabilities"
            )
        dependency["required_capabilities"] = sorted(capabilities)
        dependencies[name] = dependency
    return dependencies


def _run_compatibility_checker(
    repository: Path, compatibility_path: str
) -> None:
    checker = repository / "scripts/release/check_rc_compatibility.py"
    if not checker.is_file() or checker.is_symlink():
        _fail("release source lacks the RC compatibility checker")
    result = subprocess.run(
        [
            sys.executable,
            os.fspath(checker),
            "--root",
            os.fspath(repository),
            "--manifest",
            os.fspath(repository / compatibility_path),
            "--require-ready",
            "--require-siblings",
            "--verify-product-source",
        ],
        cwd=repository,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        _fail(f"compatibility readiness check failed: {detail}")


def _safe_output_directory(output_root: Path, version: str) -> Path:
    _reject_symlink_ancestors(output_root, "output root")
    if output_root.exists():
        if output_root.is_symlink():
            _fail("output root must not be a symbolic link")
        if not output_root.is_dir():
            _fail("output root must be a directory")
    else:
        output_root.mkdir(parents=True, mode=0o755)

    resolved_root = output_root.resolve()
    target = output_root / version
    if target.exists() or target.is_symlink():
        _fail(f"release output already exists and will not be reused: {target}")
    if target.parent.resolve() != resolved_root:
        _fail("release output escapes its output root")
    try:
        target.mkdir(mode=0o755)
    except FileExistsError:
        _fail(f"release output was created concurrently: {target}")
    return target.resolve()


def _manifest_path(directory: Path) -> Path:
    directory = directory.resolve()
    if not directory.is_dir():
        _fail(f"release output directory does not exist: {directory}")
    return directory / MANIFEST_NAME


def _write_manifest(path: Path, manifest: dict[str, Any]) -> None:
    payload = (
        json.dumps(manifest, indent=2, sort_keys=True, ensure_ascii=True) + "\n"
    ).encode("utf-8")
    temporary = path.with_name(f".{path.name}.tmp")
    try:
        with temporary.open("xb") as output:
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
    finally:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass


def _load_manifest(directory: Path) -> tuple[Path, dict[str, Any]]:
    path = _manifest_path(directory)
    if path.is_symlink():
        _fail("release manifest must not be a symbolic link")
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        _fail(f"cannot read release manifest: {error}")
    if not isinstance(raw, dict):
        _fail("release manifest must be a JSON object")
    return path, raw


def _relative_artifact_path(raw_path: Any) -> PurePosixPath:
    if not isinstance(raw_path, str):
        _fail("artifact path must be a string")
    path = PurePosixPath(raw_path)
    if (
        not raw_path
        or path.is_absolute()
        or ".." in path.parts
        or "." in path.parts
        or "\\" in raw_path
    ):
        _fail("artifact path must be a normalized relative POSIX path")
    if path.name == MANIFEST_NAME:
        _fail("the release manifest cannot be registered as an artifact")
    return path


def _artifact_file(directory: Path, raw_path: str) -> Path:
    relative = _relative_artifact_path(raw_path)
    candidate = directory.joinpath(*relative.parts)
    if candidate.is_symlink():
        _fail(f"artifact must not be a symbolic link: {raw_path}")
    for parent in candidate.parents:
        if parent == directory:
            break
        if parent.is_symlink():
            _fail(f"artifact path traverses a symbolic link: {raw_path}")
    try:
        resolved = candidate.resolve(strict=True)
    except OSError as error:
        _fail(f"artifact is unavailable: {raw_path}: {error}")
    try:
        resolved.relative_to(directory.resolve())
    except ValueError:
        _fail(f"artifact escapes the release output: {raw_path}")
    if not resolved.is_file():
        _fail(f"artifact is not a regular file: {raw_path}")
    return resolved


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as artifact:
        for chunk in iter(lambda: artifact.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _require_nonempty_string(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value.strip() or value != value.strip():
        _fail(f"{field} must be a non-empty, trimmed string")
    return value


def _validate_manifest_shape(manifest: dict[str, Any]) -> None:
    if set(manifest) != ALLOWED_MANIFEST_KEYS:
        _fail("release manifest has missing or unknown top-level fields")
    if manifest["schema_version"] != SCHEMA_VERSION:
        _fail("unsupported release manifest schema")
    if manifest["product"] != PRODUCT:
        _fail("release manifest has the wrong product identity")
    version = _require_nonempty_string(
        manifest["release_version"], "release_version"
    )
    if not SEMVER.fullmatch(version):
        _fail("release manifest version is not valid Semantic Versioning")
    if (
        not isinstance(manifest["state"], str)
        or manifest["state"] not in ALLOWED_STATES
    ):
        _fail("release manifest has an invalid state")
    mirrors = manifest["version_mirrors"]
    if not isinstance(mirrors, dict) or set(mirrors) != {
        "deb_version",
        "firefox_manifest_version",
        "firefox_version_name",
        "macos_pkg_version",
        "product_semver",
        "rpm_release",
        "rpm_version",
    }:
        _fail("release manifest version_mirrors has missing or unknown fields")
    if mirrors["product_semver"] != version:
        _fail("release manifest product version mirror disagrees with release_version")

    compatibility = manifest["compatibility_manifest"]
    if (
        not isinstance(compatibility, dict)
        or set(compatibility) != ALLOWED_COMPATIBILITY_KEYS
    ):
        _fail(
            "release manifest compatibility_manifest has missing or unknown fields"
        )
    _relative_artifact_path(compatibility["path"])
    if (
        not isinstance(compatibility["sha256"], str)
        or not SHA256.fullmatch(compatibility["sha256"])
    ):
        _fail("compatibility manifest sha256 is invalid")

    dependencies = manifest["dependencies"]
    if not isinstance(dependencies, dict) or not dependencies:
        _fail("release manifest dependencies must be a non-empty object")
    for name, dependency in dependencies.items():
        if (
            not isinstance(name, str)
            or not isinstance(dependency, dict)
            or set(dependency)
            != {
                "minimum_version",
                "required_capabilities",
                "tested_commit",
                "tested_version",
            }
        ):
            _fail("release manifest dependency evidence is invalid")
        if not COMMIT.fullmatch(str(dependency["tested_commit"])):
            _fail("release manifest dependency tested commit is invalid")
        if (
            not isinstance(dependency["required_capabilities"], list)
            or not dependency["required_capabilities"]
        ):
            _fail("release manifest dependency capabilities are invalid")

    test_evidence = manifest["local_test_evidence"]
    if not isinstance(test_evidence, list) or not test_evidence:
        _fail("release manifest must contain local test evidence")
    previous_test_command = ""
    for evidence in test_evidence:
        if (
            not isinstance(evidence, dict)
            or set(evidence) != ALLOWED_TEST_EVIDENCE_KEYS
        ):
            _fail("local test evidence has missing or unknown fields")
        command = _require_nonempty_string(evidence["command"], "test command")
        if command <= previous_test_command:
            _fail("local test evidence must be uniquely command-sorted")
        previous_test_command = command
        if evidence["result"] != "passed":
            _fail("local test evidence may record only passed commands")

    source = manifest["source"]
    if not isinstance(source, dict) or set(source) != {"clean", "commit"}:
        _fail("release manifest source must contain only clean and commit")
    if source["clean"] is not True or not isinstance(source["commit"], str):
        _fail("release manifest source evidence is invalid")
    if not COMMIT.fullmatch(source["commit"]):
        _fail("release manifest source commit is invalid")

    commands = manifest["build_commands"]
    if not isinstance(commands, list) or not commands:
        _fail("release manifest must contain at least one build command")
    for command in commands:
        _require_nonempty_string(command, "build command")
    if len(commands) != len(set(commands)):
        _fail("release manifest contains duplicate build commands")

    artifacts = manifest["artifacts"]
    if not isinstance(artifacts, list):
        _fail("release manifest artifacts must be a list")
    previous = ""
    for artifact in artifacts:
        if not isinstance(artifact, dict) or set(artifact) != ALLOWED_ARTIFACT_KEYS:
            _fail("release manifest artifact has missing or unknown fields")
        path = str(_relative_artifact_path(artifact["path"]))
        if path <= previous:
            _fail("release manifest artifacts must be uniquely path-sorted")
        previous = path
        kind = _require_nonempty_string(artifact["kind"], "artifact kind")
        if kind not in ALLOWED_ARTIFACT_KINDS:
            _fail(f"unsupported artifact kind: {kind}")
        architecture = _require_nonempty_string(
            artifact["architecture"], "architecture"
        )
        if architecture not in ALLOWED_ARCHITECTURES:
            _fail(f"unsupported artifact architecture: {architecture}")
        if (
            not isinstance(artifact["size_bytes"], int)
            or isinstance(artifact["size_bytes"], bool)
            or artifact["size_bytes"] < 0
        ):
            _fail("artifact size_bytes must be a non-negative integer")
        if (
            not isinstance(artifact["sha256"], str)
            or not SHA256.fullmatch(artifact["sha256"])
        ):
            _fail("artifact sha256 is invalid")


def prepare(
    repository: Path,
    output_root: Path,
    version: str,
    build_commands: Sequence[str],
    compatibility_manifest: str,
    test_commands_passed: Sequence[str],
) -> Path:
    repository, commit = _repository_details(repository)
    _validate_release_version(repository, version)
    commands = [_require_nonempty_string(command, "build command") for command in build_commands]
    if not commands:
        _fail("at least one build command is required")
    if len(commands) != len(set(commands)):
        _fail("build commands must not be duplicated")
    compatibility_path, compatibility_file = _repository_file(
        repository, compatibility_manifest
    )
    _require_release_ready_compatibility(compatibility_file)
    _run_compatibility_checker(repository, compatibility_path)
    tests = sorted(
        {
            _require_nonempty_string(command, "test command")
            for command in test_commands_passed
        }
    )
    if not tests:
        _fail("at least one passed local test command is required")
    if len(tests) != len(test_commands_passed):
        _fail("passed local test commands must not be duplicated")
    target = _safe_output_directory(output_root, version)
    manifest = {
        "artifacts": [],
        "build_commands": commands,
        "compatibility_manifest": {
            "path": compatibility_path,
            "sha256": _sha256(compatibility_file),
        },
        "dependencies": _dependencies_from_compatibility(compatibility_file),
        "local_test_evidence": [
            {"command": command, "result": "passed"} for command in tests
        ],
        "product": PRODUCT,
        "release_version": version,
        "schema_version": SCHEMA_VERSION,
        "source": {"clean": True, "commit": commit},
        "state": "prepared",
        "version_mirrors": _version_mirrors_from_compatibility(
            compatibility_file, version
        ),
    }
    _write_manifest(target / MANIFEST_NAME, manifest)
    return target


def add_artifact(
    directory: Path,
    artifact_path: str,
    kind: str,
    architecture: str,
) -> None:
    manifest_path, manifest = _load_manifest(directory)
    _validate_manifest_shape(manifest)
    if manifest["state"] != "prepared":
        _fail("sealed release output cannot accept another artifact")
    kind = _require_nonempty_string(kind, "artifact kind")
    if kind not in ALLOWED_ARTIFACT_KINDS:
        _fail(f"unsupported artifact kind: {kind}")
    architecture = _require_nonempty_string(architecture, "architecture")
    if architecture not in ALLOWED_ARCHITECTURES:
        _fail(f"unsupported artifact architecture: {architecture}")
    relative = str(_relative_artifact_path(artifact_path))
    artifact = _artifact_file(directory.resolve(), relative)
    if any(entry["path"] == relative for entry in manifest["artifacts"]):
        _fail(f"artifact is already registered: {relative}")
    manifest["artifacts"].append(
        {
            "architecture": architecture,
            "kind": kind,
            "path": relative,
            "sha256": _sha256(artifact),
            "size_bytes": artifact.stat().st_size,
        }
    )
    manifest["artifacts"].sort(key=lambda entry: entry["path"])
    _write_manifest(manifest_path, manifest)


def _listed_output_files(directory: Path) -> set[str]:
    files: set[str] = set()
    for path in directory.rglob("*"):
        if path.is_symlink():
            _fail(
                f"release output contains a symbolic link: "
                f"{path.relative_to(directory).as_posix()}"
            )
        mode = path.lstat().st_mode
        if stat.S_ISDIR(mode):
            continue
        if not stat.S_ISREG(mode):
            _fail(
                f"release output contains a special filesystem entry: "
                f"{path.relative_to(directory).as_posix()}"
            )
        if path.name != MANIFEST_NAME:
            files.add(path.relative_to(directory).as_posix())
    return files


def verify(directory: Path, require_sealed: bool = False) -> dict[str, Any]:
    _, manifest = _load_manifest(directory)
    _validate_manifest_shape(manifest)
    directory = directory.resolve()
    if directory.name != manifest["release_version"]:
        _fail("release output directory name does not match its version")
    if require_sealed and manifest["state"] != "sealed":
        _fail("release output is not sealed")
    listed: set[str] = set()
    for artifact in manifest["artifacts"]:
        path = _artifact_file(directory, artifact["path"])
        if path.stat().st_size != artifact["size_bytes"]:
            _fail(f"artifact size mismatch: {artifact['path']}")
        if _sha256(path) != artifact["sha256"]:
            _fail(f"artifact checksum mismatch: {artifact['path']}")
        listed.add(artifact["path"])
    present = _listed_output_files(directory)
    if present != listed:
        missing = sorted(listed - present)
        unlisted = sorted(present - listed)
        _fail(
            "artifact inventory mismatch: "
            f"missing={missing!r}, unlisted={unlisted!r}"
        )
    if manifest["state"] == "sealed" and not manifest["artifacts"]:
        _fail("sealed release output must contain at least one artifact")
    return manifest


def _require_artifact_matrix(manifest: dict[str, Any]) -> None:
    actual = {
        (artifact["kind"], artifact["architecture"])
        for artifact in manifest["artifacts"]
    }
    duplicates = len(actual) != len(manifest["artifacts"])
    missing = sorted(REQUIRED_ARTIFACT_MATRIX - actual)
    extra = sorted(actual - REQUIRED_ARTIFACT_MATRIX)
    if duplicates or missing or extra:
        _fail(
            "release artifact matrix mismatch: "
            f"duplicates={duplicates!r}, missing={missing!r}, extra={extra!r}"
        )


def seal(directory: Path, repository: Path) -> None:
    manifest_path, manifest = _load_manifest(directory)
    _validate_manifest_shape(manifest)
    if manifest["state"] != "prepared":
        _fail("release output is already sealed")
    if not manifest["artifacts"]:
        _fail("cannot seal release output without artifacts")
    _require_artifact_matrix(manifest)
    repository, commit = _repository_details(repository)
    if commit != manifest["source"]["commit"]:
        _fail("source HEAD changed after release output preparation")
    _validate_release_version(repository, manifest["release_version"])
    compatibility_path, compatibility_file = _repository_file(
        repository, manifest["compatibility_manifest"]["path"]
    )
    _require_release_ready_compatibility(compatibility_file)
    _run_compatibility_checker(repository, compatibility_path)
    if (
        compatibility_path != manifest["compatibility_manifest"]["path"]
        or _sha256(compatibility_file)
        != manifest["compatibility_manifest"]["sha256"]
    ):
        _fail("compatibility manifest changed after release output preparation")
    if manifest["version_mirrors"] != _version_mirrors_from_compatibility(
        compatibility_file, manifest["release_version"]
    ):
        _fail("release version mirrors differ from compatibility evidence")
    if manifest["dependencies"] != _dependencies_from_compatibility(
        compatibility_file
    ):
        _fail("release dependency evidence differs from compatibility evidence")
    verify(directory)
    manifest["state"] = "sealed"
    _write_manifest(manifest_path, manifest)
    verify(directory, require_sealed=True)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    prepare_parser = subparsers.add_parser(
        "prepare", help="create a fresh version-specific release output"
    )
    prepare_parser.add_argument("--repository", type=Path, default=Path.cwd())
    prepare_parser.add_argument("--output-root", type=Path, required=True)
    prepare_parser.add_argument("--version", required=True)
    prepare_parser.add_argument(
        "--build-command",
        action="append",
        required=True,
        dest="build_commands",
        help="exact reproducible build command; repeat in execution order",
    )
    prepare_parser.add_argument(
        "--compatibility-manifest",
        required=True,
        help="repository-relative required RC compatibility manifest",
    )
    prepare_parser.add_argument(
        "--test-command-passed",
        action="append",
        required=True,
        dest="test_commands_passed",
        help="exact local command with a passing result; repeat as needed",
    )

    artifact_parser = subparsers.add_parser(
        "add-artifact", help="checksum and register one produced artifact"
    )
    artifact_parser.add_argument("--directory", type=Path, required=True)
    artifact_parser.add_argument("--artifact", required=True)
    artifact_parser.add_argument("--kind", required=True)
    artifact_parser.add_argument("--architecture", required=True)

    seal_parser = subparsers.add_parser(
        "seal", help="verify and logically seal the release manifest"
    )
    seal_parser.add_argument("--directory", type=Path, required=True)
    seal_parser.add_argument("--repository", type=Path, default=Path.cwd())

    verify_parser = subparsers.add_parser(
        "verify", help="verify a prepared or sealed release output"
    )
    verify_parser.add_argument("--directory", type=Path, required=True)
    verify_parser.add_argument("--require-sealed", action="store_true")
    return parser


def main(arguments: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(arguments)
    try:
        if args.command == "prepare":
            target = prepare(
                args.repository,
                args.output_root,
                args.version,
                args.build_commands,
                args.compatibility_manifest,
                args.test_commands_passed,
            )
            print(target)
        elif args.command == "add-artifact":
            add_artifact(
                args.directory,
                args.artifact,
                args.kind,
                args.architecture,
            )
            print(f"registered {args.artifact}")
        elif args.command == "seal":
            seal(args.directory, args.repository)
            print(f"sealed {args.directory}")
        elif args.command == "verify":
            manifest = verify(args.directory, args.require_sealed)
            print(
                f"verified {manifest['release_version']} "
                f"({manifest['state']}, {len(manifest['artifacts'])} artifacts)"
            )
        else:  # pragma: no cover - argparse enforces a known command.
            _fail(f"unknown command: {args.command}")
    except ReleaseOutputError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
