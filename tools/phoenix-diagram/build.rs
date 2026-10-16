use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const INPUTS: &[&str] = &[
    "package.json",
    "tools/phoenix-diagram/Cargo.toml",
    "tools/phoenix-diagram/Cargo.lock",
    "tools/phoenix-diagram/build.rs",
    "tools/phoenix-diagram/src",
    "tools/phoenix-diagram/assets",
    "tools/phoenix-diagram/viewer",
    "tools/phoenix-diagram/upstream/SOURCE.json",
    "tools/phoenix-diagram/upstream/legacy-2.17/schemas",
    "skills/phoenix-diagram",
];

fn collect(root: &Path, path: &Path, files: &mut Vec<PathBuf>) {
    let metadata = fs::symlink_metadata(path).expect("build input exists");
    assert!(
        !metadata.file_type().is_symlink(),
        "symbolic build input: {}",
        path.display()
    );
    if metadata.is_dir() {
        for entry in fs::read_dir(path).expect("read build inputs") {
            collect(root, &entry.expect("read build entry").path(), files);
        }
    } else {
        assert!(
            metadata.is_file(),
            "non-regular build input: {}",
            path.display()
        );
        let relative = path.strip_prefix(root).expect("input within source tree");
        if relative != Path::new("skills/phoenix-diagram/ENGINE.json") {
            files.push(relative.to_path_buf());
        }
    }
}

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest directory"))
        .join("../..")
        .canonicalize()
        .expect("source root");
    let mut files = Vec::new();
    let package: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("package.json")).expect("read npm package identity"),
    )
    .expect("npm package JSON");
    println!(
        "cargo:rustc-env=PHOENIX_DIAGRAM_NPM_PACKAGE_VERSION={}",
        package["version"].as_str().expect("npm package version")
    );
    for input in INPUTS {
        let path = root.join(input);
        println!("cargo:rerun-if-changed={}", path.display());
        collect(&root, &path, &mut files);
    }
    let mut entries: Vec<_> = files
        .iter()
        .map(|relative| {
            let bytes = fs::read(root.join(relative)).expect("read source input bytes");
            (
                relative
                    .to_str()
                    .expect("UTF-8 build path")
                    .replace('\\', "/"),
                bytes.len() as u64,
                format!("{:x}", Sha256::digest(&bytes)),
            )
        })
        .collect();
    // PathBuf orders components; the portable manifest orders full UTF-8 paths.
    entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    entries.dedup_by(|a, b| a.0 == b.0);
    let manifest = serde_json::to_vec(&entries).expect("canonical source manifest");
    let digest = format!("{:x}", Sha256::digest(&manifest));
    println!("cargo:rustc-env=PHOENIX_DIAGRAM_SOURCE_INPUT_SHA256={digest}");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"))
            .join("source-inputs.json"),
        manifest,
    )
    .expect("write source input manifest");
}
