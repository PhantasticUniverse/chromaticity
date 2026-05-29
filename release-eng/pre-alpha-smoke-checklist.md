<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Chromaticity Pre-Alpha Smoke Checklist

Operator checklist for testing unsigned local builds of the Chromaticity
desktop app on macOS and Windows. The pipeline-side equivalent (signed +
notarized + auto-update) is gated on production cert procurement per
[`docs/adr/ADR-014-signing-notarization-auto-update.md`][adr-014] and
[`README.md`][release-eng-readme] in this directory; this document
covers the **pre-cert pre-alpha** path.

## Audience + scope

- **Pre-alpha testers**: the AO and a small trusted cohort.
- **Platforms**: macOS today (Apple Silicon or Intel); Windows 10/11 when
  the AO's Windows machine is provisioned this week.
- **Builds**: unsigned local artifacts produced via `npm run tauri build`.
  Gatekeeper + SmartScreen will both warn — see "Launch" below.
- **Not in scope here**: signed/notarized distribution (cert-gated),
  staging appcast (`stage-appcast.sh`), Sparkle 2 auto-update.

## Repository state baseline

- Branch: `main` at the time of build.
- Recent landings (2026-05-12 through 2026-05-13): the Wave 2.6 Audio
  page-by-page wiring (5 PRs) closed at HEAD `ea9331a` after the
  narrative refresh + README catch-up. Wave 2.6: `#463` (AUD.E
  snapshot capture), `#464` (AUD.D 8→5 FFT reduction band tiles),
  `#465` (AUD.C live peak/rms dB readout), `#467` (AUD.B live
  monitoring meter), `#469` (AUD.A live audio device enumeration).
  Wave 2.5 Library: `#457` (card thumbnails fade-in + 50-card budget),
  `#458` (source/recent/sort filters), `#459` (tag badges + favorites),
  `#460` (empty state polish + handoff), `#461` (sidebar preview +
  metadata + Author handoff). Wave 2.4 Perform: `#451` (snap labels),
  `#452` (media filter), `#453` (action buttons), `#454` (adapter
  pills), `#455` (scene list load), `#456` (preset save engine
  widening). Wave 2.3: `#449` (Q.1 renderer signatures + composition),
  `#450` (Q.2 Perform layout fidelity). Wave 2.2: `#447` (R.2 shader
  sourcing research catalog), `#448` (R.1 SceneCanvas + SceneStill
  wired into Perform / Library / Playlist / Author preview panes).
  DESIGN.md alignment: `#462` (rail.width 48→76, status-bar.height
  22→28, snap slot count 16→8, snap labels anatomy note). Narrative
  refresh: `#468` (CLAUDE.md Wave 2.2-2.5 catch-up). README
  catch-up: `#466`.
- Prior landings (2026-05-11): the 11-PR Wave 2.1 cross-seam activation
  sequence closed at HEAD `c40e63e`. Lane A: `#436` (scene load), `#441`
  (control dispatch), `#442` (snap), `#445` (preset+playlist). Lane B:
  `#435` (Perform variant + STANDARD/CONSOLE toggle), `#438` (top bar HUD).
  Lane C: `#437` (audio bootstrap doc cleanup), `#440` (Settings TOML
  persistence). Lane D: `#439` (MIDI device list), `#443` (MIDI learn),
  `#444` (OSC routes + IP/port persistence).
- Prior landings (2026-05-10): Wave 1d-α PR 3 (`#426` help corpus closure),
  Wave 1d-β PR 2 (`#425` scene library closure), Wave 1e-γ release pipeline
  scaffold (`#423`), plus three release.yml bug-fix PRs (`#428` / `#429` /
  `#430`) covering mkdocs-after-vite, asset-name collision, and
  upload-artifact ancestor-stripping respectively.
- Last release.yml smoke run: `25628685936` (success, synthetic-signed).
- Phase 4a / 4b passable; Phase 4d at 5/6 (calendar gate ~2026-05-20).

## Prerequisites

### macOS

| Tool                     | Minimum                | Install                                                                   |
| ------------------------ | ---------------------- | ------------------------------------------------------------------------- |
| Xcode Command Line Tools | latest                 | `xcode-select --install`                                                  |
| Node                     | 20.11+                 | `brew install node` (or `nvm install 20`)                                 |
| Rust toolchain           | 1.85.0 (workspace pin) | `curl https://sh.rustup.rs -sSf \| sh`; honored via `rust-toolchain.toml` |
| Python                   | 3.10+                  | system Python or `brew install python@3.12`                               |
| mkdocs + mkdocs-material | 1.6+                   | `pip install --user mkdocs mkdocs-material`                               |

### Windows (Fletcher, later this week)

| Tool                      | Minimum                | Install                                       |
| ------------------------- | ---------------------- | --------------------------------------------- |
| Visual Studio Build Tools | 2022, C++ workload     | https://visualstudio.microsoft.com/downloads/ |
| Node                      | 20.11+                 | https://nodejs.org/ (LTS)                     |
| Rust toolchain            | 1.85.0 (workspace pin) | `rustup-init.exe` from https://rustup.rs/     |
| Python                    | 3.10+                  | python.org installer, check "Add to PATH"     |
| mkdocs + mkdocs-material  | 1.6+                   | `pip install --user mkdocs mkdocs-material`   |

### Verify

```bash
node --version    # >= v20.11
rustc --version   # 1.85.0
mkdocs --version  # 1.6+
```

## Build

From the repo root:

```bash
npm ci                # one-time, after clone
npm run tauri build   # full release build; ~5-10 min first run, faster on rebuild
```

What this does, in order:

1. `npm run tsc` — typecheck app + node configs
2. `npm run check` — svelte-check
3. `npm run build` — vite build → `dist/` (wipes `dist/help/` and `dist/.gitkeep`)
4. `npm run build:help` — mkdocs rebuild → `dist/help/` (repopulates 11 help pages)
5. `cargo build --release -p chromaticity-app` — release binary with `dist/` baked in via Tauri custom-protocol (per [`feedback_tauri_custom_protocol_dist_baked`][memory-protocol-baked])

Build artifact: `target/release/chromaticity-app` (Mac) or
`target\release\chromaticity-app.exe` (Windows). Bundle is intentionally
off (`bundle.active = false` in `tauri.conf.json`) — pre-alpha uses the
raw binary, signed bundles wait for cert procurement.

### Build hygiene

The build chain leaves three working-tree artifacts that need cleanup
before any `git add` if you're working in a repo clone (testers running
the binary outside a repo can ignore this):

| Path                       | What changed                                                                 |
| -------------------------- | ---------------------------------------------------------------------------- |
| `dist/.gitkeep`            | wiped by `vite build` (the load-bearing sentinel — see CLAUDE.md)            |
| `dist/help/.gitkeep`       | wiped by `mkdocs build` (clean rebuild)                                      |
| `dist/help/sitemap.xml.gz` | gzip header timestamp differs each build (deterministic content, noisy diff) |

Restore in one shot before staging:

```bash
touch dist/.gitkeep dist/help/.gitkeep
git checkout -- dist/help/sitemap.xml.gz
git status   # should show only your intentional changes
```

Run this BEFORE `git add` — committing the deletions breaks `cargo
check` on fresh clones (see [`feedback_vite_build_clobbers_dist_gitkeep`][memory-gitkeep]).

## Launch

### macOS

```bash
./target/release/chromaticity-app
```

The binary is unsigned. Gatekeeper does not block raw binaries (only
`.app` bundles + `.dmg` images), so it launches without a warning
dialog. The window opens at 1280×800.

### Windows

Double-click `target\release\chromaticity-app.exe` from Explorer, or
run from a PowerShell / cmd prompt. SmartScreen may show "Windows
protected your PC" — click **More info → Run anyway**.

## Smoke walkthrough

Aim for ~15–20 minutes end-to-end. File anything that surprises you per
"Bug reports" below.

### A. App shell + navigation (≈ 2 min)

- [ ] Window renders at 1280×800; resizable; theme defaults to Pulse.
- [ ] Sidebar shows the 13-icon rail (Perform / Library / Author /
      Playlist / Media / MIDI / OSC / Audio / Patch / Onboard / Settings
      / Help / etc.). Hover reveals tooltips.
- [ ] Status bar at the bottom shows the active tier, adapter state
      pill, and theme name.
- [ ] Click each sidebar icon — every view renders without error or
      blank panel. Settings, Library, Help, Author, Perform are the most
      load-bearing.

### B. Theme + chrome (≈ 2 min)

- [ ] Cycle the three themes (Pulse → Atelier → Console). Sidebar +
      status bar + buttons all re-style; no flash of unstyled content.
- [ ] Press `?` — hotkey overlay appears with ~57 rows grouped by
      category. Press `?` or `Esc` to dismiss.
- [ ] Open Settings → confirm the theme switcher matches the live state.

### C. Library + scene loading (≈ 3 min)

- [ ] Library view shows scenes grouped by the bundle's four-subdir
      taxonomy: `welcome/`, `examples/`, `library/`, `shadertoy-fixtures/`.
- [ ] **Double-click any scene** (e.g. `welcome/controls-tour.chroma`) —
      the `load_scene` Tauri command invokes; switch to Perform view; the
      Scene Controls header reflects the loaded scene's NAME (not the
      fixture's `perform-fixtures` literal); the Scene Controls + Meta
      Controls panels re-render against the loaded scene's control schema.
      This is the Wave 2.1 A.1 acceptance gate.
- [ ] Open `welcome/quick-start.chroma` — the preview pane stays a
      gradient placeholder (live render pipeline deferred per ADR-011);
      Scene Controls header binds.
- [ ] Open `library/demo-kaleido.chroma` — control schema populates the
      Scene Controls panel; uniforms panel populated.
- [ ] Open `library/demo-spectrum.chroma` — control schema populates;
      spectrum bars in the inline AudioUniforms widget render synthetic
      (see "Known synthetic surfaces").

### D. Author view (≈ 3 min)

- [ ] Open any `examples/*.chroma` scene in Author view.
- [ ] CodeMirror editor loads GLSL/JSON sources with syntax highlighting.
- [ ] Inputs Inspector shows the full control widget palette (Fader /
      Knob / Toggle / Swatch / ControlGroup).
- [ ] Diagnostics panel renders for the active scene. JSON-class
      diagnostics show spans; compile/script/coercion classes show
      message-only (per the `EV-AUTHORING-DIAGNOSTICS` narrowing).
- [ ] Edit a control value via a widget — the preview updates.

### E. Audio reactivity — LIVE IPC (≈ 3 min)

> The audio IPC seam is live in production per Wave 1c-α-2. The 4 named
> band tiles + BPM strip drive from real audio. The 48-bar FFT and the
> AudioUniforms Spectrum widget remain synthetic — see "Known synthetic
> surfaces".

**Important — what "real audio" means on macOS.** The producer uses
cpal's default INPUT device (i.e. your microphone), NOT system audio
output. Capturing system output natively IS possible on macOS 14.4+
(Core Audio process taps) and Windows (WASAPI loopback), but
Chromaticity does not yet implement that path (the engine is on cpal
0.15, which predates cpal's 0.17 loopback support). Until it lands, to
get a meaningful reaction:

- **Easy path**: play music on your computer speakers and let the
  built-in mic pick it up acoustically. Speak / clap near the mic.
- **Cleaner path**: install a virtual audio loopback that routes
  system output to a virtual input device:
  - [BlackHole](https://existential.audio/blackhole/) — free,
    MIT-licensed, ships a 2-channel or 16-channel virtual driver.
  - [Loopback](https://rogueamoeba.com/loopback/) — paid, more
    flexible GUI-driven routing.
    After installing, restart Chromaticity, open Settings → Audio,
    pick the BlackHole / Loopback device in the Input Device dropdown.
    Wave 2.8 Lane AUD.SRC: `AudioRuntime` now honours the persisted
    selection at startup (previously the dropdown was cosmetic).
    Most performers prefer this path over the acoustic mic-pickup
    route above.
- **No-mic path**: if your machine has no input device at all, the
  producer falls back to a `SilentSource` and emits all-zero
  snapshots. Visualizers stay on synthetic fallback — that's
  expected, not a bug.

- [ ] Open the Audio view.
- [ ] Confirm the BpmRing widget animates. Without audio at the mic, it
      shows synthetic motion. Play music near the mic — the ring should
      react to real beats within a couple seconds.
- [ ] Confirm BeatDots animate.
- [ ] Open `examples/four-band-reactive.chroma` from the Library, then
      switch back to the scene preview. The 4 named bands should drive
      visuals when audio is reaching the mic.
- [ ] Open the AudioUniforms view (or Perform view's audio panel) and
      confirm the synthetic 64-bar spectrum renders — this is expected
      synthetic behavior, not a bug.

### F. Hotkeys (≈ 2 min)

Test a sample from the 57-row canonical vocabulary. Full list in
**Help → Hotkeys Reference** or via the `?` overlay.

- [ ] `?` — toggle hotkey overlay
- [ ] `Esc` — close any open modal / overlay
- [ ] `Space` — primary play/pause action (where bound)
- [ ] `B` — beatsync tap (Audio view)
- [ ] `L` — toggle Library
- [ ] `,` — open Settings
- [ ] Number keys `1`–`9` — snap recall (where bound by scene)

### G. Perform: control dispatch + snaps + presets + playlist (≈ 4 min)

This section consolidates the Wave 2.1 Lane A acceptance gates that all run
in the Perform view. Make sure a scene with at least one Fader / Knob is
loaded (e.g. `welcome/controls-tour.chroma` or any `examples/*.chroma`).

**G.1 Control dispatch (A.2)**

- [ ] Drag a Fader in Scene Controls — `set_control` IPC fires; widget
      visibly reflects the engine-committed value (not just the local
      drag position).
- [ ] Rotate a Knob — same round-trip.
- [ ] Toggle a Toggle — same round-trip.

**G.2 Snap save + recall (A.3)**

- [ ] In the SnapBar, **Cmd-click slot 1** — current control state saves
      to slot 1 (active-slot pill highlights).
- [ ] Drag a fader to a new position.
- [ ] **Click slot 1** (plain click) — controls restore to the saved
      snapshot; the dragged fader snaps back.
- [ ] Cmd-click slot 2 to save the new (dragged) state; click slot 1 +
      slot 2 to ping-pong; **Alt-click slot 1** to clear that slot.

**G.3 Preset + playlist (A.4)**

- [ ] Click a preset in the Perform right scene column's preset rail —
      `recall_preset` fires; active preset highlights.
- [ ] Click **Next** in the right scene column — `advance_playlist(+1)`
      fires; playlist position advances.
- [ ] Click **Previous** — playlist position decrements.
- [ ] Click **Shuffle** — advances by +1 (engine has no shuffle
      command; see "Known synthetic surfaces").

**G.4 Playlist view + transitions (existing)**

- [ ] Playlist view loads with a cue list.
- [ ] CueTransitionCurve widget renders a cubic bezier preview.
- [ ] Drag-to-reorder a cue (display-level; preset-bank durability is
      Phase-4+ deferred).
- [ ] Trigger a manual cue transition — verify the curve drives a
      visible transition (the Perform preview pane stays placeholder;
      see "Known synthetic surfaces").

### H. OSC + MIDI views (≈ 3 min)

**H.1 MIDI device listing + learn (D.1 + D.2)**

- [ ] MIDI view's Devices tab renders the live device list via
      midir 0.10's cross-platform enumeration (CoreMIDI / WinMM /
      ALSA). If a MIDI controller is plugged in, its name +
      manufacturer + port info appears.
- [ ] No MIDI controller? The list renders empty — that's expected,
      not a bug.
- [ ] Switch to the Learn tab (or click a Learn affordance from a
      control's context menu, where bound). Click **Start Learn** and
      strike a MIDI key / pad / CC. The captured message decodes
      (NoteOn / Off / CC / PitchBend / ProgramChange) and persists to
      localStorage.
- [ ] Note: the captured event is NOT yet wired to a specific control
      id (Wave 2.2 scope) and multi-message capture is single-message
      MVP only — both flagged in "Known synthetic surfaces".

**H.2 OSC route inspection + IP/port persistence (D.3)**

- [ ] OSC view renders the live routing table via
      `osc_routes_snapshot`. The 4 canonical route prefixes
      (`/chroma/control/...`, `/chroma/playlist/...`,
      `/chroma/preset/...`, `/chroma/snap/...`) appear.
- [ ] Edit the OSC inbound IP and port in OSC view's Settings panel
      (or in Settings → OSC). Click save. Quit + relaunch — the IP +
      port persist (TOML-backed via the Settings store).
- [ ] Real OSC wire-level testing requires a peer (e.g. TouchOSC) and
      is optional for pre-alpha.

### I. Help corpus (≈ 1 min)

- [ ] Open Help view.
- [ ] Sidebar lists all 11 pages: Welcome, About, Hotkeys Reference,
      OSC Routes, Chroma Uniforms, Audio Reactivity, Scene Workflow,
      Library Workflow, Perform Mode, Settings Reference, Import
      Shadertoy.
- [ ] Each page loads in the content iframe without a 404 or broken-
      content placeholder.
- [ ] Internal links within the corpus navigate (mkdocs-material's
      built-in nav).

### J. Restart + persistence (≈ 2 min)

Wave 2.1 C.3 added a real TOML-backed Settings store under Tauri's
`app_data_dir`. Theme + last-opened view continue to persist via
localStorage; the new TOML store carries everything else.

- [ ] In Settings, change the FPS target.
- [ ] Change the active theme (Pulse / Atelier / Console).
- [ ] In OSC, edit inbound IP and port.
- [ ] Quit the app (Cmd-Q on Mac / Alt-F4 on Windows).
- [ ] Relaunch.
- [ ] Verify: theme + last-opened view + FPS target + OSC IP + OSC port
      all restore to the values you set.
- [ ] Audio device + folder picker outputs (if exercised) persist
      through the same TOML.

## Known synthetic surfaces — do NOT file bugs

These ship under explicit Phase-4 narrowings (per the EXIT records in
[`docs/EV-GATE-MODEL-AUDIT-checklist.md`][audit-checklist]) and the
Wave 2.1 follow-on records in [`docs/53-v1-release-parity-gaps.md`][docs-53]
§4 G6. Their UI scaffolding is in place but the underlying I/O is
synthetic or partially wired. Treat the visual presence as the contract;
don't expect real-world data.

| Surface                                                                 | Status                                                                                                                                                                                                       |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Perform preview pane** (live render)                                  | ADR-011 Phase-2+ deferred. `set_control` IPC fires end-to-end and the engine commits the value, but the preview pane stays a gradient placeholder. This is expected — do NOT file as "scene doesn't render". |
| **Camera input** (Media view, Perform)                                  | Wave 1c-β closed synthetic; per-platform `nokhwa` real bindings deferred.                                                                                                                                    |
| **Syphon publish/subscribe** (macOS)                                    | UI scaffolding present; real `publish_syphon_output` + observer deferred.                                                                                                                                    |
| **AudioReactivity 48-bar FFT**                                          | Synthetic-driven post-MVP; only the 4 named band tiles + BPM strip are live.                                                                                                                                 |
| **AudioUniforms Spectrum widget** (64-bar)                              | Synthetic post-MVP (the bundle's inline Spectrum, not a separate widget).                                                                                                                                    |
| **Dual-GPU selection**                                                  | Synthetic 10-fixture validation only; physical dual-GPU validation deferred.                                                                                                                                 |
| **Crash injection drill**                                               | Synthetic harness only; do NOT deliberately SIGKILL expecting recovery.                                                                                                                                      |
| **GPU thumbnail rendering** (Library)                                   | Deterministic byte-hash stub; real GPU thumbnails deferred post-MVP.                                                                                                                                         |
| **MIDI binding-to-control assignment** (Wave 2.1 D.2)                   | Learn mode captures the message + persists to localStorage; binding it to a specific control id carries forward as a future-wave follow-up.                                                                  |
| **MIDI multi-message learn** (Wave 2.1 D.2)                             | Single-message MVP only; debounced multi-message capture (composite bindings) carries forward as a future-wave follow-up.                                                                                    |
| **Top bar Camera + Record buttons** (Wave 2.1 B.2)                      | Visual stubs only; real wiring rides the Wave 1c-β camera follow-ups (per-platform `nokhwa` + Syphon `publish_syphon_output`).                                                                               |
| **Perform preset save disk persistence** (Wave 2.4 PERF.C)              | Engine widened with `Command::PresetSave` for in-process snapshot capture; Cmd-click on a preset slot saves, click recalls. Settings-TOML disk persistence deferred.                                         |
| **SnapBar slot labels disk persistence** (Wave 2.4 PERF.F)              | Right-click-rename persists custom labels through localStorage; engine snap state stays binary saved/unsaved.                                                                                                |
| **Library favorites / recent / source filter** (Wave 2.5 LIB.B / LIB.C) | Frontend-only stores backed by localStorage; engine-side support intentionally deferred per the clean-room scene-write narrowing.                                                                            |
| **Perform Shuffle button** (Wave 2.1 A.4)                               | Engine has no shuffle command; the "Shuffle" button advances by +1 like Next (label parity with the bundle, no random ordering).                                                                             |
| **Top bar FPS readout**                                                 | Placeholder pending ADR-011 rendering backend.                                                                                                                                                               |

## Wave 2.1 acceptance criteria

The Wave 2.1 cross-seam activation sequence (closed 2026-05-11 at HEAD
`c40e63e` via 11 PRs) targeted 8 end-to-end user stories. Each maps to a
checklist step above; this section restates them as a single
verification matrix so testers can confirm the wave's gate at a glance.

| #   | User story                      | Steps to verify                                                                                      | Wave 2.1 lane / PR          | Sections               |
| --- | ------------------------------- | ---------------------------------------------------------------------------------------------------- | --------------------------- | ---------------------- |
| 1   | **Scene load**                  | Double-click a Library scene → Perform header binds to the scene NAME + control schema re-renders.   | A.1 / `#436` / `8171306`    | §C                     |
| 2   | **Control dispatch**            | Drag a Fader / Knob / Toggle → `set_control` IPC fires + widget reflects the engine-committed value. | A.2 / `#441` / `cc12452`    | §G.1                   |
| 3   | **Snap save + recall**          | Cmd-click slot → save; drag a fader; click slot → state restores; Alt-click → clear.                 | A.3 / `#442` / `1d2d93b`    | §G.2                   |
| 4   | **Audio reactivity (live IPC)** | Play music near the mic (or BlackHole loopback) → 4 named bands + BPM strip animate from real audio. | (already live, Wave 1c-α-2) | §E                     |
| 5   | **MIDI device listing**         | Connect a MIDI controller → MIDI view's Devices tab shows real device + manufacturer + port info.    | D.1 / `#439` / `ed0df45`    | §H.1                   |
| 6   | **OSC IP/port persistence**     | Edit OSC inbound IP + port → quit + relaunch → values persist.                                       | D.3 / `#444` / `c40e63e`    | §H.2, §J               |
| 7   | **Settings persistence**        | Change theme + FPS target → quit + relaunch → all restore.                                           | C.3 / `#440` / `56ea703`    | §J                     |
| 8   | **Perform variant toggle**      | Top bar STANDARD/CONSOLE toggle switches between classic 2-column (default) and dense 3-column.      | B.1 / `#435` / `95d25cb`    | §A (per-view exercise) |

Bonus surfaces shipped in Wave 2.1 (not on the original 8-criterion list
but exercised in the walkthrough): top bar transport HUD (B.2 / `#438` /
`838f1ed`); preset rail + prev/next/shuffle (A.4 / `#445` / `06d52a8`);
MIDI learn-mode capture (D.2 / `#443` / `11839c7`); audio bootstrap doc
re-verification + mic note (C.1 / `#437` / `41a389c`).

If any of the 8 fail under the verification steps, file via the
bug-report template below. Items in the "Known synthetic surfaces"
table are NOT failure modes — please check that table first.

## Wave 2.4 + 2.5 acceptance criteria addendum

The Wave 2.4 Perform-page wiring (6 PRs, `#451` / `#452` / `#453` /
`#454` / `#455` / `#456`) and Wave 2.5 Library-page wiring (5 PRs,
`#457` / `#458` / `#459` / `#460` / `#461`) plus the DESIGN.md token
alignment (`#462`) layer twelve new acceptance gates on top of the
Wave 2.1 set. Verify in place during the §C / §D / §G smoke
walkthrough.

### Perform (Wave 2.4)

- [ ] **PERF.A action buttons.** Default / Random / Lock-all buttons in
      Scene Controls + Meta Controls headers reset / randomize / lock
      controls. Per-row Lock icon dims that row + blocks `set_control`
      dispatch (engine never sees suppressed values).
- [ ] **PERF.B scene list load.** Single-click any row in the Perform
      right-column scene list — `load_scene` fires; the active-row
      marker reflects `loadedSceneStore.snapshot?.id` (4 of 12
      `SETLIST_FIXTURE` rows resolve to corpus paths; the remaining 8
      stay display-only with no click handler attached — that's
      expected, not a bug).
- [ ] **PERF.C preset save.** Cmd-click (macOS) / Ctrl-click (Windows
      / Linux) an empty preset slot — current control state saves to
      that slot (in-process snapshot, NOT disk-persistent); plain click
      recalls.
- [ ] **PERF.D adapter pills.** The TopBar OUT / MIDI / OSC pills
      reflect live state: OUT green when a scene is loaded; MIDI green
      when at least one device enumerated; OSC polled every 5 s via
      `osc_health_snapshot` (Healthy → ok / Recovering → warming /
      Degraded → disconnected / no-snapshot → off).
- [ ] **PERF.E media filter.** The media-tab pills filter the media
      grid (`all` / `live` / `media` per `MediaCardKind`). Three dead
      controls dropped (Setlist dropdown / Play Setlist / media
      Refresh).
- [ ] **PERF.F snap labels.** Right-click any SnapBar slot — inline
      rename input opens; Enter commits, Escape cancels, blur commits.
      Quit + relaunch — custom labels persist via localStorage (engine
      snap state stays binary saved/unsaved).

### Library (Wave 2.5)

- [ ] **LIB.A sidebar preview + Author handoff.** Single-click a
      Library card — the right-pane sidebar renders a `<SceneCanvas>`
      preview keyed off scene metadata, plus a metadata column (title /
      source pill / credit / description / category + tag chips /
      controls-count badge). Action buttons round-trip through
      `load_scene`; Open-in-Author hands off to Author view.
- [ ] **LIB.B tag badges + favorites.** Tag chips render on each card
      (top-left, max 3 visible + `+N more` overflow). Heart icon
      (top-right) toggles favorite membership (localStorage). FAVORITES
      filter chip narrows the grid to favorited cards only.
- [ ] **LIB.C source filter + recent + sort.** Source `<select>` in
      the panel head (`all` / `built-in` / `my-scenes`) filters by
      bundled vs user. New `RECENT` chip prepended to the category-chip
      row. Sort `<select>` in the toolbar (`id` / `title` / `recent`
      / `favorites`).
- [ ] **LIB.D card thumbnails.** Scene cards render `<SceneStill>`
      thumbnails (canvas-2d procedural, NOT real GPU output) with a
      240 ms ease-out fade-in animation on first paint
      (`prefers-reduced-motion: reduce` suppresses).
- [ ] **LIB.E empty state + handoff polish.** Library empty state
      shows the supported-formats footnote (".chroma · .chromaPack ·
      .synScene conversion · Shadertoy URL · ISF reject"). When all
      cards filter out via search + chip filters, the
      `EMPTY_LIBRARY_FILTERED` branch renders instead of an empty
      grid container. Double-click a card — the actively-loading tile
      flips to `aria-busy="true"` + dimmed `.scene-tile--loading`
      affordance (sibling tiles unaffected); on success, route flips
      to Perform.

### DESIGN.md tokens (PR `#462` + 2026-05-18 refresh)

- [ ] **Rail width.** Sidebar rail renders at 52 px (`rail.width` in
      `DESIGN.md` frontmatter; `AppShell.svelte` grid template).
      Status bar renders at 28 px (`status-bar.height`).
- [ ] **Snap slot count.** SnapBar trims to 8 slots
      (`SNAP_SLOT_COUNT = 8` in `snap.svelte.ts`) even though the
      engine carries 16.

## Pre-alpha fix-pass round 1 → round 3 acceptance criteria addendum

Three parallel rounds (2026-05-17 through 2026-05-18) closed every
mega-review and clean-room comparison finding scoped to pre-alpha.
The acceptance steps below are NEW operator-visible behaviors layered
on top of Wave 2.1 / 2.4 / 2.5 / 2.6 / 2.7 / 2.8. Verify in place
during the walkthrough; failures should be filed via the bug-report
template.

### Audio engine (rounds 1, 2, 3)

- [ ] **Audio warming arm visible** (Round 2 Lane A, PR `#484` Round
      2 amendment). With audio reaching the mic (or BlackHole /
      Loopback loopback), the first ~1.2 s after launch shows
      BpmRing + BeatDots + AudioReactivity band tiles in a
      dimmed / `--` placeholder state. Once the FFT / BPM analysis
      window fills, all four band tiles + the BPM strip transition
      to live values. This is the new `AudioAxisAvailability::Warming
→ Ready` state — what looks like "audio not working for the
      first second" is the documented warmup behavior, NOT a bug.
- [ ] **AUDIO adapter pill in TopBar** (Round 2 Lane A). Top bar
      now carries a fourth pill (`AUDIO` alongside `OUT / MIDI /
OSC`). Healthy = green; degraded = red (set on poison-recovery
      from the producer thread); warming = yellow during warmup.
      Try unplugging / replugging the audio input device — the pill
      reflects state transitions within ~1 scheduler tick (~17 ms).
- [ ] **Native uniform symbol parity** (Round 3 Lane Q, PR `#488`).
      Scene authors referencing the four legacy invented `syn_RMS`
      / `syn_PEAK` / `syn_FFT_LO` / `syn_FFT_HI` names will fail to
      bind; the canonical native spellings are now `chroma_AudioRMS`
      / `chroma_AudioPeak` / `chroma_AudioFFT[8]` per
      `docs/36-reactive-symbol-catalog.md` v1.3.0 (the catalog
      reflects the rename; any in-flight scene that referenced the
      old names must be updated).
- [ ] **RMS / peak smoothness** (Round 3 Lane Q). Watch the
      AudioReactivity band tile percentage labels for ~10 seconds
      with audio playing. The values should glide smoothly without
      sub-second jitter — RMS / peak are now computed over the
      same analysis window as the FFT, eliminating the source of
      jitter that earlier UI EMA + label-hysteresis patches
      band-aided.

### Settings + crash-resume (round 1, round 2 Lane B)

- [ ] **Settings atomic write + quarantine** (Round 1 + Round 2
      Lane B, PR `#486`). In Settings, change a value (FPS target,
      OSC port, theme). The settings.toml under Tauri's
      `app_data_dir` is written atomically (temp + fsync + rename);
      a hand-corruption of the file (drop a brace, save) before
      relaunch should land at `settings.toml.malformed-<timestamp>`
      and the app comes up on defaults rather than failing to
      launch.
- [ ] **Crash-resume save / recover wired into launch** (Round 2
      Lane B). Load a scene, drag controls, save a snap, advance the
      playlist. From a terminal: `kill -9 <chromaticity-app pid>`.
      Relaunch. The `CrashResumeBanner.svelte` surfaces a "session
      recovered" affordance and active scene + Snap slot + playlist
      position + (Round-1 class-d) OSC / MIDI device bindings
      restore within ~1 s of the save-cadence window. The discard
      gate is intentionally not auto-fired on clean exits;
      restoration is idempotent.

### Renderer control coverage (round 2 Lane C, PR `#487` round 2)

- [ ] **rotation_speed + warp_amount across all responsive
      renderers.** Load Demo Kaleido or any `examples/*.chroma`
      scene whose schema declares `rotation_speed` / `warp_amount`.
      Drag the controls; preview canvas should respond visibly on
      kaleido / lattice / bars / **rings / orbit / tunnel / fluid /
      plasma** renderers. (Round-1 baseline covered the first
      three; Round-2 widened to the last five.)
- [ ] **Universal `invert` / `mirror` post-pass.** Toggle any
      scene's `invert` control (or use a fixture that declares it);
      preview applies a color-channel invert post-pass. Toggle
      `mirror`; preview flips horizontally. These apply to all 16
      canvas-2d renderers uniformly.
- [ ] **`data-preview-wired="false"` affordance.** Load a scene
      with a control name NOT in the preview adapter's known set
      (e.g. `gamma` / `focus` / any custom name). The control row
      in Perform renders with `opacity: 0.7` and a hover tooltip
      ("engine-only — preview adapter not implemented"). This
      closes the silent-no-op feel from earlier rounds —
      controls that fire `set_control` to the engine but have no
      preview visualization are now operator-visibly dimmed.

### OSC dispatch (round 3 Lane R, PR `#489`)

- [ ] **OSC default IN / OUT ports on fresh install.** Open
      Settings → OSC (or the OSC view). On a fresh install (no
      pre-existing `settings.toml`) the IN port is `9000` and the
      OUT port is `9001`; both are settable. The bind IP defaults
      to `127.0.0.1` (ADR-013 loopback default; Round 1 Lane B
      reconciliation moved it off the previous `0.0.0.0`).
- [ ] **OSC prefix-parser coverage.** Send an inbound OSC packet
      to a route under `/preset/` or `/snap/` or `/playlist/`
      (using TouchOSC or `oscsend` from your peer machine). The
      OSC view's diagnostic counters should NOT increment a
      `parse-error` counter; the wire parser now accepts all seven
      registry-family prefixes (Control / Scene / Preset / Snap /
      Playlist / Audio / Reserved). Sending to `/data/...` returns
      a distinct `prefix-reserved` diagnostic (not a generic
      unknown-prefix error).
- [ ] **OSC bind-to-action dispatch.** Add a route binding
      via the OSC view: target = `scene:demo-kaleido` on route
      `/scene/kaleido`. Send an OSC packet (any float arg) to
      `/scene/kaleido`. The scene loads through the runtime
      command bus — Perform's preview + Scene Controls header
      bind to the loaded scene. Repeat with `snap:1` /
      `preset:0` / `playlist:next` / `control:fx:speed` (the
      last takes the OSC float arg as the new control value).
      The new typed `BindTarget` enum covers all five verb
      families.

## What's fair game for bug reports

Anything else — paste into the bug-report template below:

- Shell rendering (sidebar / status bar / window decoration / layout overflow)
- Theme inconsistencies or unstyled flashes
- Scene loading failures or visible artifacts
- Hotkey conflicts or non-functional bindings
- Author/CodeMirror crashes or hangs
- IPC errors visible in the in-app console or DevTools
- Audio IPC dropouts on the BPM ring or named band tiles (live-IPC surfaces)
- OSC routing UI quirks (not real-wire OSC unless you also test with a peer)
- MIDI device listing / binding failures
- Help corpus 404s or broken navigation
- Performance: visible drops, stutter, or memory growth at idle
- Crashes (please grab the macOS crash report from
  `~/Library/Logs/DiagnosticReports/` or Windows Event Viewer)

## Bug-report template

```
Title: <short summary>

Platform: macOS 14.5 / Windows 11 23H2 / etc.
Hardware: GPU model + audio interface
Built from: <git rev-parse HEAD> on <date>
Tester: AO / Fletcher

Steps to reproduce
1. ...
2. ...

Expected
...

Actual
...

DevTools / console excerpt
<paste>

Screenshot
<attach>
```

File issues at https://github.com/PhantasticUniverse/chromaticity/issues
with the label `pre-alpha-feedback`, or drop them in the shared channel.

## After testing

If you used the same worktree to build:

```bash
git checkout -- dist/.gitkeep          # restore sentinel
git status                              # confirm clean
```

The binary at `target/release/chromaticity-app` is safe to keep or
delete; `cargo clean` clears the entire target dir if you want a fresh
build next time.

## Carry-forward (post-pre-alpha)

- Production cert procurement (AO action) — gates
  `EV-RELEASE-PIPELINE` `Planned` → `Pass`. Required for signed +
  notarized distribution to external testers via the staging appcast.
- Wave 1c-β real-binding follow-ups (per-platform `nokhwa` + Syphon
  `Send`-trait reconciliation + `publish_syphon_output` real-impl) —
  unlocks the synthetic surfaces in the Media view.
- Future-wave candidate scope per [`docs/53` G6][docs-53]: MIDI
  binding-to-control assignment, MIDI multi-message learn, top bar
  Camera / Record real wiring (rides Wave 1c-β camera follow-ups),
  optional engine `Command::Shuffle`, Perform preset save Settings-
  TOML disk persistence, SnapBar slot-labels Settings-TOML disk
  persistence. Live preview render pipeline (ADR-011 Phase-2+) is the
  larger cross-wave gap.
- `EV-ANALYTICS-ZERO-DEFAULT` closing-docs PR (~2026-05-20) — 14-day
  soak window completes; closes Phase-4d at 6/6.

[adr-014]: ../docs/adr/ADR-014-signing-notarization-auto-update.md
[release-eng-readme]: README.md
[audit-checklist]: ../docs/EV-GATE-MODEL-AUDIT-checklist.md
[docs-53]: ../docs/53-v1-release-parity-gaps.md
[memory-protocol-baked]: ../CLAUDE.md
[memory-gitkeep]: ../CLAUDE.md
