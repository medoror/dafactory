//! The registry: the only persistent state `factory` owns (ADR-0003).
//!
//! Maps each app to its code root, factory root, mode, and last-known status.
//! Serialized as JSON at `<home>/registry.json`.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Whether a line was scaffolded as greenfield or brownfield. v0 treats both modes
/// identically when laying down templates (SPEC), but the registry records which was
/// asked for so `ls` can report it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Greenfield,
    Brownfield,
}

/// One registered app and its last-known status. Status fields are `None` until a
/// `validate`/`run` fills them in (B2+).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppEntry {
    pub code_root: PathBuf,
    pub factory_root: PathBuf,
    pub mode: Mode,
    #[serde(default)]
    pub last_satisfaction: Option<u8>,
    #[serde(default)]
    pub last_terminal_state: Option<String>,
    /// Opaque pointer to the last run's evidence bundle (the run id). Format private;
    /// never parsed for its time — that is `last_run_at`'s job.
    #[serde(default)]
    pub last_run_id: Option<String>,
    /// ISO-8601 UTC timestamp of the last run/validate.
    #[serde(default)]
    pub last_run_at: Option<String>,
}

/// The registry document. Keyed by app name for deterministic ordering and easy
/// upsert.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub apps: BTreeMap<String, AppEntry>,
}

impl Registry {
    /// Load the registry from `path`. A missing file is an empty registry, not an
    /// error — a project survives the registry being deleted (ADR-0003).
    pub fn load(path: &Path) -> Result<Registry> {
        match fs::read_to_string(path) {
            Ok(contents) => serde_json::from_str(&contents)
                .with_context(|| format!("failed to parse registry at {}", path.display())),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Registry::default()),
            Err(err) => {
                Err(err).with_context(|| format!("failed to read registry at {}", path.display()))
            }
        }
    }

    /// Write the registry to `path`, creating parent directories as needed.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self).context("failed to serialize registry")?;
        fs::write(path, json)
            .with_context(|| format!("failed to write registry at {}", path.display()))
    }

    /// Insert or replace the entry for `name`. Re-`init` overwrites in place,
    /// leaving a single entry (idempotent registration).
    pub fn upsert(&mut self, name: &str, entry: AppEntry) {
        self.apps.insert(name.to_string(), entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> AppEntry {
        AppEntry {
            code_root: PathBuf::from("/code/myapp"),
            factory_root: PathBuf::from("/home/factories/myapp"),
            mode: Mode::Greenfield,
            last_satisfaction: None,
            last_terminal_state: None,
            last_run_id: None,
            last_run_at: None,
        }
    }

    #[test]
    fn should_return_empty_registry_when_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("registry.json");

        let registry = Registry::load(&path).unwrap();

        assert!(registry.apps.is_empty());
    }

    #[test]
    fn should_round_trip_registry_through_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("registry.json");
        let mut registry = Registry::default();
        registry.upsert("myapp", sample_entry());

        registry.save(&path).unwrap();
        let loaded = Registry::load(&path).unwrap();

        assert_eq!(loaded, registry);
    }

    #[test]
    fn should_overwrite_entry_when_upserting_existing_app() {
        let mut registry = Registry::default();
        registry.upsert("myapp", sample_entry());

        let mut updated = sample_entry();
        updated.mode = Mode::Brownfield;
        registry.upsert("myapp", updated.clone());

        assert_eq!(registry.apps.len(), 1);
        assert_eq!(registry.apps["myapp"], updated);
    }

    #[test]
    fn should_serialize_mode_as_lowercase() {
        let json = serde_json::to_string(&Mode::Greenfield).unwrap();
        assert_eq!(json, "\"greenfield\"");
    }
}
