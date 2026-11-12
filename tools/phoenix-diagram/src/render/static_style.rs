use super::FontPolicy;
use crate::diagnostic::Diagnostics;
use crate::geometry::format_number as number;
use crate::scene::CompiledDiagram;

const ROOT: &str = "svg[data-phoenix-static]";
const VIEWER_CSS: &str = include_str!("../../viewer/viewer.css");
const FONT_TEMPLATE: &str = include_str!("../../viewer/template.source.html");
const EMBEDDED_FONT_STACK: &str = "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, 'DejaVu Sans Mono', 'Liberation Mono', 'Noto Sans Mono CJK SC', 'PingFang SC', 'Hiragino Sans GB', 'Microsoft YaHei', monospace";
const SYSTEM_FONT_STACK: &str = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";

/// Convert the shared Reader scene without changing its geometry; the
/// candidate/policy prefix namespaces local IDs and references so several
/// static figures can share one host page.
pub(super) fn render(
    scene: &CompiledDiagram,
    id_prefix: &str,
    policy: FontPolicy,
) -> Result<Vec<u8>, Diagnostics> {
    let reader_svg = super::namespace_svg(&super::render_svg(scene), id_prefix);
    let viewer = derive_stylesheet(VIEWER_CSS, SourceKind::Viewer, policy)
        .map_err(|message| super::failure(scene, "static/style", "/style/viewer.css", message))?;
    let fonts = match policy {
        FontPolicy::Embedded => Some(extract_fonts().map_err(|message| {
            super::failure(
                scene,
                "static/style",
                "/style/phoenix-diagram-fonts",
                message,
            )
        })?),
        FontPolicy::System => None,
    };
    SvgConversion {
        scene,
        id: format!("{id_prefix}-figure"),
        grid: format!("url(#{id_prefix}-grid)"),
        policy,
        viewer,
        fonts,
        common_written: false,
        background_written: false,
    }
    .convert(&reader_svg)
    .map(String::into_bytes)
    .map_err(|message| super::failure(scene, "static/style", "/svg", message))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceKind {
    Viewer,
    Figure,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Theme {
    Base,
    Light,
    Dark,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Media {
    Screen,
    Print,
    Other,
}

enum CssRule {
    Style {
        selectors: String,
        declarations: String,
    },
    Media {
        condition: String,
        rules: Vec<CssRule>,
    },
    FontFace(String),
}

fn derive_stylesheet(source: &str, kind: SourceKind, policy: FontPolicy) -> Result<String, String> {
    let source = remove_comments(source)?;
    let rules = CssParser::new(&source).parse()?;
    let mut output = String::new();
    emit_rules(&rules, kind, policy, Media::Screen, None, &mut output)?;
    if kind == SourceKind::Viewer {
        let family = match policy {
            FontPolicy::Embedded => EMBEDDED_FONT_STACK,
            FontPolicy::System => SYSTEM_FONT_STACK,
        };
        let name = match policy {
            FontPolicy::Embedded => "embedded",
            FontPolicy::System => "system",
        };
        output.push_str(&format!(
            "{ROOT}[data-font-policy=\"{name}\"]{{font-family:{family};}}"
        ));
        if matches!(policy, FontPolicy::System) {
            output.push_str(&format!(
                "{ROOT}[data-font-policy=\"system\"] text,{ROOT}[data-font-policy=\"system\"] tspan{{font-family:{SYSTEM_FONT_STACK}!important;}}"
            ));
        }
        output.push_str(&format!("{ROOT} .c-bg-rect{{fill:var(--bg);}}"));
    }
    Ok(output)
}

fn emit_rules(
    rules: &[CssRule],
    kind: SourceKind,
    policy: FontPolicy,
    media: Media,
    condition: Option<&str>,
    output: &mut String,
) -> Result<(), String> {
    for rule in rules {
        match rule {
            CssRule::FontFace(_) => {
                return Err("@font-face is only accepted in the bundled font block".into());
            }
            CssRule::Media {
                condition: nested,
                rules,
            } => {
                let normalized = nested.trim().to_ascii_lowercase();
                validate_media_condition(&normalized)?;
                if normalized == "(prefers-reduced-motion: reduce)" {
                    continue;
                }
                let (next_media, next_condition) = if normalized == "print" {
                    (Media::Print, None)
                } else {
                    if media != Media::Screen {
                        return Err(format!("nested media rule is unsupported: {nested}"));
                    }
                    (Media::Other, Some(nested.as_str()))
                };
                emit_rules(rules, kind, policy, next_media, next_condition, output)?;
            }
            CssRule::Style {
                selectors,
                declarations,
            } => {
                let mut base = Vec::new();
                let mut light = Vec::new();
                let mut dark = Vec::new();
                for selector in split_top_level(selectors, ',')? {
                    if let Some(scoped) = scope_selector(selector.trim(), kind)? {
                        match scoped.1 {
                            Theme::Base => base.push(scoped.0),
                            Theme::Light => light.push(scoped.0),
                            Theme::Dark => dark.push(scoped.0),
                        }
                    }
                }
                if base.is_empty() && light.is_empty() && dark.is_empty() {
                    continue;
                }
                let declarations = filter_declarations(declarations, policy, false)?;
                if declarations.is_empty() {
                    continue;
                }
                if media == Media::Print {
                    let selectors = [base, light, dark].concat().join(",");
                    write_rule(output, "print", &selectors, &declarations);
                    continue;
                }
                if media == Media::Other {
                    if !light.is_empty() || !dark.is_empty() {
                        return Err(format!(
                            "themed rules inside media are unsupported: {}",
                            condition.unwrap_or("")
                        ));
                    }
                    if !base.is_empty() {
                        write_rule(
                            output,
                            condition.expect("other media has a condition"),
                            &base.join(","),
                            &declarations,
                        );
                    }
                    continue;
                }
                if !base.is_empty() {
                    output.push_str(&format!("{}{{{declarations}}}", base.join(",")));
                }
                if !light.is_empty() {
                    emit_theme(output, Theme::Light, &light, &declarations);
                }
                if !dark.is_empty() {
                    emit_theme(output, Theme::Dark, &dark, &declarations);
                }
            }
        }
    }
    Ok(())
}

fn write_rule(output: &mut String, media: &str, selectors: &str, declarations: &str) {
    output.push_str(&format!("@media {media}{{{selectors}{{{declarations}}}}}"));
}

fn emit_theme(output: &mut String, theme: Theme, selectors: &[String], declarations: &str) {
    let name = if theme == Theme::Light {
        "light"
    } else {
        "dark"
    };
    let mut explicit = Vec::with_capacity(selectors.len() * 2);
    let mut automatic = Vec::with_capacity(selectors.len() * 2);
    for selector in selectors {
        let tail = selector
            .strip_prefix(ROOT)
            .expect("scoped selectors start with the static root");
        explicit.push(format!(":root[data-theme=\"{name}\"] {selector}"));
        explicit.push(format!(
            ":root[data-phoenix-static][data-theme=\"{name}\"]{tail}"
        ));
        automatic.push(format!(
            ":root:not([data-theme=\"light\"]):not([data-theme=\"dark\"]) {selector}"
        ));
        automatic.push(format!(
            ":root[data-phoenix-static]:not([data-theme=\"light\"]):not([data-theme=\"dark\"]){tail}"
        ));
    }
    output.push_str(&format!(
        "@media screen{{{}{{{declarations}}}}}",
        explicit.join(",")
    ));
    output.push_str(&format!(
        "@media screen and (prefers-color-scheme: {name}){{{}{{{declarations}}}}}",
        automatic.join(",")
    ));
    if theme == Theme::Light {
        write_rule(output, "print", &selectors.join(","), declarations);
    }
}

struct ScopedSelector(String, Theme);

fn scope_selector(selector: &str, kind: SourceKind) -> Result<Option<ScopedSelector>, String> {
    if selector.is_empty() {
        return Err("empty CSS selector".into());
    }
    if interactive_selector(selector) {
        return Ok(None);
    }
    if selector.bytes().any(|byte| byte == 0 || byte == b'\\')
        || selector.contains('<')
        || selector.contains('>') && !selector.contains(" > ")
    {
        return Err(format!("unsupported CSS selector: {selector}"));
    }
    let root_variables = root_variable_selector(selector);
    let figure_class = [".c-", ".t-", ".a-", ".m-"]
        .iter()
        .any(|prefix| selector.starts_with(prefix));
    if kind == SourceKind::Viewer
        && !root_variables
        && !figure_class
        && !references_svg_element(selector)
    {
        return Ok(None);
    }
    if kind == SourceKind::Figure && !root_variables && !references_svg_element(selector) {
        return Err(format!("non-SVG rule in figure stylesheet: {selector}"));
    }
    if selector.contains(":has(") {
        if kind == SourceKind::Viewer {
            return Ok(None);
        }
        return Err(format!("unsupported :has() in figure selector: {selector}"));
    }
    let theme = selector_theme(selector)?;
    if root_variables {
        let mut scoped = String::from(ROOT);
        scoped.push_str(&attribute_conditions(selector, "data-preset")?);
        return Ok(Some(ScopedSelector(scoped, theme)));
    }
    if figure_class {
        return Ok(Some(ScopedSelector(format!("{ROOT} {selector}"), theme)));
    }
    if selector.starts_with(":is(") {
        let close = matching_paren(selector, 3)?;
        let branches = split_top_level(&selector[4..close], ',')?;
        let mut base: Option<ScopedSelector> = None;
        for branch in branches {
            let scoped = scope_selector(branch.trim(), SourceKind::Figure)?
                .ok_or_else(|| format!("interactive branch in root selector: {selector}"))?;
            if let Some(previous) = &base {
                if previous.0 != scoped.0 || previous.1 != scoped.1 {
                    return Err(format!("root :is() branches disagree: {selector}"));
                }
            } else {
                base = Some(scoped);
            }
        }
        let mut base = base.ok_or_else(|| format!("empty root :is(): {selector}"))?;
        if base.1 == Theme::Base {
            return Err(format!("root :is() has no theme: {selector}"));
        }
        if close + 1 < selector.len() && !selector.as_bytes()[close + 1].is_ascii_whitespace() {
            return Err(format!("unsupported selector after root :is(): {selector}"));
        }
        base.0.push_str(&selector[close + 1..]);
        return Ok(Some(base));
    }
    let Some(svg) = svg_token(selector) else {
        return Err(format!("cannot find the SVG root in selector: {selector}"));
    };
    let prefix = &selector[..svg];
    let rest = &selector[svg + 3..];
    let split = root_tail_split(rest);
    let (tail, descendants) = rest.split_at(split);
    let tail = remove_theme_conditions(tail);
    if tail.contains("data-theme") || tail.contains(":root") {
        return Err(format!("unsupported root condition: {selector}"));
    }
    let mut scoped = String::from(ROOT);
    scoped.push_str(&attribute_conditions(prefix, "data-preset")?);
    scoped.push_str(&tail);
    scoped.push_str(descendants);
    Ok(Some(ScopedSelector(scoped, theme)))
}

fn interactive_selector(selector: &str) -> bool {
    let selector = selector.to_ascii_lowercase();
    const PSEUDO: [&str; 7] = [
        ":hover",
        ":focus",
        ":active",
        ":target",
        ":visited",
        ":focus-within",
        ":focus-visible",
    ];
    if PSEUDO.iter().any(|value| selector.contains(value)) {
        return true;
    }
    const RUNTIME: [&str; 39] = [
        "data-focus",
        "data-reach",
        "data-lens",
        "data-legend-preview",
        "data-legend-selected",
        "data-legend-zero",
        "data-legend-hit",
        "data-legend-count-badge",
        "data-intent-trace",
        "data-route",
        "data-relationship",
        "data-motion",
        "data-animation",
        "data-ambient",
        "data-reader",
        "data-detail-level",
        "data-tree-collapsed",
        "data-tree-hidden",
        "data-tree-toggle",
        "data-tree-expand-all",
        "data-lifecycle-rail",
        ".ambient-",
        ".intent-trace-",
        ".route-probe-",
        ".route-journey-",
        ".relationship-hit-",
        ".relationship-focus-",
        ".relationship-flow-",
        ".relationship-pulse-",
        ".semantic-lens-",
        ".semantic-flow-",
        ".overview-map-",
        ".er-legend-hit",
        ".cl-legend-hit",
        ".tree-toggle-",
        ".tree-count-pill",
        "[data-legend-kind][role=",
        "svg[data-focus",
        "svg[data-route",
    ];
    RUNTIME.iter().any(|value| selector.contains(value))
}

fn root_variable_selector(selector: &str) -> bool {
    let mut rest = selector.trim();
    if let Some(after) = rest.strip_prefix("html") {
        rest = after;
    }
    if let Some(after) = rest.strip_prefix(":root") {
        rest = after;
    }
    while rest.starts_with("[data-theme=") || rest.starts_with("[data-preset=") {
        let Some(end) = rest.find(']') else {
            return false;
        };
        rest = &rest[end + 1..];
    }
    rest.is_empty()
}

fn selector_theme(selector: &str) -> Result<Theme, String> {
    let light = selector.contains("[data-theme=\"light\"]");
    let dark = selector.contains("[data-theme=\"dark\"]");
    match (light, dark) {
        (true, true) => Err(format!("selector mixes light and dark themes: {selector}")),
        (true, false) => Ok(Theme::Light),
        (false, true) => Ok(Theme::Dark),
        (false, false) => Ok(Theme::Base),
    }
}

fn remove_theme_conditions(selector: &str) -> String {
    selector
        .replace("[data-theme=\"light\"]", "")
        .replace("[data-theme=\"dark\"]", "")
}

fn attribute_conditions(selector: &str, name: &str) -> Result<String, String> {
    let needle = format!("[{name}=\"");
    let mut output = String::new();
    let mut rest = selector;
    while let Some(start) = rest.find(&needle) {
        let value_start = start + needle.len();
        let Some(end) = rest[value_start..].find("\"]") else {
            return Err(format!("unterminated {name} condition"));
        };
        let value = &rest[value_start..value_start + end];
        if value.is_empty()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(format!("unsupported {name} value: {value}"));
        }
        output.push_str(&needle);
        output.push_str(value);
        output.push_str("\"]");
        rest = &rest[value_start + end + 2..];
    }
    Ok(output)
}

/// `svg` inside a quoted attribute value (for example `data-format^="svg"`)
/// names no element; only occurrences outside quotes reference the figure.
fn references_svg_element(selector: &str) -> bool {
    let mut unquoted = String::with_capacity(selector.len());
    let mut quote: Option<char> = None;
    for character in selector.chars() {
        match quote {
            Some(open) if character == open => quote = None,
            Some(_) => {}
            None if character == '\'' || character == '"' => quote = Some(character),
            None => unquoted.push(character),
        }
    }
    unquoted.contains("svg")
}

fn svg_token(selector: &str) -> Option<usize> {
    let bytes = selector.as_bytes();
    let mut brackets = 0usize;
    let mut parens = 0usize;
    let mut quote = 0u8;
    let mut index = 0usize;
    while index + 3 <= bytes.len() {
        let byte = bytes[index];
        if quote != 0 {
            if byte == quote {
                quote = 0;
            }
            index += 1;
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = byte,
            b'[' => brackets += 1,
            b']' => brackets = brackets.checked_sub(1)?,
            b'(' => parens += 1,
            b')' => parens = parens.checked_sub(1)?,
            _ => {}
        }
        if brackets == 0
            && parens == 0
            && &bytes[index..index + 3] == b"svg"
            && (index == 0 || bytes[index - 1].is_ascii_whitespace() || bytes[index - 1] == b'>')
            && (index + 3 == bytes.len() || matches!(bytes[index + 3], b'[' | b':' | b' ' | b'>'))
        {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn root_tail_split(rest: &str) -> usize {
    let bytes = rest.as_bytes();
    let mut brackets = 0usize;
    let mut parens = 0usize;
    let mut quote = 0u8;
    for (index, &byte) in bytes.iter().enumerate() {
        if quote != 0 {
            if byte == quote {
                quote = 0;
            }
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = byte,
            b'[' => brackets += 1,
            b']' => brackets = brackets.saturating_sub(1),
            b'(' => parens += 1,
            b')' => parens = parens.saturating_sub(1),
            b'>' | b' ' | b'\t' | b'\n' | b'\r' if brackets == 0 && parens == 0 => {
                return index;
            }
            _ => {}
        }
    }
    rest.len()
}

fn matching_paren(input: &str, open: usize) -> Result<usize, String> {
    let bytes = input.as_bytes();
    if bytes.get(open) != Some(&b'(') {
        return Err("internal selector parser error".into());
    }
    let mut depth = 0usize;
    let mut quote = 0u8;
    for (index, &byte) in bytes.iter().enumerate().skip(open) {
        if quote != 0 {
            if byte == quote {
                quote = 0;
            }
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = byte,
            b'(' => depth += 1,
            b')' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| "unbalanced selector parentheses".to_string())?;
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => {}
        }
    }
    Err("unterminated selector parentheses".into())
}

fn filter_declarations(block: &str, policy: FontPolicy, inline: bool) -> Result<String, String> {
    let mut output = String::new();
    for declaration in split_top_level(block, ';')? {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            continue;
        }
        let colon = top_level_colon(declaration)?;
        let name = declaration[..colon].trim().to_ascii_lowercase();
        let mut value = declaration[colon + 1..].trim();
        let important = value
            .to_ascii_lowercase()
            .strip_suffix("!important")
            .is_some();
        if important {
            value = value[..value.len() - "!important".len()].trim_end();
        }
        if name.is_empty() || value.is_empty() {
            return Err(format!("malformed declaration: {declaration}"));
        }
        if inline && name == "--step" {
            continue;
        }
        if name == "font-family" {
            validate_value(value)?;
            continue;
        }
        if interactive_declaration(&name, value) {
            continue;
        }
        if !supported_declaration(&name) {
            return Err(format!("unsupported figure declaration `{name}`"));
        }
        validate_value(value)?;
        if matches!(policy, FontPolicy::System) && value.to_ascii_lowercase().contains("jetbrains")
        {
            return Err(format!(
                "font shorthand reintroduces JetBrains Mono: {declaration}"
            ));
        }
        output.push_str(&name);
        output.push(':');
        output.push_str(value);
        if important {
            output.push_str("!important");
        }
        output.push(';');
    }
    Ok(output)
}

fn interactive_declaration(name: &str, value: &str) -> bool {
    name == "animation"
        || name.starts_with("animation-")
        || name == "transition"
        || name.starts_with("transition-")
        || matches!(
            name,
            "cursor"
                | "user-select"
                | "touch-action"
                | "scroll-behavior"
                | "scroll-snap-type"
                | "scroll-snap-align"
                | "overscroll-behavior"
                | "overscroll-behavior-x"
                | "overscroll-behavior-y"
        )
        || name == "pointer-events" && !value.trim().eq_ignore_ascii_case("none")
        || matches!(
            name,
            "width" | "height" | "min-width" | "min-height" | "max-width" | "max-height"
        )
}

fn supported_declaration(name: &str) -> bool {
    name.starts_with("--")
        || matches!(
            name,
            "color"
                | "display"
                | "flex"
                | "flex-direction"
                | "flex-wrap"
                | "align-items"
                | "justify-content"
                | "gap"
                | "visibility"
                | "opacity"
                | "fill"
                | "fill-opacity"
                | "fill-rule"
                | "stroke"
                | "stroke-opacity"
                | "stroke-width"
                | "stroke-dasharray"
                | "stroke-dashoffset"
                | "stroke-linecap"
                | "stroke-linejoin"
                | "stroke-miterlimit"
                | "filter"
                | "clip-path"
                | "mask"
                | "font"
                | "font-size"
                | "font-style"
                | "font-weight"
                | "font-variant"
                | "font-variant-numeric"
                | "font-size-adjust"
                | "letter-spacing"
                | "word-spacing"
                | "text-anchor"
                | "text-decoration"
                | "text-transform"
                | "transform"
                | "transform-box"
                | "transform-origin"
                | "vector-effect"
                | "paint-order"
                | "shape-rendering"
                | "text-rendering"
                | "pointer-events"
                | "rx"
                | "ry"
        )
}

fn validate_media_condition(condition: &str) -> Result<(), String> {
    if condition.is_empty()
        || !condition.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b' ' | b'\t' | b'\n' | b'\r' | b':' | b'(' | b')' | b',' | b'-' | b'.' | b'%'
                )
        })
    {
        return Err(format!("unsupported media condition: {condition}"));
    }
    split_top_level(condition, ',').map(|_| ())
}

fn validate_value(value: &str) -> Result<(), String> {
    if value.bytes().any(|byte| byte == 0 || byte == b'\\')
        || value.contains('<')
        || value.contains('>')
        || value.contains('&')
    {
        return Err(format!("unsafe or unsupported CSS value: {value}"));
    }
    let lower = value.to_ascii_lowercase();
    for forbidden in [
        "expression(",
        "javascript:",
        "vbscript:",
        "behavior:",
        "-moz-binding",
        "@import",
        "data:",
        "http:",
        "https:",
        "//",
    ] {
        if lower.contains(forbidden) {
            return Err(format!("unsafe CSS value: {value}"));
        }
    }
    let mut rest = value;
    while let Some(start) = rest.to_ascii_lowercase().find("url(") {
        let after = &rest[start + 4..];
        let Some(end) = after.find(')') else {
            return Err(format!("unterminated CSS URL: {value}"));
        };
        let target = after[..end]
            .trim()
            .trim_matches(|character| character == '\'' || character == '"');
        if !valid_fragment(target) {
            return Err(format!(
                "only local SVG fragment URLs are allowed: {target}"
            ));
        }
        rest = &after[end + 1..];
    }
    Ok(())
}

fn valid_fragment(value: &str) -> bool {
    value.len() > 1
        && value.starts_with('#')
        && value[1..]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn remove_comments(source: &str) -> Result<String, String> {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let rest = source;
    let mut quote = 0u8;
    let mut index = 0usize;
    let mut copied = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if quote != 0 {
            if byte == b'\\' {
                index += 2;
                continue;
            }
            if byte == quote {
                quote = 0;
            }
            index += 1;
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            quote = byte;
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            output.push_str(&rest[copied..index]);
            let Some(end) = source[index + 2..].find("*/") else {
                return Err("unterminated CSS comment".into());
            };
            output.push(' ');
            index += 2 + end + 2;
            copied = index;
            continue;
        }
        index += 1;
    }
    if quote != 0 {
        return Err("unterminated CSS string".into());
    }
    output.push_str(&rest[copied..]);
    Ok(output)
}

struct CssParser<'a> {
    source: &'a str,
    position: usize,
}

impl<'a> CssParser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            position: 0,
        }
    }

    fn parse(mut self) -> Result<Vec<CssRule>, String> {
        let rules = self.rules(false)?;
        self.whitespace();
        if self.position != self.source.len() {
            return Err(format!("unexpected CSS at byte {}", self.position));
        }
        Ok(rules)
    }

    fn rules(&mut self, nested: bool) -> Result<Vec<CssRule>, String> {
        let mut rules = Vec::new();
        loop {
            self.whitespace();
            if self.position == self.source.len() {
                if nested {
                    return Err("unterminated CSS block".into());
                }
                return Ok(rules);
            }
            if self.source.as_bytes()[self.position] == b'}' {
                if !nested {
                    return Err(format!("unexpected }} at byte {}", self.position));
                }
                self.position += 1;
                return Ok(rules);
            }
            let prelude = self.prelude()?;
            let block = self.block()?;
            let trimmed = prelude.trim();
            if trimmed.is_empty() {
                return Err("empty CSS rule prelude".into());
            }
            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("@media") {
                let condition = trimmed[6..].trim();
                if condition.is_empty() {
                    return Err("@media needs a condition".into());
                }
                let nested_rules = CssParser::new(block).parse()?;
                rules.push(CssRule::Media {
                    condition: condition.to_owned(),
                    rules: nested_rules,
                });
            } else if lower.starts_with("@keyframes") || lower == "@page" {
                continue;
            } else if lower.starts_with("@font-face") {
                rules.push(CssRule::FontFace(block.to_owned()));
            } else if trimmed.starts_with('@') {
                return Err(format!("unsupported CSS at-rule: {trimmed}"));
            } else {
                rules.push(CssRule::Style {
                    selectors: trimmed.to_owned(),
                    declarations: block.to_owned(),
                });
            }
        }
    }

    fn whitespace(&mut self) {
        while self
            .source
            .as_bytes()
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }

    fn prelude(&mut self) -> Result<&'a str, String> {
        let start = self.position;
        let bytes = self.source.as_bytes();
        let mut quote = 0u8;
        let mut parens = 0usize;
        let mut brackets = 0usize;
        while self.position < bytes.len() {
            let byte = bytes[self.position];
            if quote != 0 {
                if byte == b'\\' {
                    return Err("CSS escapes are unsupported".into());
                }
                if byte == quote {
                    quote = 0;
                }
                self.position += 1;
                continue;
            }
            match byte {
                b'\'' | b'"' => quote = byte,
                b'(' => parens += 1,
                b')' => {
                    parens = parens
                        .checked_sub(1)
                        .ok_or_else(|| "unbalanced CSS parentheses".to_string())?;
                }
                b'[' => brackets += 1,
                b']' => {
                    brackets = brackets
                        .checked_sub(1)
                        .ok_or_else(|| "unbalanced CSS brackets".to_string())?;
                }
                b'{' if parens == 0 && brackets == 0 => {
                    return Ok(&self.source[start..self.position]);
                }
                b';' | b'}' if parens == 0 && brackets == 0 => {
                    return Err(format!("CSS rule needs a block at byte {}", self.position));
                }
                _ => {}
            }
            self.position += 1;
        }
        Err("unterminated CSS rule".into())
    }

    fn block(&mut self) -> Result<&'a str, String> {
        let bytes = self.source.as_bytes();
        if bytes.get(self.position) != Some(&b'{') {
            return Err(format!("expected CSS block at byte {}", self.position));
        }
        self.position += 1;
        let start = self.position;
        let mut depth = 1usize;
        let mut quote = 0u8;
        while self.position < bytes.len() {
            let byte = bytes[self.position];
            if quote != 0 {
                if byte == b'\\' {
                    return Err("CSS escapes are unsupported".into());
                }
                if byte == quote {
                    quote = 0;
                }
                self.position += 1;
                continue;
            }
            match byte {
                b'\'' | b'"' => quote = byte,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        let block = &self.source[start..self.position];
                        self.position += 1;
                        return Ok(block);
                    }
                }
                _ => {}
            }
            self.position += 1;
        }
        Err("unterminated CSS block".into())
    }
}

fn split_top_level(input: &str, delimiter: char) -> Result<Vec<&str>, String> {
    let bytes = input.as_bytes();
    let delimiter = delimiter as u8;
    let mut output = Vec::new();
    let mut start = 0usize;
    let mut quote = 0u8;
    let mut parens = 0usize;
    let mut brackets = 0usize;
    for (index, &byte) in bytes.iter().enumerate() {
        if quote != 0 {
            if byte == b'\\' {
                return Err("CSS escapes are unsupported".into());
            }
            if byte == quote {
                quote = 0;
            }
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = byte,
            b'(' => parens += 1,
            b')' => {
                parens = parens
                    .checked_sub(1)
                    .ok_or_else(|| "unbalanced CSS parentheses".to_string())?;
            }
            b'[' => brackets += 1,
            b']' => {
                brackets = brackets
                    .checked_sub(1)
                    .ok_or_else(|| "unbalanced CSS brackets".to_string())?;
            }
            _ => {}
        }
        if byte == delimiter && parens == 0 && brackets == 0 {
            output.push(&input[start..index]);
            start = index + 1;
        }
    }
    if quote != 0 || parens != 0 || brackets != 0 {
        return Err("unterminated CSS component".into());
    }
    output.push(&input[start..]);
    Ok(output)
}

fn top_level_colon(input: &str) -> Result<usize, String> {
    let bytes = input.as_bytes();
    let mut quote = 0u8;
    let mut parens = 0usize;
    for (index, &byte) in bytes.iter().enumerate() {
        if quote != 0 {
            if byte == quote {
                quote = 0;
            }
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = byte,
            b'(' => parens += 1,
            b')' => parens = parens.saturating_sub(1),
            b':' if parens == 0 => return Ok(index),
            _ => {}
        }
    }
    Err(format!("declaration has no colon: {input}"))
}

fn extract_fonts() -> Result<String, String> {
    let marker = "<style id=\"phoenix-diagram-fonts\">";
    let start = FONT_TEMPLATE
        .find(marker)
        .ok_or_else(|| "bundled font style block is missing".to_string())?
        + marker.len();
    if FONT_TEMPLATE[start..].contains(marker) {
        return Err("bundled font style block is not unique".into());
    }
    let end = FONT_TEMPLATE[start..]
        .find("</style>")
        .ok_or_else(|| "unterminated bundled font style block".to_string())?
        + start;
    let raw = &FONT_TEMPLATE[start..end];
    let stripped = remove_comments(raw)?;
    let rules = CssParser::new(&stripped).parse()?;
    if rules.len() != 6 {
        return Err(format!("expected six font faces, found {}", rules.len()));
    }
    for rule in &rules {
        let CssRule::FontFace(block) = rule else {
            return Err("bundled font block contains a non-font rule".into());
        };
        validate_font_face(block)?;
    }
    if raw.to_ascii_lowercase().contains("</style") {
        return Err("unsafe bundled font style content".into());
    }
    Ok(format!("<style>{raw}</style>"))
}

fn validate_font_face(block: &str) -> Result<(), String> {
    let mut family = false;
    let mut source = false;
    for declaration in split_top_level(block, ';')? {
        if declaration.trim().is_empty() {
            continue;
        }
        let colon = top_level_colon(declaration)?;
        let name = declaration[..colon].trim().to_ascii_lowercase();
        let value = declaration[colon + 1..].trim();
        match name.as_str() {
            "font-family" => {
                family = value == "'JetBrains Mono'" || value == "\"JetBrains Mono\"";
                if !family {
                    return Err(format!("unexpected bundled font family: {value}"));
                }
            }
            "font-style" if value == "normal" => {}
            "font-weight" if value == "400 800" => {}
            "font-display" if value == "swap" => {}
            "unicode-range" => {
                if !value.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(byte, b'+' | b'-' | b',' | b'?' | b' ' | b'\t')
                }) {
                    return Err("unsupported font unicode range".into());
                }
            }
            "src" => {
                validate_font_source(value)?;
                source = true;
            }
            _ => return Err(format!("unsupported @font-face declaration: {name}")),
        }
    }
    if !family || !source {
        return Err("font face needs the bundled family and data source".into());
    }
    Ok(())
}

fn validate_font_source(value: &str) -> Result<(), String> {
    let prefix = "url(data:font/woff2;base64,";
    if !value.starts_with(prefix) {
        return Err("font source must be an embedded WOFF2 data URL".into());
    }
    let after = &value[prefix.len()..];
    let Some(end) = after.find(')') else {
        return Err("unterminated font data URL".into());
    };
    let encoded = &after[..end];
    if !valid_base64(encoded) {
        return Err("malformed WOFF2 base64".into());
    }
    let format = after[end + 1..].trim();
    if format != "format('woff2')" && format != "format(\"woff2\")" {
        return Err(format!("unsupported font source format: {format}"));
    }
    Ok(())
}

fn valid_base64(value: &str) -> bool {
    if value.is_empty() || !value.len().is_multiple_of(4) {
        return false;
    }
    let padding = value.bytes().rev().take_while(|byte| *byte == b'=').count();
    padding <= 2
        && value[..value.len() - padding]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/'))
}

struct SvgConversion<'a> {
    scene: &'a CompiledDiagram,
    id: String,
    grid: String,
    policy: FontPolicy,
    viewer: String,
    fonts: Option<String>,
    common_written: bool,
    background_written: bool,
}

impl SvgConversion<'_> {
    fn convert(mut self, source: &str) -> Result<String, String> {
        let mut output = String::with_capacity(source.len() + self.viewer.len());
        let mut position = 0usize;
        let mut depth = 0usize;
        let mut skip_depth: Option<usize> = None;
        while let Some(relative) = source[position..].find('<') {
            let start = position + relative;
            output.push_str(&source[position..start]);
            if source[start..].starts_with("<!--") {
                let Some(end) = source[start + 4..].find("-->") else {
                    return Err("unterminated SVG comment".into());
                };
                let end = start + 4 + end + 3;
                output.push_str(&source[start..end]);
                position = end;
                continue;
            }
            if source[start..].starts_with("<!") || source[start..].starts_with("<?") {
                return Err("declarations are not accepted in static SVG".into());
            }
            let end = tag_end(source, start)?;
            let inside = &source[start + 1..end - 1];
            if let Some(closing) = inside.strip_prefix('/') {
                let name = closing.trim();
                if !valid_tag_name(name) {
                    return Err(format!("invalid SVG closing tag: {name}"));
                }
                if depth == 0 {
                    return Err(format!("unexpected closing tag: {name}"));
                }
                if skip_depth == Some(depth) {
                    skip_depth = None;
                } else if skip_depth.is_none() {
                    output.push_str(&source[start..end]);
                    if name == "desc" && depth == 2 {
                        self.write_common(&mut output);
                    }
                }
                depth -= 1;
                position = end;
                continue;
            }
            let (name, mut attributes, self_closing) = parse_opening_tag(inside)?;
            if skip_depth.is_some() {
                if !self_closing {
                    depth += 1;
                }
                position = end;
                continue;
            }
            if matches!(
                name.as_str(),
                "script"
                    | "foreignObject"
                    | "iframe"
                    | "object"
                    | "embed"
                    | "animate"
                    | "animateMotion"
                    | "animateTransform"
                    | "set"
            ) {
                return Err(format!("unsafe SVG element: {name}"));
            }
            if name == "style" {
                if self_closing {
                    return Err("self-closing SVG style element".into());
                }
                let Some(close) = source[end..].find("</style>") else {
                    return Err("unterminated SVG style element".into());
                };
                let close = end + close;
                let css = &source[end..close];
                let derived = derive_stylesheet(css, SourceKind::Figure, self.policy)?;
                if !derived.is_empty() {
                    output.push_str("<style>");
                    output.push_str(&derived);
                    output.push_str("</style>");
                }
                position = close + "</style>".len();
                continue;
            }
            if depth == 0 && name != "svg" {
                return Err(format!("static output does not start with SVG: {name}"));
            }
            if depth == 0 {
                self.root_attributes(&mut attributes)?;
            } else {
                sanitize_attributes(&mut attributes, self.policy)?;
            }
            if drop_element(&name, &attributes) {
                if !self_closing {
                    depth += 1;
                    skip_depth = Some(depth);
                }
                position = end;
                continue;
            }
            if name == "rect" && depth == 1 && is_grid(&attributes, &self.grid) {
                self.write_background(&mut output);
            }
            serialize_tag(&mut output, &name, &attributes, self_closing);
            if !self_closing {
                depth += 1;
            }
            position = end;
        }
        output.push_str(&source[position..]);
        if depth != 0 || skip_depth.is_some() {
            return Err("unbalanced SVG structure".into());
        }
        if !self.common_written || !self.background_written {
            return Err("static root metadata or background insertion point is missing".into());
        }
        Ok(output)
    }

    fn root_attributes(&self, attributes: &mut Vec<Attribute>) -> Result<(), String> {
        sanitize_attributes(attributes, self.policy)?;
        set_attribute(attributes, "id", self.id.as_str());
        set_attribute(attributes, "data-phoenix-static", "");
        set_attribute(
            attributes,
            "data-font-policy",
            match self.policy {
                FontPolicy::Embedded => "embedded",
                FontPolicy::System => "system",
            },
        );
        set_attribute(attributes, "width", &number(self.scene.view_box.width));
        set_attribute(attributes, "height", &number(self.scene.view_box.height));
        let style = attributes
            .iter_mut()
            .find(|attribute| attribute.name == "style")
            .map(|attribute| {
                if !attribute.value.is_empty() {
                    attribute.value.push(';');
                }
                attribute.value.push_str("min-width:0");
                attribute.value.clone()
            })
            .unwrap_or_else(|| "min-width:0".into());
        set_attribute(attributes, "style", &style);
        for required in ["viewBox", "role", "aria-labelledby", "lang"] {
            if !attributes
                .iter()
                .any(|attribute| attribute.name == required)
            {
                return Err(format!("Reader SVG root lost {required}"));
            }
        }
        Ok(())
    }

    fn write_common(&mut self, output: &mut String) {
        output.push_str("<style>");
        output.push_str(&self.viewer);
        output.push_str("</style>");
        if let Some(fonts) = &self.fonts {
            output.push_str(fonts);
        }
        self.common_written = true;
    }

    fn write_background(&mut self, output: &mut String) {
        let rect = self.scene.view_box;
        output.push_str(&format!(
            "<rect class=\"c-bg-rect\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
            number(rect.x),
            number(rect.y),
            number(rect.width),
            number(rect.height)
        ));
        self.background_written = true;
    }
}

#[derive(Clone)]
struct Attribute {
    name: String,
    value: String,
}

impl Attribute {
    fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

fn parse_opening_tag(input: &str) -> Result<(String, Vec<Attribute>, bool), String> {
    let mut input = input.trim();
    let self_closing = input.ends_with('/');
    if self_closing {
        input = input[..input.len() - 1].trim_end();
    }
    let bytes = input.as_bytes();
    let mut position = 0usize;
    while position < bytes.len() && !bytes[position].is_ascii_whitespace() {
        position += 1;
    }
    let name = input[..position].to_owned();
    if !valid_tag_name(&name) {
        return Err(format!("invalid SVG tag: {name}"));
    }
    let mut attributes = Vec::new();
    while position < bytes.len() {
        while position < bytes.len() && bytes[position].is_ascii_whitespace() {
            position += 1;
        }
        if position == bytes.len() {
            break;
        }
        let start = position;
        while position < bytes.len()
            && !bytes[position].is_ascii_whitespace()
            && bytes[position] != b'='
        {
            position += 1;
        }
        let name = &input[start..position];
        if name.is_empty()
            || !name.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
            })
        {
            return Err(format!("invalid SVG attribute: {name}"));
        }
        while position < bytes.len() && bytes[position].is_ascii_whitespace() {
            position += 1;
        }
        if bytes.get(position) != Some(&b'=') {
            return Err(format!("SVG attribute needs a quoted value: {name}"));
        }
        position += 1;
        while position < bytes.len() && bytes[position].is_ascii_whitespace() {
            position += 1;
        }
        let Some(&quote) = bytes
            .get(position)
            .filter(|byte| matches!(**byte, b'\'' | b'"'))
        else {
            return Err(format!("SVG attribute needs a quoted value: {name}"));
        };
        position += 1;
        let start = position;
        while position < bytes.len() && bytes[position] != quote {
            position += 1;
        }
        if position == bytes.len() {
            return Err(format!("unterminated SVG attribute: {name}"));
        }
        let value = &input[start..position];
        position += 1;
        if attributes
            .iter()
            .any(|attribute: &Attribute| attribute.name == name)
        {
            return Err(format!("duplicate SVG attribute: {name}"));
        }
        attributes.push(Attribute::new(name, value));
    }
    Ok((name, attributes, self_closing))
}

fn tag_end(source: &str, start: usize) -> Result<usize, String> {
    let bytes = source.as_bytes();
    let mut quote = 0u8;
    for (index, &byte) in bytes[start + 1..].iter().enumerate() {
        let index = start + 1 + index;
        if quote != 0 {
            if byte == quote {
                quote = 0;
            }
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = byte,
            b'>' => return Ok(index + 1),
            _ => {}
        }
    }
    Err("unterminated SVG tag".into())
}

fn valid_tag_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
}

fn sanitize_attributes(attributes: &mut Vec<Attribute>, policy: FontPolicy) -> Result<(), String> {
    let mut index = 0usize;
    while index < attributes.len() {
        let name = attributes[index].name.to_ascii_lowercase();
        if name.starts_with("on")
            || matches!(
                name.as_str(),
                "tabindex"
                    | "focusable"
                    | "aria-pressed"
                    | "aria-expanded"
                    | "aria-controls"
                    | "aria-current"
                    | "data-animate"
                    | "data-delta-animate"
                    | "data-animation"
                    | "data-legend-kind"
                    | "data-legend-label"
                    | "data-tree-initially-collapsed"
                    | "data-tree-any-collapsed"
            )
            || name.starts_with("data-motion-")
            || name.starts_with("data-reader-")
        {
            attributes.remove(index);
            continue;
        }
        if name == "role" && attributes[index].value != "img" {
            attributes.remove(index);
            continue;
        }
        if name == "style" {
            let value = filter_declarations(&attributes[index].value, policy, true)?;
            if value.is_empty() {
                attributes.remove(index);
            } else {
                attributes[index].value = value;
                index += 1;
            }
            continue;
        }
        if matches!(name.as_str(), "href" | "xlink:href") {
            validate_href(&attributes[index].value)?;
        } else if name == "src" {
            return Err("SVG src attributes are not supported".into());
        } else if matches!(
            name.as_str(),
            "fill"
                | "stroke"
                | "filter"
                | "clip-path"
                | "mask"
                | "marker-start"
                | "marker-mid"
                | "marker-end"
        ) {
            validate_value(&attributes[index].value)?;
        }
        if name == "font-family" && matches!(policy, FontPolicy::System) {
            attributes[index].value = SYSTEM_FONT_STACK.into();
        }
        index += 1;
    }
    Ok(())
}

fn validate_href(value: &str) -> Result<(), String> {
    if valid_fragment(value) {
        return Ok(());
    }
    const MIME: [&str; 5] = [
        "image/png",
        "image/jpeg",
        "image/webp",
        "image/x-icon",
        "image/vnd.microsoft.icon",
    ];
    for mime in MIME {
        let prefix = format!("data:{mime};base64,");
        if let Some(encoded) = value.strip_prefix(&prefix) {
            if valid_base64(encoded) {
                return Ok(());
            }
            return Err("malformed embedded image base64".into());
        }
    }
    Err(format!("unsafe external SVG resource: {value}"))
}

fn drop_element(name: &str, attributes: &[Attribute]) -> bool {
    if name == "g"
        && attributes.iter().any(|attribute| {
            matches!(
                attribute.name.as_str(),
                "data-tree-toggle" | "data-tree-expand-all"
            )
        })
    {
        return true;
    }
    attributes.iter().any(|attribute| {
        attribute.name == "data-legend-hit"
            || attribute.name == "class"
                && attribute.value.split_ascii_whitespace().any(|class| {
                    matches!(class, "er-legend-hit" | "cl-legend-hit" | "tree-toggle-hit")
                })
    })
}

fn is_grid(attributes: &[Attribute], fill: &str) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.name == "fill" && attribute.value == fill)
        && attributes.iter().all(|attribute| attribute.name != "class")
        && attributes
            .iter()
            .any(|attribute| attribute.name == "width" && attribute.value == "100%")
        && attributes
            .iter()
            .any(|attribute| attribute.name == "height" && attribute.value == "100%")
}

fn set_attribute(attributes: &mut Vec<Attribute>, name: &str, value: &str) {
    if let Some(attribute) = attributes
        .iter_mut()
        .find(|attribute| attribute.name == name)
    {
        attribute.value = value.into();
    } else {
        attributes.push(Attribute::new(name, value));
    }
}

fn serialize_tag(output: &mut String, name: &str, attributes: &[Attribute], self_closing: bool) {
    output.push('<');
    output.push_str(name);
    for attribute in attributes {
        output.push(' ');
        output.push_str(&attribute.name);
        output.push_str("=\"");
        output.push_str(&attribute.value);
        output.push('"');
    }
    if self_closing {
        output.push_str("/>");
    } else {
        output.push('>');
    }
}
