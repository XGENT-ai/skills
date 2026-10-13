use super::*;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PacketRef {
    pub packet_hash: String,
    pub manifest: FileEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnreviewedUnit {
    pub unit_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PacketCapture {
    pub current: bool,
    pub input: CaptureInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub cycle_id: String,
    pub round_id: String,
    pub target_fingerprint: String,
    pub units: Vec<String>,
    pub capture_count: usize,
    pub unreviewed_units: Vec<UnreviewedUnit>,
    pub captures: Vec<PacketCapture>,
    #[serde(default)]
    pub previous_units: Vec<history::PreviousUnit>,
    pub files: Vec<FileEvidence>,
}

// Plain text retains the expectation wording without navigable destinations,
// embedded HTML, or implementation examples from linked documents.
pub(super) fn expectation_text(markdown: &str) -> String {
    let mut output = String::new();
    let mut code_block = false;
    let mut autolink = false;
    for event in Parser::new_ext(markdown, Options::ENABLE_TABLES) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_block = true,
            Event::End(TagEnd::CodeBlock) => code_block = false,
            _ if code_block => (),
            Event::Start(Tag::Link { link_type, .. }) => {
                autolink = matches!(
                    link_type,
                    pulldown_cmark::LinkType::Autolink | pulldown_cmark::LinkType::Email
                );
            }
            Event::End(TagEnd::Link) => autolink = false,
            Event::Text(text) | Event::Code(text) if !autolink => output.push_str(&text),
            Event::SoftBreak | Event::HardBreak => output.push('\n'),
            Event::End(TagEnd::TableCell) => output.push_str(" | "),
            Event::End(
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item | TagEnd::TableRow,
            ) => output.push('\n'),
            _ => (),
        }
    }
    output
}

fn collect_files(
    directory: &Path,
    prefix: &Path,
    entries: &mut BTreeSet<PathBuf>,
) -> Result<(), StoreError> {
    for entry in fs::read_dir(directory).map_err(|e| error(3, e))? {
        let entry = entry.map_err(|e| error(3, e))?;
        let meta = fs::symlink_metadata(entry.path()).map_err(|e| error(3, e))?;
        let path = prefix.join(entry.file_name());
        if meta.is_dir() {
            collect_files(&entry.path(), &path, entries)?;
        } else if meta.is_file() {
            entries.insert(path);
        } else {
            return Err(error(3, "Packets cannot contain links or special files"));
        }
    }
    Ok(())
}

pub(super) fn load(
    lock: &CycleLock,
    round: &Round,
    reference: &PacketRef,
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<Manifest, StoreError> {
    let cycle =
        load_cycle(lock, &round.prefix)?.ok_or_else(|| error(4, "Managed cycle is missing"))?;
    let key = format!(
        "packet:{}:{}:{}",
        hash(&encode(&cycle)?),
        hash(&encode(round)?),
        reference.packet_hash
    );
    lock.memoized(key, sources, |sources| {
        validate(lock, round, reference, sources)
    })
}

fn validate(
    lock: &CycleLock,
    round: &Round,
    reference: &PacketRef,
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<Manifest, StoreError> {
    let directory = round_path(&round.prefix, ordinal(&round.round_id)?)
        .parent()
        .unwrap()
        .join("judge-pack")
        .join(&reference.packet_hash);
    if reference.packet_hash.len() != 64
        || !reference
            .packet_hash
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        || reference.manifest.path != directory.join("manifest.json").to_string_lossy()
        || reference.manifest.sha256 != reference.packet_hash
    {
        return Err(error(4, "Packet identity or manifest binding is invalid"));
    }
    let manifest: Manifest =
        serde_json::from_slice(&require_file(lock, &reference.manifest, sources)?)
            .map_err(|e| error(4, e))?;
    let mut units = BTreeSet::new();
    if manifest.schema_version != 1
        || manifest.cycle_id != round.cycle_id
        || manifest.round_id != round.round_id
        || manifest.target_fingerprint != round.target_fingerprint
        || manifest.units.is_empty()
        || manifest.capture_count > round.captures.len()
        || manifest.units.iter().any(|unit| {
            !round.matrix.selected_units().contains(&unit.as_str()) || !units.insert(unit)
        })
    {
        return Err(error(4, "Packet schema, units or round binding changed"));
    }
    let captures = packet_captures(round, manifest.capture_count, &manifest.units)?;
    if encode(&captures)? != encode(&manifest.captures)? {
        return Err(error(
            4,
            "Packet captures do not match the registered evidence",
        ));
    }
    let cycle =
        load_cycle(lock, &round.prefix)?.ok_or_else(|| error(4, "Managed cycle is missing"))?;
    let (mut previous, previous_files) =
        history::materials(lock, &cycle, round, &manifest.units, sources)?;
    history::validate_changes(&manifest.previous_units, round, &manifest.files)?;
    for unit in &mut previous {
        if let Some(exported) = manifest
            .previous_units
            .iter()
            .find(|old| old.unit_id == unit.unit_id)
        {
            unit.target_changes = exported.target_changes.clone();
        }
    }
    if encode(&previous)? != encode(&manifest.previous_units)?
        || previous_files.iter().any(|(path, bytes)| {
            !manifest.files.contains(&FileEvidence {
                path: path.clone(),
                sha256: hash(bytes),
            })
        })
    {
        return Err(error(
            4,
            "Packet previous-unit material does not match the nearest sealed rounds",
        ));
    }
    let mut expected = BTreeSet::from([PathBuf::from("manifest.json")]);
    for file in &manifest.files {
        let local = relative(&file.path)?;
        if !expected.insert(local.clone()) {
            return Err(error(4, "Packet files must be unique"));
        }
        require_file(
            lock,
            &FileEvidence {
                path: directory.join(local).to_string_lossy().into_owned(),
                sha256: file.sha256.clone(),
            },
            sources,
        )?;
    }
    for capture in &captures {
        if let Observation::Captured { image, .. } = &capture.input.observation {
            if !manifest.files.contains(image) {
                return Err(error(
                    4,
                    "Packet capture image is missing from the file manifest",
                ));
            }
        }
    }
    let missing_units: BTreeSet<_> = round
        .matrix
        .selected_units()
        .into_iter()
        .filter(|unit| !manifest.units.iter().any(|selected| selected == unit))
        .collect();
    let declared_missing: BTreeSet<_> = manifest
        .unreviewed_units
        .iter()
        .map(|unit| unit.unit_id.as_str())
        .collect();
    if declared_missing != missing_units
        || declared_missing.len() != manifest.unreviewed_units.len()
        || manifest
            .unreviewed_units
            .iter()
            .any(|unit| unit.reason.trim().is_empty())
    {
        return Err(error(
            4,
            "Packet unreviewed units do not match its frozen scope",
        ));
    }
    let mut actual = BTreeSet::new();
    collect_files(&lock.path(&directory)?, Path::new(""), &mut actual)?;
    if actual != expected {
        return Err(error(
            3,
            "Packet directory contains missing or unlisted files",
        ));
    }
    Ok(manifest)
}

fn packet_captures(
    round: &Round,
    count: usize,
    units: &[String],
) -> Result<Vec<PacketCapture>, StoreError> {
    let registered = &round.captures[..count];
    registered
        .iter()
        .filter(|entry| units.contains(&entry.input.cell.unit_id))
        .map(|entry| {
            let current = !registered.iter().any(|other| {
                other.input.cell == entry.input.cell && other.input.attempt > entry.input.attempt
            });
            let mut input = entry.input.clone();
            if let Observation::Captured { image, .. } = &mut input.observation {
                image.path = format!(
                    "current/{}",
                    round.matrix.screenshot_name(&input.cell, input.attempt)?
                );
            }
            Ok(PacketCapture { current, input })
        })
        .collect()
}

pub(super) fn current(round: &Round, manifest: &Manifest) -> Result<(), StoreError> {
    for cell in round
        .matrix
        .cells
        .iter()
        .filter(|cell| manifest.units.contains(&cell.key.unit_id))
    {
        let latest = round
            .captures
            .iter()
            .filter(|capture| capture.input.cell == cell.key)
            .max_by_key(|capture| capture.input.attempt);
        let frozen = manifest
            .captures
            .iter()
            .find(|capture| capture.current && capture.input.cell == cell.key);
        if !latest.zip(frozen).is_some_and(|(latest, frozen)| {
            latest.input.attempt == frozen.input.attempt
                && matches!(frozen.input.observation, Observation::Captured { .. })
        }) {
            return Err(error(
                3,
                "Packet is incomplete or stale after current evidence changed",
            ));
        }
    }
    Ok(())
}

pub fn create(
    project: &Path,
    path: &Path,
    units: Option<Vec<String>>,
) -> Result<PacketRef, StoreError> {
    create_with_changes(project, path, units, Vec::new())
}

pub fn create_with_changes(
    project: &Path,
    path: &Path,
    units: Option<Vec<String>>,
    changes: Vec<history::TargetChange>,
) -> Result<PacketRef, StoreError> {
    create_packet(project, path, units, Some(changes))
}

/// Binds each changed unit to the frozen target package's change record. Every
/// field is fixed by frozen facts (previous and current fingerprints, the frozen
/// target source), so the caller has nothing to choose.
pub fn create_derived(
    project: &Path,
    path: &Path,
    units: Option<Vec<String>>,
) -> Result<PacketRef, StoreError> {
    create_packet(project, path, units, None)
}

fn create_packet(
    project: &Path,
    path: &Path,
    units: Option<Vec<String>>,
    changes: Option<Vec<history::TargetChange>>,
) -> Result<PacketRef, StoreError> {
    let root = fs::canonicalize(project).map_err(|e| error(4, e))?;
    let prefix = prefix_from_round(path)?;
    let index = relative(&format!("{prefix}.ui-review.json"))?;
    let lock = CycleLock::acquire_project(&root, &index)?;
    lock.recover_publication()?;
    let cycle = load_cycle(&lock, &prefix)?.ok_or_else(|| error(4, "Managed cycle is missing"))?;
    let (before, mut round) = load_round(&lock, &cycle, path)?;
    if round.finalization.is_some() {
        return Err(error(3, "Finalized rounds cannot create new packets"));
    }
    let (mut sources, target) = verified_target(&root, &lock, &cycle, &round, path)?;
    let selected = round.matrix.selected_units();
    let requested =
        units.unwrap_or_else(|| selected.iter().map(|unit| (*unit).to_owned()).collect());
    let mut unique = BTreeSet::new();
    if requested.is_empty()
        || requested
            .iter()
            .any(|unit| !selected.contains(&unit.as_str()) || !unique.insert(unit))
    {
        return Err(error(
            2,
            "Packet units must list unique units in the round scope",
        ));
    }
    // Packets that registered judges used must stay intact; an unused packet need not
    // reproduce (e.g. one exported under an earlier engine's material rules).
    for file in &round.judges {
        judgement::load(&lock, &round, &cycle, file, &mut sources)?;
    }
    let complete = |unit: &str| {
        target.missing_for(unit).next().is_none()
            && round
                .matrix
                .cells
                .iter()
                .filter(|cell| cell.key.unit_id == unit)
                .all(|cell| {
                    round
                        .captures
                        .iter()
                        .filter(|entry| entry.input.cell == cell.key)
                        .max_by_key(|entry| entry.input.attempt)
                        .is_some_and(|entry| {
                            matches!(entry.input.observation, Observation::Captured { .. })
                        })
                })
    };
    let units: Vec<String> = requested
        .iter()
        .filter(|unit| complete(unit))
        .cloned()
        .collect();
    if units.is_empty() {
        return Err(error(2, "No selected unit has complete evidence"));
    }
    let unreviewed_units = selected
        .iter()
        .filter(|unit| !units.iter().any(|u| u == **unit))
        .map(|unit| UnreviewedUnit {
            unit_id: (*unit).into(),
            reason: if requested.iter().any(|u| u == *unit) {
                "Target or current capture evidence is incomplete".into()
            } else {
                "Outside this packet's selected units".into()
            },
        })
        .collect();
    let (mut previous_units, mut files) =
        history::materials(&lock, &cycle, &round, &units, &mut sources)?;
    let changes = changes.unwrap_or_else(|| {
        let mut derived: Vec<history::TargetChange> = Vec::new();
        for old in previous_units
            .iter()
            .filter(|old| old.target_fingerprint != round.target_fingerprint)
        {
            match derived
                .iter_mut()
                .find(|change| change.from_fingerprint == old.target_fingerprint)
            {
                Some(change) => change.units.push(old.unit_id.clone()),
                None => derived.push(history::TargetChange {
                    from_fingerprint: old.target_fingerprint.clone(),
                    to_fingerprint: round.target_fingerprint.clone(),
                    units: vec![old.unit_id.clone()],
                    source: round.target_snapshot.source.clone(),
                }),
            }
        }
        derived
    });
    history::apply_changes(
        &lock,
        &round,
        &mut previous_units,
        changes,
        &mut files,
        &mut sources,
    )?;
    let expectations = require_file(&lock, &round.target_snapshot.expectations, &mut sources)?;
    let mut text = expectation_text(std::str::from_utf8(&expectations).map_err(|e| error(4, e))?);
    for (n, attachment) in round.target_snapshot.attachments.iter().enumerate() {
        let Some(file) = &attachment.frozen else {
            continue;
        };
        let name = Path::new(&attachment.logical_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let extension = Path::new(name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp") {
            let name = format!("target/{n:04}.{extension}");
            text.push_str(&format!("\n目标图片：{name}\n"));
            files.push((name, require_file(&lock, file, &mut sources)?));
        } else if name.eq_ignore_ascii_case("DESIGN.md") {
            let bytes = require_file(&lock, file, &mut sources)?;
            text.push_str("\n已接受项目规范：\n");
            text.push_str(&expectation_text(
                std::str::from_utf8(&bytes).map_err(|e| error(2, e))?,
            ));
        } else if name.eq_ignore_ascii_case("tokens.json")
            || name.eq_ignore_ascii_case("token.json")
            || name.ends_with(".tokens.json")
            || name.ends_with(".token.json")
        {
            let bytes = require_file(&lock, file, &mut sources)?;
            let tokens: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|e| error(2, e))?;
            text.push_str("\n规范 token 值：\n");
            text.push_str(&serde_json::to_string_pretty(&tokens).map_err(|e| error(2, e))?);
        }
    }
    files.push(("expectations.txt".into(), text.into_bytes()));
    let mut captures = Vec::new();
    for entry in &round.captures {
        if !units.contains(&entry.input.cell.unit_id) {
            continue;
        }
        let current = !round.captures.iter().any(|other| {
            other.input.cell == entry.input.cell && other.input.attempt > entry.input.attempt
        });
        let mut input = entry.input.clone();
        if let Observation::Captured { image, .. } = &mut input.observation {
            let frozen = entry.frozen_image.as_ref().unwrap();
            let filename = round.matrix.screenshot_name(&input.cell, input.attempt)?;
            let name = format!("current/{filename}");
            files.push((name.clone(), require_file(&lock, frozen, &mut sources)?));
            image.path = name;
        }
        captures.push(PacketCapture { current, input });
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let manifest = Manifest {
        schema_version: 1,
        cycle_id: round.cycle_id.clone(),
        round_id: round.round_id.clone(),
        target_fingerprint: round.target_fingerprint.clone(),
        units,
        capture_count: round.captures.len(),
        unreviewed_units,
        captures,
        previous_units,
        files: files
            .iter()
            .map(|(path, bytes)| FileEvidence {
                path: path.clone(),
                sha256: hash(bytes),
            })
            .collect(),
    };
    let bytes = encode(&manifest)?;
    let packet_hash = hash(&bytes);
    if let Some(reference) = round
        .packets
        .iter()
        .find(|old| old.packet_hash == packet_hash)
    {
        load(&lock, &round, reference, &mut sources)?;
        return Ok(reference.clone());
    }
    let directory = path.parent().unwrap().join("judge-pack").join(&packet_hash);
    let reference = PacketRef {
        packet_hash: packet_hash.clone(),
        manifest: FileEvidence {
            path: directory
                .join("manifest.json")
                .to_string_lossy()
                .into_owned(),
            sha256: packet_hash,
        },
    };
    let mut output: Vec<ReportFile> = files
        .into_iter()
        .map(|(file, bytes)| ReportFile {
            path: directory.join(file),
            expected_hash: None,
            bytes,
        })
        .collect();
    output.push(ReportFile {
        path: relative(&reference.manifest.path)?,
        expected_hash: None,
        bytes,
    });
    round.packets.push(reference.clone());
    round.revision = round
        .revision
        .checked_add(1)
        .ok_or_else(|| error(2, "Round revision limit reached"))?;
    if let Some(snapshot) = lock.read(&index)? {
        sources.push((index, snapshot.hash));
    }
    lock.prepare_draft(path, Some(&before), &encode(&round)?, &output, &sources)?;
    lock.recover_publication()?;
    Ok(reference)
}
