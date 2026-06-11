//! The real agent (ADR-0001): spawns `claude -p` in the code root to implement an
//! intent. Production path; not exercised by `cargo test` (no live model call) — the
//! `scripted` agent covers `run`'s plumbing deterministically (ADR-0009).

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
        let output = command_in_code_root(AGENT_BIN, &request.code_root)
            .arg("-p")
            .arg(&prompt)
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
}
