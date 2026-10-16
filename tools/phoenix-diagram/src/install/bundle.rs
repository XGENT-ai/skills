// Apache-2.0 source patterns: Phoenix UI install/plan.rs and
// skills/phoenix-ui/scripts/phoenix-bootstrap.cjs. Phoenix diagram validates its
// own manifest, derived ENGINE bytes, four targets, and native contract identity.
use super::disk::{self, Directory, Snapshot};
use super::{
    FileDigest, InstallEngine, InstallError, InstallErrorKind, InstallRequest, InstallResult,
};
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

const TARGETS: [&str; 4] = ["darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64"];
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Version {
    pub schema_version: u32,
    pub distribution: String,
    pub tool: String,
    pub tool_version: String,
    pub npm_package_version: String,
    pub source_commit: String,
    pub source_input_sha256: String,
    pub input_contract_schema: u32,
    pub receipt_schema: u32,
    pub viewer_revision: String,
    pub bundle_sha256: String,
    pub engines: BTreeMap<String, Engine>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Engine {
    pub asset: String,
    pub size: u64,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    tool: String,
    version: String,
    files: Vec<Entry>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Entry {
    path: String,
    size: u64,
    sha256: String,
    mode: u32,
}

pub(super) struct Bundle {
    pub root: Arc<Directory>,
    pub version: Version,
    pub version_sha256: String,
    pub manifest_sha256: String,
    pub engine: InstallEngine,
    pub files: BTreeMap<String, Snapshot>,
    version_path: PathBuf,
    version_snapshot: Snapshot,
    manifest_snapshot: Snapshot,
    engine_path: PathBuf,
    engine_identity: crate::output::FileIdentity,
}
fn invalid(code: &str, message: impl Into<String>) -> InstallError {
    InstallError {
        kind: InstallErrorKind::Invalid,
        code: code.into(),
        message: message.into(),
    }
}
pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn digest(bytes: &[u8]) -> FileDigest {
    FileDigest {
        sha256: hash(bytes),
        bytes: bytes.len() as u64,
    }
}
pub(super) fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn semver(value: &str) -> bool {
    let (core, suffix) = value
        .split_once('-')
        .map_or((value, None), |(a, b)| (a, Some(b)));
    let parts: Vec<_> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        && suffix.is_none_or(|s| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
        })
}
pub(super) fn platform() -> InstallResult<&'static str> {
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
pub(super) fn allowed(path: &str, derived: bool) -> bool {
    if disk::relative(path).is_err() {
        return false;
    }
    match path {
        "SKILL.md"
        | "scripts/phoenix-diagram"
        | "agents/openai.yaml"
        | "schemas/README.md"
        | "scripts/phoenix-launcher.cjs"
        | "scripts/phoenix-bootstrap.cjs" => true,
        "ENGINE.json" => derived,
        _ => {
            let parts: Vec<_> = path.split('/').collect();
            parts.len() == 2
                && match parts[0] {
                    "schemas" | "examples" => parts[1].ends_with(".json") && parts[1].len() > 5,
                    "references" => parts[1].ends_with(".md") && parts[1].len() > 3,
                    _ => false,
                }
        }
    }
}
fn snapshot(path: &Path) -> InstallResult<Snapshot> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let directory = Directory::open_source(parent)?;
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| invalid("install/unsafe-path", path.display().to_string()))?;
    directory
        .read(name, &[1])?
        .ok_or_else(|| invalid("install/missing-file", path.display().to_string()))
}
fn required(root: &Arc<Directory>, path: &str) -> InstallResult<Snapshot> {
    root.read_path(path)?
        .ok_or_else(|| invalid("install/missing-bundle-file", path))
}
fn inventory(
    root: &Arc<Directory>,
    relative: &str,
    files: &BTreeSet<String>,
    count: &mut usize,
) -> InstallResult<()> {
    let directory = if relative.is_empty() {
        root.clone()
    } else {
        root.descend(relative)?
            .ok_or_else(|| invalid("install/bundle-changed", relative))?
    };
    for name in directory.list()? {
        *count += 1;
        if *count > 50_000 {
            return Err(invalid(
                "install/bundle-inventory-incomplete",
                "Bundle inventory exceeds 50,000 entries",
            ));
        }
        let path = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };
        let entry = directory
            .entry(&name)?
            .ok_or_else(|| invalid("install/bundle-changed", &path))?;
        match entry.kind {
            libc::S_IFDIR
                if files
                    .iter()
                    .any(|file| file.starts_with(&format!("{path}/"))) =>
            {
                inventory(root, &path, files, count)?
            }
            libc::S_IFREG if files.contains(&path) => {}
            _ => return Err(invalid("install/extra-bundle-entry", path)),
        }
    }
    Ok(())
}
impl Bundle {
    pub fn load(request: &InstallRequest) -> InstallResult<Self> {
        let root = Directory::open_source(&request.bundle_root)?;
        let manifest_snapshot = required(&root, "manifest.json")?;
        if manifest_snapshot.identity.bytes > 16 * 1024 * 1024 {
            return Err(invalid(
                "install/manifest-too-large",
                "Bundle manifest exceeds 16 MiB",
            ));
        }
        let manifest: Manifest =
            parse(&manifest_snapshot.bytes, "install/invalid-bundle-manifest")?;
        if manifest.schema_version != 1
            || manifest.tool != "phoenix-diagram"
            || manifest.version != crate::TOOL_VERSION
            || manifest.files.is_empty()
            || manifest.files.len() > 10_000
        {
            return Err(invalid(
                "install/invalid-bundle-manifest",
                "Unexpected bundle schema, tool, version, or file count",
            ));
        }
        let version_snapshot = snapshot(&request.version_manifest)?;
        if version_snapshot.identity.bytes > 1024 * 1024 {
            return Err(invalid(
                "install/version-too-large",
                "VERSION exceeds 1 MiB",
            ));
        }
        let version: Version = parse(&version_snapshot.bytes, "install/invalid-version-manifest")?;
        let manifest_sha256 = hash(&manifest_snapshot.bytes);
        version.validate(&manifest_sha256)?;
        let mut files = BTreeMap::new();
        let mut folded = BTreeSet::new();
        let mut physical = BTreeSet::new();
        let mut total = 0u64;
        for entry in manifest.files {
            if !allowed(&entry.path, false)
                || !valid_hash(&entry.sha256)
                || ![0o644, 0o755].contains(&entry.mode)
                || entry.size > 64 * 1024 * 1024
                || !folded.insert(entry.path.to_lowercase())
            {
                return Err(invalid("install/unsafe-bundle-file", entry.path));
            }
            total = total
                .checked_add(entry.size)
                .ok_or_else(|| invalid("install/bundle-too-large", "Bundle size overflow"))?;
            if total > 256 * 1024 * 1024 {
                return Err(invalid(
                    "install/bundle-too-large",
                    "Bundle exceeds 256 MiB",
                ));
            }
            let file = required(&root, &entry.path)?;
            if !physical.insert((file.identity.device.clone(), file.identity.inode.clone())) {
                return Err(invalid("install/unsafe-bundle-file", entry.path));
            }
            if file.identity.sha256 != entry.sha256
                || file.identity.bytes != entry.size
                || file.identity.mode != entry.mode
            {
                return Err(invalid("install/bundle-file-mismatch", entry.path));
            }
            files.insert(entry.path, file);
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
        let engine_manifest = required(&root, "ENGINE.json")?;
        if engine_manifest.bytes != version_snapshot.bytes || engine_manifest.identity.mode != 0o644
        {
            return Err(invalid(
                "install/engine-manifest-mismatch",
                "ENGINE.json must be a 0644 byte-for-byte copy of VERSION.json",
            ));
        }
        files.insert("ENGINE.json".into(), engine_manifest);
        let mut all: BTreeSet<_> = files.keys().cloned().collect();
        all.insert("manifest.json".into());
        inventory(&root, "", &all, &mut 0)?;
        let engine_snapshot = snapshot(&request.engine)?;
        let engine_meta = &version.engines[platform()?];
        if engine_snapshot.identity.bytes != engine_meta.size
            || engine_snapshot.identity.sha256 != engine_meta.sha256
            || engine_snapshot.identity.mode & 0o111 == 0
        {
            return Err(invalid(
                "install/engine-mismatch",
                "Native engine size, digest, or executable mode differs from VERSION",
            ));
        }
        let engine = InstallEngine {
            source_input_sha256: version.source_input_sha256.clone(),
            input_contract_schema: version.input_contract_schema,
            receipt_schema: version.receipt_schema,
            viewer_revision: version.viewer_revision.clone(),
            binary: FileDigest {
                sha256: engine_snapshot.identity.sha256.clone(),
                bytes: engine_snapshot.identity.bytes,
            },
        };
        let engine_identity = engine_snapshot.identity.clone();
        drop(engine_snapshot);
        probe(&request.engine)?;
        let bundle = Self {
            root,
            version,
            version_sha256: hash(&version_snapshot.bytes),
            manifest_sha256,
            engine,
            files,
            version_path: request.version_manifest.clone(),
            version_snapshot,
            manifest_snapshot,
            engine_path: request.engine.clone(),
            engine_identity,
        };
        bundle.verify()?;
        Ok(bundle)
    }
    pub fn verify(&self) -> InstallResult<()> {
        self.root.verify()?;
        for (path, expected) in &self.files {
            let actual = required(&self.root, path)?;
            if !disk::matches(&expected.identity, &actual.identity) {
                return Err(invalid("install/bundle-changed", path));
            }
        }
        let manifest = required(&self.root, "manifest.json")?;
        let version = snapshot(&self.version_path)?;
        let engine = snapshot(&self.engine_path)?;
        if !disk::matches(&self.manifest_snapshot.identity, &manifest.identity)
            || !disk::matches(&self.version_snapshot.identity, &version.identity)
            || !disk::matches(&self.engine_identity, &engine.identity)
        {
            return Err(invalid(
                "install/source-changed",
                "The validated package or native engine changed",
            ));
        }
        Ok(())
    }
}
impl Version {
    fn validate(&self, manifest_sha256: &str) -> InstallResult<()> {
        if self.schema_version != 1
            || self.tool != "phoenix-diagram"
            || self.tool_version != crate::TOOL_VERSION
            || !semver(&self.npm_package_version)
            || self.npm_package_version != env!("PHOENIX_DIAGRAM_NPM_PACKAGE_VERSION")
            || !matches!(
                self.distribution.as_str(),
                "development" | "candidate" | "release"
            )
            || self.source_commit.len() != 40
            || !self
                .source_commit
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.source_input_sha256 != env!("PHOENIX_DIAGRAM_SOURCE_INPUT_SHA256")
            || self.input_contract_schema != crate::INPUT_CONTRACT_SCHEMA
            || self.receipt_schema != crate::RECEIPT_SCHEMA
            || self.viewer_revision != crate::VIEWER_REVISION
            || self.bundle_sha256 != manifest_sha256
            || !valid_hash(&self.bundle_sha256)
            || self.engines.is_empty()
            || !self.engines.contains_key(platform()?)
            || self
                .engines
                .keys()
                .any(|key| !TARGETS.contains(&key.as_str()))
            || (self.distribution != "development"
                && TARGETS
                    .iter()
                    .any(|target| !self.engines.contains_key(*target)))
        {
            return Err(invalid(
                "install/version-identity-mismatch",
                "VERSION does not match this native engine, bundle, or supported distribution",
            ));
        }
        for (target, engine) in &self.engines {
            if engine.asset != format!("phoenix-diagram-{target}")
                || engine.size == 0
                || engine.size > 256 * 1024 * 1024
                || !valid_hash(&engine.sha256)
            {
                return Err(invalid("install/invalid-engine-entry", target));
            }
            if let Some(value) = &engine.url {
                let url = url::Url::parse(value)
                    .map_err(|_| invalid("install/invalid-engine-url", target))?;
                let local = matches!(
                    url.host_str(),
                    Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
                );
                if !url.username().is_empty()
                    || url.password().is_some()
                    || !(url.scheme() == "https" || (url.scheme() == "http" && local))
                {
                    return Err(invalid("install/invalid-engine-url", target));
                }
            }
        }
        Ok(())
    }
}

fn probe(path: &Path) -> InstallResult<()> {
    let mut child = Command::new(path)
        .arg("engine-probe")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| invalid("install/engine-probe", e.to_string()))?;
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    for descriptor in [stdout.as_raw_fd(), stderr.as_raw_fd()] {
        let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
        if flags < 0
            || unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(invalid(
                "install/engine-probe",
                "Cannot bound engine probe output",
            ));
        }
    }
    let started = std::time::Instant::now();
    let mut out = vec![];
    let mut err = vec![];
    let result = 'probe: loop {
        for (reader, bytes) in [
            (&mut stdout as &mut dyn Read, &mut out),
            (&mut stderr as &mut dyn Read, &mut err),
        ] {
            let mut buffer = [0u8; 4097];
            match reader.read(&mut buffer) {
                Ok(count) => bytes.extend_from_slice(&buffer[..count]),
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => break 'probe Err(invalid("install/engine-probe", e.to_string())),
            }
        }
        if out.len() > 4096 || err.len() > 4096 {
            break Err(invalid(
                "install/engine-probe",
                "Engine probe output exceeds 4 KiB",
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                // One final nonblocking drain includes bytes written just before exit.
                let _ = stdout.take(4097).read_to_end(&mut out);
                let _ = stderr.take(4097).read_to_end(&mut err);
                let expected = format!(
                    "phoenix-diagram-engine {} input={} receipt={} viewer={}\n",
                    crate::TOOL_VERSION,
                    crate::INPUT_CONTRACT_SCHEMA,
                    crate::RECEIPT_SCHEMA,
                    crate::VIEWER_REVISION
                );
                break if status.success() && out == expected.as_bytes() && err.is_empty() {
                    Ok(())
                } else {
                    Err(invalid(
                        "install/engine-probe",
                        "Native engine probe identity differs from the package",
                    ))
                };
            }
            Ok(None) => {}
            Err(e) => break 'probe Err(invalid("install/engine-probe", e.to_string())),
        }
        if started.elapsed() >= std::time::Duration::from_secs(3) {
            break Err(invalid(
                "install/engine-probe",
                "Engine probe exceeded three seconds",
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

// Reject duplicate keys before typed deserialization can overwrite a map key.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Check;
        impl<'de> Visitor<'de> for Check {
            type Value = Unique;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(Value::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Unique, E> {
                Ok(Unique(Value::from(v)))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Unique, E> {
                Ok(Unique(Value::from(v)))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Unique, E> {
                Ok(Unique(Value::from(v)))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Unique, E> {
                Ok(Unique(Value::String(v.into())))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Unique, E> {
                Ok(Unique(Value::String(v)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Unique, A::Error> {
                let mut values = vec![];
                while let Some(Unique(v)) = seq.next_element()? {
                    values.push(v);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Unique, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom(format!("Duplicate key: {key}")));
                    }
                    let Unique(value) = map.next_value()?;
                    values.insert(key, value);
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Check)
    }
}
pub(super) fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8], code: &str) -> InstallResult<T> {
    let Unique(value) = serde_json::from_slice(bytes).map_err(|e| invalid(code, e.to_string()))?;
    serde_json::from_value(value).map_err(|e| invalid(code, e.to_string()))
}
