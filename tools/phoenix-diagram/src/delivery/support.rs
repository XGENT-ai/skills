use super::*;
use crate::check::{ArtifactCheck, FileIdentity};
use crate::diagnostic::Diagnostic;
use crate::input::sha256;
use crate::output::{self, FileBinding, OutputSlot};
use serde::Serialize;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};

pub(super) struct Failure {
    pub diagnostic: Diagnostic,
    pub exit: u8,
}
pub(super) type Result<T> = std::result::Result<T, Box<Failure>>;

pub(super) fn fail(stage: &str, code: &str, message: impl Into<String>, exit: u8) -> Box<Failure> {
    let mut diagnostic = Diagnostic::error(None, code, "", message);
    diagnostic.stage = stage.into();
    Box::new(Failure { diagnostic, exit })
}

pub(super) fn output_error(stage: &str, error: output::OutputError) -> Box<Failure> {
    let mut failure = fail(stage, &error.code, &error.message, error.exit_code());
    failure.diagnostic.evidence = serde_json::to_value(error).expect("output errors serialize");
    failure
}

pub(super) fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn valid_engine(engine: &EngineIdentity) -> bool {
    valid_hash(&engine.source_input_sha256)
        && valid_hash(&engine.binary.sha256)
        && engine.binary.bytes > 0
        && engine.input_contract_schema == crate::INPUT_CONTRACT_SCHEMA
        && engine.receipt_schema == crate::RECEIPT_SCHEMA
        && engine.viewer_revision == crate::VIEWER_REVISION
}

pub(super) fn header(
    schema: u32,
    tool: &str,
    version: &str,
    command: &str,
    expected: &str,
) -> bool {
    schema == 1
        && tool == "phoenix-diagram"
        && version == crate::TOOL_VERSION
        && command == expected
}

pub(super) fn reference(binding: &FileBinding) -> FileReference {
    FileReference {
        path: binding.path().to_owned(),
        identity: FileIdentity::of(binding.bytes()),
        original_binding: binding.identity().clone(),
    }
}

pub(super) fn capture_expected(
    expected: &FileReference,
    stage: &str,
    code: &str,
) -> Result<FileBinding> {
    let file = FileBinding::capture(&expected.path).map_err(|error| {
        let mut failure = output_error(stage, error);
        failure.diagnostic.code = code.into();
        failure
    })?;
    if reference(&file) != *expected {
        return Err(fail(stage, code, "The original file binding changed", 1));
    }
    Ok(file)
}

pub(super) fn verify(files: &[&FileBinding], stage: &str) -> Result<()> {
    for file in files {
        file.verify().map_err(|error| output_error(stage, error))?;
    }
    Ok(())
}

pub(super) fn settled(slot: &OutputSlot, stage: &str) -> Result<()> {
    let pending = slot.pending_publications().map_err(|error| {
        let mut failure = output_error(stage, error);
        failure.diagnostic.code = "provenance/unfinished-publication".into();
        failure
    })?;
    if !pending.is_empty() {
        let mut failure=fail(stage,"provenance/unfinished-publication","A publication for this output or receipt still has durable recovery or cleanup evidence",3);
        failure.diagnostic.evidence = json!({"target":slot.path(),"pending":pending});
        return Err(failure);
    }
    Ok(())
}

pub(super) fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| fail("delivery", "delivery/json", error.to_string(), 4))?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(super) fn write(
    slot: &OutputSlot,
    bytes: &[u8],
    protected: &[&FileBinding],
) -> Result<FileBinding> {
    let publication = output::publish_bound(slot, bytes, protected)
        .map_err(|error| output_error("delivery", error))?;
    if !publication.cleanup_complete {
        let mut failure = fail(
            "delivery",
            "output/cleanup-incomplete",
            "Published bytes require output cleanup or recovery",
            3,
        );
        failure.diagnostic.evidence =
            serde_json::to_value(&publication).expect("publication serializes");
        return Err(failure);
    }
    let file =
        FileBinding::capture(&publication.path).map_err(|error| output_error("delivery", error))?;
    if file.identity() != &publication.identity || file.bytes() != bytes {
        return Err(fail(
            "delivery",
            "output/publication-changed",
            "Published file changed before receipt binding",
            3,
        ));
    }
    Ok(file)
}

pub(super) fn unique_id() -> Result<String> {
    let mut random = [0_u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut random))
        .map_err(|error| fail("deliver", "delivery/entropy", error.to_string(), 4))?;
    random[6] = (random[6] & 0x0f) | 0x40;
    random[8] = (random[8] & 0x3f) | 0x80;
    let hex: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}

pub(super) fn valid_id(id: &str) -> bool {
    id.len() == 36
        && id.as_bytes()[14] == b'4'
        && matches!(id.as_bytes()[19], b'8' | b'9' | b'a' | b'b')
        && id.bytes().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            }
        })
}

pub(super) fn passed_check(check: &ArtifactCheck) -> bool {
    check.status == "pass"
        && !check.checks.is_empty()
        && check.checks.iter().all(|entry| entry.status == "pass")
        && check.composition["schemaVersion"] == 1
        && check.composition["status"] == "pass"
        && check.composition["summary"]["errors"] == 0
        && (check.quality != crate::input::Quality::Showcase
            || check.composition["summary"]["warnings"] == 0)
}

/// Finalize receipts and summaries repeat only the evidence status and digest.
pub(super) fn evidence_summary(validation: &Value) -> Value {
    match validation.get("status") {
        Some(status) => json!({"status":status,"evidenceSha256":validation["evidenceSha256"]}),
        None => json!({}),
    }
}

pub(super) fn config(input: &crate::input::DiagramInput) -> EffectiveConfig {
    EffectiveConfig {
        brand_resources: input
            .brands
            .iter()
            .map(|(path, brand)| (path.clone(), brand.resource.clone()))
            .collect(),
        diagram_type: input.diagram_type,
        input_contract: input.input_contract,
        quality: input.quality,
        viewer_revision: input.viewer_revision.clone(),
    }
}

pub(super) fn scope_names(value: &[crate::check::ArtifactCheckEntry]) -> bool {
    let mut names = std::collections::HashSet::new();
    !value.is_empty()
        && value.iter().all(|entry| {
            !entry.name.is_empty()
                && names.insert(&entry.name)
                && entry.status == "pass"
                && entry.scope.is_object()
        })
}

pub(super) fn artifact_quality(bytes: &[u8]) -> crate::input::Quality {
    use scraper::{Html, Selector};
    let document = Html::parse_document(std::str::from_utf8(bytes).unwrap_or_default());
    let selector = Selector::parse("svg[data-quality-profile]").expect("static selector");
    if document
        .select(&selector)
        .next()
        .and_then(|svg| svg.value().attr("data-quality-profile"))
        == Some("standard")
    {
        crate::input::Quality::Standard
    } else {
        crate::input::Quality::Showcase
    }
}

const SUFFIXES: [&str; 6] = [
    ".delivery.json",
    ".finalize-summary.json",
    ".browser-check.json",
    ".visual-check.json",
    ".visual-review.json",
    ".visual-check.2048x1320.light.png",
];
pub(super) fn bounded_stem(output: &Path) -> String {
    let basename = output.file_name().unwrap_or_default().to_string_lossy();
    let original = basename.strip_suffix(".html").unwrap_or(&basename);
    let reserved = original
        .rsplit_once(".~phoenix-diagram-")
        .filter(|(_, hash)| valid_hash(hash));
    let fits = |stem: &str| {
        SUFFIXES.iter().all(|suffix| {
            stem.len() + suffix.len() <= 255 && stem.encode_utf16().count() + suffix.len() <= 255
        })
    };
    if basename.ends_with(".html") && reserved.is_none() && fits(original) {
        return original.into();
    }
    let marker = format!(".~phoenix-diagram-{}", sha256(basename.as_bytes()));
    let mut prefix = reserved.map_or(original, |(prefix, _)| prefix).to_owned();
    while !fits(&format!("{prefix}{marker}")) {
        prefix.pop();
    }
    format!("{prefix}{marker}")
}

pub(super) fn paths(output: &Path, out_dir: Option<&Path>) -> SidecarPaths {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let root = out_dir.unwrap_or(parent);
    let stem = bounded_stem(output);
    SidecarPaths {
        delivery: parent.join(format!("{stem}.delivery.json")),
        summary: parent.join(format!("{stem}.finalize-summary.json")),
        browser: root.join(format!("{stem}.browser-check.json")),
        visual: root.join(format!("{stem}.visual-check.json")),
        review: root.join(format!("{stem}.visual-review.json")),
    }
}

pub(super) fn bound_json(file: &FileBinding) -> Value {
    json!({"path":file.path(),"identity":FileIdentity::of(file.bytes())})
}

pub(super) fn absolute(path: &Path) -> Result<PathBuf> {
    if path.to_str().is_none() {
        return Err(fail(
            "delivery",
            "output/non-utf8-path",
            "Paths must be UTF-8",
            2,
        ));
    }
    if path.is_absolute() {
        Ok(path.into())
    } else {
        Ok(std::env::current_dir()
            .map_err(|e| fail("delivery", "output/current-directory", e.to_string(), 4))?
            .join(path))
    }
}
