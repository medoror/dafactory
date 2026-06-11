//! `factory` — wraps a coding agent with held-out validation, explicit terminal
//! states, and an evidence bundle as the unit of output. Single binary crate for v0
//! (ADR-0007); `main` stays thin and dispatches into command modules.

mod agent;
mod backlog;
mod cli;
mod clock;
mod commands;
mod evidence;
mod git;
mod judge;
mod paths;
mod registry;
mod templates;

use std::env;
use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::Parser;

use cli::{Cli, Commands};
use paths::Paths;
use registry::Mode;

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Commands::Init {
            app, brownfield, ..
        } => {
            let paths = Paths::resolve()?;
            let mode = if brownfield {
                Mode::Brownfield
            } else {
                Mode::Greenfield
            };
            let outcome = commands::init::init(&paths, &app, mode)?;
            println!("Initialized '{app}'");
            println!("  code root:    {}", outcome.code_root.display());
            println!("  factory root: {}", outcome.factory_root.display());
            Ok(ExitCode::SUCCESS)
        }
        Commands::Validate { app, judge } => {
            let paths = Paths::resolve()?;
            let judge = build_judge(judge.as_deref())?;
            let stamp = clock::RunStamp::now()?;
            let outcome = commands::validate::validate(&paths, &app, judge.as_ref(), &stamp)?;
            println!(
                "Validated '{app}': {}% ({}/{} scenarios satisfied)",
                outcome.satisfaction, outcome.satisfied_count, outcome.total_count
            );
            println!("  evidence: {}", outcome.bundle_dir.display());
            // validate emits a measurement, not a terminal state — it exits 0 when it
            // ran (ADR-0012). Gate on the fraction via the bundle/registry.
            Ok(ExitCode::SUCCESS)
        }
        Commands::Run {
            app,
            once,
            agent,
            judge,
        } => {
            if !once {
                bail!("v0 supports only `factory run <app> --once`");
            }
            let paths = Paths::resolve()?;
            let agent = build_agent(agent.as_deref())?;
            let judge = build_judge(judge.as_deref())?;
            let stamp = clock::RunStamp::now()?;
            let outcome = commands::run::run(&paths, &app, agent.as_ref(), judge.as_ref(), &stamp)?;
            match outcome.satisfaction {
                Some(value) => println!("Ran '{app}': {} ({value}%)", outcome.terminal_state),
                None => println!("Ran '{app}': {}", outcome.terminal_state),
            }
            println!("  evidence: {}", outcome.bundle_dir.display());
            // The exit code reflects the terminal state so a wrapper / the AFK loop
            // can branch on $? (ADR-0012).
            Ok(ExitCode::from(outcome.terminal_state.exit_code()))
        }
        Commands::Ls => {
            let paths = Paths::resolve()?;
            print!("{}", commands::ls::ls(&paths)?);
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// Build the judge from a `--judge` flag, the `FACTORY_JUDGE` env, and (for scripted)
/// `FACTORY_JUDGE_SCRIPT`. Default `real`.
fn build_judge(flag: Option<&str>) -> Result<Box<dyn judge::Judge>> {
    let provider = judge::Provider::resolve(flag, env_var("FACTORY_JUDGE").as_deref())?;
    judge::build(provider, env_var("FACTORY_JUDGE_SCRIPT").map(Into::into))
}

/// Build the agent from an `--agent` flag, the `FACTORY_AGENT` env, and (for scripted)
/// `FACTORY_AGENT_SCRIPT`. Default `real`.
fn build_agent(flag: Option<&str>) -> Result<Box<dyn agent::Agent>> {
    let provider = agent::Provider::resolve(flag, env_var("FACTORY_AGENT").as_deref())?;
    agent::build(provider, env_var("FACTORY_AGENT_SCRIPT"))
}

/// A non-empty environment variable, or `None`.
fn env_var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.is_empty())
}
