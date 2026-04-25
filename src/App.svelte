<script lang="ts">
  import { onMount, tick } from "svelte";

  import brandMarkUrl from "../hatecreeper2.png?url";

  import {
    buildCatalogItems,
    filterCatalogItems,
    findCatalogItemForInstalledVersion,
    type CatalogFilter,
    type CatalogItem
  } from "./lib/catalog";
  import {
    autoInstallGraphicsDependency,
    autoInstallJava,
    browseMinecraftDir,
    checkGraphicsDependency,
    checkJavaDependency,
    deleteInstalledVersion,
    detectDefaultMinecraftDir,
    ensureVersionsDir,
    getSystemMemoryProfile,
    installOptifineVersion,
    installReduxVersion,
    installVanillaVersion,
    launchVersion,
    listOptifineInstallOptions,
    listReduxInstallOptions,
    listVanillaReleases,
    listVersions,
    onLauncherLog,
    onLauncherStatus,
    onOptifineInstallStatus,
    onReduxInstallStatus,
    onVanillaInstallStatus
  } from "./lib/tauri";
  import {
    formatLauncherStatusMessage,
    formatOptifineInstallMessage,
    formatReduxInstallMessage,
    formatVanillaInstallMessage
  } from "./lib/launcher-messages";
  import DependencyBanner from "./lib/components/DependencyBanner.svelte";
  import LanguagePanel from "./lib/components/LanguagePanel.svelte";
  import LaunchStage from "./lib/components/LaunchStage.svelte";
  import MemorySettingsPanel from "./lib/components/MemorySettingsPanel.svelte";
  import MinecraftDirectoryPanel from "./lib/components/MinecraftDirectoryPanel.svelte";
  import OfflineProfilePanel from "./lib/components/OfflineProfilePanel.svelte";
  import VersionCatalogPanel from "./lib/components/VersionCatalogPanel.svelte";
  import {
    clearSelectedVersionId,
    incrementPopularity,
    loadLaunchMemoryMb,
    loadLaunchMemoryMode,
    loadStoredMinecraftDir,
    loadStoredVersionId,
    loadFavoriteKeys,
    loadPopularity,
    storeMinecraftDir,
    storeSelectedVersionId,
    loadOfflineSkinDataUrl,
    loadOfflineUsername,
    storeLaunchMemoryMb,
    storeLaunchMemoryMode,
    storeOfflineSkinDataUrl,
    storeOfflineUsername,
    toggleFavoriteKey
  } from "./lib/storage";
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
    LaunchMemoryMode,
    LauncherLogEvent,
    LaunchState,
    LauncherStatusEvent,
    MinecraftVersionSummary,
    OptifineInstallOption,
    OptifineInstallStatusEvent,
    ReduxInstallOption,
    ReduxInstallStatusEvent,
    SystemMemoryProfile,
    VanillaInstallStatusEvent,
    VanillaRelease
  } from "./lib/types";

  const DEFAULT_OFFLINE_USERNAME = "Player";
  const THEME_STORAGE_KEY = "mecha-launcher.themeMode";
  const MAX_LOG_LINES = 250;
  const MAX_OFFLINE_SKIN_BYTES = 1024 * 1024;
  const VALID_OFFLINE_SKIN_DIMENSIONS = new Set(["64x32", "64x64"]);
  const FALLBACK_SYSTEM_MEMORY_PROFILE: SystemMemoryProfile = {
    detected: false,
    totalMemoryMb: 8192,
    minAllocatableMb: 1024,
    maxAllocatableMb: 6144,
    recommendedAllocatableMb: 2048
  };

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
  let reduxOptions: ReduxInstallOption[] = [];
  let vanillaReleases: VanillaRelease[] = [];
  let selectedVersionId = "";
  let offlineUsername = loadOfflineUsername() ?? DEFAULT_OFFLINE_USERNAME;
  let offlineSkinUrl = loadOfflineSkinDataUrl();
  let errorMessage = "";
  let statusMessage = translate(locale, "statusWaitingDir");
  let isLoadingVersions = false;
  let isEnsuringVersionsDir = false;
  let isInstallingJava = false;
  let isInstallingGraphicsDependency = false;
  let javaStatus:
    | {
        installed: boolean;
        meetsRequirement: boolean;
        detectedMajor?: number | null;
        suggestedLinuxCommands?: string[] | null;
        suggestedWindowsLinks?: { label: string; url: string }[] | null;
        canAutoInstall?: boolean | null;
        autoInstallHint?: string | null;
        requiredMajor?: number | null;
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
  let installingReduxVersionId: string | null = null;
  let reduxInstallProgress: ReduxInstallStatusEvent | null = null;
  let lastLoggedInstallStage = "";
  let lastLoggedVanillaInstallStage = "";
  let lastLoggedReduxInstallStage = "";
  let activeLaunchId: string | null = null;
  let activeLaunchState: LaunchState | null = null;
  let logLines: LogLine[] = [];
  let logViewport: HTMLDivElement | null = null;
  let copiedLog = false;
  let themeMode: ThemeMode = "dark";
  let versionFilter: CatalogFilter = "installed";
  let lastDependencyRequirement: number | null | undefined = undefined;
  let systemMemoryProfile: SystemMemoryProfile = FALLBACK_SYSTEM_MEMORY_PROFILE;
  let launchMemoryMode: LaunchMemoryMode = loadLaunchMemoryMode();
  let manualLaunchMemoryMb =
    loadLaunchMemoryMb() ?? FALLBACK_SYSTEM_MEMORY_PROFILE.recommendedAllocatableMb;

  $: needsJavaAttention = Boolean(
    javaStatus && !selectedVersionProvidesRuntime && !javaStatus.installed
  );

  $: needsGraphicsAttention = Boolean(
    graphicsStatus &&
      graphicsStatus.required &&
      (!graphicsStatus.installed || !graphicsStatus.usable)
  );

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

  function clampLaunchMemoryMb(memoryMb: number): number {
    if (!Number.isFinite(memoryMb)) {
      return systemMemoryProfile.recommendedAllocatableMb;
    }

    return Math.min(
      systemMemoryProfile.maxAllocatableMb,
      Math.max(systemMemoryProfile.minAllocatableMb, Math.round(memoryMb))
    );
  }

  function setLaunchMemoryMode(nextMode: LaunchMemoryMode): void {
    launchMemoryMode = nextMode;
    storeLaunchMemoryMode(nextMode);
  }

  function setManualLaunchMemoryMb(nextMemoryMb: number): void {
    const clamped = clampLaunchMemoryMb(nextMemoryMb);
    manualLaunchMemoryMb = clamped;
    storeLaunchMemoryMb(clamped);
  }

  async function hydrateMinecraftDir(): Promise<void> {
    const storedDir = loadStoredMinecraftDir();
    const detectedDir = await detectDefaultMinecraftDir();

    detectedMinecraftDir = detectedDir ?? "";
    minecraftDir = storedDir ?? detectedMinecraftDir;
  }

  function handleUsernameChange(username: string): void {
    offlineUsername = username;
    storeOfflineUsername(username);
  }

  async function readFileAsDataUrl(file: File): Promise<string> {
    return new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result ?? ""));
      reader.onerror = () => reject(new Error(t("errSkinRead")));
      reader.readAsDataURL(file);
    });
  }

  async function validateSkinDimensions(dataUrl: string): Promise<void> {
    const image = new Image();

    await new Promise<void>((resolve, reject) => {
      image.onload = () => resolve();
      image.onerror = () => reject(new Error(t("errSkinRead")));
      image.src = dataUrl;
    });

    const sizeKey = `${image.naturalWidth}x${image.naturalHeight}`;
    if (!VALID_OFFLINE_SKIN_DIMENSIONS.has(sizeKey)) {
      throw new Error(t("errSkinInvalidDimensions"));
    }
  }

  async function handleLocalSkinFile(file: File | null): Promise<void> {
    if (!file) {
      return;
    }
    if (file.type !== "image/png") {
      errorMessage = t("errSkinMustBePng");
      await appendLog("system", errorMessage);
      return;
    }

    if (file.size > MAX_OFFLINE_SKIN_BYTES) {
      errorMessage = t("errSkinTooLarge");
      await appendLog("system", errorMessage);
      return;
    }

    try {
      const dataUrl = await readFileAsDataUrl(file);
      await validateSkinDimensions(dataUrl);
      offlineSkinUrl = dataUrl;
      if (!storeOfflineSkinDataUrl(dataUrl)) {
        throw new Error(t("errSkinStorage"));
      }
    } catch (error) {
      errorMessage = getErrorMessage(error, t("errSkinRead"));
      await appendLog("system", errorMessage);
    }
  }

  async function refreshDependencies(requiredMajor = selectedSystemJavaMajorRequirement): Promise<void> {
    try {
      javaStatus = await checkJavaDependency(
        requiredMajor ? { requiredMajor } : undefined
      );
      graphicsStatus = await checkGraphicsDependency();
    } catch (error) {
      await appendLog("system", getErrorMessage(error, t("errCheckDependencies")));
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
      const result = await autoInstallJava(
        selectedSystemJavaMajorRequirement ? { requiredMajor: selectedSystemJavaMajorRequirement } : undefined
      );
      if (!result.ok) {
        const combined = [result.stdout, result.stderr].filter(Boolean).join("\n").trim();
        errorMessage = combined || t("depsJavaInstallFailed");
        await appendLog("system", errorMessage);
      } else {
        await appendLog(
          "system",
          result.alreadyPresent ? t("depsJavaWingetAlreadyCurrent") : t("depsJavaInstalledOk")
        );
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
      await refreshDependencies();
    } catch (error) {
      versions = [];
      selectedVersionId = "";
      clearSelectedVersionId();
      errorMessage = getErrorMessage(error, translate(locale, "errReadVersions"));
      statusMessage = translate(locale, "statusDirInvalid");
      await appendLog("system", errorMessage);
      await refreshDependencies();
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

  async function handleInstallRedux(option: ReduxInstallOption): Promise<void> {
    if (installingReduxVersionId || isLaunching) {
      return;
    }

    const targetDir = minecraftDir.trim() || detectedMinecraftDir.trim();
    if (!targetDir) {
      errorMessage = t("versionInstallNeedDir");
      await appendLog("system", errorMessage);
      return;
    }

    minecraftDir = targetDir;
    storeMinecraftDir(targetDir);
    installingReduxVersionId = option.versionId;
    if (selectedVersionId === option.versionId) {
      selectedVersionId = "";
      clearSelectedVersionId();
    }
    reduxInstallProgress = {
      optionId: option.id,
      versionId: option.versionId,
      stage: "queued",
      message: t("versionDownloading")
    };
    lastLoggedReduxInstallStage = "";
    errorMessage = "";
    statusMessage = t("versionDownloading");

    try {
      const result = await installReduxVersion({
        minecraftDir: targetDir,
        optionId: option.id
      });
      await refreshVersions();
      selectedVersionId = result.versionId;
      storeSelectedVersionId(result.versionId);
      await appendLog("system", fillTemplate(t("logVersionInstalled"), { id: result.versionId }));
      reduxInstallProgress = null;
      await refreshDependencies();
    } catch (error) {
      errorMessage = getErrorMessage(error, t("errInstallReduxFailed"));
      statusMessage = errorMessage;
      await appendLog("system", errorMessage);
    } finally {
      installingReduxVersionId = null;
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
        username: offlineUsername.trim() || DEFAULT_OFFLINE_USERNAME,
        requiredJavaMajor: selectedRequiredJavaMajor,
        maxMemoryMb: launchMemoryMode === "manual" ? clampLaunchMemoryMb(manualLaunchMemoryMb) : null
      });

      activeLaunchId = response.launchId;
      isLaunching = true;
      activeLaunchState = "launching";
      statusMessage = formatLauncherStatusMessage(locale, {
        launchId: response.launchId,
        state: "launching"
      }, selectedVersionId);
      if (selectedInstalledCatalogItem) {
        incrementPopularity(selectedInstalledCatalogItem.key);
      }
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
    activeLaunchState = event.state === "error" || event.state === "exited" ? null : event.state;

    statusMessage = formatLauncherStatusMessage(locale, event, selectedVersionId);

    if (event.state === "error") {
      errorMessage = event.message ?? translate(locale, "errLaunchFailed");
    }
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

    const localizedMessage = formatOptifineInstallMessage(locale, event);
    installProgress = { ...event, message: localizedMessage };
    statusMessage = localizedMessage;

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
      await appendLog("system", localizedMessage);
    }
  }

  async function handleVanillaStatus(event: VanillaInstallStatusEvent): Promise<void> {
    if (installingVanillaVersionId && event.versionId !== installingVanillaVersionId) {
      return;
    }

    const localizedMessage = formatVanillaInstallMessage(locale, event);
    vanillaInstallProgress = { ...event, message: localizedMessage };
    statusMessage = localizedMessage;

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
      await appendLog("system", localizedMessage);
    }
  }

  async function handleReduxStatus(event: ReduxInstallStatusEvent): Promise<void> {
    if (installingReduxVersionId && event.versionId !== installingReduxVersionId) {
      return;
    }

    const localizedMessage = formatReduxInstallMessage(locale, event);
    reduxInstallProgress = { ...event, message: localizedMessage };
    statusMessage = localizedMessage;

    const shouldLogInstallStatus =
      event.stage !== lastLoggedReduxInstallStage ||
      event.stage === "done" ||
      (event.current !== null &&
        event.current !== undefined &&
        event.total !== null &&
        event.total !== undefined &&
        event.current === event.total) ||
      (!event.current && !event.total);

    if (shouldLogInstallStatus) {
      lastLoggedReduxInstallStage = event.stage;
      await appendLog("system", localizedMessage);
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
    void refreshDependencies();
  }

  $: selectedVersion = versions.find((version) => version.id === selectedVersionId) ?? null;
  $: selectedVersionProvidesRuntime = Boolean(selectedVersion?.javaComponent);
  $: catalogItems = buildCatalogItems(versions, vanillaReleases, optifineOptions, reduxOptions);

  $: selectedInstalledCatalogItem =
    findCatalogItemForInstalledVersion(catalogItems, selectedVersionId);
  $: selectedRequiredJavaMajor =
    selectedInstalledCatalogItem?.requiredJavaMajor ?? selectedVersion?.javaMajorVersion ?? null;
  $: selectedSystemJavaMajorRequirement = selectedVersionProvidesRuntime
    ? null
    : selectedRequiredJavaMajor;
  $: if (
    javaStatus !== null &&
    lastDependencyRequirement !== selectedSystemJavaMajorRequirement
  ) {
    lastDependencyRequirement = selectedSystemJavaMajorRequirement;
    void refreshDependencies(selectedSystemJavaMajorRequirement);
  }

  $: needsSelectedVersionJavaAttention = Boolean(
    javaStatus &&
      selectedSystemJavaMajorRequirement &&
      !selectedVersionProvidesRuntime &&
      !javaStatus.meetsRequirement
  );

  $: showDependencyWarnings =
    needsJavaAttention || needsGraphicsAttention || needsSelectedVersionJavaAttention;

  $: javaLaunchReady = Boolean(
    selectedVersionProvidesRuntime ||
      (javaStatus?.installed &&
        (!selectedSystemJavaMajorRequirement || javaStatus.meetsRequirement))
  );

  $: systemDependenciesOk =
    graphicsStatus !== null &&
    !needsGraphicsAttention &&
    javaLaunchReady;

  $: canPlay = Boolean(
    minecraftDir.trim() &&
      selectedVersionId &&
      selectedVersion &&
      !isLaunching &&
      !installingOptifineOptionId &&
      !installingVanillaVersionId &&
      !installingReduxVersionId &&
      selectedVersionId !== installingOptifineVersionId &&
      systemDependenciesOk
  );
  $: playButtonLabel =
    activeLaunchState === "launching"
      ? t("launching")
      : activeLaunchState === "running"
        ? t("playBusy")
        : t("play");
  $: playBlockedMessage =
    activeLaunchState === "launching" || activeLaunchState === "running"
      ? t("playBlockedActiveInstance")
      : "";
  $: installPercent = progressPercent(installProgress);
  $: if (manualLaunchMemoryMb !== clampLaunchMemoryMb(manualLaunchMemoryMb)) {
    manualLaunchMemoryMb = clampLaunchMemoryMb(manualLaunchMemoryMb);
    storeLaunchMemoryMb(manualLaunchMemoryMb);
  }

  let favoriteKeys = loadFavoriteKeys();
  let popularity = loadPopularity();
  $: filteredCatalogItems = filterCatalogItems(
    catalogItems,
    versionFilter,
    favoriteKeys,
    popularity
  );

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
    if (
      isLaunching ||
      installingOptifineOptionId ||
      installingVanillaVersionId ||
      installingReduxVersionId
    ) {
      return;
    }

    const targetDir = minecraftDir.trim() || detectedMinecraftDir.trim();
    if (!targetDir) {
      errorMessage = translate(locale, "versionInstallNeedDir");
      await appendLog("system", errorMessage);
      return;
    }

    minecraftDir = targetDir;
    storeMinecraftDir(targetDir);
    errorMessage = "";

    if (item.kind === "optifine") {
      const option = optifineOptions.find((opt) => opt.id === item.id);
      if (!option) {
        errorMessage = t("errUnknownOptifineOption");
        await appendLog("system", errorMessage);
        return;
      }
      incrementPopularity(item.key);
      await handleInstallOptifine(option);
      popularity = loadPopularity();
      return;
    }

    if (item.kind === "redux") {
      const option = reduxOptions.find((opt) => opt.id === item.id);
      if (!option) {
        errorMessage = t("errUnknownReduxOption");
        await appendLog("system", errorMessage);
        return;
      }
      incrementPopularity(item.key);
      await handleInstallRedux(option);
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
      await appendLog("system", fillTemplate(t("logVersionInstalled"), { id: result.versionId }));
      vanillaInstallProgress = null;
      await refreshDependencies();
    } catch (error) {
      errorMessage = getErrorMessage(error, t("errInstallVanillaFailed"));
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
    if (
      isLaunching ||
      installingOptifineOptionId ||
      installingVanillaVersionId ||
      installingReduxVersionId
    ) {
      return;
    }

    const targetDir = minecraftDir.trim() || detectedMinecraftDir.trim();
    if (!targetDir) {
      errorMessage = translate(locale, "versionInstallNeedDir");
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
      await appendLog("system", fillTemplate(t("logVersionDeleted"), { id: versionId }));
      await refreshDependencies();
    } catch (error) {
      errorMessage = getErrorMessage(error, t("errDeleteVersionFailed"));
      await appendLog("system", errorMessage);
    }
  }

  onMount(() => {
    let unlistenStatus = () => undefined;
    let unlistenLog = () => undefined;
    let unlistenOptifineInstall = () => undefined;
    let unlistenVanillaInstall = () => undefined;
    let unlistenReduxInstall = () => undefined;
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
      unlistenReduxInstall = await onReduxInstallStatus((event) => {
        void handleReduxStatus(event);
      });

      try {
        try {
          systemMemoryProfile = await getSystemMemoryProfile();
          manualLaunchMemoryMb = clampLaunchMemoryMb(manualLaunchMemoryMb);
          storeLaunchMemoryMb(manualLaunchMemoryMb);
        } catch (error) {
          await appendLog("system", getErrorMessage(error, t("errLoadMemoryProfile")));
        }

        await refreshDependencies();
        await hydrateMinecraftDir();
        offlineUsername = loadOfflineUsername() ?? DEFAULT_OFFLINE_USERNAME;
        offlineSkinUrl = loadOfflineSkinDataUrl();
        if (minecraftDir) {
          await refreshVersions();
        }
      } catch (error) {
        errorMessage = getErrorMessage(error, translate(locale, "errInitLauncher"));
        await appendLog("system", errorMessage);
      }

      const [optifineResult, vanillaResult, reduxResult] = await Promise.allSettled([
        listOptifineInstallOptions(),
        listVanillaReleases(),
        listReduxInstallOptions()
      ]);

      if (optifineResult.status === "fulfilled") {
        optifineOptions = optifineResult.value;
      } else {
        await appendLog(
          "system",
          getErrorMessage(optifineResult.reason, t("errLoadOptifineCatalog"))
        );
      }

      if (vanillaResult.status === "fulfilled") {
        vanillaReleases = vanillaResult.value;
      } else {
        await appendLog(
          "system",
          getErrorMessage(vanillaResult.reason, t("errLoadVanillaCatalog"))
        );
      }

      if (reduxResult.status === "fulfilled") {
        reduxOptions = reduxResult.value;
      } else {
        await appendLog("system", getErrorMessage(reduxResult.reason, t("errLoadReduxCatalog")));
      }
    })();

    return () => {
      unlistenStatus();
      unlistenLog();
      unlistenOptifineInstall();
      unlistenVanillaInstall();
      unlistenReduxInstall();
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
  <symbol id="icon-memory" viewBox="0 0 24 24">
    <rect x="5" y="7" width="14" height="10" rx="2" />
    <path d="M9 7V5" />
    <path d="M15 7V5" />
    <path d="M9 19v-2" />
    <path d="M15 19v-2" />
    <path d="M8.5 11.5h7" />
    <path d="M8.5 14.5h4" />
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
      class:active={
        isLaunching ||
        installingOptifineOptionId ||
        installingVanillaVersionId ||
        installingReduxVersionId
      }
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
    <LaunchStage
      bind:logViewport
      {brandMarkUrl}
      {themeMode}
      {offlineUsername}
      defaultOfflineUsername={DEFAULT_OFFLINE_USERNAME}
      {selectedVersionId}
      versionCount={versions.length}
      {isLaunching}
      isInstalling={Boolean(installingOptifineOptionId || installingVanillaVersionId)}
      {offlineSkinUrl}
      {logLines}
      {copiedLog}
      {copyLog}
      {formatSourceLabel}
      {statusMessage}
      {minecraftDir}
      {t}
    />

    <aside class="control-panel" aria-label={t("controlsAria")}>
      <div class="control-panel-scroll">
        <DependencyBanner
          {showDependencyWarnings}
          {needsJavaAttention}
          {needsGraphicsAttention}
          {needsSelectedVersionJavaAttention}
          {javaStatus}
          {graphicsStatus}
          {selectedSystemJavaMajorRequirement}
          {isInstallingJava}
          {isInstallingGraphicsDependency}
          {t}
          {fillTemplate}
          onAutoInstallJava={handleAutoInstallJava}
          onAutoInstallGraphicsDependency={handleAutoInstallGraphicsDependency}
        />

        <OfflineProfilePanel
          {offlineUsername}
          {t}
          onUsernameInput={handleUsernameChange}
          onSkinChange={handleLocalSkinFile}
        />

        <MemorySettingsPanel
          memoryProfile={systemMemoryProfile}
          memoryMode={launchMemoryMode}
          manualMemoryMb={manualLaunchMemoryMb}
          {t}
          onSetMemoryMode={setLaunchMemoryMode}
          onSetManualMemoryMb={setManualLaunchMemoryMb}
        />

        <MinecraftDirectoryPanel
          {minecraftDir}
          {detectedMinecraftDir}
          {errorMessage}
          {statusMessage}
          {missingVersionsDirPath}
          {isLoadingVersions}
          {isEnsuringVersionsDir}
          {t}
          onMinecraftDirInput={(path) => {
            minecraftDir = path;
          }}
          onBrowse={handleBrowse}
          onReload={refreshVersions}
          onCreateVersionsDir={handleCreateVersionsDir}
        />

        <VersionCatalogPanel
          {versionFilter}
          {filteredCatalogItems}
          {selectedVersionId}
          {isLaunching}
          {installingOptifineOptionId}
          {installingVanillaVersionId}
          {installingReduxVersionId}
          {installProgress}
          {vanillaInstallProgress}
          {reduxInstallProgress}
          {installPercent}
          {t}
          {isFavorite}
          onSetVersionFilter={setVersionFilter}
          onSelectCatalogItem={selectCatalogItem}
          onToggleFavorite={toggleFavorite}
          onDownload={handleDownload}
          onDelete={handleDelete}
        />

        <LanguagePanel {locale} {t} onSetLocale={setLocale} />
      </div>

      <div class="control-panel-footer">
        <button
          class:active={canPlay}
          class="btn primary play-button control-panel-play"
          type="button"
          on:click={handlePlay}
          disabled={!canPlay}
        >
          <svg class="app-icon" aria-hidden="true"><use href="#icon-play" /></svg>
          {playButtonLabel}
        </button>
        {#if playBlockedMessage}
          <p class="play-blocked-note" role="status">{playBlockedMessage}</p>
        {/if}
      </div>
    </aside>
  </div>
</div>
