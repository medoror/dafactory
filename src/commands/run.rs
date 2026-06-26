//! `factory run <app> --once` (B3): one outer-loop pass (SPEC).
//!
//! Select the next backlog intent → delegate to the agent (in the code root, seams
//! scrubbed) → observe the change via git → validate → emit exactly one terminal
//! state with an evidence bundle, committing only on `PR_READY` (ADR-0009).
//!
//! B3 is the happy path; the full NO_OP semantics are B4 and the snooping boundary is
//! B6. The terminal-state branches are established here: no change / no intent →
//! `NO_OP`; change + 100% → `PR_READY` (commit); change + <100% → `ESCALATE`;
//! machinery failure (agent can't run, no verdict) → `RETRYABLE`.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::agent::{Agent, AgentRequest, Intent};
use crate::backlog;
use crate::clock::RunStamp;
use crate::commands::validate::evaluate;
use crate::evidence::{ChangeRecord, IntentRecord, RunBundle, TerminalState};
use crate::git;
use crate::judge::Judge;
use crate::paths::Paths;
use crate::registry::Registry;

pub struct RunOutcome {
    pub terminal_state: TerminalState,
    pub bundle_dir: PathBuf,
    pub satisfaction: Option<u8>,
    pub intent: Option<Intent>,
    pub summary: String,
    pub residual: String,
}

/// Map an observed agent effect + validation to a terminal state, for the path where
/// an intent existed, the agent ran, and the judge produced a verdict (ADR-0010).
/// `PR_READY` requires `== 100` exactly; `<100` is `ESCALATE` whether or not the agent
/// changed anything. Machinery failures (`RETRYABLE`) and the no-intent case are
/// handled by `run`'s control flow.
pub fn decide(changed: bool, satisfaction: u8) -> TerminalState {
    match (changed, satisfaction) {
        (_, s) if s < 100 => TerminalState::Escalate,
        (true, _) => TerminalState::PrReady,
        (false, _) => TerminalState::NoOp,
    }
}

pub fn run(
    paths: &Paths,
    app: &str,
    agent: &dyn Agent,
    judge: &dyn Judge,
    stamp: &RunStamp,
) -> Result<RunOutcome> {
    // The run id names the evidence bundle; the readable time goes to the registry.
    let run_id = stamp.id.as_str();
    let registry_path = paths.registry_path();
    let mut registry = Registry::load(&registry_path)?;
    let entry = match registry.apps.get(app) {
        Some(entry) => entry.clone(),
        None => bail!("app '{app}' is not registered; run `factory init {app}` first"),
    };
    let code_root = entry.code_root.clone();
    let factory_root = entry.factory_root.clone();

    // Holdout guard (ADR-0011, defense-in-depth): refuse to run over a holdout that
    // sits inside the code tree, where the agent (cwd = code_root) could read it.
    // Both paths are canonicalized first so a symlink (e.g. macOS /var→/private/var)
    // or a relative path cannot hide a real nesting. A nested or unverifiable holdout
    // is structural, not transient → ESCALATE (a retry re-trips it identically).
    match holdout_inside_code(&code_root, &factory_root) {
        Ok(false) => {}
        Ok(true) => {
            let bundle = escalate(
                app,
                run_id,
                "the holdout factory root is inside the code root — the implementer could read the scenarios",
            );
            return finish(
                &mut registry,
                &registry_path,
                &factory_root,
                bundle,
                stamp,
                None,
            );
        }
        Err(err) => {
            let bundle = escalate(
                app,
                run_id,
                &format!("cannot verify the holdout boundary: {err}"),
            );
            return finish(
                &mut registry,
                &registry_path,
                &factory_root,
                bundle,
                stamp,
                None,
            );
        }
    }

    // `run` requires the repo `init` created, and a clean baseline so the agent's
    // effect is attributable. Either being absent is a machinery problem → RETRYABLE.
    if !git::is_repo(&code_root) {
        let bundle = machinery(
            app,
            run_id,
            None,
            None,
            "code root is not a git repo (was it `init`ed?)",
        );
        return finish(
            &mut registry,
            &registry_path,
            &factory_root,
            bundle,
            stamp,
            None,
        );
    }
    if !git::is_clean(&code_root)? {
        let bundle = base(
            app,
            run_id,
            TerminalState::Retryable,
            None,
            String::new(),
            "working tree was not clean before the run — factory needs a clean baseline \
             to attribute the agent's change",
            "Commit, stash (`git stash`), or discard (`git reset --hard`) the changes in \
             the code root, then run again.",
        );
        return finish(
            &mut registry,
            &registry_path,
            &factory_root,
            bundle,
            stamp,
            None,
        );
    }

    // Step 2: select the next unaddressed intent (None → nothing queued). When there
    // is one, delegate to the agent and observe its effect via git (ADR-0009); a
    // failure to run is machinery → RETRYABLE.
    let backlog_text = std::fs::read_to_string(code_root.join("BACKLOG.md")).unwrap_or_default();
    let intent = backlog::next_intent(&backlog_text);
    let (changed, diff, agent_log) = match &intent {
        Some(intent) => {
            eprintln!("factory: intent → {}", intent_label(intent));
            let baseline = git::head(&code_root)?;
            let request = AgentRequest {
                app: app.to_string(),
                code_root: code_root.clone(),
                intent: intent.clone(),
            };
            eprintln!("factory: running agent...");
            let log = match agent.implement(&request) {
                Ok(outcome) => outcome.log,
                Err(err) => {
                    let bundle = machinery(
                        app,
                        run_id,
                        Some(intent),
                        None,
                        &format!("agent failed to run: {err:#}"),
                    );
                    return finish(
                        &mut registry,
                        &registry_path,
                        &factory_root,
                        bundle,
                        stamp,
                        None,
                    );
                }
            };
            // The agent contract is an *uncommitted* working-tree change (ADR-0009).
            // Some agents (Claude Code by default) self-commit; if HEAD moved, rewind to
            // the baseline so the agent's work is observed and committed by factory —
            // otherwise `add_all` sees an empty tree and the evidence bundle would miss
            // the real change (dogfooding finding from screener B1).
            if git::head(&code_root)? != baseline {
                git::reset_soft(&code_root, &baseline)?;
            }
            git::add_all(&code_root)?;
            let diff = git::diff_cached(&code_root)?;
            (!diff.trim().is_empty(), diff, Some(log))
        }
        None => {
            eprintln!("factory: no open intent — validating");
            (false, String::new(), None)
        }
    };

    // Step 3: validate on every pass — even no-change ones. NO_OP is earned by a
    // passing validation, never assumed from a clean tree (ADR-0010). No verdict is
    // machinery (RETRYABLE), NOT a sub-100 satisfaction.
    eprintln!("factory: validating...");
    let validation = match evaluate(app, &code_root, &factory_root, judge, run_id) {
        Ok(validation) => validation,
        Err(err) => {
            let bundle = machinery(
                app,
                run_id,
                intent.as_ref(),
                Some(diff),
                &format!("validation produced no verdict: {err:#}"),
            );
            return finish(
                &mut registry,
                &registry_path,
                &factory_root,
                bundle,
                stamp,
                None,
            );
        }
    };
    let satisfaction = validation.satisfaction;

    // Step 4: terminal state. With no open intent the loop has nothing to pull → NO_OP
    // (the reason distinguishes completion from a quiet alarm). Otherwise the matrix.
    // (Future: honor an agent-emitted terminal tag here; v0 has no such channel, so
    // ESCALATE is run-originated — ADR-0010.)
    let terminal = match &intent {
        None => TerminalState::NoOp,
        Some(_) => decide(changed, satisfaction),
    };

    // Step 5: commit only on PR_READY; the bundle's diff is exactly the committed one.
    let change = if terminal == TerminalState::PrReady {
        let hash = git::commit(
            &code_root,
            &commit_message(intent.as_ref().expect("PR_READY implies an intent"), app),
        )?;
        ChangeRecord {
            committed: true,
            commit: Some(hash),
            diff,
        }
    } else {
        ChangeRecord {
            committed: false,
            commit: None,
            diff,
        }
    };

    let (summary, residual) = describe(terminal, intent.as_ref(), changed, &validation);
    let bundle = RunBundle {
        kind: "run".to_string(),
        app: app.to_string(),
        run_id: run_id.to_string(),
        terminal_state: terminal,
        summary,
        intent: intent.as_ref().map(intent_record),
        agent_log,
        validation: Some(validation),
        change,
        residual,
    };
    finish(
        &mut registry,
        &registry_path,
        &factory_root,
        bundle,
        stamp,
        Some(satisfaction),
    )
}

pub struct LoopOutcome {
    pub last_terminal_state: TerminalState,
    pub passes_completed: u32,
    pub satisfaction: Option<u8>,
    pub last_bundle_dir: PathBuf,
    pub summary: String,
    pub residual: String,
}

pub fn run_loop(
    paths: &Paths,
    app: &str,
    agent: &dyn Agent,
    judge: &dyn Judge,
    max_iters: u32,
    retries: u32,
) -> Result<LoopOutcome> {
    let code_root = {
        let registry = Registry::load(&paths.registry_path())?;
        registry
            .apps
            .get(app)
            .ok_or_else(|| {
                anyhow::anyhow!("app '{app}' is not registered; run `factory init {app}` first")
            })?
            .code_root
            .clone()
    };

    if max_iters == 0 {
        bail!("max_iters must be at least 1");
    }

    let mut retries_left = retries;
    let mut passes_completed: u32 = 0;
    let mut last_outcome: Option<RunOutcome> = None;

    for _pass in 1..=max_iters {
        eprintln!("factory: pass {}/{max_iters}", passes_completed + 1);
        let stamp = RunStamp::now()?;
        let outcome = run(paths, app, agent, judge, &stamp)?;
        passes_completed += 1;

        let stop = match outcome.terminal_state {
            TerminalState::PrReady => {
                retries_left = retries; // reset for the next intent
                if let Some(ref intent) = outcome.intent {
                    tick_and_commit(&code_root, intent)
                        .with_context(|| format!("tick failed after pass {passes_completed}"))?;
                }
                false
            }
            TerminalState::Retryable if retries_left > 0 => {
                retries_left -= 1;
                eprintln!("factory: RETRYABLE — retrying ({retries_left} remaining)...");
                false
            }
            _ => true,
        };

        last_outcome = Some(outcome);
        if stop {
            break;
        }
    }

    let outcome = last_outcome.expect("max_iters >= 1 ensures at least one pass ran");
    Ok(LoopOutcome {
        last_terminal_state: outcome.terminal_state,
        passes_completed,
        satisfaction: outcome.satisfaction,
        last_bundle_dir: outcome.bundle_dir,
        summary: outcome.summary,
        residual: outcome.residual,
    })
}

fn tick_and_commit(code_root: &std::path::Path, intent: &Intent) -> Result<()> {
    let backlog_path = code_root.join("BACKLOG.md");
    let text = std::fs::read_to_string(&backlog_path)
        .with_context(|| format!("failed to read BACKLOG.md at {}", backlog_path.display()))?;
    let ticked = backlog::tick_intent(&text, &intent.raw);
    std::fs::write(&backlog_path, ticked)
        .with_context(|| format!("failed to write BACKLOG.md at {}", backlog_path.display()))?;
    git::add_all(code_root)?;
    let msg = match &intent.id {
        Some(id) => format!("Advance backlog: tick {id}"),
        None => "Advance backlog: tick intent".to_string(),
    };
    git::commit(code_root, &msg)?;
    Ok(())
}

/// The human-readable summary + residual for a validated terminal state, written so
/// the bundle is distinguishable in the evidence trail beyond the state field alone
/// (ADR-0006, ADR-0010).
fn describe(
    terminal: TerminalState,
    intent: Option<&Intent>,
    changed: bool,
    validation: &crate::evidence::ValidationBundle,
) -> (String, String) {
    let fraction = format!(
        "{}/{} scenarios satisfied ({}%)",
        validation.satisfied_count, validation.total_count, validation.satisfaction
    );
    match (terminal, intent) {
        (TerminalState::PrReady, Some(intent)) => (
            format!("PR_READY: {fraction} for {}", intent_label(intent)),
            "Change committed; held-out validation passed.".to_string(),
        ),
        (TerminalState::Escalate, _) => {
            let what = if changed {
                "a change was made but validation did not pass"
            } else {
                "the agent made no change and the app does not pass"
            };
            (
                format!("ESCALATE: {what} — {fraction}"),
                "Left uncommitted; the result is not passing and a retry will not help — human attention needed.".to_string(),
            )
        }
        (TerminalState::NoOp, Some(intent)) => (
            format!("NO_OP: no change made; scenarios already satisfied — {fraction} for {}", intent_label(intent)),
            "No change was the correct outcome; the intent was already satisfied.".to_string(),
        ),
        (TerminalState::NoOp, None) if validation.satisfaction == 100 => (
            format!("NO_OP: BACKLOG COMPLETE — all {fraction}"),
            "Completion signal: no unaddressed intent and validation passes. The project's success condition is met.".to_string(),
        ),
        (TerminalState::NoOp, None) => (
            format!("NO_OP: backlog exhausted but only {fraction}"),
            "Quiet alarm: no unaddressed intent yet validation is below 100% — incomplete backlog or a regression; add intents/scenarios or investigate.".to_string(),
        ),
        // PR_READY without an intent and RETRYABLE never reach here.
        (other, _) => (format!("{other}: {fraction}"), String::new()),
    }
}

/// Write the bundle, update the registry status fields, and surface the outcome.
fn finish(
    registry: &mut Registry,
    registry_path: &Path,
    factory_root: &Path,
    bundle: RunBundle,
    stamp: &RunStamp,
    satisfaction: Option<u8>,
) -> Result<RunOutcome> {
    let bundle_dir = bundle.write(factory_root)?;
    if let Some(entry) = registry
        .apps
        .values_mut()
        .find(|e| e.factory_root == factory_root)
    {
        entry.last_terminal_state = Some(bundle.terminal_state.to_string());
        entry.last_run_id = Some(stamp.id.clone());
        entry.last_run_at = Some(stamp.at.clone());
        if let Some(value) = satisfaction {
            entry.last_satisfaction = Some(value);
        }
    }
    registry.save(registry_path)?;
    Ok(RunOutcome {
        terminal_state: bundle.terminal_state,
        bundle_dir,
        satisfaction,
        intent: bundle.intent.as_ref().map(|ir| Intent {
            id: ir.id.clone(),
            title: ir.title.clone(),
            raw: ir.raw.clone(),
        }),
        summary: bundle.summary.clone(),
        residual: bundle.residual.clone(),
    })
}

fn intent_record(intent: &Intent) -> IntentRecord {
    IntentRecord {
        id: intent.id.clone(),
        title: intent.title.clone(),
        raw: intent.raw.clone(),
    }
}

fn intent_label(intent: &Intent) -> String {
    match &intent.id {
        Some(id) => format!("{id} ({})", intent.title),
        None => intent.title.clone(),
    }
}

/// A commit message that puts the originating intent — and the scenario it cites — in
/// the git history, so the history is itself part of the evidence trail.
fn commit_message(intent: &Intent, app: &str) -> String {
    let subject = match &intent.id {
        Some(id) => format!("{id}: {}", intent.title),
        None => intent.title.clone(),
    };
    format!(
        "{subject}\n\nImplemented by `factory run` for `{app}`.\nIntent: {}\n",
        intent.raw
    )
}

/// Is the factory (holdout) root inside the code root? Both paths are canonicalized
/// first (ADR-0011) so symlinks and relative/absolute differences cannot mask a real
/// nesting. Errors if either path can't be resolved (missing/unreadable); the caller
/// fails closed.
fn holdout_inside_code(code_root: &Path, factory_root: &Path) -> std::io::Result<bool> {
    let code = code_root.canonicalize()?;
    let factory = factory_root.canonicalize()?;
    Ok(factory.starts_with(&code))
}

/// An `ESCALATE` bundle for a structural problem `run` originates itself (no agent
/// tag, no validation) — e.g. a holdout-boundary violation (ADR-0010, ADR-0011).
fn escalate(app: &str, run_id: &str, reason: &str) -> RunBundle {
    base(
        app,
        run_id,
        TerminalState::Escalate,
        None,
        String::new(),
        reason,
        "Structural problem; a retry re-trips it identically — a human must fix the layout.",
    )
}

/// A `RETRYABLE` bundle for a machinery failure (no validation reached).
fn machinery(
    app: &str,
    run_id: &str,
    intent: Option<&Intent>,
    diff: Option<String>,
    reason: &str,
) -> RunBundle {
    base(
        app,
        run_id,
        TerminalState::Retryable,
        intent,
        diff.unwrap_or_default(),
        reason,
        "Machinery failed; a retry may resolve it if the cause is transient.",
    )
}

#[allow(clippy::too_many_arguments)]
fn base(
    app: &str,
    run_id: &str,
    terminal: TerminalState,
    intent: Option<&Intent>,
    diff: String,
    summary: &str,
    residual: &str,
) -> RunBundle {
    RunBundle {
        kind: "run".to_string(),
        app: app.to_string(),
        run_id: run_id.to_string(),
        terminal_state: terminal,
        summary: summary.to_string(),
        intent: intent.map(intent_record),
        agent_log: None,
        validation: None,
        change: ChangeRecord {
            committed: false,
            commit: None,
            diff,
        },
        residual: residual.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentOutcome;
    use crate::commands::init;
    use crate::judge::{ScenarioOutcome, Verdict};
    use crate::registry::Mode;

    /// An agent double whose effect on the code root is a closure (ADR-0009: the
    /// agent's effect is the working tree, not its stdout).
    struct FakeAgent<F: Fn(&Path)> {
        effect: F,
    }
    impl<F: Fn(&Path)> Agent for FakeAgent<F> {
        fn implement(&self, request: &AgentRequest) -> Result<AgentOutcome> {
            (self.effect)(&request.code_root);
            Ok(AgentOutcome {
                log: "did the thing".to_string(),
            })
        }
    }

    struct FakeJudge {
        verdict: Verdict,
    }
    impl Judge for FakeJudge {
        fn judge(&self, _request: &crate::judge::JudgeRequest) -> Result<Verdict> {
            Ok(self.verdict.clone())
        }
    }

    fn all_satisfied() -> FakeJudge {
        FakeJudge {
            verdict: Verdict {
                scenarios: vec![ScenarioOutcome {
                    id: "S003".into(),
                    satisfied: true,
                    observed: "drove it, passed".into(),
                    expected: None,
                }],
            },
        }
    }

    fn one_failing() -> FakeJudge {
        FakeJudge {
            verdict: Verdict {
                scenarios: vec![
                    ScenarioOutcome {
                        id: "S003".into(),
                        satisfied: true,
                        observed: "ok".into(),
                        expected: None,
                    },
                    ScenarioOutcome {
                        id: "S004".into(),
                        satisfied: false,
                        observed: "nope".into(),
                        expected: None,
                    },
                ],
            },
        }
    }

    /// init a sandbox app and give its code root a backlog with one open intent.
    fn sandbox(dir: &Path) -> Paths {
        let paths = Paths::new(dir.join("home"), dir.join("work"));
        init::init(&paths, "demo", Mode::Greenfield).unwrap();
        let backlog = paths.code_root("demo").join("BACKLOG.md");
        std::fs::write(&backlog, "- [ ] **B3 (→ S003) — do the thing.**\n").unwrap();
        // Commit so the baseline is clean before the run.
        git::add_all(&paths.code_root("demo")).unwrap();
        git::commit(&paths.code_root("demo"), "Set backlog").unwrap();
        paths
    }

    fn stamp() -> RunStamp {
        RunStamp {
            id: "run-1".into(),
            at: "2026-06-07T00:00:00Z".into(),
        }
    }

    #[test]
    fn should_decide_terminal_state_from_change_and_satisfaction() {
        // 100% earns NO_OP (no change) or PR_READY (change); anything <100% escalates.
        assert_eq!(decide(false, 100), TerminalState::NoOp);
        assert_eq!(decide(true, 100), TerminalState::PrReady);
        assert_eq!(decide(false, 99), TerminalState::Escalate);
        assert_eq!(decide(true, 99), TerminalState::Escalate);
        assert_eq!(decide(false, 0), TerminalState::Escalate);
        assert_eq!(decide(true, 0), TerminalState::Escalate);
    }

    #[test]
    fn should_emit_pr_ready_and_commit_on_passing_change() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        let agent = FakeAgent {
            effect: |root: &Path| std::fs::write(root.join("feature.txt"), "implemented").unwrap(),
        };

        let outcome = run(&paths, "demo", &agent, &all_satisfied(), &stamp()).unwrap();

        assert_eq!(outcome.terminal_state, TerminalState::PrReady);
        assert_eq!(outcome.satisfaction, Some(100));
        // The change is committed and the tree is clean again.
        assert!(git::is_clean(&paths.code_root("demo")).unwrap());
        // The bundle records the committed diff and the commit hash.
        let json = std::fs::read_to_string(outcome.bundle_dir.join("bundle.json")).unwrap();
        let bundle: RunBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(bundle.terminal_state, TerminalState::PrReady);
        assert!(bundle.change.committed);
        assert!(bundle.change.commit.is_some());
        assert!(bundle.change.diff.contains("feature.txt"));
        assert_eq!(bundle.intent.unwrap().id.as_deref(), Some("B3"));
        // Registry status updated.
        let registry = Registry::load(&paths.registry_path()).unwrap();
        let entry = &registry.apps["demo"];
        assert_eq!(entry.last_terminal_state.as_deref(), Some("PR_READY"));
        assert_eq!(entry.last_satisfaction, Some(100));
    }

    #[test]
    fn should_escalate_and_not_commit_when_validation_does_not_pass() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        let agent = FakeAgent {
            effect: |root: &Path| std::fs::write(root.join("feature.txt"), "partial").unwrap(),
        };

        let outcome = run(&paths, "demo", &agent, &one_failing(), &stamp()).unwrap();

        assert_eq!(outcome.terminal_state, TerminalState::Escalate);
        // The agent's change is left uncommitted (still in the tree, but not clean).
        assert!(!git::is_clean(&paths.code_root("demo")).unwrap());
        let json = std::fs::read_to_string(outcome.bundle_dir.join("bundle.json")).unwrap();
        let bundle: RunBundle = serde_json::from_str(&json).unwrap();
        assert!(!bundle.change.committed);
        assert!(bundle.change.diff.contains("feature.txt"));
    }

    #[test]
    fn should_emit_no_op_when_no_change_and_already_satisfied() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        let agent = FakeAgent {
            effect: |_root: &Path| {},
        };

        let outcome = run(&paths, "demo", &agent, &all_satisfied(), &stamp()).unwrap();

        // NO_OP is earned by a passing validation, not assumed from a clean tree.
        assert_eq!(outcome.terminal_state, TerminalState::NoOp);
        assert_eq!(outcome.satisfaction, Some(100));
        let registry = Registry::load(&paths.registry_path()).unwrap();
        let entry = &registry.apps["demo"];
        assert_eq!(entry.last_terminal_state.as_deref(), Some("NO_OP"));
        assert_eq!(entry.last_satisfaction, Some(100));
        let json = std::fs::read_to_string(outcome.bundle_dir.join("bundle.json")).unwrap();
        let bundle: RunBundle = serde_json::from_str(&json).unwrap();
        assert!(bundle.validation.is_some());
        assert!(bundle.summary.contains("already satisfied"));
    }

    #[test]
    fn should_escalate_when_no_change_and_not_satisfied() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        // The agent does nothing, but the scenarios do not pass — no change was NOT
        // the correct outcome, so this must not be a clean NO_OP.
        let agent = FakeAgent {
            effect: |_root: &Path| {},
        };

        let outcome = run(&paths, "demo", &agent, &one_failing(), &stamp()).unwrap();

        assert_eq!(outcome.terminal_state, TerminalState::Escalate);
        let registry = Registry::load(&paths.registry_path()).unwrap();
        assert_eq!(
            registry.apps["demo"].last_terminal_state.as_deref(),
            Some("ESCALATE")
        );
    }

    #[test]
    fn should_no_op_as_completion_signal_when_backlog_exhausted_and_satisfied() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        // Close out the backlog (every item checked).
        let backlog = paths.code_root("demo").join("BACKLOG.md");
        std::fs::write(&backlog, "- [x] **B3 (→ S003) — done.**\n").unwrap();
        git::add_all(&paths.code_root("demo")).unwrap();
        git::commit(&paths.code_root("demo"), "Close backlog").unwrap();
        // An agent that would leave a trace if invoked — it must NOT be, since there
        // is no intent to pull.
        let agent = FakeAgent {
            effect: |root: &Path| std::fs::write(root.join("should-not-exist.txt"), "x").unwrap(),
        };

        let outcome = run(&paths, "demo", &agent, &all_satisfied(), &stamp()).unwrap();

        assert_eq!(outcome.terminal_state, TerminalState::NoOp);
        assert!(!paths
            .code_root("demo")
            .join("should-not-exist.txt")
            .exists());
        let json = std::fs::read_to_string(outcome.bundle_dir.join("bundle.json")).unwrap();
        let bundle: RunBundle = serde_json::from_str(&json).unwrap();
        assert!(bundle.intent.is_none());
        assert!(bundle.summary.contains("BACKLOG COMPLETE"));
        assert_eq!(outcome.satisfaction, Some(100));
    }

    #[test]
    fn should_no_op_as_quiet_alarm_when_backlog_exhausted_but_failing() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        let backlog = paths.code_root("demo").join("BACKLOG.md");
        std::fs::write(&backlog, "- [x] **B3 (→ S003) — done.**\n").unwrap();
        git::add_all(&paths.code_root("demo")).unwrap();
        git::commit(&paths.code_root("demo"), "Close backlog").unwrap();
        let agent = FakeAgent {
            effect: |_root: &Path| {},
        };

        let outcome = run(&paths, "demo", &agent, &one_failing(), &stamp()).unwrap();

        // Backlog-exhausted is still NO_OP, but the bundle flags the regression.
        assert_eq!(outcome.terminal_state, TerminalState::NoOp);
        let json = std::fs::read_to_string(outcome.bundle_dir.join("bundle.json")).unwrap();
        let bundle: RunBundle = serde_json::from_str(&json).unwrap();
        assert!(bundle.residual.contains("Quiet alarm"));
        assert_eq!(outcome.satisfaction, Some(50));
    }

    #[test]
    fn should_be_retryable_when_code_root_is_not_a_repo() {
        let dir = tempfile::tempdir().unwrap();
        // Register an app whose code root is not a git repo.
        let paths = Paths::new(dir.path().join("home"), dir.path().join("work"));
        let mut registry = Registry::default();
        registry.upsert(
            "demo",
            crate::registry::AppEntry {
                code_root: dir.path().join("not-a-repo"),
                factory_root: dir.path().join("factory"),
                mode: Mode::Greenfield,
                last_satisfaction: None,
                last_terminal_state: None,
                last_run_id: None,
                last_run_at: None,
            },
        );
        registry.save(&paths.registry_path()).unwrap();
        // Both roots must exist so the holdout guard can canonicalize them; the point
        // under test is the missing .git, not the boundary.
        std::fs::create_dir_all(dir.path().join("not-a-repo")).unwrap();
        std::fs::create_dir_all(dir.path().join("factory")).unwrap();
        let agent = FakeAgent {
            effect: |_root: &Path| {},
        };

        let outcome = run(&paths, "demo", &agent, &all_satisfied(), &stamp()).unwrap();

        assert_eq!(outcome.terminal_state, TerminalState::Retryable);
    }

    #[test]
    fn should_escalate_when_holdout_is_inside_the_code_root() {
        let dir = tempfile::tempdir().unwrap();
        // A misconfigured registry whose factory root is nested in the code tree —
        // the agent (cwd = code_root) could read the scenarios.
        let paths = Paths::new(dir.path().join("home"), dir.path().join("work"));
        let code_root = dir.path().join("work/demo");
        let nested_factory = code_root.join(".factory");
        std::fs::create_dir_all(&nested_factory).unwrap();
        let mut registry = Registry::default();
        registry.upsert(
            "demo",
            crate::registry::AppEntry {
                code_root: code_root.clone(),
                factory_root: nested_factory,
                mode: Mode::Greenfield,
                last_satisfaction: None,
                last_terminal_state: None,
                last_run_id: None,
                last_run_at: None,
            },
        );
        registry.save(&paths.registry_path()).unwrap();
        let agent = FakeAgent {
            effect: |root: &Path| std::fs::write(root.join("x.txt"), "x").unwrap(),
        };

        let outcome = run(&paths, "demo", &agent, &all_satisfied(), &stamp()).unwrap();

        // Structural misconfig → ESCALATE (never RETRYABLE), and the agent never ran.
        assert_eq!(outcome.terminal_state, TerminalState::Escalate);
        assert!(!code_root.join("x.txt").exists());
    }

    #[test]
    fn should_be_retryable_when_agent_cannot_run() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        struct FailingAgent;
        impl Agent for FailingAgent {
            fn implement(&self, _request: &AgentRequest) -> Result<AgentOutcome> {
                bail!("boom")
            }
        }

        let outcome = run(&paths, "demo", &FailingAgent, &all_satisfied(), &stamp()).unwrap();

        assert_eq!(outcome.terminal_state, TerminalState::Retryable);
    }

    #[test]
    fn should_capture_the_real_diff_even_when_the_agent_self_commits() {
        // ADR-0009 assumes the agent leaves an uncommitted change. Claude self-commits
        // by default; when it does, factory must still observe and record the real diff
        // (not the empty post-commit tree) so the evidence bundle is honest.
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        let agent = FakeAgent {
            effect: |root: &Path| {
                std::fs::write(root.join("feature.txt"), "implemented").unwrap();
                // The agent commits its own work, moving HEAD past factory's baseline.
                git::add_all(root).unwrap();
                git::commit(root, "agent self-commit").unwrap();
            },
        };

        let outcome = run(&paths, "demo", &agent, &all_satisfied(), &stamp()).unwrap();

        assert_eq!(outcome.terminal_state, TerminalState::PrReady);
        let json = std::fs::read_to_string(outcome.bundle_dir.join("bundle.json")).unwrap();
        let bundle: RunBundle = serde_json::from_str(&json).unwrap();
        assert!(bundle.change.committed);
        assert!(
            bundle.change.diff.contains("feature.txt"),
            "factory must capture the agent's real change, not the post-commit empty diff"
        );
        // Factory's own commit (citing the intent) is the tip; the tree is clean.
        assert!(git::is_clean(&paths.code_root("demo")).unwrap());
    }

    #[test]
    fn should_give_actionable_guidance_when_the_tree_is_not_clean() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        // An uncommitted file (e.g. left by an interrupted run) dirties the baseline.
        std::fs::write(paths.code_root("demo").join("dirty.txt"), "x").unwrap();
        let agent = FakeAgent {
            effect: |_root: &Path| {},
        };

        let outcome = run(&paths, "demo", &agent, &all_satisfied(), &stamp()).unwrap();

        assert_eq!(outcome.terminal_state, TerminalState::Retryable);
        let json = std::fs::read_to_string(outcome.bundle_dir.join("bundle.json")).unwrap();
        let bundle: RunBundle = serde_json::from_str(&json).unwrap();
        // The bail must say HOW to recover, not merely that the tree was dirty.
        let guidance = format!("{} {}", bundle.summary, bundle.residual).to_lowercase();
        assert!(
            guidance.contains("stash"),
            "bail should suggest how to recover (stash / commit / reset): {guidance}"
        );
        assert!(
            guidance.contains("run again") || guidance.contains("re-run"),
            "bail should tell the user to run again after cleaning: {guidance}"
        );
    }

    // ─── run_loop tests ──────────────────────────────────────────────────────────

    struct AlwaysFailAgent;
    impl Agent for AlwaysFailAgent {
        fn implement(&self, _request: &AgentRequest) -> Result<AgentOutcome> {
            bail!("boom")
        }
    }

    #[test]
    fn should_run_all_max_iters_passes_when_every_pass_is_pr_ready() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        // Two open intents so both passes find work.
        let backlog_path = paths.code_root("demo").join("BACKLOG.md");
        std::fs::write(
            &backlog_path,
            "- [ ] **B1 (→ S001) — first.**\n- [ ] **B2 (→ S002) — second.**\n",
        )
        .unwrap();
        git::add_all(&paths.code_root("demo")).unwrap();
        git::commit(&paths.code_root("demo"), "Two intents").unwrap();

        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = counter.clone();
        let agent = FakeAgent {
            effect: move |root: &Path| {
                let n = c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                std::fs::write(root.join(format!("feature-{n}.txt")), "x").unwrap();
            },
        };

        let outcome = run_loop(&paths, "demo", &agent, &all_satisfied(), 2, 0).unwrap();

        assert_eq!(outcome.last_terminal_state, TerminalState::PrReady);
        assert_eq!(outcome.passes_completed, 2);
        assert_eq!(
            counter.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "agent should have run exactly twice"
        );
        // Both intents ticked.
        let backlog = std::fs::read_to_string(&backlog_path).unwrap();
        assert!(backlog.contains("- [x] **B1"), "B1 should be ticked");
        assert!(backlog.contains("- [x] **B2"), "B2 should be ticked");
        // Tree clean after the loop.
        assert!(git::is_clean(&paths.code_root("demo")).unwrap());
    }

    #[test]
    fn should_stop_on_escalate_and_return_last_terminal_state() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        // No-op agent + failing judge → ESCALATE.
        let agent = FakeAgent {
            effect: |_root: &Path| {},
        };

        let outcome = run_loop(&paths, "demo", &agent, &one_failing(), 3, 0).unwrap();

        assert_eq!(outcome.last_terminal_state, TerminalState::Escalate);
        assert_eq!(outcome.passes_completed, 1);
    }

    #[test]
    fn should_retry_once_then_stop_on_second_retryable() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        // retries=1: first RETRYABLE is retried, second stops the loop.
        let outcome = run_loop(&paths, "demo", &AlwaysFailAgent, &all_satisfied(), 5, 1).unwrap();

        assert_eq!(outcome.last_terminal_state, TerminalState::Retryable);
        assert_eq!(
            outcome.passes_completed, 2,
            "retry consumes one max-iters slot"
        );
    }

    #[test]
    fn should_stop_immediately_on_retryable_when_retries_is_zero() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());

        let outcome = run_loop(&paths, "demo", &AlwaysFailAgent, &all_satisfied(), 5, 0).unwrap();

        assert_eq!(outcome.last_terminal_state, TerminalState::Retryable);
        assert_eq!(outcome.passes_completed, 1);
    }

    #[test]
    fn should_propagate_summary_and_residual_to_loop_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        // AlwaysFailAgent → RETRYABLE; machinery() sets both summary and residual.
        let outcome = run_loop(&paths, "demo", &AlwaysFailAgent, &all_satisfied(), 1, 0).unwrap();

        assert!(
            !outcome.summary.is_empty(),
            "summary must propagate from RunBundle to LoopOutcome"
        );
        assert!(
            !outcome.residual.is_empty(),
            "residual must propagate from RunBundle to LoopOutcome"
        );
    }

    #[test]
    fn should_stop_on_no_op_when_backlog_is_exhausted() {
        let dir = tempfile::tempdir().unwrap();
        let paths = sandbox(dir.path());
        // Close the backlog so the first pass sees no intent.
        let backlog = paths.code_root("demo").join("BACKLOG.md");
        std::fs::write(&backlog, "- [x] **B3 — done.**\n").unwrap();
        git::add_all(&paths.code_root("demo")).unwrap();
        git::commit(&paths.code_root("demo"), "Close backlog").unwrap();

        let agent = FakeAgent {
            effect: |_root: &Path| {},
        };

        let outcome = run_loop(&paths, "demo", &agent, &all_satisfied(), 5, 0).unwrap();

        assert_eq!(outcome.last_terminal_state, TerminalState::NoOp);
        assert_eq!(outcome.passes_completed, 1);
    }
}
