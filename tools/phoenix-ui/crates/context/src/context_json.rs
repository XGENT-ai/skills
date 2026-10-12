use crate::context::{
    is_context_surface_target, resolve_context_documents, resolve_context_selection,
    resolve_context_target,
};
use crate::design_parser::parse_design_md;
use crate::surface_briefs::{resolve_surface_brief, split_frontmatter};
use crate::target_args::{parse_target_options, TargetOptions};
use impeccable_common::Io;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const GENERIC_PROFILE: &str = include_str!("../../../profiles/generic.v1.json");
const PORTAL_PROFILE: &str = include_str!("../../../profiles/portal.v1.json");

fn hash(body: &str) -> String {
    format!("{:x}", Sha256::digest(body.as_bytes()))
}

fn document(path: Option<&str>) -> Result<Value, String> {
    let Some(path) = path else {
        return Ok(
            json!({"status":"missing", "path":null, "body":null, "sha256":null, "tokens":null}),
        );
    };
    let body =
        std::fs::read_to_string(path).map_err(|error| format!("Cannot read {path}: {error}"))?;
    Ok(json!({"status":"present", "path":path, "sha256":hash(&body), "body":body, "tokens":null}))
}

fn provenance(artifact: &str, document: &Value, project_root: &str) -> Option<Value> {
    let path = document["path"].as_str()?;
    Some(json!({
        "artifact":artifact, "path":path, "section":"document", "sha256":document["sha256"],
        "relationship":if crate::context::is_path_inside_or_equal(path, project_root) {"explicit"} else {"inherited"}
    }))
}

fn unsupported_yaml(body: &str) -> Vec<Value> {
    let Some((yaml, _)) = split_frontmatter(body) else {
        return if body.starts_with("---\n") || body.starts_with("---\r\n") {
            vec![json!({"line":1, "reason":"Unclosed YAML frontmatter"})]
        } else {
            vec![]
        };
    };
    yaml.lines().enumerate().filter_map(|(line, raw)| {
        let text = raw.trim();
        if text.is_empty() || text.starts_with('#') {
            return None;
        }
        let unsupported = raw.contains('\t') || text.starts_with('-') || text.split_once(':').is_none_or(|(_, value)| {
            let value = value.trim_start();
            value.starts_with(['[', '{', '|', '>', '&', '*', '!'])
        });
        unsupported.then(|| json!({"line":line+2, "reason":"Only scalar mappings are parsed; retain and read this YAML as written", "source":raw}))
    }).collect()
}

static TOKEN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{([A-Za-z][A-Za-z0-9_.-]*)\}").unwrap());

fn token_at<'a>(tokens: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(tokens, |value, key| value.get(key))
}

fn reference_problem(
    tokens: &Value,
    name: &str,
    visiting: &mut BTreeSet<String>,
) -> Option<(&'static str, String)> {
    let Some(value) = token_at(tokens, name) else {
        return Some(("invalid_token_reference", name.to_string()));
    };
    if value.is_null()
        || value.is_object()
        || value.is_array()
        || value.as_str().is_some_and(|text| text.trim().is_empty())
    {
        return Some(("invalid_token_reference", name.to_string()));
    }
    if !visiting.insert(name.to_string()) {
        return Some(("cyclic_token_reference", name.to_string()));
    }
    if let Some(text) = value.as_str() {
        for capture in TOKEN_RE.captures_iter(text) {
            if let Some(problem) = reference_problem(tokens, &capture[1], visiting) {
                return Some(problem);
            }
        }
    }
    visiting.remove(name);
    None
}

fn token_conflicts(
    value: &Value,
    tokens: &Value,
    owner: &str,
    path: &Value,
    conflicts: &mut Vec<Value>,
) {
    if let Some(object) = value.as_object() {
        for (key, child) in object {
            let name = if owner.is_empty() {
                key.clone()
            } else {
                format!("{owner}.{key}")
            };
            token_conflicts(child, tokens, &name, path, conflicts);
        }
    } else if let Some(text) = value.as_str() {
        for capture in TOKEN_RE.captures_iter(text) {
            let mut visiting = BTreeSet::from([owner.to_string()]);
            if let Some((kind, token)) = reference_problem(tokens, &capture[1], &mut visiting) {
                conflicts.push(json!({"kind":kind, "path":path, "owner":owner, "token":token}));
            }
        }
    }
}

fn coverage(artifact: &str, document: &mut Value, conflicts: &mut Vec<Value>) -> Option<Value> {
    let body = document["body"].as_str()?.to_string();
    let frontmatter = split_frontmatter(&body);
    let unsupported = unsupported_yaml(&body);
    let model = parse_design_md(&body);
    if artifact == "design" && frontmatter.is_some() && unsupported.is_empty() {
        let tokens = model.frontmatter.map(Value::Object).unwrap_or(Value::Null);
        token_conflicts(&tokens, &tokens, "", &document["path"], conflicts);
        document["tokens"] = tokens;
    }
    Some(json!({
        "artifact":artifact, "path":document["path"],
        "format":if frontmatter.is_some() {"markdown-yaml-subset"} else {"markdown"},
        "coverage":"partial", "recognizedSections":model.sections, "unsupported":unsupported,
        "limits":["Narrative rules, themes and responsive behavior require reading the original document", "The YAML parser supports scalar mappings; it is not a full YAML validator"]
    }))
}

fn profile(
    project_root: &str,
    design: &Value,
    conflicts: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut selected = None;
    for name in ["config.json", "config.local.json"] {
        let path = std::path::Path::new(project_root)
            .join(".phoenix-ui")
            .join(name);
        let body = match std::fs::read_to_string(&path) {
            Ok(body) => body,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("Cannot read {}: {error}", path.display())),
        };
        let config: Value = serde_json::from_str(&body)
            .map_err(|error| format!("Invalid {}: {error}", path.display()))?;
        if !config.is_object() {
            return Err(format!(
                "Invalid {}: expected a configuration object",
                path.display()
            ));
        }
        if let Some(selection) = config.get("profile") {
            selected = Some((
                selection.clone(),
                json!({"path":path, "section":"profile", "sha256":hash(&body)}),
            ));
        }
    }
    let Some((selection, source)) = selected else {
        return Ok(json!({"status":"disabled"}));
    };
    if selection["enabled"] == false {
        return Ok(json!({"status":"disabled", "source":source}));
    }
    let id = selection["id"].as_str().unwrap_or("");
    let version = selection["version"].as_str().unwrap_or("");
    if selection["enabled"] != true || id.is_empty() || version.is_empty() {
        conflicts.push(json!({"kind":"invalid_profile_selection", "source":source}));
        return Ok(json!({"status":"invalid", "source":source}));
    }
    let pinned_hash = selection.get("definitionSha256");
    if pinned_hash.is_some_and(|value| {
        value.as_str().is_none_or(|digest| {
            digest.len() != 64
                || !digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
    }) {
        conflicts.push(json!({"kind":"invalid_profile_selection", "source":source}));
        return Ok(json!({"status":"invalid", "source":source}));
    }
    let body = match id {
        "generic" => GENERIC_PROFILE,
        "portal" => PORTAL_PROFILE,
        _ => {
            conflicts.push(json!({"kind":"unknown_profile", "id":id, "source":source}));
            return Ok(
                json!({"status":"unavailable", "id":id, "version":version, "source":source}),
            );
        }
    };
    let mut definition: Value = serde_json::from_str(body).map_err(|error| error.to_string())?;
    let definition_hash = hash(body);
    definition["definitionSha256"] = json!(definition_hash);
    if definition["version"] != version
        || pinned_hash.is_some_and(|value| value != &definition["definitionSha256"])
    {
        let version_mismatch = definition["version"] != version;
        conflicts.push(json!({"kind":if version_mismatch {"profile_version_mismatch"} else {"profile_definition_mismatch"}, "id":id,
            "requested":version, "available":definition["version"], "requestedSha256":pinned_hash, "availableSha256":definition_hash, "source":source}));
        return Ok(json!({
            "status":if version_mismatch {"version_mismatch"} else {"definition_mismatch"},
            "id":id, "version":version, "definitionSha256":pinned_hash,
            "availableVersion":definition["version"], "source":source,
            "upgrade":{"comparison":"unavailable", "reason":"The selected definition is not bundled; read its original rules to identify differences before accepting the candidate",
                "candidate":definition, "requiresExplicitAcceptance":true, "applied":false}
        }));
    }
    let mut bindings = serde_json::Map::new();
    for reference in definition["tokenReferences"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if reference_problem(&design["tokens"], reference, &mut BTreeSet::new()).is_some() {
            conflicts.push(json!({"kind":"unresolved_profile_token", "profile":id, "token":reference, "path":design["path"]}));
            bindings.insert(
                reference.into(),
                json!({"status":"unresolved", "path":design["path"]}),
            );
        } else {
            bindings.insert(reference.into(), json!({"status":"present", "value":token_at(&design["tokens"], reference), "path":design["path"], "sha256":design["sha256"]}));
        }
    }
    definition["status"] = json!("enabled");
    definition["source"] = source;
    definition["acceptedDefinitionSha256"] = pinned_hash.cloned().unwrap_or(Value::Null);
    definition["appliesToFrozenTargets"] = json!(false);
    definition["tokenBindings"] = Value::Object(bindings);
    Ok(definition)
}

fn build(args: &[String], io: &Io) -> Result<Value, (i32, String)> {
    let cwd = io.cwd.to_string_lossy();
    let options = parse_target_options(args, true).map_err(|error| (2, error))?;
    for arg in args.iter().filter(|arg| arg.starts_with('-')) {
        if !matches!(arg.as_str(), "--json" | "--target" | "-t") && !arg.starts_with("--target=") {
            return Err((2, format!("Unknown context option: {arg}")));
        }
    }
    let target = options
        .target_path
        .as_deref()
        .map(|path| resolve_context_target(&cwd, path, &io.env))
        .transpose()
        .map_err(|error| (2, error))?;
    let selection = resolve_context_selection(&cwd, &options, &io.env);
    let options = TargetOptions {
        target_path: target
            .as_ref()
            .filter(|target| !is_context_surface_target(target))
            .cloned(),
    };
    let resolved = resolve_context_documents(&cwd, &options, &io.env);
    let mut product = document(resolved.product_path.as_deref()).map_err(|error| (4, error))?;
    let mut design = document(resolved.design_path.as_deref()).map_err(|error| (4, error))?;
    let mut conflicts = vec![];
    let mut parse_coverage = vec![];
    for (artifact, document) in [("product", &mut product), ("design", &mut design)] {
        if let Some(value) = coverage(artifact, document, &mut conflicts) {
            parse_coverage.push(value);
        }
    }
    let briefs = resolve_surface_brief(&resolved.project_root, target.as_deref());
    let linked_brief = crate::accepted_briefs::read(
        &resolved.project_root,
        &resolved.repo_root,
        target.as_deref(),
    )
    .map_err(|error| (4, error))?;
    let mut surface_brief = if let Some(brief) = linked_brief {
        let conflict = match brief["status"].as_str() {
            Some("stale") => Some("accepted_brief_changed"),
            Some("missing") => Some("accepted_brief_missing"),
            Some("ambiguous") => Some("ambiguous_accepted_brief"),
            _ => None,
        };
        if let Some(kind) = conflict {
            conflicts
                .push(json!({"kind":kind, "path":brief["path"], "candidates":brief["candidates"]}));
        }
        brief
    } else {
        document(
            briefs
                .brief
                .as_ref()
                .and_then(|brief| brief.path.as_deref()),
        )
        .map_err(|error| (4, error))?
    };
    for document in [&mut product, &mut design] {
        document["authority"] = json!(if document["status"] == "present" {
            "project_norm"
        } else {
            "missing"
        });
    }
    surface_brief["usable"] = json!(surface_brief["status"] == "accepted");
    surface_brief["authority"] = json!(match surface_brief["status"].as_str() {
        Some("accepted") => "accepted_direction",
        Some("stale" | "ambiguous") => "unresolved_direction",
        Some("missing") if !surface_brief["acceptedDecision"].is_null() => "unresolved_direction",
        Some("present") => "proposal",
        _ => "missing",
    });
    if surface_brief["acceptedDecision"].is_null()
        && surface_brief["status"] != "ambiguous"
        && briefs.reason.starts_with("ambiguous")
    {
        conflicts.push(json!({"kind":"ambiguous_surface_brief", "candidates":briefs.candidates.iter().filter_map(|brief| brief.path.as_deref()).collect::<Vec<_>>()}));
    }
    let profile =
        profile(&resolved.project_root, &design, &mut conflicts).map_err(|error| (4, error))?;
    let mut default_advice: Value =
        serde_json::from_str(GENERIC_PROFILE).map_err(|error| (4, error.to_string()))?;
    default_advice["status"] = json!("advisory");
    default_advice["definitionSha256"] = json!(hash(GENERIC_PROFILE));
    let mut provenance: Vec<_> = [
        ("product", &product),
        ("design", &design),
        ("surfaceBrief", &surface_brief),
    ]
    .into_iter()
    .filter_map(|(artifact, document)| provenance(artifact, document, &resolved.project_root))
    .collect();
    if let Some(mut source) = profile.get("source").cloned() {
        source["artifact"] = json!("profile");
        source["relationship"] = json!("explicit");
        provenance.push(source);
    }
    provenance.push(json!({"artifact":"defaultAdvice", "path":"tools/phoenix-ui/profiles/generic.v1.json", "section":"rules",
        "sha256":default_advice["definitionSha256"], "relationship":"advisory"}));
    Ok(json!({
        "schemaVersion":1, "status":if selection.is_some() {"selection_required"} else {"ready"},
        "projectRoot":resolved.project_root, "repoRoot":resolved.repo_root,
        "target":{"requested":args.iter().any(|arg| arg == "--target" || arg == "-t" || arg.starts_with("--target=")), "path":target, "exists":target.as_deref().filter(|target| !is_context_surface_target(target)).map(|path| std::path::Path::new(path).exists()), "candidates":selection.map(|selection| selection.target_candidates).unwrap_or_default()},
        "product":product, "design":design, "surfaceBrief":surface_brief,
        "profile":profile, "defaultAdvice":default_advice, "provenance":provenance,
        "precedence":{"order":["current_user_decision", "frozen_target_or_accepted_project_norm", "accepted_project_profile", "tool_default_advice"],
            "conflictPolicy":"require_user_decision", "profileRulesBecomeTargets":"only_after_acceptance_as_project_norm",
            "evaluation":"policy_only; read original sources to determine narrative conflicts; this command does not change frozen targets"},
        "parseCoverage":parse_coverage, "conflicts":conflicts,
        "capabilities":{"rawDocuments":true, "designParser":"yaml-scalar-mappings-and-canonical-heading-presence", "networkRequired":false}
    }))
}

pub fn run(args: &[String], io: &mut Io) -> i32 {
    match build(args, io) {
        Ok(value) => {
            io.out(&format!("{value}\n"));
            0
        }
        Err((code, message)) => {
            io.err(&format!("{message}\n"));
            io.out(&format!("{}\n", json!({"schemaVersion":1, "status":if code == 2 {"invalid_input"} else {"unavailable"}, "error":message})));
            code
        }
    }
}
