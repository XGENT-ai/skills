//! Fixed brand resources. Compilation never invokes the network capture path.
mod cache;
pub mod capture;
mod network;

use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::input::{sha256, DiagramInput, DiagramType};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::OnceLock,
};
use url::Url;

#[derive(Debug)]
pub struct BrandError {
    pub code: &'static str,
    pub message: String,
}
impl BrandError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    pub fn exit_code(&self) -> u8 {
        match self.code {
            "brand/capture-unavailable" | "brand/cache-missing" => 5,
            "brand/io" => 4,
            "brand/conflict" => 3,
            _ => 1,
        }
    }
    pub fn diagnostic(&self, path: &str) -> Diagnostic {
        let mut d = Diagnostic::error(None, self.code, path, &self.message);
        d.stage = "brands".into();
        d
    }
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrandMark {
    pub id: String,
    pub title: String,
    pub category: String,
    pub aliases: Vec<String>,
    pub domains: Vec<String>,
    pub view_box: f64,
    pub hex: String,
    pub path: String,
    pub provenance: Value,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    catalog_version: String,
    marks: Vec<BrandMark>,
}
fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/brand-marks.json"))
            .expect("validated fixed brand catalog")
    })
}
fn forms(value: &str) -> Vec<String> {
    let raw = value.trim().to_lowercase();
    vec![
        raw.clone(),
        raw.chars()
            .map(|c| {
                if c.is_whitespace() || c == '_' {
                    '-'
                } else {
                    c
                }
            })
            .collect(),
        raw.chars()
            .filter(|c| !c.is_whitespace() && !matches!(c, '_' | '.' | '-'))
            .collect(),
    ]
}
pub fn find(value: &str) -> Option<&'static BrandMark> {
    if let Ok(url) = Url::parse(value) {
        let url = network::normalize_url(url.as_str()).ok()?;
        let host = url.host_str()?.trim_end_matches('.');
        return catalog()
            .marks
            .iter()
            .filter_map(|mark| {
                mark.domains
                    .iter()
                    .filter(|domain| {
                        host == domain.as_str() || host.ends_with(&format!(".{domain}"))
                    })
                    .map(|domain| (domain.len(), mark))
                    .max_by_key(|(len, _)| *len)
            })
            .max_by_key(|(len, _)| *len)
            .map(|(_, mark)| mark);
    }
    let keys = forms(value);
    catalog().marks.iter().find(|mark| {
        std::iter::once(&mark.id)
            .chain(std::iter::once(&mark.title))
            .chain(&mark.aliases)
            .any(|v| forms(v).iter().any(|key| keys.contains(key)))
    })
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogReceipt {
    pub schema_version: u32,
    pub tool: &'static str,
    pub tool_version: &'static str,
    pub command: &'static str,
    pub status: &'static str,
    pub catalog_version: String,
    pub query: String,
    pub matches: Vec<Value>,
    pub diagnostics: Diagnostics,
}
pub fn list(query: &str) -> CatalogReceipt {
    let needle = query.trim().to_lowercase();
    let matches = catalog().marks.iter().filter(|mark| needle.is_empty() || std::iter::once(&mark.id).chain(std::iter::once(&mark.title)).chain(std::iter::once(&mark.category)).chain(&mark.aliases).chain(&mark.domains).any(|s| s.to_lowercase().contains(&needle))).map(|mark| json!({"id":mark.id,"title":mark.title,"category":mark.category,"aliases":mark.aliases,"domains":mark.domains,"provenance":mark.provenance})).collect();
    CatalogReceipt {
        schema_version: 1,
        tool: "phoenix-diagram",
        tool_version: crate::TOOL_VERSION,
        command: "brands",
        status: "pass",
        catalog_version: catalog().catalog_version.clone(),
        query: query.into(),
        matches,
        diagnostics: vec![],
    }
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceIdentity {
    pub id: String,
    pub sha256: String,
    pub bytes: usize,
    pub media_type: String,
    pub url_sha256: Option<String>,
    pub source_url: String,
    pub resolved_url: String,
    pub catalog_version: Option<String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedBrand {
    pub resource: ResourceIdentity,
    pub title: String,
    pub mark: Option<BrandMark>,
    pub data_base64: Option<String>,
}
pub type ResolvedBrands = BTreeMap<String, ResolvedBrand>;
fn builtin(mark: &BrandMark) -> ResolvedBrand {
    let bytes = serde_json::to_vec(mark).expect("fixed mark serializes");
    ResolvedBrand {
        resource: ResourceIdentity {
            id: mark.id.clone(),
            sha256: sha256(&bytes),
            bytes: bytes.len(),
            media_type: "image/svg+xml".into(),
            url_sha256: None,
            source_url: mark.provenance["source"]
                .as_str()
                .unwrap_or_default()
                .into(),
            resolved_url: mark.provenance["source"]
                .as_str()
                .unwrap_or_default()
                .into(),
            catalog_version: Some(catalog().catalog_version.clone()),
        },
        title: mark.title.clone(),
        mark: Some(mark.clone()),
        data_base64: None,
    }
}
pub fn collection(kind: DiagramType) -> Option<&'static str> {
    match kind {
        DiagramType::Architecture => Some("components"),
        DiagramType::Workflow | DiagramType::Dataflow => Some("nodes"),
        DiagramType::Sequence => Some("participants"),
        DiagramType::Lifecycle => Some("states"),
        _ => None,
    }
}
pub fn default_cache() -> PathBuf {
    PathBuf::from(".phoenix-diagram/brands/v1")
}
pub fn prepare(input: &DiagramInput, cache: &Path) -> Result<ResolvedBrands, Diagnostics> {
    resolve(input, |value| cache::load(value, cache))
}
fn resolve(
    input: &DiagramInput,
    mut remote: impl FnMut(&Value) -> Result<ResolvedBrand, BrandError>,
) -> Result<ResolvedBrands, Diagnostics> {
    let mut brands = BTreeMap::new();
    let mut errors = vec![];
    if let Some(collection) = collection(input.diagram_type) {
        for (i, node) in input.document[collection]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            let Some(brand) = node.get("brand") else {
                continue;
            };
            let path = format!("/{collection}/{i}/brand");
            let resolved = if let Some(text) = brand.as_str() {
                if let Some(mark) = find(text) {
                    Ok(builtin(mark))
                } else {
                    Err(BrandError::new(if Url::parse(text).is_ok() {"brand/unpinned-url"} else {"brand/unknown"}, "Choose a fixed catalog ID with `brands`, or explicitly capture the official URL and use its pinned digest"))
                }
            } else {
                remote(brand)
            };
            match resolved {
                Ok(brand) => {
                    brands.insert(path, brand);
                }
                Err(e) => errors.push(e.diagnostic(&path)),
            }
        }
    }
    if errors.is_empty() {
        Ok(brands)
    } else {
        Err(errors)
    }
}
/// Reconstruct resource bindings from immutable artifact data, without a cache or network.
pub fn bound(
    input: &DiagramInput,
    supplied: &ResolvedBrands,
) -> Result<ResolvedBrands, Diagnostics> {
    let result = resolve(input, |value| {
        let url = network::normalize_url(value["url"].as_str().unwrap_or_default())?;
        let url_sha = sha256(url.as_str().as_bytes());
        let item = supplied
            .values()
            .find(|b| {
                b.resource.url_sha256.as_deref() == Some(&url_sha)
                    && value["sha256"].as_str() == Some(&b.resource.sha256)
            })
            .ok_or_else(|| {
                BrandError::new(
                    "brand/digest-mismatch",
                    "Artifact resource binding is missing",
                )
            })?;
        cache::verify_resolved(item, &url, value["sha256"].as_str().unwrap_or_default())?;
        Ok(item.clone())
    })?;
    if &result != supplied {
        return Err(vec![BrandError::new(
            "brand/digest-mismatch",
            "Artifact brand resources differ from the input",
        )
        .diagnostic("")]);
    }
    Ok(result)
}
/// Public facts retain identity while dropping request query and fragment values.
pub fn sanitize_document(document: &mut Value, kind: DiagramType) {
    if let Some(key) = collection(kind) {
        if let Some(nodes) = document[key].as_array_mut() {
            for node in nodes {
                if let Some(brand) = node.get_mut("brand") {
                    let url_value = if brand.is_object() {
                        brand.get_mut("url")
                    } else {
                        Some(brand)
                    };
                    if let Some(value) = url_value {
                        if let Some(url) = value.as_str().and_then(|s| Url::parse(s).ok()) {
                            *value = json!(network::public_url(&url));
                        }
                    }
                }
            }
        }
    }
}
pub struct CaptureRequest<'a> {
    pub url: &'a str,
    pub brand_cache: &'a Path,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureReceipt {
    pub schema_version: u32,
    pub tool: &'static str,
    pub tool_version: &'static str,
    pub command: &'static str,
    pub status: &'static str,
    pub brand: Value,
    pub resource: Option<ResourceIdentity>,
    pub cache_path: Option<PathBuf>,
    pub diagnostics: Diagnostics,
    #[serde(skip)]
    pub exit: u8,
}
pub fn capture(request: &CaptureRequest<'_>) -> CaptureReceipt {
    let mut result = CaptureReceipt {
        schema_version: 1,
        tool: "phoenix-diagram",
        tool_version: crate::TOOL_VERSION,
        command: "brands capture",
        status: "failed",
        brand: Value::Null,
        resource: None,
        cache_path: None,
        diagnostics: vec![],
        exit: 1,
    };
    let attempt = (|| {
        let url = network::normalize_url(request.url)?;
        if let Some(mark) = find(url.as_str()) {
            let b = builtin(mark);
            result.brand = json!(mark.id);
            result.resource = Some(b.resource);
            return Ok(());
        }
        let captured = capture::fetch(url.as_str())?;
        let (brand, path) = cache::store(&url, &captured, request.brand_cache)?;
        result.brand = json!({"url":url.as_str(),"sha256":brand.resource.sha256});
        result.resource = Some(brand.resource);
        result.cache_path = Some(path);
        Ok::<_, BrandError>(())
    })();
    match attempt {
        Ok(()) => {
            result.status = "pass";
            result.exit = 0;
        }
        Err(e) => {
            result.exit = e.exit_code();
            result.diagnostics.push(e.diagnostic(""));
        }
    }
    result
}

pub(crate) fn resource_bindings(
    brands: &ResolvedBrands,
    cache: &Path,
) -> Result<Vec<crate::output::FileBinding>, BrandError> {
    cache::bindings(brands, cache)
}
