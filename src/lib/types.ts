export interface MinecraftVersionSummary {
  id: string;
  folderName: string;
  jarPath: string;
  manifestPath: string;
  javaComponent?: string | null;
  javaMajorVersion?: number | null;
}

export interface LaunchRequest {
  minecraftDir: string;
  versionId: string;
  username: string;
}

export interface LaunchResponse {
  launchId: string;
}

export type LaunchState = "launching" | "running" | "exited" | "error";

export interface LauncherStatusEvent {
  launchId: string;
  state: LaunchState;
  message?: string | null;
}

export type LogSource = "stdout" | "stderr" | "system";

export interface LauncherLogEvent {
  launchId: string;
  source: LogSource;
  line: string;
}
