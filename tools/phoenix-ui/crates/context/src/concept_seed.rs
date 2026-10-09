//! JS: concept-seed.mjs -> `impeccable concept-seed`

use crate::catalog::*;
use crate::context::load_context;
use crate::jsp;
use crate::roll_selection::*;
use crate::seed_text as t;
use crate::target_args::TargetOptions;
use crate::util::Env;
use impeccable_common::Io;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn fill(tpl: &str, pairs: &[(&str, &str)]) -> String {
    let mut s = tpl.to_string();
    for (k, v) in pairs {
        s = s.replace(&format!("@@{}@@", k), v);
    }
    s
}

/// The mode file that carries a seed mode's rules: experience shares persuade's.
pub fn mode_rules_file(mode: &str) -> &'static str {
    match mode {
        "operate" => "mode-operate",
        "read" => "mode-read",
        _ => "mode-persuade",
    }
}

/// A level-2 section's body, verbatim: every line after `## <name>` up to the
/// next `## ` heading or EOF, trimmed of leading and trailing blank lines.
/// `None` when the heading is absent or the body is empty. CRLF tolerant.
pub fn extract_section(text: &str, name: &str) -> Option<String> {
    let text = text.replace("\r\n", "\n");
    let heading = format!("## {}", name);
    let mut lines = text.split('\n');
    lines.by_ref().find(|l| l.trim_end() == heading)?;
    let body: Vec<&str> = lines.take_while(|l| !l.starts_with("## ")).collect();
    let start = body.iter().position(|l| !l.trim().is_empty())?;
    let end = body.iter().rposition(|l| !l.trim().is_empty())?;
    Some(body[start..=end].join("\n"))
}

/// The MODE RULES block for `mode`, or the one-line fallback naming the file
/// when it cannot be read or lacks a section. Never fails the roll.
pub fn mode_rules_block(env: &Env, cwd: &str, mode: &str) -> String {
    let file = mode_rules_file(mode);
    let path = crate::provider::detect(env, cwd).reference_path(file);
    let shown = path.clone().unwrap_or_else(|| format!("reference/{}.md", file));
    let sections = path
        .as_deref()
        .and_then(crate::util::safe_read)
        .and_then(|text| Some((extract_section(&text, "Directions")?, extract_section(&text, "Comps")?)));
    match sections {
        Some((directions, comps)) => fill(t::MODE_RULES_BLOCK, &[("MODE", mode), ("PATH", &shown), ("DIRECTIONS", &directions), ("COMPS", &comps)]),
        None => fill(t::MODE_RULES_UNAVAILABLE, &[("PATH", &shown)]),
    }
}

fn vs(v: &Value, key: &str) -> String {
    match v.get(key) {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => crate::critique_storage::js_string_value(other),
    }
}

/// JS: renderChallenger
fn render_challenger(concept: &Value, index: usize) -> String {
    let system: Vec<String> = concept
        .get("system")
        .and_then(|s| s.as_array())
        .map(|a| {
            a.iter()
                .map(|r| fill(t::RENDER_CHALLENGER_RULE, &[("RULE", &value_str(r))]))
                .collect()
        })
        .unwrap_or_default();
    let id = vs(concept, "id");
    let images: Vec<String> = ["cardBoard", "cardHero"]
        .iter()
        .filter_map(|key| {
            concept
                .get(*key)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
        })
        .map(str::to_string)
        .collect();
    let media = if images.is_empty() {
        "text-only candidate; no local image supplied".to_string()
    } else {
        format!(
            "local references: {} (verify availability before comparison)",
            images.join(" · ")
        )
    };
    fill(
        t::RENDER_CHALLENGER,
        &[
            ("INDEX_PLUS_ONE", &(index + 1).to_string()),
            ("CONCEPT_FORM", &vs(concept, "form")),
            ("CONCEPT_ID", &id),
            ("CONCEPT_SPARK", &vs(concept, "spark")),
            ("SYSTEM", &system.join("\n")),
            ("CONCEPT_WEBLEVERAGE", &vs(concept, "webLeverage")),
            ("MEDIA", &media),
        ],
    )
}

fn value_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        other => crate::critique_storage::js_string_value(other),
    }
}

/// JS: renderComposition
fn render_composition(c: &Value, index: Option<usize>) -> String {
    let grammar: Vec<String> = c
        .get("grammar")
        .and_then(|s| s.as_array())
        .map(|a| a.iter().map(|r| fill(t::RENDER_COMPOSITION_RULE, &[("RULE", &value_str(r))])).collect())
        .unwrap_or_default();
    fill(
        t::RENDER_COMPOSITION,
        &[
            ("COMP_INDEX_PREFIX", &index.map(|i| format!("{}. ", i + 1)).unwrap_or_default()),
            ("COMPOSITION_FORM", &vs(c, "form")),
            ("COMPOSITION_ID", &vs(c, "id")),
            ("COMPOSITION_SPARK", &vs(c, "spark")),
            ("GRAMMAR", &grammar.join("\n")),
            ("COMPOSITION_WEBLEVERAGE", &vs(c, "webLeverage")),
        ],
    )
}

struct Local {
    catalog_version: String,
    concepts: Vec<Value>,
    compositions: Vec<Value>,
}

fn local_from_values(
    catalog: Value,
    reviews: Value,
    composition: Value,
    composition_reviews: Value,
) -> Result<Local, String> {
    let mut errors = validate_concept_catalog(&catalog, &reviews);
    errors.extend(validate_composition_catalog(
        &composition,
        &composition_reviews,
    ));
    if catalog.get("catalogVersion") != composition.get("catalogVersion") {
        errors.push("concept and composition catalogVersion must match".into());
    }
    if !errors.is_empty() {
        return Err(format!(
            "concept-seed: invalid local catalog: {}",
            errors.join("; ")
        ));
    }
    let cat = concept_catalog_from_values(catalog, reviews)
        .ok_or("concept-seed: invalid concept catalog structure")?;
    let comp = composition_catalog_from_values(composition, composition_reviews)
        .ok_or("concept-seed: invalid composition catalog structure")?;
    Ok(Local {
        catalog_version: vs(&cat.catalog, "catalogVersion"),
        concepts: cat.concepts,
        compositions: comp.compositions,
    })
}

fn load_local(catalog_dir: &str) -> Result<Local, String> {
    let read = |name: &str| -> Result<Value, String> {
        let file = jsp::join(&[catalog_dir, name]);
        let text = crate::util::safe_read(&file)
            .ok_or_else(|| format!("concept-seed: missing or unreadable catalog file {file}"))?;
        serde_json::from_str(&text)
            .map_err(|e| format!("concept-seed: invalid JSON in {file}: {e}"))
    };
    local_from_values(
        read("concept-ingredients.json")?,
        read("concept-reviews.json")?,
        read("composition-ingredients.json")?,
        read("composition-reviews.json")?,
    )
}

fn builtin_catalog() -> Result<Local, String> {
    let parse = |text| {
        serde_json::from_str(text)
            .map_err(|e| format!("concept-seed: invalid built-in catalog: {e}"))
    };
    local_from_values(
        parse(include_str!("../../../../../skills/phoenix-ui/src/scripts/data/catalog/concept-ingredients.json"))?,
        parse(include_str!("../../../../../skills/phoenix-ui/src/scripts/data/catalog/concept-reviews.json"))?,
        parse(include_str!("../../../../../skills/phoenix-ui/src/scripts/data/catalog/composition-ingredients.json"))?,
        parse(include_str!("../../../../../skills/phoenix-ui/src/scripts/data/catalog/composition-reviews.json"))?)
}

struct RollData {
    catalog_version: String,
    source: String,
    pool_revision: String,
    approved_count: String,
    catalog_count: String,
    challengers: Vec<Value>,
    compositions: Vec<Value>,
    composition_match: Option<CompositionMatch>,
}

pub struct SeedArgs {
    pub scope: Option<String>,
    pub key: String,
    pub reroll: f64,
    pub register: Option<Option<String>>, // None = not given; Some(None) = flag without value
    pub mode: Option<Option<String>>,
    pub grain: Option<Option<String>>,
    pub platform: Option<Option<String>>,
    pub candidate_count: f64,
}

fn unit(scope: &str, salt: &str, key: &str) -> f64 {
    let d = Sha256::digest(format!("{}:{}:{}", scope, salt, key).as_bytes());
    u32::from_be_bytes([d[0], d[1], d[2], d[3]]) as f64 / 4294967295.0
}

/// JS: renderConceptSeed
fn render_concept_seed(env: &Env, cwd: &str, a: &SeedArgs) -> Result<String, (i32, String)> {
    let scope = match a.scope.as_deref() {
        Some("surface") => "surface",
        Some("direction") => "direction",
        _ => {
            return Err((
                1,
                "concept-seed: --scope must be direction or surface".into(),
            ))
        }
    };
    if !(a.reroll.is_finite() && a.reroll.fract() == 0.0) || a.reroll < 0.0 {
        return Err((
            1,
            "concept-seed: --reroll must be a non-negative integer".into(),
        ));
    }
    let reroll = a.reroll as usize;
    let register: Option<&str> = match &a.register {
        None => None,
        Some(Some(r)) if r == "safer" || r == "bolder" => Some(r.as_str()),
        Some(_) => return Err((1, "concept-seed: --register must be safer or bolder".into())),
    };
    if register.is_some() && reroll < 1 {
        return Err((
            1,
            "concept-seed: --register steers a re-roll round; pass --reroll <n> with it".into(),
        ));
    }
    if register.is_some() && scope != "direction" {
        return Err((
            1,
            "concept-seed: --register applies to direction rounds only".into(),
        ));
    }
    let mode: Option<&str> = match &a.mode {
        None => None,
        Some(Some(m)) if SEED_MODES.contains(&m.as_str()) => Some(m.as_str()),
        Some(_) => {
            return Err((
                1,
                "concept-seed: --mode must be persuade, operate, read, or experience".into(),
            ))
        }
    };
    let grain: Option<&str> = match &a.grain {
        None => None,
        Some(Some(g)) if COMPOSITION_GRAINS.contains(&g.as_str()) => Some(g.as_str()),
        Some(_) => {
            return Err((
                1,
                format!(
                    "concept-seed: --grain must be one of {}",
                    COMPOSITION_GRAINS.join(", ")
                ),
            ))
        }
    };
    let platform: Option<&str> = match &a.platform {
        None => None,
        Some(Some(p)) if COMPOSITION_PLATFORMS.contains(&p.as_str()) => Some(p.as_str()),
        Some(_) => {
            return Err((
                1,
                format!(
                    "concept-seed: --platform must be one of {}",
                    COMPOSITION_PLATFORMS.join(", ")
                ),
            ))
        }
    };
    if !(a.candidate_count.is_finite() && a.candidate_count.fract() == 0.0)
        || a.candidate_count < 5.0
        || a.candidate_count > 7.0
    {
        return Err((
            1,
            "concept-seed: --candidate-count must be an integer from 5 to 7".into(),
        ));
    }
    let candidate_count = a.candidate_count as usize;
    let key = a.key.as_str();

    let index_salt = if reroll == 0 { "index".to_string() } else { format!("index:reroll-{}", reroll) };
    let build_index = 3 + (unit(scope, &index_salt, key) * (candidate_count as f64 - 2.0)).floor() as usize;
    let mut dealt: Vec<usize> = vec![build_index];
    let want = 3.min(candidate_count);
    let mut draw = 0usize;
    while scope == "surface" && dealt.len() < want {
        let idx = 1 + (unit(scope, &format!("{}:deal-{}", index_salt, draw), key) * candidate_count as f64).floor() as usize;
        if !dealt.contains(&idx) {
            dealt.push(idx);
        }
        if draw > 64 {
            let mut f = 1;
            while dealt.len() < want {
                if !dealt.contains(&f) {
                    dealt.push(f);
                }
                f += 1;
            }
        }
        draw += 1;
    }
    let dealt_str = dealt
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    // Explicit local catalogs fail visibly. The default is the fixed own catalog.
    let local = match env.get("PHOENIX_UI_CATALOG_DIR").filter(|v| !v.is_empty()) {
        Some(dir) => load_local(dir),
        None => builtin_catalog(),
    }
    .map_err(|message| (2, message))?;
    let sel = select_approved_challengers(scope, key, reroll, mode, &local.concepts)
        .map_err(|message| (1, message))?;
    let comps = select_approved_compositions(
        scope,
        key,
        reroll,
        mode,
        grain,
        platform,
        &local.compositions,
        3,
    );
    let data = RollData {
        catalog_version: local.catalog_version,
        source: "local".into(),
        pool_revision: approved_pool_revision(&local.concepts),
        approved_count: sel.approved.len().to_string(),
        catalog_count: local.concepts.len().to_string(),
        challengers: sel.picks,
        compositions: comps.picks,
        composition_match: Some(comps.match_),
    };

    let mode_flag = mode.map(|m| format!(" --mode {}", m)).unwrap_or_default();
    let reroll_flag = if reroll > 0 { format!(" --reroll {}", reroll) } else { String::new() };
    let register_flag = register.map(|r| format!(" --register {}", r)).unwrap_or_default();
    let mode_or_unscoped = mode.unwrap_or("unscoped").to_string();
    let scope_upper = scope.to_uppercase();
    let build_index_s = build_index.to_string();
    let count_s = candidate_count.to_string();
    let common: Vec<(&str, &str)> = vec![
        ("SCOPE_UPPER", &scope_upper),
        ("SCOPE", scope),
        ("KEY", key),
        ("MODE_OR_UNSCOPED", &mode_or_unscoped),
        ("MODE_FLAG", &mode_flag),
        ("REROLL_FLAG", &reroll_flag),
        ("REGISTER_FLAG", &register_flag),
        ("CANDIDATECOUNT", &count_s),
        ("BUILDINDEX", &build_index_s),
        ("DEALT_INDICES", &dealt_str),
    ];
    let promoted = if scope == "direction" { fill(t::PROMOTED_DIRECTION, &common) } else { fill(t::PROMOTED_SURFACE, &common) };
    let challenger_instruction = if scope == "direction" { t::CHALLENGER_DIRECTION.to_string() } else { t::CHALLENGER_SURFACE.to_string() };
    let authority = if scope == "direction" { t::AUTHORITY_DIRECTION.to_string() } else { t::AUTHORITY_SURFACE.to_string() };
    // MODE RULES ride after RICHNESS (after AUTHORITY on the degraded paths,
    // which carry no RICHNESS), on every round including re-rolls.
    let mode_block = mode.map(|m| format!("\n{}", mode_rules_block(env, cwd, m))).unwrap_or_default();
    let richness = format!("{}{}", t::RICHNESS, mode_block);
    let assigned_or_dealt = if scope == "direction" { format!("ASSIGNED INDEX: {}", build_index) } else { format!("DEALT INDICES: {} (index {} leads)", dealt_str, build_index) };
    let restated_assigned_or_dealt = if scope == "direction" {
        fill(t::RESTATED_DIRECTION, &common)
    } else {
        fill(t::RESTATED_SURFACE, &common)
    };

    let compositions_enabled = env
        .get("PHOENIX_UI_COMPOSITIONS")
        .map(|v| v == "1")
        .unwrap_or(false);
    let compositions: Vec<Value> = if compositions_enabled {
        data.compositions.clone()
    } else {
        vec![]
    };
    let grain_note = match &data.composition_match {
        Some(m) if m.grain.is_some() => {
            let g = m.grain.clone().unwrap();
            let comp_len = compositions.len().to_string();
            if m.grain_available == Some(0) {
                fill(t::GRAIN_NONE_AVAIL, &[("MATCH_GRAIN", &g)])
            } else if m.at_grain == Some(0) {
                fill(t::GRAIN_NONE_AT, &[("MATCH_GRAIN", &g), ("MATCH_GRAINAVAILABLE", &m.grain_available.unwrap_or(0).to_string())])
            } else if m.at_grain.unwrap_or(0) < compositions.len() {
                fill(t::GRAIN_PARTIAL, &[("MATCH_ATGRAIN", &m.at_grain.unwrap_or(0).to_string()), ("COMPOSITIONS_LENGTH", &comp_len), ("MATCH_GRAIN", &g)])
            } else {
                String::new()
            }
        }
        _ => String::new(),
    };
    let composition_block = if !compositions.is_empty() {
        let header = if scope == "direction" {
            "FIRST-SURFACE COMPOSITION INPUTS (identity-free; test them with shortlisted worlds and keep world plus composition one decision):"
        } else {
            "COMPOSITION CHALLENGERS (identity-free; dress them in the committed visual identity before judging):"
        };
        let rendered: Vec<String> = compositions.iter().enumerate().map(|(i, c)| render_composition(c, Some(i))).collect();
        fill(t::COMPOSITION_BLOCK, &[("COMPOSITION_HEADER", header), ("COMPOSITIONS_RENDERED", &rendered.join("\n")), ("GRAINNOTE", &grain_note)])
    } else {
        String::new()
    };
    let reroll_block = if reroll > 0 {
        let tag = register.map(|r| format!(" ({} REGISTER, user-requested)", r.to_uppercase())).unwrap_or_default();
        let derive = if register.is_some() { "" } else { t::REROLL_DERIVE_TEXT };
        fill(t::REROLL_BLOCK, &[("REROLL", &reroll.to_string()), ("REROLL_REGISTER_TAG", &tag), ("REROLL_DERIVE", derive)])
    } else {
        String::new()
    };
    let assigned_block = match register {
        None => fill(
            t::ASSIGNED_BLOCK,
            &[
                ("ASSIGNED_OR_DEALT", &assigned_or_dealt),
                ("PROMOTEDINSTRUCTION", &promoted),
            ],
        ),
        Some("safer") => t::SAFER_BLOCK.to_string(),
        _ => t::BOLDER_BLOCK.to_string(),
    };
    let round_challenger_instruction = if register == Some("bolder") { t::BOLDER_CHALLENGER.to_string() } else { challenger_instruction };
    let challenger_section = if register == Some("safer") {
        String::new()
    } else {
        let rendered: Vec<String> = data
            .challengers
            .iter()
            .enumerate()
            .map(|(i, c)| render_challenger(c, i))
            .collect();
        fill(
            t::CHALLENGER_SECTION,
            &[
                ("CHALLENGERS_RENDERED", &rendered.join("\n")),
                ("COMPOSITIONBLOCK", &composition_block),
                ("ROUNDCHALLENGERINSTRUCTION", &round_challenger_instruction),
            ],
        )
    };
    let restated = match register {
        None => restated_assigned_or_dealt,
        Some(r) => fill(t::RESTATED_REGISTER, &[("REGISTER", r), ("KEY", key)]),
    };
    let mut pairs = common.clone();
    pairs.push(("DATA_SOURCE", &data.source));
    pairs.push(("DATA_CATALOGVERSION", &data.catalog_version));
    pairs.push(("DATA_POOLREVISION", &data.pool_revision));
    pairs.push(("DATA_APPROVEDCOUNT", &data.approved_count));
    pairs.push(("DATA_CATALOGCOUNT", &data.catalog_count));
    pairs.push(("REROLLBLOCK", &reroll_block));
    pairs.push(("ASSIGNEDBLOCK", &assigned_block));
    pairs.push(("CHALLENGERSECTION", &challenger_section));
    pairs.push(("AUTHORITYINSTRUCTION", &authority));
    pairs.push(("RICHNESSINSTRUCTION", &richness));
    pairs.push(("RESTATED", &restated));
    let text = fill(t::MAIN, &pairs);
    if data.challengers.len() < 6 {
        let available = data.challengers.len();
        let references: Vec<&str> = ["PRODUCT.md", "DESIGN.md"]
            .into_iter()
            .filter(|name| std::path::Path::new(cwd).join(name).is_file())
            .collect();
        let list: Vec<String> = data
            .challengers
            .iter()
            .enumerate()
            .map(|(i, c)| render_challenger(c, i))
            .collect();
        let guidance = if register == Some("bolder") && available == 0 {
            "No eligible challenger leads this round. Choose existing project references or ask for a new direction."
        } else {
            "Use only the available candidates and existing project references; do not claim a complete catalog draw."
        };
        return Ok(format!("INSUFFICIENT: status=insufficient; requested=6; available={available}; source=local; catalog={}; seed={key}; reroll={reroll}; mode={mode_or_unscoped}; pool={}\nAvailable project references: {}\n{}\n{}\n{}\n{}\n", data.catalog_version, data.pool_revision,
            references.join(", "), list.join("\n"), guidance, authority, mode_block));
    }
    Ok(text)
}

fn random_hex8() -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let mut x = (t as u64) ^ ((std::process::id() as u64) << 32) ^ 0x9E3779B97F4A7C15;
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    format!("{:08x}", (x & 0xffffffff) as u32)
}

pub fn run(args: &[String], io: &mut Io) -> i32 {
    let cwd = io.cwd.to_string_lossy().into_owned();
    let env = io.env.clone();
    let idx = |name: &str| args.iter().position(|a| a == name);
    // args[idx+1] may be undefined -> None
    let val = |name: &str| -> Option<Option<String>> { idx(name).map(|i| args.get(i + 1).cloned()) };
    let chosen = val("--chosen");
    let kind = val("--kind");
    if chosen.is_some() || kind.is_some() {
        io.out("choice kept local; telemetry disabled\n");
        return 0;
    }
    let ctx = load_context(&cwd, &TargetOptions::default(), &env);
    if !ctx.has_product {
        io.out("NO_PRODUCT_MD: the dice stay in the cup until product truth exists. Complete the init ask round and write PRODUCT.md first (reference/init.md), then re-run this exact command. Challengers fuse their form with facts from PRODUCT.md; without it every direction is ungrounded.\n");
        return 1;
    }
    let num = |v: Option<Option<String>>| -> Option<f64> { v.map(|x| x.map(|s| crate::critique_storage::js_number(&s)).unwrap_or(f64::NAN)) };
    let seed = SeedArgs {
        scope: match val("--scope") {
            None => Some("surface".to_string()),
            Some(v) => v, // undefined value -> None -> invalid scope
        },
        key: match val("--from") {
            Some(Some(k)) => k,
            Some(None) => "undefined".to_string(),
            None => env
                .get("PHOENIX_UI_CONCEPT_SEED")
                .filter(|v| !v.is_empty())
                .cloned()
                .unwrap_or_else(random_hex8),
        },
        reroll: num(val("--reroll")).unwrap_or(0.0),
        register: val("--register"),
        mode: val("--mode"),
        grain: val("--grain"),
        platform: val("--platform"),
        candidate_count: num(val("--candidate-count")).unwrap_or(7.0),
    };
    match render_concept_seed(&env, &cwd, &seed) {
        Ok(text) => {
            io.out(&text);
            0
        }
        Err((code, msg)) => {
            io.err(&format!("{}\n", msg));
            code
        }
    }
}

#[cfg(test)]
mod local_catalog_tests;

#[cfg(test)]
mod tests {
    use super::*;

    const MODE_FILE: &str = "# Operate mode\n\nIntro the engine ignores.\n\n## Directions\n\n- Keep standard navigation.\n\n  Indented continuation.\n\n## Comps\n- Show a working state.\n### Sub heading stays\n\n";

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("impeccable-seed-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A project with PRODUCT.md plus a skill dir holding the given mode files.
    fn fixture(name: &str, files: &[(&str, &str)]) -> (std::path::PathBuf, std::path::PathBuf) {
        let root = temp_dir(name);
        let proj = root.join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join("PRODUCT.md"), "# Product\n\n## Users\n\nPeople.\n").unwrap();
        let skill = root.join("skill");
        std::fs::create_dir_all(skill.join("reference")).unwrap();
        for (file, body) in files {
            std::fs::write(skill.join("reference").join(format!("{}.md", file)), body).unwrap();
        }
        (proj, skill)
    }

    fn seed(proj: &std::path::Path, skill: &std::path::Path, catalog: bool, args: &[&str]) -> (i32, String) {
        let catalog_dir = if catalog {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/concept-catalog")
        } else {
            proj.join("no-such-catalog")
        };
        let env = Env::from([
            (
                "PHOENIX_UI_SKILL_DIR".into(),
                skill.to_string_lossy().into_owned(),
            ),
            (
                "PHOENIX_UI_CATALOG_DIR".into(),
                catalog_dir.to_string_lossy().into_owned(),
            ),
            ("PHOENIX_UI_API_URL".into(), "http://127.0.0.1:9/api".into()),
            ("PHOENIX_UI_API_TIMEOUT".into(), "300".into()),
            ("PHOENIX_UI_NO_TELEMETRY".into(), "1".into()),
            ("PHOENIX_UI_NO_STALENESS_CHECK".into(), "1".into()),
        ]);
        let (mut io, cap) = Io::captured("", proj.to_path_buf(), env);
        let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let code = run(&argv, &mut io);
        let out = String::from_utf8(cap.stdout.borrow().clone()).unwrap();
        (code, out.replace('\\', "/"))
    }

    #[test]
    fn extracts_both_sections_verbatim_and_trimmed() {
        assert_eq!(extract_section(MODE_FILE, "Directions").unwrap(), "- Keep standard navigation.\n\n  Indented continuation.");
        assert_eq!(extract_section(MODE_FILE, "Comps").unwrap(), "- Show a working state.\n### Sub heading stays");
    }

    #[test]
    fn missing_or_empty_section_is_none() {
        assert_eq!(extract_section("# T\n\n## Directions\n\nx\n", "Comps"), None);
        assert_eq!(extract_section("# T\n\n## Directions\n\n\n## Comps\ny\n", "Directions"), None);
        assert_eq!(extract_section("# T\n\n### Directions\nx\n", "Directions"), None);
    }

    #[test]
    fn crlf_files_extract_like_lf() {
        let crlf = MODE_FILE.replace('\n', "\r\n");
        assert_eq!(extract_section(&crlf, "Directions"), extract_section(MODE_FILE, "Directions"));
        assert_eq!(extract_section(&crlf, "Comps"), extract_section(MODE_FILE, "Comps"));
    }

    #[test]
    fn mode_files_map_experience_to_persuade() {
        assert_eq!(mode_rules_file("persuade"), "mode-persuade");
        assert_eq!(mode_rules_file("experience"), "mode-persuade");
        assert_eq!(mode_rules_file("operate"), "mode-operate");
        assert_eq!(mode_rules_file("read"), "mode-read");
    }

    #[test]
    fn block_for_each_mode_names_its_file_and_bodies() {
        let files = [
            ("mode-persuade", "# P\n\n## Directions\nP directions.\n\n## Comps\nP comps.\n"),
            ("mode-operate", MODE_FILE),
            ("mode-read", "# R\r\n\r\n## Directions\r\nR directions.\r\n## Comps\r\nR comps.\r\n"),
        ];
        let (proj, skill) = fixture("each-mode", &files);
        let skill_s = skill.to_string_lossy().replace('\\', "/");
        for (mode, file, dir, comps) in [
            ("persuade", "mode-persuade", "P directions.", "P comps."),
            ("experience", "mode-persuade", "P directions.", "P comps."),
            ("operate", "mode-operate", "- Keep standard navigation.\n\n  Indented continuation.", "- Show a working state.\n### Sub heading stays"),
            ("read", "mode-read", "R directions.", "R comps."),
        ] {
            let (code, out) = seed(&proj, &skill, true, &["--scope", "direction", "--mode", mode, "--from", "k1"]);
            assert_eq!(code, 0, "{out}");
            let block = format!(
                "\nMODE RULES ({}, from {}/reference/{}.md). They govern this surface's directions and every comp you write or judge for it, the decision comps included; where shared guidance conflicts, these win.\nDIRECTIONS\n{}\nCOMPS\n{}\nA user- or brief-pinned decision beats the roll, always.\n",
                mode, skill_s, file, dir, comps
            );
            assert!(out.contains(&block), "{mode}: {out}");
            assert!(out.contains("behavior.\nMODE RULES ("), "block follows RICHNESS: {out}");
        }
    }

    #[test]
    fn no_mode_prints_no_block_and_richness_is_mode_neutral() {
        let (proj, skill) = fixture("no-mode", &[("mode-persuade", MODE_FILE)]);
        let (code, out) = seed(&proj, &skill, true, &["--scope", "direction", "--from", "k1"]);
        assert_eq!(code, 0, "{out}");
        assert!(!out.contains("MODE RULES"), "{out}");
        assert!(!out.contains("Persuade") && !out.contains("Operate or Read"), "{out}");
        assert!(out.contains("Keep a literal carrier only when it\nbecomes functional."), "{out}");
    }

    #[test]
    fn missing_file_or_section_prints_the_fallback_and_still_rolls() {
        let (proj, skill) = fixture("missing", &[("mode-operate", "# O\n\n## Directions\nonly directions\n")]);
        let skill_s = skill.to_string_lossy().replace('\\', "/");
        for (mode, file) in [("operate", "mode-operate"), ("read", "mode-read")] {
            let (code, out) = seed(&proj, &skill, true, &["--scope", "surface", "--mode", mode, "--from", "k1"]);
            assert_eq!(code, 0, "{out}");
            assert!(out.contains(&format!("\nMODE RULES unavailable: read {}/reference/{}.md before writing directions or comps.\n", skill_s, file)), "{out}");
            assert!(!out.contains("DIRECTIONS\n"), "{out}");
        }
    }

    #[test]
    fn rerolls_carry_the_block_and_missing_catalogs_fail() {
        let (proj, skill) = fixture("reroll", &[("mode-persuade", MODE_FILE)]);
        let (_, out) = seed(
            &proj,
            &skill,
            true,
            &[
                "--scope",
                "direction",
                "--mode",
                "persuade",
                "--from",
                "k1",
                "--reroll",
                "2",
                "--register",
                "bolder",
            ],
        );
        assert!(out.contains("MODE RULES (persuade, from "), "{out}");
        let (code, _) = seed(
            &proj,
            &skill,
            false,
            &[
                "--scope",
                "direction",
                "--mode",
                "experience",
                "--from",
                "k1",
            ],
        );
        assert_eq!(
            code, 2,
            "missing catalog must not contact a remote roll service"
        );
    }
}
