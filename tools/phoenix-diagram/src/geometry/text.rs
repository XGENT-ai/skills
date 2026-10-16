// Unicode 16 helpers shared by the sequence and dataflow renderers.
// ICU4X 2.0 compiled data and the generated property table match the pinned
// Node 25.2.1 / ICU 77.1 / Unicode 16.0 generation used by the source oracle.
use icu_segmenter::GraphemeClusterSegmenter;
use serde::Deserialize;
use std::sync::OnceLock;
mod cjk;
mod southeast_asian;
mod words;

/// Extended grapheme clusters, preserving combining marks and emoji sequences.
pub fn graphemes(text: &str) -> Vec<&str> {
    let mut start = 0;
    GraphemeClusterSegmenter::new()
        .segment_str(text)
        .skip(1)
        .map(|end| {
            let cluster = &text[start..end];
            start = end;
            cluster
        })
        .collect()
}

/// Dictionary word boundaries as UTF-8 byte offsets, including 0 and the end.
/// Empty input has the single boundary `[0]`.
pub fn word_boundaries(text: &str) -> Vec<usize> {
    words::boundaries(text)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Properties {
    han: Vec<[u32; 2]>,
    mark: Vec<[u32; 2]>,
    js_whitespace: Vec<[u32; 2]>,
}

fn properties() -> &'static Properties {
    static PROPERTIES: OnceLock<Properties> = OnceLock::new();
    PROPERTIES.get_or_init(|| {
        serde_json::from_str(include_str!("../../assets/unicode16-properties.json"))
            .expect("embedded Unicode 16 property ranges are generated valid JSON")
    })
}

fn in_ranges(character: char, ranges: &[[u32; 2]]) -> bool {
    let codepoint = u32::from(character);
    let index = ranges.partition_point(|range| range[1] < codepoint);
    ranges.get(index).is_some_and(|range| range[0] <= codepoint)
}

/// JavaScript `/\p{Script=Han}/u`, rather than Script_Extensions or CJK blocks.
pub fn is_han(character: char) -> bool {
    in_ranges(character, &properties().han)
}

/// JavaScript `/\p{Mark}/u` under the pinned Unicode 16 data.
pub fn is_mark(character: char) -> bool {
    in_ranges(character, &properties().mark)
}

/// JavaScript `/\s/u`; includes BOM and excludes NEXT LINE and ZERO WIDTH SPACE.
pub fn is_js_whitespace(character: char) -> bool {
    in_ranges(character, &properties().js_whitespace)
}
