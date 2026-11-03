// Provider names/defaults adapted from Phoenix UI install/cli.rs (Apache-2.0).
// Diagram writes only skills/phoenix-diagram; Codex's .codex half has no hooks.
use super::bundle::{self, Bundle};
use super::{
    invalid, io, InstallReceipt, InstallRequest, InstallResult, InstallStatus, InstallWrite,
};
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const PROVIDERS: [&str; 19] = [
    ".agent",
    ".agents",
    ".claude",
    ".codex",
    ".cursor",
    ".dsh",
    ".gemini",
    ".github",
    ".grok",
    ".hermes",
    ".kiro",
    ".opencode",
    ".pi",
    ".qoder",
    ".rovodev",
    ".trae",
    ".trae-cn",
    ".veto",
    ".vibe",
];
fn providers(requested: Option<&[String]>, project: &Path) -> InstallResult<Vec<String>> {
    let mut result = BTreeSet::new();
    if let Some(tokens) = requested {
        if tokens.is_empty() {
            return Err(invalid(
                "install/empty-providers",
                "Choose at least one provider",
            ));
        }
        for token in tokens {
            let token = token.trim();
            let provider = if token == "codex" {
                ".agents".into()
            } else if token.starts_with('.') {
                token.to_owned()
            } else {
                format!(".{token}")
            };
            if !PROVIDERS.contains(&provider.as_str()) {
                return Err(invalid("install/unknown-provider", token));
            }
            result.insert(provider);
        }
    } else {
        for provider in PROVIDERS {
            if project.join(provider).is_dir() {
                result.insert(if provider == ".codex" {
                    ".agents".into()
                } else {
                    provider.into()
                });
            }
        }
        if result.is_empty() {
            result.insert(".claude".into());
        }
    }
    if result.contains(".agents") || result.contains(".codex") {
        result.insert(".agents".into());
        result.insert(".codex".into());
    }
    Ok(result.into_iter().collect())
}

pub(super) fn install(request: &InstallRequest, apply: bool) -> InstallResult<InstallReceipt> {
    if !request.project.is_dir() {
        return Err(invalid(
            "install/project",
            format!("{} is not a directory", request.project.display()),
        ));
    }
    let bundle = Bundle::load(request)?;
    let mut report = InstallReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "install".into(),
        status: InstallStatus::Planned,
        diagnostics: vec![],
        engine: bundle.engine.clone(),
        npm_package_version: bundle.version.npm_package_version.clone(),
        project: request.project.clone(),
        providers: providers(request.providers.as_deref(), &request.project)?,
        bundle_manifest_sha256: bundle.manifest_sha256.clone(),
        version_manifest_sha256: bundle.version_sha256.clone(),
        writes: vec![],
        unchanged: vec![],
        removals: vec![],
        reload_steps: vec!["Reload the project in a new provider session and verify the loaded phoenix-diagram skill path.".into()],
    };
    let mut pending = vec![];
    for provider in report.providers.iter().filter(|p| p.as_str() != ".codex") {
        // The skill folder belongs to this tool. A symlinked folder (a source
        // checkout or skills manager) is replaced by a copy rather than written
        // through; files a newer package no longer ships are removed.
        let dir = format!("{provider}/skills/phoenix-diagram");
        let root = request.project.join(&dir);
        let linked = root.is_symlink();
        if linked {
            report.removals.push(dir.clone());
        } else {
            for name in files_under(&root) {
                if !bundle.files.contains_key(&name) {
                    report.removals.push(format!("{dir}/{name}"));
                }
            }
        }
        for (name, source) in &bundle.files {
            let path = format!("{dir}/{name}");
            let target = request.project.join(&path);
            let current = (!linked).then(|| fs::read(&target).ok()).flatten().zip(
                fs::metadata(&target)
                    .ok()
                    .map(|meta| meta.permissions().mode() & 0o777),
            );
            if current.is_some_and(|(bytes, mode)| bytes == source.bytes && mode == source.mode) {
                report.unchanged.push(path);
            } else {
                report.writes.push(InstallWrite {
                    path,
                    after: bundle::digest(&source.bytes),
                    mode: source.mode,
                });
                pending.push((target, source));
            }
        }
    }
    if report.writes.is_empty() && report.removals.is_empty() {
        report.status = InstallStatus::Unchanged;
    }
    if apply {
        for path in &report.removals {
            fs::remove_file(request.project.join(path)).map_err(io)?;
        }
        for (path, source) in pending {
            fs::create_dir_all(path.parent().expect("skill file parent")).map_err(io)?;
            fs::write(&path, &source.bytes).map_err(io)?;
            fs::set_permissions(&path, fs::Permissions::from_mode(source.mode)).map_err(io)?;
        }
        // Earlier versions kept install journals here; nothing reads them now.
        let old_state = request.project.join(".phoenix-diagram/install");
        if old_state.is_dir() {
            fs::remove_dir_all(old_state).map_err(io)?;
        }
        if report.status != InstallStatus::Unchanged {
            report.status = InstallStatus::Installed;
        }
    }
    Ok(report)
}

/// Relative paths of the files below `root`; empty when it does not exist.
fn files_under(root: &Path) -> Vec<String> {
    let mut files = vec![];
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
                files.push(relative.to_string_lossy().into_owned());
            }
        }
    }
    files
}
