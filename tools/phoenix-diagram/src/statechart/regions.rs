//! Parallel regions: the condition each arrow puts on the other regions, the
//! notes on steps that change several regions, and whether the regions chart
//! tells the truth about the model.
use super::chart::{Clause, Combo, Fallback, Move, Note, Projection, Walk, When};
use std::collections::{HashMap, HashSet};

/// When an arrow of one region can be taken, over the leaves of the others.
#[derive(Clone, Debug)]
pub(super) enum Condition {
    /// Every other region is in its set.
    All(Vec<HashSet<u32>>),
    /// Some other region is in its set.
    Any(Vec<HashSet<u32>>),
    /// Only these combinations; no sentence can say it.
    Tuples(HashSet<Vec<u32>>),
}

impl Condition {
    pub fn holds(&self, rest: &[u32]) -> bool {
        match self {
            Condition::All(sets) => sets.iter().zip(rest).all(|(set, leaf)| set.contains(leaf)),
            Condition::Any(sets) => sets.iter().zip(rest).any(|(set, leaf)| set.contains(leaf)),
            Condition::Tuples(tuples) => tuples.contains(rest),
        }
    }
}

fn others(projections: &[Projection], r: usize) -> Vec<usize> {
    (0..projections.len()).filter(|o| *o != r).collect()
}

fn values(projection: &Projection, leaves: impl IntoIterator<Item = u32>) -> Vec<String> {
    let mut texts: Vec<String> = leaves
        .into_iter()
        .map(|leaf| projection.leaves[leaf as usize].join(" / "))
        .collect();
    texts.sort();
    texts
}

/// The other-region combinations seen with the move's source, and the
/// simplest "and" or "or" condition that matches exactly the ones taking it.
pub(super) fn condition(
    walk: &Walk<'_>,
    projections: &[Projection],
    r: usize,
    item: &Move,
    seen_cache: &mut HashMap<u32, Vec<Vec<u32>>>,
) -> (Condition, Option<When>) {
    let others = others(projections, r);
    let tuple = |state: u32| -> Vec<u32> {
        others
            .iter()
            .map(|o| projections[*o].leaf_of[state as usize])
            .collect()
    };
    let ok: HashSet<Vec<u32>> = item
        .steps
        .iter()
        .map(|j| tuple(walk.exploration.steps[*j as usize].from))
        .collect();
    let seen = seen_cache.entry(item.from).or_insert_with(|| {
        let mut unique = HashSet::new();
        projections[r].members[item.from as usize]
            .iter()
            .map(|state| tuple(*state))
            .filter(|t| unique.insert(t.clone()))
            .collect()
    });
    let width = others.len();
    let mut all: Vec<Vec<u32>> = vec![Vec::new(); width];
    let mut taken: Vec<HashSet<u32>> = vec![HashSet::new(); width];
    let mut untaken: Vec<HashSet<u32>> = vec![HashSet::new(); width];
    for t in seen.iter() {
        let takes = ok.contains(t);
        for (x, leaf) in t.iter().enumerate() {
            if !all[x].contains(leaf) {
                all[x].push(*leaf);
            }
            if takes {
                taken[x].insert(*leaf);
            } else {
                untaken[x].insert(*leaf);
            }
        }
    }
    let fits = |condition: &Condition| seen.iter().all(|t| condition.holds(t) == ok.contains(t));
    let name = |x: usize| projections[others[x]].region();

    let and = Condition::All(taken.clone());
    if fits(&and) {
        let mut clauses = Vec::new();
        for x in 0..width {
            let no: Vec<u32> = all[x]
                .iter()
                .copied()
                .filter(|leaf| !taken[x].contains(leaf))
                .collect();
            if no.is_empty() {
                continue;
            }
            let projection = &projections[others[x]];
            clauses.push(if taken[x].len() <= no.len() {
                Clause {
                    region: name(x),
                    negated: false,
                    values: values(projection, taken[x].iter().copied()),
                }
            } else {
                Clause {
                    region: name(x),
                    negated: true,
                    values: values(projection, no),
                }
            });
        }
        return (and, (!clauses.is_empty()).then_some(When::All(clauses)));
    }
    let only: Vec<HashSet<u32>> = (0..width)
        .map(|x| {
            all[x]
                .iter()
                .copied()
                .filter(|leaf| !untaken[x].contains(leaf))
                .collect()
        })
        .collect();
    let or = Condition::Any(only.clone());
    if fits(&or) {
        let clauses = (0..width)
            .filter(|x| !only[*x].is_empty())
            .map(|x| Clause {
                region: name(x),
                negated: false,
                values: values(&projections[others[x]], only[x].iter().copied()),
            })
            .collect();
        return (or, Some(When::Any(clauses)));
    }
    (Condition::Tuples(ok), None)
}

type LabelChanges<'a> = (&'a str, Vec<bool>, Vec<Vec<usize>>);

/// Notes on the labels that change two or more regions, and the labels that
/// change several regions in one step.
pub(super) fn notes(walk: &Walk<'_>, projections: &[Projection]) -> (Vec<Note>, HashSet<String>) {
    let regions = projections.len();
    // Per label: the regions it changes, and each set it changes at once.
    let mut by_label: Vec<LabelChanges<'_>> = Vec::new();
    let mut index: HashMap<&str, usize> = HashMap::new();
    for &j in &walk.sorted {
        let step = &walk.exploration.steps[j as usize];
        let label = walk.labels[j as usize]
            .as_deref()
            .expect("sorted steps are shown");
        let at = *index.entry(label).or_insert_with(|| {
            by_label.push((label, vec![false; regions], Vec::new()));
            by_label.len() - 1
        });
        let changed: Vec<usize> = (0..regions)
            .filter(|r| {
                let leaf_of = &projections[*r].leaf_of;
                leaf_of[step.from as usize] != leaf_of[step.to as usize]
            })
            .collect();
        let entry = &mut by_label[at];
        for r in &changed {
            entry.1[*r] = true;
        }
        if changed.len() > 1 && !entry.2.contains(&changed) {
            entry.2.push(changed);
        }
    }
    let name = |r: &usize| projections[*r].region();
    let mut together = HashSet::new();
    let mut notes = Vec::new();
    for (label, changed, sets) in by_label {
        let changed: Vec<String> = (0..regions)
            .filter(|r| changed[*r])
            .map(|r| name(&r))
            .collect();
        if changed.len() < 2 {
            continue;
        }
        if sets.is_empty() {
            notes.push(Note::OneAtATime {
                step: label.into(),
                regions: changed,
            });
        } else {
            together.insert(label.to_owned());
            notes.push(Note::Together {
                step: label.into(),
                sets: sets
                    .iter()
                    .map(|set| set.iter().map(name).collect())
                    .collect(),
            });
        }
    }
    (notes, together)
}

fn words(projections: &[Projection], combo: &[u32]) -> Combo {
    projections
        .iter()
        .zip(combo)
        .map(|(p, leaf)| (p.region(), p.leaves[*leaf as usize].clone()))
        .collect()
}

/// Insertion-ordered set of moves between combinations.
#[derive(Default)]
struct Moves {
    list: Vec<(Vec<u32>, String, Vec<u32>)>,
    set: HashSet<(Vec<u32>, String, Vec<u32>)>,
}

impl Moves {
    fn add(&mut self, from: Vec<u32>, label: &str, to: Vec<u32>) {
        let key = (from, label.to_owned(), to);
        if self.set.insert(key.clone()) {
            self.list.push(key);
        }
    }
}

/// The first way a regions chart would misstate the model, if any: a start
/// the model never has, a condition no sentence can say, or a move the chart
/// allows and the model does not make, or the reverse.
pub(super) fn distortion(
    walk: &Walk<'_>,
    projections: &[Projection],
    moves: &[Vec<Move>],
    together: &HashSet<String>,
) -> Option<Fallback> {
    let exploration = walk.exploration;
    let combo = |state: u32| -> Vec<u32> {
        projections
            .iter()
            .map(|p| p.leaf_of[state as usize])
            .collect()
    };

    let real_starts: HashSet<Vec<u32>> = (0..exploration.initial as u32).map(combo).collect();
    let mut product: Vec<Vec<u32>> = vec![Vec::new()];
    for projection in projections {
        let mut starts: Vec<u32> = (0..exploration.initial)
            .map(|state| projection.leaf_of[state])
            .collect();
        starts.sort_unstable();
        starts.dedup();
        product = product
            .into_iter()
            .flat_map(|prefix| {
                starts.iter().map(move |leaf| {
                    let mut next = prefix.clone();
                    next.push(*leaf);
                    next
                })
            })
            .collect();
    }
    if let Some(start) = product.iter().find(|start| !real_starts.contains(*start)) {
        return Some(Fallback::Starts(words(projections, start)));
    }

    for (r, region) in moves.iter().enumerate() {
        if let Some(item) = region
            .iter()
            .find(|item| matches!(item.condition, Condition::Tuples(_)))
        {
            return Some(Fallback::Condition {
                region: projections[r].region(),
                label: item.label.clone(),
                from: projections[r].leaves[item.from as usize].clone(),
            });
        }
    }

    let mut real = Moves::default();
    for &j in &walk.sorted {
        let step = &exploration.steps[j as usize];
        let (before, after) = (combo(step.from), combo(step.to));
        if before != after {
            real.add(
                before,
                walk.labels[j as usize]
                    .as_deref()
                    .expect("sorted steps are shown"),
                after,
            );
        }
    }
    let mut combos = Vec::new();
    let mut unique = HashSet::new();
    for &state in &walk.order {
        let key = combo(state);
        if unique.insert(key.clone()) {
            combos.push(key);
        }
    }
    let by_source: Vec<HashMap<u32, Vec<&Move>>> = moves
        .iter()
        .map(|region| {
            let mut index: HashMap<u32, Vec<&Move>> = HashMap::new();
            for item in region {
                index.entry(item.from).or_default().push(item);
            }
            index
        })
        .collect();
    let mut drawn = Moves::default();
    for c in &combos {
        let mut enabled: Vec<(&str, Vec<(usize, u32)>)> = Vec::new();
        for (r, index) in by_source.iter().enumerate() {
            let rest: Vec<u32> = c
                .iter()
                .enumerate()
                .filter(|(k, _)| *k != r)
                .map(|(_, leaf)| *leaf)
                .collect();
            for item in index.get(&c[r]).into_iter().flatten() {
                if !item.condition.holds(&rest) {
                    continue;
                }
                match enabled.iter_mut().find(|(label, _)| *label == item.label) {
                    Some((_, list)) => list.push((r, item.to)),
                    None => enabled.push((&item.label, vec![(r, item.to)])),
                }
            }
        }
        for (label, list) in enabled {
            let groups: Vec<Vec<(usize, u32)>> = if together.contains(label) {
                let mut by_region: Vec<(usize, Vec<(usize, u32)>)> = Vec::new();
                for option in list {
                    match by_region.iter_mut().find(|(r, _)| *r == option.0) {
                        Some((_, options)) => options.push(option),
                        None => by_region.push((option.0, vec![option])),
                    }
                }
                let mut groups = vec![Vec::new()];
                for (_, options) in by_region {
                    groups = groups
                        .into_iter()
                        .flat_map(|group: Vec<(usize, u32)>| {
                            options.iter().map(move |option| {
                                let mut next = group.clone();
                                next.push(*option);
                                next
                            })
                        })
                        .collect();
                }
                groups
            } else {
                list.into_iter().map(|option| vec![option]).collect()
            };
            for group in groups {
                let mut to = c.clone();
                for (r, leaf) in group {
                    to[r] = leaf;
                }
                drawn.add(c.clone(), label, to);
            }
        }
    }
    if let Some((from, label, to)) = drawn.list.iter().find(|item| !real.set.contains(*item)) {
        return Some(Fallback::MoveExtra {
            from: words(projections, from),
            label: label.clone(),
            to: words(projections, to),
        });
    }
    if let Some((from, label, to)) = real.list.iter().find(|item| !drawn.set.contains(*item)) {
        return Some(Fallback::MoveMissing {
            from: words(projections, from),
            label: label.clone(),
            to: words(projections, to),
        });
    }
    None
}
