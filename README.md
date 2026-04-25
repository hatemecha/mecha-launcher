# mecha-launcher

Launcher de Minecraft hecho con **Tauri + Svelte**, enfocado en:

- Instalar y gestionar versiones (vanilla y variantes).
- Resolver dependencias (por ejemplo Java) según la versión seleccionada.
- Mantener un flujo simple: elegir versión → instalar (si hace falta) → jugar.

> Este proyecto **no está afiliado** con Mojang/Microsoft. Minecraft es una marca registrada de sus respectivos dueños.

## Requisitos

- Node.js (recomendado: LTS)
- Rust toolchain (para Tauri)
- Dependencias del sistema para compilar Tauri (GTK/WebKit, etc. según tu distro)

## Desarrollo

Instalar dependencias:

```bash
npm install
```

Levantar en modo dev:

```bash
npm run tauri:dev
```

## Tests / checks

```bash
npm run test:frontend
npm run build
```

## Estructura rápida

- `src/`: UI (Svelte)
- `src-tauri/`: backend nativo (Rust/Tauri)
- `public/`: assets servidos por la app
- `minecraft-1.16.5-redux/`: pack local (metadata + mods) para la variante Redux.

## Notas

Documentación interna y seguimiento: `docs/`.

