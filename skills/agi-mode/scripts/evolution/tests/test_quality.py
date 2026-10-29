import copy
import unittest
from unittest.mock import patch

from support import JobFixture
from protocol import ProtocolError, canonical, prepare, sha256
from quality import (
    aggregate_quality,
    context_manifest,
    validate_assessment,
    validate_rubric,
    validate_replay,
)


def rubric(case_id="search-case"):
    return {
        "rubric_id": "bug-fix", "version": "1", "case_id": case_id,
        "criteria": [
            {
                "criterion_id": name,
                "dimension": dimension,
                "public_requirement_ref": "instruction.md: Fix the bug using the public contract.",
                "checker_kind": "judge", "weight": 0.5, "gate": False,
                "anchors": {"0": bad, "0.5": partial, "1": good},
                "context_paths": ["patch.diff"],
            }
            for name, dimension, bad, partial, good in [
                ("boundary", "boundary_completeness", "The public boundary is broken.",
                 "The ordinary path works but the documented boundary is broken.",
                 "The ordinary and documented boundary paths work."),
                ("regression", "regression_protection", "Tests do not expose the original defect.",
                 "The test exposes only one of the documented defect paths.",
                 "The test exposes the defect and passes after the repair."),
            ]
        ] + [{
            "criterion_id": "correctness", "dimension": "correctness",
            "public_requirement_ref": "instruction.md: Fix the bug using the public contract.",
            "checker_kind": "existing_verifier", "checker": {"name": "task_contract"},
            "anchors": {"0": "The original verifier fails.", "1": "The original verifier passes."},
            "weight": 0, "gate": True, "minimum": 1,
            "context_paths": ["verifier.txt"],
        }],
    }


def evidence(path, content):
    return {"path": path, "sha256": sha256(content.encode()),
            "start_line": 1, "end_line": len(content.splitlines()),
            "quote": content.rstrip("\n")}


def assessment(levels=(1, 1), *, execution_id="judge-a"):
    context = {"patch.diff": b"repair and regression test\n", "verifier.txt": b"PASS\n"}
    identity = {name: "a" * 64 for name in (
        "trial_sha256", "patch_sha256", "rubric_sha256", "judge_config_sha256")}
    identity["context_sha256"] = sha256(canonical(context_manifest(context)))
    identity["trial_id"] = "anonymous-artifact"
    value = {
        **identity, "execution_id": execution_id, "status": "valid",
        "criteria": [{"criterion_id": name, "status": "valid", "level": level,
                      "evidence": [evidence("patch.diff", context["patch.diff"].decode())],
                      "reason": "The quoted change satisfies this anchor."}
                     for name, level in zip(["boundary", "regression"], levels)],
    }
    return value, identity, context


def continuous_rubric():
    value = rubric()
    value["scoring"] = "continuous_100_v1"
    for item in value["criteria"]:
        if item["checker_kind"] == "judge":
            item["anchors"] = {str(level): f"Public requirement is satisfied to the defined {level}-point degree."
                               for level in (0, 25, 50, 75, 100)}
    return value


def continuous_assessment(scores=(73.42, 81.64), *, execution_id="judge-a"):
    value, identity, context = assessment(scores, execution_id=execution_id)
    for item in value["criteria"]:
        item["deductions"] = [{"points": round(100 - item["level"], 2),
                               "reason": "The cited requirement has the stated remaining gap.",
                               "evidence": item["evidence"]}] if item["level"] < 100 else []
    return value, identity, context


def continuous_calibration(value):
    value = copy.deepcopy(value)
    for artifact in value["artifacts"]:
        if artifact["artifact_id"] in {"weak", "strong", "renamed"}:
            artifact["expected_levels"]["regression"] = 0.8342 if artifact["artifact_id"] == "weak" else 0.8542
    return value


class ContinuousQualityTests(unittest.TestCase):
    def test_derived_assessment_status_keeps_values_and_rejects_bad_evidence(self):
        r = continuous_rubric()
        r['derive_assessment_status'] = True
        a, identity, context = continuous_assessment((81.64, 75))
        r['criteria'].append({**copy.deepcopy(r['criteria'][0]), 'criterion_id': 'scope_gate',
                              'gate': True, 'weight': 0, 'minimum': 1,
                              'anchors': {'0': 'Scope violated.', '1': 'Scope preserved.'}})
        a['criteria'].append({**copy.deepcopy(a['criteria'][0]), 'criterion_id': 'scope_gate',
                              'level': 0, 'deductions': []})
        a['status'] = 'invalid'
        original = copy.deepcopy(a)
        derived = validate_assessment(r, a, identity, context, execution_id='judge-a')
        self.assertEqual(derived['status'], 'valid')
        self.assertEqual(a, original)
        result = aggregate_quality(r, [derived, {**derived, 'execution_id': 'judge-b'}], task_success=1,
            deterministic={'correctness': {'status': 'valid', 'level': 1}})
        self.assertEqual(result['task_quality'], 0)
        self.assertEqual(result['quality_components']['boundary']['quality_score'], 81.64)
        del a['status']
        self.assertEqual(validate_assessment(r, a, identity, context, execution_id='judge-a')['status'], 'valid')
        a['criteria'][1].update(status='unknown', level=None, deductions=[], evidence=[])
        derived = validate_assessment(r, a, identity, context, execution_id='judge-a')
        self.assertEqual(derived['status'], 'unknown')
        self.assertIsNone(derived['criteria'][1]['level'])
        broken = copy.deepcopy(a)
        broken['criteria'][0]['evidence'][0]['sha256'] = '0'*64
        with self.assertRaises(ProtocolError):
            validate_assessment(r, broken, identity, context, execution_id='judge-a')
        legacy = copy.deepcopy(r)
        del legacy['derive_assessment_status']
        self.assertEqual(validate_assessment(legacy, original, identity, context, execution_id='judge-a')['status'], 'invalid')

    def test_derived_status_is_explicit_and_only_accepts_the_percent_contract(self):
        for flag in (1, 'true', None):
            value = continuous_rubric()
            value['derive_assessment_status'] = flag
            with self.subTest(flag=flag), self.assertRaises(ProtocolError):
                validate_rubric(value)
        value = rubric()
        value['derive_assessment_status'] = True
        with self.assertRaises(ProtocolError):
            validate_rubric(value)

    def test_one_unknown_criterion_retains_other_known_degrees(self):
        r = continuous_rubric()
        a, identity, context = continuous_assessment((81.64, 75))
        a['status'] = 'unknown'
        a['criteria'][1].update(status='unknown', level=None, deductions=[], evidence=[],
                                reason='The failed entrypoint prevented this observation.')
        r['criteria'].append({**copy.deepcopy(r['criteria'][0]), 'criterion_id': 'scope_gate',
                              'gate': True, 'weight': 0, 'minimum': 1,
                              'anchors': {'0': 'Scope violated.', '1': 'Scope preserved.'}})
        a['criteria'].append({**copy.deepcopy(a['criteria'][0]), 'criterion_id': 'scope_gate',
                              'level': 1, 'deductions': []})
        validate_assessment(r, a, identity, context, execution_id='judge-a')
        b = {**copy.deepcopy(a), 'execution_id': 'judge-b'}
        for success in (0, 1):
            result = aggregate_quality(r, [a, b], task_success=success,
                deterministic={'correctness': {'status': 'valid', 'level': success}})
            self.assertEqual(result['quality_components']['boundary']['quality_score'], 81.64)
            self.assertEqual(result['quality_components']['scope_gate']['level'], 1)
            self.assertEqual(result['quality_components']['regression']['status'], 'unknown')
            self.assertEqual(result['quality_components']['regression']['reason'], 'judges_agree_unknown')
            self.assertEqual(result['task_quality'], 0 if success == 0 else None)
        result = aggregate_quality(r, [a, {**b, 'status': 'invalid'}], task_success=0,
            deterministic={'correctness': {'status': 'valid', 'level': 0}})
        self.assertNotEqual(result['quality_components']['regression']['reason'], 'judges_agree_unknown')

    def test_continuous_full_score_respects_allowed_weight_rounding(self):
        r = continuous_rubric()
        original = r["criteria"][:2]
        r["criteria"] = [{**copy.deepcopy(original[i % 2]), "criterion_id": f"item-{i}", "weight": 0.3333333333333334}
                         for i in range(3)] + [r["criteria"][-1]]
        a, _, _ = continuous_assessment((100, 100))
        a["criteria"] = [{**copy.deepcopy(a["criteria"][0]), "criterion_id": f"item-{i}"} for i in range(3)]
        result = aggregate_quality(r, [a, {**a, "execution_id": "judge-b"}], task_success=1,
                                   deterministic={"correctness": {"status": "valid", "level": 1}})
        self.assertEqual(result["task_quality"], 1)
        self.assertEqual(result["quality_score"], 100)

    def test_continuous_gate_empty_deductions_preserve_pass_failure_and_unknown(self):
        r = continuous_rubric()
        gate = {**copy.deepcopy(r["criteria"][0]), "criterion_id": "scope_gate", "gate": True,
                "weight": 0, "minimum": 1, "anchors": {"0": "The permitted scope is violated.",
                                                         "1": "The permitted scope is preserved."}}
        r["criteria"].append(gate)
        for status, level in (("valid", 1), ("valid", 0), ("unknown", None)):
            a, identity, context = continuous_assessment()
            a["status"] = status
            a["criteria"].append({"criterion_id": "scope_gate", "status": status, "level": level,
                                  "reason": "Captured actions determine this binary gate.",
                                  "evidence": copy.deepcopy(a["criteria"][0]["evidence"])})
            empty = copy.deepcopy(a)
            empty["criteria"][-1]["deductions"] = []
            with self.subTest(status=status, level=level):
                for value in (a, empty):
                    validate_assessment(r, value, identity, context, execution_id="judge-a")
                results = [aggregate_quality(r, [value, {**value, "execution_id": "judge-b"}],
                            task_success=1, deterministic={"correctness": {"status": "valid", "level": 1}})
                           for value in (a, empty)]
                self.assertEqual(results[0], results[1])
                self.assertEqual(results[0]["quality_components"]["scope_gate"]["level"], level)
                if status == "unknown":
                    self.assertIsNone(results[0]["task_quality"])
                elif level == 0:
                    self.assertEqual(results[0]["task_quality"], 0)
                self.assertNotIn("deductions", a["criteria"][-1])
                self.assertEqual(empty["criteria"][-1]["deductions"], [])

    def test_gate_empty_array_compatibility_does_not_accept_deductions_or_change_legacy(self):
        r = continuous_rubric()
        r["criteria"].append({**copy.deepcopy(r["criteria"][0]), "criterion_id": "scope_gate",
                              "gate": True, "weight": 0, "minimum": 1,
                              "anchors": {"0": "Scope violated.", "1": "Scope preserved."}})
        a, identity, context = continuous_assessment()
        a["criteria"].append({"criterion_id": "scope_gate", "status": "valid", "level": 1,
                              "reason": "The complete captured actions preserve scope.",
                              "evidence": copy.deepcopy(a["criteria"][0]["evidence"])})
        for deductions in (None, {}, 0, False, [{"points": 1, "reason": "Not a binary gate.",
                                               "evidence": a["criteria"][0]["evidence"]}]):
            value = copy.deepcopy(a)
            value["criteria"][-1]["deductions"] = deductions
            with self.subTest(deductions=deductions), self.assertRaises(ProtocolError):
                validate_assessment(r, value, identity, context, execution_id="judge-a")
        for level in (0.5, 100, True):
            value = copy.deepcopy(a)
            value["criteria"][-1].update(level=level, deductions=[])
            with self.subTest(level=level), self.assertRaises(ProtocolError):
                validate_assessment(r, value, identity, context, execution_id="judge-a")
        value, identity, context = assessment()
        value["criteria"][0]["deductions"] = []
        with self.assertRaises(ProtocolError):
            validate_assessment(rubric(), value, identity, context, execution_id="judge-a")

    def test_continuous_quality_requires_distinct_dimensions(self):
        r = continuous_rubric()
        for criterion in r["criteria"]:
            criterion["dimension"] = "one_dimension"
        with self.assertRaises(ProtocolError):
            validate_rubric(r)

    def test_continuous_scores_keep_degrees_deductions_and_two_raw_judgments(self):
        r = continuous_rubric()
        a, identity, context = continuous_assessment()
        b, _, _ = continuous_assessment((74.54, 82.96), execution_id="judge-b")
        for value in (a, b):
            validate_assessment(r, value, identity, context, execution_id=value["execution_id"])
        result = aggregate_quality(r, [a, b], task_success=1,
                                   deterministic={"correctness": {"status": "valid", "level": 1}})
        self.assertEqual(result["quality_scoring"], "continuous_100_v1")
        self.assertEqual(result["quality_score"], 78.14)
        self.assertAlmostEqual(result["task_quality"], 0.7814)
        self.assertEqual(result["quality_components"]["boundary"]["judge_scores"], [73.42, 74.54])
        self.assertEqual(result["quality_components"]["boundary"]["quality_score"], 73.98)
        self.assertEqual(len(result["quality_components"]["boundary"]["deductions"]), 2)

    def test_continuous_contract_rejects_fabricated_deductions_precision_and_legacy_relabeling(self):
        for mutate in (
            lambda v: v["criteria"][0].update(level=73.421),
            lambda v: v["criteria"][0]["deductions"][0].update(points=5),
            lambda v: v["criteria"][0]["deductions"][0]["evidence"][0].update(quote="invented"),
            lambda v: v["criteria"][0].update(level=True),
            lambda v: v["criteria"][0].update(level=100.01),
        ):
            value, identity, context = continuous_assessment()
            mutate(value)
            with self.subTest(mutate=mutate), self.assertRaises(ProtocolError):
                validate_assessment(continuous_rubric(), value, identity, context, execution_id="judge-a")
        value, identity, context = continuous_assessment()
        with self.assertRaises(ProtocolError):
            validate_assessment(rubric(), value, identity, context, execution_id="judge-a")

    def test_fixed_budget_score_disagreement_is_not_averaged_when_required(self):
        r = continuous_rubric()
        a, identity, context = continuous_assessment((87.50, 100))
        b, _, _ = continuous_assessment((91.25, 100), execution_id="judge-b")
        for value in (a, b):
            validate_assessment(r, value, identity, context, execution_id=value["execution_id"])
        checks = {"correctness": {"status": "valid", "level": 1}}
        ordinary = aggregate_quality(r, [a, b], task_success=1, deterministic=checks)
        self.assertEqual(ordinary["quality_components"]["boundary"]["quality_score"], 89.38)
        r["criteria"][0]["exact_score_agreement"] = True
        result = aggregate_quality(r, [a, b], task_success=1, deterministic=checks)
        self.assertIsNone(result["task_quality"])
        self.assertEqual(result["unknown_criteria"], ["boundary"])
        self.assertEqual(result["quality_components"]["boundary"]["judge_scores"], [87.50, 91.25])
        self.assertEqual(len(result["quality_components"]["boundary"]["deductions"]), 2)
        self.assertEqual(result["quality_components"]["regression"]["quality_score"], 100)

    def test_exact_score_agreement_accepts_equal_degrees_and_explicit_false(self):
        checks = {"correctness": {"status": "valid", "level": 1}}
        for flag, second in ((True, 87.50), (False, 91.25)):
            with self.subTest(flag=flag):
                r = continuous_rubric()
                r["criteria"][0]["exact_score_agreement"] = flag
                a, _, _ = continuous_assessment((87.50, 100))
                b, _, _ = continuous_assessment((second, 100), execution_id="judge-b")
                result = aggregate_quality(r, [a, b], task_success=1, deterministic=checks)
                self.assertEqual(result["quality_status"], "valid")
                self.assertEqual(result["unknown_criteria"], [])

    def test_exact_score_agreement_rejects_invalid_types_scales_and_checker_kinds(self):
        for flag in (None, 0, 1, "true"):
            r = continuous_rubric()
            r["criteria"][0]["exact_score_agreement"] = flag
            with self.subTest(flag=flag), self.assertRaises(ProtocolError):
                validate_rubric(r)
        for r, index in ((rubric(), 0), (continuous_rubric(), 2)):
            r["criteria"][index]["exact_score_agreement"] = True
            with self.subTest(index=index), self.assertRaises(ProtocolError):
                validate_rubric(r)
        r = continuous_rubric()
        gate = r["criteria"][2]
        gate["checker_kind"] = "judge"
        del gate["checker"]
        gate["exact_score_agreement"] = True
        with self.assertRaises(ProtocolError):
            validate_rubric(r)

    def test_small_numeric_difference_cannot_hide_a_different_degree_or_total_disagreement(self):
        for first, second in [((100, 80), (99.99, 80)), ((72, 82), (74.01, 86.01)), ((80, 82), (85.01, 82))]:
            a, _, _ = continuous_assessment(first)
            b, _, _ = continuous_assessment(second, execution_id="judge-b")
            result = aggregate_quality(continuous_rubric(), [a, b], task_success=1,
                                       deterministic={"correctness": {"status": "valid", "level": 1}})
            self.assertIsNone(result["task_quality"])
            self.assertTrue(result["unknown_criteria"])


class QualityTests(unittest.TestCase):
    def test_counterfactual_scores_are_recomputed_from_the_original_snapshot_and_logs(self):
        checker = {"expected_failure": {"exit_code": 1, "contains": "union TypeError"}}
        result = {"base_tree": "a" * 40, "status": "valid", "level": 1, "reason": "counterfactual_replay",
                  "original": {"exit_code": 1, "stdout": "union TypeError", "stderr": ""},
                  "fixed": {"exit_code": 0, "stdout": "PASS", "stderr": ""}, "defect_attributed": True}
        validate_replay(checker, result, "a" * 40)
        for mutate in (
            lambda r: r.update(base_tree="b" * 40),
            lambda r: r["original"].update(stdout="a different failure"),
            lambda r: r["fixed"].update(exit_code=1),
            lambda r: r["original"].update(stderr="ModuleNotFoundError: missing"),
        ):
            changed = copy.deepcopy(result)
            mutate(changed)
            with self.assertRaises(ProtocolError):
                validate_replay(checker, changed, "a" * 40)

    def test_same_success_has_different_quality_and_keeps_components(self):
        r = validate_rubric(rubric())
        weak, identity, context = assessment((1, 0))
        strong, _, _ = assessment((1, 1))
        for value in (weak, strong):
            validate_assessment(r, value, identity, context, execution_id="judge-a")
        deterministic = {"correctness": {"status": "valid", "level": 1}}
        first = aggregate_quality(r, [weak, {**weak, "execution_id": "judge-b"}],
                                  task_success=1, deterministic=deterministic)
        second = aggregate_quality(r, [strong, {**strong, "execution_id": "judge-b"}],
                                   task_success=1, deterministic=deterministic)
        self.assertEqual((first["task_quality"], second["task_quality"]), (0.5, 1))
        self.assertEqual(first["quality_components"]["regression"]["level"], 0)

    def test_disagreement_and_missing_evidence_are_not_averaged(self):
        a, identity, context = assessment()
        b, _, _ = assessment((1, 0.5), execution_id="judge-b")
        result = aggregate_quality(rubric(), [a, b], task_success=1,
                                  deterministic={"correctness": {"status": "valid", "level": 1}})
        self.assertIsNone(result["task_quality"])
        self.assertEqual(result["quality_status"], "unknown")
        for mutation in (lambda v: v["criteria"][0].update(evidence=[]),
                         lambda v: v["criteria"][0]["evidence"][0].update(quote="invented"),
                         lambda v: v["criteria"][0]["evidence"][0].update(path="private/answer"),
                         lambda v: v["criteria"][0]["evidence"][0].update(start_line=20),
                         lambda v: v["criteria"][0]["evidence"][0].update(sha256="f" * 64),
                         lambda v: v.update(context_sha256="f" * 64),
                         lambda v: v.update(execution_id="self-reported")):
            with self.subTest(mutation=mutation):
                value = copy.deepcopy(a)
                mutation(value)
                with self.assertRaises(ProtocolError):
                    validate_assessment(rubric(), value, identity, context, execution_id="judge-a")

    def test_failed_verifier_or_gate_cannot_be_offset_by_quality(self):
        a, _, _ = assessment()
        for success, gates in [(0, {}), (1, {"correctness": {"status": "valid", "level": 0}})]:
            result = aggregate_quality(rubric(), [a, {**a, "execution_id": "judge-b"}],
                                       task_success=success, deterministic=gates)
            self.assertEqual(result["task_quality"], 0)
        result = aggregate_quality(rubric(), [a, {**a, "execution_id": "judge-b"}], task_success=1)
        self.assertIsNone(result["task_quality"])

    def test_two_contexts_must_have_different_execution_ids(self):
        a, _, _ = assessment()
        with self.assertRaises(ProtocolError):
            aggregate_quality(rubric(), [a, a], task_success=1)

    def test_only_declared_anchors_weights_and_checkers_are_accepted(self):
        for mutation in (
            lambda r: r["criteria"][0].update(weight=-1),
            lambda r: r["criteria"][0].update(weight=0.6),
            lambda r: r["criteria"][0].update(weight=True),
            lambda r: r["criteria"][0].update(checker_kind="arbitrary_script"),
            lambda r: r["criteria"][0]["anchors"].update({"0.7": "Looks good"}),
            lambda r: r["criteria"][0].update(criterion_id="regression"),
            lambda r: r["criteria"][2].pop("checker"),
        ):
            r = rubric()
            mutation(r)
            with self.assertRaises(ProtocolError):
                validate_rubric(r)

    def test_judge_cannot_score_a_deterministic_criterion(self):
        a, identity, context = assessment()
        a["criteria"].append({**a["criteria"][0], "criterion_id": "correctness"})
        with self.assertRaises(ProtocolError):
            validate_assessment(rubric(), a, identity, context, execution_id="judge-a")


class QualityFixture(JobFixture):
    def quality_spec(self):
        self.spec["schema_version"] = 2
        self.spec["protocol_id"] = "fixture-quality-v2"
        self.spec["mode"] = "calibrate"
        self.spec["runtime"].update(model="gpt-6.1-sol", effort="high", tools=["shell"],
                                    sampling={"seed": None}, authentication={
                                        "mode": "remote_codex_login", "path": "/home/test/.codex/auth.json"})
        self.spec["authority"].update(auth_path="/home/test/.codex/auth.json",
                                     evaluation_root=str(self.root / "evaluation"))
        self.spec["measurement"].update(primary_metric="task_quality",
                                        minimum_effect_reason="One boundary anchor advances by half its weight.")
        root = self.root / "evaluation"
        root.mkdir()
        (root / "rubric.json").write_bytes(canonical(rubric()))
        artifacts = []
        for name, levels, success in [
            ("weak", [1, 0, 1], 1), ("strong", [1, 1, 1], 1),
            ("renamed", [1, 1, 1], 1), ("injected", [0, 0, 0], 0),
            ("bad", [0, 0, 0], 0),
        ]:
            (root / (name + ".patch")).write_text("saved patch " + name + "\n")
            artifacts.append({"artifact_id": name, "case_id": "search-case",
                              "patch_path": name + ".patch", "context_paths": {},
                              "expected_success": success,
                              "expected_levels": dict(zip(["boundary", "regression", "correctness"], levels)),
                              "origin": "saved_agent" if name == "strong" else "control", "development": True})
        relations = [{"kind": kind, "left": left, "right": right, "criterion_ids": ids}
                     for kind, left, right, ids in [
                         ("same_success_quality", "weak", "strong", ["regression"]),
                         ("proxy", "weak", "strong", ["regression"]),
                         ("equivalent", "renamed", "strong", ["boundary", "regression"]),
                         ("injection", "injected", "bad", ["boundary", "regression"]),
                         ("rejection", "bad", "strong", ["correctness"]),
                     ]]
        next(r for r in relations if r["kind"] == "injection")["injected_level"] = 1
        (root / "calibration.json").write_bytes(canonical({"artifacts": artifacts, "relations": relations}))
        self.spec["evaluation"] = {"root": str(root), "paths": ["rubric.json", "calibration.json"],
                                    "rubrics": {"search-case": "rubric.json"}, "calibration": "calibration.json"}
        self.spec["evaluation"]["paths"].extend(name + ".patch" for name in ["weak", "strong", "renamed", "injected", "bad"])
        return root


class QualityProtocolTests(QualityFixture):
    def test_one_protocol_requires_a_uniform_assessment_status_format(self):
        import json
        root = self.quality_spec()
        r = continuous_rubric()
        r['derive_assessment_status'] = True
        (root/'rubric.json').write_bytes(canonical(r))
        p = root/'calibration.json'
        p.write_bytes(canonical(continuous_calibration(json.loads(p.read_bytes()))))
        second = copy.deepcopy(self.spec['cases'][0])
        second['case_id'] = 'second-case'
        self.spec['cases'].append(second)
        self.spec['measurement']['weights']['second-case'] = 1
        other = continuous_rubric()
        other['case_id'] = 'second-case'
        (root/'second.json').write_bytes(canonical(other))
        self.spec['evaluation']['paths'].append('second.json')
        self.spec['evaluation']['rubrics']['second-case'] = 'second.json'
        with self.assertRaisesRegex(ProtocolError, 'assessment status formats'):
            prepare(self.write())
        other['derive_assessment_status'] = True
        (root/'second.json').write_bytes(canonical(other))
        job = prepare(self.write())
        self.assertTrue(all(json.loads(job.evaluation_files[path]['content'])['derive_assessment_status']
                            for path in job.spec['evaluation']['rubrics'].values()))

    def test_native_arm64_is_explicit_and_reaches_only_v2_repo_payloads(self):
        from remote import trial_payload
        self.quality_spec()
        original = prepare(self.write())
        self.assertEqual(original.spec["cases"][0]["platform"], "linux/amd64")
        self.spec["cases"][0]["platform"] = "linux/arm64"
        native = prepare(self.write())
        self.assertNotEqual(native.protocol_hash, original.protocol_hash)
        payload = trial_payload(native, native.spec["cases"][0], native.bundles["baseline"])
        self.assertEqual(payload["platform"], "linux/arm64")
        self.assertEqual(payload["image"], self.spec["cases"][0]["image"])
        self.spec["cases"][0]["platform"] = "linux/ppc64le"
        with self.assertRaisesRegex(ProtocolError, "case.platform"):
            prepare(self.write())

    def test_pro_v2_does_not_accept_a_native_platform_substitution(self):
        from protocol import PRO_V2_REVISION
        self.quality_spec()
        case = self.spec["cases"][0]
        case["source"].update(kind="pro_v2", revision=PRO_V2_REVISION)
        case["platform"] = "linux/arm64"
        with self.assertRaisesRegex(ProtocolError, "case.platform"):
            prepare(self.write())

    def test_continuous_run_binds_public_contract_and_controller_implementations(self):
        import evolution, json
        from pathlib import Path
        from evolution import freeze, restore_job, ConflictError
        root = self.quality_spec()
        (root / "rubric.json").write_bytes(canonical(continuous_rubric()))
        path = root / "calibration.json"
        path.write_bytes(canonical(continuous_calibration(json.loads(path.read_bytes()))))
        job = prepare(self.write())
        self.assertTrue({"protocol.py", "evolution.py", "compare.py", "remote.py"}.issubset(job.worker_files))
        output = self.root / "controller-binding"
        freeze(job, output)
        original = evolution.read_file
        for changed in ["protocol.py", "evolution.py", "compare.py", "remote.py"]:
            def drift(directory, name):
                value = original(directory, name)
                return {**value, "sha256": "f" * 64} if name == changed and directory == Path(evolution.__file__).parent else value
            with self.subTest(changed=changed), patch("evolution.read_file", side_effect=drift):
                with self.assertRaisesRegex(ConflictError, "matching tool version"):
                    restore_job(output)

    def test_declared_quality_contract_is_shared_without_editing_public_task_files(self):
        from evolution import Experiment, freeze
        from remote import trial_payload
        import base64, json
        root = self.quality_spec()
        r = continuous_rubric()
        requirement = "Added regression assertions must protect the documented public boundary."
        self.spec["cases"][0]["observable_requirements"] = [requirement]
        r["criteria"][0]["public_requirement_ref"] = "task-contract.md: " + requirement
        r["criteria"][0]["context_paths"].append("task-contract.md")
        (root / "rubric.json").write_bytes(canonical(r))
        path = root / "calibration.json"
        path.write_bytes(canonical(continuous_calibration(json.loads(path.read_bytes()))))
        job = prepare(self.write())
        case = job.spec["cases"][0]
        self.assertNotIn("task-contract.md", job.case_files[case["case_id"]]["public_paths"])
        payload = trial_payload(job, case, job.bundles["baseline"])
        raw = base64.b64decode(payload["files"]["/evolution-input/task-contract.md"]["data"])
        self.assertIn(requirement.encode(), raw)
        self.assertIn(requirement, payload["command"]["args"][-1])
        output = self.root / "contract-run"
        engine = Experiment(job, output, freeze(job, output), None)
        context = engine.quality_context(case, b"patch\n", None, {})
        self.assertEqual(context["task-contract.md"], raw)

        manifest = json.loads((root / "calibration.json").read_bytes())
        manifest["artifacts"][0]["context_paths"]["task-contract.md"] = "weak.patch"
        from quality import validate_calibration
        with self.assertRaisesRegex(ProtocolError, "task-contract"):
            validate_calibration(manifest, job.evaluation_files, {"search-case": r})

    def test_continuous_calibration_cannot_just_rescale_legacy_control_endpoints(self):
        root = self.quality_spec()
        (root / "rubric.json").write_bytes(canonical(continuous_rubric()))
        with self.assertRaisesRegex(ProtocolError, "degree"):
            prepare(self.write())

    def test_continuous_protocol_binds_its_scoring_mode_and_accepts_partial_references(self):
        root = self.quality_spec()
        (root / "rubric.json").write_bytes(canonical(continuous_rubric()))
        import json
        manifest = json.loads((root / "calibration.json").read_bytes())
        manifest = continuous_calibration(manifest)
        (root / "calibration.json").write_bytes(canonical(manifest))
        job = prepare(self.write())
        self.assertEqual(job.spec["evaluation"]["scoring"], "continuous_100_v1")

    def test_candidate_limit_is_a_cap_rather_than_a_required_number_of_rounds(self):
        root = self.quality_spec()
        self.compare()
        self.spec["schema_version"] = 2
        self.spec["mode"] = "search"
        self.spec["runtime"].update(model="gpt-6.1-sol", effort="high")
        self.spec["conditions"] = self.spec["conditions"][:1]
        self.spec.pop("candidate_condition")
        heldout = rubric("confirmation-case")
        for criterion in heldout["criteria"]:
            criterion["public_requirement_ref"] = "instruction.md: Fix a different heldout bug."
        (root / "heldout-rubric.json").write_bytes(canonical(heldout))
        self.spec["evaluation"]["paths"].append("heldout-rubric.json")
        self.spec["evaluation"]["rubrics"]["confirmation-case"] = "heldout-rubric.json"
        method = self.root / "method.md"
        method.write_text("Propose one mechanism from search-only evidence.")
        self.spec["optimizer"] = {"model": "gpt-6.1-sol", "effort": "high",
                                    "method_path": str(method), "method_sha256": sha256(method.read_bytes())}
        self.spec["budget"].update(max_candidates=5, max_trials=64)
        job = prepare(self.write())
        self.assertLessEqual(job.minimum_calls, 64)
        self.assertGreater(job.call_plan["maximum_planned_calls"], 64)
        self.assertGreater(job.confirmation_reserve, 0)

    def test_frozen_scorer_drift_requires_a_matching_tool_version(self):
        from evolution import freeze, restore_job, ConflictError
        self.quality_spec()
        job = prepare(self.write())
        output = self.root / "run"
        freeze(job, output)
        with patch("evolution.read_file", return_value={"sha256": "f" * 64}):
            with self.assertRaisesRegex(ConflictError, "matching tool version"):
                restore_job(output)

    def test_validation_and_freeze_use_the_same_read_once_evaluation_bytes(self):
        import protocol
        root = self.quality_spec()
        original = protocol.read_file

        def read_once(directory, name):
            value = original(directory, name)
            if directory == root and name == "rubric.json":
                (root / name).write_text("changed after snapshot")
            return value

        with patch("protocol.read_file", side_effect=read_once):
            job = prepare(self.write())
        self.assertEqual(job.evaluation_files["rubric.json"]["content"], canonical(rubric()))

    def test_v2_freezes_authorized_evaluation_bytes_and_hides_them_from_optimizer(self):
        from evolution import freeze, restore_job
        root = self.quality_spec()
        job = prepare(self.write())
        self.assertEqual(job.summary()["schema_version"], 2)
        self.assertIn("evaluation", job.inputs())
        self.assertNotIn("anchors", str(job.optimizer_view()))
        self.assertEqual(job.confirmation_reserve, 0)
        output = self.root / "run"
        freeze(job, output)
        (root / "rubric.json").write_text("changed live rubric")
        restored, state = restore_job(output)
        self.assertEqual(restored.protocol_hash, job.protocol_hash)
        self.assertEqual(restored.evaluation_files["rubric.json"], job.evaluation_files["rubric.json"])
        self.assertEqual(state["schema_version"], 2)

    def test_v2_rejects_missing_risk_coverage_and_export_authority(self):
        root = self.quality_spec()
        (root / "calibration.json").write_bytes(canonical({"artifacts": [], "relations": []}))
        with self.assertRaises(ProtocolError):
            prepare(self.write())
        self.spec["authority"]["evaluation_root"] = str(self.repo)
        with self.assertRaises(ProtocolError):
            prepare(self.write())
