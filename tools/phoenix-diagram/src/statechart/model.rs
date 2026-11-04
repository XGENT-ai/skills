//! The typed model and view behind a statechart document.
use super::expr::{literal, Compiler, Domain, Expr, Scope, Slot, Strings, Ty, Val};
use super::Problems;
use crate::diagnostic::Diagnostics;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};

pub struct Action {
    pub id: String,
    pub params: Vec<Slot>,
    pub when: Option<Expr>,
    pub set: Vec<(usize, Expr)>,
}

pub struct Rule {
    pub id: String,
    pub text: String,
    pub holds: Expr,
}

pub struct Deadlock {
    pub ends: Option<Expr>,
    pub text: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Part {
    Text(String),
    Var(usize),
    Param(usize),
}

#[derive(Clone, Debug)]
pub struct Template(pub Vec<Part>);

impl Template {
    pub fn render(&self, strings: &Strings, state: &[Val], params: &[Val]) -> String {
        self.0
            .iter()
            .map(|part| match part {
                Part::Text(text) => text.clone(),
                Part::Var(index) => strings.display(state[*index]),
                Part::Param(index) => strings.display(params[*index]),
            })
            .collect()
    }
}

pub struct LabelRule {
    pub when: Option<Expr>,
    /// One element per path level; a longer path nests the state in parents.
    pub label: Vec<Template>,
}

pub struct Region {
    pub name: Option<String>,
    /// `None` labels every model state with its default name.
    pub rules: Option<Vec<LabelRule>>,
}

pub struct StepRule {
    pub action: usize,
    pub when: Option<Expr>,
    /// `None` hides the step.
    pub label: Option<Template>,
}

pub struct GroupRule {
    pub when: Option<Expr>,
    pub group: String,
}

pub struct View {
    pub regions: Vec<Region>,
    pub steps: Vec<StepRule>,
    pub groups: Vec<GroupRule>,
    pub colors: BTreeMap<String, String>,
}

pub struct Model {
    pub strings: Strings,
    pub variables: Vec<Slot>,
    /// The alternative initial values of each variable, in declaration order.
    pub init: Vec<Vec<Val>>,
    pub actions: Vec<Action>,
    pub rules: Vec<Rule>,
    /// `None` when deadlock checking is switched off.
    pub deadlock: Option<Deadlock>,
    pub view: View,
}

fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

fn unique(problems: &mut Problems, items: &[Value], field: &str, path: &str) {
    let mut seen = HashMap::new();
    for (index, item) in items.iter().enumerate() {
        let Some(id) = item[field].as_str() else {
            continue;
        };
        if let Some(first) = seen.insert(id, index) {
            problems.push(
                "input/duplicate-id",
                &format!("{path}/{index}/{field}"),
                format!("Duplicate {field} {id}"),
                json!({"id":id,"first":format!("{path}/{first}/{field}")}),
            );
        }
    }
}

impl Model {
    pub fn parse(document: &Value) -> Result<Model, Diagnostics> {
        let mut problems = Problems(Vec::new());
        let mut strings = Strings::default();
        let model = &document["model"];
        let raw_variables = list(&model["variables"]);
        unique(&mut problems, raw_variables, "id", "/model/variables");
        let variables: Vec<Slot> = raw_variables
            .iter()
            .enumerate()
            .filter_map(|(index, raw)| {
                let domain = if let Some(values) = raw["values"].as_array() {
                    let values: Vec<_> = values
                        .iter()
                        .filter_map(|value| literal(value, &mut strings))
                        .collect();
                    (!values.is_empty()).then_some(Domain::Values(values))?
                } else {
                    let (lo, hi) = (raw["range"][0].as_i64()?, raw["range"][1].as_i64()?);
                    if lo > hi {
                        problems.push(
                            "input/range",
                            &format!("/model/variables/{index}/range"),
                            "range must list its lowest value first",
                            json!({"range":[lo,hi]}),
                        );
                        return None;
                    }
                    Domain::Range(lo, hi)
                };
                Some(Slot {
                    name: raw["id"].as_str()?.into(),
                    domain,
                })
            })
            .collect();
        if variables.len() != raw_variables.len() {
            return Err(problems.0);
        }

        let init = parse_init(&model["init"], &variables, &mut strings, &mut problems);

        let raw_actions = list(&model["actions"]);
        unique(&mut problems, raw_actions, "id", "/model/actions");
        let mut actions = Vec::new();
        for (index, raw) in raw_actions.iter().enumerate() {
            let path = format!("/model/actions/{index}");
            let mut params = Vec::new();
            for (name, values) in raw["pick"].as_object().into_iter().flatten() {
                let mut domain = Vec::new();
                for value in list(values)
                    .iter()
                    .filter_map(|value| literal(value, &mut strings))
                {
                    if !domain.contains(&value) {
                        domain.push(value);
                    }
                }
                params.push(Slot {
                    name: name.clone(),
                    domain: Domain::Values(domain),
                });
            }
            let mut compiler = Compiler {
                scope: Scope {
                    variables: &variables,
                    params: Some(&params),
                    next: false,
                },
                strings: &mut strings,
                problems: &mut problems,
            };
            let when = raw
                .get("when")
                .and_then(|value| compiler.expect(value, &format!("{path}/when"), Ty::Bool));
            let mut set = Vec::new();
            for (name, value) in raw["set"].as_object().into_iter().flatten() {
                let at = format!("{path}/set/{}", name.replace('~', "~0").replace('/', "~1"));
                let Some(variable) = variables.iter().position(|slot| &slot.name == name) else {
                    compiler.problems.push(
                        "statechart/unknown-variable",
                        &at,
                        format!("Unknown variable {name}"),
                        json!({"name":name}),
                    );
                    continue;
                };
                let domain = &variables[variable].domain;
                if let Some(expr) = compiler.expect(value, &at, domain.ty()) {
                    if let Expr::Lit(value) = expr {
                        if !domain.contains(value) {
                            let shown = compiler.strings.literal(value);
                            compiler.problems.push(
                                "statechart/value-not-in-domain",
                                &at,
                                format!("{shown} is not a value of {name}"),
                                json!({"name":name,"value":shown}),
                            );
                        }
                    }
                    set.push((variable, expr));
                }
            }
            actions.push(Action {
                id: raw["id"].as_str().unwrap_or_default().into(),
                params,
                when,
                set,
            });
        }

        let raw_rules = list(&model["rules"]);
        unique(&mut problems, raw_rules, "id", "/model/rules");
        let mut rules = Vec::new();
        for (index, raw) in raw_rules.iter().enumerate() {
            let path = format!("/model/rules/{index}");
            let id = raw["id"].as_str().unwrap_or_default();
            if id == "deadlock" {
                problems.push(
                    "statechart/reserved-rule-id",
                    &format!("{path}/id"),
                    "The rule id deadlock is reserved for the built-in deadlock check",
                    Value::Null,
                );
            }
            let mut compiler = Compiler {
                scope: Scope {
                    variables: &variables,
                    params: None,
                    next: false,
                },
                strings: &mut strings,
                problems: &mut problems,
            };
            if let Some(holds) = compiler.expect(&raw["holds"], &format!("{path}/holds"), Ty::Bool)
            {
                rules.push(Rule {
                    id: id.into(),
                    text: raw["text"].as_str().unwrap_or_default().into(),
                    holds,
                });
            }
        }

        let raw_deadlock = &model["deadlock"];
        let deadlock = (raw_deadlock["check"] != false).then(|| {
            let mut compiler = Compiler {
                scope: Scope {
                    variables: &variables,
                    params: None,
                    next: false,
                },
                strings: &mut strings,
                problems: &mut problems,
            };
            Deadlock {
                ends: raw_deadlock
                    .get("ends")
                    .and_then(|value| compiler.expect(value, "/model/deadlock/ends", Ty::Bool)),
                text: raw_deadlock["text"].as_str().map(str::to_owned),
            }
        });

        let view = parse_view(
            &document["view"],
            &variables,
            &actions,
            &mut strings,
            &mut problems,
        );
        if problems.0.is_empty() {
            Ok(Model {
                strings,
                variables,
                init,
                actions,
                rules,
                deadlock,
                view,
            })
        } else {
            Err(problems.0)
        }
    }

    /// One variable shows its value; several show `id=value` in declaration order.
    pub fn state_name(&self, state: &[Val]) -> String {
        if let [value] = state {
            return self.strings.display(*value);
        }
        self.variables
            .iter()
            .zip(state)
            .map(|(slot, value)| format!("{}={}", slot.name, self.strings.literal(*value)))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn parse_init(
    raw: &Value,
    variables: &[Slot],
    strings: &mut Strings,
    problems: &mut Problems,
) -> Vec<Vec<Val>> {
    let empty = Map::new();
    let object = raw.as_object().unwrap_or(&empty);
    for key in object
        .keys()
        .filter(|key| !variables.iter().any(|slot| &slot.name == *key))
    {
        problems.push(
            "statechart/unknown-variable",
            &format!("/model/init/{}", key.replace('~', "~0").replace('/', "~1")),
            format!("Unknown variable {key}"),
            json!({"name":key}),
        );
    }
    let mut init = Vec::new();
    for slot in variables {
        let at = format!("/model/init/{}", slot.name);
        let Some(value) = object.get(&slot.name) else {
            problems.push(
                "statechart/init-incomplete",
                "/model/init",
                format!("init must give variable {} its initial value", slot.name),
                json!({"variable":slot.name}),
            );
            init.push(vec![]);
            continue;
        };
        let candidates: Vec<(String, &Value)> = match value.as_array() {
            Some(items) => items
                .iter()
                .enumerate()
                .map(|(index, item)| (format!("{at}/{index}"), item))
                .collect(),
            None => vec![(at.clone(), value)],
        };
        let mut values = Vec::new();
        for (path, item) in candidates {
            match literal(item, strings).filter(|value| slot.domain.contains(*value)) {
                Some(value) => values.push(value),
                None => problems.push(
                    "statechart/value-not-in-domain",
                    &path,
                    format!("{item} is not a value of {}", slot.name),
                    json!({"name":slot.name,"value":item}),
                ),
            }
        }
        init.push(values);
    }
    init
}

fn template(
    raw: &str,
    path: &str,
    variables: &[Slot],
    params: &[Slot],
    problems: &mut Problems,
) -> Option<Template> {
    let mut parts = Vec::new();
    let mut rest = raw;
    let mut ok = true;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|close| open + close) else {
            break;
        };
        if open > 0 {
            parts.push(Part::Text(rest[..open].into()));
        }
        let name = &rest[open + 1..close];
        if let Some(index) = params.iter().position(|slot| slot.name == name) {
            parts.push(Part::Param(index));
        } else if let Some(index) = variables.iter().position(|slot| slot.name == name) {
            parts.push(Part::Var(index));
        } else {
            problems.push(
                "statechart/unknown-placeholder",
                path,
                format!(
                    "{{{name}}} names no variable{}",
                    if params.is_empty() {
                        ""
                    } else {
                        " or parameter"
                    }
                ),
                json!({"placeholder":name}),
            );
            ok = false;
        }
        rest = &rest[close + 1..];
    }
    if !rest.is_empty() {
        parts.push(Part::Text(rest.into()));
    }
    ok.then_some(Template(parts))
}

fn label_rules(
    raw: &Value,
    path: &str,
    variables: &[Slot],
    strings: &mut Strings,
    problems: &mut Problems,
) -> Vec<LabelRule> {
    let mut rules = Vec::new();
    for (index, rule) in list(raw).iter().enumerate() {
        let at = format!("{path}/{index}");
        let mut compiler = Compiler {
            scope: Scope {
                variables,
                params: None,
                next: false,
            },
            strings: &mut *strings,
            problems: &mut *problems,
        };
        let when = rule
            .get("when")
            .and_then(|value| compiler.expect(value, &format!("{at}/when"), Ty::Bool));
        let labels: Vec<(String, &str)> = match &rule["label"] {
            Value::String(text) => vec![(format!("{at}/label"), text.as_str())],
            Value::Array(items) => items
                .iter()
                .enumerate()
                .filter_map(|(level, item)| Some((format!("{at}/label/{level}"), item.as_str()?)))
                .collect(),
            _ => vec![],
        };
        let label: Option<Vec<_>> = labels
            .into_iter()
            .map(|(path, text)| template(text, &path, variables, &[], problems))
            .collect::<Vec<_>>()
            .into_iter()
            .collect();
        if let Some(label) = label {
            rules.push(LabelRule { when, label });
        }
    }
    rules
}

fn parse_view(
    raw: &Value,
    variables: &[Slot],
    actions: &[Action],
    strings: &mut Strings,
    problems: &mut Problems,
) -> View {
    let regions = if let Some(items) = raw["regions"].as_array() {
        unique(problems, items, "name", "/view/regions");
        items
            .iter()
            .enumerate()
            .map(|(index, region)| Region {
                name: region["name"].as_str().map(str::to_owned),
                rules: Some(label_rules(
                    &region["state"],
                    &format!("/view/regions/{index}/state"),
                    variables,
                    strings,
                    problems,
                )),
            })
            .collect()
    } else {
        vec![Region {
            name: None,
            rules: raw
                .get("state")
                .map(|state| label_rules(state, "/view/state", variables, strings, problems)),
        }]
    };

    let mut steps = Vec::new();
    for (index, rule) in list(&raw["steps"]).iter().enumerate() {
        let at = format!("/view/steps/{index}");
        let name = rule["action"].as_str().unwrap_or_default();
        let Some(action) = actions.iter().position(|action| action.id == name) else {
            problems.push(
                "input/unknown-reference",
                &format!("{at}/action"),
                format!("view step names an unknown action {name}"),
                json!({"action":name}),
            );
            continue;
        };
        let params = &actions[action].params;
        let mut compiler = Compiler {
            scope: Scope {
                variables,
                params: Some(params),
                next: true,
            },
            strings: &mut *strings,
            problems: &mut *problems,
        };
        let when = rule
            .get("when")
            .and_then(|value| compiler.expect(value, &format!("{at}/when"), Ty::Bool));
        let label = match rule["label"].as_str() {
            Some(text) => match template(text, &format!("{at}/label"), variables, params, problems)
            {
                Some(label) => Some(label),
                None => continue,
            },
            None => None,
        };
        steps.push(StepRule {
            action,
            when,
            label,
        });
    }

    let colors: BTreeMap<String, String> = raw["colors"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(group, color)| Some((group.clone(), color.as_str()?.to_owned())))
        .collect();
    let mut groups = Vec::new();
    for (index, rule) in list(&raw["groups"]).iter().enumerate() {
        let at = format!("/view/groups/{index}");
        let group = rule["group"].as_str().unwrap_or_default();
        if !colors.contains_key(group) {
            problems.push(
                "statechart/group-color",
                &format!("{at}/group"),
                format!("Group {group} needs a color in view.colors"),
                json!({"group":group}),
            );
        }
        let mut compiler = Compiler {
            scope: Scope {
                variables,
                params: None,
                next: false,
            },
            strings: &mut *strings,
            problems: &mut *problems,
        };
        let when = rule
            .get("when")
            .and_then(|value| compiler.expect(value, &format!("{at}/when"), Ty::Bool));
        groups.push(GroupRule {
            when,
            group: group.into(),
        });
    }
    View {
        regions,
        steps,
        groups,
        colors,
    }
}
