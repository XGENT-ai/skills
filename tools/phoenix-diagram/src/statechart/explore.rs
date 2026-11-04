//! Breadth-first exploration of the model and the verdict of every rule.
use super::expr::{Env, Overflow, Val};
use super::model::Model;
use super::STATE_LIMIT;
use crate::diagnostic::{Diagnostic, Diagnostics};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct Step {
    pub from: u32,
    pub to: u32,
    pub action: usize,
    pub params: Box<[Val]>,
}

pub struct Exploration {
    /// Model states in discovery order; the first `initial` are the starts.
    pub states: Vec<Box<[Val]>>,
    pub depth: Vec<u32>,
    /// The step that first reached each state; `None` for a start.
    pub via: Vec<Option<u32>>,
    pub initial: usize,
    pub steps: Vec<Step>,
    pub expanded: usize,
    pub complete: bool,
    /// The rules each state breaks: author rule indices, then the deadlock
    /// index `model.rules.len()`.
    pub violations: Vec<Vec<usize>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    Holds,
    Breaks,
    NotChecked,
}

#[derive(Clone, Debug)]
pub struct Bug {
    /// The first state, in discovery order, that breaks the rule.
    pub state: u32,
    /// Step indices from a start to `state`.
    pub path: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub rule: String,
    /// The author's sentence; `None` for a deadlock check without one.
    pub text: Option<String>,
    pub verdict: Verdict,
    pub bug: Option<Bug>,
}

pub const DEADLOCK: &str = "deadlock";

impl Exploration {
    pub fn path_steps(&self, mut state: u32) -> Vec<u32> {
        let mut path = Vec::new();
        while let Some(step) = self.via[state as usize] {
            path.push(step);
            state = self.steps[step as usize].from;
        }
        path.reverse();
        path
    }

    pub fn path_states(&self, state: u32) -> Vec<u32> {
        let path = self.path_steps(state);
        let start = path
            .first()
            .map_or(state, |step| self.steps[*step as usize].from);
        std::iter::once(start)
            .chain(path.iter().map(|step| self.steps[*step as usize].to))
            .collect()
    }

    /// Author rules in declaration order, then deadlock; broken rules sort by
    /// the length of their shortest path, ties keeping that order.
    pub fn outcomes(&self, model: &Model) -> Vec<Outcome> {
        let count = model.rules.len() + usize::from(model.deadlock.is_some());
        let mut first: Vec<Option<u32>> = vec![None; count];
        for (state, broken) in self.violations.iter().enumerate() {
            for rule in broken {
                first[*rule].get_or_insert(state as u32);
            }
        }
        let mut outcomes: Vec<Outcome> = (0..count)
            .map(|index| {
                let bug = first[index].map(|state| Bug {
                    state,
                    path: self.path_steps(state),
                });
                let verdict = match (&bug, self.complete) {
                    (Some(_), _) => Verdict::Breaks,
                    (None, true) => Verdict::Holds,
                    (None, false) => Verdict::NotChecked,
                };
                let (rule, text) = match model.rules.get(index) {
                    Some(rule) => (rule.id.clone(), Some(rule.text.clone())),
                    None => (
                        DEADLOCK.into(),
                        model.deadlock.as_ref().and_then(|d| d.text.clone()),
                    ),
                };
                Outcome {
                    rule,
                    text,
                    verdict,
                    bug,
                }
            })
            .collect();
        outcomes.sort_by_key(|outcome| {
            outcome
                .bug
                .as_ref()
                .map_or(usize::MAX, |bug| bug.path.len())
        });
        outcomes
    }
}

fn product(choices: &[Vec<Val>]) -> Vec<Vec<Val>> {
    let mut rows = vec![vec![]];
    for choice in choices {
        rows = rows
            .into_iter()
            .flat_map(|row| {
                choice.iter().map(move |value| {
                    let mut row = row.clone();
                    row.push(*value);
                    row
                })
            })
            .collect();
    }
    rows
}

/// One enabled step before it is numbered: action, parameters, target values.
type Candidate = (usize, Box<[Val]>, Box<[Val]>);

struct Explorer<'a> {
    model: &'a Model,
    exploration: Exploration,
    index: HashMap<Box<[Val]>, u32>,
}

impl Explorer<'_> {
    fn path_names(&self, state: u32) -> Vec<String> {
        self.exploration
            .path_states(state)
            .into_iter()
            .map(|state| {
                self.model
                    .state_name(&self.exploration.states[state as usize])
            })
            .collect()
    }

    fn failure(
        &self,
        code: &str,
        path: &str,
        message: String,
        state: u32,
        mut evidence: Value,
    ) -> Diagnostics {
        evidence["state"] = json!(self
            .model
            .state_name(&self.exploration.states[state as usize]));
        evidence["path"] = json!(self.path_names(state));
        let mut diagnostic = Diagnostic::error(Some("statechart"), code, path, message);
        diagnostic.evidence = evidence;
        vec![diagnostic]
    }

    fn overflow(&self, path: &str, state: u32, evidence: Value) -> Diagnostics {
        self.failure(
            "statechart/arithmetic-overflow",
            path,
            "Integer arithmetic left the signed 64-bit range".into(),
            state,
            evidence,
        )
    }

    /// Record a newly discovered state and the rules it breaks.
    fn discover(
        &mut self,
        values: Box<[Val]>,
        depth: u32,
        via: Option<u32>,
    ) -> Result<u32, Diagnostics> {
        let state = self.exploration.states.len() as u32;
        self.index.insert(values.clone(), state);
        self.exploration.states.push(values);
        self.exploration.depth.push(depth);
        self.exploration.via.push(via);
        let mut broken = Vec::new();
        let values = &self.exploration.states[state as usize];
        for (index, rule) in self.model.rules.iter().enumerate() {
            let env = Env {
                state: values,
                next: &[],
                params: &[],
            };
            match rule.holds.eval(&env) {
                Ok(Val::Bool(true)) => {}
                Ok(_) => broken.push(index),
                Err(Overflow) => {
                    return Err(self.overflow(
                        &format!("/model/rules/{index}/holds"),
                        state,
                        json!({"rule":rule.id}),
                    ))
                }
            }
        }
        self.exploration.violations.push(broken);
        Ok(state)
    }

    /// Every enabled step of `state`: actions by id, parameter choices in
    /// declaration order, then the target's default name.
    fn successors(&self, state: u32, order: &[usize]) -> Result<Vec<Candidate>, Diagnostics> {
        let values = &self.exploration.states[state as usize];
        let mut candidates = Vec::new();
        for &action_index in order {
            let action = &self.model.actions[action_index];
            let choices: Vec<Vec<Val>> = action
                .params
                .iter()
                .map(|param| match &param.domain {
                    super::expr::Domain::Values(values) => values.clone(),
                    super::expr::Domain::Range(..) => unreachable!("parameters list their values"),
                })
                .collect();
            for params in product(&choices) {
                let env = Env {
                    state: values,
                    next: &[],
                    params: &params,
                };
                let path = format!("/model/actions/{action_index}");
                let context = || {
                    json!({"action":action.id,"params":action.params.iter().zip(&params)
                        .map(|(slot, value)| (slot.name.clone(), json!(self.model.strings.literal(*value))))
                        .collect::<serde_json::Map<_, _>>()})
                };
                if let Some(when) = &action.when {
                    match when.eval(&env) {
                        Ok(Val::Bool(true)) => {}
                        Ok(_) => continue,
                        Err(Overflow) => {
                            return Err(self.overflow(&format!("{path}/when"), state, context()))
                        }
                    }
                }
                let mut next = values.clone();
                for (variable, expr) in &action.set {
                    let slot = &self.model.variables[*variable];
                    let at = format!("{path}/set/{}", slot.name);
                    let value = expr
                        .eval(&env)
                        .map_err(|_| self.overflow(&at, state, context()))?;
                    if !slot.domain.contains(value) {
                        let mut evidence = context();
                        evidence["variable"] = json!(slot.name);
                        evidence["value"] = json!(self.model.strings.literal(value));
                        return Err(self.failure(
                            "statechart/value-out-of-domain",
                            &at,
                            format!(
                                "{} would become {}, outside its domain",
                                slot.name,
                                self.model.strings.literal(value)
                            ),
                            state,
                            evidence,
                        ));
                    }
                    next[*variable] = value;
                }
                candidates.push((action_index, params.into_boxed_slice(), next));
            }
        }
        let rank: HashMap<usize, usize> = order
            .iter()
            .enumerate()
            .map(|(rank, action)| (*action, rank))
            .collect();
        candidates
            .sort_by_cached_key(|(action, _, next)| (rank[action], self.model.state_name(next)));
        Ok(candidates)
    }
}

pub fn explore(model: &Model) -> Result<Exploration, Diagnostics> {
    let mut explorer = Explorer {
        model,
        exploration: Exploration {
            states: vec![],
            depth: vec![],
            via: vec![],
            initial: 0,
            steps: vec![],
            expanded: 0,
            complete: true,
            violations: vec![],
        },
        index: HashMap::new(),
    };
    let mut starts: Vec<Box<[Val]>> = product(&model.init)
        .into_iter()
        .map(Vec::into_boxed_slice)
        .collect();
    starts.sort_by_cached_key(|state| model.state_name(state));
    starts.dedup();
    for start in starts {
        explorer.discover(start, 0, None)?;
    }
    explorer.exploration.initial = explorer.exploration.states.len();
    let mut order: Vec<usize> = (0..model.actions.len()).collect();
    order.sort_by(|a, b| {
        model.actions[*a]
            .id
            .as_bytes()
            .cmp(model.actions[*b].id.as_bytes())
    });
    let deadlock = model.rules.len();
    let mut state = 0;
    'explore: while state < explorer.exploration.states.len() {
        let id = state as u32;
        let candidates = explorer.successors(id, &order)?;
        let depth = explorer.exploration.depth[state] + 1;
        let enabled = !candidates.is_empty();
        for (action, params, next) in candidates {
            let step = explorer.exploration.steps.len() as u32;
            let to = match explorer.index.get(&next) {
                Some(to) => *to,
                None if explorer.exploration.states.len() >= STATE_LIMIT => {
                    explorer.exploration.complete = false;
                    break 'explore;
                }
                None => explorer.discover(next, depth, Some(step))?,
            };
            explorer.exploration.steps.push(Step {
                from: id,
                to,
                action,
                params,
            });
        }
        explorer.exploration.expanded += 1;
        if let Some(rule) = &model.deadlock {
            let values = &explorer.exploration.states[state];
            let ends = match &rule.ends {
                Some(ends) => match ends.eval(&Env {
                    state: values,
                    next: &[],
                    params: &[],
                }) {
                    Ok(value) => value == Val::Bool(true),
                    Err(Overflow) => {
                        return Err(explorer.overflow("/model/deadlock/ends", id, json!({})))
                    }
                },
                None => false,
            };
            if !enabled && !ends {
                explorer.exploration.violations[state].push(deadlock);
            }
        }
        state += 1;
    }
    Ok(explorer.exploration)
}
