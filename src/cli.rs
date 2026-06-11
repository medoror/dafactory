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
    /// Perform one outer-loop pass for an app.
    Run {
        /// Name of the registered app.
        app: String,
        /// Perform exactly one pass (the only mode in v0).
        #[arg(long)]
        once: bool,
        /// Agent provider: `real` (default, spawns claude -p) or `scripted`.
        #[arg(long)]
        agent: Option<String>,
        /// Judge provider: `real` (default, spawns claude -p) or `scripted`.
        #[arg(long)]
        judge: Option<String>,
    },
    /// List every registered app with its last-known state.
    Ls,
}
