"""Paired task/group estimates; never edit trials or decide confirmation state."""

from collections import defaultdict
import math
import random

from protocol import canonical, sha256


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


def paired_groups(spec, records, baseline, candidate, hashes, split):
    cases = {case["case_id"]: case for case in spec["cases"] if case["split"] == split}
    repeats = spec["measurement"]["repeats"]
    relevant = [
        row
        for row in records
        if row.get("split") == split
        and row.get("candidate_id") in {baseline, candidate}
    ]
    slots, by_id, errors = defaultdict(list), {}, []
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
        if status not in {"ok", "behavior_failure", "infra_invalid"}:
            errors.append("unknown trial status")
        elif status == "infra_invalid":
            if score is not None:
                errors.append("infrastructure failure cannot have a behavior score")
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
            qualities.append(pair[1]["score"] - pair[0]["score"])
            costs.append(None if None in pair_cost else pair_cost[0] - pair_cost[1])
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
        if not complete:
            continue
        for guardrail in spec["guardrails"]:
            observed = checks[guardrail["name"]]
            if (
                len(observed) == repeats
                and sum(observed) / repeats < guardrail["minimum"]
            ):
                regressions.append({"case_id": case_id, "check": guardrail["name"]})
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


def compare_results(spec, records, baseline, candidate, hashes, *, split):
    measurement = spec["measurement"]
    groups, relevant, errors, missing, regressions, unknown_checks = paired_groups(
        spec, records, baseline, candidate, hashes, split
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
    if groups:
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
    return report


def interaction_results(spec, records, lanes, hashes, *, split):
    required = {"s0a0", "s1a0", "s0a1", "s1a1"}
    if set(lanes) != required or len(set(lanes.values())) != 4:
        raise ValueError("interaction requires four distinct predeclared conditions")
    first = paired_groups(spec, records, lanes["s0a0"], lanes["s1a0"], hashes, split)
    second = paired_groups(spec, records, lanes["s0a1"], lanes["s1a1"], hashes, split)
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
