# ADR-0007 — Single binary crate for v0

## Decision
v0 is a single Cargo binary crate (`factory`), not a workspace with a separate
library crate. Command logic lives in modules within the one crate; `main.rs` stays
thin but the core is not yet split into its own `lib.rs`/crate.

## Why
The lib/bin split is a real and good pattern, but it earns its keep only when
something external needs to import the core — another binary, an integration test
harness that links the library directly, or a published crate. None of that is true
for the walking skeleton. Choosing the simplest structure now keeps Phase 1 fast,
and the split is a mechanical, low-risk refactor later (move logic into `lib.rs`,
leave a thin `main.rs` that calls it). This is a universal "keep the entry point
thin" decision, not a Rust-specific one; it is recorded here only because Rust names
the alternative explicitly.

## Revisit when
You actually want to import the core elsewhere — e.g. a second binary, a daemon, or
publishing the core as a reusable crate. That want is the signal to do the split;
until then it is speculative structure.

## Consequences
- One `Cargo.toml`, one crate, modules for command surface / registry / providers /
  evidence.
- In-crate `cargo test` with trait doubles remains fully available; the single-crate
  choice does not compromise testability for v0.
- If the split happens later, it does not change any ADR above it — the traits,
  holdout boundary, and registry design are structure-independent.
