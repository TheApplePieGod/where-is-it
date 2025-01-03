// Heavily adapted from https://github.com/frostming/findpython

use std::{collections::HashMap, io};

use crate::python::{helpers::suffix_preference, providers::*, python::PythonVersion};
use fancy_regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    static ref VERSION_REGEX: Regex = Regex::new(
        r#"(?x)
        ^(?P<major>\d+)(?:\.(?P<minor>\d+)(?:\.(?P<patch>[0-9]+))?)?\.?
        (?:(?P<prerel>[abc]|rc|dev)(?:(?P<prerelversion>\d+(?:\.\d+)*))?)
        ?(?P<postdev>(\.post(?P<post>\d+))?(\.dev(?P<dev>\d+))?)?
        (?:-(?P<architecture>32|64))?"#
    )
    .unwrap();
}

pub struct Finder {
    providers: Vec<Box<dyn Provider>>,
    resolve_symlinks: bool,
    same_file: bool,
    same_interpreter: bool,
}

impl Default for Finder {
    fn default() -> Self {
        let f = Self {
            providers: vec![],
            resolve_symlinks: false,
            same_file: true,
            same_interpreter: true,
        };
        f.select_providers(&ALL_PROVIDERS[..]).unwrap()
    }
}

impl Finder {
    pub fn select_providers(mut self, names: &[&str]) -> Result<Self, io::Error> {
        self.providers = names.iter().filter_map(|n| get_provider(*n)).collect();
        Ok(self)
    }

    pub fn resolve_symlinks(mut self, resolve_symlinks: bool) -> Self {
        self.resolve_symlinks = resolve_symlinks;
        self
    }

    pub fn same_file(mut self, same_file: bool) -> Self {
        self.same_file = same_file;
        self
    }

    pub fn same_interpreter(mut self, same_interpreter: bool) -> Self {
        self.same_interpreter = same_interpreter;
        self
    }

    fn find_all_python_versions(&self) -> Vec<PythonVersion> {
        self.providers
            .iter()
            .flat_map(|p| p.find_pythons())
            .collect()
    }

    pub fn find_all(&self, options: MatchOptions) -> Vec<PythonVersion> {
        let pythons = self.find_all_python_versions();
        let mut filtered = vec![];
        for python in pythons {
            if python.matches(&options) {
                filtered.push(python);
            }
        }
        self.deduplicate(filtered)
    }

    pub fn find(&self, options: MatchOptions) -> Option<PythonVersion> {
        self.find_all(options).first().cloned()
    }

    fn deduplicate_key(&self, python: &mut PythonVersion) -> String {
        if !self.same_interpreter {
            return python.interpreter().unwrap().to_str().unwrap().to_string();
        }
        if !self.same_file {
            return python.content_hash().unwrap();
        }
        if self.resolve_symlinks && !python.keep_symlink {
            return python.real_path().to_str().unwrap().to_string();
        }
        python.executable.to_str().unwrap().to_string()
    }

    fn deduplicate(&self, versions: Vec<PythonVersion>) -> Vec<PythonVersion> {
        let mut result = HashMap::new();
        let mut versions = versions;

        versions.sort_by_cached_key(|p| {
            (
                p.executable.is_symlink(),
                suffix_preference(&p.executable),
                -(p.executable.to_string_lossy().len() as isize),
            )
        });

        for version in versions.iter_mut() {
            let key = self.deduplicate_key(version);
            result.entry(key).or_insert(version.to_owned());
        }
        let mut py_versions = result.into_values().collect::<Vec<_>>();
        py_versions.sort_by(|a, b| {
            (b.version().unwrap(), b.executable.to_string_lossy().len())
                .cmp(&(a.version().unwrap(), a.executable.to_string_lossy().len()))
        });
        py_versions
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MatchOptions {
    pub major: Option<usize>,
    pub minor: Option<usize>,
    pub patch: Option<usize>,
    pub pre: Option<bool>,
    pub dev: Option<bool>,
    pub name: Option<String>,
    pub architecture: Option<String>,
}

impl MatchOptions {
    fn from_version(version: &str) -> Option<Self> {
        match VERSION_REGEX.captures(version) {
            Ok(Some(capture)) => Some(Self {
                major: capture.name("major").map(|m| m.as_str().parse().unwrap()),
                minor: capture.name("minor").map(|m| m.as_str().parse().unwrap()),
                patch: capture.name("patch").map(|m| m.as_str().parse().unwrap()),
                pre: capture.name("prerel").map(|_| true),
                dev: capture.name("dev").map(|_| true),
                name: None,
                architecture: capture
                    .name("architecture")
                    .map(|m| format!("{}bit", m.as_str())),
            }),
            _ => None,
        }
    }

    pub fn version_spec(self, version: &str) -> Self {
        if let Some(res) = Self::from_version(version) {
            res
        } else {
            self.name(version)
        }
    }

    pub fn major(mut self, major: usize) -> Self {
        self.major = Some(major);
        self
    }

    pub fn minor(mut self, minor: usize) -> Self {
        self.minor = Some(minor);
        self
    }

    pub fn patch(mut self, patch: usize) -> Self {
        self.patch = Some(patch);
        self
    }

    pub fn pre(mut self, pre: bool) -> Self {
        self.pre = Some(pre);
        self
    }

    pub fn dev(mut self, dev: bool) -> Self {
        self.dev = Some(dev);
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn architecture(mut self, architecture: &str) -> Self {
        self.architecture = Some(architecture.to_string());
        self
    }
}
