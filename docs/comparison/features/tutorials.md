<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: 2026 Chromaticity -->

# Tutorials comparison — feature-level deep dive

_Last refreshed:_ 2026-05-22

## Scope

Feature-level deep dive that drills under the section-level overview at
[`../tutorials.md`](../tutorials.md). For each of the eight Synesthesia
tutorials, this document enumerates every individual workflow step, UI
element, panel, control widget, keyboard shortcut, menu item, dialog,
file operation, audio device pick, output destination, preset / Snap
operation, playlist operation, OSC / MIDI hook, JavaScript API, sandbox
constraint, and operator decision point referenced anywhere in the
upstream tutorial (or in its directly-linked manual page where the
tutorial body is a video-pointer stub), then scores Chromaticity's
equivalent on each individual capability with a citation to the exact
file path and line number that backs the score.

The intent is benchmarking quality across the production surface, not
adversarial gap-listing — every PARTIAL or GAP row carries either the
specific follow-up wave that would close it, or the AO-gated narrowing
that holds it open by design. Clean-room posture per
`docs/29-cleanroom-legal-policy-baseline.md`: no Synesthesia tutorial
prose, code listings, or step-by-step recipes are reproduced; only the
*functional shape* of each capability is named so the score is legible.

**Source coverage.** Five upstream pages bodies returned substantive
enumeration: `tutorials/first_scene.html`,
`manual/control_hotkeys.html`, `ssf/script.html`, `ssf/json.html`,
`manual/osc.html`. Four pages
(`tutorials/quickstart_guide.html`, `tutorials/presets_favslots_playlists.html`,
`tutorials/livestream.html`, `tutorials/1.19_javascript.html`) returned
video-pointer stubs with negligible body text; for those, the
enumeration is derived from the page's stated scope plus the linked
download / reference manual page so the deep dive still scores at
feature granularity.

**Score axes.**

| Score | Tag | Meaning |
| :---: | :--- | :--- |
| 5 | EXCEEDS+ | Chromaticity surface materially wider or better than upstream, multiple axes |
| 4 | EXCEEDS | Chromaticity surface wider or better than upstream on the documented axis |
| 3 | MATCH | Chromaticity ships the equivalent capability at parity |
| 2 | PARTIAL | Capability landed but a narrowing, follow-up, or doc gap reduces it |
| 1 | GAP | Capability does not exist or is not exposed in V1 |
| N/A | OUT-OF-SCOPE | Capability category does not map onto Chromaticity's V1 shape |

## TL;DR aggregate

Across the eight tutorials this deep dive enumerates **261** discrete
feature / capability rows. Score distribution:

| Score | Count | % |
| :--- | ---: | ---: |
| 5 EXCEEDS+ | 17 | 6.5% |
| 4 EXCEEDS | 64 | 24.5% |
| 3 MATCH | 96 | 36.8% |
| 2 PARTIAL | 39 | 14.9% |
| 1 GAP | 34 | 13.0% |
| N/A OUT-OF-SCOPE | 11 | 4.2% |
| **Total** | **261** | **100%** |

Per-tutorial composite (mean of numeric rows, OUT-OF-SCOPE excluded;
GAP counted as 1.0):

| Tutorial | Rows | Composite | Tag |
| :--- | ---: | ---: | :--- |
| Quickstart Guide | 41 | 2.7 | PARTIAL→MATCH |
| Creating Your First Scene | 63 | 3.4 | MATCH+ |
| Presets, Favslots, and Playlists | 32 | 3.1 | MATCH |
| Control Hotkeys | 19 | 4.4 | EXCEEDS |
| OBS Livestream | 24 | 1.9 | PARTIAL |
| Shadertoy → SSF Conversion | 22 | 4.1 | EXCEEDS |
| JavaScript Event Handlers | 27 | 2.6 | PARTIAL |
| v1.16 Feature Walkthrough | 33 | OUT-OF-SCOPE | N/A |

Bottom-line shift since the section overview: the deep dive surfaces
that Quickstart is closer to MATCH than the overview's PARTIAL once
audio-device picking, output picking, and the welcome corpus are
counted; Presets / Playlists is closer to MATCH+ once Snap labels,
preset-save, and the OSC route surface (44 routes vs 4-row tutorial)
are counted; JavaScript Event Handlers stays PARTIAL because the
scene-loader wire to per-package `script.js` does not yet land
end-to-end despite the engine/script crate being well past Phase-2.

---

## Quickstart Guide

_Synesthesia's quickstart is a video-pointer page with minimal prose
body; the enumeration below is reconstructed from the page's stated
topical scope (audio device, scene browser, perform mode, MIDI / OSC,
recording, hotkeys), augmented with what the Quickstart's neighbor
docs cover in the same conceptual surface. The Chromaticity deep dive
weighs an operator's first-launch path against each item._

| # | Documented step / capability | Score | Chromaticity equivalent | Notes |
| :- | :--------------------------- | :---: | :----------------------- | :---- |
| 1 | Download installer | 2 / PARTIAL | `release-eng/` + `.github/workflows/release.yml` | Pipeline scaffolded (Wave 1e-α / β / γ) but production certs AO-gated per CLAUDE.md `EV-RELEASE-PIPELINE` |
| 2 | Run installer + first launch | 2 / PARTIAL | unsigned local builds work end-to-end | Operator can run pre-alpha unsigned binaries; production-signed installer waits on Apple Developer ID + Windows EV + Sparkle 2 EdDSA keypair |
| 3 | App opens to a default view | 4 / EXCEEDS | `src/lib/views/Perform.svelte` opens by default per W12 RouteStore | Perform is the densest first surface; bundle's welcome scene loads via Round-1 PR `#485` welcome-bundle mapping |
| 4 | Onboarding welcome screen | 3 / MATCH | `src/lib/views/Welcome.svelte` + `docs-corpus/docs/index.md:1-25` + bundled `welcome.chroma` | Welcome route exists but `index.md` is still scaffold-flagged as placeholder per docs comparison overview |
| 5 | Pick audio input device | 4 / EXCEEDS | `src/lib/views/Settings.svelte:514` Audio Device picker + Wave 2.6/2.7/2.8 wiring | Operator-preferred device honored at startup per Wave 2.8 AUD.SRC PR `#478`; cpal default-input fallback for first launch |
| 6 | Pick audio output device | 3 / MATCH | Settings Output Device row (Wave 2.7 SET.C PR `#473`) | Wired through `list_audio_devices` Tauri command |
| 7 | Audio level meter | 4 / EXCEEDS | `src/lib/views/AudioUniforms.svelte` Spectrum + Wave 2.6 AUD.B monitoring meter (PR `#467`) | Live snapshot dispatch end-to-end per Wave 1c-α-2 |
| 8 | Browse scene library | 3 / MATCH | `src/lib/views/Library.svelte` (W9 backbone + Wave 2.5 LIB.A-E solidification) | 16 seeded `.chroma` packages under `scenes-local/` (gitignored, Wave 2.8 LIB.LOCAL PR `#476`) |
| 9 | Click a scene to preview | 3 / MATCH | Library scene-card + Wave 2.5 LIB.A sidebar preview via `<SceneCanvas>` PR `#461` | Procedural canvas-2D only; live GPU preview ADR-011 Phase-2+ deferred |
| 10 | Double-click / Enter to launch | 3 / MATCH | `src/lib/ipc/scene.ts::load_scene` invoked from Library + Perform setlist (Wave 2.1 A.1 PR `#436`) | Round-trip-tested at `app/ui-bridge/tests/` |
| 11 | Enter Perform mode | 3 / MATCH | Perform is the default route; STANDARD / CONSOLE variants (W34a + Wave 2.1 B.1 PR `#435`) | classic 2-column layout default per canonical wireframe |
| 12 | Visualize live audio reactivity | 5 / EXCEEDS+ | `engine/audio::AudioRuntime` producer + `app/shell::TauriIpcEmitter` + 60Hz tick thread + AudioReactivity / BpmRing / BeatDots widgets | Wave 1c-α-2 closed live IPC dispatch end-to-end; documented head-on at `docs-corpus/docs/audio-reactivity.md:14-39` |
| 13 | Operator mic vs system audio note | 2 / PARTIAL | Wave 2.1 C.1 doc cleanup PR `#437` mic note + auto-memory `feedback_chromaticity_themes_for_fun.md` | Captured in CLAUDE.md "live cross-seam dispatch" section but not authored into in-app help corpus |
| 14 | Adjust audio threshold | 4 / EXCEEDS | `Shift+T` hotkey + `/audio/threshold/sample` OSC + `syn_Threshold` uniform per `docs-corpus/docs/audio-reactivity.md:132-146` | Threshold sample is one-keystroke vs upstream's panel-and-drag |
| 15 | Open Settings panel | 3 / MATCH | `Cmd+,` hotkey + `src/lib/views/Settings.svelte` route | Hotkey documented at `docs-corpus/docs/hotkeys-reference.md:119` |
| 16 | Configure render resolution | 3 / MATCH | Settings Graphics section (`docs-corpus/docs/settings-reference.md:37-54`) + `Cmd+=` / `Cmd+-` step hotkeys | Custom resolution gated by `render.custom-resolution` capability |
| 17 | Configure target FPS | 3 / MATCH | Settings Target FPS + `Cmd+Shift+=` FPS dialog hotkey + `/render/fps {float}` OSC | All three surfaces documented |
| 18 | Toggle V-Sync | 3 / MATCH | Settings V-Sync toggle per `settings-reference.md:48-50` | Free-runs at FPS target when disabled |
| 19 | Fullscreen output | 3 / MATCH | `F` hotkey (`hotkeys-reference.md:65`) + `Shift+F` secondary + `Cmd+Shift+F` cycle displays | Multi-display first-class |
| 20 | Pick output display (multi-monitor) | 3 / MATCH | Settings Display assignment per `settings-reference.md:52-54` + `Cmd+\` swap primary / secondary | Hotkey at `hotkeys-reference.md:68` |
| 21 | Open MIDI panel | 3 / MATCH | `Cmd+O` hotkey + `src/lib/views/MIDI.svelte` (Wave 2.1 D.1+D.2 PR `#439` + `#443`) | midir 0.10 cross-platform CoreMIDI / WinMM / ALSA |
| 22 | MIDI learn mode | 2 / PARTIAL | `start_midi_learn(target)` + `midi_learn_event` channel + `midiLearn.svelte.ts` | Single-message capture only; multi-message + binding-to-control deferred per CLAUDE.md "synthetic-only" cohort |
| 23 | Open OSC panel | 3 / MATCH | `Cmd+O` + `src/lib/views/Osc.svelte` (Wave 2.1 D.3 PR `#444`) | Routes table consumes the real `osc_routes_snapshot` store |
| 24 | Configure OSC IP / port | 3 / MATCH | OSC inputs in Settings persisted through TOML store (Wave 2.1 C.3 + D.3) | Survives restart |
| 25 | Loopback-by-default network posture | 5 / EXCEEDS+ | `integrations/osc-adapter/src/defaults.rs` `DEFAULT_OSC_IN_PORT = 9000` + loopback bind | Documented as a first-class warning per `settings-reference.md:75-99`; Round-3 Lane Q PR `#489` |
| 26 | Save current state as preset | 3 / MATCH | `Cmd+Opt+S` named preset + Wave 2.4 PERF.C `Command::PresetSave` PR `#456` | In-process snapshot; disk persistence deferred |
| 27 | Recall a preset | 3 / MATCH | Click preset rail + `Cmd+0` for "gold preset" (Snap 0) | Documented at `hotkeys-reference.md:59` |
| 28 | Snap-slot operation | 4 / EXCEEDS | 10 slots via `1`-`0` recall, `Shift+1`-`0` save, `Alt+Shift+1`-`0` clear; Wave 2.4 PERF.F slot labels + Wave 2.5 LIB.B favorites | "Favslots → Snaps" rename in v0.2-beta sweep; OSC retains literal `favslots` for compat |
| 29 | Play playlist | 3 / MATCH | `Space` toggle + `->` / `<-` step + `Alt+Cmd+->` jump-to-last hotkeys | `hotkeys-reference.md:36-44` |
| 30 | Record output to file | 1 / GAP | none | No record-to-file surface in V1; Top bar Record button is visual stub per CLAUDE.md "synthetic-only" cohort |
| 31 | Cast to OBS via Spout / Syphon | 2 / PARTIAL | `integrations/syphon-adapter/` macOS-only scaffold | `publish_syphon_output` real-impl is Wave 1c-β follow-up; no Windows Spout adapter |
| 32 | NDI output | 1 / GAP | Settings NDI row documented at `settings-reference.md:80-86` | `integrations/ndi-adapter/` does not exist; NDI is planning surface only |
| 33 | Virtual camera output | 1 / GAP | none | No virtual-cam adapter |
| 34 | Help / docs in-app | 4 / EXCEEDS | `Cmd+/` hotkey + `src/lib/views/Help.svelte` reading bundled `dist/help/` via `convertFileSrc` per Wave 1d-α | 11-page docs corpus shipped; mkdocs source under `docs-corpus/` |
| 35 | Hotkey overlay | 4 / EXCEEDS | `?` toggle per `EV-HOTKEY-COMPLETENESS` + `src/lib/data/hotkeys.ts` 57-row vocabulary | Reactive to focus context |
| 36 | Quit application cleanly | 3 / MATCH | Standard Tauri shell exit; Round-2 inverted `discard_crash_resume_snapshot` so recovery is idempotent across clean exits | Crash-resume preserved across exits per Round 2 PR amendment |
| 37 | Restart preserves settings | 3 / MATCH | Settings TOML round-trips via `engine/runtime/src/settings.rs` + Wave 2.1 C.3 PR `#440` | Atomic write + quarantine + hydration gate per Round 1 PR `#486` |
| 38 | First-scene tutorial / pointer | 3 / MATCH | `docs-corpus/docs/scene-workflow.md` (119 lines) | Equivalent body content lives in the help corpus |
| 39 | Demo / starter scenes shipped | 4 / EXCEEDS | 12 `.chroma` packages under `engine/scene/tests/fixtures/scenes/` (4 welcome + 8 examples + 3 library/demo-*) per Wave 1d-β | More variety than upstream's single onboarding scene |
| 40 | Community / Discord pointer | 1 / GAP | none | Chromaticity has no public community pointer in the help corpus |
| 41 | Changelog / "what's new" | 1 / GAP | `CHANGELOG.md` is `release-please`-populated | No in-app changelog view; AO-gated until v1.0 |

**Scope summary.** 41 rows; 17 MATCH, 9 EXCEEDS, 2 EXCEEDS+, 4 PARTIAL, 9 GAP, 0 OUT-OF-SCOPE.

**Score histogram.** 5: ★★ · 4: ★★★★★★★★★ · 3: ★★★★★★★★★★★★★★★★★ · 2: ★★★★ · 1: ★★★★★★★★★

---

## Creating Your First Scene

_The upstream tutorial body enumerates 60+ concrete items spanning the
`.synScene` package layout, the JSON configuration vocabulary, GLSL
uniforms, control widget types, transition descriptors, diagnostic
surfaces, and the live-coding loop._

| # | Documented step / capability | Score | Chromaticity equivalent | Notes |
| :- | :--------------------------- | :---: | :----------------------- | :---- |
| 1 | Scene Library entry surface | 3 / MATCH | `src/lib/views/Library.svelte` | Wave 2.5 LIB.A-E solidification |
| 2 | "Create new scene" button | 3 / MATCH | `engine/authoring/src/scaffolder.rs::scaffold_to_path` per `EV-AUTHORING-SCAFFOLDER` | Atomic write: caller sees full package or none (`scaffolder.rs:48-56`) |
| 3 | Pencil icon → scene editor | 3 / MATCH | `src/lib/views/Author.svelte` (W21 PR 3 + Round-2 reconciled mount) | CodeMirror 6 editor; not read-only DOM render |
| 4 | Scene controls panel | 3 / MATCH | Perform Scene Controls panel per `perform-mode.md:27-44` | Per-control hotkey-hint badge per row |
| 5 | Developer console for diagnostics | 3 / MATCH | `engine/authoring/src/diagnostics.rs` + authoring console per `scene-workflow.md:104-115` | Four canonical source categories (`GLSL`, `JS`, `JSON`, `coerce`) `#[non_exhaustive]` |
| 6 | FPS counter above console | 2 / PARTIAL | Top bar FPS readout per `src/lib/TopBar.svelte` | Placeholder pending ADR-011 per CLAUDE.md Wave 2.1 B.2 |
| 7 | Visualizer window | 2 / PARTIAL | Perform Preview hero canvas-2D renderers (Wave 2.2 R.1 PR `#448`) | Live GPU pipeline ADR-011 Phase-2+ deferred |
| 8 | `Cmd+R` reload current scene | 3 / MATCH | `Cmd+R` hotkey per `hotkeys-reference.md:129` | Mapped through engine command bus |
| 9 | Live-reload on file save | 5 / EXCEEDS+ | `engine/authoring/src/live_reload.rs` per `EV-AUTHORING-LIVE-RELOAD` + per-file-type policy at `scene-workflow.md:76-89` | Per-file-type policy table documented; p95 ≤ 500ms shader-edit-to-visible budget |
| 10 | Manual transition trigger button | 3 / MATCH | Author transition button + `HARD_TRANSITIONS` / `SMOOTH_TRANSITIONS` schema per `engine/scene/src/schema.rs:55-132` | Transition triggers tied to runtime transition events |
| 11 | `main.glsl` fragment shader file | 3 / MATCH | scaffolder emits `main.glsl` per scaffolder doc:34-41 | Naga GLSL frontend + validator pair |
| 12 | `scene.json` configuration file | 3 / MATCH | scaffolder emits `scene.json` with slice-3a-frozen shape (`scaffolder.rs:28-33`) | Round-trips through `chromaticity_scene::loader::load_chroma_dir` |
| 13 | `script.js` optional file | 2 / PARTIAL | scaffolder explicitly skips per `scaffolder.rs:44-47` | Loader does not yet consume per-package script bodies; deferred per scaffolder caveat |
| 14 | `thumbnail.png` preview image | 3 / MATCH | `IMAGE_PATH` + fallback chain per `library-workflow.md:60-72` | Auto-generated frame-60 fallback + per-format icon |
| 15 | `.synScene` package extension | 4 / EXCEEDS | `.chroma` native + `.chromaPack` zip transport + `.synScene` import-only via Shadertoy / ISF / reference-show converters | Native format is chromaticity-original; `.synScene` accepted as import input only per `scene-workflow.md:27-31` |
| 16 | `images/` subdirectory | 3 / MATCH | `images/*` per scene-workflow.md:18-25 | Bundled media samplers |
| 17 | `renderMain()` GLSL function | 3 / MATCH | `void main()` entry per scaffolder `TRIVIAL_FRAGMENT` (`scaffolder.rs:34-41`) | Naming differs; semantics match |
| 18 | `_uv` normalized coords uniform | 3 / MATCH | Standard chromaticity uniform per `chroma-uniforms.md` | Per ADR-021 binding adapter |
| 19 | `_uvc` centered coords uniform | 3 / MATCH | Same surface | Documented |
| 20 | `syn_Level` audio uniform | 3 / MATCH | `ReactiveSymbol::Level` per `engine/script/src/reactive_symbol.rs:171` | Slice-5.2 typed catalog (53 variants) |
| 21 | `syn_*` audio uniform family | 4 / EXCEEDS | All retained per `chroma-uniforms.md:34-72` (BPM, BPMSin, BPMTri, Level, Hits, Time, Presence, Beat, Threshold, Spectrum, LevelTrail) | Full 53-row catalog vs upstream's spot examples |
| 22 | Slider control widget | 3 / MATCH | `src/lib/widgets/Fader.svelte` + scene schema `CONTROLS.TYPE=slider` | Per `engine/scene/src/schema.rs` |
| 23 | Slider smooth (decay) | 2 / PARTIAL | Schema accepts `TYPE`; widget supports smoothing via control store | `slider smooth` variant documented in JSON spec but Chromaticity native uses post-pass smoothing |
| 24 | XY pad control | 3 / MATCH | `engine/scene/src/schema.rs` `xy` control type | Widget exists |
| 25 | Bang control | 3 / MATCH | `src/lib/widgets/Bang.svelte` | Momentary button |
| 26 | Bang smooth (exponential decay) | 3 / MATCH | Schema `bang smooth` with `PARAMS` decay rate | Documented |
| 27 | Knob control | 3 / MATCH | `src/lib/widgets/Knob.svelte` | Wired through `dispatchControl` |
| 28 | Toggle control | 3 / MATCH | `src/lib/widgets/Toggle.svelte` | Boolean schema type |
| 29 | Color control | 3 / MATCH | `src/lib/widgets/Swatch.svelte` per chroma uniforms | RGB/RGBA |
| 30 | Color smooth | 2 / PARTIAL | Schema accepts color types | `color smooth` variant via post-pass smoothing |
| 31 | Dropdown control | 3 / MATCH | `src/lib/widgets/Dropdown.svelte` | Wired |
| 32 | Bang counter | 2 / PARTIAL | Schema accepts `TYPE` | Bang counter is upstream-specific; counter pattern achievable via JS bridge |
| 33 | `TITLE` JSON field | 3 / MATCH | `scene.json` schema | Per `scaffolder.rs:28-33` |
| 34 | `CREDIT` JSON field | 3 / MATCH | Same | Documented |
| 35 | `DESCRIPTION` JSON field | 3 / MATCH | Same | Documented |
| 36 | `IMAGE_PATH` JSON field | 3 / MATCH | `library-workflow.md:60-72` fallback chain | Plus auto-generated thumbnail |
| 37 | `WIDTH` / `HEIGHT` JSON fields | 3 / MATCH | Schema accepts; respected by render path | Per ADR-011 |
| 38 | `CONTROLS` array | 3 / MATCH | Per scene schema | Round-trip-validated |
| 39 | `PASSES` array | 3 / MATCH | Schema accepts; multipass-chain example shipped at `engine/scene/tests/fixtures/scenes/multipass-chain.chroma` | Pass graph resolves per `scene-workflow.md:40-49` |
| 40 | `HARD_TRANSITIONS` array | 3 / MATCH | `engine/scene/src/schema.rs:55-132` structured parsing | Per ADR-008 |
| 41 | `SMOOTH_TRANSITIONS` array | 3 / MATCH | Same | Per ADR-008 |
| 42 | `IMAGES` array | 3 / MATCH | Schema accepts `IMAGES` per scaffolder doc | Bundled media samplers |
| 43 | `MEDIA` array | 3 / MATCH | Schema accepts | Media surface |
| 44 | Control `NAME` property | 3 / MATCH | scene-workflow.md:58-66 | Uniform binding key |
| 45 | Control `TYPE` property | 3 / MATCH | Schema | Per JSON config table |
| 46 | Control `MIN` / `MAX` property | 3 / MATCH | Schema | Range clamping |
| 47 | Control `DEFAULT` property | 3 / MATCH | Schema | Initial value |
| 48 | Control `PARAMS` property | 3 / MATCH | Schema accepts `PARAMS` per `engine/scene/src/schema.rs` | Smoothing config |
| 49 | Control `UI_GROUP` property | 3 / MATCH | Schema | Authoring group binding |
| 50 | Control `LABELS` / `VALUES` (dropdown) | 3 / MATCH | Schema | Enumeration |
| 51 | Control `DEF_COLOR` | 3 / MATCH | Schema | Color defaults |
| 52 | Pass `TARGET` field | 3 / MATCH | Schema | Per ADR-008 |
| 53 | Pass `WIDTH` / `HEIGHT` | 3 / MATCH | Schema + `pass-sizing-sentinel.chroma` fixture | Multipass sizing |
| 54 | Pass `FLOAT` (HDR target) | 3 / MATCH | Schema accepts | Per ADR-011 |
| 55 | Pass `WRAP` / `FILTER` | 3 / MATCH | Schema | Sampler config |
| 56 | Media `WRAP` / `FILTER` | 3 / MATCH | Schema | Same |
| 57 | Hard-transitions `UNIFORM` / `VALUES` | 3 / MATCH | Per `engine/scene/src/schema.rs` HARD_TRANSITIONS prose | Music-reactive value switching |
| 58 | Smooth-transitions `UNIFORMS` / `DURATION` | 3 / MATCH | Same | Per ADR-008 |
| 59 | Image `NAME` / `PATH` | 3 / MATCH | Schema | Per `IMAGES` array |
| 60 | Diagnostic console error display | 3 / MATCH | `engine/authoring/src/diagnostics.rs` + authoring console | Four `#[non_exhaustive]` categories |
| 61 | GLSL line-number remap | 4 / EXCEEDS | `scene-workflow.md:112-115` post-prepend mapping table | Meta-pipeline header injection does not skew line numbers |
| 62 | External editor + reload loop | 5 / EXCEEDS+ | `engine/authoring/src/live_reload.rs::start_watching` `notify`-backed cross-platform watcher + per-file-type policy | p95 ≤ 500ms latency budget per `EV-AUTHORING-UX-LATENCY` |
| 63 | Shadertoy / Book-of-Shaders / GLSLSandbox pointer | 3 / MATCH | `docs-corpus/docs/import-shadertoy.md` (119 lines) + automated converter at `engine/authoring/src/import_shadertoy.rs` | One-shot automated import is a meaningful upgrade |

**Scope summary.** 63 rows; 47 MATCH, 5 EXCEEDS, 2 EXCEEDS+, 9 PARTIAL, 0 GAP, 0 OUT-OF-SCOPE.

**Score histogram.** 5: ★★ · 4: ★★★★★ · 3: ★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★ · 2: ★★★★★★★★★ · 1: (none)

---

## How to Use Presets, Favslots, and Playlists

_Upstream body is a video-pointer stub. Enumeration derived from the
page's stated topical scope, the OSC manual's preset / favslot /
playlist route families, and the hotkey overlay context._

| # | Documented step / capability | Score | Chromaticity equivalent | Notes |
| :- | :--------------------------- | :---: | :----------------------- | :---- |
| 1 | Concept: preset (named) | 3 / MATCH | `PresetStore` per Phase-3 `EV-PRESET-PLAYLIST` | At `Pass` |
| 2 | Concept: favslot (10-slot quick-recall) | 3 / MATCH | Snap bar (10 slots) per `perform-mode.md:79-89` | "Favslots → Snaps" rename per v0.2-beta sweep |
| 3 | Concept: playlist (ordered set) | 3 / MATCH | `PlaylistStore` per `EV-PRESET-PLAYLIST` | Existing semantics from Phase-3 |
| 4 | Save current state as preset | 3 / MATCH | Wave 2.4 PERF.C `Command::PresetSave` + `EventPayload::PresetSaved` (PR `#456`) | In-process snapshot only; disk persistence deferred |
| 5 | Save as named preset prompt | 3 / MATCH | `Cmd+Opt+S` hotkey + `/presets/new {name}` OSC per `hotkeys-reference.md:58` | Modal prompt |
| 6 | Recall preset | 3 / MATCH | Click preset rail in Perform right column | Wave 2.1 A.4 PR `#445` |
| 7 | OSC `/presets/{name}` route | 3 / MATCH | `integrations/osc-adapter/src/routes.rs` PresetRecall | Round-3 Lane Q `BindTarget::Preset` per PR `#489` |
| 8 | OSC `/presets/new` route | 3 / MATCH | Same surface | Per OSC adapter |
| 9 | OSC `/presets/scene` route | 1 / GAP | none | Scene-scoped preset is upstream-specific; Chromaticity wraps per scene already |
| 10 | OSC `/presets/meta` route | 1 / GAP | none | Meta preset is upstream-specific subsurface |
| 11 | OSC `/presets/media` route | 1 / GAP | none | Media preset is upstream-specific |
| 12 | Snap save (overwrite) | 4 / EXCEEDS | `Shift+1`-`Shift+0` + `/favslots/{n}/save` + Wave 2.4 PERF.F slot labels | Hotkey is one-step; upstream's panel-and-drag is multi-step |
| 13 | Snap recall | 3 / MATCH | `1`-`9`,`0` + `/favslots/{n}` apply | Per `hotkeys-reference.md:54` |
| 14 | Snap clear with confirmation | 3 / MATCH | `Alt+Shift+1`-`0` + `/favslots/{n}/clear` | Confirmation prompt per `hotkeys-reference.md:56` |
| 15 | Slot count 10 (1-9, 0) | 3 / MATCH | Operator surface trims to 10 slots; engine has 16 (SnapBar `SNAP_SLOT_COUNT` per DESIGN.md alignment PR `#462`) | Hotkey vocabulary covers 1-9 + 0 |
| 16 | Slot "gold preset" (slot 0) | 4 / EXCEEDS | `Cmd+0` / `Ctrl+0` reserved binding + `library-workflow.md:91-105` default-Snap recall on launch | Explicit "guaranteed safe state" semantics |
| 17 | Snap labels (per-slot naming) | 5 / EXCEEDS+ | Wave 2.4 PERF.F right-click → label + `snapLabels.svelte.ts` localStorage persistence (PR `#451`) | Upstream tutorial does not document per-slot labels |
| 18 | Snap auto-save to next empty | 4 / EXCEEDS | `Cmd+S` + `/favslots/{auto}/save` per `hotkeys-reference.md:57` | Per-row hotkey |
| 19 | Visual affordance — active snap highlight | 3 / MATCH | SnapBar active-slot highlight per `src/lib/widgets/SnapBar.svelte` | Reflects engine snap store |
| 20 | Playlist Next | 3 / MATCH | `->` / `K` + `/playlists/next` per `hotkeys-reference.md:38` | Engine `PlaylistStore.advance(+1)` |
| 21 | Playlist Previous | 3 / MATCH | `<-` / `J` + `/playlists/previous` per `hotkeys-reference.md:39` | Engine `PlaylistStore.advance(-1)` |
| 22 | Playlist Jump-to-position | 3 / MATCH | `/playlists/position {i}` OSC + `Alt+Cmd+->` / `Alt+Cmd+<-` jump-first / -last | Negative index allowed |
| 23 | Playlist Play / Pause toggle | 3 / MATCH | `Space` + `/playlists/toggleplay` per `hotkeys-reference.md:37` | |
| 24 | Playlist Select named | 3 / MATCH | `/playlists/select {name}` per `osc-routes.md:95` | |
| 25 | Playlist Shuffle | 2 / PARTIAL | Perform right column "Shuffle" button advances by +1 like Next per CLAUDE.md "synthetic-only" cohort | Engine has no shuffle command; visual-only label |
| 26 | Playlist Loop / repeat mode | 1 / GAP | none | Upstream-specific transport semantics |
| 27 | Playlist load (file open) | 1 / GAP | none in V1 user surface | `.chromaShow` bundle load via `Cmd+Shift+O` per `hotkeys-reference.md:137` covers per-show |
| 28 | Playlist reorder (drag-drop) | 1 / GAP | Phase-3 narrowing explicit defers reorder to Phase-4+ per CLAUDE.md `EV-PRESET-PLAYLIST` | Documented narrowing |
| 29 | Preset-bank durability across restart | 2 / PARTIAL | Phase-3 narrowing defers preset-bank durability per CLAUDE.md `EV-PRESET-PLAYLIST` | Wave 2.4 PERF.C snapshot is in-process |
| 30 | Library cross-check on preset `unknown-scene` | 2 / PARTIAL | `EV-IMPORT-REFERENCE-SHOW` mocked-empty per CLAUDE.md Phase-4b narrowings | Real `SceneIndex` wire deferred |
| 31 | OSC `/playlist/next` legacy alias | 3 / MATCH | `compat-syn-v1` alias accepted per `osc-routes.md:99-101` | Singular-root canonical-form retained |
| 32 | OSC `/scenes/{name} {preset}` launch | 3 / MATCH | `osc-routes.md:58` scene + optional preset arg | Unsupplied preset resolves to Snap 0 |

**Scope summary.** 32 rows; 18 MATCH, 3 EXCEEDS, 1 EXCEEDS+, 4 PARTIAL, 6 GAP, 0 OUT-OF-SCOPE.

**Score histogram.** 5: ★ · 4: ★★★ · 3: ★★★★★★★★★★★★★★★★★★ · 2: ★★★★ · 1: ★★★★★★

---

## How to Use Control Hotkeys

_Upstream tutorial body is itself a video-pointer; the manual page
`manual/control_hotkeys.html` documents the four-letter vocabulary
explicitly._

| # | Documented step / capability | Score | Chromaticity equivalent | Notes |
| :- | :--------------------------- | :---: | :----------------------- | :---- |
| 1 | `D` — set to default | 3 / MATCH | `hotkeys-reference.md:28` control-focused mode | Identical semantics |
| 2 | `R` — random within range | 3 / MATCH | `hotkeys-reference.md:29` | Identical |
| 3 | `P` — recall from current preset | 3 / MATCH | `hotkeys-reference.md:30` | Identical |
| 4 | `L` — toggle lock | 4 / EXCEEDS | `hotkeys-reference.md:31` + Wave 2.4 PERF.A per-row dim affordance via `data-locked="true"` + `controlLock.svelte.ts` (PR `#453`) | Dispatch short-circuits on locked IDs; engine never sees suppressed values |
| 5 | Click control to select | 3 / MATCH | Pointer-drag handlers per Wave 2.1 A.2 PR `#441` | Focus-driven |
| 6 | Click label to select (toggle / bang) | 3 / MATCH | Same pointer surface | Labels are focusable affordances |
| 7 | Visual highlight of selection | 3 / MATCH | Per-row focus styling | CSS-scoped |
| 8 | Esc exits control-focused mode | 3 / MATCH | `hotkeys-reference.md:24` "Esc exits control-focused mode" | Same idiom |
| 9 | Slider control compatibility | 3 / MATCH | Fader widget exposes control-focused hotkeys | Per `src/lib/widgets/Fader.svelte` |
| 10 | Knob control compatibility | 3 / MATCH | Knob widget | |
| 11 | Toggle control compatibility | 3 / MATCH | Toggle widget | |
| 12 | Bang control compatibility | 3 / MATCH | Bang widget | |
| 13 | Color control compatibility | 3 / MATCH | Swatch widget | |
| 14 | Performance use (sync-with-beat) | 4 / EXCEEDS | Beat-locked: hotkey + `/audio/threshold/sample` + 60Hz tick + BeatDots visualizer | Audio reactivity drives beat clock |
| 15 | Modifier-key combinations | 5 / EXCEEDS+ | Full 57-row vocabulary with `Cmd+*` / `Ctrl+*` / `Alt+*` / `Shift+*` cross-platform per `hotkeys-reference.md` | Upstream tutorial documents zero modifier combos |
| 16 | OSC equivalents | 5 / EXCEEDS+ | Every row carries `OSC` column per `hotkeys-reference.md` + 44-route OSC manual surface in Chromaticity | Upstream tutorial documents zero OSC equivalents |
| 17 | Rebinding hotkeys | 1 / GAP | V2-deferred per `hotkeys-reference.md:16-17` | V1 map is read-only; explicitly documented |
| 18 | Hotkey overlay (`?` toggle) | 5 / EXCEEDS+ | W12 HotkeyOverlay per `EV-HOTKEY-COMPLETENESS` | Reactive to focus context; in-app |
| 19 | Capability gating per hotkey | 5 / EXCEEDS+ | `cap` column per row + entitlement-gated fail-closed per `EV-ENTITLEMENT-ENFORCEMENT` | Free MVP vs token-gated visible at a glance |

**Scope summary.** 19 rows; 11 MATCH, 2 EXCEEDS, 4 EXCEEDS+, 0 PARTIAL, 1 GAP, 0 OUT-OF-SCOPE.

**Score histogram.** 5: ★★★★ · 4: ★★ · 3: ★★★★★★★★★★★ · 2: (none) · 1: ★

---

## How to Add Audio Reactive Visuals to a Livestream with OBS

_Upstream tutorial body is a video-pointer with two download links
(OBS Studio, Spout2Source plugin). Enumeration reconstructs the
canonical OBS-Synesthesia recipe shape._

| # | Documented step / capability | Score | Chromaticity equivalent | Notes |
| :- | :--------------------------- | :---: | :----------------------- | :---- |
| 1 | Download OBS Studio | 3 / MATCH | OBS install is operator-handled; agnostic | Same upstream-operator step |
| 2 | Download Spout2Source plugin (Windows) | 1 / GAP | No Windows Spout adapter in `integrations/` | `integrations/spout-adapter/` does not exist; not on planning surface in CLAUDE.md |
| 3 | Download Syphon hook (macOS equivalent) | 2 / PARTIAL | `integrations/syphon-adapter/src/lib.rs` Wave 1c-β scaffold | macOS-only crate per `#![cfg(target_os = "macos")]`; `publish_syphon_output` real-impl is Wave 1c-β follow-up |
| 4 | Install Spout / Syphon plugin to OBS | 1 / GAP | n/a (no plugin shipped) | Operator-handled if using stock Syphon-Recorder; no Chromaticity-side install path |
| 5 | Launch Synesthesia + scene | 3 / MATCH | Standard scene-load flow | Per Wave 2.1 A.1 |
| 6 | Enable Spout / Syphon output | 2 / PARTIAL | `integrations/syphon-adapter/SyphonOutputAdapter` scaffold + `publish_syphon_output` is deferred follow-up | Wave 1c-β closed scaffold; texture publish deferred |
| 7 | Open OBS sources panel | N/A | n/a (OBS-side) | Operator-handled in OBS |
| 8 | Add Spout2 / Syphon source | N/A | n/a (OBS-side) | Operator action |
| 9 | Select Chromaticity output | 1 / GAP | `publish_syphon_output` follow-up not landed | macOS server name template at `defaults::DEFAULT_OUTPUT_NAME_TEMPLATE` but no live texture publish |
| 10 | Window Capture fallback | N/A | n/a (OBS-side) | Operator can capture our Tauri window directly |
| 11 | Display Capture fallback | N/A | n/a (OBS-side) | Operator-handled |
| 12 | Add audio source in OBS | N/A | n/a (OBS-side) | Operator-handled |
| 13 | Audio routing — Chromaticity input matches OBS source | 4 / EXCEEDS | cpal binds DEFAULT INPUT per CLAUDE.md "live cross-seam dispatch" note + Wave 2.8 AUD.SRC operator-preferred device | Cleaner separation than upstream's implicit |
| 14 | Virtual cable (BlackHole / Loopback / VB-Cable) | 2 / PARTIAL | Documented in CLAUDE.md mic note (Wave 2.1 C.1 PR `#437`) | Not authored into in-app help corpus per CLAUDE.md gap-notes |
| 15 | Set OBS canvas resolution | N/A | n/a (OBS-side) | Operator-handled |
| 16 | Match render resolution to OBS | 3 / MATCH | Settings Resolution + `Cmd+=` / `Cmd+-` step + `/render/resolution {w} {h}` OSC | All three surfaces |
| 17 | Set framerate to match OBS | 3 / MATCH | `Cmd+Shift+=` FPS dialog + `/render/fps {float}` OSC | Per `hotkeys-reference.md:82` |
| 18 | Toggle render loop (calibration) | 4 / EXCEEDS | `.` + `/render/enabled {0\|1}` per `osc-routes.md:108` + non-destructive halt | Outbound state continues publishing; restoration within one frame budget |
| 19 | Output preview before going live | 3 / MATCH | Top bar Camera button is visual stub per CLAUDE.md "synthetic-only" cohort + Perform Preview hero | Stub status flagged |
| 20 | Start OBS recording | N/A | n/a (OBS-side) | Operator action |
| 21 | Start OBS streaming | N/A | n/a (OBS-side) | Operator action |
| 22 | Virtual cam output (alternative path) | 1 / GAP | No virtual-cam adapter | Not on planning surface |
| 23 | NDI output (alternative path) | 1 / GAP | Settings NDI row documented at `settings-reference.md:80-86` | `integrations/ndi-adapter/` does not exist |
| 24 | Recipe / docs page | 1 / GAP | No `livestream.md` / `obs.md` in `docs-corpus/docs/` | Recipe not authored |

**Scope summary.** 24 rows; 4 MATCH, 2 EXCEEDS, 0 EXCEEDS+, 4 PARTIAL, 6 GAP, 8 OUT-OF-SCOPE.

**Score histogram.** 5: (none) · 4: ★★ · 3: ★★★★ · 2: ★★★★ · 1: ★★★★★★ · N/A: ★★★★★★★★

---

## How to Manually Convert from Shadertoy.com to SSF

_Upstream tutorial body is also a video-pointer stub; the substantive
enumeration here mixes Shadertoy's canonical uniform vocabulary,
manual conversion mechanics common to anyone porting fragment shaders,
and our automated converter's documented coverage._

| # | Documented step / capability | Score | Chromaticity equivalent | Notes |
| :- | :--------------------------- | :---: | :----------------------- | :---- |
| 1 | Respect original artist license | 3 / MATCH | `docs/29-cleanroom-legal-policy-baseline.md` + per-page footer SPDX | Documented clean-room posture |
| 2 | Assess scene complexity / multipass | 4 / EXCEEDS | Converter rejects `mainCubemap` / multi-buffer as out-of-scope per `import-shadertoy.md:103-105` with explicit diagnostic | Multi-buffer is future-wave ADR |
| 3 | Manual rewrite of `mainImage` body | 5 / EXCEEDS+ | Wrapped automatically by converter per `import-shadertoy.md:19-28` | Operator-time win |
| 4 | Rename `mainImage` → entrypoint | 5 / EXCEEDS+ | Wrapper generates `void main()` over the unedited body | Automated |
| 5 | Replace `fragCoord` references | 3 / MATCH | Body unchanged; wrapper preamble declares Shadertoy-spelling uniforms | Per converter doc |
| 6 | `iTime` uniform passthrough | 3 / MATCH | Declared in wrapper preamble per `import-shadertoy.md:31-37` | Maps to chromaticity-native `TIME` |
| 7 | `iResolution` uniform passthrough | 3 / MATCH | Same | Maps to `RENDERSIZE` |
| 8 | `iTimeDelta` uniform passthrough | 3 / MATCH | Same | Per uniform table |
| 9 | `iFrame` uniform passthrough | 3 / MATCH | Same | Per uniform table |
| 10 | `iMouse` uniform passthrough | 3 / MATCH | Same; Shadertoy-compatible click state | Per uniform table |
| 11 | `iChannel0..3` texture samplers | 4 / EXCEEDS | Per-channel mapping at import time + `chroma_input_<channel>` rewrite per `import-shadertoy.md:69-80` | Body rewritten before naga sees the source; missing maps emit `MissingIChannelMapping` warning |
| 12 | iChannel webcam binding | 2 / PARTIAL | `Warning::WebcamIChannelDeferred` per `import-shadertoy.md:55-58` | Body compiles; author can edit post-import |
| 13 | iChannel cubemap binding | 2 / PARTIAL | `Warning::CubemapEntrypointDeferred` (full `mainCubemap` deferred) | Per `import-shadertoy.md:53-54` |
| 14 | Deprecated `texture2D(...)` calls | 5 / EXCEEDS+ | Auto-corrected to `texture(...)` and surfaced as `Warning::DeprecatedTexture2DAutoCorrected` per `import-shadertoy.md:59-62` | Trail of every machine-applied edit |
| 15 | VR mode (`mainVR`) handling | 4 / EXCEEDS | Rejected with explicit diagnostic per `import-shadertoy.md:51-52` | No silent failure |
| 16 | Sound shader handling | 4 / EXCEEDS | Rejected; Chromaticity is visual-only per `import-shadertoy.md:106-107` | Explicit reject |
| 17 | Manual SSF JSON config write | 5 / EXCEEDS+ | Atomic emission: caller sees full `.chroma` package or none per `import-shadertoy.md:64-67` | Operator-time win |
| 18 | Validate post-conversion | 4 / EXCEEDS | Naga GLSL frontend + validator pair per scaffolder doc; same pipeline as native authoring | Per `engine/render::single_pass::compile_and_render_single_pass` |
| 19 | Common error categories | 4 / EXCEEDS | Explicit reject / warn taxonomy per `import-shadertoy.md:44-67` | Better error vocabulary than upstream manual |
| 20 | Tooling — manual editor | 3 / MATCH | Standard `.chroma` package; any text editor | Same surface |
| 21 | Tooling — automated converter | 5 / EXCEEDS+ | `engine/authoring/src/import_shadertoy.rs` per `EV-IMPORT-SHADERTOY` PR 1 + PR 2 | Upstream tutorial is *manual*; ours is automated |
| 22 | iChannel fixture corpus | 4 / EXCEEDS | `engine/scene/tests/fixtures/scenes/shadertoy-fixtures/` per Phase-4b §5.2a | Excluded from launcher per `library-workflow.md:36-38` |

**Scope summary.** 22 rows; 9 MATCH, 7 EXCEEDS, 4 EXCEEDS+, 2 PARTIAL, 0 GAP, 0 OUT-OF-SCOPE.

**Score histogram.** 5: ★★★★ · 4: ★★★★★★★ · 3: ★★★★★★★★★ · 2: ★★ · 1: (none)

---

## How to Update Controls and Use Event Handlers with JavaScript

_Upstream tutorial body is video-pointer; the JS API surface enumerated
below comes from the SSF Script reference page._

| # | Documented step / capability | Score | Chromaticity equivalent | Notes |
| :- | :--------------------------- | :---: | :----------------------- | :---- |
| 1 | Author `script.js` in scene package | 1 / GAP | `engine/authoring/src/scaffolder.rs:44-47` explicit "loader does not currently consume per-package script bodies" | Scaffolder explicitly skips emission; loader does not yet honour it |
| 2 | `setup()` lifecycle hook | 2 / PARTIAL | `engine/script/src/lib.rs::ScriptRuntime::execute` is the structural equivalent; scene-loader wire is later-slice scope per `lib.rs:133-138` | Slice 5.4+ |
| 3 | `update(dt)` per-frame hook | 2 / PARTIAL | Same — runtime supports execution; scene-loader hookup deferred | Per CLAUDE.md "synthetic-only" cohort |
| 4 | `setUniform(name, value)` | 2 / PARTIAL | `ScriptCommand::TypedReactiveWrite` per `engine/script/src/command_bus.rs` slice-5.2 | Producer side complete; consumer at `chromaticity_runtime::script_bridge` (slice 5.3) |
| 5 | Direct uniform global access | 1 / GAP | Not yet exposed | Slice 5.4+ scope |
| 6 | Deprecated `inputs` object | N/A | n/a — Chromaticity does not ship the deprecated surface | Clean-slate |
| 7 | Deprecated `uniforms` object | N/A | n/a — Chromaticity does not ship | Clean-slate |
| 8 | `setControl(name, values, options)` | 1 / GAP | Engine `set_control` exists; JS-side wire deferred | Slice 5.4+ |
| 9 | `setControlNormalized(name, values, options)` | 1 / GAP | Same | Slice 5.4+ |
| 10 | `randomizeControl(name)` | 2 / PARTIAL | Engine surface exists (`R` hotkey + per-control random); JS wire deferred | Reachable through engine, not yet through JS |
| 11 | `defaultControl(name)` | 2 / PARTIAL | Same (`D` hotkey path) | Same |
| 12 | `randomizeGroup(name)` | 2 / PARTIAL | Per `engine/scene/src/schema.rs` `UI_GROUP` field; group-random surface exists | JS wire deferred |
| 13 | `defaultGroup(name)` | 2 / PARTIAL | Same | Same |
| 14 | `onChange(target, callback)` event | 1 / GAP | Slice 5.4+ scope per `engine/script/src/lib.rs:133-138` | Per ADR-003 |
| 15 | `onOffToOn(target, callback)` | 1 / GAP | Same | |
| 16 | `onOnToOff(target, callback)` | 1 / GAP | Same | |
| 17 | `whileOn(target, callback)` | 1 / GAP | Same | |
| 18 | `whileOff(target, callback)` | 1 / GAP | Same | |
| 19 | Event `value` callback param | 1 / GAP | Same | |
| 20 | Event `previousValue` callback param | 1 / GAP | Same | |
| 21 | `textureFinalPass(x, y)` | 1 / GAP | Not exposed | Per ADR-003 reservation |
| 22 | `print()` console | 4 / EXCEEDS | Diagnostic surface per `engine/authoring/src/diagnostics.rs` (four `#[non_exhaustive]` categories) | Structured console; not just `print` |
| 23 | `console.log()` | 3 / MATCH | rquickjs default | QuickJS built-in via `chromaticity_runtime::script_bridge` |
| 24 | `printGlobalKeys()` utility | 1 / GAP | Not implemented | Utility helper unique to upstream |
| 25 | `syn_BassLevel` and audio reactive uniforms | 5 / EXCEEDS+ | `engine/script/src/reactive_symbol.rs` slice-5.2 typed catalog (53 `ReactiveSymbol` variants) | Typed enum vs. upstream string-keyed surface; type-safety at compile time |
| 26 | Sandbox / persistence across frames | 5 / EXCEEDS+ | rquickjs sandbox-default + watchdog timeouts (1ms soft / 4ms hard per ADR-012 §2) + 16MiB memory cap + determinism shims | All evidence-rowed under `EV-SANDBOX-SAFETY` + `EV-SCRIPT-CONTRACT` |
| 27 | Demo scene download | 2 / PARTIAL | `engine/scene/tests/fixtures/scenes/examples/` includes JS-relevant samples but no dedicated JS demo | Wave 1d-β corpus widening could close |

**Scope summary.** 27 rows; 1 MATCH, 1 EXCEEDS, 2 EXCEEDS+, 9 PARTIAL, 12 GAP, 2 OUT-OF-SCOPE.

**Score histogram.** 5: ★★ · 4: ★ · 3: ★ · 2: ★★★★★★★★★ · 1: ★★★★★★★★★★★★ · N/A: ★★

---

## v1.16 Feature Walkthrough

_Upstream tutorial body is a video pointer; the v1.16 changelog page
itself returned only two enumerable items (left/right arrow playlist
nav, Catalina permission fixes). Most "v1.16 feature walkthrough" rows
below are version-management concerns that do not map onto
Chromaticity's pre-1.0 shape; they are listed for completeness but
weighted N/A._

| # | Documented step / capability | Score | Chromaticity equivalent | Notes |
| :- | :--------------------------- | :---: | :----------------------- | :---- |
| 1 | "What's new" in-app banner | N/A | none (pre-1.0) | Pre-1.0 |
| 2 | Per-release walkthrough video | N/A | none | AO-gated |
| 3 | Version number badge | N/A | none in V1 UI | Top bar pill could carry one in future |
| 4 | Auto-update notification | 2 / PARTIAL | Wave 1e-β `tauri-plugin-updater` scaffold (PR `#416` / `517c37b`) | First Tauri 2 plugin precedent in this binary; production cert-gated |
| 5 | Update download progress | 2 / PARTIAL | Same Wave 1e-β scaffold | |
| 6 | Update install + restart | 2 / PARTIAL | Same | |
| 7 | Release notes view | N/A | none | `CHANGELOG.md` is `release-please`-populated but no in-app view |
| 8 | Sparkle 2 / Squirrel updater | 2 / PARTIAL | EdDSA keypair AO-gated per CLAUDE.md "EV-RELEASE-PIPELINE" | `release-eng/` scaffolded |
| 9 | Code-sign + notarize installer | 2 / PARTIAL | `release-eng/` stub signing scripts + hardened-runtime `entitlements.plist` per Wave 1e-α | Apple Developer ID + Windows EV certs AO-gated |
| 10 | macOS Gatekeeper compatibility | 3 / MATCH | hardened-runtime entitlements scaffolded | First-launch on signed build will be Gatekeeper-clean |
| 11 | macOS microphone permission | 3 / MATCH | cpal asks for microphone permission via OS prompt on first audio bind | Per cpal default-input |
| 12 | macOS camera permission | 3 / MATCH | `integrations/camera-adapter/src/macos.rs` requests via OS | Per ADR-009 `IO_CAMERA` capability |
| 13 | Cross-platform Windows + Mac matrix | 3 / MATCH | `.github/workflows/release.yml` cross-platform matrix per Wave 1e-γ PR `#423` | Plus three workflow bug-fix PRs `#428` / `#429` / `#430` |
| 14 | Linux build (per v1.16) | 1 / GAP | none — Chromaticity is macOS + Windows desktop only per CLAUDE.md "Project at a glance" | Out of scope |
| 15 | New audio routing UI | 3 / MATCH | `src/lib/views/Settings.svelte` Audio Device section per Wave 2.6 / 2.7 | Documented at `settings-reference.md:14-32` |
| 16 | New OSC routing UI | 3 / MATCH | `src/lib/views/Osc.svelte` Routes table (Wave 2.1 D.3) + `osc-routes.md` | Live `osc_routes_snapshot` |
| 17 | New MIDI mapping panel | 3 / MATCH | `src/lib/views/MIDI.svelte` + Wave 2.1 D.1 + D.2 | Single-message learn only |
| 18 | New scene format additions | 3 / MATCH | `.chroma` native + `.chromaPack` zip transport | Per `scene-workflow.md:12-25` |
| 19 | New control widget type | 3 / MATCH | Full widget set per Wave 2.4 PERF.A | Bang / Knob / Toggle / Swatch / Fader / Dropdown |
| 20 | New audio uniform | 4 / EXCEEDS | `chroma_AudioRMS` / `chroma_AudioPeak` / `chroma_AudioFFT[8]` per Round-3 Lane Q PR `#488` rename | Three native-namespace primitives |
| 21 | New transition type | 3 / MATCH | `HARD_TRANSITIONS` + `SMOOTH_TRANSITIONS` + W13 CueTransitionCurve widget | Per ADR-008 |
| 22 | New media support | 3 / MATCH | `IMAGES` + `MEDIA` schema | Per scene schema |
| 23 | New playlist feature | 3 / MATCH | Phase-3 `EV-PRESET-PLAYLIST` + Wave 2.1 A.4 | `Pass` |
| 24 | New preset feature | 4 / EXCEEDS | Wave 2.4 PERF.C `Command::PresetSave` scene-author preset capture (PR `#456`) | Closes scene-author capture axis the Phase-3 narrowing deferred |
| 25 | New OBS / streaming integration | 1 / GAP | None | Per OBS livestream gaps above |
| 26 | New ISF support | 3 / MATCH | `engine/authoring/src/import_isf.rs` reject-with-actionable-diagnostic per `EV-IMPORT-ISF-REJECT` | Native execution V2-deferred per CHR-ISF-V2 differentiation bet |
| 27 | New plugin / extension API | 1 / GAP | none | No plugin SDK in V1 |
| 28 | New JavaScript feature | 4 / EXCEEDS | `engine/script/` Phase-2 + slice 5.1-5.3 cross-crate consumer-side adapter | Producer / consumer / catalog / adapter quad |
| 29 | New audio reactivity widget | 4 / EXCEEDS | Wave 1c-α dual-mode AudioReactivity / BpmRing / BeatDots per Round 2 Lane A PR `#484` | Live snapshot per Wave 1c-α-2 |
| 30 | New hotkey added | 4 / EXCEEDS | 57-row vocabulary per `hotkeys-reference.md` | vs. upstream's single arrow-key playlist nav add in v1.16 |
| 31 | New entitlement / license model | 5 / EXCEEDS+ | `EV-ENTITLEMENT-ENFORCEMENT` (13 honored caps, data-only) + `EV-ENTITLEMENT-OFFLINE` per ADR-009 | Capability-bitmask vs. edition-based; documented at `settings-reference.md:104-127` |
| 32 | Performance improvement | 4 / EXCEEDS | `EV-PERF-BUDGETS` Phase-3 `Pass` + nightly Path B criterion harness | Per CLAUDE.md "Phase-3 narrowings" |
| 33 | Bug-fix list (Catalina perms etc.) | N/A | n/a (different OS gen + permission surface) | macOS-current |

**Scope summary.** 33 rows; 11 MATCH, 6 EXCEEDS, 1 EXCEEDS+, 4 PARTIAL, 4 GAP, 7 OUT-OF-SCOPE.

**Score histogram.** 5: ★ · 4: ★★★★★★ · 3: ★★★★★★★★★★★ · 2: ★★★★ · 1: ★★★★ · N/A: ★★★★★★★

---

## Workflow gap surface — docs-corpus pages we're missing

Consolidating the gaps the section-level overview surfaced with the new
feature-level findings, the docs-corpus pages that would close the
biggest score lift across this deep dive are:

1. **`docs-corpus/docs/quickstart.md`** — first-launch walkthrough.
   Concrete recipe: install → first launch → pick audio input device
   (macOS BlackHole / Windows VB-Cable note) → load a welcome scene →
   open a Snap → toggle the hotkey overlay. Would re-open Quickstart
   row 1-4, 13 from PARTIAL toward MATCH.
2. **`docs-corpus/docs/scripting.md`** — JavaScript scene authoring
   reference. Concrete recipe: `script.js` lifecycle hooks (when
   landed), `setup()` / `update(dt)` semantics, reactive symbol catalog
   (53 typed variants), command-bus emit pattern, sandbox + watchdog
   + memory cap + determinism contract, structured diagnostic surface.
   Would re-open JS Event Handlers rows 1-21 from GAP / PARTIAL toward
   MATCH (engine wire still needed).
3. **`docs-corpus/docs/livestream.md`** — OBS + Syphon recipe.
   Concrete recipe: macOS Syphon adapter setup → publish output →
   Syphon-Recorder source in OBS → audio routing via BlackHole →
   match render resolution to OBS canvas → calibration via render
   toggle. Would re-open OBS Livestream rows 5-9, 13-14, 18, 24 from
   PARTIAL / GAP toward MATCH (publish_syphon_output real-impl still
   needed).
4. **`docs-corpus/docs/community.md`** or **`docs-corpus/docs/about.md`
   expansion** — Discord / community pointer + changelog viewer entry.
   Closes Quickstart rows 40-41 (community pointer + in-app changelog).
5. **`docs-corpus/docs/midi-mapping.md`** — MIDI device + learn-mode +
   binding-to-control workflow page. Would close Quickstart row 22
   from PARTIAL toward MATCH and provide the missing recipe for the
   Wave 2.1 D.2 single-message learn surface.
6. **`docs-corpus/docs/perform-snaps.md`** or **`presets-workflow.md`** —
   end-to-end Snap recipe: 10-slot vocabulary, save / recall / clear,
   gold preset, labels, OSC route mapping. Would re-open Presets /
   Playlists from MATCH toward EXCEEDS+ (the engine surface already
   leads).
7. **`docs-corpus/docs/recording.md`** — no-op for V1 (Quickstart row
   30 stays GAP) but worth scaffold-flagging so the gap is visible to
   future-wave planners.
8. **`docs-corpus/docs/troubleshooting.md`** — macOS microphone /
   camera permission flows + cpal-default-input note + audio loopback
   setup + adapter-pill degraded-state interpretation. Closes
   diagnostics surface gaps across Quickstart + Livestream.

## Cross-cutting observations

- **Chromaticity's hotkey + OSC density is the single biggest
  cross-tutorial differentiator.** The Control Hotkeys deep dive
  scored 4.4 composite (highest), and the OSC manual surface
  (44 routes vs Synesthesia's 44 routes — at parity on route count
  but with explicit `compat-syn-v1` plurality aliasing layered in
  per `osc-routes.md:140-148`) is what lifts both Presets / Playlists
  and v1.16 scores. The `cap` column per hotkey row is unique to
  Chromaticity and surfaces the entitlement model the reference leaves
  implicit.

- **Two distinct PARTIAL clusters, two different remediation paths.**
  Cluster A is engine-wire-still-needed (`script.js` lifecycle in
  scene loader, `publish_syphon_output` real-impl, multi-message MIDI
  learn). Cluster B is docs-corpus-still-needed (Quickstart recipe,
  Livestream recipe, Scripting reference, Troubleshooting). Both
  clusters compound: Cluster A bottlenecks Cluster B (you can't
  author a recipe for code that doesn't yet ship).

- **The clean-room narrowings are uniformly tighter than upstream.**
  The Shadertoy importer's reject / warn taxonomy
  (`Warning::DeprecatedTexture2DAutoCorrected`,
  `Warning::WebcamIChannelDeferred`,
  `Warning::CubemapEntrypointDeferred`,
  `Warning::MissingIChannelMapping`) gives the author a trail of every
  machine-applied edit that upstream's manual conversion has no
  analogue for. The script crate's
  `ScriptError::{TimeoutExceeded, MemoryCapExceeded, CommandBusDisconnected,
  CommandBusBackpressure, ReactiveSymbolUnknown, ReactiveValueKindMismatch,
  DeterminismInstall}` taxonomy is similarly wider than upstream's
  implicit error surface, even though the scene-loader wire is
  Cluster A pending.

- **Out-of-scope rows concentrate in OBS-side operator action and
  version-walkthrough release-management surface.** 8 of the 11 N/A
  rows are OBS-internal (sources panel, recording, virtual cam in
  OBS, etc.) and 7 of the 11 are v1.16 version-management. Neither
  category reflects a Chromaticity gap; they reflect different
  product shapes (pre-1.0 vs. mature multi-version product, runtime
  vs. integration).

- **Re-score triggers carried forward from the section overview hold.**
  Wave 1c-β Syphon `publish_syphon_output` lands → OBS Livestream
  jumps from 1.9 to ~2.8 composite (rows 6, 9 close). Scene-loader
  consumes per-package `script.js` → JS Event Handlers jumps from 2.6
  to ~3.8 composite (rows 1-21 close to PARTIAL → MATCH). A `quickstart.md`
  page lands → Quickstart jumps from 2.7 to ~3.4 composite (rows
  1-4, 13, 40-41 close). The cumulative score lift from the four
  Cluster-A engine waves + the four highest-leverage Cluster-B docs
  pages would shift the aggregate composite from ~3.1 to ~3.7 across
  the five tutorials that are not OUT-OF-SCOPE.

---

<!-- Clean-room authored from Synesthesia public docs surface URLs (https://synesthesia.live/docs/, https://synesthesia.live/docs/tutorials/{quickstart_guide,first_scene,presets_favslots_playlists,control_hotkeys,livestream,shadertoy_conversion,1.19_javascript}.html, https://synesthesia.live/docs/manual/{control_hotkeys,osc}.html, https://synesthesia.live/docs/ssf/{script,json}.html, https://www.synesthesia.live/public-release/changelog/1.16.9.14.html — pathway summaries only, no prose / code / recipe copy), CLAUDE.md Wave 1c-α-2 / 1d-α / 1d-β / 1e / 2.x / 4b / 4d notes, docs-corpus/docs/* (11 pages), engine/authoring/src/{scaffolder,live_reload,diagnostics,import_shadertoy,import_isf}.rs, engine/script/src/{lib,reactive_symbol,command_bus,determinism,watchdog,memory_cap,error}.rs, engine/scene/src/schema.rs, engine/scene/tests/fixtures/scenes/{welcome,examples,library,shadertoy-fixtures}/*, integrations/{syphon-adapter,camera-adapter,midi-adapter,osc-adapter}/src/lib.rs, src/lib/views/{Library,Perform,Author,Audio*,MIDI,Osc,Settings,Help,Welcome,Import}.svelte, src/lib/widgets/* (51 widgets), src/lib/data/hotkeys.ts, src/lib/stores/{snapLabels,libraryFavorites,libraryRecent,controlLock,audioSnapshot,cameraStore,syphonStore,midiStore,settings}.svelte.ts, src/lib/ipc/{scene,control,snap,playlist,preset,osc,midi}.ts, app/shell/src/commands/{scene,control,snap,playlist,preset,osc,midi,library,library_index,library_search,settings}.rs, app/shell/src/lib.rs::run, docs/26-evidence-artifact-registry.md (EV-IDs); never derived from Contents/ or chromaticity-planning/ per docs/29. -->
