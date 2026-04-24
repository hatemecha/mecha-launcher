import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type {
  LaunchRequest,
  SystemMemoryProfile,
  LaunchResponse,
  DependencyInstallResult,
  GraphicsDependencyStatus,
  JavaDependencyRequest,
  JavaDependencyStatus,
  LauncherLogEvent,
  LauncherStatusEvent,
  MinecraftVersionSummary,
  OptifineInstallOption,
  OptifineInstallRequest,
  OptifineInstallResult,
  OptifineInstallStatusEvent,
  ReduxInstallOption,
  ReduxInstallRequest,
  ReduxInstallResult,
  ReduxInstallStatusEvent,
  VanillaInstallRequest,
  VanillaInstallResult,
  VanillaInstallStatusEvent,
  VanillaRelease,
  DeleteInstalledVersionRequest
} from "./types";

export async function detectDefaultMinecraftDir(): Promise<string | null> {
  return invoke<string | null>("detect_default_minecraft_dir");
}

export async function getSystemMemoryProfile(): Promise<SystemMemoryProfile> {
  return invoke<SystemMemoryProfile>("get_system_memory_profile");
}

export async function browseMinecraftDir(): Promise<string | null> {
  return invoke<string | null>("browse_minecraft_dir");
}

export async function checkJavaDependency(
  request?: JavaDependencyRequest
): Promise<JavaDependencyStatus> {
  return invoke<JavaDependencyStatus>("check_java_dependency", { request });
}

export async function checkGraphicsDependency(): Promise<GraphicsDependencyStatus> {
  return invoke<GraphicsDependencyStatus>("check_graphics_dependency");
}

export async function autoInstallJava(
  request?: JavaDependencyRequest
): Promise<DependencyInstallResult> {
  return invoke<DependencyInstallResult>("auto_install_java", { request });
}

export async function autoInstallGraphicsDependency(): Promise<DependencyInstallResult> {
  return invoke<DependencyInstallResult>("auto_install_graphics_dependency");
}

export async function ensureVersionsDir(minecraftDir: string): Promise<string> {
  return invoke<string>("ensure_versions_dir", { minecraftDir });
}

export async function listVersions(
  minecraftDir: string
): Promise<MinecraftVersionSummary[]> {
  return invoke<MinecraftVersionSummary[]>("list_versions", { minecraftDir });
}

export async function listOptifineInstallOptions(): Promise<OptifineInstallOption[]> {
  return invoke<OptifineInstallOption[]>("list_optifine_install_options");
}

export async function listVanillaReleases(): Promise<VanillaRelease[]> {
  return invoke<VanillaRelease[]>("list_vanilla_releases");
}

export async function listReduxInstallOptions(): Promise<ReduxInstallOption[]> {
  return invoke<ReduxInstallOption[]>("list_redux_install_options");
}

export async function installOptifineVersion(
  request: OptifineInstallRequest
): Promise<OptifineInstallResult> {
  return invoke<OptifineInstallResult>("install_optifine_version", { request });
}

export async function installVanillaVersion(
  request: VanillaInstallRequest
): Promise<VanillaInstallResult> {
  return invoke<VanillaInstallResult>("install_vanilla_version", { request });
}

export async function installReduxVersion(
  request: ReduxInstallRequest
): Promise<ReduxInstallResult> {
  return invoke<ReduxInstallResult>("install_redux_version", { request });
}

export async function deleteInstalledVersion(
  request: DeleteInstalledVersionRequest
): Promise<void> {
  return invoke<void>("delete_installed_version", { request });
}

export async function launchVersion(
  request: LaunchRequest
): Promise<LaunchResponse> {
  return invoke<LaunchResponse>("launch_version", { request });
}

export function onLauncherStatus(
  handler: (event: LauncherStatusEvent) => void
): Promise<() => void> {
  return listen<LauncherStatusEvent>("launcher:status", (event) => {
    handler(event.payload);
  });
}

export function onLauncherLog(
  handler: (event: LauncherLogEvent) => void
): Promise<() => void> {
  return listen<LauncherLogEvent>("launcher:log", (event) => {
    handler(event.payload);
  });
}

export function onOptifineInstallStatus(
  handler: (event: OptifineInstallStatusEvent) => void
): Promise<() => void> {
  return listen<OptifineInstallStatusEvent>("optifine-install:status", (event) => {
    handler(event.payload);
  });
}

export function onVanillaInstallStatus(
  handler: (event: VanillaInstallStatusEvent) => void
): Promise<() => void> {
  return listen<VanillaInstallStatusEvent>("vanilla-install:status", (event) => {
    handler(event.payload);
  });
}

export function onReduxInstallStatus(
  handler: (event: ReduxInstallStatusEvent) => void
): Promise<() => void> {
  return listen<ReduxInstallStatusEvent>("redux-install:status", (event) => {
    handler(event.payload);
  });
}
