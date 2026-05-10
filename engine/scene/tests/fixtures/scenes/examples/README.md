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

Wave 1d-β PR 1 (2026-05-09) seeded this subdirectory with its first
clean-room scene under `EV-LIBRARY-CORPUS-MIN`. The subdirectory was
established empty at Wave 4b.1 closure under
`EV-LIBRARY-BUNDLE-LAYOUT` (`docs/26-evidence-artifact-registry.md`).

## Scenes

- `audio-reactive-demo.chroma` — minimal audio-reactive example with
  four named band-gain sliders (`bass_gain`, `low_mid_gain`,
  `high_mid_gain`, `treble_gain`), a `pulse` knob, and a `tint` color
  control. Author-facing demonstration of the chromaticity-native
  audio-reactivity surface that the Wave 1c-α-2 production audio
  bootstrap drives at runtime; binding to the producer-side
  `chroma_AudioRMS` / `chroma_AudioPeak` / `chroma_AudioFFT[8]`
  uniforms per `docs/36` §2.11 lands when the per-pass shader pipeline
  for example scenes ships in a follow-up wave.

## Planned content

Per `docs/38` §2.1 cross-reference to the embedded-docs generation flow,
additional candidate examples in PR 2+ scope include:

- `multipass-chain.chroma` — pass chaining demo (a clean-room equivalent
  of the existing T2 fixture under the parent `scenes/` corpus).
- `transition-hard.chroma` — hard transition sentinel demo.
- `meta-controls.chroma` — `IS_META: true` ingestion demo.

Examples surfaced here are linked from the embedded docs per
ADR-010 once the embedded-docs generator lands.

## Clean-room provenance

Every scene in this subdirectory is authored from scratch using only
the chromaticity-internal scene schema (`engine/scene/src/schema.rs`),
the chromaticity-native uniform vocabulary in
`docs/36-reactive-symbol-catalog.md` (`chroma_*` namespace per the
governance line at §Namespace governance), ADRs, and
`docs/29-cleanroom-legal-policy-baseline.md` §Authoring as references.
Scenes are NOT derived from `Contents/` or any other reference-app
material per the clean-room SOP.

## Walk semantics

Scenes that land here surface through the walk in
`app/shell/src/commands/library.rs::list_bundled_scene_packages`
alongside the `library/` corpus with the same `SceneSource::Bundled`
tag.
