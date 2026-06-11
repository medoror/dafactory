//! The `scripted` agent (ADR-0009): runs a caller-supplied command via `sh -c` in the
//! code root. Trusted-runner-only — the external harness uses it to drive `run`
//! deterministically (correct-impl / no-work / snooping). Its effect is the
//! working-tree change, observed by `run` via git; its stdout is narration only.

use anyhow::{bail, Context, Result};

use super::{command_in_code_root, Agent, AgentOutcome, AgentRequest};

pub struct ScriptedAgent {
    command: String,
}

impl ScriptedAgent {
    pub fn new(command: String) -> ScriptedAgent {
        ScriptedAgent { command }
    }
}

impl Agent for ScriptedAgent {
    fn implement(&self, request: &AgentRequest) -> Result<AgentOutcome> {
        let output = command_in_code_root("sh", &request.code_root)
            .arg("-c")
            .arg(&self.command)
            .output()
            .context("failed to spawn scripted agent (`sh -c`)")?;
        if !output.status.success() {
            bail!(
                "scripted agent exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(AgentOutcome {
            log: String::from_utf8_lossy(&output.stdout).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Intent;
    use std::fs;

    fn request(code_root: &std::path::Path) -> AgentRequest {
        AgentRequest {
            app: "demo".into(),
            code_root: code_root.to_path_buf(),
            intent: Intent {
                id: Some("B1".into()),
                title: "do the thing".into(),
                raw: "- [ ] B1 — do the thing".into(),
            },
        }
    }

    #[test]
    fn should_run_command_in_code_root_and_change_the_tree() {
        let dir = tempfile::tempdir().unwrap();
        let code_root = dir.path();
        let agent = ScriptedAgent::new("echo hi > made.txt; echo done".into());

        let outcome = agent.implement(&request(code_root)).unwrap();

        assert!(code_root.join("made.txt").is_file());
        assert!(outcome.log.contains("done"));
    }

    #[test]
    fn should_scrub_seam_variables_from_the_agent_environment() {
        let dir = tempfile::tempdir().unwrap();
        let code_root = dir.path();
        // The seam is set in our environment; the agent subprocess must not see it.
        std::env::set_var("FACTORY_HOME", "/should/be/hidden");
        let agent = ScriptedAgent::new("printf 'home=[%s]' \"$FACTORY_HOME\"".into());

        let outcome = agent.implement(&request(code_root)).unwrap();
        std::env::remove_var("FACTORY_HOME");

        assert_eq!(outcome.log, "home=[]");
        // sanity: the script genuinely cannot reach the factory root via the seam.
        assert!(fs::read_dir(code_root).is_ok());
    }
}
