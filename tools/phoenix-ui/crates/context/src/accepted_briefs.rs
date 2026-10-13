use crate::context::{is_context_surface_target, resolve_context_project, resolve_context_target};
use crate::surface_briefs::normalize_surface_target;
use crate::target_args::TargetOptions;
use impeccable_common::Io;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions, TryLockError};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

const INDEX: &str = ".phoenix-ui/briefs.json";

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Index {
    schema_version: u32,
    briefs: Vec<Link>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Link {
    target: String,
    path: String,
    #[serde(default, skip_serializing_if = "SourceRoot::is_project")]
    source_root: SourceRoot,
    sha256: String,
    accepted_decision: String,
}

#[derive(Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
enum SourceRoot {
    #[default]
    Project,
    Repository,
}

impl SourceRoot {
    fn is_project(&self) -> bool {
        matches!(self, Self::Project)
    }
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn safe_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let mut path = root.to_path_buf();
    let components: Vec<_> = Path::new(relative).components().collect();
    if components.is_empty()
        || components
            .iter()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(
            "Brief paths must be relative to the selected project without parent traversal".into(),
        );
    }
    for part in components {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!(
                    "Refusing a symlinked brief path: {}",
                    path.display()
                ))
            }
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(error.to_string())
            }
            _ => (),
        }
    }
    Ok(path)
}

fn index_bytes(root: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(safe_path(root, INDEX)?) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Cannot read {INDEX}: {error}")),
    }
}

fn source_path(project: &Path, repository: &Path, brief: &Link) -> Result<PathBuf, String> {
    safe_path(
        match brief.source_root {
            SourceRoot::Project => project,
            SourceRoot::Repository => repository,
        },
        &brief.path,
    )
}

fn parse_index(root: &Path, repository: &Path, bytes: Option<&[u8]>) -> Result<Index, String> {
    let index: Index = match bytes {
        Some(bytes) => {
            serde_json::from_slice(bytes).map_err(|error| format!("Invalid {INDEX}: {error}"))?
        }
        None => Index {
            schema_version: 1,
            briefs: vec![],
        },
    };
    if index.schema_version != 1 {
        return Err("Unsupported accepted brief index version".into());
    }
    let mut seen = BTreeSet::new();
    for brief in &index.briefs {
        let credentialed = crate::url::parse(&brief.target)
            .is_some_and(|url| !url.username.is_empty() || !url.password.is_empty());
        if credentialed
            || normalize_surface_target(Some(&brief.target), &root.to_string_lossy()).as_deref()
                != Some(&brief.target)
            || brief.sha256.len() != 64
            || !brief
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            || brief.accepted_decision.trim().is_empty()
            || Path::new(&brief.path)
                .components()
                .any(|part| part.as_os_str() == ".phoenix-ui")
            || !seen.insert((&brief.target, &brief.path, &brief.source_root))
        {
            return Err("Invalid or duplicate accepted brief mapping".into());
        }
        source_path(root, repository, brief)?;
    }
    Ok(index)
}

pub fn read(root: &str, repo_root: &str, target: Option<&str>) -> Result<Option<Value>, String> {
    let project = Path::new(root);
    let repository = Path::new(repo_root);
    let bytes = index_bytes(project)?;
    let index = parse_index(project, repository, bytes.as_deref())?;
    let normalized = normalize_surface_target(target, root);
    let candidates: Vec<_> = index
        .briefs
        .iter()
        .filter(|brief| {
            normalized
                .as_ref()
                .is_none_or(|target| target == &brief.target)
        })
        .collect();
    if candidates.is_empty() {
        return Ok(None);
    }
    if candidates.len() > 1 {
        return Ok(Some(
            json!({"status":"ambiguous", "path":null, "body":null, "sha256":null, "tokens":null, "candidates":candidates}),
        ));
    }
    let brief = candidates[0];
    let path = source_path(project, repository, brief)?;
    let body = match fs::read_to_string(&path) {
        Ok(body) => body,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Some(
                json!({"status":"missing", "path":path, "body":null, "sha256":null, "tokens":null, "acceptedSha256":brief.sha256, "acceptedDecision":brief.accepted_decision}),
            ))
        }
        Err(error) => return Err(format!("Cannot read {}: {error}", path.display())),
    };
    let current = hash(body.as_bytes());
    Ok(Some(
        json!({"status":if current == brief.sha256 {"accepted"} else {"stale"}, "path":path, "body":body, "sha256":current, "tokens":null,
        "target":brief.target, "acceptedSha256":brief.sha256, "acceptedDecision":brief.accepted_decision}),
    ))
}

fn link(args: &[String], io: &Io) -> Result<Value, (i32, String)> {
    if args.len() != 5 || args[3] != "--decision" || args[4].trim().is_empty() {
        return Err((2, "usage: phoenix-ui surface-brief link <primary-target> <brief-file> --decision <user-acceptance-reference>".into()));
    }
    let requested_target = resolve_context_target(&io.cwd.to_string_lossy(), &args[1], &io.env)
        .map_err(|error| (2, error))?;
    let resolved = resolve_context_project(
        &io.cwd.to_string_lossy(),
        &TargetOptions {
            target_path: (!is_context_surface_target(&requested_target))
                .then(|| requested_target.clone()),
        },
        &io.env,
    );
    let root = resolved.project_root;
    let target = normalize_surface_target(Some(&requested_target), &root)
        .ok_or((2, "A concrete project target is required".into()))?;
    let project = Path::new(&root);
    let repository = Path::new(&resolved.repo_root);
    let requested = PathBuf::from(crate::jsp::resolve(&io.cwd.to_string_lossy(), &[&args[2]]));
    let (source_root, source_base) = if requested.starts_with(project) {
        (SourceRoot::Project, project)
    } else if requested.starts_with(repository) {
        (SourceRoot::Repository, repository)
    } else {
        return Err((
            2,
            "Brief source must be inside the selected repository".into(),
        ));
    };
    let relative = requested
        .strip_prefix(source_base)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    if Path::new(&relative)
        .components()
        .any(|part| part.as_os_str() == ".phoenix-ui")
    {
        return Err((
            2,
            "Link the original project brief instead of an editable state copy".into(),
        ));
    }
    let source = safe_path(source_base, &relative).map_err(|error| (2, error))?;
    let body = fs::read(&source)
        .map_err(|error| (4, format!("Cannot read {}: {error}", source.display())))?;
    std::str::from_utf8(&body).map_err(|_| (2, "Brief source must be UTF-8 text".into()))?;
    let digest = hash(&body);
    let state = safe_path(project, ".phoenix-ui").map_err(|error| (2, error))?;
    fs::create_dir_all(&state).map_err(|error| (4, error.to_string()))?;
    let lock_path = safe_path(project, ".phoenix-ui/briefs.lock").map_err(|error| (2, error))?;
    let _lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|error| (4, error.to_string()))?;
    _lock.try_lock().map_err(|error| match error {
        TryLockError::WouldBlock => (
            3,
            "Another writer holds the accepted brief index lock".into(),
        ),
        TryLockError::Error(error) => (4, error.to_string()),
    })?;
    let before = index_bytes(project).map_err(|error| (4, error))?;
    let mut index =
        parse_index(project, repository, before.as_deref()).map_err(|error| (4, error))?;
    index.briefs.retain(|brief| {
        brief.target != target || brief.path != relative || brief.source_root != source_root
    });
    index.briefs.push(Link {
        target,
        path: relative,
        source_root,
        sha256: digest.clone(),
        accepted_decision: args[4].clone(),
    });
    let bytes = serde_json::to_vec_pretty(&index).map_err(|error| (4, error.to_string()))?;
    let destination = safe_path(project, INDEX).map_err(|error| (2, error))?;
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temporary = state.join(format!(
        ".briefs-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| (4, error.to_string()))?;
    let write = (|| {
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| (4, error.to_string()))?;
        if hash(&fs::read(&source).map_err(|error| (4, error.to_string()))?) != digest
            || index_bytes(project).map_err(|error| (4, error))? != before
        {
            return Err((
                3,
                "Brief or index changed during linking; read it again before accepting".into(),
            ));
        }
        drop(file);
        fs::rename(&temporary, &destination).map_err(|error| (4, error.to_string()))
    })();
    if write.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write?;
    Ok(
        json!({"schemaVersion":1, "status":"linked", "index":destination, "brief":index.briefs.last()}),
    )
}

pub fn run(args: &[String], io: &mut Io) -> i32 {
    match link(args, io) {
        Ok(value) => {
            io.out(&format!("{value}\n"));
            0
        }
        Err((code, error)) => {
            io.err(&format!("{error}\n"));
            io.out(&format!("{}\n", json!({"schemaVersion":1, "status":match code {2=>"invalid_input",3=>"conflict",_=>"unavailable"}, "error":error})));
            code
        }
    }
}
