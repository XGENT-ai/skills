//! Local bundle comparison and provider copy/refresh/link/agent operations
//! over a staged bundle directory.
//! JS: skills.mjs `listSkillTreeFiles`, `normalizeForHash`, `hashSkillFile`,
//! `isUpToDate`, `copyProviderSkills`, `refreshProviderSkills`,
//! `copyProviderAgents`, `reportProviderAgents`, `isInProjectProviderLink`,
//! `resolveLinkSource`, `linkProviderSkills`.

use impeccable_common::Io;
use once_cell::sync::Lazy;
use regex::Regex;
use sha2::{Digest, Sha256};

use crate::providers::{
    opencode_global_config_dir, provider_display_name, Scope, Sys, PROVIDER_DIRS,
};
use crate::util::{self, jsp};

/// JS: listSkillTreeFiles(root): every file, sorted, relative with `/`.
pub fn list_skill_tree_files(root: &str) -> Vec<String> {
    fn walk(root: &str, dir: &str, out: &mut Vec<String>) {
        let Some(names) = util::read_dir_names(dir) else {
            return;
        };
        for name in names {
            let full = jsp::join(&[dir, &name]);
            let meta = match std::fs::metadata(&full) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_dir() {
                walk(root, &full, out);
            } else if meta.is_file() {
                out.push(jsp::relative("/", root, &full));
            }
        }
    }
    let mut out = Vec::new();
    if util::exists(root) {
        walk(root, root, &mut out);
    }
    out
}

static PROVIDER_PATH_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\.(claude|cursor|dsh|agents|agent|github|gemini|codex|grok|hermes|kiro|opencode|pi|qoder|trae|trae-cn|rovodev|vibe|veto)/skills/").unwrap()
});

/// JS: normalizeForHash(content)
pub fn normalize_for_hash(content: &str) -> String {
    PROVIDER_PATH_RE
        .replace_all(content, ".PROVIDER/skills/")
        .into_owned()
}

/// JS: hashSkillFile(filePath)
pub fn hash_skill_file(path: &str) -> Result<String, String> {
    let bytes = util::read_bytes(path)?;
    let mut hasher = Sha256::new();
    if let Ok(text) = std::str::from_utf8(&bytes) {
        hasher.update(normalize_for_hash(text).as_bytes());
    } else {
        hasher.update(bytes);
    }
    Ok(hex(&hasher.finalize()))
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// JS: isUpToDate(root, providers, bundleDir, scope, agentScope = scope).
/// `agent_scope` lets a home-rooted implicit update keep checking the user
/// agent dirs while the skill scope stays inferred (upstream d2a9efb9).
pub fn is_up_to_date(
    sys: &Sys,
    root: &str,
    providers: &[&str],
    bundle_dir: &str,
    scope: Option<Scope>,
    agent_scope: Option<Scope>,
) -> Result<bool, String> {
    let unique = sys.deduplicate_providers(root, providers, scope);
    if unique.is_empty() {
        return Ok(false);
    }
    for (provider, local_skills_dir) in unique {
        let bundle_skills_dir = jsp::join(&[bundle_dir, provider, "skills"]);
        // JS-PARITY: a provider with no bundled skills dir `continue`s past
        // the agents check too.
        if !util::exists(&bundle_skills_dir) {
            continue;
        }
        for name in util::read_dir_names(&bundle_skills_dir).unwrap_or_default() {
            let bundle_skill_dir = jsp::join(&[&bundle_skills_dir, &name]);
            let local_skill_dir = jsp::join(&[&local_skills_dir, &name]);
            if !util::exists(&jsp::join(&[&bundle_skill_dir, "SKILL.md"])) {
                continue;
            }
            if !util::exists(&local_skill_dir) {
                return Ok(false);
            }
            let bundle_files = list_skill_tree_files(&bundle_skill_dir);
            let local_files = list_skill_tree_files(&local_skill_dir);
            if bundle_files != local_files {
                return Ok(false);
            }
            for rel in &bundle_files {
                let b = hash_skill_file(&jsp::join(&[&bundle_skill_dir, rel]))?;
                let l = hash_skill_file(&jsp::join(&[&local_skill_dir, rel]))?;
                if b != l {
                    return Ok(false);
                }
            }
        }
        // Provider command artifacts (OpenCode's `commands/impeccable.md`) are
        // part of "current" too: an install whose skills match but whose
        // bridge is missing or drifted must refresh, otherwise
        // reinstall/update report success while the slash command stays
        // absent (#483). Only bundle-shipped files are checked, so pinned or
        // user commands never affect freshness. The commands dir sits next to
        // the matched skills dir, so deriving it from `local_skills_dir` stays
        // correct for every layout `copy_provider_commands` can write.
        let bundle_commands_dir = jsp::join(&[bundle_dir, provider, "commands"]);
        if util::exists(&bundle_commands_dir) {
            let local_commands_dir = jsp::join(&[&jsp::dirname(&local_skills_dir), "commands"]);
            for entry in util::read_dir_names(&bundle_commands_dir).unwrap_or_default() {
                let bundle_file = jsp::join(&[&bundle_commands_dir, &entry]);
                if !util::is_file(&bundle_file) {
                    continue;
                }
                let local_file = jsp::join(&[&local_commands_dir, &entry]);
                if !util::exists(&local_file) {
                    return Ok(false);
                }
                if hash_skill_file(&bundle_file)? != hash_skill_file(&local_file)? {
                    return Ok(false);
                }
            }
        }
        if !provider_agents_up_to_date(bundle_dir, root, provider, agent_scope)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// JS: isInProjectProviderLink(localSkillsDir, root, provider)
fn is_in_project_provider_link(local_skills_dir: &str, root: &str, provider: &str) -> bool {
    if !util::is_symlink(local_skills_dir) {
        return false;
    }
    let Some(target) = util::readlink(local_skills_dir) else {
        return false;
    };
    let resolved_target = jsp::resolve(&jsp::dirname(local_skills_dir), &[&target]);
    PROVIDER_DIRS
        .iter()
        .filter(|other| **other != provider)
        .any(|other| resolved_target == jsp::join(&[root, other, "skills"]))
}

/// The bundle's skill directories under `<bundle>/<provider>/skills` (real
/// directories only, like `Dirent.isDirectory()`).
fn bundle_skill_dirs(src_dir: &str) -> Vec<String> {
    util::read_dir_names(src_dir)
        .unwrap_or_default()
        .into_iter()
        .filter(|name| util::is_real_dir(&jsp::join(&[src_dir, name])))
        .collect()
}

/// JS: copyProviderSkills(bundleDir, root, targets, {scope}). Returns the
/// skill directories written (the JS returned only the count).
pub fn copy_provider_skills(
    sys: &Sys,
    bundle_dir: &str,
    root: &str,
    targets: &[&str],
    scope: Option<Scope>,
) -> Result<Vec<String>, String> {
    let mut written = Vec::new();
    for provider in targets {
        let src_dir = jsp::join(&[bundle_dir, provider, "skills"]);
        if !util::exists(&src_dir) {
            continue;
        }
        let local_skills_dir = if scope == Some(Scope::User) {
            sys.user_provider_skills_dir(root, provider)
        } else {
            jsp::join(&[root, provider, "skills"])
        };
        if is_in_project_provider_link(&local_skills_dir, root, provider) {
            let _ = std::fs::remove_file(&local_skills_dir);
        }
        let skills = bundle_skill_dirs(&src_dir);
        for skill in &skills {
            let src = jsp::join(&[&src_dir, skill]);
            let dest = jsp::join(&[&local_skills_dir, skill]);
            util::rm_rf(&dest);
            util::copy_dir(&src, &dest)?;
            written.push(dest);
        }
    }
    Ok(written)
}

/// JS: skills.mjs#providerCommandsDir. Project installs land at
/// `<root>/<configDir>/commands`; a user-scope OpenCode install must target
/// the config dir OpenCode actually scans.
fn provider_commands_dir(
    sys: &Sys,
    root: &str,
    provider_entry: &str,
    scope: Option<Scope>,
) -> String {
    if scope == Some(Scope::User) {
        jsp::join(&[&opencode_global_config_dir(&sys.env, root), "commands"])
    } else {
        jsp::join(&[root, provider_entry, "commands"])
    }
}

/// JS: skills.mjs#copyProviderCommands(bundleDir, root, targets, {scope}).
///
/// OpenCode discovers custom commands from `{command,commands}/**.md` under
/// any active config dir, so this mirrors `copy_provider_skills`: project
/// scope writes `<root>/<configDir>/commands/`, user scope writes
/// `opencode_global_config_dir(home)/commands`.
pub fn copy_provider_commands(
    sys: &Sys,
    bundle_dir: &str,
    root: &str,
    targets: &[&str],
    scope: Option<Scope>,
) -> usize {
    let mut written = 0usize;
    for target in targets {
        let dotted = format!(".{target}");
        let provider_entry: &str = if PROVIDER_DIRS.contains(&dotted.as_str()) {
            &dotted
        } else {
            target
        };
        let src_dir = jsp::join(&[bundle_dir, provider_entry, "commands"]);
        if !util::exists(&src_dir) {
            continue;
        }
        let local_commands_dir = provider_commands_dir(sys, root, provider_entry, scope);
        let _ = std::fs::create_dir_all(&local_commands_dir);
        let entries = util::read_dir_names(&src_dir).unwrap_or_default();
        for entry in &entries {
            let src = jsp::join(&[&src_dir, entry]);
            if !util::is_file(&src) {
                continue;
            }
            let dest = jsp::join(&[&local_commands_dir, entry]);
            util::rm_rf(&dest);
            if std::fs::copy(&src, &dest).is_ok() {
                written += 1;
            }
        }
    }
    written
}

/// JS: refreshProviderSkills(bundleDir, root, providers, scope). Returns the
/// skill directories refreshed.
pub fn refresh_provider_skills(
    sys: &Sys,
    bundle_dir: &str,
    root: &str,
    providers: &[&str],
    scope: Option<Scope>,
) -> Result<Vec<String>, String> {
    let mut updated = Vec::new();
    for (provider, local_skills_dir) in sys.deduplicate_providers(root, providers, scope) {
        let src_dir = jsp::join(&[bundle_dir, provider, "skills"]);
        if !util::exists(&src_dir) {
            continue;
        }
        for skill in bundle_skill_dirs(&src_dir) {
            let src = jsp::join(&[&src_dir, &skill]);
            let dest = jsp::join(&[&local_skills_dir, &skill]);
            if util::exists(&dest) {
                util::rm_rf(&dest);
            }
            util::copy_dir(&src, &dest)?;
            updated.push(dest);
        }
    }
    Ok(updated)
}

pub struct AgentResult {
    pub provider: &'static str,
    pub written: usize,
    pub dest_dir: String,
    pub user_dir: String,
    pub shadowed: Vec<String>,
}

struct AgentArtifact {
    ext: &'static str,
    user_dir: fn(&str) -> String,
    user_shadows_project: bool,
}

fn agent_artifact(provider: &str) -> Option<AgentArtifact> {
    match provider {
        // Claude Code's agents live at `.claude/agents/impeccable-*.md`;
        // project agents take precedence over user agents (upstream 7b945856).
        ".claude" => Some(AgentArtifact {
            ext: ".md",
            user_dir: |home| jsp::join(&[home, ".claude", "agents"]),
            user_shadows_project: false,
        }),
        ".github" => Some(AgentArtifact {
            ext: ".agent.md",
            user_dir: |home| jsp::join(&[home, ".copilot", "agents"]),
            user_shadows_project: true,
        }),
        ".cursor" => Some(AgentArtifact {
            ext: ".md",
            user_dir: |home| jsp::join(&[home, ".cursor", "agents"]),
            user_shadows_project: false,
        }),
        _ => None,
    }
}

/// JS: providerAgentsUpToDate(bundleDir, root, provider, scope): every
/// bundled agent file exists at its destination with a matching hash.
fn provider_agents_up_to_date(
    bundle_dir: &str,
    root: &str,
    provider: &str,
    scope: Option<Scope>,
) -> Result<bool, String> {
    let Some(artifact) = agent_artifact(provider) else {
        return Ok(true);
    };
    let src_dir = jsp::join(&[bundle_dir, provider, "agents"]);
    if !util::exists(&src_dir) {
        return Ok(true);
    }
    let dest_dir = if scope == Some(Scope::User) {
        (artifact.user_dir)(root)
    } else {
        jsp::join(&[root, provider, "agents"])
    };
    for name in util::read_dir_names(&src_dir).unwrap_or_default() {
        if !name.ends_with(artifact.ext) {
            continue;
        }
        let local_path = jsp::join(&[&dest_dir, &name]);
        if !util::exists(&local_path) {
            return Ok(false);
        }
        if hash_skill_file(&jsp::join(&[&src_dir, &name]))? != hash_skill_file(&local_path)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// JS: copyProviderAgents(bundleDir, root, providers, {scope, home})
pub fn copy_provider_agents(
    sys: &Sys,
    bundle_dir: &str,
    root: &str,
    providers: &[&'static str],
    scope: Option<Scope>,
) -> Result<Vec<AgentResult>, String> {
    let mut results = Vec::new();
    for provider in providers {
        let Some(artifact) = agent_artifact(provider) else {
            continue;
        };
        let src_dir = jsp::join(&[bundle_dir, provider, "agents"]);
        if !util::exists(&src_dir) {
            continue;
        }
        let agent_files: Vec<String> = util::read_dir_names(&src_dir)
            .unwrap_or_default()
            .into_iter()
            .filter(|n| n.ends_with(artifact.ext))
            .collect();
        if agent_files.is_empty() {
            continue;
        }
        let dest_dir = if scope == Some(Scope::User) {
            (artifact.user_dir)(root)
        } else {
            jsp::join(&[root, provider, "agents"])
        };
        util::mkdir_p(&dest_dir)?;
        for name in &agent_files {
            util::write_bytes(
                &jsp::join(&[&dest_dir, name]),
                &util::read_bytes(&jsp::join(&[&src_dir, name]))?,
            )?;
        }
        let user_dir = (artifact.user_dir)(&sys.home);
        let shadowed: Vec<String> = if artifact.user_shadows_project && scope != Some(Scope::User) {
            agent_files
                .iter()
                .filter(|n| util::exists(&jsp::join(&[&user_dir, n])))
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        results.push(AgentResult {
            provider,
            written: agent_files.len(),
            dest_dir,
            user_dir,
            shadowed,
        });
    }
    Ok(results)
}

/// JS: reportProviderAgents(results)
pub fn report_provider_agents(sys: &Sys, io: &mut Io, results: &[AgentResult]) {
    for result in results {
        if result.written == 0 {
            continue;
        }
        io.out(&format!(
            "Installed {} agents into: {}\n",
            provider_display_name(result.provider),
            sys.format_path_for_display(&result.dest_dir)
        ));
        if !result.shadowed.is_empty() {
            io.err(&format!(
                "Warning: user-level agents in {} shadow the project copies just installed: {}.\n",
                sys.format_path_for_display(&result.user_dir),
                result.shadowed.join(", ")
            ));
            io.err("Run `phoenix-ui update --user --bundle-root DIR` to refresh them, or remove them so the project agents apply.\n");
        }
    }
}

pub struct LinkSource {
    pub checkout_root: String,
    pub bundle_root: String,
}

/// JS: resolveLinkSource(sourceValue, root)
pub fn resolve_link_source(source_value: Option<&str>, root: &str) -> Result<LinkSource, String> {
    let source_path = source_value.unwrap_or(".phoenix-ui");
    let checkout_root = if jsp::is_absolute(source_path) {
        source_path.to_string()
    } else {
        jsp::resolve(root, &[source_path])
    };
    let universal_root = jsp::join(&[&checkout_root, "dist", "universal"]);
    if util::exists(&universal_root) {
        return Ok(LinkSource {
            checkout_root,
            bundle_root: universal_root,
        });
    }
    if PROVIDER_DIRS
        .iter()
        .any(|p| util::exists(&jsp::join(&[&checkout_root, p, "skills"])))
    {
        return Ok(LinkSource {
            bundle_root: checkout_root.clone(),
            checkout_root,
        });
    }
    Err(format!("Could not find compiled skills in {source_path}. Expected dist/universal/ or provider skill folders."))
}

/// JS: isSymlinkTo(dest, expectedSource)
fn is_symlink_to(dest: &str, expected_source: &str) -> bool {
    if !util::is_symlink(dest) {
        return false;
    }
    let Some(target) = util::readlink(dest) else {
        return false;
    };
    let resolved_target = jsp::resolve(&jsp::dirname(dest), &[&target]);
    match (
        util::realpath(&resolved_target),
        util::realpath(expected_source),
    ) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

pub struct LinkResult {
    pub linked: usize,
    pub already: usize,
    pub skipped: usize,
}

/// JS: linkProviderSkills(bundleRoot, root, targets, {force})
pub fn link_provider_skills(
    io: &mut Io,
    bundle_root: &str,
    root: &str,
    targets: &[&str],
    force: bool,
) -> Result<LinkResult, String> {
    let mut result = LinkResult {
        linked: 0,
        already: 0,
        skipped: 0,
    };
    // JS: resolveUniqueLinkTargets
    let mut seen: Vec<String> = Vec::new();
    let mut unique: Vec<(&str, String)> = Vec::new();
    for provider in targets {
        let local_skills_dir = jsp::join(&[root, provider, "skills"]);
        util::mkdir_p(&local_skills_dir)?;
        let real = util::realpath(&local_skills_dir).unwrap_or_else(|| local_skills_dir.clone());
        if seen.contains(&real) {
            continue;
        }
        seen.push(real);
        unique.push((provider, local_skills_dir));
    }
    for (provider, local_skills_dir) in unique {
        let src_dir = jsp::join(&[bundle_root, provider, "skills"]);
        if !util::exists(&src_dir) {
            continue;
        }
        for skill in bundle_skill_dirs(&src_dir) {
            let src = jsp::join(&[&src_dir, &skill]);
            let dest = jsp::join(&[&local_skills_dir, &skill]);
            if util::exists_or_link(&dest) {
                if is_symlink_to(&dest, &src) {
                    result.already += 1;
                    continue;
                }
                if !force {
                    io.err(&format!("Skipped existing {provider}/skills/{skill}. Use --force to replace it with a link.\n"));
                    result.skipped += 1;
                    continue;
                }
                util::rm_rf(&dest);
            }
            let mut target = jsp::relative("/", &jsp::dirname(&dest), &src);
            if target.is_empty() {
                target = ".".to_string();
            }
            util::symlink_dir(&target, &dest)?;
            result.linked += 1;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_normalizes_provider_paths() {
        assert_eq!(
            normalize_for_hash("x .claude/skills/y .trae-cn/skills/z .agent/skills/"),
            "x .PROVIDER/skills/y .PROVIDER/skills/z .PROVIDER/skills/"
        );
        assert_eq!(normalize_for_hash(".other/skills/"), ".other/skills/");
    }
}
