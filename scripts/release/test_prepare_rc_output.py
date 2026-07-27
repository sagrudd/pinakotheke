#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Regression tests for the logically sealed RC release-output helper."""

from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("prepare_rc_output.py")
SPEC = importlib.util.spec_from_file_location("prepare_rc_output", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)


class ReleaseOutputTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.repository = self.root / "repository"
        self.repository.mkdir()
        (self.repository / "Cargo.toml").write_text(
            '[workspace]\nmembers = []\n\n'
            '[workspace.package]\nversion = "1.29.0-rc.1"\n',
            encoding="utf-8",
        )
        compatibility = self.repository / "contracts" / "release"
        compatibility.mkdir(parents=True)
        (compatibility / "compatibility-manifest.json").write_text(
            '{"schema_version":"pinakotheke.rc-compatibility.v1",'
            '"release":{"status":"release_ready","target_version":"1.29.0-rc.1",'
            '"deb_version":"1.29.0~rc.1","rpm_version":"1.29.0",'
            '"rpm_release":"0.rc.1","macos_pkg_version":"1.29.0.1",'
            '"firefox_manifest_version":"1.29.0.1",'
            '"firefox_version_name":"1.29.0-rc.1"},'
            '"dependencies":{"monas":{'
            '"tested_commit":"1111111111111111111111111111111111111111",'
            '"tested_version":"0.9.1","minimum_version":"0.9.1",'
            '"required_capabilities":["monas.runtime.v1"]}},'
            '"blocking_gates":[]}\n',
            encoding="utf-8",
        )
        checker = self.repository / "scripts" / "release"
        checker.mkdir(parents=True)
        (checker / "check_rc_compatibility.py").write_text(
            "#!/usr/bin/env python3\n"
            "import json,sys\n"
            "path=sys.argv[sys.argv.index('--manifest')+1]\n"
            "document=json.load(open(path, encoding='utf-8'))\n"
            "raise SystemExit(0 if document['release']['status']=='release_ready' "
            "and document['blocking_gates']==[] else 1)\n",
            encoding="utf-8",
        )
        self.git("init", "-q")
        self.git("config", "user.email", "release@example.invalid")
        self.git("config", "user.name", "Release Test")
        self.git("add", "Cargo.toml", "contracts", "scripts")
        self.git("commit", "-qm", "Initial")
        self.outputs = self.root / "outputs"

    def git(self, *arguments: str) -> str:
        result = subprocess.run(
            ["git", "-C", str(self.repository), *arguments],
            check=True,
            stdout=subprocess.PIPE,
            text=True,
        )
        return result.stdout.strip()

    def prepare(self) -> Path:
        return release.prepare(
            self.repository,
            self.outputs,
            "1.29.0-rc.1",
            ["make packages", "make firefox"],
            "contracts/release/compatibility-manifest.json",
            ["make quality", "make docs"],
        )

    def add_required_artifacts(self, directory: Path) -> None:
        for index, (kind, architecture) in enumerate(
            sorted(release.REQUIRED_ARTIFACT_MATRIX)
        ):
            path = f"artifact-{index}-{kind}-{architecture}"
            (directory / path).write_bytes(
                f"{kind}:{architecture}".encode("utf-8")
            )
            release.add_artifact(directory, path, kind, architecture)

    def test_prepare_records_clean_source_and_is_deterministic(self) -> None:
        directory = self.prepare()
        manifest_bytes = (directory / release.MANIFEST_NAME).read_bytes()
        manifest = json.loads(manifest_bytes)
        self.assertEqual(manifest["source"]["commit"], self.git("rev-parse", "HEAD"))
        self.assertTrue(manifest["source"]["clean"])
        self.assertEqual(manifest["release_version"], "1.29.0-rc.1")
        self.assertEqual(
            manifest["build_commands"], ["make packages", "make firefox"]
        )
        self.assertEqual(manifest["state"], "prepared")
        self.assertEqual(manifest["artifacts"], [])
        self.assertEqual(
            manifest["version_mirrors"],
            {
                "deb_version": "1.29.0~rc.1",
                "firefox_manifest_version": "1.29.0.1",
                "firefox_version_name": "1.29.0-rc.1",
                "macos_pkg_version": "1.29.0.1",
                "product_semver": "1.29.0-rc.1",
                "rpm_release": "0.rc.1",
                "rpm_version": "1.29.0",
            },
        )
        self.assertEqual(
            manifest["compatibility_manifest"]["path"],
            "contracts/release/compatibility-manifest.json",
        )
        self.assertEqual(
            manifest["dependencies"]["monas"]["tested_commit"],
            "1" * 40,
        )
        self.assertEqual(
            [entry["command"] for entry in manifest["local_test_evidence"]],
            ["make docs", "make quality"],
        )
        release.verify(directory)
        self.assertEqual(
            manifest_bytes,
            (directory / release.MANIFEST_NAME).read_bytes(),
        )

    def test_dirty_source_and_version_mismatch_are_rejected(self) -> None:
        (self.repository / "untracked").write_text("dirty", encoding="utf-8")
        with self.assertRaisesRegex(release.ReleaseOutputError, "dirty"):
            self.prepare()
        (self.repository / "untracked").unlink()
        with self.assertRaisesRegex(release.ReleaseOutputError, "does not match"):
            release.prepare(
                self.repository,
                self.outputs,
                "1.29.0-rc.2",
                ["make packages"],
                "contracts/release/compatibility-manifest.json",
                ["make quality"],
            )

    def test_blocked_compatibility_cannot_seed_release_output(self) -> None:
        compatibility = (
            self.repository / "contracts" / "release" / "compatibility-manifest.json"
        )
        compatibility.write_text(
            '{"schema_version":"pinakotheke.rc-compatibility.v1",'
            '"release":{"status":"blocked"},"blocking_gates":[{"id":"gate"}]}\n',
            encoding="utf-8",
        )
        self.git("add", str(compatibility))
        self.git("commit", "-qm", "Block compatibility")
        with self.assertRaisesRegex(release.ReleaseOutputError, "blocked"):
            self.prepare()

    def test_existing_empty_or_nonempty_target_is_never_reused(self) -> None:
        target = self.outputs / "1.29.0-rc.1"
        target.mkdir(parents=True)
        with self.assertRaisesRegex(release.ReleaseOutputError, "will not be reused"):
            self.prepare()
        (target / "historic.deb").write_bytes(b"historic")
        with self.assertRaisesRegex(release.ReleaseOutputError, "will not be reused"):
            self.prepare()

    def test_output_root_symlink_is_rejected(self) -> None:
        actual = self.root / "actual"
        actual.mkdir()
        self.outputs.symlink_to(actual, target_is_directory=True)
        with self.assertRaisesRegex(release.ReleaseOutputError, "symbolic link"):
            self.prepare()

    def test_artifacts_are_sorted_checked_and_sealed(self) -> None:
        directory = self.prepare()
        self.add_required_artifacts(directory)
        release.seal(directory, self.repository)

        manifest = release.verify(directory, require_sealed=True)
        self.assertEqual(
            [artifact["path"] for artifact in manifest["artifacts"]],
            sorted(artifact["path"] for artifact in manifest["artifacts"]),
        )
        self.assertEqual(manifest["state"], "sealed")
        with self.assertRaisesRegex(
            release.ReleaseOutputError, "cannot accept another"
        ):
            release.add_artifact(
                directory,
                manifest["artifacts"][0]["path"],
                manifest["artifacts"][0]["kind"],
                manifest["artifacts"][0]["architecture"],
            )

    def test_checksum_tampering_and_unlisted_files_are_rejected(self) -> None:
        directory = self.prepare()
        artifact = directory / "pinakotheke.deb"
        artifact.write_bytes(b"package")
        release.add_artifact(
            directory, "pinakotheke.deb", "deb", "x86_64"
        )
        release.verify(directory)
        artifact.write_bytes(b"changed")
        with self.assertRaisesRegex(
            release.ReleaseOutputError, "size mismatch|checksum mismatch"
        ):
            release.verify(directory)
        artifact.write_bytes(b"package")
        (directory / "unlisted.rpm").write_bytes(b"not recorded")
        with self.assertRaisesRegex(release.ReleaseOutputError, "unlisted"):
            release.verify(directory)

    def test_artifact_escape_and_symlink_are_rejected(self) -> None:
        directory = self.prepare()
        outside = self.root / "outside.pkg"
        outside.write_bytes(b"outside")
        with self.assertRaisesRegex(release.ReleaseOutputError, "normalized"):
            release.add_artifact(
                directory, "../outside.pkg", "macos-pkg", "arm64"
            )
        link = directory / "linked.pkg"
        link.symlink_to(outside)
        with self.assertRaisesRegex(release.ReleaseOutputError, "symbolic link"):
            release.add_artifact(directory, "linked.pkg", "macos-pkg", "arm64")

    def test_seal_requires_artifacts_and_exact_inventory(self) -> None:
        directory = self.prepare()
        with self.assertRaisesRegex(release.ReleaseOutputError, "without artifacts"):
            release.seal(directory, self.repository)
        (directory / "unlisted").write_bytes(b"unexpected")
        with self.assertRaisesRegex(release.ReleaseOutputError, "unlisted"):
            release.verify(directory)

    def test_seal_rejects_incomplete_or_duplicate_artifact_matrix(self) -> None:
        directory = self.prepare()
        (directory / "only.deb").write_bytes(b"package")
        release.add_artifact(directory, "only.deb", "deb", "x86_64")
        with self.assertRaisesRegex(release.ReleaseOutputError, "matrix mismatch"):
            release.seal(directory, self.repository)

    def test_manifest_unknown_fields_and_directory_version_are_rejected(
        self,
    ) -> None:
        directory = self.prepare()
        manifest_path = directory / release.MANIFEST_NAME
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["unexpected"] = True
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
        with self.assertRaisesRegex(release.ReleaseOutputError, "unknown"):
            release.verify(directory)

        manifest.pop("unexpected")
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
        moved = directory.with_name("wrong-version")
        directory.rename(moved)
        with self.assertRaisesRegex(release.ReleaseOutputError, "directory name"):
            release.verify(moved)

    def test_version_mirror_drift_is_rejected(self) -> None:
        directory = self.prepare()
        manifest_path = directory / release.MANIFEST_NAME
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["version_mirrors"]["product_semver"] = "1.29.0"
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
        with self.assertRaisesRegex(release.ReleaseOutputError, "product version mirror"):
            release.verify(directory)

    def test_malformed_artifact_types_fail_closed(self) -> None:
        directory = self.prepare()
        manifest_path = directory / release.MANIFEST_NAME
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["artifacts"] = [
            {
                "architecture": "x86_64",
                "kind": "deb",
                "path": 7,
                "sha256": "a" * 64,
                "size_bytes": 1,
            }
        ]
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
        with self.assertRaisesRegex(release.ReleaseOutputError, "path.*string"):
            release.verify(directory)

    def test_unknown_artifact_kind_and_architecture_are_rejected(self) -> None:
        directory = self.prepare()
        (directory / "artifact").write_bytes(b"payload")
        with self.assertRaisesRegex(release.ReleaseOutputError, "kind"):
            release.add_artifact(directory, "artifact", "installer", "x86_64")
        with self.assertRaisesRegex(release.ReleaseOutputError, "architecture"):
            release.add_artifact(directory, "artifact", "deb", "mips64")

    @unittest.skipUnless(hasattr(os, "mkfifo"), "FIFO requires POSIX")
    def test_special_files_are_rejected(self) -> None:
        directory = self.prepare()
        os.mkfifo(directory / "unexpected.fifo")
        with self.assertRaisesRegex(release.ReleaseOutputError, "special"):
            release.verify(directory)

    def test_seal_rechecks_clean_head_and_compatibility_manifest(self) -> None:
        directory = self.prepare()
        self.add_required_artifacts(directory)
        (self.repository / "dirty").write_text("change", encoding="utf-8")
        with self.assertRaisesRegex(release.ReleaseOutputError, "dirty"):
            release.seal(directory, self.repository)
        (self.repository / "dirty").unlink()
        compatibility = (
            self.repository / "contracts" / "release" / "compatibility-manifest.json"
        )
        compatibility.write_text('{"changed":true}\n', encoding="utf-8")
        self.git("add", str(compatibility))
        self.git("commit", "-qm", "Change compatibility")
        with self.assertRaisesRegex(release.ReleaseOutputError, "HEAD changed"):
            release.seal(directory, self.repository)

    def test_symlinked_output_ancestor_is_rejected(self) -> None:
        real_parent = self.root / "real-parent"
        real_parent.mkdir()
        linked_parent = self.root / "linked-parent"
        linked_parent.symlink_to(real_parent, target_is_directory=True)
        with self.assertRaisesRegex(release.ReleaseOutputError, "traverses"):
            release.prepare(
                self.repository,
                linked_parent / "outputs",
                "1.29.0-rc.1",
                ["make packages"],
                "contracts/release/compatibility-manifest.json",
                ["make quality"],
            )


if __name__ == "__main__":
    unittest.main()
