#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Tests for the coordinated Pinakotheke cutover gate."""

import importlib.util
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check_v1_cutover.py")
SPEC = importlib.util.spec_from_file_location("check_v1_cutover", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class CutoverGateTests(unittest.TestCase):
    def test_matrix_has_exact_required_inventory(self) -> None:
        self.assertEqual(MODULE.validate_matrix(), [])

    def test_release_candidate_is_not_misrepresented_as_cutover_ready(self) -> None:
        checks = MODULE.current_checks(github=False)
        self.assertTrue(any(not check.ready for check in checks))
        self.assertTrue(next(check for check in checks if check.surface == "legacy-schemas").ready)


if __name__ == "__main__":
    unittest.main()
