# CI Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a two-file GitHub Actions CI pipeline with a check job (PR + main) and a release job (tag push), backed by cargo-release for version management.

**Architecture:** `ci.yml` triggers on PRs and pushes to main and runs fmt/clippy/test. `release.yml` triggers on `v*.*.*` tag pushes and runs a release build. `cargo-release` is the local developer tool that bumps `Cargo.toml`, commits, tags, and pushes — CI never auto-tags. `rust-toolchain.toml` pins the toolchain so CI and local dev are identical.

**Tech Stack:** GitHub Actions, `dtolnay/rust-toolchain`, `Swatinem/rust-cache`, `cargo-release`, `actionlint` (local validation)

---

## File Map

| Action | Path | Purpose |
|---|---|---|
| Create | `rust-toolchain.toml` | Pin stable toolchain + rustfmt + clippy for CI and local dev |
| Create | `release.toml` | cargo-release config: publish=false, tag format, commit message |
| Create | `.github/workflows/ci.yml` | Check job — fmt, clippy, test on PR and push to main |
| Create | `.github/workflows/release.yml` | Publish job — release build on tag push `v*.*.*` |

---

## Task 1: Pin the Rust toolchain

**Files:**
- Create: `rust-toolchain.toml`

- [ ] **Step 1: Create `rust-toolchain.toml`**

  Create at the repo root:

  ```toml
  [toolchain]
  channel = "stable"
  components = ["rustfmt", "clippy"]
  ```

- [ ] **Step 2: Verify rustup picks it up**

  Run from the repo root:

  ```bash
  rustup show active-toolchain
  ```

  Expected: output contains `stable` and the toolchain path.

  Then verify components are available:

  ```bash
  cargo fmt --version | cat
  cargo clippy --version | cat
  ```

  Expected: both print a version string without error.

- [ ] **Step 3: Commit**

  ```bash
  git add rust-toolchain.toml
  git commit -m "Add rust-toolchain.toml pinning stable with rustfmt and clippy"
  ```

---

## Task 2: Configure cargo-release

**Files:**
- Create: `release.toml`

- [ ] **Step 1: Install cargo-release (if not already installed)**

  ```bash
  cargo install cargo-release | cat
  ```

  Verify:

  ```bash
  cargo release --version | cat
  ```

  Expected: prints version like `cargo-release 0.25.x`.

- [ ] **Step 2: Create `release.toml`**

  Create at the repo root:

  ```toml
  publish = false
  tag-name = "v{{version}}"
  pre-release-commit-message = "Release {{version}}"
  ```

- [ ] **Step 3: Verify cargo-release reads the config**

  Run a dry-run from the repo root (does NOT modify any files or push anything):

  ```bash
  cargo release patch --dry-run 2>&1 | cat
  ```

  Expected: output shows the planned actions — bumping version in `Cargo.toml`,
  a commit message of `Release 0.0.1`, and a tag of `v0.0.1`. It should NOT
  say "will publish to crates.io".

  If it says it will publish, double-check `release.toml` has `publish = false`.

- [ ] **Step 4: Commit**

  ```bash
  git add release.toml
  git commit -m "Add release.toml configuring cargo-release"
  ```

---

## Task 3: CI workflow — check job

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create the `.github/workflows/` directory and `ci.yml`**

  ```bash
  mkdir -p .github/workflows
  ```

  Create `.github/workflows/ci.yml`:

  ```yaml
  name: CI

  on:
    pull_request:
    push:
      branches: [main]

  jobs:
    check:
      name: Check
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@stable
          with:
            components: rustfmt, clippy
        - uses: Swatinem/rust-cache@v2
        - name: Format
          run: cargo fmt --check
        - name: Clippy
          run: cargo clippy -- -D warnings
        - name: Test
          run: cargo test
  ```

- [ ] **Step 2: Validate the YAML with actionlint**

  Install actionlint if not present:

  ```bash
  brew install actionlint
  ```

  Run:

  ```bash
  actionlint .github/workflows/ci.yml
  ```

  Expected: no output (actionlint is silent on success). Any output is an error
  to fix before committing.

  If actionlint is unavailable, validate YAML structure manually:

  ```bash
  python3 -c "import yaml, sys; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo "YAML valid" | cat
  ```

- [ ] **Step 3: Commit**

  ```bash
  git add .github/workflows/ci.yml
  git commit -m "Add CI workflow with fmt, clippy, and test gates"
  ```

---

## Task 4: Release workflow — publish job

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create `.github/workflows/release.yml`**

  ```yaml
  name: Release

  on:
    push:
      tags:
        - 'v*.*.*'

  jobs:
    publish:
      name: Publish
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@stable
        - uses: Swatinem/rust-cache@v2
        - name: Build release
          run: cargo build --release
  ```

- [ ] **Step 2: Validate the YAML with actionlint**

  ```bash
  actionlint .github/workflows/release.yml
  ```

  Expected: no output.

  Fallback if actionlint is unavailable:

  ```bash
  python3 -c "import yaml, sys; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "YAML valid" | cat
  ```

- [ ] **Step 3: Commit**

  ```bash
  git add .github/workflows/release.yml
  git commit -m "Add release workflow triggered by version tags"
  ```

---

## Task 5: Verify CI triggers on push

- [ ] **Step 1: Push main to GitHub**

  ```bash
  git push origin main | cat
  ```

- [ ] **Step 2: Confirm the CI workflow triggered**

  Open the repository on GitHub → Actions tab. You should see a new `CI` workflow
  run triggered by the push to `main`. Wait for it to complete.

  Expected: all three steps (Format, Clippy, Test) pass green.

  If any step fails:
  - **Format:** run `cargo fmt` locally, commit the fix, push again
  - **Clippy:** run `cargo clippy -- -D warnings | cat` locally and fix the warnings
  - **Test:** run `cargo test | cat` locally and investigate failures

- [ ] **Step 3: Verify release workflow trigger with a dry-run tag**

  > **Note:** This pushes a real tag to GitHub. Delete it after confirming.

  First set `Cargo.toml` version to something testable. Check current version:

  ```bash
  grep '^version' Cargo.toml | cat
  ```

  Run cargo-release dry-run to confirm what it will do:

  ```bash
  cargo release patch --dry-run 2>&1 | cat
  ```

  Expected: shows it will create tag `v0.0.1` (or next patch from current version).

  When ready to cut the first real release, run without `--dry-run`:

  ```bash
  cargo release patch
  ```

  Then go to GitHub → Actions and confirm the `Release` workflow triggered on the
  tag push and the `Build release` step passes.

---

## Upgrade path notes (do NOT implement now)

These are recorded here for future reference only:

**Enable crates.io publishing:**
1. Set `publish = true` in `release.toml`
2. Add `CARGO_REGISTRY_TOKEN` to GitHub repository secrets (Settings → Secrets)
3. Add this step to `release.yml` after the build step:
   ```yaml
   - name: Publish to crates.io
     run: cargo publish
     env:
       CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
   ```

**Enable devenv build (ADR-0018 Phase 2):**
Replace the `Build release` step in `release.yml` with `devenv build` when
`devenv.nix` exists in the project.
