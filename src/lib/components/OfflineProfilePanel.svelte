<script lang="ts">
  import type { MessageKey } from "../i18n";

  export let offlineUsername = "";
  export let t: (key: MessageKey) => string;
  export let onUsernameInput: (username: string) => void;
  export let onSkinChange: (file: File | null) => void | Promise<void>;
</script>

<section class="panel-section">
  <div class="section-title accent">
    <svg class="app-icon" aria-hidden="true"><use href="#icon-user" /></svg>
    {t("offlineProfileTitle")}
  </div>

  <input
    class="text-input spaced-input"
    type="text"
    value={offlineUsername}
    on:input={(event) => onUsernameInput((event.currentTarget as HTMLInputElement).value)}
    placeholder={t("offlineUsernamePlaceholder")}
    aria-label={t("offlineUsernameAria")}
  />

  <div class="offline-skin-field">
    <div class="hint">{t("offlineSkinLabel")}</div>
    <input
      class="text-input"
      type="file"
      accept="image/png"
      on:change={(event) => onSkinChange((event.currentTarget as HTMLInputElement).files?.[0] ?? null)}
      aria-label={t("offlineSkinInputAria")}
    />
  </div>
</section>
