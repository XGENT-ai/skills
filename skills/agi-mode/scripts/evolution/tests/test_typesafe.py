import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from urllib.error import HTTPError, URLError

from test_quality import QualityFixture
from protocol import ProtocolError, canonical, sha256
from quality import (build_auxiliary_request, validate_auxiliary, auxiliary_certificate,
                     auxiliary_assessment, validate_auxiliary_assessment, context_manifest)
from typesafe_worker import run_request


def auxiliary_rubric():
    return {"criteria": {"boundary": {
        "dimension": "boundary_completeness",
        "instructions": "Judge whether `files.patch.diff` repairs the public ordinary and boundary paths in `files.instruction.md`.",
        "anchors": {"0": "The public ordinary path remains broken.",
                    "0.5": "The ordinary path works but the documented boundary path remains broken.",
                    "1": "The ordinary path and the documented boundary path both work."},
        "context_paths": ["patch.diff", "instruction.md"],
    }}}


def request():
    return build_auxiliary_request(auxiliary_rubric(),
                                   {"patch.diff": b"a repair\n", "instruction.md": b"Fix ordinary and boundary paths.\n"},
                                   ["boundary"])


def response(score=1.5, probabilities=None):
    return {"model": "jev-1.14.0", "answers": {"boundary": {
        "type": "score", "score": score,
        "legend": {
            str(i): request()["questions"]["boundary"]["criteria"][i] for i in range(3)},
        "probabilities": probabilities or {"0": 0, "1": 0.5, "2": 0.5},
        "confidence": 0.5,
    }}, "usage": {"input_tokens": 100, "output_tokens": 12}}


class TypeSafeContractTests(unittest.TestCase):
    def test_only_calibrated_applicable_dimensions_are_requested(self):
        value = auxiliary_rubric()
        value["criteria"]["scope"] = {
            **value["criteria"]["boundary"], "dimension": "change_scope",
            "context_paths": ["other-context.txt"],
        }
        result = build_auxiliary_request(value, {"patch.diff": b"x", "instruction.md": b"y"}, ["boundary"])
        self.assertEqual(set(result["questions"]), {"boundary"})
        self.assertNotIn("other-context.txt", result["state"]["files"])

    def test_auxiliary_records_bind_original_response_context_worker_and_execution(self):
        req = request()
        identity = {name: "a" * 64 for name in (
            "trial_sha256", "patch_sha256", "rubric_sha256", "worker_sha256")}
        identity.update(trial_id="anonymous-artifact", context_sha256=sha256(canonical(context_manifest(
            {name: raw.encode() for name, raw in req["state"]["files"].items()}))))
        worker = {"status": "valid", "reason": "response_received", "response": response(),
                  "request_sha256": sha256(canonical(req)), "latency_sec": 0.1, "request_attempted": True}
        record = auxiliary_assessment(req, worker, identity, execution_id="api-worker")
        validate_auxiliary_assessment(record, req, worker, identity, execution_id="api-worker")
        for field in ("worker_sha256", "patch_sha256", "rubric_sha256", "context_sha256", "execution_id"):
            changed = {**record, field: "b" * 64}
            with self.assertRaises(ProtocolError):
                validate_auxiliary_assessment(changed, req, worker, identity, execution_id="api-worker")
        drift = {**worker, "response": {**response(), "model": "jev-latest"}}
        self.assertEqual(auxiliary_assessment(req, drift, identity, execution_id="api-worker")["status"], "unknown")
        self.assertEqual(auxiliary_assessment(req, {**worker, "request_sha256": "f" * 64}, identity,
                                              execution_id="api-worker")["status"], "unknown")

    def test_independently_calibrated_real_control_pair_can_enable_one_dimension(self):
        samples = []
        for name, level, origin, development in [
            ("weak", 0, "control", True), ("good", 1, "control", True),
            ("renamed", 1, "control", True), ("injected", 1, "control", True),
            ("proxy", 0, "control", True),
            ("real-weak", 0, "saved_agent", False), ("real-good", 1, "saved_agent", False),
        ]:
            raw = response(level * 2, {str(i): int(i == level * 2) for i in range(3)})
            samples.append({"artifact_id": name, "expected_level": level, "origin": origin,
                            "development": development, "results": [
                                {**validate_auxiliary(request(), raw), "execution_id": name + ":" + str(repeat)} for repeat in range(2)]})
        relations = [{"kind": kind, "left": left, "right": right} for kind, left, right in [
            ("same_success_quality", "weak", "good"), ("same_success_quality", "real-weak", "real-good"),
            ("equivalent", "renamed", "good"), ("injection", "injected", "good"), ("proxy", "proxy", "good")]]
        result = auxiliary_certificate("boundary", samples, {"worker_sha256": "a" * 64}, relations=relations)
        self.assertTrue(result["eligible"])
        samples[-1]["results"][1]["scores"]["boundary"]["probabilities"] = {"0": 1, "1": 0, "2": 0}
        self.assertFalse(auxiliary_certificate("boundary", samples, {}, relations=relations)["eligible"])

    def test_score_is_auxiliary_distribution_and_not_main_evidence(self):
        result = validate_auxiliary(request(), response())
        self.assertEqual(result["scores"]["boundary"]["normalized_score"], 0.75)
        self.assertEqual(result["scores"]["boundary"]["raw_score"], 1.5)
        self.assertNotIn("task_quality", result)
        self.assertNotIn("evidence", result["scores"]["boundary"])

    def test_decimal_rounding_has_a_fixed_bound_and_preserves_provider_values(self):
        # The real worker returned 1.99 with probabilities {0: 0, 1: 0, 2: 1}.
        result = validate_auxiliary(request(), response(1.99, {"0": 0, "1": 0, "2": 1}))
        self.assertEqual(result["scores"]["boundary"]["raw_score"], 1.99)
        self.assertEqual(result["scores"]["boundary"]["normalized_score"], 0.995)
        rounded = response(1.95, {"0": 0.01, "1": 0.02, "2": 0.96})
        self.assertEqual(validate_auxiliary(request(), rounded)["scores"]["boundary"]["probabilities"],
                         rounded["answers"]["boundary"]["probabilities"])
        for invalid in [response(1.9, {"0": 0, "1": 0, "2": 1}),
                        response(1.96, {"0": 0, "1": 0, "2": 0.98})]:
            with self.assertRaises(ProtocolError):
                validate_auxiliary(request(), invalid)

    def test_invalid_answers_still_record_valid_billable_usage(self):
        req = request()
        identity = {name: "a" * 64 for name in (
            "trial_sha256", "patch_sha256", "rubric_sha256", "worker_sha256")}
        identity.update(trial_id="artifact", context_sha256=sha256(canonical(context_manifest(
            {name: raw.encode() for name, raw in req["state"]["files"].items()}))))
        worker = {"status": "valid", "reason": "response_received", "response": response(),
                  "request_sha256": sha256(canonical(req)), "latency_sec": 0.1, "request_attempted": True}
        worker["response"]["answers"]["boundary"]["score"] = 0
        result = auxiliary_assessment(req, worker, identity, execution_id="api-worker")
        self.assertEqual(result["status"], "unknown")
        self.assertEqual(result["scores"], {})
        self.assertEqual(result["usage"], {"input_tokens": 100, "output_tokens": 12})
        worker["response"]["usage"]["input_tokens"] = -1
        self.assertIsNone(auxiliary_assessment(req, worker, identity, execution_id="api-worker")["usage"])

    def test_model_drift_bad_distributions_and_incomplete_responses_are_rejected(self):
        for mutate in (
            lambda r: r.update(model="jev-latest"),
            lambda r: r["answers"]["boundary"].update(score=1.9),
            lambda r: r["answers"]["boundary"]["probabilities"].update({"0": 0.1}),
            lambda r: r["answers"]["boundary"].update(confidence=True),
            lambda r: r["answers"]["boundary"].update(type="choice"),
            lambda r: r["answers"]["boundary"].pop("legend"),
            lambda r: r["answers"]["boundary"]["legend"].update({"2": "different rubric"}),
            lambda r: r["usage"].update(input_tokens=-1),
            lambda r: r["answers"].clear(),
        ):
            value = copy.deepcopy(response())
            mutate(value)
            with self.assertRaises(ProtocolError):
                validate_auxiliary(request(), value)

    def test_strict_report_and_deterministic_dimensions_never_reach_provider(self):
        for dimension in ["delivery_evidence", "authorization", "test_count"]:
            r = auxiliary_rubric()
            r["criteria"]["boundary"]["dimension"] = dimension
            with self.assertRaises(ProtocolError):
                build_auxiliary_request(r, {"patch.diff": b"x", "instruction.md": b"y"}, ["boundary"])
        with self.assertRaises(ProtocolError):
            build_auxiliary_request(auxiliary_rubric(), {"patch.diff": b"x"}, ["boundary"])

    def test_only_explicit_minimal_files_are_sent(self):
        value = build_auxiliary_request(auxiliary_rubric(), {
            "patch.diff": b"x", "instruction.md": b"y", "private-answer.txt": b"secret answer",
        }, ["boundary"])
        self.assertNotIn("private-answer", str(value))
        self.assertEqual(set(value), {"state", "model", "questions"})
        self.assertEqual(value["questions"]["boundary"]["type"], "score")

    def test_calibration_rejects_tied_or_wrong_peaks_despite_high_confidence(self):
        samples = [
            {"artifact_id": "weak", "expected_level": 0, "origin": "control", "development": True,
             "results": [validate_auxiliary(request(), response(0, {"0": 1, "1": 0, "2": 0}))] * 2},
            {"artifact_id": "real", "expected_level": 1, "origin": "saved_agent", "development": False,
             "results": [validate_auxiliary(request(), response(1.5))] * 2},
        ]
        certificate = auxiliary_certificate("boundary", samples, {"worker_sha256": "a" * 64},
                                             relations=[{"kind": kind, "left": "weak", "right": "real"}
                                                        for kind in ["same_success_quality", "equivalent", "injection", "proxy"]])
        self.assertFalse(certificate["eligible"])


class TypeSafeWorkerTests(unittest.TestCase):
    def test_worker_sends_one_fixed_http_request_and_writes_atomic_sanitized_result(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            key = root / "key"
            key.write_text("sentinel-api-secret")
            key.chmod(0o600)
            calls = []

            class Reply:
                def __enter__(self):
                    return self

                def __exit__(self, *args):
                    pass

                def read(self, size):
                    return canonical(response())

            def http(req, timeout):
                calls.append(req)
                self.assertEqual(req.full_url, "https://api.typesafe.ai/v1/systemone")
                self.assertEqual(req.get_header("Authorization"), "Bearer sentinel-api-secret")
                self.assertNotIn("typesafe-danger-zone-long-context", {k.lower() for k, _ in req.header_items()})
                self.assertEqual(json.loads(req.data), request())
                return Reply()

            with patch("typesafe_worker.urlopen", http):
                result = run_request(request(), key, root / "result.json", timeout=5)
            self.assertEqual(result["status"], "valid")
            self.assertEqual(len(calls), 1)
            self.assertNotIn("sentinel-api-secret", (root / "result.json").read_text())
            self.assertFalse(key.exists())

    def test_cli_long_context_adds_only_the_fixed_header_and_keeps_request_bytes(self):
        from typesafe_worker import main
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            key = root / "key"
            key.write_text("sentinel-api-secret")
            key.chmod(0o600)
            req = root / "request.json"
            req.write_bytes(canonical(request()))
            class Reply:
                def __enter__(self):
                    return self

                def __exit__(self, *args):
                    pass

                def read(self, size):
                    return canonical(response())
            def http(packet, timeout):
                headers = {k.lower(): v for k, v in packet.header_items()}
                self.assertEqual(headers["typesafe-danger-zone-long-context"], "true")
                self.assertEqual(packet.data, req.read_bytes())
                return Reply()
            with patch("typesafe_worker.urlopen", side_effect=http) as call, patch("sys.argv", [
                "typesafe-worker", "--request", str(req), "--credential", str(key),
                "--output", str(root / "result.json"), "--timeout", "5", "--long-context"]):
                main()
            self.assertEqual(call.call_count, 1)
            self.assertFalse(key.exists())
            self.assertNotIn("sentinel-api-secret", (root / "result.json").read_text())

    def test_timeout_and_http_errors_never_retry_or_echo_provider_diagnostics(self):
        for error, status in [(TimeoutError("sentinel-api-secret"), "unavailable"),
                              (URLError("sentinel-api-secret"), "unavailable"),
                              (HTTPError("https://api.typesafe.ai", 429, "sentinel-api-secret", {}, None), "unavailable")]:
            with self.subTest(error=error), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                key = root / "key"
                key.write_text("sentinel-api-secret")
                key.chmod(0o600)
                with patch("typesafe_worker.urlopen", side_effect=error) as http:
                    result = run_request(request(), key, root / "result.json", timeout=5)
                self.assertEqual(http.call_count, 1)
                self.assertEqual(result["status"], status)
                self.assertNotIn("sentinel-api-secret", str(result))
                self.assertFalse(key.exists())

    def test_worker_rejects_custom_endpoints_and_assets_before_http(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for extra in ({"endpoint": "https://example.invalid"}, {"assets": {}}, {"model": "jev-latest"}):
                key = root / "key"
                key.write_text("sentinel-api-secret")
                key.chmod(0o600)
                with patch("typesafe_worker.urlopen") as http:
                    result = run_request({**request(), **extra}, key, root / "result.json", timeout=5)
                self.assertEqual(result["status"], "unknown")
                http.assert_not_called()


class TypeSafeReferenceTests(QualityFixture):
    def scoped_reference_spec(self):
        root = self.quality_spec()
        manifest = json.loads((root / "calibration.json").read_bytes())
        for artifact in manifest["artifacts"]:
            artifact["artifact_id"] = "aux-" + artifact["artifact_id"]
            levels = artifact["expected_levels"]
            artifact["expected_levels"] = {"boundary": levels["regression"],
                                           "correctness": levels["correctness"]}
        for relation in manifest["relations"]:
            relation["left"] = "aux-" + relation["left"]
            relation["right"] = "aux-" + relation["right"]
            relation["criterion_ids"] = list(dict.fromkeys(
                "boundary" if name == "regression" else name for name in relation["criterion_ids"]))
        (root / "aux-calibration.json").write_bytes(canonical(manifest))
        (root / "auxiliary-rubric.json").write_bytes(canonical(auxiliary_rubric()))
        self.spec["evaluation"]["paths"] += ["aux-calibration.json", "auxiliary-rubric.json"]
        self.spec["evaluation"]["auxiliary"] = {
            "provider": "typesafe", "model": "jev-1.14.0", "criterion_ids": ["boundary"],
            "rubric_path": "auxiliary-rubric.json", "calibration_path": "aux-calibration.json",
            "authentication": {"source": "controller_env", "name": "TYPESAFE_API_KEY"}}
        self.spec["budget"]["max_auxiliary_calls"] = 10
        self.spec["authority"]["allow_typesafe_auxiliary"] = True
        return root, manifest

    def test_auxiliary_reference_does_not_require_unjudged_semantic_scores(self):
        from protocol import prepare
        root, manifest = self.scoped_reference_spec()
        job = prepare(self.write())
        self.assertEqual(set(manifest["artifacts"][0]["expected_levels"]), {"boundary", "correctness"})
        primary = json.loads(job.evaluation_files["calibration.json"]["content"])
        self.assertIn("regression", primary["artifacts"][0]["expected_levels"])

    def test_scoped_reference_still_requires_configured_item_and_deterministic_checks(self):
        from protocol import prepare
        root, manifest = self.scoped_reference_spec()
        for missing in ("boundary", "correctness"):
            changed = copy.deepcopy(manifest)
            changed["artifacts"][0]["expected_levels"].pop(missing)
            (root / "aux-calibration.json").write_bytes(canonical(changed))
            with self.subTest(missing=missing), self.assertRaises(ProtocolError):
                prepare(self.write())
        changed = copy.deepcopy(manifest)
        changed["artifacts"][0]["expected_levels"]["invented"] = 1
        (root / "aux-calibration.json").write_bytes(canonical(changed))
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_scoped_auxiliary_shape_cannot_replace_full_primary_references(self):
        from protocol import prepare
        self.scoped_reference_spec()
        self.spec["evaluation"]["calibration"] = "aux-calibration.json"
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_scoped_reference_cannot_omit_a_nonselected_regression_replay(self):
        from protocol import prepare
        root, manifest = self.scoped_reference_spec()
        primary = json.loads((root / "rubric.json").read_bytes())
        item = next(c for c in primary["criteria"] if c["criterion_id"] == "regression")
        item.update(checker_kind="regression_replay", anchors={"0": "No attributed protection.", "1": "Attributed protection."},
                    checker={"test_paths": ["test_app.py"], "argv": ["python3", "-m", "unittest", "test_app"],
                             "expected_failure": {"exit_code": 1, "contains": "AssertionError"}})
        (root / "rubric.json").write_bytes(canonical(primary))
        for artifact in manifest["artifacts"]:
            artifact["expected_levels"]["regression"] = artifact["expected_levels"]["boundary"]
        (root / "aux-calibration.json").write_bytes(canonical(manifest))
        prepare(self.write())
        manifest["artifacts"][0]["expected_levels"].pop("regression")
        (root / "aux-calibration.json").write_bytes(canonical(manifest))
        with self.assertRaisesRegex(ProtocolError, "required criterion"):
            prepare(self.write())

    def test_scoped_references_reach_existing_independent_calibration_checks(self):
        from protocol import prepare
        from evolution import Experiment, freeze
        root, manifest = self.scoped_reference_spec()
        job = prepare(self.write())
        output = self.root / "run"
        engine = Experiment(job, output, freeze(job, output),
                            type("Remote", (), {"runner_hash": "a" * 64})())
        engine.calibration_product = lambda artifact: {
            "case": job.spec["cases"][0], "patch": b"repair", "context": {},
            "task_success": artifact["expected_success"], "deterministic": {
                "correctness": {"status": "valid", "level": artifact["expected_levels"]["correctness"]}}}
        levels = {a["artifact_id"]: a["expected_levels"]["boundary"] for a in manifest["artifacts"]}
        calls = []
        def score(key, case, patch, context, **options):
            calls.append(key)
            level = levels[options["trial_id"]]
            return {**validate_auxiliary(request(), response(level * 2,
                    {str(i): int(i == level * 2) for i in range(3)})), "execution_id": key}
        engine.auxiliary_score = score
        result = engine.calibrate_auxiliary()
        self.assertEqual(len(calls), 2 * len(manifest["artifacts"]))
        self.assertFalse(result["certificates"]["boundary"]["eligible"])
        self.assertIn("missing_unseen_real_agent_control_pair", result["certificates"]["boundary"]["errors"])
        product = engine.calibration_product
        def uncertain(artifact):
            result = product(artifact)
            if artifact["artifact_id"] == "aux-strong":
                result["deterministic"]["correctness"] = {"status": "unknown", "level": None}
            return result
        engine.calibration_product = uncertain
        calls.clear()
        result = engine.calibrate_auxiliary(persist=False)
        self.assertEqual(len(calls), 2 * (len(manifest["artifacts"]) - 1))
        self.assertFalse(result["certificates"]["boundary"]["eligible"])
        self.assertIn("aux-strong: auxiliary_unknown", result["certificates"]["boundary"]["errors"])


class TypeSafeRemoteTests(QualityFixture):
    def auxiliary_experiment(self, max_auxiliary_calls=10, long_context=None):
        from protocol import prepare
        from evolution import Experiment, freeze
        root = self.quality_spec()
        (root / "auxiliary-rubric.json").write_bytes(canonical(auxiliary_rubric()))
        self.spec["evaluation"]["paths"].append("auxiliary-rubric.json")
        self.spec["evaluation"]["auxiliary"] = {
            "provider": "typesafe", "model": "jev-1.14.0", "criterion_ids": ["boundary"],
            "rubric_path": "auxiliary-rubric.json", "calibration_path": "calibration.json",
            "authentication": {"source": "controller_env", "name": "TYPESAFE_API_KEY"}}
        if long_context is not None:
            self.spec["evaluation"]["auxiliary"]["long_context"] = long_context
        self.spec["budget"]["max_auxiliary_calls"] = max_auxiliary_calls
        self.spec["authority"]["allow_typesafe_auxiliary"] = True
        job = prepare(self.write())
        output = self.root / "run"
        state = freeze(job, output)
        experiment = Experiment(job, output, state, type("Remote", (), {"runner_hash": "a" * 64})())
        case = job.spec["cases"][0]
        experiment.calibration_product = lambda artifact: {
            "case": case, "patch": job.evaluation_files[artifact["patch_path"]]["content"],
            "context": {"patch.diff": job.evaluation_files[artifact["patch_path"]]["content"],
                        "instruction.md": b"Fix ordinary and boundary paths.\n"},
            "task_success": artifact["expected_success"], "deterministic": {
                "correctness": {"status": "valid", "level": artifact["expected_levels"]["correctness"]}},
        }
        return experiment

    def test_long_context_is_optional_boolean_and_part_of_the_frozen_protocol(self):
        from protocol import prepare
        engine = self.auxiliary_experiment()
        original = engine.job.protocol_hash
        self.assertNotIn("long_context", engine.job.spec["evaluation"]["auxiliary"])
        for invalid in [1, "true", None]:
            self.spec["evaluation"]["auxiliary"]["long_context"] = invalid
            with self.subTest(value=invalid), self.assertRaises(ProtocolError):
                prepare(self.write())
        for value in [False, True]:
            self.spec["evaluation"]["auxiliary"]["long_context"] = value
            job = prepare(self.write())
            self.assertIs(job.spec["evaluation"]["auxiliary"]["long_context"], value)
            self.assertNotEqual(job.protocol_hash, original)
        del self.spec["evaluation"]["auxiliary"]["long_context"]
        self.assertEqual(prepare(self.write()).protocol_hash, original)

    def test_long_context_configuration_reaches_the_existing_api_payload(self):
        engine = self.auxiliary_experiment(long_context=True)
        def stop_before_submit(key, role, payload, **options):
            self.assertIn("--long-context", payload["command"]["args"])
            raise RuntimeError("stop before submission")
        engine.artifacts = stop_before_submit
        with patch.dict("os.environ", TYPESAFE_API_KEY="sentinel-api-secret"), self.assertRaisesRegex(RuntimeError, "stop before submission"):
            engine.auxiliary_score("long", engine.job.spec["cases"][0], b"repair",
                {"patch.diff": b"repair", "instruction.md": b"Fix ordinary and boundary paths.\n"},
                trial_id="trial", trial_sha256="a" * 64, criterion_ids=["boundary"], purpose="calibration")

    def test_export_interruption_recovers_the_same_worker_without_a_second_request(self):
        import base64
        from evolution import Experiment, restore_job, write_json, ConflictError
        from test_search import RecordedRemote
        from remote import UnresolvedError
        experiment = self.auxiliary_experiment()

        class API(RecordedRemote):
            def __init__(self):
                super().__init__()
                self.payloads = {}

            def submit(self, execution, payload, save):
                self.payloads[execution["remote_task_id"]] = payload
                return super().submit(execution, payload, save)

            def export(self, execution, directory, save):
                super().export(execution, directory, save)
                packet = self.payloads[execution["remote_task_id"]]
                req = json.loads(base64.b64decode(packet["files"]["/work/request.json"]["data"]))
                write_json(directory / "credential-receipt.json", {"provider": "typesafe", "destroyed": True})
                write_json(directory / "auxiliary-response.json", {
                    "status": "valid", "reason": "response_received", "response": response(),
                    "request_sha256": sha256(canonical(req)), "latency_sec": 0.1, "request_attempted": True})

        api = API()
        experiment = Experiment(experiment.job, experiment.output, experiment.state, api)
        case = experiment.job.spec["cases"][0]
        context = {"patch.diff": b"repair\n", "instruction.md": b"Fix ordinary and boundary paths.\n"}
        arguments = {"trial_id": "trial", "trial_sha256": "a" * 64, "criterion_ids": ["boundary"], "purpose": "calibration"}
        api.fail_export = True
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "sentinel-secret"}):
            with self.assertRaises(UnresolvedError):
                experiment.auxiliary_score("recovery", case, b"repair", context, **arguments)
        self.assertEqual(experiment.state["auxiliary_calls_used"], 1)
        job, state = restore_job(experiment.output)
        recovered = Experiment(job, experiment.output, state, api)
        api.fail_export = False
        with patch.dict("os.environ", {}, clear=True):
            result = recovered.auxiliary_score("recovery", case, b"repair", context, **arguments)
        self.assertEqual(result["status"], "valid")
        self.assertEqual(len(api.submissions), 1)
        self.assertEqual(recovered.state["auxiliary_calls_used"], 1)
        with self.assertRaises(ConflictError):
            recovered.auxiliary_score("recovery", case, b"different patch", context, **arguments)
        with self.assertRaises(ProtocolError):
            recovered.auxiliary_score("confirmation", case, b"repair", context, **{**arguments, "purpose": "confirmation"})
        self.assertEqual(len(api.submissions), 1)

    def test_auxiliary_subbudget_exhaustion_allocates_no_worker(self):
        experiment = self.auxiliary_experiment(max_auxiliary_calls=0)
        case = experiment.job.spec["cases"][0]
        context = {"patch.diff": b"repair\n", "instruction.md": b"Fix ordinary and boundary paths.\n"}
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "sentinel-secret"}), patch.object(experiment, "artifacts") as calls:
            result = experiment.auxiliary_score("budget", case, b"repair", context,
                trial_id="trial", trial_sha256="a" * 64, criterion_ids=["boundary"], purpose="calibration")
        self.assertEqual(result["reason"], "auxiliary_budget_exhausted")
        self.assertEqual(experiment.state["auxiliary_calls_used"], 0)
        calls.assert_not_called()

    def test_unavailable_calibration_is_recorded_per_dimension_and_never_backfilled(self):
        experiment = self.auxiliary_experiment()
        with patch.dict("os.environ", {}, clear=True), patch.object(experiment, "artifacts") as calls:
            experiment.calibrate_auxiliary()
            first = copy.deepcopy(experiment.state["auxiliary_calibration"])
            self.assertFalse(first["boundary"]["eligible"])
            self.assertEqual(experiment.state["auxiliary_calls_used"], 0)
            calls.assert_not_called()
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "configured-later"}), patch.object(experiment, "artifacts") as calls:
            experiment.calibrate_auxiliary()
            self.assertEqual(experiment.state["auxiliary_calibration"], first)
            calls.assert_not_called()

    def test_auxiliary_calibration_uses_two_registered_requests_and_a_finite_subbudget(self):
        from evolution import write_json
        import base64
        experiment = self.auxiliary_experiment()
        calls = []

        def artifacts(key, role, payload, **kwargs):
            calls.append((key, role))
            directory = experiment.output / "executions" / str(len(calls)) / "artifacts"
            directory.mkdir(parents=True)
            req = json.loads(base64.b64decode(payload["files"]["/work/request.json"]["data"]))
            write_json(directory / "capture.json", {"gaps": []})
            write_json(directory / "credential-receipt.json", {"provider": "typesafe", "destroyed": True})
            write_json(directory / "auxiliary-response.json", {
                "status": "valid", "reason": "response_received", "response": response(),
                "request_sha256": sha256(canonical(req)), "latency_sec": 0.1, "request_attempted": True})
            return {"remote_task_id": "api-" + str(len(calls)), "exit_code": 0}, directory

        experiment.artifacts = artifacts
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "sentinel-secret"}):
            experiment.calibrate_auxiliary()
            self.assertEqual(experiment.state["auxiliary_calls_used"], 10)
            self.assertEqual(len(calls), 10)
            self.assertEqual(len({key for key, role in calls}), 10)
            self.assertTrue(all(role == "grader" for key, role in calls))
            experiment.calibrate_auxiliary()
            self.assertEqual(len(calls), 10)

    def test_preparation_interruption_retains_cleanup_obligation_and_same_identity(self):
        from remote import destroy_typesafe_credential, prepare_typesafe_credential
        from evolution import write_json
        from protocol import load_json
        root = self.root / "remote-control"
        root.mkdir()
        write_json(root / "payload.json", {"needs_typesafe": True, "needs_codex": False,
                                            "command": {"typesafe_credential": "/home/evolution/.typesafe/api-key"}})
        original = {"role": "grader", "remote_task_id": "same-execution"}
        with patch("remote.host_control", return_value=root), \
             patch("remote.container_exec"), \
             patch("remote.container_files", side_effect=RuntimeError("injected transport interruption")):
            with self.assertRaises(RuntimeError):
                prepare_typesafe_credential({}, original, {"key": "sentinel-private-key"})
        self.assertTrue((root / "typesafe.key").exists())
        self.assertFalse(load_json(root / "typesafe-credential.json")["destroyed"])
        calls = []
        with patch("remote.host_control", return_value=root), \
             patch("remote.container_exec", side_effect=lambda runtime, execution, args: calls.append(execution["remote_task_id"])):
            self.assertTrue(destroy_typesafe_credential({}, original, {"Id": "original-container"}))
        self.assertEqual(calls, ["same-execution"])
        self.assertFalse((root / "typesafe.key").exists())
        receipt = load_json(root / "typesafe-credential.json")
        self.assertTrue(receipt["destroyed"])
        self.assertNotIn("sentinel-private-key", str(receipt))

    def test_enabled_provider_is_explicit_and_freezes_worker_bytes(self):
        from protocol import prepare
        from evolution import freeze, restore_job
        root = self.quality_spec()
        (root / "auxiliary-rubric.json").write_bytes(canonical(auxiliary_rubric()))
        self.spec["evaluation"]["paths"].append("auxiliary-rubric.json")
        self.spec["evaluation"]["auxiliary"] = {
            "provider": "typesafe", "model": "jev-1.14.0", "criterion_ids": ["boundary"],
            "rubric_path": "auxiliary-rubric.json", "calibration_path": "calibration.json",
            "authentication": {"source": "controller_env", "name": "TYPESAFE_API_KEY"}}
        self.spec["budget"]["max_auxiliary_calls"] = 12
        with self.assertRaises(ProtocolError):
            prepare(self.write())
        self.spec["authority"]["allow_typesafe_auxiliary"] = True
        job = prepare(self.write())
        self.assertIn("typesafe_worker.py", job.inputs()["workers"])
        output = self.root / "run"
        freeze(job, output)
        restored, _ = restore_job(output)
        self.assertEqual(restored.worker_files, job.worker_files)
        self.assertEqual(job.summary()["call_plan"]["auxiliary_maximum"], 12)

    def test_secret_is_sent_only_after_identity_is_saved_and_never_enters_payload(self):
        from protocol import prepare, new_execution, read_file
        from remote import RemoteClient, typesafe_payload
        import time

        self.quality_spec()
        # A worker payload can be checked before enabling the optional provider.
        job = prepare(self.write())
        job.worker_files["typesafe_worker.py"] = read_file(Path(__file__).resolve().parents[1], "typesafe_worker.py")
        payload = typesafe_payload(job, request())
        client = RemoteClient(job.spec["runtime"])
        execution = new_execution(job, "grader", "auxiliary", sha256(canonical(payload["command"])),
                                  client.runner_hash, now=time.time(), run_deadline=time.time() + 300)
        execution["input_hash"] = sha256(canonical(payload))
        calls, saves = [], []

        def rpc(operation, original, packet=None):
            calls.append(operation)
            if operation == "create":
                self.assertNotIn("sentinel-secret", str(packet))
                return {"sandbox_id": "owned-container", "engine_id": "engine"}
            if operation == "prepare_typesafe":
                self.assertEqual(original["sandbox_id"], "owned-container")
                self.assertEqual(saves[-1]["submission_status"], "resource_created")
                self.assertEqual(packet["key"], "sentinel-secret")
                return {"prepared": True}
            if operation == "launch":
                return {}
            self.fail(operation)

        client._rpc = rpc
        client.query = lambda original, save: {}
        with patch.dict("os.environ", {"TYPESAFE_API_KEY": "sentinel-secret"}):
            client.submit(execution, payload, lambda e: saves.append(copy.deepcopy(e)))
        self.assertEqual(calls, ["create", "prepare_typesafe", "launch"])
        self.assertNotIn("sentinel-secret", str(payload))
        self.assertNotIn("sentinel-secret", str(execution))
        self.assertFalse(payload["needs_codex"])
