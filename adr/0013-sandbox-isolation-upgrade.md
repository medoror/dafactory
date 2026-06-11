# ADR-0013 — Sandbox isolation upgrade (Docker Sandboxes)

## Status
Deferred — planned, not implemented in v0. This is the concrete realization of the
upgrade ADR-0011 points to. The construction boundary (ADR-0002, ADR-0011) remains
the active boundary until the trigger below is met.

## Decision
Harden the agent's execution boundary from *construction* to *hypervisor
enforcement* by running the implement step inside a Docker Sandbox (`sbx`) microVM.
Implemented as an optional `--sandbox` wrapper over the existing agent provider
interface (ADR-0001) that composes with any provider, rather than as a new provider
variant. `factory`, the registry, the holdout, and validation all remain on the
host; only the implement step is pushed into the sandbox.

## Why
v0's holdout boundary is hygiene, not enforcement: the agent is not *handed* the
holdout, but as a host subprocess it could read the convention path if it knew it
(this is the honest weak claim S006 tests, per ADR-0011). That is sufficient against
an honest, non-adversarial agent. It is not sufficient once the loop runs unattended
or touches untrusted code, where an agent reaching the holdout shifts from "unlikely
accident" to "eventually happens."

Docker Sandboxes provides the missing layer: full hypervisor-level microVM
isolation, each sandbox with its own Docker daemon, filesystem, and network. The key
property for us follows directly from ADR-0003 (registry/holdout live in the OS data
dir, outside the code tree) plus how sandboxes mount: the workspace is mounted into
the VM via filesystem passthrough at the same absolute path, but only the workspace
is mounted. Because the holdout lives outside the code tree, it is not part of the
mounted workspace and therefore has no passthrough into the VM. The agent cannot
read the holdout because it does not exist inside the sandbox.

So the upgrade requires no change to the holdout layout. The construction boundary
("not handed to the agent") becomes a hypervisor boundary ("not reachable from the
agent"), purely by routing the implement step through `sbx`.

## Mechanics that make it compatible
- **Passthrough mount, same absolute path, instant bidirectional.** After the
  sandboxed agent edits the code root, the changes are already on the host at the
  same path. `factory`'s git-diff observation, the commit, and the evidence bundle
  all continue to run on the host exactly as in v0 — the agent-as-subprocess
  contract (ADR-0001) is preserved; only where the subprocess runs changes.
- **Isolated daemon + network.** Outbound traffic routes through a proxy on the host
  that handles credential injection, so the agent's API auth is the sandbox's
  responsibility, not `factory`'s.

## Consequences
- **Wrapper, not a provider variant.** `--sandbox` is orthogonal and composes with
  any agent provider (`claude+sandbox`, `codex+sandbox`), avoiding an N×M provider
  explosion. It lives behind the ADR-0001 interface as a decorator over the chosen
  provider.
- **Optional and graceful.** `factory` works with no `sbx` installed (construction
  boundary). `--sandbox` upgrades to hypervisor isolation when `sbx` is present.
  Sandboxing is opt-in; recommended-on for unattended operation, not worth the VM
  overhead for interactive `--once` runs.
- **Validation stays on the host.** Only the implement step is sandboxed. A useful
  side effect: the agent's environment (sandbox) and the validation environment
  (host) differ, so anything the agent installs only inside the sandbox does not
  persist — host validation will fail any change that is not self-contained. This is
  a feature, not a bug; it catches "works in my sandbox" changes.
- **Lifecycle tied to the loop, not the single run.** A microVM carries real
  resource overhead. `sbx` lets a sandbox be stopped and restarted without
  recreation, with setup carried over, until `sbx rm`. So the multi-iteration /
  unattended loop should create a sandbox once, reuse it across iterations, and
  remove it at the end — not spin up and tear down per `run`.
- **S006 gains a strong-claim tier.** Under `--sandbox`, the strong claim ("the agent
  cannot reach the holdout") becomes true, so a sandboxed S006 variant can have the
  snoop try the convention path directly and assert it fails — a test that passes
  only under the microVM boundary, never under construction-only. This closes the
  honesty gap ADR-0011 deliberately left open, by adding a second tier rather than
  by overclaiming the first.

## Trigger
Implement when the unattended/at-scale loop (`--afk` / multi-iteration) ships, or
when `factory` is pointed at untrusted code. Until then the construction boundary
stands and this ADR is informational.

## Open question to resolve before implementation
The exact non-interactive invocation. The documented entry point is interactive
(`sbx run <agent>`); the headless/scripted passthrough the loop needs (equivalent of
`claude -p "..."` non-interactively) must be confirmed. The likely escape hatch is
the supported Shell / custom-environment agent: run the same command `factory` would
have run on the host, inside the VM (in spirit, `sbx run shell -- <agent-cmd>`),
which preserves the agent contract exactly. Verify the headless flag and the
env/credential-injection specifics against the `sbx` CLI reference and the per-agent
Claude Code page before building.

## Risks
- **Early Access / Experimental.** The `sbx` CLI surface will change. This is the
  primary reason the integration is an optional, swappable wrapper behind one
  interface — when `sbx` moves, only that wrapper changes.
- **Added dependencies.** Requires `sbx login` and a Docker account; Linux requires
  KVM (macOS uses its own hypervisor). Acceptable for a personal tool; a real
  consideration for distribution, which is why it stays optional.
- **Resource overhead.** A VM plus its own daemon per sandbox — mitigated by the
  loop-scoped lifecycle above.

## Alternatives considered
- **Plain container with a mounted Docker socket.** Only partial (namespace)
  isolation and a shared host daemon — inappropriate for an autonomous, potentially
  untrusted agent.
- **Run `factory` itself inside the sandbox.** Rejected: the holdout would have to
  be inside the sandbox for `factory` to validate, which puts the agent and the
  holdout back in the same environment and collapses the boundary entirely.
- **Stay construction-only.** Acceptable for v0 and interactive use; insufficient
  once the loop is unattended or the code is untrusted — which is exactly the
  trigger condition above.

## References
- Docker Sandboxes overview: https://docs.docker.com/ai/sandboxes/
- Architecture (workspace mounting, networking, lifecycle):
  https://docs.docker.com/ai/sandboxes/architecture/
- Security model / isolation:
  https://docs.docker.com/ai/sandboxes/security/
- CLI reference (verify invocation): https://docs.docker.com/reference/cli/sbx
