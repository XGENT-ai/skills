//! Explicit, verified local distribution input. No resolver or network fallback.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::providers::{Sys, PROVIDER_DIRS};
use crate::util::{self, jsp};

const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_BLOB_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
const MAX_FILES: usize = 50_000;
pub const RUNTIME_MANIFEST_PATH: &str = "skills/phoenix-ui/scripts/ENGINE.json";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u64,
    #[serde(rename = "derivedRuntimeManifest")]
    derived_runtime_manifest: String,
    providers: BTreeMap<String, Provider>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
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
    // Keep the checked bytes: staging never reopens a pathname after validation.
    blobs: BTreeMap<String, Vec<u8>>,
}

pub fn source(sys: &Sys) -> Option<&str> {
    sys.env
        .get("PHOENIX_UI_BUNDLE_PATH")
        .filter(|s| !s.is_empty())
        .map(String::as_str)
}

fn plain_dir(path: &Path) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.is_symlink() || !meta.is_dir() {
        return Err(format!(
            "Local bundle directory must not be a symlink: {}",
            path.display()
        ));
    }
    Ok(())
}

fn read_file(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.is_symlink() || !meta.is_file() || meta.len() > limit {
        return Err(format!(
            "Local bundle file is not a bounded regular file: {}",
            path.display()
        ));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 != meta.len() || bytes.len() as u64 > limit {
        return Err(format!(
            "Local bundle file size changed or exceeds the limit: {}",
            path.display()
        ));
    }
    Ok(bytes)
}

fn digest(bytes: &[u8]) -> String {
    crate::bundle::hex(&Sha256::digest(bytes))
}

fn relative_file(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

impl LocalBundle {
    /// Validate VERSION, every provider path and every referenced blob before
    /// creating any staging directory or touching an installation.
    pub fn load(sys: &Sys) -> Result<Self, String> {
        let input = source(sys).ok_or("Local bundle unavailable. Pass --bundle-root DIR or PHOENIX_UI_BUNDLE_PATH; Phoenix does not download bundles or engines.")?;
        let source = PathBuf::from(jsp::resolve(&sys.cwd, &[input]));
        plain_dir(&source)?;
        plain_dir(&source.join("blobs"))?;
        let parent = source
            .parent()
            .ok_or("Local bundle has no adjacent VERSION.json directory")?;
        let version_bytes = read_file(&parent.join("VERSION.json"), 1024 * 1024)?;
        let version: serde_json::Value = serde_json::from_slice(&version_bytes)
            .map_err(|e| format!("Invalid VERSION.json: {e}"))?;
        if version
            .get("toolVersion")
            .and_then(serde_json::Value::as_str)
            != Some(env!("CARGO_PKG_VERSION"))
            || version
                .get("bundleSchema")
                .and_then(serde_json::Value::as_u64)
                != Some(1)
            || version
                .get("reviewSchema")
                .and_then(serde_json::Value::as_u64)
                != Some(1)
            || ["npmPackageVersion", "sourceCommit"].iter().any(|key| {
                version
                    .get(*key)
                    .and_then(serde_json::Value::as_str)
                    .is_none_or(str::is_empty)
            })
        {
            return Err("VERSION.json does not match the Phoenix runtime/schema contract".into());
        }
        let bytes = read_file(&source.join("manifest.json"), MAX_MANIFEST_BYTES)?;
        let manifest_digest = digest(&bytes);
        if version
            .get("bundleSha256")
            .and_then(serde_json::Value::as_str)
            != Some(&manifest_digest)
        {
            return Err("Local bundle manifest SHA-256 does not match VERSION.json".into());
        }
        let manifest: Manifest =
            serde_json::from_slice(&bytes).map_err(|e| format!("Invalid bundle manifest: {e}"))?;
        if manifest.schema != 1
            || manifest.providers.is_empty()
            || manifest.derived_runtime_manifest != RUNTIME_MANIFEST_PATH
        {
            return Err("Unsupported or empty local bundle manifest".into());
        }
        let mut blobs = BTreeMap::new();
        let mut total = 0u64;
        let mut files = 0usize;
        for (provider, entry) in &manifest.providers {
            if !PROVIDER_DIRS.contains(&provider.as_str()) && provider != ".codex" {
                return Err(format!("Unsupported bundle provider: {provider}"));
            }
            let mut portable_paths = BTreeSet::new();
            for (path, sha) in &entry.files {
                files += 1;
                if files > MAX_FILES
                    || path.eq_ignore_ascii_case(RUNTIME_MANIFEST_PATH)
                    || !relative_file(path)
                    || !portable_paths.insert(path.to_lowercase())
                    || (provider == ".codex" && path.starts_with("skills/"))
                {
                    return Err(format!("Unsafe bundle provider path: {provider}/{path}"));
                }
                if sha.len() != 64
                    || !sha
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err(format!("Invalid bundle blob SHA-256: {provider}/{path}"));
                }
                if !blobs.contains_key(sha) {
                    let bytes = read_file(&source.join("blobs").join(sha), MAX_BLOB_BYTES)?;
                    total = total
                        .checked_add(bytes.len() as u64)
                        .ok_or("Local bundle size overflow")?;
                    if total > MAX_TOTAL_BYTES || digest(&bytes) != *sha {
                        return Err(format!("Local bundle blob hash/size mismatch: {sha}"));
                    }
                    blobs.insert(sha.clone(), bytes);
                }
            }
            let mut exec = BTreeSet::new();
            if entry
                .exec
                .iter()
                .any(|path| !entry.files.contains_key(path) || !exec.insert(path))
            {
                return Err(format!("Invalid bundle executable paths for {provider}"));
            }
            if entry.files.contains_key("skills/phoenix-ui/SKILL.md") {
                portable_paths.insert(RUNTIME_MANIFEST_PATH.to_lowercase());
            }
            // Reject file/directory collisions before materialization.
            for path in &portable_paths {
                let parts: Vec<&str> = path.split('/').collect();
                for count in 1..parts.len() {
                    if portable_paths.contains(&parts[..count].join("/")) {
                        return Err(format!(
                            "Bundle file/directory collision: {provider}/{path}"
                        ));
                    }
                }
            }
        }
        for entry in std::fs::read_dir(source.join("blobs")).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            let sha = path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or("Invalid local blob filename")?;
            if sha.len() != 64
                || !sha
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err("Invalid local blob filename".into());
            }
            if !blobs.contains_key(sha) {
                let bytes = read_file(&path, MAX_BLOB_BYTES)?;
                total = total
                    .checked_add(bytes.len() as u64)
                    .ok_or("Local bundle size overflow")?;
                if total > MAX_TOTAL_BYTES || blobs.len() >= MAX_FILES || digest(&bytes) != sha {
                    return Err(format!("Local bundle blob hash/size mismatch: {sha}"));
                }
                blobs.insert(sha.into(), bytes);
            } else if std::fs::symlink_metadata(&path)
                .map_err(|e| e.to_string())?
                .is_symlink()
            {
                return Err("Local bundle blob must not be a symlink".into());
            }
        }
        Ok(Self {
            source,
            digest: manifest_digest,
            manifest,
            version_bytes,
            blobs,
        })
    }

    /// The caller owns this directory. Fresh staging has no incoming links.
    pub fn expand(&self, destination: &Path) -> Result<(), String> {
        plain_dir(destination)?;
        if std::fs::read_dir(destination)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
        {
            return Err("Local bundle expansion requires an empty staging directory".into());
        }
        for (provider, entry) in &self.manifest.providers {
            for (relative, sha) in &entry.files {
                let file = destination.join(provider).join(relative);
                std::fs::create_dir_all(file.parent().unwrap()).map_err(|e| e.to_string())?;
                // create_new refuses an existing file or symlink in staging.
                use std::io::Write;
                let mut out = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&file)
                    .map_err(|e| e.to_string())?;
                out.write_all(&self.blobs[sha]).map_err(|e| e.to_string())?;
                if entry.exec.contains(relative) {
                    util::set_executable(&file.to_string_lossy())?;
                }
            }
            if entry.files.contains_key("skills/phoenix-ui/SKILL.md") {
                let file = destination.join(provider).join(RUNTIME_MANIFEST_PATH);
                std::fs::create_dir_all(file.parent().unwrap()).map_err(|e| e.to_string())?;
                use std::io::Write;
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(file)
                    .and_then(|mut out| out.write_all(&self.version_bytes))
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

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

    /// Links need persistent, content-addressed staging. Validate a reused
    /// staging tree too: user edits are never silently replaced under a link.
    pub fn stage_linked(&self, project: &str) -> Result<String, String> {
        let state = Path::new(project).join(".phoenix-ui");
        let parent = state.join("linked-bundles");
        for path in [&state, &parent] {
            if path.exists() || path.is_symlink() {
                plain_dir(path)?;
            } else {
                std::fs::create_dir(path).map_err(|e| e.to_string())?;
            }
        }
        let destination = parent.join(&self.digest);
        if destination.exists() || destination.is_symlink() {
            plain_dir(&destination)?;
            let mut expected = BTreeSet::new();
            for (provider, entry) in &self.manifest.providers {
                for (relative, sha) in &entry.files {
                    let path = destination.join(provider).join(relative);
                    let mut ancestor = path.parent().unwrap();
                    while ancestor != destination {
                        plain_dir(ancestor)?;
                        ancestor = ancestor.parent().unwrap();
                    }
                    if digest(&read_file(&path, MAX_BLOB_BYTES)?) != *sha {
                        return Err(format!(
                            "Linked local staging was modified: {}",
                            path.display()
                        ));
                    }
                    expected.insert(path);
                }
                if entry.files.contains_key("skills/phoenix-ui/SKILL.md") {
                    let path = destination.join(provider).join(RUNTIME_MANIFEST_PATH);
                    if read_file(&path, 1024 * 1024)? != self.version_bytes {
                        return Err("Linked local runtime manifest was modified".into());
                    }
                    expected.insert(path);
                }
            }
            fn walk(path: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), String> {
                plain_dir(path)?;
                for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
                    let path = entry.map_err(|e| e.to_string())?.path();
                    let meta = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
                    if meta.is_symlink() {
                        return Err("Linked local staging contains a symlink".into());
                    }
                    if meta.is_dir() {
                        walk(&path, files)?;
                    } else if meta.is_file() {
                        files.insert(path);
                    } else {
                        return Err("Linked local staging contains a non-regular file".into());
                    }
                }
                Ok(())
            }
            let mut actual = BTreeSet::new();
            walk(&destination, &mut actual)?;
            if expected != actual {
                return Err("Linked local staging contains unexpected files".into());
            }
        } else {
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
