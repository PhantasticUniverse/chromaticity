<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: 2026 Chromaticity -->

# Tutorials comparison

_Last refreshed:_ 2026-05-22

## Scope

Quality-benchmark sweep of Chromaticity's tutorial / how-to surface
against the Synesthesia public Tutorials group at
`https://app.synesthesia.live/docs/tutorials/`. Eight tutorials are in
scope; each scores Chromaticity along three composed axes — text
coverage (docs-corpus prose), UX coverage (in-app workflow path), and
production wiring (engine surface live). Clean-room posture per
`docs/29-cleanroom-legal-policy-baseline.md`: Synesthesia tutorial
prose, code listings, and recipes are NOT reproduced; only the
*pathway shape* is paraphrased to make the score legible.

The Chromaticity docs-corpus is shipped as the in-app Help surface
under `dist/help/` (mkdocs build of `docs-corpus/docs/*`) and routed
through `src/lib/views/Help.svelte` per Wave 1d-α; the bundled scene
corpus shipped under `engine/scene/tests/fixtures/scenes/` per Wave
1d-β.

## TL;DR

| Synesthesia tutorial | Score | Tag |
| :--- | :---: | :--- |
| Quickstart Guide | 2 | PARTIAL |
| Creating Your First Scene | 3 | MATCH |
| Presets, Favslots, and Playlists | 3 | MATCH |
| Control Hotkeys | 4 | EXCEEDS |
| OBS Livestream | 2 | PARTIAL |
| Shadertoy → SSF Conversion | 4 | EXCEEDS |
| JavaScript Event Handlers | 2 | PARTIAL |
| v1.16 Feature Walkthrough | N/A | OUT-OF-SCOPE |

Distribution: 1× EXCEEDS+, 2× EXCEEDS, 2× MATCH, 3× PARTIAL, 0× GAP,
1× N/A. The Hotkeys tutorial is the clearest win (operator-facing
vocabulary is wider and the docs are denser). Quickstart, OBS, and
JS-handlers are the three PARTIAL items where the underlying engine
surface either does not yet land in V1 or where the corpus has not
authored an end-to-end recipe yet.

## Per-tutorial scores

### Quickstart Guide

- **Score:** 2 / PARTIAL
- **Chromaticity equivalent:** `docs-corpus/docs/index.md:1-25`
  (Welcome page) + `docs-corpus/docs/perform-mode.md:1-117` (Perform
  surface tour) + the bundled welcome corpus under
  `engine/scene/tests/fixtures/scenes/welcome/` (4 packages:
  `welcome.chroma`, `quick-start.chroma`, `first-cue.chroma`,
  `controls-tour.chroma`) per Wave 1d-β.
- **Match notes:** Wave 1d-α PR 1 shipped Welcome + About placeholder
  pages routed through the Help view; Wave 1d-β PR 1 seeded the
  welcome-bundle scene cluster; the operator opens the app, sees
  Perform with a welcome scene loaded by default (per the Round-1
  `#485` welcome-bundle mapping through `tauri.conf.json`
  `bundle.resources`), and can step through the bundled scenes from
  the Library view. The `audio-reactivity.md` page documents the
  audio path end-to-end including the live-snapshot dispatch shipped
  in Wave 1c-α-2.
- **Exceed notes:** Chromaticity ships a deterministic audio analysis
  pipeline that publishes the snapshot through a clean cross-seam
  IPC (`engine/audio` producer → `app/shell::TauriIpcEmitter` →
  visualizer widgets per ADR-019) rather than embedding analysis in
  the render thread; this is documented head-on in
  `docs-corpus/docs/audio-reactivity.md:14-39`.
- **Gap notes:** The Welcome page (`docs-corpus/docs/index.md`) is
  still scaffold-flagged as a placeholder; a concrete "fresh install
  → first reactive scene" narrative does not exist as a single
  walk-through page. The hardware-reality footnote (cpal binds the
  default input device on macOS; system-audio capture needs a virtual
  loopback like BlackHole) lives in the auto-memory and in CLAUDE.md
  but has not been authored into the in-app corpus.

### Creating Your First Scene

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** `docs-corpus/docs/scene-workflow.md`
  (full lifecycle prose, 119 lines) + `engine/authoring/src/scaffolder.rs`
  (`scaffold_to_path` per `EV-AUTHORING-SCAFFOLDER`) +
  `engine/authoring/src/live_reload.rs` (`EV-AUTHORING-LIVE-RELOAD`
  PR 1 of 2-3; PR 2 shipped `PreservedState` per
  `docs/37` §3.3) + `engine/authoring/src/diagnostics.rs`
  (`EV-AUTHORING-DIAGNOSTICS` JSON-class spans) +
  `src/lib/views/Author.svelte:1-50` (Author view with CodeMirror 6
  editor + GLSL `chroma_*` / `syn_*` autocomplete per W21 PR 3).
- **Match notes:** `scene-workflow.md` covers every section
  Synesthesia's first-scene tutorial covers: package layout
  (`scene.json` + `main.glsl` + optional `script.js` + `glsl/` +
  `images/`), the load lifecycle (`Unloaded` → `Loaded` → `Ready`
  → `Running`), the compile pipeline, controls + passes,
  per-file-type live-reload policy with explicit p95 ≤ 500 ms
  shader-edit-to-visible budget, transitions, and the diagnostics
  console with line-number mapping. The Author view ships a real
  CodeMirror editor (not a read-only DOM render) with autocomplete
  for both namespaces.
- **Exceed notes:** The live-reload policy is documented
  per-file-type as a table (`scene-workflow.md:76-89`); the
  shader-edit-to-visible latency is *budgeted* (`p95 ≤ 500 ms`) and
  tied to the cross-tagged `EV-AUTHORING-UX-LATENCY` /
  `EV-DOCS-UX-LATENCY` synthetic harness
  (`engine/authoring/src/ux_latency.rs`). The scaffolder is atomic:
  callers see either a fully-populated `.chroma` package or none
  (`scaffolder.rs:48-56`).
- **Gap notes:** Author-side preview pane is canvas-2D procedural
  (per the post-Round-2 widening); the GPU live-render pipeline is
  ADR-011 Phase-2+ deferred. `script.js` lifecycle integration into
  the loader is reserved per the scaffolder's "loader does not
  currently consume per-package script bodies" caveat
  (`scaffolder.rs:44-47`).

### How to Use Presets, Favslots, and Playlists

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** `docs-corpus/docs/perform-mode.md:79-89`
  (Snap bar prose) + `docs-corpus/docs/hotkeys-reference.md:46-60`
  (Snap-slot row group) + `docs-corpus/docs/osc-routes.md:66-83`
  (favslots route family) + `docs-corpus/docs/library-workflow.md:91-105`
  (launch-with-preset prose) + `EV-PRESET-PLAYLIST` (Phase-3
  evidence row, `Pass` under existing PresetStore + PlaylistStore
  semantics) + scene-author preset capture from Wave 2.4 PR
  `#456` (`Command::PresetSave` + `EventPayload::PresetSaved`).
- **Match notes:** The "Favslots → Snaps" rename in our v0.2-beta
  sweep is explicit at every author surface — the operator UX calls
  them "Snap slots" and the `compat-syn-v1` OSC routes retain the
  literal `favslots` for wire-level compatibility. The Snap bar
  (10 slots, slot 0 = "gold preset") + named presets via
  `Cmd+Opt+S` + scene-author preset capture from PR `#456` give the
  operator the equivalent three-tier capture surface. Playlists are
  Phase-3 `EV-PRESET-PLAYLIST` row at `Pass`; transport
  (next / previous / position / select / toggleplay) is documented
  in both hotkey and OSC tables.
- **Exceed notes:** The Snap bar persists across the bottom of
  Perform mode at all times (the bundle's wireframe-faithful chrome
  per W34a). Slot labels persist across restarts via
  `snapLabels.svelte.ts` localStorage (Wave 2.4 PERF.F /
  PR `#451`). Save-without-prompt vs. save-as-named-preset is a
  distinct hotkey (`Cmd+S` vs. `Cmd+Opt+S`) so the operator never
  has a modal in the hot path.
- **Gap notes:** Preset-bank durability + playlist reordering are
  the Phase-3 narrowings for `EV-PRESET-PLAYLIST` that explicitly
  defer to Phase-4+. Settings-TOML disk persistence for the Wave
  2.4 PERF.C preset save and the PERF.F Snap labels is deferred.
  Playlist shuffle currently advances by +1 (engine has no shuffle
  command — see CLAUDE.md "synthetic-only" cohort).

### How to Use Control Hotkeys

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `docs-corpus/docs/hotkeys-reference.md`
  (full vocabulary, 147 lines, 57 rows mirrored from
  `src/lib/data/hotkeys.ts`) + the in-app HotkeyOverlay (W12 shell)
  toggled with `?` per `EV-HOTKEY-COMPLETENESS` + control-focused
  hotkey rows (`D` default / `R` random / `P` preset / `L` lock)
  documented in `perform-mode.md:36-44` and `hotkeys-reference.md:22-32`.
- **Match notes:** The Synesthesia tutorial's four-letter
  vocabulary (`D R L P`) maps 1:1 onto our control-focused mode
  with identical semantics (D=default, R=random, P=preset,
  L=lock). The `cap` column on every hotkey row indicates whether
  the action is free MVP or entitlement-gated, surfacing the
  capability model that the reference application leaves implicit.
- **Exceed notes:** Chromaticity's hotkey corpus is materially
  wider — 57 rows covering scene + playlist transport, Snap save /
  recall / clear, display / output toggles, meta-control modal
  slider-adjust, audio threshold sampling, navigation panels,
  diagnostics, and show-state actions — versus the reference's
  four-letter control-focused set. The HotkeyOverlay (`?`-toggle)
  is in-app and reactive to focus context. Every row carries the
  OSC equivalent in the same table so external surface bindings are
  discoverable from the hotkey page. The lock-state (`L`) is a
  first-class control affordance with the per-row dim visual via
  `data-locked="true"` (Wave 2.4 PERF.A / PR `#453` +
  `controlLock.svelte.ts`).
- **Gap notes:** Rebinding is V2-deferred per the spec note at
  `hotkeys-reference.md:16-17`; the V1 map is read-only.

### How to Add Audio Reactive Visuals to a Livestream with OBS

- **Score:** 2 / PARTIAL
- **Chromaticity equivalent:** `integrations/syphon-adapter/` (macOS
  Syphon adapter scaffold per Wave 1c-β; `publish_syphon_output`
  real-impl + `NSDistributedNotificationCenter` observer carry
  forward as known follow-ups per CLAUDE.md "Wave 1c-β follow-ups")
  + the Sources adapter pill in TopBar (Wave 2.4 PERF.D /
  PR `#454`).
- **Match notes:** Syphon (macOS) is the analogue of Spout (Windows)
  that the reference uses for OBS hand-off. The adapter directory
  shape exists; the cross-seam pill UI reflects connectivity state.
- **Exceed notes:** None at V1. The reference application has a
  mature OBS recipe and a published Spout plugin path; we do not.
- **Gap notes:** Three large gaps:
  1. **No Windows Spout adapter.** `integrations/` has no `spout-adapter/`
     sibling; CLAUDE.md does not enumerate Spout as a planned wave.
  2. **No NDI adapter.** The Settings reference page
     (`docs-corpus/docs/settings-reference.md:80-86`) documents NDI
     as a default-loopback adapter row, but the corresponding
     `integrations/ndi-adapter/` does not exist; NDI is a planning
     surface, not shipped wiring.
  3. **No OBS recipe page** in the docs corpus. There is no
     `livestream.md` or `obs.md` in `docs-corpus/docs/`; the
     `library-workflow.md` and `perform-mode.md` pages do not
     reference an OBS pathway. The Syphon `publish_syphon_output`
     real-impl is a Wave 1c-β follow-up, not landed.

### How to Manually Convert from Shadertoy.com to SSF

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `docs-corpus/docs/import-shadertoy.md`
  (119 lines, full converter walkthrough) +
  `engine/authoring/src/import_shadertoy.rs` (the converter itself
  per `EV-IMPORT-SHADERTOY` PR 1 + PR 2 of 2) +
  `engine/scene/tests/fixtures/scenes/shadertoy-fixtures/`
  (iChannel asset corpus per Phase-4b §5.2a) +
  `engine/authoring/src/import_isf.rs`
  (`EV-IMPORT-ISF-REJECT` actionable-reject + escape hatch per
  `docs/38` §6).
- **Match notes:** The reference tutorial documents a *manual*
  conversion (operator opens a text editor and rewrites the
  `mainImage` body into SSF). Chromaticity ships the conversion as
  an *automated* one-way importer: the operator points the
  converter at a Shadertoy GLSL body and gets back a fully-formed
  `.chroma` package. The accepted Shadertoy uniforms (`iTime`,
  `iResolution`, `iTimeDelta`, `iFrame`, `iMouse`) are declared in
  the wrapper preamble so the imported body compiles unedited.
- **Exceed notes:** Automated conversion vs. manual conversion is a
  meaningful operator-time win. The converter is **atomic** (caller
  sees a fully-populated package or none —
  `import-shadertoy.md:64-67`). The reject / warn taxonomy is
  explicit: VR mode (`mainVR`) rejected, cubemap entrypoint
  deferred, webcam iChannel mapping deferred, deprecated
  `texture2D(...)` auto-corrected and surfaced as
  `Warning::DeprecatedTexture2DAutoCorrected` so the author has a
  trail of every machine-applied edit. ISF inputs get an actionable
  reject diagnostic pointing at the Shadertoy converter or the
  scaffolder; ISF native import is deferred to V2 with a
  `CHR-ISF-V2` differentiation bet rather than a silent failure.
- **Gap notes:** Multi-buffer Shadertoy compositions are out of
  scope for V1 (multipass-import is a future-wave ADR per
  `import-shadertoy.md:103-105`). Sound-shader bodies are
  intentionally rejected — Chromaticity is a visual-only runtime.

### How to Update Controls and Use Event Handlers with JavaScript

- **Score:** 2 / PARTIAL
- **Chromaticity equivalent:** `engine/script/` crate (per ADR-003 +
  ADR-012) + `engine/script/src/lib.rs:1-223` (slice 1 through
  5.3 architecture commentary) + `engine/script/src/reactive_symbol.rs`
  (typed catalog of 53 `ReactiveSymbol` variants binding
  `docs/36` retentions to a typed enum per slice 5.2) +
  `engine/script/src/command_bus.rs` (producer-side
  `__chromaticityEmit(name, value)` JS-host shim per slice 5.1) +
  `EV-SCRIPT-CONTRACT` (Phase-2 evidence row, surface evidence
  shipped; full conformance per `docs/26` Phase-2-exit definition
  requires successive slices).
- **Match notes:** Chromaticity exposes a sandboxed JavaScript
  runtime via rquickjs binding to QuickJS (MIT) with structured
  error surface, a typed reactive-symbol catalog covering the same
  `syn_*` audio uniforms the reference's JavaScript surface
  manipulates, and a producer-side command bus emitting reactive
  writes through the typed `ScriptCommand::TypedReactiveWrite`
  variant. The slice-3 runtime adapter
  (`chromaticity_runtime::script_bridge`) lands the
  cross-crate consumer side.
- **Exceed notes:** Chromaticity's script surface is
  hardened-by-default in ways the reference's documentation does
  not surface: sandbox-default (no `globalThis.fs` / `process` /
  `window`, no `import()`), watchdog timeouts (1 ms p99 soft /
  4 ms hard per ADR-012 §2 enforced via
  `rquickjs::Runtime::set_interrupt_handler`), 16 MiB memory cap
  (`with_memory_limit` per ADR-012 §3), and determinism shims
  (seeded `Math.random()` via xoroshiro128+, frame-locked
  `Date.now()` via a caller-injected `Clock`). These are all
  evidence-rowed under `EV-SANDBOX-SAFETY` + `EV-SCRIPT-CONTRACT`.
- **Gap notes:** **No tutorial page in the docs corpus for JS
  authoring.** `docs-corpus/docs/` has no `scripting.md` or
  `javascript.md` page; the script crate's surface lives in
  `engine/script/src/lib.rs` module prose, which is not bundled
  into the in-app help. The scene-author surface (control
  bindings, event handlers, preset-recall hooks per ADR-003 / per
  the slice 5.4+ planning note at `engine/script/src/lib.rs:133-138`)
  is later-slice scope. Without a `script.js` lifecycle entry in
  the scene loader (per the scaffolder's "loader does not currently
  consume per-package script bodies" caveat at
  `engine/authoring/src/scaffolder.rs:44-47`), an author cannot
  ship a JS event handler in a `.chroma` package today.

### v1.16 Feature Walkthrough

- **Score:** N/A / OUT-OF-SCOPE
- **Chromaticity equivalent:** Chromaticity is pre-v1.0; the closest
  shape is the CLAUDE.md "Wave progression" section + the
  per-release `CHANGELOG.md` populated by `release-please` on
  merges to `main` + `release-eng/` (Wave 1e signing + appcast
  scaffold per `EV-RELEASE-PIPELINE`).
- **Match notes:** N/A — version-walkthrough tutorials are a
  release-management concern that does not match the V1
  pre-release shape Chromaticity ships today.
- **Exceed notes:** N/A.
- **Gap notes:** This is an OUT-OF-SCOPE row, not a GAP — the
  Chromaticity equivalent (per-release change announcements + an
  in-app changelog) is a Phase-5 release-engineering concern that
  the `EV-RELEASE-PIPELINE` row stays `Planned` until production
  cert procurement (Apple Developer ID + Windows EV + Sparkle 2
  EdDSA keypair) which remains AO-gated. Re-score in a future
  refresh once a v1.0 ships and a first walkthrough page becomes
  the right artifact to author.

## Cross-cutting observations

- **Chromaticity leads on:** (1) operator-facing vocabulary density
  — 57 hotkey rows + a `?`-toggle overlay vs. the reference's
  four-letter control-focused set; (2) clean-room namespace
  hygiene — `syn_*` reserved for `compat-syn-v1`, `chroma_*` for
  net-new native primitives, enforced at the typed `ReactiveSymbol`
  catalog per slice 5.2; (3) safety-by-default in the JS surface —
  sandbox isolation, watchdog timeouts, memory cap, and determinism
  shims as constitutive surface, not as opt-in modes; (4) import
  automation — Shadertoy conversion is a one-shot atomic converter
  rather than a manual rewrite; (5) network defaults — every
  adapter binds to loopback unless explicitly opted in
  (`docs-corpus/docs/settings-reference.md:75-99`); (6) ADR-rooted
  documentation provenance — every page footer cites the upstream
  ADR + EV-ID it derives from, clean-room-attested.

- **Synesthesia leads on:** (1) tutorial maturity — the reference's
  tutorials are *narrative recipes* that walk the operator
  end-to-end with example code, whereas Chromaticity's
  docs-corpus is heavier on *reference prose* and lighter on
  step-by-step pathways (Quickstart and JS-handlers are the two
  PARTIAL items where this gap is sharpest); (2) OBS / livestream
  integration — a published Spout recipe and a video tutorial
  versus our pending Wave 1c-β Syphon real-impl plus no Windows
  Spout adapter and no OBS recipe page; (3) version walkthroughs —
  a v1.16 feature tour as a per-release artifact, which
  Chromaticity does not yet ship under the AO-gated
  `EV-RELEASE-PIPELINE` posture; (4) scene-author JavaScript
  surface — the reference's JS tutorial assumes a `script.js`
  lifecycle that the engine/scene loader does not yet honour for
  per-package script bodies.

- **Re-score triggers for the next refresh:**
  - Wave 1c-β follow-ups land — Syphon `publish_syphon_output`
    real-impl + per-platform `nokhwa` camera bindings will
    re-open OBS livestream from PARTIAL toward MATCH on macOS
    (Windows Spout would need a separate adapter wave).
  - A docs-corpus quickstart walkthrough page lands — would
    re-open Quickstart Guide from PARTIAL toward MATCH or
    EXCEEDS depending on hardware-reality coverage.
  - The engine/scene loader widens to consume per-package
    `main.script.js` bodies — would re-open JavaScript Event
    Handlers from PARTIAL toward MATCH; a dedicated
    `docs-corpus/docs/scripting.md` page would compound the
    score lift.
  - A v1.0 release ships under `EV-RELEASE-PIPELINE` — would
    convert the v1.16 walkthrough row from OUT-OF-SCOPE to a
    real comparison.

---

<!-- Clean-room authored from Synesthesia tutorial slugs (pathway summaries only — no prose / code / recipe copy), CLAUDE.md Wave 1d-α / 1d-β / 2.x / 4b notes, docs-corpus/docs/* (11 pages), engine/authoring/src/*, engine/script/src/lib.rs, engine/scene/tests/fixtures/scenes/*, integrations/syphon-adapter/src/lib.rs, docs/comparison/README.md (scoring rubric + per-item template), and docs/26-evidence-artifact-registry.md (EV-IDs); never derived from Contents/ or chromaticity-planning/ per docs/29. -->
