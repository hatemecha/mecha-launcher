<script lang="ts">
  import type { CatalogFilter, CatalogItem } from "../catalog";
  import type { MessageKey } from "../i18n";
  import type {
    OptifineInstallStatusEvent,
    ReduxInstallStatusEvent,
    VanillaInstallStatusEvent
  } from "../types";

  export let versionFilter: CatalogFilter = "installed";
  export let filteredCatalogItems: CatalogItem[] = [];
  export let selectedVersionId = "";
  export let isLaunching = false;
  export let installingOptifineOptionId: string | null = null;
  export let installingVanillaVersionId: string | null = null;
  export let installingReduxVersionId: string | null = null;
  export let installProgress: OptifineInstallStatusEvent | null = null;
  export let vanillaInstallProgress: VanillaInstallStatusEvent | null = null;
  export let reduxInstallProgress: ReduxInstallStatusEvent | null = null;
  export let installPercent: number | null = null;
  export let t: (key: MessageKey) => string;
  export let isFavorite: (item: CatalogItem) => boolean;
  export let onSetVersionFilter: (filter: CatalogFilter) => void;
  export let onSelectCatalogItem: (item: CatalogItem) => void;
  export let onToggleFavorite: (item: CatalogItem) => void;
  export let onDownload: (item: CatalogItem) => void | Promise<void>;
  export let onDelete: (item: CatalogItem) => void | Promise<void>;

  let query = "";

  function matchesQuery(item: CatalogItem, rawQuery: string): boolean {
    const q = rawQuery.trim().toLowerCase();
    if (!q) {
      return true;
    }

    return (
      item.title.toLowerCase().includes(q) ||
      item.subtitle.toLowerCase().includes(q) ||
      item.id.toLowerCase().includes(q) ||
      item.key.toLowerCase().includes(q)
    );
  }

  $: displayedCatalogItems = filteredCatalogItems.filter((item) => matchesQuery(item, query));
</script>

<section class="panel-section versions-section">
  <div class="section-title accent">
    <svg class="app-icon" aria-hidden="true"><use href="#icon-list" /></svg>
    {t("versionsCatalogTitle")}
  </div>

  <div class="version-filters" role="tablist" aria-label={t("versionsCatalogTitle")}>
    <button
      class:active={versionFilter === "installed"}
      class="chip"
      type="button"
      on:click={() => onSetVersionFilter("installed")}
    >
      {t("versionsFilterInstalled")}
    </button>
    <button
      class:active={versionFilter === "all"}
      class="chip"
      type="button"
      on:click={() => onSetVersionFilter("all")}
    >
      {t("versionsFilterAll")}
    </button>
    <button
      class:active={versionFilter === "popular"}
      class="chip"
      type="button"
      on:click={() => onSetVersionFilter("popular")}
    >
      {t("versionsFilterPopular")}
    </button>
    <button
      class:active={versionFilter === "favorites"}
      class="chip"
      type="button"
      on:click={() => onSetVersionFilter("favorites")}
    >
      {t("versionsFilterFavorites")}
    </button>
    <button
      class:active={versionFilter === "vanilla"}
      class="chip"
      type="button"
      on:click={() => onSetVersionFilter("vanilla")}
    >
      {t("versionsFilterVanilla")}
    </button>
    <button
      class:active={versionFilter === "redux"}
      class="chip"
      type="button"
      on:click={() => onSetVersionFilter("redux")}
    >
      {t("versionsFilterRedux")}
    </button>
    <button
      class:active={versionFilter === "optifine"}
      class="chip"
      type="button"
      on:click={() => onSetVersionFilter("optifine")}
    >
      {t("versionsFilterOptifine")}
    </button>
  </div>

  <input
    class="text-input"
    type="search"
    bind:value={query}
    placeholder={t("versionsSearchPlaceholder")}
    aria-label={t("versionsSearchAria")}
    autocomplete="off"
    spellcheck="false"
  />

  <div class="version-list" role="list" aria-label={t("versionsListAria")}>
    {#if displayedCatalogItems.length === 0}
      <div class="empty-state">
        <p>{t("versionsEmptyTitle")}</p>
        <span>{t("versionsEmptyDetail")} <code>.json</code> + <code>.jar</code>.</span>
      </div>
    {:else}
      {#each displayedCatalogItems as item (item.key)}
        {@const selected = item.installedVersionId === selectedVersionId}
        {@const busy = Boolean(
          isLaunching || installingOptifineOptionId || installingVanillaVersionId || installingReduxVersionId
        )}
        <div class:selected class="catalog-item" role="listitem">
          <button
            class="catalog-main"
            type="button"
            on:click={() => onSelectCatalogItem(item)}
            disabled={!item.installed || busy}
          >
            <span class="version-name">{item.title}</span>
            <span class="version-meta">
              {item.subtitle}
              {#if item.requiredJavaMajor}
                · Java {item.requiredJavaMajor}
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
              on:click={() => onToggleFavorite(item)}
              disabled={busy}
            >
              <svg class="app-icon" aria-hidden="true"><use href="#icon-star" /></svg>
            </button>

            {#if item.installed}
              <button
                class="icon-btn"
                type="button"
                aria-label={t("versionActionDelete")}
                on:click={() => onDelete(item)}
                disabled={busy}
              >
                <svg class="app-icon" aria-hidden="true"><use href="#icon-trash" /></svg>
              </button>
            {:else}
              <button
                class="icon-btn"
                type="button"
                aria-label={t("versionActionDownload")}
                on:click={() => onDownload(item)}
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

  {#if reduxInstallProgress}
    {@const percent = reduxInstallProgress.current && reduxInstallProgress.total
      ? Math.min(100, Math.round((reduxInstallProgress.current / reduxInstallProgress.total) * 100))
      : null}
    <div class="install-progress" role="status">
      <div class="install-progress-copy">
        <span>{reduxInstallProgress.stage}</span>
        <strong>{reduxInstallProgress.message}</strong>
      </div>
      <div class:indeterminate={percent === null} class="progress-track">
        <span style={`width: ${percent ?? 100}%`}></span>
      </div>
    </div>
  {/if}
</section>
