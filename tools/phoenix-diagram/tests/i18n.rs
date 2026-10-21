use phoenix_diagram::i18n::resolve;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

const LOCALES: [&str; 5] = ["en", "zh-CN", "zh-TW", "es", "ko"];

fn catalogs() -> Map<String, Value> {
    serde_json::from_str(include_str!("../assets/catalogs.json")).unwrap()
}

#[test]
fn bundled_tags_match_case_insensitively_and_unsupported_tags_fall_back_to_english() {
    let none = resolve(&json!({"title":"t"}));
    assert_eq!(
        (
            none.requested.as_deref(),
            none.resolved.as_str(),
            none.fallback
        ),
        (None, "en", false)
    );
    assert_eq!(none.text("timeline.axis", &[]), "Time");
    let chinese = resolve(&json!({"locale":"zh-cn"}));
    assert_eq!(chinese.resolved, "zh-CN");
    assert!(!chinese.fallback);
    assert_eq!(chinese.text("timeline.axis", &[]), "时间");
    for (tag, axis) in [("zh-TW", "時間"), ("ES", "Hora"), ("ko", "시간")] {
        let locale = resolve(&json!({"locale":tag}));
        assert!(!locale.fallback);
        assert_eq!(locale.text("timeline.axis", &[]), axis, "{tag}");
    }
    let french = resolve(&json!({"locale":"fr"}));
    assert_eq!(french.requested.as_deref(), Some("fr"));
    assert_eq!((french.resolved.as_str(), french.fallback), ("en", true));
    assert_eq!(french.text("timeline.axis", &[]), "Time");
}

#[test]
fn usable_overrides_win_and_mismatched_or_unregistered_overrides_are_ignored() {
    let french = resolve(&json!({"locale":"fr","translations":{
        "timeline.axis":"Heure",
        "timeline.break":"Coupure sans durée",
        "unknown.key":"x",
    }}));
    assert_eq!((french.resolved.as_str(), french.fallback), ("fr", false));
    assert_eq!(french.text("timeline.axis", &[]), "Heure");
    assert_eq!(
        french.text("timeline.break", &[("duration", "2h")]),
        "Axis break: 2h without events omitted"
    );
    let chinese = resolve(&json!({"locale":"zh-CN","translations":{"timeline.axis":"时刻"}}));
    assert_eq!(chinese.text("timeline.axis", &[]), "时刻");
    assert_eq!(chinese.text("waterfall.operation", &[]), "操作");
    let unregistered = resolve(&json!({"translations":{"timeline.axis":"Heure"}}));
    assert_eq!(unregistered.text("timeline.axis", &[]), "Time");
}

#[test]
fn placeholders_are_filled_and_unknown_names_stay_literal() {
    let english = resolve(&json!({}));
    assert_eq!(
        english.text("waterfall.share", &[("percent", "40%"), ("total", "1 s")]),
        "40% of 1 s wall-clock"
    );
    assert_eq!(
        english.text("waterfall.share", &[("percent", "40%")]),
        "40% of {total} wall-clock"
    );
    assert_eq!(
        english.text("viewer.export.card.routeBadge", &[("hops", "3")]),
        "PHOENIX DIAGRAM · ROUTE · 3"
    );
}

#[test]
fn five_catalogs_share_keys_placeholders_and_phoenix_identity() {
    let catalogs = catalogs();
    assert_eq!(
        catalogs.keys().map(String::as_str).collect::<Vec<_>>(),
        LOCALES
    );
    let placeholders = |message: &str| -> BTreeSet<String> {
        message
            .split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}'))
            .map(|(name, _)| name.to_owned())
            .filter(|name| {
                !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            })
            .collect()
    };
    let english = catalogs["en"].as_object().unwrap();
    let mut required = vec![
        "viewer.export.downloadPngInstead",
        "timeline.precision.day",
        "timeline.precision.dayDisclosure",
        "viewer.passport.evidence.unverified",
        "viewer.passport.evidence.unsourced",
        "viewer.passport.evidence.localOnly",
    ];
    let reasons = [
        "repo-root-not-provided",
        "git-unavailable",
        "git-timeout",
        "repository-unreadable",
        "root-not-top-level",
        "origin-mismatch",
        "partial-clone-unsafe-git",
        "revision-unavailable",
        "object-unavailable",
        "file-missing",
        "line-out-of-range",
        "worktree-differs",
    ]
    .map(|code| format!("viewer.passport.evidence.reason.{code}"));
    required.extend(reasons.iter().map(String::as_str));
    for locale in LOCALES {
        let catalog = catalogs[locale].as_object().unwrap();
        assert_eq!(
            catalog.keys().collect::<BTreeSet<_>>(),
            english.keys().collect::<BTreeSet<_>>(),
            "{locale}"
        );
        for (key, message) in catalog {
            let message = message.as_str().unwrap();
            assert!(!message.is_empty(), "{locale} {key}");
            assert!(
                !message.to_lowercase().contains("archify"),
                "{locale} {key}: {message}"
            );
            assert_eq!(
                placeholders(message),
                placeholders(english[key].as_str().unwrap()),
                "{locale} {key}"
            );
        }
        for key in &required {
            assert!(catalog.contains_key(*key), "{locale} lacks {key}");
        }
    }
}
