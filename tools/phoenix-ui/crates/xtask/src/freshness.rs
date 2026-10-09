//! Logical build inputs and hashes of the committed browser artifacts.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub const ASSETS: [&str; 2] = [
    "crates/live/assets/detect-antipatterns-browser.js",
    "crates/live/assets/antipatterns.json",
];
const MANIFEST: &str = "crates/live/assets/bundle-inputs.json";

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FileHash {
    sha256: String,
    size: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    input_sha256: String,
    inputs: BTreeMap<String, FileHash>,
    options: serde_json::Value,
    outputs: BTreeMap<String, FileHash>,
}

fn hash(path: &Path) -> Result<FileHash, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(FileHash {
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        size: bytes.len() as u64,
    })
}

fn tree(root: &Path, dir: &str, inputs: &mut BTreeMap<String, FileHash>) -> Result<(), String> {
    for entry in fs::read_dir(root.join(dir)).map_err(|e| format!("{dir}: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = format!("{dir}/{}", entry.file_name().to_string_lossy());
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            tree(root, &name, inputs)?;
        } else if kind.is_file() {
            inputs.insert(name.clone(), hash(&root.join(name))?);
        } else {
            return Err(format!(
                "Build input must be a regular file or directory: {name}"
            ));
        }
    }
    Ok(())
}

fn inputs(root: &Path) -> Result<BTreeMap<String, FileHash>, String> {
    let mut inputs = BTreeMap::new();
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "build-tools.lock.json",
        "mise.toml",
        ".cargo/config.toml",
    ] {
        inputs.insert(name.to_string(), hash(&root.join(name))?);
    }
    for name in ["foundation", "core", "wasm", "bundle", "xtask"] {
        let dir = format!("crates/{name}");
        let manifest = format!("{dir}/Cargo.toml");
        inputs.insert(manifest.clone(), hash(&root.join(manifest))?);
        tree(root, &format!("{dir}/src"), &mut inputs)?;
    }
    tree(root, "browser-bundle", &mut inputs)?;
    let repository = root.join("../..");
    for name in [
        "rust-toolchain.toml",
        "mise.toml",
        "scripts/phoenix-build-tools.mjs",
    ] {
        inputs.insert(format!("repository/{name}"), hash(&repository.join(name))?);
    }
    Ok(inputs)
}

fn options() -> serde_json::Value {
    serde_json::json!({
        "target": "wasm32-unknown-unknown", "profile": "release", "features": [],
        "optLevel": "z", "bindgen": ["--target", "no-modules", "--no-typescript", "--out-name", "impeccable"],
        "binaryen": ["-Oz", "--all-features"],
        "pathRemap": ["source=/build/phoenix-ui", "home=/build/home"],
        "cargo": ["--locked", "--offline"], "backend": "cargo-bindgen-binaryen-v1"
    })
}

fn input_hash(inputs: &BTreeMap<String, FileHash>, options: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(&(inputs, options)).expect("serializable build inputs");
    format!("{:x}", Sha256::digest(bytes))
}

pub fn record(root: &Path) -> Result<(), String> {
    let inputs = inputs(root)?;
    let options = options();
    let outputs = ASSETS
        .iter()
        .map(|name| hash(&root.join(name)).map(|hash| (name.to_string(), hash)))
        .collect::<Result<_, _>>()?;
    let manifest = Manifest {
        schema_version: 1,
        input_sha256: input_hash(&inputs, &options),
        inputs,
        options,
        outputs,
    };
    let mut bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    fs::write(root.join(MANIFEST), bytes).map_err(|e| e.to_string())
}

pub fn check(root: &Path, registry: &str) -> Result<(), String> {
    let bytes =
        fs::read(root.join(MANIFEST)).map_err(|e| format!("Missing build manifest: {e}"))?;
    let manifest: Manifest =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid build manifest: {e}"))?;
    let current = inputs(root)?;
    let options = options();
    if manifest.schema_version != 1
        || manifest.inputs != current
        || manifest.options != options
        || manifest.input_sha256 != input_hash(&current, &options)
    {
        return Err(
            "Browser bundle inputs changed; run the Phoenix build entry to regenerate".into(),
        );
    }
    if manifest.outputs.len() != ASSETS.len() {
        return Err("Browser bundle output manifest is incomplete".into());
    }
    for name in ASSETS {
        if manifest.outputs.get(name) != Some(&hash(&root.join(name))?) {
            return Err(format!(
                "Generated artifact changed: {name}; regenerate the browser bundle"
            ));
        }
    }
    if fs::read(root.join(ASSETS[1])).map_err(|e| e.to_string())? != registry.as_bytes() {
        return Err("Generated rule registry differs from the current Rust registry".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(base: &Path) -> std::path::PathBuf {
        let root = base.join("tools/phoenix-ui");
        for dir in ["browser-bundle", "crates/live/assets", ".cargo"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        for name in ["foundation", "core", "wasm", "bundle", "xtask"] {
            let dir = root.join(format!("crates/{name}"));
            fs::create_dir_all(dir.join("src")).unwrap();
            fs::write(dir.join("Cargo.toml"), "[package]\n").unwrap();
            fs::write(dir.join("src/lib.rs"), "fn example() {}\n").unwrap();
        }
        for name in [
            "Cargo.toml",
            "Cargo.lock",
            "build-tools.lock.json",
            "mise.toml",
            ".cargo/config.toml",
        ] {
            fs::write(root.join(name), name).unwrap();
        }
        fs::create_dir_all(base.join("scripts")).unwrap();
        for name in [
            "rust-toolchain.toml",
            "mise.toml",
            "scripts/phoenix-build-tools.mjs",
        ] {
            fs::write(base.join(name), name).unwrap();
        }
        fs::write(root.join(ASSETS[0]), "generated wasm bundle").unwrap();
        fs::write(root.join(ASSETS[1]), "{\"rules\":[]}").unwrap();
        root
    }

    #[test]
    fn relocation_preserves_freshness_and_manifest() {
        let base =
            std::env::temp_dir().join(format!("phoenix-freshness-relocate-{}", std::process::id()));
        let first = fixture(&base.join("first checkout"));
        let second = fixture(&base.join("different/path"));
        record(&first).unwrap();
        record(&second).unwrap();
        assert_eq!(
            fs::read(first.join(MANIFEST)).unwrap(),
            fs::read(second.join(MANIFEST)).unwrap()
        );
        assert!(check(&first, "{\"rules\":[]}").is_ok());
        assert!(check(&second, "{\"rules\":[]}").is_ok());
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn detects_changed_rust_tools_lock_and_generated_bytes() {
        let base =
            std::env::temp_dir().join(format!("phoenix-freshness-tamper-{}", std::process::id()));
        let root = fixture(&base);
        for name in [
            "crates/core/src/lib.rs",
            "build-tools.lock.json",
            "Cargo.lock",
            ASSETS[0],
        ] {
            record(&root).unwrap();
            let original = fs::read(root.join(name)).unwrap();
            fs::write(root.join(name), "changed").unwrap();
            assert!(check(&root, "{\"rules\":[]}").is_err(), "accepted {name}");
            fs::write(root.join(name), original).unwrap();
        }
        fs::remove_dir_all(base).unwrap();
    }
}
