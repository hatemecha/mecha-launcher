# Revision del proyecto `mecha-launcher`

## Summary
La revision anterior quedo desactualizada. En el estado actual ya estan resueltos varios hallazgos fuertes: bootstrap offline separado de catalogos remotos, politica de Java basada en la version seleccionada, CSP explicita con fuente local, parser de OptiFine ordenado por build, versiones locales/custom con tipo propio, validacion de skins, fallbacks visuales, pruebas frontend, limpieza de artefactos sobrantes y documentacion de diseno mas alineada al producto.

En esta pasada tambien se corrigio una regresion real: si faltaba el runtime Java declarado por el manifest, el launcher podia caer silenciosamente a `java` del sistema. Ahora falla de forma accionable.

## Verificacion realizada
- `npm run test:frontend`: OK.
- `npx tsc --noEmit`: OK.
- `cargo test --target-dir target-review`: OK.
- `npm run build`: OK. Solo quedaron warnings de Vite por chunks grandes (`three` + asset principal), pero el build completa correctamente.

## Cerrado desde la revision anterior
- **Bootstrap local ya no depende de OptiFine o catalogos remotos.** Evidencia: `src/App.svelte:915-983` hace bootstrap local primero y luego carga catalogos remotos con `Promise.allSettled`.
- **La resolucion de Java ya se alinea con la version seleccionada.** Evidencia: `src/App.svelte:719-767`, `src-tauri/src/commands.rs:184-317,461-541` y `src-tauri/src/launcher/java.rs:11-52`.
- **El launcher ya no usa `PATH` como fallback silencioso cuando falta el runtime empaquetado que declara el manifest.** Evidencia: `src-tauri/src/launcher/java.rs:16-27`, test de regresion en `src-tauri/src/launcher/java.rs` y cobertura de integracion en `src-tauri/tests/launcher_integration.rs:388-399`.
- **La superficie del webview ya no esta abierta como antes.** Evidencia: `src-tauri/tauri.conf.json:12-16`, `src/app.css:1-34`, `public/Minecraft.ttf`; ya no hay `@html` ni fuentes remotas en runtime.
- **El parser de OptiFine ya no depende del primer match del HTML.** Evidencia: `src-tauri/src/launcher/install.rs:268-345`.
- **Las instalaciones locales/custom ya no se etiquetan como `vanilla`.** Evidencia: `src/lib/catalog.ts:7-15,90-109`.
- **La validacion de skins dejo de aceptar cualquier PNG sin limites.** Evidencia: `src/App.svelte:236-286` y `src/lib/storage.ts:91-104`.
- **Ya existe una capa minima de pruebas frontend.** Evidencia: `package.json:8-11` y `scripts/run-frontend-tests.mjs`.
- **Los previews 3D ya degradan con fallback visible.** Evidencia: `src/lib/CatScene.svelte:221-223,333-336` y `src/lib/PlayerScene.svelte:35-41,88-94`.
- **`build.rs` y `DESIGN.md` ya reflejan mejor el producto real.** Evidencia: `src-tauri/build.rs:1-19` y `DESIGN.md:1-36`.
- **Se limpiaron restos sobrantes del repo.** Se borraron `hatecreeper.png`, `2026_04_22_spring-trap-24010173.png` y `minecraft_player_model/`.

## Pendientes reales
- **La localizacion sigue fragmentada.** Impacto: todavia hay copy visible y mensajes tecnicos fuera de la capa comun. Evidencia: `src/lib/catalog.ts:68,82,104` sigue hardcodeando copy de UI; `src-tauri/src/launcher/install.rs:322-327` sigue emitiendo `Stable` / `Preview`; y el backend todavia propaga muchos `error.to_string()` / mensajes en ingles (`src-tauri/src/commands.rs:461-835`, `src-tauri/src/launcher/process.rs:66-95`, `src-tauri/src/launcher/mod.rs:198-214`).
- **`App.svelte` sigue demasiado cargado.** Impacto: mejoro respecto de la revision anterior, pero el archivo todavia concentra bootstrap, dependencias, acciones de catalogo, lanzamiento, logs y ensamblado principal de UI en 1198 lineas. Evidencia: `src/App.svelte`.
- **La logica de deteccion/autoinstalacion de dependencias sigue duplicada.** Impacto: las rutas de Java y graficos repiten decisiones por distro, hints y planes de instalacion en el mismo modulo. Evidencia: `src-tauri/src/commands.rs:184-419`.

## Nota
La seccion "Pendiente Por Sandbox" del informe anterior quedo obsoleta: en este entorno ya pude verificar `npm run build` y borrar los assets binarios sobrantes.

