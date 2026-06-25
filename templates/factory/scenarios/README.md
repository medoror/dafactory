# Scenarios for `{{app}}`

Held-out scenarios live here. They are never visible to the implementing agent:
`factory` runs the implementer with its working directory set to the code root and an
environment that does not expose this holdout root.

Each scenario is a markdown file paired to a backlog intent by id (`S001` ↔ `B1`) and
judges the app from the outside on observable behavior only — see `../judge.md`.

---

## Scenario format

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

## Good vs. loose scenarios

**Good:** specific, observable, behavior-level. The judge can verify it by running the
app and observing output — no source reading required.

**Loose:** vague or generous. "The output looks reasonable" or "the command succeeds"
grants benefit of the doubt the holdout is supposed to deny. A loose scenario is a hole
in the holdout.

Ask for each scenario: "could the judge verify this purely by observing the app's
behavior, without reading source?" If no, the scenario is mis-written.
