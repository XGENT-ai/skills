use super::*;

#[derive(Clone)]
pub(super) struct LegendEntry {
    kind: String,
    label: String,
    width: f64,
    swatch: f64,
}
const KINDS: [&str; 8] = [
    "start", "active", "waiting", "decision", "success", "failure", "neutral", "external",
];

fn translation(input: &DiagramInput, key: &str, fallback: &str) -> String {
    if input.input_contract == InputContract::PhoenixV1
        && !string(&input.document["meta"], "locale").is_empty()
    {
        if let Some(value) = input.document["meta"]["translations"][key]
            .as_str()
            .filter(|s| {
                !s.is_empty()
                    && !s.split('{').skip(1).any(|part| {
                        part.split_once('}').is_some_and(|(token, _)| {
                            !token.is_empty()
                                && token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                        })
                    })
            })
        {
            return value.into();
        }
    }
    fallback.into()
}
pub(super) fn legend_entries(
    input: &DiagramInput,
    nodes: &[Node],
    has_final: bool,
) -> Vec<LegendEntry> {
    let config = &input.document["meta"]["legend"];
    let mode = config["mode"].as_str().unwrap_or("auto");
    if mode == "hidden" {
        return vec![];
    }
    let legacy = input.input_contract == InputContract::Legacy217;
    let locale = string(&input.document["meta"], "locale").to_lowercase();
    let labels = match (legacy, locale.as_str()) {
        (true, "zh-cn") => [
            "开始",
            "活动状态",
            "等待",
            "决策",
            "成功终态",
            "失败 / 退出",
            "中性状态",
            "外部状态",
            "终态",
        ],
        (true, _) => [
            "start",
            "active state",
            "waiting",
            "decision",
            "terminal success",
            "failure / exit",
            "neutral",
            "external",
            "final state",
        ],
        (false, "zh-cn") => [
            "起点",
            "进行中",
            "等待",
            "决策",
            "成功",
            "失败 / 退出",
            "中性状态",
            "外部状态",
            "终态",
        ],
        (false, "zh-tw") => [
            "初始狀態",
            "進行中",
            "等待",
            "決策",
            "成功",
            "失敗 / 結束",
            "中性狀態",
            "外部狀態",
            "最終狀態",
        ],
        (false, "es") => [
            "estado inicial",
            "activo",
            "en espera",
            "decisión",
            "éxito",
            "fallo / salida",
            "neutral",
            "externo",
            "estado final",
        ],
        (false, "ko") => [
            "초기 상태",
            "활성",
            "대기",
            "판단",
            "성공",
            "실패 / 종료",
            "중립 상태",
            "외부 상태",
            "종료 상태",
        ],
        _ => [
            "initial state",
            "active",
            "waiting",
            "decision",
            "success",
            "failure / exit",
            "neutral",
            "external",
            "final state",
        ],
    };
    let mut entries = Vec::new();
    for (index, kind) in KINDS.iter().enumerate() {
        let raw = &config["entries"][kind];
        let present =
            nodes.iter().any(|node| node.authored.kind == *kind) || (!legacy && *kind == "start");
        if raw["visible"] != true && (!(mode == "all" || present) || raw["visible"] == false) {
            continue;
        }
        let translated = translation(input, &format!("legend.lifecycle.{kind}"), labels[index]);
        let label = raw["label"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(&translated)
            .to_owned();
        let swatch = if !legacy && *kind == "start" {
            26.0
        } else {
            14.0
        };
        let measured_font = if legacy { 8.0 } else { 10.0 };
        let width = (swatch
            + 8.0
            + units(input, &label) * measured_font * 0.62
            + if present { 21.0 } else { 0.0 })
        .ceil();
        entries.push(LegendEntry {
            kind: (*kind).into(),
            label,
            width,
            swatch,
        });
    }
    if !legacy && !entries.is_empty() && has_final {
        let label = translation(input, "legend.lifecycle.final", labels[8]);
        entries.push(LegendEntry {
            kind: "final".into(),
            width: (16.0 + 8.0 + units(input, &label) * 10.0 * 0.62).ceil(),
            label,
            swatch: 16.0,
        });
    }
    entries
}
pub(super) fn legend_rows(entries: &[LegendEntry], width: f64) -> Vec<Vec<usize>> {
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut cursor = 0.0;
    for (index, entry) in entries.iter().enumerate() {
        if rows.is_empty()
            || (!rows.last().unwrap().is_empty() && cursor + 22.0 + entry.width > width)
        {
            rows.push(vec![index]);
            cursor = entry.width;
        } else {
            cursor += if rows.last().unwrap().is_empty() {
                0.0
            } else {
                22.0
            };
            cursor += entry.width;
            rows.last_mut().unwrap().push(index);
        }
    }
    rows
}
pub(super) fn legend_extra_height(entries: &[LegendEntry], width: f64) -> f64 {
    legend_rows(entries, width).len().saturating_sub(1) as f64 * 22.0
}
pub(super) fn legends(
    input: &DiagramInput,
    entries: &[LegendEntry],
    canvas: Rect,
    baseline: f64,
    min_title: f64,
) -> Result<Vec<Card>, Diagnostics> {
    if entries.is_empty() {
        return Ok(vec![]);
    }
    let rows = legend_rows(entries, canvas.width - 80.0);
    let title_y = baseline - rows.len().saturating_sub(1) as f64 * 22.0 - 20.0;
    let too_wide = entries
        .iter()
        .any(|entry| entry.width > canvas.width - 80.0);
    if too_wide || title_y - 10.0 < min_title {
        if input.document["meta"].get("legend").is_none() {
            return Ok(vec![]);
        }
        return Err(vec![failure(
            if too_wide {
                "legend/label-too-wide"
            } else {
                "legend/vertical-overflow"
            },
            "/meta/legend",
            "The authored legend does not fit the lifecycle legend band",
            json!({"titleY":title_y,"availableTopY":min_title,"rowCount":rows.len()}),
        )]);
    }
    let locale = string(&input.document["meta"], "locale").to_lowercase();
    let title = translation(
        input,
        "legend.title",
        match locale.as_str() {
            "zh-cn" => "图例",
            "zh-tw" => "圖例",
            "es" => "Leyenda",
            "ko" => "범례",
            _ => "Legend",
        },
    );
    let mut cards = vec![Card {
        authored: generated("legend-title", "legend-title".into(), "/meta/legend"),
        bounds: Rect {
            x: 40.0,
            y: title_y - 10.0,
            width: 48.0,
            height: 14.0,
        },
        text: vec![text("/meta/legend", title, 40.0, title_y, 12.0, "start")],
    }];
    for (row_index, row) in rows.iter().enumerate() {
        let y = baseline - (rows.len() - row_index - 1) as f64 * 22.0;
        let mut x = 40.0;
        for &index in row {
            let entry = &entries[index];
            cards.push(Card {
                authored: generated(
                    &format!("legend-{}", entry.kind),
                    format!("legend-{}", entry.kind),
                    "/meta/legend",
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
                    x + entry.swatch + 8.0,
                    y,
                    10.0,
                    "start",
                )],
            });
            x += entry.width + 22.0;
        }
    }
    Ok(cards)
}

// Source nodeLabelLayout, without deferred brand/source decorations.
pub(super) fn node_text_layout(
    label: &str,
    label_font: f64,
    width: f64,
    height: f64,
    rows: &[(f64, f64)],
    step: &str,
    input: &DiagramInput,
) -> (f64, Vec<f64>, f64, f64) {
    let mut x = width / 2.0;
    let mut ys: Vec<_> = rows.iter().map(|r| r.0).collect();
    let label_width = units(input, label) * label_font * 0.6;
    let left = 19.0_f64.max(if step.is_empty() {
        0.0
    } else {
        23.0 + units(input, step) * 8.0 * 0.6 + 3.0
    });
    let right = width - 4.0;
    if x - label_width / 2.0 >= left && x + label_width / 2.0 <= right {
        return (x, ys, 6.0, 11.0);
    }
    if label_width <= right - left {
        x = (((right - label_width / 2.0) * 10.0).floor() / 10.0)
            .min(x.max(((left + label_width / 2.0) * 10.0).ceil() / 10.0));
        return (x, ys, 6.0, 11.0);
    }
    let mut bottom: f64 = 17.0;
    let shifted: Vec<_> = rows
        .iter()
        .map(|(y, font)| {
            let y = y.max(((bottom + 2.0 + font * 1.2) * 10.0).ceil() / 10.0);
            bottom = y + font * 0.3;
            y
        })
        .collect();
    if bottom <= height - 2.0 {
        ys = shifted;
        return (x, ys, 6.0, 11.0);
    }
    (
        x,
        ys,
        1.0,
        (rows[0].0 - rows[0].1 * 1.2 - 3.0).floor().clamp(1.0, 11.0),
    )
}
