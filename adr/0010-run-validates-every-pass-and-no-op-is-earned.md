# ADR-0010 — `run` validates every pass; NO_OP is earned, not assumed

Supersedes ADR-0009's "no diff → NO_OP — nothing to commit" line. Builds on ADR-0006.

## Decision

**Validate on every pass.** `run` invokes the judge on every pass, including passes
where the agent produced no change. A `NO_OP` is never asserted from a no-change tree
alone — that is the agent grading its own homework, the exact failure the holdout
exists to prevent. `NO_OP` is *earned* by a passing validation.

**Terminal mapping** (when an intent exists and the agent ran):

| observed change | satisfaction | terminal state |
|---|---|---|
| changed     | 100  | `PR_READY` (commit) |
| no change   | 100  | `NO_OP` (already satisfied — no change was correct) |
| changed     | <100 | `ESCALATE` |
| no change   | <100 | `ESCALATE` |

`PR_READY` requires `== 100` exactly. `<100` is `ESCALATE` whether or not the agent
changed anything: everything ran, the result is not passing, and a retry will not
help. Machinery failures (agent can't run, no verdict, not a repo, dirty baseline)
remain `RETRYABLE` (ADR-0009) — distinct from a sub-100 verdict.

**No unaddressed intent.** When the backlog has no open `- [ ]` item, `run` does not
invoke the agent; it validates and emits `NO_OP`. Backlog-exhausted ≠ work-done — they
diverge under regressions or optimistically-checked items — so the bundle's reason
distinguishes two sub-cases that share the `NO_OP` terminal state:
- exhausted + 100 → the **completion signal** (the project's success condition);
- exhausted + <100 → a **quiet alarm** (incomplete backlog or a regression).

The five terminal states stay closed: no `DONE`/`COMPLETE` is added. The completion
meaning lives in the bundle's reason field, not a new state.

**Terminal-state authorship.**
- `NEEDS_DECISION` names a specific, answerable question blocking progress and is
  *authored by the agent* (per CLAUDE.md rails: the agent raises options when an
  intent is ambiguous, rather than silently doing nothing).
- `ESCALATE` is *originated by `run`* when something is wrong but `run` cannot frame
  it as a clean choice — e.g. the agent made no change and the result is bad and
  unexplained.
- `run` honors an agent-emitted terminal tag if one is present, and only originates
  `ESCALATE` for the unexplained-failure case. **v0 defines no channel** for an agent
  to emit a tag, so in practice `run` originates `ESCALATE`; the honor-the-tag seam is
  a clean extension point for a later item.

## Why
"Never force a green" (SPEC) means a do-nothing pass must be backed by evidence that
the app actually passes. Validating every pass — the small, always-paid cost — is what
lets even an unattended loop (the future AFK loop) catch a post-completion regression:
skipping validation when there's "nothing to do" blinds the loop exactly when it runs
alone. A NO_OP that records "no change; already at 100%" is honest; an unvalidated one
is a cheap lie, and cheap lies are worse than a little extra work.

## Consequences
- `evaluate` (the judge call) runs on every `run` pass; re-validation skipping is a
  later AFK-loop optimization, not v0.
- `NO_OP` and `ESCALATE` bundles embed the validation result and a reason that reads
  plainly (e.g. "no change made; scenarios already at 100%", "backlog complete", or
  "agent made no change and the app does not pass"), so the evidence trail is legible
  beyond the terminal-state field alone (ADR-0006).
- `NO_OP` now records `last_satisfaction` in the registry (it has a verdict).
