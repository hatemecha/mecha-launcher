use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use tempfile::TempDir;
use zip::ZipArchive;

use crate::launcher::{
    manifest::{LibraryExtract, ResolvedManifest},
    rules::RuntimeEnvironment,
    LauncherError, LauncherResult,
};

#[derive(Debug)]
pub struct PreparedNativesDirectory {
    pub path: PathBuf,
    pub temp_dir: Option<TempDir>,
}

pub fn prepare_natives_directory(
    minecraft_dir: &Path,
    version_dir: &Path,
    manifest: &ResolvedManifest,
) -> LauncherResult<PreparedNativesDirectory> {
    let existing_natives_dir = version_dir.join("natives");
    if existing_natives_dir.is_dir() {
        return Ok(PreparedNativesDirectory {
            path: existing_natives_dir,
            temp_dir: None,
        });
    }

    let environment = RuntimeEnvironment::current();
    let libraries_dir = minecraft_dir.join("libraries");
    let temp_dir = tempfile::tempdir().map_err(|error| {
        LauncherError::new(format!(
            "Failed to create a temporary natives directory: {error}"
        ))
    })?;

    for library in &manifest.libraries {
        if !library.is_allowed(&environment)? {
            continue;
        }

        let Some(classifier) = library.native_classifier(&environment) else {
            continue;
        };
        let Some(archive_path) = library.classifier_path(&libraries_dir, &classifier)? else {
            return Err(LauncherError::new(format!(
                "The manifest references native classifier {classifier} but no archive could be resolved."
            )));
        };

        if !archive_path.is_file() {
            return Err(LauncherError::new(format!(
                "Missing native archive: {}",
                archive_path.display()
            )));
        }

        extract_native_archive(&archive_path, temp_dir.path(), library.extract.as_ref())?;
    }

    Ok(PreparedNativesDirectory {
        path: temp_dir.path().to_path_buf(),
        temp_dir: Some(temp_dir),
    })
}

fn extract_native_archive(
    archive_path: &Path,
    destination_dir: &Path,
    extract: Option<&LibraryExtract>,
) -> LauncherResult<()> {
    let archive_file = File::open(archive_path).map_err(|error| {
        LauncherError::new(format!(
            "Failed to open native archive {}: {error}",
            archive_path.display()
        ))
    })?;
    let mut archive = ZipArchive::new(archive_file)
        .map_err(|error| LauncherError::new(format!("Failed to read native archive: {error}")))?;

    let excluded_prefixes = extract
        .map(|extract| extract.exclude.clone())
        .unwrap_or_default();

    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|error| LauncherError::new(format!("Failed to read zip entry: {error}")))?;

        if file.is_dir() {
            continue;
        }

        let Some(relative_path) = file.enclosed_name().map(|path| path.to_path_buf()) else {
            continue;
        };
        let normalized_path = relative_path.to_string_lossy().replace('\\', "/");

        if normalized_path.starts_with("META-INF/")
            || excluded_prefixes
                .iter()
                .any(|prefix| normalized_path.starts_with(prefix))
        {
            continue;
        }

        let output_path = destination_dir.join(&relative_path);
        if let Some(parent_dir) = output_path.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        let mut output_file = File::create(&output_path)?;
        io::copy(&mut file, &mut output_file).map_err(|error| {
            LauncherError::new(format!(
                "Failed to extract native file {}: {error}",
                output_path.display()
            ))
        })?;
    }

    Ok(())
}
