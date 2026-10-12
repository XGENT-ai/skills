use crate::{
    capture::CellKey,
    storage::{error, hash, StoreError},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Identity {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct State {
    pub identity: Identity,
    pub steps: Vec<Identity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Viewport {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub dpr: f64,
    pub full_page: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Unit {
    pub identity: Identity,
    pub states: Vec<State>,
    pub viewports: Vec<Viewport>,
    pub themes: Vec<Identity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenCell {
    pub key: CellKey,
    pub viewport: Viewport,
    pub filename: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenMatrix {
    pub schema_version: u32,
    pub units: Vec<Unit>,
    pub scope: Option<Vec<String>>,
    pub full: bool,
    pub cells: Vec<FrozenCell>,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

fn identities<'a>(entries: impl Iterator<Item = &'a Identity>) -> Result<(), StoreError> {
    let mut ids = BTreeSet::new();
    for entry in entries {
        if !valid_id(&entry.id)
            || entry.label.trim().is_empty()
            || !ids.insert(entry.id.to_ascii_lowercase())
        {
            return Err(error(
                2,
                "Matrix identities need unique portable IDs and display labels",
            ));
        }
    }
    if ids.is_empty() {
        return Err(error(2, "Frozen matrix axes cannot be empty"));
    }
    Ok(())
}

fn cell_digest(key: &CellKey) -> String {
    hash(
        &serde_json::to_vec(&[
            &key.unit_id,
            &key.state_id,
            &key.viewport_id,
            &key.theme_id,
            &key.step_id,
        ])
        .unwrap(),
    )
}

pub fn freeze(units: Vec<Unit>, scope: Option<Vec<String>>) -> Result<FrozenMatrix, StoreError> {
    identities(units.iter().map(|unit| &unit.identity))?;
    if let Some(scope) = &scope {
        let mut selected = BTreeSet::new();
        if scope.is_empty()
            || scope
                .iter()
                .any(|id| !selected.insert(id) || !units.iter().any(|unit| unit.identity.id == *id))
        {
            return Err(error(2, "Scope must list unique known units"));
        }
    }
    let mut bindings = BTreeMap::new();
    let mut cells = Vec::new();
    for unit in &units {
        identities(unit.states.iter().map(|state| &state.identity))?;
        identities(unit.themes.iter())?;
        let mut viewport_ids = BTreeSet::new();
        if unit.viewports.is_empty() {
            return Err(error(2, "Frozen matrix needs viewports"));
        }
        for viewport in &unit.viewports {
            if !valid_id(&viewport.id)
                || !viewport_ids.insert(viewport.id.to_ascii_lowercase())
                || viewport.width == 0
                || viewport.height == 0
                || !viewport.dpr.is_finite()
                || viewport.dpr <= 0.0
            {
                return Err(error(
                    2,
                    "Viewport needs a unique ID, dimensions and a finite positive DPR",
                ));
            }
            for length in [viewport.width, viewport.height] {
                let pixels = (f64::from(length) * viewport.dpr).round();
                if !(1.0..=f64::from(u32::MAX)).contains(&pixels) {
                    return Err(error(2, "Viewport pixel dimensions exceed the image limit"));
                }
            }
            if bindings
                .insert(viewport.id.to_ascii_lowercase(), viewport)
                .is_some_and(|prior| prior != viewport)
            {
                return Err(error(
                    2,
                    "A viewport ID cannot change its dimension or DPR binding",
                ));
            }
        }
        for state in &unit.states {
            identities(state.steps.iter())?;
        }
        let steps: usize = unit.states.iter().map(|state| state.steps.len()).sum();
        let count = steps
            .checked_mul(unit.viewports.len())
            .and_then(|count| count.checked_mul(unit.themes.len()))
            .and_then(|count| count.checked_add(cells.len()));
        if count.is_none_or(|count| count > 100_000) {
            return Err(error(2, "Frozen matrix exceeds the cell resource limit"));
        }
        for state in &unit.states {
            for viewport in &unit.viewports {
                let same_width = unit
                    .viewports
                    .iter()
                    .filter(|other| other.width == viewport.width)
                    .count()
                    > 1;
                let dimensions = if same_width {
                    format!(
                        "{}-h{}-dpr{}",
                        viewport.width, viewport.height, viewport.dpr
                    )
                } else {
                    viewport.width.to_string()
                };
                for theme in &unit.themes {
                    for step in &state.steps {
                        let suffix = if step.id == "initial" {
                            String::new()
                        } else {
                            format!("-{}", step.id)
                        };
                        cells.push(FrozenCell {
                            key: CellKey {
                                unit_id: unit.identity.id.clone(),
                                state_id: state.identity.id.clone(),
                                viewport_id: viewport.id.clone(),
                                theme_id: theme.id.clone(),
                                step_id: step.id.clone(),
                            },
                            viewport: viewport.clone(),
                            filename: format!(
                                "{}-{}-{}-{}{}.png",
                                unit.identity.id, state.identity.id, dimensions, theme.id, suffix
                            ),
                        });
                    }
                }
            }
        }
    }
    let mut counts = BTreeMap::<String, usize>::new();
    for cell in &cells {
        *counts
            .entry(cell.filename.to_ascii_lowercase())
            .or_default() += 1;
    }
    for cell in &mut cells {
        if cell.filename.len() > 200
            || (counts[&cell.filename.to_ascii_lowercase()] > 1 && cell.filename.len() + 65 > 200)
        {
            cell.filename = format!(
                "{}-{}-{}.png",
                cell.key.unit_id,
                cell.key.state_id,
                cell_digest(&cell.key)
            );
        } else if counts[&cell.filename.to_ascii_lowercase()] > 1 {
            cell.filename = format!(
                "{}-{}.png",
                cell.filename.strip_suffix(".png").unwrap(),
                cell_digest(&cell.key)
            );
        }
    }
    let mut names = BTreeSet::new();
    if cells
        .iter()
        .any(|cell| !names.insert(cell.filename.to_ascii_lowercase()))
    {
        return Err(error(2, "Screenshot names are ambiguous"));
    }
    Ok(FrozenMatrix {
        schema_version: 1,
        units,
        full: scope.is_none(),
        scope,
        cells,
    })
}

impl FrozenMatrix {
    pub fn validate(&self) -> Result<(), StoreError> {
        if self.schema_version != 1 || freeze(self.units.clone(), self.scope.clone())? != *self {
            return Err(error(
                2,
                "Frozen matrix coverage or filename binding changed",
            ));
        }
        Ok(())
    }
    pub fn selected_units(&self) -> Vec<&str> {
        match &self.scope {
            Some(scope) => scope.iter().map(String::as_str).collect(),
            None => self
                .units
                .iter()
                .map(|unit| unit.identity.id.as_str())
                .collect(),
        }
    }
    pub fn screenshot_name(&self, cell: &CellKey, attempt: u32) -> Result<String, StoreError> {
        self.validate()?;
        if attempt == 0 {
            return Err(error(2, "Capture attempt must be positive"));
        }
        let name = &self
            .cells
            .iter()
            .find(|entry| entry.key == *cell)
            .ok_or_else(|| error(2, "Capture cell is outside the frozen matrix"))?
            .filename;
        Ok(if attempt == 1 {
            name.clone()
        } else {
            format!("{}-a{attempt}.png", name.strip_suffix(".png").unwrap())
        })
    }
}
