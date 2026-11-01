//! Project installation over the verified bundle plan. This adds what the
//! bundle plan leaves to its caller: first-install legacy data and pins, the
//! derived runtime manifest, the transitional
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

const LEGACY_AUXILIARY_SKILLS: [&str; 5] = [
    "apply-ui-review",
    "design-md",
    "design-reference",
    "review-ui",
    "ui-pattern-research",
];
const RUNTIME_MANIFEST: &str = "skills/phoenix-ui/scripts/ENGINE.json";
const MAIN_SKILL: &str = "skills/phoenix-ui/SKILL.md";
const TRANSITION: &str = include_str!("../templates/impeccable-transition.md");

pub struct Request<'a> {
    pub project: &'a Path,
    pub bundle_root: &'a Path,
    /// Bundle provider directories, e.g. `.agents`, `.claude`, `.codex`.
    pub providers: Vec<String>,
    /// Home directory for read-only personal discovery; `None` skips it.
    pub home: Option<PathBuf>,
    /// Scan business sources only for an explicit `migrate --from impeccable`.
    pub scan_sources: bool,
    /// Install alongside Impeccable without inspecting or migrating its state.
    pub keep_impeccable: bool,
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
    /// Phoenix is active, but an integration finding remains.
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

/// These files belonged to earlier npm installs; skills.sh now owns their lifecycle.
pub(crate) fn legacy_auxiliary(path: &str) -> bool {
    let mut parts = path.split('/');
    parts
        .next()
        .is_some_and(|provider| provider.starts_with('.'))
        && parts.next() == Some("skills")
        && parts
            .next()
            .is_some_and(|skill| LEGACY_AUXILIARY_SKILLS.contains(&skill))
        && parts.next().is_some()
}

/// Read-only personal discovery for the selected providers. Old main entries
/// and design hooks there would run beside Phoenix, so they block activation.
fn discover_personal(
    home: &Path,
    project: &Path,
    providers: &[String],
    conflicts: &mut Vec<Conflict>,
    findings: &mut Vec<Finding>,
    keep_impeccable: bool,
) {
    let mut skill_roots = Vec::new();
    let mut hook_files = Vec::new();
    for provider in providers {
        match provider.as_str() {
            ".claude" => {
                skill_roots.push(".claude/skills");
                hook_files.push(".claude/settings.json");
                hook_files.push(".claude/settings.local.json");
            }
            ".agents" | ".codex" => {
                skill_roots.push(".agents/skills");
                skill_roots.push(".codex/skills");
                hook_files.push(".codex/hooks.json");
            }
            ".cursor" => {
                skill_roots.push(".cursor/skills");
                hook_files.push(".cursor/hooks.json");
            }
            _ => {}
        }
    }
    skill_roots.sort();
    skill_roots.dedup();
    hook_files.sort();
    hook_files.dedup();
    let locked_upstream = skills_sh_upstream_skills(home);
    for root in skill_roots {
        let old = home.join(root).join("impeccable");
        if !keep_impeccable && fs::symlink_metadata(&old).is_ok() {
            conflicts.push(Conflict {
                path: old.display().to_string(),
                provider: None,
                kind: ConflictKind::PersonalEntry,
                message: "personal Impeccable skill would run beside Phoenix; back it up and remove or disable it from the personal skills directory, reload the agent, then rerun the install".into(),
            });
        }
        findings.extend(personal_old_commands(home, root, &locked_upstream));
    }

    for file in hook_files {
        let path = home.join(file);
        let Ok(bytes) = read_bounded(&path, 4 * 1024 * 1024) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        if !keep_impeccable
            && (text.contains("skills/impeccable/scripts") || text.contains("impeccable hook"))
        {
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
    for path in enabled_in.into_iter().filter(|_| !keep_impeccable) {
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
fn personal_old_commands(home: &Path, root: &str, locked_upstream: &[String]) -> Vec<Finding> {
    let Ok(entries) = fs::read_dir(home.join(root)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            name != "impeccable"
                && name != "phoenix-ui"
                && !LEGACY_AUXILIARY_SKILLS.contains(&name.as_str())
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
    let active = match project.verified_installation_with_unmanaged(legacy_auxiliary) {
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
        request.keep_impeccable,
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
    report
        .preserved
        .extend(prepared.preserved.iter().cloned().map(|mut item| {
            if legacy_auxiliary(&item.path) {
                item.reason =
                "independently managed skill; Phoenix releases ownership without changing files"
                    .into();
            }
            item
        }));
    report.findings.extend(document_mentions(project.root()));
    if let Some(home) = &request.home {
        discover_personal(
            home,
            project.root(),
            &request.providers,
            &mut report.conflicts,
            &mut report.findings,
            request.keep_impeccable,
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
    let legacy = if active.is_none() && !request.keep_impeccable {
        let data = data::prepare_legacy_data(&project, &prepared.inventory, request.scan_sources)?;
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
                .filter(|o| !o.shared_entries && !legacy_auxiliary(&o.path))
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
    // Carry forward pins and migrated data outside the bundle.
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
