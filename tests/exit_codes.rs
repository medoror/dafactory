//! `factory run`'s process exit code must reflect the terminal state (ADR-0012).
//! These drive the real built binary so the `main` wiring can't regress to "always 0".

use std::path::Path;
use std::process::Command;

/// A `factory` invocation with the `FACTORY_*` seams cleared and `FACTORY_HOME`
/// pointed at the test's isolated home, so each test is hermetic.
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

#[test]
fn no_op_exits_zero() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    init_demo(home.path(), work.path());
    // Pristine init has no open intent → NO_OP. A scripted judge so validate runs.
    let verdict = write_verdict(
        home.path(),
        r#"{"scenarios":[{"id":"x","satisfied":true}]}"#,
    );

    let status = factory(home.path())
        .env("FACTORY_JUDGE", "scripted")
        .env("FACTORY_JUDGE_SCRIPT", &verdict)
        .current_dir(work.path())
        .args(["run", "demo", "--once"])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(0), "NO_OP must exit 0");
}

#[test]
fn retryable_exits_ten() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    init_demo(home.path(), work.path());
    // A dirty tree → run refuses before the agent (machinery → RETRYABLE). Default
    // providers are built but never invoked.
    std::fs::write(work.path().join("demo/dirty.txt"), "x").unwrap();

    let status = factory(home.path())
        .current_dir(work.path())
        .args(["run", "demo", "--once"])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(10), "RETRYABLE must exit 10");
}

#[test]
fn escalate_exits_eleven() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    init_demo(home.path(), work.path());
    let code_root = work.path().join("demo");
    // An open, committed intent (clean tree); a no-op agent; a failing verdict →
    // no change + <100 → ESCALATE.
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
            "intent",
        ],
    );
    let verdict = write_verdict(
        home.path(),
        r#"{"scenarios":[{"id":"x","satisfied":false}]}"#,
    );

    let status = factory(home.path())
        .env("FACTORY_AGENT", "scripted")
        .env("FACTORY_AGENT_SCRIPT", "true")
        .env("FACTORY_JUDGE", "scripted")
        .env("FACTORY_JUDGE_SCRIPT", &verdict)
        .current_dir(work.path())
        .args(["run", "demo", "--once"])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(11), "ESCALATE must exit 11");
}
