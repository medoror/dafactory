# ADR-0018 — Environment and build scaffolding via devenv

## Status
Accepted as the direction for reproducible environments and release builds. Phase 1
(environment + validation) is the higher-priority, more foundational half and is a
candidate for v1 because it is a precondition for validation on real projects. Phase 2
(release build) is genuinely post-v0. Both are recorded here because they share one
substrate — a single `devenv.nix` — and splitting them would obscure that they are one
decision.

## Context
`factory init` does *thin* scaffolding today: a spec/backlog/ADR skeleton and a bare
git repo. It has no notion of a development environment or a path to a released
artifact. Two gaps follow, at opposite ends of the pipeline.

**Environment (upstream of validation).** `validate` drives the app and observes
behavior — which presupposes the app can be *built and run*. v0 silently assumes that
environment exists; it did for `factory` building itself only because the toolchain was
already set up. Point `factory` at a fresh greenfield project and the first `run` has
the agent implementing against scenarios in an environment with no way to compile or
execute anything, and validation has nothing to drive. A reproducible environment is
therefore not a nice-to-have bolted onto `init`; it is a *precondition for validation to
mean anything*. It is also the same concern as the "validation environment" previously
treated as a distinct second use of the sandbox (ADR-0013): "the agent builds and runs
reproducibly" and "the judge validates against a known-good environment" are one thing.

**Release (downstream of `PR_READY`).** The terminal states stop at "validated,
committed." `PR_READY` → "actually shipped" (tag, CI, build the cross-compiled release
artifact per ADR-0005, publish) is an unmodeled stage.

[devenv](https://devenv.sh/) fits both. It provides fast, declarative, reproducible,
composable environments from a single `devenv.nix` (100k+ packages, 50+ languages).
Crucially for this design: `devenv up` stands processes/services up, `devenv test` runs
behavior checks against the running environment, `devenv build` / `devenv container`
emit distributable artifacts (best per-language packaging chosen automatically, e.g.
crate2nix for Rust), and it ships integrations for both Claude Code and GitHub Actions.

## Decision

### Consume devenv; do not reimplement it
`factory` treats `devenv.nix` as the **environment contract** and consumes devenv the
same way it consumes Claude Code (implementation) and `sbx` (isolation). It does not
become a Nix expert, does not generate sophisticated `devenv.nix` files itself, and does
not own the environment's contents. The moment `factory` starts reasoning about packages
or services, it is building a worse devenv.

### The agent writes the Nix
`devenv.nix` is a declarative file in the code root — an ideal surface for the agent to
write and edit, guided by scenarios that include "the environment builds and the app
runs." `factory` only *runs* devenv commands; it does not author or interpret the Nix.

### Optional and detected (the lean-by-default posture)
Nix is a real adoption cost — the single heaviest prerequisite `factory` would take on,
and a barrier for non-Nix users. So devenv is a power-up, not a tax: a project *with* a
`devenv.nix` gets a reproducible environment and devenv-driven validation; a project
*without* one falls back to running on the host as v0 does. `factory` never *requires*
devenv — it *lights up* when devenv is present, the same posture as `--sandbox`
(ADR-0013).

### Brownfield vs. greenfield finally earns the flag
- **Greenfield:** `init` lays down a starter `devenv.nix` (or the agent creates it as
  early backlog work).
- **Brownfield:** `init` detects and uses an existing `devenv.nix`.
This is the first real job for the `--greenfield` / `--brownfield` distinction, which
has been largely cosmetic in v0.

### Phase 1 — Environment + validation (foundational; v1 candidate)
- `run` executes the agent inside `devenv shell`.
- `validate` stands the app up with `devenv up` and drives it via `devenv test` /
  scenario drivers, so the judge validates against a reproducible, known-good
  environment instead of whatever happens to be on the host.
- The agent's environment, the validator's environment, and (Phase-2) CI's environment
  become **one** `devenv.nix` — so a passing local run is meaningful in CI because they
  are the same definition, not two that happen to agree. This three-way unification is
  the prize.

### Phase 2 — Release build (post-v0)
- The same `devenv.nix` emits a release artifact: `devenv build` (Nix-derivation
  outputs, crate2nix for the Rust binary) and/or `devenv container`.
- This is the CI/release stage the loop does not model today. It introduces a new
  outcome the five terminal states do not cleanly cover: **validated locally but the
  pipeline (CI / release build) failed.** That new condition must be designed
  deliberately when Phase 2 is built — it is not any existing terminal state.

## Relationship to other ADRs
- **ADR-0013 (sandbox) is orthogonal and complementary.** devenv defines *what* the
  environment contains (reproducibly); the sandbox defines *how isolated* the agent
  running in it is. `devenv shell` inside an `sbx` microVM is the full-strength version:
  reproducible *and* hypervisor-isolated. This also simplifies ADR-0013 — the
  "validation environment" concern moves to devenv, leaving the sandbox purely about
  isolation/safety.
- **ADR-0005 (Rust, cross-compiled distribution):** Phase 2's `devenv build` is a
  concrete path to those release artifacts.
- **ADR-0015 (capability ordering):** Phase 1 is upstream of meaningful autonomous work
  on real projects, so it sits early; Phase 2 sits late, alongside scale/shipping.

## Consequences
- `init`, `run`, and `validate` gain a devenv-aware path, gated on a `devenv.nix` being
  present; the host-only path remains the fallback.
- Validation on real (non-self) projects becomes possible, which is the difference
  between "a tool that builds itself" and "a tool you can point at anything."
- A new pipeline-failure outcome must be designed for Phase 2.
- Both phases are bounded product lines the factory could build on itself once the loop
  exists.

## Verify before building
- Confirm the devenv Claude Code integration cleanly supports running the agent
  non-interactively inside `devenv shell` (interaction with the agent-as-subprocess
  contract and, later, the `sbx` invocation).
- Confirm `devenv test` / `devenv up` give the judge a stable way to drive the app for
  behavior-level scenarios.
- Confirm `devenv build` produces the cross-compiled Rust artifacts ADR-0005 wants.

## Alternatives considered
- **Reimplement environment setup in `factory`.** Rejected — building a worse
  devcontainer/Nix is the ADR-0004 trap.
- **Require devenv unconditionally.** Rejected — Nix is too heavy a prerequisite to make
  mandatory; optional/detected preserves the lean default.
- **Devcontainers or plain Dockerfiles instead of devenv.** Viable, but imperative and
  less composable; devenv's declarative `devenv.nix` plus its built-in `up`/`test`/`build`
  story maps more directly onto the agent-edits-it / validate-drives-it / build-ships-it
  pipeline.
- **Split into two unrelated ADRs.** Rejected — both phases are outputs of one
  `devenv.nix`; one ADR with two phases keeps that visible.
