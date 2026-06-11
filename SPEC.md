# SPEC — `factory` v0

## What this is

`factory` is a command-line tool that wraps a coding agent (Claude Code, etc.) with
the parts a coding agent does not give you on its own: held-out validation, explicit
terminal states including the freedom to do nothing, an evidence bundle as the unit
of output, and a registry so many projects stay resumable.

It does not implement code itself and it does not run its own agent loop internals.
It **delegates** implementation to a coding agent and **owns** the outer loop around
it. The line between owns and delegates is the product. See the ADRs.

## v0 scope (frozen)

v0 is exactly four commands. Nothing is added to this list without a NEEDS_DECISION
raised to the human first.

### `factory init <app> [--greenfield|--brownfield]`
Scaffolds a new factory line. Creates the code root and the factory (holdout) root
from templates embedded in the binary, and registers `<app>` in the registry. `--greenfield` is the
default. The mode only affects which templates are laid down; v0 may treat both
modes identically if that is simpler, as long as the project is registered.

### `factory validate <app>`
Runs the held-out judge against the app's scenarios and reports a satisfaction
fraction (0-100) plus a written evidence bundle. Validation is executed by
`factory`, never by the implementer agent. The judge observes the app's external
behavior only; it never reads the app's source. Writes the satisfaction value where
`run` can read it, and writes the full bundle under the factory root's `evidence/`.

### `factory run <app> --once`
Performs exactly one outer-loop pass:
1. Read the app's spec, ADRs, backlog, progress.
2. Select the next unaddressed backlog intent.
3. Delegate implementation of that intent to the coding agent (in the code root,
   with no path to the factory root).
4. Run `validate`.
5. Emit exactly one terminal state with an evidence bundle:
   `PR_READY` | `NO_OP` | `ESCALATE` | `NEEDS_DECISION` | `RETRYABLE`.
On `PR_READY`, the change is committed. On `NO_OP`, nothing is committed and the
bundle explains why no change was correct. The pass must never fabricate a passing
result; a scenario it cannot satisfy is reported honestly, not forced green.

### `factory ls`
Lists every registered project with its last-known state (mode, last satisfaction,
last terminal state, last run time).

## Out of scope for v0 (deferred to backlog, built by dogfooding)

`--afk`, `--watch`, `--max-iters`, the multi-iteration loop, the integrity/ADR-drift
loop, `factory spec`, `factory scenario add`, `factory status`, `factory resume`,
additional *real* agent providers beyond Claude Code (Codex/Gemini), any TUI.
The `scripted` agent provider is in scope as a validation affordance (see ADR-0001).

## Definition of done for v0

All v0 scenarios in the holdout set pass, and pass on two consecutive full
validation runs. A human can `init` a fresh project, `run --once` against it, watch
it produce an honest terminal state and an evidence bundle, and `ls` it — with the
implementer agent provably unable to read the holdout.
