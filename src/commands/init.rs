//! `factory init <app>` (B1): scaffold a line and register it.
//!
//! Creates the code root and the factory (holdout) root from embedded templates and
//! upserts the app into the registry. Idempotent: re-`init` rewrites templates and
//! re-registers, leaving a single registry entry.

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::git;
use crate::paths::Paths;
use crate::registry::{AppEntry, Mode, Registry};
use crate::templates;

/// What `init` created, for reporting back to the caller.
pub struct InitOutcome {
    pub code_root: PathBuf,
    pub factory_root: PathBuf,
}

pub fn init(paths: &Paths, app: &str, mode: Mode) -> Result<InitOutcome> {
    let code_root = paths.code_root(app);
    let factory_root = paths.factory_root(app);

    templates::scaffold(&code_root, &factory_root, app)?;
    commit_scaffold(&code_root, app)?;

    let registry_path = paths.registry_path();
    let mut registry = Registry::load(&registry_path)?;
    registry.upsert(
        app,
        AppEntry {
            code_root: code_root.clone(),
            factory_root: factory_root.clone(),
            mode,
            last_satisfaction: None,
            last_terminal_state: None,
            last_run_id: None,
            last_run_at: None,
        },
    );
    registry.save(&registry_path)?;

    Ok(InitOutcome {
        code_root,
        factory_root,
    })
}

/// Make the code root a git repo with the scaffold committed, so `run` has a clean
/// baseline to observe the agent against (ADR-0009). Idempotent: on re-`init` it only
/// commits if the rewritten scaffold actually changed something.
fn commit_scaffold(code_root: &Path, app: &str) -> Result<()> {
    if !git::is_repo(code_root) {
        git::init(code_root)?;
    }
    git::add_all(code_root)?;
    if !git::is_clean(code_root)? {
        git::commit(code_root, &format!("Scaffold {app} via factory init"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `Paths` rooted entirely inside a tempdir so the test touches neither
    /// the real data dir nor the real working directory.
    fn isolated_paths(dir: &std::path::Path) -> Paths {
        Paths::new(dir.join("home"), dir.join("work"))
    }

    #[test]
    fn should_create_both_roots_with_expected_files() {
        let dir = tempfile::tempdir().unwrap();
        let paths = isolated_paths(dir.path());

        let outcome = init(&paths, "myapp", Mode::Greenfield).unwrap();

        assert!(outcome.code_root.join("SPEC.md").is_file());
        assert!(outcome.code_root.join("CLAUDE.md").is_file());
        assert!(outcome.factory_root.join("judge.md").is_file());
        assert!(outcome.factory_root.join("evidence").is_dir());
    }

    #[test]
    fn should_make_code_root_a_git_repo_with_committed_scaffold() {
        let dir = tempfile::tempdir().unwrap();
        let paths = isolated_paths(dir.path());

        let outcome = init(&paths, "myapp", Mode::Greenfield).unwrap();

        // The code root is a repo with a clean tree (scaffold committed), and the
        // scaffold ships a .gitignore (ADR-0009).
        assert!(git::is_repo(&outcome.code_root));
        assert!(git::is_clean(&outcome.code_root).unwrap());
        assert!(outcome.code_root.join(".gitignore").is_file());
        let diff = git::diff_cached(&outcome.code_root).unwrap();
        assert!(diff.is_empty(), "scaffold should be committed, tree clean");
    }

    #[test]
    fn should_stay_a_single_clean_commit_baseline_on_reinit() {
        let dir = tempfile::tempdir().unwrap();
        let paths = isolated_paths(dir.path());
        init(&paths, "myapp", Mode::Greenfield).unwrap();

        // Re-init writes identical templates; the tree should remain clean and not
        // error on an empty commit.
        let outcome = init(&paths, "myapp", Mode::Greenfield).unwrap();

        assert!(git::is_clean(&outcome.code_root).unwrap());
    }

    #[test]
    fn should_register_app_with_roots_and_mode() {
        let dir = tempfile::tempdir().unwrap();
        let paths = isolated_paths(dir.path());

        init(&paths, "myapp", Mode::Brownfield).unwrap();

        let registry = Registry::load(&paths.registry_path()).unwrap();
        let entry = &registry.apps["myapp"];
        assert_eq!(entry.code_root, paths.code_root("myapp"));
        assert_eq!(entry.factory_root, paths.factory_root("myapp"));
        assert_eq!(entry.mode, Mode::Brownfield);
        assert_eq!(entry.last_satisfaction, None);
        assert_eq!(entry.last_terminal_state, None);
    }

    #[test]
    fn should_keep_single_entry_when_reinitializing() {
        let dir = tempfile::tempdir().unwrap();
        let paths = isolated_paths(dir.path());
        init(&paths, "myapp", Mode::Greenfield).unwrap();

        init(&paths, "myapp", Mode::Brownfield).unwrap();

        let registry = Registry::load(&paths.registry_path()).unwrap();
        assert_eq!(registry.apps.len(), 1);
        assert_eq!(registry.apps["myapp"].mode, Mode::Brownfield);
    }

    #[test]
    fn should_keep_factory_root_outside_code_root() {
        let dir = tempfile::tempdir().unwrap();
        let paths = isolated_paths(dir.path());

        let outcome = init(&paths, "myapp", Mode::Greenfield).unwrap();

        assert!(
            !outcome.factory_root.starts_with(&outcome.code_root),
            "factory root must not live inside the code root (ADR-0002)"
        );
    }
}
