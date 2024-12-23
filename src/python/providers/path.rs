// Heavily adapted from https://github.com/frostming/findpython

use std::path::PathBuf;

use super::Provider;
use crate::python::python::PythonVersion;

/// A provider that searches Python interpreters in the PATH.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PathProvider {
    paths: Vec<PathBuf>,
}

impl PathProvider {
    pub fn new() -> Self {
        let path_env = std::env::var_os("PATH").unwrap_or_default();
        Self {
            paths: std::env::split_paths(&path_env).collect(),
        }
    }
}

impl Provider for PathProvider {
    fn create() -> Option<Self> {
        Some(Self::new())
    }

    fn find_pythons(&self) -> Vec<PythonVersion> {
        self.paths
            .iter()
            .flat_map(|path| super::find_pythons_from_path(path, false))
            .collect()
    }
}
