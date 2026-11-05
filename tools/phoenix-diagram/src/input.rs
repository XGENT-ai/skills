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
    Statechart,
}

impl DiagramType {
    pub const ALL: [Self; 11] = [
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
        Self::Statechart,
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
            Self::Statechart => "statechart",
        }
    }

    /// The first five types, which share lane/phase/group identity rules and
    /// the arrow-route check scope that the later types do not.
    pub fn is_original_five(self) -> bool {
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
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Standard,
    Showcase,
}

#[derive(Clone, Debug)]
pub struct CompileOptions {
    pub quality: Option<Quality>,
    pub viewer_revision: String,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            quality: None,
            viewer_revision: crate::VIEWER_REVISION.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DiagramInput {
    pub diagram_type: DiagramType,
    pub quality: Quality,
    pub viewer_revision: String,
    pub source_bytes: Vec<u8>,
    pub source_sha256: String,
    pub config_sha256: String,
    pub document: Value,
    pub brands: crate::brands::ResolvedBrands,
    evidence_sha256: Option<String>,
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
    let validator = crate::schema::validator(kind)
        .map_err(|cause| error("input/schema-resource", "", cause.into()))?;
    let errors: Diagnostics = validator
        .iter_errors(&document)
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
    semantics::validate(&document, kind)?;
    let mut input = DiagramInput {
        diagram_type: kind,
        quality,
        viewer_revision: options.viewer_revision.clone(),
        source_bytes: bytes.to_vec(),
        source_sha256: sha256(bytes),
        config_sha256: String::new(),
        document,
        brands: Default::default(),
        evidence_sha256: None,
    };
    input.bind_evidence(None);
    Ok(input)
}

impl DiagramInput {
    /// The configuration digest also binds the source evidence observed at
    /// delivery (null when nothing was cited), never the repository path.
    pub fn bind_evidence(&mut self, evidence_sha256: Option<&str>) {
        self.evidence_sha256 = evidence_sha256.map(str::to_owned);
        self.update_config();
    }
    pub fn bind_brands(&mut self, brands: crate::brands::ResolvedBrands) {
        self.brands = brands;
        self.update_config();
    }
    fn update_config(&mut self) {
        let mut config = serde_json::json!({
            "diagramType":self.diagram_type, "quality":self.quality,
            "viewerRevision":self.viewer_revision, "evidenceSha256":self.evidence_sha256,
        });
        if !self.brands.is_empty() {
            config["brandResources"] = serde_json::json!(self
                .brands
                .iter()
                .map(|(path, brand)| (path, &brand.resource))
                .collect::<std::collections::BTreeMap<_, _>>());
        }
        self.config_sha256 =
            sha256(&serde_json::to_vec(&config).expect("serializable compile options"));
    }
}

// Applied to meta.output of every type after the schema; the schemas of erd,
// class, tree, timeline and waterfall leave meta.output a plain string.
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
