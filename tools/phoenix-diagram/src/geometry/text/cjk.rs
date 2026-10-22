// Adapted to Rust/UTF-8 from ICU 77.1 CjkBreakEngine (dictbe.cpp).
// Copyright (C) 2006-2016 IBM and others; Copyright © 2016-2025 Unicode, Inc.
// SPDX-License-Identifier: Unicode-3.0
// Modification: Rust storage, baked ICU4X dictionaries and native byte offsets.
// License: assets/icu77-LICENSE.txt. The algorithm retains ICU costs and NFKC mapping.
use icu_collections::char16trie::{Char16Trie, TrieResult};
use icu_collections::codepointtrie::CodePointTrie;
use icu_normalizer::{provider::NormalizerNfkdDataV1, ComposingNormalizer};
use icu_provider::prelude::*;
use icu_segmenter::provider::{Baked, SegmenterDictionaryAutoV1};
use std::sync::OnceLock;

fn dictionary() -> &'static Char16Trie<'static> {
    static DATA: OnceLock<DataPayload<SegmenterDictionaryAutoV1>> = OnceLock::new();
    static DICTIONARY: OnceLock<Char16Trie<'static>> = OnceLock::new();
    DICTIONARY.get_or_init(|| {
        let data = DATA.get_or_init(|| {
            <Baked as DataProvider<SegmenterDictionaryAutoV1>>::load(
                &Baked,
                DataRequest {
                    id: DataIdentifierBorrowed::for_marker_attributes(
                        DataMarkerAttributes::from_str_or_panic("cjdict"),
                    ),
                    ..Default::default()
                },
            )
            .expect("the pinned compiled ICU dictionary exists")
            .payload
        });
        Char16Trie::new(data.get().trie_data.clone())
    })
}

fn nfkd_trie() -> &'static CodePointTrie<'static, u32> {
    static DATA: OnceLock<DataPayload<NormalizerNfkdDataV1>> = OnceLock::new();
    static TRIE: OnceLock<CodePointTrie<'static, u32>> = OnceLock::new();
    TRIE.get_or_init(|| {
        let data = DATA.get_or_init(|| {
            <icu_normalizer::provider::Baked as DataProvider<NormalizerNfkdDataV1>>::load(
                &icu_normalizer::provider::Baked,
                Default::default(),
            )
            .expect("the pinned compiled NFKD data exists")
            .payload
        });
        data.get().trie.clone()
    })
}

// ICU4X 2.0's documented NFKD trie bit 31 marks a decomposition whose first
// character can combine backwards: exactly the boundary needed for NFKC chunks.
fn boundary_before(character: char) -> bool {
    nfkd_trie().get32(u32::from(character)) & 0x8000_0000 == 0
}

fn normalized_input(text: &str) -> (Vec<char>, Vec<usize>) {
    let nfkc = ComposingNormalizer::new_nfkc();
    if nfkc.is_normalized(text) {
        let (positions, characters): (Vec<_>, Vec<_>) = text.char_indices().unzip();
        let mut positions = positions;
        positions.push(text.len());
        return (characters, positions);
    }
    let mut characters = Vec::new();
    let mut positions = Vec::new();
    let mut start = 0;
    for end in text
        .char_indices()
        .skip(1)
        .filter_map(|(offset, character)| boundary_before(character).then_some(offset))
        .chain(std::iter::once(text.len()))
    {
        for character in nfkc.normalize(&text[start..end]).chars() {
            characters.push(character);
            positions.push(start);
        }
        start = end;
    }
    positions.push(text.len());
    (characters, positions)
}

fn katakana(character: char) -> bool {
    matches!(u32::from(character), 0x30a1..=0x30fe | 0xff66..=0xff9f) && character != '\u{30fb}'
}

/// Internal word boundaries in one contiguous ICU CJK engine character range.
pub(super) fn boundaries(text: &str) -> Vec<usize> {
    let (characters, positions) = normalized_input(text);
    let length = characters.len();
    let mut best = vec![u32::MAX; length + 1];
    let mut previous = vec![usize::MAX; length + 1];
    best[0] = 0;
    let mut previous_katakana = false;
    for start in 0..length {
        if best[start] == u32::MAX {
            continue;
        }
        let mut candidates = Vec::new();
        let mut trie = dictionary().iter();
        let mut utf16_length = 0;
        for (index, &character) in characters[start..].iter().take(20).enumerate() {
            utf16_length += character.len_utf16();
            match trie.next(character) {
                TrieResult::FinalValue(cost) => {
                    candidates.push((index + 1, cost as u32));
                    break;
                }
                TrieResult::Intermediate(cost) => candidates.push((index + 1, cost as u32)),
                TrieResult::NoValue => (),
                TrieResult::NoMatch => break,
            }
            if utf16_length >= 20 {
                break;
            }
        }
        if candidates.first().is_none_or(|&(length, _)| length != 1)
            && !matches!(characters[start], '\u{ac00}'..='\u{d7a3}')
        {
            candidates.push((1, 255));
        }
        for (length, cost) in candidates {
            let end = start + length;
            let cost = best[start].wrapping_add(cost);
            if cost < best[end] {
                best[end] = cost;
                previous[end] = start;
            }
        }
        let is_katakana = katakana(characters[start]);
        if !previous_katakana && is_katakana {
            let run = characters[start..]
                .iter()
                .take(20)
                .take_while(|&&character| katakana(character))
                .count();
            if run < 20 {
                const COSTS: [u32; 9] = [8192, 984, 408, 240, 204, 252, 300, 372, 480];
                let cost = best[start].wrapping_add(COSTS.get(run).copied().unwrap_or(8192));
                if cost < best[start + run] {
                    best[start + run] = cost;
                    previous[start + run] = start;
                }
            }
        }
        previous_katakana = is_katakana;
    }
    if best[length] == u32::MAX {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut position = previous[length];
    while position != usize::MAX && position > 0 {
        result.push(positions[position]);
        position = previous[position];
    }
    result.reverse();
    result.dedup();
    result.retain(|&offset| offset > 0 && offset < text.len());
    result
}
