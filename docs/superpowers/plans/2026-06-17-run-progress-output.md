# Run Progress Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Emit five stage-entry markers to stderr during `factory run --once` so a healthy pass is distinguishable from a hung process.

**Architecture:** `progress_terminal` helper in `main.rs` formats the terminal-state line; four `eprintln!` calls in `src/commands/run.rs` mark the other stages. Agent output stays captured (`.output()`). No threads, no new dependencies.

**Tech Stack:** Rust, `std::process::Command`, `cargo test`

---

## Files

| File | Change |
|------|--------|
| `src/main.rs` | Add `progress_terminal(state, satisfaction)` helper + `eprintln!` in `Run` arm |
| `src/commands/run.rs` | Add four `eprintln!` stage markers |
| `tests/progress_output.rs` | New integration test file |

---

## Task 1: `progress_terminal` helper + terminal-state marker

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write the failing unit tests**

Add to the bottom of `src/main.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::TerminalState;

    #[test]
    fn should_include_satisfaction_in_terminal_marker_when_present() {
        assert_eq!(
            progress_terminal(TerminalState::PrReady, Some(100)),
            "factory: → PR_READY (100%)"
        );
        assert_eq!(
            progress_terminal(TerminalState::Escalate, Some(42)),
            "factory: → ESCALATE (42%)"
        );
        assert_eq!(
            progress_terminal(TerminalState::NoOp, Some(100)),
            "factory: → NO_OP (100%)"
        );
    }

    #[test]
    fn should_omit_satisfaction_in_terminal_marker_when_absent() {
        assert_eq!(
            progress_terminal(TerminalState::Retryable, None),
            "factory: → RETRYABLE"
        );
        assert_eq!(
            progress_terminal(TerminalState::Escalate, None),
            "factory: → ESCALATE"
        );
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test progress_terminal 2>&1 | cat
```

Expected: compile error — `progress_terminal` not found.

- [ ] **Step 3: Implement `progress_terminal` and add the terminal-state `eprintln!`**

Add the helper just before `fn run(cli: Cli)` in `src/main.rs`:

```rust
fn progress_terminal(state: evidence::TerminalState, satisfaction: Option<u8>) -> String {
    match satisfaction {
        Some(pct) => format!("factory: → {state} ({pct}%)"),
        None => format!("factory: → {state}"),
    }
}
```

In the `Commands::Run` arm of `fn run(cli: Cli)`, add one `eprintln!` after `outcome` is available and before the `println!` summary. The Run arm currently reads:

```rust
Commands::Run { app, once, agent, judge } => {
    if !once {
        bail!("v0 supports only `factory run <app> --once`");
    }
    let paths = Paths::resolve()?;
    let agent = build_agent(agent.as_deref())?;
    let judge = build_judge(judge.as_deref())?;
    let stamp = clock::RunStamp::now()?;
    let outcome = commands::run::run(&paths, &app, agent.as_ref(), judge.as_ref(), &stamp)?;
    match outcome.satisfaction {
        Some(value) => println!("Ran '{app}': {} ({value}%)", outcome.terminal_state),
        None => println!("Ran '{app}': {}", outcome.terminal_state),
    }
    println!("  evidence: {}", outcome.bundle_dir.display());
    Ok(ExitCode::from(outcome.terminal_state.exit_code()))
}
```

Change it to:

```rust
Commands::Run { app, once, agent, judge } => {
    if !once {
        bail!("v0 supports only `factory run <app> --once`");
    }
    let paths = Paths::resolve()?;
    let agent = build_agent(agent.as_deref())?;
    let judge = build_judge(judge.as_deref())?;
    let stamp = clock::RunStamp::now()?;
    let outcome = commands::run::run(&paths, &app, agent.as_ref(), judge.as_ref(), &stamp)?;
    eprintln!("{}", progress_terminal(outcome.terminal_state, outcome.satisfaction));
    match outcome.satisfaction {
        Some(value) => println!("Ran '{app}': {} ({value}%)", outcome.terminal_state),
        None => println!("Ran '{app}': {}", outcome.terminal_state),
    }
    println!("  evidence: {}", outcome.bundle_dir.display());
    Ok(ExitCode::from(outcome.terminal_state.exit_code()))
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cargo test progress_terminal 2>&1 | cat
```

Expected: `test tests::should_include_satisfaction_in_terminal_marker_when_present ... ok` and `test tests::should_omit_satisfaction_in_terminal_marker_when_absent ... ok`.

- [ ] **Step 5: Run the full suite to confirm nothing broke**

```bash
cargo test 2>&1 | cat
```

Expected: all tests pass, no new failures.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "Add progress_terminal helper and terminal-state stderr marker"
```

---

## Task 2: Stage markers in `run.rs` (TDD — integration test first)

**Files:**
- Create: `tests/progress_output.rs`
- Modify: `src/commands/run.rs`

- [ ] **Step 1: Write the failing integration tests**

Create `tests/progress_output.rs`:

```rust
//! Factory emits stage-entry markers to stderr during `run` so a healthy pass
//! is distinguishable from a hung process.

use std::path::Path;
use std::process::Command;

fn factory(home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_factory"));
    command
        .env_remove("FACTORY_AGENT")
        .env_remove("FACTORY_AGENT_SCRIPT")
        .env_remove("FACTORY_JUDGE")
        .env_remove("FACTORY_JUDGE_SCRIPT")
        .env("FACTORY_HOME", home);
    command
}

fn init_demo(home: &Path, work: &Path) {
    let ok = factory(home)
        .current_dir(work)
        .args(["init", "demo"])
        .status()
        .unwrap()
        .success();
    assert!(ok, "factory init failed");
}

fn git(code_root: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(code_root)
        .args(args)
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?} failed");
}

fn write_verdict(home: &Path, json: &str) -> std::path::PathBuf {
    let path = home.join("verdict.json");
    std::fs::write(&path, json).unwrap();
    path
}

/// With an open intent: intent → running agent → validating → terminal state, in order.
#[test]
fn should_emit_all_stage_markers_in_order_when_intent_exists() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    init_demo(home.path(), work.path());
    let code_root = work.path().join("demo");
    std::fs::write(code_root.join("BACKLOG.md"), "- [ ] **B1 — do it.**\n").unwrap();
    git(
        &code_root,
        &["-c", "user.name=t", "-c", "user.email=t@e", "commit", "-aqm", "add intent"],
    );
    let verdict = write_verdict(
        home.path(),
        r#"{"scenarios":[{"id":"x","satisfied":false}]}"#,
    );

    let output = factory(home.path())
        .env("FACTORY_AGENT", "scripted")
        .env("FACTORY_AGENT_SCRIPT", "true")
        .env("FACTORY_JUDGE", "scripted")
        .env("FACTORY_JUDGE_SCRIPT", &verdict)
        .current_dir(work.path())
        .args(["run", "demo", "--once"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let intent_pos = stderr
        .find("factory: intent →")
        .unwrap_or_else(|| panic!("intent marker missing from stderr:\n{stderr}"));
    let agent_pos = stderr
        .find("factory: running agent...")
        .unwrap_or_else(|| panic!("running-agent marker missing from stderr:\n{stderr}"));
    let validate_pos = stderr
        .find("factory: validating...")
        .unwrap_or_else(|| panic!("validating marker missing from stderr:\n{stderr}"));
    let terminal_pos = stderr
        .find("factory: → ESCALATE")
        .unwrap_or_else(|| panic!("terminal-state marker missing from stderr:\n{stderr}"));

    assert!(intent_pos < agent_pos, "intent marker must precede agent marker");
    assert!(agent_pos < validate_pos, "agent marker must precede validating marker");
    assert!(validate_pos < terminal_pos, "validating marker must precede terminal marker");
}

/// With no open intent: no-intent marker → validating → terminal state.
#[test]
fn should_emit_no_intent_marker_when_backlog_exhausted() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    init_demo(home.path(), work.path());
    let verdict = write_verdict(
        home.path(),
        r#"{"scenarios":[{"id":"x","satisfied":true}]}"#,
    );

    let output = factory(home.path())
        .env("FACTORY_JUDGE", "scripted")
        .env("FACTORY_JUDGE_SCRIPT", &verdict)
        .current_dir(work.path())
        .args(["run", "demo", "--once"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("factory: no open intent — validating"),
        "no-intent marker missing from stderr:\n{stderr}"
    );
    let validate_pos = stderr
        .find("factory: validating...")
        .unwrap_or_else(|| panic!("validating marker missing from stderr:\n{stderr}"));
    let terminal_pos = stderr
        .find("factory: → NO_OP")
        .unwrap_or_else(|| panic!("terminal-state marker missing from stderr:\n{stderr}"));
    assert!(
        validate_pos < terminal_pos,
        "validating marker must precede terminal marker"
    );
}
```

- [ ] **Step 2: Run the integration tests to confirm they fail**

```bash
cargo test --test progress_output 2>&1 | cat
```

Expected: both tests FAIL with "intent marker missing from stderr" and "no-intent marker missing from stderr".

- [ ] **Step 3: Add the four stage markers to `src/commands/run.rs`**

Find the section after `let intent = backlog::next_intent(&backlog_text);` and modify the `match &intent` block. Currently:

```rust
let backlog_text = std::fs::read_to_string(code_root.join("BACKLOG.md")).unwrap_or_default();
let intent = backlog::next_intent(&backlog_text);
let (changed, diff, agent_log) = match &intent {
    Some(intent) => {
        let request = AgentRequest {
            app: app.to_string(),
            code_root: code_root.clone(),
            intent: intent.clone(),
        };
        let log = match agent.implement(&request) {
            Ok(outcome) => outcome.log,
```

Change to:

```rust
let backlog_text = std::fs::read_to_string(code_root.join("BACKLOG.md")).unwrap_or_default();
let intent = backlog::next_intent(&backlog_text);
let (changed, diff, agent_log) = match &intent {
    Some(intent) => {
        eprintln!("factory: intent → {}", intent_label(intent));
        let request = AgentRequest {
            app: app.to_string(),
            code_root: code_root.clone(),
            intent: intent.clone(),
        };
        eprintln!("factory: running agent...");
        let log = match agent.implement(&request) {
            Ok(outcome) => outcome.log,
```

And in the `None` arm, currently:

```rust
    None => (false, String::new(), None),
```

Change to:

```rust
    None => {
        eprintln!("factory: no open intent — validating");
        (false, String::new(), None)
    }
```

Then find the line just before `let validation = match evaluate(...)` and add:

```rust
    eprintln!("factory: validating...");
    let validation = match evaluate(app, &code_root, &factory_root, judge, run_id) {
```

- [ ] **Step 4: Run the integration tests to confirm they pass**

```bash
cargo test --test progress_output 2>&1 | cat
```

Expected: `should_emit_all_stage_markers_in_order_when_intent_exists ... ok` and `should_emit_no_intent_marker_when_backlog_exhausted ... ok`.

- [ ] **Step 5: Run the full suite to confirm nothing broke**

```bash
cargo test 2>&1 | cat
```

Expected: all existing tests still pass, 2 new integration tests pass.

- [ ] **Step 6: Commit**

```bash
git add tests/progress_output.rs src/commands/run.rs
git commit -m "Emit stage markers to stderr during run"
```

---

## Done

After both tasks, `factory run --once` emits to stderr:

```
factory: intent → B7 — add progress output
factory: running agent...
factory: validating...
factory: → PR_READY (100%)
```

or, when backlog is exhausted:

```
factory: no open intent — validating
factory: validating...
factory: → NO_OP (100%)
```
