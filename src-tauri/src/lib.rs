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
                    .permission("allow-auto-install-java")
                    .permission("allow-auto-install-graphics-dependency")
                    .permission("allow-check-java-dependency")
                    .permission("allow-check-graphics-dependency")
                    .permission("allow-detect-default-minecraft-dir")
                    .permission("allow-get-system-memory-profile")
                    .permission("allow-ensure-versions-dir")
                    .permission("allow-install-optifine-version")
                    .permission("allow-install-redux-version")
                    .permission("allow-install-vanilla-version")
                    .permission("allow-launch-version")
                    .permission("allow-list-optifine-install-options")
                    .permission("allow-list-redux-install-options")
                    .permission("allow-list-vanilla-releases")
                    .permission("allow-delete-installed-version")
                    .permission("allow-list-versions"),
            )?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::browse_minecraft_dir,
            commands::auto_install_java,
            commands::auto_install_graphics_dependency,
            commands::check_java_dependency,
            commands::check_graphics_dependency,
            commands::detect_default_minecraft_dir,
            commands::get_system_memory_profile,
            commands::ensure_versions_dir,
            commands::install_optifine_version,
            commands::install_redux_version,
            commands::install_vanilla_version,
            commands::delete_installed_version,
            commands::launch_version,
            commands::list_optifine_install_options,
            commands::list_redux_install_options,
            commands::list_vanilla_releases,
            commands::list_versions
        ])
        .run(tauri::generate_context!())
        .expect("error while running mecha launcher");
}
