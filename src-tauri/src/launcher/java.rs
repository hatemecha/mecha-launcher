use std::{
    path::{Path, PathBuf},
    process::Command,
};

use regex::Regex;
use walkdir::WalkDir;

use crate::launcher::{manifest::JavaVersion, LauncherError, LauncherResult};

pub fn resolve_java_executable(
    minecraft_dir: &Path,
    java_version: Option<&JavaVersion>,
) -> LauncherResult<PathBuf> {
    if let Some(java_version) = java_version {
        if let Some(runtime_java) = find_runtime_java(minecraft_dir, &java_version.component) {
            return Ok(runtime_java);
        }
    }

    let system_candidate = if cfg!(windows) { "java.exe" } else { "java" };
    let required_major = java_version.map(|version| version.major_version);

    let detected_major = detect_java_major(Path::new(system_candidate)).ok_or_else(|| {
        LauncherError::new(
            "Unable to find a compatible Java runtime in .minecraft/runtime or in PATH.",
        )
    })?;

    if let Some(required_major) = required_major {
        if detected_major != required_major {
            return Err(LauncherError::new(format!(
                "The system Java runtime reports major version {detected_major}, but the selected version requires Java {required_major}."
            )));
        }
    }

    Ok(PathBuf::from(system_candidate))
}

fn find_runtime_java(minecraft_dir: &Path, component: &str) -> Option<PathBuf> {
    let runtime_root = minecraft_dir.join("runtime").join(component);
    if !runtime_root.exists() {
        return None;
    }

    let executable_name = if cfg!(windows) { "java.exe" } else { "java" };

    WalkDir::new(runtime_root)
        .into_iter()
        .filter_map(Result::ok)
        .find_map(|entry| {
            if entry.file_type().is_file() && entry.file_name() == executable_name {
                return Some(entry.into_path());
            }

            None
        })
}

fn detect_java_major(java_executable: &Path) -> Option<u32> {
    let output = Command::new(java_executable)
        .arg("-version")
        .output()
        .ok()?;
    let combined_output = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let version_pattern =
        Regex::new(r#"version "(?:1\.)?(\d+)"#).expect("java version regex should be valid");
    let captures = version_pattern.captures(&combined_output)?;
    let major_version = captures.get(1)?.as_str().parse::<u32>().ok()?;

    Some(major_version)
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::detect_java_major;

    #[test]
    fn parses_java_8_version_output() {
        let output = r#"java version "1.8.0_291""#;
        let regex = Regex::new(r#"version "(?:1\.)?(\d+)"#).expect("regex should compile");
        let captures = regex.captures(output).expect("captures should exist");
        assert_eq!(captures.get(1).expect("major version").as_str(), "8");
    }

    #[test]
    fn detect_java_major_returns_none_for_missing_binary() {
        assert!(detect_java_major(std::path::Path::new("definitely-not-a-java-binary")).is_none());
    }
}
