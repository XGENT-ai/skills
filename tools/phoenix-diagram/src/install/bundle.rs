// Apache-2.0 source patterns: Phoenix UI install/plan.rs and
// skills/phoenix-ui/scripts/phoenix-bootstrap.cjs. The diagram bundle names its
// files in manifest.json; ENGINE.json describes the native engine for each target.
use super::{invalid, FileDigest, InstallEngine, InstallRequest, InstallResult};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Version {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub npm_package_version: String,
    pub source_input_sha256: String,
    pub input_contract_schema: u32,
    pub receipt_schema: u32,
    pub viewer_revision: String,
    pub bundle_sha256: String,
    pub engines: BTreeMap<String, Engine>,
}
#[derive(Deserialize)]
pub(super) struct Engine {
    pub size: u64,
    pub sha256: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    tool: String,
    version: String,
    files: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Entry {
    path: String,
    size: u64,
    sha256: String,
    mode: u32,
}

pub(super) struct File {
    pub bytes: Vec<u8>,
    pub mode: u32,
}
pub(super) struct Bundle {
    pub version: Version,
    pub version_sha256: String,
    pub manifest_sha256: String,
    pub engine: InstallEngine,
    pub files: BTreeMap<String, File>,
}
pub(super) fn digest(bytes: &[u8]) -> FileDigest {
    FileDigest {
        sha256: format!("{:x}", Sha256::digest(bytes)),
        bytes: bytes.len() as u64,
    }
}
fn platform() -> InstallResult<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("darwin-arm64"),
        ("macos", "x86_64") => Ok("darwin-x64"),
        ("linux", "aarch64") => Ok("linux-arm64"),
        ("linux", "x86_64") => Ok("linux-x64"),
        _ => Err(invalid(
            "install/unsupported-platform",
            "This native installer supports the four declared Unix targets",
        )),
    }
}
/// Manifest paths are written below each provider's skill root, so they must
/// stay inside the known skill layout.
fn allowed(path: &str) -> bool {
    match path {
        "SKILL.md"
        | "scripts/phoenix-diagram"
        | "agents/openai.yaml"
        | "schemas/README.md"
        | "scripts/phoenix-launcher.cjs"
        | "scripts/phoenix-bootstrap.cjs" => true,
        _ => match path.split_once('/') {
            Some((directory, name)) => {
                let safe =
                    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\', ':']);
                safe && match directory {
                    "schemas" | "examples" => name.ends_with(".json"),
                    "references" => name.ends_with(".md"),
                    _ => false,
                }
            }
            None => false,
        },
    }
}
fn read(path: &Path) -> InstallResult<(Vec<u8>, u32)> {
    let missing = |error: std::io::Error| {
        invalid(
            "install/missing-file",
            format!("{}: {error}", path.display()),
        )
    };
    let bytes = std::fs::read(path).map_err(missing)?;
    let mode = std::fs::metadata(path)
        .map_err(missing)?
        .permissions()
        .mode()
        & 0o777;
    Ok((bytes, mode))
}
fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8], code: &str) -> InstallResult<T> {
    serde_json::from_slice(bytes).map_err(|e| invalid(code, e.to_string()))
}

impl Bundle {
    pub fn load(request: &InstallRequest) -> InstallResult<Self> {
        let (manifest_bytes, _) = read(&request.bundle_root.join("manifest.json"))?;
        let manifest: Manifest = parse(&manifest_bytes, "install/invalid-bundle-manifest")?;
        if manifest.schema_version != 1
            || manifest.tool != "phoenix-diagram"
            || manifest.version != crate::TOOL_VERSION
        {
            return Err(invalid(
                "install/invalid-bundle-manifest",
                "Unexpected bundle schema, tool, or version",
            ));
        }
        let (version_bytes, _) = read(&request.bundle_root.join("ENGINE.json"))?;
        let version: Version = parse(&version_bytes, "install/invalid-version-manifest")?;
        let manifest_sha256 = digest(&manifest_bytes).sha256;
        let target = platform()?;
        if version.schema_version != 1
            || version.tool != "phoenix-diagram"
            || version.tool_version != crate::TOOL_VERSION
            || version.bundle_sha256 != manifest_sha256
        {
            return Err(invalid(
                "install/version-identity-mismatch",
                "ENGINE.json does not match this engine version or bundle",
            ));
        }
        let engine_entry = version.engines.get(target).ok_or_else(|| {
            invalid(
                "install/version-identity-mismatch",
                format!("ENGINE.json has no engine for {target}"),
            )
        })?;
        let mut files = BTreeMap::new();
        for entry in manifest.files {
            if !allowed(&entry.path) || ![0o644, 0o755].contains(&entry.mode) {
                return Err(invalid("install/unsafe-bundle-file", entry.path));
            }
            let (bytes, mode) = read(&request.bundle_root.join(&entry.path))?;
            let actual = digest(&bytes);
            if actual.sha256 != entry.sha256 || actual.bytes != entry.size || mode != entry.mode {
                return Err(invalid("install/bundle-file-mismatch", entry.path));
            }
            files.insert(entry.path, File { bytes, mode });
        }
        for path in [
            "SKILL.md",
            "scripts/phoenix-diagram",
            "scripts/phoenix-launcher.cjs",
            "scripts/phoenix-bootstrap.cjs",
        ] {
            if !files.contains_key(path) {
                return Err(invalid("install/incomplete-bundle", path));
            }
        }
        let (engine_bytes, _) = read(&request.engine)?;
        let binary = digest(&engine_bytes);
        if binary.bytes != engine_entry.size || binary.sha256 != engine_entry.sha256 {
            return Err(invalid(
                "install/engine-mismatch",
                "Native engine size or digest differs from ENGINE.json",
            ));
        }
        probe(&request.engine)?;
        let engine = InstallEngine {
            source_input_sha256: version.source_input_sha256.clone(),
            input_contract_schema: version.input_contract_schema,
            receipt_schema: version.receipt_schema,
            viewer_revision: version.viewer_revision.clone(),
            binary,
        };
        files.insert(
            "ENGINE.json".into(),
            File {
                bytes: version_bytes.clone(),
                mode: 0o644,
            },
        );
        Ok(Self {
            version,
            version_sha256: digest(&version_bytes).sha256,
            manifest_sha256,
            engine,
            files,
        })
    }
}

fn probe(path: &Path) -> InstallResult<()> {
    let output = Command::new(path)
        .arg("engine-probe")
        .output()
        .map_err(|e| invalid("install/engine-probe", e.to_string()))?;
    let expected = format!(
        "phoenix-diagram-engine {} input={} receipt={} viewer={}\n",
        crate::TOOL_VERSION,
        crate::INPUT_CONTRACT_SCHEMA,
        crate::RECEIPT_SCHEMA,
        crate::VIEWER_REVISION
    );
    if output.status.success() && output.stdout == expected.as_bytes() {
        Ok(())
    } else {
        Err(invalid(
            "install/engine-probe",
            "Native engine probe identity differs from the package",
        ))
    }
}
