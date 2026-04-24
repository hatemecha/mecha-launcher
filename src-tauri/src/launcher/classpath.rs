use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::launcher::{
    manifest::ResolvedManifest, path_to_string, rules::RuntimeEnvironment, LauncherError,
    LauncherResult,
};

pub fn build_classpath_entries(
    minecraft_dir: &Path,
    manifest: &ResolvedManifest,
    version_jar: PathBuf,
) -> LauncherResult<Vec<PathBuf>> {
    if !version_jar.is_file() {
        return Err(LauncherError::new(format!(
            "The selected version jar does not exist: {}",
            version_jar.display()
        )));
    }

    let environment = RuntimeEnvironment::current();
    let libraries_dir = minecraft_dir.join("libraries");
    let mut classpath_entries = Vec::new();
    let mut seen_entries = HashSet::new();

    for library in &manifest.libraries {
        if !library.is_allowed(&environment)? {
            continue;
        }

        let Some(artifact_path) = library.artifact_path(&libraries_dir)? else {
            continue;
        };

        if !artifact_path.is_file() {
            return Err(LauncherError::new(format!(
                "Missing library jar: {}",
                artifact_path.display()
            )));
        }

        let key = path_to_string(&artifact_path);
        if seen_entries.insert(key) {
            classpath_entries.push(artifact_path);
        }
    }

    classpath_entries.push(version_jar);

    Ok(classpath_entries)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs};

    use tempfile::tempdir;

    use super::build_classpath_entries;
    use crate::launcher::manifest::{
        ArgumentsBlock, JavaVersion, Library, LibraryDownload, LibraryDownloads, ResolvedManifest,
    };

    #[test]
    fn build_classpath_includes_libraries_and_version_jar() {
        let temp_dir = tempdir().expect("tempdir should be created");
        let minecraft_dir = temp_dir.path();
        let library_path = minecraft_dir
            .join("libraries")
            .join("com")
            .join("example")
            .join("demo")
            .join("1.0.0");
        let version_jar = minecraft_dir.join("versions").join("demo").join("demo.jar");

        fs::create_dir_all(&library_path).expect("library directory should exist");
        fs::create_dir_all(
            version_jar
                .parent()
                .expect("version jar parent should exist"),
        )
        .expect("version directory should exist");
        fs::write(library_path.join("demo-1.0.0.jar"), b"").expect("library jar should exist");
        fs::write(&version_jar, b"").expect("version jar should exist");

        let manifest = ResolvedManifest {
            id: "demo".to_string(),
            main_class: "net.minecraft.Main".to_string(),
            asset_index_id: Some("demo".to_string()),
            asset_index_url: None,
            asset_index_sha1: None,
            asset_index_size: None,
            java_version: Some(JavaVersion {
                component: "jre-legacy".to_string(),
                major_version: 8,
            }),
            version_type: "release".to_string(),
            game_directory: None,
            source_kind: None,
            libraries: vec![Library {
                name: Some("com.example:demo:1.0.0".to_string()),
                artifact: None,
                classifiers: Default::default(),
                classifies: Default::default(),
                downloads: Some(LibraryDownloads {
                    artifact: Some(LibraryDownload {
                        path: Some("com/example/demo/1.0.0/demo-1.0.0.jar".to_string()),
                    }),
                    classifiers: Default::default(),
                }),
                rules: Vec::new(),
                natives: Default::default(),
                extract: None,
            }],
            modern_arguments: Some(ArgumentsBlock::default()),
            legacy_minecraft_arguments: None,
        };

        let classpath = build_classpath_entries(minecraft_dir, &manifest, version_jar)
            .expect("classpath should build");

        assert_eq!(classpath.len(), 2);
    }

    #[test]
    fn build_classpath_supports_legacy_top_level_artifact_entries() {
        let temp_dir = tempdir().expect("tempdir should be created");
        let minecraft_dir = temp_dir.path();
        let library_path = minecraft_dir
            .join("libraries")
            .join("net")
            .join("minecraft")
            .join("launchwrapper")
            .join("1.7");
        let version_jar = minecraft_dir.join("versions").join("demo").join("demo.jar");

        fs::create_dir_all(&library_path).expect("library directory should exist");
        fs::create_dir_all(
            version_jar
                .parent()
                .expect("version jar parent should exist"),
        )
        .expect("version directory should exist");
        fs::write(library_path.join("launchwrapper-1.7.jar"), b"")
            .expect("library jar should exist");
        fs::write(&version_jar, b"").expect("version jar should exist");

        let manifest = ResolvedManifest {
            id: "demo".to_string(),
            main_class: "net.minecraft.Main".to_string(),
            asset_index_id: Some("demo".to_string()),
            asset_index_url: None,
            asset_index_sha1: None,
            asset_index_size: None,
            java_version: Some(JavaVersion {
                component: "jre-legacy".to_string(),
                major_version: 8,
            }),
            version_type: "release".to_string(),
            game_directory: None,
            source_kind: None,
            libraries: vec![Library {
                name: Some("net.minecraft:launchwrapper:1.7".to_string()),
                artifact: Some(LibraryDownload {
                    path: Some("net/minecraft/launchwrapper/1.7/launchwrapper-1.7.jar".to_string()),
                }),
                classifiers: Default::default(),
                classifies: Default::default(),
                downloads: None,
                rules: Vec::new(),
                natives: Default::default(),
                extract: None,
            }],
            modern_arguments: Some(ArgumentsBlock::default()),
            legacy_minecraft_arguments: None,
        };

        let classpath = build_classpath_entries(minecraft_dir, &manifest, version_jar)
            .expect("classpath should build");

        assert_eq!(classpath.len(), 2);
    }

    #[test]
    fn build_classpath_skips_native_only_libraries_without_base_artifacts() {
        let temp_dir = tempdir().expect("tempdir should be created");
        let minecraft_dir = temp_dir.path();
        let version_jar = minecraft_dir.join("versions").join("demo").join("demo.jar");

        fs::create_dir_all(
            version_jar
                .parent()
                .expect("version jar parent should exist"),
        )
        .expect("version directory should exist");
        fs::write(&version_jar, b"").expect("version jar should exist");

        let mut natives = HashMap::new();
        let os_name = match std::env::consts::OS {
            "macos" => "osx",
            value => value,
        };
        natives.insert(os_name.to_string(), "natives-current".to_string());

        let mut classifiers = HashMap::new();
        classifiers.insert(
            "natives-current".to_string(),
            LibraryDownload {
                path: Some(
                    "org/lwjgl/lwjgl/lwjgl-platform/2.9.4-nightly-20150209/lwjgl-platform-2.9.4-nightly-20150209-natives-current.jar"
                        .to_string(),
                ),
            },
        );

        let manifest = ResolvedManifest {
            id: "demo".to_string(),
            main_class: "net.minecraft.Main".to_string(),
            asset_index_id: Some("demo".to_string()),
            asset_index_url: None,
            asset_index_sha1: None,
            asset_index_size: None,
            java_version: Some(JavaVersion {
                component: "jre-legacy".to_string(),
                major_version: 8,
            }),
            version_type: "release".to_string(),
            game_directory: None,
            source_kind: None,
            libraries: vec![Library {
                name: Some("org.lwjgl.lwjgl:lwjgl-platform:2.9.4-nightly-20150209".to_string()),
                artifact: None,
                classifiers,
                classifies: Default::default(),
                downloads: None,
                rules: Vec::new(),
                natives,
                extract: None,
            }],
            modern_arguments: Some(ArgumentsBlock::default()),
            legacy_minecraft_arguments: None,
        };

        let classpath = build_classpath_entries(minecraft_dir, &manifest, version_jar.clone())
            .expect("classpath should build");

        assert_eq!(classpath.len(), 1);
        assert_eq!(classpath[0], version_jar);
    }

    #[test]
    fn build_classpath_skips_legacy_native_placeholder_artifact() {
        let temp_dir = tempdir().expect("tempdir should be created");
        let minecraft_dir = temp_dir.path();
        let version_jar = minecraft_dir.join("versions").join("demo").join("demo.jar");

        fs::create_dir_all(
            version_jar
                .parent()
                .expect("version jar parent should exist"),
        )
        .expect("version directory should exist");
        fs::write(&version_jar, b"").expect("version jar should exist");

        let mut natives = HashMap::new();
        let os_name = match std::env::consts::OS {
            "macos" => "osx",
            value => value,
        };
        natives.insert(os_name.to_string(), "natives-current".to_string());

        let manifest = ResolvedManifest {
            id: "demo".to_string(),
            main_class: "net.minecraft.Main".to_string(),
            asset_index_id: Some("demo".to_string()),
            asset_index_url: None,
            asset_index_sha1: None,
            asset_index_size: None,
            java_version: Some(JavaVersion {
                component: "jre-legacy".to_string(),
                major_version: 8,
            }),
            version_type: "release".to_string(),
            game_directory: None,
            source_kind: None,
            libraries: vec![Library {
                name: Some("org.lwjgl.lwjgl:lwjgl-platform:2.9.4-nightly-20150209".to_string()),
                artifact: Some(LibraryDownload {
                    path: Some(
                        "org/lwjgl/lwjgl/lwjgl-platform/2.9.4-nightly-20150209/lwjgl-platform-2.9.4-nightly-20150209.jar"
                            .to_string(),
                    ),
                }),
                classifiers: Default::default(),
                classifies: Default::default(),
                downloads: None,
                rules: Vec::new(),
                natives,
                extract: None,
            }],
            modern_arguments: Some(ArgumentsBlock::default()),
            legacy_minecraft_arguments: None,
        };

        let classpath = build_classpath_entries(minecraft_dir, &manifest, version_jar.clone())
            .expect("classpath should build");

        assert_eq!(classpath, vec![version_jar]);
    }
}
