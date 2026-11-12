use regex::Regex;
use serde_json::Value;
use std::collections::{HashSet, VecDeque};
use std::sync::LazyLock;

static SEPARATOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\p{L}\p{N}]+").expect("valid word pattern"));
static LATIN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{Latin}\p{N}]+$").expect("valid script pattern"));
static HTML: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]*>").expect("valid tag pattern"));
const JOINERS: &[&str] = &[
    "the", "and", "for", "with", "from", "der", "die", "das", "les", "des", "del", "van", "von",
];

pub(super) fn query_terms(query: &str) -> Vec<String> {
    SEPARATOR
        .split(&query.to_lowercase())
        .filter(|word| {
            !word.is_empty()
                && (!LATIN.is_match(word)
                    || word.chars().any(char::is_numeric)
                    || (word.chars().count() > 2 && !JOINERS.contains(word)))
        })
        .map(str::to_owned)
        .collect()
}

pub(super) fn match_count(text: &str, terms: &[String]) -> usize {
    let lower = text.to_lowercase();
    let flat = HTML.replace_all(&lower, " ");
    let list: Vec<_> = SEPARATOR
        .split(&flat)
        .filter(|word| !word.is_empty())
        .collect();
    let mut words: HashSet<String> = list.iter().map(|word| (*word).to_owned()).collect();
    words.extend(
        list.windows(2)
            .map(|pair| format!("{}{}", pair[0], pair[1])),
    );
    terms
        .iter()
        .filter(|term| {
            if !LATIN.is_match(term) {
                return flat.contains(term.as_str());
            }
            words.contains(term.as_str())
                || words.contains(&format!("{term}s"))
                || words.contains(&format!("{term}es"))
                || (term.len() > 3
                    && term
                        .strip_suffix('s')
                        .is_some_and(|singular| words.contains(singular)))
                || (term.chars().count() >= 5
                    && words.iter().any(|word| word.starts_with(term.as_str())))
        })
        .count()
}

pub(super) fn rank(items: Vec<Value>, query: &str, limit: usize) -> (Vec<Value>, usize) {
    let terms = query_terms(query);
    let filter_misses = !terms.is_empty() && terms.iter().all(|term| LATIN.is_match(term));
    let mut filtered = 0;
    let mut groups: Vec<(String, VecDeque<Value>, VecDeque<Value>)> = Vec::new();
    for item in items {
        let score = match_count(
            &format!(
                "{} {}",
                item["title"].as_str().unwrap_or(""),
                item["detail"].as_str().unwrap_or("")
            ),
            &terms,
        );
        if filter_misses && score == 0 {
            filtered += 1;
            continue;
        }
        let provider = item["provider"].as_str().unwrap_or("");
        let pos = groups
            .iter()
            .position(|group| group.0 == provider)
            .unwrap_or_else(|| {
                groups.push((provider.to_owned(), VecDeque::new(), VecDeque::new()));
                groups.len() - 1
            });
        if score == terms.len() {
            groups[pos].1.push_back(item);
        } else {
            groups[pos].2.push_back(item);
        }
    }
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for full in [true, false] {
        loop {
            let mut advanced = false;
            for group in &mut groups {
                let queue = if full { &mut group.1 } else { &mut group.2 };
                while let Some(item) = queue.pop_front() {
                    advanced = true;
                    let key = (
                        group.0.clone(),
                        item["sourceId"].as_str().unwrap_or("").to_owned(),
                    );
                    if seen.insert(key) {
                        result.push(item);
                        break;
                    }
                }
            }
            if !advanced {
                break;
            }
        }
    }
    result.sort_by_key(|item| match item["use"].as_str() {
        Some("page") => 0,
        Some("reference") => 2,
        _ => 1,
    });
    result.truncate(limit);
    (result, filtered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn terms_match_subjects_without_losing_cjk_and_numbers() {
        assert_eq!(query_terms("The map of 日本 1920"), ["map", "日本", "1920"]);
        assert_eq!(
            match_count(
                "<b>Maps</b> and architectural elevations, 茶碗展 1920",
                &query_terms("map architect 茶碗 1920")
            ),
            4
        );
        assert_eq!(match_count("configuration", &query_terms("fig")), 0);
    }

    #[test]
    fn rank_filters_latin_misses_interleaves_and_deduplicates() {
        let item = |p: &str, id: &str, title: &str, grade: &str| json!({"provider":p,"sourceId":id,"title":title,"detail":"","use":grade});
        let (ranked, filtered) = rank(
            vec![
                item("aic", "1", "world map", "page"),
                item("aic", "2", "world map", "page"),
                item("commons", "1", "world map", "page"),
                item("commons", "1", "world map", "page"),
                item("commons", "2", "flower", "page"),
                item("vam", "1", "map", "reference"),
            ],
            "world map",
            10,
        );
        assert_eq!(filtered, 1);
        assert_eq!(
            ranked
                .iter()
                .map(|v| (
                    v["provider"].as_str().unwrap(),
                    v["sourceId"].as_str().unwrap()
                ))
                .collect::<Vec<_>>(),
            [("aic", "1"), ("commons", "1"), ("aic", "2"), ("vam", "1")]
        );
        assert_eq!(
            rank(vec![item("aic", "3", "tea bowl", "page")], "茶碗", 1)
                .0
                .len(),
            1
        );
    }
}
