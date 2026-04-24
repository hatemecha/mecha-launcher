use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    sync::Arc,
    thread,
};

use crate::{
    launcher::{
        LaunchPlan, LauncherError, LauncherLogEvent, LauncherLogSource, LauncherResult,
        LauncherStatusEvent, LauncherStatusState,
    },
    state::LaunchTracker,
};

pub trait EventSink: Send + Sync + 'static {
    fn emit_status(&self, event: LauncherStatusEvent);
    fn emit_log(&self, event: LauncherLogEvent);
}

pub fn spawn_launch(plan: LaunchPlan, sink: Arc<dyn EventSink>, tracker: LaunchTracker) {
    thread::spawn(move || {
        let launch_id = plan.launch_id.clone();
        let result = run_launch(plan, sink.as_ref());

        tracker.clear_if_matches(&launch_id);

        if let Err(error) = result {
            sink.emit_status(LauncherStatusEvent::new(
                launch_id,
                LauncherStatusState::Error,
                Some(error.to_string()),
            ));
        }
    });
}

pub fn run_launch(plan: LaunchPlan, sink: &dyn EventSink) -> LauncherResult<()> {
    let launch_id = plan.launch_id.clone();
    let command_arguments = plan.command_arguments();

    sink.emit_log(LauncherLogEvent::new(
        launch_id.clone(),
        LauncherLogSource::System,
        format!(
            "Spawning {} {}",
            plan.java_executable.display(),
            command_arguments.join(" ")
        ),
    ));
    sink.emit_log(LauncherLogEvent::new(
        launch_id.clone(),
        LauncherLogSource::System,
        format!("Classpath entries: {}", plan.classpath_entries.len()),
    ));

    let mut command = Command::new(&plan.java_executable);
    command
        .args(&plan.jvm_args)
        .arg(&plan.main_class)
        .args(&plan.game_args)
        .current_dir(&plan.game_directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|error| {
        LauncherError::new(format!(
            "Failed to spawn Java process at {}: {error}",
            plan.java_executable.display()
        ))
    })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        LauncherError::new("Failed to capture stdout from the Minecraft process.")
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        LauncherError::new("Failed to capture stderr from the Minecraft process.")
    })?;

    sink.emit_status(LauncherStatusEvent::new(
        launch_id.clone(),
        LauncherStatusState::Running,
        Some(format!("Minecraft {} is running.", plan.version_id)),
    ));

    thread::scope(|scope| -> LauncherResult<()> {
        let stdout_launch_id = launch_id.clone();
        let stderr_launch_id = launch_id.clone();
        let stdout_handle = scope
            .spawn(move || read_output(stdout, sink, &stdout_launch_id, LauncherLogSource::Stdout));
        let stderr_handle = scope
            .spawn(move || read_output(stderr, sink, &stderr_launch_id, LauncherLogSource::Stderr));

        let exit_status = child.wait().map_err(|error| {
            LauncherError::new(format!("Failed to wait for Minecraft process: {error}"))
        })?;

        stdout_handle
            .join()
            .expect("stdout thread should not panic")?;
        stderr_handle
            .join()
            .expect("stderr thread should not panic")?;

        let message = match exit_status.code() {
            Some(code) => format!("Minecraft exited with code {code}."),
            None => "Minecraft exited without a process code.".to_string(),
        };

        sink.emit_status(LauncherStatusEvent::new(
            launch_id,
            LauncherStatusState::Exited,
            Some(message),
        ));

        Ok(())
    })?;

    Ok(())
}

fn read_output<R: std::io::Read>(
    reader: R,
    sink: &dyn EventSink,
    launch_id: &str,
    source: LauncherLogSource,
) -> LauncherResult<()> {
    let mut buffered_reader = BufReader::new(reader);
    let mut buf = Vec::new();

    loop {
        buf.clear();
        let bytes_read = buffered_reader.read_until(b'\n', &mut buf).map_err(|error| {
            LauncherError::new(format!("Failed to read Minecraft process output: {error}"))
        })?;
        if bytes_read == 0 {
            break;
        }
        while matches!(buf.last(), Some(&b'\n') | Some(&b'\r')) {
            buf.pop();
        }
        let line = String::from_utf8_lossy(&buf).into_owned();
        sink.emit_log(LauncherLogEvent::new(launch_id.to_string(), source, line));
    }

    Ok(())
}
