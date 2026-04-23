const STORAGE_KEYS = {
  minecraftDir: "mecha-launcher.minecraftDir",
  selectedVersionId: "mecha-launcher.selectedVersionId"
} as const;

function readString(key: string): string | null {
  const value = localStorage.getItem(key);
  return value && value.trim().length > 0 ? value : null;
}

export function loadStoredMinecraftDir(): string | null {
  return readString(STORAGE_KEYS.minecraftDir);
}

export function storeMinecraftDir(path: string): void {
  localStorage.setItem(STORAGE_KEYS.minecraftDir, path);
}

export function loadStoredVersionId(): string | null {
  return readString(STORAGE_KEYS.selectedVersionId);
}

export function storeSelectedVersionId(versionId: string): void {
  localStorage.setItem(STORAGE_KEYS.selectedVersionId, versionId);
}

export function clearSelectedVersionId(): void {
  localStorage.removeItem(STORAGE_KEYS.selectedVersionId);
}
