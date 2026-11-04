// Chart semantics derived from pstack skills/statechart (MIT); see NOTICE.md.
//! Statechart: a finite JSON model explored exhaustively, its rules judged in
//! every reachable state, and the shortest path to each broken rule.
use crate::diagnostic::{Diagnostic, Diagnostics, Severity};
use serde_json::Value;

pub mod chart;
pub mod explore;
pub mod expr;
pub mod model;
mod regions;
pub mod report;

pub use chart::Chart;
pub use explore::{Exploration, Outcome, Verdict};
pub use model::Model;

/// Explored model states stop here; unseen violations are then not checked.
pub const STATE_LIMIT: usize = 100_000;

pub(crate) struct Problems(pub Diagnostics);

impl Problems {
    pub fn push(&mut self, code: &str, path: &str, message: impl Into<String>, evidence: Value) {
        let mut diagnostic = Diagnostic::error(Some("statechart"), code, path, message);
        diagnostic.evidence = evidence;
        self.0.push(diagnostic);
    }
}

/// Model and view rules the schema cannot express: names, types and domains.
pub fn check_static(document: &Value) -> Diagnostics {
    Model::parse(document).err().unwrap_or_default()
}

pub struct Analysis {
    pub model: Model,
    pub exploration: Exploration,
    pub outcomes: Vec<Outcome>,
    pub chart: Chart,
    pub warnings: Diagnostics,
}

impl Analysis {
    pub fn complete(&self) -> bool {
        self.exploration.complete
    }
    pub fn outcome(&self, rule: &str) -> Option<&Outcome> {
        self.outcomes.iter().find(|outcome| outcome.rule == rule)
    }
    /// The default names of the states along a bug path, initial state first.
    pub fn path_names(&self, outcome: &Outcome) -> Vec<String> {
        let Some(bug) = &outcome.bug else {
            return vec![];
        };
        self.exploration
            .path_states(bug.state)
            .into_iter()
            .map(|state| {
                self.model
                    .state_name(&self.exploration.states[state as usize])
            })
            .collect()
    }
}

/// Explore a statically valid document, judge its rules and project the chart.
pub fn analyze(document: &Value) -> Result<Analysis, Diagnostics> {
    let model = Model::parse(document)?;
    let exploration = explore::explore(&model)?;
    let outcomes = exploration.outcomes(&model);
    let chart = chart::project(&model, &exploration, &outcomes)?;
    let mut warnings = Vec::new();
    if !exploration.complete {
        let mut warning = Diagnostic::error(
            Some("statechart"),
            "statechart/state-limit",
            "/model",
            format!(
                "Stopped after {STATE_LIMIT} model states; rules without a violation so far are not checked"
            ),
        );
        warning.severity = Severity::Warning;
        warning.evidence = serde_json::json!({"limit":STATE_LIMIT,"expanded":exploration.expanded});
        warnings.push(warning);
    }
    Ok(Analysis {
        model,
        exploration,
        outcomes,
        chart,
        warnings,
    })
}
