import json
from pathlib import Path
from unittest.mock import patch

from support import JobFixture
from evolution import ConflictError, apply_run, freeze_correction, replace_files
from protocol import ProtocolError


class CorrectionTests(JobFixture):
    def package(self, changes=None, authorized=True):
        changes = changes or {
            "AGENTS.md": "Preserve authorization.\nRun verified tests.\n"
        }
        output = self.root / "correction"
        freeze_correction(
            self.repo, changes, ["src/app.py"], output, authorized=authorized
        )
        return output

    def test_explicit_fact_authority_applies_atomically_and_remains_unverified(self):
        output = self.package()
        result = apply_run(output, self.repo)
        self.assertEqual(result["verification"], "unverified")
        self.assertIsNone(result["confirmed_best"])
        self.assertIn("verified tests", (self.repo / "AGENTS.md").read_text())
        self.assertEqual(apply_run(output, self.repo)["status"], "already_applied")
        self.assertEqual(
            json.loads((output / "apply.json").read_text())["verification"],
            "unverified",
        )

    def test_missing_fact_authority_cannot_be_inferred_from_repo_text(self):
        output = self.package(authorized=False)
        before = (self.repo / "AGENTS.md").read_bytes()
        with self.assertRaises(ProtocolError):
            apply_run(output, self.repo)
        self.assertEqual((self.repo / "AGENTS.md").read_bytes(), before)

    def test_original_private_file_permissions_are_preserved(self):
        target = self.repo / "AGENTS.md"
        target.chmod(0o600)
        output = self.package()
        apply_run(output, self.repo)
        self.assertEqual(target.stat().st_mode & 0o777, 0o600)

    def test_target_and_fact_source_cas_conflicts_preserve_user_bytes(self):
        output = self.package()
        self.put("AGENTS.md", "user update")
        with self.assertRaises(ConflictError):
            apply_run(output, self.repo)
        self.assertEqual((self.repo / "AGENTS.md").read_text(), "user update")
        self.put("AGENTS.md", "Preserve authorization.\nRun the project tests.\n")
        self.put("src/app.py", "changed source fact")
        with self.assertRaises(ConflictError):
            apply_run(output, self.repo)

    def test_other_repo_and_symlink_target_are_rejected(self):
        output = self.package()
        other = self.root / "other"
        other.mkdir()
        with self.assertRaises(ProtocolError):
            apply_run(output, other)
        target = self.repo / "AGENTS.md"
        target.unlink()
        outside = self.root / "outside"
        outside.write_text("keep")
        target.symlink_to(outside)
        with self.assertRaises((ProtocolError, ConflictError)):
            apply_run(output, self.repo)
        self.assertEqual(outside.read_text(), "keep")

    def test_partial_failure_restores_only_tool_owned_bytes(self):
        self.put("SECOND.md", "second original")
        output = self.package(
            {"AGENTS.md": "Preserve authorization.\nnew", "SECOND.md": "new second"}
        )
        import evolution

        original = evolution.os.replace

        def replace(source, target):
            if Path(target).name == "SECOND.md":
                self.put("AGENTS.md", "concurrent user content")
                raise OSError("injected replace failure")
            return original(source, target)

        with (
            patch("evolution.os.replace", side_effect=replace),
            self.assertRaises(ConflictError),
        ):
            apply_run(output, self.repo)
        self.assertEqual(
            (self.repo / "AGENTS.md").read_text(), "concurrent user content"
        )
        self.assertEqual((self.repo / "SECOND.md").read_text(), "second original")
        self.assertTrue((output / "recovery.json").is_file())

    def test_unconfirmed_experiment_cannot_use_fact_correction_route(self):
        from evolution import freeze
        from protocol import prepare

        output = self.root / "run"
        freeze(prepare(self.write()), output)
        with self.assertRaises(ProtocolError):
            apply_run(output, self.repo)
        correction = self.package()
        (output / "correction.json").write_bytes(
            (correction / "correction.json").read_bytes()
        )
        with self.assertRaises(ProtocolError):
            apply_run(output, self.repo)

    def test_transaction_can_add_and_remove_authorized_skill_files(self):
        from protocol import read_file, sha256

        original = read_file(self.repo, "AGENTS.md")
        output = self.root / "transaction"
        output.mkdir()
        raw = b"new supporting reference"
        replace_files(
            self.repo,
            {"AGENTS.md": original, "references/new.md": None},
            {
                "AGENTS.md": None,
                "references/new.md": {
                    "content": raw,
                    "sha256": sha256(raw),
                    "mode": 0o600,
                },
            },
            output,
        )
        self.assertFalse((self.repo / "AGENTS.md").exists())
        self.assertEqual((self.repo / "references/new.md").read_bytes(), raw)
        self.assertEqual(
            (self.repo / "references/new.md").stat().st_mode & 0o777, 0o600
        )


class ConfirmedApplyTests(JobFixture):
    def confirmed(self):
        import shutil
        import time
        from compare import compare_results
        from evolution import (
            Experiment,
            append_result,
            freeze,
            write_json,
            policy_requirements,
        )
        from protocol import canonical, manifest, new_execution, prepare, sha256

        self.compare()
        self.spec["authority"]["allow_apply"] = True
        self.spec["budget"]["max_trials"] = 1000
        template = self.spec["cases"][1]
        for index in range(11):
            task = self.root / ("confirmation-" + str(index))
            shutil.copytree(Path(template["task_path"]), task)
            (task / "instruction.md").write_text(
                "Independent fixture contract " + str(index)
            )
            case = json.loads(json.dumps(template))
            case.update(
                case_id="confirmation-" + str(index),
                group_id="independent-" + str(index),
                task_path=str(task),
            )
            case["source"]["family"] = case["group_id"]
            self.spec["cases"].append(case)
            self.spec["measurement"]["weights"][case["case_id"]] = 1
        job = prepare(self.write())
        output = self.root / "confirmed"
        state = freeze(job, output)
        runner = b"predetermined test fixture; no remote execution"
        (output / "runner.py").write_bytes(runner)
        rows = []

        def execution(role, files, exit_code=0):
            packet = {"command": {"args": ["fixture"]}}
            item = new_execution(
                job,
                role,
                "fixture",
                sha256(canonical(packet["command"])),
                sha256(runner),
                now=time.time(),
                run_deadline=state["deadline"],
            )
            item.update(
                input_hash=sha256(canonical(packet)),
                phase="captured",
                exit_code=exit_code,
                termination_reason="exited",
                submission_status="runner_submitted",
                sandbox_id="fixture-container",
                engine_id="fixture-engine",
                export_status="exported",
                cleanup_status="terminated",
            )
            relative = "executions/" + item["remote_task_id"]
            directory = output / relative
            (directory / "artifacts").mkdir(parents=True)
            item["artifact_manifest"] = []
            for name, raw in files.items():
                (directory / "artifacts" / name).write_bytes(raw)
                item["artifact_manifest"].append(
                    {"path": name, "size": len(raw), "sha256": sha256(raw)}
                )
            write_json(directory / "payload.json", packet)
            write_json(directory / "remote.json", item)
            state["executions"].append(relative + "/remote.json")
            return item, relative + "/artifacts"

        for case in job.spec["cases"]:
            if case["split"] != "confirmation":
                continue
            for condition in ["baseline", "candidate"]:
                score = int(condition == "candidate")
                patch = b"known distinct fixture patch" if score else b""
                agent, artifact = execution(
                    "trial",
                    {
                        "model.patch": patch,
                        "stdout.jsonl": b'{"type":"turn.completed"}\n',
                        "capture.json": b'{"gaps":[]}',
                    },
                )
                grader, grade = execution(
                    "grader",
                    {"reward.txt": str(score).encode(), "capture.json": b'{"gaps":[]}'},
                    exit_code=0 if score else 1,
                )
                row = {
                    "run_id": state["run_id"],
                    "protocol_id": job.spec["protocol_id"],
                    "trial_id": agent["remote_task_id"],
                    "remote_task_id": agent["remote_task_id"],
                    "case_id": case["case_id"],
                    "group_id": case["group_id"],
                    "split": "confirmation",
                    "repeat": 0,
                    "candidate_id": condition,
                    "candidate_hash": job.bundle_hashes[condition],
                    "configuration_hash": sha256(canonical(job.spec["runtime"])),
                    "image": case["image"],
                    "grader_version": case["grader_ref"],
                    "status": "ok",
                    "score": score,
                    "grader_score": score,
                    "checks": {"authorization": True},
                    "retry_of": None,
                    "cost": None,
                    "purpose": "task",
                    "comparison_id": "confirmation",
                    "artifact": artifact,
                    "grade_evidence": grade,
                    "grader_remote_task_id": grader["remote_task_id"],
                    "patch_sha256": sha256(patch),
                }
                rows.append(row)
                append_result(output, row)
        policy = {
            "passed": True,
            "checks": [
                {
                    "requirement": requirement,
                    "preserved": True,
                    "evidence": "Preserve authorization.",
                    "reason": "known fixture rule",
                }
                for requirement in sorted(policy_requirements(job.spec))
            ],
        }
        reviewer, _ = execution(
            "trial", {"policy.json": canonical(policy), "capture.json": b'{"gaps":[]}'}
        )
        digest = job.bundle_hashes["candidate"]
        state.update(
            candidate_hashes=job.bundle_hashes,
            policy_checks={digest: policy},
            steps={
                "policy:" + digest: "executions/"
                + reviewer["remote_task_id"]
                + "/remote.json"
            },
            frozen_candidate=digest,
            confirmed_best=digest,
            confirmation_exposed=True,
            confirmation_status="passed",
            comparisons={
                "confirmation": compare_results(
                    job.spec,
                    rows,
                    "baseline",
                    "candidate",
                    job.bundle_hashes,
                    split="confirmation",
                )
            },
        )
        self.assertEqual(state["comparisons"]["confirmation"]["verdict"], "keep")
        write_json(output / "state.json", state)
        Experiment(job, output, state, None).finish("keep", "known fixture")
        return output

    def test_confirmation_evidence_adopts_only_authorized_files(self):
        output = self.confirmed()
        result = apply_run(output, self.repo)
        self.assertEqual(result["verification"], "confirmed")
        self.assertEqual(
            (self.repo / "AGENTS.md").read_text(),
            self.spec["conditions"][1]["overrides"]["AGENTS.md"],
        )

    def test_changed_confirmation_artifact_cannot_be_adopted(self):
        output = self.confirmed()
        path = next((output / "executions").glob("*/artifacts/reward.txt"))
        path.write_text("changed")
        with self.assertRaises((ConflictError, ProtocolError)):
            apply_run(output, self.repo)

    def test_confirmation_reward_cannot_override_a_failed_grader_exit(self):
        output = self.confirmed()
        for path in (output / "executions").glob("*/remote.json"):
            item = json.loads(path.read_text())
            if (
                item["role"] == "grader"
                and (path.parent / "artifacts/reward.txt").read_text() == "1"
            ):
                item["exit_code"] = 1
                path.write_text(json.dumps(item))
                break
        with self.assertRaises(ConflictError):
            apply_run(output, self.repo)


if __name__ == "__main__":
    import unittest

    unittest.main()
