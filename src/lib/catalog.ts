import type {
  MinecraftVersionSummary,
  OptifineInstallOption,
  ReduxInstallOption,
  VanillaRelease
} from "./types";

export type CatalogKind = "vanilla" | "optifine" | "redux" | "local";
export type CatalogFilter =
  | "installed"
  | "all"
  | "popular"
  | "favorites"
  | "optifine"
  | "redux"
  | "vanilla";
export type CatalogKey = `${CatalogKind}:${string}`;

export type CatalogItem = {
  key: CatalogKey;
  id: string;
  kind: CatalogKind;
  title: string;
  subtitle: string;
  installed: boolean;
  installedVersionId?: string;
  requiredJavaMajor?: number | null;
  sourceUrl?: string | null;
};

export function recommendedJavaMajorForMinecraft(version: string): number | null {
  const parts = version
    .split(".")
    .map((segment) => Number(segment))
    .filter((segment) => Number.isFinite(segment));

  const major = parts[0] ?? 0;
  const minor = parts[1] ?? 0;
  if (major !== 1 || minor === 0) {
    return null;
  }

  if (minor >= 21) {
    return 21;
  }
  if (minor >= 18) {
    return 17;
  }
  if (minor >= 17) {
    return 16;
  }
  return 8;
}

export function buildCatalogItems(
  installedVersions: MinecraftVersionSummary[],
  releases: VanillaRelease[],
  optifineOptions: OptifineInstallOption[],
  reduxOptions: ReduxInstallOption[]
): CatalogItem[] {
  const installedById = new Map(installedVersions.map((version) => [version.id, version]));
  const items: CatalogItem[] = [];

  for (const release of releases) {
    const key: CatalogKey = `vanilla:${release.id}`;
    items.push({
      key,
      id: release.id,
      kind: "vanilla",
      title: `Minecraft ${release.id}`,
      subtitle: "Vanilla",
      installed: installedById.has(release.id),
      installedVersionId: release.id,
      requiredJavaMajor: recommendedJavaMajorForMinecraft(release.id)
    });
  }

  for (const option of optifineOptions) {
    const key: CatalogKey = `optifine:${option.id}`;
    items.push({
      key,
      id: option.id,
      kind: "optifine",
      title: option.title,
      subtitle: `${option.optifineVersion} · ${option.releaseKind}`,
      installed: installedById.has(option.versionId),
      installedVersionId: option.versionId,
      requiredJavaMajor: option.recommendedJavaMajor,
      sourceUrl: option.sourceUrl
    });
  }

  for (const option of reduxOptions) {
    const key: CatalogKey = `redux:${option.id}`;
    items.push({
      key,
      id: option.id,
      kind: "redux",
      title: option.title,
      subtitle: "Fabric · Redux",
      installed: installedById.has(option.versionId),
      installedVersionId: option.versionId,
      requiredJavaMajor: option.recommendedJavaMajor
    });
  }

  for (const version of installedVersions) {
    const alreadyListed =
      items.some((item) => item.installedVersionId === version.id) ||
      items.some((item) => item.kind === "vanilla" && item.id === version.id);
    if (alreadyListed) {
      continue;
    }

    const inferredKind: CatalogKind = version.sourceKind === "redux" ? "redux" : "local";
    const key: CatalogKey = `${inferredKind}:${version.id}`;
    items.push({
      key,
      id: version.id,
      kind: inferredKind,
      title: `Minecraft ${version.id}`,
      subtitle: inferredKind === "redux" ? "Fabric · Redux" : "Local",
      installed: true,
      installedVersionId: version.id,
      requiredJavaMajor: version.javaMajorVersion ?? null
    });
  }

  return items;
}

export function filterCatalogItems(
  items: CatalogItem[],
  filter: CatalogFilter,
  favoriteKeys: Set<CatalogKey>,
  popularity: Record<CatalogKey, number>
): CatalogItem[] {
  const kindRank: Record<CatalogKind, number> = {
    vanilla: 0,
    optifine: 1,
    redux: 2,
    local: 3
  };

  return items
    .map((item, index) => ({ item, index }))
    .filter(({ item }) => {
      switch (filter) {
        case "installed":
          return item.installed;
        case "favorites":
          return favoriteKeys.has(item.key);
        case "optifine":
          return item.kind === "optifine";
        case "redux":
          return item.kind === "redux";
        case "vanilla":
          return item.kind === "vanilla";
        case "popular":
        case "all":
        default:
          return true;
      }
    })
    .sort((left, right) => {
      if (filter === "popular") {
        const scoreDifference = (popularity[right.item.key] ?? 0) - (popularity[left.item.key] ?? 0);
        if (scoreDifference !== 0) {
          return scoreDifference;
        }
      }

      if (left.item.installed !== right.item.installed) {
        return left.item.installed ? -1 : 1;
      }

      if (left.item.kind !== right.item.kind) {
        return kindRank[left.item.kind] - kindRank[right.item.kind];
      }

      return left.index - right.index;
    })
    .map(({ item }) => item);
}

export function findCatalogItemForInstalledVersion(
  items: CatalogItem[],
  selectedVersionId: string
): CatalogItem | null {
  return (
    items.find(
      (item) => item.installed && item.installedVersionId === selectedVersionId
    ) ?? null
  );
}
