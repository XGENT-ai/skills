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


if __name__ == "__main__":
    unittest.main()
