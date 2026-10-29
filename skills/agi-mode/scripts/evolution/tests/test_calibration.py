from test_quality import QualityFixture, rubric, assessment
from protocol import ProtocolError, canonical, prepare
from quality import aggregate_quality, calibration_certificate, validate_calibration
from evolution import Experiment, freeze, write_json
import base64
import json
import difflib
import subprocess
import sys
import unittest
from copy import deepcopy
import compare
from support import JobFixture


class CalibrationTests(QualityFixture):
    def setUp(self):
        super().setUp()
        self.evaluation = self.quality_spec()

    def manifest(self):
        import json
        return json.loads((self.evaluation / "calibration.json").read_bytes())

    def results(self):
        result = {}
        for artifact in self.manifest()["artifacts"]:
            levels = artifact["expected_levels"]
            a, _, _ = assessment((levels["boundary"], levels["regression"]))
            result[artifact["artifact_id"]] = aggregate_quality(
                rubric(), [a, {**a, "execution_id": "judge-b"}],
                task_success=artifact["expected_success"], deterministic={
                    "correctness": {"status": "valid", "level": levels["correctness"]}})
        return result

    def test_continuous_calibration_preserves_close_degrees_and_rejects_reversed_order(self):
        from test_quality import continuous_rubric, continuous_assessment
        r, manifest, results = continuous_rubric(), self.manifest(), {}
        for artifact in manifest["artifacts"]:
            name = artifact["artifact_id"]
            if name in {"weak", "strong", "renamed"}:
                artifact["expected_levels"]["regression"] = 0.8342 if name == "weak" else 0.8542
            levels = artifact["expected_levels"]
            a, _, _ = continuous_assessment((100 * levels["boundary"], 100 * levels["regression"]))
            results[name] = aggregate_quality(r, [a, {**a, "execution_id": "judge-b"}],
                task_success=artifact["expected_success"], deterministic={
                    "correctness": {"status": "valid", "level": levels["correctness"]}})
        options = {"rubrics": {"search-case": r}}
        certificate = calibration_certificate(manifest, results, {"protocol": "frozen"}, **options)
        self.assertEqual(certificate["status"], "passed")
        self.assertNotEqual(results["weak"]["quality_score"], results["strong"]["quality_score"])
        a, _, _ = continuous_assessment((100, 86))
        results["weak"] = aggregate_quality(r, [a, {**a, "execution_id": "judge-b"}], task_success=1,
            deterministic={"correctness": {"status": "valid", "level": 1}})
        self.assertEqual(calibration_certificate(manifest, results, {"protocol": "frozen"}, **options)["status"], "failed")

    def test_continuous_reference_of_complete_cannot_accept_an_invented_gap(self):
        from test_quality import continuous_rubric, continuous_assessment
        r, manifest, results = continuous_rubric(), self.manifest(), {}
        for artifact in manifest["artifacts"]:
            levels = artifact["expected_levels"]
            a, _, _ = continuous_assessment((100 * levels["boundary"], 100 * levels["regression"]))
            results[artifact["artifact_id"]] = aggregate_quality(r, [a, {**a, "execution_id": "judge-b"}],
                task_success=artifact["expected_success"], deterministic={
                    "correctness": {"status": "valid", "level": levels["correctness"]}})
        for name in ("strong", "renamed"):
            a, _, _ = continuous_assessment((95, 95))
            results[name] = aggregate_quality(r, [a, {**a, "execution_id": "judge-b"}], task_success=1,
                deterministic={"correctness": {"status": "valid", "level": 1}})
        self.assertEqual(calibration_certificate(manifest, results, {"protocol": "frozen"},
            rubrics={"search-case": r})["status"], "failed")

    def test_fixed_budget_reference_rejects_agreed_incorrect_partial_scores(self):
        from test_quality import continuous_rubric, continuous_assessment
        r, manifest, results = continuous_rubric(), self.manifest(), {}
        next(a for a in manifest["artifacts"] if a["artifact_id"] == "weak")["expected_levels"]["boundary"] = 0.875
        for artifact in manifest["artifacts"]:
            levels = artifact["expected_levels"]
            a, _, _ = continuous_assessment((100 * levels["boundary"], 100 * levels["regression"]))
            results[artifact["artifact_id"]] = aggregate_quality(r, [a, {**a, "execution_id": "judge-b"}],
                task_success=artifact["expected_success"], deterministic={
                    "correctness": {"status": "valid", "level": levels["correctness"]}})
        options = {"rubrics": {"search-case": r}}
        correct = deepcopy(results["weak"])
        a, _, _ = continuous_assessment((91.25, 0))
        results["weak"] = aggregate_quality(r, [a, {**a, "execution_id": "judge-b"}], task_success=1,
            deterministic={"correctness": {"status": "valid", "level": 1}})
        self.assertEqual(calibration_certificate(manifest, results, {}, **options)["status"], "passed")
        r["criteria"][0]["exact_score_agreement"] = True
        certificate = calibration_certificate(manifest, results, {}, **options)
        self.assertEqual(certificate["status"], "failed")
        self.assertIn("weak: anchor_reference_mismatch:boundary", certificate["errors"])
        results["weak"] = correct
        self.assertEqual(calibration_certificate(manifest, results, {}, **options)["status"], "passed")

    def test_all_risk_controls_bind_artifacts_protocol_and_environment(self):
        manifest = self.manifest()
        job = prepare(self.write())
        validate_calibration(manifest, job.evaluation_files, {"search-case": rubric()})
        identity = {"protocol_hash": job.protocol_hash, "runtime_hash": "a" * 64}
        certificate = calibration_certificate(manifest, self.results(), identity)
        self.assertEqual(certificate["status"], "passed")
        changed = calibration_certificate(manifest, self.results(), {**identity, "runtime_hash": "b" * 64})
        self.assertNotEqual(changed["certificate_sha256"], certificate["certificate_sha256"])

    def test_declared_unobservable_soft_item_is_limited_to_a_known_failed_control(self):
        from test_quality import continuous_rubric, continuous_assessment, continuous_calibration
        r, manifest, results = continuous_rubric(), continuous_calibration(self.manifest()), {}
        next(v for v in manifest['relations'] if v['kind'] == 'injection')['criterion_ids'] = ['boundary']
        job = prepare(self.write())
        for artifact in manifest['artifacts']:
            levels = artifact['expected_levels']
            a, _, _ = continuous_assessment((100 * levels['boundary'], 100 * levels['regression']))
            if artifact['artifact_id'] == 'bad':
                a['status'] = 'unknown'
                a['criteria'][1].update(status='unknown', level=None, deductions=[], evidence=[],
                                        reason='The failing entrypoint prevents downstream observation.')
                artifact['expected_unknown_criteria'] = ['regression']
                del levels['regression']
            results[artifact['artifact_id']] = aggregate_quality(r, [a, {**deepcopy(a), 'execution_id': 'judge-b'}],
                task_success=artifact['expected_success'], deterministic={
                    'correctness': {'status': 'valid', 'level': levels['correctness']}})
        validate_calibration(manifest, job.evaluation_files, {'search-case': r})
        options = {'rubrics': {'search-case': r}}
        certificate = calibration_certificate(manifest, results, {'protocol': 'frozen'}, **options)
        self.assertEqual(certificate['status'], 'passed', certificate['errors'])
        self.assertIsNone(results['bad']['quality_components']['regression']['level'])
        self.assertEqual(results['bad']['task_quality'], 0)
        for change in ('success', 'gate', 'duplicate', 'overlap', 'relation', 'legacy', 'auxiliary'):
            value = deepcopy(manifest)
            bad = next(a for a in value['artifacts'] if a['artifact_id'] == 'bad')
            rubrics, kwargs = {'search-case': r}, {}
            if change == 'success':
                bad.update(expected_success=1)
                bad['expected_levels']['correctness'] = 1
            elif change == 'gate':
                bad['expected_unknown_criteria'] = ['correctness']
                del bad['expected_levels']['correctness']
                bad['expected_levels']['regression'] = 0
            elif change == 'duplicate':
                bad['expected_unknown_criteria'] *= 2
            elif change == 'overlap':
                bad['expected_levels']['regression'] = 0
            elif change == 'relation':
                next(v for v in value['relations'] if v['kind'] == 'rejection')['criterion_ids'] = ['regression']
            elif change == 'legacy':
                rubrics = {'search-case': rubric()}
            else:
                kwargs = {'auxiliary': True, 'criterion_ids': ['regression']}
            with self.subTest(change=change), self.assertRaises(ProtocolError):
                validate_calibration(value, job.evaluation_files, rubrics, **kwargs)
        for reason in ('judge_disagreement_or_missing_evidence', 'missing_checker_evidence'):
            changed = deepcopy(results)
            changed['bad']['quality_components']['regression']['reason'] = reason
            self.assertEqual(calibration_certificate(manifest, changed, {}, **options)['status'], 'failed')
        changed = deepcopy(results)
        changed['bad']['quality_components']['correctness'].update(status='unknown', level=None)
        self.assertEqual(calibration_certificate(manifest, changed, {}, **options)['status'], 'failed')

    def test_weak_proxy_injection_unknown_and_always_bad_judges_fail(self):
        for name, mutation in [
            ("weak", lambda r: r["quality_components"]["regression"].update(level=1)),
            ("injected", lambda r: r["quality_components"]["boundary"].update(level=1)),
            ("renamed", lambda r: r.update(quality_status="unknown")),
            ("strong", lambda r: r.update(task_quality=0)),
            ("bad", lambda r: r.update(task_quality=1)),
        ]:
            results = self.results()
            mutation(results[name])
            certificate = calibration_certificate(self.manifest(), results, {"protocol": "frozen"})
            self.assertEqual(certificate["status"], "failed", name)

    def test_calibration_cannot_skip_risk_families_or_invent_reference_order(self):
        job = prepare(self.write())
        for mutate in (
            lambda m: m["relations"].pop(),
            lambda m: m["artifacts"][0].update(expected_success=0),
            lambda m: m["artifacts"][0]["expected_levels"].update(regression=1),
            lambda m: m["artifacts"][0].update(patch_path="outside.patch"),
            lambda m: next(r for r in m["relations"] if r["kind"] == "injection").update(injected_level=0),
            lambda m: m["relations"][0].update(injected_level=1),
        ):
            manifest = self.manifest()
            mutate(manifest)
            with self.assertRaises(ProtocolError):
                validate_calibration(manifest, job.evaluation_files, {"search-case": rubric()})

    def experiment(self, *, corrupt=False):
        job = prepare(self.write())
        output = self.root / "run"
        state = freeze(job, output)
        remote = type("Remote", (), {"runner_hash": "a" * 64})()
        experiment = Experiment(job, output, state, remote)
        calls = []

        def artifacts(key, role, payload, **kwargs):
            calls.append((key, role))
            name = key.split(":")[1]
            artifact = next(a for a in self.manifest()["artifacts"] if a["artifact_id"] == name)
            directory = output / "executions" / str(len(calls)) / "artifacts"
            directory.mkdir(parents=True)
            write_json(directory / "capture.json", {"gaps": []})
            if key.endswith(":verifier"):
                (directory / "reward.txt").write_text(str(artifact["expected_success"]))
                exit_code = 0 if artifact["expected_success"] else 1
            else:
                source = json.loads(base64.b64decode(payload["files"]["/work/source.json"]["data"]))
                from test_quality import evidence
                items = [{"criterion_id": c["criterion_id"], "status": "valid",
                          "level": artifact["expected_levels"][c["criterion_id"]],
                          "evidence": [evidence("patch.diff", source["files"]["patch.diff"])],
                          "reason": "Matches the frozen anchor."} for c in source["criteria"]]
                if corrupt and name == "weak":
                    items[1]["evidence"][0]["quote"] = "forged result"
                write_json(directory / "assessment.json", {"status": "valid", "criteria": items})
                exit_code = 0
            return {"remote_task_id": "execution-" + str(len(calls)), "exit_code": exit_code}, directory

        experiment.artifacts = artifacts
        return experiment, calls

    def test_calibration_runs_two_fresh_judges_without_optimizer_and_resumes_without_calls(self):
        experiment, calls = self.experiment()
        experiment.calibrate_quality()
        self.assertEqual(experiment.state["calibration_status"], "passed")
        self.assertEqual(len(calls), 15)
        self.assertTrue(all(role == "grader" for key, role in calls))
        experiment.calibrate_quality()
        self.assertEqual(len(calls), 15)

    def test_forged_judge_evidence_stops_before_baseline_or_optimizer(self):
        experiment, calls = self.experiment(corrupt=True)
        experiment.calibrate = experiment.calibrate_quality
        result = experiment.run()
        self.assertEqual(result["verdict"], "invalid")
        self.assertEqual(result["reason"], "quality_calibration_failed")
        self.assertFalse(experiment.state.get("aa_done"))
        self.assertFalse(experiment.state["attempts"])

    def test_missing_baseline_evidence_stops_all_modes_before_candidate_generation(self):
        experiment, calls = self.experiment()
        experiment.state.update(calibration_passed=True, calibration_status="passed")
        experiment.evaluate = lambda *args: None
        experiment.generate = lambda *args: self.fail("optimizer must not run with missing baseline evidence")
        for mode in ("calibrate", "compare", "search"):
            experiment.state["next_step"] = "probe"
            experiment.state["baseline_diagnostic"] = None
            experiment.job.spec["mode"] = mode
            result = experiment.run()
            self.assertEqual(result["reason"], "baseline_quality_unknown")
            self.assertTrue(experiment.state.get("aa_done"))
            self.assertEqual(calls, [])

    def test_diagnostic_rejections_never_invoke_an_optimizer(self):
        experiment, calls = self.experiment()
        experiment.job.spec["mode"] = "search"
        experiment.state.update(calibration_passed=True, calibration_status="passed", aa_done=True)
        experiment.generate = lambda *args: self.fail("optimizer must remain behind the diagnostic gate")
        for reason in ("quality_headroom_absent", "measurement_noise_too_large", "precision_budget_insufficient"):
            experiment.state.update(next_step="baseline_aa", baseline_diagnostic=None)
            experiment.diagnose_baseline = lambda reason=reason: {"proceed": False, "reason": reason}
            report = experiment.run()
            self.assertEqual(report["reason"], reason)
            self.assertEqual(experiment.state["candidates_used"], 0)
            self.assertEqual(calls, [])

    def test_original_boolean_checks_remain_real_gates_and_nonbinary_values_are_unknown(self):
        value = rubric()
        value["criteria"][-1]["checker"]["name"] = "authorization"
        (self.evaluation / "rubric.json").write_bytes(canonical(value))
        experiment, _ = self.experiment()
        execution, directory = experiment.artifacts("calibration:strong:verifier", "grader", {})
        for raw, expected in [(True, 1), (False, 0), (0.5, None)]:
            write_json(directory / "checks.json", {"authorization": raw})
            deterministic, _ = experiment.deterministic_quality("replay", experiment.job.spec["cases"][0], b"", execution, directory)
            a, _, _ = assessment()
            result = aggregate_quality(value, [a, {**a, "execution_id": "judge-b"}],
                                       task_success=1, deterministic=deterministic)
            self.assertEqual(result["task_quality"], expected)


class BaselineDiagnosticTests(unittest.TestCase):
    def baseline(self, groups=40, repeats=1):
        from test_compare import QualityCompareTests
        spec, records, _ = QualityCompareTests().experiment()
        spec["mode"] = "calibrate"
        spec["cases"] = spec["cases"][:groups]
        names = {case["case_id"] for case in spec["cases"]}
        spec["measurement"]["repeats"] = repeats
        rows = []
        for original in records:
            if original["case_id"] not in names:
                continue
            for repeat in range(repeats):
                row = deepcopy(original)
                row.update(purpose="task", candidate_id="aa_0" if original["candidate_id"] == "a" else "aa_1",
                           candidate_hash="a" * 64, comparison_id="aa", repeat=repeat,
                           trial_id=original["trial_id"] + ":" + str(repeat), score=0.5, task_quality=0.5)
                row["quality_components"]["regression"]["level"] = 0
                rows.append(row)
        hashes = {"aa_0": "a" * 64, "aa_1": "a" * 64}
        return spec, rows, hashes

    def diagnose(self, spec, rows, hashes, *, remaining=10000, noise=None):
        report = compare.compare_results(spec, rows, "aa_0", "aa_1", hashes,
                                        split="search", calibration_sha256="e" * 64)
        return compare.baseline_diagnostic(spec, rows, report,
            judge_noise=noise or {"status": "valid", "disagreements": 0, "max_difference": 0},
            budget={"remaining_calls": remaining, "required_calls": 100, "confirmation_reserve": 40})

    def test_stable_actionable_quality_headroom_can_proceed_with_finite_precision(self):
        spec, rows, hashes = self.baseline()
        result = self.diagnose(spec, rows, hashes)
        self.assertTrue(result["proceed"])
        self.assertEqual(result["headroom"]["maximum_gain"], 0.5)
        self.assertEqual(result["generation_noise"]["weighted_absolute_difference"], 0)
        self.assertEqual(result["precision"]["search"]["independent_groups"], 40)
        self.assertGreater(result["precision"]["search"]["optimistic_lower_bound"], 0.05)

    def test_saturated_quality_does_not_become_a_cost_objective(self):
        spec, rows, hashes = self.baseline()
        for row in rows:
            row.update(score=1, task_quality=1)
            row["quality_components"]["regression"]["level"] = 1
        result = self.diagnose(spec, rows, hashes)
        self.assertFalse(result["proceed"])
        self.assertEqual(result["reason"], "quality_headroom_absent")

    def test_generation_noise_and_judge_disagreement_stop_search(self):
        spec, rows, hashes = self.baseline()
        for row in rows:
            if row["candidate_id"] == "aa_1":
                row.update(score=1, task_quality=1)
                row["quality_components"]["regression"]["level"] = 1
        self.assertEqual(self.diagnose(spec, rows, hashes)["reason"], "measurement_noise_too_large")
        spec, rows, hashes = self.baseline()
        noise = {"status": "unknown", "disagreements": 1, "max_difference": 0.5}
        self.assertEqual(self.diagnose(spec, rows, hashes, noise=noise)["reason"], "measurement_noise_too_large")

    def test_repeats_and_insufficient_budget_do_not_substitute_for_independent_groups(self):
        spec, rows, hashes = self.baseline(groups=1, repeats=12)
        result = self.diagnose(spec, rows, hashes)
        self.assertEqual(result["reason"], "precision_budget_insufficient")
        self.assertEqual(result["precision"]["search"]["independent_groups"], 1)
        spec, rows, hashes = self.baseline()
        self.assertEqual(self.diagnose(spec, rows, hashes, remaining=10)["reason"], "precision_budget_insufficient")

    def test_missing_quality_keeps_the_diagnostic_unknown(self):
        spec, rows, hashes = self.baseline()
        rows[0].update(score=None, task_quality=None, quality_status="unknown", unknown_criteria=["regression"])
        rows[0]["quality_components"]["regression"].update(status="unknown", level=None)
        result = self.diagnose(spec, rows, hashes)
        self.assertFalse(result["proceed"])
        self.assertEqual(result["reason"], "baseline_quality_unknown")

    def test_invalid_trial_does_not_produce_a_numeric_baseline_claim(self):
        spec, rows, hashes = self.baseline()
        rows[0].update(score="invalid", task_quality="invalid")
        result = self.diagnose(spec, rows, hashes)
        self.assertEqual(result["reason"], "baseline_quality_unknown")
        self.assertIsNone(result["headroom"]["baseline_quality"])


class RegressionReplayTests(JobFixture):
    def test_replay_runs_a_new_module_selected_by_exact_path_or_pattern(self):
        from remote import regression_payload

        self.put("src/app.py", "def double(value):\n    return value\n")
        self.git("add", ".")
        self.git("-c", "user.name=Test", "-c", "user.email=test@invalid", "-c", "core.hooksPath=/dev/null", "commit", "-qm", "Defect fixture")
        self.base = self.git("rev-parse", "HEAD").strip()
        self.spec["target"]["base_commit"] = self.base
        self.spec["cases"][0]["base_commit"] = self.base
        self.spec["cases"][0]["source"]["revision"] = self.base
        job = prepare(self.write())
        added = "import unittest\nfrom src.app import double\n\nclass NewTests(unittest.TestCase):\n    def test_nonzero(self):\n        self.assertEqual(double(1), 2)\n"
        source = "".join(difflib.unified_diff("def double(value):\n    return value\n".splitlines(True),
            "def double(value):\n    return value * 2\n".splitlines(True), fromfile="a/src/app.py", tofile="b/src/app.py"))
        source += "diff --git a/test_added.py b/test_added.py\nnew file mode 100644\n"
        source += "".join(difflib.unified_diff([], added.splitlines(True), fromfile="/dev/null", tofile="b/test_added.py"))
        for index, selector in enumerate(["test_added.py", "test_*.py"]):
            with self.subTest(selector=selector):
                self.git("reset", "--hard", self.base)
                self.git("clean", "-fd")
                checker = {"test_paths": [selector], "argv": [sys.executable, "-m", "unittest", "test_added"],
                    "expected_failure": {"exit_code": 1, "contains": "AssertionError: 1 != 2"}}
                payload = regression_payload(job, job.spec["cases"][0], source.encode(), checker)
                directory = self.root / ("new-test-" + str(index))
                for name in ["input", "control", "logs"]:
                    (directory / name).mkdir(parents=True)
                (directory / "input/model.patch").write_text(source)
                (directory / "input/checker.json").write_bytes(canonical(checker))
                script = payload["command"]["args"][-1]
                for old, new in [("/evolution-input", directory / "input"), ("/logs/verifier", directory / "logs")]:
                    script = script.replace(old, str(new))
                executed = subprocess.run([sys.executable, "-c", script], cwd=self.repo, capture_output=True, text=True)
                self.assertEqual(executed.returncode, 0, executed.stderr)
                result = json.loads((directory / "logs/regression-replay.json").read_bytes())
                self.assertEqual(result["level"], 1, result)
                self.assertEqual(result["original"]["exit_code"], 1)
                self.assertEqual(result["fixed"]["exit_code"], 0)

    def test_replay_requires_the_original_defect_to_fail_and_repair_to_pass(self):
        from remote import regression_payload

        self.put("src/app.py", "def double(value):\n    return value\n")
        old_test = "import unittest\nfrom src.app import double\n\nclass DoubleTests(unittest.TestCase):\n    def test_zero(self):\n        self.assertEqual(double(0), 0)\n"
        self.put("test_app.py", old_test)
        self.git("add", ".")
        self.git("-c", "user.name=Test", "-c", "user.email=test@invalid", "-c", "core.hooksPath=/dev/null", "commit", "-qm", "Defect fixture")
        self.base = self.git("rev-parse", "HEAD").strip()
        self.spec["target"]["base_commit"] = self.base
        self.spec["cases"][0]["base_commit"] = self.base
        self.spec["cases"][0]["source"]["revision"] = self.base
        job = prepare(self.write())
        checker = {"test_paths": ["test_app.py"], "argv": [sys.executable, "-m", "unittest", "test_app"],
                   "expected_failure": {"exit_code": 1, "contains": "AssertionError: 1 != 2"}}
        for kind, test in [
            ("valid", old_test + "\n    def test_nonzero(self):\n        self.assertEqual(double(1), 2)\n"),
            ("proxy", old_test + "\n    def test_zero_again(self):\n        self.assertEqual(double(0), 0)\n"),
            ("environment", old_test + "\nimport nonexistent_evaluation_dependency\n"),
        ]:
            self.git("reset", "--hard", self.base)
            source = "".join(difflib.unified_diff("def double(value):\n    return value\n".splitlines(True),
                            "def double(value):\n    return value * 2\n".splitlines(True), fromfile="a/src/app.py", tofile="b/src/app.py"))
            source += "".join(difflib.unified_diff(old_test.splitlines(True), test.splitlines(True),
                                                fromfile="a/test_app.py", tofile="b/test_app.py"))
            payload = regression_payload(job, job.spec["cases"][0], source.encode(), checker)
            directory = self.root / kind
            directory.mkdir()
            for name in ["input", "control", "logs"]:
                (directory / name).mkdir()
            (directory / "input/model.patch").write_text(source)
            (directory / "input/checker.json").write_bytes(canonical(checker))
            (directory / "control/base-tree.txt").write_text(self.git("write-tree").strip())
            script = payload["command"]["args"][-1]
            for old, new in [("/evolution-input", directory / "input"), ("/evolution-control", directory / "control"),
                             ("/logs/verifier", directory / "logs")]:
                script = script.replace(old, str(new))
            # The real worker cannot read the root-owned control namespace.
            barrier = """
import pathlib
original_read = pathlib.Path.read_text
def read_public(path, *args, **kwargs):
    if path.is_relative_to(pathlib.Path(CONTROL)):
        raise PermissionError('control namespace is private')
    return original_read(path, *args, **kwargs)
pathlib.Path.read_text = read_public
""".replace("CONTROL", repr(str(directory / "control")))
            executed = subprocess.run([sys.executable, "-c", barrier + script], cwd=self.repo, capture_output=True, text=True)
            self.assertEqual(executed.returncode, 0, executed.stderr)
            result = json.loads((directory / "logs/regression-replay.json").read_bytes())
            if kind == "valid":
                self.assertEqual(result["level"], 1)
                self.assertEqual(result["original"]["exit_code"], 1)
                self.assertEqual(result["fixed"]["exit_code"], 0)
            elif kind == "proxy":
                self.assertEqual(result["level"], 0)
            else:
                self.assertEqual(result["status"], "unknown")
