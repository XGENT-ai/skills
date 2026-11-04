//! Projecting explored model states onto the drawn chart: labels, merged
//! states, arrows, parent lifting, marks and the story of every bug.
use super::explore::{Exploration, Outcome};
use super::expr::{Env, Expr, Overflow, Val};
use super::model::{Model, Region as ViewRegion};
use super::regions::{self, Condition};
use crate::diagnostic::{Diagnostic, Diagnostics};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

/// One region's condition on the other regions, in the words of their labels.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Clause {
    pub region: String,
    pub negated: bool,
    /// Leaf paths joined by ` / `, sorted.
    pub values: Vec<String>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum When {
    All(Vec<Clause>),
    Any(Vec<Clause>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mark {
    Breaks,
    MayBreak,
}

#[derive(Clone, Debug)]
pub struct State {
    pub path: Vec<String>,
    pub start: bool,
    /// The shallowest BFS depth among the merged model states.
    pub depth: u32,
    /// Model states merged into this chart state.
    pub members: usize,
    pub mark: Option<Mark>,
    /// Rules every member breaks, then rules only some members break; only
    /// for a marked state.
    pub breaks: Vec<String>,
    pub may_break: Vec<String>,
    pub bug_path: bool,
    pub color: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Child {
    State(usize),
    Parent(usize),
}

#[derive(Clone, Debug)]
pub struct Parent {
    pub path: Vec<String>,
    pub children: Vec<Child>,
}

#[derive(Clone, Debug)]
pub struct Arrow {
    /// A leaf, or a parent after lifting.
    pub from: Vec<String>,
    pub to: Vec<String>,
    /// Merged labels in order of appearance.
    pub labels: Vec<String>,
    pub when: Option<When>,
    pub bug_path: bool,
}

#[derive(Clone, Debug)]
pub struct Region {
    pub name: Option<String>,
    /// Leaves in draw order.
    pub states: Vec<State>,
    pub parents: Vec<Parent>,
    /// The region's own children, in order of appearance.
    pub top: Vec<Child>,
    pub arrows: Vec<Arrow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Note {
    OneAtATime {
        step: String,
        regions: Vec<String>,
    },
    Together {
        step: String,
        sets: Vec<Vec<String>>,
    },
}

/// One chart state per region: the region's name and the leaf path.
pub type Combo = Vec<(String, Vec<String>)>;

/// Why a regions chart would misstate the model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Fallback {
    Starts(Combo),
    Condition {
        region: String,
        label: String,
        from: Vec<String>,
    },
    MoveExtra {
        from: Combo,
        label: String,
        to: Combo,
    },
    MoveMissing {
        from: Combo,
        label: String,
        to: Combo,
    },
}

/// The chart state of a model state in each authored region; `None` names
/// the single unnamed region.
pub type Shape = Vec<(Option<String>, Vec<String>)>;

#[derive(Clone, Debug)]
pub struct Story {
    pub rule: String,
    pub start: Shape,
    pub steps: Vec<(String, Shape)>,
}

#[derive(Debug)]
pub struct Chart {
    pub regions: Vec<Region>,
    /// The authored region names; kept when the chart falls back to parents.
    pub region_names: Vec<String>,
    pub fallback: Option<Fallback>,
    pub notes: Vec<Note>,
    /// Drawn steps that stay in one chart state and are not drawn.
    pub inside: usize,
    /// Steps the view hides.
    pub hidden: usize,
    pub stories: Vec<Story>,
}

impl Chart {
    pub fn state_count(&self) -> usize {
        self.regions.iter().map(|region| region.states.len()).sum()
    }
    pub fn arrow_count(&self) -> usize {
        self.regions.iter().map(|region| region.arrows.len()).sum()
    }
}

/// The leaf of every model state in one region.
pub(super) struct Projection {
    pub name: Option<String>,
    /// Leaf paths in draw order.
    pub leaves: Vec<Vec<String>>,
    pub leaf_of: Vec<u32>,
    /// The model states of each leaf, in draw order.
    pub members: Vec<Vec<u32>>,
}

impl Projection {
    fn new(name: Option<String>, states: usize) -> Self {
        Projection {
            name,
            leaves: Vec::new(),
            leaf_of: vec![0; states],
            members: Vec::new(),
        }
    }

    fn place(&mut self, index: &mut HashMap<Vec<String>, u32>, state: u32, path: Vec<String>) {
        let leaf = *index.entry(path).or_insert_with_key(|path| {
            self.leaves.push(path.clone());
            self.members.push(Vec::new());
            (self.leaves.len() - 1) as u32
        });
        self.leaf_of[state as usize] = leaf;
        self.members[leaf as usize].push(state);
    }

    pub fn region(&self) -> String {
        self.name.clone().unwrap_or_default()
    }
}

/// Everything the projection reads, computed once per model.
pub(super) struct Walk<'a> {
    pub exploration: &'a Exploration,
    /// Model states by (depth, default name).
    pub order: Vec<u32>,
    /// `None` for a hidden step.
    pub labels: Vec<Option<String>>,
    /// Shown steps by (from rank, to rank, action id).
    pub sorted: Vec<u32>,
    pub bug_steps: HashSet<u32>,
}

/// A candidate arrow of one region before lifting and merging.
pub(super) struct Move {
    pub from: u32,
    pub to: u32,
    pub label: String,
    pub steps: Vec<u32>,
    pub condition: Condition,
    pub when: Option<When>,
}

fn truth(expr: Option<&Expr>, env: &Env<'_>) -> Result<bool, Overflow> {
    expr.map_or(Ok(true), |expr| Ok(expr.eval(env)? == Val::Bool(true)))
}

fn path_names(model: &Model, exploration: &Exploration, state: u32) -> Vec<String> {
    exploration
        .path_states(state)
        .into_iter()
        .map(|state| model.state_name(&exploration.states[state as usize]))
        .collect()
}

fn failure(code: &str, path: &str, message: String, evidence: Value) -> Diagnostics {
    let mut diagnostic = Diagnostic::error(Some("statechart"), code, path, message);
    diagnostic.evidence = evidence;
    vec![diagnostic]
}

fn overflow(walk: (&Model, &Exploration), path: &str, state: u32) -> Diagnostics {
    let (model, exploration) = walk;
    failure(
        "statechart/arithmetic-overflow",
        path,
        "Integer arithmetic left the signed 64-bit range".into(),
        json!({"state":model.state_name(&exploration.states[state as usize]),"path":path_names(model, exploration, state)}),
    )
}

fn label_region(
    model: &Model,
    exploration: &Exploration,
    order: &[u32],
    at: usize,
    region: &ViewRegion,
) -> Result<Projection, Diagnostics> {
    let pointer = if region.name.is_some() {
        format!("/view/regions/{at}/state")
    } else {
        "/view/state".into()
    };
    let mut projection = Projection::new(region.name.clone(), exploration.states.len());
    let mut index = HashMap::new();
    for &state in order {
        let values = &exploration.states[state as usize];
        let path = match &region.rules {
            None => vec![model.state_name(values)],
            Some(rules) => {
                let env = Env {
                    state: values,
                    next: &[],
                    params: &[],
                };
                let mut matched = None;
                for (k, rule) in rules.iter().enumerate() {
                    let when = truth(rule.when.as_ref(), &env).map_err(|_| {
                        overflow((model, exploration), &format!("{pointer}/{k}/when"), state)
                    })?;
                    if when {
                        matched = Some((k, rule));
                        break;
                    }
                }
                let Some((k, rule)) = matched else {
                    let name = model.state_name(values);
                    return Err(failure(
                        "statechart/unlabeled-state",
                        &pointer,
                        format!("No label rule matches the model state {name}; add a rule without when as the last one"),
                        json!({"state":name,"path":path_names(model, exploration, state)}),
                    ));
                };
                let mut path = Vec::new();
                for (level, template) in rule.label.iter().enumerate() {
                    let text = template.render(&model.strings, values, &[]);
                    if text.is_empty() {
                        let at = if rule.label.len() > 1 {
                            format!("{pointer}/{k}/label/{level}")
                        } else {
                            format!("{pointer}/{k}/label")
                        };
                        return Err(failure(
                            "statechart/empty-label",
                            &at,
                            "The label is empty for this model state".into(),
                            json!({"state":model.state_name(values)}),
                        ));
                    }
                    path.push(text);
                }
                path
            }
        };
        projection.place(&mut index, state, path);
    }
    Ok(projection)
}

enum StepLabel {
    Shown(String),
    Hidden(usize),
}

fn label_steps(model: &Model, exploration: &Exploration) -> Result<Vec<StepLabel>, Diagnostics> {
    let mut labels = Vec::with_capacity(exploration.steps.len());
    for step in &exploration.steps {
        let env = Env {
            state: &exploration.states[step.from as usize],
            next: &exploration.states[step.to as usize],
            params: &step.params,
        };
        let mut label = None;
        for (k, rule) in model.view.steps.iter().enumerate() {
            if rule.action != step.action {
                continue;
            }
            let when = truth(rule.when.as_ref(), &env).map_err(|_| {
                overflow(
                    (model, exploration),
                    &format!("/view/steps/{k}/when"),
                    step.from,
                )
            })?;
            if !when {
                continue;
            }
            label = Some(match &rule.label {
                None => StepLabel::Hidden(k),
                Some(template) => {
                    let text = template.render(&model.strings, env.state, env.params);
                    if text.is_empty() {
                        return Err(failure(
                            "statechart/empty-label",
                            &format!("/view/steps/{k}/label"),
                            "The step label is empty for this step".into(),
                            json!({"action":model.actions[step.action].id,"state":model.state_name(env.state)}),
                        ));
                    }
                    StepLabel::Shown(text)
                }
            });
            break;
        }
        labels
            .push(label.unwrap_or_else(|| StepLabel::Shown(model.actions[step.action].id.clone())));
    }
    Ok(labels)
}

/// Candidate arrows of region `r`; a step that stays in one leaf becomes a
/// self-loop only on a bug path in a single-region chart.
fn moves(walk: &Walk<'_>, projections: &[Projection], r: usize) -> Vec<Move> {
    let projection = &projections[r];
    let single = projections.len() == 1;
    let mut moves: Vec<Move> = Vec::new();
    let mut index: HashMap<(u32, u32, &str), usize> = HashMap::new();
    for &j in &walk.sorted {
        let step = &walk.exploration.steps[j as usize];
        let (from, to) = (
            projection.leaf_of[step.from as usize],
            projection.leaf_of[step.to as usize],
        );
        if from == to && !(single && walk.bug_steps.contains(&j)) {
            continue;
        }
        let label = walk.labels[j as usize]
            .as_deref()
            .expect("sorted steps are shown");
        let at = *index.entry((from, to, label)).or_insert_with(|| {
            moves.push(Move {
                from,
                to,
                label: label.into(),
                steps: Vec::new(),
                condition: Condition::All(Vec::new()),
                when: None,
            });
            moves.len() - 1
        });
        moves[at].steps.push(j);
    }
    if !single {
        let mut seen = HashMap::new();
        for item in &mut moves {
            let (condition, when) = regions::condition(walk, projections, r, item, &mut seen);
            item.condition = condition;
            item.when = when;
        }
    }
    for item in &mut moves {
        // The author's bracket already states the condition.
        if item.label.ends_with(']') {
            item.when = None;
        }
    }
    moves
}

fn inside(walk: &Walk<'_>, projections: &[Projection]) -> usize {
    let single = projections.len() == 1;
    walk.sorted
        .iter()
        .filter(|j| {
            let step = &walk.exploration.steps[**j as usize];
            projections
                .iter()
                .all(|p| p.leaf_of[step.from as usize] == p.leaf_of[step.to as usize])
                && !(single && walk.bug_steps.contains(j))
        })
        .count()
}

/// The parent states of one region and the region's own children.
fn parents(projection: &Projection, at: usize) -> Result<(Vec<Parent>, Vec<Child>), Diagnostics> {
    let mut parents: Vec<Parent> = Vec::new();
    let mut index: HashMap<&[String], usize> = HashMap::new();
    let mut top = Vec::new();
    let mut attached: HashSet<(Option<usize>, Child)> = HashSet::new();
    let mut attach =
        |parents: &mut Vec<Parent>, top: &mut Vec<Child>, owner: Option<usize>, child: Child| {
            if attached.insert((owner, child)) {
                match owner {
                    Some(owner) => parents[owner].children.push(child),
                    None => top.push(child),
                }
            }
        };
    for (leaf, path) in projection.leaves.iter().enumerate() {
        let mut owner = None;
        for depth in 1..path.len() {
            let prefix = &path[..depth];
            let parent = match index.get(prefix) {
                Some(parent) => *parent,
                None => {
                    parents.push(Parent {
                        path: prefix.to_vec(),
                        children: Vec::new(),
                    });
                    index.insert(prefix, parents.len() - 1);
                    parents.len() - 1
                }
            };
            attach(&mut parents, &mut top, owner, Child::Parent(parent));
            owner = Some(parent);
        }
        attach(&mut parents, &mut top, owner, Child::State(leaf));
    }
    if let Some(path) = projection
        .leaves
        .iter()
        .find(|path| index.contains_key(path.as_slice()))
    {
        let pointer = if projection.name.is_some() {
            format!("/view/regions/{at}/state")
        } else {
            "/view/state".into()
        };
        return Err(failure(
            "statechart/leaf-is-parent",
            &pointer,
            format!(
                "{} is both a state and a parent state; give that state a child name of its own",
                path.join(" / ")
            ),
            json!({"path":path,"region":projection.name}),
        ));
    }
    Ok((parents, top))
}

/// Replace every group of arrows that leaves a parent from each of its leaves
/// with one arrow from the parent, outer parents first.
fn lift(leaves: &[Vec<String>], arrows: Vec<Arrow>) -> Vec<Arrow> {
    let mut prefixes: Vec<(Vec<String>, usize)> = Vec::new();
    let mut count: HashMap<Vec<String>, usize> = HashMap::new();
    for leaf in leaves {
        for depth in 1..leaf.len() {
            let prefix = leaf[..depth].to_vec();
            let entry = count.entry(prefix.clone()).or_insert(0);
            if *entry == 0 {
                prefixes.push((prefix, depth));
            }
            *entry += 1;
        }
    }
    if prefixes.is_empty() {
        return arrows;
    }
    let mut leaving: HashMap<&[String], Vec<usize>> = HashMap::new();
    for (k, arrow) in arrows.iter().enumerate() {
        let same = arrow
            .from
            .iter()
            .zip(&arrow.to)
            .take_while(|(a, b)| a == b)
            .count();
        for depth in 1..arrow.from.len() {
            if depth <= same && depth < arrow.to.len() {
                continue;
            }
            leaving.entry(&arrow.from[..depth]).or_default().push(k);
        }
    }
    prefixes.sort_by_key(|(_, depth)| *depth);
    let mut removed = vec![false; arrows.len()];
    let mut lifted = Vec::new();
    for (prefix, _) in &prefixes {
        let under = count[prefix];
        let mut groups: Vec<Vec<usize>> = Vec::new();
        let mut group_of: HashMap<(&String, &Option<When>, &Vec<String>), usize> = HashMap::new();
        for &k in leaving.get(prefix.as_slice()).into_iter().flatten() {
            if removed[k] {
                continue;
            }
            let arrow = &arrows[k];
            let at = *group_of
                .entry((&arrow.labels[0], &arrow.when, &arrow.to))
                .or_insert_with(|| {
                    groups.push(Vec::new());
                    groups.len() - 1
                });
            groups[at].push(k);
        }
        for group in groups {
            if under < 2 || group.len() < under {
                continue;
            }
            for &k in &group {
                removed[k] = true;
            }
            let first = &arrows[group[0]];
            lifted.push(Arrow {
                from: prefix.clone(),
                to: first.to.clone(),
                labels: first.labels.clone(),
                when: first.when.clone(),
                bug_path: group.iter().any(|k| arrows[*k].bug_path),
            });
        }
    }
    arrows
        .into_iter()
        .zip(removed)
        .filter_map(|(arrow, removed)| (!removed).then_some(arrow))
        .chain(lifted)
        .collect()
}

/// The ends and condition that identify one drawn arrow.
type ArrowKey = (Vec<String>, Vec<String>, Option<When>);

/// Arrows with the same ends and condition become one with joined labels.
fn merge(arrows: Vec<Arrow>) -> Vec<Arrow> {
    let mut merged: Vec<Arrow> = Vec::new();
    let mut index: HashMap<ArrowKey, usize> = HashMap::new();
    for arrow in arrows {
        match index.entry((arrow.from.clone(), arrow.to.clone(), arrow.when.clone())) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                let prior = &mut merged[*entry.get()];
                prior.labels.extend(arrow.labels);
                prior.bug_path |= arrow.bug_path;
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(merged.len());
                merged.push(arrow);
            }
        }
    }
    merged
}

fn shape(projections: &[Projection], state: u32) -> Shape {
    projections
        .iter()
        .map(|p| {
            (
                p.name.clone(),
                p.leaves[p.leaf_of[state as usize] as usize].clone(),
            )
        })
        .collect()
}

/// Each region's paths prefixed with its name, nested as one parent chart.
fn as_parents(projections: &[Projection], order: &[u32], states: usize) -> Projection {
    let mut parents = Projection::new(None, states);
    let mut index = HashMap::new();
    for &state in order {
        let path = projections
            .iter()
            .flat_map(|p| {
                let name = p.region();
                p.leaves[p.leaf_of[state as usize] as usize]
                    .iter()
                    .map(move |label| format!("{name} {label}"))
            })
            .collect();
        parents.place(&mut index, state, path);
    }
    parents
}

pub(super) fn project(
    model: &Model,
    exploration: &Exploration,
    outcomes: &[Outcome],
) -> Result<Chart, Diagnostics> {
    let n = exploration.states.len();
    let names: Vec<String> = exploration
        .states
        .iter()
        .map(|state| model.state_name(state))
        .collect();
    let mut by_name: Vec<u32> = (0..n as u32).collect();
    by_name.sort_by(|a, b| names[*a as usize].cmp(&names[*b as usize]));
    let mut name_rank = vec![0u32; n];
    for (rank, state) in by_name.iter().enumerate() {
        name_rank[*state as usize] = rank as u32;
    }
    let mut order: Vec<u32> = (0..n as u32).collect();
    order.sort_by_key(|state| {
        (
            exploration.depth[*state as usize],
            name_rank[*state as usize],
        )
    });
    let mut draw_rank = vec![0u32; n];
    for (rank, state) in order.iter().enumerate() {
        draw_rank[*state as usize] = rank as u32;
    }

    let step_labels = label_steps(model, exploration)?;
    let mut bug_steps = HashSet::new();
    let mut bug_states = HashSet::new();
    for outcome in outcomes {
        let Some(bug) = &outcome.bug else { continue };
        for &j in &bug.path {
            if let StepLabel::Hidden(k) = step_labels[j as usize] {
                let step = &exploration.steps[j as usize];
                let action = &model.actions[step.action].id;
                return Err(failure(
                    "statechart/hidden-bug-step",
                    &format!("/view/steps/{k}/label"),
                    format!(
                        "The view hides the step {action}, which is on the shortest path that breaks {}; give it a label",
                        outcome.rule
                    ),
                    json!({"action":action,"rule":outcome.rule,"path":path_names(model, exploration, bug.state)}),
                ));
            }
            bug_steps.insert(j);
        }
        bug_states.extend(exploration.path_states(bug.state));
    }
    let hidden = step_labels
        .iter()
        .filter(|label| matches!(label, StepLabel::Hidden(_)))
        .count();
    let labels: Vec<Option<String>> = step_labels
        .into_iter()
        .map(|label| match label {
            StepLabel::Shown(text) => Some(text),
            StepLabel::Hidden(_) => None,
        })
        .collect();
    let mut action_rank = vec![0usize; model.actions.len()];
    let mut actions: Vec<usize> = (0..model.actions.len()).collect();
    actions.sort_by(|a, b| model.actions[*a].id.cmp(&model.actions[*b].id));
    for (rank, action) in actions.iter().enumerate() {
        action_rank[*action] = rank;
    }
    let mut sorted: Vec<u32> = (0..exploration.steps.len() as u32)
        .filter(|j| labels[*j as usize].is_some())
        .collect();
    sorted.sort_by_key(|j| {
        let step = &exploration.steps[*j as usize];
        (
            draw_rank[step.from as usize],
            draw_rank[step.to as usize],
            action_rank[step.action],
        )
    });
    let walk = Walk {
        exploration,
        order,
        labels,
        sorted,
        bug_steps,
    };

    let mut authored = Vec::new();
    for (at, region) in model.view.regions.iter().enumerate() {
        authored.push(label_region(model, exploration, &walk.order, at, region)?);
    }
    let region_names: Vec<String> = authored.iter().filter_map(|p| p.name.clone()).collect();
    let mut drawn: Vec<Vec<Move>> = (0..authored.len())
        .map(|r| moves(&walk, &authored, r))
        .collect();
    let (mut notes, mut fallback) = (Vec::new(), None);
    if authored.len() > 1 {
        let together;
        (notes, together) = regions::notes(&walk, &authored);
        fallback = regions::distortion(&walk, &authored, &drawn, &together);
    }
    let fallen;
    let projections: &[Projection] = if fallback.is_some() {
        notes.clear();
        fallen = vec![as_parents(&authored, &walk.order, n)];
        drawn = vec![moves(&walk, &fallen, 0)];
        &fallen
    } else {
        &authored
    };

    // A combination is dirty when any of its model states breaks a rule; a
    // chart state is feasible when some clean combination includes it.
    let combo = |state: u32| -> Vec<u32> {
        projections
            .iter()
            .map(|p| p.leaf_of[state as usize])
            .collect()
    };
    let mut dirty: Vec<(Vec<u32>, u32)> = Vec::new();
    let mut dirty_set = HashSet::new();
    for &state in &walk.order {
        if !exploration.violations[state as usize].is_empty() {
            let key = combo(state);
            if dirty_set.insert(key.clone()) {
                dirty.push((key, state));
            }
        }
    }
    let mut works: Vec<Vec<bool>> = projections
        .iter()
        .map(|p| vec![false; p.leaves.len()])
        .collect();
    for &state in &walk.order {
        let key = combo(state);
        if !dirty_set.contains(&key) {
            for (r, leaf) in key.iter().enumerate() {
                works[r][*leaf as usize] = true;
            }
        }
    }
    let hidden_dirty: Vec<&Vec<u32>> = dirty
        .iter()
        .map(|(key, _)| key)
        .filter(|key| {
            key.iter()
                .enumerate()
                .all(|(r, leaf)| works[r][*leaf as usize])
        })
        .collect();
    for key in hidden_dirty {
        for (r, leaf) in key.iter().enumerate() {
            works[r][*leaf as usize] = false;
        }
    }

    let rule_ids: Vec<&str> = outcomes_in_declaration(model);
    let groups: Vec<Option<&str>> = {
        let mut groups = Vec::with_capacity(n);
        for (state, values) in exploration.states.iter().enumerate() {
            let env = Env {
                state: values,
                next: &[],
                params: &[],
            };
            let mut group = None;
            for (k, rule) in model.view.groups.iter().enumerate() {
                let when = truth(rule.when.as_ref(), &env).map_err(|_| {
                    overflow(
                        (model, exploration),
                        &format!("/view/groups/{k}/when"),
                        state as u32,
                    )
                })?;
                if when {
                    group = Some(rule.group.as_str());
                    break;
                }
            }
            groups.push(group);
        }
        groups
    };

    let mut regions_out = Vec::new();
    for (r, projection) in projections.iter().enumerate() {
        let (parents, top) = parents(projection, r)?;
        let states = projection
            .leaves
            .iter()
            .zip(&projection.members)
            .enumerate()
            .map(|(leaf, (path, members))| {
                let marked = !works[r][leaf];
                let (mut breaks, mut may_break) = (Vec::new(), Vec::new());
                if marked {
                    for (rule, id) in rule_ids.iter().enumerate() {
                        let count = members
                            .iter()
                            .filter(|m| exploration.violations[**m as usize].contains(&rule))
                            .count();
                        if count == members.len() {
                            breaks.push((*id).to_owned());
                        } else if count > 0 {
                            may_break.push((*id).to_owned());
                        }
                    }
                }
                let mut member_groups = members.iter().map(|m| groups[*m as usize]);
                let first = member_groups.next().flatten();
                let color = first
                    .filter(|group| member_groups.all(|other| other == Some(group)))
                    .and_then(|group| model.view.colors.get(group).cloned());
                State {
                    path: path.clone(),
                    start: members.iter().any(|m| (*m as usize) < exploration.initial),
                    depth: exploration.depth[members[0] as usize],
                    members: members.len(),
                    mark: marked.then_some(if breaks.is_empty() {
                        Mark::MayBreak
                    } else {
                        Mark::Breaks
                    }),
                    breaks,
                    may_break,
                    bug_path: members.iter().any(|m| bug_states.contains(m)),
                    color,
                }
            })
            .collect();
        let arrows = drawn[r]
            .iter()
            .map(|item| Arrow {
                from: projection.leaves[item.from as usize].clone(),
                to: projection.leaves[item.to as usize].clone(),
                labels: vec![item.label.clone()],
                when: item.when.clone(),
                bug_path: item.steps.iter().any(|j| walk.bug_steps.contains(j)),
            })
            .collect();
        regions_out.push(Region {
            name: projection.name.clone(),
            states,
            parents,
            top,
            arrows: merge(lift(&projection.leaves, arrows)),
        });
    }

    let stories = outcomes
        .iter()
        .filter_map(|outcome| {
            let bug = outcome.bug.as_ref()?;
            let states = exploration.path_states(bug.state);
            Some(Story {
                rule: outcome.rule.clone(),
                start: shape(&authored, states[0]),
                steps: bug
                    .path
                    .iter()
                    .zip(&states[1..])
                    .map(|(j, state)| {
                        let label = walk.labels[*j as usize]
                            .clone()
                            .expect("bug steps are shown");
                        (label, shape(&authored, *state))
                    })
                    .collect(),
            })
        })
        .collect();

    Ok(Chart {
        inside: inside(&walk, projections),
        regions: regions_out,
        region_names,
        fallback,
        notes,
        hidden,
        stories,
    })
}

/// Rule ids by violation index: author rules, then deadlock.
fn outcomes_in_declaration(model: &Model) -> Vec<&str> {
    model
        .rules
        .iter()
        .map(|rule| rule.id.as_str())
        .chain(model.deadlock.as_ref().map(|_| super::explore::DEADLOCK))
        .collect()
}
