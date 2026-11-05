//! The engine's words about a statechart: arrow conditions, state texts, the
//! report cards placed before the author's, and the receipt's `model` record.
use super::chart::{Arrow, Combo, Fallback, Note, Shape, Story, When};
use super::explore::{Outcome, Verdict};
use super::Analysis;
use crate::i18n::Locale;
use serde_json::{json, Value};

fn regions_text<'a>(
    parts: impl Iterator<Item = (Option<&'a str>, &'a [String])>,
    locale: &Locale,
) -> String {
    parts
        .map(|(name, path)| match name {
            Some(name) => format!("{name} {}", path.join(" / ")),
            None => path.join(" / "),
        })
        .collect::<Vec<_>>()
        .join(&locale.text("statechart.join.regions", &[]))
}

/// The chart state in every authored region, e.g. `会话 未登录，错误次数 0`.
pub fn state_text(shape: &Shape, locale: &Locale) -> String {
    regions_text(
        shape
            .iter()
            .map(|(name, path)| (name.as_deref(), path.as_slice())),
        locale,
    )
}

fn combo_text(combo: &Combo, locale: &Locale) -> String {
    regions_text(
        combo
            .iter()
            .map(|(name, path)| (Some(name.as_str()), path.as_slice())),
        locale,
    )
}

/// `regions` counts the authored regions: with only one other region a
/// clause leaves out the region's name.
pub fn when_text(when: &When, regions: usize, locale: &Locale) -> String {
    let (clauses, joiner) = match when {
        When::All(clauses) => (clauses, "statechart.when.and"),
        When::Any(clauses) => (clauses, "statechart.when.or"),
    };
    let joiner = locale.text(joiner, &[]);
    if regions < 3 {
        let or = locale.text("statechart.when.or", &[]);
        return clauses
            .iter()
            .map(|clause| {
                let key = if clause.negated {
                    "statechart.when.unless"
                } else {
                    "statechart.when.either"
                };
                locale.text(key, &[("values", &clause.values.join(&or))])
            })
            .collect::<Vec<_>>()
            .join(&joiner);
    }
    let list = locale.text("statechart.join.list", &[]);
    let parts = clauses
        .iter()
        .map(|clause| {
            let key = match (clause.negated, clause.values.len() > 1) {
                (false, false) => "statechart.when.is",
                (false, true) => "statechart.when.one-of",
                (true, false) => "statechart.when.is-not",
                (true, true) => "statechart.when.none-of",
            };
            let values = clause.values.join(&list);
            locale.text(
                key,
                &[
                    ("region", &clause.region),
                    ("value", &values),
                    ("values", &values),
                ],
            )
        })
        .collect::<Vec<_>>()
        .join(&joiner);
    locale.text("statechart.when.either", &[("values", &parts)])
}

pub fn arrow_label(arrow: &Arrow, regions: usize, locale: &Locale) -> String {
    let labels = arrow.labels.join(&locale.text("statechart.join.or", &[]));
    match &arrow.when {
        Some(when) => format!("{labels} [{}]", when_text(when, regions, locale)),
        None => labels,
    }
}

pub fn rule_text(outcome: &Outcome, locale: &Locale) -> String {
    outcome
        .text
        .clone()
        .unwrap_or_else(|| locale.text("statechart.rule.deadlock", &[]))
}

fn fallback_text(fallback: &Fallback, locale: &Locale) -> String {
    match fallback {
        Fallback::Starts(combo) => locale.text(
            "statechart.fallback.starts",
            &[("combo", &combo_text(combo, locale))],
        ),
        Fallback::Condition {
            region,
            label,
            from,
        } => locale.text(
            "statechart.fallback.condition",
            &[
                ("region", region),
                ("label", label),
                ("from", &from.join(" / ")),
            ],
        ),
        Fallback::MoveExtra { from, label, to } | Fallback::MoveMissing { from, label, to } => {
            let key = if matches!(fallback, Fallback::MoveExtra { .. }) {
                "statechart.fallback.move-extra"
            } else {
                "statechart.fallback.move-missing"
            };
            locale.text(
                key,
                &[
                    ("from", &combo_text(from, locale)),
                    ("label", label),
                    ("to", &combo_text(to, locale)),
                ],
            )
        }
    }
}

fn note_text(note: &Note, locale: &Locale) -> String {
    let list = locale.text("statechart.join.list", &[]);
    match note {
        Note::OneAtATime { step, regions } => locale.text(
            "statechart.note.one-at-a-time",
            &[("step", step), ("regions", &regions.join(&list))],
        ),
        Note::Together { step, sets } => {
            let sets = sets
                .iter()
                .map(|set| set.join(&list))
                .collect::<Vec<_>>()
                .join(&locale.text("statechart.join.or", &[]));
            locale.text(
                "statechart.note.together",
                &[("step", step), ("sets", &sets)],
            )
        }
    }
}

fn story<'a>(analysis: &'a Analysis, outcome: &Outcome) -> Option<&'a Story> {
    analysis
        .chart
        .stories
        .iter()
        .find(|story| story.rule == outcome.rule)
}

fn card(dot: &str, title: String, items: Vec<String>) -> Value {
    json!({"dot": dot, "title": title, "items": items})
}

fn rules_card(
    analysis: &Analysis,
    verdict: Verdict,
    dot: &str,
    title: &str,
    locale: &Locale,
) -> Option<Value> {
    let items: Vec<String> = analysis
        .outcomes
        .iter()
        .filter(|outcome| outcome.verdict == verdict)
        .map(|outcome| rule_text(outcome, locale))
        .collect();
    (!items.is_empty()).then(|| card(dot, locale.text(title, &[]), items))
}

/// Summary, one card per broken rule, then the rules that hold and the rules
/// not checked; the last two only when they list something.
pub fn cards(analysis: &Analysis, locale: &Locale) -> Vec<Value> {
    let chart = &analysis.chart;
    let count = |n: usize| n.to_string();
    let mut summary = vec![locale.text(
        "statechart.summary.counts",
        &[
            ("states", &count(analysis.exploration.states.len())),
            ("steps", &count(analysis.exploration.steps.len())),
            ("chartStates", &count(chart.state_count())),
            ("arrows", &count(chart.arrow_count())),
        ],
    )];
    if !analysis.complete() {
        summary.push(locale.text(
            "statechart.summary.incomplete",
            &[("limit", &count(super::STATE_LIMIT))],
        ));
    }
    if chart.inside > 0 {
        summary.push(locale.text(
            "statechart.summary.inside",
            &[("count", &count(chart.inside))],
        ));
    }
    if chart.hidden > 0 {
        summary.push(locale.text(
            "statechart.summary.hidden",
            &[("count", &count(chart.hidden))],
        ));
    }
    if let Some(fallback) = &chart.fallback {
        summary.push(locale.text(
            "statechart.summary.fallback",
            &[("reason", &fallback_text(fallback, locale))],
        ));
    }
    summary.extend(chart.notes.iter().map(|note| note_text(note, locale)));

    let mut cards = vec![card(
        "slate",
        locale.text("statechart.card.summary", &[]),
        summary,
    )];
    for outcome in &analysis.outcomes {
        let Some(story) = story(analysis, outcome) else {
            continue;
        };
        let mut items = vec![
            locale.text(
                "statechart.path.length",
                &[("count", &count(story.steps.len()))],
            ),
            locale.text(
                "statechart.path.start",
                &[("state", &state_text(&story.start, locale))],
            ),
        ];
        items.extend(
            story
                .steps
                .iter()
                .enumerate()
                .map(|(index, (step, shape))| {
                    locale.text(
                        "statechart.path.step",
                        &[
                            ("index", &count(index + 1)),
                            ("step", step),
                            ("state", &state_text(shape, locale)),
                        ],
                    )
                }),
        );
        let title = locale.text(
            "statechart.card.rule-breaks",
            &[
                ("text", &rule_text(outcome, locale)),
                ("rule", &outcome.rule),
            ],
        );
        cards.push(card("rose", title, items));
    }
    cards.extend(rules_card(
        analysis,
        Verdict::Holds,
        "emerald",
        "statechart.card.rules-hold",
        locale,
    ));
    cards.extend(rules_card(
        analysis,
        Verdict::NotChecked,
        "amber",
        "statechart.card.rules-not-checked",
        locale,
    ));
    cards
}

/// The receipt's `model` record.
pub fn receipt(analysis: &Analysis, locale: &Locale) -> Value {
    let chart = &analysis.chart;
    let rules: Vec<Value> =
        analysis
            .outcomes
            .iter()
            .map(|outcome| {
                let mut rule = json!({
                    "id": outcome.rule,
                    "text": rule_text(outcome, locale),
                    "outcome": outcome.verdict,
                });
                if let Some(story) = story(analysis, outcome) {
                    let path: Vec<Value> = std::iter::once(
                        json!({"state": state_text(&story.start, locale)}),
                    )
                    .chain(story.steps.iter().map(
                        |(step, shape)| json!({"step": step, "state": state_text(shape, locale)}),
                    ))
                    .collect();
                    rule["path"] = Value::Array(path);
                }
                rule
            })
            .collect();
    json!({
        "states": analysis.exploration.states.len(),
        "steps": analysis.exploration.steps.len(),
        "complete": analysis.complete(),
        "stateLimit": super::STATE_LIMIT,
        "chart": {
            "states": chart.state_count(),
            "arrows": chart.arrow_count(),
            "regions": chart.region_names,
            "regionsFallback": chart.fallback.as_ref().map(|fallback| fallback_text(fallback, locale)),
            "inside": chart.inside,
            "hidden": chart.hidden,
        },
        "rules": rules,
    })
}
