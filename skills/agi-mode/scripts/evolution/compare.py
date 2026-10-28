"""Paired task/group estimates; never edit trials or decide confirmation state."""

from collections import defaultdict
import math
import random

from protocol import ProtocolError, canonical, fingerprint, sha256


BOOTSTRAP_SAMPLES = 10000


def weighted_mean(groups, key):
    return sum(group[key] * group["weight"] for group in groups) / sum(
        group["weight"] for group in groups
    )


def interval(groups, key, measurement, *, span=None):
    if not groups or any(group[key] is None for group in groups):
        return None, None
    values = [group[key] for group in groups]
    # An unbounded resource variable has no useful bootstrap interval when
    # every observed group is identical. Preserve that uncertainty explicitly.
    if span is None and len(set(values)) < 2:
        return None, None
    rng = random.Random(measurement["seed"])
    count = len(groups)
    draws = sorted(
        weighted_mean([groups[rng.randrange(count)] for _ in range(count)], key)
        for _ in range(BOOTSTRAP_SAMPLES)
    )
    alpha = 1 - measurement["confidence_level"]
    low = draws[int(BOOTSTRAP_SAMPLES * alpha / 2)]
    high = draws[
        min(BOOTSTRAP_SAMPLES - 1, math.ceil(BOOTSTRAP_SAMPLES * (1 - alpha / 2)) - 1)
    ]
    bootstrap = [low, high]
    if span is not None:
        # Bounded independent group means admit a Hoeffding bound. Taking the
        # union prevents a degenerate bootstrap sample from claiming certainty.
        total = sum(group["weight"] for group in groups)
        squares = sum((group["weight"] / total) ** 2 for group in groups)
        radius = span * math.sqrt(squares * math.log(2 / alpha) / 2)
        mean = weighted_mean(groups, key)
        low = min(low, max(-span / 2, mean - radius))
        high = max(high, min(span / 2, mean + radius))
    return [low, high], bootstrap


def quality_trial_errors(row, calibration_sha256, *, scoring=None):
    from quality import CONTINUOUS_SCORING, points
    required = {"score", "task_success", "task_quality", "quality_status", "quality_components",
                "quality_evidence_ref", "quality_calibration_sha256", "failed_gates", "unknown_criteria"}
    if scoring == CONTINUOUS_SCORING:
        required |= {"quality_scoring", "quality_score"}
    if not required.issubset(row):
        return ["missing versioned quality fields"]
    errors = []
    if row.get("quality_scoring") != scoring or scoring not in (None, CONTINUOUS_SCORING):
        errors.append("quality scoring contract differs")
    continuous = scoring == CONTINUOUS_SCORING
    try:
        fingerprint(calibration_sha256, "quality calibration identity")
    except ProtocolError:
        errors.append("missing quality calibration identity")
    if row["quality_calibration_sha256"] != calibration_sha256:
        errors.append("quality calibration identity differs")
    if row["status"] == "infra_invalid":
        return errors + (["infrastructure failure cannot have quality or success"]
                         if row["task_quality"] is not None or row["task_success"] is not None else [])
    if not (row["task_success"] is None and row["quality_status"] == "unknown") and (
        type(row["task_success"]) is not int or row["task_success"] not in (0, 1)
    ):
        errors.append("invalid original task success")
    if row["quality_status"] == "unknown":
        if row["task_quality"] is not None or row["score"] is not None:
            errors.append("unknown quality cannot have a score")
    elif row["quality_status"] != "valid" or (
        type(row["task_quality"]) not in (int, float)
        or not math.isfinite(row["task_quality"]) or not 0 <= row["task_quality"] <= 1
        or row["score"] != row["task_quality"] or type(row["score"]) is bool
    ):
        errors.append("invalid task quality or score alias")
    public = row.get("quality_score")
    if continuous and ((public is not None and (type(public) not in (int, float) or not math.isfinite(public))) or
                       public != (points(row["task_quality"] * 100)
                                  if type(row["task_quality"]) in (int, float) and math.isfinite(row["task_quality"]) else None)):
        errors.append("public quality score differs from normalized evidence")
    if (row["task_success"] == 0 or row["status"] == "behavior_failure") and row["task_quality"] != 0:
        errors.append("behavior failure must retain zero quality")
    if not isinstance(row["quality_evidence_ref"], str) or not row["quality_evidence_ref"]:
        errors.append("quality evidence reference is missing")
    components = row["quality_components"]
    if not isinstance(components, dict) or not components:
        return errors + ["quality components are missing"]
    for name, component in components.items():
        if not isinstance(name, str) or not name or not isinstance(component, dict):
            errors.append("invalid quality component")
        elif component.get("status") not in ("valid", "unknown", "invalid") or (
            component.get("status") == "valid" and (
                type(component.get("level")) not in (int, float) or not math.isfinite(component["level"])
                or not 0 <= component["level"] <= 1
                or (not continuous and component["level"] not in (0, 0.5, 1))
                or (continuous and component.get("gate") and component["level"] not in (0, 1))
            )
        ):
            errors.append("invalid quality component level")
        elif continuous and ("quality_score" not in component or
                (component["quality_score"] is not None and type(component["quality_score"]) not in (int, float)) or
                component.get("quality_score") != (points(component["level"] * 100) if component.get("status") == "valid" else None)):
            errors.append("public component score differs")
    for name in ("failed_gates", "unknown_criteria"):
        value = row[name]
        if not isinstance(value, list) or any(not isinstance(item, str) or item not in components for item in value):
            errors.append("invalid quality gap or gate identities")
    if row["quality_status"] == "valid" and row["task_success"] == 1 and not row["failed_gates"] and any(
        isinstance(c, dict) and c.get("status") != "valid" for c in components.values()
    ):
        errors.append("incomplete quality evidence cannot have a valid score")
    return errors


def quality_details(spec, relevant, baseline, candidate, split):
    from quality import CONTINUOUS_SCORING, points
    cases = {c["case_id"]: c for c in spec["cases"] if c["split"] == split}
    repeats = spec["measurement"]["repeats"]
    valid = [r for r in relevant if r["status"] != "infra_invalid"]
    expected = len(cases) * repeats
    conditions = {}
    for condition in (baseline, candidate):
        rows = [r for r in valid if r["candidate_id"] == condition]
        measured = [r for r in rows if r["quality_status"] == "valid"]
        successes = [r for r in rows if r["task_success"] is not None]
        complete = len(rows) == expected and expected > 0
        values = [{"weight": spec["measurement"]["weights"][r["case_id"]] / repeats,
                   "success": r["task_success"], "quality": r["task_quality"]} for r in rows]
        conditions[condition] = {"expected_runs": expected, "agent_runs": len(rows),
            "task_successes": sum(r["task_success"] for r in successes),
            "task_failures": sum(r["task_success"] == 0 for r in rows),
            "task_success_measured": len(successes), "task_success_unknown": len(rows) - len(successes),
            "quality_measured": len(measured), "quality_unknown": len(rows) - len(measured),
            "task_success": weighted_mean(values, "success") if complete and len(successes) == expected else None,
            "task_quality": weighted_mean(values, "quality") if complete and len(measured) == expected else None}
    details = []
    for case_id, case in sorted(cases.items()):
        rows = [r for r in valid if r["case_id"] == case_id]
        for name in sorted({name for r in rows for name in r["quality_components"]}):
            means = {}
            for condition in (baseline, candidate):
                items = [r["quality_components"].get(name, {}) for r in rows if r["candidate_id"] == condition]
                means[condition] = sum(c["level"] for c in items) / repeats if len(items) == repeats and all(
                    c.get("status") == "valid" for c in items) else None
            details.append({"case_id": case_id, "group_id": case["group_id"], "criterion_id": name,
                "baseline": means[baseline], "candidate": means[candidate],
                "difference": None if None in means.values() else means[candidate] - means[baseline],
                "evidence_refs": [r["quality_evidence_ref"] for r in rows]})
    left, right = conditions[baseline]["task_success"], conditions[candidate]["task_success"]
    if spec.get("evaluation", {}).get("scoring") == CONTINUOUS_SCORING:
        for condition in conditions.values():
            condition["quality_score"] = points(condition["task_quality"] * 100) if condition["task_quality"] is not None else None
        for item in details:
            item["difference_points"] = points(item["difference"] * 100) if item["difference"] is not None else None
    return {"conditions": conditions, "criterion_differences": details,
            "task_success_difference": None if left is None or right is None else right - left,
            "missing_quality": [{"trial_id": r["trial_id"], "case_id": r["case_id"],
                                 "candidate_id": r["candidate_id"], "criteria": r["unknown_criteria"]}
                                for r in valid if r["quality_status"] != "valid"]}


def paired_groups(spec, records, baseline, candidate, hashes, split, *, calibration_sha256=None):
    cases = {case["case_id"]: case for case in spec["cases"] if case["split"] == split}
    repeats = spec["measurement"]["repeats"]
    relevant = [
        row
        for row in records
        if row.get("split") == split
        and row.get("candidate_id") in {baseline, candidate}
    ]
    slots, by_id, errors = defaultdict(list), {}, []
    version = spec.get("schema_version", 1)
    if version not in (1, 2):
        errors.append("unknown result schema version")
    expected_config = sha256(canonical(spec["runtime"]))
    for row in relevant:
        identity = row.get("trial_id")
        case = cases.get(row.get("case_id"))
        if not isinstance(identity, str) or identity in by_id:
            errors.append("missing or duplicate trial identity")
            continue
        by_id[identity] = row
        if case is None:
            errors.append("trial names an unknown case")
            continue
        if (
            row.get("protocol_id") != spec["protocol_id"]
            or row.get("group_id") != case["group_id"]
            or row.get("configuration_hash") != expected_config
            or row.get("image") != case["image"]
            or row.get("candidate_hash") != hashes.get(row["candidate_id"])
            or row.get("grader_version") != case["grader_ref"]
        ):
            errors.append("protocol, configuration, instruction, image or grader drift")
        if type(row.get("repeat")) is not int or row["repeat"] not in range(repeats):
            errors.append("repeat is outside the frozen protocol")
        status = row.get("status")
        score = row.get("score")
        if row.get("schema_version", 1) != version:
            errors.append("trial schema version differs")
        if status not in {"ok", "behavior_failure", "infra_invalid"}:
            errors.append("unknown trial status")
        elif status == "infra_invalid":
            if score is not None:
                errors.append("infrastructure failure cannot have a behavior score")
            if version == 2:
                errors.extend(quality_trial_errors(row, calibration_sha256, scoring=spec.get("evaluation", {}).get("scoring")))
        elif version == 2:
            errors.extend(quality_trial_errors(row, calibration_sha256, scoring=spec.get("evaluation", {}).get("scoring")))
        elif (
            type(score) not in (int, float)
            or not math.isfinite(score)
            or not 0 <= score <= 1
        ):
            errors.append("invalid task success score")
        elif status == "behavior_failure" and score != 0:
            errors.append(
                "behavior failure must remain in the denominator with score zero"
            )
        cost = row.get("cost")
        if cost is not None and (
            type(cost) not in (int, float) or not math.isfinite(cost) or cost < 0
        ):
            errors.append("invalid resource cost")
        slots[(case["case_id"], row.get("repeat"), row["candidate_id"])].append(row)
    for row in relevant:
        prior = row.get("retry_of")
        visited = {row.get("trial_id")}
        depth = 0
        while prior is not None:
            original = by_id.get(prior)
            if original is None or prior in visited:
                errors.append("missing or cyclic retry chain")
                break
            visited.add(prior)
            if original["status"] != "infra_invalid" or any(
                original.get(key) != row.get(key)
                for key in ["case_id", "repeat", "candidate_id"]
            ):
                errors.append("retry changes a valid outcome or paired condition")
                break
            depth += 1
            prior = original.get("retry_of")
        if depth > spec["budget"]["max_retries"]:
            errors.append("retry chain exceeds the frozen budget")
    missing, regressions, unknown_checks, group_cases = [], [], [], defaultdict(list)
    if errors:
        return [], relevant, errors, missing, regressions, unknown_checks
    for case_id, case in sorted(cases.items()):
        qualities, costs, checks = [], [], defaultdict(list)
        complete = True
        for repeat in range(repeats):
            pair = []
            pair_cost = []
            for condition in [baseline, candidate]:
                attempts = slots.get((case_id, repeat, condition), [])
                valid = [row for row in attempts if row["status"] != "infra_invalid"]
                if len(valid) > 1:
                    errors.append("multiple valid outcomes for one frozen paired slot")
                if not valid:
                    missing.append(
                        {
                            "case_id": case_id,
                            "repeat": repeat,
                            "candidate_id": condition,
                        }
                    )
                    complete = False
                    continue
                pair.append(valid[0])
                pair_cost.append(
                    None
                    if any(row.get("cost") is None for row in attempts)
                    else sum(row["cost"] for row in attempts)
                )
            if len(pair) != 2:
                continue
            if version == 2:
                for gate in pair[1]["failed_gates"]:
                    regressions.append({"case_id": case_id, "check": "quality_gate:" + gate})
                if pair[1]["task_success"] == 0:
                    regressions.append({"case_id": case_id, "check": "task_success"})
            for guardrail in spec["guardrails"]:
                observed = pair[1].get("checks", {}).get(guardrail["name"])
                if (
                    type(observed) not in (bool, int, float)
                    or not math.isfinite(observed)
                    or not 0 <= observed <= 1
                ):
                    unknown_checks.append(
                        {"case_id": case_id, "check": guardrail["name"]}
                    )
                else:
                    checks[guardrail["name"]].append(float(observed))
            if version == 2 and any(r["quality_status"] != "valid" for r in pair):
                complete = False
                unknown_checks.extend({"case_id": case_id, "check": "task_quality",
                                       "candidate_id": r["candidate_id"]} for r in pair if r["quality_status"] != "valid")
                continue
            qualities.append(pair[1]["score"] - pair[0]["score"])
            costs.append(None if None in pair_cost else pair_cost[0] - pair_cost[1])
        for guardrail in spec["guardrails"]:
            observed = checks[guardrail["name"]]
            if (
                len(observed) == repeats
                and sum(observed) / repeats < guardrail["minimum"]
            ):
                regressions.append({"case_id": case_id, "check": guardrail["name"]})
        if not complete:
            continue
        group_cases[(case["repo_id"], case["group_id"])].append(
            {
                "case_id": case_id,
                "weight": spec["measurement"]["weights"][case_id],
                "quality_difference": sum(qualities) / repeats,
                "cost_saving": None if None in costs else sum(costs) / repeats,
            }
        )
    groups = []
    for (repo_id, group_id), items in sorted(group_cases.items()):
        groups.append(
            {
                "repo_id": repo_id,
                "group_id": group_id,
                "case_ids": [item["case_id"] for item in items],
                "weight": sum(item["weight"] for item in items),
                "quality_difference": weighted_mean(items, "quality_difference"),
                "cost_saving": None
                if any(item["cost_saving"] is None for item in items)
                else weighted_mean(items, "cost_saving"),
            }
        )
    return groups, relevant, errors, missing, regressions, unknown_checks


def compare_results(spec, records, baseline, candidate, hashes, *, split, calibration_sha256=None):
    measurement = spec["measurement"]
    groups, relevant, errors, missing, regressions, unknown_checks = paired_groups(
        spec, records, baseline, candidate, hashes, split, calibration_sha256=calibration_sha256
    )
    costs_known = bool(relevant) and all(
        row.get("cost") is not None for row in relevant
    )
    report = {
        "protocol_id": spec["protocol_id"],
        "baseline": baseline,
        "candidate": candidate,
        "split": split,
        "verdict": "inconclusive",
        "reason": "insufficient_evidence",
        "raw_runs": len(relevant),
        "valid_runs": sum(
            row.get("status") in {"ok", "behavior_failure"} for row in relevant
        ),
        "behavior_failures": sum(
            row.get("status") == "behavior_failure" for row in relevant
        ),
        "infra_invalid": sum(row.get("status") == "infra_invalid" for row in relevant),
        "independent_groups": len(groups),
        "paired_cases": sum(len(group["case_ids"]) for group in groups),
        "missing_pairs": missing,
        "guardrail_regressions": regressions,
        "unknown_checks": unknown_checks,
        "invalid_reasons": sorted(set(errors)),
        "quality_difference": None,
        "quality_interval": None,
        "bootstrap_quality_interval": None,
        "cost_saving": None,
        "cost_saving_interval": None,
        "total_cost": sum(row["cost"] for row in relevant)
        if costs_known and not errors
        else None,
        "total_cost_unknown_reason": None
        if costs_known and not errors
        else "missing_or_invalid_resource_evidence",
        "method": "paired_source_group_bootstrap_with_bounded_mean_guard",
        "bootstrap_samples": BOOTSTRAP_SAMPLES,
        "confidence_level": measurement["confidence_level"],
        "group_differences": groups,
    }
    if errors:
        report.update(verdict="invalid", reason="inconsistent_trial_evidence")
        return report
    incomplete_quality = False
    if spec.get("schema_version", 1) == 2:
        report.update(schema_version=2, primary_metric="task_quality",
                      quality_calibration_sha256=calibration_sha256,
                      **quality_details(spec, relevant, baseline, candidate, split))
        incomplete_quality = bool(missing or report["missing_quality"])
    if groups and not incomplete_quality:
        report["quality_difference"] = weighted_mean(groups, "quality_difference")
        report["quality_interval"], report["bootstrap_quality_interval"] = interval(
            groups, "quality_difference", measurement, span=2
        )
        report["cost_saving_interval"], _ = interval(groups, "cost_saving", measurement)
        if all(group["cost_saving"] is not None for group in groups):
            report["cost_saving"] = weighted_mean(groups, "cost_saving")
    if regressions:
        report.update(verdict="discard", reason="guardrail_regression")
    elif missing or unknown_checks or len(groups) < measurement["minimum_groups"]:
        report["reason"] = "missing_pairs_checks_or_independent_groups"
    elif measurement["objective"] == "quality":
        low, high = report["quality_interval"]
        if low > measurement["minimum_effect"]:
            report.update(verdict="keep", reason="quality_improvement_supported")
        elif high <= measurement["minimum_effect"]:
            report.update(
                verdict="discard", reason="quality_improvement_below_threshold"
            )
    elif report["cost_saving_interval"] is None:
        report["reason"] = "cost_evidence_missing_or_degenerate"
    else:
        quality_low, quality_high = report["quality_interval"]
        cost_low, cost_high = report["cost_saving_interval"]
        if (
            quality_low >= -measurement["quality_loss_limit"]
            and cost_low > measurement["minimum_cost_effect"]
        ):
            report.update(
                verdict="keep", reason="noninferiority_and_cost_reduction_supported"
            )
        elif (
            quality_high < -measurement["quality_loss_limit"]
            or cost_high <= measurement["minimum_cost_effect"]
        ):
            report.update(verdict="discard", reason="quality_or_cost_threshold_failed")
    if spec.get("evaluation", {}).get("scoring") == "continuous_100_v1":
        from quality import points
        report["quality_scoring"] = "continuous_100_v1"
        report["quality_difference_points"] = points(report["quality_difference"] * 100) if report["quality_difference"] is not None else None
        report["quality_interval_points"] = [points(v * 100) for v in report["quality_interval"]] if report["quality_interval"] is not None else None
    return report


def baseline_diagnostic(spec, records, comparison, *, judge_noise, budget):
    measurement = spec["measurement"]
    cases = {c["case_id"]: c for c in spec["cases"] if c["split"] == "search"}
    rows = [r for r in records if r.get("comparison_id") == "aa" and r.get("purpose") == "task"
            and r.get("split") == "search" and r.get("candidate_id") in {"aa_0", "aa_1"}]
    if comparison.get("invalid_reasons"):
        rows = []
    means = [comparison.get("conditions", {}).get(name, {}).get("task_quality") for name in ("aa_0", "aa_1")]
    mean = sum(means) / 2 if None not in means else None
    maximum_gain = 1 - mean if mean is not None else None
    slots = {(r["case_id"], r["repeat"], r["candidate_id"]): r for r in rows if r["status"] != "infra_invalid"}
    differences = []
    for case_id in cases:
        for repeat in range(measurement["repeats"]):
            pair = [slots.get((case_id, repeat, name), {}) for name in ("aa_0", "aa_1")]
            values = [r.get("task_quality") for r in pair]
            if None not in values:
                differences.append({"value": abs(values[1] - values[0]),
                                    "weight": measurement["weights"][case_id] / measurement["repeats"]})
    noise = weighted_mean(differences, "value") if len(differences) == len(cases) * measurement["repeats"] and differences else None
    weak = [{"case_id": r["case_id"], "criterion_id": name, "level": c["level"],
             "evidence_ref": r["quality_evidence_ref"]} for r in rows if r["status"] != "infra_invalid"
            for name, c in r.get("quality_components", {}).items() if c.get("status") == "valid"
            and c["level"] < 1 and (not c["gate"] or name in r["failed_gates"])]
    precision = {}
    for split in ("search", "confirmation"):
        groups = defaultdict(float)
        for case in spec["cases"]:
            if case["split"] == split:
                groups[(case["repo_id"], case["group_id"])] += measurement["weights"][case["case_id"]]
        if not groups:
            precision[split] = None
            continue
        total = sum(groups.values())
        squares = sum((weight / total) ** 2 for weight in groups.values())
        radius = 2 * math.sqrt(squares * math.log(2 / (1 - measurement["confidence_level"])) / 2)
        lower = max(-1, maximum_gain - radius) if maximum_gain is not None else None
        gap = maximum_gain - measurement["minimum_effect"] if maximum_gain is not None else 0
        precision[split] = {"independent_groups": len(groups), "effective_groups": 1 / squares,
            "bounded_radius": radius, "minimum_detectable_effect": measurement["minimum_effect"] + radius,
            "optimistic_lower_bound": lower,
            "equal_weight_groups_required": math.floor(2 * math.log(2 / (1 - measurement["confidence_level"])) / gap ** 2) + 1 if gap > 0 else None,
            "adequate": lower is not None and lower > measurement["minimum_effect"] and len(groups) >= measurement["minimum_groups"]}
    mode = spec.get("mode", "calibrate")
    required_splits = ["confirmation"] if mode == "compare" else ["search", "confirmation"] if mode == "search" else ["search"]
    precision_ok = all(precision[split] is not None and precision[split]["adequate"] for split in required_splits)
    reason = "quality_search_can_proceed"
    if comparison.get("invalid_reasons"):
        reason = "baseline_quality_unknown"
    elif judge_noise.get("disagreements", 0):
        reason = "measurement_noise_too_large"
    elif (comparison.get("invalid_reasons") or comparison.get("missing_pairs") or comparison.get("missing_quality")
          or comparison.get("unknown_checks") or mean is None or noise is None or judge_noise["status"] != "valid"):
        reason = "baseline_quality_unknown"
    elif maximum_gain <= 1e-12:
        reason = "quality_headroom_absent"
    elif noise >= measurement["minimum_effect"] or judge_noise.get("max_difference", 0) >= measurement["minimum_effect"]:
        reason = "measurement_noise_too_large"
    elif not precision_ok or budget["remaining_calls"] < budget["required_calls"]:
        reason = "precision_budget_insufficient"
    elif not weak:
        reason = "no_actionable_quality_weakness"
    result = {"proceed": reason == "quality_search_can_proceed", "reason": reason,
        "headroom": {"baseline_quality": mean, "maximum_gain": maximum_gain, "weak_criteria": weak,
                     "conditions": comparison.get("conditions", {})},
        "judge_noise": judge_noise,
        "generation_noise": {"weighted_absolute_difference": noise, "paired_observations": len(differences)},
        "precision": precision, "budget": budget,
        "precision_assumption": "optimistic planning uses observed search headroom; confirmation outcomes remain unseen",
        "comparison_evidence": {name: comparison.get(name) for name in ("invalid_reasons", "missing_pairs", "missing_quality", "unknown_checks")}}
    if measurement.get("search_policy") == "bounded_exploration_v1":
        checks = {
            "search_mode": spec.get("schema_version") == 2 and mode == "search",
            "known_stable_baseline": reason in {"quality_search_can_proceed", "precision_budget_insufficient"},
            "actionable_headroom": maximum_gain is not None and maximum_gain > measurement["minimum_effect"] and bool(weak),
            "hard_floors": bool(rows) and all(type(r.get("task_success")) is int and r["task_success"] == 1 and not r.get("failed_gates") for r in rows),
            "declared_guardrails": all(type(value := r.get("checks", {}).get(g["name"])) in (bool, int, float)
                and math.isfinite(value) and g["minimum"] <= value <= 1 for r in rows for g in spec["guardrails"]),
            "call_budget": budget["remaining_calls"] >= budget["required_calls"],
        }
        result["search_permission"] = {"policy": "bounded_exploration_v1", "allowed": all(checks.values()),
            "scope": "one_candidate_no_adoption", "checks": checks}
    return result


def interaction_results(spec, records, lanes, hashes, *, split, calibration_sha256=None):
    required = {"s0a0", "s1a0", "s0a1", "s1a1"}
    if set(lanes) != required or len(set(lanes.values())) != 4:
        raise ValueError("interaction requires four distinct predeclared conditions")
    first = paired_groups(spec, records, lanes["s0a0"], lanes["s1a0"], hashes, split,
                          calibration_sha256=calibration_sha256)
    second = paired_groups(spec, records, lanes["s0a1"], lanes["s1a1"], hashes, split,
                           calibration_sha256=calibration_sha256)
    if any(first[index] or second[index] for index in [2, 3, 4, 5]):
        return {
            "quality_interaction": None,
            "quality_interval": None,
            "independent_groups": 0,
            "reason": "incomplete_or_invalid_crossed_evidence",
        }
    right = {(group["repo_id"], group["group_id"]): group for group in second[0]}
    groups = []
    for left in first[0]:
        other = right.get((left["repo_id"], left["group_id"]))
        if other is None or left["case_ids"] != other["case_ids"]:
            return {
                "quality_interaction": None,
                "quality_interval": None,
                "independent_groups": 0,
                "reason": "crossed_groups_do_not_match",
            }
        groups.append(
            {
                "weight": left["weight"],
                "interaction": other["quality_difference"] - left["quality_difference"],
            }
        )
    bounds, bootstrap = interval(groups, "interaction", spec["measurement"], span=4)
    return {
        "quality_interaction": weighted_mean(groups, "interaction") if groups else None,
        "quality_interval": bounds,
        "bootstrap_interval": bootstrap,
        "independent_groups": len(groups),
        "bootstrap_samples": BOOTSTRAP_SAMPLES,
        "reason": "estimate_only_no_adoption_verdict",
    }
