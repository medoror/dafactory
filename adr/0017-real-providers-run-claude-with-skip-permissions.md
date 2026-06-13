# ADR-0017 — The real agent and judge run `claude -p --dangerously-skip-permissions`

Builds on ADR-0001 (the real provider spawns `claude -p`), ADR-0009 (the agent
contract), ADR-0011 (construction boundary), ADR-0013 (the deferred OS sandbox).

## Status
Accepted for v0. Discovered by the first real end-to-end `factory run` — the real
provider path is not exercised by `cargo test` (which uses in-crate doubles and the
scripted providers), so this gap was latent until a live run.

## Context
The real agent (`src/agent/real.rs`) and real judge (`src/judge/real.rs`) invoke
`claude -p`. On their first real use both produced nothing: the agent left an empty
diff (→ RETRYABLE) and the judge emitted prose instead of JSON (→ parse failure).

Root cause, confirmed by a direct probe: headless `claude -p` runs in the default
permission mode, which blocks file writes and tool/Bash calls pending an interactive
approval that never arrives non-interactively. It exits 0 having done nothing. With
`--dangerously-skip-permissions` the same invocation writes files and runs commands.

## Decision
Launch both real providers with `--dangerously-skip-permissions`:
- the **agent** must write files and run its tests (TDD) autonomously;
- the **judge** must run the app (Bash) to observe its external behavior.

`--permission-mode acceptEdits` was considered and **rejected**: it auto-accepts file
edits but still blocks Bash, so the agent could not run its tests and the judge could
not drive the app — insufficient for headless operation.

Fixed alongside: `parse_verdict` now extracts the verdict object from prose / a
```` ```json ```` fence (string-aware brace matching, skipping non-verdict objects)
instead of assuming the whole stdout is JSON, because `claude -p` narrates around its
answer. The strict-parse path is preserved for clean JSON.

## Consequences / safety
- This grants the agent unrestricted command execution in the code root. Combined
  with the construction-only holdout boundary (ADR-0011 — the factory-root path is
  not secret), this is exactly the autonomous posture ADR-0013 names as the trigger
  for OS-level sandboxing.
- Acceptable for v0's **interactive, attended, trusted-code** use: a human runs
  `--once` and reviews the result. It is **not** acceptable for the unattended/at-scale
  loop or untrusted code — that is the trigger to land the ADR-0013 sandbox, inside
  which `--dangerously-skip-permissions` is *contained by the microVM* rather than
  trusted.
- The flag is unconditional in v0. The obvious next refinement is to gate it (e.g.
  require `--sandbox`, or an explicit opt-in for unattended runs). Recorded, not built.
