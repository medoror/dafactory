# Multi-Iteration Loop (`--max-iters`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn `factory run --once` (single pass, manual re-invocation) into a bounded
self-advancing loop: `factory run <app> --max-iters <N>` runs up to N passes, ticks
`- [ ]` → `- [x]` in BACKLOG.md after each PR_READY pass, and stops on any non-PR_READY
terminal state.

**Architecture:** Four independent layers built bottom-up. `backlog::tick_intent` is a
pure function (easy to test in isolation). `run_loop` wraps the existing `run` function —
`run` is untouched. The CLI surface (`--once` → `--max-iters` + `--retries`) is the last
layer before the integration test.

**Tech Stack:** Rust, clap, `cargo test`, real `git` in unit tests (existing `sandbox()`
pattern in `src/commands/run.rs`).

---

## Files

| File | Change |
|------|--------|
| `src/backlog.rs` | Add `pub fn tick_intent` (pure function) |
| `src/commands/run.rs` | Add `intent` to `RunOutcome`; add `LoopOutcome`, `tick_and_commit`, `run_loop` |
| `src/cli.rs` | Replace `once: bool` with `max_iters: u32`; add `retries: u32` |
| `src/main.rs` | Drop `--once` guard; call `run_loop`; update display |
| `tests/exit_codes.rs` | `"--once"` → `"--max-iters", "1"` |
| `tests/progress_output.rs` | `"--once"` → `"--max-iters", "1"` |
| `tests/loop_run.rs` | New integration test for two-pass PR_READY loop |

---

## Task 1: `backlog::tick_intent`

**Files:**
- Modify: `src/backlog.rs`

- [ ] **Step 1: Write the failing tests**

Add to the bottom of the `#[cfg(test)] mod tests { ... }` block in `src/backlog.rs`:

```rust
#[test]
fn should_tick_the_matching_intent_line() {
    let backlog = "- [x] **B1 — done.**\n- [ ] **B2 — do it.**\n- [ ] **B3 — later.**\n";
    let result = tick_intent(backlog, "- [ ] **B2 — do it.**");
    assert_eq!(
        result,
        "- [x] **B1 — done.**\n- [x] **B2 — do it.**\n- [ ] **B3 — later.**\n"
    );
}

#[test]
fn should_leave_text_unchanged_when_line_not_found() {
    let backlog = "- [ ] **B1 — something.**\n";
    let result = tick_intent(backlog, "- [ ] **B99 — nonexistent.**");
    assert_eq!(result, backlog);
}

#[test]
fn should_only_tick_the_first_matching_line() {
    let backlog = "- [ ] **B1 — dup.**\n- [ ] **B1 — dup.**\n";
    let result = tick_intent(backlog, "- [ ] **B1 — dup.**");
    assert_eq!(result, "- [x] **B1 — dup.**\n- [ ] **B1 — dup.**\n");
}

#[test]
fn should_preserve_trailing_newline() {
    let backlog = "- [ ] **B1 — x.**\n";
    let result = tick_intent(backlog, "- [ ] **B1 — x.**");
    assert!(result.ends_with('\n'), "trailing newline must be preserved");
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test -q backlog::tests::should_tick 2>&1 | cat
```

Expected: `error[E0425]: cannot find function 'tick_intent'`

- [ ] **Step 3: Implement `tick_intent`**

Add this function to `src/backlog.rs`, right after the `next_intent` function (before
the private helpers):

```rust
/// Replace `[ ]` with `[x]` on the first line whose trimmed content equals `raw`.
/// `raw` is already trimmed (from `Intent.raw`). Returns the original text if the
/// line is not found — safe no-op.
pub fn tick_intent(backlog: &str, raw: &str) -> String {
    let mut ticked = false;
    let body = backlog
        .lines()
        .map(|line| {
            if !ticked && line.trim() == raw {
                ticked = true;
                line.replacen("[ ]", "[x]", 1)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if backlog.ends_with('\n') {
        body + "\n"
    } else {
        body
    }
}
```

- [ ] **Step 4: Run to confirm passing**

```bash
cargo test -q backlog::tests 2>&1 | cat
```

Expected: all `backlog::tests::*` pass, no failures.

- [ ] **Step 5: Run the full suite**

```bash
cargo test -q 2>&1 | cat
```

Expected: all tests pass, no regressions.

- [ ] **Step 6: Commit**

```bash
git add src/backlog.rs
git commit -m "$(cat <<'EOF'
Add backlog::tick_intent to advance a completed intent

Pure function: finds the first line matching intent.raw (trim-aware)
and replaces [ ] with [x]. Safe no-op when the line is not found.
The loop calls this after each PR_READY pass to advance the backlog
without the agent knowing (ADR-0009, the agent is not in the tick path).
EOF
)"
```

---

## Task 2: `RunOutcome.intent` + `LoopOutcome` + `run_loop`

**Files:**
- Modify: `src/commands/run.rs`

This task adds three things: (1) expose the selected intent on `RunOutcome` so `run_loop`
knows what to tick; (2) the `LoopOutcome` struct; (3) `run_loop` + its private helper
`tick_and_commit`.

- [ ] **Step 1: Write the failing unit tests**

Add these tests inside the `#[cfg(test)] mod tests { ... }` block at the bottom of
`src/commands/run.rs`, after the last existing test (`should_give_actionable_guidance...`):

```rust
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
    assert_eq!(outcome.passes_completed, 2, "retry consumes one max-iters slot");
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
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test -q run_loop 2>&1 | cat
```

Expected: `error[E0425]: cannot find function 'run_loop'`

- [ ] **Step 3: Add `intent` field to `RunOutcome`**

In `src/commands/run.rs`, change the `RunOutcome` struct (lines 26–30):

```rust
pub struct RunOutcome {
    pub terminal_state: TerminalState,
    pub bundle_dir: PathBuf,
    pub satisfaction: Option<u8>,
    pub intent: Option<Intent>,
}
```

Then in `finish()` (around line 342), change the `Ok(RunOutcome { ... })` block:

```rust
    Ok(RunOutcome {
        terminal_state: bundle.terminal_state,
        bundle_dir,
        satisfaction,
        intent: bundle.intent.as_ref().map(|ir| Intent {
            id: ir.id.clone(),
            title: ir.title.clone(),
            raw: ir.raw.clone(),
        }),
    })
```

Run `cargo check` to confirm it compiles before continuing:

```bash
cargo check 2>&1 | cat
```

Expected: no errors. (`main.rs` uses `outcome.terminal_state`, `outcome.satisfaction`, and
`outcome.bundle_dir` — all still present. The new `intent` field is ignored there for now.)

- [ ] **Step 4: Add `LoopOutcome`, `tick_and_commit`, and `run_loop`**

Add the following block to `src/commands/run.rs` immediately after the closing brace of the
`run` function (after line 271, before `fn describe`):

```rust
pub struct LoopOutcome {
    pub last_terminal_state: TerminalState,
    pub passes_completed: u32,
    pub satisfaction: Option<u8>,
    pub last_bundle_dir: PathBuf,
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
            .ok_or_else(|| anyhow::anyhow!("app '{app}' is not registered; run `factory init {app}` first"))?
            .code_root
            .clone()
    };

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
                    tick_and_commit(&code_root, intent)?;
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
```

The `with_context` call requires `anyhow::Context`. Add it to the import at the top of
`src/commands/run.rs` (line 14):

```rust
use anyhow::{bail, Context, Result};
```

- [ ] **Step 5: Run to confirm the new tests pass**

```bash
cargo test -q run_loop 2>&1 | cat
```

Expected: all five `run_loop` tests pass.

- [ ] **Step 6: Run the full suite**

```bash
cargo test -q 2>&1 | cat
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/commands/run.rs
git commit -m "$(cat <<'EOF'
Add run_loop, LoopOutcome, and tick_and_commit to run.rs

run_loop wraps the existing run() function in a bounded loop. On
PR_READY it calls tick_and_commit to advance BACKLOG.md as a separate
git commit ("Advance backlog: tick Bn"), then continues. On any other
terminal state (NO_OP, ESCALATE, NEEDS_DECISION, or RETRYABLE after
the retry budget is exhausted) it stops and returns the last state.

RunOutcome gains an `intent` field so run_loop knows what to tick
without re-reading the backlog — populated from bundle.intent in
finish(), same field round-tripped through IntentRecord.
EOF
)"
```

---

## Task 3: Write the failing integration test

**Files:**
- Create: `tests/loop_run.rs`

Write the test BEFORE updating the CLI. The test will fail because `--max-iters` does not
exist yet (clap rejects it with exit 2). That's the intentional red state.

- [ ] **Step 1: Create `tests/loop_run.rs`**

```rust
//! `factory run --max-iters N` loops up to N passes, ticking BACKLOG.md on each
//! PR_READY result and exiting with the last terminal state's exit code.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

fn factory(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_factory"));
    cmd.env_remove("FACTORY_AGENT")
        .env_remove("FACTORY_AGENT_SCRIPT")
        .env_remove("FACTORY_JUDGE")
        .env_remove("FACTORY_JUDGE_SCRIPT")
        .env("FACTORY_HOME", home);
    cmd
}

fn init_demo(home: &Path, work: &Path) {
    assert!(
        factory(home)
            .current_dir(work)
            .args(["init", "demo"])
            .status()
            .unwrap()
            .success(),
        "factory init failed"
    );
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .unwrap()
            .success(),
        "git {args:?} failed"
    );
}

fn write_verdict(home: &Path, json: &str) -> std::path::PathBuf {
    let path = home.join("verdict.json");
    std::fs::write(&path, json).unwrap();
    path
}

/// Two passes each return PR_READY: both intents are ticked and exit code is 0.
#[test]
fn should_tick_two_intents_and_exit_zero_after_two_pr_ready_passes() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    init_demo(home.path(), work.path());

    let code_root = work.path().join("demo");
    std::fs::write(
        code_root.join("BACKLOG.md"),
        "- [ ] **B1 — first.**\n- [ ] **B2 — second.**\n",
    )
    .unwrap();
    git(
        &code_root,
        &["-c", "user.name=t", "-c", "user.email=t@e", "commit", "-aqm", "Two intents"],
    );

    // A scripted agent that creates a uniquely-named file on each pass by reading a
    // pass counter from disk.
    let agent_script = home.path().join("agent.sh");
    std::fs::write(
        &agent_script,
        "#!/bin/sh\n\
         N=$(cat .pass_count 2>/dev/null || echo 0)\n\
         N=$((N+1))\n\
         echo $N > .pass_count\n\
         echo pass > feature-$N.txt\n",
    )
    .unwrap();
    let mut perms = std::fs::metadata(&agent_script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&agent_script, perms).unwrap();

    let verdict = write_verdict(
        home.path(),
        r#"{"scenarios":[{"id":"x","satisfied":true}]}"#,
    );

    let output = factory(home.path())
        .env("FACTORY_AGENT", "scripted")
        .env("FACTORY_AGENT_SCRIPT", &agent_script)
        .env("FACTORY_JUDGE", "scripted")
        .env("FACTORY_JUDGE_SCRIPT", &verdict)
        .current_dir(work.path())
        .args(["run", "demo", "--max-iters", "2"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Both intents ticked.
    let backlog = std::fs::read_to_string(code_root.join("BACKLOG.md")).unwrap();
    assert!(
        backlog.contains("- [x] **B1 — first.**"),
        "B1 not ticked:\n{backlog}\nstderr:\n{stderr}"
    );
    assert!(
        backlog.contains("- [x] **B2 — second.**"),
        "B2 not ticked:\n{backlog}\nstderr:\n{stderr}"
    );

    // Exit 0 (last terminal state = PR_READY).
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit 0;\nstdout: {stdout}\nstderr: {stderr}"
    );

    // Both agent artefacts committed into the code root.
    assert!(
        code_root.join("feature-1.txt").exists(),
        "feature-1.txt missing"
    );
    assert!(
        code_root.join("feature-2.txt").exists(),
        "feature-2.txt missing"
    );
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test --test loop_run 2>&1 | cat
```

Expected: test FAILS — clap exits 2 with `"error: unexpected argument '--max-iters'"`.

---

## Task 4: CLI surface + `main.rs` + fix existing integration tests

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `tests/exit_codes.rs`
- Modify: `tests/progress_output.rs`

All four changes must land together — updating `cli.rs` breaks the `main.rs` pattern
match on `once`, so both must compile together.

- [ ] **Step 1: Update `src/cli.rs`**

Replace the entire `Run` variant (lines 37–49):

```rust
/// Run the outer loop for an app (up to --max-iters passes).
Run {
    /// Name of the registered app.
    app: String,
    /// Number of loop passes to attempt (≥ 1).
    #[arg(long, value_name = "N")]
    max_iters: u32,
    /// Maximum retries allowed on RETRYABLE per occurrence (default 1).
    #[arg(long, default_value_t = 1)]
    retries: u32,
    /// Agent provider: `real` (default, spawns claude -p) or `scripted`.
    #[arg(long)]
    agent: Option<String>,
    /// Judge provider: `real` (default, spawns claude -p) or `scripted`.
    #[arg(long)]
    judge: Option<String>,
},
```

- [ ] **Step 2: Update the `Commands::Run` arm in `src/main.rs`**

Replace the entire `Commands::Run { ... }` arm (lines 68–94):

```rust
Commands::Run {
    app,
    max_iters,
    retries,
    agent,
    judge,
} => {
    let paths = Paths::resolve()?;
    let agent = build_agent(agent.as_deref())?;
    let judge = build_judge(judge.as_deref())?;
    let outcome = commands::run::run_loop(
        &paths,
        &app,
        agent.as_ref(),
        judge.as_ref(),
        max_iters,
        retries,
    )?;
    eprintln!(
        "{}",
        progress_terminal(outcome.last_terminal_state, outcome.satisfaction)
    );
    match outcome.satisfaction {
        Some(value) => println!(
            "Ran '{app}': {} ({value}%) in {}/{max_iters} passes",
            outcome.last_terminal_state, outcome.passes_completed
        ),
        None => println!(
            "Ran '{app}': {} in {}/{max_iters} passes",
            outcome.last_terminal_state, outcome.passes_completed
        ),
    }
    println!("  evidence: {}", outcome.last_bundle_dir.display());
    Ok(ExitCode::from(outcome.last_terminal_state.exit_code()))
}
```

Also remove the now-unused `bail!` import item if `bail` is no longer called in `main.rs`.
Check with `cargo check` and remove from the `use anyhow::{bail, Result};` line if needed:

```bash
cargo check 2>&1 | cat
```

If you see `warning: unused import: 'bail'`, change line 20 in `src/main.rs` to:

```rust
use anyhow::Result;
```

- [ ] **Step 3: Update `tests/exit_codes.rs`**

Three tests pass `"--once"` — change each to `"--max-iters", "1"`:

In `no_op_exits_zero` (line 62):
```rust
        .args(["run", "demo", "--max-iters", "1"])
```

In `retryable_exits_ten` (line 80):
```rust
        .args(["run", "demo", "--max-iters", "1"])
```

In `escalate_exits_eleven` (line 119):
```rust
        .args(["run", "demo", "--max-iters", "1"])
```

- [ ] **Step 4: Update `tests/progress_output.rs`**

Two tests pass `"--once"` — change each to `"--max-iters", "1"`:

In `should_emit_all_stage_markers_in_order_when_intent_exists` (line 76):
```rust
        .args(["run", "demo", "--max-iters", "1"])
```

In `should_emit_no_intent_marker_when_backlog_exhausted` (line 123):
```rust
        .args(["run", "demo", "--max-iters", "1"])
```

- [ ] **Step 5: Run the full suite (all tests including loop_run)**

```bash
cargo test 2>&1 | cat
```

Expected: all tests pass including `loop_run::should_tick_two_intents_and_exit_zero_after_two_pr_ready_passes`.

- [ ] **Step 6: Commit all files together**

```bash
git add src/cli.rs src/main.rs tests/exit_codes.rs tests/progress_output.rs tests/loop_run.rs
git commit -m "$(cat <<'EOF'
Replace --once with --max-iters loop and add integration test

--once is removed. --max-iters <N> (required, >= 1) replaces it;
--retries <N> (default 1) controls RETRYABLE retry budget. Existing
single-pass callers use --max-iters 1. main.rs dispatches to
run_loop instead of run; exit code reflects the last terminal state
(ADR-0012 unchanged). Integration test drives two PR_READY passes
with a scripted agent and asserts both intents are ticked and the
process exits 0.
EOF
)"
```

---

## Self-review

**Spec coverage:**

| Spec requirement | Task(s) covering it |
|------------------|---------------------|
| `--once` removed, `--max-iters <N>` required (≥ 1) | Task 4: `cli.rs` |
| `--retries <N>` default 1 | Task 4: `cli.rs` |
| Per-pass `factory: pass N/M` stderr marker | Task 2: `run_loop` |
| PR_READY → tick BACKLOG.md as separate commit | Tasks 1 + 2: `tick_intent` + `tick_and_commit` |
| NO_OP / ESCALATE / NEEDS_DECISION → stop | Task 2: `run_loop` stop condition |
| RETRYABLE + retries_left > 0 → retry (consumes slot) | Task 2: `run_loop` retry branch |
| RETRYABLE + retries_left == 0 → stop | Task 2: `run_loop` `_ =>` branch |
| Exit code = last terminal state (ADR-0012) | Task 4: `main.rs` |
| Integration test: 2 passes, 2 ticks, exit 0 | Task 3 + 4: `loop_run.rs` |

**No placeholders found.**

**Type consistency:** `LoopOutcome.last_terminal_state: TerminalState` — used as
`outcome.last_terminal_state.exit_code()` in `main.rs` and
`evidence::TerminalState::PrReady` in unit tests. ✓ `RunOutcome.intent: Option<Intent>` —
populated in `finish()` from `bundle.intent: Option<IntentRecord>` (same three fields). ✓
`tick_intent(backlog: &str, raw: &str)` called from `tick_and_commit` as
`tick_intent(&text, &intent.raw)` — `intent.raw: String`, passed by ref. ✓
