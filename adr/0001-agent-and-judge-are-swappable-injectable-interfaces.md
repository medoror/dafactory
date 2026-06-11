# ADR-0001 — Agent and judge are swappable, injectable interfaces

> Amended by ADR-0008: the judge gets a runtime-selectable `scripted` provider too,
> symmetric to the agent's.
> Amended by ADR-0009: the agent contract is "a subprocess in the code root whose
> effect is the working-tree change," observed via git.

## Decision
The coding agent and the LLM judge are each accessed through a single narrow trait.
Production implementations spawn a subprocess (`claude -p` for v0). The traits are
injectable so fast `cargo test` can substitute a double without a live model call.
In addition, the binary ships a runtime-selectable **`scripted`** agent provider
that returns canned responses (from an env var or file). This is a first-class
provider, not a test-only object, because a compiled release binary contains no test
doubles — and the external held-out judge must be able to drive the *real binary*
deterministically.

## Why
- Provider-agnostic by construction. Adding Codex or Gemini later is a new trait
  impl plus a config line, not a rewrite. The real lesson from StrongDM's `codergen`
  was that the factory routes work between stations; it does not pretend they are
  the same machine.
- `run` and `validate` are untestable if the only path is a live, nondeterministic,
  paid model call. Trait doubles cover fast in-crate tests; the `scripted` provider
  covers end-to-end scenarios where the external judge drives the built binary.

## Consequences
- v0 ships exactly one **real** provider (Claude Code) plus the `scripted` provider.
  Additional real providers (Codex, Gemini) are deferred (see backlog).
- The provider is selected at runtime (flag/env), defaulting to Claude Code. The
  `scripted` provider's behavior (correct impl / no-work / snooping) is set by the
  caller for scenario runs.
