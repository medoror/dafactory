//! The `scripted` judge (ADR-0008): returns canned per-scenario verdicts from a
//! caller-supplied JSON file. Trusted-runner-only — it exists to make `validate`'s
//! plumbing deterministically testable, not to make judgments.

use std::path::PathBuf;

use anyhow::Result;

use super::{load_verdict, Judge, JudgeRequest, Verdict};

pub struct ScriptedJudge {
    /// Path to a JSON file in the `judge.md` verdict shape.
    script: PathBuf,
}

impl ScriptedJudge {
    pub fn new(script: PathBuf) -> ScriptedJudge {
        ScriptedJudge { script }
    }
}

impl Judge for ScriptedJudge {
    fn judge(&self, _request: &JudgeRequest) -> Result<Verdict> {
        load_verdict(&self.script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn should_return_the_canned_verdict_from_its_file() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("verdict.json");
        fs::write(
            &script,
            r#"{"scenarios":[
                {"id":"S001","satisfied":true,"observed":"ok"},
                {"id":"S002","satisfied":false,"observed":"nope"}
            ]}"#,
        )
        .unwrap();
        let judge = ScriptedJudge::new(script);
        let request = JudgeRequest {
            app: "demo".into(),
            code_root: dir.path().join("code"),
            factory_root: dir.path().join("factory"),
        };

        let verdict = judge.judge(&request).unwrap();

        assert_eq!(verdict.total_count(), 2);
        assert_eq!(verdict.satisfied_count(), 1);
        assert_eq!(verdict.satisfaction(), 50);
    }
}
