<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Bundled scenes — `library/` subdirectory

This subdirectory holds the canonical bundled scene corpus that ships in the
Tauri AppBundle at `<AppBundle>/Resources/scenes/library/` per
`docs/38-scene-library-and-import-spec.md` §2.1. These are clean-room
Chromaticity-original scenes per
`docs/29-cleanroom-legal-policy-baseline.md`.

## Current corpus

- `demo-kaleido.chroma` — geometric reference scene exercising slider, knob,
  toggle, color, and xy controls.
- `demo-spectrum.chroma` — audio-reactive reference scene carrying the
  `audio` tag for library-scanner coverage.
- `demo-text.chroma` — text-category reference scene carrying the `TEXT`
  category for library-scanner coverage.

## Sibling subdirectories

The four-subdirectory bundled scenes taxonomy per `docs/38` §2.1 is:

- `library/` — the full bundled scene corpus (this directory).
- `welcome/` — onboarding scene(s) shown on first launch.
- `examples/` — minimal demonstration scenes for each authoring feature.
- `shadertoy-fixtures/` — Shadertoy iChannel fixture pack inputs.

The walk in `app/shell/src/commands/library.rs::list_bundled_scene_packages`
recurses into `library/`, `welcome/`, and `examples/`; `shadertoy-fixtures/`
is intentionally skipped since it carries import-test asset inputs, not
playable scenes.
