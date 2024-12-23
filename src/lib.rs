#[cfg(feature = "java")]
pub mod java;

#[cfg(feature = "python")]
pub mod python;


// =================================


#[cfg(feature = "node-compile")]
use napi_derive::napi;

#[napi]
#[cfg(feature = "node-compile")]
pub fn node_find_python(
    major: Option<u32>,
    minor: Option<u32>,
    patch: Option<u32>,
    pre: Option<bool>,
    dev: Option<bool>,
    name: Option<String>,
    architecture: Option<String>
) -> Vec<python::Version> {
    python::run(python::MatchOptions {
        major: match major {
            Some(m) => Some(m as usize),
            None => None
        },
        minor: match minor {
            Some(m) => Some(m as usize),
            None => None
        },
        patch: match patch {
            Some(p) => Some(p as usize),
            None => None
        },
        pre,
        dev,
        name,
        architecture
    })
}

#[napi]
#[cfg(feature = "node-compile")]
pub fn node_find_java(name: Option<String>, arch: Option<String>, version: Option<String>) -> Vec<java::Jvm> {
    java::run(java::MatchOptions {
        name,
        arch,
        version
    })
}
