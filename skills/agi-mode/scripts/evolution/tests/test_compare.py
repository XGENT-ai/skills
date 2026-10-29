from copy import deepcopy
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from compare import compare_results, interaction_results
from protocol import canonical, sha256


class CompareTests(unittest.TestCase):
    def experiment(self, groups=40, repeats=1, objective="quality"):
        spec = {
            "protocol_id": "statistics-v1",
            "runtime": {"model": "fixed-model", "effort": "high"},
            "measurement": {
                "objective": objective,
                "minimum_effect": 0.05,
                "confidence_level": 0.95,
                "minimum_groups": 5,
                "repeats": repeats,
                "seed": 19,
                "quality_loss_limit": 0.02,
                "minimum_cost_effect": 0.1,
                "weights": {},
            },
            "budget": {"max_retries": 1},
            "guardrails": [{"name": "authorization", "minimum": 1}],
            "cases": [],
        }
        results = []
        hashes = {"a": "a" * 64, "b": "b" * 64, "c": "c" * 64, "d": "d" * 64}
        for group in range(groups):
            name = f"case-{group}"
            case = {
                "case_id": name,
                "group_id": f"family-{group}",
                "repo_id": "repo",
                "split": "search",
                "image": "image@sha256:" + "a" * 64,
                "grader_ref": "frozen-grader",
            }
            spec["cases"].append(case)
            spec["measurement"]["weights"][name] = 1
            for repeat in range(repeats):
                for condition, score in [("a", 0), ("b", 0 if group % 4 == 0 else 1)]:
                    results.append(
                        {
                            "trial_id": f"{name}-{repeat}-{condition}",
                            "protocol_id": spec["protocol_id"],
                            "case_id": name,
                            "group_id": case["group_id"],
                            "split": "search",
                            "repeat": repeat,
                            "candidate_id": condition,
                            "candidate_hash": hashes[condition],
                            "configuration_hash": sha256(canonical(spec["runtime"])),
                            "image": case["image"],
                            "grader_version": "frozen-grader",
                            "status": "ok",
                            "reason": "completed",
                            "score": score,
                            "checks": {"authorization": True},
                            "retry_of": None,
                            "cost": (2 + group / groups)
                            if condition == "a"
                            else (0.5 + group / groups),
                        }
                    )
        return spec, results, hashes

    def compare(self, spec, results, hashes):
        return compare_results(spec, results, "a", "b", hashes, split="search")

    def test_known_win_loss_and_report_does_not_modify_raw_results(self):
        spec, results, hashes = self.experiment()
        original = deepcopy(results)
        report = self.compare(spec, results, hashes)
        self.assertEqual(report["verdict"], "keep")
        self.assertEqual(report["quality_difference"], 0.75)
        self.assertEqual(report["bootstrap_samples"], 10000)
        self.assertIsNone(report["total_cost_unknown_reason"])
        self.assertEqual(results, original)
        loss = compare_results(spec, results, "b", "a", hashes, split="search")
        self.assertEqual(loss["verdict"], "discard")

    def test_ties_and_all_behavior_failures_are_not_improvements(self):
        spec, results, hashes = self.experiment()
        for row in results:
            row.update(score=0, status="behavior_failure", reason="timeout")
        report = self.compare(spec, results, hashes)
        self.assertEqual(report["verdict"], "inconclusive")
        self.assertEqual(report["behavior_failures"], len(results))
        self.assertEqual(report["valid_runs"], len(results))
        self.assertEqual(report["quality_difference"], 0)

    def test_repeats_do_not_become_independent_groups(self):
        spec, results, hashes = self.experiment(groups=3, repeats=12)
        report = self.compare(spec, results, hashes)
        self.assertEqual(report["independent_groups"], 3)
        self.assertEqual(report["verdict"], "inconclusive")

    def test_three_perfect_groups_do_not_produce_certain_improvement(self):
        spec, results, hashes = self.experiment(groups=3)
        spec["measurement"]["minimum_groups"] = 3
        for row in results:
            row["score"] = int(row["candidate_id"] == "b")
        report = self.compare(spec, results, hashes)
        self.assertEqual(report["verdict"], "inconclusive")
        self.assertLessEqual(report["quality_interval"][0], 0)

    def test_missing_pair_and_missing_repeat_are_inconclusive(self):
        spec, results, hashes = self.experiment(repeats=2)
        report = self.compare(spec, results[:-1], hashes)
        self.assertEqual(report["verdict"], "inconclusive")
        self.assertTrue(report["missing_pairs"])

    def test_guardrails_cannot_be_hidden_by_average_success(self):
        spec, results, hashes = self.experiment()
        results[1]["checks"]["authorization"] = False
        report = self.compare(spec, results, hashes)
        self.assertEqual(report["verdict"], "discard")
        self.assertIn("authorization", str(report["guardrail_regressions"]))

    def test_infra_invalid_retry_chain_is_preserved_but_not_a_behavior_failure(self):
        spec, results, hashes = self.experiment()
        failed = deepcopy(results[1])
        failed.update(
            trial_id="first-try", status="infra_invalid", reason="network", score=None
        )
        results[1]["retry_of"] = "first-try"
        results.insert(0, failed)
        report = self.compare(spec, results, hashes)
        self.assertEqual(report["verdict"], "keep")
        self.assertEqual(report["infra_invalid"], 1)
        self.assertEqual(report["raw_runs"], len(results))

    def test_behavior_failures_cannot_be_retried_for_a_better_score(self):
        spec, results, hashes = self.experiment()
        failed = deepcopy(results[1])
        failed.update(trial_id="failed-behavior", status="behavior_failure", score=0)
        results[1]["retry_of"] = "failed-behavior"
        results.insert(0, failed)
        self.assertEqual(self.compare(spec, results, hashes)["verdict"], "invalid")

    def test_hash_model_image_grader_and_duplicate_result_drift_are_invalid(self):
        spec, results, hashes = self.experiment()
        for key, value in [
            ("candidate_hash", "f" * 64),
            ("configuration_hash", "different"),
            ("image", "different"),
            ("grader_version", "changed"),
        ]:
            rows = deepcopy(results)
            rows[1][key] = value
            with self.subTest(key=key):
                self.assertEqual(self.compare(spec, rows, hashes)["verdict"], "invalid")
        rows = deepcopy(results)
        rows.append({**rows[1], "trial_id": "another-valid-run"})
        self.assertEqual(self.compare(spec, rows, hashes)["verdict"], "invalid")

    def test_missing_cost_cannot_prove_noninferiority_cost_objective(self):
        spec, results, hashes = self.experiment(objective="cost_at_quality")
        results[1]["cost"] = None
        report = self.compare(spec, results, hashes)
        self.assertEqual(report["verdict"], "inconclusive")
        self.assertIsNone(report["cost_saving_interval"])
        self.assertIsNone(report["total_cost"])

    def test_no_significant_loss_is_not_noninferiority(self):
        spec, results, hashes = self.experiment(groups=8, objective="cost_at_quality")
        for row in results:
            row["score"] = 1
        report = self.compare(spec, results, hashes)
        self.assertEqual(report["verdict"], "inconclusive")

    def test_large_paired_noninferiority_and_cost_reduction(self):
        spec, results, hashes = self.experiment(groups=40, objective="cost_at_quality")
        # This protocol permits a wider, explicitly fixed quality bound.
        spec["measurement"]["quality_loss_limit"] = 0.5
        for row in results:
            row["score"] = 1
            if row["candidate_id"] == "b":
                row["cost"] += int(row["case_id"].split("-")[-1]) / 200
        self.assertEqual(self.compare(spec, results, hashes)["verdict"], "keep")

    def test_interaction_uses_four_conditions_and_difference_of_differences(self):
        spec, results, hashes = self.experiment()
        more = []
        for row in results:
            copied = deepcopy(row)
            copied["candidate_id"] = "c" if row["candidate_id"] == "a" else "d"
            copied["trial_id"] += "-background-1"
            copied["candidate_hash"] = hashes[copied["candidate_id"]]
            copied["score"] = 1
            more.append(copied)
        results += more
        report = interaction_results(
            spec,
            results,
            {"s0a0": "a", "s1a0": "b", "s0a1": "c", "s1a1": "d"},
            hashes,
            split="search",
        )
        self.assertEqual(report["quality_interaction"], -0.75)
        self.assertEqual(report["independent_groups"], 40)
        self.assertNotIn("verdict", report)


class QualityCompareTests(unittest.TestCase):
    calibration = "e" * 64

    def experiment(self):
        spec, results, hashes = CompareTests().experiment()
        spec.update(schema_version=2, protocol_id="statistics-quality-v2")
        spec["measurement"]["primary_metric"] = "task_quality"
        for row in results:
            level = int(row["candidate_id"] == "b")
            quality = (1 + level) / 2
            row.update(schema_version=2, protocol_id=spec["protocol_id"], score=quality,
                       task_success=1, task_quality=quality, grader_score=1,
                       quality_status="valid", failed_gates=[], unknown_criteria=[],
                       quality_calibration_sha256=self.calibration,
                       quality_evidence_ref="assessments/" + row["trial_id"] + "/quality.json",
                       quality_components={
                           "scope": {"status": "valid", "level": 1, "weight": 0.5,
                                     "dimension": "change_scope", "gate": False},
                           "regression": {"status": "valid", "level": level, "weight": 0.5,
                                          "dimension": "regression_protection", "gate": False},
                           "correctness": {"status": "valid", "level": 1, "weight": 0,
                                           "dimension": "correctness", "gate": True},
                       })
        return spec, results, hashes

    def compare(self, spec, results, hashes):
        return compare_results(spec, results, "a", "b", hashes, split="search",
                               calibration_sha256=self.calibration)

    def test_continuous_quality_preserves_degree_scores_and_rejects_mixed_contracts(self):
        from quality import points
        spec, results, hashes = self.experiment()
        spec["evaluation"] = {"scoring": "continuous_100_v1"}
        for row in results:
            level = 0.7342 if row["candidate_id"] == "a" else 0.8543
            row["quality_components"]["regression"]["level"] = level
            row.update(score=(1 + level) / 2, task_quality=(1 + level) / 2,
                       quality_scoring="continuous_100_v1", quality_score=points(50 + 50 * level))
            for component in row["quality_components"].values():
                component["quality_score"] = points(100 * component["level"])
        report = self.compare(spec, results, hashes)
        self.assertFalse(report["invalid_reasons"])
        self.assertAlmostEqual(report["quality_difference_points"], 6.01, places=2)
        self.assertEqual(report["conditions"]["a"]["quality_score"], 86.71)
        results[0].pop("quality_scoring")
        self.assertTrue(self.compare(spec, results, hashes)["invalid_reasons"])

    def test_continuous_zero_score_cannot_be_a_boolean_or_missing(self):
        from compare import quality_trial_errors
        _, results, _ = self.experiment()
        row = results[0]
        row.update(task_success=0, task_quality=0, score=0, quality_score=False,
                   quality_scoring="continuous_100_v1", failed_gates=["correctness"])
        row["quality_components"]["correctness"]["level"] = 0
        for component in row["quality_components"].values():
            component["quality_score"] = 100 * component["level"]
        self.assertTrue(quality_trial_errors(row, self.calibration, scoring="continuous_100_v1"))
        row.pop("quality_score")
        self.assertTrue(quality_trial_errors(row, self.calibration, scoring="continuous_100_v1"))

    def test_same_success_quality_difference_and_criterion_evidence_are_separate(self):
        spec, results, hashes = self.experiment()
        original = deepcopy(results)
        report = self.compare(spec, results, hashes)
        self.assertEqual(report["verdict"], "keep")
        self.assertEqual(report["quality_difference"], 0.5)
        self.assertEqual(report["task_success_difference"], 0)
        self.assertEqual(report["conditions"]["a"]["task_quality"], 0.5)
        self.assertEqual(report["conditions"]["b"]["task_successes"], 40)
        regression = next(r for r in report["criterion_differences"]
                          if r["case_id"] == "case-0" and r["criterion_id"] == "regression")
        self.assertEqual(regression["difference"], 1)
        self.assertEqual(results, original)

    def test_measurement_gap_keeps_agent_denominator_and_does_not_raise_the_mean(self):
        spec, results, hashes = self.experiment()
        results[1].update(score=None, task_quality=None, quality_status="unknown",
                          unknown_criteria=["regression"])
        results[1]["quality_components"]["regression"].update(status="unknown", level=None)
        report = self.compare(spec, results, hashes)
        self.assertEqual(report["verdict"], "inconclusive")
        self.assertEqual(report["valid_runs"], 80)
        self.assertEqual(report["behavior_failures"], 0)
        self.assertEqual(report["conditions"]["b"]["task_successes"], 40)
        self.assertEqual(report["conditions"]["b"]["quality_measured"], 39)
        self.assertIsNone(report["quality_difference"])
        self.assertIsNone(report["conditions"]["b"]["task_quality"])
        self.assertEqual(report["missing_quality"][0]["trial_id"], results[1]["trial_id"])

    def test_version_calibration_and_score_alias_drift_cannot_promote(self):
        spec, results, hashes = self.experiment()
        for mutate in (
            lambda r: r.pop("schema_version"),
            lambda r: r.update(quality_calibration_sha256="f" * 64),
            lambda r: r.update(score=0.5),
            lambda r: r.update(task_success=True),
            lambda r: r["quality_components"]["regression"].update(status="unknown", level=None),
        ):
            rows = deepcopy(results)
            mutate(rows[1])
            self.assertEqual(self.compare(spec, rows, hashes)["verdict"], "invalid")
        self.assertEqual(compare_results(spec, results, "a", "b", hashes,
                                         split="search")["verdict"], "invalid")
        legacy = deepcopy(spec)
        legacy["schema_version"] = 1
        self.assertEqual(compare_results(legacy, results, "a", "b", hashes,
                                         split="search")["verdict"], "invalid")

    def test_no_agent_results_still_report_the_frozen_denominators(self):
        spec, _, hashes = self.experiment()
        report = self.compare(spec, [], hashes)
        self.assertEqual(report["verdict"], "inconclusive")
        self.assertEqual(report["conditions"]["a"]["expected_runs"], 40)
        self.assertEqual(report["conditions"]["a"]["agent_runs"], 0)
        self.assertIsNone(report["conditions"]["a"]["task_success"])

    def test_verifier_measurement_failure_preserves_the_agent_without_inventing_success(self):
        spec, results, hashes = self.experiment()
        results[1].update(score=None, task_quality=None, task_success=None, grader_score=None,
                          quality_status="unknown", unknown_criteria=["correctness"])
        results[1]["quality_components"]["correctness"].update(status="unknown", level=None)
        report = self.compare(spec, results, hashes)
        self.assertEqual(report["verdict"], "inconclusive")
        self.assertEqual(report["valid_runs"], 80)
        self.assertEqual(report["conditions"]["b"]["task_success_unknown"], 1)
        self.assertEqual(report["conditions"]["b"]["task_successes"], 39)
        self.assertEqual(report["conditions"]["b"]["task_failures"], 0)
        self.assertIsNone(report["conditions"]["b"]["task_success"])

    def test_infra_retry_does_not_allow_resampling_a_valid_agent_after_a_judge_gap(self):
        spec, results, hashes = self.experiment()
        original = deepcopy(results[1])
        original.update(trial_id="first-infra", status="infra_invalid", score=None,
                        task_success=None, task_quality=None, quality_status="unknown")
        results[1]["retry_of"] = original["trial_id"]
        rows = [original] + results
        self.assertEqual(self.compare(spec, rows, hashes)["verdict"], "keep")
        invalid = deepcopy(rows)
        invalid[0]["task_quality"] = 1
        self.assertEqual(self.compare(spec, invalid, hashes)["verdict"], "invalid")
        measurement_gap = deepcopy(rows)
        measurement_gap[0].update(status="ok", task_success=1, unknown_criteria=["regression"])
        measurement_gap[0]["quality_components"]["regression"].update(status="unknown", level=None)
        self.assertEqual(self.compare(spec, measurement_gap, hashes)["verdict"], "invalid")

    def test_one_failed_hard_gate_cannot_be_offset_by_other_quality_gains(self):
        spec, results, hashes = self.experiment()
        results[1].update(score=0, task_quality=0, task_success=0, grader_score=0,
                          status="behavior_failure", failed_gates=["correctness"])
        results[1]["quality_components"]["correctness"]["level"] = 0
        report = self.compare(spec, results, hashes)
        self.assertEqual(report["verdict"], "discard")
        self.assertGreater(report["quality_difference"], 0)
        self.assertEqual(report["conditions"]["b"]["task_failures"], 1)
        self.assertIn("correctness", str(report["guardrail_regressions"]))

    def test_auxiliary_values_cannot_change_primary_comparison(self):
        spec, results, hashes = self.experiment()
        before = self.compare(spec, results, hashes)
        for row in results:
            row["auxiliary_ref"] = "auxiliary/scores.json"
            row["auxiliary_scores"] = {"normalized_score": -1000 if row["candidate_id"] == "b" else 1000}
        self.assertEqual(self.compare(spec, results, hashes), before)


if __name__ == "__main__":
    unittest.main()
