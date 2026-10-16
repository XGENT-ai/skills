use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subject {
    pub diagram_type: Option<String>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub stage: String,
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub subject: Subject,
    pub evidence: Value,
    pub supported_fixes: Vec<String>,
}

pub type Diagnostics = Vec<Diagnostic>;

impl Diagnostic {
    pub fn error(kind: Option<&str>, code: &str, path: &str, message: impl Into<String>) -> Self {
        Self {
            stage: "input".into(),
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            subject: Subject {
                diagram_type: kind.map(str::to_owned),
                path: path.into(),
                identity: None,
            },
            evidence: Value::Null,
            supported_fixes: Vec::new(),
        }
    }
}
