# CI Pipeline Design — factory

**Date:** 2026-06-27
**Status:** Approved

## Summary

Two GitHub Actions workflow files with clean, independent triggers: `ci.yml` validates
every PR and push to `main`; `release.yml` fires on tag pushes produced by
`cargo-release`. No `if:` conditions routing jobs within a shared workflow. No
auto-tagging inside CI. No crates.io publishing yet. Devenv integration deferred to
ADR-0018 Phase 2.

## Files

| File | Purpose |
|---|---|
| `.github/workflows/ci.yml` | Check job — triggers on PR and push to `main` |
| `.github/workflows/release.yml` | Publish job — triggers on tag push `v*.*.*` |
| `rust-toolchain.toml` | Pins stable toolchain with rustfmt and clippy components |
| `release.toml` | cargo-release config: publish=false, tag format, commit message |
| `adr/0020-ci-pipeline-and-release-via-cargo-release.md` | Companion ADR |

## Jobs

### `ci.yml` — check

**Triggers:** `pull_request`, `push` to `main`

**Steps:**
1. `cargo fmt --check` — formatting gate
2. `cargo clippy -- -D warnings` — lint gate (warnings are errors)
3. `cargo test` — full test suite

**Caching:** `Swatinem/rust-cache` caches Cargo registry and build artifacts between
runs. Significant speedup since factory's dependencies are stable.

### `release.yml` — publish

**Triggers:** tag push matching `v*.*.*`

**Steps:**
1. `cargo build --release` — verify the release build compiles

No artifact upload and no `cargo publish` yet. The file exists as a clean, verified
gate on every release tag, ready to extend when crates.io publishing is added.

Each file has a single, unambiguous `on:` trigger — no `if:` conditions routing jobs
within a shared workflow.

## Toolchain

`rust-toolchain.toml` at the project root pins the stable channel and declares the
`rustfmt` and `clippy` components. GitHub Actions picks this up automatically via
`dtolnay/rust-toolchain`. Ensures CI and local dev use identical toolchains.

## Release workflow (developer-facing)

`cargo-release` is a local CLI tool. The developer runs it; CI never auto-tags.

```bash
cargo install cargo-release   # one-time setup

cargo release patch           # 0.0.0 → 0.0.1: bumps Cargo.toml, commits, tags, pushes
cargo release minor           # 0.0.1 → 0.1.0
cargo release major           # 0.1.0 → 1.0.0
```

The tag push triggers the `publish` job. The commit message and tag format are
configured in `release.toml`.

`release.toml` config:
- `publish = false` — no crates.io upload until explicitly enabled
- `tag-name = "v{{version}}"` — standard semver tag format
- `pre-release-commit-message = "Release {{version}}"` — clean commit history

## Upgrade paths

**crates.io:** Set `publish = true` in `release.toml` and add `CARGO_REGISTRY_TOKEN`
to GitHub repository secrets. No other changes required.

**devenv (ADR-0018 Phase 2):** The `publish` job's build step may eventually become
`devenv build` when a `devenv.nix` is present. The job is intentionally thin now to
make that substitution straightforward.

## What this is not

- No auto-tagging on merge to `main` — `cargo-release` is the release mechanism
- No release binary artifacts yet — deferred alongside devenv
- No matrix builds (multi-platform) — deferred; both jobs run on `ubuntu-latest`
  (cheaper, faster; no platform-specific code in factory)
