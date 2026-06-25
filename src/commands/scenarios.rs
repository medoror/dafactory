//! `factory scenarios <app>`: copy spec and backlog into the holdout root and write
//! a scenario-authoring CLAUDE.md so the user can open a fresh session there.

use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use crate::paths::Paths;

#[derive(Debug)]
pub struct ScenariosOutcome {
    pub factory_root: PathBuf,
}

pub fn scenarios(paths: &Paths, app: &str) -> Result<ScenariosOutcome> {
    let registry = crate::registry::Registry::load(&paths.registry_path())?;
    let entry = registry.apps.get(app).ok_or_else(|| {
        anyhow::anyhow!("app '{app}' is not registered — run `factory init {app}` first")
    })?;
    let code_root = entry.code_root.clone();
    let factory_root = entry.factory_root.clone();

    // Validate SPEC.md
    let spec_path = code_root.join("SPEC.md");
    if !spec_path.is_file() {
        bail!("SPEC.md not found in the code root — write it before drafting scenarios");
    }
    let spec = fs::read_to_string(&spec_path)
        .with_context(|| format!("failed to read {}", spec_path.display()))?;
    if spec.contains("<!-- One paragraph") {
        bail!("SPEC.md still contains the unfilled template stub — fill in the 'What this is' section before drafting scenarios");
    }

    // Validate BACKLOG.md
    let backlog_path = code_root.join("BACKLOG.md");
    if !backlog_path.is_file() {
        bail!("BACKLOG.md not found in the code root — write it before drafting scenarios");
    }
    let backlog = fs::read_to_string(&backlog_path)
        .with_context(|| format!("failed to read {}", backlog_path.display()))?;
    if crate::backlog::next_intent(&backlog).is_none() {
        bail!("BACKLOG.md has no open intents — add at least one `- [ ]` item (outside HTML comments) before drafting scenarios");
    }

    // Copy spec and backlog into the holdout root
    fs::create_dir_all(&factory_root)
        .with_context(|| format!("failed to create factory root {}", factory_root.display()))?;
    fs::copy(&spec_path, factory_root.join("SPEC.md"))
        .context("failed to copy SPEC.md to factory root")?;
    fs::copy(&backlog_path, factory_root.join("BACKLOG.md"))
        .context("failed to copy BACKLOG.md to factory root")?;

    // Write the scenario-authoring CLAUDE.md from the embedded template
    let claude_md = crate::templates::SCENARIO_CLAUDE.replace("{{app}}", app);
    fs::write(factory_root.join("CLAUDE.md"), claude_md)
        .context("failed to write CLAUDE.md to factory root")?;

    Ok(ScenariosOutcome { factory_root })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::init;
    use crate::registry::Mode;

    fn setup(dir: &std::path::Path) -> (Paths, PathBuf, PathBuf) {
        let paths = Paths::new(dir.join("home"), dir.join("work"));
        let code_root = paths.code_root("myapp");
        let factory_root = paths.factory_root("myapp");
        init::init(&paths, "myapp", Mode::Greenfield).unwrap();
        (paths, code_root, factory_root)
    }

    fn setup_ready(dir: &std::path::Path) -> (Paths, PathBuf, PathBuf) {
        let paths = Paths::new(dir.join("home"), dir.join("work"));
        let code_root = paths.code_root("myapp");
        let factory_root = paths.factory_root("myapp");
        init::init(&paths, "myapp", Mode::Greenfield).unwrap();
        fs::write(
            code_root.join("SPEC.md"),
            "# SPEC\n\n## What this is\n\nA real app.\n",
        )
        .unwrap();
        fs::write(
            code_root.join("BACKLOG.md"),
            "# BACKLOG\n\n- [ ] **B1 (→ S001) — Greet by name.**\n",
        )
        .unwrap();
        (paths, code_root, factory_root)
    }

    #[test]
    fn should_error_when_app_not_registered() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path().join("home"), dir.path().join("work"));

        let err = scenarios(&paths, "unregistered").unwrap_err();

        assert!(
            err.to_string().contains("not registered"),
            "expected 'not registered' in: {err}"
        );
    }

    #[test]
    fn should_error_when_spec_md_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, code_root, _) = setup(dir.path());
        fs::remove_file(code_root.join("SPEC.md")).unwrap();

        let err = scenarios(&paths, "myapp").unwrap_err();

        assert!(
            err.to_string().to_lowercase().contains("spec"),
            "expected spec mention in: {err}"
        );
    }

    #[test]
    fn should_error_when_spec_md_is_unfilled_template() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, _, _) = setup(dir.path());
        // init writes the template stub which still contains the placeholder comment

        let err = scenarios(&paths, "myapp").unwrap_err();

        assert!(
            err.to_string().to_lowercase().contains("spec"),
            "expected spec mention in: {err}"
        );
    }

    #[test]
    fn should_error_when_backlog_md_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, code_root, _) = setup(dir.path());
        fs::write(
            code_root.join("SPEC.md"),
            "# SPEC\n\n## What this is\n\nA real app.\n",
        )
        .unwrap();
        fs::remove_file(code_root.join("BACKLOG.md")).unwrap();

        let err = scenarios(&paths, "myapp").unwrap_err();

        assert!(
            err.to_string().to_lowercase().contains("backlog"),
            "expected backlog mention in: {err}"
        );
    }

    #[test]
    fn should_error_when_backlog_has_no_open_intents() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, code_root, _) = setup(dir.path());
        fs::write(
            code_root.join("SPEC.md"),
            "# SPEC\n\n## What this is\n\nA real app.\n",
        )
        .unwrap();
        fs::write(
            code_root.join("BACKLOG.md"),
            "# BACKLOG\n\n- [x] **B1 — done.**\n",
        )
        .unwrap();

        let err = scenarios(&paths, "myapp").unwrap_err();

        assert!(
            err.to_string().to_lowercase().contains("backlog")
                || err.to_string().to_lowercase().contains("intent"),
            "expected backlog/intent mention in: {err}"
        );
    }

    #[test]
    fn should_copy_spec_and_backlog_into_factory_root() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, code_root, factory_root) = setup_ready(dir.path());

        scenarios(&paths, "myapp").unwrap();

        let copied_spec = fs::read_to_string(factory_root.join("SPEC.md")).unwrap();
        let original_spec = fs::read_to_string(code_root.join("SPEC.md")).unwrap();
        assert_eq!(copied_spec, original_spec);

        let copied_backlog = fs::read_to_string(factory_root.join("BACKLOG.md")).unwrap();
        let original_backlog = fs::read_to_string(code_root.join("BACKLOG.md")).unwrap();
        assert_eq!(copied_backlog, original_backlog);
    }

    #[test]
    fn should_write_claude_md_into_factory_root() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, _, factory_root) = setup_ready(dir.path());

        scenarios(&paths, "myapp").unwrap();

        let claude_md = fs::read_to_string(factory_root.join("CLAUDE.md")).unwrap();
        assert!(
            claude_md.contains("myapp"),
            "app name should be substituted"
        );
        assert!(
            claude_md.contains("scenario"),
            "CLAUDE.md should mention scenarios"
        );
        assert!(
            claude_md.contains("session"),
            "CLAUDE.md should mention session discipline"
        );
        assert!(
            !claude_md.contains("{{app}}"),
            "template token should be substituted"
        );
    }

    #[test]
    fn should_return_the_factory_root_path() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, _, factory_root) = setup_ready(dir.path());

        let outcome = scenarios(&paths, "myapp").unwrap();

        assert_eq!(outcome.factory_root, factory_root);
    }

    #[test]
    fn should_be_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, _, _) = setup_ready(dir.path());

        scenarios(&paths, "myapp").unwrap();
        scenarios(&paths, "myapp").unwrap();
    }
}
