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
clean-room scene under `EV-LIBRARY-CORPUS-MIN`. Wave 1d-β PR 2
(2026-05-10) extends the corpus with seven more feature-demonstration
scenes spanning audio reactivity, OSC routing, hotkey-bound preset
cycling, control-widget coverage, and playlist transitions, closing
`EV-LIBRARY-CORPUS-MIN` to `Pass`. The subdirectory was established
empty at Wave 4b.1 closure under `EV-LIBRARY-BUNDLE-LAYOUT`
(`docs/26-evidence-artifact-registry.md`).

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
- `bass-pulse.chroma` — single-band bass-driven pulse demo. A
  bass-gain slider controls inbound audio sensitivity; a pulse-curve
  knob shapes the response envelope; attack and release sliders govern
  the post-strike settle. Demonstrates the chromaticity-native
  audio-reactivity surface at its narrowest single-band cross-section.
- `four-band-reactive.chroma` — four-band audio split (low, low-mid,
  high-mid, high) feeding distinct color tiles, demonstrating the
  chromaticity-native 8 -> 4 reduction the AudioReactivity widget
  consumes. Per-band saturation and per-band lift controls let authors
  trim each band independently.
- `uniform-palette.chroma` — authoring-side palette example exercising
  every Inspector widget kind in one scene (slider, knob, toggle,
  color, xy, dropdown). Useful as a reference fixture when wiring new
  control types or smoke-testing widget rendering.
- `osc-listener.chroma` — single-channel OSC listener example with
  three controls routable from external OSC senders under the
  chromaticity-native `/chroma/` route prefix. Author-facing fixture
  demonstrating how OSC-bound controls compose into a minimal
  remote-driven scene.
- `osc-grid.chroma` — multi-cell OSC grid example. Sixteen scalar
  cells expressed as four routed group sliders plus four toggle gates
  form a coarse 4x4 mapping surface remote OSC senders can drive
  simultaneously. Pairs with grid-routing chrome and OSC route-prefix
  collision exercises.
- `hotkey-cycle.chroma` — preset cycling scene driven by hotkey-bound
  bang controls. Four named bang triggers (next, previous, reset,
  randomize) advance a four-step preset selector dropdown that
  downstream cues can read. Demonstrates the hotkey -> bang ->
  selector pattern.
- `transition-fade.chroma` — smooth-transition example pairing a fade
  ratio slider with a duration knob and a cue-curve dropdown (linear,
  ease-in, ease-out, ease-in-out). Exercises the smooth-transition
  cue-curve selection surface that the CueTransitionCurve widget
  renders.
- `transition-cut.chroma` — hard-transition example with an
  instantaneous cut between an A-state and a B-state, gated by a state
  toggle and paired with a guard delay slider and an emphasis slider
  for the post-cut hold. Demonstrates the hard-transition CP-09
  ingestion path.

## Planned content

Per `docs/38` §2.1 cross-reference to the embedded-docs generation
flow, future candidate examples include richer per-pass shader
pipelines once the per-pass widening of these example scenes ships:

- `multipass-chain.chroma` — pass chaining demo (a clean-room
  equivalent of the existing T2 fixture under the parent `scenes/`
  corpus).
- `transition-hard.chroma` — hard transition sentinel demo with
  structured `HARD_TRANSITIONS` execution.
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
