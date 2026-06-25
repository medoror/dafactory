# ADR-0019 — `factory scenarios`: scenario-authoring context bootstrapping

## Status
Accepted.

## Context
After `factory init`, the workflow breaks in two places:

1. The user writes the spec and backlog in the **code root** with agent help — Claude has
   full context there.
2. The user must then write held-out scenarios in the **holdout root** (a separate
   directory the agent never touches), but Claude has no context there: it doesn't know
   the spec, the intent IDs, the scenario format, or why this session must stay separate
   from implementation.

The ergonomic fix considered first — drafting scenarios in the code root and copying them
to the holdout — was ruled out because scenarios drafted in the same Claude session as
implementation work contaminate that session's context window even after the files are
deleted from disk. The session is the boundary, not the filesystem location.

## Decision

Add `factory scenarios <app>`. It:

1. Validates the code root has a filled-in SPEC.md (presence of `<!-- One paragraph`
   means it's still the empty stub → error).
2. Validates BACKLOG.md has at least one open intent (using the same comment-stripping
   logic as `backlog::next_intent`, so commented-out scaffold examples don't count).
3. Copies SPEC.md and BACKLOG.md from the code root into the holdout root. This is
   **safe**: these files are not secret — the implementing agent already reads them. Only
   scenarios are secret, and they always flow the other way (stay in the holdout).
4. Writes a `CLAUDE.md` to the holdout root from an embedded template. This CLAUDE.md:
   - States the role (scenario authoring only)
   - Warns about session discipline (close this session when done, never use it for
     implementation — the holdout boundary is only as strong as this discipline)
   - Contains the scenario format from the README
   - Tells Claude to read the SPEC.md and BACKLOG.md copies in the same directory
5. Prints the holdout path with instructions to open a fresh Claude session there.

The command is idempotent: re-running overwrites the copies and the CLAUDE.md.

## Why the copy is one-directional

SPEC.md and BACKLOG.md are not secrets. The implementing agent reads them during `run`.
Copying them into the holdout root does not weaken the holdout boundary — only scenarios
are secret, and they are never written into the code root. The copy direction is always
code root → holdout root, never the reverse.

## Why a separate command rather than a side effect of `init` or `run`

- `init` is too early: SPEC.md and BACKLOG.md don't exist yet.
- `run` is too late: scenario authoring must happen *before* the first `run`, and
  silently copying context on `run` would not surface the session-discipline guidance
  at the moment the user needs it.
- A separate command makes the handoff explicit and surfaces the right guidance
  (the holdout CLAUDE.md) at the right moment.

## Consequences

- v0 gains a fifth command. The "four commands, frozen" posture in SPEC.md was a scope
  guard against feature creep; this command is a workflow necessity that was not visible
  until the first real end-to-end authoring session. The SPEC.md out-of-scope list
  (`factory spec`, `factory scenario add`) remains off-limits; this is not in that class.
- The holdout root now contains SPEC.md, BACKLOG.md, and CLAUDE.md in addition to
  judge.md and scenarios/. All are authored by `factory`, not the implementing agent.
- The session-discipline warning is in CLAUDE.md (the holdout root), not enforced
  programmatically. The boundary is still construction, not OS enforcement (ADR-0011).

## Alternatives considered

- **Draft in the code root, manually copy.** Rejected: even with file deletion, scenario
  content in the context window of a session used for implementation contaminates it.
- **Holdout CLAUDE.md only (no file copy).** Rejected: forces the user to paste spec and
  backlog into the holdout session manually, losing the ergonomic benefit.
- **Auto-copy on `factory run`.** Rejected: too late for scenario authoring, and the
  session-discipline guidance is not surfaced at the right moment.
