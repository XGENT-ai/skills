// Rust adaptation of ICU 77.1 Thai/Lao/Burmese/Khmer dictionary engines.
// Copyright (C) 2006-2016 IBM and others; Copyright © 2016-2025 Unicode, Inc.
// SPDX-License-Identifier: Unicode-3.0
// Modification: shared lookahead implementation with explicit script policies,
// UTF-8 source offsets and the complete pinned ICU4X dictionary tries.
use super::is_mark;
use icu_collections::char16trie::{Char16Trie, TrieResult};
use icu_properties::props::{LineBreak, Script};
use icu_properties::CodePointMapData;
use icu_provider::prelude::*;
use icu_segmenter::provider::{Baked, SegmenterDictionaryExtendedV1};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Language {
    Thai,
    Lao,
    Burmese,
    Khmer,
}

impl Language {
    pub(super) fn of(character: char) -> Option<Self> {
        if CodePointMapData::<LineBreak>::new().get(character) != LineBreak::ComplexContext {
            return None;
        }
        match CodePointMapData::<Script>::new().get(character) {
            Script::Thai => Some(Self::Thai),
            Script::Lao => Some(Self::Lao),
            Script::Myanmar => Some(Self::Burmese),
            Script::Khmer => Some(Self::Khmer),
            _ => None,
        }
    }

    pub(super) fn handles(self, character: char) -> bool {
        Self::of(character) == Some(self)
    }

    fn begin(self, character: char) -> bool {
        let c = u32::from(character);
        match self {
            Self::Thai => matches!(c, 0x0e01..=0x0e2e | 0x0e40..=0x0e44),
            Self::Lao => matches!(c, 0x0e81..=0x0eae | 0x0edc..=0x0edd | 0x0ec0..=0x0ec4),
            Self::Burmese => matches!(c, 0x1000..=0x102a),
            Self::Khmer => matches!(c, 0x1780..=0x17b3),
        }
    }

    fn end(self, character: char) -> bool {
        self.handles(character)
            && match self {
                Self::Thai => !matches!(u32::from(character), 0x0e31 | 0x0e40..=0x0e44),
                Self::Lao => !matches!(u32::from(character), 0x0ec0..=0x0ec4),
                Self::Khmer => character != '\u{17d2}',
                Self::Burmese => true,
            }
    }

    fn mark(self, character: char) -> bool {
        character == ' ' || self.handles(character) && is_mark(character)
    }
}

fn dictionary(language: Language) -> &'static Char16Trie<'static> {
    static DATA: OnceLock<[DataPayload<SegmenterDictionaryExtendedV1>; 4]> = OnceLock::new();
    static TRIES: OnceLock<[Char16Trie<'static>; 4]> = OnceLock::new();
    let tries = TRIES.get_or_init(|| {
        let data = DATA.get_or_init(|| {
            ["thaidict", "laodict", "burmesedict", "khmerdict"].map(|name| {
                <Baked as DataProvider<SegmenterDictionaryExtendedV1>>::load(
                    &Baked,
                    DataRequest {
                        id: DataIdentifierBorrowed::for_marker_attributes(
                            DataMarkerAttributes::from_str_or_panic(name),
                        ),
                        ..Default::default()
                    },
                )
                .expect("the pinned compiled Southeast Asian dictionary exists")
                .payload
            })
        });
        std::array::from_fn(|index| Char16Trie::new(data[index].get().trie_data.clone()))
    });
    &tries[match language {
        Language::Thai => 0,
        Language::Lao => 1,
        Language::Burmese => 2,
        Language::Khmer => 3,
    }]
}

#[derive(Default)]
struct PossibleWord {
    offset: Option<usize>,
    choices: Vec<usize>,
    prefix: usize,
    current: usize,
    marked: usize,
}

impl PossibleWord {
    fn candidates(
        &mut self,
        characters: &[char],
        cursor: &mut usize,
        dictionary: &Char16Trie<'_>,
    ) -> usize {
        let start = *cursor;
        if self.offset != Some(start) {
            self.offset = Some(start);
            self.choices.clear();
            self.prefix = 0;
            let mut trie = dictionary.iter();
            for (index, &character) in characters[start..].iter().enumerate() {
                // ICU's matcher counts the failed character in longestPrefix.
                self.prefix = index + 1;
                match trie.next(character) {
                    TrieResult::FinalValue(_) => {
                        if self.choices.len() < 20 {
                            self.choices.push(index + 1);
                        }
                        break;
                    }
                    TrieResult::Intermediate(_) => {
                        if self.choices.len() < 20 {
                            self.choices.push(index + 1);
                        }
                    }
                    TrieResult::NoMatch => break,
                    TrieResult::NoValue => (),
                }
            }
        }
        *cursor = start + self.choices.last().copied().unwrap_or(0);
        self.current = self.choices.len().saturating_sub(1);
        self.marked = self.current;
        self.choices.len()
    }

    fn accept(&self, cursor: &mut usize) -> usize {
        let length = self.choices[self.marked];
        *cursor = self.offset.expect("a candidate offset was set") + length;
        length
    }

    fn backup(&mut self, cursor: &mut usize) -> bool {
        if self.current == 0 {
            return false;
        }
        self.current -= 1;
        *cursor = self.offset.expect("a candidate offset was set") + self.choices[self.current];
        true
    }
}

pub(super) fn boundaries(text: &str, language: Language) -> Vec<usize> {
    let (mut positions, characters): (Vec<_>, Vec<_>) = text.char_indices().unzip();
    positions.push(text.len());
    if text.encode_utf16().count() < 4 || language == Language::Thai && characters.len() <= 4 {
        return Vec::new();
    }
    let dictionary = dictionary(language);
    let mut words: [PossibleWord; 3] = std::array::from_fn(|_| PossibleWord::default());
    let mut found = 0;
    let mut cursor = 0;
    let mut result = Vec::new();
    while cursor < characters.len() {
        let start = cursor;
        let mut length = 0;
        let first = found % 3;
        let second = (found + 1) % 3;
        let third = (found + 2) % 3;
        let candidates = words[first].candidates(&characters, &mut cursor, dictionary);
        if candidates > 1 && cursor < characters.len() {
            'choose: loop {
                if words[second].candidates(&characters, &mut cursor, dictionary) > 0 {
                    words[first].marked = words[first].current;
                    if cursor >= characters.len() {
                        break;
                    }
                    loop {
                        if words[third].candidates(&characters, &mut cursor, dictionary) > 0 {
                            words[first].marked = words[first].current;
                            break 'choose;
                        }
                        if !words[second].backup(&mut cursor) {
                            break;
                        }
                    }
                }
                if !words[first].backup(&mut cursor) {
                    break;
                }
            }
        }
        if candidates > 0 {
            length = words[first].accept(&mut cursor);
            found += 1;
        }
        if cursor < characters.len() && length < 3 {
            let current = found % 3;
            if words[current].candidates(&characters, &mut cursor, dictionary) == 0
                && (length == 0 || words[current].prefix < 3)
            {
                let unknown_start = cursor;
                while cursor < characters.len() {
                    let previous = characters[cursor];
                    cursor += 1;
                    if cursor == characters.len() {
                        break;
                    }
                    if language.end(previous) && language.begin(characters[cursor]) {
                        let boundary = cursor;
                        let matches =
                            words[(found + 1) % 3].candidates(&characters, &mut cursor, dictionary);
                        cursor = boundary;
                        if matches > 0 {
                            break;
                        }
                    }
                }
                if length == 0 {
                    found += 1;
                }
                length += cursor - unknown_start;
            } else {
                cursor = start + length;
            }
        }
        while cursor < characters.len() && language.mark(characters[cursor]) {
            cursor += 1;
            length += 1;
        }
        if language == Language::Thai && cursor < characters.len() && length > 0 {
            let end = start + length;
            if words[found % 3].candidates(&characters, &mut cursor, dictionary) == 0
                && matches!(characters[cursor], '\u{0e2f}' | '\u{0e46}')
            {
                if characters[cursor] == '\u{0e2f}'
                    && !matches!(characters[cursor - 1], '\u{0e2f}' | '\u{0e46}')
                {
                    cursor += 1;
                    length += 1;
                }
                if cursor < characters.len()
                    && characters[cursor] == '\u{0e46}'
                    && characters[cursor - 1] != '\u{0e46}'
                {
                    cursor += 1;
                    length += 1;
                }
            } else {
                cursor = end;
            }
        }
        if length > 0 {
            result.push(positions[start + length]);
        }
    }
    result.retain(|&offset| offset < text.len());
    result
}
