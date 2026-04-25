use std::{path::Path, sync::Arc};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::{
    launcher::{
        self,
        install::{
            OptifineInstallOption, OptifineInstallResult, OptifineInstallStatusEvent,
            ReduxInstallOption, ReduxInstallResult, ReduxInstallStatusEvent,
            VanillaInstallStatusEvent, VanillaRelease,
        },
        process::spawn_launch,
        process::EventSink,
        LaunchRequest, LaunchResponse, LauncherLogEvent, LauncherStatusEvent,
        LauncherStatusState, MinecraftVersionSummary, SystemMemoryProfile,
    },
    state::LaunchTracker,
};

struct TauriEventSink {
    app_handle: AppHandle,
}

impl TauriEventSink {
    fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl EventSink for TauriEventSink {
    fn emit_status(&self, event: LauncherStatusEvent) {
        let _ = self.app_handle.emit("launcher:status", event);
    }

    fn emit_log(&self, event: LauncherLogEvent) {
        let _ = self.app_handle.emit("launcher:log", event);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaDependencyStatus {
    pub installed: bool,
    pub meets_requirement: bool,
    pub detected_major: Option<u32>,
    pub detected_raw: Option<String>,
    pub suggested_linux_commands: Option<Vec<String>>,
    pub suggested_windows_links: Option<Vec<DependencyLink>>,
    pub can_auto_install: bool,
    pub auto_install_hint: Option<String>,
    pub required_major: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphicsDependencyStatus {
    pub required: bool,
    pub installed: bool,
    pub usable: bool,
    pub detected_raw: Option<String>,
    pub suggested_linux_commands: Option<Vec<String>>,
    pub can_auto_install: bool,
    pub auto_install_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyInstallResult {
    pub ok: bool,
    /// True when winget reported the Temurin JRE package was already current (often with a non-zero exit code).
    #[serde(default)]
    pub already_present: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct JavaDependencyRequest {
    #[serde(default)]
    pub required_major: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptifineInstallRequest {
    pub minecraft_dir: String,
    pub option_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VanillaInstallRequest {
    pub minecraft_dir: String,
    pub version_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReduxInstallRequest {
    pub minecraft_dir: String,
    pub option_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VanillaInstallResult {
    pub version_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteInstalledVersionRequest {
    pub minecraft_dir: String,
    pub version_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteVersionMetadata {
    #[serde(default)]
    mecha: Option<DeleteVersionMechaMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteVersionMechaMetadata {
    #[serde(default)]
    game_directory: Option<String>,
    #[serde(default)]
    source_kind: Option<String>,
}

#[tauri::command]
pub fn detect_default_minecraft_dir() -> Option<String> {
    launcher::detect_default_minecraft_dir().map(|path| launcher::path_to_string(&path))
}

#[tauri::command]
pub fn get_system_memory_profile() -> SystemMemoryProfile {
    launcher::resolve_system_memory_profile()
}

fn read_os_release() -> Option<String> {
    std::fs::read_to_string("/etc/os-release").ok()
}

fn parse_os_release_value(contents: &str, key: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if k.trim() != key {
            continue;
        }
        let raw = v.trim();
        let unquoted = raw
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(raw);
        return Some(unquoted.to_string());
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinuxDistro {
    Fedora,
    Debian,
    Arch,
    Other,
}

fn detect_linux_distro() -> LinuxDistro {
    if !cfg!(target_os = "linux") {
        return LinuxDistro::Other;
    }

    let os_release = read_os_release().unwrap_or_default();
    let distro_id = parse_os_release_value(&os_release, "ID").unwrap_or_default();
    let distro_like = parse_os_release_value(&os_release, "ID_LIKE").unwrap_or_default();
    let id = distro_id.to_ascii_lowercase();
    let like = distro_like.to_ascii_lowercase();

    if id == "fedora" || like.contains("fedora") || like.contains("rhel") {
        LinuxDistro::Fedora
    } else if id == "debian" || like.contains("debian") || like.contains("ubuntu") {
        LinuxDistro::Debian
    } else if id == "arch" || like.contains("arch") {
        LinuxDistro::Arch
    } else {
        LinuxDistro::Other
    }
}

fn normalize_java_requirement(required_major: Option<u32>) -> Option<u32> {
    match required_major {
        Some(major) if major <= 8 => Some(8),
        Some(major) if major <= 17 => Some(17),
        Some(_) => Some(21),
        None => None,
    }
}

fn java_download_label(major: u32) -> String {
    format!("Java {major}")
}

fn temurin_release_url(major: u32) -> String {
    format!("https://adoptium.net/temurin/releases/?version={major}")
}

fn temurin_winget_id(major: u32) -> Option<&'static str> {
    match major {
        8 => Some("EclipseAdoptium.Temurin.8.JRE"),
        17 => Some("EclipseAdoptium.Temurin.17.JRE"),
        21 => Some("EclipseAdoptium.Temurin.21.JRE"),
        _ => None,
    }
}

fn linux_java_package_name(distro: LinuxDistro, major: u32) -> Option<&'static str> {
    match (distro, major) {
        (LinuxDistro::Fedora, 8) => None,
        (LinuxDistro::Fedora, 17) => Some("java-17-openjdk"),
        (LinuxDistro::Fedora, 21) => Some("java-21-openjdk"),
        (LinuxDistro::Debian, 8) => Some("openjdk-8-jre"),
        (LinuxDistro::Debian, 17) => Some("openjdk-17-jre"),
        (LinuxDistro::Debian, 21) => Some("openjdk-21-jre"),
        (LinuxDistro::Arch, 8) => Some("jre8-openjdk"),
        (LinuxDistro::Arch, 17) => Some("jre17-openjdk"),
        (LinuxDistro::Arch, 21) => Some("jre21-openjdk"),
        _ => None,
    }
}

fn requested_java_major_hint(required_major: Option<u32>) -> String {
    match required_major {
        Some(major) => format!("Install Java {major} or newer for the selected version."),
        None => "Select a Minecraft version first, then install the Java version it requires."
            .to_string(),
    }
}

fn suggested_linux_java_install_commands(required_major: Option<u32>) -> Option<Vec<String>> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    let distro = detect_linux_distro();
    let Some(install_major) = normalize_java_requirement(required_major) else {
        return Some(vec![requested_java_major_hint(required_major)]);
    };
    let Some(package_name) = linux_java_package_name(distro, install_major) else {
        let mut commands = vec![requested_java_major_hint(required_major)];
        if distro == LinuxDistro::Fedora && install_major == 8 {
            commands.push(
                "En Fedora recientes Java 8 ya no suele estar disponible como paquete dnf estándar; instala un JRE/JDK 8 manualmente (por ejemplo Eclipse Temurin 8).".to_string(),
            );
        }
        commands.push("java -version".to_string());
        return Some(commands);
    };

    let commands = match distro {
        LinuxDistro::Fedora => vec![
            format!("sudo dnf install -y {package_name}"),
            "java -version".to_string(),
        ],
        LinuxDistro::Debian => vec![
            "sudo apt update".to_string(),
            format!("sudo apt install -y {package_name}"),
            "java -version".to_string(),
        ],
        LinuxDistro::Arch => vec![
            format!("sudo pacman -Syu --noconfirm {package_name}"),
            "java -version".to_string(),
        ],
        LinuxDistro::Other => vec![
            requested_java_major_hint(required_major),
            "java -version".to_string(),
        ],
    };

    Some(commands)
}

fn suggested_windows_java_links(required_major: Option<u32>) -> Option<Vec<DependencyLink>> {
    if !cfg!(windows) {
        return None;
    }

    let mut links = Vec::new();

    if let Some(major) = normalize_java_requirement(required_major) {
        links.push(DependencyLink {
            label: format!("Eclipse Temurin (OpenJDK) {major}"),
            url: temurin_release_url(major),
        });
    } else {
        for major in [8, 17, 21] {
            links.push(DependencyLink {
                label: format!("Eclipse Temurin (OpenJDK) {major}"),
                url: temurin_release_url(major),
            });
        }
    }

    links.push(DependencyLink {
        label: "Microsoft Build of OpenJDK".to_string(),
        url: "https://learn.microsoft.com/java/openjdk/download".to_string(),
    });

    Some(links)
}

fn is_command_available(command: &str) -> bool {
    let locator = if cfg!(windows) { "where" } else { "which" };
    std::process::Command::new(locator)
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn linux_auto_install_plan(required_major: Option<u32>) -> Option<(String, Vec<String>, String)> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    let install_major = normalize_java_requirement(required_major)?;
    let distro = detect_linux_distro();
    let package_name = linux_java_package_name(distro, install_major)?;
    let requirement_label = java_download_label(install_major);

    if distro == LinuxDistro::Fedora && is_command_available("dnf") {
        return Some((
            "dnf".to_string(),
            vec![
                "install".to_string(),
                "-y".to_string(),
                package_name.to_string(),
            ],
            format!(
                "Will install {requirement_label} with dnf (administrator privileges required)."
            ),
        ));
    }

    if distro == LinuxDistro::Debian && is_command_available("apt") {
        return Some((
            "sh".to_string(),
            vec![
                "-lc".to_string(),
                format!("apt update && apt install -y {package_name}"),
            ],
            format!(
                "Will install {requirement_label} with apt (administrator privileges required)."
            ),
        ));
    }

    if distro == LinuxDistro::Arch && is_command_available("pacman") {
        return Some((
            "pacman".to_string(),
            vec![
                "-Syu".to_string(),
                "--noconfirm".to_string(),
                package_name.to_string(),
            ],
            format!(
                "Will install {requirement_label} with pacman (administrator privileges required)."
            ),
        ));
    }

    None
}

fn suggested_linux_graphics_install_commands() -> Option<Vec<String>> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    let commands = match detect_linux_distro() {
        LinuxDistro::Fedora => vec![
            "sudo dnf install -y xrandr".to_string(),
            "xrandr -q".to_string(),
        ],
        LinuxDistro::Debian => vec![
            "sudo apt update".to_string(),
            "sudo apt install -y x11-xserver-utils".to_string(),
            "xrandr -q".to_string(),
        ],
        LinuxDistro::Arch => vec![
            "sudo pacman -Syu --noconfirm xorg-xrandr".to_string(),
            "xrandr -q".to_string(),
        ],
        LinuxDistro::Other => vec![
            "xrandr -q".to_string(),
            "# Install xrandr with your distribution package manager.".to_string(),
        ],
    };

    Some(commands)
}

fn linux_graphics_auto_install_plan() -> Option<(String, Vec<String>, String)> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    match detect_linux_distro() {
        LinuxDistro::Fedora if is_command_available("dnf") => Some((
            "dnf".to_string(),
            vec!["install".to_string(), "-y".to_string(), "xrandr".to_string()],
            "Will install xrandr for legacy Minecraft on Linux (administrator privileges required)."
                .to_string(),
        )),
        LinuxDistro::Debian if is_command_available("apt") => Some((
            "sh".to_string(),
            vec![
                "-lc".to_string(),
                "apt update && apt install -y x11-xserver-utils".to_string(),
            ],
            "Will install x11-xserver-utils for legacy Minecraft on Linux (administrator privileges required)."
                .to_string(),
        )),
        LinuxDistro::Arch if is_command_available("pacman") => Some((
            "pacman".to_string(),
            vec![
                "-Syu".to_string(),
                "--noconfirm".to_string(),
                "xorg-xrandr".to_string(),
            ],
            "Will install xorg-xrandr for legacy Minecraft on Linux (administrator privileges required)."
                .to_string(),
        )),
        _ => None,
    }
}

/// winget may return failure even when the JRE is already installed and up to date
/// (<https://github.com/microsoft/winget-cli/issues/4262>).
fn winget_java_install_output_means_already_current(combined: &str) -> bool {
    let s = combined.to_ascii_lowercase();
    (s.contains("found an existing package") && s.contains("already installed"))
        || s.contains("no available upgrade found")
        || s.contains("no newer package versions are available")
        || s.contains("no applicable upgrade found")
        || s.contains("a newer version is already installed")
}

fn windows_auto_install_plan(required_major: Option<u32>) -> Option<(String, Vec<String>, String)> {
    if !cfg!(windows) {
        return None;
    }

    if !is_command_available("winget") {
        return None;
    }

    let install_major = normalize_java_requirement(required_major)?;
    let package_id = temurin_winget_id(install_major)?;

    Some((
        "winget".to_string(),
        vec![
            "install".to_string(),
            "--id".to_string(),
            package_id.to_string(),
            "-e".to_string(),
            "--accept-package-agreements".to_string(),
            "--accept-source-agreements".to_string(),
        ],
        format!("Will install Eclipse Temurin JRE {install_major} with winget (UAC may prompt)."),
    ))
}

#[tauri::command]
pub fn check_java_dependency(
    request: Option<JavaDependencyRequest>,
) -> Result<JavaDependencyStatus, String> {
    let required_major = request.and_then(|request| request.required_major);
    let java_executable = if cfg!(windows) { "java.exe" } else { "java" };
    let output = std::process::Command::new(java_executable)
        .arg("-version")
        .output();

    let (installed, detected_raw, detected_major) = match output {
        Ok(output) => {
            let combined = format!(
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let major = {
                let version_pattern = regex::Regex::new(r#"version "(?:1\.)?(\d+)"#)
                    .map_err(|error| error.to_string())?;
                version_pattern
                    .captures(&combined)
                    .and_then(|captures| captures.get(1))
                    .and_then(|value| value.as_str().parse::<u32>().ok())
            };
            (true, Some(combined.trim().to_string()), major)
        }
        Err(_) => (false, None, None),
    };

    let meets_requirement = match (required_major, detected_major) {
        (Some(required_major), Some(detected_major)) => {
            crate::launcher::java::java_major_satisfies_requirement(detected_major, required_major)
        }
        (None, Some(_)) => true,
        _ => false,
    };
    let needs_requested_java = !installed || required_major.is_some() && !meets_requirement;

    let (can_auto_install, auto_install_hint) = if !needs_requested_java {
        (false, None)
    } else if let Some((_, _, hint)) = linux_auto_install_plan(required_major) {
        (true, Some(hint))
    } else if let Some((_, _, hint)) = windows_auto_install_plan(required_major) {
        (true, Some(hint))
    } else {
        (false, None)
    };

    Ok(JavaDependencyStatus {
        installed,
        meets_requirement,
        detected_major,
        detected_raw,
        suggested_linux_commands: if needs_requested_java {
            suggested_linux_java_install_commands(required_major)
        } else {
            None
        },
        suggested_windows_links: if needs_requested_java {
            suggested_windows_java_links(required_major)
        } else {
            None
        },
        can_auto_install,
        auto_install_hint,
        required_major,
    })
}

#[tauri::command]
pub fn check_graphics_dependency() -> Result<GraphicsDependencyStatus, String> {
    let required = cfg!(target_os = "linux");

    if !required {
        return Ok(GraphicsDependencyStatus {
            required,
            installed: true,
            usable: true,
            detected_raw: None,
            suggested_linux_commands: None,
            can_auto_install: false,
            auto_install_hint: None,
        });
    }

    let installed = is_command_available("xrandr");
    let output = if installed {
        std::process::Command::new("xrandr").arg("-q").output().ok()
    } else {
        None
    };
    let detected_raw = output.as_ref().map(|output| {
        format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .trim()
        .to_string()
    });
    let usable = output
        .as_ref()
        .map(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains(" connected")
        })
        .unwrap_or(false);

    let needs_install = !installed;
    let (can_auto_install, auto_install_hint) = if needs_install {
        linux_graphics_auto_install_plan()
            .map(|(_, _, hint)| (true, Some(hint)))
            .unwrap_or((false, None))
    } else {
        (false, None)
    };

    Ok(GraphicsDependencyStatus {
        required,
        installed,
        usable,
        detected_raw,
        suggested_linux_commands: if needs_install {
            suggested_linux_graphics_install_commands()
        } else {
            None
        },
        can_auto_install,
        auto_install_hint,
    })
}

#[tauri::command]
pub async fn auto_install_java(
    request: Option<JavaDependencyRequest>,
) -> Result<DependencyInstallResult, String> {
    let required_major = request.and_then(|request| request.required_major);

    if cfg!(target_os = "linux") {
        let Some((program, args, _)) = linux_auto_install_plan(required_major) else {
            return Err(
                "No supported auto-install plan is available for the requested Java version."
                    .to_string(),
            );
        };

        let elevated_program = if is_command_available("pkexec") {
            "pkexec".to_string()
        } else {
            return Err(
                "pkexec is not available. Install it (polkit) or run the suggested commands manually."
                    .to_string(),
            );
        };

        // pkexec expects program + args
        let mut elevated_args = vec![program];
        elevated_args.extend(args);

        let result = tauri::async_runtime::spawn_blocking(move || {
            std::process::Command::new(elevated_program)
                .args(elevated_args)
                .output()
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())??;

        return Ok(DependencyInstallResult {
            ok: result.status.success(),
            already_present: false,
            exit_code: result.status.code(),
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
        });
    }

    if cfg!(windows) {
        let Some((program, args, _)) = windows_auto_install_plan(required_major) else {
            return Err(
                "winget was not found or no auto-install plan is available for the requested Java version."
                    .to_string(),
            );
        };

        let result = tauri::async_runtime::spawn_blocking(move || {
            std::process::Command::new(program)
                .args(args)
                .output()
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())??;

        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();
        let combined = format!("{stdout}\n{stderr}");
        let already_present =
            !result.status.success() && winget_java_install_output_means_already_current(&combined);
        let ok = result.status.success() || already_present;

        return Ok(DependencyInstallResult {
            ok,
            already_present,
            exit_code: result.status.code(),
            stdout: if already_present {
                String::new()
            } else {
                stdout
            },
            stderr: if already_present {
                String::new()
            } else {
                stderr
            },
        });
    }

    Err("Auto-install is not supported on this platform.".to_string())
}

#[tauri::command]
pub async fn auto_install_graphics_dependency() -> Result<DependencyInstallResult, String> {
    if cfg!(target_os = "linux") {
        let Some((program, args, _)) = linux_graphics_auto_install_plan() else {
            return Err(
                "No supported graphics dependency auto-install plan was found.".to_string(),
            );
        };

        let elevated_program = if is_command_available("pkexec") {
            "pkexec".to_string()
        } else {
            return Err(
                "pkexec is not available. Install xrandr manually or install polkit.".to_string(),
            );
        };

        let mut elevated_args = vec![program];
        elevated_args.extend(args);

        let result = tauri::async_runtime::spawn_blocking(move || {
            std::process::Command::new(elevated_program)
                .args(elevated_args)
                .output()
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())??;

        return Ok(DependencyInstallResult {
            ok: result.status.success(),
            already_present: false,
            exit_code: result.status.code(),
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
        });
    }

    Err("Graphics dependency auto-install is not supported on this platform.".to_string())
}

#[tauri::command]
pub async fn browse_minecraft_dir(app: AppHandle) -> Result<Option<String>, String> {
    let (sender, receiver) =
        tokio::sync::oneshot::channel::<Result<Option<std::path::PathBuf>, String>>();

    app.dialog()
        .file()
        .set_title("Select .minecraft directory")
        .pick_folder(move |file_path| {
            let picked = file_path
                .map(|path| path.into_path().map_err(|error| error.to_string()))
                .transpose();
            let _ = sender.send(picked);
        });

    let picked_path = receiver
        .await
        .map_err(|_| "The folder picker was interrupted.".to_string())?;

    let picked_path = picked_path?;

    Ok(picked_path.map(|path| launcher::path_to_string(&path)))
}

#[tauri::command]
pub fn list_versions(minecraft_dir: String) -> Result<Vec<MinecraftVersionSummary>, String> {
    launcher::list_versions(Path::new(&minecraft_dir)).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_optifine_install_options() -> Result<Vec<OptifineInstallOption>, String> {
    launcher::install::list_optifine_install_options()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_vanilla_releases() -> Result<Vec<VanillaRelease>, String> {
    launcher::install::list_vanilla_releases()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_redux_install_options() -> Result<Vec<ReduxInstallOption>, String> {
    launcher::install::list_redux_install_options().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn install_optifine_version(
    app: AppHandle,
    request: OptifineInstallRequest,
) -> Result<OptifineInstallResult, String> {
    let app_handle = app.clone();
    let progress = Arc::new(move |event: OptifineInstallStatusEvent| {
        let _ = app_handle.emit("optifine-install:status", event);
    });

    launcher::install::install_optifine_version(
        Path::new(&request.minecraft_dir),
        &request.option_id,
        progress,
    )
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn install_vanilla_version(
    app: AppHandle,
    request: VanillaInstallRequest,
) -> Result<VanillaInstallResult, String> {
    let app_handle = app.clone();
    let version_id = request.version_id.clone();
    let progress = Arc::new(move |event: VanillaInstallStatusEvent| {
        let _ = app_handle.emit("vanilla-install:status", event);
    });

    launcher::install::install_vanilla_version(
        Path::new(&request.minecraft_dir),
        &request.version_id,
        progress,
    )
    .await
    .map_err(|error| error.to_string())?;

    Ok(VanillaInstallResult { version_id })
}

#[tauri::command]
pub async fn install_redux_version(
    app: AppHandle,
    request: ReduxInstallRequest,
) -> Result<ReduxInstallResult, String> {
    let app_handle = app.clone();
    let progress = Arc::new(move |event: ReduxInstallStatusEvent| {
        let _ = app_handle.emit("redux-install:status", event);
    });

    let result = launcher::install::install_redux_version(
        Path::new(&request.minecraft_dir),
        &request.option_id,
        progress,
    )
    .await
    .map_err(|error| error.to_string())?;

    Ok(result)
}

#[tauri::command]
pub fn delete_installed_version(request: DeleteInstalledVersionRequest) -> Result<(), String> {
    let minecraft_dir = Path::new(request.minecraft_dir.trim());
    let versions_dir = minecraft_dir.join("versions");
    let target_dir = versions_dir.join(request.version_id.trim());
    let manifest_path = target_dir.join(format!("{}.json", request.version_id.trim()));

    let versions_dir = versions_dir
        .canonicalize()
        .map_err(|error| format!("Failed to resolve versions directory: {error}"))?;

    let target_dir = if target_dir.exists() {
        target_dir
            .canonicalize()
            .map_err(|error| format!("Failed to resolve target directory: {error}"))?
    } else {
        target_dir
    };

    if !target_dir.starts_with(&versions_dir) {
        return Err("Refusing to delete version outside versions directory.".to_string());
    }

    if !target_dir.exists() {
        return Ok(());
    }

    if !target_dir.is_dir() {
        return Err("Target version path is not a directory.".to_string());
    }

    let redux_game_directory = std::fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|contents| serde_json::from_str::<DeleteVersionMetadata>(&contents).ok())
        .and_then(|metadata| metadata.mecha)
        .and_then(|mecha| {
            if mecha.source_kind.as_deref() == Some("redux") {
                mecha.game_directory
            } else {
                None
            }
        })
        .map(std::path::PathBuf::from);

    std::fs::remove_dir_all(&target_dir).map_err(|error| error.to_string())?;

    if let Some(redux_game_directory) = redux_game_directory {
        let instances_root = minecraft_dir.join("mecha-instances");
        if let (Ok(instances_root), Ok(redux_game_directory)) =
            (instances_root.canonicalize(), redux_game_directory.canonicalize())
        {
            if redux_game_directory.starts_with(&instances_root) && redux_game_directory.exists() {
                std::fs::remove_dir_all(redux_game_directory).map_err(|error| error.to_string())?;
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn ensure_versions_dir(minecraft_dir: String) -> Result<String, String> {
    let versions_dir = Path::new(&minecraft_dir).join("versions");

    std::fs::create_dir_all(&versions_dir).map_err(|error| error.to_string())?;

    Ok(launcher::path_to_string(&versions_dir))
}

#[tauri::command]
pub async fn launch_version(
    app: AppHandle,
    tracker: State<'_, LaunchTracker>,
    request: LaunchRequest,
) -> Result<LaunchResponse, String> {
    let launch_id = Uuid::new_v4().to_string();

    if !tracker.try_acquire(launch_id.clone()) {
        return Err("Another Minecraft instance is already being launched.".to_string());
    }

    let sink: Arc<dyn EventSink> = Arc::new(TauriEventSink::new(app));

    sink.emit_status(LauncherStatusEvent::new(
        launch_id.clone(),
        LauncherStatusState::Launching,
        Some(format!("Preparing launch plan for {}.", request.version_id)),
    ));

    sink.emit_log(LauncherLogEvent::new(
        launch_id.clone(),
        crate::launcher::LauncherLogSource::System,
        format!("Checking required files for {}.", request.version_id),
    ));

    if let Err(error) =
        launcher::install::ensure_launch_requirements(Path::new(&request.minecraft_dir), &request.version_id)
            .await
    {
        tracker.clear_if_matches(&launch_id);
        sink.emit_status(LauncherStatusEvent::new(
            launch_id.clone(),
            LauncherStatusState::Error,
            Some(error.to_string()),
        ));
        return Err(error.to_string());
    }

    let plan = match launcher::prepare_launch(&request, launch_id.clone()) {
        Ok(plan) => plan,
        Err(error) => {
            tracker.clear_if_matches(&launch_id);
            sink.emit_status(LauncherStatusEvent::new(
                launch_id.clone(),
                LauncherStatusState::Error,
                Some(error.to_string()),
            ));
            return Err(error.to_string());
        }
    };

    spawn_launch(plan, sink, tracker.inner().clone());

    Ok(LaunchResponse { launch_id })
}
