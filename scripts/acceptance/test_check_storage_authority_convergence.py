#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Focused regression tests for the synthetic RC-B authority acceptance harness."""

from __future__ import annotations

import copy
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts/acceptance/check_storage_authority_convergence.py"
SPEC = importlib.util.spec_from_file_location("storage_authority_acceptance", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
checker = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = checker
SPEC.loader.exec_module(checker)


class StorageAuthorityAcceptanceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture_path = ROOT / "fixtures/storage-authority/v1/rc-b-cases.json"
        cls.fixture = json.loads(cls.fixture_path.read_text(encoding="utf-8"))

    def fixture_copy(self, document: dict) -> Path:
        handle = tempfile.NamedTemporaryFile(mode="w", suffix=".json", encoding="utf-8", delete=False)
        self.addCleanup(lambda: Path(handle.name).unlink(missing_ok=True))
        json.dump(document, handle)
        handle.close()
        return Path(handle.name)

    def test_all_synthetic_cases_converge(self) -> None:
        results = checker.run_fixture(self.fixture_path)
        self.assertEqual([result["id"] for result in results], [
            "authority-mediated-ingest",
            "idempotent-ingest-retry",
            "crash-repair-before-admission",
            "provider-absent-authoritative-delete-retry",
            "dry-run-stale-and-orphan-detection",
        ])

    def test_ingest_is_catalogued_projected_and_capacity_accounted_once(self) -> None:
        result = checker.run_fixture(self.fixture_path, {"idempotent-ingest-retry"})[0]
        self.assertEqual(result["state"]["capacity_used_bytes"], 700)
        self.assertEqual(len(result["state"]["catalogue_keys"]), 1)
        self.assertIn("idempotent_ingest", result["audit"])

    def test_crash_repair_has_no_projection_before_settlement(self) -> None:
        case = copy.deepcopy(self.fixture["cases"][2])
        partial = {"id": "partial", "actions": case["actions"][:1], "expected": {"provider_keys": ["rc-b/generated/videos/synthetic-video-002.bin"], "catalogue_keys": [], "projection_keys": [], "capacity_used_bytes": 0, "pending_keys": ["rc-b/generated/videos/synthetic-video-002.bin"], "diagnostics": []}}
        actual, _ = checker.execute_case(partial, self.fixture["capacity_limit_bytes"], self.fixture["generated_prefix"])
        self.assertEqual(actual, partial["expected"])
        repaired = checker.run_fixture(self.fixture_path, {"crash-repair-before-admission"})[0]
        self.assertEqual(repaired["state"]["capacity_used_bytes"], 4096)

    def test_provider_absence_does_not_prevent_authoritative_delete_or_retry(self) -> None:
        result = checker.run_fixture(self.fixture_path, {"provider-absent-authoritative-delete-retry"})[0]
        self.assertEqual(result["state"]["capacity_used_bytes"], 0)
        self.assertEqual(result["state"]["catalogue_keys"], [])
        self.assertIn("provider_absent", result["audit"])
        self.assertIn("idempotent_delete", result["audit"])

    def test_dry_run_detects_without_repairing(self) -> None:
        result = checker.run_fixture(self.fixture_path, {"dry-run-stale-and-orphan-detection"})[0]
        self.assertEqual(result["state"]["provider_keys"], ["rc-b/generated/orphans/synthetic-orphan-001.bin"])
        self.assertEqual(result["state"]["projection_keys"], ["rc-b/generated/stale/synthetic-stale-001.bin"])
        self.assertEqual(len(result["state"]["diagnostics"]), 2)
        self.assertIn("reconcile_dry_run", result["audit"])

    def test_future_schema_and_non_synthetic_fixture_are_rejected(self) -> None:
        future = copy.deepcopy(self.fixture)
        future["schema_version"] = "pinakotheke.storage-authority-acceptance.v2"
        with self.assertRaisesRegex(checker.AcceptanceError, "unsupported schema"):
            checker.load_fixture(self.fixture_copy(future))
        non_synthetic = copy.deepcopy(self.fixture)
        non_synthetic["fixture_kind"] = "live"
        with self.assertRaisesRegex(checker.AcceptanceError, "must be synthetic"):
            checker.load_fixture(self.fixture_copy(non_synthetic))

    def test_generated_prefix_prevents_escape(self) -> None:
        escaped = copy.deepcopy(self.fixture)
        escaped["cases"][0]["actions"][0]["object_key"] = "outside/image.bin"
        with self.assertRaisesRegex(checker.AcceptanceError, "below generated_prefix"):
            checker.load_fixture(self.fixture_copy(escaped))


if __name__ == "__main__":
    unittest.main()
