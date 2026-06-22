# Design: Bounded multi-iteration loop (`--max-iters`)

## Problem

`factory run --once` requires manual re-invocation per backlog intent. Nothing ticks
`- [ ]` → `- [x]` after a PR_READY pass, so the next run re-selects the same intent.
The attended loop composes the now-hardened single pass into a bounded, self-advancing
loop — the keystone before unattended operation (`--afk`), devenv Phase 1, and the
human-in-the-loop control plane (ADR-0016/ADR-0018).

## CLI surface

`--once` is removed. `--max-iters <N>` (required, ≥ 1) replaces it. `--retries <N>`
(default 1) controls how many times a RETRYABLE result is retried before the loop stops.

```
factory run demo --max-iters 5
factory run demo --max-iters 1               # equivalent to old --once
factory run demo --max-iters 5 --retries 0   # stop immediately on RETRYABLE
```

`--max-iters` has no default — the attended loop should have an explicit bound.
`--afk` (future) is the "run until done" mode.

## Loop control flow

```
retries_left = retries (flag, default 1)

for pass in 1..=max_iters:
    emit "factory: pass {pass}/{max_iters}" to stderr
    result = run_once(...)

    PR_READY       → tick BACKLOG.md + commit tick; print pass summary; continue
    NO_OP          → print pass summary; stop (exit 0)
    ESCALATE       → print pass summary; stop (exit 11)
    NEEDS_DECISION → print pass summary; stop (exit 12)
    RETRYABLE      → if retries_left > 0: retries_left -= 1; print "retrying..."; continue
                       (retry re-runs the same intent — backlog was not ticked, so
                        next_intent re-selects the same unchecked line; retry consumes
                        one max_iters slot)
                     else: print pass summary; stop (exit 10)

if loop exhausts max_iters with all PR_READY:
    emit "Loop: {max_iters}/{max_iters} passes PR_READY"; exit 0
```

Exit codes follow ADR-0012 (last terminal state wins).

## Intent advancement

Factory ticks the intent — not the agent. On PR_READY, after `run_once` commits the
implementation, the loop:
1. Calls `backlog::tick_intent(backlog_text, intent.raw)` — pure function, replaces
   `- [ ]` with `- [x]` on the exact `intent.raw` line
2. Writes the result back to `BACKLOG.md`
3. `git::add_all(code_root)`
4. `git::commit(code_root, "Advance backlog: tick {intent.id}")`

This is a separate commit from the implementation commit, so git log stays readable.
`run_once` does not change — the tick is entirely the loop's concern.

## Code structure

**`src/commands/run.rs`** — add two items:

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
) -> Result<LoopOutcome>
```

`run_loop` wraps the existing `run` function. `main.rs` dispatches to `run_loop`
instead of `run` directly. The `Commands::Run` arm drops the `if !once` guard and
reads `--max-iters` / `--retries`.

**`src/backlog.rs`** — add:

```rust
pub fn tick_intent(backlog: &str, raw: &str) -> String
```

Pure function. Iterates lines; finds the first where `line.trim() == raw` and replaces
its `[ ]` with `[x]` (trim-aware — `intent.raw` is already trimmed; file lines have
trailing whitespace). Returns the original text unchanged if no matching line is found
(safe no-op).

**`src/cli.rs`** — replace `once: bool` with `max_iters: u32` and add `retries: u32`
(default 1) in `Commands::Run`.

## Testing

**`backlog::tick_intent` (unit, `src/backlog.rs`):**
- Ticks the correct line given `intent.raw`; leaves other lines unchanged
- No-ops gracefully if the line is not found

**`run_loop` (unit, `src/commands/run.rs`):**
- Runs all `max_iters` passes when every pass is PR_READY; ticks BACKLOG.md each time
- Stops on ESCALATE after some PR_READY passes; last state is ESCALATE
- Retries once on RETRYABLE when `retries=1`; stops on second RETRYABLE
- Stops immediately on RETRYABLE when `retries=0`
- Stops on NO_OP with exit 0

**Integration (`tests/loop_run.rs`):**
- `factory run demo --max-iters 2` with scripted agent (writes a file) + scripted judge
  (100%) — assert 2 PR_READY commits land, BACKLOG.md has 2 ticked intents, exit 0

## What does NOT change

- `run_once` (the existing `run` function) — unchanged
- ADR-0012 exit codes — loop exits with last terminal state's code
- Evidence bundles — one bundle per pass, same format as today
- `factory validate`, `factory init`, `factory ls` — unaffected
