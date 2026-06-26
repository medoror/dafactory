# Design: Surface summary and residual in `factory run` output

## Problem

After `factory run` completes, the terminal shows the terminal state and an evidence
bundle path. To understand *why* a run produced `RETRYABLE` (or any other state), the
user must open the bundle JSON manually. The bundle already has `summary` and `residual`
fields with the right diagnostic content — they just aren't surfaced.

## Solution

Propagate `summary` and `residual` from `RunBundle` through `RunOutcome` and
`LoopOutcome` to `main.rs`, where they are printed inline below the headline.

## Output format

```
Ran 'guess-that-sample': RETRYABLE in 2/3 passes
  summary: validation produced no verdict: agent exited with code 1
  residual: Machinery failed; a retry may resolve it if the cause is transient.
  evidence: /Users/.../evidence/1782486609_770935000
```

Both fields are printed only when non-empty. The `evidence:` line stays at the end.

## Changes

### `src/commands/run.rs`

**`RunOutcome`** — add two fields:
```rust
pub summary: String,
pub residual: String,
```

**`finish()`** — populate from the bundle before returning:
```rust
summary: bundle.summary.clone(),
residual: bundle.residual.clone(),
```

**`LoopOutcome`** — add the same two fields:
```rust
pub summary: String,
pub residual: String,
```

**`run_loop()`** — propagate from the last `RunOutcome`:
```rust
summary: outcome.summary,
residual: outcome.residual,
```

### `src/main.rs`

After the `Ran '...'` headline (lines 91–100), before the `evidence:` line:
```rust
if !outcome.summary.is_empty() {
    println!("  summary:  {}", outcome.summary);
}
if !outcome.residual.is_empty() {
    println!("  residual: {}", outcome.residual);
}
println!("  evidence: {}", outcome.last_bundle_dir.display());
```

## What does NOT change

- `describe()` — RETRYABLE never reaches it (RETRYABLE always comes from `machinery()`
  or `base()`, which already set both fields to meaningful strings). No logic changes.
- Bundle JSON format — `summary` and `residual` were already written there.
- `validate` command output — out of scope; validate has its own output path.

## Testing

- Update `RunOutcome` construction in existing tests to include `summary`/`residual`.
- Update `LoopOutcome` construction/assertions similarly.
- One new test: verify `summary` and `residual` propagate from a single-pass `RunOutcome`
  through `run_loop()` into `LoopOutcome`.
