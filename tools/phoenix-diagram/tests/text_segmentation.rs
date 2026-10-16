use icu_collections::char16trie::{Char16Trie, TrieResult};
use icu_provider::prelude::*;
use icu_segmenter::provider::{Baked, SegmenterDictionaryAutoV1, SegmenterDictionaryExtendedV1};
use phoenix_diagram::geometry::text::{
    graphemes, is_han, is_js_whitespace, is_mark, word_boundaries,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn oracle() -> Value {
    serde_json::from_str(include_str!("fixtures/text-segmentation/oracle.json")).unwrap()
}

#[test]
fn shared_segmenters_match_pinned_node_intl_across_renderer_text() {
    let oracle = oracle();
    assert_eq!(oracle["format"], 1);
    assert_eq!(
        oracle["source"],
        json!({ "node": "25.2.1", "icu": "77.1", "unicode": "16.0", "locale": "en-US" })
    );
    let cases = oracle["cases"].as_array().unwrap();
    assert!(cases.len() >= 180);
    let mut differences = Vec::new();
    for case in cases {
        let text = case["text"].as_str().unwrap();
        let actual_graphemes = json!(graphemes(text));
        let actual_words = json!(word_boundaries(text));
        if actual_graphemes != case["graphemes"] || actual_words != case["wordBoundaries"] {
            differences.push(format!("{}: {:?}\ngraphemes actual={actual_graphemes} expected={}\nwords actual={actual_words} expected={}",
                case["name"], text, case["graphemes"], case["wordBoundaries"]));
        }
    }
    assert!(
        differences.is_empty(),
        "{} segmentation differences (first 12 shown):\n{}",
        differences.len(),
        differences
            .iter()
            .take(12)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn unicode_properties_match_node_over_every_unicode_scalar() {
    let oracle = oracle();
    for (name, property) in [
        ("han", is_han as fn(char) -> bool),
        ("mark", is_mark),
        ("jsWhitespace", is_js_whitespace),
    ] {
        let mut digest = Sha256::new();
        let mut count = 0_u64;
        for character in (0..=0x10ffff).filter_map(char::from_u32) {
            let member = property(character);
            count += u64::from(member);
            digest.update([u8::from(member)]);
        }
        assert_eq!(
            count,
            oracle["properties"][name]["count"].as_u64().unwrap(),
            "{name}"
        );
        assert_eq!(
            format!("{:x}", digest.finalize()),
            oracle["properties"][name]["sha256"].as_str().unwrap(),
            "{name}"
        );
    }
}

#[test]
fn dictionary_words_and_grapheme_clusters_are_not_scalar_or_space_splits() {
    let text = "已持有：处理完成或失败";
    let words = word_boundaries(text);
    let words = words
        .windows(2)
        .map(|pair| &text[pair[0]..pair[1]])
        .collect::<Vec<_>>();
    assert_eq!(
        words,
        vec!["已", "持有", "：", "处理", "完成", "或", "失败"]
    );
    assert_eq!(graphemes("e\u{301}👩🏽‍💻🇨🇳"), vec!["e\u{301}", "👩🏽‍💻", "🇨🇳"]);
    assert_eq!(word_boundaries(""), vec![0]);
    assert!(graphemes("").is_empty());
}

#[test]
fn javascript_properties_preserve_the_source_regex_boundaries() {
    assert!(is_han('漢'));
    assert!(is_han('\u{31350}'));
    assert!(!is_han('あ'));
    assert!(!is_han('。'));
    assert!(is_mark('\u{301}'));
    assert!(is_mark('\u{fe0f}'));
    assert!(is_mark('\u{1e5ee}'));
    assert!(!is_mark('a'));
    assert!(is_js_whitespace('\u{feff}'));
    assert!(is_js_whitespace('\u{a0}'));
    assert!(!is_js_whitespace('\u{85}'));
    assert!(!is_js_whitespace('\u{180e}'));
    assert!(!is_js_whitespace('\u{200b}'));
}

#[test]
fn baked_dictionary_preserves_the_complete_icu77_words_and_frequency_costs() {
    let source = include_str!("fixtures/text-segmentation/icu77-cjdict.txt");
    assert_eq!(
        format!("{:x}", Sha256::digest(source.as_bytes())),
        "e73fd72048981d0cc13e9dc436a7eaba07ffb6eff58c8a59dc75c1df746663a0"
    );
    let response = <Baked as DataProvider<SegmenterDictionaryAutoV1>>::load(
        &Baked,
        DataRequest {
            id: DataIdentifierBorrowed::for_marker_attributes(
                DataMarkerAttributes::from_str_or_panic("cjdict"),
            ),
            ..Default::default()
        },
    )
    .unwrap();
    let trie = Char16Trie::new(response.payload.get().trie_data.clone());
    let mut entries = 0;
    let mut differences = Vec::new();
    for line in source.lines() {
        let line = line.trim_start_matches('\u{feff}');
        if line.trim_start().starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let (word, expected) = line.rsplit_once('\t').unwrap();
        let expected = expected.trim().parse::<i32>().unwrap();
        let mut lookup = trie.iter();
        let mut actual = TrieResult::NoValue;
        for character in word.chars() {
            actual = lookup.next(character);
        }
        let actual = match actual {
            TrieResult::FinalValue(value) | TrieResult::Intermediate(value) => Some(value),
            _ => None,
        };
        if actual != Some(expected) {
            differences.push(format!("{word:?}: {actual:?} != {expected}"));
        }
        entries += 1;
    }
    assert!(entries >= 300_000, "{entries}");
    assert!(
        differences.is_empty(),
        "{} of {entries} complete ICU dictionary entries differ; first 12: {}",
        differences.len(),
        differences
            .iter()
            .take(12)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn southeast_asian_words_match_independent_pinned_node_source_cases() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/text-segmentation/sea-oracle.json")).unwrap();
    assert_eq!(
        oracle["source"],
        json!({ "node": "25.2.1", "icu": "77.1", "unicode": "16.0", "locale": "en-US" })
    );
    let cases = oracle["cases"].as_array().unwrap();
    assert!(cases.len() >= 1200);
    let mut differences = Vec::new();
    for case in cases {
        let actual = json!(word_boundaries(case["text"].as_str().unwrap()));
        if actual != case["wordBoundaries"] {
            differences.push(format!(
                "{}: {}\nactual={actual} expected={}",
                case["name"], case["text"], case["wordBoundaries"]
            ));
        }
    }
    assert!(
        differences.is_empty(),
        "{} Southeast Asian differences; first 12:\n{}",
        differences.len(),
        differences
            .iter()
            .take(12)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn baked_southeast_asian_dictionaries_preserve_every_frozen_source_word() {
    let sources: Value =
        serde_json::from_str(include_str!("fixtures/text-segmentation/sources.json")).unwrap();
    let files = [
        (
            "thaidict",
            include_str!("fixtures/text-segmentation/icu77-thaidict.txt"),
        ),
        (
            "laodict",
            include_str!("fixtures/text-segmentation/icu77-laodict.txt"),
        ),
        (
            "burmesedict",
            include_str!("fixtures/text-segmentation/icu77-burmesedict.txt"),
        ),
        (
            "khmerdict",
            include_str!("fixtures/text-segmentation/icu77-khmerdict.txt"),
        ),
    ];
    for (name, source) in files {
        let filename = format!("icu77-{name}.txt");
        let identity = sources["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["file"] == filename)
            .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(source.as_bytes())),
            identity["sha256"].as_str().unwrap()
        );
        let response = <Baked as DataProvider<SegmenterDictionaryExtendedV1>>::load(
            &Baked,
            DataRequest {
                id: DataIdentifierBorrowed::for_marker_attributes(
                    DataMarkerAttributes::from_str_or_panic(name),
                ),
                ..Default::default()
            },
        )
        .unwrap();
        let trie = Char16Trie::new(response.payload.get().trie_data.clone());
        let mut entries = 0;
        let mut differences = Vec::new();
        for word in source
            .lines()
            .map(|line| line.trim_start_matches('\u{feff}'))
            .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
        {
            let mut lookup = trie.iter();
            let mut actual = TrieResult::NoValue;
            for character in word.chars() {
                actual = lookup.next(character);
            }
            if !matches!(
                actual,
                TrieResult::FinalValue(_) | TrieResult::Intermediate(_)
            ) {
                differences.push(word);
            }
            entries += 1;
        }
        assert!(entries > 1000, "{name}: {entries}");
        assert!(
            differences.is_empty(),
            "{} of {entries} {name} words missing; first 12: {:?}",
            differences.len(),
            &differences[..differences.len().min(12)]
        );
    }
}
