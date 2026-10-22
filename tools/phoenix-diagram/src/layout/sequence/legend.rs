use super::*;

pub(super) struct Entry {
    kind: String,
    label: String,
    raw: Value,
    width: f64,
}

fn translated(layout: &Layout<'_>, key: &str, fallback: &str) -> String {
    if !layout.legacy && !string(&layout.document["meta"], "locale").is_empty() {
        if let Some(value) = layout.document["meta"]["translations"][key]
            .as_str()
            .filter(|value| !value.is_empty())
        {
            let placeholder = value.split('{').skip(1).any(|piece| {
                piece.split_once('}').is_some_and(|(name, _)| {
                    !name.is_empty()
                        && name
                            .chars()
                            .all(|character| character.is_ascii_alphanumeric() || character == '_')
                })
            });
            if !placeholder {
                return value.into();
            }
        }
    }
    fallback.into()
}

pub(super) fn entries(layout: &Layout<'_>) -> Vec<Entry> {
    let meta = &layout.document["meta"];
    let config = &meta["legend"];
    let mode = config["mode"].as_str().unwrap_or("auto");
    if mode == "hidden" {
        return vec![];
    }
    let locale = string(meta, "locale").to_lowercase();
    let labels = match locale.as_str() {
        "zh-cn" if layout.legacy => ["请求", "返回", "安全", "异步追踪", "默认消息"],
        "zh-cn" => ["主路径请求", "返回", "安全", "异步追踪", "普通消息"],
        "zh-tw" if !layout.legacy => ["主路徑請求", "回傳", "安全", "非同步追蹤", "一般訊息"],
        "es" if !layout.legacy => [
            "solicitud principal",
            "respuesta",
            "seguridad",
            "traza asíncrona",
            "mensaje",
        ],
        "ko" if !layout.legacy => ["주요 요청", "반환", "보안", "비동기 추적", "메시지"],
        _ if layout.legacy => [
            "request",
            "return",
            "security",
            "async trace",
            "default message",
        ],
        _ => [
            "main request",
            "return",
            "security",
            "async trace",
            "message",
        ],
    };
    ["emphasis", "return", "security", "dashed", "default"]
        .iter()
        .enumerate()
        .filter_map(|(index, kind)| {
            let raw = &config["entries"][kind];
            let present = list(&layout.document["messages"])
                .iter()
                .any(|message| message["variant"].as_str().unwrap_or("default") == *kind);
            if raw["visible"] != true && (!(mode == "all" || present) || raw["visible"] == false) {
                return None;
            }
            let fallback = translated(layout, &format!("legend.sequence.{kind}"), labels[index]);
            let label = raw["label"]
                .as_str()
                .filter(|value| !value.is_empty())
                .unwrap_or(&fallback)
                .to_owned();
            let measured_font = if layout.legacy { 8.0 } else { 10.0 };
            let width = (34.0 + 9.0 + layout.units(&label) * measured_font * 0.62).ceil();
            Some(Entry {
                kind: (*kind).into(),
                label,
                raw: raw.clone(),
                width,
            })
        })
        .collect()
}

fn rows(entries: &[Entry], width: f64) -> Vec<Vec<usize>> {
    if entries.is_empty() {
        return vec![];
    }
    let mut result = vec![vec![]];
    let mut cursor = 0.0;
    for (index, entry) in entries.iter().enumerate() {
        let row = result.last_mut().unwrap();
        let required = entry.width + if row.is_empty() { 0.0 } else { 22.0 };
        if !row.is_empty() && cursor + required > width {
            result.push(vec![index]);
            cursor = entry.width;
        } else {
            row.push(index);
            cursor += required;
        }
    }
    result
}

pub(super) fn required_height(layout: &Layout<'_>, width: f64) -> f64 {
    if layout.legend_entries.is_empty() {
        return 0.0;
    }
    let rows = rows(&layout.legend_entries, width - 80.0);
    (layout.content_bottom + 12.0 + 86.0 + rows.len().saturating_sub(1) as f64 * 22.0).ceil()
}

pub(super) fn measure(layout: &Layout<'_>) -> (Vec<Card>, Diagnostics) {
    let entries = &layout.legend_entries;
    if entries.is_empty() {
        return (vec![], vec![]);
    }
    let rows = rows(entries, layout.width - 80.0);
    let baseline = layout.height - 54.0;
    let title_y = baseline - rows.len().saturating_sub(1) as f64 * 22.0 - 20.0;
    let too_wide = entries
        .iter()
        .find(|entry| entry.width > layout.width - 80.0);
    let min_top = if layout.legacy {
        baseline - 30.0
    } else {
        142.0_f64.max(layout.content_bottom + 12.0)
    };
    let too_high = title_y - 10.0 < min_top;
    if too_wide.is_some() || too_high {
        if layout.document["meta"].get("legend").is_none() {
            return (vec![], vec![]);
        }
        let code = if too_wide.is_some() {
            "legend/label-too-wide"
        } else {
            "legend/vertical-overflow"
        };
        let evidence = if let Some(entry) = too_wide {
            json!({"kind":entry.kind,"measuredWidthPx":entry.width,"availableWidthPx":layout.width-80.0})
        } else {
            json!({"rowCount":rows.len(),"requiredTopY":title_y-10.0,"availableTopY":min_top})
        };
        let path = too_wide.map_or_else(
            || "/meta/legend".into(),
            |entry| format!("/meta/legend/entries/{}/label", entry.kind),
        );
        return(vec![],vec![diagnostic(code,&path,"Sequence legend does not fit the authored canvas",evidence,vec!["Provide sufficient canvas width/height while preserving every message and legend meaning".into()])]);
    }
    let locale = string(&layout.document["meta"], "locale").to_lowercase();
    let title = translated(
        layout,
        "legend.title",
        match locale.as_str() {
            "zh-cn" => "图例",
            "zh-tw" if !layout.legacy => "圖例",
            "es" if !layout.legacy => "Leyenda",
            "ko" if !layout.legacy => "범례",
            _ => "Legend",
        },
    );
    let mut cards = vec![Card {
        authored: authored(
            &layout.document["meta"]["legend"],
            "/meta/legend".into(),
            "legend-title".into(),
            "legend-title",
            vec!["resolved-legend-footprint".into()],
        ),
        bounds: Rect {
            x: 40.0,
            y: title_y - 10.0,
            width: 48.0,
            height: 14.0,
        },
        text: vec![text(
            "/meta/legend".into(),
            &title,
            40.0,
            title_y,
            12.0,
            "start",
        )],
    }];
    for (row_index, row) in rows.iter().enumerate() {
        let mut x = 40.0;
        let y = baseline - (rows.len() - row_index - 1) as f64 * 22.0;
        for index in row {
            let entry = &entries[*index];
            cards.push(Card {
                authored: authored(
                    &entry.raw,
                    format!("/meta/legend/entries/{}", entry.kind),
                    format!("legend-{}", entry.kind),
                    &entry.kind,
                    vec!["resolved-legend-footprint".into()],
                ),
                bounds: Rect {
                    x,
                    y: y - 10.0,
                    width: entry.width,
                    height: 14.0,
                },
                text: vec![text(
                    format!("/meta/legend/entries/{}/label", entry.kind),
                    &entry.label,
                    x + 43.0,
                    y,
                    10.0,
                    "start",
                )],
            });
            x += entry.width + 22.0;
        }
    }
    (cards, vec![])
}
