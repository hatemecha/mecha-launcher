<script lang="ts">
  import type { MessageKey } from "../i18n";

  export let minecraftDir = "";
  export let detectedMinecraftDir = "";
  export let errorMessage = "";
  export let statusMessage = "";
  export let missingVersionsDirPath: string | null = null;
  export let isLoadingVersions = false;
  export let isEnsuringVersionsDir = false;
  export let t: (key: MessageKey) => string;
  export let onMinecraftDirInput: (path: string) => void;
  export let onBrowse: () => void | Promise<void>;
  export let onReload: () => void | Promise<void>;
  export let onCreateVersionsDir: () => void | Promise<void>;
</script>

<section class="panel-section">
  <div class="section-title accent">
    <svg class="app-icon" aria-hidden="true"><use href="#icon-folder" /></svg>
    {t("gameDirectory")}
  </div>
  <p class="help-text">{t("gameDirectoryHelp")} <code>.minecraft</code>.</p>

  <input
    class="text-input"
    type="text"
    value={minecraftDir}
    on:input={(event) => onMinecraftDirInput((event.currentTarget as HTMLInputElement).value)}
    placeholder={t("mcPathPlaceholder")}
    aria-label={t("minecraftDirAria")}
  />

  <div class="btn-row">
    <button class="btn" type="button" on:click={onBrowse}>
      <svg class="app-icon" aria-hidden="true"><use href="#icon-folder" /></svg>
      {t("browse")}
    </button>
    <button class="btn" type="button" on:click={onReload} disabled={isLoadingVersions}>
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
          on:click={onCreateVersionsDir}
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
