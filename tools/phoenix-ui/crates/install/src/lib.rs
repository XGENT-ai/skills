//! Project installation: copy the bundle's provider files into the project,
//! write ENGINE.json beside each installed skill and splice the Phoenix hook
//! handlers into shared hook configuration, keeping every other handler.
//! Rerunning the install is the repair path; nothing is journaled or backed up.
pub mod cli;

use impeccable_context::hook_markers::parse_manifest_jsonc;
use impeccable_skills::hook_manifest::{merge_hook_manifests, provider_hook_artifacts};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const MAIN_SKILL: &str = "skills/phoenix-ui/SKILL.md";
const SKILL_DIR: &str = "skills/phoenix-ui";
const RUNTIME_MANIFEST: &str = "skills/phoenix-ui/scripts/ENGINE.json";

#[derive(Debug)]
pub enum Error {
    /// Argument or path input; exit 2.
    Usage(String),
    /// Bundle, project file or environment failure; exit 4.
    Failed(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Usage(message) | Error::Failed(message) => f.write_str(message),
        }
    }
}

fn failed(path: &Path, error: impl std::fmt::Display) -> Error {
    Error::Failed(format!("{}: {error}", path.display()))
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Planned,
    Installed,
    Unchanged,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub mode: &'static str,
    pub status: Status,
    pub project: String,
    pub package_version: String,
    pub providers: Vec<String>,
    pub writes: Vec<String>,
    pub shared_json: Vec<String>,
    /// Files the package no longer ships and symlinked skill folders replaced by a copy.
    pub removals: Vec<String>,
}

#[derive(Deserialize)]
struct Manifest {
    providers: BTreeMap<String, Provider>,
}

#[derive(Deserialize)]
struct Provider {
    files: BTreeMap<String, String>,
    #[serde(default)]
    exec: Vec<String>,
}

/// A bundle directory with `VERSION.json` beside it, as shipped in the package.
struct Bundle {
    root: PathBuf,
    version: Vec<u8>,
    package_version: String,
    providers: BTreeMap<String, Provider>,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

impl Bundle {
    fn load(root: &Path) -> Result<Self, Error> {
        let read = |path: PathBuf| fs::read(&path).map_err(|e| failed(&path, e));
        // Lexical parent: a symlinked bundle still pairs with its own VERSION.json.
        let version_path = root.parent().unwrap_or(root).join("VERSION.json");
        let version = read(version_path.clone())?;
        let manifest = read(root.join("manifest.json"))?;
        let meta: Value = serde_json::from_slice(&version).map_err(|e| failed(&version_path, e))?;
        if meta["bundleSha256"].as_str() != Some(sha256(&manifest).as_str()) {
            return Err(Error::Failed(
                "bundle manifest does not match VERSION.json".into(),
            ));
        }
        let parsed: Manifest = serde_json::from_slice(&manifest)
            .map_err(|e| failed(&root.join("manifest.json"), e))?;
        Ok(Self {
            root: root.to_path_buf(),
            package_version: meta["npmPackageVersion"].as_str().unwrap_or("").into(),
            version,
            providers: parsed.providers,
        })
    }

    fn blob(&self, sha: &str) -> Result<Vec<u8>, Error> {
        let path = self.root.join("blobs").join(sha);
        let bytes = fs::read(&path).map_err(|e| failed(&path, e))?;
        if sha256(&bytes) != sha {
            return Err(failed(&path, "bundle file does not match its SHA-256"));
        }
        Ok(bytes)
    }

    /// Explicit names (`claude`, `codex`) or directories (`.claude`). Without a
    /// list, the provider folders already in the project, else `.claude`.
    /// Codex skills live in `.agents` and its hooks in `.codex`, so either
    /// selects both.
    fn select(&self, requested: Option<&str>, project: &Path) -> Result<Vec<String>, Error> {
        let known = |dir: &str| self.providers.contains_key(dir);
        let mut out: Vec<String> = match requested {
            Some(list) => list
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(|token| {
                    let dir = match token {
                        "codex" => ".agents".to_string(),
                        t if t.starts_with('.') => t.to_string(),
                        t => format!(".{t}"),
                    };
                    if known(&dir) {
                        Ok(dir)
                    } else {
                        let names: Vec<&str> = self.providers.keys().map(String::as_str).collect();
                        Err(Error::Usage(format!(
                            "unknown provider: {token}; choose from {}",
                            names.join(", ")
                        )))
                    }
                })
                .collect::<Result<_, _>>()?,
            None => self
                .providers
                .keys()
                .filter(|p| project.join(p).is_dir())
                .cloned()
                .collect(),
        };
        if out.is_empty() {
            if requested.is_some() {
                return Err(Error::Usage(
                    "--providers needs at least one provider".into(),
                ));
            }
            out.push(".claude".into());
        }
        if out.iter().any(|p| p == ".agents" || p == ".codex") {
            out.extend([".agents".to_string(), ".codex".to_string()]);
        }
        out.sort();
        out.dedup();
        Ok(out)
    }
}

/// True for a bundle file that is a provider's hook configuration, which the
/// project may share with other tools.
fn is_hook_config(provider: &str, path: &str) -> bool {
    let owner = if provider == ".codex" {
        ".agents"
    } else {
        provider
    };
    provider_hook_artifacts(owner)
        .iter()
        .any(|spec| spec.source_provider == provider && spec.rel == path)
}

/// The Phoenix handlers spliced into the project's configuration at `path`.
fn merged_hooks(path: &Path, fresh: &[u8]) -> Result<Vec<u8>, Error> {
    let fresh: Value = serde_json::from_slice(fresh).map_err(|e| failed(path, e))?;
    let existing = match fs::read_to_string(path) {
        Ok(text) => match parse_manifest_jsonc(&text) {
            Some((value @ Value::Object(_), _)) => value,
            _ => {
                return Err(failed(
                    path,
                    "not a JSON object; fix it and rerun the Phoenix UI install",
                ))
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Value::Object(Default::default()),
        Err(e) => return Err(failed(path, e)),
    };
    let merged = merge_hook_manifests(&existing, &fresh);
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&merged).unwrap_or_default()
    )
    .into_bytes())
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_symlink())
}

#[cfg(unix)]
fn file_mode(path: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .ok()
        .map(|m| m.permissions().mode() & 0o777)
}

#[cfg(not(unix))]
fn file_mode(_: &Path) -> Option<u32> {
    None
}

struct Write {
    relative: String,
    bytes: Vec<u8>,
    /// None keeps the permissions of shared configuration.
    mode: Option<u32>,
}

/// Plan, and unless `dry_run`, apply the project installation.
pub fn install(
    project: &Path,
    bundle_root: &Path,
    providers: Option<&str>,
    dry_run: bool,
) -> Result<Report, Error> {
    let bundle = Bundle::load(bundle_root)?;
    let providers = bundle.select(providers, project)?;
    let mut report = Report {
        mode: if dry_run { "dry-run" } else { "apply" },
        status: Status::Planned,
        project: project.display().to_string(),
        package_version: bundle.package_version.clone(),
        providers: providers.clone(),
        writes: Vec::new(),
        shared_json: Vec::new(),
        removals: Vec::new(),
    };
    // A symlinked skill folder (a source checkout or skills manager) is
    // replaced by a copy; writing through it would change the link target.
    let linked: Vec<String> = providers
        .iter()
        .map(|p| format!("{p}/{SKILL_DIR}"))
        .filter(|dir| is_symlink(&project.join(dir)))
        .collect();
    let mut writes = Vec::new();
    let mut shipped = std::collections::BTreeSet::new();
    for provider in &providers {
        let entry = &bundle.providers[provider];
        let mut files = Vec::new();
        for (relative, sha) in &entry.files {
            let path = project.join(provider).join(relative);
            let bytes = bundle.blob(sha)?;
            files.push(if is_hook_config(provider, relative) {
                (relative.as_str(), merged_hooks(&path, &bytes)?, None)
            } else {
                let exec = entry.exec.contains(relative);
                (
                    relative.as_str(),
                    bytes,
                    Some(if exec { 0o755 } else { 0o644 }),
                )
            });
        }
        if entry.files.contains_key(MAIN_SKILL) {
            files.push((RUNTIME_MANIFEST, bundle.version.clone(), Some(0o644)));
        }
        for (relative, bytes, mode) in files {
            let relative = format!("{provider}/{relative}");
            shipped.insert(relative.clone());
            let path = project.join(&relative);
            let replaced = linked
                .iter()
                .any(|dir| relative.starts_with(&format!("{dir}/")));
            let current = !replaced
                && fs::read(&path).is_ok_and(|b| b == bytes)
                && (cfg!(not(unix)) || mode.is_none() || file_mode(&path) == mode);
            if current {
                continue;
            }
            if mode.is_some() {
                report.writes.push(relative.clone());
            } else {
                report.shared_json.push(relative.clone());
            }
            writes.push(Write {
                relative,
                bytes,
                mode,
            });
        }
    }
    // The skill folder belongs to Phoenix: files a newer package no longer
    // ships are removed instead of lingering in the agent's view.
    let stale: Vec<String> = providers
        .iter()
        .map(|p| format!("{p}/{SKILL_DIR}"))
        .filter(|dir| !linked.contains(dir))
        .flat_map(|dir| {
            files_under(&project.join(&dir))
                .into_iter()
                .map(move |file| format!("{dir}/{file}"))
        })
        .filter(|relative| !shipped.contains(relative))
        .collect();
    report.removals.extend(stale.iter().cloned());
    report.removals.extend(linked.iter().cloned());
    if writes.is_empty() && report.removals.is_empty() {
        report.status = Status::Unchanged;
        return Ok(report);
    }
    if dry_run {
        return Ok(report);
    }
    for dir in &linked {
        let path = project.join(dir);
        fs::remove_file(&path)
            .or_else(|_| fs::remove_dir(&path))
            .map_err(|e| failed(&path, e))?;
    }
    for relative in &stale {
        let path = project.join(relative);
        fs::remove_file(&path).map_err(|e| failed(&path, e))?;
    }
    for write in writes {
        let path = project.join(&write.relative);
        fs::create_dir_all(path.parent().expect("file parent")).map_err(|e| failed(&path, e))?;
        fs::write(&path, &write.bytes).map_err(|e| failed(&path, e))?;
        #[cfg(unix)]
        if let Some(mode) = write.mode {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(mode))
                .map_err(|e| failed(&path, e))?;
        }
    }
    report.status = Status::Installed;
    Ok(report)
}

/// Relative paths of the files below `root`; empty when it does not exist.
fn files_under(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                stack.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                files.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    files
}
