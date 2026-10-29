use crate::{
    check::FileIdentity,
    diagnostic::{Diagnostic, Diagnostics},
    evidence::SourceEvidence,
    input::DiagramType,
    render::escape_text,
    scene::CompiledDiagram,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fmt::Write};

#[derive(Debug)]
pub struct ParsedComparison {
    pub ir: Value,
    pub receipt_id: String,
    pub receipt_filename: String,
    pub views: BTreeMap<String, Vec<u8>>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Metadata {
    schema_version: u32,
    artifact_kind: String,
    receipt_filename: String,
    receipt_id: String,
    comparison_ir: Value,
    view_identities: BTreeMap<String, FileIdentity>,
}
fn failure(message: impl Into<String>) -> Diagnostics {
    let mut diagnostic = Diagnostic::error(Some("architecture"), "compare/html", "", message);
    diagnostic.stage = "compare".into();
    vec![diagnostic]
}
fn safe_filename(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !matches!(value, "." | "..")
        && !value.ends_with(['.', ' '])
        && !value
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '?' | '#'))
}
fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
fn selector(value: &str) -> Selector {
    Selector::parse(value).expect("fixed comparison selector")
}
pub fn parse(html: &[u8]) -> Result<ParsedComparison, Diagnostics> {
    let html = std::str::from_utf8(html).map_err(|_| failure("Comparison HTML must be UTF-8"))?;
    let document = Html::parse_document(html);
    let metadata_selector = selector("script#phoenix-diagram-comparison[type='application/json']");
    let mut scripts = document.select(&metadata_selector);
    let script = scripts
        .next()
        .ok_or_else(|| failure("Comparison metadata is missing"))?;
    if scripts.next().is_some() {
        return Err(failure("Comparison metadata is ambiguous"));
    }
    let metadata: Metadata = serde_json::from_str(&script.text().collect::<String>())
        .map_err(|_| failure("Comparison metadata is invalid"))?;
    if metadata.schema_version != 1
        || metadata.artifact_kind != "architecture-comparison"
        || !safe_filename(&metadata.receipt_filename)
        || !safe_id(&metadata.receipt_id)
    {
        return Err(failure("Comparison identity is invalid"));
    }
    let frame_selector = selector("iframe");
    let frames: Vec<_> = document.select(&frame_selector).collect();
    if frames.len() != 3 || metadata.view_identities.len() != 3 {
        return Err(failure("Comparison must contain exactly three bound views"));
    }
    let mut views = BTreeMap::new();
    for frame in frames {
        let id = frame
            .value()
            .attr("data-comparison-view")
            .ok_or_else(|| failure("Comparison view identity is missing"))?;
        if !matches!(id, "before" | "delta" | "after") || frame.value().attr("src").is_some() {
            return Err(failure("Comparison view is unsupported or external"));
        }
        let embedded = frame
            .value()
            .attr("srcdoc")
            .ok_or_else(|| failure("Comparison view content is missing"))?;
        let source = if let Some(encoded) = frame.value().attr("data-comparison-original-base64") {
            let original = STANDARD
                .decode(encoded)
                .map_err(|_| failure("Comparison original view bytes are invalid"))?;
            let text = std::str::from_utf8(&original)
                .map_err(|_| failure("Comparison original view must be UTF-8"))?;
            if !original.contains(&0)
                || STANDARD.encode(&original) != encoded
                || text.replace('\0', "\u{fffd}") != embedded
            {
                return Err(failure(
                    "Comparison original view does not match its normalized srcdoc",
                ));
            }
            original
        } else {
            embedded.as_bytes().to_vec()
        };
        if metadata.view_identities.get(id) != Some(&FileIdentity::of(&source))
            || views.insert(id.into(), source).is_some()
        {
            return Err(failure(
                "Comparison view bytes do not match their bound identity",
            ));
        }
    }
    if document
        .select(&selector("script[src],link[href]"))
        .next()
        .is_some()
    {
        return Err(failure("Comparison shell must be self-contained"));
    }
    Ok(ParsedComparison {
        ir: metadata.comparison_ir,
        receipt_id: metadata.receipt_id,
        receipt_filename: metadata.receipt_filename,
        views,
    })
}
pub fn views(html: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, Diagnostics> {
    Ok(parse(html)?.views)
}

fn safe_json(value: &impl Serialize) -> String {
    serde_json::to_string(value)
        .expect("comparison metadata serializes")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}
fn srcdoc(bytes: &[u8]) -> Result<String, Diagnostics> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| failure("Comparison snapshots must be UTF-8"))?;
    Ok(escape_text(text)
        .replace('\0', "\u{fffd}")
        .replace('\r', "&#13;"))
}
fn source_evidence(html: &[u8]) -> Result<SourceEvidence, Diagnostics> {
    let text =
        std::str::from_utf8(html).map_err(|_| failure("Comparison snapshot must be UTF-8"))?;
    let doc = Html::parse_document(text);
    let sel = selector("script#phoenix-diagram-metadata[type='application/json']");
    let mut scripts = doc.select(&sel);
    let script = scripts
        .next()
        .ok_or_else(|| failure("Comparison snapshot metadata is missing"))?;
    if scripts.next().is_some() {
        return Err(failure("Comparison snapshot metadata is ambiguous"));
    }
    let meta: Value = serde_json::from_str(&script.text().collect::<String>())
        .map_err(|_| failure("Comparison snapshot metadata is invalid"))?;
    serde_json::from_value(meta["sourceEvidence"].clone())
        .map_err(|_| failure("Comparison snapshot evidence is invalid"))
}

fn text(locale: &str, key: &str) -> &'static str {
    let index = match locale {
        "zh-CN" => 1,
        "zh-TW" => 2,
        "es" => 3,
        "ko" => 4,
        _ => 0,
    };
    const LABELS:&[(&str,[&str;5])]=&[
        ("title",["Architecture comparison","架构比较","架構比較","Comparación de arquitectura","아키텍처 비교"]),
        ("before",["Before","之前","之前","Antes","이전"]),("delta",["Delta","变化","變更","Cambios","변경"]),("after",["After","之后","之後","Después","이후"]),
        ("snapshot",["Architecture snapshot","架构视图","架構檢視","Vista de arquitectura","아키텍처 보기"]),
        ("changes",["Exact authored changes","输入中的确切变化","輸入中的確切變更","Cambios exactos declarados","작성된 정확한 변경"]),
        ("empty",["No authored architecture changes.","输入中没有架构变化。","輸入中沒有架構變更。","No hay cambios de arquitectura declarados.","작성된 아키텍처 변경이 없습니다."]),
        ("provenance",["Repository provenance changed","仓库来源信息已变化","儲存庫來源資訊已變更","Cambió la procedencia del repositorio","저장소 출처 정보가 변경됨"]),
        ("presentation",["Presentation settings changed","展示设置已变化","呈現設定已變更","Cambió la presentación","표시 설정이 변경됨"]),
        ("authored",["Authored snapshots","作者提供的快照","作者提供的快照","Instantáneas declaradas","작성된 스냅샷"]),
        ("pinned",["Revision-pinned inputs","已固定修订的输入","已固定修訂的輸入","Entradas con revisión fijada","리비전이 고정된 입력"]),
        ("limits",["Authored facts only; no runtime impact, cause, risk or mergeability is inferred.","仅比较输入事实；不推断运行影响、原因、风险或可合并性。","僅比較輸入事實；不推斷執行影響、原因、風險或可合併性。","Solo hechos declarados; no se infieren efectos, causas, riesgos ni posibilidad de fusión.","작성된 사실만 비교하며 실행 영향, 원인, 위험 또는 병합 가능성을 추론하지 않습니다."]),
        ("theme",["Theme","主题","主題","Tema","테마"]),("overview",["Overview","总览","總覽","Resumen","전체 보기"]),
        ("previous",["Previous change","上一项变化","上一項變更","Cambio anterior","이전 변경"]),("next",["Next change","下一项变化","下一項變更","Cambio siguiente","다음 변경"]),
        ("component",["Component","组件","元件","Componente","구성 요소"]),("relationship",["Relationship","关系","關係","Relación","관계"]),("boundary",["Boundary","边界","邊界","Límite","경계"]),
        ("added",["Added","新增","新增","Añadido","추가됨"]),("removed",["Removed","移除","移除","Eliminado","제거됨"]),("changed",["Changed","改变","變更","Modificado","변경됨"]),
        ("moved",["Moved","移动","移動","Movido","이동됨"]),("rerouted",["Rerouted","重新布线","重新佈線","Redirigido","경로 변경됨"]),
        ("geometry-changed",["Geometry changed","几何变化","幾何變更","Geometría modificada","기하 변경됨"]),("evidence-changed",["Evidence changed","依据变化","依據變更","Evidencia modificada","근거 변경됨"]),
        ("fields",["Field","字段","欄位","Campo","필드"]),("identity",["Identity","身份","身分","Identidad","식별자"]),("undeclared",["Undeclared","未声明","未宣告","No declarado","선언되지 않음"]),
    ];
    LABELS
        .iter()
        .find(|(name, _)| *name == key)
        .expect("fixed comparison label")
        .1[index]
}
fn fact<'a>(scene: &'a CompiledDiagram, kind: &str, id: &str) -> Option<&'a Value> {
    match kind {
        "component" => scene
            .nodes
            .iter()
            .find(|node| node.authored.id == id)
            .map(|node| &node.authored.facts),
        "relationship" => scene
            .relations
            .iter()
            .find(|relation| relation.authored.id == id)
            .map(|relation| &relation.authored.facts),
        _ => scene
            .containers
            .iter()
            .find(|container| {
                format!(
                    "{}:{}",
                    container.authored.kind,
                    container.authored.facts["label"]
                        .as_str()
                        .unwrap_or_default()
                ) == id
            })
            .map(|container| &container.authored.facts),
    }
}
fn field_value(value: Option<&Value>, locale: &str) -> String {
    value
        .map(|value| escape_text(&serde_json::to_string_pretty(value).expect("authored value")))
        .unwrap_or_else(|| escape_text(text(locale, "undeclared")))
}
fn rows(base: &CompiledDiagram, head: &CompiledDiagram, ir: &Value, locale: &str) -> String {
    let mut changes = vec![];
    for (collection, kind, sort_kind) in [
        ("components", "component", "Component"),
        ("connections", "relationship", "Relationship"),
        ("boundaries", "boundary", "Boundary"),
    ] {
        for change in ir["changes"][collection].as_array().into_iter().flatten() {
            let id = change[if kind == "boundary" { "key" } else { "id" }]
                .as_str()
                .unwrap_or_default();
            changes.push((
                format!(
                    "{}:{sort_kind}:{id}",
                    change["status"].as_str().unwrap_or_default()
                ),
                kind,
                id,
                change,
            ));
        }
    }
    changes.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
    if changes.is_empty() {
        return format!(
            "<li data-comparison-empty>{}</li>",
            escape_text(text(locale, "empty"))
        );
    }
    let mut html = String::new();
    for (_, kind, id, change) in changes {
        let state = change["status"].as_str().unwrap_or("changed");
        let label = change["headLabel"]
            .as_str()
            .or_else(|| change["baseLabel"].as_str())
            .or_else(|| change["head"]["label"].as_str())
            .or_else(|| change["base"]["label"].as_str())
            .or_else(|| change["label"].as_str())
            .filter(|value| !value.is_empty())
            .unwrap_or(id);
        let key = format!("{kind}:{id}");
        write!(html,"<li><button type=\"button\" class=\"change-row\" data-comparison-change=\"{}\" data-comparison-kind=\"{kind}\" data-comparison-id=\"{}\" data-comparison-label=\"{}\"><span class=\"change-state\" data-state=\"{}\">{}</span><span>{}</span><strong>{}</strong><code>{}</code></button><details class=\"field-details\"><summary>{}</summary><table><thead><tr><th>{}</th><th>{}</th><th>{}</th></tr></thead><tbody>",escape_text(&key),escape_text(id),escape_text(label),escape_text(state),escape_text(text(locale,state)),escape_text(text(locale,kind)),escape_text(label),escape_text(id),escape_text(text(locale,"fields")),escape_text(text(locale,"fields")),escape_text(text(locale,"before")),escape_text(text(locale,"after"))).unwrap();
        let fields: Vec<_> = change["changedFields"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if fields.is_empty() {
            write!(
                html,
                "<tr><th>{}</th><td><pre>{}</pre></td><td><pre>{}</pre></td></tr>",
                escape_text(text(locale, "identity")),
                field_value(fact(base, kind, id), locale),
                field_value(fact(head, kind, id), locale)
            )
            .unwrap();
        }
        for field in fields {
            write!(
                html,
                "<tr><th><code>{}</code></th><td><pre>{}</pre></td><td><pre>{}</pre></td></tr>",
                escape_text(field),
                field_value(
                    fact(base, kind, id).and_then(|value| value.pointer(field)),
                    locale
                ),
                field_value(
                    fact(head, kind, id).and_then(|value| value.pointer(field)),
                    locale
                )
            )
            .unwrap();
        }
        html.push_str("</tbody></table></details></li>");
    }
    html
}

pub(crate) fn render(
    base: &CompiledDiagram,
    head: &CompiledDiagram,
    base_html: &[u8],
    head_html: &[u8],
    ir: &Value,
    receipt_filename: &str,
    receipt_id: &str,
) -> Result<Vec<u8>, Diagnostics> {
    if base.diagram_type != DiagramType::Architecture
        || head.diagram_type != DiagramType::Architecture
    {
        return Err(failure("Comparison views require architecture scenes"));
    }
    if !safe_filename(receipt_filename) || !safe_id(receipt_id) {
        return Err(failure(
            "Comparison receipt must have a safe relative filename and identity",
        ));
    }
    let delta =
        crate::render::render_comparison_delta(base, head, ir, &source_evidence(head_html)?)?;
    let child_views = BTreeMap::from([
        ("before", base_html),
        ("delta", delta.as_slice()),
        ("after", head_html),
    ]);
    let metadata = Metadata {
        schema_version: 1,
        artifact_kind: "architecture-comparison".into(),
        receipt_filename: receipt_filename.into(),
        receipt_id: receipt_id.into(),
        comparison_ir: ir.clone(),
        view_identities: child_views
            .iter()
            .map(|(id, bytes)| ((*id).into(), FileIdentity::of(bytes)))
            .collect(),
    };
    let locale = crate::i18n::resolve(&head.meta).resolved;
    let mut frames = String::new();
    let mut tabs = String::new();
    for id in ["before", "delta", "after"] {
        let selected = id == "delta";
        write!(tabs,"<button id=\"comparison-tab-{id}\" type=\"button\" role=\"tab\" data-comparison-select=\"{id}\" aria-selected=\"{selected}\" aria-controls=\"comparison-panel-{id}\" tabindex=\"{}\">{}</button>",if selected{"0"}else{"-1"},escape_text(text(&locale,id))).unwrap();
        // HTML normalizes NUL to U+FFFD; retain only those exceptional source
        // bytes separately, while binding their exact normalized live document.
        let original = if child_views[id].contains(&0) {
            format!(
                " data-comparison-original-base64=\"{}\"",
                STANDARD.encode(child_views[id])
            )
        } else {
            String::new()
        };
        write!(frames,"<section id=\"comparison-panel-{id}\" role=\"tabpanel\" aria-labelledby=\"comparison-tab-{id}\"{}><iframe title=\"{} · {}\" data-comparison-view=\"{id}\" sandbox=\"allow-scripts allow-same-origin allow-downloads allow-popups allow-popups-to-escape-sandbox\"{original} srcdoc=\"{}\"></iframe></section>",if selected{""}else{" hidden"},escape_text(text(&locale,"snapshot")),escape_text(text(&locale,id)),srcdoc(child_views[id])?).unwrap();
    }
    let mut notices = String::new();
    for (field, key) in [
        ("provenanceChanged", "provenance"),
        ("presentationChanged", "presentation"),
    ] {
        if ir["summary"][field] == true {
            write!(
                notices,
                "<aside class=\"comparison-notice\" data-comparison-notice=\"{field}\">{}</aside>",
                escape_text(text(&locale, key))
            )
            .unwrap();
        }
    }
    if ir["summary"]["provenanceChanged"] == true {
        write!(
            notices,
            "<details class=\"provenance-details\"><summary>{}</summary><pre>{}</pre></details>",
            escape_text(text(&locale, "provenance")),
            field_value(ir.get("provenance"), &locale)
        )
        .unwrap();
    }
    let replacements = BTreeMap::from([
        ("LOCALE", escape_text(&locale)),
        ("TITLE", escape_text(text(&locale, "title"))),
        (
            "SUBTITLE",
            format!(
                "{} → {}",
                escape_text(base.meta["title"].as_str().unwrap_or_default()),
                escape_text(head.meta["title"].as_str().unwrap_or_default())
            ),
        ),
        (
            "PROOF",
            escape_text(text(
                &locale,
                if ir["proofLevel"] == "revision-pinned" {
                    "pinned"
                } else {
                    "authored"
                },
            )),
        ),
        ("TABS", tabs),
        ("FRAMES", frames),
        ("NOTICES", notices),
        ("ROWS", rows(base, head, ir, &locale)),
        ("METADATA", safe_json(&metadata)),
        ("SCRIPT", include_str!("../../viewer/compare.js").into()),
        ("CHANGES", escape_text(text(&locale, "changes"))),
        ("LIMITS", escape_text(text(&locale, "limits"))),
        ("THEME", escape_text(text(&locale, "theme"))),
        ("OVERVIEW", escape_text(text(&locale, "overview"))),
        ("PREVIOUS", escape_text(text(&locale, "previous"))),
        ("NEXT", escape_text(text(&locale, "next"))),
    ]);
    let template = include_str!("../../assets/compare-template.html");
    let mut rest = template;
    let mut html = String::new();
    while let Some(at) = rest.find("{{COMPARISON:") {
        html.push_str(&rest[..at]);
        rest = &rest[at + 13..];
        let end = rest
            .find("}}")
            .ok_or_else(|| failure("Comparison template slot is unterminated"))?;
        let value = replacements
            .get(&rest[..end])
            .ok_or_else(|| failure("Comparison template slot is unknown"))?;
        html.push_str(value);
        rest = &rest[end + 2..];
    }
    html.push_str(rest);
    let bytes = html.into_bytes();
    parse(&bytes)?;
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compare::canonical::compare_architecture,
        input::{parse_input, CompileOptions, DiagramType, InputContract, Quality},
        layout,
        render::{self, EmbeddedAssets},
    };
    use serde_json::json;
    fn compiled(bytes: &[u8]) -> (CompiledDiagram, Vec<u8>, Value) {
        let options = CompileOptions {
            input_contract: Some(InputContract::PhoenixV1),
            quality: Some(Quality::Standard),
            ..Default::default()
        };
        let input = parse_input(bytes, DiagramType::Architecture, &options).unwrap();
        let scene = layout::compile(&input, &options).unwrap();
        let html = render::render(&scene, &EmbeddedAssets::builtin()).unwrap();
        (scene, html, input.document)
    }
    #[test]
    fn comparison_embeds_three_views_and_preserves_snapshot_bytes() {
        let (base, base_html, base_input) = compiled(include_bytes!(
            "../../tests/fixtures/input/phoenix-v1/checkout-platform.base.architecture.json"
        ));
        let (head, head_html, head_input) = compiled(include_bytes!(
            "../../tests/fixtures/input/phoenix-v1/checkout-platform.head.architecture.json"
        ));
        let ir = compare_architecture(&base_input, &head_input, &json!({})).unwrap();
        let html = super::render(
            &base,
            &head,
            &base_html,
            &head_html,
            &ir,
            "comparison.html.compare.json",
            "receipt-test",
        )
        .unwrap();
        let parsed = parse(&html).unwrap();
        assert_eq!(parsed.views.len(), 3);
        assert_eq!(parsed.views["before"], base_html);
        assert_eq!(parsed.views["after"], head_html);
        assert_eq!(parsed.ir, ir);
        let document = Html::parse_document(std::str::from_utf8(&html).unwrap());
        let expected: std::collections::BTreeSet<_> = [
            "allow-scripts",
            "allow-same-origin",
            "allow-downloads",
            "allow-popups",
            "allow-popups-to-escape-sandbox",
        ]
        .into_iter()
        .collect();
        for frame in document.select(&Selector::parse("iframe").unwrap()) {
            let permissions: std::collections::BTreeSet<_> = frame
                .attr("sandbox")
                .unwrap()
                .split_ascii_whitespace()
                .collect();
            assert_eq!(permissions, expected);
            assert!(frame.attr("src").is_none());
        }
    }
    #[test]
    fn delta_uses_structured_ghost_layers_and_keeps_head_reader_graph_exact() {
        let (base, base_html, base_input) = compiled(include_bytes!(
            "../../tests/fixtures/input/phoenix-v1/checkout-platform.base.architecture.json"
        ));
        let (head, head_html, head_input) = compiled(include_bytes!(
            "../../tests/fixtures/input/phoenix-v1/checkout-platform.head.architecture.json"
        ));
        let ir = compare_architecture(&base_input, &head_input, &json!({})).unwrap();
        let html = super::render(
            &base,
            &head,
            &base_html,
            &head_html,
            &ir,
            "comparison.html.compare.json",
            "receipt-test",
        )
        .unwrap();
        let parsed = parse(&html).unwrap();
        let delta = std::str::from_utf8(&parsed.views["delta"]).unwrap();
        let doc = Html::parse_document(delta);
        let select = |value: &str| Selector::parse(value).unwrap();
        let ids: Vec<_> = doc
            .select(&select("svg [id]"))
            .map(|element| element.attr("id").unwrap().to_owned())
            .collect();
        assert_eq!(
            ids.len(),
            ids.iter().collect::<std::collections::BTreeSet<_>>().len()
        );
        assert!(ids.iter().all(|id| id.starts_with("delta-")));
        let actual: std::collections::BTreeSet<_> = doc
            .select(&select("svg [data-node-id]"))
            .map(|element| element.attr("data-node-id").unwrap().to_owned())
            .collect();
        let expected: std::collections::BTreeSet<_> = head
            .nodes
            .iter()
            .map(|node| node.authored.id.clone())
            .collect();
        assert_eq!(actual, expected);
        let ghosts: Vec<_> = doc.select(&select("[data-delta-ghost='true']")).collect();
        assert!(!ghosts.is_empty());
        assert!(ghosts
            .iter()
            .all(|element| element.attr("data-node-id").is_none()
                && element.attr("data-edge-from").is_none()));
        for (collection, kind) in [
            ("components", "component"),
            ("connections", "relationship"),
            ("boundaries", "boundary"),
        ] {
            for change in ir["changes"][collection].as_array().unwrap() {
                let id = change[if kind == "boundary" { "key" } else { "id" }]
                    .as_str()
                    .unwrap();
                let key = format!("{kind}:{id}");
                let elements: Vec<_> = doc
                    .select(&select("[data-delta-key]"))
                    .filter(|element| element.attr("data-delta-key") == Some(key.as_str()))
                    .collect();
                assert!(!elements.is_empty(), "missing {key}");
                for element in elements {
                    assert_eq!(
                        element.attr("data-delta-state"),
                        change["status"].as_str(),
                        "{key}"
                    );
                }
            }
        }
        for element in doc.select(&select("[marker-end]")) {
            let reference = element.attr("marker-end").unwrap();
            let id = reference
                .strip_prefix("url(#")
                .unwrap()
                .strip_suffix(')')
                .unwrap();
            assert!(
                ids.iter().any(|existing| existing == id),
                "dangling {reference}"
            );
        }
        assert!(doc
            .select(&select("[data-semantic-sigil]"))
            .next()
            .is_some());
        assert!(delta.contains("PhoenixDiagram.view"));
    }
    #[test]
    fn comparison_empty_states_and_five_interface_languages_are_explicit() {
        let (original, _, input) = compiled(include_bytes!(
            "../../tests/fixtures/input/phoenix-v1/checkout-platform.base.architecture.json"
        ));
        let ir = compare_architecture(&input, &input, &json!({})).unwrap();
        for (locale, before) in [
            ("en", "Before"),
            ("zh-CN", "之前"),
            ("zh-TW", "之前"),
            ("es", "Antes"),
            ("ko", "이전"),
        ] {
            let mut scene = original.clone();
            scene.meta["locale"] = json!(locale);
            let snapshot = render::render(&scene, &EmbeddedAssets::builtin()).unwrap();
            let html = super::render(
                &scene,
                &scene,
                &snapshot,
                &snapshot,
                &ir,
                "comparison.html.compare.json",
                "receipt-test",
            )
            .unwrap();
            let doc = Html::parse_document(std::str::from_utf8(&html).unwrap());
            assert_eq!(
                doc.select(&Selector::parse("html").unwrap())
                    .next()
                    .unwrap()
                    .attr("lang"),
                Some(locale)
            );
            let tab = doc
                .select(&Selector::parse("[data-comparison-select='before']").unwrap())
                .next()
                .unwrap();
            assert_eq!(tab.text().collect::<String>(), before);
            assert_eq!(
                doc.select(&Selector::parse("[data-comparison-change]").unwrap())
                    .count(),
                0
            );
            assert_eq!(
                doc.select(&Selector::parse("[data-comparison-empty]").unwrap())
                    .count(),
                1
            );
        }
    }
    #[test]
    fn comparison_rejects_unsafe_receipt_locations_and_escapes_literal_labels() {
        let (mut scene, _, input) = compiled(include_bytes!(
            "../../tests/fixtures/input/phoenix-v1/checkout-platform.base.architecture.json"
        ));
        scene.meta["title"] = json!("</script><script>alert(1)</script> $$ $& url(#grid)");
        let snapshot = render::render(&scene, &EmbeddedAssets::builtin()).unwrap();
        let ir = compare_architecture(&input, &input, &json!({})).unwrap();
        for filename in [
            "../compare.json",
            "/private/compare.json",
            "https://site/compare.json",
            "compare.json#fragment",
            "compare\\private.json",
        ] {
            assert!(super::render(
                &scene,
                &scene,
                &snapshot,
                &snapshot,
                &ir,
                filename,
                "receipt-test"
            )
            .is_err());
        }
        let html = super::render(
            &scene,
            &scene,
            &snapshot,
            &snapshot,
            &ir,
            "comparison.html.compare.json",
            "receipt-test",
        )
        .unwrap();
        let parsed = parse(&html).unwrap();
        let delta = std::str::from_utf8(&parsed.views["delta"]).unwrap();
        assert!(delta.contains("$$ $&amp; url(#grid)"));
        let doc = Html::parse_document(std::str::from_utf8(&html).unwrap());
        assert_eq!(doc.select(&Selector::parse("script").unwrap()).count(), 2);
    }
    #[test]
    fn comparison_preserves_native_literal_nul_bytes_without_changing_live_facts() {
        let (mut scene, _, mut input) = compiled(include_bytes!(
            "../../tests/fixtures/input/phoenix-v1/checkout-platform.base.architecture.json"
        ));
        scene.meta["title"] = json!("Native\0title");
        input["meta"]["title"] = scene.meta["title"].clone();
        let snapshot = render::render(&scene, &EmbeddedAssets::builtin()).unwrap();
        assert!(snapshot.contains(&0));
        let ir = compare_architecture(&input, &input, &json!({})).unwrap();
        let html = super::render(
            &scene,
            &scene,
            &snapshot,
            &snapshot,
            &ir,
            "comparison.html.compare.json",
            "receipt-test",
        )
        .unwrap();
        let parsed = parse(&html).unwrap();
        assert_eq!(parsed.views["before"], snapshot);
        assert_eq!(parsed.views["after"], snapshot);
        let source = std::str::from_utf8(&html).unwrap();
        let doc = Html::parse_document(source);
        for frame in doc.select(&Selector::parse("iframe").unwrap()) {
            assert!(frame.attr("data-comparison-original-base64").is_some());
            assert!(!frame.attr("srcdoc").unwrap().contains('\0'));
        }
        let tampered = source.replacen(
            "data-comparison-original-base64=",
            "data-unbound-original=",
            1,
        );
        assert!(parse(tampered.as_bytes()).is_err());
    }
}
