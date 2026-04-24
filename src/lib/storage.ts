const STORAGE_KEYS = {
  minecraftDir: "mecha-launcher.minecraftDir",
  selectedVersionId: "mecha-launcher.selectedVersionId",
  favorites: "mecha-launcher.versionFavorites",
  popularity: "mecha-launcher.versionPopularity",
  offlineUsername: "mecha-launcher.offlineUsername",
  offlineSkinDataUrl: "mecha-launcher.offlineSkinDataUrl"
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

type VersionCatalogKey = `${"vanilla" | "optifine"}:${string}`;

function readJson<T>(key: string, fallback: T): T {
  const raw = localStorage.getItem(key);
  if (!raw) {
    return fallback;
  }
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

function writeJson<T>(key: string, value: T): void {
  localStorage.setItem(key, JSON.stringify(value));
}

export function loadFavoriteKeys(): Set<VersionCatalogKey> {
  const values = readJson<VersionCatalogKey[]>(STORAGE_KEYS.favorites, []);
  return new Set(values);
}

export function storeFavoriteKeys(keys: Set<VersionCatalogKey>): void {
  writeJson(STORAGE_KEYS.favorites, Array.from(keys));
}

export function toggleFavoriteKey(key: VersionCatalogKey): Set<VersionCatalogKey> {
  const next = loadFavoriteKeys();
  if (next.has(key)) {
    next.delete(key);
  } else {
    next.add(key);
  }
  storeFavoriteKeys(next);
  return next;
}

export function loadPopularity(): Record<VersionCatalogKey, number> {
  return readJson<Record<VersionCatalogKey, number>>(STORAGE_KEYS.popularity, {});
}

export function incrementPopularity(key: VersionCatalogKey, by = 1): void {
  const next = loadPopularity();
  next[key] = Math.max(0, (next[key] ?? 0) + by);
  writeJson(STORAGE_KEYS.popularity, next);
}

export function loadOfflineUsername(): string | null {
  return readString(STORAGE_KEYS.offlineUsername);
}

export function storeOfflineUsername(username: string): void {
  localStorage.setItem(STORAGE_KEYS.offlineUsername, username);
}

export function loadOfflineSkinDataUrl(): string | null {
  return readString(STORAGE_KEYS.offlineSkinDataUrl);
}

export function storeOfflineSkinDataUrl(dataUrl: string | null): boolean {
  try {
    if (!dataUrl) {
      localStorage.removeItem(STORAGE_KEYS.offlineSkinDataUrl);
      return true;
    }
    localStorage.setItem(STORAGE_KEYS.offlineSkinDataUrl, dataUrl);
    return true;
  } catch {
    return false;
  }
}
