# Design: `run` progress output

## Problem

`factory run --once` is silent for several minutes while the agent runs and the judge
validates. A healthy run is indistinguishable from a hung process, so operators Ctrl-C
it — leaving a dirty working tree that breaks the next run. This is the root cause
documented in BACKLOG.md (dogfooding, screener B2).

## Decision

Stage-entry markers to stderr. The agent subprocess keeps its stdout/stderr captured
(for the evidence bundle). No threads, no async, no heartbeat ticker.

## Stage markers

Five `eprintln!` calls in `commands/run.rs` and `main.rs`, all prefixed `factory: `:

| When | Message |
|------|---------|
| Intent selected | `factory: intent → B7 — add progress output` |
| No open intent | `factory: no open intent — validating` |
| Before `agent.implement()` | `factory: running agent...` |
| Before `evaluate()` | `factory: validating...` |
| After `commands::run::run()` returns (in `main.rs`) | `factory: → PR_READY (100%)` or `factory: → RETRYABLE` |

The terminal-state marker is emitted in `main.rs` after `outcome` is known — not inside
`commands/run.rs` — so all early-return paths (holdout ESCALATE, machinery RETRYABLE)
are covered by one location. Satisfaction is included when present:
`factory: → {state} ({pct}%)` if `outcome.satisfaction` is `Some`, otherwise
`factory: → {state}` with no percentage.

All markers go to stderr. The existing `println!` in `main.rs` (`Ran 'app': PR_READY (100%)`)
stays on stdout as the machine-readable summary line.

## What does NOT change

- `RealAgent::implement()` — stays `.output()`, capturing stdout+stderr for the log.
- `AgentOutcome.log: String` — no type change.
- `ScriptedAgent` — unchanged.
- No new dependencies. No threads. No async.

The agent's captured output remains available in the evidence bundle for post-hoc
review. The tradeoff (no live claude output during the agent step) is accepted: the
stage markers solve the "looks like a hang" problem; the live output is a separate
concern.

## Files changed

- `src/commands/run.rs` — four `eprintln!` calls: intent-selected, no-intent,
  before-agent, and before-validate.
- `src/main.rs` — one `eprintln!` for the terminal-state marker, emitted after
  `commands::run::run()` returns and `outcome` is available, before the `println!`
  summary. Extracts a small helper `progress_terminal(state, satisfaction)` so the
  format string is testable without a subprocess.

## Test coverage

1. **Integration test** (`tests/progress_output.rs` or added to existing integration
   suite): run the real binary with scripted agent + scripted judge; capture stderr;
   assert the four stage lines appear and are in the correct order.
2. **Terminal-state marker format** (`commands/run.rs` or `main.rs` unit test): assert
   the string produced for the final stderr line matches the expected format for each
   terminal state. Pure string test, no subprocess.

## Out of scope

- Live streaming of the agent's output to the terminal (requires tee + thread; deferred).
- Heartbeat ticker within the agent step.
- Progress for `factory validate` (separate command, shorter, less urgent).
