use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use mecha_launcher_lib::launcher::{
    list_versions, prepare_launch, process::run_launch, process::EventSink, LaunchPlan,
    LaunchRequest, LauncherLogEvent, LauncherLogSource, LauncherStatusEvent, LauncherStatusState,
};
use tempfile::tempdir;

#[derive(Default)]
struct RecordingSink {
    statuses: Mutex<Vec<LauncherStatusEvent>>,
    logs: Mutex<Vec<LauncherLogEvent>>,
}

impl RecordingSink {
    fn statuses(&self) -> Vec<LauncherStatusEvent> {
        self.statuses
            .lock()
            .expect("status mutex should not be poisoned")
            .clone()
    }

    fn logs(&self) -> Vec<LauncherLogEvent> {
        self.logs
            .lock()
            .expect("log mutex should not be poisoned")
            .clone()
    }
}

impl EventSink for RecordingSink {
    fn emit_status(&self, event: LauncherStatusEvent) {
        self.statuses
            .lock()
            .expect("status mutex should not be poisoned")
            .push(event);
    }

    fn emit_log(&self, event: LauncherLogEvent) {
        self.logs
            .lock()
            .expect("log mutex should not be poisoned")
            .push(event);
    }
}

#[test]
fn list_versions_only_returns_complete_version_directories() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let minecraft_dir = create_minecraft_layout(temp_dir.path());

    write_version(
        &minecraft_dir,
        "valid",
        r#"{"id":"valid","mainClass":"net.minecraft.Main","assetIndex":{"id":"demo"}}"#,
        true,
    );
    write_version(
        &minecraft_dir,
        "missing-jar",
        r#"{"id":"missing-jar","mainClass":"net.minecraft.Main","assetIndex":{"id":"demo"}}"#,
        false,
    );

    let versions = list_versions(&minecraft_dir).expect("versions should load");

    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].id, "valid");
}

#[test]
fn prepare_launch_supports_modern_and_legacy_manifests() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let minecraft_dir = create_minecraft_layout(temp_dir.path());

    write_asset_index(&minecraft_dir, "1.20");
    write_asset_index(&minecraft_dir, "1.8");
    write_runtime_java(&minecraft_dir, "java-runtime-delta");
    write_runtime_java(&minecraft_dir, "jre-legacy");
    write_library(&minecraft_dir, "com/example/demo/1.0.0/demo-1.0.0.jar");

    write_version(
        &minecraft_dir,
        "modern",
        r#"{
          "id":"modern",
          "mainClass":"net.minecraft.client.main.Main",
          "assetIndex":{"id":"1.20"},
          "javaVersion":{"component":"java-runtime-delta","majorVersion":21},
          "arguments":{
            "jvm":["-Djava.library.path=${natives_directory}","-cp","${classpath}"],
            "game":["--username","${auth_player_name}","--version","${version_name}","--gameDir","${game_directory}","--assetsDir","${assets_root}","--assetIndex","${assets_index_name}"]
          },
          "libraries":[
            {
              "name":"com.example:demo:1.0.0",
              "downloads":{"artifact":{"path":"com/example/demo/1.0.0/demo-1.0.0.jar"}}
            }
          ]
        }"#,
        true,
    );
    write_version(
        &minecraft_dir,
        "legacy",
        r#"{
          "id":"legacy",
          "mainClass":"net.minecraft.client.main.Main",
          "assetIndex":{"id":"1.8"},
          "javaVersion":{"component":"jre-legacy","majorVersion":8},
          "minecraftArguments":"--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory}",
          "libraries":[
            {
              "name":"com.example:demo:1.0.0",
              "downloads":{"artifact":{"path":"com/example/demo/1.0.0/demo-1.0.0.jar"}}
            }
          ]
        }"#,
        true,
    );

    let modern_plan = prepare_launch(
        &LaunchRequest {
            minecraft_dir: minecraft_dir.to_string_lossy().to_string(),
            version_id: "modern".to_string(),
            username: "Player".to_string(),
        },
        "modern-launch".to_string(),
    )
    .expect("modern plan should build");

    assert!(modern_plan.jvm_args.iter().any(|value| value == "-cp"));
    assert!(modern_plan
        .game_args
        .windows(2)
        .any(|pair| pair == ["--version", "modern"]));
    assert!(modern_plan.command_arguments().join(" ").contains("Player"));

    let legacy_plan = prepare_launch(
        &LaunchRequest {
            minecraft_dir: minecraft_dir.to_string_lossy().to_string(),
            version_id: "legacy".to_string(),
            username: "Player".to_string(),
        },
        "legacy-launch".to_string(),
    )
    .expect("legacy plan should build");

    assert!(legacy_plan.jvm_args.iter().any(|value| value == "-cp"));
    assert!(legacy_plan
        .game_args
        .windows(2)
        .any(|pair| pair == ["--version", "legacy"]));
}

#[test]
fn prepare_launch_supports_values_alias_and_offline_placeholders() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let minecraft_dir = create_minecraft_layout(temp_dir.path());

    write_asset_index(&minecraft_dir, "1.21");
    write_runtime_java(&minecraft_dir, "java-runtime-delta");
    write_library(&minecraft_dir, "com/example/demo/1.0.0/demo-1.0.0.jar");

    write_version(
        &minecraft_dir,
        "modern-values",
        r#"{
          "id":"modern-values",
          "mainClass":"net.minecraft.client.main.Main",
          "assetIndex":{"id":"1.21"},
          "javaVersion":{"component":"java-runtime-delta","majorVersion":21},
          "arguments":{
            "jvm":[
              {
                "rules":[],
                "values":["-Djava.library.path=${natives_directory}","-cp","${classpath}"]
              }
            ],
            "game":[
              {
                "rules":[],
                "values":["--username","${auth_player_name}","--clientId","${clientid}","--xuid","${auth_xuid}"]
              }
            ]
          },
          "libraries":[
            {
              "name":"com.example:demo:1.0.0",
              "downloads":{"artifact":{"path":"com/example/demo/1.0.0/demo-1.0.0.jar"}}
            }
          ]
        }"#,
        true,
    );

    let plan = prepare_launch(
        &LaunchRequest {
            minecraft_dir: minecraft_dir.to_string_lossy().to_string(),
            version_id: "modern-values".to_string(),
            username: "Player".to_string(),
        },
        "modern-values-launch".to_string(),
    )
    .expect("modern values plan should build");

    assert!(plan.jvm_args.iter().any(|value| value == "-cp"));
    assert!(plan
        .game_args
        .windows(2)
        .any(|pair| pair == ["--clientId", "offline-client-id"]));
    assert!(plan
        .game_args
        .windows(2)
        .any(|pair| pair == ["--xuid", "0"]));
}

#[test]
fn prepare_launch_supports_legacy_library_artifacts_and_skips_native_only_classpath_entries() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let minecraft_dir = create_minecraft_layout(temp_dir.path());

    write_asset_index(&minecraft_dir, "1.8");
    write_runtime_java(&minecraft_dir, "jre-legacy");
    write_library(
        &minecraft_dir,
        "net/minecraft/launchwrapper/1.7/launchwrapper-1.7.jar",
    );
    write_library(
        &minecraft_dir,
        "org/lwjgl/lwjgl/lwjgl-platform/2.9.4-nightly-20150209/lwjgl-platform-2.9.4-nightly-20150209-natives-windows.jar",
    );

    write_version(
        &minecraft_dir,
        "legacy-native-only",
        r#"{
          "id":"legacy-native-only",
          "mainClass":"net.minecraft.launchwrapper.Launch",
          "assetIndex":{"id":"1.8"},
          "javaVersion":{"component":"jre-legacy","majorVersion":8},
          "minecraftArguments":"--username ${auth_player_name} --version ${version_name}",
          "libraries":[
            {
              "name":"net.minecraft:launchwrapper:1.7",
              "artifact":{"path":"net/minecraft/launchwrapper/1.7/launchwrapper-1.7.jar"}
            },
            {
              "name":"org.lwjgl.lwjgl:lwjgl-platform:2.9.4-nightly-20150209",
              "natives":{"windows":"natives-windows"},
              "artifact":{"path":"org/lwjgl/lwjgl/lwjgl-platform/2.9.4-nightly-20150209/lwjgl-platform-2.9.4-nightly-20150209.jar"},
              "classifies":{
                "windows":{"path":"org/lwjgl/lwjgl/lwjgl-platform/2.9.4-nightly-20150209/lwjgl-platform-2.9.4-nightly-20150209-natives-windows.jar"}
              }
            }
          ]
        }"#,
        true,
    );
    fs::create_dir_all(
        minecraft_dir
            .join("versions")
            .join("legacy-native-only")
            .join("natives"),
    )
    .expect("existing natives directory should be created");

    let plan = prepare_launch(
        &LaunchRequest {
            minecraft_dir: minecraft_dir.to_string_lossy().to_string(),
            version_id: "legacy-native-only".to_string(),
            username: "Player".to_string(),
        },
        "legacy-native-only-launch".to_string(),
    )
    .expect("legacy native-only plan should build");

    assert!(plan
        .classpath_entries
        .iter()
        .any(|path| path.ends_with("launchwrapper-1.7.jar")));
    assert!(!plan.classpath_entries.iter().any(|path| {
        path.to_string_lossy()
            .contains("lwjgl-platform-2.9.4-nightly-20150209.jar")
    }));
}

#[test]
fn prepare_launch_reports_actionable_missing_inputs() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let minecraft_dir = create_minecraft_layout(temp_dir.path());

    write_asset_index(&minecraft_dir, "assets-ok");
    write_library(&minecraft_dir, "com/example/demo/1.0.0/demo-1.0.0.jar");

    write_version(
        &minecraft_dir,
        "missing-runtime",
        r#"{
          "id":"missing-runtime",
          "mainClass":"net.minecraft.client.main.Main",
          "assetIndex":{"id":"assets-ok"},
          "javaVersion":{"component":"java-runtime-delta","majorVersion":21},
          "minecraftArguments":"--username ${auth_player_name}",
          "libraries":[
            {
              "name":"com.example:demo:1.0.0",
              "downloads":{"artifact":{"path":"com/example/demo/1.0.0/demo-1.0.0.jar"}}
            }
          ]
        }"#,
        true,
    );
    write_version(
        &minecraft_dir,
        "missing-library",
        r#"{
          "id":"missing-library",
          "mainClass":"net.minecraft.client.main.Main",
          "assetIndex":{"id":"assets-ok"},
          "javaVersion":{"component":"jre-legacy","majorVersion":8},
          "minecraftArguments":"--username ${auth_player_name}",
          "libraries":[
            {
              "name":"com.example:missing:1.0.0",
              "downloads":{"artifact":{"path":"com/example/missing/1.0.0/missing-1.0.0.jar"}}
            }
          ]
        }"#,
        true,
    );
    write_version(
        &minecraft_dir,
        "missing-assets",
        r#"{
          "id":"missing-assets",
          "mainClass":"net.minecraft.client.main.Main",
          "assetIndex":{"id":"assets-missing"},
          "javaVersion":{"component":"jre-legacy","majorVersion":8},
          "minecraftArguments":"--username ${auth_player_name}"
        }"#,
        true,
    );
    let native_os = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    };
    let native_classifier = format!("natives-{native_os}");
    let missing_natives_manifest = format!(
        r#"{{
          "id":"missing-natives",
          "mainClass":"net.minecraft.client.main.Main",
          "assetIndex":{{"id":"assets-ok"}},
          "javaVersion":{{"component":"jre-legacy","majorVersion":8}},
          "minecraftArguments":"--username ${{auth_player_name}}",
          "libraries":[
            {{
              "name":"com.example:demo:1.0.0",
              "downloads":{{
                "artifact":{{"path":"com/example/demo/1.0.0/demo-1.0.0.jar"}},
                "classifiers":{{"{native_classifier}":{{"path":"com/example/demo/1.0.0/demo-1.0.0-{native_classifier}.jar"}}}}
              }},
              "natives":{{"{native_os}":"{native_classifier}"}}
            }}
          ]
        }}"#
    );

    write_version(
        &minecraft_dir,
        "missing-natives",
        &missing_natives_manifest,
        true,
    );
    write_runtime_java(&minecraft_dir, "jre-legacy");

    let runtime_error = prepare_launch(
        &LaunchRequest {
            minecraft_dir: minecraft_dir.to_string_lossy().to_string(),
            version_id: "missing-runtime".to_string(),
            username: "Player".to_string(),
        },
        "runtime".to_string(),
    )
    .expect_err("runtime should fail");
    assert!(runtime_error.to_string().contains("requires Java 21"));

    let library_error = prepare_launch(
        &LaunchRequest {
            minecraft_dir: minecraft_dir.to_string_lossy().to_string(),
            version_id: "missing-library".to_string(),
            username: "Player".to_string(),
        },
        "library".to_string(),
    )
    .expect_err("library should fail");
    assert!(library_error.to_string().contains("Missing library jar"));

    let assets_error = prepare_launch(
        &LaunchRequest {
            minecraft_dir: minecraft_dir.to_string_lossy().to_string(),
            version_id: "missing-assets".to_string(),
            username: "Player".to_string(),
        },
        "assets".to_string(),
    )
    .expect_err("assets should fail");
    assert!(assets_error
        .to_string()
        .contains("Missing asset index file"));

    let natives_error = prepare_launch(
        &LaunchRequest {
            minecraft_dir: minecraft_dir.to_string_lossy().to_string(),
            version_id: "missing-natives".to_string(),
            username: "Player".to_string(),
        },
        "natives".to_string(),
    )
    .expect_err("natives should fail");
    assert!(natives_error.to_string().contains("Missing native archive"));
}

#[test]
fn run_launch_forwards_stdout_and_stderr_logs() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let (java_executable, jvm_args) = if cfg!(windows) {
        let script_path = temp_dir.path().join("fake-java.cmd");
        fs::write(
            &script_path,
            "@echo off\r\necho hello stdout\r\necho hello stderr 1>&2\r\n",
        )
        .expect("script should be written");
        (
            PathBuf::from("cmd.exe"),
            vec!["/C".to_string(), script_path.to_string_lossy().to_string()],
        )
    } else {
        (
            PathBuf::from("/bin/sh"),
            vec![
                "-c".to_string(),
                "echo hello stdout; echo hello stderr >&2".to_string(),
            ],
        )
    };

    let sink = Arc::new(RecordingSink::default());
    let plan = LaunchPlan {
        launch_id: "fake-launch".to_string(),
        minecraft_dir: temp_dir.path().to_path_buf(),
        version_id: "fake".to_string(),
        java_executable,
        main_class: "ignored".to_string(),
        jvm_args,
        game_args: Vec::new(),
        classpath_entries: Vec::new(),
        cleanup_temp_dir: None,
    };

    run_launch(plan, sink.as_ref()).expect("process should run");

    let statuses = sink.statuses();
    assert!(statuses
        .iter()
        .any(|event| event.state == LauncherStatusState::Running));
    assert!(statuses
        .iter()
        .any(|event| event.state == LauncherStatusState::Exited));

    let logs = sink.logs();
    assert!(logs.iter().any(|event| {
        event.source == LauncherLogSource::Stdout && event.line.contains("hello stdout")
    }));
    assert!(logs.iter().any(|event| {
        event.source == LauncherLogSource::Stderr && event.line.contains("hello stderr")
    }));
}

fn create_minecraft_layout(root: &Path) -> PathBuf {
    let minecraft_dir = root.join(".minecraft");
    fs::create_dir_all(minecraft_dir.join("versions")).expect("versions dir should exist");
    fs::create_dir_all(minecraft_dir.join("libraries")).expect("libraries dir should exist");
    fs::create_dir_all(minecraft_dir.join("assets").join("indexes"))
        .expect("asset indexes dir should exist");
    minecraft_dir
}

fn write_version(minecraft_dir: &Path, version_id: &str, manifest_json: &str, with_jar: bool) {
    let version_dir = minecraft_dir.join("versions").join(version_id);
    fs::create_dir_all(&version_dir).expect("version dir should exist");
    fs::write(
        version_dir.join(format!("{version_id}.json")),
        manifest_json,
    )
    .expect("manifest should be written");
    if with_jar {
        fs::write(
            version_dir.join(format!("{version_id}.jar")),
            b"version-jar",
        )
        .expect("version jar should be written");
    }
}

fn write_library(minecraft_dir: &Path, relative_path: &str) {
    let jar_path = minecraft_dir.join("libraries").join(relative_path);
    if let Some(parent_dir) = jar_path.parent() {
        fs::create_dir_all(parent_dir).expect("library dir should exist");
    }
    fs::write(jar_path, b"library-jar").expect("library jar should be written");
}

fn write_asset_index(minecraft_dir: &Path, asset_index_id: &str) {
    let asset_path = minecraft_dir
        .join("assets")
        .join("indexes")
        .join(format!("{asset_index_id}.json"));
    fs::write(asset_path, b"{}").expect("asset index should be written");
}

fn write_runtime_java(minecraft_dir: &Path, component: &str) {
    let executable_name = if cfg!(windows) { "java.exe" } else { "java" };
    let bin_dir = minecraft_dir
        .join("runtime")
        .join(component)
        .join(if cfg!(windows) { "windows" } else { "linux" })
        .join(component)
        .join("bin");
    fs::create_dir_all(&bin_dir).expect("runtime dir should exist");
    fs::write(bin_dir.join(executable_name), b"fake-java").expect("runtime java should be written");
}
