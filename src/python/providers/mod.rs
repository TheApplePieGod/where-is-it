// Heavily adapted from https://github.com/frostming/findpython

use std::path::PathBuf;

use crate::python::helpers;
use crate::python::python::PythonVersion;
use lazy_static::lazy_static;

mod asdf;
mod path;
mod conda;
mod pyenv;
mod rye;

#[cfg(windows)]
mod winreg;

#[cfg(windows)]
lazy_static! {
    pub static ref ALL_PROVIDERS: [&'static str; 6] = ["path", "conda", "pyenv", "rye", "asdf", "winreg"];
}

#[cfg(not(windows))]
lazy_static! {
    pub static ref ALL_PROVIDERS: [&'static str; 5] = ["path", "conda", "pyenv", "rye", "asdf"];
}

pub trait Provider: Send + Sync {
    fn create() -> Option<Self>
    where
        Self: Sized;

    fn find_pythons(&self) -> Vec<PythonVersion>;
}

pub fn get_provider(name: &str) -> Option<Box<dyn Provider>> {
    match name {
        "path" => path::PathProvider::create().map(|p| Box::new(p) as Box<dyn Provider>),
        "conda" => conda::CondaProvider::create().map(|p| Box::new(p) as Box<dyn Provider>),
        "pyenv" => pyenv::PyenvProvider::create().map(|p| Box::new(p) as Box<dyn Provider>),
        "rye" => rye::RyeProvider::create().map(|p| Box::new(p) as Box<dyn Provider>),
        "asdf" => asdf::AsdfProvider::create().map(|p| Box::new(p) as Box<dyn Provider>),
        #[cfg(windows)]
        "winreg" => winreg::WinRegProvider::create().map(|p| Box::new(p) as Box<dyn Provider>),
        _ => None,
    }
}

/// Find all Python versions under the given path.
/// ### Arguments:
///
/// path: The path to search for Python versions under.
/// as_interpreter: Whether to use the path as an interpreter.
///     Must not be true if it might be a wrapper script.
///
/// ### Returns:
/// A list of Python versions found under the given path.
pub fn find_pythons_from_path(path: &PathBuf, as_interpreter: bool) -> Vec<PythonVersion> {
    let mut found = false;
    let first_only = true;
    match path.read_dir() {
        Ok(entries) => entries
            .into_iter()
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if !(found && first_only) && helpers::path_is_python(&path) {
                    let mut python = PythonVersion::new(path.to_owned());
                    if as_interpreter {
                        python = python.with_interpreter(path.to_owned());
                    }
                    found = true;
                    Some(python)
                } else {
                    None
                }
            })
            .collect(),
        Err(_) => vec![],
    }
}
