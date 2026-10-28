import json
import base64
import shutil
from pathlib import Path
from unittest.mock import patch

from support import JobFixture
from evolution import ConflictError, apply_run, freeze_correction, replace_files
from protocol import ProtocolError, canonical, sha256, prepare
from test_quality import QualityFixture
from test_search import QualityRecordedRemote


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


class QualityApplyRemote(QualityRecordedRemote):
    def query(self, execution, save):
        result = super().query(execution, save)
        payload = self.payloads[execution["remote_task_id"]]
        if "reward.txt" in payload["command"]["capture"]["files"]:
            patch = base64.b64decode(payload["files"]["/evolution-input/model.patch"]["data"]).strip()
            execution["exit_code"] = int(patch in (b"saved patch bad", b"saved patch injected"))
            save(execution)
        return result

    def export(self, execution, destination, save):
        identity = execution["remote_task_id"]
        payload = self.payloads[identity]
        captures = payload["command"]["capture"]["files"]
        if "assessment.json" in captures:
            source = json.loads(base64.b64decode(payload["files"]["/work/source.json"]["data"]))
            patch = source["files"]["patch.diff"].strip()
            if source.get("scoring") == "continuous_100_v1":
                self.levels = ((0, 0) if patch in ("saved patch bad", "saved patch injected", "baseline artifact")
                               else (100, 83.42) if patch == "saved patch weak"
                               else (100, 85.42) if patch in ("saved patch strong", "saved patch renamed")
                               else (100, 0) if patch == "saved patch proxy" else (93.42, 95.64))
            else:
                self.levels = (0, 0) if patch in ("saved patch bad", "saved patch injected", "baseline artifact") else (1, 0) if patch == "saved patch weak" else (1, 1)
        super().export(execution, destination, save)
        files = self.outputs[identity]
        if "assessment.json" in files and getattr(self, "omit_summary", False):
            assessment = json.loads(files["assessment.json"])
            assessment.pop("status", None)
            files["assessment.json"] = canonical(assessment)
        if "proposal.json" in captures:
            import difflib
            source = json.loads(base64.b64decode(payload["files"]["/work/source.json"]["data"]))
            original = source["target"]["AGENTS.md"]["content"]
            target = "Preserve authorization.\nRun tests before delivery.\n"
            evidence = next(r for r in source["search"]["results"] if r["quality_components"]["regression"]["level"] == 0)
            files["proposal.json"] = canonical({"parent_hash": source["parent_hash"],
                "hypothesis": "Verification is delayed until after delivery.", "expected_effect": "Regression protection improves.",
                "counterexample": "The measured regression criterion remains zero.", "changed_paths": ["AGENTS.md"],
                "mechanism": {"criterion_id": "regression", "patch_sha256": evidence["patch_sha256"],
                    "cause": "The recorded artifact does not protect the documented defect.",
                    "expected_criteria": ["regression"], "regression_risk": "Verification could overrun the task budget."}})
            files["candidate.patch"] = "".join(difflib.unified_diff(original.splitlines(True), target.splitlines(True),
                fromfile="a/AGENTS.md", tofile="b/AGENTS.md")).encode()
        elif "policy.json" in captures:
            from evolution import policy_requirements
            files["policy.json"] = canonical({"passed": True, "checks": [
                {"requirement": name, "preserved": True, "evidence": "Preserve authorization.", "reason": "Recorded policy fixture."}
                for name in sorted(policy_requirements(self.spec))]})
        elif execution["role"] == "trial":
            target = base64.b64decode(payload["files"]["/app/AGENTS.md"]["data"])
            files["model.patch"] = b"candidate artifact\n" if b"before delivery" in target else b"baseline artifact\n"
        elif "reward.txt" in captures:
            files["reward.txt"] = str(1 - execution["exit_code"]).encode()
        for name, raw in files.items():
            (destination / name).write_bytes(raw)
        execution["artifact_manifest"] = [{"path": name, "sha256": sha256(raw), "size": len(raw)} for name, raw in sorted(files.items())]
        save(execution)


class QualityApplyTests(QualityFixture):
    def confirmed(self, *, auxiliary=False, run_entry=False, search=False, continuous=False, derived_status=False):
        from evolution import Experiment, freeze, write_json

        self.compare()
        root = self.quality_spec()
        if continuous:
            from test_quality import continuous_rubric, continuous_calibration
            rubric_value = continuous_rubric()
            if derived_status:
                rubric_value['derive_assessment_status'] = True
            (root / "rubric.json").write_bytes(canonical(rubric_value))
            path = root / "calibration.json"
            path.write_bytes(canonical(continuous_calibration(json.loads(path.read_bytes()))))
        self.spec["mode"] = "compare"
        self.spec["authority"]["allow_apply"] = True
        self.spec["budget"]["max_trials"] = 1000
        self.spec["guardrails"] = [{"name": "task_contract", "minimum": 1}]
        template = self.spec["cases"][1]
        for index in range(11):
            task = self.root / ("independent-fixture-" + str(index))
            shutil.copytree(Path(template["task_path"]), task)
            (task / "instruction.md").write_text("Independent fixture contract " + str(index))
            case = json.loads(json.dumps(template))
            case.update(case_id=task.name, group_id=task.name, task_path=str(task))
            case["source"]["family"] = task.name
            self.spec["cases"].append(case)
            self.spec["measurement"]["weights"][case["case_id"]] = 1
        if search:
            self.spec["mode"] = "search"
            self.spec["conditions"] = self.spec["conditions"][:1]
            self.spec.pop("candidate_condition")
            self.spec["budget"]["max_trials"] = 2000
            method = self.root / "method.md"
            method.write_text("Use one measured mechanism and a falsifiable expected quality change.")
            self.spec["optimizer"] = {"model": "gpt-6.1-sol", "effort": "high",
                "method_path": str(method), "method_sha256": sha256(method.read_bytes())}
            template = self.spec["cases"][0]
            for index in range(39):
                task = self.root / ("search-fixture-" + str(index))
                shutil.copytree(Path(template["task_path"]), task)
                (task / "instruction.md").write_text("Independent search fixture contract " + str(index))
                case = json.loads(json.dumps(template))
                case.update(case_id=task.name, group_id=task.name, task_path=str(task))
                case["source"]["family"] = task.name
                self.spec["cases"].append(case)
                self.spec["measurement"]["weights"][case["case_id"]] = 1
        for case in self.spec["cases"]:
            if case["case_id"] == "search-case":
                continue
            rubric = json.loads((root / "rubric.json").read_text())
            rubric["case_id"] = case["case_id"]
            for criterion in rubric["criteria"]:
                criterion["public_requirement_ref"] = "instruction.md: " + (Path(case["task_path"]) / "instruction.md").read_text()
            name = case["case_id"] + "-rubric.json"
            (root / name).write_bytes(canonical(rubric))
            self.spec["evaluation"]["paths"].append(name)
            self.spec["evaluation"]["rubrics"][case["case_id"]] = name
        if auxiliary:
            primary = json.loads((root / "rubric.json").read_text())["criteria"][0]
            (root / "auxiliary-rubric.json").write_bytes(canonical({"criteria": {"boundary": {
                "dimension": "boundary_completeness", "instructions": "Judge the public boundary repair in the patch.",
                "anchors": primary["anchors"], "context_paths": ["instruction.md", "patch.diff"]}}}))
            self.spec["evaluation"]["paths"].append("auxiliary-rubric.json")
            self.spec["evaluation"]["auxiliary"] = {"provider": "typesafe", "model": "jev-1.14.0",
                "criterion_ids": ["boundary"], "rubric_path": "auxiliary-rubric.json", "calibration_path": "calibration.json",
                "authentication": {"source": "controller_env", "name": "TYPESAFE_API_KEY"}}
            self.spec["authority"]["allow_typesafe_auxiliary"] = True
            self.spec["budget"]["max_auxiliary_calls"] = 12
        job = prepare(self.write())
        output = self.root / "quality-confirmed"
        state = freeze(job, output)
        (output / "runner.py").write_bytes(job.worker_files["remote.py"]["content"])
        remote = QualityApplyRemote()
        remote.omit_summary = derived_status
        remote.runner_hash = sha256((output / "runner.py").read_bytes())
        remote.spec = job.spec
        engine = Experiment(job, output, state, remote)
        if run_entry:
            engine.calibrate = engine.calibrate_quality
            result = engine.run()
            self.assertEqual(result["verdict"], "keep")
            self.job, self.output, self.state, self.remote = job, output, state, remote
            return output
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "fixture-secret"}):
            engine.calibrate_quality()
        self.assertEqual(state["calibration_status"], "passed")
        state["calibration_passed"] = True
        engine.evaluate("aa", ["aa_0", "aa_1"], "search")
        self.assertTrue(engine.diagnose_baseline()["proceed"])
        engine.evaluate("confirmation", ["baseline", "candidate"], "confirmation")
        digest = job.bundle_hashes["candidate"]
        policy = engine.policy_check(job.bundles["candidate"])
        state.update(policy_checks={digest: policy}, frozen_candidate=digest, confirmed_best=digest,
                     confirmation_exposed=True, confirmation_status="passed")
        state["comparisons"]["confirmation"] = engine.comparison("confirmation", "baseline", "candidate", "confirmation")
        self.assertEqual(state["comparisons"]["confirmation"]["verdict"], "keep")
        engine.finish("keep", "Recorded fixture, not provider acceptance.")
        self.job, self.output, self.state, self.remote = job, output, state, remote
        return output

    def test_quality_confirmation_replays_original_receipts_without_model_calls(self):
        output = self.confirmed()
        before = list(self.remote.submissions)
        result = apply_run(output, self.repo)
        self.assertEqual(result["schema_version"], 2)
        self.assertEqual(result["verification"], "confirmed")
        self.assertEqual(self.remote.submissions, before)

    def test_continuous_confirmation_replays_fractional_scores_before_apply(self):
        output = self.confirmed(continuous=True)
        rows = [json.loads(line) for line in (output / "results.jsonl").read_text().splitlines()]
        candidate = next(r for r in rows if r["comparison_id"] == "confirmation" and r["candidate_id"] == "candidate")
        self.assertEqual(candidate["quality_score"], 94.53)
        self.assertEqual(candidate["quality_components"]["boundary"]["judge_scores"], [93.42, 93.42])
        before = list(self.remote.submissions)
        self.assertEqual(apply_run(output, self.repo)["verification"], "confirmed")
        self.assertEqual(self.remote.submissions, before)

    def test_derived_status_raw_outputs_replay_without_requiring_model_summaries(self):
        output = self.confirmed(continuous=True, derived_status=True)
        raw = list(output.glob('executions/*/artifacts/assessment.json'))
        self.assertTrue(raw)
        self.assertTrue(all('status' not in json.loads(path.read_text()) for path in raw))
        before = list(self.remote.submissions)
        self.assertEqual(apply_run(output, self.repo)['verification'], 'confirmed')
        self.assertEqual(self.remote.submissions, before)

    def test_rehashed_continuous_score_claim_cannot_replace_original_judgments(self):
        output = self.confirmed(continuous=True)
        original = (self.repo / "AGENTS.md").read_bytes()
        rows = [json.loads(line) for line in (output / "results.jsonl").read_text().splitlines()]
        for row in rows:
            if row["comparison_id"] == "confirmation" and row["candidate_id"] == "candidate":
                row.update(score=0.9, task_quality=0.9, quality_score=90.0)
        self.rewrite_claims(rows)
        before = list(self.remote.submissions)
        with self.assertRaises((ConflictError, ProtocolError)):
            apply_run(output, self.repo)
        self.assertEqual((self.repo / "AGENTS.md").read_bytes(), original)
        self.assertEqual(self.remote.submissions, before)

    def test_v2_compare_entry_runs_diagnostic_then_independent_confirmation(self):
        output = self.confirmed(run_entry=True)
        self.assertTrue(self.state["baseline_diagnostic"]["proceed"])
        self.assertEqual(self.state["confirmation_status"], "passed")
        self.assertEqual(self.remote.api_count, 0)
        self.assertEqual(apply_run(output, self.repo)["verification"], "confirmed")

    def test_v2_search_entry_generates_one_mechanism_and_confirms_only_the_frozen_candidate(self):
        output = self.confirmed(run_entry=True, search=True)
        self.assertEqual(self.state["candidates_used"], 1)
        self.assertEqual(len(self.state["attempts"]), 1)
        self.assertEqual(self.state["attempts"][0]["mechanism"]["criterion_id"], "regression")
        self.assertEqual(self.state["confirmation_status"], "passed")
        before = list(self.remote.submissions)
        self.assertEqual(apply_run(output, self.repo)["verification"], "confirmed")
        self.assertEqual(self.remote.submissions, before)

    def rewrite_claims(self, rows):
        from compare import compare_results
        from evolution import write_json
        path = self.output / "results.jsonl"
        path.write_bytes(b"".join(canonical(row) + b"\n" for row in rows))
        comparison = compare_results(self.job.spec, [r for r in rows if r["comparison_id"] == "confirmation"],
            "baseline", "candidate", self.state["candidate_hashes"], split="confirmation",
            calibration_sha256=self.state["quality_calibration_certificate"]["certificate_sha256"])
        self.assertEqual(comparison["verdict"], "keep")
        self.state["comparisons"]["confirmation"] = comparison
        write_json(self.output / "state.json", self.state)
        report = json.loads((self.output / "report.json").read_text())
        report["comparisons"]["confirmation"] = comparison
        report["results_sha256"] = sha256(path.read_bytes())
        write_json(self.output / "report.json", report)

    def test_rehashed_aggregate_claim_cannot_replace_original_quality_evidence(self):
        output = self.confirmed()
        original = (self.repo / "AGENTS.md").read_bytes()
        rows = [json.loads(line) for line in (output / "results.jsonl").read_text().splitlines()]
        for row in rows:
            if row["comparison_id"] == "confirmation" and row["candidate_id"] == "candidate":
                row.update(score=0.9, task_quality=0.9)
        self.rewrite_claims(rows)
        with self.assertRaises((ConflictError, ProtocolError)):
            apply_run(output, self.repo)
        self.assertEqual((self.repo / "AGENTS.md").read_bytes(), original)

    def test_changed_derived_assessment_is_rejected(self):
        output = self.confirmed()
        rows = [json.loads(line) for line in (output / "results.jsonl").read_text().splitlines()]
        candidate = next(r for r in rows if r["comparison_id"] == "confirmation" and r["candidate_id"] == "candidate")
        path = output / candidate["quality_evidence_ref"]
        record = json.loads(path.read_text())
        record["task_quality"] = 0.9
        path.write_bytes(canonical(record))
        with self.assertRaises((ConflictError, ProtocolError)):
            apply_run(output, self.repo)

    def test_calibration_identity_drift_cannot_be_adopted(self):
        output = self.confirmed()
        path = output / "calibration.json"
        document = json.loads(path.read_text())
        document["certificate"]["identity"]["quality_worker_sha256"] = "f" * 64
        path.write_bytes(canonical(document))
        with self.assertRaises((ConflictError, ProtocolError)):
            apply_run(output, self.repo)

    def test_confirmation_cannot_gain_an_auxiliary_reference(self):
        output = self.confirmed()
        rows = [json.loads(line) for line in (output / "results.jsonl").read_text().splitlines()]
        candidate = next(r for r in rows if r["comparison_id"] == "confirmation" and r["candidate_id"] == "candidate")
        candidate["auxiliary_ref"] = "auxiliary/forbidden.json"
        self.rewrite_claims(rows)
        with self.assertRaises((ConflictError, ProtocolError)):
            apply_run(output, self.repo)

    def test_failed_auxiliary_calibration_does_not_veto_main_confirmation(self):
        output = self.confirmed(auxiliary=True)
        self.assertFalse(self.state["auxiliary_calibration"]["boundary"]["eligible"])
        self.assertEqual(apply_run(output, self.repo)["verification"], "confirmed")

    def test_changed_auxiliary_worker_response_is_rejected_even_with_a_new_manifest_hash(self):
        output = self.confirmed(auxiliary=True)
        path = next((output / "executions").glob("*/artifacts/auxiliary-response.json"))
        document = json.loads(path.read_text())
        document["response"]["model"] = "jev-other"
        path.write_bytes(canonical(document))
        meta_path = path.parent.parent / "remote.json"
        metadata = json.loads(meta_path.read_text())
        artifact = next(a for a in metadata["artifact_manifest"] if a["path"] == path.name)
        artifact.update(sha256=sha256(path.read_bytes()), size=len(path.read_bytes()))
        meta_path.write_bytes(canonical(metadata))
        with self.assertRaises((ConflictError, ProtocolError)):
            apply_run(output, self.repo)


if __name__ == "__main__":
    import unittest

    unittest.main()
