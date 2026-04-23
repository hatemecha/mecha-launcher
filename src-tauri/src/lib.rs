mod commands;
pub mod launcher;
mod state;

use tauri::{ipc::CapabilityBuilder, Manager};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state::LaunchTracker::default())
        .setup(|app| {
            app.add_capability(
                CapabilityBuilder::new("main-capability")
                    .window("main")
                    .permission("core:default")
                    .permission("dialog:default")
                    .permission("allow-browse-minecraft-dir")
                    .permission("allow-detect-default-minecraft-dir")
                    .permission("allow-launch-version")
                    .permission("allow-list-versions"),
            )?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::browse_minecraft_dir,
            commands::detect_default_minecraft_dir,
            commands::launch_version,
            commands::list_versions
        ])
        .run(tauri::generate_context!())
        .expect("error while running mecha launcher");
}
