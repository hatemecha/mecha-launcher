<script lang="ts">
  import type { MessageKey } from "../i18n";
  import type { LaunchMemoryMode, SystemMemoryProfile } from "../types";

  export let memoryProfile: SystemMemoryProfile;
  export let memoryMode: LaunchMemoryMode = "auto";
  export let manualMemoryMb = 2048;
  export let t: (key: MessageKey) => string;
  export let onSetMemoryMode: (mode: LaunchMemoryMode) => void;
  export let onSetManualMemoryMb: (memoryMb: number) => void;

  function clampMemoryMb(value: number): number {
    if (!Number.isFinite(value)) {
      return memoryProfile.recommendedAllocatableMb;
    }

    return Math.min(
      memoryProfile.maxAllocatableMb,
      Math.max(memoryProfile.minAllocatableMb, Math.round(value))
    );
  }

  function handleManualMemoryInput(event: Event): void {
    const target = event.currentTarget as HTMLInputElement;
    onSetManualMemoryMb(clampMemoryMb(Number(target.value)));
  }

  $: appliedMemoryMb =
    memoryMode === "manual" ? clampMemoryMb(manualMemoryMb) : memoryProfile.recommendedAllocatableMb;
</script>

<section class="panel-section">
  <div class="section-title accent">
    <svg class="app-icon" aria-hidden="true"><use href="#icon-memory" /></svg>
    {t("memoryTitle")}
  </div>
  <p class="help-text">{t("memoryHelp")}</p>

  <div class="memory-summary-grid">
    <div class="memory-summary-row">
      <span class="memory-summary-label">{t("memoryDetectedTotal")}</span>
      <strong class="memory-summary-value">{memoryProfile.totalMemoryMb} MB</strong>
    </div>
    <div class="memory-summary-row">
      <span class="memory-summary-label">{t("memoryRange")}</span>
      <strong class="memory-summary-value">
        {memoryProfile.minAllocatableMb} - {memoryProfile.maxAllocatableMb} MB
      </strong>
    </div>
    <div class="memory-summary-row">
      <span class="memory-summary-label">{t("memoryWillUse")}</span>
      <strong class="memory-summary-value">{appliedMemoryMb} MB</strong>
    </div>
  </div>

  <div class="memory-mode-toggle" role="tablist" aria-label={t("memoryTitle")}>
    <button
      class:active={memoryMode === "auto"}
      class="chip"
      type="button"
      on:click={() => onSetMemoryMode("auto")}
    >
      {t("memoryModeAuto")}
    </button>
    <button
      class:active={memoryMode === "manual"}
      class="chip"
      type="button"
      on:click={() => onSetMemoryMode("manual")}
    >
      {t("memoryModeManual")}
    </button>
  </div>

  {#if memoryMode === "auto"}
    <p class="status-msg info">{t("memoryAutoHint")}</p>
  {:else}
    <p class="status-msg info">{t("memoryManualHint")}</p>
    <div class="memory-control-grid">
      <input
        class="memory-range"
        type="range"
        min={memoryProfile.minAllocatableMb}
        max={memoryProfile.maxAllocatableMb}
        step="256"
        value={clampMemoryMb(manualMemoryMb)}
        aria-label={t("memorySliderAria")}
        on:input={handleManualMemoryInput}
      />
      <input
        class="text-input memory-number-input"
        type="number"
        min={memoryProfile.minAllocatableMb}
        max={memoryProfile.maxAllocatableMb}
        step="256"
        value={clampMemoryMb(manualMemoryMb)}
        aria-label={t("memoryInputAria")}
        on:input={handleManualMemoryInput}
      />
    </div>
  {/if}

  {#if !memoryProfile.detected}
    <p class="status-msg info">{t("memoryFallbackHint")}</p>
  {/if}
</section>
