// Rust adaptation of ICU 77.1 word.txt's chained word rules and dictionary dispatch.
// Copyright (C) 2002-2016 IBM and others; Copyright © 2016-2025 Unicode, Inc.
// SPDX-License-Identifier: Unicode-3.0
// Modification: explicit Unicode properties, UTF-8 offsets and Rust CJK dispatch.
// The original rules and license are retained in tests/fixtures/text-segmentation.
use super::southeast_asian::Language;
use super::{cjk, is_han, southeast_asian};
use icu_properties::props::{
    ExtendedPictographic, GraphemeClusterBreak, LineBreak, Script, WordBreak as W,
};
use icu_properties::{CodePointMapData, CodePointSetData};

#[derive(Clone, Copy)]
struct Unit {
    offset: usize,
    character: char,
    word: W,
    kana_kanji: bool,
    cjk_engine: bool,
    hangul: bool,
    letter: bool,
    pictographic: bool,
}

impl Unit {
    fn new(offset: usize, character: char) -> Self {
        let han = is_han(character);
        let script = CodePointMapData::<Script>::new().get(character);
        let mut word = CodePointMapData::<W>::new().get(character);
        // ICU 77 explicitly subtracts Han-script marks from $Extend.
        if han && word == W::Extend {
            word = W::Other;
        }
        let kana_kanji = han || script == Script::Hiragana || word == W::Katakana;
        let hangul = matches!(character, '\u{ac00}'..='\u{d7a3}');
        let complex =
            CodePointMapData::<LineBreak>::new().get(character) == LineBreak::ComplexContext;
        let cjk_engine = han
            || matches!(script, Script::Hiragana | Script::Katakana)
            || matches!(character, '\u{30fc}' | '\u{ff70}' | '\u{ff9e}' | '\u{ff9f}');
        let letter = (word == W::ALetter && !kana_kanji && !hangul)
            || (complex
                && word != W::Extend
                && CodePointMapData::<GraphemeClusterBreak>::new().get(character)
                    != GraphemeClusterBreak::Control);
        Self {
            offset,
            character,
            word,
            kana_kanji,
            cjk_engine,
            hangul,
            letter,
            pictographic: CodePointSetData::new::<ExtendedPictographic>().contains(character),
        }
    }

    fn ignored(self) -> bool {
        matches!(self.word, W::Extend | W::Format | W::ZWJ)
    }
    fn newline(self) -> bool {
        matches!(self.word, W::CR | W::LF | W::Newline)
    }
    fn alpha(self) -> bool {
        self.letter || self.word == W::HebrewLetter
    }
    fn numeric(self) -> bool {
        self.word == W::Numeric
    }
    fn extender(self) -> bool {
        self.alpha() || self.numeric() || matches!(self.word, W::Katakana | W::ExtendNumLet)
    }
}

fn previous(units: &[Unit], end: usize) -> Option<usize> {
    (0..end).rev().find(|&index| !units[index].ignored())
}

fn following(units: &[Unit], start: usize) -> Option<usize> {
    (start..units.len()).find(|&index| !units[index].ignored())
}

fn boundary(units: &[Unit], index: usize) -> bool {
    let immediate = units[index - 1];
    let right = units[index];
    if immediate.word == W::CR && right.word == W::LF {
        return false;
    }
    if immediate.word == W::ZWJ && right.pictographic {
        return false;
    }
    if immediate.word == W::WSegSpace && right.word == W::WSegSpace {
        return false;
    }
    if immediate.newline() || right.newline() {
        return true;
    }
    if right.ignored() {
        return false;
    }
    let Some(left_index) = previous(units, index) else {
        return true;
    };
    let left = units[left_index];
    if left.newline() {
        return true;
    }
    if left.alpha() && right.alpha() {
        return false;
    }
    if left.word == W::HebrewLetter && right.word == W::SingleQuote {
        return false;
    }
    if (left.numeric() && (right.numeric() || right.alpha())) || (left.alpha() && right.numeric()) {
        return false;
    }
    if left.word == W::Katakana && right.word == W::Katakana {
        return false;
    }
    if left.extender() && right.word == W::ExtendNumLet {
        return false;
    }
    if left.word == W::ExtendNumLet && right.extender() {
        return false;
    }
    let alpha_mid = |word| matches!(word, W::MidLetter | W::MidNumLet | W::SingleQuote);
    let numeric_mid = |word| matches!(word, W::MidNum | W::MidNumLet | W::SingleQuote);
    if let Some(next) = following(units, index + 1).map(|next| units[next]) {
        if left.alpha() && alpha_mid(right.word) && next.alpha() {
            return false;
        }
        if left.numeric() && numeric_mid(right.word) && next.numeric() {
            return false;
        }
        if left.word == W::HebrewLetter
            && right.word == W::DoubleQuote
            && next.word == W::HebrewLetter
        {
            return false;
        }
    }
    if let Some(before) = previous(units, left_index).map(|before| units[before]) {
        if before.alpha() && alpha_mid(left.word) && right.alpha() {
            return false;
        }
        if before.numeric() && numeric_mid(left.word) && right.numeric() {
            return false;
        }
        if before.word == W::HebrewLetter
            && left.word == W::DoubleQuote
            && right.word == W::HebrewLetter
        {
            return false;
        }
    }
    if left.word == W::RegionalIndicator && right.word == W::RegionalIndicator {
        let mut count = 0;
        let mut current = Some(left_index);
        while let Some(index) = current {
            if units[index].word != W::RegionalIndicator {
                break;
            }
            count += 1;
            current = previous(units, index);
        }
        if count % 2 == 1 {
            return false;
        }
    }
    // ICU's chained dictionary runs are adjacent characters, whereas the
    // Katakana rule above also permits intervening Format/Extend characters.
    if immediate.kana_kanji && right.kana_kanji {
        return false;
    }
    if immediate.hangul && right.hangul {
        return false;
    }
    true
}

pub(super) fn boundaries(text: &str) -> Vec<usize> {
    let units = text
        .char_indices()
        .map(|(offset, character)| Unit::new(offset, character))
        .collect::<Vec<_>>();
    if units.is_empty() {
        return vec![0];
    }
    let mut raw = vec![0];
    raw.extend((1..units.len()).filter(|&index| boundary(&units, index)));
    raw.push(units.len());
    let mut result = vec![0];
    for pair in raw.windows(2) {
        let start = pair[0];
        let end = pair[1];
        let end_offset = units.get(end).map_or(text.len(), |unit| unit.offset);
        let mut index = start;
        while index < end {
            if units[index].kana_kanji && units[index].cjk_engine || units[index].hangul {
                let start_offset = units[index].offset;
                let hangul = units[index].hangul;
                index += 1;
                while index < end
                    && if hangul {
                        units[index].hangul
                    } else {
                        units[index].cjk_engine
                    }
                {
                    index += 1;
                }
                let limit = units.get(index).map_or(text.len(), |unit| unit.offset);
                result.extend(
                    cjk::boundaries(&text[start_offset..limit])
                        .into_iter()
                        .map(|offset| start_offset + offset),
                );
            } else if let Some(language) = Language::of(units[index].character) {
                let start_offset = units[index].offset;
                index += 1;
                while index < end && language.handles(units[index].character) {
                    index += 1;
                }
                let limit = units.get(index).map_or(text.len(), |unit| unit.offset);
                result.extend(
                    southeast_asian::boundaries(&text[start_offset..limit], language)
                        .into_iter()
                        .map(|offset| start_offset + offset),
                );
            } else {
                index += 1;
            }
        }
        result.push(end_offset);
    }
    result.sort_unstable();
    result.dedup();
    result
}
