//! Unit-scoped, hash-checked history projections for independent judge packets.
use super::*;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GapIdentity {
    pub id: String,
    pub unit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviousJudge {
    pub reviewer_id: String,
    pub source_sha256: String,
    pub raw_sha256: String,
    pub unit: judgement::UnitInput,
    pub previous: Vec<judgement::OldAssessment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TargetChange {
    pub from_fingerprint: String,
    pub to_fingerprint: String,
    pub units: Vec<String>,
    pub source: FileEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenChange {
    pub from_fingerprint: String,
    pub to_fingerprint: String,
    pub source_sha256: String,
    pub record: FileEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviousUnit {
    pub unit_id: String,
    pub round_id: String,
    pub target_fingerprint: String,
    pub captures: Vec<packet::PacketCapture>,
    pub judges: Vec<PreviousJudge>,
    pub gaps: Vec<GapIdentity>,
    pub unreviewed_reason: Option<String>,
    pub target_changes: Vec<FrozenChange>,
    /// Imported Markdown material when no managed round covered the unit yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy: Option<LegacyPrevious>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyPrevious {
    pub judge: FileEvidence,
    pub images: Vec<FileEvidence>,
}

type Files = BTreeMap<String, Vec<u8>>;
type ExportFiles = Vec<(String, Vec<u8>)>;

fn change_text(markdown: &str) -> Result<String, StoreError> {
    let mut headings = Vec::new();
    let mut depth = 0;
    let mut current: Option<(usize, HeadingLevel, String)> = None;
    for (event, range) in Parser::new_ext(markdown, Options::ENABLE_TABLES).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. })
                if depth == 0 && matches!(level, HeadingLevel::H1 | HeadingLevel::H2) =>
            {
                current = Some((range.start, level, String::new()));
                depth += 1;
            }
            Event::Start(_) => depth += 1,
            Event::Text(value) | Event::Code(value) => {
                if let Some((_, _, name)) = &mut current {
                    name.push_str(&value);
                }
            }
            Event::End(tag) => {
                if depth == 1 && matches!(tag, TagEnd::Heading(HeadingLevel::H1 | HeadingLevel::H2))
                {
                    let (start, level, name) = current.take().unwrap();
                    headings.push((start, range.end, level, name));
                }
                depth -= 1;
            }
            _ => (),
        }
    }
    let sections: Vec<_> = headings
        .iter()
        .enumerate()
        .filter(|(_, (_, _, level, name))| *level == HeadingLevel::H2 && name == "变更记录")
        .collect();
    if sections.len() != 1 {
        return Err(error(
            2,
            "Target needs exactly one real second-level change-record section",
        ));
    }
    let (index, (_, start, _, _)) = sections[0];
    let end = headings
        .get(index + 1)
        .map_or(markdown.len(), |heading| heading.0);
    let text = packet::expectation_text(&markdown[*start..end]);
    if text.trim().is_empty() {
        return Err(error(2, "Target change-record section cannot be empty"));
    }
    Ok(text)
}

fn insert(files: &mut Files, path: String, bytes: Vec<u8>) -> Result<FileEvidence, StoreError> {
    let evidence = FileEvidence {
        path: path.clone(),
        sha256: hash(&bytes),
    };
    if files.get(&path).is_some_and(|old| old != &bytes) {
        return Err(error(4, "Historical packet file binding conflicts"));
    }
    files.insert(path, bytes);
    Ok(evidence)
}

fn image_path(round_id: &str, image: &FileEvidence) -> Result<String, StoreError> {
    let local = relative(&image.path)?;
    if !local.starts_with("current") || local.components().count() != 2 {
        return Err(error(
            4,
            "Historical judge image must belong to its packet's captures",
        ));
    }
    Ok(format!(
        "previous/{round_id}/{}",
        local.file_name().unwrap().to_string_lossy()
    ))
}

fn clean_unit(unit: &mut judgement::UnitInput) {
    for text in unit.reasons.iter_mut().chain(&mut unit.suggestions) {
        *text = packet::expectation_text(text);
    }
    for unseen in &mut unit.unseen_cells {
        unseen.reason = packet::expectation_text(&unseen.reason);
    }
    for gap in &mut unit.gaps {
        for text in [
            &mut gap.location,
            &mut gap.current,
            &mut gap.expected,
            &mut gap.basis_location,
            &mut gap.fix,
        ] {
            *text = packet::expectation_text(text);
        }
        if let Some(summary) = &mut gap.summary {
            *summary = packet::expectation_text(summary);
        }
    }
}

fn captures(
    lock: &CycleLock,
    round: &Round,
    unit: &str,
    files: &mut Files,
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<Vec<packet::PacketCapture>, StoreError> {
    let mut history = Vec::new();
    let cells: Vec<_> = round
        .matrix
        .cells
        .iter()
        .map(|cell| cell.key.clone())
        .collect();
    let directory = round_path(&round.prefix, ordinal(&round.round_id)?);
    let mut output = Vec::new();
    for entry in &round.captures {
        let filename = round
            .matrix
            .screenshot_name(&entry.input.cell, entry.input.attempt)?;
        if entry.input.schema_version != 1
            || entry.input.round_id != round.round_id
            || entry.content_hash
                != hash(&serde_json::to_vec(&entry.input).map_err(|e| error(4, e))?)
            || !append_capture(&cells, &mut history, capture_attempt(entry), false)
                .map_err(|e| error(4, e))?
        {
            return Err(error(
                4,
                "Historical capture identity, attempts or content changed",
            ));
        }
        let mut input = entry.input.clone();
        match &mut input.observation {
            Observation::Missing { reason } => {
                if reason.trim().is_empty()
                    || entry.frozen_image.is_some()
                    || entry.image_pixels.is_some()
                {
                    return Err(error(4, "Historical missing capture cannot claim an image"));
                }
            }
            Observation::Captured {
                image,
                viewport,
                tested_version,
                human_check,
                ..
            } => {
                let frozen = entry
                    .frozen_image
                    .as_ref()
                    .ok_or_else(|| error(4, "Historical frozen capture is missing"))?;
                let cell = round
                    .matrix
                    .cells
                    .iter()
                    .find(|cell| cell.key == input.cell)
                    .unwrap();
                if relative(&frozen.path)?
                    != directory.parent().unwrap().join("evidence").join(&filename)
                    || Path::new(&image.path) != directory.parent().unwrap().join(&filename)
                    || image.sha256 != frozen.sha256
                    || viewport.as_ref() != &cell.viewport
                    || tested_version.as_ref() != &round.tested_version
                    || !human_check.opened
                    || !human_check.current_product
                    || !human_check.current_version
                    || human_check.notes.trim().is_empty()
                {
                    return Err(error(
                        4,
                        "Historical capture snapshot or provenance binding changed",
                    ));
                }
                let bytes = require_file(lock, frozen, sources)?;
                let mut reader = png::Decoder::new(std::io::Cursor::new(&bytes))
                    .read_info()
                    .map_err(|e| error(4, e))?;
                let pixels = [reader.info().width, reader.info().height];
                let expected = [
                    (f64::from(viewport.width) * viewport.dpr).round() as u32,
                    (f64::from(viewport.height) * viewport.dpr).round() as u32,
                ];
                if entry.image_pixels != Some(pixels)
                    || pixels[0] != expected[0]
                    || (!viewport.full_page && pixels[1] != expected[1])
                    || (viewport.full_page && pixels[1] < expected[1])
                {
                    return Err(error(4, "Historical capture pixel binding changed"));
                }
                let size = reader
                    .output_buffer_size()
                    .filter(|size| *size <= 64 * 1024 * 1024)
                    .ok_or_else(|| {
                        error(4, "Historical capture exceeds the pixel resource limit")
                    })?;
                reader
                    .next_frame(&mut vec![0; size])
                    .map_err(|e| error(4, e))?;
                reader.finish().map_err(|e| error(4, e))?;
                if input.cell.unit_id == unit {
                    *image = insert(
                        files,
                        format!("previous/{}/{filename}", round.round_id),
                        bytes,
                    )?;
                }
            }
        }
        if input.cell.unit_id == unit {
            let current = !round
                .captures
                .iter()
                .any(|other| other.input.cell == input.cell && other.input.attempt > input.attempt);
            output.push(packet::PacketCapture { current, input });
        }
    }
    Ok(output)
}

pub(super) fn materials(
    lock: &CycleLock,
    cycle: &Cycle,
    round: &Round,
    units: &[String],
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<(Vec<PreviousUnit>, ExportFiles), StoreError> {
    let mut output = Vec::new();
    let mut files = Files::new();
    let legacy = legacy::load(lock, cycle, sources)?;
    for unit in units {
        let covered = output.len();
        for link in cycle.rounds.iter().rev() {
            if ordinal(&link.round_id)? >= ordinal(&round.round_id)? {
                continue;
            }
            let (snapshot, previous) = load_round(lock, cycle, Path::new(&link.path))?;
            sources.push((relative(&link.path)?, snapshot.hash));
            if !previous.matrix.selected_units().contains(&unit.as_str()) {
                continue;
            }
            let result = finalize::load_result(lock, cycle, &previous, sources)?;
            let conclusion = result
                .units
                .iter()
                .find(|old| old.unit_id == *unit)
                .ok_or_else(|| error(4, "Historical unit conclusion is missing"))?;
            let captures = captures(lock, &previous, unit, &mut files, sources)?;
            let mut judges = Vec::new();
            for file in &conclusion.judge_refs {
                let record = judgement::load(lock, &previous, cycle, file, sources)?;
                let mut scored = record
                    .input
                    .units
                    .iter()
                    .find(|old| old.unit_id == *unit)
                    .ok_or_else(|| error(4, "Historical judge does not cover its assigned unit"))?
                    .clone();
                let packet_directory = round_path(&round.prefix, ordinal(&previous.round_id)?);
                let packet_directory = packet_directory
                    .parent()
                    .unwrap()
                    .join("judge-pack")
                    .join(&record.input.packet_hash);
                for gap in &mut scored.gaps {
                    let assignment = record
                        .assignments
                        .iter()
                        .find(|assignment| assignment.key == gap.key && assignment.unit_id == *unit)
                        .ok_or_else(|| error(4, "Historical issue assignment is missing"))?;
                    gap.key = assignment.id.clone();
                    gap.image.path = image_path(&previous.round_id, &gap.image)?;
                    let basis_source = FileEvidence {
                        path: packet_directory
                            .join(relative(&gap.basis.path)?)
                            .to_string_lossy()
                            .into_owned(),
                        sha256: gap.basis.sha256.clone(),
                    };
                    let bytes = require_file(lock, &basis_source, sources)?;
                    gap.basis = insert(
                        &mut files,
                        format!("previous/{}/{}", previous.round_id, gap.basis.path),
                        bytes,
                    )?;
                }
                let mut assessments = Vec::new();
                for assessment in &record.input.previous {
                    let judgement::AssessmentBasis::Current(basis) = &assessment.basis else {
                        return Err(error(
                            4,
                            "Historical assessments need current-image evidence",
                        ));
                    };
                    if basis.cell.unit_id != *unit {
                        continue;
                    }
                    let mut assessment = assessment.clone();
                    let judgement::AssessmentBasis::Current(basis) = &mut assessment.basis else {
                        unreachable!()
                    };
                    basis.image.path = image_path(&previous.round_id, &basis.image)?;
                    basis.notes = packet::expectation_text(&basis.notes);
                    assessments.push(assessment);
                }
                clean_unit(&mut scored);
                judges.push(PreviousJudge {
                    reviewer_id: record.input.reviewer_id.clone(),
                    source_sha256: file.sha256.clone(),
                    raw_sha256: record.raw.sha256,
                    unit: scored,
                    previous: assessments,
                });
            }
            output.push(PreviousUnit {
                unit_id: unit.clone(),
                round_id: previous.round_id,
                target_fingerprint: previous.target_fingerprint,
                captures,
                judges,
                // Reviewers reassess the unit's open reviewer-sourced gaps; host-measured
                // gaps are rechecked by the host and closed gaps are not reopened.
                gaps: conclusion
                    .gaps
                    .iter()
                    .filter(|gap| {
                        gap.origin == finalize::GapOrigin::Reviewer && !finalize::closed(gap.status)
                    })
                    .map(|gap| GapIdentity {
                        id: gap.id.clone(),
                        unit_id: unit.clone(),
                    })
                    .collect(),
                unreviewed_reason: conclusion.unreviewed_reason.clone(),
                target_changes: Vec::new(),
                legacy: None,
            });
            break;
        }
        if let Some((map, material)) = legacy
            .as_ref()
            .filter(|_| output.len() == covered)
            .and_then(|map| {
                map.units
                    .iter()
                    .find(|material| material.unit_id == *unit)
                    .map(|material| (map, material))
            })
        {
            let judge = require_file(lock, &material.judge, sources)?;
            let text =
                packet::expectation_text(std::str::from_utf8(&judge).map_err(|e| error(4, e))?);
            let judge = insert(
                &mut files,
                format!("previous/{}/legacy-{unit}.txt", material.round_id),
                text.into_bytes(),
            )?;
            let mut images = Vec::new();
            for image in &material.images {
                let bytes = require_file(lock, image, sources)?;
                let name = Path::new(&image.path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                images.push(insert(
                    &mut files,
                    format!("previous/{}/{name}", material.round_id),
                    bytes,
                )?);
            }
            output.push(PreviousUnit {
                unit_id: unit.clone(),
                round_id: material.round_id.clone(),
                target_fingerprint: map
                    .round(&material.round_id)
                    .and_then(|round| round.target_fingerprint.as_ref())
                    .map_or_else(|| "unknown".into(), |fingerprint| fingerprint.value.clone()),
                captures: Vec::new(),
                judges: Vec::new(),
                gaps: map
                    .open_gaps()
                    .filter(|gap| {
                        gap.unit_id == *unit && gap.origin == finalize::GapOrigin::Reviewer
                    })
                    .map(|gap| GapIdentity {
                        id: gap.id.clone(),
                        unit_id: unit.clone(),
                    })
                    .collect(),
                unreviewed_reason: None,
                target_changes: Vec::new(),
                legacy: Some(LegacyPrevious { judge, images }),
            });
        }
    }
    Ok((output, files.into_iter().collect()))
}

pub(super) fn apply_changes(
    lock: &CycleLock,
    round: &Round,
    previous: &mut [PreviousUnit],
    changes: Vec<TargetChange>,
    files: &mut ExportFiles,
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<(), StoreError> {
    let mut covered = BTreeSet::new();
    for change in changes {
        let mut unique = BTreeSet::new();
        if change.units.is_empty() || change.to_fingerprint != round.target_fingerprint {
            return Err(error(
                2,
                "Target changes must bind assigned units and the current fingerprint",
            ));
        }
        if change.source.path != round.target_snapshot.source.path {
            return Err(error(
                2,
                "Target changes must come from this round's actual target package",
            ));
        }
        if change.source.sha256 != round.target_snapshot.source.sha256 {
            return Err(error(
                3,
                "Target change source hash does not match the frozen target package",
            ));
        }
        let bytes = require_file(lock, &change.source, sources)?;
        let text = change_text(std::str::from_utf8(&bytes).map_err(|e| error(2, e))?)?;
        for unit in change.units {
            let old = previous
                .iter_mut()
                .find(|old| old.unit_id == unit)
                .ok_or_else(|| error(2, "Target change names a unit without previous material"))?;
            if !unique.insert(unit.clone())
                || !covered.insert(unit.clone())
                || old.target_fingerprint == round.target_fingerprint
                || old.target_fingerprint != change.from_fingerprint
            {
                return Err(error(
                    2,
                    "Target change coverage or previous fingerprint is invalid",
                ));
            }
            let path = format!("previous/{}/changes/{unit}.txt", old.round_id);
            old.target_changes.push(FrozenChange {
                from_fingerprint: change.from_fingerprint.clone(),
                to_fingerprint: change.to_fingerprint.clone(),
                source_sha256: change.source.sha256.clone(),
                record: FileEvidence {
                    path: path.clone(),
                    sha256: hash(text.as_bytes()),
                },
            });
            files.push((path, text.as_bytes().to_vec()));
        }
    }
    validate_changes(
        previous,
        round,
        &files
            .iter()
            .map(|(path, bytes)| FileEvidence {
                path: path.clone(),
                sha256: hash(bytes),
            })
            .collect::<Vec<_>>(),
    )
    .map_err(|e| error(2, e.message))
}

pub(super) fn validate_changes(
    previous: &[PreviousUnit],
    round: &Round,
    files: &[FileEvidence],
) -> Result<(), StoreError> {
    for old in previous {
        let required = usize::from(old.target_fingerprint != round.target_fingerprint);
        if old.target_changes.len() != required {
            return Err(error(
                4,
                "Changed target expectations require an explicit actual change record",
            ));
        }
        for change in &old.target_changes {
            if change.from_fingerprint != old.target_fingerprint
                || change.to_fingerprint != round.target_fingerprint
                || change.source_sha256.len() != 64
                || !change
                    .source_sha256
                    .bytes()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
                || change.record.path
                    != format!("previous/{}/changes/{}.txt", old.round_id, old.unit_id)
                || !files.contains(&change.record)
            {
                return Err(error(
                    4,
                    "Frozen target change fingerprint, source hash or file binding changed",
                ));
            }
        }
    }
    Ok(())
}
