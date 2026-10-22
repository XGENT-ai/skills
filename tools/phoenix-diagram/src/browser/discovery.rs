// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Paul Bakaus. See LICENSE-APACHE-2.0.
// Adapted from Phoenix UI browser/src/discovery.rs at 4a65a1525f9ed03c775a0e18935901e1edbb773f.
// Phoenix changes: diagram override only, Chrome/Chromium and Unix targets only.

use super::cdp::Error;
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn executable(path: &Path) -> bool {
    std::fs::metadata(path)
        .is_ok_and(|stat| stat.is_file() && stat.permissions().mode() & 0o111 != 0)
}

pub(super) fn find(override_path: Option<&Path>) -> Result<PathBuf, Error> {
    let configured = override_path
        .map(Path::to_owned)
        .or_else(|| std::env::var_os("PHOENIX_DIAGRAM_BROWSER").map(PathBuf::from));
    let env: HashMap<_, _> = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    find_in(configured.as_deref(), &env, &candidates(&env))
}

fn find_in(
    override_path: Option<&Path>,
    env: &HashMap<String, String>,
    candidates: &[PathBuf],
) -> Result<PathBuf, Error> {
    let configured = override_path
        .map(Path::to_owned)
        .or_else(|| env.get("PHOENIX_DIAGRAM_BROWSER").map(PathBuf::from));
    if let Some(path) = configured {
        if executable(&path) {
            return Ok(path);
        }
        return Err(Error::new("browser/configured-path", "discovery", format!("PHOENIX_DIAGRAM_BROWSER or the explicit browser path is not an executable ordinary file: {}", path.display())));
    }
    candidates.iter().find(|path| executable(path)).cloned().ok_or_else(|| Error::new("browser/unavailable", "discovery", "No Chrome or Chromium installation was found. Set PHOENIX_DIAGRAM_BROWSER to an installed browser executable."))
}

fn candidates(env: &HashMap<String, String>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if cfg!(target_os = "macos") {
        let mut roots = vec![PathBuf::from("/Applications")];
        if let Some(home) = env.get("HOME") {
            roots.push(Path::new(home).join("Applications"));
        }
        for bundle in [
            "Google Chrome.app/Contents/MacOS/Google Chrome",
            "Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
            "Chromium.app/Contents/MacOS/Chromium",
        ] {
            for root in &roots {
                paths.push(root.join(bundle));
            }
        }
    } else {
        let directories: Vec<_> = env
            .get("PATH")
            .map(|value| std::env::split_paths(value).collect())
            .unwrap_or_default();
        for name in [
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
        ] {
            for directory in &directories {
                paths.push(directory.join(name));
            }
        }
        paths.push(PathBuf::from("/opt/google/chrome/chrome"));
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_missing_installation_is_distinct_from_a_bad_explicit_override() {
        let env = HashMap::new();
        assert_eq!(
            find_in(None, &env, &[]).unwrap_err().code,
            "browser/unavailable"
        );
        let env = HashMap::from([("PHOENIX_DIAGRAM_BROWSER".into(), "/missing/browser".into())]);
        let fallback = std::env::current_exe().unwrap();
        assert_eq!(
            find_in(None, &env, &[fallback]).unwrap_err().code,
            "browser/configured-path"
        );
    }
    #[test]
    fn an_explicit_option_does_not_mutate_or_defer_to_environment() {
        let executable = std::env::current_exe().unwrap();
        let env = HashMap::from([("PHOENIX_DIAGRAM_BROWSER".into(), "/missing/browser".into())]);
        assert_eq!(find_in(Some(&executable), &env, &[]).unwrap(), executable);
        assert_eq!(env["PHOENIX_DIAGRAM_BROWSER"], "/missing/browser");
    }
}
