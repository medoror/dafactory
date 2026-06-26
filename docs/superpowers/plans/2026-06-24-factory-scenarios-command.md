# `factory scenarios` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `factory scenarios <app>` — a command that validates the spec and backlog are ready, copies them into the holdout root, and writes a scenario-authoring CLAUDE.md there, so the user can open a fresh Claude session in the holdout root with full context to draft scenarios.

**Architecture:** New `src/commands/scenarios.rs` module following the existing command pattern (registry lookup via `Paths`, anyhow errors, outcome struct). The holdout CLAUDE.md is a new embedded template in `templates/factory/CLAUDE.md`, exposed as a pub const in `templates.rs` and substituted at runtime. BACKLOG.md validation reuses `backlog::next_intent` to honor the existing comment-stripping logic.

**Tech Stack:** Rust, clap, anyhow, std::fs. No new dependencies.

---

## File map

| Action | File |
|---|---|
| Create | `adr/0019-scenarios-command-scenario-authoring-bootstrapping.md` |
| Create | `templates/factory/CLAUDE.md` |
| Modify | `src/templates.rs` — add `SCENARIO_CLAUDE` pub const |
| Modify | `src/cli.rs` — add `Scenarios` variant |
| Create | `src/commands/scenarios.rs` |
| Modify | `src/commands/mod.rs` — expose `scenarios` module |
| Modify | `src/main.rs` — dispatch `Scenarios` |
| Modify | `README.md` — Commands section, Quick start, workflow table |
| Modify | `BACKLOG.md` — add new intent |
| Modify | `PROGRESS.md` — add entry |

---

## Task 1: Write ADR-0019

**Files:**
- Create: `adr/0019-scenarios-command-scenario-authoring-bootstrapping.md`

- [ ] **Step 1: Create the ADR**

Write `adr/0019-scenarios-command-scenario-authoring-bootstrapping.md` with this exact content:

```markdown
# ADR-0019 — `factory scenarios`: scenario-authoring context bootstrapping

## Status
Accepted.

## Context
After `factory init`, the workflow breaks in two places:

1. The user writes the spec and backlog in the **code root** with agent help — Claude has
   full context there.
2. The user must then write held-out scenarios in the **holdout root** (a separate
   directory the agent never touches), but Claude has no context there: it doesn't know
   the spec, the intent IDs, the scenario format, or why this session must stay separate
   from implementation.

The ergonomic fix considered first — drafting scenarios in the code root and copying them
to the holdout — was ruled out because scenarios drafted in the same Claude session as
implementation work contaminate that session's context window even after the files are
deleted from disk. The session is the boundary, not the filesystem location.

## Decision

Add `factory scenarios <app>`. It:

1. Validates the code root has a filled-in SPEC.md (presence of `<!-- One paragraph`
   means it's still the empty stub → error).
2. Validates BACKLOG.md has at least one open intent (using the same comment-stripping
   logic as `backlog::next_intent`, so commented-out scaffold examples don't count).
3. Copies SPEC.md and BACKLOG.md from the code root into the holdout root. This is
   **safe**: these files are not secret — the implementing agent already reads them. Only
   scenarios are secret, and they always flow the other way (stay in the holdout).
4. Writes a `CLAUDE.md` to the holdout root from an embedded template. This CLAUDE.md:
   - States the role (scenario authoring only)
   - Warns about session discipline (close this session when done, never use it for
     implementation — the holdout boundary is only as strong as this discipline)
   - Contains the scenario format from the README
   - Tells Claude to read the SPEC.md and BACKLOG.md copies in the same directory
5. Prints the holdout path with instructions to open a fresh Claude session there.

The command is idempotent: re-running overwrites the copies and the CLAUDE.md.

## Why the copy is one-directional

SPEC.md and BACKLOG.md are not secrets. The implementing agent reads them during `run`.
Copying them into the holdout root does not weaken the holdout boundary — only scenarios
are secret, and they are never written into the code root. The copy direction is always
code root → holdout root, never the reverse.

## Why a separate command rather than a side effect of `init` or `run`

- `init` is too early: SPEC.md and BACKLOG.md don't exist yet.
- `run` is too late: scenario authoring must happen *before* the first `run`, and
  silently copying context on `run` would not surface the session-discipline guidance
  at the moment the user needs it.
- A separate command makes the handoff explicit and surfaces the right guidance
  (the holdout CLAUDE.md) at the right moment.

## Consequences

- v0 gains a fifth command. The "four commands, frozen" posture in SPEC.md was a scope
  guard against feature creep; this command is a workflow necessity that was not visible
  until the first real end-to-end authoring session. The SPEC.md out-of-scope list
  (`factory spec`, `factory scenario add`) remains off-limits; this is not in that class.
- The holdout root now contains SPEC.md, BACKLOG.md, and CLAUDE.md in addition to
  judge.md and scenarios/. All are authored by `factory`, not the implementing agent.
- The session-discipline warning is in CLAUDE.md (the holdout root), not enforced
  programmatically. The boundary is still construction, not OS enforcement (ADR-0011).

## Alternatives considered

- **Draft in the code root, manually copy.** Rejected: even with file deletion, scenario
  content in the context window of a session used for implementation contaminates it.
- **Holdout CLAUDE.md only (no file copy).** Rejected: forces the user to paste spec and
  backlog into the holdout session manually, losing the ergonomic benefit.
- **Auto-copy on `factory run`.** Rejected: too late for scenario authoring, and the
  session-discipline guidance is not surfaced at the right moment.
```

- [ ] **Step 2: Verify the file exists**

```bash
ls adr/0019-scenarios-command-scenario-authoring-bootstrapping.md
```

Expected: the file is listed.

- [ ] **Step 3: Commit**

```bash
git add adr/0019-scenarios-command-scenario-authoring-bootstrapping.md
git commit -m "Add ADR-0019: factory scenarios command"
```

---

## Task 2: Create the holdout CLAUDE.md template

**Files:**
- Create: `templates/factory/CLAUDE.md`

- [ ] **Step 1: Write the template**

Write `templates/factory/CLAUDE.md` with this exact content:

````markdown
# Scenario-authoring working agreement for `{{app}}`

You are helping write held-out scenarios for `{{app}}`. This is a **scenario-authoring
session only** — not an implementation session.

## Session discipline (critical)

**Close this session when you finish writing scenarios. Never use it for implementation
or for `factory run` work.** The holdout boundary is only as strong as this discipline:
if scenarios exist in your context window when you implement, the agent can draw on them
even if the files are deleted from disk. Start a fresh session for any implementation
work.

## What you have here

- `SPEC.md` — the contract for this version (copied from the code root by `factory scenarios`)
- `BACKLOG.md` — the open intents, each paired to a scenario id (copied from the code root)
- `scenarios/` — write held-out scenario files here (one per intent)
- `judge.md` — the judge's working agreement (do not modify)

## Your task

Read `SPEC.md` and `BACKLOG.md`. For each open `- [ ]` intent in the backlog, draft a
scenario file in `scenarios/`. Name each file after its id (e.g. `S001.md`, `S002.md`).

Scenarios are the concrete, behavior-level acceptance checks that the implementing agent
never sees. The agent is judged against them from the outside. If a scenario is vague or
generous, a model can satisfy it without doing the real work — every loose scenario is a
hole in the holdout.

## Scenario format

```markdown
# S001 — short title of the behavior
Pairs with: B1

## Driver
How to exercise the app (the exact command to run, the exact inputs to give).

## Steps
1. Concrete, numbered steps the judge follows.

## Expected observable behavior
- What must be true afterward — stdout content, exit code, files created, etc.
- Be exact. "contains X" is better than "shows something about X".

## Not satisfied if
- The conditions that make this a fail, stated explicitly.
- "I could not tell" is unsatisfied, not satisfied.
```

## What makes a good scenario vs. a loose one

**Good:** specific, observable, behavior-level. The judge can verify it by running the
app and observing what it does — no source reading required.

**Loose:** vague, implementation-level, or generous. Examples of loose scenarios the agent
can quietly satisfy without doing real work: "the output looks reasonable", "the command
succeeds", "the app handles the case correctly". Each of these grants benefit of the doubt
the holdout is supposed to deny.

Ask for each scenario: "could the judge verify this purely by running the app and
observing its behavior, without reading source?" If no, the scenario is mis-written.
````

- [ ] **Step 2: Verify**

```bash
grep "{{app}}" templates/factory/CLAUDE.md
```

Expected: at least two matches (the title and the intro line).

- [ ] **Step 3: Commit**

```bash
git add templates/factory/CLAUDE.md
git commit -m "Add holdout CLAUDE.md template for scenario authoring"
```

---

## Task 3: Expose the template in `templates.rs`

**Files:**
- Modify: `src/templates.rs`

- [ ] **Step 1: Write the failing test**

Add to `src/templates.rs` in the `#[cfg(test)]` block:

```rust
#[test]
fn should_expose_scenario_claude_template_with_app_token() {
    assert!(SCENARIO_CLAUDE.contains("{{app}}"));
    assert!(SCENARIO_CLAUDE.contains("scenario"));
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test --lib templates::tests::should_expose_scenario_claude_template_with_app_token 2>&1 | cat
```

Expected: compile error — `SCENARIO_CLAUDE` not found.

- [ ] **Step 3: Add the pub const**

Add after the `FACTORY_TEMPLATES` constant in `src/templates.rs`:

```rust
/// The scenario-authoring working agreement written into the holdout root by
/// `factory scenarios`. Not part of `FACTORY_TEMPLATES` (written by `init`) — it
/// is written by `scenarios` once the spec and backlog are ready.
pub const SCENARIO_CLAUDE: &str = include_str!("../templates/factory/CLAUDE.md");
```

- [ ] **Step 4: Run to verify it passes**

```bash
cargo test --lib templates 2>&1 | cat
```

Expected: all templates tests pass including the new one.

- [ ] **Step 5: Commit**

```bash
git add src/templates.rs
git commit -m "Expose SCENARIO_CLAUDE template constant"
```

---

## Task 4: Add `Scenarios` to the CLI surface

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Add the variant**

In `src/cli.rs`, add to the `Commands` enum after the `Ls` variant:

```rust
/// Copy spec and backlog into the holdout root and write a scenario-authoring
/// CLAUDE.md so you can open a fresh session there to draft scenarios.
Scenarios {
    /// Name of the registered app.
    app: String,
},
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build 2>&1 | cat
```

Expected: compile warning about non-exhaustive match in `main.rs` (the new variant is unhandled). No other errors.

- [ ] **Step 3: Commit**

```bash
git add src/cli.rs
git commit -m "Add Scenarios variant to CLI surface"
```

---

## Task 5: TDD — `scenarios.rs` error paths

**Files:**
- Create: `src/commands/scenarios.rs`

- [ ] **Step 1: Write the failing tests**

Create `src/commands/scenarios.rs` with only the tests and a stub:

```rust
//! `factory scenarios <app>`: copy spec and backlog into the holdout root and write
//! a scenario-authoring CLAUDE.md so the user can open a fresh session there.

use std::path::PathBuf;

use anyhow::Result;

use crate::paths::Paths;

pub struct ScenariosOutcome {
    pub factory_root: PathBuf,
}

pub fn scenarios(paths: &Paths, app: &str) -> Result<ScenariosOutcome> {
    todo!("not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::init;
    use crate::registry::Mode;
    use std::fs;

    fn setup(dir: &std::path::Path) -> (Paths, PathBuf, PathBuf) {
        let paths = Paths::new(dir.join("home"), dir.join("work"));
        let code_root = paths.code_root("myapp");
        let factory_root = paths.factory_root("myapp");
        init::init(&paths, "myapp", Mode::Greenfield).unwrap();
        (paths, code_root, factory_root)
    }

    #[test]
    fn should_error_when_app_not_registered() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path().join("home"), dir.path().join("work"));

        let err = scenarios(&paths, "unregistered").unwrap_err();

        assert!(
            err.to_string().contains("not registered"),
            "expected 'not registered' in: {err}"
        );
    }

    #[test]
    fn should_error_when_spec_md_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, code_root, _) = setup(dir.path());
        fs::remove_file(code_root.join("SPEC.md")).unwrap();

        let err = scenarios(&paths, "myapp").unwrap_err();

        assert!(
            err.to_string().to_lowercase().contains("spec"),
            "expected spec mention in: {err}"
        );
    }

    #[test]
    fn should_error_when_spec_md_is_unfilled_template() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, _, _) = setup(dir.path());
        // init writes the template stub which still contains the placeholder comment

        let err = scenarios(&paths, "myapp").unwrap_err();

        assert!(
            err.to_string().to_lowercase().contains("spec"),
            "expected spec mention in: {err}"
        );
    }

    #[test]
    fn should_error_when_backlog_md_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, code_root, _) = setup(dir.path());
        // Fill in SPEC.md so it passes that check
        fs::write(
            code_root.join("SPEC.md"),
            "# SPEC\n\n## What this is\n\nA real app.\n",
        )
        .unwrap();
        fs::remove_file(code_root.join("BACKLOG.md")).unwrap();

        let err = scenarios(&paths, "myapp").unwrap_err();

        assert!(
            err.to_string().to_lowercase().contains("backlog"),
            "expected backlog mention in: {err}"
        );
    }

    #[test]
    fn should_error_when_backlog_has_no_open_intents() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, code_root, _) = setup(dir.path());
        // Fill in SPEC.md so it passes that check
        fs::write(
            code_root.join("SPEC.md"),
            "# SPEC\n\n## What this is\n\nA real app.\n",
        )
        .unwrap();
        // Backlog with only checked items and no open intents
        fs::write(
            code_root.join("BACKLOG.md"),
            "# BACKLOG\n\n- [x] **B1 — done.**\n",
        )
        .unwrap();

        let err = scenarios(&paths, "myapp").unwrap_err();

        assert!(
            err.to_string().to_lowercase().contains("backlog")
                || err.to_string().to_lowercase().contains("intent"),
            "expected backlog/intent mention in: {err}"
        );
    }
}
```

- [ ] **Step 2: Expose the module and run**

Add `pub mod scenarios;` to `src/commands/mod.rs`, then:

```bash
cargo test --lib commands::scenarios 2>&1 | cat
```

Expected: all 5 error-path tests fail with "not yet implemented" (panics from `todo!()`).

- [ ] **Step 3: Implement the validation logic**

Replace the `scenarios` stub in `src/commands/scenarios.rs`:

```rust
pub fn scenarios(paths: &Paths, app: &str) -> Result<ScenariosOutcome> {
    use anyhow::bail;
    use std::fs;

    let registry = crate::registry::Registry::load(&paths.registry_path())?;
    let entry = registry
        .apps
        .get(app)
        .ok_or_else(|| anyhow::anyhow!("app '{app}' is not registered — run `factory init {app}` first"))?;
    let code_root = entry.code_root.clone();
    let factory_root = entry.factory_root.clone();

    // Validate SPEC.md
    let spec_path = code_root.join("SPEC.md");
    if !spec_path.is_file() {
        bail!("SPEC.md not found in the code root — write it before drafting scenarios");
    }
    let spec = fs::read_to_string(&spec_path)
        .with_context(|| format!("failed to read {}", spec_path.display()))?;
    if spec.contains("<!-- One paragraph") {
        bail!("SPEC.md still contains the unfilled template stub — fill in the 'What this is' section before drafting scenarios");
    }

    // Validate BACKLOG.md
    let backlog_path = code_root.join("BACKLOG.md");
    if !backlog_path.is_file() {
        bail!("BACKLOG.md not found in the code root — write it before drafting scenarios");
    }
    let backlog = fs::read_to_string(&backlog_path)
        .with_context(|| format!("failed to read {}", backlog_path.display()))?;
    if crate::backlog::next_intent(&backlog).is_none() {
        bail!("BACKLOG.md has no open intents — add at least one `- [ ]` item (outside HTML comments) before drafting scenarios");
    }

    todo!("copy files — implemented in next task")
}
```

Add `use anyhow::Context;` to the imports at the top:

```rust
use anyhow::{Context, Result};
```

- [ ] **Step 4: Run error-path tests**

```bash
cargo test --lib commands::scenarios 2>&1 | cat
```

Expected: the 5 error-path tests pass; the happy-path tests (not yet written) are absent; `todo!` panic for the copy step is fine — no happy-path tests call it yet.

- [ ] **Step 5: Commit**

```bash
git add src/commands/scenarios.rs src/commands/mod.rs
git commit -m "Add scenarios command: validation error paths"
```

---

## Task 6: TDD — `scenarios.rs` happy path

**Files:**
- Modify: `src/commands/scenarios.rs`

- [ ] **Step 1: Write the failing happy-path tests**

Add these tests to the `#[cfg(test)]` block in `src/commands/scenarios.rs`:

```rust
fn setup_ready(dir: &std::path::Path) -> (Paths, PathBuf, PathBuf) {
    let paths = Paths::new(dir.join("home"), dir.join("work"));
    let code_root = paths.code_root("myapp");
    let factory_root = paths.factory_root("myapp");
    init::init(&paths, "myapp", Mode::Greenfield).unwrap();
    // Write a filled-in spec and a backlog with one open intent
    fs::write(
        code_root.join("SPEC.md"),
        "# SPEC\n\n## What this is\n\nA real app.\n",
    )
    .unwrap();
    fs::write(
        code_root.join("BACKLOG.md"),
        "# BACKLOG\n\n- [ ] **B1 (→ S001) — Greet by name.**\n",
    )
    .unwrap();
    (paths, code_root, factory_root)
}

#[test]
fn should_copy_spec_and_backlog_into_factory_root() {
    let dir = tempfile::tempdir().unwrap();
    let (paths, code_root, factory_root) = setup_ready(dir.path());

    scenarios(&paths, "myapp").unwrap();

    let copied_spec = fs::read_to_string(factory_root.join("SPEC.md")).unwrap();
    let original_spec = fs::read_to_string(code_root.join("SPEC.md")).unwrap();
    assert_eq!(copied_spec, original_spec);

    let copied_backlog = fs::read_to_string(factory_root.join("BACKLOG.md")).unwrap();
    let original_backlog = fs::read_to_string(code_root.join("BACKLOG.md")).unwrap();
    assert_eq!(copied_backlog, original_backlog);
}

#[test]
fn should_write_claude_md_into_factory_root() {
    let dir = tempfile::tempdir().unwrap();
    let (paths, _, factory_root) = setup_ready(dir.path());

    scenarios(&paths, "myapp").unwrap();

    let claude_md = fs::read_to_string(factory_root.join("CLAUDE.md")).unwrap();
    assert!(claude_md.contains("myapp"), "app name should be substituted");
    assert!(
        claude_md.contains("scenario"),
        "CLAUDE.md should mention scenarios"
    );
    assert!(
        claude_md.contains("session"),
        "CLAUDE.md should mention session discipline"
    );
    assert!(
        !claude_md.contains("{{app}}"),
        "template token should be substituted"
    );
}

#[test]
fn should_return_the_factory_root_path() {
    let dir = tempfile::tempdir().unwrap();
    let (paths, _, factory_root) = setup_ready(dir.path());

    let outcome = scenarios(&paths, "myapp").unwrap();

    assert_eq!(outcome.factory_root, factory_root);
}

#[test]
fn should_be_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let (paths, _, _) = setup_ready(dir.path());

    scenarios(&paths, "myapp").unwrap();
    // Second call should also succeed without error
    scenarios(&paths, "myapp").unwrap();
}
```

- [ ] **Step 2: Run to verify they fail**

```bash
cargo test --lib commands::scenarios 2>&1 | cat
```

Expected: the 4 new tests fail (panic at `todo!`).

- [ ] **Step 3: Implement the copy and write logic**

Replace the `todo!("copy files — implemented in next task")` line in the `scenarios` function with:

```rust
    // Copy spec and backlog into the holdout root
    fs::copy(&spec_path, factory_root.join("SPEC.md"))
        .with_context(|| "failed to copy SPEC.md to factory root")?;
    fs::copy(&backlog_path, factory_root.join("BACKLOG.md"))
        .with_context(|| "failed to copy BACKLOG.md to factory root")?;

    // Write the scenario-authoring CLAUDE.md from the embedded template
    let claude_md = crate::templates::SCENARIO_CLAUDE.replace("{{app}}", app);
    fs::write(factory_root.join("CLAUDE.md"), claude_md)
        .with_context(|| "failed to write CLAUDE.md to factory root")?;

    Ok(ScenariosOutcome { factory_root })
```

- [ ] **Step 4: Run all scenarios tests**

```bash
cargo test --lib commands::scenarios 2>&1 | cat
```

Expected: all 9 tests pass (5 error-path + 4 happy-path).

- [ ] **Step 5: Run the full test suite**

```bash
cargo test 2>&1 | cat
```

Expected: all tests pass, no clippy errors.

- [ ] **Step 6: Run clippy and fmt**

```bash
cargo clippy -- -D warnings 2>&1 | cat
cargo fmt --check 2>&1 | cat
```

Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add src/commands/scenarios.rs
git commit -m "Implement scenarios command: copy and write happy path"
```

---

## Task 7: Wire `Scenarios` in `commands/mod.rs` and `main.rs`

**Files:**
- Modify: `src/commands/mod.rs` — already done in Task 5 Step 2
- Modify: `src/main.rs`

- [ ] **Step 1: Add the dispatch arm to `main.rs`**

In the `run` function in `src/main.rs`, add this arm to the `match cli.command` block after the `Ls` arm:

```rust
        Commands::Scenarios { app } => {
            let paths = Paths::resolve()?;
            let outcome = commands::scenarios::scenarios(&paths, &app)?;
            println!("Ready to draft scenarios for '{app}'.");
            println!(
                "  Open a fresh Claude session in: {}",
                outcome.factory_root.display()
            );
            println!("  CLAUDE.md in that directory has the scenario format and session guidance.");
            println!("  Close that session when done — never use it for `factory run` work.");
            Ok(ExitCode::SUCCESS)
        }
```

- [ ] **Step 2: Verify it compiles and tests pass**

```bash
cargo test 2>&1 | cat
```

Expected: all tests pass.

- [ ] **Step 3: Smoke-test the binary**

```bash
cargo build 2>&1 | cat
./target/debug/factory scenarios --help 2>&1 | cat
```

Expected: help text mentions `scenarios` and its `<app>` argument.

- [ ] **Step 4: Run clippy and fmt**

```bash
cargo clippy -- -D warnings 2>&1 | cat
cargo fmt --check 2>&1 | cat
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "Wire factory scenarios command dispatch"
```

---

## Task 8: Update `README.md`

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add `factory scenarios` to the Commands section**

In the `## Commands` section of `README.md`, add after the `### factory init` entry and before `### factory validate`:

```markdown
### `factory scenarios <app>`
Copies `SPEC.md` and `BACKLOG.md` from the code root into the holdout root and writes
a scenario-authoring `CLAUDE.md` there. Run this after you have written the spec and
backlog. Then open a **fresh** Claude session in the printed holdout path — the
`CLAUDE.md` there has the scenario format and the session-discipline guidance. Close
that session when done; never use it for `factory run` work. The command validates that
the spec has been filled in and the backlog has at least one open intent before copying.
Re-running is idempotent.
```

- [ ] **Step 2: Update the Quick start section**

In `## Quick start`, update step 4 (currently "Write the held-out scenario(s)") to:

```markdown
### 4. Bootstrap the scenario-drafting context

```bash
factory scenarios myapp
```

This copies your spec and backlog into the holdout root and writes a `CLAUDE.md` there
with the scenario format and session-discipline guidance. Open a **fresh** Claude session
in the path it prints — this is a different session from the one you used to write the
spec. Draft your scenarios there, then close that session.
```

- [ ] **Step 3: Update the "Who writes what" table**

Add a row to the table under `### Who writes what`:

```markdown
| `factory scenarios <app>` → opens holdout context | held-out scenarios (you, in the fresh session) |
```

The full updated table should be:

```markdown
| You write, by hand | `factory` / the agent produce |
|---|---|
| `SPEC.md` — the contract for this version | the code (the agent) |
| `BACKLOG.md` intents | `PROGRESS.md` working memory (the agent) |
| held-out scenarios + `judge.md` (in the holdout root, using the context `factory scenarios` sets up) | the terminal state + evidence bundle (`factory`) |
```

- [ ] **Step 4: Verify the README renders correctly**

```bash
grep -n "factory scenarios" README.md | cat
```

Expected: at least 3 occurrences (Commands section, Quick start, table).

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "Update README for factory scenarios command"
```

---

## Task 9: Update `BACKLOG.md` and `PROGRESS.md`

**Files:**
- Modify: `BACKLOG.md`
- Modify: `PROGRESS.md`

- [ ] **Step 1: Add the intent to BACKLOG.md**

Add before `## Post-v0` in `BACKLOG.md`:

```markdown
- [x] **B7 (→ S007) — `scenarios` bootstraps the holdout for scenario authoring.**
  `factory scenarios <app>` copies the spec and backlog into the holdout root, writes
  a scenario-authoring CLAUDE.md, and prints the holdout path. Errors if the spec is
  still the empty template or the backlog has no open intents.
```

- [ ] **Step 2: Add a PROGRESS.md entry**

Add at the top of the log in `PROGRESS.md` (newest first):

```markdown
### 2026-06-24 — B7 — PR_READY

- Did: Implemented `factory scenarios <app>`. Validates SPEC.md is filled in (errors on
  the unfilled stub marker `<!-- One paragraph`) and BACKLOG.md has open intents (via
  `backlog::next_intent`, same comment-stripping as `run`). Copies both files into the
  holdout root and writes a scenario-authoring CLAUDE.md from a new embedded template.
  ADR-0019 records the decision (one-directional copy is safe; session is the boundary,
  not the filesystem; separate command is the right trigger point). 9 unit tests (5
  error-path + 4 happy-path), all tests green, clippy/fmt clean.
- Learned: the "four commands, frozen" SPEC posture was a scope guard, not a permanent
  ceiling — a fifth command warranted an ADR explaining why it's in scope.
- Next: no open v0 items. Next post-v0 work per PROGRESS.md order: devenv Phase 1
  (ADR-0018), then unattended loop + control plane (ADR-0016) + sandbox (ADR-0013).
```

- [ ] **Step 3: Commit**

```bash
git add BACKLOG.md PROGRESS.md
git commit -m "Update backlog and progress for B7 (factory scenarios)"
```

---

## Self-review

**Spec coverage:**
- `factory scenarios <app>` command exists — Task 4 + 7 ✓
- Validates spec filled in — Task 5 ✓
- Validates backlog has open intents — Task 5 ✓
- Copies SPEC.md and BACKLOG.md to holdout root — Task 6 ✓
- Writes CLAUDE.md with scenario format and session discipline — Tasks 2, 6 ✓
- Prints holdout path with instructions — Task 7 ✓
- Idempotent — Task 6 ✓
- ADR recorded — Task 1 ✓
- README updated — Task 8 ✓

**Placeholder scan:** No TBDs, no "handle edge cases", no "similar to Task N" — all code blocks are complete.

**Type consistency:** `ScenariosOutcome { factory_root: PathBuf }` defined in Task 5, used identically in Tasks 6 and 7. `scenarios(paths: &Paths, app: &str) -> Result<ScenariosOutcome>` consistent throughout.
