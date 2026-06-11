//! `factory validate <app>` (B2): run the held-out judge against an app and record
//! the result. Executed by `factory`, never the implementer (ADR-0002).
//!
//! Resolves the app from the registry, runs the (injected) judge, derives the
//! satisfaction fraction from the per-scenario booleans (ADR-0008), writes the
//! evidence bundle under the factory root (ADR-0006), and updates the registry's
//! `last_satisfaction` + `last_run_id`/`last_run_at` (ADR-0003). It records no
//! terminal state — that is `run`'s exclusive output.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::clock::RunStamp;
use crate::evidence::ValidationBundle;
use crate::judge::{Judge, JudgeRequest};
use crate::paths::Paths;
use crate::registry::Registry;

/// What `validate` produced, for surfacing to the caller.
pub struct ValidateOutcome {
    pub satisfaction: u8,
    pub satisfied_count: usize,
    pub total_count: usize,
    pub bundle_dir: PathBuf,
}

/// Run the judge and build the validation bundle — no registry update, no file
/// write. Shared by the `validate` command and by `run` (which wraps the result in a
/// run bundle). Satisfaction is derived from the per-scenario booleans (ADR-0008).
pub fn evaluate(
    app: &str,
    code_root: &Path,
    factory_root: &Path,
    judge: &dyn Judge,
    run_id: &str,
) -> Result<ValidationBundle> {
    let request = JudgeRequest {
        app: app.to_string(),
        code_root: code_root.to_path_buf(),
        factory_root: factory_root.to_path_buf(),
    };
    let verdict = judge.judge(&request)?;
    Ok(ValidationBundle::from_verdict(app, run_id, &verdict))
}

pub fn validate(
    paths: &Paths,
    app: &str,
    judge: &dyn Judge,
    stamp: &RunStamp,
) -> Result<ValidateOutcome> {
    let registry_path = paths.registry_path();
    let mut registry = Registry::load(&registry_path)?;
    let entry = match registry.apps.get(app) {
        Some(entry) => entry.clone(),
        None => bail!("app '{app}' is not registered; run `factory init {app}` first"),
    };

    let bundle = evaluate(app, &entry.code_root, &entry.factory_root, judge, &stamp.id)?;
    let bundle_dir = bundle.write(&entry.factory_root)?;

    // Persist the satisfaction value where `run` can read it (ADR-0003). No terminal
    // state — that belongs to `run`.
    let entry = registry.apps.get_mut(app).expect("entry existed above");
    entry.last_satisfaction = Some(bundle.satisfaction);
    entry.last_run_id = Some(stamp.id.clone());
    entry.last_run_at = Some(stamp.at.clone());
    registry.save(&registry_path)?;

    Ok(ValidateOutcome {
        satisfaction: bundle.satisfaction,
        satisfied_count: bundle.satisfied_count,
        total_count: bundle.total_count,
        bundle_dir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judge::{ScenarioOutcome, Verdict};
    use crate::registry::{AppEntry, Mode};

    /// A judge double (ADR-0001) that returns a fixed verdict, so the test exercises
    /// validate's plumbing, not judgment.
    struct FakeJudge {
        verdict: Verdict,
    }

    impl Judge for FakeJudge {
        fn judge(&self, _request: &JudgeRequest) -> Result<Verdict> {
            Ok(self.verdict.clone())
        }
    }

    fn register(paths: &Paths, app: &str) {
        let mut registry = Registry::default();
        registry.upsert(
            app,
            AppEntry {
                code_root: paths.code_root(app),
                factory_root: paths.factory_root(app),
                mode: Mode::Greenfield,
                last_satisfaction: None,
                last_terminal_state: None,
                last_run_id: None,
                last_run_at: None,
            },
        );
        registry.save(&paths.registry_path()).unwrap();
    }

    fn stamp() -> RunStamp {
        RunStamp {
            id: "run-1".into(),
            at: "2026-06-07T00:00:00Z".into(),
        }
    }

    fn mixed_judge() -> FakeJudge {
        FakeJudge {
            verdict: Verdict {
                scenarios: vec![
                    ScenarioOutcome {
                        id: "S001".into(),
                        satisfied: true,
                        observed: "right answer".into(),
                        expected: None,
                    },
                    ScenarioOutcome {
                        id: "S002".into(),
                        satisfied: false,
                        observed: "wrong answer".into(),
                        expected: Some("42".into()),
                    },
                ],
            },
        }
    }

    #[test]
    fn should_write_bundle_and_update_registry_satisfaction() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path().join("home"), dir.path().join("work"));
        register(&paths, "demo");

        let outcome = validate(&paths, "demo", &mixed_judge(), &stamp()).unwrap();

        assert_eq!(outcome.satisfaction, 50);
        assert!(outcome.bundle_dir.join("bundle.json").is_file());

        let registry = Registry::load(&paths.registry_path()).unwrap();
        let entry = &registry.apps["demo"];
        assert_eq!(entry.last_satisfaction, Some(50));
        assert_eq!(entry.last_run_id.as_deref(), Some("run-1"));
        assert_eq!(entry.last_run_at.as_deref(), Some("2026-06-07T00:00:00Z"));
        // validate must not touch the terminal state.
        assert_eq!(entry.last_terminal_state, None);
    }

    #[test]
    fn should_report_satisfied_and_unsatisfied_differently_in_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path().join("home"), dir.path().join("work"));
        register(&paths, "demo");

        let outcome = validate(&paths, "demo", &mixed_judge(), &stamp()).unwrap();

        let json = std::fs::read_to_string(outcome.bundle_dir.join("bundle.json")).unwrap();
        let bundle: ValidationBundle = serde_json::from_str(&json).unwrap();
        let satisfied: Vec<_> = bundle.scenarios.iter().filter(|s| s.satisfied).collect();
        let unsatisfied: Vec<_> = bundle.scenarios.iter().filter(|s| !s.satisfied).collect();
        assert_eq!(satisfied.len(), 1);
        assert_eq!(unsatisfied.len(), 1);
        assert_ne!(satisfied[0].observed, unsatisfied[0].observed);
    }

    #[test]
    fn should_error_when_app_is_not_registered() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path().join("home"), dir.path().join("work"));

        let result = validate(&paths, "ghost", &mixed_judge(), &stamp());

        assert!(result.is_err());
    }

    /// The S002 plumbing claim, exercised through the *real* scripted judge (the
    /// runtime provider S002 uses), not a double: two verdicts (one satisfied, one
    /// not) → exactly 50%, derived from the booleans even when the verdict file
    /// lies about its own totals (ADR-0008).
    #[test]
    fn should_derive_50_percent_through_the_scripted_judge_ignoring_self_report() {
        use crate::judge::scripted::ScriptedJudge;

        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path().join("home"), dir.path().join("work"));
        register(&paths, "demo");
        // A verdict file that lies: claims 2/2 satisfied and 100%, but only one
        // scenario is actually satisfied.
        let verdict = dir.path().join("verdict.json");
        std::fs::write(
            &verdict,
            r#"{
                "scenarios": [
                    {"id": "a", "satisfied": true},
                    {"id": "b", "satisfied": false}
                ],
                "satisfied_count": 2,
                "satisfaction": 100
            }"#,
        )
        .unwrap();
        let judge = ScriptedJudge::new(verdict);

        let outcome = validate(&paths, "demo", &judge, &stamp()).unwrap();

        assert_eq!(
            outcome.satisfaction, 50,
            "derived from booleans, not self-report"
        );
        assert_eq!(outcome.satisfied_count, 1);
        assert_eq!(outcome.total_count, 2);
        let entry = &Registry::load(&paths.registry_path()).unwrap().apps["demo"];
        assert_eq!(entry.last_satisfaction, Some(50));
        assert!(entry.last_run_at.is_some());
        assert_eq!(entry.last_terminal_state, None);
    }
}
