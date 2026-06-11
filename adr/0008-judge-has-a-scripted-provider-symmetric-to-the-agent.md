# ADR-0008 — The judge has a scripted provider, symmetric to the agent

Amends ADR-0001. Adds an isolation constraint in the spirit of ADR-0002.

## Decision
The judge — like the agent (ADR-0001) — has a runtime-selectable **`scripted`**
provider alongside its real subprocess implementation. The real judge (default)
spawns `claude -p` driven by the factory root's `judge.md` and `scenarios/`. The
`scripted` judge returns canned per-scenario verdicts from a caller-supplied source
(env var / file). The provider is selected at runtime (flag/env), defaulting to the
real judge.

The `scripted` judge exists to make `factory validate` **plumbing** deterministically
testable end-to-end against the real binary: the satisfaction-fraction math, the
evidence-bundle write, the registry status update, and how the result is surfaced.
It does **not** make judgments — judgment stays with the real judge. A canned verdict
is test scaffolding for the machinery around the judge, never a substitute for it.

## Why
ADR-0001 gave the agent a `scripted` provider because a compiled release binary
contains no test doubles and the external held-out harness must drive the real binary
deterministically. The exact same argument applies to `validate`: with only a live,
nondeterministic, paid `claude -p` path, the external S002 harness could not exercise
`validate` deterministically. ADR-0001 happened to name only the agent; this records
that the judge gets the same seam, so a later session does not find a judge seam the
ADRs never mention and wonder whether it is a mistake.

## Consequences
- v0 ships one **real** judge (`claude -p`) plus the `scripted` judge. Test code may
  still inject in-crate doubles for fast `cargo test` (ADR-0001 unchanged).
- `factory` derives the satisfaction fraction from the judge's per-scenario booleans,
  not from any self-reported total — a fabricated number cannot override the honest
  count.
- **Isolation (ADR-0002 note):** the scripted-judge seam (its selection flag/env and
  the canned-verdict source) is **trusted-runner-only**. It must be unreachable by the
  implementer agent — the same boundary that holds the scenarios out of reach. During
  `run`, the implementer subprocess must not be able to select or supply a scripted
  verdict (enforced when `run`'s isolation lands; see backlog B6). Letting the
  implementer drive the judge would re-open `return true` from a different door.
