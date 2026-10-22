import base64
import json
import time
from unittest.mock import patch

from support import JobFixture
from evolution import (
    ConflictError,
    ExecutionJournal,
    Experiment,
    freeze,
    restore_job,
    run_smoke,
    recover_registered,
    writer_lock,
)
from protocol import prepare, bundle_hash, canonical, sha256
from remote import UnresolvedError
from test_quality import QualityFixture


class RecordedRemote:
    runner_hash = "a" * 64

    def __init__(self):
        self.submissions = []
        self.queries = []
        self.phase = "captured"
        self.fail_export = False
        self.unknown = False

    def submit(self, execution, payload, save):
        self.submissions.append(execution["remote_task_id"])
        execution.update(
            submission_status="runner_submitted",
            sandbox_id="owned-container",
            engine_id="engine",
        )
        save(execution)
        return self.query(execution, save)

    def query(self, execution, save):
        self.queries.append(execution["remote_task_id"])
        if self.unknown:
            raise UnresolvedError("injected unknown state")
        execution.update(
            phase=self.phase, exit_code=0 if self.phase == "captured" else None
        )
        if self.phase == "running":
            execution["process"] = {
                "pid": 100,
                "pgid": 100,
                "start_identity": "original",
            }
        save(execution)
        return {
            "receipt": execution,
            "command_alive": self.phase == "running",
            "supervisor_alive": self.phase != "captured",
        }

    def export(self, execution, destination, save):
        if self.fail_export:
            raise UnresolvedError("injected export interruption")
        destination.mkdir(parents=True, exist_ok=True)
        (destination / "capture.json").write_text('{"gaps":[]}')
        execution["export_status"] = "exported"
        save(execution)

    def cleanup(self, execution, save, force_reason=None):
        execution["cleanup_status"] = "terminated"
        save(execution)

    def cancel(self, execution, reason, save):
        execution["cancel_reason"] = reason
        save(execution)


class QualityRecordedRemote(RecordedRemote):
    def __init__(self):
        super().__init__()
        self.payloads, self.roles, self.outputs = {}, {}, {}
        self.levels, self.level_schedule = (1, 0), []
        self.grader_gap = self.agent_timeout = False
        self.judge_failures_remaining = self.judge_count = 0
        self.grader_failures_remaining = 0
        self.api_count = 0
        self.api_failure = self.fail_api_export = False
        self.agent_trace = None

    def submit(self, execution, payload, save):
        self.payloads[execution["remote_task_id"]] = payload
        self.roles[execution["remote_task_id"]] = execution["role"]
        return super().submit(execution, payload, save)

    def query(self, execution, save):
        result = super().query(execution, save)
        if execution["role"] == "trial" and self.agent_timeout:
            execution.update(exit_code=1, termination_reason="timeout")
            save(execution)
        return result

    def export(self, execution, destination, save):
        if self.fail_api_export and self.payloads[execution["remote_task_id"]].get("needs_typesafe"):
            raise UnresolvedError("interrupted API response export")
        super().export(execution, destination, save)
        identity = execution["remote_task_id"]
        if identity not in self.outputs:
            payload = self.payloads[identity]
            files = {"capture.json": canonical({"gaps": []})}
            captures = payload["command"]["capture"]["files"]
            if payload.get("needs_typesafe"):
                self.api_count += 1
                request = json.loads(base64.b64decode(payload["files"]["/work/request.json"]["data"]))
                response = {"model": "jev-1.14.0", "answers": {name: {
                    "type": "score", "score": 1.2, "confidence": 0.5,
                    "legend": {str(i): level for i, level in enumerate(question["criteria"])},
                    "probabilities": {"0": 0, "1": 0.8, "2": 0.2},
                } for name, question in request["questions"].items()}, "usage": {"input_tokens": 100, "output_tokens": 4}}
                files.update({"auxiliary-response.json": canonical({
                    "status": "unavailable" if self.api_failure else "valid",
                    "reason": "http_429" if self.api_failure else "response_received",
                    "response": None if self.api_failure else response, "request_attempted": True,
                    "request_sha256": sha256(canonical(request)), "latency_sec": 0.05}),
                    "credential-receipt.json": canonical({"provider": "typesafe", "destroyed": True})})
            elif execution["role"] == "trial":
                files.update({"model.patch": b"delivered patch\n", "stderr.txt": b"",
                    "stdout.jsonl": self.agent_trace if self.agent_trace is not None else canonical({"type": "turn.completed", "usage": {"input_tokens": 10, "output_tokens": 2}}) + b"\n"})
            elif "assessment.json" in captures:
                self.judge_count += 1
                files["stdout.jsonl"] = canonical({"type": "turn.completed", "usage": {
                    "input_tokens": 20, "output_tokens": 3, "cached_input_tokens": 5}}) + b"\n"
                if self.judge_failures_remaining:
                    self.judge_failures_remaining -= 1
                    files["capture.json"] = canonical({"gaps": ["missing assessment"]})
                else:
                    source = json.loads(base64.b64decode(payload["files"]["/work/source.json"]["data"]))
                    raw = source["files"]["patch.diff"].encode()
                    levels = self.level_schedule[self.judge_count - 1] if self.judge_count <= len(self.level_schedule) else self.levels
                    evidence = {"path": "patch.diff", "sha256": sha256(raw), "start_line": 1,
                                "end_line": 1, "quote": raw.decode().strip()}
                    files["assessment.json"] = canonical({"status": "valid", "criteria": [
                        {"criterion_id": c["criterion_id"], "status": "valid", "level": level,
                         "reason": "The quoted artifact matches the declared anchor.", "evidence": [evidence]}
                        for c, level in zip(source["criteria"], levels)]})
                    if source.get("scoring") == "continuous_100_v1":
                        value = json.loads(files["assessment.json"])
                        for item, criterion in zip(value["criteria"], source["criteria"]):
                            if not criterion["gate"]:
                                item["deductions"] = [{"points": round(100 - item["level"], 2),
                                    "reason": "The quoted requirement has the remaining gap.", "evidence": [evidence]}] if item["level"] < 100 else []
                        files["assessment.json"] = canonical(value)
            elif self.grader_gap or self.grader_failures_remaining:
                if self.grader_failures_remaining:
                    self.grader_failures_remaining -= 1
                files["capture.json"] = canonical({"gaps": ["missing verifier result"]})
            else:
                files.update({"reward.txt": b"1", "quality-context.json": canonical({"files": {}, "omissions": []})})
            self.outputs[identity] = files
        for name, raw in self.outputs[identity].items():
            (destination / name).write_bytes(raw)
        execution["artifact_manifest"] = [{"path": name, "sha256": sha256(raw), "size": len(raw)}
                                          for name, raw in sorted(self.outputs[identity].items())]
        save(execution)


class JournalTests(JobFixture):
    def test_changed_runtime_builtin_sources_stop_the_batch_without_resubmission(self):
        self.state["runtime_skills_hash"] = "a" * 64
        original = self.remote.export

        def export(execution, directory, save):
            original(execution, directory, save)
            (directory / "runtime-skills.json").write_text(
                '{"skills":["changed"],"files":[]}'
            )

        self.remote.export = export
        payload = {
            "needs_codex": True,
            "command": {"capture": {"runtime_skills": True}},
        }
        for unused in range(2):
            with self.assertRaises(ConflictError):
                Experiment(self.job, self.output, self.state, self.remote).artifacts(
                    "trial", "trial", payload
                )
        self.assertEqual(len(self.remote.submissions), 1)

    def test_controller_lock_is_reentrant_but_excludes_a_fresh_process(self):
        import subprocess, sys
        from pathlib import Path

        with writer_lock(self.output):
            with writer_lock(self.output):
                pass
            source = "from evolution import writer_lock; import sys; from pathlib import Path\nwith writer_lock(Path(sys.argv[1])): pass"
            result = subprocess.run(
                [sys.executable, "-c", source, str(self.output)],
                cwd=Path(__file__).resolve().parents[1],
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("another controller", result.stderr)

    def setUp(self):
        super().setUp()
        self.job = prepare(self.write())
        self.output = self.root / "run"
        self.state = freeze(self.job, self.output)
        self.remote = RecordedRemote()

    def journal(self):
        return ExecutionJournal(self.job, self.output, self.state, self.remote)

    def test_completed_step_is_idempotent_across_fresh_controllers(self):
        first = self.journal().execute(
            "probe", "probe", {"command": {"args": ["fixture"]}}
        )
        job, state = restore_job(self.output)
        second = ExecutionJournal(job, self.output, state, self.remote).execute(
            "probe", "probe", {"command": {"args": ["fixture"]}}
        )
        self.assertEqual(first["remote_task_id"], second["remote_task_id"])
        self.assertEqual(len(self.remote.submissions), 1)
        self.assertEqual(state["trials_used"], 1)

    def test_export_interruption_recovers_original_execution_without_submit(self):
        self.remote.fail_export = True
        with self.assertRaises(UnresolvedError):
            self.journal().execute("trial", "trial", {"command": {"args": ["fixture"]}})
        original = self.remote.submissions[0]
        self.remote.fail_export = False
        job, state = restore_job(self.output)
        result = ExecutionJournal(job, self.output, state, self.remote).execute(
            "trial", "trial", {"command": {"args": ["fixture"]}}
        )
        self.assertEqual(result["remote_task_id"], original)
        self.assertEqual(self.remote.submissions, [original])
        self.assertEqual(result["cleanup_status"], "terminated")

    def test_unknown_remote_state_does_not_start_another_paid_execution(self):
        self.remote.unknown = True
        with self.assertRaises(UnresolvedError):
            self.journal().execute("trial", "trial", {"command": {}})
        job, state = restore_job(self.output)
        with self.assertRaises(UnresolvedError):
            ExecutionJournal(job, self.output, state, self.remote).execute(
                "trial", "trial", {"command": {}}
            )
        self.assertEqual(len(self.remote.submissions), 1)
        self.assertEqual(state["trials_used"], 1)

    def test_confirmation_reserve_cannot_be_spent_by_search(self):
        self.state["confirmation_reserve"] = self.job.spec["budget"]["max_trials"]
        with self.assertRaises(ConflictError):
            self.journal().execute("search", "trial", {"command": {}})
        self.assertFalse(self.remote.submissions)
        self.journal().execute(
            "confirmation", "trial", {"command": {}}, confirmation=True
        )
        self.assertEqual(
            self.state["confirmation_reserve"],
            self.job.spec["budget"]["max_trials"] - 1,
        )

    def test_frozen_step_payload_and_runner_cannot_drift(self):
        self.journal().execute("trial", "trial", {"command": {"args": ["first"]}})
        with self.assertRaises(ConflictError):
            self.journal().execute("trial", "trial", {"command": {"args": ["other"]}})
        self.remote.runner_hash = "b" * 64
        with self.assertRaises(ConflictError):
            self.journal().execute("trial", "trial", {"command": {"args": ["first"]}})

    def test_no_new_submission_after_command_window_closes(self):
        self.state["deadline"] = (
            time.time() + self.job.spec["budget"]["recovery_window_sec"] - 1
        )
        with self.assertRaises(ConflictError):
            self.journal().execute("late", "trial", {"command": {}})
        self.assertFalse(self.remote.submissions)

    def test_confirmation_retries_do_not_make_the_reserved_budget_negative(self):
        self.state["confirmation_reserve"] = 1
        journal = self.journal()
        journal.execute("confirmation", "trial", {"command": {}}, confirmation=True)
        journal.execute(
            "confirmation-retry", "trial", {"command": {}}, confirmation=True
        )
        self.assertEqual(self.state["confirmation_reserve"], 0)
        self.assertEqual(self.state["trials_used"], 2)

    def test_optimizer_feedback_has_search_outputs_but_no_confirmation_trace(self):
        from evolution import append_result

        for split in ["search", "confirmation"]:
            directory = self.output / split
            directory.mkdir()
            (directory / "stdout.jsonl").write_text(split + " visible tool trace")
            (directory / "stderr.txt").write_text("failure details")
            (directory / "model.patch").write_text("public task deliverable")
            append_result(
                self.output,
                {
                    "remote_task_id": split,
                    "purpose": "task",
                    "split": split,
                    "case_id": split + "-case",
                    "status": "behavior_failure",
                    "score": 0,
                    "reason": "timeout",
                    "patch_sha256": "a" * 64,
                    "artifact": split,
                },
            )
        engine = Experiment(self.job, self.output, self.state, self.remote)
        view = engine.feedback()
        self.assertEqual(
            view["goal"],
            {
                "question": self.job.spec["question"],
                "objective": self.job.spec["measurement"]["objective"],
                "primary_metric": self.job.spec["measurement"]["primary_metric"],
                "minimum_effect": self.job.spec["measurement"]["minimum_effect"],
                "guardrails": self.job.spec["guardrails"],
            },
        )
        feedback = json.dumps(view)
        self.assertIn("search visible tool trace", feedback)
        self.assertIn("failure details", feedback)
        self.assertIn("public task deliverable", feedback)
        self.assertNotIn("confirmation visible tool trace", feedback)
        self.assertNotIn("confirmation-case", feedback)

    def test_recovery_queries_every_registered_identity_even_after_a_gap(self):
        journal = self.journal()
        journal.execute("first", "trial", {"command": {}})
        journal.execute("second", "grader", {"command": {}})
        original = self.remote.query
        self.remote.queries = []
        first = self.remote.submissions[0]

        def query(execution, save):
            if execution["remote_task_id"] == first:
                self.remote.queries.append(first)
                raise UnresolvedError("first query unavailable")
            return original(execution, save)

        self.remote.query = query
        gaps = recover_registered(self.output, self.state, self.remote)
        self.assertEqual(len(gaps), 1)
        self.assertEqual(set(self.remote.queries), set(self.remote.submissions))
        self.assertEqual(len(self.remote.submissions), 2)

    def test_registered_but_unsubmitted_step_can_start_after_recovery(self):
        def interrupted_submit(execution, payload, save):
            raise KeyboardInterrupt

        original = self.remote.submit
        self.remote.submit = interrupted_submit
        with self.assertRaises(KeyboardInterrupt):
            self.journal().execute("pending", "trial", {"command": {}})
        self.remote.submit = original
        self.remote.unknown = True
        gaps = recover_registered(self.output, self.state, self.remote)
        self.assertFalse(gaps)
        self.assertFalse(self.remote.queries)
        self.remote.unknown = False
        self.journal().execute("pending", "trial", {"command": {}})
        self.assertEqual(len(self.remote.submissions), 1)
        self.assertEqual(self.state["trials_used"], 1)

    def test_restored_job_uses_saved_bytes_without_original_files(self):
        import shutil

        shutil.rmtree(self.repo)
        shutil.rmtree(self.task)
        job, state = restore_job(self.output)
        self.assertEqual(job.protocol_hash, self.job.protocol_hash)
        self.assertEqual(job.bundles, self.job.bundles)
        self.assertEqual(state["run_id"], self.state["run_id"])

    def test_smoke_resume_reuses_results_and_never_repeats_execution(self):
        payloads = {}
        remote = self.remote
        original_submit = remote.submit
        original_query = remote.query

        def submit(execution, payload, save):
            payloads[execution["remote_task_id"]] = payload
            return original_submit(execution, payload, save)

        def query(execution, save):
            result = original_query(execution, save)
            args = payloads[execution["remote_task_id"]]["command"]["args"]
            execution["exit_code"] = (
                1 if args == ["bash", "-c", "bash /tests/test.sh"] else 0
            )
            save(execution)
            return result

        def export(execution, destination, save):
            destination.mkdir(parents=True, exist_ok=True)
            (destination / "capture.json").write_text('{"gaps":[]}')
            (destination / "model.patch").write_text("src/evolution_capture_probe.js")
            (destination / "stdout.jsonl").write_text(
                '{"isolation_passed":true,"checks":{}}'
            )
            (destination / "reward.txt").write_text(
                "0" if execution["exit_code"] else "1"
            )
            execution["export_status"] = "exported"
            save(execution)

        remote.submit, remote.query, remote.export = submit, query, export
        with patch("remote.RemoteClient", return_value=remote):
            report = run_smoke(self.job, self.output, self.state)
            original_results = (self.output / "results.jsonl").read_bytes()
            job, state = restore_job(self.output)
            resumed = run_smoke(job, self.output, state)
        self.assertTrue(report["calibration_passed"])
        self.assertEqual(report, resumed)
        self.assertEqual(len(remote.submissions), 3)
        self.assertEqual((self.output / "results.jsonl").read_bytes(), original_results)


class TrialRecordingTests(JobFixture):
    def setUp(self):
        super().setUp()
        self.compare()
        self.spec["budget"]["max_retries"] = 1
        self.job = prepare(self.write())
        self.output = self.root / "trials"
        self.state = freeze(self.job, self.output)
        self.engine = Experiment(self.job, self.output, self.state, RecordedRemote())
        self.calls = []
        self.failure = None
        self.engine.artifacts = self.artifacts

    def artifacts(self, key, role, payload, **kwargs):
        directory = self.output / key
        directory.mkdir(exist_ok=True)
        self.calls.append((key, role))
        first = key.endswith(":0:agent") or key.endswith(":0:grader")
        gap = first and self.failure == role + "_capture"
        (directory / "capture.json").write_text(
            json.dumps({"gaps": ["injected missing output"] if gap else []})
        )
        if role == "trial":
            (directory / "model.patch").write_text("delivered patch")
            (directory / "stdout.jsonl").write_text(
                '{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":2}}\n'
            )
            (directory / "stderr.txt").write_text("")
        elif not gap:
            (directory / "reward.txt").write_text("1")
        return {
            "remote_task_id": key,
            "sandbox_id": "owned-" + key,
            "exit_code": 1
            if (self.failure == "timeout" and role == "trial")
            or (self.failure == "grader_exit" and role == "grader" and first)
            else 0,
            "termination_reason": "timeout"
            if self.failure == "timeout" and role == "trial"
            else "exited",
        }, directory

    def test_captured_agent_infrastructure_failure_is_retained_before_bounded_retry(
        self,
    ):
        self.failure = "trial_capture"
        self.engine.evaluate("known", ["baseline"], "search")
        rows = self.engine.rows()
        self.assertEqual([row["status"] for row in rows], ["infra_invalid", "ok"])
        self.assertEqual(rows[1]["retry_of"], rows[0]["trial_id"])
        self.assertIsNone(rows[0]["score"])
        self.assertEqual(rows[1]["score"], 1)

    def test_missing_grader_reward_is_infrastructure_failure_and_not_a_zero_score(self):
        self.failure = "grader_capture"
        self.engine.evaluate("known", ["baseline"], "search")
        rows = self.engine.rows()
        self.assertEqual([row["status"] for row in rows], ["infra_invalid", "ok"])
        self.assertIsNone(rows[0]["grader_score"])
        self.assertIsNone(rows[0]["score"])

    def test_grader_reward_and_original_exit_must_agree(self):
        self.failure = "grader_exit"
        self.engine.evaluate("known", ["baseline"], "search")
        rows = self.engine.rows()
        self.assertEqual([row["status"] for row in rows], ["infra_invalid", "ok"])
        self.assertIsNone(rows[0]["score"])

    def test_agent_timeout_is_a_valid_zero_and_does_not_use_infrastructure_retry(self):
        self.failure = "timeout"
        self.engine.evaluate("known", ["baseline"], "search")
        rows = self.engine.rows()
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["status"], "behavior_failure")
        self.assertEqual(rows[0]["score"], 0)
        self.assertEqual(rows[0]["grader_score"], 1)
        self.assertIsNone(rows[0]["latency"])


class QualityTrialTests(QualityFixture):
    def setUp(self):
        super().setUp()
        self.quality_spec()
        self.spec["guardrails"] = [{"name": "task_contract", "minimum": 1}]
        self.spec["budget"]["max_retries"] = 1
        self.start_engine("quality-trials")

    def start_engine(self, name):
        self.job = prepare(self.write())
        self.output = self.root / name
        self.state = freeze(self.job, self.output)
        self.state["quality_calibration_certificate"] = {"status": "passed", "certificate_sha256": "e" * 64}
        self.remote = QualityRecordedRemote()
        self.engine = Experiment(self.job, self.output, self.state, self.remote)

    def enable_auxiliary(self, criteria=("boundary",)):
        root = self.root / "evaluation"
        primary = json.loads((root / "rubric.json").read_text())
        questions = {"boundary": "Judge whether the patch repairs ordinary and documented boundary behavior from instruction.md.",
                     "regression": "Judge whether the patch's regression assertions protect the documented defect from instruction.md."}
        rubric = {"criteria": {c["criterion_id"]: {
            "dimension": c["dimension"], "instructions": questions[c["criterion_id"]],
            "anchors": c["anchors"], "context_paths": ["instruction.md", "patch.diff"],
        } for c in primary["criteria"] if c["criterion_id"] in criteria}}
        (root / "auxiliary-rubric.json").write_bytes(canonical(rubric))
        self.spec["evaluation"]["paths"].append("auxiliary-rubric.json")
        self.spec["evaluation"]["auxiliary"] = {"provider": "typesafe", "model": "jev-1.14.0",
            "criterion_ids": list(criteria), "rubric_path": "auxiliary-rubric.json", "calibration_path": "calibration.json",
            "authentication": {"source": "controller_env", "name": "TYPESAFE_API_KEY"}}
        self.spec["authority"]["allow_typesafe_auxiliary"] = True
        self.spec["budget"]["max_auxiliary_calls"] = 4
        self.start_engine("auxiliary-trials")
        self.state["auxiliary_calibration"] = {"boundary": {"eligible": True, "certificate_sha256": "f" * 64}}

    def evaluate(self):
        self.engine.evaluate("known", ["baseline"], "search")
        return self.engine.rows()

    def enable_action_log(self):
        path = self.root / "evaluation/rubric.json"
        rubric = json.loads(path.read_bytes())
        rubric["criteria"][0]["context_paths"].append("tool-log.txt")
        path.write_bytes(canonical(rubric))
        self.start_engine("action-log-trials")

    def test_requested_action_log_reaches_judges_without_reasoning_or_tool_output(self):
        self.enable_action_log()
        events = [
            {"type": "item.completed", "item": {"id": "r", "type": "reasoning", "text": "PRIVATE_REASONING_CANARY"}},
            {"type": "item.completed", "item": {"id": "a", "type": "agent_message", "text": "我准备为供应方研究ASR后端。"}},
            {"type": "item.started", "item": {"id": "b", "type": "command_execution", "command": "rg ASR vendor", "status": "in_progress"}},
            {"type": "item.completed", "item": {"id": "b", "type": "command_execution", "command": "rg ASR vendor", "status": "completed", "exit_code": 0, "aggregated_output": "TOOL_OUTPUT_CANARY"}},
            {"type": "turn.completed", "usage": {"input_tokens": 10, "output_tokens": 2}},
        ]
        self.remote.agent_trace = b"\n".join(canonical(e) for e in events) + b"\n"
        row, = self.evaluate()
        self.assertEqual(row["quality_status"], "valid")
        payloads = [p for p in self.remote.payloads.values() if "assessment.json" in p["command"]["capture"]["files"]]
        self.assertEqual(len(payloads), 2)
        for payload in payloads:
            source = json.loads(base64.b64decode(payload["files"]["/work/source.json"]["data"]))
            log = json.loads(source["files"]["tool-log.txt"])
            self.assertEqual(log["source_sha256"], sha256(self.remote.agent_trace))
            self.assertEqual(len(log["actions"]), 2)
            self.assertIn("研究ASR", json.dumps(log, ensure_ascii=False))
            self.assertEqual(log["actions"][1]["status"], "completed")
            self.assertNotIn("PRIVATE_REASONING_CANARY", json.dumps(source))
            self.assertNotIn("TOOL_OUTPUT_CANARY", json.dumps(source))

    def test_unreadable_required_action_log_is_unknown_without_resampling_agent(self):
        self.enable_action_log()
        self.remote.agent_trace = b'{"type":"turn.completed"}\nmalformed event\n'
        row, = self.evaluate()
        self.assertEqual(row["quality_status"], "unknown")
        self.assertIsNone(row["score"])
        self.assertEqual(list(self.remote.roles.values()), ["trial", "grader"])

    def test_unrequested_action_log_does_not_change_legacy_scoring_context(self):
        row, = self.evaluate()
        record = json.loads((self.output / row["quality_evidence_ref"]).read_text())
        self.assertNotIn("tool-log.txt", record["context"])

    def test_missing_required_verifier_observation_stays_unknown_without_extra_agent_or_judges(self):
        path = self.root / "evaluation/rubric.json"
        rubric = json.loads(path.read_bytes())
        rubric["criteria"][0]["context_paths"].append("verifier/runtime.json")
        path.write_bytes(canonical(rubric))
        self.start_engine("missing-verifier-observation")
        row, = self.evaluate()
        self.assertEqual(row["quality_status"], "unknown")
        self.assertIsNone(row["task_quality"])
        self.assertEqual(list(self.remote.roles.values()), ["trial", "grader"])

    def test_continuous_scores_reach_feedback_and_report_with_their_unit(self):
        from test_quality import continuous_rubric, continuous_calibration
        (self.root / "evaluation/rubric.json").write_bytes(canonical(continuous_rubric()))
        path = self.root / "evaluation/calibration.json"
        path.write_bytes(canonical(continuous_calibration(json.loads(path.read_bytes()))))
        self.start_engine("continuous-trials")
        (self.output / "runner.py").write_bytes(b"recorded remote fixture")
        self.remote.runner_hash = sha256(b"recorded remote fixture")
        self.remote.levels = (73.42, 81.64)
        row, = self.evaluate()
        self.assertEqual(row["quality_score"], 77.53)
        feedback, = self.engine.feedback()["results"]
        self.assertEqual(feedback["quality_score"], 77.53)
        self.assertEqual(feedback["quality_components"]["boundary"]["quality_score"], 73.42)
        self.engine.finish("inconclusive", "controlled numeric pipeline only")
        report = json.loads((self.output / "report.json").read_bytes())
        self.assertEqual(report["quality_scoring"], "continuous_100_v1")
        self.assertEqual(report["task_results"][0]["quality_score"], 77.53)
        self.assertIn("77.53", (self.output / "report.md").read_text())
        self.assertIn("0–100", (self.output / "report.md").read_text())
        self.assertIn("boundary_completeness / boundary | 50.00% | 73.42", (self.output / "report.md").read_text())
        from evolution import EvidenceReplay
        records = {}
        for relative in self.state["executions"]:
            execution = json.loads((self.output / relative).read_bytes())
            records[execution["remote_task_id"]] = (execution, (self.output / relative).parent / "artifacts")
        key = next(k.removesuffix(":agent") for k in self.state["steps"] if k.endswith(":agent"))
        replay = EvidenceReplay(self.job, self.output, self.state, records)
        arguments = (key, "known", self.job.spec["cases"][0], "baseline", "search", 0, None)
        self.assertEqual(replay.task_result(*arguments, persist=False), row)
        directory = next(a for e, a in records.values() if (a / "assessment.json").exists())
        value = json.loads((directory / "assessment.json").read_bytes())
        value["criteria"][0]["level"] = 73.43
        value["criteria"][0]["deductions"][0]["points"] = 26.57
        (directory / "assessment.json").write_bytes(canonical(value))
        with self.assertRaises(ConflictError):
            replay.task_result(*arguments, persist=False)

    def test_continuous_baseline_preserves_judge_noise_when_dimensions_cancel(self):
        from test_quality import continuous_rubric, continuous_calibration
        (self.root / "evaluation/rubric.json").write_bytes(canonical(continuous_rubric()))
        path = self.root / "evaluation/calibration.json"
        path.write_bytes(canonical(continuous_calibration(json.loads(path.read_bytes()))))
        self.start_engine("continuous-noise")
        self.remote.level_schedule = [(73, 83), (74, 82), (73, 83), (74, 82)]
        self.engine.evaluate("aa", ["aa_0", "aa_1"], "search")
        diagnostic = self.engine.diagnose_baseline()
        self.assertEqual(diagnostic["judge_noise"]["disagreements"], 0)
        self.assertEqual(diagnostic["judge_noise"]["numeric_differences"], 4)
        self.assertAlmostEqual(diagnostic["judge_noise"]["max_difference"], 0.01)

    def test_continuous_quality_fraction_is_not_a_categorical_auxiliary_position(self):
        from test_quality import continuous_rubric, continuous_calibration
        self.enable_auxiliary()
        (self.root / "evaluation/rubric.json").write_bytes(canonical(continuous_rubric()))
        path = self.root / "evaluation/calibration.json"
        path.write_bytes(canonical(continuous_calibration(json.loads(path.read_bytes()))))
        self.start_engine("continuous-auxiliary-scales")
        self.state["auxiliary_calibration"] = {"boundary": {"eligible": True, "certificate_sha256": "f" * 64}}
        self.remote.levels = (90, 90)
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "fixture-secret"}):
            row, = self.evaluate()
        feedback, = self.engine.feedback()["results"]
        self.assertEqual(row["quality_score"], 90)
        self.assertEqual(feedback["auxiliary_scores"]["boundary"]["normalized_score"], 0.6)
        self.assertEqual(feedback["auxiliary_disagreements"], [])
        self.assertEqual(feedback["auxiliary_comparison_status"], "unmapped_scales")

    def test_original_verifier_and_quality_have_different_scores_and_saved_evidence(self):
        row, = self.evaluate()
        self.assertEqual(row["schema_version"], 2)
        self.assertEqual((row["task_success"], row["grader_score"], row["task_quality"], row["score"]), (1, 1, 0.5, 0.5))
        self.assertEqual(row["tokens"], {"input_tokens": 10, "output_tokens": 2})
        self.assertEqual(row["quality_calibration_sha256"], "e" * 64)
        record = json.loads((self.output / row["quality_evidence_ref"]).read_text())
        self.assertEqual(len(record["assessments"]), 2)
        self.assertEqual(len({a["execution_id"] for a in record["assessments"]}), 2)
        self.assertEqual(len(self.remote.submissions), 4)

    def test_missing_verifier_keeps_the_agent_and_never_resamples_it(self):
        self.remote.grader_gap = True
        row, = self.evaluate()
        self.assertEqual(row["status"], "ok")
        self.assertIsNone(row["task_success"])
        self.assertIsNone(row["score"])
        self.assertEqual(row["quality_status"], "unknown")
        self.assertEqual(list(self.remote.roles.values()).count("trial"), 1)

    def test_bounded_verifier_retry_preserves_the_first_failure_and_one_agent(self):
        self.remote.grader_failures_remaining = 1
        row, = self.evaluate()
        self.assertEqual((row["task_success"], row["score"]), (1, 0.5))
        self.assertEqual(len(row["grading_attempts"]), 2)
        self.assertEqual(list(self.remote.roles.values()).count("trial"), 1)

    def test_no_grader_budget_keeps_the_completed_agent_and_the_confirmation_reserve(self):
        self.state["confirmation_reserve"] = self.job.spec["budget"]["max_trials"] - 1
        row, = self.evaluate()
        self.assertEqual(row["status"], "ok")
        self.assertIsNone(row["task_success"])
        self.assertIsNone(row["score"])
        self.assertEqual(len(self.remote.submissions), 1)

    def test_no_judge_budget_keeps_the_known_original_success(self):
        self.state["confirmation_reserve"] = self.job.spec["budget"]["max_trials"] - 2
        row, = self.evaluate()
        self.assertEqual(row["task_success"], 1)
        self.assertIsNone(row["score"])
        self.assertEqual(len(self.remote.submissions), 2)

    def test_judge_capture_failure_is_a_quality_gap_and_not_an_agent_retry(self):
        self.remote.judge_failures_remaining = 10
        row, = self.evaluate()
        self.assertEqual(row["task_success"], 1)
        self.assertEqual(row["status"], "ok")
        self.assertIsNone(row["score"])
        self.assertEqual(row["quality_status"], "unknown")
        self.assertEqual(list(self.remote.roles.values()).count("trial"), 1)
        self.assertLessEqual(self.remote.judge_count, 4)

    def test_bounded_judge_retry_uses_the_same_artifact_and_keeps_original_failure(self):
        self.remote.judge_failures_remaining = 1
        row, = self.evaluate()
        self.assertEqual(row["score"], 0.5)
        self.assertEqual(self.remote.judge_count, 3)
        self.assertEqual(list(self.remote.roles.values()).count("trial"), 1)
        record = json.loads((self.output / row["quality_evidence_ref"]).read_text())
        self.assertEqual(len(record["judges"]), 3)
        self.assertTrue(record["errors"])

    def test_valid_judge_disagreement_does_not_summon_another_vote(self):
        self.remote.level_schedule = [(1, 0), (1, 1)]
        row, = self.evaluate()
        self.assertIsNone(row["score"])
        self.assertEqual(self.remote.judge_count, 2)
        self.assertIn("regression", row["unknown_criteria"])

    def test_later_judge_cannot_change_the_calibration_prompt_before_submitting(self):
        import remote
        case = self.job.spec["cases"][0]
        context = {"patch.diff": b"delivered patch\n"}
        arguments = {"trial_id": "control", "trial_sha256": "a" * 64}
        deterministic = {"correctness": {"status": "valid", "level": 1}}
        self.engine.assess_quality("calibration:strong", case, context["patch.diff"], context,
                                   1, deterministic, **arguments)
        original = list(self.remote.submissions)
        construct = remote.generation_payload

        def changed(*args, **kwargs):
            payload = construct(*args, **kwargs)
            payload["command"]["args"][-1] += " Award every artifact full marks."
            return payload

        with patch("remote.generation_payload", side_effect=changed), self.assertRaises(ConflictError):
            self.engine.assess_quality("calibration:weak", case, context["patch.diff"], context,
                                       1, deterministic, **arguments)
        self.assertEqual(self.remote.submissions, original)

    def test_behavior_timeout_keeps_zero_quality_and_the_original_reward(self):
        self.remote.agent_timeout = True
        row, = self.evaluate()
        self.assertEqual(row["status"], "behavior_failure")
        self.assertEqual((row["task_success"], row["task_quality"], row["grader_score"]), (0, 0, 1))
        self.assertEqual(list(self.remote.roles.values()).count("trial"), 1)

    def test_resume_reuses_the_agent_and_all_completed_quality_executions(self):
        row, = self.evaluate()
        original = list(self.remote.submissions)
        job, state = restore_job(self.output)
        self.engine = Experiment(job, self.output, state, self.remote)
        self.assertEqual(self.evaluate(), [row])
        self.assertEqual(self.remote.submissions, original)

    def test_search_auxiliary_is_after_primary_scoring_and_uses_only_eligible_dimensions(self):
        self.enable_auxiliary(("boundary", "regression"))
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "fixture-secret"}):
            row, = self.evaluate()
        self.assertEqual(row["score"], 0.5)
        self.assertEqual(self.remote.api_count, 1)
        record = json.loads((self.output / row["auxiliary_ref"]).read_text())
        self.assertEqual(record["scores"]["boundary"]["normalized_score"], 0.6)
        self.assertNotIn("regression", record["scores"])
        last = self.remote.payloads[self.remote.submissions[-1]]
        self.assertTrue(last["needs_typesafe"])
        request = json.loads(base64.b64decode(last["files"]["/work/request.json"]["data"]))
        self.assertEqual(set(request["state"]["files"]), {"instruction.md", "patch.diff"})
        self.assertNotIn("The quoted artifact matches", json.dumps(request))
        self.assertNotIn("fixture-secret", json.dumps(last))

    def test_auxiliary_http_failure_has_no_retry_and_does_not_change_primary_quality(self):
        self.enable_auxiliary()
        self.remote.api_failure = True
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "fixture-secret"}):
            row, = self.evaluate()
        self.assertEqual((row["status"], row["task_success"], row["score"]), ("ok", 1, 0.5))
        self.assertEqual(self.remote.api_count, 1)
        record = json.loads((self.output / row["auxiliary_ref"]).read_text())
        self.assertEqual((record["status"], record["reason"]), ("unavailable", "http_429"))

    def test_auxiliary_cannot_spend_the_remaining_primary_evaluation_budget(self):
        self.enable_auxiliary()
        self.state["confirmation_reserve"] = self.job.spec["budget"]["max_trials"] - 8
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "fixture-secret"}):
            self.engine.evaluate("paired", ["aa_0", "aa_1"], "search")
        rows = self.engine.rows()
        self.assertEqual([row["score"] for row in rows], [0.5, 0.5])
        self.assertEqual(self.remote.api_count, 0)
        record = json.loads((self.output / rows[0]["auxiliary_ref"]).read_text())
        self.assertEqual(record["reason"], "primary_evaluation_budget_reserved")

    def test_completed_missing_key_slot_is_not_backfilled_on_resume(self):
        self.enable_auxiliary()
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": ""}):
            row, = self.evaluate()
        record = json.loads((self.output / row["auxiliary_ref"]).read_text())
        self.assertEqual(record["reason"], "controller_credential_unavailable")
        original = list(self.remote.submissions)
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "fixture-secret"}):
            self.assertEqual(self.evaluate(), [row])
        self.assertEqual(self.remote.api_count, 0)
        self.assertEqual(self.remote.submissions, original)

    def test_uncalibrated_auxiliary_dimension_never_calls_the_provider(self):
        self.enable_auxiliary()
        self.state["auxiliary_calibration"]["boundary"]["eligible"] = False
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "fixture-secret"}):
            row, = self.evaluate()
        record = json.loads((self.output / row["auxiliary_ref"]).read_text())
        self.assertEqual(record["reason"], "auxiliary_dimension_not_calibrated")
        self.assertEqual(self.remote.api_count, 0)
        self.assertEqual(row["score"], 0.5)

    def test_truthy_value_is_not_a_calibrated_auxiliary_certificate(self):
        self.enable_auxiliary()
        self.state["auxiliary_calibration"]["boundary"]["eligible"] = "true"
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "fixture-secret"}):
            row, = self.evaluate()
        self.assertEqual(self.remote.api_count, 0)
        record = json.loads((self.output / row["auxiliary_ref"]).read_text())
        self.assertEqual(record["reason"], "auxiliary_dimension_not_calibrated")

    def test_confirmation_never_invokes_auxiliary_even_with_eligible_dimension(self):
        self.compare()
        self.spec["runtime"]["model"] = "gpt-6.1-sol"
        root = self.root / "evaluation"
        heldout = json.loads((root / "rubric.json").read_text())
        heldout["case_id"] = "confirmation-case"
        for criterion in heldout["criteria"]:
            criterion["public_requirement_ref"] = "instruction.md: " + (self.root / "heldout/instruction.md").read_text()
        (root / "heldout-rubric.json").write_bytes(canonical(heldout))
        self.spec["evaluation"]["paths"].append("heldout-rubric.json")
        self.spec["evaluation"]["rubrics"]["confirmation-case"] = "heldout-rubric.json"
        self.enable_auxiliary()
        with patch.object(self.engine, "auxiliary_score", side_effect=AssertionError("confirmation auxiliary call")):
            row = self.engine.task_result("confirmed", "confirmation", self.job.spec["cases"][1],
                                          "baseline", "confirmation", 0, None)
        self.assertIsNone(row["auxiliary_ref"])
        self.assertEqual(self.remote.api_count, 0)

    def test_api_export_recovery_reuses_all_primary_and_api_execution_ids(self):
        self.enable_auxiliary()
        self.remote.fail_api_export = True
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "fixture-secret"}):
            with self.assertRaises(UnresolvedError):
                self.evaluate()
        original = list(self.remote.submissions)
        self.assertEqual(self.engine.rows(), [])
        self.remote.fail_api_export = False
        job, state = restore_job(self.output)
        self.engine = Experiment(job, self.output, state, self.remote)
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": ""}):
            row, = self.evaluate()
        self.assertEqual(row["score"], 0.5)
        self.assertEqual(self.remote.submissions, original)
        self.assertEqual(self.remote.api_count, 1)

    def test_quality_feedback_exposes_actionable_scores_and_disagreement_without_private_anchors(self):
        self.enable_auxiliary(("boundary", "regression"))
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "fixture-secret"}):
            self.evaluate()
        view = self.engine.feedback()
        result, = view["results"]
        self.assertEqual((result["task_success"], result["task_quality"]), (1, 0.5))
        self.assertEqual(result["auxiliary_status"], "valid")
        self.assertEqual(result["auxiliary_scores"]["boundary"]["normalized_score"], 0.6)
        self.assertIn("regression", result["auxiliary_ineligible_criteria"])
        self.assertEqual(result["auxiliary_disagreements"], [{"criterion_id": "boundary",
                          "primary_level": 1, "auxiliary_band": 0.5}])
        self.assertIn("public_requirement_ref", result["quality_components"]["regression"])
        serialized = json.dumps(view)
        for private in ["private grading command", "private oracle patch", "calibration.json",
                        "The ordinary path works but the documented boundary is broken."]:
            self.assertNotIn(private, serialized)

    def test_v2_report_keeps_actual_scores_and_separates_agent_and_auxiliary_resources(self):
        self.enable_auxiliary()
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "fixture-secret"}):
            row, = self.evaluate()
        report = self.engine.finish("inconclusive", "Insufficient independent groups in recorded fixture.")
        self.assertEqual(report["task_results"][0]["task_quality"], 0.5)
        self.assertEqual(report["task_results"][0]["task_success"], 1)
        self.assertEqual(report["task_results"][0]["quality_evidence_ref"], row["quality_evidence_ref"])
        self.assertEqual(report["resource_summary"]["by_role"]["task_agent"], 1)
        self.assertEqual(report["resource_summary"]["by_role"]["quality_judge"], 2)
        self.assertEqual(report["resource_summary"]["api_usage"], {"input_tokens": 100, "output_tokens": 4})
        self.assertEqual(report["resource_summary"]["agent_usage"], {"input_tokens": 10, "output_tokens": 2})
        self.assertEqual(report["resource_summary"]["evaluation_usage"]["input_tokens"], 40)
        self.assertEqual(report["resource_summary"]["evaluation_usage"]["cached_input_tokens"], 10)
        self.assertIsNone(report["cost"])
        text = (self.output / "report.md").read_text()
        self.assertIn("0.5", text)
        self.assertIn("task_quality", text)

    def test_unavailable_auxiliary_feedback_keeps_complete_primary_feedback(self):
        self.enable_auxiliary()
        self.remote.api_failure = True
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "fixture-secret"}):
            self.evaluate()
        result, = self.engine.feedback()["results"]
        self.assertEqual(result["task_quality"], 0.5)
        self.assertEqual(result["quality_components"]["regression"]["level"], 0)
        self.assertEqual((result["auxiliary_status"], result["auxiliary_reason"]), ("unavailable", "http_429"))
        self.assertEqual(result["auxiliary_scores"], {})

    def test_corrupt_auxiliary_record_is_not_forwarded_or_replaced(self):
        self.enable_auxiliary()
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "fixture-secret"}):
            row, = self.evaluate()
        path = self.output / row["auxiliary_ref"]
        record = json.loads(path.read_text())
        record["scores"]["boundary"]["normalized_score"] = 0.99
        path.write_bytes(canonical(record))
        result, = self.engine.feedback()["results"]
        self.assertEqual(result["task_quality"], 0.5)
        self.assertEqual(result["auxiliary_status"], "unknown")
        self.assertEqual(result["auxiliary_reason"], "auxiliary_evidence_unavailable")
        self.assertEqual(result["auxiliary_scores"], {})
        self.assertEqual(self.remote.api_count, 1)


class SearchStateTests(JobFixture):
    def setUp(self):
        super().setUp()
        self.compare()
        self.spec["mode"] = "search"
        self.spec["conditions"] = self.spec["conditions"][:1]
        del self.spec["candidate_condition"]
        self.spec["budget"]["max_candidates"] = 2
        method = self.root / "method.md"
        method.write_text("Propose a minimal general instruction change.")
        from protocol import sha256

        self.spec["optimizer"] = {
            "model": "fixed-optimizer",
            "effort": "high",
            "method_path": str(method),
            "method_sha256": sha256(method.read_bytes()),
        }
        self.job = prepare(self.write())
        self.output = self.root / "search"
        self.state = freeze(self.job, self.output)
        self.generated = 0
        self.evaluated = []

    def engine(self, state=None, job=None):
        engine = Experiment(
            job or self.job, self.output, state or self.state, RecordedRemote()
        )
        engine.calibrate = lambda: None
        engine.evaluate = lambda comparison, conditions, split: self.evaluated.append(
            (comparison, conditions, split)
        )
        engine.policy_check = lambda candidate: {"passed": True, "checks": []}
        engine.diagnose = lambda: {
            "bucket": "insufficient_evidence",
            "reason": "known fixture plateau",
        }

        def generate(parent):
            self.generated += 1
            import difflib

            old = parent["AGENTS.md"]["content"].decode()
            new = (
                "Preserve authorization.\nUse scoped commands "
                + str(self.generated)
                + ".\n"
            )
            patch = "".join(
                difflib.unified_diff(
                    old.splitlines(True),
                    new.splitlines(True),
                    fromfile="a/AGENTS.md",
                    tofile="b/AGENTS.md",
                )
            ).encode()
            return {
                "parent_hash": bundle_hash(parent),
                "hypothesis": "Clarify command scope",
                "expected_effect": "Fewer scope mistakes",
                "counterexample": "A scope mistake still occurs",
                "changed_paths": ["AGENTS.md"],
            }, patch

        engine.generate = generate
        return engine

    def test_no_winner_ends_normally_and_does_not_expose_confirmation(self):
        engine = self.engine()
        engine.comparison = lambda *args: {
            "verdict": "inconclusive",
            "reason": "fixture uncertainty",
        }
        report = engine.run()
        self.assertEqual(self.generated, 2)
        self.assertEqual(self.state["confirmation_status"], "not_run")
        self.assertFalse(self.state["confirmation_exposed"])
        self.assertEqual(self.state["search_best"], self.state["initial_baseline"])
        self.assertEqual(report["verdict"], "inconclusive")
        self.assertTrue((self.output / "diagnosis.json").exists())

    def test_keep_updates_exploration_then_confirms_only_one_frozen_candidate(self):
        engine = self.engine()
        engine.comparison = lambda *args: {
            "verdict": "keep",
            "reason": "known fixture improvement",
        }
        report = engine.run()
        confirmation = [call for call in self.evaluated if call[2] == "confirmation"]
        self.assertEqual(len(confirmation), 1)
        self.assertEqual(self.state["confirmation_status"], "passed")
        self.assertEqual(self.state["confirmed_best"], self.state["frozen_candidate"])
        self.assertEqual(report["verdict"], "keep")

    def test_failed_confirmation_preserves_original_when_nothing_was_confirmed(self):
        engine = self.engine()
        engine.comparison = lambda comparison, *args: {
            "verdict": "discard" if comparison == "confirmation" else "keep",
            "reason": "fixture",
        }
        engine.run()
        self.assertEqual(self.state["confirmation_status"], "failed")
        self.assertIsNone(self.state["confirmed_best"])
        self.assertTrue(self.state["confirmation_exposed"])

    def test_aa_noise_stops_before_any_candidate_generation(self):
        engine = self.engine()
        engine.comparison = lambda *args: {
            "verdict": "inconclusive",
            "reason": "fixture noise",
            "group_differences": [{"quality_difference": 1, "weight": 1}],
        }
        result = engine.run()
        self.assertEqual(self.generated, 0)
        self.assertEqual(result["reason"], "baseline_noise_or_invalid_pairs")

    def test_confirmation_interruption_resumes_the_same_frozen_candidate(self):
        engine = self.engine()
        engine.comparison = lambda *args: {"verdict": "keep", "reason": "fixture"}
        original = engine.evaluate

        def interrupted(comparison, conditions, split):
            if split == "confirmation":
                raise UnresolvedError("fixture confirmation interruption")
            original(comparison, conditions, split)

        engine.evaluate = interrupted
        with self.assertRaises(UnresolvedError):
            engine.run()
        frozen = self.state["frozen_candidate"]
        self.assertTrue(self.state["confirmation_exposed"])
        job, state = restore_job(self.output)
        resumed = self.engine(state, job)
        resumed.comparison = lambda *args: {"verdict": "keep", "reason": "fixture"}
        resumed.run()
        self.assertEqual(state["confirmed_best"], frozen)
        self.assertEqual(self.generated, 2)

    def test_resume_does_not_generate_the_same_candidate_or_repeat_confirmation(self):
        engine = self.engine()
        engine.comparison = lambda *args: {"verdict": "keep", "reason": "fixture"}
        original = engine.evaluate

        def interrupted(comparison, conditions, split):
            if comparison.startswith("attempt"):
                raise UnresolvedError("fixture interruption after candidate snapshot")
            return original(comparison, conditions, split)

        engine.evaluate = interrupted
        with self.assertRaises(UnresolvedError):
            engine.run()
        self.assertEqual(self.generated, 1)
        job, state = restore_job(self.output)
        resumed = self.engine(state, job)
        resumed.comparison = lambda *args: {"verdict": "keep", "reason": "fixture"}
        resumed.run()
        count = self.generated
        resumed.run()
        self.assertEqual(self.generated, count)
        self.assertEqual(count, 2)
        self.assertEqual(
            len([call for call in self.evaluated if call[2] == "confirmation"]), 1
        )


if __name__ == "__main__":
    import unittest

    unittest.main()
