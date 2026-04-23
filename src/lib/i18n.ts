export type Locale = "es" | "en";

export const LOCALE_STORAGE_KEY = "mecha-launcher.locale";

const esMessages = {
  appTitle: "Mecha Launcher",
  themeDark: "Oscuro",
  themeLight: "Claro",
  themeAriaDark: "Cambiar a modo claro",
  themeAriaLight: "Cambiar a modo oscuro",
  statusIdle: "Reposo",
  statusRunning: "En curso",
  kicker: "Solo Minecraft",
  runVersion: "Versión",
  runInstalled: "Instaladas",
  runState: "Estado",
  stateLaunching: "Iniciando",
  stateReady: "Listo",
  panePreviewSr: "Vista previa",
  panePreviewTitle: "Vista previa",
  paneOutputSr: "Salida",
  paneOutputTitle: "Salida",
  logPlaceholder: "Aquí aparecerán los eventos y la salida del proceso.",
  noVersionSelected: "Sin versión seleccionada",
  mcDirNotSet: "Sin carpeta .minecraft",
  controlsAria: "Controles del launcher",
  gameDirectory: "Carpeta del juego",
  gameDirectoryHelp:
    "Elige la carpeta <code>.minecraft</code> local donde ya tengas versiones instaladas.",
  minecraftDirAria: "Carpeta de Minecraft",
  mcPathPlaceholder: "C:\\Users\\tú\\AppData\\Roaming\\.minecraft",
  browse: "Examinar",
  reload: "Recargar",
  reloading: "Recargando",
  detectedDefault: "Ruta detectada:",
  installedVersions: "Versiones instaladas",
  versionsEmptyTitle: "No hay versiones locales válidas.",
  versionsEmptyDetail:
    "Cada versión necesita sus archivos <code>.json</code> y <code>.jar</code> emparejados.",
  versionsListAria: "Versiones de Minecraft",
  play: "Jugar",
  launching: "Iniciando…",
  runtimeAuto: "Java automático",
  languageLabel: "Idioma",
  languageEs: "Español",
  languageEn: "English",
  catSceneAria: "Vista previa 3D en rotación",
  logSourceErr: "ERR",
  logSourceOut: "SAL",
  logSourceSys: "SYS",
  errSetDirBeforeReload: "Indica una carpeta .minecraft antes de recargar.",
  statusWaitingDir: "Esperando una carpeta .minecraft válida.",
  statusNoVersionsInDir: "No se encontraron versiones en la carpeta elegida.",
  versionsReadyOne: "1 versión lista.",
  versionsReadyMany: "{count} versiones listas.",
  errReadVersions: "No se pudieron leer las versiones instaladas.",
  statusDirInvalid: "No se pudo usar la carpeta seleccionada.",
  logReloadedVersions: "Versiones recargadas desde {path}",
  logLaunchRequested: "Lanzamiento solicitado: {id}",
  errFolderPicker: "No se pudo abrir el selector de carpetas.",
  errFailedLaunch: "No se pudo iniciar Minecraft.",
  errLaunchFailed: "Fallo al lanzar.",
  statusPreparingLaunch: "Preparando el lanzamiento.",
  launchState: "Estado: {state}",
  errInitLauncher: "No se pudo inicializar el launcher.",
  runSummaryAria: "Resumen del lanzamiento"
} as const;

export type MessageKey = keyof typeof esMessages;

const enMessages: Record<MessageKey, string> = {
  appTitle: "Mecha Launcher",
  themeDark: "Dark",
  themeLight: "Light",
  themeAriaDark: "Switch to light mode",
  themeAriaLight: "Switch to dark mode",
  statusIdle: "Idle",
  statusRunning: "Running",
  kicker: "Minecraft only",
  runVersion: "Version",
  runInstalled: "Installed",
  runState: "State",
  stateLaunching: "Launching",
  stateReady: "Ready",
  panePreviewSr: "Preview",
  panePreviewTitle: "Preview",
  paneOutputSr: "Output",
  paneOutputTitle: "Output",
  logPlaceholder: "Launch events and process output will appear here.",
  noVersionSelected: "No version selected",
  mcDirNotSet: ".minecraft not set",
  controlsAria: "Launcher controls",
  gameDirectory: "Game directory",
  gameDirectoryHelp:
    "Select the local <code>.minecraft</code> directory that already contains installed versions.",
  minecraftDirAria: "Minecraft directory",
  mcPathPlaceholder: "C:\\Users\\you\\AppData\\Roaming\\.minecraft",
  browse: "Browse",
  reload: "Reload",
  reloading: "Reloading",
  detectedDefault: "Detected default:",
  installedVersions: "Installed versions",
  versionsEmptyTitle: "No valid local versions found yet.",
  versionsEmptyDetail:
    "Each version must include matching <code>.json</code> and <code>.jar</code> files.",
  versionsListAria: "Minecraft versions",
  play: "Play",
  launching: "Launching…",
  runtimeAuto: "Runtime auto",
  languageLabel: "Language",
  languageEs: "Español",
  languageEn: "English",
  catSceneAria: "Rotating 3D preview",
  logSourceErr: "ERR",
  logSourceOut: "OUT",
  logSourceSys: "SYS",
  errSetDirBeforeReload: "Set a .minecraft directory before reloading versions.",
  statusWaitingDir: "Waiting for a valid .minecraft directory.",
  statusNoVersionsInDir: "No local versions were found in the selected directory.",
  versionsReadyOne: "1 version ready.",
  versionsReadyMany: "{count} versions ready.",
  errReadVersions: "Failed to read installed versions.",
  statusDirInvalid: "The selected directory could not be used.",
  logReloadedVersions: "Reloaded versions from {path}",
  logLaunchRequested: "Launch requested for {id}",
  errFolderPicker: "The folder picker failed.",
  errFailedLaunch: "Failed to start Minecraft.",
  errLaunchFailed: "Launch failed.",
  statusPreparingLaunch: "Preparing launch plan.",
  launchState: "Launch state: {state}",
  errInitLauncher: "Failed to initialize launcher.",
  runSummaryAria: "Launch summary"
};

export function translate(locale: Locale, key: MessageKey): string {
  return locale === "en" ? enMessages[key] : esMessages[key];
}

export function fillTemplate(template: string, vars: Record<string, string | number>): string {
  let out = template;
  for (const [name, value] of Object.entries(vars)) {
    const token = `{${name}}`;
    const replacement = String(value);
    out = out.split(token).join(replacement);
  }
  return out;
}

export function readStoredLocale(): Locale {
  if (typeof localStorage === "undefined") {
    return "es";
  }

  const raw = localStorage.getItem(LOCALE_STORAGE_KEY);
  return raw === "en" ? "en" : "es";
}

export function persistLocale(locale: Locale): void {
  if (typeof localStorage === "undefined") {
    return;
  }

  localStorage.setItem(LOCALE_STORAGE_KEY, locale);
}

export function applyDocumentLang(locale: Locale): void {
  if (typeof document === "undefined") {
    return;
  }

  document.documentElement.lang = locale === "en" ? "en" : "es";
}
