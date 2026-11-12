//! Locating elements and running state steps with real input events.
//! Only the main document is searched; a failure says why and offers
//! locators that do exist, written as state-file JSON.

use std::time::{Duration, Instant};

use impeccable_browser::cdp::Page;
use serde_json::{json, Value};

use super::page_js;
use super::plan::{Action, Locator, Target, Text};

#[derive(Debug, Clone)]
pub struct Located {
    pub x: f64,
    pub y: f64,
    /// Border box in document CSS pixels.
    pub doc: [f64; 4],
    pub count: u64,
    pub warnings: Vec<String>,
    pub observed: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LocateError {
    pub reason: String,
    pub hints: Vec<String>,
    /// The target exists but cannot be used yet (hidden); `waitFor` keeps polling.
    pub not_visible: bool,
    /// Why the target may live outside the main document; `waitFor` repeats it on timeout.
    pub scope_note: Option<String>,
}

impl LocateError {
    fn plain(reason: impl Into<String>) -> Self {
        LocateError {
            reason: reason.into(),
            hints: Vec::new(),
            not_visible: false,
            scope_note: None,
        }
    }
}

/// How the located element will be used.
#[derive(Clone, Copy, Default)]
pub struct Use<'a> {
    /// Scroll it into the viewport first.
    pub scroll: bool,
    /// A mouse event lands on its center: report what covers that point.
    pub pointer: bool,
    /// Page-side operation: `fill`, `select`, `scroll`, or `probe` for
    /// a plain action check (disabled state).
    pub op: Option<&'a str>,
    pub value: Option<&'a str>,
    pub delta: (f64, f64),
}

pub fn locate(
    page: &mut Page<'_>,
    target: &Target,
    usage: Use<'_>,
) -> Result<Located, LocateError> {
    let args = json!({
        "loc": target.to_json(),
        "scroll": usage.scroll,
        "pointer": usage.pointer,
        "op": usage.op,
        "value": usage.value,
        "dx": usage.delta.0,
        "dy": usage.delta.1,
    });
    let nodes = match &target.locator {
        Locator::Role { role, name } => page
            .query_ax(role, name.as_deref())
            .map_err(|e| LocateError::plain(format!("无障碍树查询失败：{}", e.message)))?
            .into_iter()
            .map(|(id, _)| id)
            .collect(),
        _ => Vec::new(),
    };
    let result = page
        .call_with_nodes(page_js::LOCATE, &args, &nodes)
        .map_err(|e| LocateError::plain(format!("页面脚本出错：{}", e.message)))?;
    let text = |key: &str| {
        result
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    match result.get("status").and_then(Value::as_str) {
        Some("ok") => {
            let doc = result.get("doc").cloned().unwrap_or(Value::Null);
            let num = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
            Ok(Located {
                x: num(&result, "x"),
                y: num(&result, "y"),
                doc: [
                    num(&doc, "x"),
                    num(&doc, "y"),
                    num(&doc, "width"),
                    num(&doc, "height"),
                ],
                count: result.get("count").and_then(Value::as_u64).unwrap_or(1),
                warnings: strings(result.get("warnings")),
                observed: result
                    .get("observed")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
        }
        Some("hidden") => Err(LocateError {
            reason: format!("{}；先执行让它出现的那一步", text("reason")),
            hints: page_hints(page, target, &result),
            not_visible: true,
            scope_note: None,
        }),
        Some("missing") => {
            let hints = page_hints(page, target, &result);
            let mut reason = format!("找不到 {}", target.display());
            let frames = result.get("iframes").and_then(Value::as_u64).unwrap_or(0);
            let shadows = result
                .get("shadowRoots")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let scope_note = (frames + shadows > 0).then(|| {
                format!(
                    "选择器只作用于主文档：页面有 {frames} 个 iframe、{shadows} 个开放 shadow root，可直接打开被嵌入的页面"
                )
            });
            if let Some(note) = &scope_note {
                reason.push_str(&format!("；{note}"));
            }
            Err(LocateError {
                reason,
                hints,
                not_visible: true,
                scope_note,
            })
        }
        _ => Err(LocateError::plain(text("reason"))),
    }
}

/// The page script's suggestions, or ranked accessibility names for a role locator.
fn page_hints(page: &mut Page<'_>, target: &Target, result: &Value) -> Vec<String> {
    match &target.locator {
        Locator::Role { role, name } => role_hints(page, role, name.as_deref().unwrap_or("")),
        _ => strings(result.get("hints")),
    }
}

fn role_hints(page: &mut Page<'_>, role: &str, want: &str) -> Vec<String> {
    let Ok(nodes) = page.query_ax(role, None) else {
        return Vec::new();
    };
    let (ids, names): (Vec<i64>, Vec<String>) = nodes.into_iter().take(200).unzip();
    let ranked = page
        .call_with_nodes(
            page_js::ROLE_HINTS,
            &json!({ "want": want, "names": names }),
            &ids,
        )
        .unwrap_or(Value::Null);
    strings(Some(&ranked))
        .into_iter()
        .map(|name| json!({ "role": role, "name": name }).to_string())
        .collect()
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The outcome of one step as the report records it.
pub struct StepOutcome {
    pub warnings: Vec<String>,
    pub observed: String,
}

pub struct Context<'a> {
    pub env: &'a dyn Fn(&str) -> Option<String>,
    pub default_timeout: Duration,
}

/// Run one step. Navigation the step starts is awaited before it returns.
pub fn execute(
    page: &mut Page<'_>,
    step: &Action,
    ctx: &Context<'_>,
) -> Result<StepOutcome, LocateError> {
    let mut notes: Vec<String> = Vec::new();
    let mut warnings = Vec::new();
    let mut found = |page: &mut Page<'_>,
                     notes: &mut Vec<String>,
                     target: &Target,
                     usage: Use<'_>|
     -> Result<Located, LocateError> {
        let located = locate(page, target, usage)?;
        if located.count > 1 {
            notes.push(format!(
                "出现 {} 个可见匹配，取第 {} 个",
                located.count,
                target.nth.unwrap_or(1)
            ));
        }
        warnings.extend(located.warnings.iter().cloned());
        Ok(located)
    };
    let pointer = |op| Use {
        scroll: true,
        pointer: true,
        op: Some(op),
        ..Use::default()
    };
    let cdp = |e: impeccable_browser::cdp::CdpError| LocateError::plain(e.message);
    let text_of = |text: &Text| -> Result<String, LocateError> {
        match text {
            Text::Literal(s) => Ok(s.clone()),
            Text::Env(name) => {
                (ctx.env)(name).ok_or_else(|| LocateError::plain(format!("环境变量 {name} 未设置")))
            }
        }
    };
    page.watch_navigation();
    match step {
        Action::Click(t) | Action::Dblclick(t) => {
            let at = found(page, &mut notes, t, pointer("probe"))?;
            click(page, at.x, at.y, 1).map_err(cdp)?;
            if matches!(step, Action::Dblclick(_)) {
                page.mouse("mousePressed", at.x, at.y, 2, 1).map_err(cdp)?;
                page.mouse("mouseReleased", at.x, at.y, 2, 0).map_err(cdp)?;
            }
        }
        Action::Hover(t) => {
            let at = found(page, &mut notes, t, pointer("probe"))?;
            page.mouse("mouseMoved", at.x, at.y, 0, 0).map_err(cdp)?;
            page.idle(Duration::from_millis(200)).map_err(cdp)?;
        }
        Action::Fill(t, text) => {
            let value = text_of(text)?;
            let at = found(page, &mut notes, t, pointer("probe"))?;
            click(page, at.x, at.y, 1).map_err(cdp)?;
            let field = locate(
                page,
                t,
                Use {
                    op: Some("fill"),
                    value: Some(&value),
                    ..Use::default()
                },
            )?;
            match field.observed {
                Some(native) => notes.push(native),
                None => page.insert_text(&value).map_err(cdp)?,
            }
            notes.push(typed("写入", text, &value));
        }
        Action::Type(t, text) => {
            let value = text_of(text)?;
            if let Some(t) = t {
                let at = found(page, &mut notes, t, pointer("probe"))?;
                click(page, at.x, at.y, 1).map_err(cdp)?;
            }
            let native = page
                .evaluate_value(page_js::FOCUSED_NATIVE_INPUT)
                .map_err(cdp)?;
            if let Some(kind) = native.as_str().filter(|k| !k.is_empty()) {
                return Err(LocateError::plain(format!(
                    "焦点在 <input type=\"{kind}\"> 上，逐字输入进不了这类原生控件，改用 fill 设值"
                )));
            }
            page.insert_text(&value).map_err(cdp)?;
            notes.push(typed("输入", text, &value));
        }
        Action::Select(t, value) => {
            let at = found(
                page,
                &mut notes,
                t,
                Use {
                    scroll: true,
                    op: Some("select"),
                    value: Some(value),
                    ..Use::default()
                },
            )?;
            notes.extend(at.observed);
        }
        Action::Press(stroke, _) => page.key(stroke).map_err(cdp)?,
        Action::Drag(t, dx, dy) => {
            let at = found(page, &mut notes, t, pointer("probe"))?;
            page.mouse("mouseMoved", at.x, at.y, 0, 0).map_err(cdp)?;
            page.mouse("mousePressed", at.x, at.y, 1, 1).map_err(cdp)?;
            for n in 1..=24 {
                let f = n as f64 / 24.0;
                page.mouse("mouseMoved", at.x + dx * f, at.y + dy * f, 0, 1)
                    .map_err(cdp)?;
                page.idle(Duration::from_millis(16)).map_err(cdp)?;
            }
            page.mouse("mouseReleased", at.x + dx, at.y + dy, 1, 0)
                .map_err(cdp)?;
        }
        Action::Scroll { dx, dy, target } => {
            match target {
                Some(t) => {
                    found(
                        page,
                        &mut notes,
                        t,
                        Use {
                            op: Some("scroll"),
                            delta: (*dx, *dy),
                            ..Use::default()
                        },
                    )?;
                }
                None => {
                    page.evaluate_value(&format!(
                        "(scrollBy({{left:{dx},top:{dy},behavior:'instant'}}),true)"
                    ))
                    .map_err(cdp)?;
                }
            }
            page.idle(Duration::from_millis(200)).map_err(cdp)?;
        }
        Action::Wait(ms) => page.idle(Duration::from_millis(*ms)).map_err(cdp)?,
        Action::WaitFor {
            target,
            hidden,
            timeout_ms,
        } => {
            let timeout = timeout_ms
                .map(Duration::from_millis)
                .unwrap_or(ctx.default_timeout);
            let start = Instant::now();
            loop {
                let probe = locate(page, target, Use::default());
                let done = match &probe {
                    Ok(_) => !hidden,
                    Err(e) if e.not_visible => *hidden,
                    Err(e) => return Err(e.clone()),
                };
                if done {
                    let waited = start.elapsed().as_millis();
                    notes.push(if *hidden {
                        format!("{waited} ms 后不再可见")
                    } else {
                        format!("{waited} ms 后可见")
                    });
                    break;
                }
                if start.elapsed() >= timeout {
                    let visible = probe.as_ref().map(|l| l.count).unwrap_or(0);
                    let mut reason = format!(
                        "等待 {} {}超时（{} ms），当前可见匹配 {visible} 个",
                        target.display(),
                        if *hidden { "消失" } else { "出现" },
                        timeout.as_millis()
                    );
                    let (hints, note) = probe
                        .err()
                        .map(|e| (e.hints, e.scope_note))
                        .unwrap_or_default();
                    if let Some(note) = note {
                        reason.push_str(&format!("；{note}"));
                    }
                    return Err(LocateError {
                        reason,
                        hints,
                        not_visible: false,
                        scope_note: None,
                    });
                }
                page.idle(Duration::from_millis(100)).map_err(cdp)?;
            }
        }
    }
    if step.acts() {
        if page
            .settle_navigation(Duration::from_millis(150), Duration::from_secs(30))
            .map_err(cdp)?
        {
            let path = page
                .evaluate_value("location.pathname + location.search")
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default();
            notes.push(format!("导航到 {path}"));
        }
        page.evaluate_value(page_js::TWO_FRAMES).map_err(cdp)?;
    }
    Ok(StepOutcome {
        warnings,
        observed: notes.join("；"),
    })
}

/// What was typed, without revealing anything about a credential.
fn typed(verb: &str, text: &Text, value: &str) -> String {
    match text {
        Text::Literal(_) => format!("{verb} {} 个字符", value.chars().count()),
        Text::Env(name) => format!("{verb}环境变量 {name} 的值"),
    }
}

fn click(
    page: &mut Page<'_>,
    x: f64,
    y: f64,
    count: u32,
) -> Result<(), impeccable_browser::cdp::CdpError> {
    page.mouse("mouseMoved", x, y, 0, 0)?;
    page.mouse("mousePressed", x, y, count, 1)?;
    page.mouse("mouseReleased", x, y, count, 0)
}
