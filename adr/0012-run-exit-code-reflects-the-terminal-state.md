# ADR-0012 — `run`'s process exit code reflects the terminal state

Builds on ADR-0009/0010 (the terminal states).

## Decision
`factory run` maps its terminal state to the process exit code, so a wrapper script
or the future AFK loop can branch on `$?` without parsing output:

| terminal state   | exit code | meaning to a caller            |
|------------------|-----------|--------------------------------|
| `PR_READY`       | 0         | success — committed change     |
| `NO_OP`          | 0         | success — no change was correct|
| `RETRYABLE`      | 10        | machinery failed — retry may help |
| `ESCALATE`       | 11        | work failed — stop / alert a human |
| `NEEDS_DECISION` | 12        | a human decision is required   |

Reserved/unrelated codes are left alone: `1` stays "hard error" (an `anyhow` error
before a terminal state is reached — e.g. unregistered app, bad flag value), and `2`
is clap's usage-error code. The non-success terminal states use a distinct band (10+)
so they collide with neither.

## Why
A failure terminal state that exits 0 lies to every caller that checks `$?` — the AFK
loop would treat an `ESCALATE` as success and never alert, or fail to retry a
`RETRYABLE`. Success-vs-failure is the minimum; distinct codes are cheap and let the
loop act differently per state (retry on 10, stop on 11, surface a decision on 12)
without scraping stdout.

## Consequences
- The mapping lives on `TerminalState::exit_code()` (one place, unit-tested per state)
  and is applied where `main` turns the run outcome into a `process::ExitCode`.
- `validate` is unchanged: it emits a measurement, not a terminal state, so it exits 0
  when validation *executed* — the satisfaction fraction is data, not the command's
  pass/fail. A caller that wants to gate on the fraction reads it from the bundle or
  the registry.
- An integration test drives the real binary to each producible terminal state and
  asserts the exit code, so the boundary wiring can't silently regress to 0 again.
