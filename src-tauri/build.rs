fn main() {
    let attributes = tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "browse_minecraft_dir",
            "detect_default_minecraft_dir",
            "launch_version",
            "list_versions",
        ]),
    );

    tauri_build::try_build(attributes).expect("failed to run tauri build script");
}
