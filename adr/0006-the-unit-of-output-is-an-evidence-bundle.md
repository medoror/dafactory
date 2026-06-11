# ADR-0006 — The unit of output is an evidence bundle, not a diff

## Decision
Every `run` and every `validate` produces a structured evidence bundle, written
under the factory root's `evidence/<timestamp>/`. Minimum fields: the terminal
state, a summary, the originating intent, the scenario id(s) exercised, the
validation result (per-scenario satisfied + transcript), the diff or "no change",
and residual risk / uncertainty.

## Why
The human is the only reviewer and should review behavior, not syntax. Code is
cheap; the scarce artifact is validated change with evidence a skeptical reviewer
would accept. The bundle is what lets the human stay on the loop instead of in it.

## Consequences
A `PR_READY` without a complete bundle is a defect, not a success. `NO_OP` and
`ESCALATE` also produce bundles — the reason for stopping is itself the evidence.
