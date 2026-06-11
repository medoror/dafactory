# ADR-0001 — Record architecture decisions

## Decision
We record every significant architecture decision in this directory as a short,
numbered, append-only markdown file. Each ADR states the decision, why it was made,
and its consequences. ADRs are immutable once accepted; a later decision that
supersedes an earlier one is a new ADR that references the old.

## Why
Decisions drift out of memory and out of code comments. A durable, reviewable record
keeps the implementer agent and future humans honest about *why* the system is shaped
the way it is, and turns "we always did it this way" into something a newcomer can
read in an afternoon. The ADRs are a rail: honor them, and surface any task that
seems to require violating one as a NEEDS_DECISION.

## Consequences
- The first real decision for this project replaces or follows this starter ADR.
- ADRs are named `NNNN-short-kebab-title.md` and numbered in acceptance order.
