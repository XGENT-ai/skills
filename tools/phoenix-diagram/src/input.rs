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
}

impl DiagramType {
    pub const ALL: [Self; 5] = [
        Self::Architecture,
        Self::Workflow,
        Self::Sequence,
        Self::Dataflow,
        Self::Lifecycle,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Architecture => "architecture",
            Self::Workflow => "workflow",
            Self::Sequence => "sequence",
            Self::Dataflow => "dataflow",
            Self::Lifecycle => "lifecycle",
        }
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
    let input_contract = options
        .input_contract
        .or(marker)
        .unwrap_or(InputContract::Legacy217);
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
    semantics::validate(&document, kind, input_contract)?;
    let compatibility = compatibility(&document, input_contract);
    let config = serde_json::json!({
        "diagramType":kind, "inputContract":input_contract, "quality":quality,
        "viewerRevision":options.viewer_revision,
    });
    Ok(DiagramInput {
        diagram_type: kind,
        input_contract,
        quality,
        viewer_revision: options.viewer_revision.clone(),
        source_bytes: bytes.to_vec(),
        source_sha256: sha256(bytes),
        config_sha256: sha256(&serde_json::to_vec(&config).expect("serializable compile options")),
        document,
        compatibility,
    })
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
                        (_, "sources") | ("/meta", "repository") => Some(("P2", "Repository evidence is retained without fetching or verification")),
                        ("/meta", "views") => Some(("P2", "Guided views are retained for the later reader acceptance scope")),
                        ("/meta", "animation") if contract == InputContract::Legacy217 => Some(("P2", "Legacy playback preferences do not override the reader's initial Still state")),
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
