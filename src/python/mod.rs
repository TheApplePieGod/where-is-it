mod providers;
mod finder;
mod helpers;
mod python;

pub use finder::MatchOptions;

#[cfg(feature = "node-compile")]
use napi_derive::napi;

// Evaluated, simplified version of python::PythonVersion
#[derive(Debug, Clone)]
#[cfg_attr(feature = "node-compile", napi)]
pub struct Version {
    pub executable: String,
    pub formatted_name: Option<String>,
    pub version: Option<String>
}

pub fn run(args: MatchOptions) -> Vec<Version> {
    let finder = finder::Finder::default();
    finder
        .find_all(args)
        .into_iter()
        .map(|v| Version {
            executable: String::from(v.executable.to_str().unwrap()),
            formatted_name: v.formatted_name.clone(),
            version: match v.version() {
                Ok(v) => Some(v.to_string()),
                Err(_) => None
            }
        })
        .collect()
}

