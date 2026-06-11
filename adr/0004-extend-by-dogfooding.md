# ADR-0004 — Extend by dogfooding; do not build a worse Claude Code or Ralph

## Decision
The inner implement loop is delegated to an existing coding agent. The
brainstorm/plan/TDD discipline is delegated to Superpowers via the agent's system
prompt. `factory` builds none of that. Features beyond v0 are added by running
`factory` on `factory` itself, and no feature is added until the tool is good enough
to help build it.

## Why
Reimplementing the agent harness, the iterate-til-done loop, or a skills framework
means shipping worse versions of things that are already better-funded than a
personal project's weekends. Dogfooding provides a built-in scope governor: if the tool
cannot yet help build feature X, feature X is not next.

## Consequences
v0 is deliberately tiny. The backlog beyond v0 is sequenced so each item is
buildable using the items before it.
