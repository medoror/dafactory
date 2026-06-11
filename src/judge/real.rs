//! The real judge (ADR-0001): spawns `claude -p`, driven by the factory root's
//! `judge.md` and `scenarios/`, and parses the JSON verdict it emits. This is the
//! production path; it is not exercised by `cargo test` (no live model call) — the
//! `scripted` judge covers `validate`'s plumbing deterministically (ADR-0008).

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use super::{parse_verdict, Judge, JudgeRequest, Verdict};

/// The coding-agent CLI used as the judge. `claude` for v0 (ADR-0001).
const JUDGE_BIN: &str = "claude";

pub struct RealJudge;

impl RealJudge {
    pub fn new() -> RealJudge {
        RealJudge
    }
}

impl Default for RealJudge {
    fn default() -> Self {
        RealJudge::new()
    }
}

impl Judge for RealJudge {
    fn judge(&self, request: &JudgeRequest) -> Result<Verdict> {
        let prompt = build_prompt(request)?;
        let output = Command::new(JUDGE_BIN)
            .arg("-p")
            .arg(&prompt)
            .output()
            .with_context(|| format!("failed to spawn judge process `{JUDGE_BIN}`"))?;

        if !output.status.success() {
            bail!(
                "judge process `{JUDGE_BIN}` exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let stdout = String::from_utf8(output.stdout).context("judge output was not UTF-8")?;
        parse_verdict(&stdout)
    }
}

/// Compose the judge prompt from the holdout `judge.md` and the scenarios, telling
/// the judge which app to drive. The judge observes external behavior only; it is
/// instructed never to read the app's source (ADR-0002, judge.md).
fn build_prompt(request: &JudgeRequest) -> Result<String> {
    let judge_md = read_required(&request.judge_md())?;
    let scenarios = read_scenarios(&request.scenarios_dir())?;

    Ok(format!(
        "{judge_md}\n\n\
         ---\n\
         App under judgement: {app}\n\
         Drive it as a black box at: {code_root}\n\
         Do NOT read its source.\n\n\
         Scenarios:\n{scenarios}\n\n\
         Emit only the JSON verdict described above.",
        app = request.app,
        code_root = request.code_root.display(),
    ))
}

fn read_required(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))
}

/// Concatenate every scenario file under `dir`, labelled by file name, so the judge
/// receives the held-out scenarios verbatim.
fn read_scenarios(dir: &Path) -> Result<String> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("failed to read scenarios at {}", dir.display()))?
        .collect::<std::result::Result<_, _>>()
        .with_context(|| format!("failed to list scenarios at {}", dir.display()))?;
    entries.sort_by_key(|e| e.file_name());

    let mut buf = String::new();
    for entry in entries {
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let contents = read_required(&path)?;
            buf.push_str(&format!("## {name}\n{contents}\n\n"));
        }
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_build_a_prompt_from_judge_md_and_scenarios() {
        let dir = tempfile::tempdir().unwrap();
        let factory_root = dir.path().join("factory");
        fs::create_dir_all(factory_root.join("scenarios")).unwrap();
        fs::write(factory_root.join("judge.md"), "JUDGE RULES").unwrap();
        fs::write(factory_root.join("scenarios/S001.md"), "scenario one body").unwrap();
        let request = JudgeRequest {
            app: "demo".into(),
            code_root: dir.path().join("work/demo"),
            factory_root,
        };

        let prompt = build_prompt(&request).unwrap();

        assert!(prompt.contains("JUDGE RULES"));
        assert!(prompt.contains("scenario one body"));
        assert!(prompt.contains("demo"));
        assert!(prompt.contains("Do NOT read its source"));
    }

    /// The isolation seam (ADR-0002, ADR-0008): `factory` hands the judge the scenario
    /// text and a black-box driver reference (the code-root path), with an explicit
    /// do-not-read-source instruction — but never the app's source *contents*. This
    /// tests what `factory` constructs (deterministic, no model call); whether the
    /// model then obeys the instruction is the judge's discipline, not enforced in v0.
    #[test]
    fn should_hand_the_judge_scenarios_and_a_driver_but_never_source_contents() {
        let dir = tempfile::tempdir().unwrap();
        let factory_root = dir.path().join("factory");
        let code_root = dir.path().join("work/demo");
        fs::create_dir_all(factory_root.join("scenarios")).unwrap();
        fs::create_dir_all(code_root.join("src")).unwrap();
        fs::write(factory_root.join("judge.md"), "JUDGE RULES").unwrap();
        fs::write(
            factory_root.join("scenarios/S002.md"),
            "SCENARIO-MARKER: check the greeting",
        )
        .unwrap();
        // The app's source carries a sentinel that must never reach the judge.
        fs::write(
            code_root.join("src/secret.rs"),
            "fn impl_detail() { let _ = \"SOURCE-SENTINEL\"; }",
        )
        .unwrap();
        let request = JudgeRequest {
            app: "demo".into(),
            code_root: code_root.clone(),
            factory_root,
        };

        let prompt = build_prompt(&request).unwrap();

        // Handed: the scenarios, the judge rules, and a black-box driver reference.
        assert!(prompt.contains("SCENARIO-MARKER"));
        assert!(prompt.contains("JUDGE RULES"));
        assert!(prompt.contains(&code_root.display().to_string()));
        assert!(prompt.contains("Do NOT read its source"));
        // NOT handed: the app's source contents.
        assert!(
            !prompt.contains("SOURCE-SENTINEL"),
            "the judge prompt must not embed the app's source"
        );
    }
}
