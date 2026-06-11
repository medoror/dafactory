//! The evidence bundle (ADR-0006): the unit of output. Written under the factory
//! root's `evidence/<run_id>/`. This module carries the fields a `validate` produces;
//! `run` (B3) will extend the bundle with the terminal state, originating intent, and
//! diff. A `validate` bundle deliberately carries no terminal state — that is `run`'s
//! exclusive output.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::judge::Verdict;

/// One scenario's result as recorded in the bundle: the boolean plus its transcript.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub id: String,
    pub satisfied: bool,
    pub observed: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
}

/// A validation evidence bundle (ADR-0006, scaled to `validate`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationBundle {
    /// Discriminates this from a `run` bundle.
    pub kind: String,
    pub app: String,
    pub run_id: String,
    pub satisfaction: u8,
    pub satisfied_count: usize,
    pub total_count: usize,
    pub summary: String,
    pub scenarios: Vec<ScenarioResult>,
}

impl ValidationBundle {
    /// Build a validation bundle from a judge verdict. Satisfaction is derived from
    /// the per-scenario booleans (ADR-0008), not carried from the judge.
    pub fn from_verdict(app: &str, run_id: &str, verdict: &Verdict) -> ValidationBundle {
        let satisfied = verdict.satisfied_count();
        let total = verdict.total_count();
        let scenarios = verdict
            .scenarios
            .iter()
            .map(|s| ScenarioResult {
                id: s.id.clone(),
                satisfied: s.satisfied,
                observed: s.observed.clone(),
                expected: s.expected.clone(),
            })
            .collect();
        ValidationBundle {
            kind: "validate".to_string(),
            app: app.to_string(),
            run_id: run_id.to_string(),
            satisfaction: verdict.satisfaction(),
            satisfied_count: satisfied,
            total_count: total,
            summary: format!(
                "{satisfied}/{total} scenarios satisfied ({}%)",
                verdict.satisfaction()
            ),
            scenarios,
        }
    }

    /// Write the bundle as `factory_root/evidence/<run_id>/bundle.json` and return the
    /// directory it was written to.
    pub fn write(&self, factory_root: &Path) -> Result<PathBuf> {
        write_bundle(factory_root, &self.run_id, self)
    }
}

/// Write any serializable bundle to `factory_root/evidence/<run_id>/bundle.json`.
fn write_bundle<T: Serialize>(factory_root: &Path, run_id: &str, bundle: &T) -> Result<PathBuf> {
    let dir = factory_root.join("evidence").join(run_id);
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let path = dir.join("bundle.json");
    let json = serde_json::to_string_pretty(bundle).context("failed to serialize bundle")?;
    fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(dir)
}

/// The terminal state a `run` pass emits (SPEC). Serialized as the exact wire strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalState {
    #[serde(rename = "PR_READY")]
    PrReady,
    #[serde(rename = "NO_OP")]
    NoOp,
    #[serde(rename = "ESCALATE")]
    Escalate,
    #[serde(rename = "RETRYABLE")]
    Retryable,
    #[serde(rename = "NEEDS_DECISION")]
    NeedsDecision,
}

impl TerminalState {
    pub fn as_str(&self) -> &'static str {
        match self {
            TerminalState::PrReady => "PR_READY",
            TerminalState::NoOp => "NO_OP",
            TerminalState::Escalate => "ESCALATE",
            TerminalState::Retryable => "RETRYABLE",
            TerminalState::NeedsDecision => "NEEDS_DECISION",
        }
    }

    /// Process exit code for this terminal state (ADR-0012). Success states are 0;
    /// the non-success states use a distinct band (10+) so a wrapper / the AFK loop
    /// can branch on `$?` (retry on 10, stop on 11, surface a decision on 12) without
    /// colliding with `1` (hard error) or `2` (clap usage).
    pub fn exit_code(&self) -> u8 {
        match self {
            TerminalState::PrReady | TerminalState::NoOp => 0,
            TerminalState::Retryable => 10,
            TerminalState::Escalate => 11,
            TerminalState::NeedsDecision => 12,
        }
    }
}

impl std::fmt::Display for TerminalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The originating backlog intent, recorded in a run bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub title: String,
    pub raw: String,
}

/// The change a run produced (ADR-0006): the diff, whether it was committed, and the
/// commit hash. `diff` is the exact staged/committed diff; empty means "no change".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub committed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    pub diff: String,
}

/// A `run` evidence bundle (ADR-0006): terminal state, originating intent, the
/// validation result it wraps, and the change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunBundle {
    pub kind: String,
    pub app: String,
    pub run_id: String,
    pub terminal_state: TerminalState,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<IntentRecord>,
    /// The agent's stdout narration. Recorded as evidence, never interpreted as edits
    /// (ADR-0004). `None` when the agent did not run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_log: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<ValidationBundle>,
    pub change: ChangeRecord,
    pub residual: String,
}

impl RunBundle {
    pub fn write(&self, factory_root: &Path) -> Result<PathBuf> {
        write_bundle(factory_root, &self.run_id, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judge::ScenarioOutcome;

    #[test]
    fn should_map_each_terminal_state_to_its_exit_code() {
        // Success states 0; non-success states distinct non-zero (ADR-0012).
        assert_eq!(TerminalState::PrReady.exit_code(), 0);
        assert_eq!(TerminalState::NoOp.exit_code(), 0);
        assert_eq!(TerminalState::Retryable.exit_code(), 10);
        assert_eq!(TerminalState::Escalate.exit_code(), 11);
        assert_eq!(TerminalState::NeedsDecision.exit_code(), 12);
    }

    fn mixed_verdict() -> Verdict {
        Verdict {
            scenarios: vec![
                ScenarioOutcome {
                    id: "S001".into(),
                    satisfied: true,
                    observed: "drove it, got the right answer".into(),
                    expected: None,
                },
                ScenarioOutcome {
                    id: "S002".into(),
                    satisfied: false,
                    observed: "drove it, got the wrong answer".into(),
                    expected: Some("42".into()),
                },
            ],
        }
    }

    #[test]
    fn should_derive_bundle_fields_from_verdict() {
        let bundle = ValidationBundle::from_verdict("demo", "run-1", &mixed_verdict());

        assert_eq!(bundle.kind, "validate");
        assert_eq!(bundle.satisfaction, 50);
        assert_eq!(bundle.satisfied_count, 1);
        assert_eq!(bundle.total_count, 2);
        assert!(bundle.summary.contains("1/2"));
    }

    #[test]
    fn should_record_satisfied_and_unsatisfied_scenarios_differently() {
        let bundle = ValidationBundle::from_verdict("demo", "run-1", &mixed_verdict());

        let s1 = &bundle.scenarios[0];
        let s2 = &bundle.scenarios[1];
        assert!(s1.satisfied && !s2.satisfied);
        assert_ne!(s1.observed, s2.observed);
        assert_eq!(s2.expected.as_deref(), Some("42"));
    }

    #[test]
    fn should_write_bundle_under_evidence_run_id() {
        let dir = tempfile::tempdir().unwrap();
        let factory_root = dir.path().join("factory");
        let bundle = ValidationBundle::from_verdict("demo", "run-1", &mixed_verdict());

        let written = bundle.write(&factory_root).unwrap();

        assert_eq!(written, factory_root.join("evidence/run-1"));
        let json = fs::read_to_string(written.join("bundle.json")).unwrap();
        let parsed: ValidationBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, bundle);
    }
}
