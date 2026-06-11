# ADR-0005 — Stack: Rust + clap, single distributable binary

## Decision
`factory` is written in Rust. The command surface uses `clap`. State is JSON via
`serde`/`serde_json`. Agent and judge calls are subprocess invocations via
`std::process::Command`. The tool ships as a single static binary via GitHub
Releases, cross-compiled for linux and macos (x86_64 + aarch64), with `cargo install`
and a Homebrew tap as later options. Scaffolding templates are embedded in the
binary (`include_str!` / an embedded-dir crate), so a standalone binary can `init`
with no external files.

## Why
Distribution to other people is a goal (open-source / team use / portfolio), so a
single binary with no runtime to install is a real, lasting advantage — the exact
thing Python's packaging story makes painful. Rust is chosen over Go for author
fluency: for a solo project, working in a language you already know is the dominant
factor in velocity, and it removes the only serious downside of Rust here (iteration
speed is mostly a tax on people fighting the borrow checker, not people fluent in
it). Nothing in v0 is compute-bound; Rust is not chosen for performance, it's
chosen for the binary plus the author already knowing it.

## Consequences
- A Cargo project from B1. No heavy async runtime in v0 — blocking subprocess calls
  are sufficient; do not pull `tokio` to shell out to one process at a time.
- The agent and judge interfaces are traits (see ADR-0001). Fast `cargo test` uses
  trait doubles; the binary also ships a runtime-selectable `scripted` provider.
- Templates are compiled into the binary; `init` writes them out, it does not read
  them from the filesystem.
- Cross-compilation via `cargo`/`cross`; release artifacts attached to tags.
