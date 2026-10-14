//! Project installation over the verified bundle plan. This adds what the
//! bundle plan leaves to its caller: first-install legacy data and pins, the
//! derived runtime manifest, the five auxiliary skills, the transitional
//! `impeccable` entry, ownership carried forward from the active receipt and
//! read-only discovery of personal entries. All writes go through the kernel.
use crate::{
    classify::{Conflict, ConflictKind, Preserved},
    data::{self, Unconsumed},
    disk, plan, sha256, Change, Checkpoint, Error, ErrorKind, ExpectedFile, Manifest, Payload,
    Permissions, Plan, Project, Recovery, Result, RollbackPolicy,
};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub const AUXILIARY_SKILLS: [&str; 5] = [
    "apply-ui-review",
    "design-md",
    "design-reference",
    "review-ui",
    "ui-pattern-research",
];
const RUNTIME_MANIFEST: &str = "skills/phoenix-ui/scripts/ENGINE.json";
const MAIN_SKILL: &str = "skills/phoenix-ui/SKILL.md";
const TRANSITION: &str = include_str!("../templates/impeccable-transition.md");
const AUXILIARY_FILE_LIMIT: u64 = 1024 * 1024;
const AUXILIARY_FILES: usize = 200;

pub struct Request<'a> {
    pub project: &'a Path,
    pub bundle_root: &'a Path,
    /// Directory holding the five auxiliary skill directories, if installing them.
    pub auxiliary_root: Option<&'a Path>,
    /// Bundle provider directories, e.g. `.agents`, `.claude`, `.codex`.
    pub providers: Vec<String>,
    /// Home directory for read-only personal discovery; `None` skips it.
    pub home: Option<PathBuf>,
    pub dry_run: bool,
    pub transaction_id: String,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Planned,
    Installed,
    Unchanged,
    Conflict,
    RecoveryRequired,
    /// Phoenix is active, but an external auxiliary or personal copy differs.
    IntegrationPending,
    RolledBack,
    Recovered,
    Clean,
}

impl Status {
    pub fn exit_code(self) -> i32 {
        match self {
            Status::Conflict | Status::RecoveryRequired | Status::IntegrationPending => 3,
            _ => 0,
        }
    }
}

/// A non-blocking fact that still keeps the integration from being complete,
/// or a note that an identical external copy was left untouched.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub path: String,
    pub kind: &'static str,
    pub blocking_integration: bool,
    pub sha256: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub schema_version: u32,
    pub mode: &'static str,
    pub status: Status,
    pub project: String,
    pub transaction_id: Option<String>,
    pub package_version: Option<String>,
    pub providers: Vec<String>,
    pub recovered: Option<String>,
    pub writes: Vec<String>,
    pub shared_json: Vec<String>,
    pub removals: Vec<String>,
    pub backups: Option<String>,
    pub conflicts: Vec<Conflict>,
    pub findings: Vec<Finding>,
    pub preserved: Vec<Preserved>,
    pub unconsumed: Vec<Unconsumed>,
    pub migration_sources: Vec<String>,
    pub transition_entries: Vec<String>,
}

impl Report {
    fn new(mode: &'static str, status: Status, project: &Path) -> Self {
        Report {
            schema_version: 1,
            mode,
            status,
            project: project.display().to_string(),
            transaction_id: None,
            package_version: None,
            providers: Vec::new(),
            recovered: None,
            writes: Vec::new(),
            shared_json: Vec::new(),
            removals: Vec::new(),
            backups: None,
            conflicts: Vec::new(),
            findings: Vec::new(),
            preserved: Vec::new(),
            unconsumed: Vec::new(),
            migration_sources: Vec::new(),
            transition_entries: Vec::new(),
        }
    }
}

struct Version {
    bytes: Vec<u8>,
    bundle_sha256: String,
    package_version: String,
    providers_with_skill: BTreeSet<String>,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidPlan, message)
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path).map_err(|e| Error::io(path, e))?;
    if meta.is_symlink() || !meta.is_file() || meta.len() > limit {
        return Err(invalid(format!(
            "requires a bounded regular file: {}",
            path.display()
        )));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| Error::io(path, e))?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::io(path, e))?;
    if bytes.len() as u64 != meta.len() {
        return Err(invalid(format!(
            "file changed while reading: {}",
            path.display()
        )));
    }
    Ok(bytes)
}

/// VERSION.json sits next to the bundle; ENGINE.json in each installed skill is
/// a byte copy, so an installed project can later find its pinned runtime.
fn read_version(bundle_root: &Path) -> Result<Version> {
    let parent = bundle_root
        .parent()
        .ok_or_else(|| invalid("bundle root has no adjacent VERSION.json"))?;
    let bytes = read_bounded(&parent.join("VERSION.json"), 1024 * 1024)?;
    let version: Value = serde_json::from_slice(&bytes)
        .map_err(|e| invalid(format!("invalid VERSION.json: {e}")))?;
    let text = |key: &str| {
        version
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    if text("toolVersion").as_deref() != Some(env!("CARGO_PKG_VERSION"))
        || version.get("bundleSchema").and_then(Value::as_u64) != Some(1)
    {
        return Err(invalid(
            "VERSION.json does not match this Phoenix runtime and bundle schema",
        ));
    }
    let bundle_sha256 =
        text("bundleSha256").ok_or_else(|| invalid("VERSION.json has no bundleSha256"))?;
    let package_version = text("npmPackageVersion")
        .ok_or_else(|| invalid("VERSION.json has no npmPackageVersion"))?;
    let manifest_bytes = read_bounded(&bundle_root.join("manifest.json"), 16 * 1024 * 1024)?;
    if sha256(&manifest_bytes) != bundle_sha256 {
        return Err(invalid(
            "bundle manifest SHA-256 does not match VERSION.json",
        ));
    }
    let manifest: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| invalid(format!("invalid bundle manifest: {e}")))?;
    let providers_with_skill = manifest
        .get("providers")
        .and_then(Value::as_object)
        .map(|providers| {
            providers
                .iter()
                .filter(|(_, p)| {
                    p.pointer(&format!("/files/{}", MAIN_SKILL.replace('/', "~1")))
                        .is_some()
                })
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default();
    Ok(Version {
        bytes,
        bundle_sha256,
        package_version,
        providers_with_skill,
    })
}

fn transition_payload(provider: &str) -> Payload {
    let prefix = if provider == ".agents" { "$" } else { "/" };
    Payload::new(
        TRANSITION.replace("{{prefix}}", prefix).into_bytes(),
        Permissions::file(0o644),
    )
}

fn current(project: &Project, path: &str) -> Result<Option<disk::Snapshot>> {
    disk::read(project.root(), path)
}

fn write_change(path: String, expected: ExpectedFile, payload: Payload) -> Change {
    Change::Exclusive {
        path,
        expected,
        after: Some(payload),
        rollback: RollbackPolicy::Restore,
    }
}

fn is_noop(change: &Change) -> bool {
    match change {
        Change::Exclusive {
            expected: ExpectedFile::Hash(hash),
            after: Some(payload),
            ..
        } => *hash == payload.sha256,
        _ => false,
    }
}

/// Load the auxiliary skill trees as exact bytes. Symlinks and oversized trees
/// are rejected; the package copy is the source of truth for every provider.
fn load_auxiliary(root: &Path) -> Result<BTreeMap<String, BTreeMap<String, Payload>>> {
    let mut skills = BTreeMap::new();
    for skill in AUXILIARY_SKILLS {
        let mut files = BTreeMap::new();
        let mut stack = vec![(root.join(skill), String::new())];
        while let Some((dir, prefix)) = stack.pop() {
            let meta = fs::symlink_metadata(&dir).map_err(|e| Error::io(&dir, e))?;
            if !meta.is_dir() || meta.is_symlink() {
                return Err(invalid(format!(
                    "auxiliary skill requires plain directories: {}",
                    dir.display()
                )));
            }
            for entry in fs::read_dir(&dir).map_err(|e| Error::io(&dir, e))? {
                let entry = entry.map_err(|e| Error::io(&dir, e))?;
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| invalid("auxiliary skill path is not UTF-8"))?;
                let relative = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}/{name}")
                };
                let kind = entry.file_type().map_err(|e| Error::io(&entry.path(), e))?;
                if kind.is_dir() {
                    stack.push((entry.path(), relative));
                } else {
                    let bytes = read_bounded(&entry.path(), AUXILIARY_FILE_LIMIT)?;
                    files.insert(relative, Payload::new(bytes, Permissions::file(0o644)));
                    if files.len() > AUXILIARY_FILES {
                        return Err(invalid(format!(
                            "auxiliary skill has too many files: {skill}"
                        )));
                    }
                }
            }
        }
        if !files.contains_key("SKILL.md") {
            return Err(invalid(format!("auxiliary skill has no SKILL.md: {skill}")));
        }
        skills.insert(skill.to_string(), files);
    }
    Ok(skills)
}

/// Every file below `full` (following a linked root) with its hash; `None`
/// when absent. Oversized or excess entries hash as empty and never match.
fn tree_hashes(full: &Path) -> Result<Option<BTreeMap<String, String>>> {
    let full = full.to_path_buf();
    match fs::symlink_metadata(&full) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::io(&full, e)),
        Ok(_) => {}
    }
    let mut out = BTreeMap::new();
    let mut stack = vec![(full, String::new())];
    while let Some((dir, prefix)) = stack.pop() {
        for entry in fs::read_dir(&dir).map_err(|e| Error::io(&dir, e))? {
            let entry = entry.map_err(|e| Error::io(&dir, e))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let relative = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            let meta = fs::metadata(entry.path()).map_err(|e| Error::io(&entry.path(), e))?;
            if meta.is_dir() {
                stack.push((entry.path(), relative));
            } else if out.len() < AUXILIARY_FILES * 2 && meta.len() <= AUXILIARY_FILE_LIMIT {
                let bytes = fs::read(entry.path()).map_err(|e| Error::io(&entry.path(), e))?;
                out.insert(relative, sha256(&bytes));
            } else {
                out.insert(relative, String::new());
            }
        }
    }
    Ok(Some(out))
}

fn package_hashes(files: &BTreeMap<String, Payload>) -> BTreeMap<String, String> {
    files
        .iter()
        .map(|(path, payload)| (path.clone(), payload.sha256.clone()))
        .collect()
}

/// Read-only personal discovery for the selected providers. Old main entries
/// and design hooks there would run beside Phoenix, so they block activation;
/// auxiliary copies only keep the integration from being complete.
fn discover_personal(
    home: &Path,
    project: &Path,
    providers: &[String],
    auxiliary: Option<&BTreeMap<String, BTreeMap<String, Payload>>>,
    conflicts: &mut Vec<Conflict>,
    findings: &mut Vec<Finding>,
) {
    let mut skill_roots = Vec::new();
    let mut hook_files = Vec::new();
    for provider in providers {
        match provider.as_str() {
            ".claude" => {
                skill_roots.push((".claude/skills", true));
                hook_files.push(".claude/settings.json");
                hook_files.push(".claude/settings.local.json");
            }
            ".agents" | ".codex" => {
                skill_roots.push((".agents/skills", false));
                skill_roots.push((".codex/skills", false));
                hook_files.push(".codex/hooks.json");
            }
            ".cursor" => {
                skill_roots.push((".cursor/skills", false));
                hook_files.push(".cursor/hooks.json");
            }
            _ => {}
        }
    }
    skill_roots.sort();
    skill_roots.dedup();
    hook_files.sort();
    hook_files.dedup();
    let display = |relative: &str| home.join(relative).display().to_string();
    let locked_upstream = skills_sh_upstream_skills(home);
    for (root, overrides_project) in skill_roots {
        let old = home.join(root).join("impeccable");
        if fs::symlink_metadata(&old).is_ok() {
            conflicts.push(Conflict {
                path: old.display().to_string(),
                provider: None,
                kind: ConflictKind::PersonalEntry,
                message: "personal Impeccable skill would run beside Phoenix; back it up and remove or disable it from the personal skills directory, reload the agent, then rerun the install".into(),
            });
        }
        findings.extend(personal_old_commands(
            home,
            root,
            auxiliary,
            &locked_upstream,
        ));
        let Some(auxiliary) = auxiliary else { continue };
        for (skill, files) in auxiliary {
            let relative = format!("{root}/{skill}");
            let Ok(Some(found)) = tree_hashes(&home.join(&relative)) else {
                continue;
            };
            let same = found == package_hashes(files);
            findings.push(Finding {
                path: display(&relative),
                kind: if same { "personal-copy-identical" } else { "personal-copy-differs" },
                blocking_integration: !same,
                sha256: Some(sha256(serde_json::to_string(&found).unwrap_or_default().as_bytes())),
                message: if same {
                    "identical personal copy; nothing to change".into()
                } else if overrides_project {
                    "Claude loads this personal copy instead of the project copy; update or remove it, then rerun the install".into()
                } else {
                    "personal copy differs and its precedence over the project copy is not established; verify the loaded path in a new session".into()
                },
            });
        }
    }
    for file in hook_files {
        let path = home.join(file);
        let Ok(bytes) = read_bounded(&path, 4 * 1024 * 1024) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        if text.contains("skills/impeccable/scripts") || text.contains("impeccable hook") {
            conflicts.push(Conflict {
                path: path.display().to_string(),
                provider: None,
                kind: ConflictKind::PersonalEntry,
                message: "personal hook configuration runs the old Impeccable design hook; remove those handlers (keep the rest), reload the agent, then rerun the install".into(),
            });
        }
    }
    // Claude loads a plugin by the `enabledPlugins` value of the most specific
    // settings file naming it (local, then project, then user). The install
    // registry only records installs, which stay when a plugin is disabled.
    let mut decided: BTreeMap<String, (bool, PathBuf)> = BTreeMap::new();
    for path in [
        home.join(".claude/settings.json"),
        project.join(".claude/settings.json"),
    ]
    .into_iter()
    .chain(claude_local_settings(home, project))
    {
        let Ok(bytes) = read_bounded(&path, 4 * 1024 * 1024) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        for (id, state) in value
            .get("enabledPlugins")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter(|(id, _)| id.starts_with("impeccable"))
        {
            decided.insert(id.clone(), (state != &Value::Bool(false), path.clone()));
        }
    }
    let enabled_in: BTreeSet<_> = decided
        .into_values()
        .filter_map(|(enabled, path)| enabled.then_some(path))
        .collect();
    for path in enabled_in {
        conflicts.push(Conflict {
            path: path.display().to_string(),
            provider: None,
            kind: ConflictKind::PersonalEntry,
            message: "an Impeccable plugin is enabled by this settings file; disable it (/plugin, or false here), reload the agent, then rerun the install".into(),
        });
    }
}

/// The `.claude/settings.local.json` files Claude reads for a session started
/// in `project`, lowest precedence first. In a repository Claude keeps the file
/// at the checkout root, the main checkout's root from a linked worktree, and
/// still reads one an older version left in the starting directory below it.
fn claude_local_settings(home: &Path, project: &Path) -> Vec<PathBuf> {
    let started = project.join(".claude/settings.local.json");
    match claude_local_root(home, project) {
        Some(root) if root != project => vec![started, root.join(".claude/settings.local.json")],
        _ => vec![started],
    }
}

/// Outside git, on Windows, in a worktree without a main checkout, at a home
/// directory root, or when the root, its `.git` or its `.claude` belongs to
/// another user, Claude keeps the local file in the starting directory.
fn claude_local_root(home: &Path, project: &Path) -> Option<PathBuf> {
    if cfg!(windows) {
        return None;
    }
    let (top, git_dir) = enclosing_git(project)?;
    let root = match fs::read_to_string(git_dir.join("commondir")) {
        Ok(common) => {
            let common = PathBuf::from(impeccable_common::jsp::resolve(
                &git_dir.to_string_lossy(),
                &[common.trim()],
            ));
            if common.file_name()? != ".git" {
                return None;
            }
            common.parent()?.to_path_buf()
        }
        Err(_) => top,
    };
    if root == fs::canonicalize(home).unwrap_or_else(|_| home.to_path_buf()) {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // getuid always succeeds and touches no memory.
        let uid = unsafe { libc::getuid() };
        if [root.clone(), root.join(".git"), root.join(".claude")]
            .iter()
            .any(|path| fs::symlink_metadata(path).is_ok_and(|meta| meta.uid() != uid))
        {
            return None;
        }
    }
    Some(root)
}

/// The checkout holding the nearest `.git` above `dir`, and the git directory
/// that entry is or names.
fn enclosing_git(dir: &Path) -> Option<(PathBuf, PathBuf)> {
    let top = dir.ancestors().find(|d| d.join(".git").exists())?;
    let dot_git = top.join(".git");
    if dot_git.is_dir() {
        return Some((top.to_path_buf(), dot_git));
    }
    let text = fs::read_to_string(&dot_git).ok()?;
    let git_dir = top.join(text.trim().strip_prefix("gitdir:")?.trim());
    Some((top.to_path_buf(), git_dir))
}

/// Global skills that skills.sh recorded as installed from the upstream repository.
fn skills_sh_upstream_skills(home: &Path) -> Vec<String> {
    let Ok(bytes) = read_bounded(&home.join(".agents/.skill-lock.json"), 4 * 1024 * 1024) else {
        return Vec::new();
    };
    let Ok(lock) = serde_json::from_slice::<Value>(&bytes) else {
        return Vec::new();
    };
    lock.get("skills")
        .and_then(Value::as_object)
        .map(|skills| {
            skills
                .iter()
                .filter(|(_, entry)| {
                    entry
                        .get("source")
                        .and_then(Value::as_str)
                        .is_some_and(|source| source.contains("pbakaus/impeccable"))
                })
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Older upstream command sets (split skills such as `polish` next to
/// `teach-impeccable`) carry no hooks or engine, so they do not block Phoenix;
/// they stay visible beside it until the user removes them.
fn personal_old_commands(
    home: &Path,
    root: &str,
    auxiliary: Option<&BTreeMap<String, BTreeMap<String, Payload>>>,
    locked_upstream: &[String],
) -> Vec<Finding> {
    let Ok(entries) = fs::read_dir(home.join(root)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            name != "impeccable"
                && name != "phoenix-ui"
                && !auxiliary.is_some_and(|skills| skills.contains_key(name))
        })
        .collect();
    names.sort();
    names
        .into_iter()
        .filter_map(|name| {
            let dir = home.join(root).join(&name);
            let text = read_bounded(&dir.join("SKILL.md"), 1024 * 1024)
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .ok()?;
            let upstream = locked_upstream.contains(&name)
                || text.contains("teach-impeccable")
                || text.contains(".impeccable.md");
            upstream.then(|| Finding {
                path: dir.display().to_string(),
                kind: "personal-old-command",
                blocking_integration: false,
                sha256: Some(sha256(text.as_bytes())),
                message: format!(
                    "older upstream Impeccable command `{name}` stays visible beside Phoenix; back it up and remove it from the personal skills directory (or with the tool that installed it), then reload the agent"
                ),
            })
        })
        .collect()
}

/// Project instructions that still point at the old entry. They are reported
/// with a suggested replacement and never edited.
fn document_mentions(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for name in ["AGENTS.md", "CLAUDE.md", "DESIGN.md", "PRODUCT.md"] {
        let Ok(full) = disk::checked(root, name) else {
            continue;
        };
        let Ok(bytes) = read_bounded(&full, 4 * 1024 * 1024) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<String> = text
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains("impeccable"))
            .map(|(i, _)| (i + 1).to_string())
            .collect();
        if !lines.is_empty() {
            findings.push(Finding {
                path: name.into(),
                kind: "document-old-entry",
                blocking_integration: false,
                sha256: Some(sha256(&bytes)),
                message: format!(
                    "line(s) {} mention the old impeccable entry; replace /impeccable or $impeccable with the phoenix-ui command yourself (left unchanged)",
                    lines.join(", ")
                ),
            });
        }
    }
    findings
}

/// Remove directories left empty below `{provider}/{skills|agents|...}/<entry>`
/// by this transaction. `remove_dir` never removes a directory with content,
/// and a later restore recreates any parent it needs.
fn prune_empty(root: &Path, paths: impl IntoIterator<Item = String>) {
    let mut dirs = BTreeSet::new();
    for path in paths {
        if path.starts_with(".phoenix-ui/") {
            continue;
        }
        let mut parts: Vec<&str> = path.split('/').collect();
        parts.pop();
        while parts.len() >= 3 {
            dirs.insert(parts.join("/"));
            parts.pop();
        }
    }
    let mut ordered: Vec<_> = dirs.into_iter().collect();
    ordered.sort_by_key(|d| std::cmp::Reverse(d.matches('/').count()));
    for dir in ordered {
        if let Ok(full) = disk::checked(root, &dir) {
            let _ = fs::remove_dir(full);
        }
    }
}

const EXCLUDE_OPEN: &str = "# phoenix-ui-install-ignore-start";
const EXCLUDE_CLOSE: &str = "# phoenix-ui-install-ignore-end";

/// Backups, the lock and the migrated local configuration are machine-local.
/// Keep them in the enclosing repository's `.git/info/exclude`, replacing a
/// block an earlier version wrote; never edit a committed ignore file. Best
/// effort: a project outside git, or an unwritable exclude, keeps going.
fn exclude_local_state(root: &Path) {
    let Some((_, git_dir)) = enclosing_git(root) else {
        return;
    };
    let target = PathBuf::from(impeccable_common::git::info_exclude(
        &git_dir.to_string_lossy(),
    ));
    let existing = fs::read_to_string(&target).unwrap_or_default();
    let block = format!(
        "{EXCLUDE_OPEN}\n**/.phoenix-ui/migrations/\n**/.phoenix-ui/install/install.lock\n**/.phoenix-ui/config.local.json\n{EXCLUDE_CLOSE}\n"
    );
    let updated = match existing.find(EXCLUDE_OPEN).and_then(|start| {
        existing[start..]
            .find(EXCLUDE_CLOSE)
            .map(|end| (start, start + end + EXCLUDE_CLOSE.len()))
    }) {
        Some((start, end)) => {
            let end = if existing[end..].starts_with('\n') {
                end + 1
            } else {
                end
            };
            format!("{}{block}{}", &existing[..start], &existing[end..])
        }
        None => {
            let separator = if existing.is_empty() || existing.ends_with('\n') {
                ""
            } else {
                "\n"
            };
            format!("{existing}{separator}{block}")
        }
    };
    if updated == existing {
        return;
    }
    if let Some(parent) = target.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&target, updated);
}

fn journal_paths(root: &Path, transaction_id: &str) -> Vec<String> {
    let Ok(full) = disk::checked(
        root,
        &format!(".phoenix-ui/install/journals/{transaction_id}.json"),
    ) else {
        return Vec::new();
    };
    let Ok(bytes) = read_bounded(&full, 64 * 1024 * 1024) else {
        return Vec::new();
    };
    let Ok(journal) = serde_json::from_slice::<Value>(&bytes) else {
        return Vec::new();
    };
    journal
        .get("operations")
        .and_then(Value::as_array)
        .map(|ops| {
            ops.iter()
                .filter_map(|op| op.get("path").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn sorted_unique(values: impl IntoIterator<Item = String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Plan, and unless `dry_run`, apply the project installation.
pub fn install(request: &Request) -> Result<Report> {
    install_with(request, |_| {})
}

/// `install` with an observer of the kernel's apply checkpoints.
pub fn install_with(request: &Request, observer: impl FnMut(Checkpoint)) -> Result<Report> {
    let mode = if request.dry_run { "dry-run" } else { "apply" };
    let project = Project::open(request.project)?;
    let mut report = Report::new(mode, Status::Planned, project.root());
    report.providers = request.providers.clone();
    report.transaction_id = Some(request.transaction_id.clone());
    let version = read_version(request.bundle_root)?;
    report.package_version = Some(version.package_version.clone());
    if !request.dry_run {
        match project.recover() {
            Ok(Recovery::Clean) => {}
            Ok(Recovery::RolledBack(id)) => {
                prune_empty(project.root(), journal_paths(project.root(), &id));
                report.recovered = Some(format!("rolled-back:{id}"));
            }
            Ok(Recovery::Activated(id)) => report.recovered = Some(format!("activated:{id}")),
            Err(error) if error.kind == ErrorKind::Conflict => {
                report.status = Status::RecoveryRequired;
                report.conflicts.push(Conflict {
                    path: ".phoenix-ui/install".into(),
                    provider: None,
                    kind: ConflictKind::CustomizedFile,
                    message: error.message,
                });
                return Ok(report);
            }
            Err(error) => return Err(error),
        }
    }
    let active = match project.verified_installation() {
        Ok(active) => active,
        Err(error) if error.kind == ErrorKind::RecoveryRequired => {
            report.status = Status::RecoveryRequired;
            report.conflicts.push(Conflict {
                path: ".phoenix-ui/install".into(),
                provider: None,
                kind: ConflictKind::CustomizedFile,
                message: format!(
                    "{}; rerun without --dry-run to recover first",
                    error.message
                ),
            });
            return Ok(report);
        }
        Err(error) if error.kind == ErrorKind::Conflict => None,
        Err(error) => return Err(error),
    };
    let carried_sources = active
        .as_ref()
        .map(|a| a.receipt.manifest.migration_sources.clone())
        .unwrap_or_default();
    let manifest = Manifest {
        version: version.package_version.clone(),
        sha256: version.bundle_sha256.clone(),
        providers: request.providers.clone(),
        migration_sources: carried_sources.clone(),
    };
    let prepared = match plan::build_install_plan_with(
        &project,
        request.bundle_root,
        &request.providers,
        manifest,
        &request.transaction_id,
        true,
    ) {
        Ok(prepared) => prepared,
        Err(error) if error.kind == ErrorKind::RecoveryRequired => {
            report.status = Status::RecoveryRequired;
            report.conflicts.push(Conflict {
                path: ".phoenix-ui/install".into(),
                provider: None,
                kind: ConflictKind::CustomizedFile,
                message: error.message,
            });
            return Ok(report);
        }
        Err(error) => return Err(error),
    };
    report.conflicts.extend(prepared.conflicts.iter().cloned());
    report.preserved.extend(prepared.preserved.iter().cloned());
    let auxiliary = request.auxiliary_root.map(load_auxiliary).transpose()?;
    report.findings.extend(document_mentions(project.root()));
    if let Some(home) = &request.home {
        discover_personal(
            home,
            project.root(),
            &request.providers,
            auxiliary.as_ref(),
            &mut report.conflicts,
            &mut report.findings,
        );
    }
    let legacy_sources = sorted_unique(
        prepared
            .inventory
            .exclusive
            .iter()
            .flat_map(|f| f.sources.iter())
            .chain(
                prepared
                    .inventory
                    .shared
                    .iter()
                    .flat_map(|s| s.handlers.iter().flat_map(|h| h.sources.iter())),
            )
            .map(|s| s.package_version.clone()),
    );
    report.migration_sources = sorted_unique(carried_sources.into_iter().chain(legacy_sources));
    // Legacy data moves once, on the install that replaces the old tool.
    let legacy = if active.is_none() {
        let data = data::prepare_legacy_data(&project, &prepared.inventory)?;
        report.conflicts.extend(
            data.conflicts
                .iter()
                .filter(|c| {
                    !report
                        .conflicts
                        .iter()
                        .any(|r| r.path == c.path && r.message == c.message)
                })
                .cloned()
                .collect::<Vec<_>>(),
        );
        report.unconsumed = data.unconsumed.clone();
        let rebuilt: BTreeSet<_> = data
            .resolved_pin_paths
            .iter()
            .map(|p| p.old_path.clone())
            .collect();
        report.preserved.retain(|p| !rebuilt.contains(&p.path));
        Some(data)
    } else {
        None
    };
    if !report.conflicts.is_empty() {
        report.status = Status::Conflict;
        return Ok(report);
    }
    let Some(mut base) = prepared.plan else {
        return Err(invalid(
            "bundle plan is missing without a reported conflict",
        ));
    };
    let mut changes: BTreeMap<String, Change> = BTreeMap::new();
    for change in base.changes.drain(..) {
        changes.insert(change.path().to_string(), change);
    }
    let mut extra_noop = true;
    let mut add = |changes: &mut BTreeMap<String, Change>, change: Change| -> Result<()> {
        extra_noop &= is_noop(&change);
        let path = change.path().to_string();
        if changes.insert(path.clone(), change).is_some() {
            return Err(Error::conflict(format!(
                "two installation changes target {path}"
            )));
        }
        Ok(())
    };
    if let Some(data) = &legacy {
        for change in &data.changes {
            add(&mut changes, change.clone())?;
        }
    }
    let owned: BTreeMap<String, (Option<String>, RollbackPolicy)> = active
        .as_ref()
        .map(|a| {
            a.receipt
                .ownership
                .iter()
                .filter(|o| !o.shared_entries)
                .map(|o| (o.path.clone(), (o.sha256.clone(), o.rollback)))
                .collect()
        })
        .unwrap_or_default();
    let owned_with = |path: &str, hash: &str| {
        owned
            .get(path)
            .is_some_and(|(sha, _)| sha.as_deref() == Some(hash))
    };
    // Derived runtime manifest for each selected provider that carries the skill.
    for provider in request
        .providers
        .iter()
        .filter(|p| version.providers_with_skill.contains(*p))
    {
        let path = format!("{provider}/{RUNTIME_MANIFEST}");
        let payload = Payload::new(version.bytes.clone(), Permissions::file(0o644));
        let expected = match current(&project, &path)? {
            None => ExpectedFile::Absent,
            Some(snapshot) if owned_with(&path, &snapshot.state.hash) => {
                ExpectedFile::Hash(snapshot.state.hash)
            }
            Some(_) => {
                report.conflicts.push(Conflict {
                    path: path.clone(),
                    provider: Some(provider.clone()),
                    kind: ConflictKind::CustomizedFile,
                    message: "existing runtime manifest has no verified receipt ownership; preserve it and resolve manually".into(),
                });
                continue;
            }
        };
        add(&mut changes, write_change(path, expected, payload))?;
    }
    // The transitional entry replaces a retired old skill, or keeps its own ownership.
    for provider in request
        .providers
        .iter()
        .filter(|p| version.providers_with_skill.contains(*p))
    {
        let path = format!("{provider}/skills/impeccable/SKILL.md");
        let payload = transition_payload(provider);
        let expected = match changes.get(&path) {
            Some(Change::Exclusive {
                expected: ExpectedFile::Hash(old),
                after: None,
                ..
            }) => ExpectedFile::Hash(old.clone()),
            Some(_) => continue,
            None => match current(&project, &path)? {
                Some(snapshot) if owned_with(&path, &snapshot.state.hash) => {
                    ExpectedFile::Hash(snapshot.state.hash)
                }
                _ => continue,
            },
        };
        changes.remove(&path);
        report.transition_entries.push(path.clone());
        add(&mut changes, write_change(path, expected, payload))?;
    }
    if let Some(auxiliary) = &auxiliary {
        for provider in request
            .providers
            .iter()
            .filter(|p| version.providers_with_skill.contains(*p))
        {
            for (skill, files) in auxiliary {
                let dir = format!("{provider}/skills/{skill}");
                let full =
                    disk::checked(project.root(), &format!("{provider}/skills"))?.join(skill);
                let link = fs::symlink_metadata(&full).is_ok_and(|m| m.is_symlink());
                let found = tree_hashes(&full)?;
                let package = package_hashes(files);
                let all_owned = found.as_ref().is_some_and(|found| {
                    !found.is_empty()
                        && found
                            .iter()
                            .all(|(file, hash)| owned_with(&format!("{dir}/{file}"), hash))
                });
                match found {
                    Some(found) if link || !all_owned => {
                        let same = found == package;
                        report.findings.push(Finding {
                            path: dir.clone(),
                            kind: match (link, same) {
                                (true, true) => "external-symlink-identical",
                                (true, false) => "external-symlink-differs",
                                (false, true) => "external-copy-identical",
                                (false, false) => "external-copy-differs",
                            },
                            blocking_integration: !same,
                            sha256: Some(sha256(serde_json::to_string(&found).unwrap_or_default().as_bytes())),
                            message: if same {
                                "externally managed identical copy left untouched; Phoenix does not take ownership".into()
                            } else {
                                format!("externally managed copy differs and is kept; update it (for example `npx skills add XGENT-ai/skills --skill {skill}`) or back it up and move it out of {dir}, then rerun the install")
                            },
                        });
                    }
                    found => {
                        let found = found.unwrap_or_default();
                        for (file, payload) in files {
                            let path = format!("{dir}/{file}");
                            let expected = match found.get(file) {
                                Some(hash) => ExpectedFile::Hash(hash.clone()),
                                None => ExpectedFile::Absent,
                            };
                            add(&mut changes, write_change(path, expected, payload.clone()))?;
                        }
                        for (file, hash) in
                            found.iter().filter(|(file, _)| !files.contains_key(*file))
                        {
                            add(
                                &mut changes,
                                Change::Exclusive {
                                    path: format!("{dir}/{file}"),
                                    expected: ExpectedFile::Hash(hash.clone()),
                                    after: None,
                                    rollback: RollbackPolicy::Restore,
                                },
                            )?;
                        }
                    }
                }
            }
        }
    }
    // Carry forward ownership the bundle plan does not cover (pins, migrated data,
    // auxiliary files installed without --auxiliary-root this time).
    for (path, (sha, rollback)) in &owned {
        if changes.contains_key(path) || sha.is_none() {
            continue;
        }
        match current(&project, path)? {
            Some(snapshot)
                if Some(&snapshot.state.hash) == sha.as_ref()
                    || *rollback == RollbackPolicy::Retain =>
            {
                let payload =
                    Payload::new(snapshot.bytes.clone(), snapshot.state.permissions.clone());
                add(
                    &mut changes,
                    Change::Exclusive {
                        path: path.clone(),
                        expected: ExpectedFile::Hash(snapshot.state.hash),
                        after: Some(payload),
                        rollback: *rollback,
                    },
                )?;
            }
            Some(_) => report.conflicts.push(Conflict {
                path: path.clone(),
                provider: None,
                kind: ConflictKind::CustomizedFile,
                message:
                    "a Phoenix-owned file was modified; keep the change and resolve it manually"
                        .into(),
            }),
            None => report.preserved.push(Preserved {
                path: path.clone(),
                reason: "previously owned file is gone; ownership is not carried forward".into(),
            }),
        }
    }
    if !report.conflicts.is_empty() {
        report.status = Status::Conflict;
        return Ok(report);
    }
    let pending = report.findings.iter().any(|f| f.blocking_integration);
    let mut manifest = base.manifest.clone();
    manifest.migration_sources = report.migration_sources.clone();
    let unchanged = prepared.already_installed
        && extra_noop
        && active.as_ref().is_some_and(|a| {
            a.receipt.manifest == manifest
                && a.receipt
                    .ownership
                    .iter()
                    .filter(|o| !o.shared_entries && o.sha256.is_some())
                    .count()
                    == changes
                        .values()
                        .filter(|c| matches!(c, Change::Exclusive { .. }))
                        .count()
        });
    if unchanged {
        if !request.dry_run {
            exclude_local_state(project.root());
        }
        report.status = if pending {
            Status::IntegrationPending
        } else {
            Status::Unchanged
        };
        return Ok(report);
    }
    let plan = Plan {
        transaction_id: base.transaction_id,
        manifest,
        expected_receipt: base.expected_receipt,
        changes: changes.into_values().collect(),
    };
    let preflight = match project.preflight(&plan) {
        Ok(preflight) => preflight,
        Err(error) if error.kind == ErrorKind::Conflict => {
            report.conflicts.push(Conflict {
                path: "project preflight".into(),
                provider: None,
                kind: ConflictKind::CustomizedFile,
                message: error.message,
            });
            report.status = Status::Conflict;
            return Ok(report);
        }
        Err(error) => return Err(error),
    };
    report.writes = preflight.writes;
    report.shared_json = preflight.shared_json;
    report.removals = preflight.removals;
    report.backups = Some(format!(".phoenix-ui/migrations/{}", plan.transaction_id));
    if request.dry_run {
        report.status = Status::Planned;
        return Ok(report);
    }
    project.apply_with(&plan, observer)?;
    prune_empty(project.root(), report.removals.iter().cloned());
    exclude_local_state(project.root());
    report.status = if pending {
        Status::IntegrationPending
    } else {
        Status::Installed
    };
    Ok(report)
}

pub fn rollback(project: &Path, transaction_id: &str) -> Result<Report> {
    let project = Project::open(project)?;
    let mut report = Report::new("rollback", Status::RolledBack, project.root());
    report.transaction_id = Some(transaction_id.into());
    let touched = journal_paths(project.root(), transaction_id);
    match project.rollback(transaction_id) {
        Ok(()) => {
            prune_empty(project.root(), touched);
            Ok(report)
        }
        Err(error) if error.kind == ErrorKind::Conflict => {
            report.status = Status::Conflict;
            report.conflicts.push(Conflict {
                path: format!(".phoenix-ui/migrations/{transaction_id}"),
                provider: None,
                kind: ConflictKind::CustomizedFile,
                message: error.message,
            });
            Ok(report)
        }
        Err(error) => Err(error),
    }
}

pub fn recover(project: &Path) -> Result<Report> {
    let project = Project::open(project)?;
    let mut report = Report::new("recover", Status::Clean, project.root());
    match project.recover() {
        Ok(Recovery::Clean) => {}
        Ok(Recovery::RolledBack(id)) => {
            prune_empty(project.root(), journal_paths(project.root(), &id));
            report.status = Status::Recovered;
            report.recovered = Some(format!("rolled-back:{id}"));
        }
        Ok(Recovery::Activated(id)) => {
            report.status = Status::Recovered;
            report.recovered = Some(format!("activated:{id}"));
        }
        Err(error) if error.kind == ErrorKind::Conflict => {
            report.status = Status::RecoveryRequired;
            report.conflicts.push(Conflict {
                path: ".phoenix-ui/install".into(),
                provider: None,
                kind: ConflictKind::CustomizedFile,
                message: error.message,
            });
        }
        Err(error) => return Err(error),
    }
    Ok(report)
}
