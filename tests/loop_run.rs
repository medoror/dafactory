//! `factory run --max-iters N` loops up to N passes, ticking BACKLOG.md on each
//! PR_READY result and exiting with the last terminal state's exit code.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

fn factory(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_factory"));
    cmd.env_remove("FACTORY_AGENT")
        .env_remove("FACTORY_AGENT_SCRIPT")
        .env_remove("FACTORY_JUDGE")
        .env_remove("FACTORY_JUDGE_SCRIPT")
        .env("FACTORY_HOME", home);
    cmd
}

fn init_demo(home: &Path, work: &Path) {
    assert!(
        factory(home)
            .current_dir(work)
            .args(["init", "demo"])
            .status()
            .unwrap()
            .success(),
        "factory init failed"
    );
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .unwrap()
            .success(),
        "git {args:?} failed"
    );
}

fn write_verdict(home: &Path, json: &str) -> std::path::PathBuf {
    let path = home.join("verdict.json");
    std::fs::write(&path, json).unwrap();
    path
}

/// Two passes each return PR_READY: both intents are ticked and exit code is 0.
#[test]
fn should_tick_two_intents_and_exit_zero_after_two_pr_ready_passes() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    init_demo(home.path(), work.path());

    let code_root = work.path().join("demo");
    std::fs::write(
        code_root.join("BACKLOG.md"),
        "- [ ] **B1 — first.**\n- [ ] **B2 — second.**\n",
    )
    .unwrap();
    git(
        &code_root,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@e",
            "commit",
            "-aqm",
            "Two intents",
        ],
    );

    // A scripted agent that creates a uniquely-named file on each pass by reading a
    // pass counter from disk.
    let agent_script = home.path().join("agent.sh");
    std::fs::write(
        &agent_script,
        "#!/bin/sh\n\
         N=$(cat .pass_count 2>/dev/null || echo 0)\n\
         N=$((N+1))\n\
         echo $N > .pass_count\n\
         echo pass > feature-$N.txt\n",
    )
    .unwrap();
    let mut perms = std::fs::metadata(&agent_script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&agent_script, perms).unwrap();

    let verdict = write_verdict(
        home.path(),
        r#"{"scenarios":[{"id":"x","satisfied":true}]}"#,
    );

    let output = factory(home.path())
        .env("FACTORY_AGENT", "scripted")
        .env("FACTORY_AGENT_SCRIPT", &agent_script)
        .env("FACTORY_JUDGE", "scripted")
        .env("FACTORY_JUDGE_SCRIPT", &verdict)
        .current_dir(work.path())
        .args(["run", "demo", "--max-iters", "2"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Both intents ticked.
    let backlog = std::fs::read_to_string(code_root.join("BACKLOG.md")).unwrap();
    assert!(
        backlog.contains("- [x] **B1 — first.**"),
        "B1 not ticked:\n{backlog}\nstderr:\n{stderr}"
    );
    assert!(
        backlog.contains("- [x] **B2 — second.**"),
        "B2 not ticked:\n{backlog}\nstderr:\n{stderr}"
    );

    // Exit 0 (last terminal state = PR_READY).
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit 0;\nstdout: {stdout}\nstderr: {stderr}"
    );

    // Both agent artefacts committed into the code root.
    assert!(
        code_root.join("feature-1.txt").exists(),
        "feature-1.txt missing"
    );
    assert!(
        code_root.join("feature-2.txt").exists(),
        "feature-2.txt missing"
    );
}
