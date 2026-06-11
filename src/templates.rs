//! Scaffolding templates, embedded in the binary (ADR-0005). `init` writes these
//! out; it never reads them from disk, so a standalone binary can scaffold anywhere.
//!
//! Most files are blank skeleton forms with a single `{{app}}` substitution token.
//! `CLAUDE.md` and `judge.md` ship verbatim — the reusable workflow/rails and judge
//! discipline are identical for every line.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// A single embedded template: where it lands under a root, and its contents.
pub struct TemplateFile {
    pub rel_path: &'static str,
    pub contents: &'static str,
}

/// Files laid down in the code root.
pub const CODE_TEMPLATES: &[TemplateFile] = &[
    TemplateFile {
        rel_path: "SPEC.md",
        contents: include_str!("../templates/code/SPEC.md"),
    },
    TemplateFile {
        rel_path: "BACKLOG.md",
        contents: include_str!("../templates/code/BACKLOG.md"),
    },
    TemplateFile {
        rel_path: "PROGRESS.md",
        contents: include_str!("../templates/code/PROGRESS.md"),
    },
    TemplateFile {
        rel_path: "CLAUDE.md",
        contents: include_str!("../templates/code/CLAUDE.md"),
    },
    TemplateFile {
        rel_path: "adr/0001-record-architecture-decisions.md",
        contents: include_str!("../templates/code/adr/0001-record-architecture-decisions.md"),
    },
    TemplateFile {
        rel_path: ".gitignore",
        contents: include_str!("../templates/code/.gitignore"),
    },
];

/// Files laid down in the factory (holdout) root.
pub const FACTORY_TEMPLATES: &[TemplateFile] = &[
    TemplateFile {
        rel_path: "judge.md",
        contents: include_str!("../templates/factory/judge.md"),
    },
    TemplateFile {
        rel_path: "scenarios/README.md",
        contents: include_str!("../templates/factory/scenarios/README.md"),
    },
];

/// Write the full template set into both roots and create the evidence directory
/// (ADR-0006). Overwrites existing files so re-`init` is idempotent.
pub fn scaffold(code_root: &Path, factory_root: &Path, app: &str) -> Result<()> {
    write_templates(code_root, CODE_TEMPLATES, app)?;
    write_templates(factory_root, FACTORY_TEMPLATES, app)?;

    let evidence = factory_root.join("evidence");
    fs::create_dir_all(&evidence)
        .with_context(|| format!("failed to create {}", evidence.display()))?;
    Ok(())
}

fn write_templates(root: &Path, templates: &[TemplateFile], app: &str) -> Result<()> {
    for template in templates {
        let path = root.join(template.rel_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let contents = template.contents.replace("{{app}}", app);
        fs::write(&path, contents)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_write_every_expected_file_in_both_roots() {
        let dir = tempfile::tempdir().unwrap();
        let code_root = dir.path().join("myapp");
        let factory_root = dir.path().join("factories/myapp");

        scaffold(&code_root, &factory_root, "myapp").unwrap();

        for expected in ["SPEC.md", "BACKLOG.md", "PROGRESS.md", "CLAUDE.md"] {
            assert!(
                code_root.join(expected).is_file(),
                "missing code-root file: {expected}"
            );
        }
        assert!(code_root
            .join("adr/0001-record-architecture-decisions.md")
            .is_file());
        assert!(factory_root.join("judge.md").is_file());
        assert!(factory_root.join("scenarios/README.md").is_file());
        assert!(factory_root.join("evidence").is_dir());
    }

    #[test]
    fn should_substitute_app_name_into_templates() {
        let dir = tempfile::tempdir().unwrap();
        let code_root = dir.path().join("widget");
        let factory_root = dir.path().join("factories/widget");

        scaffold(&code_root, &factory_root, "widget").unwrap();

        let spec = fs::read_to_string(code_root.join("SPEC.md")).unwrap();
        assert!(spec.contains("widget"));
        assert!(!spec.contains("{{app}}"));
    }

    #[test]
    fn should_overwrite_existing_files_on_rescaffold() {
        let dir = tempfile::tempdir().unwrap();
        let code_root = dir.path().join("myapp");
        let factory_root = dir.path().join("factories/myapp");
        scaffold(&code_root, &factory_root, "myapp").unwrap();
        fs::write(code_root.join("SPEC.md"), "user edits").unwrap();

        scaffold(&code_root, &factory_root, "myapp").unwrap();

        let spec = fs::read_to_string(code_root.join("SPEC.md")).unwrap();
        assert!(spec.contains("# SPEC"));
        assert!(!spec.contains("user edits"));
    }
}
