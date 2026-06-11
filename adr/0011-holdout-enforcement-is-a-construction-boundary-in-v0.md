# ADR-0011 — Holdout enforcement is a construction boundary in v0

Refines ADR-0002 (how the boundary is enforced). Builds on ADR-0009, ADR-0010.

## Decision

v0 enforces the holdout as a **construction boundary**: a hygiene boundary against a
non-adversarial agent, not OS enforcement against a hostile one. It defends the real
v0 threat — an honest or over-eager agent stumbling onto the scenarios while working —
by never *handing* the agent a path to the factory root:

- **Clean launch.** The agent subprocess runs with `cwd = code_root` and with every
  `FACTORY_*` environment variable scrubbed (by prefix, so a future seam variable is
  covered automatically). The factory root path is not in the agent's environment,
  arguments, prompt, or working directory.
- **Run-time guard (defense-in-depth).** Before doing the run, `run` verifies the
  factory root is not inside the code root. The check **canonicalizes both paths
  first** (so the macOS `/var`→`/private/var` symlink and relative-vs-absolute
  differences cannot produce a false negative) and then tests containment. If the
  holdout is inside the code tree, or the boundary cannot be verified (a path is
  missing/unreadable), `run` refuses with **`ESCALATE`** — never `RETRYABLE`: a nested
  holdout is a *structural* misconfiguration, so retrying re-trips it identically and
  would spin an unattended loop forever. This check lives in `run`, not `init`,
  because the safety property must be verified at the moment it is depended on — it
  catches registry hand-edits, restored backups, and future layout drift, not just a
  fresh `init`.

These are independent layers that fail closed: env-scrub covers leakage, the layout
(factory root under `FACTORY_HOME`/data-dir, ADR-0003) covers path derivation, and the
guard covers "the layout silently became wrong."

## Known limit (accepted for v0)
The factory-root path is **not secret**. It is the documented convention
(`<data-dir>/factories/<app>/`) and appears in the very source the agent is building.
"Not handed to the agent" is not "unknown to the agent": an agent that deliberately
runs `ls` against the convention path, or hunts the raw filesystem for an absolute
path it was never given, would succeed. The construction boundary does not defend
against that; the deferred OS sandbox does.

Therefore the holdout-by-construction scenario (S006) must verify the **weak, true**
claim — that the boundary is *not exposed* through the launch (env / cwd / a relative
walk from the code root all dead-end) — and must **not** assert the strong claim that
the holdout is unreachable. A snoop that reads the convention path directly would
succeed; if S006 tried that, S006 should fail, not the boundary.

## Deferred upgrade (now recorded as ADR-0013)
Run the agent subprocess under an OS-enforced filesystem sandbox (Linux landlock /
macOS `sandbox-exec` / a container with the host filesystem read-only and the factory
root not mounted). It wraps the existing agent contract (`cwd = code_root`) without
changing it, slotting under this boundary as a second, stronger layer. Trigger: the
unattended/at-scale AFK loop ships, or untrusted code enters the loop — i.e. when
holdout-reach shifts from "unlikely accident" to "eventually happens." The concrete
realization (Docker Sandboxes / `sbx` microVM, as an optional `--sandbox` wrapper) is
ADR-0013.

## Consequences
- The agent launch (`command_in_code_root`) scrubs `FACTORY_*` by prefix; the judge,
  which is trusted and *is* allowed to see the holdout, is not scrubbed.
- `run` gains an early, canonical, fail-closed holdout guard that emits `ESCALATE`.
- v0 ships no OS-level sandbox; that is explicitly deferred, not forgotten.
