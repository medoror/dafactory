# ADR-0009 — Agents are subprocesses; `run` observes them via git

Amends ADR-0001 (the agent contract). Builds on ADR-0002, ADR-0004, ADR-0006.

> Refined by ADR-0010: `run` validates on every pass; the "no diff → NO_OP" line
> below is superseded by "no diff → validate → (100 → NO_OP, else → ESCALATE)."

## Decision

**The agent contract.** An agent — real or scripted — is a subprocess launched with
its working directory set to the **code root**. Its output that matters is the
resulting **working-tree change**, not its stdout. `factory` never interprets agent
stdout as edit instructions (ADR-0004): it does not parse a patch, apply a manifest,
or otherwise act as an edit-application harness. The two providers share one
interface:
- `real` (default) — spawns `claude -p <prompt>` in the code root.
- `scripted` — runs `sh -c $FACTORY_AGENT_SCRIPT` in the code root.

The scripted provider is how the external harness drives `run` deterministically, and
it expresses the three scenario behaviours directly: *correct-impl* (the script
writes good code), *no-work* (the script changes nothing), *snooping* (the script
tries to reach the factory root and fails).

**Observation via git.** `run` reads the agent's effect from git, not from what the
agent says it did. The code root is a git repo with a clean tree before the agent
runs (created by `init`; see "Consequences"). After the agent returns, `run` stages
the tree and takes the diff. That diff is the single source of truth: it is what the
evidence bundle records and, on `PR_READY`, exactly what gets committed — the bundle's
`change.diff` and the commit are the same artifact, not approximately (ADR-0006).

**Outcome → terminal state.** From the observed diff and the validation result:
- **no diff** (or no unaddressed intent) → `NO_OP` — nothing to commit.
- **diff and satisfaction == 100** → `PR_READY` — commit the diff.
- **diff and satisfaction < 100** → `ESCALATE` — *work* failed; everything ran, the
  result just is not passing, and retrying will not change that. Commit nothing.
- **machinery failed** (agent could not run, judge errored, no verdict produced) →
  `RETRYABLE` — a different concern from a sub-100 verdict. Absence of a verdict must
  **not** be collapsed into "a verdict of fail." `PR_READY` requires `== 100` exactly;
  it is never a threshold.

The transient-vs-structural split inside `RETRYABLE`/`ESCALATE` (a missing tool is
transient; a malformed scenario is structural) is noted here but only minimally
exercised in B3; B4 and the AFK loop build on it.

## Why
ADR-0001 made the agent a swappable interface with a scripted provider but did not
say what an agent *is* mechanically. Saying "the agent is a subprocess whose effect is
the working tree, observed via git" keeps `factory` thin (ADR-0004) and makes the
result legible and honest: the evidence is the actual diff and the actual commit, and
a green result means a real, committed change — never the agent's self-report.

## Consequences
- `init` creates the git repo: it `git init`s the code root and commits the scaffold,
  and the scaffold ships a `.gitignore` so build artifacts are never staged. Repo
  creation belongs with scaffold creation, not with `run`; `run` requires an existing
  repo and emits `RETRYABLE` if one is missing (this reopened B1).
- **Isolation (ADR-0002).** `FACTORY_AGENT_SCRIPT` — like the scripted-judge seam — is
  trusted-runner-only. The agent subprocess is launched with the `FACTORY_*` seam
  variables scrubbed from its environment and with no path to the factory root.
  Rigorous enforcement and the snooping scenario are B6.
