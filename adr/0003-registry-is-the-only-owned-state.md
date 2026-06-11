# ADR-0003 — The registry is the only persistent state the tool owns

## Decision
`factory` owns exactly one piece of global state: a registry mapping each app to its
code root, factory root, mode, and last-known status (last satisfaction, last
terminal state, last run time). It lives in the OS-standard per-user data directory
resolved via the `directories` crate — on Linux `~/.local/share/factory/registry.json`.
A `FACTORY_HOME` (or equivalent) env override relocates it, which sandboxed scenario
runs use to get an isolated registry. Everything else — spec, ADRs, backlog,
progress, code, evidence — lives inside the two project roots and in git.

## Why
Solo dev's real pain is context-switching across many half-live projects. The
registry turns "twelve abandoned repos" into "twelve resumable lines." Because the
tool is distributed, its state belongs in OS-standard locations, not a hardcoded
home path or a workspace folder. Keeping the registry the only owned state keeps the
tool thin and keeps each project self-describing on disk, so a project survives the
registry being deleted (re-`init` / re-point).

## Consequences
- `ls` reads the registry; `init` writes to it; `run`/`validate` update its status
  fields. No other global state in v0.
- The env override is the seam scenarios use to run against a clean, isolated
  registry without touching the real one.
