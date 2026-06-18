# BACKLOG — `factory` v0

Ordered intents. The implementer pulls the next unaddressed item. Each is paired to
a held-out scenario by id; the scenario (which you cannot see) is what judges it.
Phrasing here is intentionally loose — implement the intent, not a guess at the
assertion.

- [x] **B1 (→ S001) — `init` scaffolds a line.**
  `factory init <app>` creates the code root and the factory root from templates and
  registers the app. After it runs, both roots exist with their expected files and
  the registry lists the app.

- [x] **B2 (→ S002) — `validate` runs the held-out judge.**
  `factory validate <app>` drives the app's behavior, judges it against the held-out
  scenarios without reading source, writes a satisfaction fraction, and writes an
  evidence bundle. A satisfied scenario and an unsatisfied scenario are reported
  honestly and differently.

- [x] **B3 (→ S003) — `run --once` happy path.**
  `factory run <app> --once` selects the next backlog intent, delegates
  implementation to the agent, runs validate, and on a genuinely passing result
  emits `PR_READY` with a committed change and an evidence bundle.

- [x] **B4 (→ S004) — `run --once` can do nothing.**
  When the correct outcome is no change (the intent needs no work, or is already
  satisfied), `run --once` emits `NO_OP` with an evidence bundle explaining why,
  commits nothing, and never forces a green result.

- [x] **B5 (→ S005) — `ls` lists lines and their state.**
  `factory ls` prints every registered app with mode, last satisfaction, last
  terminal state, and last run time.

- [x] **B6 (→ S006) — holdout is unreachable by the implementer.**
  During `run`, the implementer subprocess has no filesystem path to the factory
  root; an attempt to read the scenarios from inside the implementer fails.

## Post-v0 (do not start — sequenced for dogfooding)
multi-iteration `run` loop · `--max-iters` · `--watch` · `--afk` · integrity/ADR-drift
loop · `factory status` · `factory resume` · `factory spec` · `factory scenario add`
· second agent provider · judge-model decoupling + provider routing (ADR-0014, Tier
0/1) · `--sandbox` hypervisor isolation (ADR-0013; gated on the `--afk`/untrusted-code
trigger).

Capability chain — must land in this order (ADR-0015): graph-shaped backlog →
dependency-aware selection → multi-iteration/unattended loop → dependency view. The
dependency view is the tail of this chain, not a standalone visualization task; the
graph backlog also reshapes `run` selection, terminal states (a new "blocked, not
exhausted" stall), and the integrity loop.

Devenv environment + build scaffolding (ADR-0018): two phases sharing one `devenv.nix`.
Phase 1 (v1 candidate, foundational) — `run` executes the agent inside `devenv shell`;
`validate` drives the app via `devenv up`/`devenv test` against a reproducible environment
instead of whatever is on the host. `init` gains a devenv-aware path: greenfield lays down
a starter `devenv.nix`; brownfield detects an existing one (the first real job for the
`--greenfield`/`--brownfield` flag). devenv is a power-up, not a requirement; host-only
path is the fallback. Phase 2 (post-v0, alongside release/shipping) — same `devenv.nix`
emits release artifacts via `devenv build`/`devenv container`. Verify before building:
confirm non-interactive `devenv shell` agent invocation and `devenv test` judge driving.

Human-in-the-loop escalation control plane (ADR-0016): async notifier provider +
verdict store (skip/retry/pause) so unattended ESCALATE becomes a phone hand-off.
Sits on top of the multi-iteration loop (built order: correct exit codes [done,
ADR-0012] → loop → control plane); this is what `factory resume` hangs off.

## Discovered by dogfooding (screener build, 2026-06)
First real end-to-end runs against a live app surfaced these. Concrete fixes / UX, not
new capabilities — and good first dogfood targets once `run` can build them.

- **`run` emits progress.** A pass is silent for minutes during the agent build and the
  judge, so a healthy run reads as a hang and gets Ctrl-C'd — which leaves a dirty tree.
  Emit progress to stderr (selecting intent → running agent → validating → terminal
  state), or at least a heartbeat. Root cause of the interrupted-run mess on screener B2.
- **Enforce the no-self-commit contract in `run`.** ADR-0009 assumes the agent leaves an
  uncommitted working-tree change, but `claude` self-commits by default; today only a
  CLAUDE.md instruction (ADR-0017 era) discourages it. When the agent commits, factory's
  `git add -A` sees only post-commit cruft, so the evidence bundle's `change.diff` misses
  the real change (observed on screener B1). Fix: detect a moved HEAD after the agent step
  and `git reset --soft` back so the changes return to the working tree for factory to
  observe and commit. Makes the evidence model robust, not advisory.
- **Recover from an interrupted run / dirty baseline.** A Ctrl-C'd pass leaves a dirty
  tree; the next run only reports "working tree was not clean" with no remedy. Add a
  guarded recovery (auto-stash, an opt-in `--allow-dirty`, or clear remediation guidance)
  so an interrupted pass is not manual git surgery.
- **Surface measured-vs-decided in `ls` (minor).** After a `validate` (or a salvage),
  `ls` shows the last *run's* terminal state, which can lag the satisfaction `validate`
  just refreshed (screener showed SAT 100% but LAST STATE RETRYABLE). May be
  working-as-intended (ADR-0010/0012); if kept, distinguish "satisfaction measured at T"
  from "last run state" so they are not conflated.
