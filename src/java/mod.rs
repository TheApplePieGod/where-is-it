// Heavily adapted from https://github.com/dameikle/javalocate

use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::hash::Hash;
use std::io::{self, BufReader};
use java_properties::read;

#[cfg(target_os = "macos")]
use plist::Value;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
extern crate winreg;
#[cfg(target_os = "windows")]
use winreg::RegKey;
#[cfg(target_os = "windows")]
use winreg::enums::HKEY_LOCAL_MACHINE;
#[cfg(target_os = "windows")]
use std::path::Path;

#[cfg(target_os = "linux")]
use std::collections::HashMap;

#[cfg(feature = "node-compile")]
use napi_derive::napi;

/// Command line utility to find JVM versions on macOS, Linux and Windows
#[derive(Clone, Debug)]
pub struct MatchOptions {
    /// JVM Name to filter on
    pub name: Option<String>,

    /// Architecture to filter on (e.g. x86_64, aarch64, amd64)
    pub arch: Option<String>,

    /// Version to filter on (e.g. 1.8, 11, 17, etc)
    pub version: Option<String>
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "node-compile", napi)]
pub struct Jvm {
    pub version: String,
    pub name: String,
    pub architecture: String,
    pub path: String
}

#[derive(Clone)]
struct OperatingSystem {
    name: String,
    architecture: String
}

#[derive(Default)]
struct Config {
    paths: Vec<String>
}

pub fn run(args: MatchOptions) -> Vec<Jvm> {
    let cfg: Config = Default::default();

    // Fetch default java architecture based on kernel
    let operating_system = match get_operating_system() {
        Some(os) => os,
        None => return vec![]
    };

    // Build and filter JVMs
    let jvms: Vec<Jvm> = match collate_jvms(&operating_system, &cfg) {
        Ok(j) => j.into_iter()
                  .filter(|tmp| filter_arch(&args.arch, tmp))
                  .filter(|tmp| filter_ver(&args.version, tmp))
                  .filter(|tmp| filter_name(&args.name, tmp))
                  .collect(),
        Err(_) => vec![]
    };

    jvms
}


#[cfg(any(target_os = "linux", target_os = "macos"))]
fn get_operating_system() -> Option<OperatingSystem> {
    let output = Command::new("uname")
        .arg("-ps")
        .stdout(Stdio::piped())
        .output().unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parts: Vec<String> =
        stdout.split(" ").map(|s| s.to_string()).collect();

    let os = trim_string(parts.get(0).unwrap().as_str());
    let arch = trim_string(parts.get(1).unwrap().as_str());

    let default_architecture =
        if os.eq_ignore_ascii_case("Darwin") {
            if arch.eq_ignore_ascii_case("arm") {
                "aarch64".to_string()
            } else {
                "x86_64".to_string()
            }
        } else if os.eq_ignore_ascii_case("Linux") {
            if arch.eq_ignore_ascii_case("x86_64") {
                "x86_64".to_string()
            } else if arch.eq_ignore_ascii_case("i386") {
                "x86".to_string()
            } else if arch.eq_ignore_ascii_case("i586") {
                "x86".to_string()
            } else if arch.eq_ignore_ascii_case("i686") {
                "x86".to_string()
            } else if arch.eq_ignore_ascii_case("aarch64") {
                "aarch64".to_string()
            } else if arch.eq_ignore_ascii_case("arm64") {
                "arm64".to_string()
            } else {
                return None;
            }
        } else {
            return None;
        };

    let mut name = String::new();
    if os.eq_ignore_ascii_case("Linux") {
        // Attempt to load the Release file into HashMap
        let release_file = File::open("/etc/os-release");
        let release_file = match release_file {
            Ok(release_file) => release_file,
            Err(_error) => return None
        };
        let properties = read(BufReader::new(release_file)).unwrap();
        name.push_str(properties.get("ID").unwrap_or(&"".to_string()).replace("\"", "").as_str());
    } else if os.eq_ignore_ascii_case("Darwin") {
        name.push_str("macOS");
    }

    Some(OperatingSystem {
        name,
        architecture: default_architecture
    })
}

#[cfg(target_os = "windows")]
fn get_operating_system() -> Option<OperatingSystem> {
    let current_version = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion").unwrap();
    let name: String = current_version.get_value("ProductName").unwrap();

    let environment = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment").unwrap();
    let arch: String = environment.get_value("PROCESSOR_ARCHITECTURE").unwrap();
    let default_architecture =
        if arch.eq_ignore_ascii_case("amd64") {
            "x86_64".to_string()
        } else if arch.eq_ignore_ascii_case("x86") {
            "x86".to_string()
        } else if arch.eq_ignore_ascii_case("arm64") {
            "arm64".to_string()
        } else {
            return None;
        };

    Some(OperatingSystem {
        name,
        architecture: default_architecture
    })
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn trim_string(value: &str) -> &str {
    value.strip_suffix("\r\n")
        .or(value.strip_suffix("\n"))
        .unwrap_or(value)
}

#[cfg(target_os = "linux")]
fn collate_jvms(os: &OperatingSystem, cfg: &Config) -> io::Result<Vec<Jvm>> {
    let mut jvms = HashSet::new();
    let dir_lookup = HashMap::from(
        [("ubuntu".to_string(), "/usr/lib/jvm".to_string()),
            ("debian".to_string(), "/usr/lib/jvm".to_string()),
            ("rhel".to_string(), "/usr/lib/jvm".to_string()),
            ("centos".to_string(), "/usr/lib/jvm".to_string()),
            ("fedora".to_string(), "/usr/lib/jvm".to_string())]);

    let path = dir_lookup.get(os.name.as_str());
    if path.is_none() && cfg.paths.is_empty() {
        return Ok(vec![]);
    }
    let mut paths = cfg.paths.to_vec();
    paths.push(path.unwrap().to_string());

    for path in paths {
        for path in fs::read_dir(path).unwrap() {
            let path = path.unwrap().path();
            let metadata = fs::metadata(&path).unwrap();
            let link = fs::read_link(&path);

            if metadata.is_dir() && link.is_err() {
                // Attempt to use release file, if not, attempt to build from folder name
                let release_file = File::open(path.join("release"));
                if release_file.is_ok() {
                    // Collate required information
                    let properties = read(BufReader::new(release_file.unwrap())).unwrap();
                    let version = properties.get("JAVA_VERSION").unwrap_or(&"".to_string()).replace("\"", "");
                    let architecture = properties.get("OS_ARCH").unwrap_or(&"".to_string()).replace("\"", "");
                    let name = path.file_name().unwrap().to_str().unwrap().to_string();

                    // Build JVM Struct
                    let tmp_jvm = Jvm {
                        version,
                        architecture,
                        name,
                        path: path.to_str().unwrap().to_string(),
                    };
                    jvms.insert(tmp_jvm);
                } else {
                    let file_name = path.file_name().unwrap().to_str().unwrap();
                    let parts: Vec<String> = file_name.split("-").map(|s| s.to_string()).collect();
                    // Assuming four part or more form - e.g. "java-8-openjdk-amd64"
                    if parts.len() < 3 || !parts.get(1).unwrap().to_string().eq("java") {
                        continue;
                    }

                    let version = parts.get(1).unwrap().to_string();
                    let mut architecture = parts.get(3).unwrap().to_string();
                    architecture = architecture.replace("amd64", "x86_64");
                    architecture = architecture.replace("i386", "x86");
                    let name = file_name.to_string();

                    // Build JVM Struct
                    let tmp_jvm = Jvm {
                        version,
                        architecture,
                        name,
                        path: path.to_str().unwrap().to_string(),
                    };
                    jvms.insert(tmp_jvm);
                }
            }
        }
    }
    let mut return_vec: Vec<Jvm> = jvms.into_iter().collect();
    return_vec.sort_by(|a, b| compare_boosting_architecture(a, b, &os.architecture));
    return Ok(return_vec);
}

#[cfg(target_os = "macos")]
fn collate_jvms(os: &OperatingSystem, cfg: &Config) -> io::Result<Vec<Jvm>> {
    assert!(os.name.contains("macOS"));
    let mut jvms = HashSet::new();
    let mut paths = cfg.paths.to_vec();
    paths.push("/Library/Java/JavaVirtualMachines".to_string());
    for path in paths {
        for path in fs::read_dir(path)? {
            let path = path.unwrap().path();
            let metadata = fs::metadata(&path)?;

            if metadata.is_dir() {
                // Attempt to load the Info PList
                let info =
                    Value::from_file(path.join("Contents/Info.plist"));

                let info = match info {
                    Ok(info) => info,
                    Err(_error) => continue,
                };
                let name = info
                    .as_dictionary()
                    .and_then(|dict| dict.get("CFBundleName"))
                    .and_then(|info_string| info_string.as_string());
                let name = name.unwrap_or(&"".to_string()).replace("\"", "");

                // Attempt to load the Release file into HashMap
                let release_file = File::open(path.join("Contents/Home/release"));
                let release_file = match release_file {
                    Ok(release_file) => release_file,
                    Err(_error) => continue,
                };

                // Collate required information
                let properties = match read(BufReader::new(release_file)) {
                    Ok(p) => p,
                    Err(err) => return Err(io::Error::new(io::ErrorKind::Other, err.to_string()))
                };
                let version = properties.get("JAVA_VERSION").unwrap_or(&"".to_string()).replace("\"", "");
                let architecture = properties.get("OS_ARCH").unwrap_or(&"".to_string()).replace("\"", "");

                // Build JVM Struct
                let tmp_jvm = Jvm {
                    version,
                    architecture,
                    name,
                    path: path.join("Contents/Home").to_str().unwrap().to_string(),
                };
                jvms.insert(tmp_jvm);
            }
        }
    }
    let mut return_vec: Vec<Jvm> = jvms.into_iter().collect();
    return_vec.sort_by(|a, b| compare_boosting_architecture(a, b, &os.architecture));
    return Ok(return_vec);
}

#[cfg(target_os = "windows")]
fn collate_jvms(os: &OperatingSystem, cfg: &Config) -> io::Result<Vec<Jvm>> {
    assert!(os.name.contains("Windows"));
    let mut jvms = HashSet::new();

    // Loop round software keys in the registry
    let system = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey("SOFTWARE").unwrap();
    for name in system.enum_keys().map(|x| x.unwrap()) {
        let software: String = name.clone();
        // Find software with JDK key
        for jdk in system.open_subkey(name).unwrap().enum_keys()
                            .map(|x| x.unwrap())
                            .filter(|x| x.starts_with("JDK") || x.starts_with("Java Development Kit")) {
            // Next key should be JVM
            for jvm in system.open_subkey(format!("{}\\{}", software, jdk)).unwrap().enum_keys().map(|x| x.unwrap()) {
                let mut jvm_path = String::new();
                // Old style JavaSoftware entry
                let java_home: Result<String, _> = system.open_subkey(format!("{}\\{}\\{}", software, jdk, jvm)).unwrap().get_value("JavaHome");
                if java_home.is_ok() {
                    jvm_path = java_home.unwrap();
                }
                // Per JVM Entry - check for Hotspot or OpenJ9 entry
                let hotspot_path: Result<RegKey, _> = system.open_subkey(format!("{}\\{}\\{}\\hotspot\\MSI", software, jdk, jvm));
                if hotspot_path.is_ok() {
                    jvm_path = hotspot_path.unwrap().get_value("Path").unwrap();
                }
                let openj9_path: Result<RegKey, _> = system.open_subkey(format!("{}\\{}\\{}\\openj9\\MSI", software, jdk, jvm));
                if openj9_path.is_ok() {
                    jvm_path = openj9_path.unwrap().get_value("Path").unwrap();
                }
                jvm_path = jvm_path.strip_suffix("\\").unwrap_or(jvm_path.as_str()).to_string();

                let path = Path::new(jvm_path.as_str()).join("release");
                let release_file = File::open(path);
                if release_file.is_ok() {
                    jvms.insert(process_release_file(&jvm_path, release_file.unwrap()));
                }
            }
        }
    }
    // Read from Custom JVM Location Paths
    if !cfg.paths.is_empty() {
        for path in &cfg.paths {
            for path in fs::read_dir(path).unwrap() {
                let jvm_path = path.unwrap().path();
                let metadata = fs::metadata(&jvm_path).unwrap();

                if metadata.is_dir() {
                    let path = Path::new(jvm_path.to_str().unwrap()).join("release");
                    let release_file = File::open(&path);
                    if release_file.is_ok() {
                        jvms.insert(process_release_file(&jvm_path.to_str().unwrap().to_string(), release_file.unwrap()));
                    }
                }

            }
        }
    }
    let mut return_vec: Vec<Jvm> = jvms.into_iter().collect();
    return_vec.sort_by(|a, b| compare_boosting_architecture(a, b, &os.architecture));
    return Ok(return_vec);
}

#[cfg(target_os = "windows")]
fn process_release_file(jvm_path: &String, release_file: File) -> Jvm {
    // Collate required information
    let properties = read(BufReader::new(release_file)).unwrap();
    let version = properties.get("JAVA_VERSION").unwrap_or(&"".to_string()).replace("\"", "");
    let mut architecture = properties.get("OS_ARCH").unwrap_or(&"".to_string()).replace("\"", "");
    architecture = architecture.replace("amd64", "x86_64");
    architecture = architecture.replace("i386", "x86");
    let implementor = properties.get("IMPLEMENTOR").unwrap_or(&"".to_string()).replace("\"", "");
    let name = format!("{} - {}", implementor, version);

    // Build JVM Struct
    let tmp_jvm = Jvm {
        version,
        architecture,
        name,
        path: jvm_path.to_string(),
    };
    tmp_jvm
}

fn compare_boosting_architecture(a: &Jvm, b: &Jvm, default_arch: &String) -> Ordering {
    let version_test = compare_version_values(&b.version, &a.version);
    if version_test == Ordering::Equal {
        if b.architecture != default_arch.as_str() && a.architecture == default_arch.as_str() {
            return Ordering::Less;
        }
        if b.architecture == default_arch.as_str() && a.architecture != default_arch.as_str() {
            return Ordering::Greater;
        }
    }
    return version_test;
}

fn filter_ver(ver: &Option<String>, jvm: &Jvm) -> bool {
    if !ver.is_none() {
        let version = ver.as_ref().unwrap();
        if version.contains("+") {
            let sanitised_version = version.replace("+", "");
            let compare_jvm_version = get_compare_version(jvm, &sanitised_version);
            let compare = compare_version_values(&compare_jvm_version, &sanitised_version);
            if compare.is_lt() {
                return false;
            }
        } else {
            let compare_jvm_version = get_compare_version(jvm, version);
            let compare = compare_version_values(&version, &compare_jvm_version);
            if compare.is_ne() {
                return false;
            }
        }
    }
    return true;
}

fn compare_version_values(version1: &String, version2: &String) -> Ordering {
    // Normalise old style versions - e.g. 1.8 -> 8, 1.9 -> 9
    let mut normalised1= version1.strip_prefix("1.")
        .unwrap_or(version1.as_str()).to_string();
    let mut normalised2= version2.strip_prefix("1.")
        .unwrap_or(version2.as_str()).to_string();
    // Normalise old sub versions e.g. 1.8.0_292 -> 1.8.0.292
    normalised1 = normalised1.replace("_", ".");
    normalised2 = normalised2.replace("_", ".");

    let count_version1: Vec<String> =
        normalised1.split(".").map(|s| s.to_string()).collect();
    let count_version2: Vec<String> =
        normalised2.split(".").map(|s| s.to_string()).collect();

    let compare = Ordering::Equal;
    let max_size = std::cmp::max(count_version1.len(), count_version2.len());

    for i in 0..max_size {
        if count_version1.get(i).is_none(){
            return Ordering::Less
        }
        if count_version2.get(i).is_none(){
            return Ordering::Greater
        }
        let version1_int = count_version1.get(i).unwrap().parse::<i32>().unwrap();
        let version2_int = count_version2.get(i).unwrap().parse::<i32>().unwrap();
        if version1_int > version2_int {
            return Ordering::Greater
        } else if version1_int < version2_int {
            return Ordering::Less;
        } else {
            continue;
        }
    }
    return compare;
}

fn get_compare_version(jvm: &Jvm, version: &String) -> String {
    let version_count = version.matches('.').count();
    let mut  jvm_version = jvm.version.clone();

    // Normalise single digit compares for old style versions
    if jvm.version.starts_with("1.") && version.matches('.').count() == 0 {
        if !version.starts_with("1.") {
            jvm_version = jvm_version.strip_prefix("1.")
                .unwrap_or(jvm_version.as_str()).to_string();
        }
    }

    let tmp_version: Vec<String> =
        jvm_version.split_inclusive(".").map(|s| s.to_string()).collect();
    let mut compare_version: String = String::new();
    for i in 0..version_count + 1 {
        compare_version.push_str(tmp_version.get(i).unwrap_or(&"".to_string()));
    }
    compare_version = compare_version.strip_suffix(".")
        .unwrap_or(compare_version.as_str()).to_string();
    compare_version
}

fn filter_arch(arch: &Option<String>, jvm: &Jvm) -> bool {
    if !arch.is_none() {
        if jvm.architecture != arch.as_ref().unwrap().to_string() {
            return false;
        }
    }
    return true;
}

fn filter_name(name: &Option<String>, jvm: &Jvm) -> bool {
    if !name.is_none() {
        if jvm.name != name.as_ref().unwrap().to_string() {
            return false;
        }
    }
    return true;
}
