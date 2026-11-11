//! The typography probe: what a rule pack's text rules cannot read off the
//! page snapshot. For each Han-dominant paragraph and heading it records the
//! text on every rendered line and the fonts that actually rendered the Han
//! characters. It measures; it never judges.
//!
//! Which font rendered a character is only known to the browser after layout
//! and only reachable over CDP, and a node's answer mixes every script in it.
//! So the probe copies each element's Han characters, one font request at a
//! time, into a hidden container, reads the fonts of those copies, and removes
//! the container again. Characters are never pooled across elements: a fact
//! belongs to the passage it was measured on.

use std::collections::HashMap;

use impeccable_core::browser::dom::{
    ElId, FailedWebFont, HanFontFacts, HanFontRun, RenderedFont, GENERIC_FONT_FAMILIES,
};
use serde_json::Value;

use crate::cdp::{CdpResult, Page};

/// What the probe measured, keyed by snapshot element id.
#[derive(Debug, Default, PartialEq)]
pub struct TypographyFacts {
    pub line_texts: HashMap<ElId, Vec<String>>,
    pub han_fonts: HashMap<ElId, HanFontFacts>,
}

const PROBE_SELECTOR: &str = "[data-phoenix-ui-font-probe] > span";

const CLEANUP_JS: &str =
    "document.querySelectorAll('[data-phoenix-ui-font-probe]').forEach(node => node.remove())";

/// Reading a node's fonts over CDP does not lay it out first: a node the
/// browser has not laid out yet reports no fonts at all. Measuring the
/// container makes the browser lay the probes out now.
const LAYOUT_JS: &str =
    "document.querySelectorAll('[data-phoenix-ui-font-probe]').forEach(node => node.getBoundingClientRect())";

/// Runs against the capture in `window.__impCap`, whose element table gives
/// the snapshot ids. The candidate test restates the text rules' own scope so
/// only passages a rule will read are measured.
const MEASURE_JS: &str = r##"(() => {
  document.querySelectorAll('[data-phoenix-ui-font-probe]').forEach(node => node.remove());
  const cap = window.__impCap;
  const out = { lines: [], runs: [], probes: 0 };
  if (!cap || !cap.elements) return out;
  const BODY = new Set(['P', 'LI', 'TD', 'TH', 'DD', 'BLOCKQUOTE', 'FIGCAPTION']);
  const HEADING = new Set(['H1', 'H2', 'H3', 'H4', 'H5', 'H6']);
  const HAN = /\p{Script=Han}/u;
  const HAN_ALL = /\p{Script=Han}/gu;
  const PUNCT_ALL = /\p{P}/gu;
  // Unicode White_Space, which is what the rules skip; JS `\s` differs.
  const SPACE_ALL = /[\t-\r \u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]/g;
  const count = (text, re) => (text.match(re) || []).length;
  const hanDominant = (text) => {
    const visible = [...text].length - count(text, SPACE_ALL) - count(text, PUNCT_ALL);
    return visible > 0 && count(text, HAN_ALL) * 2 > visible;
  };
  const rendered = (el) => {
    for (let cur = el; cur; cur = cur.parentElement) {
      if (cur.getAttribute('aria-hidden') === 'true') return false;
      const style = getComputedStyle(cur);
      const visibility = style.visibility.toLowerCase();
      if (style.display === 'none' || visibility === 'hidden' || visibility === 'collapse') return false;
      if ((parseFloat(style.opacity) || 0) <= 0.01) return false;
      if ((style.contentVisibility || '').toLowerCase() === 'hidden') return false;
    }
    return true;
  };
  const candidate = (el) => {
    const tag = el.tagName.toUpperCase();
    const direct = [...el.childNodes].filter(node => node.nodeType === 3);
    if (BODY.has(tag)) {
      if (!direct.some(node => node.data.trim().length > 10)) return false;
    } else if (HEADING.has(tag)) {
      if (!direct.some(node => node.data.trim() !== '')) return false;
    } else {
      return false;
    }
    return hanDominant(el.textContent || '') && rendered(el);
  };

  // A block nested in the element (a sub-list in a list item) is its own
  // passage; only text that flows in the element's own lines belongs to it.
  const inlineCache = new Map();
  const flowsInline = (el) => {
    let inline = inlineCache.get(el);
    if (inline === undefined) {
      const display = getComputedStyle(el).display;
      inline = display.startsWith('inline') || display === 'contents' || display === 'ruby';
      inlineCache.set(el, inline);
    }
    return inline;
  };
  const textNodes = (el) => {
    const nodes = [];
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      let inline = true;
      for (let up = node.parentElement; up && up !== el; up = up.parentElement) {
        if (!flowsInline(up)) { inline = false; break; }
      }
      if (inline) nodes.push(node);
    }
    return nodes;
  };

  // One entry per character that rendered. A character without a box of its
  // own (collapsed whitespace, anything under display:none) is not on a line.
  const charCache = new Map();
  const range = document.createRange();
  const charsOf = (node) => {
    let chars = charCache.get(node);
    if (chars) return chars;
    chars = [];
    const text = node.data;
    for (let i = 0; i < text.length;) {
      const size = text.codePointAt(i) > 0xffff ? 2 : 1;
      range.setStart(node, i);
      range.setEnd(node, i + size);
      for (const rect of range.getClientRects()) {
        if (rect.width > 0 && rect.height > 0) {
          chars.push({ ch: text.slice(i, i + size), top: rect.top, bottom: rect.bottom });
          break;
        }
      }
      i += size;
    }
    charCache.set(node, chars);
    return chars;
  };
  // Same row test as the snapshot's line rects: vertical bands that overlap
  // by more than half the shorter one are one line.
  const linesOf = (nodes) => {
    const lines = [];
    for (const node of nodes) {
      for (const c of charsOf(node)) {
        let line = null;
        for (let i = lines.length - 1; i >= 0; i--) {
          const overlap = Math.min(lines[i].bottom, c.bottom) - Math.max(lines[i].top, c.top);
          const shorter = Math.min(lines[i].bottom - lines[i].top, c.bottom - c.top);
          if (overlap > shorter / 2) { line = lines[i]; break; }
        }
        if (!line) {
          line = { top: c.top, bottom: c.bottom, text: '' };
          lines.push(line);
        }
        line.text += c.ch;
      }
    }
    return lines.sort((a, b) => a.top - b.top).map(line => line.text);
  };

  const unquote = (name) => name.trim().replace(/^["']|["']$/g, '').trim();
  const WEIGHTS = { normal: 400, bold: 700 };
  const STRETCHES = {
    'ultra-condensed': 50, 'extra-condensed': 62.5, condensed: 75, 'semi-condensed': 87.5,
    normal: 100, 'semi-expanded': 112.5, expanded: 125, 'extra-expanded': 150, 'ultra-expanded': 200,
  };
  const span = (value, keywords, fallback) => {
    const nums = String(value).trim().split(/\s+/)
      .map(token => token in keywords ? keywords[token] : parseFloat(token))
      .filter(Number.isFinite);
    return nums.length ? [Math.min(...nums), Math.max(...nums)] : [fallback, fallback];
  };
  const slant = (value) => value.startsWith('italic') ? 'italic' : value.startsWith('oblique') ? 'oblique' : 'normal';
  const unicodeRanges = (value) => String(value).split(',').map(part => {
    const m = /U\+([0-9a-f?]+)(?:-([0-9a-f]+))?/i.exec(part);
    if (!m) return null;
    if (m[2]) return [parseInt(m[1], 16), parseInt(m[2], 16)];
    return [parseInt(m[1].replace(/\?/g, '0'), 16), parseInt(m[1].replace(/\?/g, 'f'), 16)];
  }).filter(Boolean);
  const faces = [];
  for (const face of document.fonts) {
    faces.push({
      family: unquote(face.family).toLowerCase(),
      weight: span(face.weight, WEIGHTS, 400),
      stretch: span(face.stretch, STRETCHES, 100),
      style: slant(face.style),
      ranges: unicodeRanges(face.unicodeRange),
      status: face.status,
      raw: { weight: face.weight, style: face.style, stretch: face.stretch, unicodeRange: face.unicodeRange },
    });
  }
  // CSS font matching within one family: stretch, then style, then weight.
  // What is left is the composite face (its unicode-range subsets).
  const best = (list, score) => {
    const scored = list.map(face => [face, score(face)]);
    const min = Math.min(...scored.map(([, s]) => s));
    return scored.filter(([, s]) => s === min).map(([face]) => face);
  };
  const matchFaces = (family, run) => {
    const stretch = parseFloat(run.stretch) || 100;
    const weight = parseFloat(run.weight) || 400;
    const style = slant(run.style);
    let list = best(family, ({ stretch: [lo, hi] }) => {
      if (stretch >= lo && stretch <= hi) return 0;
      const narrower = stretch > hi;
      const first = (stretch <= 100) === narrower;
      return (first ? 1e4 : 2e4) + (narrower ? stretch - hi : lo - stretch);
    });
    const order = style === 'italic' ? ['italic', 'oblique', 'normal']
      : style === 'oblique' ? ['oblique', 'italic', 'normal'] : ['normal', 'oblique', 'italic'];
    list = best(list, face => order.indexOf(face.style));
    return best(list, ({ weight: [lo, hi] }) => {
      if (weight >= lo && weight <= hi) return 0;
      if (weight >= 400 && weight <= 500) {
        if (lo > weight && lo <= 500) return 1e4 + (lo - weight);
        if (hi < weight) return 2e4 + (weight - hi);
        return 3e4 + (lo - weight);
      }
      if (weight < 400) return hi < weight ? 1e4 + (weight - hi) : 2e4 + (lo - weight);
      return lo > weight ? 1e4 + (lo - weight) : 2e4 + (weight - hi);
    });
  };

  const probes = [];
  const probe = (run, family, text) => probes.push({ run, family, text }) - 1;
  const els = cap.elements;
  for (let id = 1; id < els.length; id++) {
    const el = els[id];
    if (!el || el.nodeType !== 1 || !candidate(el)) continue;
    const nodes = textNodes(el);
    const lines = linesOf(nodes);
    if (!lines.length) continue;
    out.lines.push([id, lines]);
    const sample = [...lines.join('').trim()].slice(0, 40).join('');

    // Text nodes that ask for the same font are one run; an inline that picks
    // another face on purpose is a run of its own.
    const runs = new Map();
    for (const node of nodes) {
      const han = charsOf(node).filter(c => HAN.test(c.ch)).map(c => c.ch).join('');
      if (!han) continue;
      const style = getComputedStyle(node.parentElement);
      const langEl = node.parentElement.closest('[lang]');
      const lang = langEl ? langEl.getAttribute('lang') || '' : '';
      const key = [style.fontFamily, style.fontWeight, style.fontStyle, style.fontStretch, lang].join('\u0000');
      let run = runs.get(key);
      if (!run) {
        run = { family: style.fontFamily, weight: style.fontWeight, style: style.fontStyle, stretch: style.fontStretch, lang, text: '' };
        runs.set(key, run);
      }
      run.text += han;
    }
    for (const run of runs.values()) {
      const tokens = run.family.split(',').map(token => token.trim()).filter(Boolean);
      const names = tokens.map(unquote);
      const chars = [...run.text];
      const record = {
        el: id, sample, family: run.family, families: names,
        weight: run.weight, style: run.style, stretch: run.stretch, lang: run.lang,
        loading: false, probe: probe(run, run.family, run.text), failed: [],
      };
      names.forEach((name, index) => {
        const family = faces.filter(face => face.family === name.toLowerCase());
        if (!family.length) return;
        let lost = '';
        let failed = null;
        for (const face of matchFaces(family, run)) {
          const covered = chars.filter(ch => {
            const code = ch.codePointAt(0);
            return face.ranges.some(([lo, hi]) => code >= lo && code <= hi);
          }).join('');
          if (!covered) continue;
          if (face.status === 'loading') record.loading = true;
          if (face.status === 'error') {
            lost += covered;
            failed = failed || face;
          }
        }
        if (!failed) return;
        // The family on its own says whether another of its faces still
        // renders this Han. The declarations before it, against a family
        // that does not exist, say whether they took the Han first.
        record.failed.push({
          family: name, index, ...failed.raw,
          alone: probe(run, tokens[index], lost),
          prefix: index > 0 ? probe(run, tokens.slice(0, index).join(', '), lost) : null,
          baseline: index > 0 ? probe(run, '"phoenix-ui-no-such-font"', lost) : null,
        });
      });
      out.runs.push(record);
    }
  }

  out.probes = probes.length;
  if (!probes.length) return out;
  const box = document.createElement('div');
  box.setAttribute('data-phoenix-ui-font-probe', '');
  box.setAttribute('aria-hidden', 'true');
  box.style.cssText = 'position:fixed;left:-99999px;top:0;visibility:hidden;pointer-events:none;white-space:nowrap;font-size:16px';
  for (const { run, family, text } of probes) {
    const node = document.createElement('span');
    node.style.setProperty('font-family', family, 'important');
    node.style.setProperty('font-weight', run.weight, 'important');
    node.style.setProperty('font-style', run.style, 'important');
    node.style.setProperty('font-stretch', run.stretch, 'important');
    if (run.lang) node.setAttribute('lang', run.lang);
    node.textContent = text;
    box.appendChild(node);
  }
  document.documentElement.appendChild(box);
  box.getBoundingClientRect();
  return out;
})()"##;

/// Measure the loaded page. `window.__impCap` must hold the capture whose ids
/// the facts are keyed by. The probe container is gone when this returns.
/// Passages whose fonts could not be established get no font fact and a line
/// in `notes` instead.
pub fn collect(
    page: &mut Page<'_>,
    url: &str,
    notes: &mut Vec<String>,
) -> CdpResult<TypographyFacts> {
    let measured = page.evaluate_value(MEASURE_JS)?;
    let fonts = if measured["probes"].as_u64().unwrap_or(0) == 0 {
        Ok(Vec::new())
    } else {
        let mut fonts = page.platform_fonts(PROBE_SELECTOR);
        // The page keeps running between the measurement and the read; if it
        // invalidated layout in that gap, a probe comes back empty. Every
        // probe holds text, so an empty one is that gap: lay out and read
        // once more.
        if fonts
            .as_ref()
            .is_ok_and(|read| read.iter().any(Vec::is_empty))
        {
            page.evaluate_value(LAYOUT_JS)?;
            fonts = page.platform_fonts(PROBE_SELECTOR);
        }
        page.evaluate_value(CLEANUP_JS)?;
        fonts.map_err(|error| error.message)
    };
    let (facts, unverified) = assemble(url, &measured, fonts);
    notes.extend(unverified);
    Ok(facts)
}

/// Turn one measurement and the fonts read for its probes into facts, plus a
/// note for every passage whose fonts stay unverified. An element gets a font
/// fact only when each of its runs is established; a verified run never
/// stands in for an unverified one.
fn assemble(
    url: &str,
    measured: &Value,
    fonts: Result<Vec<Vec<RenderedFont>>, String>,
) -> (TypographyFacts, Vec<String>) {
    let mut facts = TypographyFacts::default();
    let mut notes = Vec::new();
    for entry in measured["lines"].as_array().into_iter().flatten() {
        let Some(id) = entry[0].as_u64() else {
            continue;
        };
        let lines = entry[1]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|line| line.as_str().map(str::to_string))
            .collect();
        facts.line_texts.insert(id as ElId, lines);
    }
    let runs: Vec<&Value> = measured["runs"].as_array().into_iter().flatten().collect();
    if runs.is_empty() {
        return (facts, notes);
    }
    let expected = measured["probes"].as_u64().unwrap_or(0) as usize;
    let fonts = match fonts {
        Ok(fonts) if fonts.len() == expected => fonts,
        Ok(fonts) => {
            notes.push(format!(
                "font sources not verified at {url} (whole page): read {} of {expected} font probes",
                fonts.len()
            ));
            return (facts, notes);
        }
        Err(error) => {
            notes.push(format!(
                "font sources not verified at {url} (whole page): {error}"
            ));
            return (facts, notes);
        }
    };
    let read = |index: &Value| -> Vec<RenderedFont> {
        index
            .as_u64()
            .and_then(|i| fonts.get(i as usize))
            .map(|list| merge_fonts(list))
            .unwrap_or_default()
    };

    let mut unverified: Vec<ElId> = Vec::new();
    for run in runs {
        let Some(el) = run["el"].as_u64().map(|id| id as ElId) else {
            continue;
        };
        let text = |key: &str| run[key].as_str().unwrap_or("").to_string();
        let declared: Vec<String> = run["families"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|name| name.as_str().map(str::to_string))
            .collect();
        let rendered = read(&run["probe"]);
        let mut failed_web_fonts = Vec::new();
        let mut reason = if run["loading"].as_bool() == Some(true) {
            Some("a matching web font is still loading".to_string())
        } else if rendered.is_empty() {
            Some("the browser reported no font for this text".to_string())
        } else {
            None
        };
        if reason.is_none() {
            for failed in run["failed"].as_array().into_iter().flatten() {
                let family = failed["family"].as_str().unwrap_or("");
                let index = failed["index"].as_u64().unwrap_or(0) as usize;
                let alone = read(&failed["alone"]);
                if alone.is_empty() {
                    reason = Some(format!(
                        "could not tell whether the failed web font \"{family}\" still renders this text"
                    ));
                    break;
                }
                // Another face of the family still renders this Han: the
                // failed face is not what the text depends on.
                if alone.iter().any(|font| font.custom) {
                    continue;
                }
                let earlier = declared.get(..index).unwrap_or(&[]);
                let Some(earlier_declared_rendered) = earlier_rendered(
                    earlier,
                    &read(&failed["prefix"]),
                    &read(&failed["baseline"]),
                ) else {
                    reason = Some(format!(
                        "could not tell whether the failed web font \"{family}\" or a family declared before it was the source"
                    ));
                    break;
                };
                failed_web_fonts.push(FailedWebFont {
                    family: family.to_string(),
                    weight: failed["weight"].as_str().unwrap_or("").to_string(),
                    style: failed["style"].as_str().unwrap_or("").to_string(),
                    stretch: failed["stretch"].as_str().unwrap_or("").to_string(),
                    unicode_range: failed["unicodeRange"].as_str().unwrap_or("").to_string(),
                    declaration_index: index,
                    earlier_declared_rendered,
                });
            }
        }
        if let Some(reason) = reason {
            let note = format!(
                "font source not verified at {url} element {el} \"{}\": {reason} (font-family: {})",
                text("sample"),
                text("family")
            );
            if !notes.contains(&note) {
                notes.push(note);
            }
            unverified.push(el);
            continue;
        }
        facts
            .han_fonts
            .entry(el)
            .or_default()
            .runs
            .push(HanFontRun {
                declared_families: declared,
                weight: text("weight"),
                style: text("style"),
                stretch: text("stretch"),
                lang: text("lang"),
                rendered,
                failed_web_fonts,
            });
    }
    for el in unverified {
        facts.han_fonts.remove(&el);
    }
    (facts, notes)
}

/// Whether the families declared before a failed face rendered the Han it
/// covers. `prefix` is what rendered with only those families declared,
/// `baseline` what rendered with none that exists. `None` is "cannot tell".
fn earlier_rendered(
    earlier: &[String],
    prefix: &[RenderedFont],
    baseline: &[RenderedFont],
) -> Option<bool> {
    if earlier.is_empty() {
        return Some(false);
    }
    if prefix.is_empty() || baseline.is_empty() {
        return None;
    }
    let generic = |name: &String| GENERIC_FONT_FAMILIES.contains(&name.to_lowercase().as_str());
    let named = prefix.iter().any(|font| {
        font.custom
            || earlier
                .iter()
                .any(|name| !generic(name) && name.eq_ignore_ascii_case(&font.family))
    });
    if named || prefix != baseline {
        return Some(true);
    }
    // Same fonts as with nothing declared. A generic family resolves to the
    // platform's own choice, which can be exactly that fallback, so the two
    // cannot be told apart.
    if earlier.iter().any(generic) {
        return None;
    }
    Some(false)
}

/// One entry per font, most glyphs first: the browser can list one family
/// more than once (regular and bold faces).
fn merge_fonts(fonts: &[RenderedFont]) -> Vec<RenderedFont> {
    let mut merged: Vec<RenderedFont> = Vec::new();
    for font in fonts {
        if font.family.is_empty() || font.glyphs == 0 {
            continue;
        }
        match merged
            .iter_mut()
            .find(|seen| seen.family == font.family && seen.custom == font.custom)
        {
            Some(seen) => seen.glyphs += font.glyphs,
            None => merged.push(font.clone()),
        }
    }
    merged.sort_by(|a, b| {
        b.glyphs
            .cmp(&a.glyphs)
            .then_with(|| a.family.cmp(&b.family))
    });
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const URL: &str = "http://example.test/article";

    fn font(family: &str, custom: bool, glyphs: u32) -> RenderedFont {
        RenderedFont {
            family: family.to_string(),
            custom,
            glyphs,
        }
    }

    /// One paragraph (element 7) with one run on probe 0.
    fn measured(run: Value, probes: u64) -> Value {
        let mut run = run;
        let defaults = json!({
            "el": 7, "sample": "排版的目标是让读者忘记排版本身。",
            "family": "\"Fixture Han\", sans-serif", "families": ["Fixture Han", "sans-serif"],
            "weight": "400", "style": "normal", "stretch": "100%", "lang": "zh-CN",
            "loading": false, "probe": 0, "failed": [],
        });
        for (key, value) in defaults.as_object().unwrap() {
            if run.get(key).is_none() {
                run[key] = value.clone();
            }
        }
        json!({ "lines": [[7, ["排版的目标是让读者忘记排版本身。"]]], "runs": [run], "probes": probes })
    }

    fn failed_face(index: u64, alone: u64, prefix: Value, baseline: Value) -> Value {
        json!({
            "family": "Fixture Han", "index": index, "weight": "normal", "style": "normal",
            "stretch": "normal", "unicodeRange": "U+0-10FFFF",
            "alone": alone, "prefix": prefix, "baseline": baseline,
        })
    }

    #[test]
    fn verified_fonts_become_a_fact_and_leave_no_note() {
        let (facts, notes) = assemble(
            URL,
            &measured(json!({}), 1),
            Ok(vec![vec![
                font("PingFang SC", false, 9),
                font("PingFang SC", false, 6),
            ]]),
        );
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(
            facts.line_texts[&7],
            vec!["排版的目标是让读者忘记排版本身。"]
        );
        let run = &facts.han_fonts[&7].runs[0];
        assert_eq!(run.declared_families, vec!["Fixture Han", "sans-serif"]);
        // The same family listed twice is one font.
        assert_eq!(run.rendered, vec![font("PingFang SC", false, 15)]);
        assert!(run.failed_web_fonts.is_empty());
    }

    #[test]
    fn a_failed_font_read_leaves_the_whole_page_unverified() {
        let (facts, notes) = assemble(
            URL,
            &measured(json!({}), 1),
            Err("CSS agent was not enabled".to_string()),
        );
        assert!(facts.han_fonts.is_empty());
        assert_eq!(
            facts.line_texts.len(),
            1,
            "line texts do not depend on the font read"
        );
        assert_eq!(
            notes,
            vec!["font sources not verified at http://example.test/article (whole page): CSS agent was not enabled"]
        );
    }

    #[test]
    fn a_font_still_loading_is_a_note_that_names_the_passage() {
        let (facts, notes) = assemble(
            URL,
            &measured(json!({ "loading": true }), 1),
            Ok(vec![vec![font("PingFang SC", false, 15)]]),
        );
        assert!(facts.han_fonts.is_empty());
        assert_eq!(
            notes,
            vec!["font source not verified at http://example.test/article element 7 \"排版的目标是让读者忘记排版本身。\": a matching web font is still loading (font-family: \"Fixture Han\", sans-serif)"]
        );
    }

    #[test]
    fn no_reported_font_is_a_note_even_when_a_web_font_failed() {
        let run = json!({ "failed": [failed_face(0, 1, Value::Null, Value::Null)] });
        let (facts, notes) = assemble(URL, &measured(run, 2), Ok(vec![vec![], vec![]]));
        assert!(
            facts.han_fonts.is_empty(),
            "an empty font list is never a fact"
        );
        assert_eq!(notes.len(), 1);
        assert!(notes[0].contains("element 7"), "{notes:?}");
        assert!(
            notes[0].contains("the browser reported no font"),
            "{notes:?}"
        );
    }

    #[test]
    fn a_failed_face_is_recorded_only_when_its_family_no_longer_renders_the_han() {
        let run = json!({ "failed": [failed_face(0, 1, Value::Null, Value::Null)] });

        // The family alone falls through to a system font: the failure is real.
        let (facts, notes) = assemble(
            URL,
            &measured(run.clone(), 2),
            Ok(vec![
                vec![font("PingFang SC", false, 15)],
                vec![font("PingFang SC", false, 15)],
            ]),
        );
        assert!(notes.is_empty(), "{notes:?}");
        let failed = &facts.han_fonts[&7].runs[0].failed_web_fonts;
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].family, "Fixture Han");
        assert!(!failed[0].earlier_declared_rendered);

        // Another face of the family (the regular beside a failed bold) still
        // renders it: nothing failed for this run.
        let (facts, notes) = assemble(
            URL,
            &measured(run, 2),
            Ok(vec![
                vec![font("Fixture-Regular", true, 15)],
                vec![font("Fixture-Regular", true, 15)],
            ]),
        );
        assert!(notes.is_empty(), "{notes:?}");
        assert!(facts.han_fonts[&7].runs[0].failed_web_fonts.is_empty());
    }

    #[test]
    fn families_declared_before_a_failed_face_decide_who_was_the_source() {
        let stack = |first: &str| {
            json!({
                "family": format!("{first}, \"Fixture Han\""),
                "families": [first, "Fixture Han"],
                "failed": [failed_face(1, 1, json!(2), json!(3))],
            })
        };
        let fallback = vec![font("PingFang SC", false, 15)];
        let read = |prefix: Vec<RenderedFont>| {
            Ok(vec![
                fallback.clone(),
                fallback.clone(),
                prefix,
                fallback.clone(),
            ])
        };

        // A named family in front that rendered the Han: the failed face was
        // never the source.
        let (facts, notes) = assemble(
            URL,
            &measured(stack("Songti SC"), 4),
            read(vec![font("Songti SC", false, 15)]),
        );
        assert!(notes.is_empty(), "{notes:?}");
        assert!(facts.han_fonts[&7].runs[0].failed_web_fonts[0].earlier_declared_rendered);

        // A Latin face in front changes nothing: the failed face was the source.
        let (facts, notes) = assemble(URL, &measured(stack("Inter"), 4), read(fallback.clone()));
        assert!(notes.is_empty(), "{notes:?}");
        assert!(!facts.han_fonts[&7].runs[0].failed_web_fonts[0].earlier_declared_rendered);

        // A generic in front that resolves to the fallback itself cannot be
        // told from the fallback: unverified, never a fact.
        let (facts, notes) = assemble(
            URL,
            &measured(stack("sans-serif"), 4),
            read(fallback.clone()),
        );
        assert!(facts.han_fonts.is_empty());
        assert_eq!(notes.len(), 1);
        assert!(
            notes[0].contains("element 7") && notes[0].contains("\"Fixture Han\""),
            "{notes:?}"
        );
    }

    #[test]
    fn one_unverified_run_leaves_its_element_without_a_fact_and_others_intact() {
        let mut page = measured(json!({}), 3);
        let runs = page["runs"].as_array_mut().unwrap();
        runs.push(json!({
            "el": 7, "sample": "排版的目标是让读者忘记排版本身。", "family": "\"Slow Han\"",
            "families": ["Slow Han"], "weight": "700", "style": "normal", "stretch": "100%",
            "lang": "zh-CN", "loading": true, "probe": 1, "failed": [],
        }));
        runs.push(json!({
            "el": 9, "sample": "另一段文字。", "family": "sans-serif", "families": ["sans-serif"],
            "weight": "400", "style": "normal", "stretch": "100%", "lang": "zh-CN",
            "loading": false, "probe": 2, "failed": [],
        }));
        let fonts = vec![font("PingFang SC", false, 15)];
        let (facts, notes) = assemble(URL, &page, Ok(vec![fonts.clone(), fonts.clone(), fonts]));
        assert!(
            !facts.han_fonts.contains_key(&7),
            "a verified run does not vouch for the element"
        );
        assert!(facts.han_fonts.contains_key(&9));
        assert_eq!(notes.len(), 1);
        assert!(
            notes[0].contains("element 7") && notes[0].contains("\"Slow Han\""),
            "{notes:?}"
        );
    }
}
