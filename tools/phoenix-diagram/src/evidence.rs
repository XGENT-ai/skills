//! Repository source evidence: structural checks on authored citations, then a
//! read-only comparison with one pinned local commit and the working tree.
//! Verification never fails a delivery; it records what could be confirmed.
pub mod git;
pub(crate) mod location;

pub use git::GitRunner;

use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::input::{sha256, DiagramInput, DiagramType};
use location::Rejection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceStatus {
    None,
    Verified,
    Partial,
    Unverified,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepositoryEvidence {
    pub label: String,
    pub url: String,
    pub revision: String,
    pub short_revision: String,
    pub link_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_href: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Observation {
    /// Whether HEAD of the given checkout is the pinned revision; null when Git was not reached.
    pub head_matches_revision: Option<bool>,
    pub worktree: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceReference {
    pub pointer: String,
    pub node_id: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
}

/// Canonical evidence bound into the HTML metadata and receipts. It carries no
/// local paths or times, so the same commit and tree give the same bytes.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceEvidence {
    pub schema_version: u32,
    pub status: EvidenceStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<RepositoryEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<Observation>,
    pub references: Vec<SourceReference>,
    pub unsourced: Vec<String>,
}

impl SourceEvidence {
    pub fn none() -> Self {
        Self {
            schema_version: 1,
            status: EvidenceStatus::None,
            repository: None,
            observation: None,
            references: vec![],
            unsourced: vec![],
        }
    }

    /// The configuration digest input; absent when nothing was cited.
    pub fn digest(&self) -> Option<String> {
        (self.status != EvidenceStatus::None)
            .then(|| sha256(&serde_json::to_vec(self).expect("evidence serializes")))
    }

    fn verified(&self) -> usize {
        self.references
            .iter()
            .filter(|reference| reference.state == "verified")
            .count()
    }

    /// The `evidence` fields of validation and delivery receipts.
    pub fn receipt(&self) -> Value {
        let unverified: Vec<_> = self
            .references
            .iter()
            .filter(|reference| reference.state != "verified")
            .map(|reference| json!({"pointer":reference.pointer,"nodeId":reference.node_id,"reason":reference.reason}))
            .collect();
        let mut value = json!({
            "status":self.status,"evidenceSha256":self.digest(),
            "counts":{"references":self.references.len(),"verified":self.verified(),"unverified":unverified.len(),"unsourced":self.unsourced.len()},
            "unverified":unverified,"unsourced":self.unsourced,
        });
        if let Some(repository) = &self.repository {
            value["repository"] = json!(repository);
        }
        value
    }

    /// The `evidence` fields of finalize receipts and summaries.
    pub fn summary(&self) -> Value {
        json!({"status":self.status,"evidenceSha256":self.digest()})
    }
}

fn collection(kind: DiagramType) -> &'static str {
    match kind {
        DiagramType::Architecture => "components",
        DiagramType::Workflow | DiagramType::Dataflow | DiagramType::Tree => "nodes",
        DiagramType::Sequence => "participants",
        DiagramType::Lifecycle => "states",
        DiagramType::Erd => "entities",
        DiagramType::Class => "types",
        DiagramType::Timeline => "events",
        DiagramType::Waterfall => "spans",
    }
}

struct Citation {
    pointer: String,
    node_id: String,
    path: String,
    line: Option<u64>,
    end_line: Option<u64>,
    label: Option<String>,
}

struct Repository {
    location: location::Location,
    revision: String,
    link_mode: String,
}

fn error(kind: DiagramType, code: &str, path: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(
        Some(kind.as_str()),
        &format!("repository/{code}"),
        path,
        message,
    )
}

fn citations(input: &DiagramInput, errors: &mut Diagnostics) -> Vec<Citation> {
    let kind = input.diagram_type;
    let collection = collection(kind);
    let mut result = Vec::new();
    let nodes = input.document[collection]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for (node_index, node) in nodes.iter().enumerate() {
        let sources = node["sources"].as_array().map(Vec::as_slice).unwrap_or(&[]);
        for (index, source) in sources.iter().enumerate() {
            let pointer = format!("/{collection}/{node_index}/sources/{index}");
            let path = source["path"].as_str().unwrap_or_default();
            let (line, end_line) = (source["line"].as_u64(), source["end_line"].as_u64());
            if path.is_empty()
                || path.starts_with('/')
                || path.contains('\\')
                || path.chars().any(char::is_control)
            {
                errors.push(error(
                    kind,
                    "path-invalid",
                    &format!("{pointer}/path"),
                    "A source path must be a repository-relative POSIX path",
                ));
                continue;
            }
            let segments: Vec<_> = path.split('/').collect();
            if segments
                .iter()
                .any(|segment| segment.is_empty() || *segment == "." || *segment == "..")
                || segments[0] == ".git"
            {
                errors.push(error(
                    kind,
                    "path-escape",
                    &format!("{pointer}/path"),
                    "A source path must stay inside the repository and may not address .git",
                ));
                continue;
            }
            if end_line.is_some() && line.is_none() {
                errors.push(error(
                    kind,
                    "line-required",
                    &format!("{pointer}/end_line"),
                    "end_line requires line",
                ));
                continue;
            }
            if let (Some(line), Some(end)) = (line, end_line) {
                if end < line {
                    errors.push(error(
                        kind,
                        "line-range-invalid",
                        &pointer,
                        "end_line must be at least line",
                    ));
                    continue;
                }
            }
            result.push(Citation {
                pointer,
                node_id: node["id"].as_str().unwrap_or_default().into(),
                path: path.into(),
                line,
                end_line,
                label: source["label"].as_str().map(str::to_owned),
            });
        }
    }
    result
}

fn repository(input: &DiagramInput, errors: &mut Diagnostics) -> Option<Repository> {
    let kind = input.diagram_type;
    let authored = input.document["meta"].get("repository")?;
    let revision = authored["revision"].as_str().unwrap_or_default();
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        errors.push(error(
            kind,
            "revision-invalid",
            "/meta/repository/revision",
            "revision must be a full 40-character commit SHA",
        ));
    }
    let url = authored["url"].as_str().unwrap_or_default();
    let provider = authored["provider"].as_str();
    let link_mode = authored["link_mode"].as_str().unwrap_or("web");
    let location = match location::parse_remote(url, true, provider) {
        Ok(location) => location,
        Err(rejection) => {
            let mut diagnostic = error(kind, "url-invalid", "/meta/repository/url", format!(
                "{} must be a credential-free HTTP(S) or Git SSH repository address without query, fragment or dot segments",
                location::redact(url)
            ));
            diagnostic.evidence = json!({"reason": if rejection == Rejection::Credentials { "credentials" } else { "syntax" }});
            errors.push(diagnostic);
            return None;
        }
    };
    if provider.is_some_and(|provider| Some(provider) != location.provider) {
        errors.push(error(
            kind,
            "provider-invalid",
            "/meta/repository/provider",
            "provider must match the repository host; gitlab also names a self-managed GitLab host",
        ));
    }
    if link_mode == "web"
        && !location.provider.is_some_and(|provider| {
            location.protocol == "https:"
                && location.endpoint == "standard"
                && location::web_path(provider, &location.path)
        })
    {
        errors.push(error(kind, "links-unsupported", "/meta/repository/url", "Web source links need a canonical GitHub, Gitee or GitLab HTTPS repository URL; use link_mode local-only otherwise"));
    }
    Some(Repository {
        location,
        revision: revision.to_ascii_lowercase(),
        link_mode: link_mode.into(),
    })
}

fn line_count(content: &[u8]) -> u64 {
    let mut breaks = 0;
    let mut index = 0;
    while index < content.len() {
        match content[index] {
            b'\r' if content.get(index + 1) == Some(&b'\n') => {
                breaks += 1;
                index += 1;
            }
            b'\r' | b'\n' => breaks += 1,
            _ => {}
        }
        index += 1;
    }
    if content.is_empty() {
        0
    } else if matches!(content.last(), Some(b'\r' | b'\n')) {
        breaks
    } else {
        breaks + 1
    }
}

/// Segment by segment without following links: directories, then one regular
/// file whose bytes equal the pinned blob.
fn worktree_matches(root: &Path, path: &str, blob: &[u8]) -> bool {
    let segments: Vec<_> = path.split('/').collect();
    let mut current = root.to_path_buf();
    for (index, segment) in segments.iter().enumerate() {
        current.push(segment);
        let Ok(metadata) = std::fs::symlink_metadata(&current) else {
            return false;
        };
        let last = index + 1 == segments.len();
        if (last && !metadata.file_type().is_file()) || (!last && !metadata.file_type().is_dir()) {
            return false;
        }
    }
    std::fs::read(&current).is_ok_and(|bytes| bytes == blob)
}

enum Blob {
    Found(Vec<u8>),
    Missing,
    NotFile,
}

fn read_blobs(output: &[u8], objects: &[String]) -> Option<BTreeMap<String, Blob>> {
    let mut cursor = 0;
    let mut result = BTreeMap::new();
    for object in objects {
        let end = cursor + output[cursor..].iter().position(|byte| *byte == b'\n')?;
        let header = std::str::from_utf8(&output[cursor..end]).ok()?;
        cursor = end + 1;
        if header.ends_with(" missing") || header.ends_with(" ambiguous") {
            result.insert(object.clone(), Blob::Missing);
            continue;
        }
        let parts: Vec<_> = header.split(' ').collect();
        let size: usize = parts.get(2)?.parse().ok()?;
        if parts.len() != 3 || output.get(cursor + size) != Some(&b'\n') {
            return None;
        }
        let content = output[cursor..cursor + size].to_vec();
        cursor += size + 1;
        result.insert(
            object.clone(),
            if parts[1] == "blob" {
                Blob::Found(content)
            } else {
                Blob::NotFile
            },
        );
    }
    Some(result)
}

/// Validates citations and checks them against the local repository.
pub fn verify(
    input: &DiagramInput,
    repo_root: Option<&Path>,
) -> Result<SourceEvidence, Diagnostics> {
    verify_with(input, repo_root, &GitRunner::default())
}

pub fn verify_with(
    input: &DiagramInput,
    repo_root: Option<&Path>,
    git: &GitRunner,
) -> Result<SourceEvidence, Diagnostics> {
    let mut errors = Vec::new();
    let citations = citations(input, &mut errors);
    let repository = repository(input, &mut errors);
    if input.document["meta"].get("repository").is_none() {
        if citations.is_empty() && errors.is_empty() {
            return Ok(SourceEvidence::none());
        }
        errors.insert(
            0,
            error(
                input.diagram_type,
                "repository-required",
                "/meta/repository",
                "Source references require /meta/repository",
            ),
        );
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let repository = repository.expect("validated repository");
    let (states, observation) = match repo_root {
        None => (
            vec![Err("repo-root-not-provided"); citations.len()],
            Observation {
                head_matches_revision: None,
                worktree: "not-compared".into(),
            },
        ),
        Some(_) if citations.is_empty() => (
            vec![],
            Observation {
                head_matches_revision: None,
                worktree: "not-compared".into(),
            },
        ),
        Some(root) => check(
            root,
            &repository,
            &citations,
            git,
            input.document["meta"]["repository"]["provider"].as_str(),
        ),
    };
    let web = repository.link_mode == "web";
    let provider = repository.location.provider.unwrap_or_default();
    let references: Vec<_> = citations
        .into_iter()
        .zip(states)
        .map(|(citation, state)| SourceReference {
            href: (web && state.is_ok()).then(|| {
                location::source_href(
                    provider,
                    &repository.location.url,
                    &repository.revision,
                    &citation.path,
                    citation.line,
                    citation.end_line,
                )
            }),
            pointer: citation.pointer,
            node_id: citation.node_id,
            path: citation.path,
            line: citation.line,
            end_line: citation.end_line,
            label: citation.label,
            state: if state.is_ok() {
                "verified"
            } else {
                "unverified"
            }
            .into(),
            reason: state.err().map(str::to_owned),
        })
        .collect();
    let unsourced = unsourced(input);
    let mut evidence = SourceEvidence {
        schema_version: 1,
        status: EvidenceStatus::Unverified,
        repository: Some(RepositoryEvidence {
            label: if provider == "github" {
                repository.location.path.clone()
            } else {
                ["https://", "http://", "ssh://git@", "git@"]
                    .iter()
                    .find_map(|prefix| repository.location.url.strip_prefix(prefix))
                    .unwrap_or(&repository.location.url)
                    .into()
            },
            url: repository.location.url.clone(),
            short_revision: repository.revision[..7].into(),
            tree_href: web.then(|| {
                location::tree_href(provider, &repository.location.url, &repository.revision)
            }),
            revision: repository.revision,
            link_mode: repository.link_mode,
        }),
        observation: Some(observation),
        references,
        unsourced,
    };
    evidence.status = status(&evidence);
    Ok(evidence)
}

/// Primary elements without a citation; computed only under a declared repository.
fn unsourced(input: &DiagramInput) -> Vec<String> {
    input.document[collection(input.diagram_type)]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|node| node["sources"].as_array().is_none_or(Vec::is_empty))
        .filter_map(|node| node["id"].as_str().map(str::to_owned))
        .collect()
}

fn status(evidence: &SourceEvidence) -> EvidenceStatus {
    let verified = evidence.verified();
    if verified == 0 {
        EvidenceStatus::Unverified
    } else if verified == evidence.references.len() && evidence.unsourced.is_empty() {
        EvidenceStatus::Verified
    } else {
        EvidenceStatus::Partial
    }
}

type States = Vec<Result<(), &'static str>>;

fn check(
    root: &Path,
    repository: &Repository,
    citations: &[Citation],
    git: &GitRunner,
    provider_hint: Option<&str>,
) -> (States, Observation) {
    let mut observation = Observation {
        head_matches_revision: None,
        worktree: "not-compared".into(),
    };
    let all = |reason: &'static str| -> States { vec![Err(reason); citations.len()] };
    let failed = |failure: git::Failure| match failure {
        git::Failure::Unavailable => "git-unavailable",
        git::Failure::Timeout => "git-timeout",
    };
    macro_rules! run {
        ($args:expr, $input:expr) => {
            match git.run(root, $args, $input) {
                Ok(output) => output,
                Err(failure) => return (all(failed(failure)), observation),
            }
        };
    }
    let version = run!(&["version"], None);
    if !version.success {
        return (all("git-unavailable"), observation);
    }
    let version = git::version(&version.stdout);
    let top = run!(&["rev-parse", "--show-toplevel"], None);
    if !top.success {
        return (all("repository-unreadable"), observation);
    }
    let top = String::from_utf8_lossy(&top.stdout)
        .trim_end_matches('\n')
        .to_owned();
    let same = match (std::fs::metadata(&top), std::fs::metadata(root)) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    };
    if !same {
        return (all("root-not-top-level"), observation);
    }
    let origin = run!(&["remote", "get-url", "origin"], None);
    let origin_matches = origin.success
        && location::parse_remote(
            &String::from_utf8_lossy(&origin.stdout),
            false,
            provider_hint,
        )
        .is_ok_and(|origin| origin.identity == repository.location.identity);
    if !origin_matches {
        return (all("origin-mismatch"), observation);
    }
    // Git can register a promisor from remote.<name>.promisor or partialclonefilter
    // alone; before 2.45 only the empty protocol list stands between a missing
    // object and a fetch attempt, so such repositories are not read at all.
    let promisor = run!(
        &[
            "config",
            "--get-regexp",
            r"^(extensions\.partialclone|remote\..+\.(promisor|partialclonefilter))$"
        ],
        None
    );
    let new_enough = version.is_some_and(|version| version >= (2, 45));
    if promisor.success && !promisor.stdout.is_empty() && !new_enough {
        return (all("partial-clone-unsafe-git"), observation);
    }
    let commit_object = format!("{}^{{commit}}", repository.revision);
    let commit = run!(&["cat-file", "-e", commit_object.as_str()], None);
    if !commit.success {
        return (all("revision-unavailable"), observation);
    }
    let head = run!(&["rev-parse", "HEAD"], None);
    observation.head_matches_revision =
        Some(head.success && String::from_utf8_lossy(&head.stdout).trim() == repository.revision);
    let mut objects: Vec<String> = Vec::new();
    for citation in citations {
        let object = format!("{}:{}", repository.revision, citation.path);
        if !objects.contains(&object) {
            objects.push(object);
        }
    }
    let mut input = objects.join("\n").into_bytes();
    input.push(b'\n');
    let batch = run!(&["cat-file", "--batch"], Some(input));
    let blobs = match batch
        .success
        .then(|| read_blobs(&batch.stdout, &objects))
        .flatten()
    {
        Some(blobs) => blobs,
        // A failed batch (for example a refused promisor fetch) loses every
        // answer; ask per object, as the frozen reader does.
        None => {
            let mut blobs = BTreeMap::new();
            for object in &objects {
                let kind = run!(&["cat-file", "-t", object.as_str()], None);
                let blob = if !kind.success {
                    Blob::Missing
                } else if kind.stdout.trim_ascii() != b"blob" {
                    Blob::NotFile
                } else {
                    let content = run!(&["cat-file", "blob", object.as_str()], None);
                    if content.success {
                        Blob::Found(content.stdout)
                    } else {
                        Blob::Missing
                    }
                };
                blobs.insert(object.clone(), blob);
            }
            blobs
        }
    };
    observation.worktree = "compared".into();
    let mut states = Vec::new();
    for citation in citations {
        let object = format!("{}:{}", repository.revision, citation.path);
        let state = match &blobs[&object] {
            Blob::Found(content) => {
                let requested = citation.end_line.or(citation.line).unwrap_or(0);
                if requested > line_count(content) {
                    Err("line-out-of-range")
                } else if !worktree_matches(root, &citation.path, content) {
                    Err("worktree-differs")
                } else {
                    Ok(())
                }
            }
            Blob::NotFile => Err("file-missing"),
            Blob::Missing => {
                let tree = run!(
                    &[
                        "ls-tree",
                        "-z",
                        repository.revision.as_str(),
                        "--",
                        citation.path.as_str()
                    ],
                    None
                );
                let known = tree.success
                    && tree.stdout.split(|byte| *byte == 0).any(|entry| {
                        let entry = String::from_utf8_lossy(entry);
                        entry.split_once('\t').is_some_and(|(meta, path)| {
                            path == citation.path && meta.split(' ').nth(1) == Some("blob")
                        })
                    });
                Err(if known {
                    "object-unavailable"
                } else {
                    "file-missing"
                })
            }
        };
        states.push(state);
    }
    (states, observation)
}

/// The evidence an artifact carries, re-derived for strict currency without Git:
/// its citations must be exactly the candidate's, and its status must follow
/// from its own reference states and unsourced elements.
pub fn bound(input: &DiagramInput, metadata: &Value) -> Result<SourceEvidence, &'static str> {
    let evidence: SourceEvidence = serde_json::from_value(metadata["sourceEvidence"].clone())
        .map_err(|_| "Artifact source evidence is missing or malformed")?;
    let mut errors = Vec::new();
    let citations = citations(input, &mut errors);
    let declared = input.document["meta"].get("repository").is_some();
    let consistent = errors.is_empty()
        && evidence.schema_version == 1
        && evidence.references.len() == citations.len()
        && evidence
            .references
            .iter()
            .zip(&citations)
            .all(|(reference, citation)| {
                reference.pointer == citation.pointer
                    && reference.node_id == citation.node_id
                    && reference.path == citation.path
                    && reference.line == citation.line
                    && reference.end_line == citation.end_line
                    && reference.label == citation.label
                    && matches!(
                        (reference.state.as_str(), &reference.reason),
                        ("verified", None) | ("unverified", Some(_))
                    )
            })
        && if declared {
            evidence.repository.as_ref().is_some_and(|repository| {
                Some(repository.revision.as_str())
                    == input.document["meta"]["repository"]["revision"]
                        .as_str()
                        .map(str::to_ascii_lowercase)
                        .as_deref()
            }) && evidence.unsourced == unsourced(input)
                && evidence.status == status(&evidence)
        } else {
            evidence == SourceEvidence::none()
        };
    if consistent {
        Ok(evidence)
    } else {
        Err("Artifact source evidence does not match the candidate's citations")
    }
}
