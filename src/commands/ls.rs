//! `factory ls` (B5): list every registered app with its last-known state (SPEC,
//! ADR-0003). Reads the registry; writes nothing.
//!
//! Data-gathering (`rows`) is kept separate from rendering (`render`) so a future
//! `ls --json` is a small addition, not a refactor. Rows come out sorted by app name
//! (the registry is a `BTreeMap`), so the listing is stable between runs.

use anyhow::Result;
use comfy_table::{presets, Cell, Table};

use crate::paths::Paths;
use crate::registry::{Mode, Registry};

/// One row of the listing — the registry projected to what `ls` shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LsRow {
    pub app: String,
    pub mode: Mode,
    pub last_satisfaction: Option<u8>,
    pub last_terminal_state: Option<String>,
    pub last_run_at: Option<String>,
}

/// Project the registry into rows, in app-name order.
pub fn rows(registry: &Registry) -> Vec<LsRow> {
    registry
        .apps
        .iter()
        .map(|(app, entry)| LsRow {
            app: app.clone(),
            mode: entry.mode,
            last_satisfaction: entry.last_satisfaction,
            last_terminal_state: entry.last_terminal_state.clone(),
            last_run_at: entry.last_run_at.clone(),
        })
        .collect()
}

/// Render the rows as an aligned, borderless table. An empty registry gets a clear
/// line rather than a bare header.
pub fn render(rows: &[LsRow]) -> String {
    if rows.is_empty() {
        return "no registered projects\n".to_string();
    }
    let mut table = Table::new();
    table.load_preset(presets::NOTHING);
    table.set_header(vec!["APP", "MODE", "SAT", "LAST STATE", "LAST RUN"]);
    for row in rows {
        table.add_row(vec![
            Cell::new(&row.app),
            Cell::new(mode_str(row.mode)),
            Cell::new(sat_str(row.last_satisfaction)),
            Cell::new(or_dash(row.last_terminal_state.as_deref())),
            Cell::new(or_dash(row.last_run_at.as_deref())),
        ]);
    }
    format!("{table}\n")
}

pub fn ls(paths: &Paths) -> Result<String> {
    let registry = Registry::load(&paths.registry_path())?;
    Ok(render(&rows(&registry)))
}

fn mode_str(mode: Mode) -> &'static str {
    match mode {
        Mode::Greenfield => "greenfield",
        Mode::Brownfield => "brownfield",
    }
}

fn sat_str(satisfaction: Option<u8>) -> String {
    match satisfaction {
        Some(value) => format!("{value}%"),
        None => "—".to_string(),
    }
}

fn or_dash(value: Option<&str>) -> String {
    value.unwrap_or("—").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::AppEntry;
    use std::path::PathBuf;

    fn registry_with(entries: Vec<(&str, AppEntry)>) -> Registry {
        let mut registry = Registry::default();
        for (app, entry) in entries {
            registry.upsert(app, entry);
        }
        registry
    }

    fn entry(mode: Mode, sat: Option<u8>, state: Option<&str>, at: Option<&str>) -> AppEntry {
        AppEntry {
            code_root: PathBuf::from("/code"),
            factory_root: PathBuf::from("/factory"),
            mode,
            last_satisfaction: sat,
            last_terminal_state: state.map(str::to_string),
            last_run_id: at.map(|_| "run-x".to_string()),
            last_run_at: at.map(str::to_string),
        }
    }

    #[test]
    fn should_project_registry_into_rows_sorted_by_app() {
        let registry = registry_with(vec![
            (
                "widget",
                entry(Mode::Brownfield, Some(50), Some("ESCALATE"), None),
            ),
            (
                "demo",
                entry(
                    Mode::Greenfield,
                    Some(100),
                    Some("PR_READY"),
                    Some("2026-06-07T14:30:03Z"),
                ),
            ),
        ]);

        let rows = rows(&registry);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].app, "demo"); // sorted by name
        assert_eq!(rows[1].app, "widget");
        assert_eq!(rows[0].last_run_at.as_deref(), Some("2026-06-07T14:30:03Z"));
    }

    #[test]
    fn should_render_never_run_fields_as_dashes() {
        let registry = registry_with(vec![("fresh", entry(Mode::Greenfield, None, None, None))]);

        let out = render(&rows(&registry));

        assert!(out.contains("fresh"));
        assert!(out.contains("greenfield"));
        assert!(out.contains('—'));
    }

    #[test]
    fn should_render_clear_line_for_empty_registry() {
        assert_eq!(render(&[]), "no registered projects\n");
    }

    #[test]
    fn should_render_app_state_and_time() {
        let registry = registry_with(vec![(
            "demo",
            entry(
                Mode::Greenfield,
                Some(100),
                Some("PR_READY"),
                Some("2026-06-07T14:30:03Z"),
            ),
        )]);

        let out = render(&rows(&registry));

        assert!(out.contains("demo"));
        assert!(out.contains("100%"));
        assert!(out.contains("PR_READY"));
        assert!(out.contains("2026-06-07T14:30:03Z"));
    }
}
