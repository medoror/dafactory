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
///
/// The real judge spawns `claude -p`, which narrates around its JSON (often inside a
/// ```` ```json ```` fence) rather than emitting a bare document. So this accepts
/// either a clean JSON document or a verdict object embedded in prose: it tries a
/// strict parse first, then each balanced `{...}` object in the text, returning the
/// first that deserializes into a `Verdict` (a stray non-verdict object is skipped).
pub fn parse_verdict(text: &str) -> Result<Verdict> {
    if let Ok(verdict) = serde_json::from_str::<Verdict>(text.trim()) {
        return Ok(verdict);
    }
    for candidate in json_object_slices(text) {
        if let Ok(verdict) = serde_json::from_str::<Verdict>(candidate) {
            return Ok(verdict);
        }
    }
    anyhow::bail!("failed to parse judge verdict JSON: no verdict object found in judge output")
}

/// Every balanced `{...}` slice in `text`, in start order, so a JSON object embedded
/// in prose (or a code fence) can be recovered. String-aware so braces inside JSON
/// string values do not throw off the nesting depth.
fn json_object_slices(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut slices = Vec::new();
    for (start, _) in bytes.iter().enumerate().filter(|(_, &b)| b == b'{') {
        if let Some(end) = matching_brace(bytes, start) {
            slices.push(&text[start..=end]);
        }
    }
    slices
}

/// The index of the `}` that closes the `{` at `start`, accounting for nested objects
/// and braces inside double-quoted strings; `None` if unbalanced.
fn matching_brace(bytes: &[u8], start: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (i, &c) in bytes.iter().enumerate().skip(start) {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
            }
        } else {
            match c {
                b'"' => in_string = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
    }
    None
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

    #[test]
    fn should_extract_verdict_json_from_a_fenced_code_block() {
        // The real `claude -p` judge narrates around its JSON (verified empirically),
        // often in a ```json fence. parse_verdict must find the verdict object, not
        // assume the whole output is JSON.
        let prose = "I drove the app against the scenarios.\n\n\
             ```json\n\
             {\"scenarios\": [{\"id\": \"S001\", \"satisfied\": true, \"observed\": \"named Apple and showed a price\"}]}\n\
             ```\n\n\
             That is my verdict.";

        let verdict = parse_verdict(prose).unwrap();

        assert_eq!(verdict.total_count(), 1);
        assert!(verdict.scenarios[0].satisfied);
    }

    #[test]
    fn should_extract_verdict_json_embedded_in_prose_without_fences() {
        let prose = "Here is the verdict: \
             {\"scenarios\":[{\"id\":\"S001\",\"satisfied\":false,\"observed\":\"no runnable tool found\"}]} \
             — done.";

        let verdict = parse_verdict(prose).unwrap();

        assert_eq!(verdict.total_count(), 1);
        assert_eq!(verdict.satisfied_count(), 0);
    }

    #[test]
    fn should_skip_non_verdict_objects_and_find_the_real_one() {
        // A stray JSON-looking object in the prose must not be mistaken for the verdict.
        let prose = "Config was {\"mode\": \"black-box\"}. Verdict: \
             {\"scenarios\":[{\"id\":\"S001\",\"satisfied\":true,\"observed\":\"ok\"}]}";

        let verdict = parse_verdict(prose).unwrap();

        assert_eq!(verdict.total_count(), 1);
        assert!(verdict.scenarios[0].satisfied);
    }

    #[test]
    fn should_error_when_no_verdict_json_is_present() {
        assert!(parse_verdict("I was unable to produce a verdict.").is_err());
    }
}
