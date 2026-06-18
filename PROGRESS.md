# PROGRESS — `factory`

Ralph-style working memory. The implementer reads this at the start of every session
and updates it at the end. Newest entries on top. Keep entries short: what changed,
what was learned, what to watch next.

## Format
```
### <date> — <backlog id> — <terminal state>
- Did: <one or two lines>
- Learned: <anything that should change the spec, an ADR, or a scenario>
- Next: <the next intent, or the open question if blocked>
```

## Log
<!-- entries go here, newest first -->

### 2026-06-18 — dogfood #3 — actionable guidance on a dirty baseline
- Did: when `run` bails because the work tree is not clean, the bundle now says how to
  recover (commit / `git stash` / `git reset --hard`, then run again) and why (factory
  needs a clean baseline to attribute the agent's diff), instead of a bare "not clean"
  with a generic transient-retry residual. Stays RETRYABLE.
- Chose guidance-only over `--allow-dirty`/auto-stash: a dirty baseline conflates
  pre-existing changes with the agent's, muddying evidence attribution; automated
  set-aside belongs in the future multi-iteration loop / control plane (ADR-0016),
  not v0 `--once`.
- Test-first: dirty-tree run → RETRYABLE whose bundle text includes stash/reset + "run
  again". 72 unit + 3 integration + 2 progress tests green; clippy + fmt clean.
- Next: dogfood #4 (ls measured-vs-decided) — the last one; possibly working-as-intended.

### 2026-06-18 — dogfood #2 — enforce the no-self-commit agent contract
- Did: `run` captures HEAD before the agent and, if the agent self-committed (Claude
  does by default), `git reset --soft` back to the baseline so the change is left
  staged for factory to observe and commit (ADR-0009). New `git::head` / `reset_soft`.
  Fixes the screener-B1 finding where a self-commit left factory committing only build
  cruft and the evidence bundle missing the real diff.
- Test-first: a fake agent that self-commits now still yields PR_READY with the real
  diff in the bundle (was NoOp before). 71 unit + 3 integration + 2 progress tests
  green; clippy `-D warnings` clean.
- Also: fixed pre-existing rustfmt debt in the #1 progress-output files (main.rs,
  tests/progress_output.rs were committed unformatted) in a follow-up fmt commit.
- Next: dogfood #3 (recover from an interrupted run / dirty baseline), then #4.

### 2026-06-15 — ADR — integrated devenv environment + build scaffolding (ADR-0018)
- Added ADR-0018 (environment and build scaffolding via devenv). Two-phase decision
  sharing one `devenv.nix`. Phase 1 (v1 candidate): `run` executes the agent inside
  `devenv shell`; `validate` drives the app via `devenv up`/`devenv test` against a
  reproducible, known-good environment. `init` gains a devenv-aware path — greenfield
  lays down a starter `devenv.nix`, brownfield detects an existing one — giving the
  `--greenfield`/`--brownfield` flag its first real job. Phase 2 (post-v0): same
  `devenv.nix` emits release artifacts via `devenv build`/`devenv container`; introduces
  a pipeline-failure outcome the five terminal states don't yet cover. devenv is a
  power-up (optional/detected), not a requirement; host-only path is the fallback.
- Numbering: incoming file was authored as 0016 (already taken by the human-in-the-loop
  control plane) → renumbered to 0018. Fixed two mis-numbered cross-references in the
  content: `ADR-0012 (sandbox)` → ADR-0013, and `ADR-0014 (capability ordering)` →
  ADR-0015 (the repo's actual numbers). Added a back-reference in ADR-0013 noting that
  ADR-0018 absorbs the "validation environment" concern, leaving the sandbox focused on
  isolation/safety.
- BACKLOG post-v0 annotated (ADR-0018). Docs only; tests unaffected.
- Next: nothing queued in v0. Next post-v0 work is Phase 1 of ADR-0018 (devenv
  environment) — a precondition for meaningful autonomous `run` on real projects.

### 2026-06-12 — bugfix — real agent/judge integration (first live run)
- First real end-to-end `factory run` (the real provider path is never exercised by
  `cargo test`) failed RETRYABLE: the agent left an empty diff and the judge produced
  no parseable JSON. Root cause: bare `claude -p` runs in the default permission mode,
  which blocks file writes + Bash pending an approval that never comes headlessly — it
  exits 0 having done nothing. Confirmed with a direct probe (no flag → "write blocked
  pending permission", no file; `--dangerously-skip-permissions` → file written).
- Fix (test-first): agent (`agent_command`) and judge (`judge_command`) now pass
  `--dangerously-skip-permissions`, extracted into testable command-builders that
  assert the flag. Also hardened `parse_verdict` to extract the verdict object from
  prose / a ```json fence (string-aware brace matching, skips non-verdict objects)
  instead of strict-parsing the whole stdout. 67 unit + 3 integration tests (6 new),
  clippy `-D warnings` clean, fmt clean.
- ADR-0017 records the permission posture: the rejected `acceptEdits` fork and the
  safety tie-in (this is the autonomous posture that triggers the ADR-0013 sandbox —
  fine for attended v0 use, not for unattended/untrusted code).
- Found by dogfooding `factory` on a new app (`screener`); re-running its B1 with the
  fixed binary to confirm the live loop reaches PR_READY.

### 2026-06-10 — ADR — integrated human-in-the-loop control plane (ADR-0016)
- Added ADR-0016 (escalation hand-off via async notifications + a verdict store):
  under unattended operation ESCALATE becomes a loop→human→loop hand-off — a Notifier
  provider pushes skip/retry/pause taps to a phone; a file-based verdict store the loop
  reads between iterations records the human's decision. Design only; not built (sits
  on top of the multi-iteration loop).
- Numbering: authored as 0015 (taken by capability-ordering) → 0016. Reconciled its
  refs: "graph backlog (ADR-0014)" → ADR-0015 (both occurrences), and made the
  exit-code hard-dependency concrete — it points at ADR-0012 (and notes that bug is
  already fixed here).
- Notable tie-in: ADR-0016 names correct exit codes as hard dependency #1 — which is
  exactly the bug fixed earlier this session (ADR-0012). Build order it implies:
  exit codes [done] → multi-iteration loop → control plane.
- BACKLOG post-v0 annotated (ADR-0016). Docs only; tests green (61 unit + 3 integration).

### 2026-06-09 — ADR — integrated post-v0 capability ordering (ADR-0015)
- Added ADR-0015 (post-v0 capability ordering): sequences the graph-shaped backlog →
  dependency-aware selection → multi-iteration loop → dependency view, and records
  that the graph backlog ripples into `run` selection, terminal states (a new
  "blocked, not exhausted" stall distinct from the exhausted cases), and the integrity
  loop. Sequencing guidance only; nothing built.
- Numbering: authored as 0014 (already taken by model-routing) → 0015. Reconciled its
  internal refs to the *repo's* numbers: "ADR-0012 (sandbox)" → ADR-0013, "ADR-0013
  (model heterogeneity)" → ADR-0014 (left unfixed they'd have pointed at the exit-code
  ADR); also pinned the "B3/B4-era ADRs" mention to ADR-0009/0010.
- BACKLOG post-v0 now carries the ordered capability chain with the ADR-0015 pointer.
  Docs only; tests unaffected (61 unit + 3 integration green).

### 2026-06-09 — ADRs — integrated sandbox + provider-routing decisions
- Added two externally-authored ADRs to the log. Both are forward-looking; NEITHER is
  implemented (their triggers aren't met) — integration = recording the decisions and
  wiring cross-references, not building features.
  * ADR-0013 — Sandbox isolation upgrade (Docker Sandboxes / `sbx` microVM): the
    concrete realization of ADR-0011's deferred upgrade. Optional `--sandbox` wrapper
    over the ADR-0001 agent interface; holdout stays unmounted so it's unreachable in
    the VM. Gated on the `--afk`/untrusted-code trigger. ADR-0011's "deferred upgrade"
    section now points to it.
  * ADR-0014 — Model heterogeneity and provider routing: role→capability policy +
    judge-independence/calibration rules over the ADR-0001 seam. Tier 0 (judge on a
    different model family than the implementer) + Tier 1 (multiple harness agents,
    routable) accepted; Tier 2 (per-role models inside implement) deferred as a
    recorded fork (collides with ADR-0004). No v0 code change — Tier 0 can't be
    exercised until a second provider exists (post-v0).
- Numbering: the incoming files were authored 0012/0013, but 0012 was already taken
  this session by the exit-code ADR. By acceptance order they became 0013/0014; the
  model ADR's internal "sandboxed S006 (ADR-0012)" ref was updated to ADR-0013. No
  code references changed.
- BACKLOG post-v0 annotated with both (ADR-0013/0014). Tests unaffected (docs only):
  61 unit + 3 integration still green.

### 2026-06-09 — bugfix — run exit code + S002 plumbing/isolation split
- Bug 1 (real, fixed): `run` exited 0 for failure terminal states — `main`'s `run()`
  returned `Result<()>` and the Run arm returned `Ok(())` regardless of state, so
  ESCALATE/RETRYABLE printed and exited 0, lying to any `$?`-checking wrapper / the
  AFK loop. Fix: `TerminalState::exit_code()` (PR_READY/NO_OP→0, RETRYABLE→10,
  ESCALATE→11, NEEDS_DECISION→12; avoids 1=hard-error and 2=clap-usage), applied where
  `main` turns the run outcome into a `process::ExitCode`. ADR-0012 records the
  contract. `validate` stays 0 (it emits a measurement, not a terminal state). Tests:
  unit per state + a new `tests/exit_codes.rs` that drives the real binary to
  NO_OP/RETRYABLE/ESCALATE and asserts the code (guards the wiring that actually broke).
  Verified e2e: 0 / 10 / 11.
- Bug 2 (real, fixed): S002 conflated plumbing with isolation, and the scripted judge
  (which ignores scenarios + source) can only test plumbing — so the "judge handed
  only scenarios+driver, never source" claim was verified by nothing. Split per
  ADR-0008's measure-vs-plumbing line:
    * Plumbing stays on the scripted path: added a validate test through the REAL
      ScriptedJudge with a verdict file that lies (claims 2/2 + 100%) → still derives
      50%, two-entry bundle, last_satisfaction + last_run_at set, last_terminal_state
      untouched.
    * Isolation became an in-crate seam test on `build_prompt` (judge/real.rs): a
      source sentinel placed in the code root never appears in the judge prompt, while
      the scenarios + a black-box driver reference + the do-not-read-source directive
      do. Deterministic, no model call.
- Honest scope (ADR-0011-style): the judge's no-source-reading is `factory` not
  *handing* source (construction — tested above) PLUS the judge being *instructed* not
  to read it (discipline — the JudgeRequest still carries the code-root path so the
  model could read source if it disobeyed; unenforced in v0, like the deferred agent
  sandbox). The S002 scenario text must say this; proposed rewrite handed to the
  holdout owner (I can't write the held-out file).
- Totals: 61 unit + 3 integration tests; clippy `-D warnings` clean; fmt clean.

### 2026-06-08 — bugfix — backlog parser ignored HTML comments
- Did: `backlog::next_intent` was matching `- [ ]` lines even inside `<!-- ... -->`,
  so the scaffold's commented example intent was parsed as real work — a pristine
  `init` + `run` would pick `B1 (→ S001) — <short title>` and launch the agent on
  nonsense. Added `strip_html_comments` (handles multi-line + unterminated) before the
  scan; commented intents are now ignored. 3 regression tests; 58 total, clippy/fmt
  clean. Verified e2e: pristine init + run → NO_OP, no intent recorded.
- Note (not a bug, pre-existing B4 behavior): a brand-new project whose backlog has no
  open intent reports NO_OP via the no-open-intent branch — "BACKLOG COMPLETE" if the
  judge says 100%, "quiet alarm" otherwise. Distinguishing "empty/unwritten backlog"
  from "all items checked" is a possible future nicety; left as-is.

### 2026-06-08 — B6 — PR_READY (pending external S006 check) — v0 feature-complete
- Did: Enforced the holdout as a construction boundary (ADR-0011). Two layers:
  (1) the agent launch now scrubs `FACTORY_*` by PREFIX (future seams covered
  automatically) — `is_seam_var` + prefix removal in `command_in_code_root`; and
  (2) a run-time holdout guard, checked in `run` right after resolving the roots:
  it CANONICALIZES both paths (so the macOS /var→/private/var symlink can't hide a
  nesting) and ESCALATEs (never RETRYABLE) if the factory root is inside the code
  root or the boundary can't be verified — structural, fail-closed, catches registry
  hand-edits/restored-backups/layout drift between init and run. 55 unit tests (2
  new), clippy `-D warnings` clean, fmt clean.
- ADR-0011 recorded (refines ADR-0002): v0 boundary is HYGIENE (non-adversarial
  agent), not OS enforcement. Known limit (accepted): the factory-root path is NOT
  secret — it's the documented convention and appears in the source the agent builds;
  "not handed" ≠ "unknown". A deliberate `ls` of the convention path would succeed.
  S006 must therefore test the WEAK, true claim (boundary not exposed via env/cwd/
  relative-walk), NOT unreachability. OS sandbox (landlock/sandbox-exec/container)
  deferred to its own ADR; trigger = AFK loop ships or untrusted code enters.
  Pointer added in ADR-0002.
- Verified e2e: a snooping scripted agent sees FACTORY_HOME=[] and FACTORY_JUDGE_
  SCRIPT=[] (scrubbed), the env-derived scenario path resolves to a nonexistent
  /factories/... , and finds no registry in the code tree — every handed channel
  dead-ends. A registry hand-edited to nest the factory root inside the code root →
  ESCALATE with "holdout factory root is inside the code root", and the agent never
  ran.
- Could NOT self-verify (holdout rail): I can't run S006. The external harness should
  drive the real binary with a scripted agent that attempts the HANDED channels
  (read $FACTORY_HOME / the registry / a relative walk from cwd) and assert the snoop
  finds no scenarios. Do NOT assert unreachability of the convention path — that would
  succeed (see ADR-0011) and should fail S006, not the boundary.
- v0 status: all six backlog intents (B1–B6) implemented and locally validated;
  PR_READY pending the external scenario suite. Net new deps over the session:
  clap, serde/serde_json, directories, anyhow, comfy-table (+ tempfile dev-dep). No
  async (ADR-0005). ADRs added this build-out: 0008 (scripted judge), 0009 (agent
  contract + git observation), 0010 (validate-every-pass / NO_OP earned), 0011
  (holdout construction boundary).
- Next: nothing in v0. Post-v0 candidates (BACKLOG bottom): multi-iteration run loop,
  --max-iters/--watch/--afk, the OS-sandbox upgrade (ADR-0011 trigger), the agent-
  emitted-terminal-tag channel (ADR-0010 deferral), local-time rendering in `ls`.

### 2026-06-08 — B5 — PR_READY (pending external S005 check)
- Did: Implemented `factory ls`. Reads the registry and prints each app's mode, last
  satisfaction, last terminal state, and last run time as an aligned, borderless table
  (comfy-table), sorted by app name; `—` for never-run fields; "no registered
  projects" when empty. Data-gathering (`rows`) is separate from rendering (`render`)
  so a future `ls --json` is a small add. 53 unit tests (8 new), clippy `-D warnings`
  clean, fmt clean.
- Registry schema change (per human decision): split `last_run` into `last_run_id`
  (opaque evidence-bundle pointer, never parsed) + `last_run_at` (ISO-8601 UTC).
  `ls` reads `last_run_at`; the AFK loop later gets a real comparable timestamp. New
  `src/clock.rs`: `RunStamp{id,at}` + a dependency-free `iso8601_utc` (Hinnant
  civil-from-days), threaded through `run`/`validate` (core takes `&RunStamp`, so
  tests stay deterministic; only the binary edge calls `RunStamp::now`).
  **registry.json shape changed** — old files load via serde defaults (lose the old
  `last_run`); fresh sandboxed runs are unaffected. Flagged for re-check.
- New dependency: `comfy-table` (per human preference for a table crate over
  hand-rolled padding — gives data-driven column widths + correct alignment).
- Decisions honored: store UTC never local; stable sort (BTreeMap by name); gather/
  render split; no `--json` yet. Local-time rendering deferred (needs tz handling);
  `ls` shows the stored UTC (with `Z`).
- Verified e2e: empty → "no registered projects"; after init demo + run (PR_READY) and
  init widget (brownfield, never run), `ls` shows demo=100%/PR_READY/<UTC time> and
  widget=—/—/—; registry.json now carries last_run_id + last_run_at.
- Could NOT self-verify (holdout rail): I can't run S005. The external harness should
  init ≥1 app (optionally run/validate it), then assert `factory ls` lists each app
  with mode, last satisfaction, last terminal state, and last run time.
- Next (last v0 item): B6 — holdout-unreachable enforcement + the snooping scenario.
  The agent env scrub (SCRUBBED_ENV in src/agent/mod.rs, verified at B3) is where this
  builds; B6 hardens it (no path to the factory root) and proves a snoop fails.

### 2026-06-07 — B4 — PR_READY (pending external S004 check)
- Did: Made `run`'s NO_OP honest. `run` now validates on EVERY pass (even no-change):
  NO_OP is earned by a passing validation, never assumed from a clean tree (B3's
  emit-NO_OP-without-validating was a latent "grade your own homework" bug). New
  matrix: changed+100→PR_READY (commit); no-change+100→NO_OP (already satisfied);
  <100→ESCALATE (changed or not — ran, not passing, retry won't help); no open intent
  →validate (no agent)→NO_OP; machinery (agent can't run / no verdict / not-a-repo /
  dirty)→RETRYABLE. NO_OP/ESCALATE bundles now embed the validation + a reason that
  reads plainly. 48 unit tests (3 new), clippy `-D warnings` clean, fmt clean.
- ADR-0010 recorded (supersedes ADR-0009's "no diff → NO_OP" line): validate-every-
  pass; NO_OP earned; full matrix; no-open-intent → NO_OP with completion-signal vs
  quiet-alarm distinguished in the reason field; five terminal states stay closed (no
  DONE). Terminal-state authorship: NEEDS_DECISION is agent-authored (an answerable
  question); ESCALATE is run-originated for unexplained failure. Pointer added in 0009.
- Deferred (flagged, not silently invented): the "honor an agent-emitted terminal tag
  if present" seam has NO channel in v0 — run originates ESCALATE; a clean extension
  point is marked in run() for a later item.
- Completion signal: backlog-exhausted ≠ done. exhausted+100 → NO_OP reason "BACKLOG
  COMPLETE" (the success condition); exhausted+<100 → NO_OP reason "Quiet alarm"
  (incomplete backlog or regression). The registry only stores NO_OP for both; the
  distinction lives in the bundle reason (B5 `ls` reads the registry — accepted for v0).
- Verified e2e (scripted agent+judge): no-change+satisfied→NO_OP "already satisfied";
  no-change+failing→ESCALATE; exhausted+satisfied→NO_OP "BACKLOG COMPLETE" with the
  agent NOT invoked; exhausted+failing→NO_OP quiet alarm. Zero commits in every
  non-PR_READY case.
- Could NOT self-verify (holdout rail): I can't run S004. The external harness should
  drive the real binary with the scripted agent making no change + a scripted judge at
  100% → expect NO_OP whose bundle shows "no change; already satisfied" (NOT a forced
  green); and a no-change + <100% judge → expect ESCALATE, nothing committed.
- Next: B5 (`ls`) — read the registry and print each app's mode, last satisfaction,
  last terminal state, last run. Then B6 (holdout-unreachable enforcement + the
  snooping scenario; the agent env scrub from B3 is where it builds).

### 2026-06-07 — B3 — PR_READY (pending external S003 + S001 re-check)
- Did: Implemented `factory run <app> --once`. Agent is a trait (ADR-0001/0009) with
  `real` (`claude -p`) and `scripted` (`sh -c $FACTORY_AGENT_SCRIPT`) providers,
  selected by `--agent`/`FACTORY_AGENT`. An agent is a subprocess in the code root
  whose effect is the working-tree change; factory never interprets its stdout as
  edits. run: require repo → clean baseline → select next backlog intent → agent
  (seams scrubbed from its env) → observe diff via git → validate (shared `evaluate`)
  → one terminal state + run evidence bundle, committing only on PR_READY. New
  modules: git, agent/, backlog; RunBundle/TerminalState/decide in evidence + run.
  45 unit tests (20 new), clippy `-D warnings` clean, fmt clean.
- ADR-0009 recorded (amends ADR-0001): agent contract + git observation + the
  outcome→terminal mapping + the FACTORY_AGENT_SCRIPT seam isolation (ADR-0002).
  Cross-refs added to ADR-0001/0002.
- Terminal mapping (ADR-0009): no change / no intent → NO_OP; change + 100% →
  PR_READY (commit); change + <100% → ESCALATE (work failed, retry won't help, no
  commit); machinery failure (agent can't run, no verdict, not-a-repo, dirty tree) →
  RETRYABLE. PR_READY requires satisfaction == 100 exactly, never a threshold.
  Absence-of-verdict is RETRYABLE, NOT collapsed into a sub-100 satisfaction.
- B1 REOPENED (per human decision): `init` now `git init`s the code root and commits
  the scaffold, and the scaffold ships a `.gitignore` so `git add -A` never stages
  build artifacts. Repo creation belongs with scaffolding, not run; run only requires
  a repo. **S001 should be re-run** — the code root now has a `.git/` dir and a
  `.gitignore` it did not before.
- Honesty by construction: the bundle's `change.diff` IS the committed diff (same
  artifact), and the commit message cites the intent + scenario (history is evidence).
  Verified e2e with the scripted agent+judge: PR_READY commits the change (hash
  matches the bundle); ESCALATE leaves the change uncommitted but records the diff;
  NO_OP commits nothing; the agent subprocess sees `FACTORY_HOME` as empty (`seen=[]`).
- Could NOT self-verify (holdout rail): I can't run S003. The external harness should
  drive the real binary:
    `factory init <app>` (plant scenarios in its factory root + an open BACKLOG.md)
    `FACTORY_AGENT=scripted FACTORY_AGENT_SCRIPT='<writes good code>' \`
    `FACTORY_JUDGE=scripted FACTORY_JUDGE_SCRIPT=<verdict.json> \`
    `  factory run <app> --once`
  then assert PR_READY, a new commit whose diff equals the bundle's change.diff, and
  the evidence bundle. Also re-run S001 (B1 changed).
- Next: B4 (`run --once` can do nothing) — enrich NO_OP (e.g. confirm already-
  satisfied via validate before declaring no-op) and sharpen the RETRYABLE-vs-ESCALATE
  transient/structural split. Then B5 (`ls`) and B6 (holdout-unreachable enforcement +
  snooping scenario; the agent env scrub started here is where B6 builds).

### 2026-06-06 — B2 — PR_READY (pending external S002 check)
- Did: Implemented `factory validate <app>`. Judge is a trait (ADR-0001) with two
  shipped providers — `real` (spawns `claude -p` driven by judge.md + scenarios,
  default) and `scripted` (canned per-scenario verdicts from `FACTORY_JUDGE_SCRIPT`,
  trusted-runner-only). Provider selected by `--judge` flag or `FACTORY_JUDGE` env,
  default real. validate resolves the app from the registry, runs the judge, writes
  an evidence bundle (ADR-0006) at `factory_root/evidence/<run_id>/bundle.json`, and
  updates registry `last_satisfaction` + `last_run`. 25 unit tests total (12 new for
  B2), clippy `-D warnings` clean, fmt clean.
- Decision recorded: ADR-0008 (amends ADR-0001) — the judge gets a scripted provider
  symmetric to the agent; the seam is trusted-runner-only and unreachable by the
  implementer (ADR-0002 note, enforced in B6); scripted exercises plumbing, not
  judgment. Cross-ref lines added to ADR-0001 and ADR-0002.
- Validate semantics (human decision): records last_satisfaction + last_run + bundle
  ONLY. It does NOT emit or write a terminal state — terminal states are `run`'s
  exclusive output. `last_terminal_state` is left untouched by validate.
- Honesty by construction: `factory` DERIVES the satisfaction fraction from the
  judge's per-scenario booleans and IGNORES any self-reported total. Verified
  end-to-end: a verdict claiming `satisfaction:100` over 1-of-2 satisfied scenarios
  is recorded as 50%. All-pass→100, all-fail→0. Satisfied vs unsatisfied scenarios
  are recorded differently in the bundle (boolean + distinct `observed` transcript).
- Loud failures (no false greens): scripted without FACTORY_JUDGE_SCRIPT, unknown
  provider, and unregistered app all exit 1 with clear messages and no artifacts.
- Could NOT self-verify: per the holdout rail I can't run S002. The external harness
  should drive the real binary with the scripted judge, e.g.:
    `cargo build --release`
    `FACTORY_HOME=<tmp> factory init <app>`  (plant scenarios in its factory root)
    `FACTORY_JUDGE=scripted FACTORY_JUDGE_SCRIPT=<verdict.json> factory validate <app>`
  then assert the printed fraction, the bundle under evidence/, and registry status.
  If S002 passes against a stub or a fabricated number survives, the scenario is too
  loose — tighten it before B3.
- Next: B3 (`run --once` happy path) — introduces the agent trait + scripted agent
  provider, the run terminal states, and committing on PR_READY. validate's bundle
  is the sub-bundle a run bundle will wrap (the evidence module is built to extend).

### 2026-06-05 — B1 — PR_READY (pending external red-check)
- Did: Stood up the single binary crate (clap + serde_json + directories + anyhow).
  Implemented `factory init <app> [--greenfield|--brownfield]`: scaffolds the code
  root at `./<app>/` and the factory root at `<FACTORY_HOME|data-dir>/factories/<app>/`
  from embedded templates (`include_str!`), and upserts the app into the registry at
  `<home>/registry.json`. Idempotent (re-init overwrites templates + re-registers,
  single entry). 13 unit tests, clippy `-D warnings` clean, `fmt` clean.
- Layout decision (human): code root under CWD; factory root + registry both under
  one `FACTORY_HOME` (so a single env var isolates everything for sandboxed runs).
  Strongest holdout: factory tree lives outside the code tree.
- Templates: blank skeleton FORMS for SPEC/BACKLOG/PROGRESS/adr-0001 with a single
  `{{app}}` token; `CLAUDE.md` (generalized working agreement, stack note dropped)
  and `judge.md` (observe-behavior-only, report each scenario honestly, emit JSON
  evidence) ship VERBATIM to every line.
- INTENTIONAL, not a regression: `validate`, `run`, `ls` are declared in the clap
  surface (SPEC freezes four commands) but are explicit FAILING stubs — exit 1 with
  a clear "not implemented (backlog Bn)" message and zero side effects. A stub that
  quietly exits 0 could give a false green against a held-out scenario; that is
  exactly what the holdout design exists to prevent. Do NOT "fix" these to no-op.
- Could NOT self-verify: per the holdout rail (ADR-0002) I cannot run S002–S006.
  They MUST be red against these stubs by construction (the stubs emit no
  satisfaction value, evidence bundle, or terminal state). Human/external harness
  must run the red-check; if any of S002–S006 passes against a stub, the scenario is
  too loose and should be tightened before the command is built.
- Next: B2 (`factory validate`) — introduces the judge trait (ADR-0001) + scripted
  provider seam; first command to read/write the registry status fields + evidence.
