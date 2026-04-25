export interface MinecraftVersionSummary {
  id: string;
  folderName: string;
  jarPath: string;
  manifestPath: string;
  javaComponent?: string | null;
  javaMajorVersion?: number | null;
  gameDirectory?: string | null;
  sourceKind?: "local" | "vanilla" | "optifine" | "redux" | null;
}

export interface LaunchRequest {
  minecraftDir: string;
  versionId: string;
  username: string;
  requiredJavaMajor?: number | null;
  maxMemoryMb?: number | null;
}

export interface LaunchResponse {
  launchId: string;
}

export type LaunchMemoryMode = "auto" | "manual";

export interface SystemMemoryProfile {
  detected: boolean;
  totalMemoryMb: number;
  minAllocatableMb: number;
  maxAllocatableMb: number;
  recommendedAllocatableMb: number;
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

export interface JavaDependencyRequest {
  requiredMajor?: number | null;
}

export interface JavaDependencyStatus {
  installed: boolean;
  meetsRequirement: boolean;
  detectedMajor?: number | null;
  detectedRaw?: string | null;
  suggestedLinuxCommands?: string[] | null;
  suggestedWindowsLinks?: DependencyLink[] | null;
  canAutoInstall?: boolean | null;
  autoInstallHint?: string | null;
  requiredMajor?: number | null;
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
  /** winget: JRE package already installed and up to date (exit code may still be non-zero). */
  alreadyPresent?: boolean;
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

export interface ReduxInstallOption {
  id: string;
  versionId: string;
  title: string;
  summary: string;
  minecraftVersion: string;
  fabricLoaderVersion: string;
  recommendedJavaMajor: number;
}

export interface ReduxInstallRequest {
  minecraftDir: string;
  optionId: string;
}

export interface ReduxInstallResult {
  versionId: string;
  minecraftVersion: string;
  fabricLoaderVersion: string;
}

export interface ReduxInstallStatusEvent {
  optionId: string;
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
