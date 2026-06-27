# ADR-0020 — CI pipeline and release via cargo-release

## Status
Accepted.

## Context

ADR-0018 (Phase 2) explicitly defers the CI/release stage: *"PR_READY → 'actually
shipped' (tag, CI, build the cross-compiled release artifact, publish) is an unmodeled
stage."* This ADR fills that gap at the current scope — validation gate on PRs and a
release path via `cargo-release` — while keeping the devenv upgrade path open.

Two constraints drive the design:

1. **crates.io compatibility.** `cargo publish` requires `Cargo.toml` version to match
   the release tag exactly. Any scheme that leaves `Cargo.toml` permanently at `0.0.0`
   (e.g., git-tags-as-sole-source-of-truth) breaks publishing. The version source of
   truth must be `Cargo.toml`.

2. **No commit-back loops.** CI writing back to `main` (bumping `Cargo.toml` and
   committing) is fragile: it relies on `[skip ci]` conventions, creates noisy commits,
   and can loop. Preferred: the developer controls the release commit.

## Decision

### cargo-release as the release mechanism

`cargo-release` is a local CLI tool run by the developer. One command
(`cargo release patch|minor|major`) bumps `Cargo.toml`, commits with a clean message,
tags `vX.Y.Z`, and pushes the tag. CI never auto-tags. The developer controls when and
at what version a release happens.

This satisfies both constraints: `Cargo.toml` is always accurate (no publishing mismatch)
and CI never commits back to `main`.

### Single workflow file, two jobs

One `.github/workflows/ci.yml` with two independently-triggered jobs:

- **`check`** — triggers on PR and push to `main`. Runs `cargo fmt --check`,
  `cargo clippy -- -D warnings`, `cargo test`. This is the merge gate.
- **`publish`** — triggers on tag push matching `v*.*.*`. Runs `cargo build --release`.
  No `cargo publish` yet; `publish = false` in `release.toml` until explicitly enabled.

Two separate workflow files would require keeping toolchain versions in sync across
files. One file, two jobs with different trigger conditions is simpler and sufficient.

### rust-toolchain.toml

Pins the stable channel plus `rustfmt` and `clippy` components. Ensures CI and local
dev use identical toolchains. `dtolnay/rust-toolchain` picks this up automatically.

### Caching via Swatinem/rust-cache

Standard Rust CI caching action. Caches Cargo registry and incremental build artifacts.
Factory's dependencies are stable, so cache hit rate will be high.

## Relationship to other ADRs

- **ADR-0018 (devenv):** This ADR implements the CI side of Phase 2. The `publish` job
  is intentionally thin so that `devenv build` can replace `cargo build --release` when
  Phase 2 is fully built. The devenv GitHub Actions integration is the long-term path
  for cross-compiled release artifacts.
- **ADR-0005 (Rust stack):** The cross-compiled distribution goal referenced in ADR-0005
  is deferred here; this ADR establishes the CI foundation that cross-compilation will
  extend.
- **ADR-0007 (single binary crate):** No workspace complexity; one crate, one `cargo`
  invocation per step.

## Consequences

- Every PR gets a format + lint + test gate before merge.
- Releases are intentional developer acts, not automatic side effects of merging.
- `Cargo.toml` version is always accurate and ready for `cargo publish`.
- When devenv lands (ADR-0018 Phase 2), the `publish` job gains a `devenv build` step
  with no structural changes to the workflow.
- Adds a dev dependency: contributors need `cargo-release` installed locally to cut
  releases. This is documented in the release workflow.

## Alternatives considered

- **Auto-bump patch on every merge to main.** Rejected — creates a commit-back loop
  (CI writes to `main`, triggering another CI run) unless `[skip ci]` is used, which
  is fragile. Also decouples the developer from the release decision.
- **Git tags as sole version source, Cargo.toml stays at 0.0.0.** Rejected — breaks
  `cargo publish` which requires `Cargo.toml` version to match the tag.
- **Two workflow files (ci.yml + release.yml).** Rejected — requires keeping toolchain
  versions in sync across files; unnecessary complexity at this scope.
- **release-plz (fully automated release PRs).** Viable for higher automation, but
  heavier than needed now. Can be adopted later if the release cadence warrants it.
