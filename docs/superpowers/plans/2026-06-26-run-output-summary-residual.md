# Run Output Summary/Residual Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface `summary` and `residual` from the evidence bundle inline in `factory run` terminal output so users can see why a run produced `RETRYABLE` (or any other state) without opening the bundle JSON.

**Architecture:** Pure data-flow change. `RunBundle` already has `summary` and `residual`. They need to be propagated through `RunOutcome` → `LoopOutcome` → `main.rs` output. No logic changes — just threading existing data up the call stack.

**Tech Stack:** Rust, `src/commands/run.rs`, `src/main.rs`

---

### Task 1: Write the propagation test (red)

This test verifies the new fields exist on `LoopOutcome` and carry through from the last pass. It won't compile until Task 2 adds the fields — that compile error IS the red state.

**Files:**
- Modify: `src/commands/run.rs` (tests module, near line 983)

- [ ] **Step 1: Add the failing test**

In `src/commands/run.rs`, inside the `#[cfg(test)]` module, add this test after `should_stop_immediately_on_retryable_when_retries_is_zero` (around line 1006):

```rust
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
```

- [ ] **Step 2: Run the test to confirm it fails to compile**

```bash
cargo test should_propagate_summary_and_residual 2>&1 | cat
```

Expected: compile error — `no field 'summary' on type 'LoopOutcome'` (or similar).

---

### Task 2: Add fields to `RunOutcome` and `LoopOutcome`, wire through `finish()` and `run_loop()` (green)

**Files:**
- Modify: `src/commands/run.rs` lines 26–31, 274–279, 432–441, 339–344

- [ ] **Step 1: Add `summary` and `residual` to `RunOutcome`**

Replace the `RunOutcome` struct (lines 26–31):

```rust
pub struct RunOutcome {
    pub terminal_state: TerminalState,
    pub bundle_dir: PathBuf,
    pub satisfaction: Option<u8>,
    pub intent: Option<Intent>,
    pub summary: String,
    pub residual: String,
}
```

- [ ] **Step 2: Populate them in `finish()`**

Replace the `Ok(RunOutcome { ... })` block in `finish()` (lines 432–441):

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
    summary: bundle.summary.clone(),
    residual: bundle.residual.clone(),
})
```

- [ ] **Step 3: Add `summary` and `residual` to `LoopOutcome`**

Replace the `LoopOutcome` struct (lines 274–279):

```rust
pub struct LoopOutcome {
    pub last_terminal_state: TerminalState,
    pub passes_completed: u32,
    pub satisfaction: Option<u8>,
    pub last_bundle_dir: PathBuf,
    pub summary: String,
    pub residual: String,
}
```

- [ ] **Step 4: Propagate from the last `RunOutcome` in `run_loop()`**

Replace the `Ok(LoopOutcome { ... })` block in `run_loop()` (lines 339–344):

```rust
Ok(LoopOutcome {
    last_terminal_state: outcome.terminal_state,
    passes_completed,
    satisfaction: outcome.satisfaction,
    last_bundle_dir: outcome.bundle_dir,
    summary: outcome.summary,
    residual: outcome.residual,
})
```

- [ ] **Step 5: Run the propagation test**

```bash
cargo test should_propagate_summary_and_residual 2>&1 | cat
```

Expected: PASS. If there are compile errors on other tests due to struct construction changes, fix them now (add `summary: String::new(), residual: String::new()` to any `RunOutcome { ... }` or `LoopOutcome { ... }` literals in test helpers — search for `RunOutcome {` and `LoopOutcome {` in the test module).

- [ ] **Step 6: Run the full test suite**

```bash
cargo test 2>&1 | cat
```

Expected: all tests pass. Fix any remaining compile errors (same pattern — add `summary`/`residual` fields to struct literals).

- [ ] **Step 7: Commit**

```bash
git add src/commands/run.rs
git commit -m "Propagate summary and residual through RunOutcome and LoopOutcome"
```

---

### Task 3: Print `summary` and `residual` in `main.rs`

**Files:**
- Modify: `src/main.rs` lines 100–101

- [ ] **Step 1: Add the inline output**

In `src/main.rs`, replace lines 100–101:

```rust
println!("  evidence: {}", outcome.last_bundle_dir.display());
```

with:

```rust
if !outcome.summary.is_empty() {
    println!("  summary:  {}", outcome.summary);
}
if !outcome.residual.is_empty() {
    println!("  residual: {}", outcome.residual);
}
println!("  evidence: {}", outcome.last_bundle_dir.display());
```

- [ ] **Step 2: Run all tests**

```bash
cargo test 2>&1 | cat
```

Expected: all tests pass.

- [ ] **Step 3: Build and do a quick smoke check**

```bash
cargo build 2>&1 | cat
```

Expected: compiles clean with no warnings.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "Print summary and residual inline in factory run output"
```
