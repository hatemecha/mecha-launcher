fn main() {
    let attributes = tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "auto_install_graphics_dependency",
            "auto_install_java",
            "browse_minecraft_dir",
            "check_graphics_dependency",
            "check_java_dependency",
            "delete_installed_version",
            "detect_default_minecraft_dir",
            "get_system_memory_profile",
            "ensure_versions_dir",
            "install_optifine_version",
            "install_redux_version",
            "install_vanilla_version",
            "launch_version",
            "list_optifine_install_options",
            "list_redux_install_options",
            "list_vanilla_releases",
            "list_versions",
        ]),
    );

    tauri_build::try_build(attributes).expect("failed to run tauri build script");
}
