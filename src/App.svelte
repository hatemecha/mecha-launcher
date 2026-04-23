<script lang="ts">
  import { onMount, tick } from "svelte";

  import brandMarkUrl from "../hatecreeper2.png?url";

  import {
    autoInstallGraphicsDependency,
    autoInstallJava,
    browseMinecraftDir,
    checkGraphicsDependency,
    checkJavaDependency,
    deleteInstalledVersion,
    detectDefaultMinecraftDir,
    ensureVersionsDir,
    installOptifineVersion,
    installVanillaVersion,
    launchVersion,
    listOptifineInstallOptions,
    listVanillaReleases,
    listVersions,
    onLauncherLog,
    onLauncherStatus,
    onOptifineInstallStatus,
    onVanillaInstallStatus
  } from "./lib/tauri";
  import {
    clearSelectedVersionId,
    incrementPopularity,
    loadStoredMinecraftDir,
    loadStoredVersionId,
    loadFavoriteKeys,
    loadPopularity,
    storeMinecraftDir,
    storeSelectedVersionId,
    toggleFavoriteKey
  } from "./lib/storage";
  import CatScene from "./lib/CatScene.svelte";
  import {
    applyDocumentLang,
    fillTemplate,
    persistLocale,
    readStoredLocale,
    translate,
    type Locale,
    type MessageKey
  } from "./lib/i18n";
  import type {
    LauncherLogEvent,
    LauncherStatusEvent,
    MinecraftVersionSummary,
    OptifineInstallOption,
    OptifineInstallStatusEvent,
    VanillaInstallStatusEvent,
    VanillaRelease
  } from "./lib/types";

  const FIXED_USERNAME = "Player";
  const THEME_STORAGE_KEY = "mecha-launcher.themeMode";
  const MAX_LOG_LINES = 250;

  let locale: Locale = readStoredLocale();

  type LogLine = {
    launchId: string;
    source: "stdout" | "stderr" | "system";
    line: string;
  };

  type ThemeMode = "light" | "dark";

  let minecraftDir = "";
  let detectedMinecraftDir = "";
  let versions: MinecraftVersionSummary[] = [];
  let optifineOptions: OptifineInstallOption[] = [];
  let vanillaReleases: VanillaRelease[] = [];
  let selectedVersionId = "";
  let errorMessage = "";
  let statusMessage = translate(locale, "statusWaitingDir");
  let isLoadingVersions = false;
  let isEnsuringVersionsDir = false;
  let isInstallingJava = false;
  let isInstallingGraphicsDependency = false;
  let javaStatus:
    | {
        installed: boolean;
        detectedMajor?: number | null;
        suggestedLinuxCommands?: string[] | null;
        suggestedWindowsLinks?: { label: string; url: string }[] | null;
        canAutoInstall?: boolean | null;
        autoInstallHint?: string | null;
        recommendedMajor?: number | null;
      }
    | null = null;
  let graphicsStatus:
    | {
        required: boolean;
        installed: boolean;
        usable: boolean;
        detectedRaw?: string | null;
        suggestedLinuxCommands?: string[] | null;
        canAutoInstall?: boolean | null;
        autoInstallHint?: string | null;
      }
    | null = null;
  let isLaunching = false;
  let installingOptifineOptionId: string | null = null;
  let installingOptifineVersionId: string | null = null;
  let installProgress: OptifineInstallStatusEvent | null = null;
  let installingVanillaVersionId: string | null = null;
  let vanillaInstallProgress: VanillaInstallStatusEvent | null = null;
  let lastLoggedInstallStage = "";
  let lastLoggedVanillaInstallStage = "";
  let activeLaunchId: string | null = null;
  let logLines: LogLine[] = [];
  let logViewport: HTMLDivElement | null = null;
  let copiedLog = false;
  let themeMode: ThemeMode = "dark";
  let versionFilter:
    | "installed"
    | "all"
    | "popular"
    | "favorites"
    | "optifine"
    | "vanilla" = "installed";

  $: needsJavaAttention = Boolean(
    javaStatus &&
      (!javaStatus.installed ||
        (javaStatus.recommendedMajor &&
          (!javaStatus.detectedMajor || javaStatus.detectedMajor < javaStatus.recommendedMajor)))
  );

  $: needsGraphicsAttention = Boolean(
    graphicsStatus &&
      graphicsStatus.required &&
      (!graphicsStatus.installed || !graphicsStatus.usable)
  );

  $: showDependencyWarnings = needsJavaAttention || needsGraphicsAttention;

  $: t = (key: MessageKey) => translate(locale, key);

  function setLocale(next: Locale): void {
    if (next === locale) {
      return;
    }

    locale = next;
    persistLocale(next);
    applyDocumentLang(next);
  }

  async function appendLog(source: LogLine["source"], line: string, launchId = "system") {
    const lastLine = logLines[logLines.length - 1];
    if (
      lastLine &&
      lastLine.source === source &&
      lastLine.line === line &&
      lastLine.launchId === launchId
    ) {
      return;
    }

    logLines = [...logLines, { source, line, launchId }].slice(-MAX_LOG_LINES);
    await tick();
    if (logViewport) {
      logViewport.scrollTop = logViewport.scrollHeight;
    }
  }

  async function copyLog(): Promise<void> {
    if (logLines.length === 0) {
      return;
    }

    const serializedLog = logLines
      .map((entry) => `[${formatSourceLabel(entry.source)}] ${entry.line}`)
      .join("\n");

    try {
      await navigator.clipboard.writeText(serializedLog);
      copiedLog = true;
      window.setTimeout(() => {
        copiedLog = false;
      }, 1600);
    } catch (error) {
      errorMessage = getErrorMessage(error, t("errCopyLog"));
      await appendLog("system", errorMessage);
    }
  }

  function getErrorMessage(error: unknown, fallbackMessage: string): string {
    if (typeof error === "string") {
      return error;
    }

    if (error instanceof Error) {
      return error.message;
    }

    if (
      error &&
      typeof error === "object" &&
      "message" in error &&
      typeof error.message === "string"
    ) {
      return error.message;
    }

    return fallbackMessage;
  }

  function isThemeMode(value: string | null): value is ThemeMode {
    return value === "light" || value === "dark";
  }

  function extractMissingVersionsDirPath(message: string): string | null {
    const patterns = [
      /Missing versions directory:\s*(.+)\s*$/i,
      /Missing required Minecraft directory:\s*(.+)\s*$/i
    ];

    for (const pattern of patterns) {
      const match = message.match(pattern);
      const rawPath = match?.[1]?.trim();
      if (!rawPath) {
        continue;
      }

      const normalized = rawPath.replace(/\\/g, "/").replace(/\/+$/, "");
      if (normalized.toLowerCase().endsWith("/versions")) {
        return rawPath;
      }
    }

    return null;
  }

  $: missingVersionsDirPath = errorMessage ? extractMissingVersionsDirPath(errorMessage) : null;

  function readStoredThemeMode(): ThemeMode | null {
    const storedThemeMode = localStorage.getItem(THEME_STORAGE_KEY);
    return isThemeMode(storedThemeMode) ? storedThemeMode : null;
  }

  function detectPreferredThemeMode(): ThemeMode {
    return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  }

  function applyThemeMode(nextThemeMode: ThemeMode): void {
    themeMode = nextThemeMode;
    document.documentElement.dataset.theme = nextThemeMode;
  }

  function toggleThemeMode(): void {
    const nextThemeMode = themeMode === "dark" ? "light" : "dark";
    applyThemeMode(nextThemeMode);
    localStorage.setItem(THEME_STORAGE_KEY, nextThemeMode);
  }

  async function hydrateMinecraftDir(): Promise<void> {
    const storedDir = loadStoredMinecraftDir();
    const detectedDir = await detectDefaultMinecraftDir();

    detectedMinecraftDir = detectedDir ?? "";
    minecraftDir = storedDir ?? detectedMinecraftDir;
  }

  async function refreshDependencies(): Promise<void> {
    try {
      javaStatus = await checkJavaDependency();
      graphicsStatus = await checkGraphicsDependency();
    } catch (error) {
      await appendLog("system", getErrorMessage(error, "Failed to check dependencies."));
    }
  }

  async function handleAutoInstallJava(): Promise<void> {
    if (isInstallingJava) {
      return;
    }

    isInstallingJava = true;
    errorMessage = "";
    statusMessage = t("depsJavaInstalling");

    try {
      const result = await autoInstallJava();
      if (!result.ok) {
        const combined = [result.stdout, result.stderr].filter(Boolean).join("\n").trim();
        errorMessage = combined || t("depsJavaInstallFailed");
        await appendLog("system", errorMessage);
      } else {
        await appendLog("system", t("depsJavaInstalledOk"));
      }
    } catch (error) {
      errorMessage = getErrorMessage(error, t("depsJavaInstallFailed"));
      await appendLog("system", errorMessage);
    } finally {
      isInstallingJava = false;
      await refreshDependencies();
    }
  }

  async function handleAutoInstallGraphicsDependency(): Promise<void> {
    if (isInstallingGraphicsDependency) {
      return;
    }

    isInstallingGraphicsDependency = true;
    errorMessage = "";
    statusMessage = t("depsGraphicsInstalling");

    try {
      const result = await autoInstallGraphicsDependency();
      if (!result.ok) {
        const combined = [result.stdout, result.stderr].filter(Boolean).join("\n").trim();
        errorMessage = combined || t("depsGraphicsInstallFailed");
        await appendLog("system", errorMessage);
      } else {
        await appendLog("system", t("depsGraphicsInstalledOk"));
        statusMessage = t("depsGraphicsInstalledOk");
      }
    } catch (error) {
      errorMessage = getErrorMessage(error, t("depsGraphicsInstallFailed"));
      await appendLog("system", errorMessage);
    } finally {
      isInstallingGraphicsDependency = false;
      await refreshDependencies();
      if (!errorMessage && graphicsStatus?.installed) {
        statusMessage = graphicsStatus.usable ? t("depsGraphicsOk") : t("depsGraphicsInstalledOk");
      }
    }
  }

  function syncSelectedVersion(nextVersions: MinecraftVersionSummary[]): void {
    const storedVersionId = loadStoredVersionId();
    const nextSelected =
      nextVersions.find((version) => version.id === selectedVersionId)?.id ??
      nextVersions.find((version) => version.id === storedVersionId)?.id ??
      nextVersions[0]?.id ??
      "";

    selectedVersionId = nextSelected;

    if (nextSelected) {
      storeSelectedVersionId(nextSelected);
    } else {
      clearSelectedVersionId();
    }
  }

  async function refreshVersions(): Promise<void> {
    const trimmedDir = minecraftDir.trim();

    errorMessage = "";
    isLoadingVersions = true;

    if (!trimmedDir) {
      versions = [];
      selectedVersionId = "";
      errorMessage = translate(locale, "errSetDirBeforeReload");
      statusMessage = translate(locale, "statusWaitingDir");
      isLoadingVersions = false;
      return;
    }

    try {
      const nextVersions = await listVersions(trimmedDir);
      minecraftDir = trimmedDir;
      storeMinecraftDir(trimmedDir);
      versions = nextVersions;
      syncSelectedVersion(nextVersions);

      if (nextVersions.length === 0) {
        statusMessage = translate(locale, "statusNoVersionsInDir");
      } else if (nextVersions.length === 1) {
        statusMessage = translate(locale, "versionsReadyOne");
      } else {
        statusMessage = fillTemplate(translate(locale, "versionsReadyMany"), {
          count: nextVersions.length
        });
      }

      await appendLog(
        "system",
        fillTemplate(translate(locale, "logReloadedVersions"), { path: trimmedDir })
      );
    } catch (error) {
      versions = [];
      selectedVersionId = "";
      clearSelectedVersionId();
      errorMessage = getErrorMessage(error, translate(locale, "errReadVersions"));
      statusMessage = translate(locale, "statusDirInvalid");
      await appendLog("system", errorMessage);
    } finally {
      isLoadingVersions = false;
    }
  }

  async function handleBrowse(): Promise<void> {
    try {
      const pickedPath = await browseMinecraftDir();
      if (!pickedPath) {
        return;
      }

      minecraftDir = pickedPath;
      await refreshVersions();
    } catch (error) {
      errorMessage = getErrorMessage(error, translate(locale, "errFolderPicker"));
      await appendLog("system", errorMessage);
    }
  }

  async function handleCreateVersionsDir(): Promise<void> {
    const baseDir = minecraftDir.trim() || detectedMinecraftDir.trim();
    if (!baseDir || isEnsuringVersionsDir) {
      return;
    }

    isEnsuringVersionsDir = true;

    try {
      const createdPath = await ensureVersionsDir(baseDir);
      await appendLog(
        "system",
        fillTemplate(translate(locale, "versionsDirCreated"), { path: createdPath })
      );
      await refreshVersions();
    } catch (error) {
      errorMessage = getErrorMessage(error, translate(locale, "errCreateVersionsDir"));
      await appendLog("system", errorMessage);
    } finally {
      isEnsuringVersionsDir = false;
    }
  }

  function isOptifineInstalled(option: OptifineInstallOption): boolean {
    return versions.some((version) => version.id === option.versionId);
  }

  function progressPercent(progress: OptifineInstallStatusEvent | null): number | null {
    if (!progress?.current || !progress.total) {
      return null;
    }

    return Math.min(100, Math.round((progress.current / progress.total) * 100));
  }

  async function handleInstallOptifine(option: OptifineInstallOption): Promise<void> {
    if (installingOptifineOptionId || isLaunching) {
      return;
    }

    const targetDir = minecraftDir.trim() || detectedMinecraftDir.trim();
    if (!targetDir) {
      errorMessage = t("optifineInstallNeedDir");
      await appendLog("system", errorMessage);
      return;
    }

    minecraftDir = targetDir;
    storeMinecraftDir(targetDir);
    installingOptifineOptionId = option.id;
    installingOptifineVersionId = option.versionId;
    if (selectedVersionId === option.versionId) {
      selectedVersionId = "";
      clearSelectedVersionId();
    }
    installProgress = {
      optionId: option.id,
      stage: "queued",
      message: t("optifineInstalling")
    };
    lastLoggedInstallStage = "";
    errorMessage = "";
    statusMessage = t("optifineInstalling");

    try {
      const result = await installOptifineVersion({
        minecraftDir: targetDir,
        optionId: option.id
      });
      await refreshVersions();
      selectedVersionId = result.versionId;
      storeSelectedVersionId(result.versionId);
      statusMessage = fillTemplate(t("optifineInstallDone"), { id: result.versionId });
      await appendLog("system", statusMessage);
    } catch (error) {
      errorMessage = getErrorMessage(error, t("optifineInstallFailed"));
      statusMessage = t("optifineInstallFailed");
      await appendLog("system", errorMessage);
    } finally {
      installingOptifineOptionId = null;
      installingOptifineVersionId = null;
      installProgress = null;
    }
  }

  async function handlePlay(): Promise<void> {
    if (!canPlay) {
      if (installingOptifineOptionId) {
        errorMessage = t("errInstallInProgress");
        await appendLog("system", errorMessage);
      }
      return;
    }

    errorMessage = "";

    try {
      const response = await launchVersion({
        minecraftDir: minecraftDir.trim(),
        versionId: selectedVersionId,
        username: FIXED_USERNAME
      });

      activeLaunchId = response.launchId;
      isLaunching = true;
      statusMessage = translate(locale, "statusPreparingLaunch");
      const optifineMatch = optifineOptions.find((option) => option.versionId === selectedVersionId);
      const popKey = (optifineMatch ? `optifine:${optifineMatch.id}` : `vanilla:${selectedVersionId}`) as const;
      incrementPopularity(popKey);
      popularity = loadPopularity();
      await appendLog(
        "system",
        fillTemplate(translate(locale, "logLaunchRequested"), { id: selectedVersionId }),
        response.launchId
      );
    } catch (error) {
      errorMessage = getErrorMessage(error, translate(locale, "errFailedLaunch"));
      await appendLog("system", errorMessage);
    }
  }

  function handleStatus(event: LauncherStatusEvent): void {
    if (activeLaunchId && event.launchId !== activeLaunchId) {
      return;
    }

    if (event.state === "launching" || event.state === "running") {
      isLaunching = true;
    } else {
      isLaunching = false;
    }

    activeLaunchId = event.launchId;

    if (event.state === "error") {
      errorMessage = event.message ?? translate(locale, "errLaunchFailed");
    }

    statusMessage =
      event.message ?? fillTemplate(translate(locale, "launchState"), { state: event.state });
  }

  async function handleLog(event: LauncherLogEvent): Promise<void> {
    if (activeLaunchId && event.launchId !== activeLaunchId) {
      return;
    }

    await appendLog(event.source, event.line, event.launchId);
  }

  async function handleOptifineStatus(event: OptifineInstallStatusEvent): Promise<void> {
    if (installingOptifineOptionId && event.optionId !== installingOptifineOptionId) {
      return;
    }

    installProgress = event;
    statusMessage = event.message;

    const shouldLogInstallStatus =
      event.stage !== lastLoggedInstallStage ||
      event.stage === "done" ||
      (event.current !== null &&
        event.current !== undefined &&
        event.total !== null &&
        event.total !== undefined &&
        event.current === event.total) ||
      (!event.current && !event.total);

    if (shouldLogInstallStatus) {
      lastLoggedInstallStage = event.stage;
      await appendLog("system", event.message);
    }
  }

  async function handleVanillaStatus(event: VanillaInstallStatusEvent): Promise<void> {
    if (installingVanillaVersionId && event.versionId !== installingVanillaVersionId) {
      return;
    }

    vanillaInstallProgress = event;
    statusMessage = event.message;

    const shouldLogInstallStatus =
      event.stage !== lastLoggedVanillaInstallStage ||
      event.stage === "done" ||
      (event.current !== null &&
        event.current !== undefined &&
        event.total !== null &&
        event.total !== undefined &&
        event.current === event.total) ||
      (!event.current && !event.total);

    if (shouldLogInstallStatus) {
      lastLoggedVanillaInstallStage = event.stage;
      await appendLog("system", event.message);
    }
  }

  function formatSourceLabel(source: LogLine["source"]): string {
    switch (source) {
      case "stderr":
        return t("logSourceErr");
      case "stdout":
        return t("logSourceOut");
      default:
        return t("logSourceSys");
    }
  }

  function handleVersionSelection(versionId: string): void {
    selectedVersionId = versionId;
    storeSelectedVersionId(versionId);
  }

  $: selectedVersion = versions.find((version) => version.id === selectedVersionId) ?? null;
  $: canPlay = Boolean(
    minecraftDir.trim() &&
      selectedVersionId &&
      selectedVersion &&
      !isLaunching &&
      !installingOptifineOptionId &&
      !installingVanillaVersionId &&
      selectedVersionId !== installingOptifineVersionId
  );
  $: installPercent = progressPercent(installProgress);

  type CatalogKind = "vanilla" | "optifine";
  type CatalogKey = `${CatalogKind}:${string}`;
  type CatalogItem = {
    key: CatalogKey;
    id: string;
    kind: CatalogKind;
    title: string;
    subtitle: string;
    installed: boolean;
    installedVersionId?: string;
    recommendedJavaMajor?: number | null;
    sourceUrl?: string | null;
  };

  let favoriteKeys = loadFavoriteKeys();
  let popularity = loadPopularity();

  $: catalogItems = buildCatalogItems(versions, vanillaReleases, optifineOptions);
  $: filteredCatalogItems = filterCatalogItems(catalogItems, versionFilter);

  function filterCatalogItems(items: CatalogItem[], filter: typeof versionFilter): CatalogItem[] {
    const favorites = favoriteKeys;
    const popularityMap = popularity;

    const sortNewestFirst = (a: CatalogItem, b: CatalogItem) => {
      // Vanilla releases are provided newest-first from the server.
      // Keep that order for vanilla; otherwise stable sort below.
      return 0;
    };

    const withRank = items.map((item, index) => ({ item, index }));

    const base = withRank
      .filter(({ item }) => {
        switch (filter) {
          case "installed":
            return item.installed;
          case "favorites":
            return favorites.has(item.key);
          case "optifine":
            return item.kind === "optifine";
          case "vanilla":
            return item.kind === "vanilla";
          case "popular":
          case "all":
          default:
            return true;
        }
      })
      .sort((a, b) => {
        if (filter === "popular") {
          const aScore = popularityMap[a.item.key] ?? 0;
          const bScore = popularityMap[b.item.key] ?? 0;
          if (aScore !== bScore) {
            return bScore - aScore;
          }
        }
        if (a.item.kind !== b.item.kind) {
          return a.item.kind === "vanilla" ? -1 : 1;
        }
        if (a.item.installed !== b.item.installed) {
          return a.item.installed ? -1 : 1;
        }
        const keepOrder = sortNewestFirst(a.item, b.item);
        if (keepOrder !== 0) {
          return keepOrder;
        }
        return a.index - b.index;
      })
      .map(({ item }) => item);

    return base;
  }

  function setVersionFilter(next: typeof versionFilter): void {
    versionFilter = next;
  }

  function isFavorite(item: CatalogItem): boolean {
    return favoriteKeys.has(item.key);
  }

  function toggleFavorite(item: CatalogItem): void {
    favoriteKeys = toggleFavoriteKey(item.key);
  }

  function selectCatalogItem(item: CatalogItem): void {
    if (!item.installed || !item.installedVersionId) {
      return;
    }
    handleVersionSelection(item.installedVersionId);
  }

  async function handleDownload(item: CatalogItem): Promise<void> {
    if (isLaunching || installingOptifineOptionId || installingVanillaVersionId) {
      return;
    }

    const targetDir = minecraftDir.trim() || detectedMinecraftDir.trim();
    if (!targetDir) {
      errorMessage = translate(locale, "optifineInstallNeedDir");
      await appendLog("system", errorMessage);
      return;
    }

    minecraftDir = targetDir;
    storeMinecraftDir(targetDir);
    errorMessage = "";

    if (item.kind === "optifine") {
      const option = optifineOptions.find((opt) => opt.id === item.id);
      if (!option) {
        errorMessage = "Unknown OptiFine option.";
        await appendLog("system", errorMessage);
        return;
      }
      incrementPopularity(item.key);
      await handleInstallOptifine(option);
      popularity = loadPopularity();
      return;
    }

    installingVanillaVersionId = item.id;
    vanillaInstallProgress = {
      versionId: item.id,
      stage: "queued",
      message: t("versionDownloading")
    };
    lastLoggedVanillaInstallStage = "";
    statusMessage = t("versionDownloading");
    incrementPopularity(item.key);
    popularity = loadPopularity();

    try {
      const result = await installVanillaVersion({
        minecraftDir: targetDir,
        versionId: item.id
      });
      await refreshVersions();
      selectedVersionId = result.versionId;
      storeSelectedVersionId(result.versionId);
      await appendLog("system", `Minecraft ${result.versionId} instalado.`);
      vanillaInstallProgress = null;
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to install vanilla version.");
      statusMessage = errorMessage;
      await appendLog("system", errorMessage);
    } finally {
      installingVanillaVersionId = null;
    }
  }

  async function handleDelete(item: CatalogItem): Promise<void> {
    if (!item.installed || !item.installedVersionId) {
      return;
    }
    if (isLaunching || installingOptifineOptionId || installingVanillaVersionId) {
      return;
    }

    const targetDir = minecraftDir.trim() || detectedMinecraftDir.trim();
    if (!targetDir) {
      errorMessage = translate(locale, "optifineInstallNeedDir");
      await appendLog("system", errorMessage);
      return;
    }

    const versionId = item.installedVersionId;
    const optifineToDelete =
      item.kind === "vanilla"
        ? optifineOptions
            .filter((option) => option.minecraftVersion === item.id)
            .filter((option) => versions.some((v) => v.id === option.versionId))
            .map((option) => option.versionId)
        : [];

    const confirmMessage =
      optifineToDelete.length > 0
        ? fillTemplate(t("versionDeleteConfirmWithOptifine"), {
            id: versionId,
            optifine: optifineToDelete.join(", ")
          })
        : fillTemplate(t("versionDeleteConfirm"), { id: versionId });

    const confirmed = window.confirm(confirmMessage);
    if (!confirmed) {
      return;
    }

    statusMessage = t("versionDeleting");
    try {
      await deleteInstalledVersion({ minecraftDir: targetDir, versionId });
      for (const optifineVersionId of optifineToDelete) {
        await deleteInstalledVersion({ minecraftDir: targetDir, versionId: optifineVersionId });
      }
      if (selectedVersionId === versionId) {
        selectedVersionId = "";
        clearSelectedVersionId();
      }
      await refreshVersions();
      await appendLog("system", `Versión borrada: ${versionId}`);
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to delete version.");
      await appendLog("system", errorMessage);
    }
  }

  function buildCatalogItems(
    installedVersions: MinecraftVersionSummary[],
    releases: VanillaRelease[],
    optifine: OptifineInstallOption[]
  ): CatalogItem[] {
    const installedById = new Map(installedVersions.map((v) => [v.id, v]));
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
        installedVersionId: release.id
      });
    }

    for (const option of optifine) {
      const key: CatalogKey = `optifine:${option.id}`;
      items.push({
        key,
        id: option.id,
        kind: "optifine",
        title: option.title,
        subtitle: `${option.optifineVersion} · ${option.releaseKind}`,
        installed: installedById.has(option.versionId),
        installedVersionId: option.versionId,
        recommendedJavaMajor: option.recommendedJavaMajor,
        sourceUrl: option.sourceUrl
      });
    }

    // Add locally installed versions that are not present in the catalog (offline/unknown/custom).
    for (const version of installedVersions) {
      const alreadyListed =
        items.some((item) => item.installedVersionId === version.id) ||
        items.some((item) => item.kind === "vanilla" && item.id === version.id);
      if (alreadyListed) {
        continue;
      }
      const key: CatalogKey = `vanilla:${version.id}`;
      items.push({
        key,
        id: version.id,
        kind: "vanilla",
        title: `Minecraft ${version.id}`,
        subtitle: "Local",
        installed: true,
        installedVersionId: version.id
      });
    }

    return items;
  }

  onMount(() => {
    let unlistenStatus = () => undefined;
    let unlistenLog = () => undefined;
    let unlistenOptifineInstall = () => undefined;
    let unlistenVanillaInstall = () => undefined;
    let removeThemePreferenceListener = () => undefined;

    applyThemeMode(readStoredThemeMode() ?? detectPreferredThemeMode());

    const themePreference = window.matchMedia("(prefers-color-scheme: light)");
    const handleThemePreferenceChange = (event: MediaQueryListEvent) => {
      if (!readStoredThemeMode()) {
        applyThemeMode(event.matches ? "light" : "dark");
      }
    };

    themePreference.addEventListener("change", handleThemePreferenceChange);
    removeThemePreferenceListener = () => {
      themePreference.removeEventListener("change", handleThemePreferenceChange);
    };

    void (async () => {
      unlistenStatus = await onLauncherStatus(handleStatus);
      unlistenLog = await onLauncherLog((event) => {
        void handleLog(event);
      });
      unlistenOptifineInstall = await onOptifineInstallStatus((event) => {
        void handleOptifineStatus(event);
      });
      unlistenVanillaInstall = await onVanillaInstallStatus((event) => {
        void handleVanillaStatus(event);
      });

      try {
        optifineOptions = await listOptifineInstallOptions();
        try {
          vanillaReleases = await listVanillaReleases();
        } catch (error) {
          vanillaReleases = [];
          await appendLog("system", getErrorMessage(error, "Failed to load vanilla release catalog."));
        }
        await refreshDependencies();
        await hydrateMinecraftDir();
        if (minecraftDir) {
          await refreshVersions();
        }
      } catch (error) {
        errorMessage = getErrorMessage(error, translate(locale, "errInitLauncher"));
        await appendLog("system", errorMessage);
      }
    })();

    return () => {
      unlistenStatus();
      unlistenLog();
      unlistenOptifineInstall();
      unlistenVanillaInstall();
      removeThemePreferenceListener();
    };
  });
</script>

<svelte:head>
  <title>{t("appTitle")}</title>
</svelte:head>

<svg class="icon-sprite" aria-hidden="true" focusable="false">
  <symbol id="icon-folder" viewBox="0 0 24 24">
    <path d="M3.5 6.5h6l2 2h9v9a2 2 0 0 1-2 2h-15z" />
    <path d="M3.5 6.5v11a2 2 0 0 0 2 2" />
  </symbol>
  <symbol id="icon-refresh" viewBox="0 0 24 24">
    <path d="M20 12a8 8 0 0 1-13.7 5.7" />
    <path d="M4 12A8 8 0 0 1 17.7 6.3" />
    <path d="M17.7 3.7v2.6h-2.6" />
    <path d="M6.3 20.3v-2.6h2.6" />
  </symbol>
  <symbol id="icon-play" viewBox="0 0 24 24">
    <path d="M8 5.5v13l10-6.5z" />
  </symbol>
  <symbol id="icon-list" viewBox="0 0 24 24">
    <path d="M8 7h12" />
    <path d="M8 12h12" />
    <path d="M8 17h12" />
    <path d="M4 7h.01" />
    <path d="M4 12h.01" />
    <path d="M4 17h.01" />
  </symbol>
  <symbol id="icon-terminal" viewBox="0 0 24 24">
    <path d="M4 5h16v14H4z" />
    <path d="m7 9 3 3-3 3" />
    <path d="M12 15h5" />
  </symbol>
  <symbol id="icon-cat" viewBox="0 0 24 24">
    <path d="M6 9V5l3 2h6l3-2v4" />
    <path d="M5.5 10.5v4.2A4.3 4.3 0 0 0 9.8 19h4.4a4.3 4.3 0 0 0 4.3-4.3v-4.2" />
    <path d="M9 12h.01" />
    <path d="M15 12h.01" />
    <path d="M11 15h2" />
  </symbol>
  <symbol id="icon-user" viewBox="0 0 24 24">
    <path d="M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8z" />
    <path d="M4.5 20a7.5 7.5 0 0 1 15 0" />
  </symbol>
  <symbol id="icon-theme" viewBox="0 0 24 24">
    <path d="M12 3v2" />
    <path d="M12 19v2" />
    <path d="M4.2 4.2 5.6 5.6" />
    <path d="m18.4 18.4 1.4 1.4" />
    <path d="M3 12h2" />
    <path d="M19 12h2" />
    <path d="m4.2 19.8 1.4-1.4" />
    <path d="m18.4 5.6 1.4-1.4" />
    <circle cx="12" cy="12" r="4" />
  </symbol>
  <symbol id="icon-status" viewBox="0 0 24 24">
    <circle cx="12" cy="12" r="4" />
  </symbol>
  <symbol id="icon-cube" viewBox="0 0 24 24">
    <path d="m12 3 8 4.5v9L12 21l-8-4.5v-9z" />
    <path d="M12 12 4 7.5" />
    <path d="m12 12 8-4.5" />
    <path d="M12 12v9" />
  </symbol>
  <symbol id="icon-download" viewBox="0 0 24 24">
    <path d="M12 3v10" />
    <path d="m8 11 4 4 4-4" />
    <path d="M5 19h14" />
  </symbol>
  <symbol id="icon-trash" viewBox="0 0 24 24">
    <path d="M4 7h16" />
    <path d="M10 11v6" />
    <path d="M14 11v6" />
    <path d="M6 7l1 14h10l1-14" />
    <path d="M9 7V4h6v3" />
  </symbol>
  <symbol id="icon-star" viewBox="0 0 24 24">
    <path d="m12 3 2.7 5.8 6.3.6-4.8 4.1 1.4 6.2L12 16.9 6.4 19.7l1.4-6.2L3 9.4l6.3-.6z" />
  </symbol>
</svg>

<div class="app-shell">
  <header class="header-bar">
    <div class="brand-lockup">
      <img
        class="brand-mark"
        src={brandMarkUrl}
        alt=""
        width="26"
        height="26"
        decoding="async"
      />
      <span class="logo">MECHA <span class="logo-accent">LAUNCHER</span></span>
    </div>

    <button
      class="header-chip theme-toggle"
      type="button"
      on:click={toggleThemeMode}
      aria-label={themeMode === "dark" ? t("themeAriaDark") : t("themeAriaLight")}
      aria-pressed={themeMode === "dark"}
    >
      <svg class="app-icon" aria-hidden="true"><use href="#icon-theme" /></svg>
      <span>{themeMode === "dark" ? t("themeDark") : t("themeLight")}</span>
    </button>

    <div
      class:active={isLaunching || installingOptifineOptionId || installingVanillaVersionId}
      class="status-indicator"
      role="status"
      aria-live="polite"
    >
      <svg class="app-icon status-dot" aria-hidden="true"><use href="#icon-status" /></svg>
      {isLaunching || installingOptifineOptionId || installingVanillaVersionId
        ? t("statusRunning")
        : t("statusIdle")}
    </div>
  </header>

  <div class="app-container">
    <main class="preview-area">
      <section class="launch-stage" aria-labelledby="launcher-title">
        <div class="stage-header">
          <div>
            <h1 id="launcher-title" class="stage-title-lockup">
              <img class="stage-brand-mark" src={brandMarkUrl} alt="" decoding="async" />
              <span class="stage-title-wordmark">MECHA <span class="logo-accent">LAUNCHER</span></span>
            </h1>
            <p class="kicker">{t("kicker")}</p>
          </div>
          <div class="stage-badge">
            <svg class="app-icon" aria-hidden="true"><use href="#icon-user" /></svg>
            <span>{FIXED_USERNAME}</span>
          </div>
        </div>

        <div class="run-summary" aria-label={t("runSummaryAria")}>
          <div>
            <span>{t("runVersion")}</span>
            <strong>{selectedVersionId || "—"}</strong>
          </div>
          <div>
            <span>{t("runInstalled")}</span>
            <strong>{versions.length}</strong>
          </div>
          <div>
            <span>{t("runState")}</span>
            <strong>
              {installingOptifineOptionId || installingVanillaVersionId
                ? t("stateInstalling")
                : isLaunching
                  ? t("stateLaunching")
                  : t("stateReady")}
            </strong>
          </div>
        </div>

        <div class="stage-content split-stage">
          <section class="stage-pane cat-pane" aria-label={t("panePreviewSr")}>
            <div class="pane-title pane-title-icon" title={t("panePreviewTitle")}>
              <svg class="app-icon" aria-hidden="true"><use href="#icon-cat" /></svg>
              <span class="sr-only">{t("panePreviewSr")}</span>
            </div>
            <CatScene {themeMode} sceneAriaLabel={t("catSceneAria")} />
          </section>

          <section class="stage-pane log-pane" aria-label={t("paneOutputSr")}>
            <div class="pane-title pane-title-icon log-title" title={t("paneOutputTitle")}>
              <span class="pane-title-left">
                <svg class="app-icon" aria-hidden="true"><use href="#icon-terminal" /></svg>
                <span class="sr-only">{t("paneOutputSr")}</span>
              </span>
              <button
                class="log-copy-btn"
                type="button"
                on:click={copyLog}
                disabled={logLines.length === 0}
              >
                {copiedLog ? t("copyLogDone") : t("copyLog")}
              </button>
            </div>
            <div bind:this={logViewport} class="log-console" role="log" aria-live="polite">
              {#if logLines.length === 0}
                <p class="log-placeholder">{t("logPlaceholder")}</p>
              {:else}
                {#each logLines as entry}
                  <div class="log-line">
                    <span class="log-source {entry.source}">{formatSourceLabel(entry.source)}</span>
                    <span class="log-message">{entry.line}</span>
                  </div>
                {/each}
              {/if}
            </div>
          </section>
        </div>
      </section>

      <div class="preview-status-bar">
        <span>{selectedVersionId || t("noVersionSelected")}</span>
        <span>{statusMessage}</span>
        <span>{minecraftDir.trim() || t("mcDirNotSet")}</span>
      </div>
    </main>

    <aside class="control-panel" aria-label={t("controlsAria")}>
      {#if showDependencyWarnings}
        <section class="panel-section deps-banner">
          <div class="section-title accent">{t("depsTitle")}</div>

          {#if needsJavaAttention}
            <div class="help-text">
              <strong>{t("depsJavaTitle")}</strong>
              {#if javaStatus?.installed && javaStatus?.detectedMajor && javaStatus.recommendedMajor && javaStatus.detectedMajor < javaStatus.recommendedMajor}
                <div class="status-msg error">
                  {fillTemplate(t("depsJavaNotRecommended"), {
                    major: javaStatus.detectedMajor,
                    recommended: javaStatus.recommendedMajor
                  })}
                </div>
              {:else}
                <div class="status-msg error">{t("depsJavaMissing")}</div>
              {/if}

              <div style="margin-top: 8px;">
                <div><strong>{t("depsJavaHowTo")}</strong></div>
                {#if javaStatus?.canAutoInstall}
                  <div class="btn-row" style="margin-top: 8px;">
                    <button class="btn" type="button" on:click={handleAutoInstallJava} disabled={isInstallingJava}>
                      {isInstallingJava ? t("depsJavaInstalling") : t("depsJavaAutoInstall")}
                    </button>
                  </div>
                  {#if javaStatus?.autoInstallHint}
                    <div class="hint">{javaStatus.autoInstallHint}</div>
                  {/if}
                {/if}
                {#if javaStatus?.suggestedLinuxCommands?.length}
                  <div>{t("depsJavaLinuxNote")}</div>
                  <pre class="log-console" style="min-height: unset; height: auto; padding: 10px; margin: 8px 0 0;">{javaStatus.suggestedLinuxCommands.join("\n")}</pre>
                {/if}
                {#if javaStatus?.suggestedWindowsLinks?.length}
                  <div>{t("depsJavaWindowsNote")}</div>
                  <ul style="margin: 8px 0 0; padding-left: 18px;">
                    {#each javaStatus.suggestedWindowsLinks as link}
                      <li><a href={link.url} target="_blank" rel="noreferrer">{link.label}</a></li>
                    {/each}
                  </ul>
                {/if}
              </div>
            </div>
          {/if}

          {#if needsGraphicsAttention}
            <div class="help-text dependency-subcard">
              <strong>{t("depsGraphicsTitle")}</strong>
              {#if graphicsStatus?.installed && !graphicsStatus?.usable}
                <div class="status-msg error">{t("depsGraphicsNotUsable")}</div>
              {:else}
                <div class="status-msg error">{t("depsGraphicsMissing")}</div>
              {/if}

              <div style="margin-top: 8px;">
                <div><strong>{t("depsJavaHowTo")}</strong></div>
                {#if graphicsStatus?.canAutoInstall}
                  <div class="btn-row" style="margin-top: 8px;">
                    <button
                      class="btn"
                      type="button"
                      on:click={handleAutoInstallGraphicsDependency}
                      disabled={isInstallingGraphicsDependency}
                    >
                      {isInstallingGraphicsDependency
                        ? t("depsGraphicsInstalling")
                        : t("depsGraphicsAutoInstall")}
                    </button>
                  </div>
                  {#if graphicsStatus?.autoInstallHint}
                    <div class="hint">{graphicsStatus.autoInstallHint}</div>
                  {/if}
                {/if}
                {#if graphicsStatus?.suggestedLinuxCommands?.length}
                  <div>{t("depsJavaLinuxNote")}</div>
                  <pre class="log-console" style="min-height: unset; height: auto; padding: 10px; margin: 8px 0 0;">{graphicsStatus.suggestedLinuxCommands.join("\n")}</pre>
                {/if}
              </div>
            </div>
          {/if}
        </section>
      {/if}

      <section class="panel-section">
        <div class="section-title accent">
          <svg class="app-icon" aria-hidden="true"><use href="#icon-folder" /></svg>
          {t("gameDirectory")}
        </div>
        <p class="help-text">{@html t("gameDirectoryHelp")}</p>

        <input
          bind:value={minecraftDir}
          class="text-input"
          type="text"
          placeholder={t("mcPathPlaceholder")}
          aria-label={t("minecraftDirAria")}
        />

        <div class="btn-row">
          <button class="btn" type="button" on:click={handleBrowse}>
            <svg class="app-icon" aria-hidden="true"><use href="#icon-folder" /></svg>
            {t("browse")}
          </button>
          <button class="btn" type="button" on:click={refreshVersions} disabled={isLoadingVersions}>
            <svg class="app-icon" aria-hidden="true"><use href="#icon-refresh" /></svg>
            {isLoadingVersions ? t("reloading") : t("reload")}
          </button>
        </div>

        {#if detectedMinecraftDir}
          <p class="hint">{t("detectedDefault")} {detectedMinecraftDir}</p>
        {/if}

        {#if errorMessage}
          <p class="status-msg error">{errorMessage}</p>
          {#if missingVersionsDirPath}
            <div class="btn-row">
              <button
                class="btn"
                type="button"
                on:click={handleCreateVersionsDir}
                disabled={isEnsuringVersionsDir}
              >
                {isEnsuringVersionsDir ? t("creatingVersionsDir") : t("createVersionsDir")}
              </button>
            </div>
          {/if}
        {:else}
          <p class="status-msg info">{statusMessage}</p>
        {/if}
      </section>

      <section class="panel-section versions-section">
        <div class="section-title accent">
          <svg class="app-icon" aria-hidden="true"><use href="#icon-list" /></svg>
          {t("versionsCatalogTitle")}
        </div>

        <div class="version-filters" role="tablist" aria-label={t("versionsCatalogTitle")}>
          <button class:active={versionFilter === "installed"} class="chip" type="button" on:click={() => setVersionFilter("installed")}>
            {t("versionsFilterInstalled")}
          </button>
          <button class:active={versionFilter === "all"} class="chip" type="button" on:click={() => setVersionFilter("all")}>
            {t("versionsFilterAll")}
          </button>
          <button class:active={versionFilter === "popular"} class="chip" type="button" on:click={() => setVersionFilter("popular")}>
            {t("versionsFilterPopular")}
          </button>
          <button class:active={versionFilter === "favorites"} class="chip" type="button" on:click={() => setVersionFilter("favorites")}>
            {t("versionsFilterFavorites")}
          </button>
          <button class:active={versionFilter === "vanilla"} class="chip" type="button" on:click={() => setVersionFilter("vanilla")}>
            {t("versionsFilterVanilla")}
          </button>
          <button class:active={versionFilter === "optifine"} class="chip" type="button" on:click={() => setVersionFilter("optifine")}>
            {t("versionsFilterOptifine")}
          </button>
        </div>

        <div class="version-list" role="list" aria-label={t("versionsListAria")}>
          {#if filteredCatalogItems.length === 0}
            <div class="empty-state">
              <p>{t("versionsEmptyTitle")}</p>
              <span>{@html t("versionsEmptyDetail")}</span>
            </div>
          {:else}
            {#each filteredCatalogItems as item (item.key)}
              {@const selected = item.installedVersionId === selectedVersionId}
              {@const busy = Boolean(isLaunching || installingOptifineOptionId || installingVanillaVersionId)}
              <div class:selected class="catalog-item" role="listitem">
                <button
                  class="catalog-main"
                  type="button"
                  on:click={() => selectCatalogItem(item)}
                  disabled={!item.installed || busy}
                >
                  <span class="version-name">{item.title}</span>
                  <span class="version-meta">
                    {item.subtitle}
                    {#if item.kind === "optifine" && item.recommendedJavaMajor}
                      · Java {item.recommendedJavaMajor}
                    {/if}
                    {#if item.installed}
                      · {t("optifineInstalled")}
                    {/if}
                  </span>
                </button>

                <div class="catalog-actions">
                  <button
                    class:active={isFavorite(item)}
                    class="icon-btn"
                    type="button"
                    aria-label={t("versionsFilterFavorites")}
                    on:click={() => toggleFavorite(item)}
                    disabled={busy}
                  >
                    <svg class="app-icon" aria-hidden="true"><use href="#icon-star" /></svg>
                  </button>

                  {#if item.installed}
                    <button
                      class="icon-btn"
                      type="button"
                      aria-label={t("versionActionDelete")}
                      on:click={() => handleDelete(item)}
                      disabled={busy}
                    >
                      <svg class="app-icon" aria-hidden="true"><use href="#icon-trash" /></svg>
                    </button>
                  {:else}
                    <button
                      class="icon-btn"
                      type="button"
                      aria-label={t("versionActionDownload")}
                      on:click={() => handleDownload(item)}
                      disabled={busy}
                    >
                      <svg class="app-icon" aria-hidden="true"><use href="#icon-download" /></svg>
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          {/if}
        </div>

        {#if installProgress}
          <div class="install-progress" aria-label={t("optifineProgressAria")} role="status">
            <div class="install-progress-copy">
              <span>{installProgress.stage}</span>
              <strong>{installProgress.message}</strong>
            </div>
            <div class:indeterminate={installPercent === null} class="progress-track">
              <span style={`width: ${installPercent ?? 100}%`}></span>
            </div>
          </div>
        {/if}

        {#if vanillaInstallProgress}
          {@const percent = vanillaInstallProgress.current && vanillaInstallProgress.total
            ? Math.min(100, Math.round((vanillaInstallProgress.current / vanillaInstallProgress.total) * 100))
            : null}
          <div class="install-progress" role="status">
            <div class="install-progress-copy">
              <span>{vanillaInstallProgress.stage}</span>
              <strong>{vanillaInstallProgress.message}</strong>
            </div>
            <div class:indeterminate={percent === null} class="progress-track">
              <span style={`width: ${percent ?? 100}%`}></span>
            </div>
          </div>
        {/if}

        <button class:active={canPlay} class="btn primary play-button" type="button" on:click={handlePlay} disabled={!canPlay}>
          <svg class="app-icon" aria-hidden="true"><use href="#icon-play" /></svg>
          {isLaunching ? t("launching") : t("play")}
        </button>
      </section>

      <section class="panel-section panel-lang">
        <div class="lang-row">
          <span class="lang-label">{t("languageLabel")}</span>
          <div class="lang-toggle" role="group" aria-label={t("languageLabel")}>
            <button
              class:active={locale === "es"}
              class="lang-btn"
              type="button"
              on:click={() => setLocale("es")}
            >
              {t("languageEs")}
            </button>
            <button
              class:active={locale === "en"}
              class="lang-btn"
              type="button"
              on:click={() => setLocale("en")}
            >
              {t("languageEn")}
            </button>
          </div>
        </div>
      </section>
    </aside>
  </div>
</div>
