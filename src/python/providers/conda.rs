use std::{fmt::Debug, path::PathBuf};

use super::Provider;

use crate::python::python::PythonVersion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CondaProvider {
    roots: Vec<PathBuf>,
}

impl CondaProvider {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }
}

impl Provider for CondaProvider {
    fn create() -> Option<Self> {
        let roots = vec![
            std::env::var_os("CONDA_ROOT")
                .or_else(|| Some(dirs::home_dir()?.join(".conda").join("envs").into_os_string()))?,
            dirs::home_dir()?.join("miniconda3").join("envs").into_os_string(),
            dirs::home_dir()?.join("anaconda3").join("envs").into_os_string(),
            dirs::home_dir()?.join("conda").join("envs").into_os_string()
        ];
            
        Some(Self::new(
            roots
                .into_iter()
                .map(|r| PathBuf::from(r))
                .collect()
        ))
    }

    fn find_pythons(&self) -> Vec<PythonVersion> {
        let mut versions = vec![];

        for root in &self.roots {
            versions.extend(match root.read_dir() {
                Ok(entries) => entries
                    .into_iter()
                    .flat_map(|entry| match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            let env = path.file_name().unwrap().to_str().unwrap();
                            if path.is_dir() {
                                let bin = path.join("bin");
                                let mut found = super::find_pythons_from_path(&bin, true);
                                found.iter_mut()
                                    .for_each(|v| v.formatted_name = Some(format!("Conda '{}'", env)));
                                found
                            } else {
                                vec![]
                            }
                        },
                        Err(_) => vec![]
                    })
                    .collect(),
                Err(_) => vec![]
            })
        }

        versions
    }
}
