use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::launcher::{
    manifest::{deserialize_optional_u32_from_number_or_string, ResolvedManifest},
    LauncherError, LauncherResult, MinecraftVersionSummary,
};

#[derive(Debug, Clone)]
pub struct VersionArtifactPaths {
    pub version_dir: PathBuf,
    pub manifest_file: PathBuf,
    pub version_jar: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionMetadata {
    #[serde(default)]
    java_version: Option<JavaVersionMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaVersionMetadata {
    #[serde(default)]
    component: Option<String>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_u32_from_number_or_string")]
    major_version: Option<u32>,
}

pub fn detect_default_minecraft_dir() -> Option<PathBuf> {
    let home_dir = std::env::var_os("HOME").map(PathBuf::from);
    let app_data_dir = std::env::var_os("APPDATA").map(PathBuf::from);

    default_minecraft_dir_for(
        std::env::consts::OS,
        home_dir.as_deref(),
        app_data_dir.as_deref(),
    )
}

pub fn default_minecraft_dir_for(
    os_name: &str,
    home_dir: Option<&Path>,
    app_data_dir: Option<&Path>,
) -> Option<PathBuf> {
    match os_name {
        "windows" => {
            if let Some(app_data_dir) = app_data_dir {
                return Some(app_data_dir.join(".minecraft"));
            }

            home_dir.map(|path| path.join("AppData").join("Roaming").join(".minecraft"))
        }
        "macos" => home_dir.map(|path| {
            path.join("Library")
                .join("Application Support")
                .join("minecraft")
        }),
        _ => home_dir.map(|path| path.join(".minecraft")),
    }
}

pub fn validate_minecraft_directory(minecraft_dir: &Path) -> LauncherResult<()> {
    if !minecraft_dir.exists() {
        return Err(LauncherError::new(format!(
            "Minecraft directory does not exist: {}",
            minecraft_dir.display()
        )));
    }

    if !minecraft_dir.is_dir() {
        return Err(LauncherError::new(format!(
            "Minecraft path is not a directory: {}",
            minecraft_dir.display()
        )));
    }

    for required_directory in ["versions", "libraries", "assets"] {
        let required_path = minecraft_dir.join(required_directory);

        if !required_path.is_dir() {
            return Err(LauncherError::new(format!(
                "Missing required Minecraft directory: {}",
                required_path.display()
            )));
        }
    }

    Ok(())
}

pub fn list_versions(minecraft_dir: &Path) -> LauncherResult<Vec<MinecraftVersionSummary>> {
    let versions_dir = minecraft_dir.join("versions");

    if !versions_dir.is_dir() {
        return Err(LauncherError::new(format!(
            "Missing versions directory: {}",
            versions_dir.display()
        )));
    }

    let mut versions = Vec::new();

    for entry in fs::read_dir(&versions_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let folder_name = entry.file_name().to_string_lossy().to_string();
        let artifacts = match resolve_version_artifact_paths(minecraft_dir, &folder_name) {
            Ok(artifacts) => artifacts,
            Err(_) => continue,
        };

        let metadata = fs::read_to_string(&artifacts.manifest_file)
            .ok()
            .and_then(|contents| serde_json::from_str::<VersionMetadata>(&contents).ok());

        versions.push(MinecraftVersionSummary {
            id: folder_name.clone(),
            folder_name,
            jar_path: artifacts.version_jar.to_string_lossy().to_string(),
            manifest_path: artifacts.manifest_file.to_string_lossy().to_string(),
            java_component: metadata
                .as_ref()
                .and_then(|manifest| manifest.java_version.as_ref())
                .and_then(|java_version| java_version.component.clone()),
            java_major_version: metadata
                .as_ref()
                .and_then(|manifest| manifest.java_version.as_ref())
                .and_then(|java_version| java_version.major_version),
        });
    }

    versions.sort_by_key(|version| version.folder_name.to_ascii_lowercase());

    Ok(versions)
}

pub fn resolve_version_artifact_paths(
    minecraft_dir: &Path,
    version_id: &str,
) -> LauncherResult<VersionArtifactPaths> {
    let version_dir = minecraft_dir.join("versions").join(version_id);
    let manifest_file = version_dir.join(format!("{version_id}.json"));
    let version_jar = version_dir.join(format!("{version_id}.jar"));

    if !version_dir.is_dir() {
        return Err(LauncherError::new(format!(
            "Missing version directory: {}",
            version_dir.display()
        )));
    }

    if !manifest_file.is_file() {
        return Err(LauncherError::new(format!(
            "Missing version manifest file: {}",
            manifest_file.display()
        )));
    }

    if !version_jar.is_file() {
        return Err(LauncherError::new(format!(
            "Missing version jar file: {}",
            version_jar.display()
        )));
    }

    Ok(VersionArtifactPaths {
        version_dir,
        manifest_file,
        version_jar,
    })
}

pub fn resolve_asset_index_path(
    minecraft_dir: &Path,
    manifest: &ResolvedManifest,
) -> LauncherResult<PathBuf> {
    let asset_index_id = manifest.asset_index_id.as_ref().ok_or_else(|| {
        LauncherError::new("The selected manifest does not declare an asset index.")
    })?;

    Ok(minecraft_dir
        .join("assets")
        .join("indexes")
        .join(format!("{asset_index_id}.json")))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{default_minecraft_dir_for, list_versions};

    #[test]
    fn windows_default_uses_appdata() {
        let result = default_minecraft_dir_for(
            "windows",
            Some(Path::new("C:/Users/example")),
            Some(Path::new("C:/Users/example/AppData/Roaming")),
        )
        .expect("windows default should resolve");

        assert_eq!(
            result,
            Path::new("C:/Users/example/AppData/Roaming").join(".minecraft")
        );
    }

    #[test]
    fn macos_default_uses_application_support() {
        let result = default_minecraft_dir_for("macos", Some(Path::new("/Users/example")), None)
            .expect("macos default should resolve");

        assert_eq!(
            result,
            Path::new("/Users/example")
                .join("Library")
                .join("Application Support")
                .join("minecraft")
        );
    }

    #[test]
    fn linux_default_uses_home_hidden_folder() {
        let result = default_minecraft_dir_for("linux", Some(Path::new("/home/example")), None)
            .expect("linux default should resolve");

        assert_eq!(result, Path::new("/home/example").join(".minecraft"));
    }

    #[test]
    fn list_versions_accepts_float_java_major_version() {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let minecraft_dir = temp_dir.path().join(".minecraft");
        let version_dir = minecraft_dir.join("versions").join("demo");

        fs::create_dir_all(&version_dir).expect("version dir should exist");
        fs::write(
            version_dir.join("demo.json"),
            r#"{
              "id": "demo",
              "javaVersion": {
                "component": "java-runtime-delta",
                "majorVersion": 21.0
              }
            }"#,
        )
        .expect("manifest should be written");
        fs::write(version_dir.join("demo.jar"), b"demo").expect("jar should be written");

        let versions = list_versions(&minecraft_dir).expect("versions should load");

        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].java_major_version, Some(21));
    }
}
