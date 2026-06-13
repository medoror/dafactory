# dafactory

**A personal software factory for solo developers.** `factory` wraps a coding agent with the parts a coding agent doesn't give you on its own: held-out validation the agent can't cheat, the discipline to do nothing when nothing is the right answer, an evidence bundle for every change, and a registry that keeps all your projects resumable.

It does not write code. It runs the loop *around* the thing that writes code.

---

## Contents

- [The idea](#the-idea)
- [Concepts](#concepts)
- [Install](#install)
- [Quick start](#quick-start)
- [Commands](#commands)
- [How a run resolves](#how-a-run-resolves)
- [Project layout](#project-layout)
- [Writing scenarios](#writing-scenarios)
- [Configuration](#configuration)
- [Providers](#providers)
- [The holdout boundary](#the-holdout-boundary)
- [Status](#status)
- [Roadmap](#roadmap)
- [Design decisions](#design-decisions)
- [Contributing](#contributing)
- [Lineage](#lineage)
- [License](#license)

---

## The idea

Coding agents made code cheap. The scarce thing now isn't a diff — it's a *validated* change you can trust without reading every line. That's the actual hard problem in agentic development: how do you know the work is correct when you've stopped reading the code? Tests don't save you if the same agent writes the tests, because an agent optimizing for "make it pass" will happily write `return true`.

`factory` takes the answer that teams running this at the frontier have converged on: put validation **outside the agent's reach**. Each project has two roots. The agent works in the *code root* and sees a spec and a backlog of intents. The judge works against a *holdout root* — concrete scenarios the agent can't read and can't edit, validated by observing the app's behavior from the outside. The agent can't overfit to assertions it never sees.

The other half of the idea is a division of labor. The coding agent owns the inner loop — plan, implement, review. `factory` owns the outer loop — pick the next intent, delegate implementation, validate against the holdout, decide what happened, and record the evidence. The leverage lives in the outer loop, which is exactly the part no coding agent ships. So `factory` is deliberately thin: it delegates implementation to whatever agent you point it at and spends its own complexity on the things that make autonomous work *trustworthy*.

This is the "dark factory" pattern (see [lineage](#lineage)), scaled down to one developer with a dozen half-finished projects instead of a team burning tokens around the clock.

---

## Concepts

A handful of ideas carry the whole tool. Understanding them is most of understanding `factory`.

**Two roots.** Every project is split in two. The **code root** is an ordinary git repo the agent works in — it holds the spec, the backlog, the ADRs, and the code. The **holdout root** lives elsewhere (in your OS data dir) and holds the things the agent must never see: the scenarios and the judge instructions. The split is the foundation of everything else.

**Intents and the backlog.** Work is described as a backlog of *intents* — loosely-phrased units of work (`BACKLOG.md`, one `- [ ]` line each). The agent pulls the next open intent. Intents are paired to scenarios by id.

**Scenarios (the holdout).** A scenario is a concrete, behavior-level acceptance check — "given X, the app does Y" — stored in the holdout root where the agent can't read or edit it. Scenarios are the contract. Because the agent can't see them, it can't write code that games them; it has to actually satisfy the behavior.

**The judge.** Validation is performed by a judge that **observes the app's behavior only** and never reads its source. It returns, per scenario, satisfied or not, with a transcript of what it observed. The judge is the thing that makes a "pass" mean something.

**Satisfaction (measure) vs. terminal state (decide).** `validate` *measures* — it produces a satisfaction fraction (0–100), nothing more. `run` *decides* — it reads that measurement plus context and picks exactly one terminal state. The measurement instrument never makes routing decisions, so a number in the registry is always something `run` consciously decided.

**Terminal states.** Every `run` ends in exactly one of five:

| State | Meaning |
|---|---|
| `PR_READY` | Validated at 100%, change committed. |
| `NO_OP` | Nothing to do — the scenarios already pass; no change made. |
| `ESCALATE` | The machinery ran, the work genuinely doesn't pass, retrying won't help — a human must look. |
| `NEEDS_DECISION` | A specific, answerable choice is blocking progress (raised by the agent, with options). |
| `RETRYABLE` | The machinery failed transiently (couldn't run, no verdict) — another pass might fix it. |

The distinction between the last three is deliberate. `ESCALATE` is a *structural* problem — retrying re-trips it identically. `RETRYABLE` is a *transient* one — safe to retry. `NEEDS_DECISION` has a clean question to answer. An unattended loop needs to tell these apart.

**Evidence bundle.** The unit of output is not code, it's an evidence bundle — terminal state, the intent, the satisfaction result with per-scenario transcripts, the diff (or "no change"), and residual risk. You review behavior in a couple of minutes instead of reading a diff line by line.

**The registry.** The one piece of global state `factory` owns: a small index, in your OS data dir, of every project and its last-known status (mode, last satisfaction, last terminal state, last run time). It's what turns "twelve abandoned repos" into "twelve resumable lines." Everything else lives in the two project roots and in git.

**Providers.** The coding agent and the judge are each reached through a swappable interface. v0 ships the **Claude Code** provider plus a **scripted** provider used for deterministic, zero-cost testing.

---

## Install

### Prerequisites
- **A coding agent.** `factory` delegates implementation to a coding agent — v0 ships the Claude Code provider, so you need Claude Code installed and authenticated. The discipline layer it leans on, [Superpowers](https://github.com/obra/superpowers), is recommended.
- **Git.** Each code root is a git repo; `run` observes the agent's work as a diff and commits on a passing run.
- **Rust toolchain.** Needed to build or install from source, until prebuilt binaries are published.

### From source
Replace the URL with this repo:

```bash
# build from a clone
git clone https://github.com/<your-github>/factory-cli
cd factory-cli
cargo build --release          # binary at target/release/factory

# or install straight from git
cargo install --git https://github.com/<your-github>/factory-cli
```

Planned: prebuilt binaries on GitHub Releases (linux/macos, x86_64 + aarch64) and `brew install`.

---

## Quick start

`init` scaffolds the project; **then you write three things by hand** — the spec, the backlog, and the held-out scenarios — and only then does the tool start working. Here's the whole loop, start to finish.

> **Before you start:** Claude Code installed and authenticated, `git`, and `factory` on your `PATH` (see [Install](#install)). A real run spends tokens — to rehearse the flow for free, use the scripted providers ([Providers](#providers)).

### 1. Scaffold the project

```bash
factory init myapp --greenfield
```

Creates the **code root** (`./myapp`, a git repo with the empty forms committed) and the **holdout root** in your OS data dir, and registers the app. It prints both paths — **copy the holdout path; you need it in step 4.**

### 2. Write the spec — `myapp/SPEC.md`

Fill in the scaffolded form: what the app is, the frozen scope, and the definition of done. Keep it small and honest. This is the durable contract for the whole version.

### 3. Add at least one open intent — `myapp/BACKLOG.md`

One unchecked `- [ ]` line, paired to a scenario id. `run` picks the first open intent; without one it has nothing to do. Phrasing is intentionally loose — describe the intent, not the assertion.

```markdown
- [ ] **B1 (→ S001) — Greet by name.** `myapp --name Ada` prints a greeting that includes the name.
```

### 4. Write the held-out scenario(s) — in the **holdout** root, not the code root

This is the contract the agent never sees. Put one file per scenario in the `scenarios/` dir of the holdout path from step 1. Make it **concrete and behavior-level** — a loose scenario is one the agent can satisfy without doing the real work (see [Writing scenarios](#writing-scenarios)).

```markdown
# S001 — greets by name
Pairs with: B1

## Driver
Run `myapp --name Ada`.

## Expected observable behavior
- stdout contains `Ada`
- exit code is 0

## Not satisfied if
- the name is missing from the output, or the command errors
```

`judge.md` is already scaffolded in the holdout root — leave it or tune it.

### 5. Commit the code root

```bash
cd myapp && git add -A && git commit -m "Write spec and first intent"
```

A clean git tree is a **precondition**: `run` observes the agent's work as a diff against this baseline, so an uncommitted tree makes it bail.

### 6. Run one pass

```bash
factory run myapp --once
```

`factory` picks `B1`, hands it to the agent in the code root (holdout out of reach), validates against `S001`, and emits exactly one [terminal state](#how-a-run-resolves) plus an evidence bundle. On `PR_READY` it commits the change. The exit code reflects the state (`0` = `PR_READY`/`NO_OP`).

### 7. Check status, then repeat

```bash
factory ls
```

Shows each app's mode, last satisfaction, last terminal state, and last run time. v0 does **one intent per `run --once`**, so add the next intent + scenario and run again for each piece of work.

### Who writes what

The division of labor is the point — you own the *contract*, the agent owns the *code*:

| You write, by hand | `factory` / the agent produce |
|---|---|
| `SPEC.md` — the contract for this version | the code (the agent) |
| `BACKLOG.md` intents | `PROGRESS.md` working memory (the agent) |
| held-out scenarios + `judge.md` (in the holdout root) | the terminal state + evidence bundle (`factory`) |

There is deliberately no `factory spec` command: you author the spec and the scenarios yourself, because they're what makes a green result trustworthy. If the tool wrote the spec *and* the code *and* judged itself, "passing" would mean nothing.

---

## Commands

v0 is four commands. That's the whole surface — small on purpose.

### `factory init <app> [--greenfield | --brownfield]`
Scaffolds a new project line. Creates the code root and the holdout root from templates embedded in the binary, makes the code root a git repo with an initial commit, and registers the app. `--greenfield` is the default; the mode is recorded in the registry. Re-running is idempotent. Prints the path to both roots.

### `factory validate <app>`
Runs the held-out judge against the app's scenarios and reports a satisfaction fraction (0–100), plus an evidence bundle written under the holdout root's `evidence/`. The judge observes behavior only — it never reads the app's source. `validate` *measures*: it writes `last_satisfaction` and `last_run_at` to the registry and leaves `last_terminal_state` untouched. `factory` derives the fraction itself and ignores any self-reported count from the judge.

### `factory run <app> --once`
Performs exactly one pass of the loop:

1. Read the spec, ADRs, backlog, and progress.
2. Select the next open backlog intent.
3. Delegate implementation to the agent, in the code root, with the holdout out of reach.
4. Validate against the holdout.
5. Emit exactly one [terminal state](#how-a-run-resolves) plus an evidence bundle. On `PR_READY`, commit.

**Preconditions:** an open intent (`- [ ]`) must exist, and the code root's git tree must be clean. Exit code reflects the terminal state: `PR_READY` and `NO_OP` exit 0; `ESCALATE`, `NEEDS_DECISION`, and `RETRYABLE` exit non-zero.

### `factory ls`
Lists every registered project with its mode, last satisfaction, last terminal state, and last run time (a readable timestamp). Never-run fields show `—`; an empty registry prints a clear "no registered projects" line.

---

## How a run resolves

`run` decides the terminal state from two independent facts: whether the agent **changed** the tree, and what the validation **satisfaction** was. The agent may also raise a terminal state itself (a clean `NEEDS_DECISION`), which `run` relays.

| Agent effect | Satisfaction | Terminal state |
|---|---|---|
| Changed the tree | 100% | `PR_READY` (commit) |
| Changed the tree | < 100% | `ESCALATE` |
| No change | 100% | `NO_OP` (already satisfied) |
| No change | < 100% | `ESCALATE` (no change was the wrong outcome) |
| No open intent | — (still validates) | `NO_OP` (backlog exhausted; satisfaction reported) |
| Agent raised a question | — | `NEEDS_DECISION` (relayed, with options) |
| Machinery failed / no verdict | — | `RETRYABLE` |

Two things worth noting:

- **`PR_READY` requires exactly 100%** — never a threshold. Anything less is not done.
- **`NO_OP` is earned, not asserted.** Even when the agent changes nothing, `run` still validates, so `NO_OP` always means "the scenarios actually pass," never "the agent claimed there was nothing to do." When the backlog is exhausted at 100%, the bundle marks it as the completion signal; exhausted below 100% is a quiet alarm (an incomplete backlog or a regression).

---

## Project layout

`factory init myapp` creates:

```
./myapp/                         # CODE ROOT (a git repo; the agent works here)
├── SPEC.md                      # what you're building (you fill this in)
├── BACKLOG.md                   # intents, one `- [ ]` line each, paired to scenarios
├── PROGRESS.md                  # the loop's working memory (what's done + learnings)
├── CLAUDE.md                    # the agent's working agreement (workflow + rails)
├── adr/
│   └── 0001-record-architecture-decisions.md
├── .gitignore
└── .git/

<data-dir>/factory/
├── registry.json                # the one piece of global state factory owns
└── factories/myapp/             # HOLDOUT ROOT (the agent never sees this)
    ├── judge.md                 # judge instructions
    ├── scenarios/               # your held-out scenarios (you write these)
    └── evidence/                # one bundle per run (created at runtime)
```

`<data-dir>` is your platform data directory — `~/.local/share` on Linux, `~/Library/Application Support` on macOS — overridable with `FACTORY_HOME` (see [Configuration](#configuration)).

An **evidence bundle** (`evidence/<run_id>/bundle.json`) records, at minimum: the terminal state, a summary, the intent, the agent's log, the validation result (satisfaction + per-scenario satisfied flags and transcripts), the change (committed flag + diff), and residual risk.

---

## Writing scenarios

Scenarios are the contract and the holdout. Each lives as a markdown file in the holdout root's `scenarios/` dir, and the agent never sees it.

A good scenario is **concrete** and **behavior-level** — it describes what the app should observably do, not how it's implemented. A loose, suggestive scenario is one a model can quietly satisfy without doing the real work. The shape:

```markdown
# S001 — short title of the behavior
Pairs with: B1

## Driver
How to exercise the app (the commands to run, the inputs to give).

## Steps
1. Concrete steps the judge follows.

## Expected observable behavior
- What must be observably true afterward — output, files, exit codes, etc.

## Not satisfied if
- The conditions that make this a fail.
```

The **judge** (`judge.md`, also in the holdout root) is instructed to drive the app per the scenario, observe actual behavior, and return a strict JSON verdict — never reading source, never giving benefit of the doubt, never modifying anything to make something pass:

```json
{ "scenario_id": "S001", "satisfied": true, "transcript": "...", "evidence": "...", "notes": "..." }
```

---

## Configuration

`factory` is configured by environment variables and a couple of flags.

| Variable | Purpose |
|---|---|
| `FACTORY_HOME` | Relocates the registry and all holdout roots. Set it to an isolated dir for sandboxed/test runs; one variable sandboxes the whole registry + factory tree. |
| `FACTORY_AGENT_SCRIPT` | The command the **scripted agent** runs (its effect on the working tree is the implementation). Used for deterministic testing. |
| `FACTORY_JUDGE_SCRIPT` | Path to the JSON verdict file the **scripted judge** reads. |

| Flag | Purpose |
|---|---|
| `--agent <claude\|scripted>` | Select the agent provider (default: Claude Code). |
| `--judge <claude\|scripted>` | Select the judge provider (default: Claude Code). |
| `--greenfield` / `--brownfield` | Project mode at `init` (mutually exclusive). |

The registry lives at `<data-dir>/factory/registry.json` unless `FACTORY_HOME` overrides it.

---

## Providers

The coding agent and the judge are each accessed through a swappable interface, so the same outer loop works regardless of what's behind them.

**Agent providers.** The real provider is **Claude Code** (`claude`), which runs in the code root and leaves its work as a changed working tree — `factory` never interprets the agent's output as edit instructions, it observes the tree via git. The **scripted** provider (`--agent scripted`) runs whatever command you put in `FACTORY_AGENT_SCRIPT`; there is no special "mode," the command itself is the behavior. It's used to drive `run` deterministically in tests (correct implementation, no-op, or a snoop), with no model call.

**Judge providers.** The real judge is **Claude Code** (`claude -p`), observing behavior and returning a verdict. The **scripted** judge (`--judge scripted`) reads canned verdicts from `FACTORY_JUDGE_SCRIPT`, so validation is deterministic and free in tests. `factory` derives the satisfaction fraction itself and ignores any self-reported count, so a scripted verdict claiming 100% over a 1-of-2 set still records 50%.

A real `run` uses the Claude Code agent and judge by default and costs tokens. The scripted providers exist so the tool's own scenarios (and yours) can be exercised without live, paid, nondeterministic calls.

---

## The holdout boundary

The holdout's strength is stated honestly, because a boundary you've mislabeled is worse than one you understand.

In v0 the boundary is **construction, not enforcement**. The agent is never *handed* the holdout: its environment is scrubbed of `FACTORY_*` variables, its working directory is the code root, and the holdout path is resolved internally and never passed in. A `run` also refuses to launch — emitting `ESCALATE` — if the holdout is structurally misconfigured (nested inside the code tree), with the path comparison canonicalized so symlinks can't fool it.

This reliably stops an honest agent from stumbling onto the scenarios while it works. It is **not** an OS sandbox: a determined process that knew the convention path could still read it. Hardening this into a true security boundary — running the implement step in a Docker Sandbox microVM with only the code root mounted, so the holdout is genuinely unreachable — is designed in [ADR-0012](adr/0012-sandbox-isolation-upgrade.md) and triggered when the loop runs unattended or touches untrusted code.

---

## Status

v0 is complete and validated end-to-end against its own held-out scenarios — not by self-graded unit tests, but by running the real binary through the same held-out judging the tool itself implements.

Being honest about what v0 is and isn't:

- **One developer, one machine.** The registry lives in your OS data directory; there's no multi-user story and that's fine.
- **One real agent provider** (Claude Code). The seam for others exists; the switch doesn't.
- **`--once` only.** Each invocation does a single pass. The multi-iteration and unattended loops are deliberately deferred.
- **Construction boundary, not a sandbox** (see [The holdout boundary](#the-holdout-boundary)).

---

## Roadmap

From here, `factory` extends by being pointed at itself — every item is meant to be built *by* the factory, *on* the factory, which is also the cleanest test that the pattern holds. Each links to the ADR that records the decision.

**Autonomy and dependency-aware work.** A multi-iteration loop, then unattended operation (`--afk` / `--watch` / `--max-iters`), plus an integrity loop that compares the growing code against its ADRs and pulls drift back. A deeper change is moving the backlog from a linear list to a graph where intents declare dependencies, so the loop picks the next *unblocked* intent — the prerequisite for dependency-aware execution and any dependency *view*. Sequenced in [ADR-0014](adr/0014-post-v0-capability-ordering.md).

**Safety — Docker Sandboxes.** Upgrade the construction boundary to hypervisor isolation via Docker Sandboxes (`sbx`), as an optional `--sandbox` wrapper over the agent interface. [ADR-0012](adr/0012-sandbox-isolation-upgrade.md).

**Model heterogeneity and cost routing.** Different models for different roles — a judge from a different family than the implementer (anti-collusion), frontier models for implementation, local/cheap (Ollama via an OpenAI-compatible endpoint) for high-volume and low-stakes roles — plus routing between multiple coding agents. [ADR-0013](adr/0013-model-heterogeneity-and-provider-routing.md).

**Human-in-the-loop control plane.** When the loop escalates while unattended, hand off to a human: set the intent aside, notify the operator's phone, and accept tap-sized verdicts (`skip` / `retry` / `pause`) back through an async, file-based verdict store — no public ingress, designed to run on a home server. [ADR-0015](adr/0015-human-in-the-loop-control-plane.md).

**Scale.** Cross-project orchestration — workspace-per-repo, retries, review queues, cost limits — the registry's real payoff at fleet size.

**Visualization.** Read-only projections of the registry and evidence bundles — never a second source of truth: a per-run bundle review, a fleet/project progress view, and a run-provenance graph showing how a project reached its current state.

The multi-iteration loop is the convergence point — it's the trigger for the sandbox upgrade, cost-routing, the control plane, and dependency-aware work all at once. None of this is committed to dates.

---

## Design decisions

Every non-obvious decision is recorded as an ADR in [`adr/`](adr/), including the forks deliberately *not* taken and the conditions that would reopen them:

- **0001–0007** — the v0 foundation: swappable agent/judge interfaces, the holdout-by-construction boundary, the registry as the only owned state, dogfooding as the growth model, the Rust stack, the evidence bundle, the single-binary crate.
- **0008–0011** — build-time decisions: registry field ownership (measure vs. decide), where the repo gets created, the agent contract and git-based observation, and the threat model behind the construction boundary.
- **[0012](adr/0012-sandbox-isolation-upgrade.md)** — the Docker Sandboxes safety upgrade.
- **[0013](adr/0013-model-heterogeneity-and-provider-routing.md)** — model heterogeneity and provider routing.
- **[0014](adr/0014-post-v0-capability-ordering.md)** — post-v0 capability ordering.
- **[0015](adr/0015-human-in-the-loop-control-plane.md)** — the human-in-the-loop control plane.

---

## Contributing

`factory` builds itself: past v0, features are added by pointing the factory at its own codebase. If you're working on it by hand:

- Build and test with `cargo build` / `cargo test`. The agent and judge are trait interfaces with in-crate doubles, so the core loop is testable without live model calls.
- The tool's own held-out scenarios live in its holdout root and show how the scripted agent/judge providers are driven — use them as worked examples when adding behavior.
- Record non-obvious decisions as ADRs in `adr/`, including the forks you don't take. The decision trail is a first-class part of the project.

---

## Lineage

A small, personal take on a pattern other people articulated first. Worth reading:

- StrongDM's [Software Factory](https://factory.strongdm.ai/) — the principles, techniques, and Digital Twin Universe that defined the approach.
- Simon Willison's [writeup](https://simonwillison.net/2026/Feb/7/software-factory/) of it.
- Matt Wynne's [Don't Fear the Dark Factory](https://mattwynne.net/dont-fear-the-dark-factory) — the de-escalated, "it's TDD at a bigger scale" framing.
- Geoffrey Huntley's Ralph loop and Jesse Vincent's [Superpowers](https://github.com/obra/superpowers) — the loop and discipline layers this tool deliberately delegates to rather than reinventing.

---

## License

TODO — add a license before publishing. For a tool intended for distribution, MIT or Apache-2.0 are the usual defaults.
