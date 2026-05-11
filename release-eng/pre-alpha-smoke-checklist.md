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
- Recent landings (2026-05-10): Wave 1d-α PR 3 (`#426` help corpus closure),
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
- [ ] Open `welcome/quick-start.chroma` — preview renders within ~2s.
- [ ] Open `welcome/controls-tour.chroma` — control widgets appear in
      the Inputs Inspector.
- [ ] Open `library/demo-kaleido.chroma` — visuals animate; uniforms
      panel populated.
- [ ] Open `library/demo-spectrum.chroma` — confirm spectrum bars
      render (synthetic data is expected; see "Known synthetic surfaces").

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

- [ ] Open the Audio view.
- [ ] Confirm the BpmRing widget animates. Without an audio source, it
      shows synthetic motion; with system audio active (music
      playing), the ring should react to real beats within a couple
      seconds.
- [ ] Confirm BeatDots animate.
- [ ] Open `examples/four-band-reactive.chroma` from the Library, then
      switch back to the scene preview. The 4 named bands should drive
      visuals when audio is playing.
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

### G. Playlist + transitions (≈ 2 min)

- [ ] Playlist view loads with a cue list.
- [ ] CueTransitionCurve widget renders a cubic bezier preview.
- [ ] Drag-to-reorder a cue (display-level; preset-bank durability is
      Phase-4+ deferred).
- [ ] Trigger a manual cue transition — verify the curve drives a
      visible transition in the preview.

### H. OSC + MIDI views (≈ 1 min, no real I/O)

- [ ] OSC view renders the routing table and any synthetic routes.
      Real OSC wire-level testing requires a peer (e.g. TouchOSC) and is
      optional for pre-alpha.
- [ ] MIDI view renders the device list. If a MIDI controller is
      plugged in, it should appear in the list (real device enumeration
      via midir 0.10 is live). Binding it to a control is in-scope but
      not required.

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

### J. Restart + persistence (≈ 1 min)

- [ ] Quit the app (Cmd-Q on Mac / Alt-F4 on Windows).
- [ ] Relaunch.
- [ ] Theme preference + last-opened view persist.

## Known synthetic surfaces — do NOT file bugs

These ship under explicit Phase-4 narrowings (per the EXIT records in
[`docs/EV-GATE-MODEL-AUDIT-checklist.md`][audit-checklist]). Their UI
scaffolding is in place but the underlying I/O is synthetic. Treat the
visual presence as the contract; don't expect real-world data.

| Surface                                    | Status                                                                       |
| ------------------------------------------ | ---------------------------------------------------------------------------- |
| **Camera input** (Media view, Perform)     | Wave 1c-β closed synthetic; per-platform `nokhwa` real bindings deferred.    |
| **Syphon publish/subscribe** (macOS)       | UI scaffolding present; real `publish_syphon_output` + observer deferred.    |
| **AudioReactivity 48-bar FFT**             | Synthetic-driven post-MVP; only the 4 named band tiles + BPM strip are live. |
| **AudioUniforms Spectrum widget** (64-bar) | Synthetic post-MVP (the bundle's inline Spectrum, not a separate widget).    |
| **Dual-GPU selection**                     | Synthetic 10-fixture validation only; physical dual-GPU validation deferred. |
| **Crash injection drill**                  | Synthetic harness only; do NOT deliberately SIGKILL expecting recovery.      |
| **GPU thumbnail rendering** (Library)      | Deterministic byte-hash stub; real GPU thumbnails deferred post-MVP.         |

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
- `EV-ANALYTICS-ZERO-DEFAULT` closing-docs PR (~2026-05-20) — 14-day
  soak window completes; closes Phase-4d at 6/6.

[adr-014]: ../docs/adr/ADR-014-signing-notarization-auto-update.md
[release-eng-readme]: README.md
[audit-checklist]: ../docs/EV-GATE-MODEL-AUDIT-checklist.md
[memory-protocol-baked]: ../CLAUDE.md
[memory-gitkeep]: ../CLAUDE.md
