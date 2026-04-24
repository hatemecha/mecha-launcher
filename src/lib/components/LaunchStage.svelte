<script lang="ts">
  import CatScene from "../CatScene.svelte";
  import PlayerScene from "../PlayerScene.svelte";
  import type { MessageKey } from "../i18n";

  type ThemeMode = "light" | "dark";
  type LogLine = {
    launchId: string;
    source: "stdout" | "stderr" | "system";
    line: string;
  };

  export let brandMarkUrl = "";
  export let themeMode: ThemeMode = "dark";
  export let offlineUsername = "";
  export let defaultOfflineUsername = "Player";
  export let selectedVersionId = "";
  export let versionCount = 0;
  export let isLaunching = false;
  export let isInstalling = false;
  export let offlineSkinUrl: string | null = null;
  export let logLines: LogLine[] = [];
  export let copiedLog = false;
  export let copyLog: () => void | Promise<void>;
  export let formatSourceLabel: (source: LogLine["source"]) => string;
  export let statusMessage = "";
  export let minecraftDir = "";
  export let t: (key: MessageKey) => string;
  export let logViewport: HTMLDivElement | null = null;
</script>

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
        <span>{offlineUsername || defaultOfflineUsername}</span>
      </div>
    </div>

    <div class="run-summary" aria-label={t("runSummaryAria")}>
      <div>
        <span>{t("runVersion")}</span>
        <strong>{selectedVersionId || "—"}</strong>
      </div>
      <div>
        <span>{t("runInstalled")}</span>
        <strong>{versionCount}</strong>
      </div>
      <div>
        <span>{t("runState")}</span>
        <strong>
          {isInstalling ? t("stateInstalling") : isLaunching ? t("stateLaunching") : t("stateReady")}
        </strong>
      </div>
    </div>

    <div class="stage-content split-stage">
      <section class="stage-pane cat-pane" aria-label={t("panePreviewSr")}>
        <div class="pane-title pane-title-icon" title={t("panePreviewTitle")}>
          <svg class="app-icon" aria-hidden="true"><use href="#icon-cat" /></svg>
          <span class="sr-only">{t("panePreviewSr")}</span>
        </div>
        <CatScene
          {themeMode}
          sceneAriaLabel={t("catSceneAria")}
          fallbackMessage={t("sceneUnavailable")}
        />
      </section>

      <section class="stage-pane" aria-label={t("panePlayerPreviewSr")}>
        <div class="pane-title pane-title-icon" title={t("panePlayerPreviewTitle")}>
          <svg class="app-icon" aria-hidden="true"><use href="#icon-cube" /></svg>
          <span class="sr-only">{t("panePlayerPreviewSr")}</span>
        </div>
        <PlayerScene
          skinUrl={offlineSkinUrl}
          sceneAriaLabel={t("playerSceneAria")}
          fallbackMessage={t("sceneUnavailable")}
        />
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
