use crate::diagnostic::{Diagnostic, Diagnostics};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::str::FromStr;

mod semantics;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagramType {
    Architecture,
    Workflow,
    Sequence,
    Dataflow,
    Lifecycle,
    Erd,
    Class,
    Tree,
    Timeline,
    Waterfall,
}

impl DiagramType {
    pub const ALL: [Self; 10] = [
        Self::Architecture,
        Self::Workflow,
        Self::Sequence,
        Self::Dataflow,
        Self::Lifecycle,
        Self::Erd,
        Self::Class,
        Self::Tree,
        Self::Timeline,
        Self::Waterfall,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Architecture => "architecture",
            Self::Workflow => "workflow",
            Self::Sequence => "sequence",
            Self::Dataflow => "dataflow",
            Self::Lifecycle => "lifecycle",
            Self::Erd => "erd",
            Self::Class => "class",
            Self::Tree => "tree",
            Self::Timeline => "timeline",
            Self::Waterfall => "waterfall",
        }
    }

    /// Only the types frozen in Archify 2.17 have a legacy input contract.
    pub fn has_legacy_contract(self) -> bool {
        matches!(
            self,
            Self::Architecture | Self::Workflow | Self::Sequence | Self::Dataflow | Self::Lifecycle
        )
    }
}

impl FromStr for DiagramType {
    type Err = Diagnostics;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == value)
            .ok_or_else(|| {
                vec![Diagnostic::error(
                    None,
                    "input/unsupported-type",
                    "/diagram_type",
                    format!("Unsupported diagram type: {value}"),
                )]
            })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InputContract {
    #[serde(rename = "legacy-2.17")]
    Legacy217,
    #[serde(rename = "phoenix-v1")]
    PhoenixV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Standard,
    Showcase,
}

#[derive(Clone, Debug)]
pub struct CompileOptions {
    pub input_contract: Option<InputContract>,
    pub quality: Option<Quality>,
    pub viewer_revision: String,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            input_contract: None,
            quality: None,
            viewer_revision: crate::VIEWER_REVISION.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeferredField {
    pub path: String,
    pub reason: String,
    pub phase: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compatibility {
    pub status: String,
    pub core_compatibility: String,
    pub deferred_fields: Vec<DeferredField>,
}

#[derive(Clone, Debug)]
pub struct DiagramInput {
    pub diagram_type: DiagramType,
    pub input_contract: InputContract,
    pub quality: Quality,
    pub viewer_revision: String,
    pub source_bytes: Vec<u8>,
    pub source_sha256: String,
    pub config_sha256: String,
    pub document: Value,
    pub compatibility: Compatibility,
}

pub fn parse_input(
    bytes: &[u8],
    kind: DiagramType,
    options: &CompileOptions,
) -> Result<DiagramInput, Diagnostics> {
    let error = |code: &str, path: &str, message: String| {
        vec![Diagnostic::error(Some(kind.as_str()), code, path, message)]
    };
    let document: Value = serde_json::from_slice(bytes)
        .map_err(|cause| error("input/json", "", cause.to_string()))?;
    let marker = document
        .pointer("/meta/input_contract")
        .map(|value| serde_json::from_value::<InputContract>(value.clone()))
        .transpose()
        .map_err(|cause| {
            error(
                "input/unknown-contract",
                "/meta/input_contract",
                cause.to_string(),
            )
        })?;
    if let (Some(explicit), Some(persisted)) = (options.input_contract, marker) {
        if explicit != persisted {
            return Err(error(
                "input/contract-conflict",
                "/meta/input_contract",
                "Explicit and persisted input contracts differ".into(),
            ));
        }
    }
    let input_contract =
        options
            .input_contract
            .or(marker)
            .unwrap_or(if kind.has_legacy_contract() {
                InputContract::Legacy217
            } else {
                InputContract::PhoenixV1
            });
    if input_contract == InputContract::Legacy217 && !kind.has_legacy_contract() {
        return Err(error(
            "input/contract-unsupported",
            "/meta/input_contract",
            format!(
                "{} has no legacy-2.17 input contract; use phoenix-v1",
                kind.as_str()
            ),
        ));
    }
    let authored_quality = document
        .pointer("/meta/quality_profile")
        .map(|value| serde_json::from_value::<Quality>(value.clone()))
        .transpose()
        .map_err(|cause| {
            error(
                "input/unknown-quality",
                "/meta/quality_profile",
                cause.to_string(),
            )
        })?;
    if let (Some(explicit), Some(persisted)) = (options.quality, authored_quality) {
        if explicit != persisted {
            return Err(error(
                "input/quality-conflict",
                "/meta/quality_profile",
                "Explicit and persisted quality profiles differ".into(),
            ));
        }
    }
    let quality = options
        .quality
        .or(authored_quality)
        .unwrap_or(Quality::Showcase);
    if document
        .get("diagram_type")
        .is_some_and(|value| value.as_str() != Some(kind.as_str()))
    {
        return Err(error(
            "input/type-mismatch",
            "/diagram_type",
            format!("Expected diagram_type {}", kind.as_str()),
        ));
    }
    let validator = crate::schema::validator(kind, input_contract)
        .map_err(|cause| error("input/schema-resource", "", cause.into()))?;
    let mut schema_input = document.clone();
    // Phoenix's persisted selector is not a field in the frozen legacy schema.
    if input_contract == InputContract::Legacy217 {
        if let Some(meta) = schema_input.get_mut("meta").and_then(Value::as_object_mut) {
            meta.remove("input_contract");
        }
    }
    let errors: Diagnostics = validator
        .iter_errors(&schema_input)
        .map(|cause| {
            let mut diagnostic = Diagnostic::error(
                Some(kind.as_str()),
                "input/schema",
                &cause.instance_path().to_string(),
                cause.to_string(),
            );
            diagnostic.evidence = serde_json::json!({"schemaPath":cause.schema_path().to_string()});
            diagnostic
        })
        .collect();
    if !errors.is_empty() {
        return Err(errors);
    }
    if input_contract == InputContract::PhoenixV1 {
        if let Some(Err(reason)) = document["meta"]["output"].as_str().map(portable_output) {
            let mut diagnostic = Diagnostic::error(
                Some(kind.as_str()),
                "input/schema",
                "/meta/output",
                format!("meta.output must satisfy the portable output path contract ({reason})"),
            );
            diagnostic.evidence = serde_json::json!({"keyword":"portablePath","reason":reason});
            return Err(vec![diagnostic]);
        }
    }
    semantics::validate(&document, kind, input_contract)?;
    let compatibility = compatibility(&document, input_contract);
    let mut input = DiagramInput {
        diagram_type: kind,
        input_contract,
        quality,
        viewer_revision: options.viewer_revision.clone(),
        source_bytes: bytes.to_vec(),
        source_sha256: sha256(bytes),
        config_sha256: String::new(),
        document,
        compatibility,
    };
    input.bind_evidence(None);
    Ok(input)
}

impl DiagramInput {
    /// The configuration digest also binds the source evidence observed at
    /// delivery (null when nothing was cited), never the repository path.
    pub fn bind_evidence(&mut self, evidence_sha256: Option<&str>) {
        let config = serde_json::json!({
            "diagramType":self.diagram_type, "inputContract":self.input_contract, "quality":self.quality,
            "viewerRevision":self.viewer_revision, "evidenceSha256":evidence_sha256,
        });
        self.config_sha256 =
            sha256(&serde_json::to_vec(&config).expect("serializable compile options"));
    }
}

// The frozen 3.1.0 validators apply this check to meta.output of every type after
// the schema (renderers/shared/portable-path.mjs, profile "output"); the schemas of
// erd, class, tree, timeline and waterfall leave meta.output a plain string.
fn portable_output(value: &str) -> Result<(), &'static str> {
    let bytes = value.as_bytes();
    let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if value.is_empty() {
        return Err("empty");
    }
    if value.starts_with(['/', '\\']) || (drive && matches!(bytes.get(2), Some(b'/' | b'\\'))) {
        return Err("absolute");
    }
    if drive {
        return Err("drive-relative");
    }
    let scheme = value.find(':').is_some_and(|end| {
        end > 0
            && bytes[0].is_ascii_alphabetic()
            && bytes[1..end]
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-'))
    });
    if scheme {
        return Err("uri");
    }
    if value.contains('\\') {
        return Err("backslash");
    }
    for segment in value.split('/') {
        let lower = segment.to_ascii_lowercase();
        let device = lower.split('.').next().unwrap_or_default();
        let short_name = segment.match_indices('~').any(|(at, _)| {
            let digits: String = segment[at + 1..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            let rest = &segment[at + 1 + digits.len()..];
            !digits.is_empty()
                && !digits.starts_with('0')
                && (rest.is_empty() || rest.starts_with('.'))
        });
        let reason = if segment.is_empty() {
            "empty-segment"
        } else if segment == "." || segment == ".." {
            "dot-segment"
        } else if segment.chars().any(char::is_control) {
            "control"
        } else if segment.contains(':') {
            "windows-ads"
        } else if segment.contains(['<', '>', '"', '|', '?', '*']) {
            "windows-invalid-character"
        } else if segment.ends_with(['.', ' ']) {
            "windows-trailing-dot-space"
        } else if matches!(device, "con" | "prn" | "aux" | "nul" | "conin$" | "conout$")
            || ((device.starts_with("com") || device.starts_with("lpt"))
                && device.chars().count() == 4
                && device
                    .chars()
                    .nth(3)
                    .is_some_and(|c| matches!(c, '1'..='9' | '¹' | '²' | '³')))
        {
            "windows-reserved-name"
        } else if short_name {
            "windows-short-name"
        } else if segment.len() > 255 || segment.encode_utf16().count() > 255 {
            "component-too-long"
        } else {
            continue;
        };
        return Err(reason);
    }
    Ok(())
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn compatibility(document: &Value, contract: InputContract) -> Compatibility {
    fn walk(value: &Value, path: &str, contract: InputContract, fields: &mut Vec<DeferredField>) {
        match value {
            Value::Object(object) => {
                for (key, value) in object {
                    let pointer = format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"));
                    let phase = match (path, key.as_str()) {
                        (_, "brand") => Some(("P3", "Brand resources are outside this delivery phase")),
                        ("/meta", "views") => Some(("P3", "Guided views are retained for the later reader acceptance scope")),
                        ("/meta", "animation") if contract == InputContract::Legacy217 => Some(("P3", "Legacy playback preferences do not override the reader's initial Still state")),
                        _ => None,
                    };
                    if let Some((phase, reason)) = phase {
                        fields.push(DeferredField {
                            path: pointer,
                            phase: phase.into(),
                            reason: reason.into(),
                        });
                    } else {
                        walk(value, &pointer, contract, fields);
                    }
                }
            }
            Value::Array(array) => {
                for (index, value) in array.iter().enumerate() {
                    walk(value, &format!("{path}/{index}"), contract, fields);
                }
            }
            _ => {}
        }
    }
    let mut deferred_fields = Vec::new();
    walk(document, "", contract, &mut deferred_fields);
    Compatibility {
        status: if deferred_fields.is_empty() {
            "complete"
        } else {
            "partial"
        }
        .into(),
        core_compatibility: "pass".into(),
        deferred_fields,
    }
}
