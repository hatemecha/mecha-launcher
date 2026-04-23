pub mod classpath;
pub mod discovery;
mod hash;
pub mod install;
pub mod java;
pub mod manifest;
pub mod natives;
pub mod process;
pub mod rules;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use thiserror::Error;
use uuid::Uuid;

use crate::launcher::{
    classpath::build_classpath_entries,
    discovery::{
        detect_default_minecraft_dir as detect_default_minecraft_dir_impl,
        resolve_asset_index_path, resolve_version_artifact_paths, validate_minecraft_directory,
    },
    java::resolve_java_executable,
    manifest::{collect_launch_arguments, load_merged_manifest, ResolvedManifest},
    natives::prepare_natives_directory,
};

pub const DEFAULT_USERNAME: &str = "Player";
pub const DEFAULT_ACCESS_TOKEN: &str = "offline-access-token";
pub const DEFAULT_USER_PROPERTIES: &str = "{}";
pub const DEFAULT_USER_TYPE: &str = "legacy";
pub const DEFAULT_LAUNCHER_NAME: &str = "mecha-launcher";
pub const DEFAULT_CLIENT_ID: &str = "offline-client-id";
pub const DEFAULT_AUTH_XUID: &str = "0";
pub const DEFAULT_RESOLUTION_WIDTH: &str = "854";
pub const DEFAULT_RESOLUTION_HEIGHT: &str = "480";

#[derive(Debug, Error, Clone)]
#[error("{0}")]
pub struct LauncherError(pub String);

impl LauncherError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl From<std::io::Error> for LauncherError {
    fn from(value: std::io::Error) -> Self {
        Self::new(value.to_string())
    }
}

impl From<serde_json::Error> for LauncherError {
    fn from(value: serde_json::Error) -> Self {
        Self::new(value.to_string())
    }
}

pub type LauncherResult<T> = Result<T, LauncherError>;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    pub minecraft_dir: String,
    pub version_id: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResponse {
    pub launch_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftVersionSummary {
    pub id: String,
    pub folder_name: String,
    pub jar_path: String,
    pub manifest_path: String,
    pub java_component: Option<String>,
    pub java_major_version: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherStatusEvent {
    pub launch_id: String,
    pub state: LauncherStatusState,
    pub message: Option<String>,
}

impl LauncherStatusEvent {
    pub fn new(launch_id: String, state: LauncherStatusState, message: Option<String>) -> Self {
        Self {
            launch_id,
            state,
            message,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LauncherStatusState {
    Launching,
    Running,
    Exited,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherLogEvent {
    pub launch_id: String,
    pub source: LauncherLogSource,
    pub line: String,
}

impl LauncherLogEvent {
    pub fn new(launch_id: String, source: LauncherLogSource, line: impl Into<String>) -> Self {
        Self {
            launch_id,
            source,
            line: line.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LauncherLogSource {
    Stdout,
    Stderr,
    System,
}

#[derive(Debug)]
pub struct LaunchPlan {
    pub launch_id: String,
    pub minecraft_dir: PathBuf,
    pub version_id: String,
    pub java_executable: PathBuf,
    pub main_class: String,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    pub classpath_entries: Vec<PathBuf>,
    pub cleanup_temp_dir: Option<TempDir>,
}

impl LaunchPlan {
    pub fn command_arguments(&self) -> Vec<String> {
        let mut arguments = self.jvm_args.clone();
        arguments.push(self.main_class.clone());
        arguments.extend(self.game_args.clone());
        arguments
    }
}

fn is_command_available(command: &str) -> bool {
    let locator = if cfg!(windows) { "where" } else { "which" };
    Command::new(locator)
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn manifest_needs_legacy_lwjgl_linux_graphics(manifest: &ResolvedManifest) -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }

    manifest.libraries.iter().any(|library| {
        library
            .name
            .as_deref()
            .map(|name| {
                name.starts_with("org.lwjgl.lwjgl:lwjgl:2.")
            })
            .unwrap_or(false)
    })
}

fn validate_legacy_lwjgl_linux_graphics(manifest: &ResolvedManifest) -> LauncherResult<()> {
    if !manifest_needs_legacy_lwjgl_linux_graphics(manifest) {
        return Ok(());
    }

    if !is_command_available("xrandr") {
        return Err(LauncherError::new(
            "Falta compatibilidad grafica Linux para Minecraft antiguo: LWJGL 2 necesita xrandr. Instalalo desde el panel de Dependencias y volve a jugar.",
        ));
    }

    let output = Command::new("xrandr").arg("-q").output().map_err(|error| {
        LauncherError::new(format!(
            "No se pudo ejecutar xrandr para validar la pantalla: {error}"
        ))
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() || !stdout.contains(" connected") {
        return Err(LauncherError::new(
            "xrandr esta instalado pero no devolvio pantallas conectadas. Minecraft 1.8.9/LWJGL 2 suele crashear asi; inicia sesion con X11/XWayland disponible o revisa tu configuracion de pantalla.",
        ));
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct LaunchVariables {
    values: HashMap<String, String>,
}

impl LaunchVariables {
    pub fn new(
        manifest: &ResolvedManifest,
        username: &str,
        minecraft_dir: &Path,
        natives_dir: &Path,
        classpath_entries: &[PathBuf],
    ) -> LauncherResult<Self> {
        let asset_index_name = manifest
            .asset_index_id
            .clone()
            .ok_or_else(|| LauncherError::new("The selected version is missing an asset index."))?;

        let classpath_separator = if cfg!(windows) { ";" } else { ":" };
        let classpath = classpath_entries
            .iter()
            .map(|path| path_to_string(path))
            .collect::<Vec<_>>()
            .join(classpath_separator);

        let mut values = HashMap::new();
        values.insert("auth_player_name".to_string(), username.to_string());
        values.insert("version_name".to_string(), manifest.id.clone());
        values.insert("game_directory".to_string(), path_to_string(minecraft_dir));
        values.insert(
            "assets_root".to_string(),
            path_to_string(&minecraft_dir.join("assets")),
        );
        values.insert("assets_index_name".to_string(), asset_index_name);
        values.insert(
            "auth_uuid".to_string(),
            offline_uuid(username).hyphenated().to_string(),
        );
        values.insert(
            "auth_access_token".to_string(),
            DEFAULT_ACCESS_TOKEN.to_string(),
        );
        values.insert("clientid".to_string(), DEFAULT_CLIENT_ID.to_string());
        values.insert("auth_xuid".to_string(), DEFAULT_AUTH_XUID.to_string());
        values.insert(
            "user_properties".to_string(),
            DEFAULT_USER_PROPERTIES.to_string(),
        );
        values.insert("user_type".to_string(), DEFAULT_USER_TYPE.to_string());
        values.insert("version_type".to_string(), manifest.version_type.clone());
        values.insert(
            "launcher_name".to_string(),
            DEFAULT_LAUNCHER_NAME.to_string(),
        );
        values.insert(
            "launcher_version".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        );
        values.insert("classpath".to_string(), classpath);
        values.insert(
            "classpath_separator".to_string(),
            classpath_separator.to_string(),
        );
        values.insert(
            "library_directory".to_string(),
            path_to_string(&minecraft_dir.join("libraries")),
        );
        values.insert("natives_directory".to_string(), path_to_string(natives_dir));
        values.insert(
            "resolution_width".to_string(),
            DEFAULT_RESOLUTION_WIDTH.to_string(),
        );
        values.insert(
            "resolution_height".to_string(),
            DEFAULT_RESOLUTION_HEIGHT.to_string(),
        );
        values.insert("quickPlayPath".to_string(), String::new());
        values.insert("quickPlaySingleplayer".to_string(), String::new());
        values.insert("quickPlayMultiplayer".to_string(), String::new());
        values.insert("quickPlayRealms".to_string(), String::new());

        Ok(Self { values })
    }

    pub fn replace_placeholders(&self, raw_value: &str) -> LauncherResult<String> {
        rules::replace_placeholders(raw_value, |placeholder| {
            self.values.get(placeholder).cloned().ok_or_else(|| {
                LauncherError::new(format!("Unsupported placeholder: ${{{placeholder}}}"))
            })
        })
    }
}

pub fn detect_default_minecraft_dir() -> Option<PathBuf> {
    detect_default_minecraft_dir_impl()
}

pub fn list_versions(minecraft_dir: &Path) -> LauncherResult<Vec<MinecraftVersionSummary>> {
    discovery::list_versions(minecraft_dir)
}

pub fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn prepare_launch(request: &LaunchRequest, launch_id: String) -> LauncherResult<LaunchPlan> {
    let username = if request.username.trim().is_empty() {
        DEFAULT_USERNAME
    } else {
        request.username.trim()
    };

    let minecraft_dir = PathBuf::from(request.minecraft_dir.trim());
    validate_minecraft_directory(&minecraft_dir)?;

    let artifact_paths = resolve_version_artifact_paths(&minecraft_dir, &request.version_id)?;
    let manifest = load_merged_manifest(&minecraft_dir, &request.version_id)?;
    validate_legacy_lwjgl_linux_graphics(&manifest)?;

    let asset_index_path = resolve_asset_index_path(&minecraft_dir, &manifest)?;
    if !asset_index_path.exists() {
        return Err(LauncherError::new(format!(
            "Missing asset index file at {}.",
            path_to_string(&asset_index_path)
        )));
    }

    let classpath_entries = build_classpath_entries(
        &minecraft_dir,
        &manifest,
        artifact_paths.version_jar.clone(),
    )?;

    let prepared_natives =
        prepare_natives_directory(&minecraft_dir, &artifact_paths.version_dir, &manifest)?;

    let launch_variables = LaunchVariables::new(
        &manifest,
        username,
        &minecraft_dir,
        &prepared_natives.path,
        &classpath_entries,
    )?;

    let java_executable = resolve_java_executable(&minecraft_dir, manifest.java_version.as_ref())?;

    let launch_arguments = collect_launch_arguments(&manifest, &launch_variables)?;

    let mut jvm_args = vec!["-Xms512M".to_string(), "-Xmx2G".to_string()];
    jvm_args.extend(launch_arguments.jvm_arguments);

    Ok(LaunchPlan {
        launch_id,
        minecraft_dir,
        version_id: request.version_id.clone(),
        java_executable,
        main_class: manifest.main_class,
        jvm_args,
        game_args: launch_arguments.game_arguments,
        classpath_entries,
        cleanup_temp_dir: prepared_natives.temp_dir,
    })
}

pub fn offline_uuid(username: &str) -> Uuid {
    let mut digest = hash::md5(format!("OfflinePlayer:{username}").as_bytes());
    digest[6] = (digest[6] & 0x0f) | 0x30;
    digest[8] = (digest[8] & 0x3f) | 0x80;
    Uuid::from_bytes(digest)
}
