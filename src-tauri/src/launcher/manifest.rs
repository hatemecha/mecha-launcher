use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize};

use crate::launcher::{
    discovery::resolve_version_artifact_paths,
    rules::{rules_allow, Rule, RuntimeEnvironment},
    LaunchVariables, LauncherError, LauncherResult,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    #[serde(deserialize_with = "deserialize_u32_from_number_or_string")]
    pub major_version: u32,
}

#[derive(Debug, Clone)]
pub struct ResolvedManifest {
    pub id: String,
    pub main_class: String,
    pub asset_index_id: Option<String>,
    pub java_version: Option<JavaVersion>,
    pub version_type: String,
    pub libraries: Vec<Library>,
    pub modern_arguments: Option<ArgumentsBlock>,
    pub legacy_minecraft_arguments: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LaunchArguments {
    pub jvm_arguments: Vec<String>,
    pub game_arguments: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub artifact: Option<LibraryDownload>,
    #[serde(default)]
    pub classifiers: HashMap<String, LibraryDownload>,
    #[serde(default)]
    pub classifies: HashMap<String, LibraryDownload>,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub natives: HashMap<String, String>,
    #[serde(default)]
    pub extract: Option<LibraryExtract>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<LibraryDownload>,
    #[serde(default)]
    pub classifiers: HashMap<String, LibraryDownload>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LibraryDownload {
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LibraryExtract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArgumentsBlock {
    #[serde(default)]
    pub game: Vec<ArgumentEntry>,
    #[serde(default)]
    pub jvm: Vec<ArgumentEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArgumentEntry {
    String(String),
    Conditional(ConditionalArgument),
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConditionalArgument {
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(alias = "values")]
    pub value: ArgumentValue,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    String(String),
    Strings(Vec<String>),
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawManifest {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    inherits_from: Option<String>,
    #[serde(default)]
    main_class: Option<String>,
    #[serde(default)]
    asset_index: Option<AssetIndex>,
    #[serde(default)]
    java_version: Option<JavaVersion>,
    #[serde(default)]
    libraries: Vec<Library>,
    #[serde(default)]
    arguments: Option<ArgumentsBlock>,
    #[serde(default)]
    minecraft_arguments: Option<String>,
    #[serde(default, rename = "type")]
    version_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AssetIndex {
    #[serde(default)]
    id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum U32LikeValue {
    Integer(u32),
    Float(f64),
    String(String),
}

pub fn load_merged_manifest(
    minecraft_dir: &Path,
    version_id: &str,
) -> LauncherResult<ResolvedManifest> {
    let mut visited = HashSet::new();
    let merged_manifest = load_manifest_chain(minecraft_dir, version_id, &mut visited)?;

    let id = merged_manifest.id.unwrap_or_else(|| version_id.to_string());
    let main_class = merged_manifest.main_class.ok_or_else(|| {
        LauncherError::new(format!(
            "The selected version manifest does not define a main class: {version_id}"
        ))
    })?;

    Ok(ResolvedManifest {
        id,
        main_class,
        asset_index_id: merged_manifest
            .asset_index
            .and_then(|asset_index| asset_index.id),
        java_version: merged_manifest.java_version,
        version_type: merged_manifest
            .version_type
            .unwrap_or_else(|| "release".to_string()),
        libraries: merged_manifest.libraries,
        modern_arguments: merged_manifest.arguments,
        legacy_minecraft_arguments: merged_manifest.minecraft_arguments,
    })
}

pub fn collect_launch_arguments(
    manifest: &ResolvedManifest,
    variables: &LaunchVariables,
) -> LauncherResult<LaunchArguments> {
    let environment = RuntimeEnvironment::current();

    if let Some(arguments) = &manifest.modern_arguments {
        let jvm_arguments = collect_argument_entries(&arguments.jvm, &environment, variables)?;
        let game_arguments = collect_argument_entries(&arguments.game, &environment, variables)?;

        return Ok(LaunchArguments {
            jvm_arguments,
            game_arguments,
        });
    }

    if let Some(legacy_arguments) = &manifest.legacy_minecraft_arguments {
        let legacy_jvm_arguments = [
            "-Djava.library.path=${natives_directory}",
            "-Dminecraft.launcher.brand=${launcher_name}",
            "-Dminecraft.launcher.version=${launcher_version}",
            "-cp",
            "${classpath}",
        ];

        let jvm_arguments = legacy_jvm_arguments
            .into_iter()
            .map(|argument| variables.replace_placeholders(argument))
            .collect::<LauncherResult<Vec<_>>>()?;

        let game_arguments = split_legacy_arguments(legacy_arguments)
            .iter()
            .map(|argument| variables.replace_placeholders(argument))
            .collect::<LauncherResult<Vec<_>>>()?;

        return Ok(LaunchArguments {
            jvm_arguments,
            game_arguments,
        });
    }

    Err(LauncherError::new(
        "The selected version manifest does not expose launch arguments.",
    ))
}

impl Library {
    pub fn is_allowed(&self, environment: &RuntimeEnvironment) -> LauncherResult<bool> {
        rules_allow(&self.rules, environment)
    }

    pub fn artifact_path(&self, libraries_dir: &Path) -> LauncherResult<Option<PathBuf>> {
        if self.is_legacy_native_placeholder_artifact() {
            return Ok(None);
        }

        if let Some(artifact) = self.primary_artifact() {
            return Ok(Some(
                libraries_dir.join(
                    artifact
                        .path
                        .as_ref()
                        .ok_or_else(|| {
                            LauncherError::new("A library artifact entry is missing its path.")
                        })?
                        .replace('\\', "/"),
                ),
            ));
        }

        if !self.natives.is_empty() {
            return Ok(None);
        }

        if let Some(name) = &self.name {
            let coordinates = MavenCoordinates::parse(name)?;
            return Ok(Some(coordinates.artifact_path(libraries_dir, None)));
        }

        Ok(None)
    }

    pub fn native_classifier(&self, environment: &RuntimeEnvironment) -> Option<String> {
        let native_key = self.natives.get(&environment.os_name).or_else(|| {
            if environment.os_name == "osx" {
                self.natives.get("macos")
            } else {
                None
            }
        })?;

        Some(native_key.replace("${arch}", &environment.arch_bits))
    }

    pub fn classifier_path(
        &self,
        libraries_dir: &Path,
        classifier: &str,
    ) -> LauncherResult<Option<PathBuf>> {
        if let Some(download) = self.classifier_download(classifier) {
            return Ok(Some(
                libraries_dir.join(
                    download
                        .path
                        .as_ref()
                        .ok_or_else(|| {
                            LauncherError::new("A native classifier entry is missing its path.")
                        })?
                        .replace('\\', "/"),
                ),
            ));
        }

        if let Some(name) = &self.name {
            let coordinates = MavenCoordinates::parse(name)?;
            return Ok(Some(
                coordinates.artifact_path(libraries_dir, Some(classifier)),
            ));
        }

        Ok(None)
    }

    pub fn identity_key(&self) -> String {
        if let Some(name) = &self.name {
            return name.clone();
        }

        if let Some(artifact) = self.primary_artifact() {
            if let Some(path) = &artifact.path {
                return path.clone();
            }
        }

        format!("library-{:p}", self)
    }

    fn primary_artifact(&self) -> Option<&LibraryDownload> {
        self.artifact
            .as_ref()
            .or_else(|| self.downloads.as_ref()?.artifact.as_ref())
    }

    fn classifier_download(&self, classifier: &str) -> Option<&LibraryDownload> {
        let mut classifier_keys = vec![classifier.to_string()];
        if let Some(os_key) = classifier.strip_prefix("natives-") {
            classifier_keys.push(os_key.to_string());
            if os_key == "osx" {
                classifier_keys.push("macos".to_string());
            }
        }

        for key in classifier_keys {
            if let Some(download) = self.classifiers.get(&key) {
                return Some(download);
            }

            if let Some(download) = self.classifies.get(&key) {
                return Some(download);
            }

            if let Some(download) = self
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.classifiers.get(&key))
            {
                return Some(download);
            }
        }

        None
    }

    fn is_legacy_native_placeholder_artifact(&self) -> bool {
        !self.natives.is_empty()
            && self.artifact.is_some()
            && self
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.artifact.as_ref())
                .is_none()
    }
}

fn load_manifest_chain(
    minecraft_dir: &Path,
    version_id: &str,
    visited: &mut HashSet<String>,
) -> LauncherResult<RawManifest> {
    if !visited.insert(version_id.to_string()) {
        return Err(LauncherError::new(format!(
            "Version inheritance cycle detected at {version_id}."
        )));
    }

    let manifest_path = resolve_version_artifact_paths(minecraft_dir, version_id)?.manifest_file;
    let manifest_contents = fs::read_to_string(&manifest_path).map_err(|error| {
        LauncherError::new(format!(
            "Failed to read version manifest {}: {error}",
            manifest_path.display()
        ))
    })?;

    let manifest = serde_json::from_str::<RawManifest>(&manifest_contents).map_err(|error| {
        LauncherError::new(format!(
            "Failed to parse version manifest {}: {error}",
            manifest_path.display()
        ))
    })?;

    if let Some(parent_version_id) = &manifest.inherits_from {
        let parent_manifest = load_manifest_chain(minecraft_dir, parent_version_id, visited)?;
        return Ok(parent_manifest.merge(manifest));
    }

    Ok(manifest)
}

pub(crate) fn deserialize_u32_from_number_or_string<'de, D>(
    deserializer: D,
) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let raw_value = U32LikeValue::deserialize(deserializer)?;
    parse_u32_like_value(raw_value).map_err(D::Error::custom)
}

pub(crate) fn deserialize_optional_u32_from_number_or_string<'de, D>(
    deserializer: D,
) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw_value = Option::<U32LikeValue>::deserialize(deserializer)?;
    raw_value.map(parse_u32_like_value).transpose().map_err(D::Error::custom)
}

fn parse_u32_like_value(raw_value: U32LikeValue) -> Result<u32, String> {
    match raw_value {
        U32LikeValue::Integer(value) => Ok(value),
        U32LikeValue::Float(value) => {
            if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
                return Err(format!("expected a whole non-negative number, got {value}"));
            }

            if value > u32::MAX as f64 {
                return Err(format!("number is out of range for u32: {value}"));
            }

            Ok(value as u32)
        }
        U32LikeValue::String(value) => value
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("expected a u32-compatible string, got {value}")),
    }
}

fn collect_argument_entries(
    arguments: &[ArgumentEntry],
    environment: &RuntimeEnvironment,
    variables: &LaunchVariables,
) -> LauncherResult<Vec<String>> {
    let mut resolved_arguments = Vec::new();

    for argument in arguments {
        match argument {
            ArgumentEntry::String(value) => {
                resolved_arguments.push(variables.replace_placeholders(value)?);
            }
            ArgumentEntry::Conditional(argument) => {
                if !rules_allow(&argument.rules, environment)? {
                    continue;
                }

                match &argument.value {
                    ArgumentValue::String(value) => {
                        resolved_arguments.push(variables.replace_placeholders(value)?);
                    }
                    ArgumentValue::Strings(values) => {
                        for value in values {
                            resolved_arguments.push(variables.replace_placeholders(value)?);
                        }
                    }
                }
            }
        }
    }

    Ok(resolved_arguments)
}

pub fn split_legacy_arguments(arguments: &str) -> Vec<String> {
    let mut parsed_arguments = Vec::new();
    let mut buffer = String::new();
    let mut in_quotes = false;
    let mut escaped = false;

    for character in arguments.chars() {
        if escaped {
            buffer.push(character);
            escaped = false;
            continue;
        }

        match character {
            '\\' if in_quotes => escaped = true,
            '"' => in_quotes = !in_quotes,
            ' ' | '\t' if !in_quotes => {
                if !buffer.is_empty() {
                    parsed_arguments.push(buffer.clone());
                    buffer.clear();
                }
            }
            _ => buffer.push(character),
        }
    }

    if !buffer.is_empty() {
        parsed_arguments.push(buffer);
    }

    parsed_arguments
}

impl RawManifest {
    fn merge(self, child: RawManifest) -> RawManifest {
        let combined_libraries = self
            .libraries
            .into_iter()
            .chain(child.libraries)
            .collect::<Vec<_>>();

        let mut seen = HashSet::new();
        let mut deduplicated_libraries = Vec::new();
        for library in combined_libraries.into_iter().rev() {
            let key = library.identity_key();
            if seen.insert(key) {
                deduplicated_libraries.push(library);
            }
        }
        deduplicated_libraries.reverse();

        let arguments = match (self.arguments, child.arguments) {
            (Some(mut parent_arguments), Some(child_arguments)) => {
                parent_arguments.jvm.extend(child_arguments.jvm);
                parent_arguments.game.extend(child_arguments.game);
                Some(parent_arguments)
            }
            (None, Some(child_arguments)) => Some(child_arguments),
            (Some(parent_arguments), None) => Some(parent_arguments),
            (None, None) => None,
        };

        RawManifest {
            id: child.id.or(self.id),
            inherits_from: child.inherits_from,
            main_class: child.main_class.or(self.main_class),
            asset_index: child.asset_index.or(self.asset_index),
            java_version: child.java_version.or(self.java_version),
            libraries: deduplicated_libraries,
            arguments,
            minecraft_arguments: child.minecraft_arguments.or(self.minecraft_arguments),
            version_type: child.version_type.or(self.version_type),
        }
    }
}

#[derive(Debug)]
struct MavenCoordinates {
    group: String,
    artifact: String,
    version: String,
    classifier: Option<String>,
    extension: String,
}

impl MavenCoordinates {
    fn parse(raw_value: &str) -> LauncherResult<Self> {
        let (coordinates, extension) = match raw_value.split_once('@') {
            Some((coordinates, extension)) => (coordinates, extension.to_string()),
            None => (raw_value, "jar".to_string()),
        };

        let segments: Vec<&str> = coordinates.split(':').collect();
        let (group, artifact, version, classifier) = match segments.as_slice() {
            [group, artifact, version] => (
                group.to_string(),
                artifact.to_string(),
                version.to_string(),
                None,
            ),
            [group, artifact, version, classifier] => (
                group.to_string(),
                artifact.to_string(),
                version.to_string(),
                Some(classifier.to_string()),
            ),
            _ => {
                return Err(LauncherError::new(format!(
                    "Invalid Maven coordinate: {raw_value}"
                )))
            }
        };

        Ok(Self {
            group,
            artifact,
            version,
            classifier,
            extension,
        })
    }

    fn artifact_path(&self, libraries_dir: &Path, classifier_override: Option<&str>) -> PathBuf {
        let classifier = classifier_override
            .map(|value| value.to_string())
            .or_else(|| self.classifier.clone());

        let mut file_name = format!("{}-{}", self.artifact, self.version);
        if let Some(classifier) = classifier {
            file_name.push('-');
            file_name.push_str(&classifier);
        }
        file_name.push('.');
        file_name.push_str(&self.extension);

        libraries_dir
            .join(self.group.replace('.', "/"))
            .join(&self.artifact)
            .join(&self.version)
            .join(file_name)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{
        collect_launch_arguments, load_merged_manifest, split_legacy_arguments, JavaVersion,
        Library, LibraryDownload, ResolvedManifest,
    };
    use crate::launcher::LaunchVariables;

    #[test]
    fn merge_manifest_inheritance_keeps_child_values() {
        let temp_dir = tempdir().expect("tempdir should be created");
        let minecraft_dir = temp_dir.path().join(".minecraft");
        let parent_dir = minecraft_dir.join("versions").join("base");
        let child_dir = minecraft_dir.join("versions").join("child");

        std::fs::create_dir_all(&parent_dir).expect("parent dir should exist");
        std::fs::create_dir_all(&child_dir).expect("child dir should exist");

        std::fs::write(
            parent_dir.join("base.json"),
            r#"{
              "id": "base",
              "mainClass": "com.example.Base",
              "assetIndex": { "id": "base-assets" },
              "arguments": {
                "game": ["--base"],
                "jvm": ["-cp", "${classpath}"]
              },
              "libraries": [
                {
                  "name": "com.example:base-lib:1.0.0",
                  "downloads": { "artifact": { "path": "com/example/base-lib/1.0.0/base-lib-1.0.0.jar" } }
                }
              ]
            }"#,
        )
        .expect("parent manifest should be written");
        std::fs::write(parent_dir.join("base.jar"), b"").expect("parent jar should be written");

        std::fs::write(
            child_dir.join("child.json"),
            r#"{
              "id": "child",
              "inheritsFrom": "base",
              "mainClass": "com.example.Child",
              "arguments": {
                "game": ["--child"],
                "jvm": ["-Dchild=true"]
              }
            }"#,
        )
        .expect("child manifest should be written");
        std::fs::write(child_dir.join("child.jar"), b"").expect("child jar should be written");

        let resolved =
            load_merged_manifest(&minecraft_dir, "child").expect("manifest should resolve");
        assert_eq!(resolved.id, "child");
        assert_eq!(resolved.main_class, "com.example.Child");
        let arguments = resolved.modern_arguments.expect("arguments should exist");
        assert_eq!(arguments.game.len(), 2);
        assert_eq!(arguments.jvm.len(), 3);
    }

    #[test]
    fn split_legacy_arguments_keeps_quoted_values() {
        let parsed = split_legacy_arguments(r#"--username "Test Player" --demo"#);
        assert_eq!(parsed, vec!["--username", "Test Player", "--demo"]);
    }

    #[test]
    fn collect_launch_arguments_supports_legacy_profiles() {
        let manifest = ResolvedManifest {
            id: "test".to_string(),
            main_class: "net.minecraft.Main".to_string(),
            asset_index_id: Some("1.8".to_string()),
            java_version: None,
            version_type: "release".to_string(),
            libraries: Vec::new(),
            modern_arguments: None,
            legacy_minecraft_arguments: Some(
                "--username ${auth_player_name} --version ${version_name}".to_string(),
            ),
        };

        let variables = LaunchVariables::new(
            &manifest,
            "Player",
            std::path::Path::new("C:/minecraft"),
            std::path::Path::new("C:/minecraft/natives"),
            &[],
        )
        .expect("launch variables should resolve");

        let arguments = collect_launch_arguments(&manifest, &variables)
            .expect("legacy arguments should resolve");

        assert!(arguments.jvm_arguments.iter().any(|value| value == "-cp"));
        assert_eq!(
            arguments.game_arguments,
            vec!["--username", "Player", "--version", "test"]
        );
    }

    #[test]
    fn java_version_accepts_float_major_version() {
        let parsed = serde_json::from_str::<JavaVersion>(
            r#"{"component":"java-runtime-delta","majorVersion":21.0}"#,
        )
        .expect("java version should parse");

        assert_eq!(parsed.major_version, 21);
    }

    #[test]
    fn classifier_path_supports_legacy_classifies_by_os_key() {
        let mut classifies = std::collections::HashMap::new();
        classifies.insert(
            "windows".to_string(),
            LibraryDownload {
                path: Some(
                    "org/lwjgl/lwjgl/lwjgl-platform/2.9.4/lwjgl-platform-2.9.4-natives-windows.jar"
                        .to_string(),
                ),
            },
        );

        let library = Library {
            name: Some("org.lwjgl.lwjgl:lwjgl-platform:2.9.4".to_string()),
            artifact: None,
            classifiers: Default::default(),
            classifies,
            downloads: None,
            rules: Vec::new(),
            natives: Default::default(),
            extract: None,
        };

        let path = library
            .classifier_path(std::path::Path::new("C:/minecraft/libraries"), "natives-windows")
            .expect("classifier should resolve")
            .expect("classifier path should exist");

        assert_eq!(
            path,
            std::path::Path::new("C:/minecraft/libraries")
                .join("org/lwjgl/lwjgl/lwjgl-platform/2.9.4/lwjgl-platform-2.9.4-natives-windows.jar")
        );
    }
}
