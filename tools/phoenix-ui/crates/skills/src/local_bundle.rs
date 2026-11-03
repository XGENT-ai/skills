//! The explicit local bundle: `manifest.json` plus content-addressed blobs,
//! with `VERSION.json` beside the bundle directory. No network fallback.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::providers::Sys;
use crate::util::{self, jsp};

pub const RUNTIME_MANIFEST_PATH: &str = "skills/phoenix-ui/scripts/ENGINE.json";

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

pub struct LocalBundle {
    pub source: PathBuf,
    pub digest: String,
    manifest: Manifest,
    version_bytes: Vec<u8>,
}

pub fn source(sys: &Sys) -> Option<&str> {
    sys.env
        .get("PHOENIX_UI_BUNDLE_PATH")
        .filter(|s| !s.is_empty())
        .map(String::as_str)
}

fn digest(bytes: &[u8]) -> String {
    crate::bundle::hex(&Sha256::digest(bytes))
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

impl LocalBundle {
    pub fn load(sys: &Sys) -> Result<Self, String> {
        let input = source(sys).ok_or("Local bundle unavailable. Pass --bundle-root DIR or PHOENIX_UI_BUNDLE_PATH; Phoenix does not download bundles or engines.")?;
        let source = PathBuf::from(jsp::resolve(&sys.cwd, &[input]));
        let version_bytes = read(&source.parent().unwrap_or(&source).join("VERSION.json"))?;
        let version: serde_json::Value = serde_json::from_slice(&version_bytes)
            .map_err(|e| format!("Invalid VERSION.json: {e}"))?;
        let bytes = read(&source.join("manifest.json"))?;
        let manifest_digest = digest(&bytes);
        if version["bundleSha256"].as_str() != Some(&manifest_digest) {
            return Err("Local bundle manifest SHA-256 does not match VERSION.json".into());
        }
        let manifest: Manifest =
            serde_json::from_slice(&bytes).map_err(|e| format!("Invalid bundle manifest: {e}"))?;
        Ok(Self {
            source,
            digest: manifest_digest,
            manifest,
            version_bytes,
        })
    }

    /// Write every provider tree, plus ENGINE.json beside each main skill,
    /// into `destination`.
    fn expand(&self, destination: &Path) -> Result<(), String> {
        for (provider, entry) in &self.manifest.providers {
            let mut files: Vec<(&str, Vec<u8>)> = Vec::new();
            for (relative, sha) in &entry.files {
                let bytes = read(&self.source.join("blobs").join(sha))?;
                if digest(&bytes) != *sha {
                    return Err(format!(
                        "Local bundle blob does not match its SHA-256: {sha}"
                    ));
                }
                files.push((relative, bytes));
            }
            if entry.files.contains_key("skills/phoenix-ui/SKILL.md") {
                files.push((RUNTIME_MANIFEST_PATH, self.version_bytes.clone()));
            }
            for (relative, bytes) in files {
                let file = destination.join(provider).join(relative);
                std::fs::create_dir_all(file.parent().unwrap()).map_err(|e| e.to_string())?;
                std::fs::write(&file, bytes).map_err(|e| e.to_string())?;
                if entry.exec.iter().any(|p| p == relative) {
                    util::set_executable(&file.to_string_lossy())?;
                }
            }
        }
        Ok(())
    }

    /// A temporary provider tree for the copy-based install, update and check
    /// paths. The caller removes it.
    pub fn stage(&self, sys: &Sys) -> Result<String, String> {
        let dir = util::mkdtemp(&jsp::join(&[
            &util::tmpdir(&sys.env),
            "phoenix-local-bundle-",
        ]))?;
        if let Err(error) = self.expand(Path::new(&dir)) {
            util::rm_rf(&dir);
            return Err(error);
        }
        Ok(dir)
    }

    /// Links need a persistent tree: `.phoenix-ui/linked-bundles/<digest>`,
    /// reused while the bundle is unchanged.
    pub fn stage_linked(&self, project: &str) -> Result<String, String> {
        let parent = Path::new(project).join(".phoenix-ui/linked-bundles");
        let destination = parent.join(&self.digest);
        if !destination.exists() {
            std::fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
            let staging = util::mkdtemp(&jsp::join(&[&parent.to_string_lossy(), "staging-"]))?;
            if let Err(error) = self
                .expand(Path::new(&staging))
                .and_then(|_| std::fs::rename(&staging, &destination).map_err(|e| e.to_string()))
            {
                util::rm_rf(&staging);
                return Err(error);
            }
        }
        Ok(destination.to_string_lossy().into_owned())
    }
}
