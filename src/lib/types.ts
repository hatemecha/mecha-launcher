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

export interface DependencyLink {
  label: string;
  url: string;
}

export interface JavaDependencyStatus {
  installed: boolean;
  detectedMajor?: number | null;
  detectedRaw?: string | null;
  suggestedLinuxCommands?: string[] | null;
  suggestedWindowsLinks?: DependencyLink[] | null;
  canAutoInstall?: boolean | null;
  autoInstallHint?: string | null;
  recommendedMajor?: number | null;
}

export interface GraphicsDependencyStatus {
  required: boolean;
  installed: boolean;
  usable: boolean;
  detectedRaw?: string | null;
  suggestedLinuxCommands?: string[] | null;
  canAutoInstall?: boolean | null;
  autoInstallHint?: string | null;
}

export interface DependencyInstallResult {
  ok: boolean;
  exitCode?: number | null;
  stdout: string;
  stderr: string;
}

export interface OptifineInstallOption {
  id: string;
  minecraftVersion: string;
  optifineVersion: string;
  edition: string;
  fileName: string;
  versionId: string;
  title: string;
  summary: string;
  releaseKind: string;
  recommendedJavaMajor: number;
  sourceUrl: string;
}

export interface OptifineInstallRequest {
  minecraftDir: string;
  optionId: string;
}

export interface OptifineInstallResult {
  versionId: string;
  minecraftVersion: string;
  optifineVersion: string;
}

export interface OptifineInstallStatusEvent {
  optionId: string;
  stage: string;
  message: string;
  current?: number | null;
  total?: number | null;
}

export interface VanillaRelease {
  id: string;
  releaseTime?: string | null;
}

export interface VanillaInstallRequest {
  minecraftDir: string;
  versionId: string;
}

export interface VanillaInstallResult {
  versionId: string;
}

export interface VanillaInstallStatusEvent {
  versionId: string;
  stage: string;
  message: string;
  current?: number | null;
  total?: number | null;
}

export interface DeleteInstalledVersionRequest {
  minecraftDir: string;
  versionId: string;
}
