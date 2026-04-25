use std::{
    collections::HashMap,
    fs,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};

use regex::Regex;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use tempfile::tempdir;
use walkdir::WalkDir;

use crate::launcher::{
    hash::sha1_hex,
    manifest::{load_merged_manifest, JavaVersion, ResolvedManifest},
    rules::{rules_allow, Rule, RuntimeEnvironment},
    LauncherError, LauncherResult,
};

const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const JAVA_RUNTIME_MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";
const LIBRARIES_BASE_URL: &str = "https://libraries.minecraft.net/";
const ASSETS_BASE_URL: &str = "https://resources.download.minecraft.net/";
const OPTIFINE_ADLOAD_BASE_URL: &str = "https://optifine.net/adloadx";
const OPTIFINE_BASE_URL: &str = "https://optifine.net/";
const OPTIFINE_DOWNLOADS_URL: &str = "https://optifine.net/downloads";
const FABRIC_META_BASE_URL: &str = "https://meta.fabricmc.net/v2/versions/loader";
const OPTIFINE_INSTALLER_TIMEOUT: Duration = Duration::from_secs(300);
const OPTIFINE_INSTALLER_ERROR_TAIL_LINES: usize = 80;
const OPTIFINE_INSTALLER_ERROR_MAX_CHARS: usize = 12_000;
const OPTIFINE_OPTIONS_TTL: Duration = Duration::from_secs(10 * 60);
const INSTALLER_RUNNER_SOURCE: &str = r#"import java.io.File;
import java.lang.reflect.Method;

public class InstallOptiFine {
  public static void main(String[] args) throws Exception {
    Class<?> installer = Class.forName("optifine.Installer");
    Method doInstall = installer.getDeclaredMethod("doInstall", File.class);
    doInstall.setAccessible(true);
    doInstall.invoke(null, new File(args[0]));
  }
}
"#;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptifineInstallOption {
    pub id: String,
    pub minecraft_version: String,
    pub optifine_version: String,
    pub edition: String,
    pub file_name: String,
    pub version_id: String,
    pub title: String,
    pub summary: String,
    pub release_kind: String,
    pub recommended_java_major: u32,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptifineInstallStatusEvent {
    pub option_id: String,
    pub stage: String,
    pub message: String,
    pub current: Option<u32>,
    pub total: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptifineInstallResult {
    pub version_id: String,
    pub minecraft_version: String,
    pub optifine_version: String,
}

type ProgressSink = Arc<dyn Fn(OptifineInstallStatusEvent) + Send + Sync>;

#[derive(Debug, Clone)]
struct OptifineOptionsCache {
    fetched_at: Instant,
    options: Vec<OptifineInstallOption>,
}

static OPTIFINE_OPTIONS_CACHE: OnceLock<Mutex<Option<OptifineOptionsCache>>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct VersionManifest {
    versions: Vec<VersionManifestEntry>,
}

#[derive(Debug, Deserialize)]
struct VersionManifestEntry {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    url: String,
    #[serde(rename = "releaseTime")]
    release_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VanillaRelease {
    pub id: String,
    pub release_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VanillaInstallStatusEvent {
    pub version_id: String,
    pub stage: String,
    pub message: String,
    pub current: Option<u32>,
    pub total: Option<u32>,
}

type VanillaProgressSink = Arc<dyn Fn(VanillaInstallStatusEvent) + Send + Sync>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReduxInstallOption {
    pub id: String,
    pub version_id: String,
    pub title: String,
    pub summary: String,
    pub minecraft_version: String,
    pub fabric_loader_version: String,
    pub recommended_java_major: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReduxInstallStatusEvent {
    pub option_id: String,
    pub version_id: String,
    pub stage: String,
    pub message: String,
    pub current: Option<u32>,
    pub total: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReduxInstallResult {
    pub version_id: String,
    pub minecraft_version: String,
    pub fabric_loader_version: String,
}

type ReduxProgressSink = Arc<dyn Fn(ReduxInstallStatusEvent) + Send + Sync>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReduxPackManifest {
    id: String,
    title: String,
    minecraft_version: String,
    fabric_loader_version: String,
    #[serde(default = "default_mods_subdir")]
    mods_subdir: String,
    recommended_java_major: u32,
}

#[derive(Debug, Clone)]
struct ReduxPack {
    pack_dir: PathBuf,
    manifest: ReduxPackManifest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameVersionManifest {
    id: String,
    asset_index: Option<AssetIndex>,
    downloads: GameDownloads,
    #[serde(default)]
    libraries: Vec<GameLibrary>,
    #[serde(default)]
    java_version: Option<JavaVersion>,
}

#[derive(Debug, Deserialize)]
struct GameDownloads {
    client: DownloadArtifact,
}

#[derive(Debug, Deserialize)]
struct AssetIndex {
    id: String,
    url: String,
    #[serde(default)]
    sha1: Option<String>,
    #[serde(default)]
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AssetObjects {
    objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
struct AssetObject {
    hash: String,
    #[serde(default)]
    size: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
struct GameLibrary {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    downloads: Option<LibraryDownloads>,
    #[serde(default)]
    natives: HashMap<String, String>,
    #[serde(default)]
    rules: Vec<Rule>,
}

#[derive(Debug, Deserialize, Default)]
struct LibraryDownloads {
    #[serde(default)]
    artifact: Option<DownloadArtifact>,
    #[serde(default)]
    classifiers: HashMap<String, DownloadArtifact>,
}

#[derive(Debug, Deserialize, Clone)]
struct DownloadArtifact {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    sha1: Option<String>,
    #[serde(default)]
    size: Option<u64>,
    url: String,
}

#[derive(Debug, Deserialize)]
struct JavaRuntimeIndex(HashMap<String, HashMap<String, Vec<JavaRuntimeEntry>>>);

#[derive(Debug, Deserialize)]
struct JavaRuntimeEntry {
    manifest: JavaRuntimeManifestRef,
}

#[derive(Debug, Deserialize)]
struct JavaRuntimeManifestRef {
    url: String,
    #[serde(default)]
    sha1: Option<String>,
    #[serde(default)]
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct JavaRuntimeManifest {
    files: HashMap<String, JavaRuntimeFile>,
}

#[derive(Debug, Deserialize)]
struct JavaRuntimeFile {
    #[serde(rename = "type")]
    file_type: String,
    #[serde(default)]
    downloads: Option<JavaRuntimeDownloads>,
    #[serde(default)]
    executable: bool,
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JavaRuntimeDownloads {
    raw: Option<DownloadArtifact>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadOutcome {
    Downloaded,
    Reused,
}

fn recommended_java_for_minecraft(version: &str) -> u32 {
    // Rough mapping (UI hint):
    // - <= 1.16.x: Java 8
    // - 1.17.x: Java 16
    // - 1.18.x - 1.20.x: Java 17
    // - >= 1.21.x: Java 21
    let parts = version
        .split('.')
        .filter_map(|part| part.parse::<u32>().ok())
        .collect::<Vec<_>>();
    let minor = parts.get(1).copied().unwrap_or(0);
    if minor >= 21 {
        return 21;
    }
    if minor >= 18 {
        return 17;
    }
    if minor >= 17 {
        return 16;
    }
    8
}

fn optifine_edition_rank(edition: &str) -> (String, u32, bool, u32, String) {
    let without_prefix = edition.strip_prefix("HD_U_").unwrap_or(edition);
    let (base, prerelease) = without_prefix
        .split_once("_pre")
        .map(|(base, prerelease)| (base, prerelease.parse::<u32>().ok().unwrap_or(0)))
        .unwrap_or((without_prefix, 0));

    let pattern = Regex::new(r#"^(?P<branch>[A-Za-z]+)(?P<number>\d+)?$"#)
        .expect("valid OptiFine edition regex");
    let captures = pattern.captures(base);
    let branch = captures
        .as_ref()
        .and_then(|captures| captures.name("branch"))
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| base.to_string());
    let number = captures
        .as_ref()
        .and_then(|captures| captures.name("number"))
        .and_then(|value| value.as_str().parse::<u32>().ok())
        .unwrap_or(0);

    (
        branch,
        number,
        prerelease == 0,
        prerelease,
        without_prefix.to_string(),
    )
}

fn parse_optifine_downloads_html(html: &str) -> Vec<OptifineInstallOption> {
    let pattern =
        Regex::new(r#"OptiFine_(?P<mc>\d+\.\d+(?:\.\d+)?)_(?P<edition>HD_U_[A-Za-z0-9_]+)\.jar"#)
            .expect("valid regex");

    let mut by_mc: HashMap<String, OptifineInstallOption> = HashMap::new();

    for caps in pattern.captures_iter(html) {
        let Some(mc) = caps.name("mc").map(|value| value.as_str().to_string()) else {
            continue;
        };
        let Some(edition) = caps.name("edition").map(|value| value.as_str().to_string()) else {
            continue;
        };
        let file_name = format!("OptiFine_{mc}_{edition}.jar");
        let candidate = OptifineInstallOption {
            id: file_name.clone(),
            minecraft_version: mc.clone(),
            optifine_version: format!("OptiFine_{mc}_{edition}"),
            edition: edition.clone(),
            file_name: file_name.clone(),
            version_id: format!("{mc}-OptiFine_{edition}"),
            title: format!("Minecraft {mc}"),
            summary: format!("Best available OptiFine build for Minecraft {mc}."),
            release_kind: if edition.contains("_pre") {
                "Preview".to_string()
            } else {
                "Stable".to_string()
            },
            recommended_java_major: recommended_java_for_minecraft(&mc),
            source_url: format!("{OPTIFINE_ADLOAD_BASE_URL}?f={file_name}"),
        };

        let should_replace = by_mc
            .get(&mc)
            .map(|current| {
                optifine_edition_rank(&candidate.edition) > optifine_edition_rank(&current.edition)
            })
            .unwrap_or(true);

        if should_replace {
            by_mc.insert(mc, candidate);
        }
    }

    let mut options = by_mc.into_values().collect::<Vec<_>>();
    options.sort_by(|a, b| b.minecraft_version.cmp(&a.minecraft_version));
    options
}

async fn fetch_optifine_install_options() -> LauncherResult<Vec<OptifineInstallOption>> {
    let client = Client::builder()
        .user_agent("mecha-launcher/0.1")
        .build()
        .map_err(|error| LauncherError::new(format!("Failed to create HTTP client: {error}")))?;

    let html = fetch_text(&client, OPTIFINE_DOWNLOADS_URL).await?;
    Ok(parse_optifine_downloads_html(&html))
}

pub async fn list_optifine_install_options() -> LauncherResult<Vec<OptifineInstallOption>> {
    let cache_lock = OPTIFINE_OPTIONS_CACHE.get_or_init(|| Mutex::new(None));

    if let Ok(cache_guard) = cache_lock.lock() {
        if let Some(cache) = cache_guard.as_ref() {
            if cache.fetched_at.elapsed() <= OPTIFINE_OPTIONS_TTL && !cache.options.is_empty() {
                return Ok(cache.options.clone());
            }
        }
    }

    let options = fetch_optifine_install_options().await?;

    if let Ok(mut cache_guard) = cache_lock.lock() {
        *cache_guard = Some(OptifineOptionsCache {
            fetched_at: Instant::now(),
            options: options.clone(),
        });
    }

    Ok(options)
}

pub async fn list_vanilla_releases() -> LauncherResult<Vec<VanillaRelease>> {
    let client = Client::builder()
        .user_agent("mecha-launcher/0.1")
        .build()
        .map_err(|error| LauncherError::new(format!("Failed to create HTTP client: {error}")))?;

    let manifest = fetch_json::<VersionManifest>(&client, VERSION_MANIFEST_URL).await?;
    let releases = manifest
        .versions
        .into_iter()
        .filter(|entry| entry.kind == "release")
        .map(|entry| VanillaRelease {
            id: entry.id,
            release_time: entry.release_time,
        })
        .collect::<Vec<_>>();

    // Mojang's manifest is already ordered (newest first).
    // Keep server order to avoid lexical version sorting issues (e.g. 1.10 vs 1.9).
    Ok(releases)
}

fn default_mods_subdir() -> String {
    ".".to_string()
}

fn redux_repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join(".."))
}

fn read_redux_pack_from_dir(pack_dir: &Path) -> LauncherResult<ReduxPack> {
    let manifest_path = pack_dir.join("pack.json");
    let manifest_text = fs::read_to_string(&manifest_path).map_err(|error| {
        LauncherError::new(format!(
            "Failed to read redux pack manifest {}: {error}",
            manifest_path.display()
        ))
    })?;
    let manifest = serde_json::from_str::<ReduxPackManifest>(&manifest_text).map_err(|error| {
        LauncherError::new(format!(
            "Invalid redux pack manifest {}: {error}",
            manifest_path.display()
        ))
    })?;

    Ok(ReduxPack {
        pack_dir: pack_dir.to_path_buf(),
        manifest,
    })
}

fn read_redux_packs_from(root: &Path) -> LauncherResult<Vec<ReduxPack>> {
    let mut packs = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.ends_with("-redux") {
            continue;
        }

        let manifest_path = path.join("pack.json");
        if !manifest_path.is_file() {
            continue;
        }

        packs.push(read_redux_pack_from_dir(&path)?);
    }

    packs.sort_by(|left, right| left.manifest.id.cmp(&right.manifest.id));
    Ok(packs)
}

fn list_redux_install_options_from(root: &Path) -> LauncherResult<Vec<ReduxInstallOption>> {
    read_redux_packs_from(root).map(|packs| {
        packs.into_iter()
            .map(|pack| ReduxInstallOption {
                id: pack.manifest.id.clone(),
                version_id: pack.manifest.id.clone(),
                title: pack.manifest.title.clone(),
                summary: format!(
                    "Fabric {} con mods curados para Minecraft {}.",
                    pack.manifest.fabric_loader_version, pack.manifest.minecraft_version
                ),
                minecraft_version: pack.manifest.minecraft_version.clone(),
                fabric_loader_version: pack.manifest.fabric_loader_version.clone(),
                recommended_java_major: pack.manifest.recommended_java_major,
            })
            .collect()
    })
}

pub fn list_redux_install_options() -> LauncherResult<Vec<ReduxInstallOption>> {
    list_redux_install_options_from(&redux_repo_root())
}

fn find_redux_pack(option_id: &str) -> LauncherResult<ReduxPack> {
    read_redux_packs_from(&redux_repo_root())?
        .into_iter()
        .find(|pack| pack.manifest.id == option_id)
        .ok_or_else(|| LauncherError::new(format!("Unknown redux install option: {option_id}")))
}

pub async fn ensure_launch_requirements(
    minecraft_dir: &Path,
    version_id: &str,
) -> LauncherResult<()> {
    ensure_minecraft_layout(minecraft_dir)?;

    let manifest = load_merged_manifest(minecraft_dir, version_id)?;
    let client = Client::builder()
        .user_agent("mecha-launcher/0.1")
        .build()
        .map_err(|error| LauncherError::new(format!("Failed to create HTTP client: {error}")))?;

    ensure_manifest_libraries(&client, minecraft_dir, &manifest).await?;
    ensure_manifest_assets(&client, minecraft_dir, version_id, &manifest).await?;

    if let Some(java_version) = manifest.java_version.as_ref() {
        let silent_progress: ProgressSink = Arc::new(|_event| {});
        install_java_runtime(&client, minecraft_dir, version_id, java_version, &silent_progress)
            .await?;
    }

    Ok(())
}

fn emit_vanilla(
    progress: &VanillaProgressSink,
    version_id: &str,
    stage: &str,
    message: &str,
    current: Option<u32>,
    total: Option<u32>,
) {
    progress(VanillaInstallStatusEvent {
        version_id: version_id.to_string(),
        stage: stage.to_string(),
        message: message.to_string(),
        current,
        total,
    });
}

fn emit_redux(
    progress: &ReduxProgressSink,
    option_id: &str,
    version_id: &str,
    stage: &str,
    message: &str,
    current: Option<u32>,
    total: Option<u32>,
) {
    progress(ReduxInstallStatusEvent {
        option_id: option_id.to_string(),
        version_id: version_id.to_string(),
        stage: stage.to_string(),
        message: message.to_string(),
        current,
        total,
    });
}

pub async fn install_vanilla_version(
    minecraft_dir: &Path,
    version_id: &str,
    progress: VanillaProgressSink,
) -> LauncherResult<()> {
    let version_id = version_id.to_string();
    let version_id_str = version_id.as_str();
    let client = Client::builder()
        .user_agent("mecha-launcher/0.1")
        .build()
        .map_err(|error| LauncherError::new(format!("Failed to create HTTP client: {error}")))?;

    emit_vanilla(
        &progress,
        version_id_str,
        "prepare",
        "Preparing Minecraft directories.",
        None,
        None,
    );
    ensure_minecraft_layout(minecraft_dir)?;

    let version_manifest = fetch_json::<VersionManifest>(&client, VERSION_MANIFEST_URL).await?;
    let version_entry = version_manifest
        .versions
        .iter()
        .find(|entry| entry.id == version_id_str)
        .ok_or_else(|| {
            LauncherError::new(format!(
                "Minecraft {version_id_str} was not found in Mojang's version manifest."
            ))
        })?;

    emit_vanilla(
        &progress,
        version_id_str,
        "minecraft",
        &format!("Downloading Minecraft {version_id_str} manifest."),
        None,
        None,
    );
    let base_manifest_text = fetch_text(&client, &version_entry.url).await?;
    let base_manifest =
        serde_json::from_str::<GameVersionManifest>(&base_manifest_text).map_err(|error| {
            LauncherError::new(format!("Invalid Minecraft version manifest: {error}"))
        })?;

    // Reuse the same progress stream by mapping OptiFine-style events.
    let progress_clone = progress.clone();
    let version_id_for_progress = version_id.clone();
    let mapped_progress: ProgressSink = Arc::new(move |event| {
        emit_vanilla(
            &progress_clone,
            &version_id_for_progress,
            &event.stage,
            &event.message,
            event.current,
            event.total,
        )
    });

    // This creates `versions/<manifest.id>` with json+jar.
    install_base_manifest_and_client(
        &client,
        minecraft_dir,
        version_id_str,
        &base_manifest,
        &base_manifest_text,
        &mapped_progress,
    )
    .await?;

    install_libraries(
        &client,
        minecraft_dir,
        version_id_str,
        &base_manifest,
        &mapped_progress,
    )
    .await?;
    install_assets(
        &client,
        minecraft_dir,
        version_id_str,
        &base_manifest,
        &mapped_progress,
    )
    .await?;

    if let Some(java_version) = base_manifest.java_version.as_ref() {
        install_java_runtime(
            &client,
            minecraft_dir,
            version_id_str,
            java_version,
            &mapped_progress,
        )
        .await?;
    }

    emit_vanilla(
        &progress,
        version_id_str,
        "done",
        &format!("Minecraft {version_id_str} is ready to play."),
        Some(1),
        Some(1),
    );

    Ok(())
}

pub async fn install_redux_version(
    minecraft_dir: &Path,
    option_id: &str,
    progress: ReduxProgressSink,
) -> LauncherResult<ReduxInstallResult> {
    let pack = find_redux_pack(option_id)?;
    let version_id = pack.manifest.id.clone();
    let client = Client::builder()
        .user_agent("mecha-launcher/0.1")
        .build()
        .map_err(|error| LauncherError::new(format!("Failed to create HTTP client: {error}")))?;

    emit_redux(
        &progress,
        option_id,
        &version_id,
        "prepare",
        "Preparing redux pack installation.",
        None,
        None,
    );
    ensure_minecraft_layout(minecraft_dir)?;

    let progress_clone = progress.clone();
    let option_id_owned = option_id.to_string();
    let version_id_owned = version_id.clone();
    let mapped_progress: VanillaProgressSink = Arc::new(move |event| {
        emit_redux(
            &progress_clone,
            &option_id_owned,
            &version_id_owned,
            &event.stage,
            &event.message,
            event.current,
            event.total,
        )
    });
    install_vanilla_version(minecraft_dir, &pack.manifest.minecraft_version, mapped_progress).await?;

    let fabric_profile_url = format!(
        "{FABRIC_META_BASE_URL}/{}/{}/profile/json",
        pack.manifest.minecraft_version, pack.manifest.fabric_loader_version
    );
    emit_redux(
        &progress,
        option_id,
        &version_id,
        "fabric",
        "Downloading Fabric profile.",
        None,
        None,
    );
    let fabric_profile_text = fetch_text(&client, &fabric_profile_url).await?;
    let game_directory = minecraft_dir
        .join("mecha-instances")
        .join(&pack.manifest.id)
        .canonicalize()
        .unwrap_or_else(|_| minecraft_dir.join("mecha-instances").join(&pack.manifest.id));
    let fabric_manifest_text = build_redux_manifest(
        &fabric_profile_text,
        &pack.manifest,
        &game_directory,
    )?;
    write_redux_version_files(
        minecraft_dir,
        &pack.manifest.id,
        &pack.manifest.minecraft_version,
        &fabric_manifest_text,
    )?;

    let mods_source_dir = safe_join(&pack.pack_dir, &pack.manifest.mods_subdir)?;
    let instance_dir = minecraft_dir.join("mecha-instances").join(&pack.manifest.id);
    install_redux_instance_files(
        &instance_dir,
        &mods_source_dir,
        &progress,
        option_id,
        &version_id,
    )?;

    emit_redux(
        &progress,
        option_id,
        &version_id,
        "done",
        &format!("{} is ready to play.", pack.manifest.id),
        Some(1),
        Some(1),
    );

    Ok(ReduxInstallResult {
        version_id: pack.manifest.id,
        minecraft_version: pack.manifest.minecraft_version,
        fabric_loader_version: pack.manifest.fabric_loader_version,
    })
}

pub async fn install_optifine_version(
    minecraft_dir: &Path,
    option_id: &str,
    progress: ProgressSink,
) -> LauncherResult<OptifineInstallResult> {
    let option = list_optifine_install_options()
        .await?
        .into_iter()
        .find(|option| option.id == option_id)
        .ok_or_else(|| {
            LauncherError::new(format!("Unknown OptiFine install option: {option_id}"))
        })?;
    let client = Client::builder()
        .user_agent("mecha-launcher/0.1")
        .build()
        .map_err(|error| LauncherError::new(format!("Failed to create HTTP client: {error}")))?;

    emit(
        &progress,
        &option.id,
        "prepare",
        "Preparing Minecraft directories.",
        None,
        None,
    );
    ensure_minecraft_layout(minecraft_dir)?;
    cleanup_optifine_version_dir(minecraft_dir, &option)?;

    let version_manifest = fetch_json::<VersionManifest>(&client, VERSION_MANIFEST_URL).await?;
    let version_entry = version_manifest
        .versions
        .iter()
        .find(|entry| entry.id == option.minecraft_version)
        .ok_or_else(|| {
            LauncherError::new(format!(
                "Minecraft {} was not found in Mojang's version manifest.",
                option.minecraft_version
            ))
        })?;

    emit(
        &progress,
        &option.id,
        "minecraft",
        &format!(
            "Downloading Minecraft {} manifest.",
            option.minecraft_version
        ),
        None,
        None,
    );
    let base_manifest_text = fetch_text(&client, &version_entry.url).await?;
    let base_manifest =
        serde_json::from_str::<GameVersionManifest>(&base_manifest_text).map_err(|error| {
            LauncherError::new(format!("Invalid Minecraft version manifest: {error}"))
        })?;
    install_base_manifest_and_client(
        &client,
        minecraft_dir,
        &option.id,
        &base_manifest,
        &base_manifest_text,
        &progress,
    )
    .await?;
    install_libraries(
        &client,
        minecraft_dir,
        &option.id,
        &base_manifest,
        &progress,
    )
    .await?;
    install_assets(
        &client,
        minecraft_dir,
        &option.id,
        &base_manifest,
        &progress,
    )
    .await?;

    if let Some(java_version) = base_manifest.java_version.as_ref() {
        install_java_runtime(&client, minecraft_dir, &option.id, java_version, &progress).await?;
    }

    let optifine_jar =
        download_optifine_installer(&client, minecraft_dir, &option, &progress).await?;
    ensure_launcher_profiles_json(minecraft_dir)?;
    if let Err(error) =
        run_optifine_installer(minecraft_dir, &option, &optifine_jar, &progress).await
    {
        let _ = cleanup_optifine_version_dir(minecraft_dir, &option);
        return Err(error);
    }
    emit(
        &progress,
        &option.id,
        "optifine",
        "OptiFine applied. Verifying files.",
        Some(1),
        Some(1),
    );
    verify_optifine_install(minecraft_dir, &option)?;

    emit(
        &progress,
        &option.id,
        "done",
        &format!("{} is ready to play.", option.version_id),
        Some(1),
        Some(1),
    );

    Ok(OptifineInstallResult {
        version_id: option.version_id.clone(),
        minecraft_version: option.minecraft_version.to_string(),
        optifine_version: option.optifine_version.clone(),
    })
}

fn emit(
    progress: &ProgressSink,
    option_id: &str,
    stage: &str,
    message: &str,
    current: Option<u32>,
    total: Option<u32>,
) {
    progress(OptifineInstallStatusEvent {
        option_id: option_id.to_string(),
        stage: stage.to_string(),
        message: message.to_string(),
        current,
        total,
    });
}

fn ensure_minecraft_layout(minecraft_dir: &Path) -> LauncherResult<()> {
    for directory in [
        "versions",
        "libraries",
        "assets/indexes",
        "assets/objects",
        "runtime",
    ] {
        fs::create_dir_all(minecraft_dir.join(directory))?;
    }

    Ok(())
}

async fn ensure_manifest_libraries(
    client: &Client,
    minecraft_dir: &Path,
    manifest: &ResolvedManifest,
) -> LauncherResult<()> {
    let environment = RuntimeEnvironment::current();
    let libraries_dir = minecraft_dir.join("libraries");

    for library in &manifest.libraries {
        if !library.is_allowed(&environment)? {
            continue;
        }

        if let Some((relative_path, url, sha1, size)) = library.artifact_download_source()? {
            let target = safe_join(&libraries_dir, &relative_path)?;
            download_file(client, &url, &target, sha1.as_deref(), size, false, None).await?;
        }

        let Some(classifier) = library.native_classifier(&environment) else {
            continue;
        };

        if let Some((relative_path, url, sha1, size)) =
            library.classifier_download_source(&classifier)?
        {
            let target = safe_join(&libraries_dir, &relative_path)?;
            download_file(client, &url, &target, sha1.as_deref(), size, false, None).await?;
        }
    }

    Ok(())
}

async fn ensure_manifest_assets(
    client: &Client,
    minecraft_dir: &Path,
    job_id: &str,
    manifest: &ResolvedManifest,
) -> LauncherResult<()> {
    let Some(asset_index_id) = manifest.asset_index_id.as_ref() else {
        return Ok(());
    };
    let Some(asset_index_url) = manifest.asset_index_url.as_ref() else {
        return Ok(());
    };

    let index_target = minecraft_dir
        .join("assets")
        .join("indexes")
        .join(format!("{asset_index_id}.json"));

    let index_text = if index_target.is_file() {
        fs::read_to_string(&index_target).map_err(|error| {
            LauncherError::new(format!(
                "Failed to read asset index {}: {error}",
                index_target.display()
            ))
        })?
    } else {
        let text = fetch_text(client, asset_index_url).await?;
        validate_bytes(
            text.as_bytes(),
            manifest.asset_index_sha1.as_deref(),
            manifest.asset_index_size,
            asset_index_url,
        )?;
        fs::write(&index_target, text.as_bytes())?;
        text
    };

    let assets = serde_json::from_str::<AssetObjects>(&index_text)
        .map_err(|error| LauncherError::new(format!("Invalid asset index: {error}")))?;

    for asset in assets.objects.values() {
        let prefix = asset
            .hash
            .get(0..2)
            .ok_or_else(|| LauncherError::new("Asset hash is too short."))?;
        let target = minecraft_dir
            .join("assets")
            .join("objects")
            .join(prefix)
            .join(&asset.hash);
        let url = format!("{ASSETS_BASE_URL}{prefix}/{}", asset.hash);

        download_file(
            client,
            &url,
            &target,
            Some(&asset.hash),
            asset.size,
            false,
            None,
        )
        .await
        .map_err(|error| {
            LauncherError::new(format!(
                "Failed to prepare asset data for {job_id}: {error}"
            ))
        })?;
    }

    Ok(())
}

fn build_redux_manifest(
    base_manifest_text: &str,
    manifest: &ReduxPackManifest,
    game_directory: &Path,
) -> LauncherResult<String> {
    let mut json = serde_json::from_str::<serde_json::Value>(base_manifest_text).map_err(|error| {
        LauncherError::new(format!("Invalid Fabric profile manifest: {error}"))
    })?;
    let Some(object) = json.as_object_mut() else {
        return Err(LauncherError::new(
            "Fabric profile manifest did not contain a JSON object.",
        ));
    };

    object.insert("id".to_string(), serde_json::Value::String(manifest.id.clone()));
    object.insert(
        "inheritsFrom".to_string(),
        serde_json::Value::String(manifest.minecraft_version.clone()),
    );
    object.insert(
        "type".to_string(),
        serde_json::Value::String("release".to_string()),
    );
    object.insert(
        "mecha".to_string(),
        serde_json::json!({
            "gameDirectory": game_directory.to_string_lossy().to_string(),
            "sourceKind": "redux"
        }),
    );

    serde_json::to_string_pretty(&json)
        .map_err(|error| LauncherError::new(format!("Failed to serialize redux manifest: {error}")))
}

fn write_redux_version_files(
    minecraft_dir: &Path,
    version_id: &str,
    minecraft_version: &str,
    manifest_text: &str,
) -> LauncherResult<()> {
    let version_dir = minecraft_dir.join("versions").join(version_id);
    if version_dir.exists() {
        fs::remove_dir_all(&version_dir)?;
    }
    fs::create_dir_all(&version_dir)?;
    fs::write(version_dir.join(format!("{version_id}.json")), manifest_text.as_bytes())?;

    let base_jar = minecraft_dir
        .join("versions")
        .join(minecraft_version)
        .join(format!("{minecraft_version}.jar"));
    let target_jar = version_dir.join(format!("{version_id}.jar"));
    fs::copy(&base_jar, &target_jar).map_err(|error| {
        LauncherError::new(format!(
            "Failed to copy base Minecraft jar from {} to {}: {error}",
            base_jar.display(),
            target_jar.display()
        ))
    })?;

    Ok(())
}

fn install_redux_instance_files(
    instance_dir: &Path,
    mods_source_dir: &Path,
    progress: &ReduxProgressSink,
    option_id: &str,
    version_id: &str,
) -> LauncherResult<()> {
    fs::create_dir_all(instance_dir)?;
    for directory in ["config", "resourcepacks", "shaderpacks"] {
        fs::create_dir_all(instance_dir.join(directory))?;
    }

    let mods_target_dir = instance_dir.join("mods");
    if mods_target_dir.exists() {
        fs::remove_dir_all(&mods_target_dir)?;
    }
    fs::create_dir_all(&mods_target_dir)?;

    let mod_files = fs::read_dir(mods_source_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .map(|extension| extension.to_string_lossy().eq_ignore_ascii_case("jar"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    let total = mod_files.len() as u32;

    emit_redux(
        progress,
        option_id,
        version_id,
        "mods",
        &format!("Copying {total} mods."),
        Some(0),
        Some(total),
    );

    for (index, path) in mod_files.iter().enumerate() {
        let file_name = path.file_name().ok_or_else(|| {
            LauncherError::new(format!("Invalid mod file path: {}", path.display()))
        })?;
        fs::copy(path, mods_target_dir.join(file_name))?;
        let current = (index + 1) as u32;
        emit_redux(
            progress,
            option_id,
            version_id,
            "mods",
            &format!("Mods copied: {current}/{total}."),
            Some(current),
            Some(total),
        );
    }

    Ok(())
}

async fn install_base_manifest_and_client(
    client: &Client,
    minecraft_dir: &Path,
    job_id: &str,
    manifest: &GameVersionManifest,
    manifest_text: &str,
    progress: &ProgressSink,
) -> LauncherResult<()> {
    let version_dir = minecraft_dir.join("versions").join(&manifest.id);
    fs::create_dir_all(&version_dir)?;
    fs::write(
        version_dir.join(format!("{}.json", manifest.id)),
        manifest_text.as_bytes(),
    )?;

    let client_target = version_dir.join(format!("{}.jar", manifest.id));
    emit(
        progress,
        job_id,
        "minecraft",
        &format!("Downloading base client {}.", manifest.id),
        Some(0),
        Some(1),
    );
    download_file(
        client,
        &manifest.downloads.client.url,
        &client_target,
        manifest.downloads.client.sha1.as_deref(),
        manifest.downloads.client.size,
        false,
        None,
    )
    .await?;
    emit(
        progress,
        job_id,
        "minecraft",
        &format!("Base client {} is ready.", manifest.id),
        Some(1),
        Some(1),
    );

    Ok(())
}

async fn install_libraries(
    client: &Client,
    minecraft_dir: &Path,
    job_id: &str,
    manifest: &GameVersionManifest,
    progress: &ProgressSink,
) -> LauncherResult<()> {
    let environment = RuntimeEnvironment::current();
    let libraries = collect_required_libraries(&environment, &manifest.libraries)?;
    let total = libraries.len() as u32;

    emit(
        progress,
        job_id,
        "libraries",
        &format!("Preparing {total} libraries."),
        Some(0),
        Some(total),
    );

    for (index, library) in libraries.iter().enumerate() {
        let library_path = library.path.as_deref().ok_or_else(|| {
            LauncherError::new(format!(
                "Library download from {} is missing its target path.",
                library.url
            ))
        })?;
        let target = safe_join(&minecraft_dir.join("libraries"), library_path)?;
        download_file(
            client,
            &library.url,
            &target,
            library.sha1.as_deref(),
            library.size,
            false,
            None,
        )
        .await?;

        let current = (index + 1) as u32;
        if should_emit_progress(current, total) {
            emit(
                progress,
                job_id,
                "libraries",
                &format!("Libraries ready: {current}/{total}."),
                Some(current),
                Some(total),
            );
        }
    }

    Ok(())
}

async fn install_assets(
    client: &Client,
    minecraft_dir: &Path,
    job_id: &str,
    manifest: &GameVersionManifest,
    progress: &ProgressSink,
) -> LauncherResult<()> {
    let Some(asset_index) = manifest.asset_index.as_ref() else {
        return Ok(());
    };

    emit(
        progress,
        job_id,
        "assets",
        &format!("Downloading asset index {}.", asset_index.id),
        None,
        None,
    );

    let index_text = fetch_text(client, &asset_index.url).await?;
    let index_target = minecraft_dir
        .join("assets")
        .join("indexes")
        .join(format!("{}.json", asset_index.id));
    validate_bytes(
        index_text.as_bytes(),
        asset_index.sha1.as_deref(),
        asset_index.size,
        &asset_index.url,
    )?;
    fs::write(index_target, index_text.as_bytes())?;

    let assets = serde_json::from_str::<AssetObjects>(&index_text)
        .map_err(|error| LauncherError::new(format!("Invalid asset index: {error}")))?;
    let total = assets.objects.len() as u32;

    emit(
        progress,
        job_id,
        "assets",
        &format!("Preparing {total} assets."),
        Some(0),
        Some(total),
    );

    for (index, asset) in assets.objects.values().enumerate() {
        let prefix = asset
            .hash
            .get(0..2)
            .ok_or_else(|| LauncherError::new("Asset hash is too short."))?;
        let target = minecraft_dir
            .join("assets")
            .join("objects")
            .join(prefix)
            .join(&asset.hash);
        let url = format!("{ASSETS_BASE_URL}{prefix}/{}", asset.hash);

        download_file(
            client,
            &url,
            &target,
            Some(&asset.hash),
            asset.size,
            false,
            None,
        )
        .await?;

        let current = (index + 1) as u32;
        if should_emit_progress(current, total) {
            emit(
                progress,
                job_id,
                "assets",
                &format!("Assets ready: {current}/{total}."),
                Some(current),
                Some(total),
            );
        }
    }

    Ok(())
}

async fn install_java_runtime(
    client: &Client,
    minecraft_dir: &Path,
    job_id: &str,
    java_version: &JavaVersion,
    progress: &ProgressSink,
) -> LauncherResult<()> {
    let platform = java_runtime_platform_key().ok_or_else(|| {
        LauncherError::new(format!(
            "No Mojang Java runtime is available for {} on this architecture.",
            std::env::consts::OS
        ))
    })?;

    emit(
        progress,
        job_id,
        "runtime",
        &format!(
            "Preparing runtime {} (Java {}).",
            java_version.component, java_version.major_version
        ),
        None,
        None,
    );

    let runtime_index = fetch_json::<JavaRuntimeIndex>(client, JAVA_RUNTIME_MANIFEST_URL).await?;
    let runtime_entry = runtime_index
        .0
        .get(platform)
        .and_then(|components| components.get(&java_version.component))
        .and_then(|entries| entries.first())
        .ok_or_else(|| {
            LauncherError::new(format!(
                "Mojang does not publish {} for platform {platform}.",
                java_version.component
            ))
        })?;

    let manifest_text = fetch_text(client, &runtime_entry.manifest.url).await?;
    validate_bytes(
        manifest_text.as_bytes(),
        runtime_entry.manifest.sha1.as_deref(),
        runtime_entry.manifest.size,
        &runtime_entry.manifest.url,
    )?;
    let runtime_manifest = serde_json::from_str::<JavaRuntimeManifest>(&manifest_text)
        .map_err(|error| LauncherError::new(format!("Invalid Java runtime manifest: {error}")))?;

    let runtime_root = minecraft_dir
        .join("runtime")
        .join(&java_version.component)
        .join(platform)
        .join(&java_version.component);
    let files = runtime_manifest
        .files
        .iter()
        .filter(|(_, file)| file.file_type == "file")
        .collect::<Vec<_>>();
    let total = files.len() as u32;

    for (path, _file) in runtime_manifest
        .files
        .iter()
        .filter(|(_, file)| file.file_type == "directory")
    {
        fs::create_dir_all(safe_join(&runtime_root, path)?)?;
    }

    for (index, (path, file)) in files.iter().enumerate() {
        let Some(download) = file
            .downloads
            .as_ref()
            .and_then(|downloads| downloads.raw.as_ref())
        else {
            continue;
        };
        let target = safe_join(&runtime_root, path)?;
        download_file(
            client,
            &download.url,
            &target,
            download.sha1.as_deref(),
            download.size,
            file.executable,
            None,
        )
        .await?;

        let current = (index + 1) as u32;
        if should_emit_progress(current, total) {
            emit(
                progress,
                job_id,
                "runtime",
                &format!("Java runtime ready: {current}/{total}."),
                Some(current),
                Some(total),
            );
        }
    }

    for (path, file) in runtime_manifest
        .files
        .iter()
        .filter(|(_, file)| file.file_type == "link")
    {
        if let Some(target) = file.target.as_deref() {
            create_runtime_link(&runtime_root, path, target)?;
        }
    }

    Ok(())
}

async fn download_optifine_installer(
    client: &Client,
    minecraft_dir: &Path,
    option: &OptifineInstallOption,
    progress: &ProgressSink,
) -> LauncherResult<PathBuf> {
    emit(
        progress,
        &option.id,
        "optifine",
        &format!("Locating the official download for {}.", option.file_name),
        None,
        None,
    );

    let adload_url = format!("{OPTIFINE_ADLOAD_BASE_URL}?f={}", option.file_name);
    let html = fetch_text(client, &adload_url).await?;
    let download_path = extract_optifine_download_path(&html, &option.file_name)?;
    let download_url = format!("{OPTIFINE_BASE_URL}{download_path}");
    let installer_dir = minecraft_dir
        .join("versions")
        .join("_mecha-cache")
        .join("optifine");
    fs::create_dir_all(&installer_dir)?;
    let installer_path = installer_dir.join(&option.file_name);

    emit(
        progress,
        &option.id,
        "optifine",
        &format!("Downloading {}.", option.file_name),
        Some(0),
        Some(1),
    );
    download_file(
        client,
        &download_url,
        &installer_path,
        None,
        None,
        false,
        Some(&adload_url),
    )
    .await?;

    emit(
        progress,
        &option.id,
        "optifine",
        "OptiFine installer downloaded.",
        Some(1),
        Some(1),
    );

    Ok(installer_path)
}

async fn run_optifine_installer(
    minecraft_dir: &Path,
    option: &OptifineInstallOption,
    optifine_jar: &Path,
    progress: &ProgressSink,
) -> LauncherResult<()> {
    emit(
        progress,
        &option.id,
        "optifine",
        "Applying the OptiFine installer.",
        None,
        None,
    );

    let minecraft_dir = minecraft_dir.to_path_buf();
    let optifine_jar = optifine_jar.to_path_buf();

    tauri::async_runtime::spawn_blocking(move || {
        let java = resolve_installer_java()?;
        let java_major = detect_java_major(&java).ok_or_else(|| {
            LauncherError::new(
                "Java was not found. Install Java 21 from the dependency panel before installing OptiFine.",
            )
        })?;

        if java_major < 11 {
            return Err(LauncherError::new(format!(
                "The OptiFine installer needs Java 11 or newer, but PATH reports Java {java_major}."
            )));
        }

        let runner_dir = tempdir().map_err(|error| {
            LauncherError::new(format!("Failed to create installer runner directory: {error}"))
        })?;
        let runner_source = runner_dir.path().join("InstallOptiFine.java");
        fs::write(&runner_source, INSTALLER_RUNNER_SOURCE)?;

        let stdout_path = runner_dir.path().join("optifine-installer.stdout.log");
        let stderr_path = runner_dir.path().join("optifine-installer.stderr.log");
        let stdout_file = fs::File::create(&stdout_path).map_err(|error| {
            LauncherError::new(format!("Failed to create OptiFine stdout log: {error}"))
        })?;
        let stderr_file = fs::File::create(&stderr_path).map_err(|error| {
            LauncherError::new(format!("Failed to create OptiFine stderr log: {error}"))
        })?;

        let mut child = Command::new(java)
            .arg("-Djava.awt.headless=true")
            .arg("-cp")
            .arg(&optifine_jar)
            .arg(&runner_source)
            .arg(&minecraft_dir)
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file))
            .spawn()
            .map_err(|error| LauncherError::new(format!("Failed to run OptiFine installer: {error}")))?;

        let started_at = Instant::now();
        loop {
            if let Some(status) = child.try_wait().map_err(|error| {
                LauncherError::new(format!("Failed to poll OptiFine installer: {error}"))
            })? {
                if status.success() {
                    return Ok(());
                }

                let details = installer_output_details(&stdout_path, &stderr_path);

                return Err(LauncherError::new(format!(
                    "OptiFine installer exited with status {status}.{details}"
                )));
            }

            if started_at.elapsed() > OPTIFINE_INSTALLER_TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();

                let details = installer_output_details(&stdout_path, &stderr_path);
                return Err(LauncherError::new(
                    format!("OptiFine installer timed out while applying patches.{details}"),
                ));
            }

            thread::sleep(Duration::from_millis(250));
        }
    })
    .await
    .map_err(|error| LauncherError::new(format!("OptiFine installer task failed: {error}")))?
}

fn ensure_launcher_profiles_json(minecraft_dir: &Path) -> LauncherResult<()> {
    let profiles_path = minecraft_dir.join("launcher_profiles.json");

    if !profiles_path.exists() {
        let profiles = serde_json::json!({
            "profiles": {},
            "selectedProfile": "OptiFine"
        });
        let contents = serde_json::to_vec_pretty(&profiles)?;
        fs::write(profiles_path, contents)?;
        return Ok(());
    }

    let contents = fs::read_to_string(&profiles_path).map_err(|error| {
        LauncherError::new(format!(
            "Failed to read launcher profile file {}: {error}",
            profiles_path.display()
        ))
    })?;
    let mut profiles = serde_json::from_str::<serde_json::Value>(&contents).map_err(|error| {
        LauncherError::new(format!(
            "Invalid launcher profile file {}: {error}",
            profiles_path.display()
        ))
    })?;

    let Some(profile_object) = profiles.as_object_mut() else {
        return Err(LauncherError::new(format!(
            "Invalid launcher profile file {}: expected a JSON object.",
            profiles_path.display()
        )));
    };

    if !profile_object
        .get("profiles")
        .map(|value| value.is_object())
        .unwrap_or(false)
    {
        profile_object.insert("profiles".to_string(), serde_json::json!({}));
        let contents = serde_json::to_vec_pretty(&profiles)?;
        fs::write(profiles_path, contents)?;
    }

    Ok(())
}

fn installer_output_details(stdout_path: &Path, stderr_path: &Path) -> String {
    let stdout_log = fs::read_to_string(stdout_path).unwrap_or_default();
    let stderr_log = fs::read_to_string(stderr_path).unwrap_or_default();
    let combined = [stdout_log.trim(), stderr_log.trim()]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    if combined.is_empty() {
        return String::new();
    }

    let lines = combined.lines().collect::<Vec<_>>();
    let was_line_truncated = lines.len() > OPTIFINE_INSTALLER_ERROR_TAIL_LINES;
    let start = lines
        .len()
        .saturating_sub(OPTIFINE_INSTALLER_ERROR_TAIL_LINES);
    let mut output = lines[start..].join("\n");

    if output.chars().count() > OPTIFINE_INSTALLER_ERROR_MAX_CHARS {
        let chars = output.chars().collect::<Vec<_>>();
        let start = chars
            .len()
            .saturating_sub(OPTIFINE_INSTALLER_ERROR_MAX_CHARS);
        output = chars[start..].iter().collect::<String>();
    }

    let label = if was_line_truncated {
        format!(
            "\n\nInstaller output (last {} lines):\n",
            OPTIFINE_INSTALLER_ERROR_TAIL_LINES
        )
    } else {
        "\n\nInstaller output:\n".to_string()
    };

    format!("{label}{output}")
}

fn cleanup_optifine_version_dir(
    minecraft_dir: &Path,
    option: &OptifineInstallOption,
) -> LauncherResult<()> {
    let version_dir = minecraft_dir.join("versions").join(&option.version_id);
    if version_dir.exists() {
        fs::remove_dir_all(&version_dir)?;
    }

    Ok(())
}

fn verify_optifine_install(
    minecraft_dir: &Path,
    option: &OptifineInstallOption,
) -> LauncherResult<()> {
    let version_id = option.version_id.as_str();
    let version_dir = minecraft_dir.join("versions").join(&version_id);
    let version_json = version_dir.join(format!("{version_id}.json"));
    let version_jar = version_dir.join(format!("{version_id}.jar"));
    let optifine_library = minecraft_dir
        .join("libraries")
        .join("optifine")
        .join("OptiFine")
        .join(format!("{}_{}", option.minecraft_version, option.edition))
        .join(format!(
            "OptiFine-{}_{}.jar",
            option.minecraft_version, option.edition
        ));

    for required_file in [&version_json, &version_jar, &optifine_library] {
        if !required_file.is_file() {
            return Err(LauncherError::new(format!(
                "OptiFine installer did not create required file: {}",
                required_file.display()
            )));
        }
    }

    Ok(())
}

fn collect_required_libraries(
    environment: &RuntimeEnvironment,
    libraries: &[GameLibrary],
) -> LauncherResult<Vec<DownloadArtifact>> {
    let mut artifacts = Vec::new();

    for library in libraries {
        if !rules_allow(&library.rules, environment)? {
            continue;
        }

        if let Some(artifact) = library
            .downloads
            .as_ref()
            .and_then(|downloads| downloads.artifact.as_ref())
            .cloned()
            .or_else(|| fallback_library_artifact(library))
        {
            artifacts.push(artifact);
        }

        let Some(native_classifier) = library
            .natives
            .get(&environment.os_name)
            .or_else(|| {
                if environment.os_name == "osx" {
                    library.natives.get("macos")
                } else {
                    None
                }
            })
            .map(|value| value.replace("${arch}", &environment.arch_bits))
        else {
            continue;
        };

        if let Some(native) = library
            .downloads
            .as_ref()
            .and_then(|downloads| downloads.classifiers.get(&native_classifier))
            .cloned()
            .or_else(|| fallback_library_classifier_artifact(library, &native_classifier))
        {
            artifacts.push(native);
        }
    }

    Ok(artifacts)
}

fn fallback_library_artifact(library: &GameLibrary) -> Option<DownloadArtifact> {
    if !library.natives.is_empty() {
        return None;
    }

    fallback_library_artifact_with_classifier(library, None)
}

fn fallback_library_classifier_artifact(
    library: &GameLibrary,
    classifier: &str,
) -> Option<DownloadArtifact> {
    fallback_library_artifact_with_classifier(library, Some(classifier))
}

fn fallback_library_artifact_with_classifier(
    library: &GameLibrary,
    classifier: Option<&str>,
) -> Option<DownloadArtifact> {
    let name = library.name.as_ref()?;
    let path = maven_artifact_path(name, classifier)?;
    let base_url = library.url.as_deref().unwrap_or(LIBRARIES_BASE_URL);
    let url = format!("{}{}", ensure_trailing_slash(base_url), path);

    Some(DownloadArtifact {
        path: Some(path),
        sha1: None,
        size: None,
        url,
    })
}

fn maven_artifact_path(raw_value: &str, classifier_override: Option<&str>) -> Option<String> {
    let (coordinates, extension) = raw_value.split_once('@').unwrap_or((raw_value, "jar"));
    let segments = coordinates.split(':').collect::<Vec<_>>();
    let (group, artifact, version, classifier) = match segments.as_slice() {
        [group, artifact, version] => (*group, *artifact, *version, None),
        [group, artifact, version, classifier] => (*group, *artifact, *version, Some(*classifier)),
        _ => return None,
    };

    let classifier = classifier_override.or(classifier);
    let mut file_name = format!("{artifact}-{version}");
    if let Some(classifier) = classifier {
        file_name.push('-');
        file_name.push_str(classifier);
    }
    file_name.push('.');
    file_name.push_str(extension);

    Some(format!(
        "{}/{artifact}/{version}/{file_name}",
        group.replace('.', "/")
    ))
}

fn ensure_trailing_slash(value: &str) -> String {
    if value.ends_with('/') {
        value.to_string()
    } else {
        format!("{value}/")
    }
}

async fn fetch_json<T>(client: &Client, url: &str) -> LauncherResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    let text = fetch_text(client, url).await?;
    serde_json::from_str(&text)
        .map_err(|error| LauncherError::new(format!("Failed to parse JSON from {url}: {error}")))
}

async fn fetch_text(client: &Client, url: &str) -> LauncherResult<String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| LauncherError::new(format!("Failed to GET {url}: {error}")))?;

    let status = response.status();
    if !status.is_success() {
        return Err(LauncherError::new(format!(
            "GET {url} failed with HTTP {status}."
        )));
    }

    response
        .text()
        .await
        .map_err(|error| LauncherError::new(format!("Failed to read {url}: {error}")))
}

async fn download_file(
    client: &Client,
    url: &str,
    target: &Path,
    expected_sha1: Option<&str>,
    expected_size: Option<u64>,
    executable: bool,
    referer: Option<&str>,
) -> LauncherResult<DownloadOutcome> {
    if file_matches(target, expected_sha1, expected_size)? {
        set_executable_if_needed(target, executable)?;
        return Ok(DownloadOutcome::Reused);
    }

    if let Some(parent_dir) = target.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    let mut request = client.get(url);
    if let Some(referer) = referer {
        request = request.header(header::REFERER, referer);
    }

    let response = request
        .send()
        .await
        .map_err(|error| LauncherError::new(format!("Failed to download {url}: {error}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(LauncherError::new(format!(
            "Download {url} failed with HTTP {status}."
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|error| LauncherError::new(format!("Failed to read {url}: {error}")))?;
    validate_bytes(&bytes, expected_sha1, expected_size, url)?;

    let temp_path = part_path_for(target);
    fs::write(&temp_path, &bytes)?;
    set_executable_if_needed(&temp_path, executable)?;
    fs::rename(&temp_path, target)?;

    Ok(DownloadOutcome::Downloaded)
}

fn file_matches(
    target: &Path,
    expected_sha1: Option<&str>,
    expected_size: Option<u64>,
) -> LauncherResult<bool> {
    if !target.is_file() {
        return Ok(false);
    }

    if expected_sha1.is_none() && expected_size.is_none() {
        return Ok(fs::metadata(target)?.len() > 0);
    }

    if let Some(expected_size) = expected_size {
        let actual_size = fs::metadata(target)?.len();
        if actual_size != expected_size {
            return Ok(false);
        }
    }

    if let Some(expected_sha1) = expected_sha1 {
        let bytes = fs::read(target)?;
        if sha1_hex(&bytes) != expected_sha1 {
            return Ok(false);
        }
    }

    Ok(true)
}

fn validate_bytes(
    bytes: &[u8],
    expected_sha1: Option<&str>,
    expected_size: Option<u64>,
    url: &str,
) -> LauncherResult<()> {
    if let Some(expected_size) = expected_size {
        if bytes.len() as u64 != expected_size {
            return Err(LauncherError::new(format!(
                "Downloaded size mismatch for {url}: expected {expected_size}, got {}.",
                bytes.len()
            )));
        }
    }

    if let Some(expected_sha1) = expected_sha1 {
        let actual_sha1 = sha1_hex(bytes);
        if actual_sha1 != expected_sha1 {
            return Err(LauncherError::new(format!(
                "Downloaded SHA-1 mismatch for {url}: expected {expected_sha1}, got {actual_sha1}."
            )));
        }
    }

    Ok(())
}

fn part_path_for(target: &Path) -> PathBuf {
    let file_name = target
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "download".into());
    target.with_file_name(format!(".{file_name}.part"))
}

fn safe_join(base: &Path, relative: &str) -> LauncherResult<PathBuf> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute() {
        return Err(LauncherError::new(format!(
            "Refusing to write absolute path from manifest: {relative}"
        )));
    }

    for component in relative_path.components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return Err(LauncherError::new(format!(
                "Refusing unsafe manifest path: {relative}"
            )));
        }
    }

    Ok(base.join(relative_path))
}

fn should_emit_progress(current: u32, total: u32) -> bool {
    current == total || current == 1 || current % 25 == 0
}

fn java_runtime_platform_key() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some("windows-x64"),
        ("windows", "x86") => Some("windows-x86"),
        ("windows", "aarch64") => Some("windows-arm64"),
        ("macos", "aarch64") => Some("mac-os-arm64"),
        ("macos", _) => Some("mac-os"),
        ("linux", "x86") => Some("linux-i386"),
        ("linux", "x86_64") => Some("linux"),
        _ => None,
    }
}

fn create_runtime_link(runtime_root: &Path, path: &str, target: &str) -> LauncherResult<()> {
    let link_path = safe_join(runtime_root, path)?;
    if let Some(parent_dir) = link_path.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    if link_path.exists() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link_path)?;
    }

    #[cfg(windows)]
    {
        let target_path = link_path
            .parent()
            .map(|parent| parent.join(target))
            .unwrap_or_else(|| PathBuf::from(target));
        if target_path.is_file() {
            fs::copy(target_path, link_path)?;
        }
    }

    Ok(())
}

fn set_executable_if_needed(path: &Path, executable: bool) -> LauncherResult<()> {
    if !executable {
        return Ok(());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
    }

    #[cfg(not(unix))]
    let _ = path;

    Ok(())
}

fn extract_optifine_download_path(html: &str, file_name: &str) -> LauncherResult<String> {
    let pattern = Regex::new(r#"href=['"]([^'"]*downloadx\?f=([^'"]+))['"]"#)
        .expect("OptiFine download regex should compile");

    for captures in pattern.captures_iter(html) {
        let Some(path_match) = captures.get(1) else {
            continue;
        };
        let path = path_match.as_str().replace("&amp;", "&");
        if path.contains(file_name) {
            return Ok(path);
        }
    }

    Err(LauncherError::new(format!(
        "Could not find the official OptiFine mirror link for {file_name}."
    )))
}

fn resolve_installer_java() -> LauncherResult<PathBuf> {
    let java = PathBuf::from(if cfg!(windows) { "java.exe" } else { "java" });
    if detect_java_major(&java).is_some() {
        return Ok(java);
    }

    Err(LauncherError::new(
        "Java was not found in PATH. Install Java from the dependency panel first.",
    ))
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
    captures.get(1)?.as_str().parse::<u32>().ok()
}

#[allow(dead_code)]
fn find_runtime_java(runtime_root: &Path) -> Option<PathBuf> {
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs, sync::Arc};

    use crate::launcher::rules::RuntimeEnvironment;

    use super::{
        build_redux_manifest, collect_required_libraries, default_mods_subdir,
        extract_optifine_download_path, install_redux_instance_files, list_redux_install_options_from,
        maven_artifact_path, parse_optifine_downloads_html, safe_join, DownloadArtifact,
        GameLibrary, LibraryDownloads, ReduxPackManifest,
    };
    use tempfile::tempdir;

    #[test]
    fn parses_latest_optifine_per_minecraft_version() {
        let html = r#"
          <a href="adloadx?f=OptiFine_1.20.4_HD_U_I7.jar">Download</a>
          <a href="adloadx?f=OptiFine_1.20.4_HD_U_I6.jar">Older</a>
          <a href="adloadx?f=OptiFine_1.16.5_HD_U_G8.jar">Download</a>
        "#;
        let options = parse_optifine_downloads_html(html);
        let ids = options
            .iter()
            .map(|option| option.version_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec!["1.20.4-OptiFine_HD_U_I7", "1.16.5-OptiFine_HD_U_G8"]
        );
    }

    #[test]
    fn prefers_stable_optifine_builds_over_prereleases_for_same_branch() {
        let html = r#"
          <a href="adloadx?f=OptiFine_1.20.4_HD_U_I7_pre2.jar">Preview</a>
          <a href="adloadx?f=OptiFine_1.20.4_HD_U_I7.jar">Stable</a>
          <a href="adloadx?f=OptiFine_1.20.4_HD_U_I6.jar">Older</a>
        "#;

        let options = parse_optifine_downloads_html(html);

        assert_eq!(options.len(), 1);
        assert_eq!(options[0].edition, "HD_U_I7");
        assert_eq!(options[0].release_kind, "Stable");
    }

    #[test]
    fn extracts_official_optifine_mirror_path() {
        let html = r#"<a href='downloadx?f=OptiFine_1.16.5_HD_U_G8.jar&amp;x=abc'>Download</a>"#;
        let path = extract_optifine_download_path(html, "OptiFine_1.16.5_HD_U_G8.jar")
            .expect("path should parse");

        assert_eq!(path, "downloadx?f=OptiFine_1.16.5_HD_U_G8.jar&x=abc");
    }

    #[test]
    fn builds_maven_artifact_paths() {
        assert_eq!(
            maven_artifact_path("net.minecraft:launchwrapper:1.12", None),
            Some("net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar".to_string())
        );
        assert_eq!(
            maven_artifact_path(
                "org.lwjgl.lwjgl:lwjgl-platform:2.9.4",
                Some("natives-linux")
            ),
            Some(
                "org/lwjgl/lwjgl/lwjgl-platform/2.9.4/lwjgl-platform-2.9.4-natives-linux.jar"
                    .to_string()
            )
        );
    }

    #[test]
    fn required_libraries_skip_native_only_base_artifact() {
        let mut classifiers = HashMap::new();
        classifiers.insert(
            "natives-linux".to_string(),
            DownloadArtifact {
                path: Some(
                    "net/java/jinput/jinput-platform/2.0.5/jinput-platform-2.0.5-natives-linux.jar"
                        .to_string(),
                ),
                sha1: None,
                size: None,
                url: "https://libraries.minecraft.net/net/java/jinput/jinput-platform/2.0.5/jinput-platform-2.0.5-natives-linux.jar"
                    .to_string(),
            },
        );

        let mut natives = HashMap::new();
        natives.insert("linux".to_string(), "natives-linux".to_string());

        let environment = RuntimeEnvironment {
            os_name: "linux".to_string(),
            os_arch: "x86_64".to_string(),
            arch_bits: "64".to_string(),
            os_version: None,
            features: HashMap::new(),
        };

        let artifacts = collect_required_libraries(
            &environment,
            &[GameLibrary {
                name: Some("net.java.jinput:jinput-platform:2.0.5".to_string()),
                url: None,
                downloads: Some(LibraryDownloads {
                    artifact: None,
                    classifiers,
                }),
                natives,
                rules: Vec::new(),
            }],
        )
        .expect("libraries should resolve");

        assert_eq!(artifacts.len(), 1);
        assert!(artifacts[0]
            .path
            .as_ref()
            .expect("path should exist")
            .ends_with("jinput-platform-2.0.5-natives-linux.jar"));
    }

    #[test]
    fn ensure_launcher_profiles_json_creates_minimal_profile_file() {
        let temp_dir = tempdir().expect("tempdir should be created");

        super::ensure_launcher_profiles_json(temp_dir.path())
            .expect("launcher profiles should be created");

        let profile_path = temp_dir.path().join("launcher_profiles.json");
        let contents = fs::read_to_string(profile_path).expect("profile file should be readable");
        let parsed = serde_json::from_str::<serde_json::Value>(&contents)
            .expect("profile file should be valid JSON");

        assert!(parsed
            .get("profiles")
            .and_then(|profiles| profiles.as_object())
            .is_some());
    }

    #[test]
    fn installer_output_details_is_bounded_to_tail() {
        let temp_dir = tempdir().expect("tempdir should be created");
        let stdout_path = temp_dir.path().join("stdout.log");
        let stderr_path = temp_dir.path().join("stderr.log");
        let stdout = (0..120)
            .map(|line| format!("line {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&stdout_path, stdout).expect("stdout should be written");
        fs::write(&stderr_path, "root cause").expect("stderr should be written");

        let details = super::installer_output_details(&stdout_path, &stderr_path);

        assert!(details.contains("last 80 lines"));
        assert!(!details.contains("line 0"));
        assert!(details.contains("line 119"));
        assert!(details.contains("root cause"));
    }

    #[test]
    fn safe_join_rejects_path_traversal() {
        assert!(safe_join(std::path::Path::new("/tmp/root"), "../escape").is_err());
        assert!(safe_join(std::path::Path::new("/tmp/root"), "libraries/demo.jar").is_ok());
    }

    #[test]
    fn list_redux_install_options_reads_pack_manifest() {
        let temp_dir = tempdir().expect("tempdir should be created");
        let pack_dir = temp_dir.path().join("minecraft-1.16.5-redux");
        fs::create_dir_all(&pack_dir).expect("pack dir should exist");
        fs::write(
            pack_dir.join("pack.json"),
            r#"{
              "id":"1.16.5-redux",
              "title":"Minecraft 1.16.5 Redux",
              "minecraftVersion":"1.16.5",
              "fabricLoaderVersion":"0.16.10",
              "recommendedJavaMajor":8
            }"#,
        )
        .expect("pack manifest should be written");

        let options =
            list_redux_install_options_from(temp_dir.path()).expect("redux options should load");

        assert_eq!(options.len(), 1);
        assert_eq!(options[0].id, "1.16.5-redux");
        assert_eq!(options[0].minecraft_version, "1.16.5");
        assert_eq!(options[0].fabric_loader_version, "0.16.10");
    }

    #[test]
    fn build_redux_manifest_sets_id_inherits_and_mecha_metadata() {
        let manifest = ReduxPackManifest {
            id: "1.16.5-redux".to_string(),
            title: "Minecraft 1.16.5 Redux".to_string(),
            minecraft_version: "1.16.5".to_string(),
            fabric_loader_version: "0.16.10".to_string(),
            mods_subdir: default_mods_subdir(),
            recommended_java_major: 8,
        };

        let rendered = build_redux_manifest(
            r#"{"id":"fabric-loader-0.16.10-1.16.5","mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient"}"#,
            &manifest,
            std::path::Path::new("/tmp/.minecraft/mecha-instances/1.16.5-redux"),
        )
        .expect("redux manifest should render");
        let parsed =
            serde_json::from_str::<serde_json::Value>(&rendered).expect("manifest should parse");

        assert_eq!(parsed["id"], "1.16.5-redux");
        assert_eq!(parsed["inheritsFrom"], "1.16.5");
        assert_eq!(
            parsed["mecha"]["sourceKind"],
            serde_json::Value::String("redux".to_string())
        );
    }

    #[test]
    fn install_redux_instance_files_copies_jar_mods_into_isolated_mods_dir() {
        let temp_dir = tempdir().expect("tempdir should be created");
        let source_dir = temp_dir.path().join("pack");
        let instance_dir = temp_dir.path().join(".minecraft").join("mecha-instances").join("1.16.5-redux");
        fs::create_dir_all(&source_dir).expect("source dir should exist");
        fs::write(source_dir.join("fabric-api.jar"), "demo").expect("mod should be written");
        fs::write(source_dir.join("README.txt"), "skip").expect("non-mod file should be written");

        let progress: super::ReduxProgressSink = Arc::new(|_event| {});
        install_redux_instance_files(
            &instance_dir,
            &source_dir,
            &progress,
            "1.16.5-redux",
            "1.16.5-redux",
        )
        .expect("redux instance files should install");

        assert!(instance_dir.join("mods").join("fabric-api.jar").is_file());
        assert!(!instance_dir.join("mods").join("README.txt").exists());
        assert!(instance_dir.join("config").is_dir());
        assert!(instance_dir.join("resourcepacks").is_dir());
        assert!(instance_dir.join("shaderpacks").is_dir());
    }
}
