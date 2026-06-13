//! The real agent (ADR-0001): spawns `claude -p` in the code root to implement an
//! intent. Production path; not exercised by `cargo test` (no live model call) — the
//! `scripted` agent covers `run`'s plumbing deterministically (ADR-0009).

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use super::{command_in_code_root, Agent, AgentOutcome, AgentRequest};

/// The coding-agent CLI. `claude` for v0 (ADR-0001).
const AGENT_BIN: &str = "claude";

pub struct RealAgent;

impl RealAgent {
    pub fn new() -> RealAgent {
        RealAgent
    }
}

impl Default for RealAgent {
    fn default() -> Self {
        RealAgent::new()
    }
}

impl Agent for RealAgent {
    fn implement(&self, request: &AgentRequest) -> Result<AgentOutcome> {
        let prompt = build_prompt(request);
        let output = agent_command(&request.code_root, &prompt)
            .output()
            .with_context(|| format!("failed to spawn agent process `{AGENT_BIN}`"))?;
        if !output.status.success() {
            bail!(
                "agent process `{AGENT_BIN}` exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(AgentOutcome {
            log: String::from_utf8_lossy(&output.stdout).into_owned(),
        })
    }
}

/// The `claude` command for one implement pass: headless print mode (`-p`) in the
/// code root with the `FACTORY_*` seams scrubbed (`command_in_code_root`).
fn agent_command(code_root: &Path, prompt: &str) -> Command {
    let mut command = command_in_code_root(AGENT_BIN, code_root);
    command
        .arg("-p")
        .arg("--dangerously-skip-permissions")
        .arg(prompt);
    command
}

/// Compose the implement prompt. The working agreement (CLAUDE.md) and project
/// context live in the code root the agent runs in; the agent is told to stay inside
/// it (the holdout is enforced by construction — ADR-0002).
fn build_prompt(request: &AgentRequest) -> String {
    format!(
        "You are implementing one backlog intent for `{app}`.\n\n\
         Intent:\n{intent}\n\n\
         Read SPEC.md, the adr/ directory, BACKLOG.md, and PROGRESS.md, then implement \
         the smallest change that satisfies this intent, test-first, following \
         CLAUDE.md. Work only inside this directory; do not look outside it.",
        app = request.app,
        intent = request.intent.raw,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Intent;

    #[test]
    fn should_build_a_prompt_from_the_intent() {
        let request = AgentRequest {
            app: "demo".into(),
            code_root: "/work/demo".into(),
            intent: Intent {
                id: Some("B3".into()),
                title: "run --once happy path".into(),
                raw: "- [ ] B3 (→ S003) — run --once happy path".into(),
            },
        };

        let prompt = build_prompt(&request);

        assert!(prompt.contains("demo"));
        assert!(prompt.contains("run --once happy path"));
        assert!(prompt.contains("do not look outside it"));
    }

    #[test]
    fn should_invoke_claude_headless_with_skip_permissions() {
        // Headless `claude -p` blocks file writes pending an approval that never comes
        // in non-interactive use, so the agent must skip permission prompts or every
        // pass produces an empty diff.
        let command = agent_command(Path::new("/work/demo"), "implement B3");

        assert_eq!(command.get_program().to_string_lossy(), "claude");
        let args: Vec<String> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"implement B3".to_string()));
        assert!(
            args.contains(&"--dangerously-skip-permissions".to_string()),
            "headless agent must skip permission prompts to write files"
        );
    }
}
