use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Build the actual deduplicated distribution shape from a deterministic
/// provider fixture. Never invokes the published installer or a provider.
pub fn pack(flat: &str) -> String {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<String, (Vec<u8>, bool)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if relative == "skills/phoenix-ui/scripts/ENGINE.json" {
                    continue;
                }
                #[cfg(unix)]
                let executable = {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::metadata(&path).unwrap().permissions().mode() & 0o111 != 0
                };
                #[cfg(not(unix))]
                let executable =
                    relative.ends_with("scripts/phoenix-ui") || relative.contains("scripts/bin/");
                files.insert(relative, (std::fs::read(path).unwrap(), executable));
            }
        }
    }
    let release = PathBuf::from(format!("{flat}-release"));
    let bundle = release.join("bundle");
    std::fs::create_dir_all(bundle.join("blobs")).unwrap();
    let mut providers = BTreeMap::<String, Value>::new();
    for entry in std::fs::read_dir(flat).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let mut contents = BTreeMap::new();
        walk(&path, &path, &mut contents);
        let mut files = BTreeMap::new();
        let mut exec = Vec::new();
        for (relative, (bytes, executable)) in contents {
            let digest = sha(&bytes);
            std::fs::write(bundle.join("blobs").join(&digest), bytes).unwrap();
            if executable {
                exec.push(relative.clone());
            }
            files.insert(relative, digest);
        }
        providers.insert(
            path.file_name().unwrap().to_string_lossy().into_owned(),
            json!({"files":files,"exec":exec}),
        );
    }
    let manifest = json!({"schema":1,"derivedRuntimeManifest":"skills/phoenix-ui/scripts/ENGINE.json","providers":providers});
    let bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    std::fs::write(bundle.join("manifest.json"), &bytes).unwrap();
    std::fs::write(release.join("VERSION.json"), serde_json::to_vec_pretty(&json!({
        "toolVersion":env!("CARGO_PKG_VERSION"),"bundleSchema":1,"bundleSha256":sha(&bytes),
        "npmPackageVersion":"0.7.0-rc.0","reviewSchema":1,"sourceCommit":"1111111111111111111111111111111111111111",
        "distribution":"development","engines":{}
    })).unwrap()).unwrap();
    bundle.to_string_lossy().into_owned()
}
