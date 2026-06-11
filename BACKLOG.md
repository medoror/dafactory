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

Human-in-the-loop escalation control plane (ADR-0016): async notifier provider +
verdict store (skip/retry/pause) so unattended ESCALATE becomes a phone hand-off.
Sits on top of the multi-iteration loop (built order: correct exit codes [done,
ADR-0012] → loop → control plane); this is what `factory resume` hangs off.
