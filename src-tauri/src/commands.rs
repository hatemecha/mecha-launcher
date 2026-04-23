use std::{path::Path, sync::Arc};

use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::{
    launcher::{
        self, process::spawn_launch, process::EventSink, LaunchRequest, LaunchResponse,
        LauncherLogEvent, LauncherStatusEvent, LauncherStatusState, MinecraftVersionSummary,
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

#[tauri::command]
pub fn detect_default_minecraft_dir() -> Option<String> {
    launcher::detect_default_minecraft_dir().map(|path| launcher::path_to_string(&path))
}

#[tauri::command]
pub fn browse_minecraft_dir(app: AppHandle) -> Result<Option<String>, String> {
    let picked_path = app
        .dialog()
        .file()
        .set_title("Select .minecraft directory")
        .blocking_pick_folder()
        .map(|file_path| file_path.into_path())
        .transpose()
        .map_err(|error| error.to_string())?;

    Ok(picked_path.map(|path| launcher::path_to_string(&path)))
}

#[tauri::command]
pub fn list_versions(minecraft_dir: String) -> Result<Vec<MinecraftVersionSummary>, String> {
    launcher::list_versions(Path::new(&minecraft_dir)).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn launch_version(
    app: AppHandle,
    tracker: State<LaunchTracker>,
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
