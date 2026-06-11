//! Path resolution. `factory` owns where the registry and the factory roots live
//! (ADR-0003): the per-user data directory, or `FACTORY_HOME` when set. A single
//! `FACTORY_HOME` therefore isolates the whole factory — registry and all factory
//! roots — which is the seam sandboxed scenario runs use.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;

/// `home` is the factory-owned root for the registry and factory roots. `cwd` is
/// where code roots are created.
pub struct Paths {
    pub home: PathBuf,
    pub cwd: PathBuf,
}

impl Paths {
    /// Resolve from the real environment: `FACTORY_HOME` if set, otherwise the
    /// OS-standard per-user data directory; code roots are created under the
    /// current working directory.
    pub fn resolve() -> Result<Paths> {
        let home = match env::var_os("FACTORY_HOME") {
            Some(value) => PathBuf::from(value),
            None => ProjectDirs::from("", "", "factory")
                .context("could not determine the per-user data directory")?
                .data_dir()
                .to_path_buf(),
        };
        let cwd = env::current_dir().context("could not determine the working directory")?;
        Ok(Paths::new(home, cwd))
    }

    pub fn new(home: impl Into<PathBuf>, cwd: impl Into<PathBuf>) -> Paths {
        Paths {
            home: home.into(),
            cwd: cwd.into(),
        }
    }

    pub fn registry_path(&self) -> PathBuf {
        self.home.join("registry.json")
    }

    pub fn factory_root(&self, app: &str) -> PathBuf {
        self.home.join("factories").join(app)
    }

    pub fn code_root(&self, app: &str) -> PathBuf {
        self.cwd.join(app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn should_place_registry_and_factory_root_under_home() {
        let paths = Paths::new("/data/factory", "/work");

        assert_eq!(
            paths.registry_path(),
            Path::new("/data/factory/registry.json")
        );
        assert_eq!(
            paths.factory_root("myapp"),
            Path::new("/data/factory/factories/myapp")
        );
    }

    #[test]
    fn should_place_code_root_under_cwd() {
        let paths = Paths::new("/data/factory", "/work");

        assert_eq!(paths.code_root("myapp"), Path::new("/work/myapp"));
    }
}
