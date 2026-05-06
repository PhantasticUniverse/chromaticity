<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Bundled scenes — `welcome/` subdirectory

This subdirectory holds the one-scene welcome package shown on first launch
per `docs/38-scene-library-and-import-spec.md` §2.1. It ships in the Tauri
AppBundle at `<AppBundle>/Resources/scenes/welcome/`.

## Status

Empty placeholder. The welcome scene is authored as a future Phase-4b work
item (paired with the onboarding view per
`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/onboard.jsx`); the
taxonomy directory is established now per
`EV-LIBRARY-BUNDLE-LAYOUT` (`docs/26-evidence-artifact-registry.md` row 101)
so the four-subdirectory layout exists at Wave 4b.1 closure even though the
scene is not yet authored.

## Planned content

- One `.chroma` package whose copy and visual establishes the on-first-launch
  experience: introduces the audio reactivity, the snap-slot model, and a
  short prompt that points the operator at the Library view.
- Clean-room original per `docs/29-cleanroom-legal-policy-baseline.md`.

When a scene lands here the walk in
`app/shell/src/commands/library.rs::list_bundled_scene_packages` will surface
it alongside the `library/` corpus with the same `SceneSource::Bundled` tag.
