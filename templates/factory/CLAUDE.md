# Scenario-authoring working agreement for `{{app}}`

You are helping write held-out scenarios for `{{app}}`. This is a **scenario-authoring
session only** — not an implementation session.

## Session discipline (critical)

**Close this session when you finish writing scenarios. Never use it for implementation
or for `factory run` work.** The holdout boundary is only as strong as this discipline:
if scenarios exist in your context window when you implement, the agent can draw on them
even if the files are deleted from disk. Start a fresh session for any implementation
work.

## What you have here

- `SPEC.md` — the contract for this version (copied from the code root by `factory scenarios`)
- `BACKLOG.md` — the open intents, each paired to a scenario id (copied from the code root)
- `scenarios/` — write held-out scenario files here (one per intent)
- `judge.md` — the judge's working agreement (do not modify)

## Your task

Read `SPEC.md` and `BACKLOG.md`. For each open `- [ ]` intent in the backlog, draft a
scenario file in `scenarios/`. Name each file after its id (e.g. `S001.md`, `S002.md`).

Scenarios are the concrete, behavior-level acceptance checks that the implementing agent
never sees. The agent is judged against them from the outside. If a scenario is vague or
generous, a model can satisfy it without doing the real work — every loose scenario is a
hole in the holdout.

## Scenario format

Each scenario is a markdown file:

```markdown
# S001 — short title of the behavior
Pairs with: B1

## Driver
How to exercise the app (the exact command to run, the exact inputs to give).

## Steps
1. Concrete, numbered steps the judge follows.

## Expected observable behavior
- What must be true afterward — stdout content, exit code, files created, etc.
- Be exact. "contains X" is better than "shows something about X".

## Not satisfied if
- The conditions that make this a fail, stated explicitly.
- "I could not tell" is unsatisfied, not satisfied.
```

## What makes a good scenario vs. a loose one

**Good:** specific, observable, behavior-level. The judge can verify it by running the
app and observing what it does — no source reading required.

**Loose:** vague, implementation-level, or generous. Examples of loose scenarios the agent
can quietly satisfy without doing real work: "the output looks reasonable", "the command
succeeds", "the app handles the case correctly". Each of these grants benefit of the doubt
the holdout is supposed to deny.

Ask for each scenario: "could the judge verify this purely by running the app and
observing its behavior, without reading source?" If no, the scenario is mis-written.
