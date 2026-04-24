import { fillTemplate, translate, type Locale } from "./i18n";
import type {
  LauncherStatusEvent,
  OptifineInstallStatusEvent,
  VanillaInstallStatusEvent
} from "./types";

function formatProgress(current?: number | null, total?: number | null): string {
  if (!current || !total) {
    return "";
  }

  return ` ${current}/${total}`;
}

export function formatLauncherStatusMessage(
  locale: Locale,
  event: LauncherStatusEvent,
  versionId: string
): string {
  switch (event.state) {
    case "launching":
      return fillTemplate(translate(locale, "launchStatePreparing"), {
        id: versionId || event.launchId
      });
    case "running":
      return fillTemplate(translate(locale, "launchStateRunningVersion"), {
        id: versionId || event.launchId
      });
    case "exited":
      return translate(locale, "launchStateExited");
    case "error":
    default:
      return event.message ?? translate(locale, "errLaunchFailed");
  }
}

export function formatOptifineInstallMessage(
  locale: Locale,
  event: OptifineInstallStatusEvent
): string {
  switch (event.stage) {
    case "prepare":
      return translate(locale, "installStagePrepare");
    case "minecraft":
      return `${translate(locale, "installStageMinecraft")}${formatProgress(
        event.current,
        event.total
      )}`;
    case "libraries":
      return `${translate(locale, "installStageLibraries")}${formatProgress(
        event.current,
        event.total
      )}`;
    case "assets":
      return `${translate(locale, "installStageAssets")}${formatProgress(
        event.current,
        event.total
      )}`;
    case "runtime":
      return `${translate(locale, "installStageRuntime")}${formatProgress(
        event.current,
        event.total
      )}`;
    case "optifine":
      return translate(locale, "installStageOptifine");
    case "done":
      return fillTemplate(translate(locale, "installStageDone"), {
        id: event.optionId
      });
    default:
      return event.message;
  }
}

export function formatVanillaInstallMessage(
  locale: Locale,
  event: VanillaInstallStatusEvent
): string {
  switch (event.stage) {
    case "prepare":
      return translate(locale, "installStagePrepare");
    case "minecraft":
      return `${translate(locale, "installStageMinecraft")}${formatProgress(
        event.current,
        event.total
      )}`;
    case "libraries":
      return `${translate(locale, "installStageLibraries")}${formatProgress(
        event.current,
        event.total
      )}`;
    case "assets":
      return `${translate(locale, "installStageAssets")}${formatProgress(
        event.current,
        event.total
      )}`;
    case "runtime":
      return `${translate(locale, "installStageRuntime")}${formatProgress(
        event.current,
        event.total
      )}`;
    case "done":
      return fillTemplate(translate(locale, "installStageDone"), {
        id: event.versionId
      });
    default:
      return event.message;
  }
}
