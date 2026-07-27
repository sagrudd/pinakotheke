#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Synthetic, isolated acceptance proof for RC-B storage authority convergence.

The harness deliberately models metadata and object references only.  It never
creates media payloads, contacts a provider, or needs credentials.  Each case
uses a generated temporary prefix so it cannot inspect or alter a user gallery.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE = ROOT / "fixtures/storage-authority/v1/rc-b-cases.json"
SCHEMA_VERSION = "x-img.storage-authority-acceptance.v1"
SHA256 = re.compile(r"^[0-9a-f]{64}$")
KEY = re.compile(r"^[a-z0-9][a-z0-9._/-]*[a-z0-9]$")


class AcceptanceError(ValueError):
    """A fixture or simulated authority transition is invalid."""


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise AcceptanceError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_fixture(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"), object_pairs_hook=reject_duplicate_keys
        )
    except (OSError, json.JSONDecodeError) as error:
        raise AcceptanceError(f"cannot read fixture {path}: {error}") from error
    if not isinstance(document, dict):
        raise AcceptanceError("fixture must be an object")
    expected = {"schema_version", "fixture_kind", "generated_prefix", "capacity_limit_bytes", "cases"}
    if set(document) != expected:
        raise AcceptanceError("fixture has missing or unknown fields")
    if document["schema_version"] != SCHEMA_VERSION:
        raise AcceptanceError("fixture has an unsupported schema version")
    if document["fixture_kind"] != "synthetic":
        raise AcceptanceError("fixture_kind must be synthetic")
    prefix = document["generated_prefix"]
    if not isinstance(prefix, str) or not KEY.fullmatch(prefix) or prefix.startswith("/"):
        raise AcceptanceError("generated_prefix must be a relative safe object prefix")
    if not isinstance(document["capacity_limit_bytes"], int) or document["capacity_limit_bytes"] < 1:
        raise AcceptanceError("capacity_limit_bytes must be a positive integer")
    cases = document["cases"]
    if not isinstance(cases, list) or not cases:
        raise AcceptanceError("cases must be a non-empty array")
    identifiers: set[str] = set()
    for case in cases:
        validate_case(case, prefix, identifiers)
    return document


def require_string(value: Any, location: str) -> str:
    if not isinstance(value, str) or not value:
        raise AcceptanceError(f"{location} must be a non-empty string")
    return value


def validate_object_fields(action: dict[str, Any], prefix: str, location: str) -> None:
    required = {"kind", "canonical_id", "object_key", "sha256", "size_bytes"}
    if set(action) != required:
        raise AcceptanceError(f"{location} has invalid object action fields")
    require_string(action["canonical_id"], f"{location}.canonical_id")
    key = require_string(action["object_key"], f"{location}.object_key")
    if not KEY.fullmatch(key) or not key.startswith(prefix + "/"):
        raise AcceptanceError(f"{location}.object_key must be below generated_prefix")
    if not isinstance(action["sha256"], str) or not SHA256.fullmatch(action["sha256"]):
        raise AcceptanceError(f"{location}.sha256 must be a lower-case SHA-256")
    if not isinstance(action["size_bytes"], int) or action["size_bytes"] < 1:
        raise AcceptanceError(f"{location}.size_bytes must be a positive integer")


def validate_case(case: Any, prefix: str, identifiers: set[str]) -> None:
    if not isinstance(case, dict) or set(case) != {"id", "actions", "expected"}:
        raise AcceptanceError("each case must contain exactly id, actions, expected")
    identifier = require_string(case["id"], "case.id")
    if identifier in identifiers:
        raise AcceptanceError(f"duplicate case id: {identifier}")
    identifiers.add(identifier)
    actions = case["actions"]
    if not isinstance(actions, list) or not actions:
        raise AcceptanceError(f"case {identifier} has no actions")
    for index, raw in enumerate(actions):
        location = f"case {identifier} action {index}"
        if not isinstance(raw, dict) or not isinstance(raw.get("kind"), str):
            raise AcceptanceError(f"{location} must name an action kind")
        kind = raw["kind"]
        if kind in {"ingest", "retry_ingest", "crash_after_provider", "seed_orphan"}:
            validate_object_fields(raw, prefix, location)
        elif kind in {"delete", "retry_delete", "remove_provider_direct"}:
            if set(raw) != {"kind", "object_key"}:
                raise AcceptanceError(f"{location} has invalid delete fields")
            key = require_string(raw["object_key"], f"{location}.object_key")
            if not KEY.fullmatch(key) or not key.startswith(prefix + "/"):
                raise AcceptanceError(f"{location}.object_key must be below generated_prefix")
        elif kind == "seed_stale_projection":
            if set(raw) != {"kind", "object_key"}:
                raise AcceptanceError(f"{location} has invalid stale projection fields")
            key = require_string(raw["object_key"], f"{location}.object_key")
            if not KEY.fullmatch(key) or not key.startswith(prefix + "/"):
                raise AcceptanceError(f"{location}.object_key must be below generated_prefix")
        elif kind in {"repair", "dry_run_reconcile"}:
            if set(raw) != {"kind"}:
                raise AcceptanceError(f"{location} must not have extra fields")
        else:
            raise AcceptanceError(f"{location} has unsupported kind {kind!r}")
    expected = case["expected"]
    expected_keys = {"provider_keys", "catalogue_keys", "projection_keys", "capacity_used_bytes", "pending_keys", "diagnostics"}
    if not isinstance(expected, dict) or set(expected) != expected_keys:
        raise AcceptanceError(f"case {identifier} expected has missing or unknown fields")
    for name in {"provider_keys", "catalogue_keys", "projection_keys", "pending_keys", "diagnostics"}:
        value = expected[name]
        if not isinstance(value, list) or any(not isinstance(item, str) for item in value):
            raise AcceptanceError(f"case {identifier} expected.{name} must be a string array")
        if value != sorted(value) or len(value) != len(set(value)):
            raise AcceptanceError(f"case {identifier} expected.{name} must be sorted and unique")
    if not isinstance(expected["capacity_used_bytes"], int) or expected["capacity_used_bytes"] < 0:
        raise AcceptanceError(f"case {identifier} expected.capacity_used_bytes must be non-negative")


@dataclass(frozen=True)
class ObjectRecord:
    canonical_id: str
    object_key: str
    sha256: str
    size_bytes: int


@dataclass
class State:
    capacity_limit_bytes: int
    provider: dict[str, ObjectRecord] = field(default_factory=dict)
    catalogue: dict[str, ObjectRecord] = field(default_factory=dict)
    projection: set[str] = field(default_factory=set)
    pending: dict[str, ObjectRecord] = field(default_factory=dict)
    canonical_index: dict[str, str] = field(default_factory=dict)
    capacity_used_bytes: int = 0
    diagnostics: list[str] = field(default_factory=list)
    audit: list[str] = field(default_factory=list)

    def record(self, action: dict[str, Any]) -> ObjectRecord:
        return ObjectRecord(
            canonical_id=action["canonical_id"],
            object_key=action["object_key"],
            sha256=action["sha256"],
            size_bytes=action["size_bytes"],
        )

    def ingest(self, action: dict[str, Any]) -> None:
        record = self.record(action)
        existing_key = self.canonical_index.get(record.canonical_id)
        if existing_key is not None:
            existing = self.catalogue.get(existing_key)
            if existing == record:
                self.audit.append("idempotent_ingest")
                return
            raise AcceptanceError("canonical id retry conflicts with settled authority record")
        if record.object_key in self.provider or record.object_key in self.catalogue:
            raise AcceptanceError("object key is already authoritative")
        if self.capacity_used_bytes + record.size_bytes > self.capacity_limit_bytes:
            raise AcceptanceError("authoritative capacity limit would be exceeded")
        # The simulated normal path represents one authority-mediated receipt:
        # provider settlement, catalogue admission, capacity debit, and projection
        # update happen as one visible operation.
        self.provider[record.object_key] = record
        self.catalogue[record.object_key] = record
        self.projection.add(record.object_key)
        self.canonical_index[record.canonical_id] = record.object_key
        self.capacity_used_bytes += record.size_bytes
        self.audit.append("ingest_settled")

    def crash_after_provider(self, action: dict[str, Any]) -> None:
        record = self.record(action)
        if record.object_key in self.provider:
            raise AcceptanceError("crash seed reuses a provider key")
        self.provider[record.object_key] = record
        self.pending[record.object_key] = record
        self.audit.append("crash_after_provider")

    def delete(self, key: str) -> None:
        record = self.catalogue.pop(key, None)
        if record is None:
            self.projection.discard(key)
            self.audit.append("idempotent_delete")
            return
        # Catalogue withdrawal and capacity release are authoritative even when
        # the provider has already removed the object.
        self.capacity_used_bytes -= record.size_bytes
        self.canonical_index.pop(record.canonical_id, None)
        self.projection.discard(key)
        self.pending.pop(key, None)
        self.provider.pop(key, None)
        self.audit.append("delete_settled")

    def reconcile(self, dry_run: bool) -> None:
        diagnostics: list[str] = []
        for key in sorted(self.provider):
            if key not in self.catalogue and key not in self.pending:
                diagnostics.append(f"orphan_provider:{key}")
        for key in sorted(self.catalogue):
            if key not in self.provider:
                diagnostics.append(f"missing_provider:{key}")
        for key in sorted(self.projection):
            if key not in self.catalogue:
                diagnostics.append(f"stale_projection:{key}")
        self.diagnostics = diagnostics
        if dry_run:
            self.audit.append("reconcile_dry_run")
            return
        # Complete crash-interrupted receipts before any cleanup.
        for key, record in sorted(tuple(self.pending.items())):
            if key in self.provider and key not in self.catalogue:
                if self.capacity_used_bytes + record.size_bytes > self.capacity_limit_bytes:
                    raise AcceptanceError("repair exceeds authoritative capacity")
                self.catalogue[key] = record
                self.projection.add(key)
                self.canonical_index[record.canonical_id] = key
                self.capacity_used_bytes += record.size_bytes
            self.pending.pop(key, None)
        # A provider object absent from the catalogue is never admitted by a
        # rebuildable projection; remove only this synthetic generated-prefix
        # orphan.  A provider-missing catalogue record is authoritatively
        # withdrawn with its capacity debit.
        for diagnostic in diagnostics:
            kind, key = diagnostic.split(":", 1)
            if kind == "orphan_provider":
                self.provider.pop(key, None)
            elif kind == "missing_provider":
                self.delete(key)
            elif kind == "stale_projection":
                self.projection.discard(key)
        self.diagnostics = []
        self.audit.append("reconcile_repaired")

    def snapshot(self) -> dict[str, Any]:
        return {
            "provider_keys": sorted(self.provider),
            "catalogue_keys": sorted(self.catalogue),
            "projection_keys": sorted(self.projection),
            "capacity_used_bytes": self.capacity_used_bytes,
            "pending_keys": sorted(self.pending),
            "diagnostics": self.diagnostics,
        }


def execute_case(case: dict[str, Any], capacity_limit_bytes: int, generated_prefix: str) -> tuple[dict[str, Any], list[str]]:
    """Execute one metadata-only case in a fresh generated filesystem prefix."""
    prefix_root = Path(tempfile.mkdtemp(prefix="pinakotheke-rc-b-"))
    try:
        isolated_prefix = prefix_root / generated_prefix
        isolated_prefix.mkdir(parents=True)
        state = State(capacity_limit_bytes=capacity_limit_bytes)
        for action in case["actions"]:
            kind = action["kind"]
            if kind in {"ingest", "retry_ingest"}:
                state.ingest(action)
            elif kind == "crash_after_provider":
                state.crash_after_provider(action)
            elif kind in {"delete", "retry_delete"}:
                state.delete(action["object_key"])
            elif kind == "remove_provider_direct":
                state.provider.pop(action["object_key"], None)
                state.audit.append("provider_absent")
            elif kind == "seed_orphan":
                record = state.record(action)
                state.provider[record.object_key] = record
            elif kind == "seed_stale_projection":
                state.projection.add(action["object_key"])
            elif kind == "dry_run_reconcile":
                state.reconcile(dry_run=True)
            elif kind == "repair":
                state.reconcile(dry_run=False)
            else:  # Fixture validation makes this defensive branch unreachable.
                raise AcceptanceError(f"unsupported action {kind!r}")
        return state.snapshot(), state.audit
    finally:
        shutil.rmtree(prefix_root)


def run_fixture(path: Path, selected_ids: set[str] | None = None) -> list[dict[str, Any]]:
    document = load_fixture(path)
    results: list[dict[str, Any]] = []
    found: set[str] = set()
    for case in document["cases"]:
        if selected_ids is not None and case["id"] not in selected_ids:
            continue
        found.add(case["id"])
        actual, audit = execute_case(
            case, document["capacity_limit_bytes"], document["generated_prefix"]
        )
        if actual != case["expected"]:
            raise AcceptanceError(
                f"case {case['id']} did not converge: expected {case['expected']!r}, got {actual!r}"
            )
        results.append({"id": case["id"], "state": actual, "audit": audit})
    if selected_ids is not None and found != selected_ids:
        raise AcceptanceError(f"unknown case ids: {', '.join(sorted(selected_ids - found))}")
    return results


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument("--case", action="append", dest="cases", help="run one fixture case; repeatable")
    parser.add_argument("--json", action="store_true", help="emit only deterministic JSON results")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    try:
        results = run_fixture(args.fixture, set(args.cases) if args.cases else None)
    except AcceptanceError as error:
        print(f"storage-authority acceptance failed: {error}", file=sys.stderr)
        return 1
    if args.json:
        print(json.dumps(results, sort_keys=True, separators=(",", ":")))
    else:
        for result in results:
            print(f"storage-authority case passed: {result['id']}")
        print(f"storage-authority acceptance passed: {len(results)} synthetic cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
