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
  depsTitle: "Dependencias",
  depsJavaTitle: "Java",
  depsJavaOk: "Java detectado (v{major}).",
  depsJavaNotRecommended: "Java detectado (v{major}), pero para Minecraft/OptiFine se recomienda Java {recommended} o superior.",
  depsJavaMissing: "Java no está instalado o no está en PATH.",
  depsJavaHowTo: "Cómo instalar",
  depsJavaLinuxNote: "Ejecuta estos comandos en tu terminal:",
  depsJavaWindowsNote: "Descarga e instala desde:",
  depsJavaAutoInstall: "Instalar Java automáticamente",
  depsJavaInstalling: "Instalando Java…",
  depsJavaInstalledOk: "Java instalado. Reinicia el launcher si sigue sin detectarse.",
  depsJavaWingetAlreadyCurrent:
    "Java (Temurin 21) ya estaba instalado; winget no encontró una versión más nueva. Si el launcher no detecta Java, reinicia el launcher o la sesión de Windows.",
  depsJavaInstallFailed: "La instalación automática falló.",
  depsGraphicsTitle: "Compatibilidad gráfica Linux",
  depsGraphicsOk: "xrandr detectado. Minecraft 1.8.9/LWJGL 2 puede leer modos de pantalla.",
  depsGraphicsMissing: "Falta xrandr. Minecraft 1.8.9/LWJGL 2 suele crashear sin esta herramienta.",
  depsGraphicsNotUsable:
    "xrandr está instalado, pero no devolvió pantallas conectadas. Revisa X11/XWayland.",
  depsGraphicsAutoInstall: "Instalar compatibilidad gráfica",
  depsGraphicsInstalling: "Instalando compatibilidad…",
  depsGraphicsInstalledOk: "Compatibilidad gráfica instalada.",
  depsGraphicsInstallFailed: "No se pudo instalar la compatibilidad gráfica.",
  kicker: "Solo Minecraft",
  runVersion: "Versión",
  runInstalled: "Instaladas",
  runState: "Estado",
  stateLaunching: "Iniciando",
  stateInstalling: "Instalando",
  stateReady: "Listo",
  panePreviewSr: "Vista previa",
  panePreviewTitle: "Vista previa",
  paneOutputSr: "Salida",
  paneOutputTitle: "Salida",
  logPlaceholder: "Aquí aparecerán los eventos y la salida del proceso.",
  copyLog: "Copiar log",
  copyLogDone: "Copiado",
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
  optifineInstallerTitle: "Instalar OptiFine",
  optifineInstallerHelp:
    "Descarga Minecraft, assets, librerías, runtime de Java y OptiFine desde el launcher.",
  optifineInstall: "Instalar",
  optifineInstalling: "Instalando…",
  optifineInstalled: "Instalado",
  optifineRepair: "Reparar",
  optifineSelect: "Seleccionar",
  optifineJava: "Java {major}",
  optifineSource: "Fuente oficial",
  optifineProgressAria: "Progreso de instalación OptiFine",
  optifineInstallNeedDir: "Indica una carpeta .minecraft antes de instalar OptiFine.",
  optifineInstallDone: "{id} instalado y seleccionado.",
  optifineInstallFailed: "No se pudo instalar OptiFine.",
  installedVersions: "Versiones instaladas",
  versionsCatalogTitle: "Versiones",
  versionsFilterInstalled: "Instaladas",
  versionsFilterAll: "Todas",
  versionsFilterPopular: "Populares",
  versionsFilterFavorites: "Favoritas",
  versionsFilterOptifine: "OptiFine",
  versionsFilterVanilla: "Vanilla",
  versionActionDownload: "Descargar",
  versionActionDelete: "Borrar",
  versionDeleteConfirm: "¿Borrar la versión instalada {id}? Esto eliminará la carpeta versions/{id}.",
  versionDeleteConfirmWithOptifine:
    "¿Borrar la versión instalada {id}? También se borrarán estas versiones OptiFine: {optifine}. Esto eliminará las carpetas dentro de versions/.",
  versionDownloading: "Descargando…",
  versionDeleting: "Borrando…",
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
  createVersionsDir: "Crear carpeta versions",
  creatingVersionsDir: "Creando carpeta…",
  versionsDirCreated: "Carpeta creada: {path}",
  errCreateVersionsDir: "No se pudo crear la carpeta de versiones.",
  errFailedLaunch: "No se pudo iniciar Minecraft.",
  errInstallInProgress: "Espera a que termine la instalación antes de jugar.",
  errCopyLog: "No se pudo copiar el log.",
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
  depsTitle: "Dependencies",
  depsJavaTitle: "Java",
  depsJavaOk: "Java detected (v{major}).",
  depsJavaNotRecommended: "Java detected (v{major}), but Minecraft/OptiFine typically needs Java {recommended} or newer.",
  depsJavaMissing: "Java is not installed or not in PATH.",
  depsJavaHowTo: "How to install",
  depsJavaLinuxNote: "Run these commands in your terminal:",
  depsJavaWindowsNote: "Download and install from:",
  depsJavaAutoInstall: "Auto-install Java",
  depsJavaInstalling: "Installing Java…",
  depsJavaInstalledOk: "Java installed. Restart the launcher if it is still not detected.",
  depsJavaWingetAlreadyCurrent:
    "Java (Temurin 21) was already installed; winget reported no newer version. Restart the launcher or sign out of Windows if Java is still not detected.",
  depsJavaInstallFailed: "Automatic installation failed.",
  depsGraphicsTitle: "Linux graphics compatibility",
  depsGraphicsOk: "xrandr detected. Minecraft 1.8.9/LWJGL 2 can read display modes.",
  depsGraphicsMissing: "xrandr is missing. Minecraft 1.8.9/LWJGL 2 often crashes without it.",
  depsGraphicsNotUsable:
    "xrandr is installed, but it did not report connected displays. Check X11/XWayland.",
  depsGraphicsAutoInstall: "Install graphics compatibility",
  depsGraphicsInstalling: "Installing compatibility…",
  depsGraphicsInstalledOk: "Graphics compatibility installed.",
  depsGraphicsInstallFailed: "Failed to install graphics compatibility.",
  kicker: "Minecraft only",
  runVersion: "Version",
  runInstalled: "Installed",
  runState: "State",
  stateLaunching: "Launching",
  stateInstalling: "Installing",
  stateReady: "Ready",
  panePreviewSr: "Preview",
  panePreviewTitle: "Preview",
  paneOutputSr: "Output",
  paneOutputTitle: "Output",
  logPlaceholder: "Launch events and process output will appear here.",
  copyLog: "Copy log",
  copyLogDone: "Copied",
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
  optifineInstallerTitle: "Install OptiFine",
  optifineInstallerHelp:
    "Downloads Minecraft, assets, libraries, Java runtime, and OptiFine from the launcher.",
  optifineInstall: "Install",
  optifineInstalling: "Installing…",
  optifineInstalled: "Installed",
  optifineRepair: "Repair",
  optifineSelect: "Select",
  optifineJava: "Java {major}",
  optifineSource: "Official source",
  optifineProgressAria: "OptiFine installation progress",
  optifineInstallNeedDir: "Set a .minecraft directory before installing OptiFine.",
  optifineInstallDone: "{id} installed and selected.",
  optifineInstallFailed: "Failed to install OptiFine.",
  installedVersions: "Installed versions",
  versionsCatalogTitle: "Versions",
  versionsFilterInstalled: "Installed",
  versionsFilterAll: "All",
  versionsFilterPopular: "Popular",
  versionsFilterFavorites: "Favorites",
  versionsFilterOptifine: "OptiFine",
  versionsFilterVanilla: "Vanilla",
  versionActionDownload: "Download",
  versionActionDelete: "Delete",
  versionDeleteConfirm: "Delete installed version {id}? This will remove versions/{id}.",
  versionDeleteConfirmWithOptifine:
    "Delete installed version {id}? This will also delete these OptiFine versions: {optifine}. This removes folders under versions/.",
  versionDownloading: "Downloading…",
  versionDeleting: "Deleting…",
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
  createVersionsDir: "Create versions folder",
  creatingVersionsDir: "Creating folder…",
  versionsDirCreated: "Folder created: {path}",
  errCreateVersionsDir: "Failed to create the versions folder.",
  errFailedLaunch: "Failed to start Minecraft.",
  errInstallInProgress: "Wait until installation finishes before playing.",
  errCopyLog: "Failed to copy the log.",
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
