//! The coding agent (ADR-0001, ADR-0009). An agent is a subprocess launched in the
//! code root; its effect is the working-tree change, which `run` observes via git.
//! `factory` never interprets agent stdout as edits (ADR-0004). Two providers share
//! one interface: `real` (`claude -p`) and `scripted` (`sh -c $FACTORY_AGENT_SCRIPT`).

pub mod real;
pub mod scripted;

use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use real::RealAgent;
use scripted::ScriptedAgent;

/// The backlog intent the agent is asked to implement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intent {
    /// e.g. `B3`, when the backlog line carries one.
    pub id: Option<String>,
    pub title: String,
    /// The full backlog line, verbatim — what the real agent is handed.
    pub raw: String,
}

pub struct AgentRequest {
    pub app: String,
    pub code_root: std::path::PathBuf,
    pub intent: Intent,
}

/// The agent's narration (stdout). Recorded as evidence; never interpreted as edits
/// (ADR-0004).
pub struct AgentOutcome {
    pub log: String,
}

/// The agent interface. Production impls spawn a subprocess; tests inject doubles.
pub trait Agent {
    fn implement(&self, request: &AgentRequest) -> Result<AgentOutcome>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Real,
    Scripted,
}

impl Provider {
    pub fn resolve(flag: Option<&str>, env: Option<&str>) -> Result<Provider> {
        match flag.or(env) {
            None | Some("real") => Ok(Provider::Real),
            Some("scripted") => Ok(Provider::Scripted),
            Some(other) => {
                anyhow::bail!("unknown agent provider '{other}' (expected 'real' or 'scripted')")
            }
        }
    }
}

pub fn build(provider: Provider, script: Option<String>) -> Result<Box<dyn Agent>> {
    match provider {
        Provider::Real => Ok(Box::new(RealAgent::new())),
        Provider::Scripted => {
            let command =
                script.context("the scripted agent needs a command (set FACTORY_AGENT_SCRIPT)")?;
            Ok(Box::new(ScriptedAgent::new(command)))
        }
    }
}

/// Is `key` one of factory's trusted-runner-only seam variables? Every `FACTORY_*`
/// variable is a seam (the holdout home, the scripted-agent/judge sources, etc.), so
/// scrubbing by prefix covers any future seam automatically (ADR-0011).
fn is_seam_var(key: &OsStr) -> bool {
    key.to_string_lossy().starts_with("FACTORY_")
}

/// A `Command` for `program`, with the working directory set to the code root and
/// every `FACTORY_*` seam scrubbed from its environment — the only sanctioned way to
/// launch an agent (ADR-0002, ADR-0009, ADR-0011). The agent is thus never handed a
/// path to the holdout.
fn command_in_code_root(program: &str, code_root: &Path) -> Command {
    let mut command = Command::new(program);
    command.current_dir(code_root);
    for key in std::env::vars_os().map(|(key, _)| key) {
        if is_seam_var(&key) {
            command.env_remove(key);
        }
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_treat_every_factory_prefixed_var_as_a_seam() {
        assert!(is_seam_var(OsStr::new("FACTORY_HOME")));
        assert!(is_seam_var(OsStr::new("FACTORY_AGENT_SCRIPT")));
        // A seam added in the future is covered without changing this code.
        assert!(is_seam_var(OsStr::new("FACTORY_SOMETHING_NEW")));
        assert!(!is_seam_var(OsStr::new("PATH")));
        assert!(!is_seam_var(OsStr::new("HOME")));
    }

    #[test]
    fn should_resolve_provider_from_flag_then_env_then_default() {
        assert_eq!(Provider::resolve(None, None).unwrap(), Provider::Real);
        assert_eq!(
            Provider::resolve(None, Some("scripted")).unwrap(),
            Provider::Scripted
        );
        assert_eq!(
            Provider::resolve(Some("real"), Some("scripted")).unwrap(),
            Provider::Real
        );
        assert!(Provider::resolve(Some("bogus"), None).is_err());
    }
}
