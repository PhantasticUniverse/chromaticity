<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Welcome to Chromaticity

Chromaticity is a deterministic, audio-reactive live-visuals runtime and authoring
environment for macOS and Windows desktops. It plays back **scenes** — small,
self-contained packages that pair a GLSL render program with reactive uniforms
driven by an audio analysis snapshot — and lets you author, organize, and
sequence them on stage or in the studio.

This page is the first stop for someone who has just installed Chromaticity and
wants to understand the shape of the system before touching anything.

## Who Chromaticity is for

- **Live visualists** running visuals against incoming audio (DJ booth feed,
  line-in, virtual loopback) who want determinism, low-latency reaction, and
  predictable scene lifecycles.
- **Scene authors** writing GLSL fragments, wiring controls to uniforms, and
  publishing reusable scene packages.
- **Operators** who want hardware control surfaces (MIDI, OSC) bound to scene
  parameters and playlist progression with reproducible behaviour.

If you are not running visuals against audio, Chromaticity is probably not the
right fit — see [What Chromaticity is not](#what-chromaticity-is-not).

## Core principles

1. **Deterministic runtime.** Given the same scene, the same audio input, and
   the same configuration, Chromaticity produces the same visible output.
   Determinism is normative, not a nice-to-have; it is locked in
   `06-audio-engine.md` §7.13 and validated by the `EV-AUDIO-TRACE` evidence
   row in `26-evidence-artifact-registry.md`.
2. **Audio analysis is normative.** Audio enters the system once, is analysed
   on a single thread at a fixed cadence, and is published into a versioned
   snapshot. Every shader pass in a frame observes the same snapshot. The
   numeric contract for every reactive symbol is locked in `06-audio-engine.md`
   §7 and `36-reactive-symbol-catalog.md`.
3. **Scene authoring is the work surface.** A scene is a `.chroma` directory
   containing a `scene.json` manifest and (optionally) GLSL fragment programs.
   The schema, the validator, and the loader are the surfaces you interact
   with as an author. See `02-scene-format.md` for the manifest shape.
4. **Clean-room originals only.** Every shipped Chromaticity asset, helper
   library, scene, and document is authored by the Chromaticity team from
   first principles. See `29-cleanroom-legal-policy-baseline.md`.

## What you can do today (Phase 1)

- **Author scenes.** Write a `.chroma` directory with a `scene.json` manifest.
  The slice-3a schema in `engine/scene/src/schema.rs` accepts the
  reference-compatible field set (`SCHEMA_VERSION`, `TITLE`, `CREDIT`,
  `DESCRIPTION`, `TAGS`, `CATEGORIES`, `CONTROLS`, `PASSES`, plus parsed-and-
  reserved roots like `HARD_TRANSITIONS` / `DATA_SOURCES`).
- **Bind reactive symbols.** Per-pass `UNIFORMS` entries map a scene-author-
  declared binding name to a reactive-symbol catalog entry — see
  `03-standard-uniforms.md` for the available set.
- **Use built-in controls.** `slider`, `toggle`, `color`, `xy`, `bang`,
  `dropdown`, and `knob` controls are accepted by the slice-3a parser; their
  defaults, ranges, and metadata round-trip through the schema.
- **Multi-pass scenes.** Declare a `PASSES` array with named `TARGET` nodes;
  the validator enforces node-ID uniqueness across passes.
- **Browse the demo corpus.** The `engine/scene/tests/fixtures/scenes/`
  directory ships clean-room demo scenes you can pattern-match against.
- **Walk through the UI.** Phase-1 EXIT (commit `e818752`) shipped the app
  shell with Perform / Library / Author / Settings views; W13–W34 waves added
  Playlist v2, MIDI / OSC / Audio routed views, and the entitlement-gated
  adapter UI suite.

## What Chromaticity is not

- **Not a video editor.** There is no timeline, no clip arrangement, no
  non-real-time render pipeline. Output is generated frame-by-frame.
- **Not a DAW.** Chromaticity reads audio (line-in, loopback, file source);
  it does not produce or arrange audio. There is no sequencer, no synth,
  no audio-effects chain past the analysis pipeline in `06-audio-engine.md`.
- **Not a colour-management tool.** "Chromaticity" the colour-science term
  is unrelated; this product targets live visuals.
- **Not a shader playground hosted on the web.** Chromaticity ships as a
  signed desktop binary per `adr/ADR-014-signing-notarization-auto-update.md`;
  scene authoring is local and offline-friendly.

## What is deferred (Phase 2 and beyond)

The following surfaces are scoped but not yet implemented end-to-end. Refer
to `26-evidence-artifact-registry.md` and `13-risk-register.md` for the
authoritative status of each.

- **Full transition execution.** `HARD_TRANSITIONS` and `SMOOTH_TRANSITIONS`
  blocks are parse-reserved at slice-3a (they round-trip through
  `Scene::extensions`); the lifecycle state machine lands in slice 5b.
- **External-device audio capture.** The `cpal` device-capture pipeline lands
  with the audio engine slice; today the analysis pipeline runs against the
  documented spec in `06-audio-engine.md` §7.
- **Real-time external data ingestion.** The `chroma_Data_*` namespace is
  reserved and parse-aware but unpopulated at MVP per
  `36-reactive-symbol-catalog.md` §1.6 and `41-data-ingestion-differentiator.md`.
- **`.chromaPack` zip transport.** Today scenes load from `.chroma`
  directories; zip-packaged scenes land alongside the slice-4 loader.
- **Shadertoy / ISF import.** Tracked under `EV-IMPORT-SHADERTOY` in
  `26-evidence-artifact-registry.md`.
- **Script Host (`script.js`) execution.** The QuickJS sandbox per
  `adr/ADR-012-script-sandbox-engine.md` is in flight; scene-side script files
  are parse-aware but not executed at slice-3a.

If a feature is not in [What you can do today](#what-you-can-do-today-phase-1)
and not listed above, treat it as undocumented and check the registry.

## Read next

- **Want to make a scene right now?** Jump to
  `04-first-scene-tutorial.md` — it walks you from "I have Chromaticity
  installed" to "my visuals pulse on the beat" in one sitting.
- **Want to understand the file format?** `02-scene-format.md` lays out the
  `.chroma` manifest with a minimal example.
- **Want the full uniform catalogue?** `03-standard-uniforms.md` is the
  first-time author's rendering of `36-reactive-symbol-catalog.md` v1.1.0.
- **Going deep:**
  - `06-audio-engine.md` — audio analysis and uniform-generation contract.
  - `36-reactive-symbol-catalog.md` — normative reactive-symbol catalog.
  - `13-risk-register.md` — risk register; useful for power users planning
    around in-flight surfaces.
  - `43-development-standards.md` — engineering standards for contributors
    (commits, signing, evidence tags).
  - `adr/ADR-001-runtime-model.md` and `adr/ADR-018-non-scene-ui-toolkit.md`
    — runtime and UI-stack decisions.

## Getting help

- **Documentation index:** `docs/README.md`.
- **Bug reports / feature requests:** the GitHub issue tracker for the
  Chromaticity repository (see `CONTRIBUTING.md`).
- **Operations and privacy:** `31-supportability-privacy-ops-policy.md`
  covers support bundles, crash payloads, and what leaves your machine.
