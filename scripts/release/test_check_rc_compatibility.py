#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Regression tests for the dependency-free RC compatibility checker."""

from __future__ import annotations

import copy
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts/release/check_rc_compatibility.py"
SPEC = importlib.util.spec_from_file_location("check_rc_compatibility", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class CompatibilityCheckerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest_path = (
            ROOT / "contracts/release/pinakotheke-1.29.0-rc.1.compatibility.json"
        )
        cls.manifest = json.loads(cls.manifest_path.read_text(encoding="utf-8"))

    def check_copy(self, manifest: dict, require_ready: bool = False) -> None:
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", encoding="utf-8"
        ) as handle:
            json.dump(manifest, handle)
            handle.flush()
            checker.validate_manifest(ROOT, Path(handle.name), require_ready)

    def assert_rejected(self, manifest: dict, message: str) -> None:
        with self.assertRaisesRegex(checker.CompatibilityError, message):
            self.check_copy(manifest)

    def test_current_release_ready_manifest_is_valid(self) -> None:
        checked = checker.validate_manifest(ROOT, self.manifest_path)
        self.assertEqual(checked["release"]["status"], "release_ready")
        self.assertEqual(
            checked["dependencies"]["dasobjectstore"]["minimum_version"],
            "0.145.4",
        )

    def test_blocked_manifest_cannot_be_promoted(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["release"]["status"] = "blocked"
        dependency = changed["dependencies"]["monas"]
        dependency["evidence_status"] = "blocked_behavioral_preflight"
        dependency["missing_required_capabilities"] = [
            "monas.pinakotheke-runtime-compatibility-health.v1"
        ]
        changed["blocking_gates"] = [
            {
                "id": "runtime",
                "owner": "monas",
                "state": "unresolved",
                "required_capability": "monas.pinakotheke-runtime-compatibility-health.v1",
                "resolution": "Publish compatible runtime health evidence.",
            }
        ]
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", encoding="utf-8"
        ) as handle:
            json.dump(changed, handle)
            handle.flush()
            with self.assertRaisesRegex(
                checker.CompatibilityError, "intentionally blocked"
            ):
                checker.validate_manifest(ROOT, Path(handle.name), require_ready=True)

    def test_unresolved_das_minimum_cannot_feed_packaging(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["dependencies"]["dasobjectstore"]["minimum_version"] = None
        with self.assertRaisesRegex(checker.CompatibilityError, "minimum is unresolved"):
            checker.verified_das_minimum(changed)

    def test_clean_verified_das_minimum_can_feed_packaging(self) -> None:
        changed = copy.deepcopy(self.manifest)
        dependency = changed["dependencies"]["dasobjectstore"]
        self.assertEqual(checker.verified_das_minimum(changed), "0.145.4")

    def test_rejects_unknown_schema_major(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["schema_version"] = "x-img.rc-compatibility.v2"
        self.assert_rejected(changed, "unsupported compatibility schema")

    def test_rejects_unknown_field(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["release"]["surprise"] = True
        self.assert_rejected(changed, "unknown surprise")

    def test_rejects_missing_capability_without_gate(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["release"]["status"] = "blocked"
        changed["dependencies"]["monas"]["evidence_status"] = (
            "blocked_behavioral_preflight"
        )
        changed["dependencies"]["monas"]["missing_required_capabilities"] = [
            "monas.pinakotheke-runtime-compatibility-health.v1"
        ]
        self.assert_rejected(changed, "missing capability lacks an unresolved gate")

    def test_rejects_false_verified_dependency(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["dependencies"]["dasobjectstore"]["working_tree"] = "dirty"
        self.assert_rejected(changed, "verified evidence must be clean and complete")

    def test_rejects_release_ready_with_unresolved_gate(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["blocking_gates"] = [
            {
                "id": "unexpected",
                "owner": "monas",
                "state": "unresolved",
                "required_capability": "monas.pinakotheke-runtime-compatibility-health.v1",
                "resolution": "This gate must block a release-ready record.",
            }
        ]
        self.assert_rejected(changed, "release-ready manifest has unresolved gates")

    def test_rejects_version_drift(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["pinakotheke"]["baseline_version"] = "1.29.0"
        self.assert_rejected(changed, "expected baseline 1.29.0")

    def test_rejects_wire_contract_drift(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["dependencies"]["monas"]["wire_contracts"][0]["sha256"] = "0" * 64
        self.assert_rejected(changed, "checksum is")

    def test_rejects_incorrect_firefox_rc_mapping(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["release"]["firefox_manifest_version"] = "1.29.0"
        self.assert_rejected(changed, "not the deterministic RC mapping")

    def test_rejects_firefox_version_name_drift(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["release"]["firefox_version_name"] = "1.29.0"
        self.assert_rejected(changed, "must preserve the semantic version")

    def test_native_package_prerelease_mapping_sorts_before_final(self) -> None:
        versions = checker.expected_package_versions("1.29.0-rc.1")
        self.assertEqual(versions["deb_version"], "1.29.0~rc.1")
        self.assertEqual(versions["rpm_version"], "1.29.0")
        self.assertEqual(versions["rpm_release"], "0.rc.1")
        self.assertEqual(versions["macos_pkg_version"], "1.29.0.1")

    def test_rejects_native_package_mapping_drift(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["release"]["deb_version"] = "1.29.0"
        self.assert_rejected(changed, "release.deb_version")

    def test_rejects_external_path_dependency(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "contracts/release").mkdir(parents=True)
            schema_source = (
                ROOT
                / "contracts/release/pinakotheke-rc-compatibility.v1.schema.json"
            )
            (root / "contracts/release/pinakotheke-rc-compatibility.v1.schema.json").write_bytes(
                schema_source.read_bytes()
            )
            (root / "Cargo.toml").write_text(
                '[workspace]\nmembers=[]\n[workspace.package]\nversion="1.28.0"\n'
                '[dependencies]\nmonas={path="../monas"}\n',
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                checker.CompatibilityError, "external path dependency"
            ):
                checker.validate_path_dependencies(root)


if __name__ == "__main__":
    unittest.main()
