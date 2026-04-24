<script lang="ts">
  import type { MessageKey } from "../i18n";
  import type { GraphicsDependencyStatus, JavaDependencyStatus } from "../types";

  export let showDependencyWarnings = false;
  export let needsJavaAttention = false;
  export let needsGraphicsAttention = false;
  export let needsSelectedVersionJavaAttention = false;
  export let javaStatus: JavaDependencyStatus | null = null;
  export let graphicsStatus: GraphicsDependencyStatus | null = null;
  export let selectedSystemJavaMajorRequirement: number | null = null;
  export let isInstallingJava = false;
  export let isInstallingGraphicsDependency = false;
  export let t: (key: MessageKey) => string;
  export let fillTemplate: (
    template: string,
    vars: Record<string, string | number>
  ) => string;
  export let onAutoInstallJava: () => void | Promise<void>;
  export let onAutoInstallGraphicsDependency: () => void | Promise<void>;
</script>

{#if showDependencyWarnings}
  <section class="panel-section deps-banner">
    <div class="section-title accent">{t("depsTitle")}</div>

    {#if needsJavaAttention}
      <div class="help-text">
        <strong>{t("depsJavaTitle")}</strong>
        {#if javaStatus?.installed && javaStatus?.detectedMajor && selectedSystemJavaMajorRequirement}
          <div class="status-msg error">
            {fillTemplate(t("depsJavaNotRecommended"), {
              major: javaStatus.detectedMajor,
              recommended: selectedSystemJavaMajorRequirement
            })}
          </div>
        {:else}
          <div class="status-msg error">{t("depsJavaMissing")}</div>
        {/if}

        <div class="dependency-actions">
          <div><strong>{t("depsJavaHowTo")}</strong></div>
          {#if javaStatus?.canAutoInstall}
            <div class="btn-row dependency-actions-row">
              <button
                class="btn"
                type="button"
                on:click={onAutoInstallJava}
                disabled={isInstallingJava}
              >
                {isInstallingJava ? t("depsJavaInstalling") : t("depsJavaAutoInstall")}
              </button>
            </div>
            {#if javaStatus?.autoInstallHint}
              <div class="hint">{javaStatus.autoInstallHint}</div>
            {/if}
          {/if}
          {#if javaStatus?.suggestedLinuxCommands?.length}
            <div>{t("depsJavaLinuxNote")}</div>
            <pre class="dependency-command-list">{javaStatus.suggestedLinuxCommands.join("\n")}</pre>
          {/if}
          {#if javaStatus?.suggestedWindowsLinks?.length}
            <div>{t("depsJavaWindowsNote")}</div>
            <ul class="dependency-link-list">
              {#each javaStatus.suggestedWindowsLinks as link}
                <li><a href={link.url} target="_blank" rel="noreferrer">{link.label}</a></li>
              {/each}
            </ul>
          {/if}
        </div>
      </div>
    {/if}

    {#if needsSelectedVersionJavaAttention && javaStatus && selectedSystemJavaMajorRequirement}
      <div class="help-text dependency-subcard">
        <strong>{t("depsJavaTitle")}</strong>
        <div class="status-msg error">
          {fillTemplate(t("depsJavaNotRecommended"), {
            major: javaStatus.detectedMajor ?? "—",
            recommended: selectedSystemJavaMajorRequirement
          })}
        </div>
        <div class="dependency-actions">
          <div><strong>{t("depsJavaHowTo")}</strong></div>
          {#if javaStatus.canAutoInstall}
            <div class="btn-row dependency-actions-row">
              <button
                class="btn"
                type="button"
                on:click={onAutoInstallJava}
                disabled={isInstallingJava}
              >
                {isInstallingJava ? t("depsJavaInstalling") : t("depsJavaAutoInstall")}
              </button>
            </div>
          {/if}
          {#if javaStatus.suggestedLinuxCommands?.length}
            <div>{t("depsJavaLinuxNote")}</div>
            <pre class="dependency-command-list">{javaStatus.suggestedLinuxCommands.join("\n")}</pre>
          {/if}
          {#if javaStatus.suggestedWindowsLinks?.length}
            <div>{t("depsJavaWindowsNote")}</div>
            <ul class="dependency-link-list">
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

        <div class="dependency-actions">
          <div><strong>{t("depsJavaHowTo")}</strong></div>
          {#if graphicsStatus?.canAutoInstall}
            <div class="btn-row dependency-actions-row">
              <button
                class="btn"
                type="button"
                on:click={onAutoInstallGraphicsDependency}
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
            <pre class="dependency-command-list">{graphicsStatus.suggestedLinuxCommands.join("\n")}</pre>
          {/if}
        </div>
      </div>
    {/if}
  </section>
{/if}
