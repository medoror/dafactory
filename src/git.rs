//! Thin git wrapper (ADR-0005: subprocesses via `std::process::Command`). `init`
//! creates the code root's repo; `run` observes the agent's effect and commits the
//! change on `PR_READY` (ADR-0009). Commits carry an explicit identity so they do not
//! depend on the caller's global git config.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

const COMMIT_NAME: &str = "factory";
const COMMIT_EMAIL: &str = "factory@localhost";

/// Is `dir` already a git work tree?
pub fn is_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

/// Initialize a git repo at `dir`.
pub fn init(dir: &Path) -> Result<()> {
    run(dir, &["init", "-q"])?;
    Ok(())
}

/// Stage every change under `dir` (honoring `.gitignore`).
pub fn add_all(dir: &Path) -> Result<()> {
    run(dir, &["add", "-A"])?;
    Ok(())
}

/// The staged diff — the exact change a subsequent `commit` would record.
pub fn diff_cached(dir: &Path) -> Result<String> {
    run(dir, &["diff", "--cached"])
}

/// Is the work tree clean (nothing staged or unstaged, no untracked files)?
pub fn is_clean(dir: &Path) -> Result<bool> {
    Ok(run(dir, &["status", "--porcelain"])?.trim().is_empty())
}

/// Commit the staged changes with an explicit identity, returning the commit hash.
pub fn commit(dir: &Path, message: &str) -> Result<String> {
    run(
        dir,
        &[
            "-c",
            &format!("user.name={COMMIT_NAME}"),
            "-c",
            &format!("user.email={COMMIT_EMAIL}"),
            "commit",
            "-q",
            "-m",
            message,
        ],
    )?;
    Ok(run(dir, &["rev-parse", "HEAD"])?.trim().to_string())
}

/// Run a git command in `dir`, returning stdout on success.
fn run(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .with_context(|| format!("failed to spawn git {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).context("git output was not UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn should_report_non_repo_then_repo_after_init() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_repo(dir.path()));

        init(dir.path()).unwrap();

        assert!(is_repo(dir.path()));
    }

    #[test]
    fn should_commit_staged_changes_and_return_hash() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        fs::write(dir.path().join("a.txt"), "hello").unwrap();

        add_all(dir.path()).unwrap();
        let diff = diff_cached(dir.path()).unwrap();
        let hash = commit(dir.path(), "Add a.txt").unwrap();

        assert!(diff.contains("a.txt"));
        assert!(diff.contains("hello"));
        assert_eq!(hash.len(), 40, "expected a full sha1 commit hash");
        assert!(is_clean(dir.path()).unwrap());
    }

    #[test]
    fn should_report_dirty_tree_with_untracked_file() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        fs::write(dir.path().join("untracked.txt"), "x").unwrap();

        assert!(!is_clean(dir.path()).unwrap());
    }
}
