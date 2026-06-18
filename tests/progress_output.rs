//! Factory emits stage-entry markers to stderr during `run` so a healthy pass
//! is distinguishable from a hung process.

use std::path::Path;
use std::process::Command;

fn factory(home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_factory"));
    command
        .env_remove("FACTORY_AGENT")
        .env_remove("FACTORY_AGENT_SCRIPT")
        .env_remove("FACTORY_JUDGE")
        .env_remove("FACTORY_JUDGE_SCRIPT")
        .env("FACTORY_HOME", home);
    command
}

fn init_demo(home: &Path, work: &Path) {
    let ok = factory(home)
        .current_dir(work)
        .args(["init", "demo"])
        .status()
        .unwrap()
        .success();
    assert!(ok, "factory init failed");
}

fn git(code_root: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(code_root)
        .args(args)
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?} failed");
}

fn write_verdict(home: &Path, json: &str) -> std::path::PathBuf {
    let path = home.join("verdict.json");
    std::fs::write(&path, json).unwrap();
    path
}

/// With an open intent: intent → running agent → validating → terminal state, in order.
#[test]
fn should_emit_all_stage_markers_in_order_when_intent_exists() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    init_demo(home.path(), work.path());
    let code_root = work.path().join("demo");
    std::fs::write(code_root.join("BACKLOG.md"), "- [ ] **B1 — do it.**\n").unwrap();
    git(
        &code_root,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@e",
            "commit",
            "-aqm",
            "add intent",
        ],
    );
    let verdict = write_verdict(
        home.path(),
        r#"{"scenarios":[{"id":"x","satisfied":false}]}"#,
    );

    let output = factory(home.path())
        .env("FACTORY_AGENT", "scripted")
        .env("FACTORY_AGENT_SCRIPT", "true")
        .env("FACTORY_JUDGE", "scripted")
        .env("FACTORY_JUDGE_SCRIPT", &verdict)
        .current_dir(work.path())
        .args(["run", "demo", "--once"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let intent_pos = stderr
        .find("factory: intent →")
        .unwrap_or_else(|| panic!("intent marker missing from stderr:\n{stderr}"));
    let agent_pos = stderr
        .find("factory: running agent...")
        .unwrap_or_else(|| panic!("running-agent marker missing from stderr:\n{stderr}"));
    let validate_pos = stderr
        .find("factory: validating...")
        .unwrap_or_else(|| panic!("validating marker missing from stderr:\n{stderr}"));
    let terminal_pos = stderr
        .find("factory: → ESCALATE")
        .unwrap_or_else(|| panic!("terminal-state marker missing from stderr:\n{stderr}"));

    assert!(
        intent_pos < agent_pos,
        "intent marker must precede agent marker"
    );
    assert!(
        agent_pos < validate_pos,
        "agent marker must precede validating marker"
    );
    assert!(
        validate_pos < terminal_pos,
        "validating marker must precede terminal marker"
    );
}

/// With no open intent: no-intent marker → validating → terminal state.
#[test]
fn should_emit_no_intent_marker_when_backlog_exhausted() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    init_demo(home.path(), work.path());
    let verdict = write_verdict(
        home.path(),
        r#"{"scenarios":[{"id":"x","satisfied":true}]}"#,
    );

    let output = factory(home.path())
        .env("FACTORY_JUDGE", "scripted")
        .env("FACTORY_JUDGE_SCRIPT", &verdict)
        .current_dir(work.path())
        .args(["run", "demo", "--once"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("factory: no open intent — validating"),
        "no-intent marker missing from stderr:\n{stderr}"
    );
    let validate_pos = stderr
        .find("factory: validating...")
        .unwrap_or_else(|| panic!("validating marker missing from stderr:\n{stderr}"));
    let terminal_pos = stderr
        .find("factory: → NO_OP")
        .unwrap_or_else(|| panic!("terminal-state marker missing from stderr:\n{stderr}"));
    assert!(
        validate_pos < terminal_pos,
        "validating marker must precede terminal marker"
    );
}
