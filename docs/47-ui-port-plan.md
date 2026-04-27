# UI Port Plan - JSX reference mock -> Tauri 2 + Svelte 5 + TypeScript + CodeMirror 6

Port plan from `ui-predev-bundle/` (revision `ux-bundle-v0.1-alpha`, React/JSX + Babel-in-browser mock) to the locked non-scene UI stack defined in `adr/ADR-018-non-scene-ui-toolkit.md`.

This plan is normative for Phase 1 implementation kickoff. Accepted by Accountable Owner + Deputy per `23-decision-ownership-matrix.md` §Delegation before WP-0 exit.

## 1. Scope and intent

The JSX mock is a **design-surface reference**, not production code:

- Component contracts (props, interaction model, keyboard behaviour) are normative.
- Visual language (tokens post-`45-design-token-mapping.md` rename, typography, spacing) is normative.
- **File / folder layout, state model, build tooling, and implementation language** are **not** normative - the port plan below overrides them.

The target stack per ADR-018:

- **Tauri 2** (Rust app shell; macOS WKWebView + Windows WebView2; no bundled Chromium).
- **Svelte 5** (runes mode) + **TypeScript** for the UI tree.
- **CodeMirror 6** for the authoring editor.
- **`ts-rs`** for shared-type generation from the Rust core to the TypeScript UI.
- **wgpu** native scene surface composited under a transparent region of the web view (per ADR-018 §Scene canvas embedding).

Related: gap audit `44-ux-bundle-gap-audit.md`, token mapping `45-design-token-mapping.md`, a11y audit `46-ux-bundle-a11y-color-audit.md`, open-question ledger `48-ux-bundle-open-questions-routing.md`.

MVP entitlement enforcement is **watermark-only** per `adr/ADR-009-activation-and-entitlements.md` Sec.Capability namespace: only `render.watermark.off` is evaluated at runtime in MVP, and the watermark is drawn on the scene canvas by the Rust rendering backend (not by the non-scene UI). Capability-gated affordances that are **not** watermark-enforced must remain **visible and functional** in the MVP UI — they are not locked, muted, or hidden. See `50-ux-bundle-entitlement-surface-scope.md` §2.2 for the dormant-at-MVP capability posture; post-MVP SKUs activate additional capability-lock styling as a data/config rollout, not a UI rewrite.

## 2. Repository layout (post-port)

The Phase 1 repository skeleton aligns with the CODEOWNERS subtree layout in
`43-development-standards.md` §8 (`engine/`, `app/`, `integrations/`, `release-eng/`, with
`workspace-hack/` at root per cargo-hakari convention). The Svelte web root lives at `src/`
at the repository root per Vite convention; splitting it into its own `app/ui/` crate is a
later concern reserved for when the frontend grows beyond a single shell component.

```
/
├── Cargo.toml                       # workspace manifest (edition 2024, MSRV 1.85.0)
├── Cargo.lock
├── rust-toolchain.toml
├── deny.toml
├── package.json                     # pinned per 22-compatibility-versioning-policy.md
├── package-lock.json
├── tsconfig.json
├── tsconfig.node.json
├── svelte.config.js
├── vite.config.ts                   # Tauri-aware dev server
├── eslint.config.js
├── index.html                       # Vite entry; mounts /src/main.ts
├── workspace-hack/                  # cargo-hakari placeholder (at root by convention)
├── app/
│   ├── shell/                       # Tauri 2 binary (chromaticity-app crate)
│   │   ├── Cargo.toml
│   │   ├── tauri.conf.json
│   │   ├── build.rs                 # ts-rs export step
│   │   ├── capabilities/
│   │   │   └── default.json
│   │   ├── icons/
│   │   │   └── icon.png
│   │   └── src/
│   │       ├── main.rs              # Tauri entry
│   │       ├── lib.rs               # run() — Tauri Builder bootstrap
│   │       ├── commands/            # invoke handlers (IPC)
│   │       ├── state/               # app-level state owned by the Rust core
│   │       └── bridge/              # core<->UI marshalling (consumes app/ui-bridge/)
│   ├── ui-bridge/                   # ts-rs-decorated IPC types (chroma-ui-bridge crate)
│   ├── authoring/                   # reserved: authoring environment crate
│   ├── library/                     # reserved: scene library + import crate
│   └── ui/                          # reserved: Svelte UI crate (when src/ splits out)
├── engine/
│   ├── runtime/                     # runtime scheduler, render graph, sandbox (chroma-runtime)
│   ├── scene/                       # scene schema, .chroma package, import (chroma-scene)
│   ├── audio/                       # audio pipeline (chroma-audio)
│   └── entitlement/                 # ADR-009 capability bitmask (chroma-entitlement)
├── integrations/
│   └── io/                          # OSC, MIDI, Syphon, Spout (chroma-io)
├── release-eng/                     # reserved: signing / notarization / updater helpers
├── src/                             # Svelte 5 + TS UI tree (at web root per Vite)
│   ├── app.d.ts                     # svelte + vite/client ambient types
│   ├── main.ts                      # mount <App />
│   ├── App.svelte
│   ├── lib/
│   │   ├── tokens/                  # ADR-018 token exports (CSS custom props + TS constants)
│   │   │   ├── tokens.css           # :root + [data-theme] blocks
│   │   │   ├── tokens.ts            # const tokens = { surface: {...}, ... } for conditional styling
│   │   │   └── themes/              # pulse.css, atelier.css, console.css
│   │   ├── primitives/              # Fader, Knob, Toggle, ControlGroup, Meter, VuStereo, Preview, SceneRow, MediaCard, FxTile, CrossFader, CuePad, Spectrum, BpmRing, BeatDots, Timecode, Pill, Tab, Button, Icon, RailIcon
│   │   ├── hud/                     # Topbar, Rail, StatusBar, FavSlots, HotkeyOverlay
│   │   ├── editor/                  # CodeMirror 6 wrapper + Chromaticity extensions
│   │   ├── ipc/                     # invoke wrappers, event listeners; bindings/*.ts generated by ts-rs
│   │   ├── stores/                  # global Svelte 5 stores (session, theme, density, accent, adapter-health)
│   │   └── a11y/                    # focus-trap, live-region, landmark helpers
│   ├── views/                       # one Svelte file per view
│   │   ├── perform/
│   │   │   ├── +view.svelte
│   │   │   ├── SceneControls.svelte
│   │   │   ├── MetaControls.svelte
│   │   │   ├── MediaStrip.svelte
│   │   │   ├── ScenePanel.svelte
│   │   │   └── AudioReactivity.svelte
│   │   ├── library/
│   │   ├── scene/
│   │   ├── audio/
│   │   ├── playlist/
│   │   │   ├── +view.svelte
│   │   │   └── CueTransitionCurve.svelte
│   │   ├── media/
│   │   ├── midi/
│   │   │   ├── +view.svelte
│   │   │   ├── MidiLearnWaveform.svelte
│   │   │   └── MidiHistoryStrip.svelte
│   │   ├── osc/
│   │   ├── import/
│   │   ├── patch/
│   │   ├── settings/
│   │   │   ├── +view.svelte
│   │   │   ├── Hotkeys.svelte     # read-only render of 42-operator-hotkey-map.md
│   │   │   ├── OscPanel.svelte    # EV-UX-OSC-SETTINGS-CONTRACT full panel
│   │   │   └── MidiPanel.svelte   # EV-UX-MIDI-SETTINGS-CONTRACT full panel
│   │   └── welcome/
│   ├── error-states/                # commissioned in 49-ux-bundle-supplement-scope.md
│   └── entitlement/                 # commissioned in 50-ux-bundle-entitlement-surface-scope.md
├── public/                          # Vite static-assets dir (served at root URL)
│   ├── fonts/                       # bundled per 51-ux-bundle-legal-check.md
│   │   ├── Inter/
│   │   ├── JetBrainsMono/
│   │   └── Fraunces/                # Atelier only
│   └── icons/
├── dist/                            # Vite build output; .gitkeep is load-bearing for Tauri's compile-time frontendDist check
├── scripts/                         # policy / CI shell helpers
├── .github/workflows/               # rust, frontend, sbom, evidence-tag-lint
└── docs/                            # Diátaxis-mapped per 43-development-standards.md §11
```

Key deviations from the JSX mock:

- **One Svelte file per component.** The JSX mock co-locates multiple components per file (e.g. `views/perform.jsx` defines `PerformView`, `SceneControls`, `MetaControls`, `MediaStrip`, `ScenePanel`, `AudioReactivity`). Svelte port splits each into its own `.svelte` file.
- **No `Object.assign(window, {...})`**. Svelte component import is first-class; global window publish is gone.
- **No cache-bust `?v=N` suffixes**. Vite HMR + pinned dependency versions per `22-compatibility-versioning-policy.md`.
- **No Babel-in-browser**. TypeScript + Svelte compiler output runs through Vite at build time; zero runtime transpile.

## 3. State model

### 3.1. Boundaries

The Rust core owns everything that is **state of the world**:

- Session lifecycle, timecode, transport state, active scene, active preset, playlist position.
- Control values (per-scene and meta), Snap slots, mappings.
- Adapter connection state (OSC, MIDI, Syphon, Spout, audio device).
- Entitlement state (capability bitmask per ADR-009).
- License state, update-available flag.
- Performance HUD feed (FPS, GPU%, frame-time percentiles).

The Svelte UI owns only **view-local ephemeral state**:

- Tab selection within a view.
- Search / filter inputs.
- Hover state, drag state, open-popover state.
- Transient UI like the Tweaks panel (dev-only).

### 3.2. Svelte 5 runes vs stores

| Concern | Mechanism | Example |
|---|---|---|
| View-local ephemeral | `$state()` rune inside the component | `let searchQuery = $state('')` in `Library.svelte` |
| Cross-component, view-scoped | Context + `$state()` passed down | `setContext('perform-variant', $state({ variant: 'classic' }))` |
| App-global (session, theme, density, accent, adapter-health) | Svelte 5 store module exporting `$state()` | `stores/session.ts`, `stores/theme.ts` |
| Server-state (Rust-owned) | `$state()` store fed by Tauri event subscription | `stores/adapter-health.ts` subscribes to `adapter-health::changed` and updates `$state({...})` |
| Derived values | `$derived()` | `const bpmDisplay = $derived(session.bpm.toFixed(2))` |
| Side-effects | `$effect()` | `$effect(() => { if (session.tempo) invoke('set-audio-threshold', ...) })` |

### 3.3. IPC boundary

`lib/ipc/` wraps `@tauri-apps/api`:

```
// src/lib/ipc/index.ts
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { SceneLaunchArgs, AdapterHealth, ControlValue } from '$lib/ipc/bindings'  // ts-rs output (generated into src/lib/ipc/bindings/ by app/shell/build.rs)

export const launchScene = (args: SceneLaunchArgs) => invoke<void>('launch_scene', { args })
export const setControlValue = (v: ControlValue) => invoke<void>('set_control_value', { v })
export const onAdapterHealth = (cb: (h: AdapterHealth) => void) => listen<AdapterHealth>('adapter-health::changed', (e) => cb(e.payload))
```

All cross-boundary types live in `app/ui-bridge/` with `#[derive(ts_rs::TS)]` attributes; the `build.rs` step in `app/shell/` runs `ts-rs::export()` to produce `src/lib/ipc/bindings/*.ts`. Contract drift is prevented because the UI cannot compile with stale bindings.

### 3.4. No `localStorage` for cross-boundary state

JSX bundle uses `localStorage` e.g. `views/scene.jsx` L5 `localStorage.getItem("cm.scene.tab")`. Port replaces this with:

- Purely ephemeral view-local state -> `$state()` (lost on view re-mount; acceptable for tab memory).
- Cross-session persistence -> Rust core; UI calls `invoke('get_ui_preference', { key })` / `invoke('set_ui_preference', { key, value })`. The Rust side persists to Tauri app-data directory.

## 4. Per-component translation table

Each row maps a JSX-mock file / component to its Svelte 5 destination. Interaction contracts are preserved; implementation language changes.

### 4.1. Primitives (`lib/primitives/`)

| JSX source | Svelte destination | Notes |
|---|---|---|
| `components.jsx` `Icon` | `lib/primitives/Icon.svelte` | SVG sprite; `name` prop + `size` prop. |
| `components.jsx` `RailIcon` | `lib/primitives/RailIcon.svelte` | `aria-label` + `aria-pressed` required (fixes A-02 from `46-ux-bundle-a11y-color-audit.md`). |
| `components.jsx` `Fader` | `lib/primitives/Fader.svelte` | Native `<input type="range">`. Thumb bumped to 24 CSS px per T-01 fix. Dual readout (`/raw` + normalized) per C-03 from `44-ux-bundle-gap-audit.md`. `D / R / P / L` hotkey hookup via parent-provided context. |
| `components.jsx` `Knob` | `lib/primitives/Knob.svelte` | Becomes `role="slider"` + `aria-valuenow` + full keyboard (arrows, shift-arrows, Home/End, PageUp/PageDown) per A-05 from `46-ux-bundle-a11y-color-audit.md`. |
| `components.jsx` `Toggle` | `lib/primitives/Toggle.svelte` | `role="switch"` + `aria-checked` per A-06. |
| `components.jsx` `ControlGroup` | `lib/primitives/ControlGroup.svelte` | `tint` prop reads from `chroma.cat.*` tokens (product-tier layer). |
| `components.jsx` `Swatch` | `lib/primitives/Swatch.svelte` | Adds alpha-channel variant (C-08). |
| `components.jsx` `Meter` | `lib/primitives/Meter.svelte` | `role="img"` + `aria-label` (already present in JSX). |
| `components.jsx` `VuStereo` | `lib/primitives/VuStereo.svelte` | Same. |
| `components.jsx` `Preview` | `lib/primitives/Preview.svelte` | Preview tile: canvas for the mock; in port it connects to the wgpu preview path (see §6). |
| `components.jsx` `SceneRow` | `lib/primitives/SceneRow.svelte` | Row-level clickable affordance; inner hit-targets audited per T-06. |
| `components.jsx` `MediaCard` | `lib/primitives/MediaCard.svelte` | Keyboard activation. |
| `components.jsx` `FxTile` | `lib/primitives/FxTile.svelte` | Same. |
| `components.jsx` `CrossFader` | `lib/primitives/CrossFader.svelte` | Same. |
| `components.jsx` `CuePad` | `lib/primitives/CuePad.svelte` | Maps to Snap slot surface; count = 16 (F-01). |
| `components.jsx` `Spectrum` | `lib/primitives/Spectrum.svelte` | Accepts `chroma.band.*` tokens; five bands (fixes X-01 `band-full` missing). |
| `components.jsx` `BpmRing` | `lib/primitives/BpmRing.svelte` | Functional motion retained (exempt from motion.instant per RM-01). |
| `components.jsx` `BeatDots` | `lib/primitives/BeatDots.svelte` | Decorative motion; reduce-motion compliant. |
| `components.jsx` `Timecode` | `lib/primitives/Timecode.svelte` | Session-scoped per Q1 recommendation in `48-ux-bundle-open-questions-routing.md`. Monospace + tabular-nums. |

### 4.2. Shell (`lib/hud/`)

| JSX source | Svelte destination | Notes |
|---|---|---|
| `Chromaticity.html` topbar | `lib/hud/Topbar.svelte` | Brand, view label, variant tabs, session HUD, pills (OUT / MIDI with full state taxonomy per H-09), stats (FPS / GPU), actions. |
| `Chromaticity.html` rail | `lib/hud/Rail.svelte` | Semantic `<nav>`; 12 view icons + account / help foot. |
| `extras.jsx` StatusBar | `lib/hud/StatusBar.svelte` | 22px strip. |
| `extras.jsx` FavslotsBar | `lib/hud/SnapBar.svelte` | Renamed Snap (per Brand policy F-03). 16 slots. |
| `extras.jsx` PresetDropdown | `lib/hud/PresetDropdown.svelte` | |
| `extras.jsx` PreviewTools | `lib/hud/PreviewTools.svelte` | |
| `extras.jsx` ControlHotkeysHint | `lib/hud/ControlHotkeysHint.svelte` | Regenerated from `42-operator-hotkey-map.md` mechanically per K-02. |
| `Chromaticity.html` HotkeyOverlay | `lib/hud/HotkeyOverlay.svelte` | Same - mechanical render of `42-operator-hotkey-map.md` sections 1-10. |

### 4.3. Views (`views/`)

| JSX source | Svelte destination | Sub-components |
|---|---|---|
| `views/perform.jsx` | `views/perform/+view.svelte` | SceneControls / MetaControls / MediaStrip / ScenePanel / AudioReactivity (one `.svelte` each) |
| `views/library.jsx` | `views/library/+view.svelte` | Filters, grid, detail popover |
| `views/scene.jsx` | `views/scene/+view.svelte` | Editor (CodeMirror 6, §5) + Inspector tabs (Controls / Inputs / List). Inspector renames from "SSF" to "Inputs" per Q2 recommendation. |
| `views/audio.jsx` | `views/audio/+view.svelte` | Spectrum hero, 5 band cards (adds `full`), 4 BPM-oscillator sparklines |
| `views/playlist.jsx` | `views/playlist/+view.svelte` | Cue table + CueTransitionCurve (SVG bezier; keyboard per A-08) |
| `views/media.jsx` | `views/media/+view.svelte` | Asset browser, collections |
| `views/midi.jsx` | `views/midi/+view.svelte` | MidiLearnWaveform / MidiHistoryStrip. Full device-allowlist, MIDI-CONF-16..20 rows, BPM-sync (§"MIDI view scope expansion" below). Semantic `<table>` for mapping rows (A-01). |
| `views/osc.jsx` | `views/osc/+view.svelte` | Full OSC Operator Settings Panel per `08-io-integrations.md` §OSC Operator Settings Panel Contract (O-01 through O-17). Semantic `<table>`. |
| `views/import.jsx` | `views/import/+view.svelte` | Native scene-package inputs are `.chroma` (directory) and `.chromaPack` (archive) per `38-scene-library-and-import-spec.md` §Package formats. Shadertoy accepted via on-import conversion to `.chroma` per the same spec; conversion diagnostics surface in the import view. ISF rejected with diagnostic copy per L-05. |
| `views/patch.jsx` | `views/patch/+view.svelte` | Keyboard nav between nodes / pins per A-09 |
| `views/settings.jsx` | `views/settings/+view.svelte` | Hosts Hotkeys / OSC / MIDI subpanels |
| `views/onboard.jsx` | `views/welcome/+view.svelte` | First-run / changelog |

### 4.4. Inspector and editor

| JSX source | Svelte destination | Notes |
|---|---|---|
| `ssf_inspector.jsx` | `views/scene/InputsInspector.svelte` | Retargeted from SSF/ISF to `.chroma` input schema per Q2 + `36-reactive-symbol-catalog.md`. Auto-generates controls from `.chroma` `manifest.json` `inputs` array. |
| `cue_curve.jsx` | `views/playlist/CueTransitionCurve.svelte` | Bezier editor; transition shape catalog mapped to `adr/ADR-008-transition-and-traveler-semantics.md` shape IDs per F-04. Keyboard for bezier handles per A-08. |
| `screenshot_modal.jsx` | `lib/hud/ScreenshotModal.svelte` | `aria-modal="true"` + focus trap + labelledby per A-10. Format picker ships PNG + JPEG only per Q4 strawman (drop EXR at v0.2-beta). |
| `empty_states.jsx` | `lib/primitives/EmptyState.svelte` + `views/*/Empty*.svelte` | Shared primitive + per-view copy. |

### 4.5. Removed / renamed

| JSX concept | Disposition |
|---|---|
| `Object.assign(window, {...})` publish convention | Removed; Svelte ES-module import. |
| `?v=N` cache-bust suffix | Removed; Vite + pinned deps. |
| `cm-c` / `cm-k` / `cm-u` / `cm-n` / `cm-s` custom regex highlighter in `views/scene.jsx` | Replaced by CodeMirror 6 modes (§5). |
| Babel-standalone CDN | Removed. |
| `shaders.jsx` procedural canvas renderers for Preview tiles | Replaced by wgpu preview path (§6). Preview still has a CSS-only fallback for the non-scene Preview tiles in e.g. the Import preview where no runtime preview exists. |

## 5. CodeMirror 6 extension inventory (Scene editor)

Target: `lib/editor/ChromaEditor.svelte` is a thin Svelte wrapper over a CodeMirror 6 `EditorView` configured with the extensions below.

### 5.1. Language modes

| Extension | Purpose | Package |
|---|---|---|
| `@codemirror/lang-javascript` | Script pane syntax highlighting | `@codemirror/lang-javascript` |
| Custom GLSL stream language | Scene shader syntax highlighting | `@codemirror/language` + `@lezer/highlight`; GLSL grammar derived from the `lezer-glsl` community grammar if license-clear, else hand-rolled StreamLanguage |
| `@codemirror/lang-json` | `.chroma` `manifest.json` editing | `@codemirror/lang-json` |

Per `51-ux-bundle-legal-check.md`, GLSL grammar license must be MIT / Apache-2.0 / BSD-3-Clause for `EV-LICENSE-AUDIT` pass.

### 5.2. Chromaticity-specific extensions

Each lives in `lib/editor/extensions/`:

| Extension | Purpose | Feeds |
|---|---|---|
| `chromaSymbolCompletion` | Autocompletion for `syn_*` symbols driven by `36-reactive-symbol-catalog.md`; catalog ships as JSON shipped with the app + a compile-time constant. | `EV-SYMBOL-CATALOG` |
| `chromaSymbolHoverDoc` | Hover-doc tooltip showing catalog entry (type, range, source, version). Fixes S-02 + A-06 from `44-ux-bundle-gap-audit.md`. | `EV-SYMBOL-CATALOG`, `EV-AUTHORING-UX-LATENCY` (must hit NFR-10 p95) |
| `chromaDiagnostics` | Compile / link errors, script runtime errors from the Rust core rendered inline. Fixes A-03. | `EV-AUTHORING-DIAGNOSTICS` |
| `chromaLiveReloadIndicator` | Gutter indicator showing last-rebuild timestamp + "reloading" state. Fixes A-04. | `EV-AUTHORING-UX-LATENCY` |
| `chromaHotkeyBridge` | CodeMirror-scoped authoring hotkeys per `37-authoring-environment-spec.md` §4; respects focus-context priority rules per `42-operator-hotkey-map.md` §"Conflict and priority rules". | `EV-HOTKEY-CONFORMANCE` |
| `chromaSoftTabPolicy` | Configures tab size per `37-authoring-environment-spec.md` §3.2 (2-space tab default, configurable). | - |
| `chromaThemes` | `pulse` / `atelier` / `console` CodeMirror themes consuming ADR-018 tokens. | `EV-UX-AUTHORING-CONTRACT` |

### 5.3. Standard extensions

Standard CM6 set, pinned per `22-compatibility-versioning-policy.md`:

- `basicSetup` (line numbers, fold gutter, highlight active line, history, bracket matching).
- `@codemirror/search`.
- `@codemirror/commands` (standard key bindings; overridden where they collide with authoring hotkeys per `42-operator-hotkey-map.md` §Conflict rules - e.g. `Cmd+R` is reload-scene in operator scope, force-rebuild in authoring scope).
- `@codemirror/autocomplete`.
- `@codemirror/lint` (wired to `chromaDiagnostics`).

### 5.4. Latency budget (NFR-10)

CodeMirror 6 keystroke-to-paint baseline on GPU-A hardware must meet the p95 budget in `37-authoring-environment-spec.md` §3.3a. Measurement harness lives in `engine/runtime/tests/ui_latency.rs` plus a Playwright side-run against the system web view.

Initial target: p95 keystroke-to-paint **< 16ms** on GPU-A (one frame budget at 60Hz). If hover-doc or completion latency miss the NFR-10 budget, drop down-stream symbol lookups to a Rust-side precomputed trie rather than a linear scan over the catalog JSON.

## 6. Scene-surface composition (wgpu under transparent web view)

Per ADR-018 §Scene canvas embedding: wgpu output renders to a native surface composited **under** a transparent region of the web view. Operator fullscreen path exits the web view entirely; wgpu surface goes full-screen natively.

### 6.1. Phase 1 spike (called out in ADR-018 §Consequences)

The spike validates:

1. Native surface attach / detach as the user navigates between views (only Perform / Scene / Audio views embed the preview; others don't).
2. Colour-space preservation on profile switch (sRGB display -> P3 display mid-session): scene canvas must not reinterpret; web view continues sRGB.
3. Fullscreen transition timing: wgpu exits web-view bounds, resizes to display dimensions, and returns under the web view cleanly on `Esc`.
4. Resize correctness at DPI scales 1.0x / 1.25x / 1.5x / 1.75x / 2.0x / 2.5x / 3.0x per ADR-018 constraint 7.
5. Preview tiles in non-Perform / non-Scene views (Library, Playlist cue inspector): either a lightweight second wgpu surface or a CPU-decoded preview frame blitted into the web view. Recommend CPU-decoded preview for off-stage views to avoid GPU contention; measure against NFR-10 budgets.

Spike output: `engine/runtime/spikes/wgpu-composition/` with a README, measurement CSVs, and a go / no-go verdict. Gate: `EV-RENDER-CONFORMANCE` downstream.

### 6.2. Preview-tile fallback

For views where the wgpu surface is not composited (Library grid, Media manager, Patch, MIDI mapping preview):

- Rust core periodically (1-5Hz) reads back the current scene's rendered frame into a compressed JPEG / WebP; sends to the UI via Tauri event.
- UI renders the latest preview frame in an `<img>` tag.
- On hover, UI requests a higher-frequency feed (15Hz) for the hovered tile only.

This is materially cheaper than running many wgpu surfaces and fits the NFR-10 latency budget because the HUD UI does not need scene-rate updates on off-stage previews.

## 7. Shared types: `ts-rs` boundary

Every cross-boundary type is defined once in Rust and exported to TypeScript via `ts-rs`.

### 7.1. Canonical type inventory

These types generate under `src/lib/ipc/bindings/`:

Types are defined in their owning domain crate and re-exported (or type-aliased) through
`app/ui-bridge/` so `#[derive(ts_rs::TS)]` attaches in one place and the shell's
`build.rs` aggregates a single export.

| Type | Owning crate (path) | Consumers |
|---|---|---|
| `ControlValue`, `ControlBank`, `ControlKind`, `ControlLock` | `engine/scene` | Fader / Knob / Toggle / Swatch primitives |
| `SceneId`, `ScenePackageRef`, `SceneCategory` | `engine/scene` | Library, Perform, Playlist |
| `PresetRef`, `SnapSlotRef`, `PlaylistRef`, `PlaylistPosition` | `engine/scene` | Snap bar, Playlist view |
| `TransitionShapeId`, `CueRow`, `BeatSnapDivisor` | `engine/scene` | CueTransitionCurve, Playlist |
| `AudioBand`, `AudioUniform`, `AudioThreshold` | `engine/audio` | Audio view, Spectrum, Patch pins |
| `OscRoute`, `OscSettings`, `OscRateLimit`, `OscNetworkExposure` | `integrations/io` | OscPanel, OSC view |
| `MidiMapping`, `MidiMode`, `MidiDeviceId`, `MidiAllowlist`, `MidiBpmSource` | `integrations/io` | MidiPanel, MIDI view |
| `AdapterHealth`, `AdapterKind`, `AdapterState` | `integrations/io` | Topbar pills, degraded banner |
| `OutputRouting`, `OutputKind`, `OutputPlatform` | `integrations/io` | Output routing HUD (H-10 fix) |
| `EntitlementBitmask`, `CapabilityId`, `WatermarkState` | `engine/entitlement` | Entitlement surface |
| `PerfSample`, `PerfPercentile`, `GpuTier` | `engine/runtime` | Performance HUD (H-02 fix) |
| `CrashResumeState`, `SupportBundleRef` | `engine/runtime` | Crash-resume banner, support-bundle entry |
| `SymbolCatalogEntry`, `SymbolKind`, `SymbolRange` | `engine/scene` | Editor hover-doc, Inputs Inspector |
| `ChromaShowRef`, `ChromaPackageRef`, `ChromaPackKind` | `engine/scene` | Open / Save / Export |

`ChromaPackKind` distinguishes the two native scene-package forms per `38-scene-library-and-import-spec.md` §Package formats: `.chroma` (directory) and `.chromaPack` (archive). Both surface through the same Open / Save / Export flow.

`OscRoute` enumerates the canonical OSC route surface per `08-io-integrations.md` §OSC Planning Contract. The Snap apply route is the canonical `/favslots/{n}` (explicit `apply`; **not** `/favslots/{n}/apply`), with `/favslots/{n}/save` and `/favslots/{n}/clear` on the save / clear suffixes; the legacy `/favslots/{n} {mode}` string-argument form is accepted only as a `compat-syn-v1` alias, never rendered as canonical in the OSC panel.

`OscSettings` carries an `OscNetworkExposure` field that defaults to **Loopback only** (`127.0.0.1` inbound bind) per `adr/ADR-013-networking-defaults-and-security.md` §1 Bind defaults and `08-io-integrations.md` §OSC Operator Settings Panel Contract. Exposing OSC on a routable interface is an explicit operator opt-in; the UI must not render `0.0.0.0` as a default. The panel surfaces the three-option radio (Loopback only / Local subnet / Allowlisted IPs) the contract specifies, with Loopback preselected on first run.

### 7.2. Build pipeline

1. `cargo build --release` in `app/shell/` triggers `build.rs`.
2. `build.rs` calls `ts_rs::export_to("src/lib/ipc/bindings/index.ts", ...)`, aggregating the
   types re-exported from `app/ui-bridge/`.
3. Vite picks up the changed bindings on the next rebuild.
4. CI fails if the committed bindings are out of date vs a fresh export (`EV-COMPAT-PROFILE-DIFF` consumes this).

#### Bridge crate location

The bridge crate lives at `app/ui-bridge/` as a sibling to the reserved `app/ui/` subtree
and to `app/shell/`. Alternatives considered:

- **`engine/ui-bridge/`** was rejected because `ts-rs` is a TypeScript-generation concern,
  and engine crates are pure runtime / domain logic. If the desktop shell were swapped for
  a CLI or headless runner, the engine should not need to change — a layering indicator
  that the bridge does not belong on the engine side.
- **`app/shell/bindings/`** (a module of the shell) was rejected because engine crates may
  eventually need to depend on the bridge (to attach `#[derive(TS)]` through re-export),
  and `engine/* → app/shell/*` would be an illegal dependency direction. Co-locating the
  bindings inside the shell crate would close off the re-export pattern.
- **`app/ui-bridge/`** (chosen) is app-layer infrastructure whose purpose is to serve the
  UI's type contract. Dependency direction is clean: `engine/* → app/ui-bridge →
  app/shell → (UI TS output)`. The crate is a peer of the future `app/ui/` Svelte crate
  and of `app/shell/`, which matches the CODEOWNERS convention for app subtrees.

CODEOWNERS carries a dedicated `/app/ui-bridge/` row under "App subtrees"; see
`43-development-standards.md` §8.1.

### 7.3. Enum serialisation

All enums serialise tag-only (serde `#[serde(rename_all = "kebab-case")]` on enum variant) to match OSC route strings and docs glossary terminology. Example:

```
#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterState {
  Off,
  Warming,
  Ok,
  RateLimited,
  Disconnected,
  Blocked,
}
```

This makes the TypeScript union type: `"off" | "warming" | "ok" | "rate-limited" | "disconnected" | "blocked"` - the same strings the docs generator renders.

## 8. Version pinning (per `22-compatibility-versioning-policy.md`)

Initial pinned versions (for Phase 1 kickoff; bumps run full regression suite per ADR-018 §Enforcement):

| Dependency | Pinned version | License | Why | Source of truth |
|---|---|---|---|---|
| Tauri | 2.0.x (latest stable at kickoff) | MIT / Apache-2.0 | ADR-018 toolkit decision | `app/shell/Cargo.toml` |
| Svelte | 5.x (runes mode) | MIT | ADR-018 toolkit decision | `package.json` |
| TypeScript | 5.x | Apache-2.0 | ADR-018 | `package.json` |
| Vite | latest major | MIT | Dev server | `package.json` |
| CodeMirror | 6.x | MIT | ADR-018 editor choice | `package.json` |
| `ts-rs` | latest stable | MIT / Apache-2.0 | Shared-type boundary | `app/ui-bridge/Cargo.toml` |
| `serde` / `serde_json` | latest 1.x | MIT / Apache-2.0 | Serialisation | workspace |
| `wgpu` + `naga` | locked per `adr/ADR-011-rendering-backend.md` | MIT / Apache-2.0 | Rendering backend | workspace |

Exact version strings land at Phase 1 kickoff in the repo's `Cargo.lock` + `package-lock.json`.

All UI dependency licenses fed into `EV-LICENSE-AUDIT` per `51-ux-bundle-legal-check.md`.

## 9. Token delivery: CSS custom properties + TS constants

ADR-018's token surface exports **two parallel** representations so Svelte components can choose:

- `src/lib/tokens/tokens.css` - CSS custom properties under `:root` with `[data-theme="..."]` overrides. This is the default path: components use `var(--surface-base)` etc.
- `src/lib/tokens/tokens.ts` - TypeScript constants mirroring the same names. Use where conditional styling or canvas rendering needs the raw value (e.g. `chromaSymbolHoverDoc` renders into a CM6 widget that can't use CSS vars cleanly).

`eslint-plugin-chromaticity-ui-tokens` (Phase-0 bootstrap lint per ADR-018 §Enforcement) enforces: no hard-coded colours / spacing / radii / motion in `.svelte` files; all must go through tokens. The plugin is a separately-scoped deliverable — its in-repo location (likely under `release-eng/` tooling) or out-of-repo publication choice is deferred to the work package that lands the first token-lint gate.

## 10. Build + run workflow

### 10.1. Dev loop

- `npm run dev` starts Vite on `http://localhost:1420`.
- `npm run tauri dev` starts the Tauri shell, which auto-targets the Vite dev server.
- Hot reload works for Svelte + TypeScript; Rust changes trigger a Tauri rebuild.
- `npm run tauri dev` auto-triggers `build.rs` -> `ts-rs` export when Rust types change.

### 10.2. Production build

- `npm run build` produces `dist/`.
- `npm run tauri build` bundles into platform installers (`.dmg` + `.app` on macOS, `.msi` + `.exe` on Windows).
- Signing / notarization per `adr/ADR-014-signing-notarization-auto-update.md`.

### 10.3. CI

- `npm run check` (svelte-check) + `npm run tsc` (tsc --noEmit, both tsconfigs) + `npm run lint` (eslint; token lint added when `eslint-plugin-chromaticity-ui-tokens` lands).
- `cargo test --workspace --all-targets` (Rust + ts-rs export drift check).
- Vitest for UI logic shipped W11 PR #119 (commit `a62ef90`) — `vitest.config.ts` with jsdom environment + `@testing-library/svelte` + `@testing-library/jest-dom`; first test target is the `<ErrorMessage>` primitive at `src/lib/primitives/ErrorMessage.test.ts`. Run via `npm test` (or `npm run test:watch`). Playwright for axe-core fixtures per `46-ux-bundle-a11y-color-audit.md` §7 lands when accessibility CI gating activates.
- `cargo tauri build` dry-run on both platforms.

## 11. Implementation sequence

Port work lands inside Phase 1 implementation, not before bundle v0.2-beta. Execute in this order:

| Sequence step | Dependency posture | Exit criteria |
|---|---|---|
| S1. Tauri 2 + Svelte 5 scaffold + token surface | Start here | `src/` builds under Vite; `app/shell/` launches; token CSS custom properties compile; Pulse theme renders an empty shell. |
| S2. Primitives port | After S1 | All `lib/primitives/*.svelte` ported with ARIA + keyboard complete per §4.1. Axe-core fixture (single view + primitives) passes. Focus-trap primitive in `lib/a11y/` ready for dialog consumers (screenshot modal, license panel, entitlement-locked navigation). |
| S3. Shell + Topbar + Rail + StatusBar + HotkeyOverlay | After S2 | `lib/hud/*.svelte` complete with adapter-health taxonomy wired to Rust stub events. Command-palette action registry scaffolded (action contract per `46-ux-bundle-a11y-color-audit.md` a11y matrix when it lands); keyboard-only navigation tour defined through Rail + Topbar + HotkeyOverlay. |
| S4. Perform / Library / Playlist views | After S3 | Three views functional against stub Rust core; Snap bar with 16 slots; transition shape catalog wired. |
| S5. Scene editor + CodeMirror 6 + Inputs Inspector | After S4 | Editor with GLSL / JS / JSON modes; symbol completion + hover-doc from catalog; diagnostics; live-reload indicator. NFR-10 latency measurement passes on GPU-A baseline. |
| S6. OSC + MIDI settings panels (full 20+ row per gap audit) | After S4 (parallel with S5 allowed) | `EV-UX-OSC-SETTINGS-CONTRACT` + `EV-UX-MIDI-SETTINGS-CONTRACT` behaviour complete. |
| S7. Audio / Import / Patch / Media / Settings / Welcome | After S5 + S6 | Remaining views functional. |
| S8. Scene-surface composition spike | Parallel with S4-S7 allowed | `EV-RENDER-CONFORMANCE` spike verdict; fullscreen transition + DPI scaling + colour-space preservation validated. |
| S9. Error / degraded / crash-resume / update / entitlement surfaces | After S6 + S7 | `49-ux-bundle-supplement-scope.md` + `50-ux-bundle-entitlement-surface-scope.md` landed. |
| S10. Accessibility + color-management CI gates green | Final sequence gate | axe-core matrix passes; `EV-ACCESSIBILITY-AA` Phase-prep complete; `EV-COLOR-MANAGEMENT-SRGB` Phase-prep complete. |

S1-S10 covers the UI portion of Phase 1 only. Runtime (wgpu pipeline, audio analysis, adapters) lands in parallel under WP-1..WP-8 per `12-work-breakdown-estimates.md`.

### 11.1 Wave delivery log (Phase-1)

Wave-by-wave UI port progress; each wave maps onto the S1-S10 sequence above as noted. Phase-1 EXIT was achieved at W12 (commit `e818752` for the sixth evidence gate close, `0d83fb5` for the wave close including PR3 shell scaffolding).

| Wave | Scope | Sequence step | Reference |
|---|---|---|---|
| W6 | ThemeSwitcher primitive + tokens cascade across three themes (Pulse / Atelier / Console). | S1 (token surface complete) | `45-design-token-mapping.md` |
| W7 | Housekeeping sweep + body paint; OSC observability follow-up landed `OscHealthState::{Healthy, Recovering, Degraded}` per `EV-OBSERVABILITY` mitigation. | S1 | `13-risk-register.md` R-05 |
| W8 | HotkeyOverlay + 5 widget primitives (Fader, Knob, Toggle, Swatch, ControlGroup) + EmptyState. | S2, S3 | §4.1; `42-operator-hotkey-map.md` |
| W9 | Library backbone — filesystem scan IPC + view + search/chips/size selector. | S4 | `38-scene-library-and-import-spec.md` §2 |
| W10 | Inputs Inspector view + scene-fixtures. | S5 (inspector subset) | `37-authoring-environment-spec.md` §3 |
| W11 | ErrorMessage primitive + IpcError typed discriminated union (`src/lib/types/ipc-error.ts`) + Vitest scaffold (`vitest.config.ts` + jsdom + `@testing-library/svelte` + `@testing-library/jest-dom`) + Tauri ship-time bundling of three demo `.chroma` packages via `bundle.resources` (W11 PR #118 / #119 / #120). | S2, S4 | `EV-COMPAT-FIXTURE-COVERAGE` Pass; `EV-LIBRARY-BUNDLE-LAYOUT` partial advance |
| W12 | Reactive-symbol scene-import T2 corpus + scene-lint pipeline (closes `EV-SCENE-IMPORT` and `EV-SCHEMA-LINT`); app shell scaffolding (AppShell + SidebarRail 12-icon-4-wired + StatusBar + RouteStore + Perform/Library/Author/Settings views) (W12 PR #121 / #122 / #123). **Phase-1 EXIT.** | S3, S4 | `EV-SCENE-IMPORT` Pass; `EV-SCHEMA-LINT` Pass; `EV-EVIDENCE-TAG-COVERAGE` Pass |
| W13 | CueTransitionCurve widget primitive (cubic-bezier editor with focus-locked overlay) + Playlist v2 view (cue table + per-cue transition shape) + ui-predev bundle citation policy in `CLAUDE.md`; cue-curve reactivity fix on prop change (PR #125-#128). | S4 | bundle `cue_curve.jsx`, `playlist.jsx`; wireframe 03-playlist-cue-table |
| W14 | Settings view fleshing (hotkeys table + ThemeSwitcher) + OSC view with adapter-health stub IPC (`Healthy` / `Recovering` / `Degraded`) (PR #129-#130). | S6, S7 | `49-ux-bundle-supplement-scope.md` AdapterPill taxonomy; bundle `osc.jsx`, `settings.jsx`; wireframe 06-osc-link-settings |
| W15 | Chrome alignment with v0.2-beta wireframes — TopBar session-HUD primitive across all views + StatusBar metrics row + Library setlist sidebar (PR #131-#133). | S3 deepening | bundle `components.jsx::TopBar` / `StatusBar`, `extras.jsx`; wireframe 01-library-scene-grid |
| W17 | Sidebar v0.2 redesign (13-icon vocabulary + four new routed stub views) + five view full-ports — Welcome (onboarding), License (entitlement), Sources (live media), Audio Uniforms; OSC routes table six-col layout refactor (PR #134-#139). | S7 | bundle `onboard.jsx`, `license.jsx`, `media.jsx`, `audio.jsx`, `osc.jsx` |
| W18 | Perform / Author / MIDI / Patch full-ports from v0.2-beta JSX; TopBar brand mark (conic-gradient ring + accent dot) + chromatic-gradient "Chromaticity" wordmark + ControlGroup `--tint` CSS-variable cascade to Fader / Knob / Toggle children for category-coloured widgets (PR #140-#144). | S4, S5, S7 | bundle `perform.jsx`, `scene.jsx`, `midi.jsx`, `patch.jsx`, `components.jsx::TopBar` |
| W19 | Icon component primitive (50+ named SVG paths vendored from `components.jsx::Icon`, single ladder under `viewBox="0 0 24 24"` with `currentColor` cascade) + SidebarRail glyph→Icon swap + Import view full port (PR #145-#146). | S2, S7 | bundle `components.jsx::Icon`, `import.jsx`; wireframe 07-import-shadertoy-isf |
| W20 | View-internal icon sweep — 13 Unicode glyphs in Perform replaced with vendored Icon primitive; remaining v0.2 ports converted in a single follow-up sweep (PR #147-#148). | S7 chrome-consistency | bundle `components.jsx::Icon` vocabulary |
| W21 | Author CodeMirror 6 editor (token-driven theme + GLSL/JSON/JS modes + 147-entry `chroma_*` / `syn_*` autocomplete + save / revert / load-backup / compare-scene popovers) + Settings expansion (Audio Device, Graphics/Output, Storage, Backups/Cache; new `pick_folder` Tauri command stub) + Test Card preview mode in Perform + global Console Drawer overlay with severity-filter chips + clean-room namespace fix (`syn_Timecode` → `chroma_Timecode`; tokenizer extended to recognise both `syn_*` and `chroma_*`) (PR #149-#153). | S5 closure on editor; S7 | `36-reactive-symbol-catalog.md` §Namespace governance; `ADR-018` |
| W22 | Library expansion CTAs (`+ New Scene` modal stub + `↻ Refresh Index` + `Get More Scenes` browser-mode-disabled link) + Tempo & Sync settings section (BPM source priority chain + tap tempo + manual BPM + link offset + audio-bpm sensitivity) + Sources view split into Local Sources + Live Sources (Spout/NDI) with per-platform availability chips (PR #155-#157). Pre-wave docs/47 wave-log catchup through W21 (PR #154). | S4, S6, S7 | bundle `library.jsx`, `audio.jsx`, `media.jsx`; wireframe 01-library-scene-grid |
| W23 | Bundle primitive vendoring close-out — `BpmRing` + `BeatDots` in AudioUniforms hero (audio visualizer trio with the existing `Spectrum`) + `CrossFader` + `VuStereo` in Perform (A↔B scene mix + L/R level meter) + `CuePad` in MIDI Virtual Pad + pad-controller selector dropdown (Launchpad X + APC40 mkII) + bundle-faithful per-pad rainbow hue palette closing the W23 PR3 invented-state-vocab gap (PR #158-#161). | S2, S4 | bundle `components.jsx::{BpmRing, BeatDots, CrossFader, VuStereo, CuePad}` |
| W24 | Pad-capable controller roster extension — Push 2 (Ableton) + Launchpad Pro Mk3 (Novation) + APC mini Mk2 (Akai) join the W23 PR4 `MIDI_DEVICES` allowlist; bundle-faithfulness audit fallout — animate the MIDI learn-mode waveform + history strip per the bundle's deterministic synthetic generators (the W18 port shipped them static, citing Phase-3+/4a scope; that scope is for real device data, not synthetic visual animation) (PR #162-#163). | S6 | bundle `views/midi.jsx::{MidiLearnWaveform, MidiHistoryStrip}` |
| W25 | Audit-fallout continuation — animate AudioUniforms `Spectrum` (64-bar FFT with per-bar peak-hold) + 8 BPM-card `Sparkline`s per the bundle's per-frame formulas (the W17 port shipped them static); port the bundle's `AudioReactivity` panel composite (4 band tiles + animated 48-bar FFT + tempo strip) to Perform's right column (the W18 port skipped wholesale because the bundle uses `<canvas>`; inline-SVG bars satisfy the pre-ADR-011 no-canvas constraint while preserving the visual contract) (PR #164-#165). | S4, S7 | bundle `components.jsx::Spectrum`, `views/perform.jsx::AudioReactivity` |
| W26 | Perform view 3-column layout refactor matching the bundle's classic `PerformView` variant — wrapper `.perform__right` split into `.perform__col-middle` (Preview hero + AudioReactivity) and `.perform__col-right` (ScenePanel) under a `1.05fr 1.55fr 1fr` grid with responsive 2-col-wrap (1100-1399px) and 1-col-stack (<1100px) breakpoints; closes the column-3 deferral the W25 PR2 AudioReactivity port flagged inline (PR #167). | S4 deepening | bundle `views/perform.jsx::PerformView` (classic + dense variants) |
| W27 | SnapBar primitive port (eight named bookmarks Intro / Drop 1 / Break / Bass / Build / Ambient / Drop 2 / Outro plus disabled "Save current state" plus-slot, hue-rotating gradient surrogates for unported `<SceneStill>`, per-slot `--tint` mapped onto Chromaticity semantic tokens) mounted as Perform-route-gated chrome in `AppShell.svelte` per the bundle's `showSnapBar` adoption site, plus OutputRoutingTile primitive (six rows — Primary / Secondary / Syphon-or-Spout / cross-platform sibling / NDI v2 / Record v2; per-platform variant via the W22 PR2 `stores/platform.ts` detection; `data-cap` capability hints for Phase-3+/4a entitlement discovery) adopted in Settings → Graphics/Output (PR #168-#169). | S4, S7 | bundle `extras.jsx::SnapBar`, `v02_supplement.jsx::OutputRoutingTile`; `49-ux-bundle-supplement-scope.md` |
| W28 | DegradedBanner primitive port (warning / error severities with bundle-faithful ARIA — `role="alert"` + `aria-live="assertive"` for error, `role="status"` + `aria-live="polite"` for warning; inline-SVG triangle / circle; `color-mix` tinted `--color-warning` / `--color-danger` surfaces) mounted in AppShell between TopBar and SnapBar with Diagnostics action routing to Settings and dismiss flipping a local `bannerVisible` `$state`, plus Author preview-tools strip rebuild to the bundle's six-affordance shape (HUD eye / fullscreen / pause-play / 60fps / VuStereo embed / camera screenshot) with 200ms `prefers-reduced-motion`-guarded flash overlay on capture (PR #170-#171). | S5 closure on author preview, S9 | bundle `v02_supplement.jsx::DegradedBanner`, `extras.jsx::PreviewTools`; `49-ux-bundle-supplement-scope.md` |
| W29 | CommandPalette overlay port (520px modal, caps-stamped header, mono autofocused search with case-insensitive label+meta filter, scrollable result list, "no matching command" empty state, footer hint showing toggle + close keys) mounted in AppShell at z-index 1010 above the W8 PR2 HotkeyOverlay; window-level Cmd+K (macOS) / Ctrl+K (Windows / Linux) keydown handler with `event.preventDefault()`, Esc / outside-click / close-button dismiss, focus-trap idiom matching HotkeyOverlay; seven bundle actions verbatim with `launch-scene` and `open-hotkeys` wired to `routeStore`, the remaining five (snap-recall, snap-save, osc-address, support-bundle, offline-docs) deferred to `EV-INTEGRATION-MATRIX` (PR #172). | S2, S7 | bundle `extras.jsx::CommandPalette`; `42-operator-hotkey-map.md` |

Phase-1 EXIT was at W12; W13-W29 are post-Phase-1 sprint waves landing the v0.2-beta bundle as wireframe-faithful Svelte 5 ports under the design surface frozen by `45-design-token-mapping.md`. W24-W25 added a bundle-faithfulness audit thread closing all `requestAnimationFrame` call-sites (`audio.jsx`, `midi.jsx`, `perform.jsx`) where W17/W18 sub-agents had chosen static-frame interpretations that diverged from the bundle's synthetic-animation contract. W26-W29 advanced the chrome surface — W26 finished the Perform 3-column layout deferred since the W25 AudioReactivity port; W27 added SnapBar + OutputRoutingTile primitives; W28 added DegradedBanner + the Author preview-tools rebuild; W29 added the CommandPalette overlay (Cmd+K). Subsequent wave priorities are tracked outside this document via wave-N-visual READMEs under `.playwright-mcp/`.

## 12. Risks + mitigations

| # | Risk | Mitigation | Tracked |
|---|---|---|---|
| PR-01 | `ts-rs` drift between Rust and TypeScript ships subtly broken bindings | CI gate on export-is-fresh; `npm run check` + `cargo test` fail on stale bindings | `EV-COMPAT-PROFILE-DIFF` |
| PR-02 | CodeMirror 6 plugin ecosystem is less mature than CM5; plugin health risk | Pin exact versions; vendor any off-the-shelf plugin with license-clear code into the repo if upstream goes dormant | `EV-LICENSE-AUDIT`, `22-compatibility-versioning-policy.md` |
| PR-03 | Scene-surface composition spike fails (e.g. WebView2 can't honour a transparent region correctly on Windows) | Fallback plan: web-rendered canvas via `<canvas>` + WebGL / WebGPU for non-fullscreen preview; native wgpu only in fullscreen. ADR-018 §Consequences already lists "careful fullscreen-transition path" as a spike. | `EV-RENDER-CONFORMANCE`, `R-??` (add to `13-risk-register.md` if spike fails) |
| PR-04 | Svelte 5 runes model is new; idiom drift across team | Svelte 5 runes coding-standard subsection added to `43-development-standards.md` §13 (new); single code-owner for review until idioms settle | `43-development-standards.md` §13 |
| PR-05 | NFR-10 latency misses on WebView2 (Windows) even if it passes on WKWebView (macOS) | Measure on both during M5; if Windows misses, defer hover-doc / completion to an async render tier; fall back to pre-computed tries | `EV-AUTHORING-UX-LATENCY`, NFR-10 |
| PR-06 | Tauri 2 signing / notarization surface changes upstream between kickoff and GA | Pin Tauri 2.0.x minor; bumps run full regression suite | ADR-014, `22-compatibility-versioning-policy.md` |

## 13. Not in scope of this plan

- Final visual design language (colour, type, iconography) - out of scope per `01-charter.md` + `23-decision-ownership-matrix.md` "Final visual design language remains out of scope".
- Scene shader language beyond syntax highlighting + symbol completion - that is `36-reactive-symbol-catalog.md` + `37-authoring-environment-spec.md`.
- Audio analysis pipeline - `06-audio-engine.md`.
- Render backend - `adr/ADR-011-rendering-backend.md`.
- Script sandbox - `adr/ADR-012-script-sandbox-engine.md`.

## 14. Traceability

- `adr/ADR-018-non-scene-ui-toolkit.md` (the toolkit decision)
- `adr/ADR-011-rendering-backend.md` (wgpu + colour management)
- `adr/ADR-014-signing-notarization-auto-update.md` (Tauri build + signing)
- `22-compatibility-versioning-policy.md` (version pinning)
- `26-evidence-artifact-registry.md` (`EV-UX-*-CONTRACT`, `EV-ACCESSIBILITY-AA`, `EV-COLOR-MANAGEMENT-SRGB`, `EV-SYMBOL-CATALOG`, `EV-RENDER-CONFORMANCE`, `EV-AUTHORING-UX-LATENCY`, `EV-LICENSE-AUDIT`, `EV-COMPAT-PROFILE-DIFF`)
- `37-authoring-environment-spec.md` §3 (editor, live-reload, diagnostics, scaffolder)
- `42-operator-hotkey-map.md` (full hotkey surface; focus-context priority)
- `43-development-standards.md` §13 (Svelte 5 rune coding standard; new addition under this plan)
- `44-ux-bundle-gap-audit.md` (drives specific component fixes)
- `45-design-token-mapping.md` (the tokens surfaced via `lib/tokens/`)
- `46-ux-bundle-a11y-color-audit.md` (ARIA / keyboard / a11y per-component work)
- `48-ux-bundle-open-questions-routing.md` (Q1-Q6 closures that this plan relies on)
