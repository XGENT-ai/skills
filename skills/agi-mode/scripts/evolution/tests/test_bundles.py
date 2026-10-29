import json
from copy import deepcopy
import os
from pathlib import Path
import shutil
import subprocess
from unittest.mock import patch

from support import JobFixture
from protocol import ProtocolError, prepare, candidate_from_patch, bundle_hash
from evolution import ConflictError, freeze, load_run
from test_quality import QualityFixture


class CandidateTests(JobFixture):
    def proposal(self, job, path, content):
        import difflib

        parent = job.bundles["baseline"]
        proposal = {
            "parent_hash": bundle_hash(parent),
            "hypothesis": "Clarify test timing",
            "expected_effect": "Fewer missed tests",
            "counterexample": "A missed test",
            "changed_paths": [path],
        }
        patch = "".join(
            difflib.unified_diff(
                parent[path]["content"].decode().splitlines(True),
                content.splitlines(True),
                fromfile="a/" + path,
                tofile="b/" + path,
            )
        ).encode()
        return parent, proposal, patch

    def test_general_candidate_preserves_parent_and_protected_rule(self):
        job = prepare(self.write())
        parent, proposal, patch = self.proposal(
            job, "AGENTS.md", "Preserve authorization.\nRun tests before delivery.\n"
        )
        candidate = candidate_from_patch(job, parent, proposal, patch)
        self.assertIn(b"before delivery", candidate["AGENTS.md"]["content"])
        self.assertEqual(parent, job.bundles["baseline"])
        self.assertEqual(
            (self.repo / "AGENTS.md").read_bytes(), parent["AGENTS.md"]["content"]
        )

    def test_parent_scope_actual_changes_and_protected_rules_are_checked(self):
        job = prepare(self.write())
        for path, content, mutation in [
            ("AGENTS.md", "Preserve authorization.\nClear tests.\n", "parent"),
            ("src/app.py", "print('outside target')\n", "scope"),
            ("AGENTS.md", "Remove the old authorization rule.\n", "protected"),
        ]:
            parent, proposal, patch = self.proposal(job, path, content)
            if mutation == "parent":
                proposal["parent_hash"] = "a" * 64
            with self.subTest(mutation=mutation), self.assertRaises(ProtocolError):
                candidate_from_patch(job, parent, proposal, patch)

    def test_task_ids_and_known_reference_answers_cannot_enter_candidate(self):
        job = prepare(self.write())
        answer = job.case_files["search-case"]["oracle_paths"]["solution/solve.sh"][
            "content"
        ].decode()
        for leaked in ["search-case", answer]:
            parent, proposal, patch = self.proposal(
                job, "AGENTS.md", "Preserve authorization.\n" + leaked + "\n"
            )
            with self.subTest(leaked=leaked), self.assertRaises(ProtocolError):
                candidate_from_patch(job, parent, proposal, patch)

    def test_skill_invocation_name_is_unchanged(self):
        job = prepare(self.write())
        path = "skills/debugging/SKILL.md"
        job.spec["target"]["allowed_paths"] = [path]
        parent, proposal, patch = self.proposal(
            job,
            path,
            "---\nname: renamed\ndescription: Diagnose bugs\n---\nFind the root cause.\n",
        )
        with self.assertRaises(ProtocolError):
            candidate_from_patch(job, parent, proposal, patch)


class QualityCandidateTests(QualityFixture):
    def candidate(self, *, continuous=False):
        evaluation = self.quality_spec()
        if continuous:
            from protocol import canonical
            from test_quality import continuous_rubric, continuous_calibration
            (evaluation / "rubric.json").write_bytes(canonical(continuous_rubric()))
            path = evaluation / "calibration.json"
            path.write_bytes(canonical(continuous_calibration(json.loads(path.read_bytes()))))
        job = prepare(self.write())
        parent, proposal, patch = CandidateTests.proposal(
            self, job, "AGENTS.md", "Preserve authorization.\nRun tests before delivery.\n")
        proposal["mechanism"] = {"criterion_id": "regression", "patch_sha256": "a" * 64,
            "cause": "The observed repair has no effective regression assertion.",
            "expected_criteria": ["regression"], "regression_risk": "A narrow test could miss the boundary."}
        record = {"schema_version": 2, "purpose": "task", "split": "search", "case_id": "search-case",
            "candidate_hash": bundle_hash(parent), "patch_sha256": "a" * 64, "status": "ok",
            "task_success": 1, "quality_status": "valid", "failed_gates": [], "unknown_criteria": [],
            "quality_components": {"regression": {"status": "valid", "level": 0, "gate": False}}}
        return job, parent, proposal, patch, record

    def test_one_mechanism_is_tied_to_an_observed_parent_search_weakness(self):
        job, parent, proposal, patch, record = self.candidate()
        result = candidate_from_patch(job, parent, proposal, patch, search_records=[record])
        self.assertIn(b"before delivery", result["AGENTS.md"]["content"])
        self.assertEqual(parent, job.bundles["baseline"])

    def test_measured_correctness_failure_can_support_a_hypothesis(self):
        job, parent, proposal, patch, record = self.candidate()
        proposal["mechanism"].update(criterion_id="correctness", expected_criteria=["correctness"],
                                      cause="The measured artifact fails the frozen correctness gate.")
        record.update(task_success=0, failed_gates=["correctness"])
        record["quality_components"]["correctness"] = {"status": "valid", "level": 0, "gate": True}
        candidate_from_patch(job, parent, proposal, patch, search_records=[record])

    def test_continuous_partial_scores_support_only_current_measured_unsaturated_weaknesses(self):
        job, parent, proposal, patch, record = self.candidate(continuous=True)
        for level in (0, 0.5, 0.8342, 0.9999):
            observed = deepcopy(record)
            observed["quality_components"]["regression"]["level"] = level
            with self.subTest(supported=level):
                candidate = candidate_from_patch(job, parent, proposal, patch, search_records=[observed])
                self.assertIn(b"before delivery", candidate["AGENTS.md"]["content"])
        for level in (1, -0.01, 1.01, 83.42, True, None, float("nan")):
            observed = deepcopy(record)
            observed["quality_components"]["regression"]["level"] = level
            with self.subTest(rejected=level), self.assertRaises(ProtocolError):
                candidate_from_patch(job, parent, proposal, patch, search_records=[observed])
        for change in ({"split": "confirmation"}, {"candidate_hash": "f" * 64},
                       {"patch_sha256": "f" * 64}, {"quality_status": "unknown"}):
            observed = deepcopy(record)
            observed["quality_components"]["regression"]["level"] = 0.8342
            observed.update(change)
            with self.subTest(change=change), self.assertRaises(ProtocolError):
                candidate_from_patch(job, parent, proposal, patch, search_records=[observed])

    def test_legacy_anchor_protocol_does_not_accept_an_undeclared_continuous_level(self):
        job, parent, proposal, patch, record = self.candidate()
        record["quality_components"]["regression"]["level"] = 0.8342
        with self.assertRaises(ProtocolError):
            candidate_from_patch(job, parent, proposal, patch, search_records=[record])

    def test_multiple_mechanisms_missing_risk_and_unknown_targets_are_rejected(self):
        job, parent, proposal, patch, record = self.candidate()
        for mutate in [lambda p: p.update(mechanism=[p["mechanism"], p["mechanism"]]),
                       lambda p: p["mechanism"].pop("regression_risk"),
                       lambda p: p["mechanism"].update(criterion_id="private-heldout", expected_criteria=["private-heldout"]),
                       lambda p: p["mechanism"].update(expected_criteria=["boundary"]),
                       lambda p: p["mechanism"].update(cause="")]:
            changed = deepcopy(proposal)
            mutate(changed)
            with self.assertRaises(ProtocolError):
                candidate_from_patch(job, parent, changed, patch, search_records=[record])

    def test_confirmation_other_parent_and_unknown_or_saturated_quality_are_not_support(self):
        job, parent, proposal, patch, record = self.candidate()
        for mutate in [lambda r: r.update(split="confirmation"),
                       lambda r: r.update(candidate_hash="f" * 64),
                       lambda r: r.update(quality_status="unknown"),
                       lambda r: r["quality_components"]["regression"].update(level=1)]:
            changed = deepcopy(record)
            mutate(changed)
            with self.assertRaises(ProtocolError):
                candidate_from_patch(job, parent, proposal, patch, search_records=[changed])
        with self.assertRaises(ProtocolError):
            candidate_from_patch(job, parent, proposal, patch)


class BundleTests(JobFixture):
    def test_allowlist_includes_dirty_and_explicit_new_bytes_without_history(self):
        self.put("src/app.py", "print('dirty')\n")
        self.put("src/new.py", "print('new')\n")
        self.put("src/unauthorized.py", "private")
        self.spec["source"]["new_paths"] = ["src/new.py"]
        job = prepare(self.write())
        bundle = job.bundles["baseline"]
        self.assertEqual(bundle["src/app.py"]["content"], b"print('dirty')\n")
        self.assertIn("src/new.py", bundle)
        self.assertNotIn("src/unauthorized.py", bundle)
        self.assertFalse(any(".git" in Path(name).parts for name in bundle))

    def test_new_files_require_both_source_and_export_authority(self):
        self.put("secret.txt", "not authorized")
        self.spec["source"]["new_paths"] = ["secret.txt"]
        with self.assertRaises(ProtocolError):
            prepare(self.write())
        self.spec["source"]["new_paths"] = ["../outside"]
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_symlinks_directories_traversal_and_special_files_fail(self):
        self.put("src/app.py", "dirty")
        (self.repo / "src/app.py").unlink()
        outside = self.root / "outside"
        outside.write_text("secret")
        (self.repo / "src/app.py").symlink_to(outside)
        with self.assertRaises(ProtocolError):
            prepare(self.write())
        (self.repo / "src/app.py").unlink()
        os.mkfifo(self.repo / "src/app.py")
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_host_configs_and_git_objects_are_rejected_even_when_authorized(self):
        for name in [
            ".git/config",
            ".codex/config.toml",
            ".codex/auth.json",
            ".env",
            ".mcp.json",
            ".claude/settings.json",
            ".codex/sessions/log.jsonl",
            ".agents/plugins/cache/file",
        ]:
            spec = self.changed(["source", "paths"], [name])
            spec["authority"]["export_paths"] = [name]
            with self.subTest(name=name), self.assertRaises(ProtocolError):
                prepare(self.write(spec))

    def test_repository_native_hidden_skills_are_frozen_as_background(self):
        name = ".agents/skills/helper/SKILL.md"
        self.put(name, "Legitimate repository skill")
        self.git("add", "-f", name)
        self.git(
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "-qm",
            "repo skill",
        )
        self.spec["target"]["base_commit"] = self.git("rev-parse", "HEAD").strip()
        self.spec["cases"][0]["base_commit"] = self.spec["target"]["base_commit"]
        self.spec["cases"][0]["source"]["revision"] = self.spec["target"]["base_commit"]
        self.spec["source"]["paths"].append(".agents/skills")
        self.spec["authority"]["export_paths"].append(".agents/skills")
        self.assertIn(name, prepare(self.write()).bundles["baseline"])

    def test_grader_oracle_and_confirmation_are_not_in_optimizer_view(self):
        self.compare()
        job = prepare(self.write())
        view = job.optimizer_view()
        serialized = json.dumps(view)
        self.assertNotIn("confirmation-case", serialized)
        self.assertNotIn("private grading", serialized)
        self.assertNotIn("private oracle", serialized)
        self.assertNotIn("grader_paths", serialized)
        self.assertNotIn("oracle_paths", serialized)
        self.assertIn("Fix the bug", serialized)

    def test_public_private_overlap_and_symlink_alias_fail(self):
        for name in ["tests/test.sh", "solution/solve.sh", "../outside", "/tmp/file"]:
            with self.subTest(name=name), self.assertRaises(ProtocolError):
                prepare(self.write(self.changed(["cases", 0, "public_paths"], [name])))
        alias = self.task / "environment/alias"
        alias.symlink_to(self.task / "solution/solve.sh")
        self.spec["cases"][0]["public_paths"].append("environment/alias")
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_private_material_cannot_alias_public_or_repository_files(self):
        alias = self.task / "environment/alias"
        os.link(self.task / "solution/solve.sh", alias)
        self.spec["cases"][0]["public_paths"].append("environment/alias")
        with self.assertRaises(ProtocolError):
            prepare(self.write())
        self.spec["cases"][0]["public_paths"].remove("environment/alias")
        os.link(self.task / "tests/test.sh", self.repo / "src/private.txt")
        self.spec["source"]["new_paths"] = ["src/private.txt"]
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_fixed_public_checkout_is_verified_by_actual_git_bytes(self):
        benchmark = self.root / "benchmark"
        task = benchmark / "v2/tasks/fixture"
        shutil.copytree(self.task, task)
        (task / "task.toml").write_text('schema_version = "1.4"\n')

        def source_git(*args):
            return subprocess.check_output(
                ["git", "-C", str(benchmark), *args], text=True
            )

        source_git("init", "-q")
        source_git("add", ".")
        source_git(
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "-qm",
            "public source fixture",
        )
        revision = source_git("rev-parse", "HEAD").strip()
        case = self.spec["cases"][0]
        case["source"].update(kind="pro_v2", revision=revision)
        case["task_path"] = str(task)
        case["public_paths"].append("task.toml")
        with patch("protocol.PRO_V2_REVISION", revision):
            prepare(self.write())
            (task / "tests/test.sh").write_text("altered private grader")
            with self.assertRaises(ProtocolError):
                prepare(self.write())

    def test_parent_symlink_cannot_escape_export_root(self):
        outside = self.root / "outside"
        (self.repo / "src").rename(outside)
        (self.repo / "src").symlink_to(outside, target_is_directory=True)
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_target_skill_old_copies_and_unlisted_files_are_rejected(self):
        target = "skills/debugging/SKILL.md"
        self.spec["target"].update(
            kind="skill",
            allowed_paths=[target],
            surfaces={"skill": [target], "agents": []},
            protected_rules=[],
        )
        self.spec["authority"]["write_paths"] = [target]
        self.spec["conditions"][0].update(skill="baseline", agents="fixed")
        self.put("skills/debugging/reference.md", "Unlisted reference")
        self.spec["source"]["new_paths"] = ["skills/debugging/reference.md"]
        with self.assertRaises(ProtocolError):
            prepare(self.write())
        self.spec["source"]["new_paths"] = []
        self.put("skills/old-debugging/SKILL.md", (self.repo / target).read_text())
        self.spec["source"]["new_paths"] = ["skills/old-debugging/SKILL.md"]
        with self.assertRaises(ProtocolError):
            prepare(self.write())
        self.put(
            "skills/old-debugging/SKILL.md",
            (self.repo / target).read_text() + "An older rule.\n",
        )
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_content_hash_tracks_dirty_bytes_modes_and_protocol(self):
        first = prepare(self.write())
        self.put("src/app.py", "different")
        second = prepare(self.write())
        self.assertNotEqual(first.protocol_hash, second.protocol_hash)
        self.assertNotEqual(
            first.bundle_hashes["baseline"], second.bundle_hashes["baseline"]
        )
        (self.repo / "src/app.py").chmod(0o755)
        third = prepare(self.write())
        self.assertNotEqual(second.bundle_hashes, third.bundle_hashes)

    def test_freeze_is_self_contained_and_load_detects_tampering(self):
        job = prepare(self.write())
        output = self.root / "run"
        freeze(job, output, now=1000)
        self.assertTrue(
            (output / "bundles" / (job.bundle_hashes["baseline"] + ".json")).is_file()
        )
        self.assertEqual(load_run(output)["protocol_hash"], job.protocol_hash)
        shutil.rmtree(self.repo)
        shutil.rmtree(self.task)
        self.assertEqual(load_run(output)["protocol_hash"], job.protocol_hash)
        blob = next((output / "blobs").iterdir())
        blob.write_bytes(b"tampered")
        with self.assertRaises(ConflictError):
            load_run(output)

    def test_protocol_and_bundle_manifest_drift_are_rejected(self):
        job = prepare(self.write())
        output = self.root / "run"
        freeze(job, output)
        path = output / "bundles" / (job.bundle_hashes["baseline"] + ".json")
        original = path.read_bytes()
        path.write_text("{}")
        with self.assertRaises(ConflictError):
            load_run(output)
        path.write_bytes(original)
        protocol_path = output / "protocol.json"
        document = json.loads(protocol_path.read_text())
        document["spec"]["runtime"]["codex_version"] = "drifted"
        protocol_path.write_text(json.dumps(document))
        with self.assertRaises(ConflictError):
            load_run(output)

    def test_existing_output_is_never_overwritten(self):
        job = prepare(self.write())
        output = self.root / "run"
        output.mkdir()
        (output / "user-file").write_text("keep")
        with self.assertRaises(ConflictError):
            freeze(job, output)
        self.assertEqual((output / "user-file").read_text(), "keep")

    def test_deleted_files_and_allowed_new_instruction_file_are_recoverable(self):
        (self.repo / "src/app.py").unlink()
        self.put("src/new.py", "new")
        self.spec["source"]["new_paths"] = ["src/new.py"]
        job = prepare(self.write())
        self.assertIn("src/app.py", job.deleted_paths)
        self.assertNotIn("src/app.py", job.bundles["baseline"])
        freeze(job, self.root / "run")
        self.assertEqual(
            load_run(self.root / "run")["inputs"]["deleted_paths"], ["src/app.py"]
        )
