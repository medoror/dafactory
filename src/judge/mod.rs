//! The judge (ADR-0001, ADR-0008). A narrow trait with two shipped providers:
//! `real` (spawns `claude -p`, the default) and `scripted` (canned per-scenario
//! verdicts, trusted-runner-only). `cargo test` may also inject in-crate doubles.
//!
//! `factory` derives the satisfaction fraction from the per-scenario booleans, never
//! from a self-reported total — a fabricated number cannot override the honest count.

pub mod real;
pub mod scripted;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use real::RealJudge;
use scripted::ScriptedJudge;

/// What the judge is asked to evaluate. Gives the judge the holdout data
/// (scenarios + judge prompt) and the code root it may drive — never the source it
/// must not read; that boundary is the judge's discipline (ADR-0002, judge.md).
pub struct JudgeRequest {
    pub app: String,
    pub code_root: PathBuf,
    pub factory_root: PathBuf,
}

impl JudgeRequest {
    pub fn scenarios_dir(&self) -> PathBuf {
        self.factory_root.join("scenarios")
    }

    pub fn judge_md(&self) -> PathBuf {
        self.factory_root.join("judge.md")
    }
}

/// One scenario's verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioOutcome {
    pub id: String,
    pub satisfied: bool,
    /// What the judge did and observed, in plain language — the per-scenario
    /// transcript (ADR-0006).
    #[serde(default)]
    pub observed: String,
    #[serde(default)]
    pub expected: Option<String>,
}

/// The judge's full verdict: per-scenario outcomes. The satisfaction fraction is a
/// derived quantity, computed here from the booleans rather than carried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub scenarios: Vec<ScenarioOutcome>,
}

impl Verdict {
    pub fn satisfied_count(&self) -> usize {
        self.scenarios.iter().filter(|s| s.satisfied).count()
    }

    pub fn total_count(&self) -> usize {
        self.scenarios.len()
    }

    /// Integer percentage, rounded to nearest, derived from the per-scenario
    /// booleans. An empty verdict is 0 (nothing was satisfied).
    pub fn satisfaction(&self) -> u8 {
        let total = self.total_count();
        if total == 0 {
            return 0;
        }
        let satisfied = self.satisfied_count();
        ((100 * satisfied + total / 2) / total) as u8
    }
}

/// The judge interface. Production impls spawn a subprocess; tests inject doubles.
pub trait Judge {
    fn judge(&self, request: &JudgeRequest) -> Result<Verdict>;
}

/// Which judge provider to use. Selected at runtime (flag/env), default real.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Real,
    Scripted,
}

impl Provider {
    /// Resolve the provider from an explicit choice (e.g. a `--judge` flag) falling
    /// back to the `FACTORY_JUDGE` env value, defaulting to `real`.
    pub fn resolve(flag: Option<&str>, env: Option<&str>) -> Result<Provider> {
        match flag.or(env) {
            None | Some("real") => Ok(Provider::Real),
            Some("scripted") => Ok(Provider::Scripted),
            Some(other) => {
                anyhow::bail!("unknown judge provider '{other}' (expected 'real' or 'scripted')")
            }
        }
    }
}

/// Build the selected judge. The scripted judge's canned-verdict source is the
/// `script` path (e.g. from `FACTORY_JUDGE_SCRIPT`); it is trusted-runner-only
/// (ADR-0008).
pub fn build(provider: Provider, script: Option<PathBuf>) -> Result<Box<dyn Judge>> {
    match provider {
        Provider::Real => Ok(Box::new(RealJudge::new())),
        Provider::Scripted => {
            let path = script
                .context("the scripted judge needs a verdict file (set FACTORY_JUDGE_SCRIPT)")?;
            Ok(Box::new(ScriptedJudge::new(path)))
        }
    }
}

/// Parse a judge's JSON output into a `Verdict`, keeping only the per-scenario
/// outcomes. Any self-reported `satisfaction`/`satisfied_count` is intentionally
/// ignored — `factory` recomputes it (ADR-0008).
pub fn parse_verdict(json: &str) -> Result<Verdict> {
    serde_json::from_str(json).context("failed to parse judge verdict JSON")
}

/// Load and parse a verdict from a file. Shared by the scripted judge and by the
/// real judge when it writes its output to a file.
pub fn load_verdict(path: &Path) -> Result<Verdict> {
    let json = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read judge verdict at {}", path.display()))?;
    parse_verdict(&json)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(id: &str, satisfied: bool) -> ScenarioOutcome {
        ScenarioOutcome {
            id: id.to_string(),
            satisfied,
            observed: String::new(),
            expected: None,
        }
    }

    #[test]
    fn should_report_zero_satisfaction_for_empty_verdict() {
        let verdict = Verdict { scenarios: vec![] };
        assert_eq!(verdict.satisfaction(), 0);
    }

    #[test]
    fn should_round_satisfaction_to_nearest_percent() {
        let cases = [
            (vec![false, false], 0),
            (vec![true, false], 50),
            (vec![true, true], 100),
            (vec![true, false, false], 33),
            (vec![true, true, false], 67),
        ];
        for (bools, expected) in cases {
            let scenarios = bools
                .iter()
                .enumerate()
                .map(|(i, &b)| outcome(&format!("S{i}"), b))
                .collect();
            let verdict = Verdict { scenarios };
            assert_eq!(verdict.satisfaction(), expected, "for {bools:?}");
        }
    }

    #[test]
    fn should_parse_per_scenario_outcomes_and_ignore_self_reported_total() {
        // The judge claims 100% but only one of two scenarios is satisfied; the
        // self-reported number must not survive.
        let json = r#"{
            "scenarios": [
                {"id": "S001", "satisfied": true,  "observed": "ran ok"},
                {"id": "S002", "satisfied": false, "observed": "wrong output", "expected": "42"}
            ],
            "satisfied_count": 2,
            "satisfaction": 100
        }"#;

        let verdict = parse_verdict(json).unwrap();

        assert_eq!(verdict.total_count(), 2);
        assert_eq!(verdict.satisfied_count(), 1);
        assert_eq!(verdict.satisfaction(), 50);
        assert_eq!(verdict.scenarios[1].expected.as_deref(), Some("42"));
    }

    #[test]
    fn should_resolve_provider_from_flag_then_env_then_default() {
        assert_eq!(Provider::resolve(None, None).unwrap(), Provider::Real);
        assert_eq!(
            Provider::resolve(None, Some("scripted")).unwrap(),
            Provider::Scripted
        );
        assert_eq!(
            Provider::resolve(Some("real"), Some("scripted")).unwrap(),
            Provider::Real
        );
        assert!(Provider::resolve(Some("bogus"), None).is_err());
    }
}
