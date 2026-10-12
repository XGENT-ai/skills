//! Read-only `review status`: never takes the cycle lock, never recovers or
//! writes. Damaged, future-schema or drifted state is reported as unknown.
use super::*;
use crate::policy::Verdict;
use serde_json::{json, Value};

pub fn inspect(project: &Path, prefix: &str) -> Result<Value, StoreError> {
    relative(prefix)?;
    let root = fs::canonicalize(project).map_err(|e| error(4, e))?;
    let index = relative(&format!("{prefix}.ui-review.json"))?;
    if !checked(&root, &index.to_string_lossy())?.exists() {
        return Ok(if manual_history(&root, prefix)? {
            json!({"state": "manual", "prefix": prefix, "verdict": null,
                "reason": "Markdown review history exists without a managed index; continue manually or import it explicitly",
                "next": ["import", "continue the manual loop"]})
        } else {
            json!({"state": "absent", "prefix": prefix, "verdict": null, "next": ["begin"]})
        });
    }
    let lock = CycleLock::observe(&root, &index)?;
    Ok(managed(&lock, prefix).unwrap_or_else(|problem| {
        json!({"state": "unknown", "prefix": prefix, "verdict": null,
            "problem": {"exitCode": problem.code, "message": problem.message},
            "reason": "Managed state is damaged, from an unsupported schema, or no longer matches its evidence; it is neither absent nor passed",
            "next": ["keep the history read-only; restore the matching engine or the changed files, or import an explicit mapping"]})
    }))
}

fn intact(
    lock: &CycleLock,
    path: &Path,
    block: &str,
    expected: Option<&str>,
) -> Result<bool, StoreError> {
    let bytes = match fs::read(lock.path(path)?) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(expected.is_none()),
        Err(e) => return Err(error(4, e)),
    };
    Ok(project_block(&bytes, block, expected, b"probe").is_ok())
}

fn managed(lock: &CycleLock, prefix: &str) -> Result<Value, StoreError> {
    let pending = lock.pending_publication()?;
    let cycle = load_cycle(lock, prefix)?.ok_or_else(|| error(4, "Managed cycle is missing"))?;
    let applied = applied::load(lock, &cycle, &mut Vec::new())?;
    let legacy = legacy::load(lock, &cycle, &mut Vec::new())?;
    let mut rounds = Vec::new();
    let mut sealed: Vec<(Round, finalize::ResultRecord)> = Vec::new();
    let mut current = Value::Null;
    let mut latest: Option<Round> = None;
    for link in &cycle.rounds {
        let path = Path::new(&link.path);
        if pending.as_deref() == Some(path) {
            let revision = lock.read(path)?.map(|snapshot| snapshot.revision);
            let entry = json!({"roundId": link.round_id, "path": link.path, "state": "publishing", "revision": revision});
            rounds.push(entry.clone());
            current = entry;
            continue;
        }
        let (_, round) = load_round(lock, &cycle, path)?;
        let mut entry = json!({"roundId": round.round_id, "path": link.path, "revision": round.revision,
            "full": round.full, "scope": round.scope, "baseline": round.baseline,
            "targetFingerprint": round.target_fingerprint, "testedVersionKnown": round.tested_version.known()});
        if round.finalization.is_some() {
            let result = finalize::load_result(lock, &cycle, &round, &mut Vec::new())?;
            entry["state"] = json!("finalized");
            entry["verdict"] = json!(result.outcome.verdict);
            entry["score"] = json!(result.outcome.scoring.score);
            entry["historicalBest"] = json!(result.outcome.scoring.historical_best);
            entry["waived"] = json!(cycle.waivers.iter().any(|w| w.round_id == round.round_id));
            sealed.push((round.clone(), result));
        } else {
            let packets = round.packets.len();
            let judges = round.judges.len();
            let missing = missing_cells(&round);
            entry["state"] = json!("draft");
            current = json!({"roundId": round.round_id, "path": link.path, "state": "draft",
                "revision": round.revision, "full": round.full, "scope": round.scope,
                "missingCells": missing, "packets": packets, "judges": judges});
        }
        rounds.push(entry);
        latest = Some(round);
    }
    let last = sealed.last();
    let mut conflicts = Vec::new();
    for (round, result) in &sealed {
        let bytes =
            fs::read(lock.path(Path::new(&result.report.path))?).map_err(|e| error(4, e))?;
        if !finalize::waiver_intact(&cycle, &round.round_id, &bytes)? {
            conflicts.push(result.report.path.clone());
        }
    }
    if let Some(round) = latest.as_ref().filter(|round| round.finalization.is_none()) {
        let list = round_path(prefix, ordinal(&round.round_id)?)
            .parent()
            .unwrap()
            .join("capture.md");
        if !intact(
            lock,
            &list,
            &format!("{}-{}-capture", cycle.cycle_id, round.round_id),
            round.capture_projection_hash.as_deref(),
        )? {
            conflicts.push(list.to_string_lossy().into_owned());
        }
    }
    if let Some(ledger) = &applied.ledger {
        if !intact(
            lock,
            Path::new(&ledger.record_path),
            &format!("apply-{}", &cycle.cycle_id[..16]),
            Some(&ledger.projection_hash),
        )? {
            conflicts.push(ledger.record_path.clone());
        }
    }
    let units: Vec<Value> = latest
        .as_ref()
        .map(|round| {
            round
                .matrix
                .units
                .iter()
                .map(|unit| {
                    let id = unit.identity.id.as_str();
                    let covered = sealed
                        .iter()
                        .rev()
                        .find(|(round, _)| round.matrix.selected_units().contains(&id));
                    let conclusion = covered.and_then(|(_, result)| {
                        result.units.iter().find(|conclusion| conclusion.unit_id == id)
                    });
                    json!({"unitId": id, "lastCoveredRound": covered.map(|(round, _)| &round.round_id),
                        "score": conclusion.and_then(|c| c.score),
                        "openGaps": last.map(|(_, result)| result.units.iter().flat_map(|u| &u.gaps)
                            .chain(&result.carried)
                            .filter(|gap| gap.unit_id == id && !finalize::closed(gap.status))
                            .map(|gap| gap.id.clone()).collect::<Vec<_>>()).unwrap_or_default()})
                })
                .collect()
        })
        .unwrap_or_default();
    let legacy_open = || -> Vec<Value> {
        legacy
            .iter()
            .flat_map(|map| map.open_gaps())
            .map(|gap| {
                json!({"id": gap.id, "unitId": gap.unit_id, "severity": gap.severity,
                    "status": gap.assessments.last().unwrap().status, "origin": gap.origin,
                    "pendingDecision": gap.pending_decision, "legacy": true})
            })
            .collect()
    };
    let open: Vec<Value> = last
        .map(|(_, result)| {
            result
                .units
                .iter()
                .flat_map(|unit| &unit.gaps)
                .chain(&result.carried)
                .filter(|gap| !finalize::closed(gap.status))
                .map(|gap| {
                    json!({"id": gap.id, "unitId": gap.unit_id, "severity": gap.severity,
                        "status": gap.status, "origin": gap.origin,
                        "pendingDecision": result.outcome.pending_decisions.contains(&gap.id),
                        "attempts": gap.history.iter().filter(|a| a.prior_disposition.attempted).count()})
                })
                .collect()
        })
        .unwrap_or_else(legacy_open);
    let verdict = last.and_then(|(_, result)| result.outcome.verdict.as_ref());
    let waived =
        last.is_some_and(|(round, _)| cycle.waivers.iter().any(|w| w.round_id == round.round_id));
    let next: Vec<&str> = if pending.is_some() {
        vec!["rerun the interrupted write command to recover its publication"]
    } else if current["state"] == "draft" {
        if !current["missingCells"].as_array().unwrap().is_empty() {
            vec!["capture add"]
        } else if current["judges"] == 0 {
            vec!["judge-pack", "judge add"]
        } else {
            vec!["finalize"]
        }
    } else {
        match verdict {
            None => vec!["begin"],
            Some(Verdict::NeedModification) => vec!["apply record", "begin"],
            Some(Verdict::NeedEvidence) => vec!["begin"],
            Some(Verdict::NeedDecision) => vec![
                "user decision",
                "finalize (whole-UI waiver)",
                "begin (after a confirmed target change)",
            ],
            Some(Verdict::Passed) => vec!["done", "apply record (optional polish)"],
        }
    };
    Ok(json!({
        "state": "managed", "prefix": prefix, "publicationPending": pending.is_some(),
        "cycle": {"cycleId": cycle.cycle_id, "revision": cycle.revision, "target": cycle.target,
            "plan": cycle.plan, "recordPath": cycle.record_path, "nextIssueId": cycle.next_issue_id},
        "baseline": latest.as_ref().map(|round| &round.baseline),
        "rounds": rounds, "current": current,
        "latestFinalizedRound": last.map(|(round, _)| &round.round_id),
        "verdict": verdict, "waived": waived,
        "waivers": cycle.waivers,
        "scoring": last.map(|(_, result)| &result.outcome.scoring),
        "warnings": last.map(|(_, result)| &result.outcome.warnings),
        "pendingDecisions": last.map(|(_, result)| &result.outcome.pending_decisions),
        "units": units, "openGaps": open, "projectionConflicts": conflicts,
        "legacy": cycle.legacy.as_ref().map(|reference| json!({"marker": reference.marker,
            "input": reference.input, "frozenMap": reference.map, "lastRound": reference.last_round,
            "unknowns": legacy.as_ref().map(|map| map.unknowns()).unwrap_or_default()})),
        "applied": applied.ledger.as_ref().map(|ledger| json!({"revision": ledger.revision, "recordPath": ledger.record_path})),
        "next": next,
    }))
}
