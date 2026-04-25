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
    fs,
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
pub const OFFLINE_API_BYPASS_JVM_ARGS: [&str; 4] = [
    "-Dminecraft.api.auth.host=https://nope.invalid",
    "-Dminecraft.api.account.host=https://nope.invalid",
    "-Dminecraft.api.session.host=https://nope.invalid",
    "-Dminecraft.api.services.host=https://nope.invalid",
];
pub const MIN_MINECRAFT_MEMORY_MB: u32 = 1024;
pub const MIN_MINECRAFT_INITIAL_HEAP_MB: u32 = 512;
pub const FALLBACK_SYSTEM_MEMORY_MB: u32 = 8192;
pub const MIN_SYSTEM_MEMORY_RESERVE_MB: u32 = 2048;

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
    pub required_java_major: Option<u32>,
    pub max_memory_mb: Option<u32>,
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
    pub game_directory: Option<String>,
    pub source_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMemoryProfile {
    pub detected: bool,
    pub total_memory_mb: u32,
    pub min_allocatable_mb: u32,
    pub max_allocatable_mb: u32,
    pub recommended_allocatable_mb: u32,
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
    pub game_directory: PathBuf,
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
            .map(|name| name.starts_with("org.lwjgl.lwjgl:lwjgl:2."))
            .unwrap_or(false)
    })
}

fn validate_legacy_lwjgl_linux_graphics(manifest: &ResolvedManifest) -> LauncherResult<()> {
    if !manifest_needs_legacy_lwjgl_linux_graphics(manifest) {
        return Ok(());
    }

    if !is_command_available("xrandr") {
        return Err(LauncherError::new(
            "Legacy Minecraft on Linux requires xrandr for LWJGL 2. Install it from the Dependencies panel and try again.",
        ));
    }

    let output = Command::new("xrandr").arg("-q").output().map_err(|error| {
        LauncherError::new(format!(
            "Failed to run xrandr while validating display availability: {error}"
        ))
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() || !stdout.contains(" connected") {
        return Err(LauncherError::new(
            "xrandr is installed but did not report any connected displays. Minecraft 1.8.9 with LWJGL 2 usually crashes in this state; start the session with X11/XWayland available or review the display configuration.",
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
        game_directory: &Path,
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
        values.insert("game_directory".to_string(), path_to_string(game_directory));
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

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn parse_first_u64(text: &str) -> Option<u64> {
    let digits = text
        .chars()
        .filter(|value| value.is_ascii_digit())
        .collect::<String>();
    digits.parse::<u64>().ok()
}

#[cfg(target_os = "linux")]
fn detect_total_system_memory_mb() -> Option<u32> {
    let contents = fs::read_to_string("/proc/meminfo").ok()?;
    let line = contents
        .lines()
        .find(|value| value.trim_start().starts_with("MemTotal:"))?;
    let kilobytes = line
        .split_whitespace()
        .nth(1)?
        .parse::<u64>()
        .ok()?;
    u32::try_from(kilobytes.div_ceil(1024)).ok()
}

#[cfg(target_os = "macos")]
fn detect_total_system_memory_mb() -> Option<u32> {
    let output = Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let bytes = String::from_utf8_lossy(&output.stdout);
    let total_bytes = parse_first_u64(&bytes)?;
    u32::try_from(total_bytes.div_ceil(1024 * 1024)).ok()
}

#[cfg(target_os = "windows")]
fn detect_total_system_memory_mb() -> Option<u32> {
    let powershell_output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
        ])
        .output()
        .ok();

    if let Some(output) = powershell_output {
        if output.status.success() {
            let total_bytes = parse_first_u64(&String::from_utf8_lossy(&output.stdout))?;
            return u32::try_from(total_bytes.div_ceil(1024 * 1024)).ok();
        }
    }

    let wmic_output = Command::new("wmic")
        .args(["computersystem", "get", "TotalPhysicalMemory", "/value"])
        .output()
        .ok()?;
    if !wmic_output.status.success() {
        return None;
    }

    let total_bytes = parse_first_u64(&String::from_utf8_lossy(&wmic_output.stdout))?;
    u32::try_from(total_bytes.div_ceil(1024 * 1024)).ok()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn detect_total_system_memory_mb() -> Option<u32> {
    None
}

pub fn derive_system_memory_profile(total_memory_mb: u32, detected: bool) -> SystemMemoryProfile {
    let total_memory_mb = total_memory_mb.max(MIN_MINECRAFT_MEMORY_MB);
    let reserve_mb = total_memory_mb
        .saturating_div(4)
        .max(MIN_SYSTEM_MEMORY_RESERVE_MB)
        .min(total_memory_mb.saturating_sub(MIN_MINECRAFT_MEMORY_MB));
    let min_allocatable_mb = MIN_MINECRAFT_MEMORY_MB;
    let max_allocatable_mb = total_memory_mb
        .saturating_sub(reserve_mb)
        .max(min_allocatable_mb)
        .min(total_memory_mb);
    let recommended_allocatable_mb = total_memory_mb
        .saturating_div(4)
        .clamp(min_allocatable_mb, max_allocatable_mb);

    SystemMemoryProfile {
        detected,
        total_memory_mb,
        min_allocatable_mb,
        max_allocatable_mb,
        recommended_allocatable_mb,
    }
}

pub fn resolve_system_memory_profile() -> SystemMemoryProfile {
    detect_total_system_memory_mb()
        .map(|total_memory_mb| derive_system_memory_profile(total_memory_mb, true))
        .unwrap_or_else(|| derive_system_memory_profile(FALLBACK_SYSTEM_MEMORY_MB, false))
}

pub fn select_launch_memory_mb(
    profile: &SystemMemoryProfile,
    requested_memory_mb: Option<u32>,
) -> u32 {
    requested_memory_mb
        .unwrap_or(profile.recommended_allocatable_mb)
        .clamp(profile.min_allocatable_mb, profile.max_allocatable_mb)
}

pub fn derive_initial_heap_mb(max_memory_mb: u32) -> u32 {
    max_memory_mb
        .saturating_div(2)
        .clamp(MIN_MINECRAFT_INITIAL_HEAP_MB, 1024)
        .min(max_memory_mb)
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
    let game_directory = manifest
        .game_directory
        .clone()
        .unwrap_or_else(|| minecraft_dir.clone());
    fs::create_dir_all(&game_directory)?;

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
        &game_directory,
        &prepared_natives.path,
        &classpath_entries,
    )?;

    let required_java_major = request.required_java_major.or_else(|| {
        manifest
            .java_version
            .as_ref()
            .map(|version| version.major_version)
    });
    let java_executable = resolve_java_executable(
        &minecraft_dir,
        manifest.java_version.as_ref(),
        required_java_major,
    )?;

    let launch_arguments = collect_launch_arguments(&manifest, &launch_variables)?;
    let memory_profile = resolve_system_memory_profile();
    let selected_memory_mb = select_launch_memory_mb(&memory_profile, request.max_memory_mb);
    let initial_heap_mb = derive_initial_heap_mb(selected_memory_mb);

    let mut jvm_args = vec![
        format!("-Xms{initial_heap_mb}M"),
        format!("-Xmx{selected_memory_mb}M"),
    ];
    // This launcher only creates offline sessions. On some versions such as 1.16.5,
    // Mojang's API endpoints can incorrectly disable multiplayer/LAN for offline profiles.
    jvm_args.extend(
        OFFLINE_API_BYPASS_JVM_ARGS
            .into_iter()
            .map(std::string::ToString::to_string),
    );
    jvm_args.extend(launch_arguments.jvm_arguments);

    Ok(LaunchPlan {
        launch_id,
        minecraft_dir,
        game_directory,
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

#[cfg(test)]
mod tests {
    use super::{
        derive_initial_heap_mb, derive_system_memory_profile, select_launch_memory_mb,
        MIN_MINECRAFT_MEMORY_MB,
    };

    #[test]
    fn system_memory_profile_keeps_bounds_for_small_systems() {
        let profile = derive_system_memory_profile(4096, true);

        assert_eq!(profile.min_allocatable_mb, MIN_MINECRAFT_MEMORY_MB);
        assert_eq!(profile.max_allocatable_mb, 2048);
        assert_eq!(profile.recommended_allocatable_mb, 1024);
    }

    #[test]
    fn manual_memory_is_clamped_to_reasonable_system_bounds() {
        let profile = derive_system_memory_profile(16384, true);

        assert_eq!(select_launch_memory_mb(&profile, Some(512)), 1024);
        assert_eq!(select_launch_memory_mb(&profile, Some(24000)), profile.max_allocatable_mb);
        assert_eq!(
            select_launch_memory_mb(&profile, None),
            profile.recommended_allocatable_mb
        );
    }

    #[test]
    fn initial_heap_scales_from_selected_memory_without_exploding() {
        assert_eq!(derive_initial_heap_mb(1024), 512);
        assert_eq!(derive_initial_heap_mb(2048), 1024);
        assert_eq!(derive_initial_heap_mb(8192), 1024);
    }
}
