//! The command surface. v0 is frozen at exactly four commands (SPEC). All four are
//! declared here; only `init` is implemented in v0 (B1). The rest are explicit
//! failing stubs in `main` — never silent no-ops — so a half-built command cannot
//! accidentally satisfy a scenario.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "factory", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scaffold a new factory line and register it.
    Init {
        /// Name of the app / line.
        app: String,
        /// Scaffold as greenfield (the default).
        #[arg(long, conflicts_with = "brownfield")]
        greenfield: bool,
        /// Scaffold as brownfield.
        #[arg(long)]
        brownfield: bool,
    },
    /// Run the held-out judge against an app and write an evidence bundle.
    Validate {
        /// Name of the registered app.
        app: String,
        /// Judge provider: `real` (default, spawns claude -p) or `scripted`.
        #[arg(long)]
        judge: Option<String>,
    },
    /// Run the outer loop for an app (up to --max-iters passes).
    Run {
        /// Name of the registered app.
        app: String,
        /// Number of loop passes to attempt (≥ 1).
        #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..))]
        max_iters: u32,
        /// Maximum retries allowed on RETRYABLE per occurrence (default 1).
        #[arg(long, default_value_t = 1)]
        retries: u32,
        /// Agent provider: `real` (default, spawns claude -p) or `scripted`.
        #[arg(long)]
        agent: Option<String>,
        /// Judge provider: `real` (default, spawns claude -p) or `scripted`.
        #[arg(long)]
        judge: Option<String>,
    },
    /// List every registered app with its last-known state.
    Ls,
    /// Copy spec and backlog into the holdout root and write a scenario-authoring
    /// CLAUDE.md so you can open a fresh session there to draft scenarios.
    Scenarios {
        /// Name of the registered app.
        app: String,
    },
}
