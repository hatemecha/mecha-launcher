import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type {
  LaunchRequest,
  LaunchResponse,
  LauncherLogEvent,
  LauncherStatusEvent,
  MinecraftVersionSummary
} from "./types";

export async function detectDefaultMinecraftDir(): Promise<string | null> {
  return invoke<string | null>("detect_default_minecraft_dir");
}

export async function browseMinecraftDir(): Promise<string | null> {
  return invoke<string | null>("browse_minecraft_dir");
}

export async function listVersions(
  minecraftDir: string
): Promise<MinecraftVersionSummary[]> {
  return invoke<MinecraftVersionSummary[]>("list_versions", { minecraftDir });
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
