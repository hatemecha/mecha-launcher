<script lang="ts">
  import { onMount, tick } from "svelte";

  import {
    browseMinecraftDir,
    detectDefaultMinecraftDir,
    launchVersion,
    listVersions,
    onLauncherLog,
    onLauncherStatus
  } from "./lib/tauri";
  import {
    clearSelectedVersionId,
    loadStoredMinecraftDir,
    loadStoredVersionId,
    storeMinecraftDir,
    storeSelectedVersionId
  } from "./lib/storage";
  import type {
    LauncherLogEvent,
    LauncherStatusEvent,
    MinecraftVersionSummary
  } from "./lib/types";

  const FIXED_USERNAME = "Player";
  const THEME_STORAGE_KEY = "mecha-launcher.themeMode";

  type LogLine = {
    launchId: string;
    source: "stdout" | "stderr" | "system";
    line: string;
  };

  type ThemeMode = "light" | "dark";

  let minecraftDir = "";
  let detectedMinecraftDir = "";
  let versions: MinecraftVersionSummary[] = [];
  let selectedVersionId = "";
  let errorMessage = "";
  let statusMessage = "Waiting for a valid .minecraft directory.";
  let isLoadingVersions = false;
  let isLaunching = false;
  let activeLaunchId: string | null = null;
  let logLines: LogLine[] = [];
  let logViewport: HTMLDivElement | null = null;
  let themeMode: ThemeMode = "dark";

  async function appendLog(source: LogLine["source"], line: string, launchId = "system") {
    logLines = [...logLines, { source, line, launchId }];
    await tick();
    if (logViewport) {
      logViewport.scrollTop = logViewport.scrollHeight;
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
      errorMessage = "Set a .minecraft directory before reloading versions.";
      statusMessage = "Waiting for a valid .minecraft directory.";
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
        statusMessage = "No local versions were found in the selected directory.";
      } else {
        statusMessage = `${nextVersions.length} installed version${nextVersions.length === 1 ? "" : "s"} ready.`;
      }

      await appendLog("system", `Reloaded versions from ${trimmedDir}`);
    } catch (error) {
      versions = [];
      selectedVersionId = "";
      clearSelectedVersionId();
      errorMessage = getErrorMessage(error, "Failed to read installed versions.");
      statusMessage = "The selected directory could not be used.";
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
      errorMessage = getErrorMessage(error, "The folder picker failed.");
      await appendLog("system", errorMessage);
    }
  }

  async function handlePlay(): Promise<void> {
    if (!minecraftDir.trim() || !selectedVersionId || isLaunching) {
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
      statusMessage = "Preparing launch plan.";
      await appendLog("system", `Launch requested for ${selectedVersionId}`, response.launchId);
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to start Minecraft.");
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
      errorMessage = event.message ?? "Launch failed.";
    }

    statusMessage = event.message ?? `Launch state: ${event.state}`;
  }

  async function handleLog(event: LauncherLogEvent): Promise<void> {
    if (activeLaunchId && event.launchId !== activeLaunchId) {
      return;
    }

    await appendLog(event.source, event.line, event.launchId);
  }

  function formatSourceLabel(source: LogLine["source"]): string {
    switch (source) {
      case "stderr":
        return "ERR";
      case "stdout":
        return "OUT";
      default:
        return "SYS";
    }
  }

  function handleVersionSelection(versionId: string): void {
    selectedVersionId = versionId;
    storeSelectedVersionId(versionId);
  }

  $: canPlay = Boolean(minecraftDir.trim() && selectedVersionId && !isLaunching);

  onMount(() => {
    let unlistenStatus = () => undefined;
    let unlistenLog = () => undefined;
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

      try {
        await hydrateMinecraftDir();
        if (minecraftDir) {
          await refreshVersions();
        }
      } catch (error) {
        errorMessage = getErrorMessage(error, "Failed to initialize launcher.");
        await appendLog("system", errorMessage);
      }
    })();

    return () => {
      unlistenStatus();
      unlistenLog();
      removeThemePreferenceListener();
    };
  });
</script>

<svelte:head>
  <title>Mecha Launcher</title>
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
</svg>

<div class="app-shell">
  <header class="header-bar">
    <div class="brand-lockup">
      <span class="logo">MECHA LAUNCHER</span>
      <span class="beta-tag">LOCAL</span>
    </div>

    <button
      class="header-chip theme-toggle"
      type="button"
      on:click={toggleThemeMode}
      aria-label={themeMode === "dark" ? "Switch to light mode" : "Switch to dark mode"}
      aria-pressed={themeMode === "dark"}
    >
      <svg class="app-icon" aria-hidden="true"><use href="#icon-theme" /></svg>
      <span>{themeMode === "dark" ? "Dark" : "Light"}</span>
    </button>

    <div class:active={isLaunching} class="status-indicator" role="status" aria-live="polite">
      <svg class="app-icon status-dot" aria-hidden="true"><use href="#icon-status" /></svg>
      {isLaunching ? "RUNNING" : "IDLE"}
    </div>
  </header>

  <div class="app-container">
    <main class="preview-area">
      <section class="launch-stage" aria-labelledby="launcher-title">
        <div class="stage-header">
          <div>
            <p class="kicker">Minecraft Offline Launcher</p>
            <h1 id="launcher-title">Local Launch Panel</h1>
          </div>
          <div class="stage-badge">
            <svg class="app-icon" aria-hidden="true"><use href="#icon-user" /></svg>
            <span>{FIXED_USERNAME}</span>
          </div>
        </div>

        <p class="stage-copy">
          Point the launcher at an existing <code>.minecraft</code> folder, choose a local
          version, and stream the Rust launch output without extra accounts or downloads.
        </p>

        <div class="run-summary" aria-label="Launch summary">
          <div>
            <span>Version</span>
            <strong>{selectedVersionId || "—"}</strong>
          </div>
          <div>
            <span>Installed</span>
            <strong>{versions.length}</strong>
          </div>
          <div>
            <span>State</span>
            <strong>{isLaunching ? "Launching" : "Ready"}</strong>
          </div>
        </div>

        <div bind:this={logViewport} class="log-console" role="log" aria-live="polite">
          {#if logLines.length === 0}
            <p class="log-placeholder">Launch events and process output will appear here.</p>
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

      <div class="preview-status-bar">
        <span>{selectedVersionId || "No version selected"}</span>
        <span>{statusMessage}</span>
        <span>{minecraftDir.trim() || ".minecraft not set"}</span>
      </div>
    </main>

    <aside class="control-panel" aria-label="Launcher controls">
      <section class="panel-section">
        <div class="section-title accent">
          <svg class="app-icon" aria-hidden="true"><use href="#icon-folder" /></svg>
          Game Directory
        </div>
        <p class="help-text">Select the local <code>.minecraft</code> directory that already contains installed versions.</p>

        <input
          bind:value={minecraftDir}
          class="text-input"
          type="text"
          placeholder="C:\\Users\\you\\AppData\\Roaming\\.minecraft"
          aria-label="Minecraft directory"
        />

        <div class="btn-row">
          <button class="btn" type="button" on:click={handleBrowse}>
            <svg class="app-icon" aria-hidden="true"><use href="#icon-folder" /></svg>
            Browse
          </button>
          <button class="btn" type="button" on:click={refreshVersions} disabled={isLoadingVersions}>
            <svg class="app-icon" aria-hidden="true"><use href="#icon-refresh" /></svg>
            {isLoadingVersions ? "Reloading" : "Reload"}
          </button>
        </div>

        {#if detectedMinecraftDir}
          <p class="hint">Detected default: {detectedMinecraftDir}</p>
        {/if}

        {#if errorMessage}
          <p class="status-msg error">{errorMessage}</p>
        {:else}
          <p class="status-msg info">{statusMessage}</p>
        {/if}
      </section>

      <section class="panel-section versions-section">
        <div class="section-title accent">
          <svg class="app-icon" aria-hidden="true"><use href="#icon-list" /></svg>
          Installed Versions
        </div>

        <div class="version-list" role="list" aria-label="Minecraft versions">
          {#if versions.length === 0}
            <div class="empty-state">
              <p>No valid local versions found yet.</p>
              <span>Each version must include matching <code>.json</code> and <code>.jar</code> files.</span>
            </div>
          {:else}
            {#each versions as version}
              <button
                class:selected={version.id === selectedVersionId}
                class="version-item"
                type="button"
                on:click={() => handleVersionSelection(version.id)}
              >
                <span class="version-name">{version.folderName}</span>
                <span class="version-meta">
                  {#if version.javaMajorVersion}
                    Java {version.javaMajorVersion}
                  {:else if version.javaComponent}
                    {version.javaComponent}
                  {:else}
                    Runtime auto
                  {/if}
                </span>
              </button>
            {/each}
          {/if}
        </div>

        <button class:active={canPlay} class="btn primary play-button" type="button" on:click={handlePlay} disabled={!canPlay}>
          <svg class="app-icon" aria-hidden="true"><use href="#icon-play" /></svg>
          {isLaunching ? "Launching" : "Play"}
        </button>
      </section>

      <section class="panel-section">
        <div class="section-title">
          <svg class="app-icon" aria-hidden="true"><use href="#icon-terminal" /></svg>
          Runtime Output
        </div>
        <p class="help-text">The main panel shows system messages, stdout, and stderr from the active launch.</p>

        <div class="session-grid">
          <div>
            <span>Username</span>
            <strong>{FIXED_USERNAME}</strong>
          </div>
          <div>
            <span>Mode</span>
            <strong>{themeMode}</strong>
          </div>
          <div>
            <span>Backend</span>
            <strong>Tauri</strong>
          </div>
        </div>
      </section>
    </aside>
  </div>
</div>
