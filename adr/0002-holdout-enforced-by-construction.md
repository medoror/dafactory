# ADR-0002 — The holdout is enforced by construction, not by convention

Related ADRs:
- ADR-0008 — the scripted-judge seam is trusted-runner-only; the implementer must not
  be able to select or supply a verdict.
- ADR-0009 — the `FACTORY_AGENT_SCRIPT` seam is likewise trusted-runner-only, and the
  agent subprocess runs with the `FACTORY_*` seams scrubbed from its environment.
- ADR-0011 — v0 enforces this as a construction boundary (hygiene, not OS sandboxing)
  with a canonical, fail-closed run-time guard; the path is not secret.
- Judge isolation — "never reads the app's source" is construction (factory hands the
  scenarios + a black-box driver path, not source contents — tested in-crate) plus
  discipline (the model is told not to read the path it's given; unenforced in v0).

## Decision
The scenarios and judge live in the factory root, a directory the implementer agent
has no filesystem path to. `factory` runs the implementer subprocess with its
working directory set to the code root and an environment that does not expose the
factory root. Validation is performed by `factory` invoking the judge — never by the
implementer agent, and never by code the implementer wrote.

## Why
This is the core IP and the entire reason the tool exists. If the implementer can
see the assertions, it overfits to them; `return true` is always waiting to be
rediscovered by a machine with no shame. Holding the scenarios out of reach is what
makes a green result mean something. Convention ("please don't peek") is not
enough; the boundary must be a property of how the process is launched.

## Consequences
The factory root path is owned and resolved by `factory`, derived from the registry,
not passed through to or discoverable by the implementer. A scenario verifies this
boundary directly (see the holdout-by-construction scenario).
