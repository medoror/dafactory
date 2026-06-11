# Working agreement for the implementer agent

You are building `factory`. Read SPEC.md, every file in adr/, BACKLOG.md, and
PROGRESS.md at the start of every session.

## Workflow
- Use the Superpowers workflow: clarify the design, write a short plan, implement
  test-first in small steps, review against the plan. Prefer red/green/refactor.
- Work one backlog intent at a time. Pick the next unaddressed one unless told
  otherwise.
- Implement the smallest change that satisfies the intent. YAGNI. DRY.
- Once a pattern exists in this codebase, prefer it over inventing a new one.

## Rails (non-negotiable)
- Honor every ADR. If a task seems to require violating one, that is a
  NEEDS_DECISION, not a judgment call you make silently.
- **Do not invent product requirements.** If an intent is underspecified or two
  intents conflict, stop and surface it as NEEDS_DECISION with concrete options.
- Do not loosen, delete, or narrow tests to make them pass. Do not weaken
  scenarios. A failing check is information, not an obstacle to route around.
- Keep v0 to the four commands in SPEC.md. Anything else is NEEDS_DECISION.

## Stack (see ADR-0005)
Rust + clap, state as serde_json, subprocesses via std::process::Command. No async
runtime in v0. Build test-first with `cargo test`; the agent and judge are traits
(ADR-0001) with fast doubles in tests, and the binary also ships a runtime-
selectable `scripted` agent provider. Scaffolding templates are embedded in the
binary (`init` writes them out; it does not read them from disk).

## The holdout
There is a held-out scenario set and a judge that live **outside this repo**. You
cannot see them and must not try to. Do not search for them, read them, or write
code that targets specific assertions. Build to SPEC.md and BACKLOG.md. You are
judged from the outside on observable behavior.

## Terminal states
End every working session by updating PROGRESS.md and emitting exactly one:
- `PR_READY` — implemented, validated honestly, evidence ready.
- `NO_OP` — the correct outcome is no change; say why.
- `ESCALATE` — cannot proceed safely without human input.
- `NEEDS_DECISION` — a product/architecture choice is missing; list the options.
- `RETRYABLE` — tooling/environment failed in a way worth retrying.

## Output
Return evidence, not vibes: what changed, which intent, what you ran, what you
observed, and what remains uncertain.
