<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Bundled scenes — `examples/` subdirectory

This subdirectory holds minimal demonstration scenes for each authoring
feature (multi-pass, script, transitions, media, audio-reactive) per
`docs/38-scene-library-and-import-spec.md` §2.1. It ships in the Tauri
AppBundle at `<AppBundle>/Resources/scenes/examples/` and is referenced
from the embedded docs (`docs/adr/ADR-010-embedded-docs-generation.md`).

## Status

Empty placeholder. The examples corpus is authored across future
Phase-4b waves; the taxonomy directory is established now per
`EV-LIBRARY-BUNDLE-LAYOUT` (`docs/26-evidence-artifact-registry.md` row 101)
so the four-subdirectory layout exists at Wave 4b.1 closure even though the
example scenes are not yet authored.

## Planned content

Per `docs/38` §2.1 cross-reference to the embedded-docs generation flow,
candidate examples include:

- `multipass-chain.chroma` — pass chaining demo (a clean-room equivalent
  of the existing T2 fixture under the parent `scenes/` corpus).
- `transition-hard.chroma` — hard transition sentinel demo.
- `reactive-audio.chroma` — audio-reactive bindings demo.
- `meta-controls.chroma` — `IS_META: true` ingestion demo.

Examples surfaced here are linked from the embedded docs per
ADR-010 once the embedded-docs generator lands.

When scenes land here the walk in
`app/shell/src/commands/library.rs::list_bundled_scene_packages` will
surface them alongside the `library/` corpus with the same
`SceneSource::Bundled` tag.
