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
from protocol import prepare, bundle_hash
from remote import UnresolvedError


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
