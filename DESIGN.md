# Mecha Launcher Design Notes

## Product Direction
Mecha Launcher is a desktop-first Minecraft launcher with a retro control-room look. The interface is dense but readable, optimized for quick local actions: pick a directory, inspect dependencies, manage versions, and launch.

## Visual Language
- Base palette: charcoal and near-black backgrounds with a single saturated red accent.
- Typography: system sans for UI text and monospace for labels, logs, and compact metadata.
- Tone: utilitarian, slightly industrial, never playful or glossy.
- Surfaces: flat panels with thin borders, minimal shadows, and compact spacing.

## Core UI Rules
- Keep the window split into two zones: large preview/stage area and narrow control sidebar.
- Preserve uppercase micro-labels for status metadata and compact section headers.
- Use red only for emphasis, active state, or warnings that matter.
- Prefer direct buttons and explicit status text over decorative empty space.
- Any 3D preview must degrade to a visible textual fallback if rendering fails.

## Components
- Header bar: compact, fixed, low-noise, with launcher state and theme toggle.
- Stage panels: bordered preview modules for mascot scene, player skin preview, and logs.
- Dependency cards: actionable guidance with commands or links, never passive warnings only.
- Version catalog: dense list with filters, favorite toggle, install/delete actions, and clear Java requirement hints.
- Log console: monospace, scrollable, copyable, and safe for repeated launcher output.

## Interaction Principles
- The launcher must stay usable offline for local versions.
- Remote catalog failures should degrade to logs, not block local bootstrap.
- Status copy should be short and operational.
- Dependency checks should reflect the selected version, not a global Java guess.
- File inputs and destructive actions must validate and confirm explicitly.

## Responsive Notes
- Desktop is the primary target.
- On smaller widths, preserve the sidebar workflow before trying to invent new layouts.
- Avoid large hero-style scaling; prioritize preserving controls and log readability.
