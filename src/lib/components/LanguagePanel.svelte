<script lang="ts">
  import type { Locale, MessageKey } from "../i18n";

  export let locale: Locale = "es";
  export let t: (key: MessageKey) => string;
  export let onSetLocale: (locale: Locale) => void;
</script>

<section class="panel-section panel-lang">
  <div class="lang-row">
    <span class="lang-label">{t("languageLabel")}</span>
    <div class="lang-toggle" role="group" aria-label={t("languageLabel")}>
      <button class:active={locale === "es"} class="lang-btn" type="button" on:click={() => onSetLocale("es")}>
        {t("languageEs")}
      </button>
      <button class:active={locale === "en"} class="lang-btn" type="button" on:click={() => onSetLocale("en")}>
        {t("languageEn")}
      </button>
    </div>
  </div>
</section>
