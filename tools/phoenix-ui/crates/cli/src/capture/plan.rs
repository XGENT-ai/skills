//! The state file: which page states to capture and the steps that reach
//! each one. Everything is checked before a browser starts, and every error
//! names its JSON path (`states[1].steps[2].target`).

use std::fmt;

use impeccable_browser::cdp::KeyStroke;
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Locator {
    Css(String),
    Text(String),
    Role { role: String, name: Option<String> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub locator: Locator,
    pub nth: Option<u32>,
}

impl Target {
    /// How reports and terminal lines show the locator.
    pub fn display(&self) -> String {
        let base = match &self.locator {
            Locator::Css(css) => format!("css={css}"),
            Locator::Text(text) => format!("text={}", json!(text)),
            Locator::Role {
                role,
                name: Some(name),
            } => format!("role={role}[name={}]", json!(name)),
            Locator::Role { role, name: None } => format!("role={role}"),
        };
        match self.nth {
            Some(n) => format!("{base} >> nth={n}"),
            None => base,
        }
    }

    /// The locator as the page script reads it.
    pub fn to_json(&self) -> Value {
        let mut value = match &self.locator {
            Locator::Css(css) => json!({ "css": css }),
            Locator::Text(text) => json!({ "text": text }),
            Locator::Role { role, name } => json!({ "role": role, "name": name }),
        };
        if let Some(n) = self.nth {
            value["nth"] = json!(n);
        }
        value
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShotType {
    Viewport,
    Full,
    Element(Target),
}

impl ShotType {
    pub fn name(&self) -> &'static str {
        match self {
            ShotType::Viewport => "viewport",
            ShotType::Full => "full",
            ShotType::Element(_) => "element",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FramesFrom {
    LastStep,
    Entry,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frames {
    pub count: u32,
    pub from: FramesFrom,
}

/// Text a step types: literal, or read from an environment variable (and
/// then redacted from every output).
#[derive(Debug, Clone, PartialEq)]
pub enum Text {
    Literal(String),
    Env(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Click(Target),
    Dblclick(Target),
    Hover(Target),
    Fill(Target, Text),
    Type(Option<Target>, Text),
    Select(Target, String),
    Press(KeyStroke, String),
    Drag(Target, f64, f64),
    Scroll {
        dx: f64,
        dy: f64,
        target: Option<Target>,
    },
    Wait(u64),
    WaitFor {
        target: Target,
        hidden: bool,
        timeout_ms: Option<u64>,
    },
}

impl Action {
    pub fn name(&self) -> &'static str {
        match self {
            Action::Click(_) => "click",
            Action::Dblclick(_) => "dblclick",
            Action::Hover(_) => "hover",
            Action::Fill(..) => "fill",
            Action::Type(..) => "type",
            Action::Select(..) => "select",
            Action::Press(..) => "press",
            Action::Drag(..) => "drag",
            Action::Scroll { .. } => "scroll",
            Action::Wait(_) => "wait",
            Action::WaitFor { .. } => "waitFor",
        }
    }

    pub fn target(&self) -> Option<&Target> {
        match self {
            Action::Click(t) | Action::Dblclick(t) | Action::Hover(t) => Some(t),
            Action::Fill(t, _) | Action::Select(t, _) | Action::Drag(t, ..) => Some(t),
            Action::Type(t, _) | Action::Scroll { target: t, .. } => t.as_ref(),
            Action::WaitFor { target, .. } => Some(target),
            Action::Press(..) | Action::Wait(_) => None,
        }
    }

    pub fn env(&self) -> Option<&str> {
        match self {
            Action::Fill(_, Text::Env(name)) | Action::Type(_, Text::Env(name)) => Some(name),
            _ => None,
        }
    }

    /// Waits only observe; sequence frames start before the last real action.
    pub fn acts(&self) -> bool {
        !matches!(self, Action::Wait(_) | Action::WaitFor { .. })
    }
}

/// How a state is photographed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Shape {
    pub shot_type: Option<ShotType>,
    pub frames: Option<Frames>,
    pub mark: Vec<Target>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub name: String,
    pub steps: Vec<Action>,
    pub shape: Shape,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Plan {
    pub initial: Shape,
    pub states: Vec<State>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanError {
    pub path: String,
    pub message: String,
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

const ACTIONS: &[&str] = &[
    "click", "dblclick", "hover", "fill", "type", "select", "press", "drag", "scroll", "wait",
    "waitFor",
];
const MAX_WAIT_MS: u64 = 10_000;

pub fn valid_state_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Parse and check a state file. All problems are reported together.
pub fn parse_states(text: &str) -> Result<Plan, Vec<PlanError>> {
    let root: Value = serde_json::from_str(text).map_err(|e| {
        vec![PlanError {
            path: String::new(),
            message: format!("不是合法的 JSON：{e}"),
        }]
    })?;
    let mut p = Parser { errors: Vec::new() };
    let plan = p.plan(&root);
    if p.errors.is_empty() {
        Ok(plan)
    } else {
        Err(p.errors)
    }
}

struct Parser {
    errors: Vec<PlanError>,
}

impl Parser {
    fn err(&mut self, path: &str, message: impl Into<String>) {
        self.errors.push(PlanError {
            path: path.to_string(),
            message: message.into(),
        });
    }

    fn object<'v>(
        &mut self,
        value: &'v Value,
        path: &str,
        allowed: &[&str],
    ) -> Option<&'v Map<String, Value>> {
        let Some(obj) = value.as_object() else {
            self.err(path, "应为 JSON 对象");
            return None;
        };
        for key in obj.keys() {
            if !allowed.contains(&key.as_str()) {
                let hint = nearest(key, allowed)
                    .map(|k| format!("，是否想写 {k}？"))
                    .unwrap_or_else(|| format!("（可用：{}）", allowed.join("、")));
                self.err(&join(path, key), format!("未知字段 {key}{hint}"));
            }
        }
        Some(obj)
    }

    fn plan(&mut self, root: &Value) -> Plan {
        let mut plan = Plan::default();
        let Some(obj) = self.object(root, "", &["initial", "states"]) else {
            return plan;
        };
        if let Some(initial) = obj.get("initial") {
            if let Some(o) = self.object(initial, "initial", &["type", "frames", "mark"]) {
                plan.initial = self.shape(o, "initial", true);
            }
        }
        let Some(states) = obj.get("states") else {
            return plan;
        };
        let Some(states) = states.as_array() else {
            self.err("states", "应为数组");
            return plan;
        };
        for (i, value) in states.iter().enumerate() {
            let path = format!("states[{i}]");
            let Some(o) = self.object(value, &path, &["name", "steps", "type", "frames", "mark"])
            else {
                continue;
            };
            let name = match o.get("name").and_then(Value::as_str) {
                None => {
                    self.err(&join(&path, "name"), "缺少状态名");
                    String::new()
                }
                Some("initial") => {
                    self.err(
                        &join(&path, "name"),
                        "initial 是初始状态的保留名，不能用作状态名",
                    );
                    String::new()
                }
                Some(n) if !valid_state_name(n) => {
                    self.err(
                        &join(&path, "name"),
                        format!("状态名 {n} 只能用小写字母、数字和连字符，且以字母或数字开头"),
                    );
                    String::new()
                }
                Some(n) => {
                    if plan.states.iter().any(|s| s.name == n) {
                        self.err(&join(&path, "name"), format!("状态名 {n} 重复"));
                    }
                    n.to_string()
                }
            };
            let steps = match o.get("steps").and_then(Value::as_array) {
                Some(list) if !list.is_empty() => list
                    .iter()
                    .enumerate()
                    .filter_map(|(j, s)| self.step(s, &format!("{path}.steps[{j}]")))
                    .collect(),
                Some(_) => {
                    self.err(
                        &join(&path, "steps"),
                        "steps 不能为空；只截初始状态时用 initial",
                    );
                    Vec::new()
                }
                None => {
                    self.err(&join(&path, "steps"), "缺少 steps 数组");
                    Vec::new()
                }
            };
            let shape = self.shape(o, &path, false);
            if shape.frames.is_some() && !steps.iter().any(Action::acts) {
                self.err(
                    &join(&path, "frames"),
                    "序列帧从最后一个动作前开始录，状态里至少要有一个 wait/waitFor 以外的步骤",
                );
            }
            plan.states.push(State { name, steps, shape });
        }
        plan
    }

    fn shape(&mut self, o: &Map<String, Value>, path: &str, initial: bool) -> Shape {
        let mut shape = Shape::default();
        if let Some(t) = o.get("type") {
            let p = join(path, "type");
            shape.shot_type = match t {
                Value::String(s) if s == "viewport" => Some(ShotType::Viewport),
                Value::String(s) if s == "full" => Some(ShotType::Full),
                Value::Object(_) => self
                    .object(t, &p, &["element"])
                    .and_then(|e| match e.get("element") {
                        Some(loc) => self.target(loc, &join(&p, "element")),
                        None => {
                            self.err(&p, "元素截图写成 {\"element\": <定位>}");
                            None
                        }
                    })
                    .map(ShotType::Element),
                _ => {
                    self.err(
                        &p,
                        "type 只能是 \"viewport\"、\"full\" 或 {\"element\": <定位>}",
                    );
                    None
                }
            };
        }
        if let Some(f) = o.get("frames") {
            let p = join(path, "frames");
            let default_from = if initial {
                FramesFrom::Entry
            } else {
                FramesFrom::LastStep
            };
            shape.frames = match f {
                Value::Number(_) => self.frame_count(f, &p).map(|count| Frames {
                    count,
                    from: default_from,
                }),
                Value::Object(_) => self.object(f, &p, &["count", "from"]).and_then(|fo| {
                    let count = match fo.get("count") {
                        Some(c) => self.frame_count(c, &join(&p, "count")),
                        None => {
                            self.err(&join(&p, "count"), "缺少帧数 count");
                            None
                        }
                    };
                    let from = match fo.get("from").map(|v| v.as_str()) {
                        None => Some(default_from),
                        Some(Some("last-step")) if !initial => Some(FramesFrom::LastStep),
                        Some(Some("entry")) if initial => Some(FramesFrom::Entry),
                        Some(Some("entry")) => {
                            self.err(
                                &join(&p, "from"),
                                "from: \"entry\" 只用于 initial；其他状态从最后一个动作开始录",
                            );
                            None
                        }
                        Some(Some("last-step")) => {
                            self.err(
                                &join(&p, "from"),
                                "initial 没有步骤，只能用 from: \"entry\"",
                            );
                            None
                        }
                        Some(_) => {
                            self.err(&join(&p, "from"), "from 只能是 \"last-step\" 或 \"entry\"");
                            None
                        }
                    };
                    Some(Frames {
                        count: count?,
                        from: from?,
                    })
                }),
                _ => {
                    self.err(
                        &p,
                        "frames 应为 2–60 的整数，或 {\"count\": n, \"from\": ...}",
                    );
                    None
                }
            };
        }
        if let Some(m) = o.get("mark") {
            let p = join(path, "mark");
            match m.as_array() {
                Some(list) => {
                    shape.mark = list
                        .iter()
                        .enumerate()
                        .filter_map(|(i, v)| self.target(v, &format!("{p}[{i}]")))
                        .collect()
                }
                None => self.err(&p, "mark 应为定位数组"),
            }
        }
        shape
    }

    fn frame_count(&mut self, v: &Value, path: &str) -> Option<u32> {
        match v.as_u64() {
            Some(n) if (2..=60).contains(&n) => Some(n as u32),
            _ => {
                self.err(path, "帧数应为 2–60 的整数");
                None
            }
        }
    }

    fn target(&mut self, v: &Value, path: &str) -> Option<Target> {
        let o = self.object(v, path, &["css", "text", "role", "name", "nth"])?;
        let string = |key: &str| {
            o.get(key)
                .map(|v| v.as_str().filter(|s| !s.trim().is_empty()))
        };
        let kinds: Vec<&str> = ["css", "text", "role"]
            .into_iter()
            .filter(|k| o.contains_key(*k))
            .collect();
        if kinds.len() != 1 {
            self.err(
                path,
                "定位需要且只能有 css、text、role 之一，如 {\"role\":\"button\",\"name\":\"保存\"}",
            );
            return None;
        }
        let nth = match o.get("nth") {
            None => None,
            Some(n) => match n.as_u64() {
                Some(n) if n >= 1 && n <= u32::MAX as u64 => Some(n as u32),
                _ => {
                    self.err(&join(path, "nth"), "nth 是从 1 开始的整数");
                    return None;
                }
            },
        };
        if o.contains_key("name") && kinds[0] != "role" {
            self.err(&join(path, "name"), "name 只和 role 一起用");
            return None;
        }
        let locator = match kinds[0] {
            "css" => Locator::Css(
                self.required(string("css"), &join(path, "css"))?
                    .to_string(),
            ),
            "text" => Locator::Text(
                self.required(string("text"), &join(path, "text"))?
                    .to_string(),
            ),
            _ => {
                let role = self
                    .required(string("role"), &join(path, "role"))?
                    .to_string();
                let name = match string("name") {
                    None => None,
                    Some(Some(n)) => Some(n.to_string()),
                    Some(None) => {
                        self.err(&join(path, "name"), "name 应为非空字符串");
                        return None;
                    }
                };
                Locator::Role { role, name }
            }
        };
        Some(Target { locator, nth })
    }

    fn required<'s>(&mut self, v: Option<Option<&'s str>>, path: &str) -> Option<&'s str> {
        match v {
            Some(Some(s)) => Some(s),
            _ => {
                self.err(path, "应为非空字符串");
                None
            }
        }
    }

    fn step(&mut self, v: &Value, path: &str) -> Option<Action> {
        let Some(action) = v.get("do").and_then(Value::as_str) else {
            if v.is_object() {
                self.err(
                    &join(path, "do"),
                    format!("缺少动作 do（可用：{}）", ACTIONS.join("、")),
                );
            } else {
                self.err(path, "步骤应为 {\"do\": ..., ...} 对象");
            }
            return None;
        };
        let allowed: &[&str] = match action {
            "click" | "dblclick" | "hover" => &["do", "target"],
            "fill" => &["do", "target", "value", "env"],
            "type" => &["do", "target", "value", "env"],
            "select" => &["do", "target", "value"],
            "press" => &["do", "key"],
            "drag" => &["do", "target", "dx", "dy"],
            "scroll" => &["do", "dx", "dy", "target"],
            "wait" => &["do", "ms"],
            "waitFor" => &["do", "target", "until", "timeoutMs"],
            other => {
                let hint = nearest(other, ACTIONS)
                    .map(|k| format!("，是否想写 {k}？"))
                    .unwrap_or_else(|| format!("（可用：{}）", ACTIONS.join("、")));
                self.err(&join(path, "do"), format!("未知动作 {other}{hint}"));
                return None;
            }
        };
        let o = self.object(v, path, allowed)?;
        let before = self.errors.len();
        let target = |p: &mut Parser, required: bool| -> Option<Target> {
            match o.get("target") {
                Some(t) => p.target(t, &join(path, "target")),
                None => {
                    if required {
                        p.err(&join(path, "target"), format!("{action} 需要 target"));
                    }
                    None
                }
            }
        };
        let number = |p: &mut Parser, key: &str, required: bool| -> Option<f64> {
            match o.get(key) {
                Some(n) => match n.as_f64() {
                    Some(n) if n.is_finite() => Some(n),
                    _ => {
                        p.err(&join(path, key), "应为数字");
                        None
                    }
                },
                None => {
                    if required {
                        p.err(&join(path, key), format!("{action} 需要 {key}"));
                    }
                    None
                }
            }
        };
        let text = |p: &mut Parser| -> Option<Text> {
            match (o.get("value"), o.get("env")) {
                (Some(_), Some(_)) => {
                    p.err(path, "value 和 env 只能给一个");
                    None
                }
                (Some(Value::String(s)), None) => Some(Text::Literal(s.clone())),
                (None, Some(Value::String(s))) if !s.is_empty() => Some(Text::Env(s.clone())),
                (Some(_), None) => {
                    p.err(&join(path, "value"), "value 应为字符串");
                    None
                }
                (None, Some(_)) => {
                    p.err(&join(path, "env"), "env 应为环境变量名");
                    None
                }
                (None, None) => {
                    p.err(
                        path,
                        format!("{action} 需要 value，或用 env 引用环境变量里的凭据"),
                    );
                    None
                }
            }
        };
        let parsed = match action {
            "click" => target(self, true).map(Action::Click),
            "dblclick" => target(self, true).map(Action::Dblclick),
            "hover" => target(self, true).map(Action::Hover),
            "fill" => {
                let t = target(self, true);
                let x = text(self);
                Some(Action::Fill(t?, x?))
            }
            "type" => {
                let t = target(self, false);
                text(self).map(|x| Action::Type(t, x))
            }
            "select" => {
                let t = target(self, true);
                let value = match o.get("value") {
                    Some(Value::String(s)) => Some(s.clone()),
                    _ => {
                        self.err(
                            &join(path, "value"),
                            "select 需要字符串 value（选项的 value 或可见文字）",
                        );
                        None
                    }
                };
                Some(Action::Select(t?, value?))
            }
            "press" => match o.get("key").and_then(Value::as_str) {
                Some(k) => match KeyStroke::parse(k) {
                    Ok(stroke) => Some(Action::Press(stroke, k.to_string())),
                    Err(e) => {
                        self.err(&join(path, "key"), e);
                        None
                    }
                },
                None => {
                    self.err(
                        &join(path, "key"),
                        "press 需要 key，如 \"Enter\"、\"Control+A\"",
                    );
                    None
                }
            },
            "drag" => {
                let t = target(self, true);
                let dx = number(self, "dx", true);
                let dy = number(self, "dy", true);
                Some(Action::Drag(t?, dx?, dy?))
            }
            "scroll" => {
                let dy = number(self, "dy", true);
                let dx = number(self, "dx", false).unwrap_or(0.0);
                let t = target(self, false);
                dy.map(|dy| Action::Scroll { dx, dy, target: t })
            }
            "wait" => match o.get("ms").and_then(Value::as_u64) {
                Some(ms) if ms <= MAX_WAIT_MS => Some(Action::Wait(ms)),
                _ => {
                    self.err(
                        &join(path, "ms"),
                        format!("wait 需要 0–{MAX_WAIT_MS} 的整数 ms；等页面变化请用 waitFor"),
                    );
                    None
                }
            },
            _ => {
                let t = target(self, true);
                let hidden = match o.get("until").map(|u| u.as_str()) {
                    None | Some(Some("visible")) => Some(false),
                    Some(Some("hidden")) => Some(true),
                    Some(_) => {
                        self.err(
                            &join(path, "until"),
                            "until 只能是 \"visible\" 或 \"hidden\"",
                        );
                        None
                    }
                };
                let timeout_ms = match o.get("timeoutMs") {
                    None => Some(None),
                    Some(v) => match v.as_u64() {
                        Some(ms) if (1..=60_000).contains(&ms) => Some(Some(ms)),
                        _ => {
                            self.err(&join(path, "timeoutMs"), "timeoutMs 应为 1–60000 的整数");
                            None
                        }
                    },
                };
                Some(Action::WaitFor {
                    target: t?,
                    hidden: hidden?,
                    timeout_ms: timeout_ms?,
                })
            }
        };
        if self.errors.len() > before {
            return None;
        }
        parsed
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

/// The closest allowed name within two edits, for "did you mean" hints.
pub fn nearest<'a>(word: &str, candidates: &[&'a str]) -> Option<&'a str> {
    candidates
        .iter()
        .map(|c| (distance(&word.to_lowercase(), &c.to_lowercase()), *c))
        .filter(|(d, _)| *d <= 2)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut row = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            row.push((prev[j] + cost).min(prev[j + 1] + 1).min(row[j] + 1));
        }
        prev = row;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn errors(text: &str) -> Vec<String> {
        parse_states(text)
            .unwrap_err()
            .iter()
            .map(|e| e.to_string())
            .collect()
    }

    #[test]
    fn parses_the_documented_example() {
        let plan = parse_states(r##"{
          "initial": { "type": "full", "frames": { "count": 8, "from": "entry" }, "mark": [{ "css": "header nav" }] },
          "states": [{
            "name": "open-filter",
            "steps": [
              { "do": "fill", "target": { "role": "textbox", "name": "用户名" }, "value": "demo" },
              { "do": "fill", "target": { "css": "#password" }, "env": "DEMO_PASSWORD" },
              { "do": "click", "target": { "role": "button", "name": "登录" } },
              { "do": "waitFor", "target": { "text": "任务" } },
              { "do": "click", "target": { "role": "button", "name": "筛选" } }
            ],
            "type": "viewport", "frames": 8, "mark": [{ "role": "button", "name": "应用筛选" }]
          }]
        }"##)
        .unwrap();
        assert_eq!(plan.initial.shot_type, Some(ShotType::Full));
        assert_eq!(
            plan.initial.frames,
            Some(Frames {
                count: 8,
                from: FramesFrom::Entry
            })
        );
        let state = &plan.states[0];
        assert_eq!(state.steps.len(), 5);
        assert_eq!(state.steps[1].env(), Some("DEMO_PASSWORD"));
        assert_eq!(
            state.shape.frames,
            Some(Frames {
                count: 8,
                from: FramesFrom::LastStep
            })
        );
        assert_eq!(
            state.steps[4].target().unwrap().display(),
            "role=button[name=\"筛选\"]"
        );
    }

    #[test]
    fn reports_every_problem_with_its_path() {
        let found = errors(
            r#"{
          "states": [
            { "name": "initial", "steps": [{ "do": "clik", "target": { "css": "a" } }] },
            { "name": "a", "steps": [{ "do": "click", "taget": { "css": "a" } }] },
            { "name": "a", "steps": [] },
            { "name": "b", "steps": [{ "do": "wait", "ms": 20000 }], "frames": 8 }
          ],
          "extra": 1
        }"#,
        );
        let expected = [
            "extra: 未知字段 extra（可用：initial、states）",
            "states[0].name: initial 是初始状态的保留名，不能用作状态名",
            "states[0].steps[0].do: 未知动作 clik，是否想写 click？",
            "states[1].steps[0].taget: 未知字段 taget，是否想写 target？",
            "states[1].steps[0].target: click 需要 target",
            "states[2].name: 状态名 a 重复",
            "states[2].steps: steps 不能为空；只截初始状态时用 initial",
            "states[3].steps[0].ms: wait 需要 0–10000 的整数 ms；等页面变化请用 waitFor",
            "states[3].frames: 序列帧从最后一个动作前开始录，状态里至少要有一个 wait/waitFor 以外的步骤",
        ];
        for message in expected {
            assert!(
                found.iter().any(|f| f == message),
                "missing {message:?} in {found:#?}"
            );
        }
    }

    #[test]
    fn rejects_bad_locators_values_and_frames() {
        let found = errors(
            r#"{
          "initial": { "frames": { "count": 8, "from": "last-step" } },
          "states": [{ "name": "x", "steps": [
            { "do": "click", "target": { "css": "a", "text": "b" } },
            { "do": "fill", "target": { "text": "a" }, "value": "v", "env": "E" },
            { "do": "press", "key": "Hyper+K" },
            { "do": "click", "target": { "text": "a", "nth": 0 } }
          ], "frames": { "count": 1, "from": "entry" } }]
        }"#,
        );
        for message in [
            "initial.frames.from: initial 没有步骤，只能用 from: \"entry\"",
            "states[0].steps[0].target: 定位需要且只能有 css、text、role 之一，如 {\"role\":\"button\",\"name\":\"保存\"}",
            "states[0].steps[1]: value 和 env 只能给一个",
            "states[0].steps[2].key: 无法识别的修饰键 Hyper（可用 Alt、Control、Meta、Shift）",
            "states[0].steps[3].target.nth: nth 是从 1 开始的整数",
            "states[0].frames.count: 帧数应为 2–60 的整数",
            "states[0].frames.from: from: \"entry\" 只用于 initial；其他状态从最后一个动作开始录",
        ] {
            assert!(found.iter().any(|f| f == message), "missing {message:?} in {found:#?}");
        }
    }

    #[test]
    fn invalid_json_is_one_error() {
        let found = errors("{ states: [] }");
        assert_eq!(found.len(), 1);
        assert!(found[0].starts_with("不是合法的 JSON"), "{found:?}");
    }
}
