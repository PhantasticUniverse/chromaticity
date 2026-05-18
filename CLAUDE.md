<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# CLAUDE.md

Project memory for Claude Code and other coding agents. Everything below is non-obvious
context that a code-reader cannot derive from the tree. Read it on turn one; keep it short
(context budget).

## Project at a glance

Chromaticity is a deterministic, audio-reactive live-visuals runtime and authoring
environment (macOS + Windows desktop). Toolkit per `docs/adr/ADR-018-non-scene-ui-toolkit.md`:
Tauri 2 + Svelte 5 + TypeScript on the UI side; Rust 2024 edition on the engine side.
Phase 4 EXIT decisions remain AO-held per `docs/11-roadmap-phases.md`; closure events
below do not pre-empt the gate decisions. Authoritative planning corpus lives under
`docs/`; two documents to read first: `docs/43-development-standards.md` (standards)
and `docs/29-cleanroom-legal-policy-baseline.md` (clean-room SOP). Per-PR commit hashes,
T1/T2 fixture counts, and Per-Type-Minimums anchor-run-id tables for every closure live
in the closing-docs sections of `docs/EV-GATE-MODEL-AUDIT-checklist.md` and the row
description columns of `docs/26-evidence-artifact-registry.md`; this file records only
the cumulative narrowing axes and milestone commits.

### Phase progression

- **Phase 1 EXIT** — achieved at W12 (commit `e818752`), April 2026.
- **Phase 2 EXIT** — achieved 2026-05-01 at commit `d8e92db`.
- **Phase 3 EXIT** — achieved 2026-05-01 at commit `953fd74`. Seven of eight Phase-3
  rows (`EV-AUDIO-TRACE` cross-tag, `EV-AUDIO-ROUTING-MATRIX`, `EV-STATE-REPLAY`,
  `EV-STATE-MIGRATION`, `EV-PRESET-PLAYLIST`, `EV-SHOW-BUNDLE-RECOVERY`,
  `EV-PERF-BUDGETS`) at `Pass`; `EV-COLOR-MANAGEMENT-SRGB` deferred per AO
  direction behind a future render-HUD-diagnostics PR.
- **Phase 4a EXIT** — achieved 2026-05-03. Wave 1c-α-2 production audio bootstrap
  closure landed 2026-05-06; Wave 1c-β camera + Syphon adapter cluster closure
  landed 2026-05-07 at PR `#406` / commit `fb2bc75` under synthetic-narrowing
  (per-platform nokhwa concrete bindings + Syphon `Send`-trait reconciliation +
  hardware validation deferred as known follow-ups).
- **Phase 4b EXIT** — passable at AO discretion as of 2026-05-07; all 11 Phase-4b
  rows at `Pass` under collective narrowings (Wave 4b.1 through 4b.8 inclusive).
  `EV-IMPORT-SHADERTOY` PR 2 of 2 (broader corpus + iChannel fixture-pack
  resolution per `docs/38` §5.2a) closed the last directly-engineered Phase-4b
  follow-on at 2026-05-07; `EV-AUTHORING-LIVE-RELOAD` PR 2 + PR 3 carry
  forward at AO discretion.
- **Phase 4d** — 5 of 6 rows at `Pass` as of 2026-05-07;
  `EV-ANALYTICS-ZERO-DEFAULT` calendar-gated to ~2026-05-20 (14-day soak window).
- **Wave 1d-α (PR 1 + PR 2 + PR 3)** — in-app help corpus closed 2026-05-10.
  PR 1 (2026-05-08) established `docs-corpus/` mkdocs source tree + `dist/help/`
  static-HTML build output bundled via Tauri `bundle.resources` + routed `Help`
  view at `src/lib/views/Help.svelte` reading bundled HTML through Tauri's
  `convertFileSrc` (two placeholder pages: Welcome / About). PR 2 (`#419`,
  2026-05-09) shipped 4 first-wave pages (`hotkeys-reference` / `osc-routes` /
  `chroma-uniforms` / `audio-reactivity`). PR 3 (`#426` / `b57bbfc`,
  2026-05-10) shipped 5 more pages (`scene-workflow` / `library-workflow` /
  `perform-mode` / `settings-reference` / `import-shadertoy`) closing the
  corpus to 11 total pages; `EV-DOCS-CORPUS` now `Pass`.
- **Wave 1d-β (PR 1 + PR 2)** — bundled scene starter library closed
  2026-05-10. PR 1 (`#418`) seeded one clean-room scene per remaining empty
  bundle subdirectory (`welcome/welcome.chroma` + `examples/audio-reactive-demo.chroma`);
  PR 2 (`#425` / `0f67340`) shipped 10 additional clean-room scenes spanning
  five feature surfaces (uniforms / control-widget showcase, audio reactivity,
  OSC routing, hotkey-bound preset cycling, playlist transitions). Total
  corpus now 12 `.chroma` packages under `engine/scene/tests/fixtures/scenes/`
  (4 `welcome/` + 8 `examples/` + the 3 pre-existing `library/demo-*`);
  `EV-LIBRARY-CORPUS-MIN` now `Pass`.
- **Wave 1e (α + β + γ)** — release pipeline scaffold landed 2026-05-08
  through 2026-05-10. Wave 1e-α introduced `release-eng/` with stub signing
  scripts + hardened-runtime `entitlements.plist` + operator runbook; Wave 1e-β
  (`#416` / `517c37b`) added `tauri-plugin-updater` plugin scaffold (first
  Tauri 2 plugin precedent in this binary); Wave 1e-γ (`#423` / `ad3c96b`)
  added `.github/workflows/release.yml` with cross-platform matrix calling the
  stub scripts. Validated end-to-end via synthetic-signing smoke test
  (run `25628685936`) after three workflow bug-fix PRs (`#428` / `695c277`
  for vite-emptyOutDir-vs-`dist/help/` collision; `#429` / `db5b1f5` for
  `gh release create` asset-name collision; `#430` / `7736bc7` for
  `upload-artifact` ancestor-stripping). `EV-RELEASE-PIPELINE` stays at
  `Planned` until production cert procurement (Apple Developer ID + Windows
  EV + Sparkle 2 EdDSA keypair) which remains AO-gated.

Slice-3 scheduler-tick wave (closing 2026-05-01) shipped
`engine/runtime/src/scheduler/{mod,observability,audio,render,ipc}.rs` per
ADR-001 / ADR-006, introducing `EV-SCHEDULER-TICK` at `Pass`. Tick-step
ordering: audio pull (BEFORE state mutation) → bus drain + frame advance → render
dispatch (AFTER state mutation) → observability emit → IPC emit (last). Existing T2
corpora (`state_replay`, `preset_playlist`, `state_migration`) preserve goldens via
the additive design — corpora continue to use `commit_all_into` directly with
`frame=0` static state.

### Phase-3 narrowings

- `EV-STATE-MIGRATION` forward-migration only (rollback Phase-4+).
- `EV-PRESET-PLAYLIST` existing PresetStore + PlaylistStore semantics; preset-bank
  durability + scene-author preset capture + playlist reordering deferred Phase-4+.
- `EV-SHOW-BUNDLE-RECOVERY` bundle-format spec only; recovery / rollback semantics
  Phase-4+.
- `EV-PERF-BUDGETS` observable-subsystem proxy benchmarks; true frame-loop budget
  surface ships Phase-4 via the Path B nightly-only criterion harness in
  `engine/runtime/benches/budgets.rs` (~$20/month-vs-~$70-80/month AO-approved cost
  posture).

### Phase-4a cumulative narrowings (Wave 1a + 1b + 1c-α + 1c-α-2 + 1c-β)

- `EV-INTEGRATION-MATRIX` — OSC + MIDI + audio-uniform-IPC + camera + Syphon all
  widened post-Wave-1c-β-closure 2026-05-07 (16-PR sequence covering PRs `#389`-`#405`
  inclusive); synthetic-narrowing posture per the row 78 / row 40 / row 93 precedent;
  per-platform binding stubs + Syphon `Send`-trait reconciliation +
  `list_syphon_servers` `NSDistributedNotificationCenter` observer +
  `publish_syphon_output` real-impl + real hardware validation deferred to follow-up
  PRs per `docs/EV-GATE-MODEL-AUDIT-checklist.md` Phase-4a Wave 1c-β closure section.
- `EV-MIDI-EXTENDED` — `Pass` under `integrations/midi-adapter/` (midir 0.10
  cross-platform); MPE deferred to V2 with diagnostic-fires-once rejection;
  SysEx-logged-only at MVP.
- `EV-AUDIO-UNIFORM-IPC` — `Pass` (per ADR-019). Live IPC dispatch flows in
  production after Wave 1c-α-2 (`engine/audio::AudioRuntime` producer +
  `app/shell::TauriIpcEmitter` adapter + drift-corrected 60 Hz scheduler-tick
  thread in `app/shell/src/lib.rs::run`). Round 2 Lane A (PR `#484` round-2
  amendment) widened the payload with per-axis `AudioAxisAvailability::{Warming,
Ready}` + `degraded: bool`, surfaced cross-seam as the AUDIO adapter pill;
  Round 3 Lane Q (PR `#488`) unified RMS / peak with the FFT / BPM analysis
  window (jitter that prior UI EMA + label-hysteresis patches band-aided is now
  fixed at source) and renamed our four invented `syn_*` audio symbols to
  `chroma_*` to honour the namespace reservation. Two residual narrowings:
  (i) Spectrum widget retrofit deferred (post-MVP UX); (ii) `AudioReactivity`
  48-bar FFT visualiser stays synthetic-driven (only 4 named band tiles + BPM
  strip read live via 8 → 4 reduction; post-MVP UX).
- `EV-ENTITLEMENT-ENFORCEMENT` + `EV-ENTITLEMENT-REVOCATION` — `Pass` at the
  13-cap honored set (`HONORED_TOKEN_CAPABILITIES`, data-only); IO_CAMERA
  intentionally excluded — free at MVP per ADR-009. Revocation expiry-based
  only; deny-list activation deferred post-MVP.
- Hotkey trio (`EV-HOTKEY-PARITY-CONFORMANCE` / `EV-HOTKEY-COMPLETENESS` /
  `EV-HOTKEY-CONFLICT`), `EV-NET-HARDENING`, `EV-ENTITLEMENT-OFFLINE`,
  `EV-ENTITLEMENT-FREE-FEATURES`, `EV-HEALTH-METRICS-LOOPBACK-ONLY` — full scope.

### Phase-4b cumulative narrowings (Wave 4b.1 through 4b.8)

- `EV-LIBRARY-BUNDLE-LAYOUT` — full scope (four-subdir taxonomy).
- `EV-LIBRARY-DISCOVERY` — full scope (Wave 4b.2 indexer + Wave 4b.6
  search-filter IPC).
- `EV-LIBRARY-THUMBNAIL` — deterministic byte-hash stub renderer; real
  GPU-backed rendering deferred post-MVP.
- `EV-AUTHORING-DIAGNOSTICS` — JSON-class spans only; compile / script /
  coercion classes message-only.
- `EV-AUTHORING-SCAFFOLDER` — full scope.
- `EV-IMPORT-ISF-REJECT` — full scope; ISF native import deferred to V2 per
  design.
- `EV-AUTHORING-LIVE-RELOAD` — Wave 4b.2 PR 1 of 2 (`#356` / `d56f11d`)
  shipped the file-watch + reload-on-change scaffolding under
  `engine/authoring/src/live_reload.rs`; PR 2 of 2 (2026-05-07) ships the
  `PreservedState` (control_values + active_preset + active_snap +
  playlist_position) cross-seam contract per `docs/37` §3.3 hot-reload semantics.
  Runtime-side wiring (engine/runtime adapter capturing `State -> PreservedState`
  before reload + re-applying via command-bus + commit-thread per ADR-006)
  deferred to a future composition wave; latency-budget arm cross-tagged with
  `EV-AUTHORING-UX-LATENCY` (already `Pass` via PR `#363` `b2156bb`).
- `EV-SHADER-CACHE` — observation-only (no `scripts/budget-check.sh`
  extension).
- `EV-AUTHORING-UX-LATENCY` + `EV-DOCS-UX-LATENCY` — co-developed cross-tagged
  synthetic harness; full UI-driven validation deferred post-MVP.
- `EV-IMPORT-REFERENCE-SHOW` — synthetic-commit harness across 8 importer
  classes producing `CommitOutcome` + `RollbackSnapshot` shape proof; does NOT
  mutate Chromaticity user-data stores; rollback `Ok(())` no-op (real rollback per
  ADR-015 §6 + `docs/32` deferred behind entitlement-tier-gating +
  scene-library-write surfaces); OSC `/chroma/` route-prefix collision detection
  heuristic (in lieu of canonical-route registry); hotkey V1-locked +
  control-focus literal-prefix + named-action heuristics; library cross-check on
  preset `unknown-scene` mocked-empty pending live `SceneIndex` wiring.
- `EV-ACCESSIBILITY-AA` — axe-core automated audit only across all 14
  routed views; component-vs-page scope rules disabled per axe-core's documented
  `rules` parameter (10 rules covering landmark / page-heading / region /
  html-lang / document-title / bypass — will still apply at any future full-app
  integration audit); manual VoiceOver / NVDA review deferred to AO-led
  validation.
- Engine and integration crates (runtime, scene, audio, observability, render,
  script, entitlement, osc-adapter, midi-adapter) carry foundational evidence at
  EXIT quality; parallel UI port waves against the v0.2-beta bundle closed all
  three Vitest test-coverage tracks (widgets, shell components, routed views).

### Phase-4d cumulative narrowings (Wave 4d.1 through 4d.4 + row closures)

- `EV-SOAK-CHAOS` — CI-short synthetic-workload soak surface
  (sub-2-second-each in CI); ≥4-hour soak target per `docs/10` line 55 deferred to
  a calendar-gated nightly criterion-bench-style harness mirroring
  `EV-PERF-BUDGETS` Phase-3 Path B (~$20/month-vs-~$70-80/month posture);
  multi-profile fixtures use synthetic workloads representative of the
  `compat-syn-v1` and `native-chroma-v1` profiles, not the live runtime; GPU
  device-lost arm exercises a per-platform-supported flag at fixture level.
- `EV-GPU-SELECTION` — synthetic 10-fixture inline T2 corpus only
  (`engine/render/src/adapter_selection.rs`); physical / VM dual-adapter
  validation deferred (CI's wgpu round-trip arm covers at-least-one-adapter only);
  TDR-history warning UI surface deferred. Hardware-gated dual-GPU validation
  remains a future PR.
- `EV-SHADER-CACHE` — observation-only (see Phase-4b above).
- `EV-CRASH-RESUME` — all 7 restoration classes (a/b/c/d/e/f/g) shipped via
  the additive Wave 4d.3 partial-advance pattern as `restore_class_<x>` helpers in
  `engine/runtime/src/crash_resume.rs`; class d closes the
  deferred-pending-state-shape gap by extending `State` with `SessionBindings` +
  the V2 envelope `EXT_ID_V2_SESSION_BINDINGS` extension without breaking
  V1 → V2 forward-only migration per Phase-3 `EV-STATE-MIGRATION`. Full crash-injection drill
  (controlled `SIGKILL` / `SIGSEGV` / panic-in-render-thread / OOM at three
  wallclock points in a 10-minute scripted show) deferred to a future-wave
  full-injection harness composing on top of the synthetic restore-helper
  apparatus.
- `EV-CRASH-PAYLOAD-REDACTION` — synthetic crash-payload validation
  harness in `engine/observability/src/crash_harness.rs`
  (`SimulatedCrashClass` 4-arm + `SyntheticCrashPayload` constructor populating
  all 5 `RedactCategory` arms with class-distinct sentinel strings +
  substring-search detection over serialised redacted output); real
  platform-specific crash-handler injection (POSIX signal handlers / panic-hook
  wiring / OOM-killer integration) reserved for a future-wave full-injection
  harness — same precedent as `EV-SOAK-CHAOS`.
- `EV-ANALYTICS-ZERO-DEFAULT` — Wave 4d.1 Lane B scaffolding shipped;
  closing docs calendar-gated to ~2026-05-20 (14-day soak window).

### Out-of-scope (AO-deferred)

- **Wave 1c-β follow-ups** — Wave 1c-β closed 2026-05-07 under synthetic-narrowing;
  per-platform nokhwa concrete bindings + Syphon `Send`-trait reconciliation +
  `list_syphon_servers` `NSDistributedNotificationCenter` observer +
  `publish_syphon_output` real-impl + hardware validation carry forward as known
  follow-up PRs per `docs/EV-GATE-MODEL-AUDIT-checklist.md` Phase-4a Wave 1c-β
  closure section.
- **Hardware-gated** validation for `EV-GPU-SELECTION` (dual-GPU) and the
  `EV-CRASH-RESUME` / `EV-CRASH-PAYLOAD-REDACTION` full-injection crash harness.
- **Post-MVP UX enhancements** — Spectrum widget retrofit + `AudioReactivity`
  48-bar FFT visualiser live wiring (`EV-AUDIO-UNIFORM-IPC` narrowing).

### Live cross-seam dispatch verified (Wave 1c-α-2 audio + Wave 2.1 main loop + Wave 2.2/2.3/2.4/2.5 page-by-page solidification, 2026-05-13)

**Audio IPC seam (Wave 1c-α-2, 2026-05-06).** `engine/audio::AudioRuntime`
spawns the producer thread feeding `Arc<Mutex<AudioUniformSnapshot>>`;
`app/shell::TauriIpcEmitter` is the concrete `IpcEmit` adapter;
`app/shell/src/lib.rs::run` setup spawns the drift-corrected 60 Hz scheduler-
tick thread that pulls from `MutexAudioPull` and emits via
`app.emit("audio_uniform_snapshot_changed", ...)`. Visualizer widgets
(`AudioReactivity.svelte`, `BpmRing.svelte`, `BeatDots.svelte`) read live IPC in
production with synthetic fallback retained for the no-snapshot startup window
per the dual-mode "live snapshot present" gate. The `AudioReactivity` widget's
4 named band tiles + BPM strip read live via 8 → 4 reduction; the 48-bar FFT
visualiser stays synthetic-driven (post-MVP). Producer note: cpal binds the
DEFAULT INPUT device (microphone) on macOS, not system audio output; a virtual
loopback (BlackHole / Loopback) is the operator path for system-output capture.

**Wave 2.1 main-loop cross-seam activation (2026-05-11).** Pre-alpha smoke
testing on 2026-05-11 surfaced that ~60% of cross-seam IPC was gapped despite
the v0.2-beta bundle port shipping a wireframe-faithful surface. Wave 2.1
landed 11 PRs in a fast parallel sequence (Lane A scene + control + snap +
preset/playlist; Lane B Perform variant + Top bar HUD; Lane C audio bootstrap
doc cleanup + Settings persistence; Lane D MIDI device enumeration + learn
capture + OSC route inspection). All 8 user-story acceptance criteria (load a
scene, drag a control, save / recall a snap, observe audio reactivity, list
real MIDI devices, edit + persist OSC IP/port, persist Settings across
restart, toggle Perform STANDARD/CONSOLE variant) work end-to-end on the
local unsigned build. Cumulative wiring:

- **Scene load** (Lane A.1, PR `#436` / `8171306`) — `load_scene(path)` Tauri
  command in `app/shell/src/commands/scene.rs` invokes
  `engine/scene::loader::load_chroma_dir` + commits a new `Command::SceneLoad`
  through the runtime command bus; bridge `LoadedScenePayload` (id / name /
  control schema / metadata) round-trip-tested at `app/ui-bridge/tests/`;
  frontend `src/lib/ipc/scene.ts` + `src/lib/stores/loadedScene.svelte.ts`
  drive the Perform Scene Controls header binding off Library's
  double-click handler.
- **Control dispatch** (Lane A.2, PR `#441` / `cc12452`) — Fader / Knob /
  Toggle / Swatch widgets in Perform's Scene Controls + Meta Controls panels
  attach pointer-drag handlers dispatching
  `src/lib/stores/control.ts::dispatchAndRead(id, value)` which `invoke`s
  `set_control` through `app/shell/src/commands/control.rs`; engine commits
  via command bus; frontend reads back the committed value (existing
  `set_control` / `get_control` engine surface; no engine change).
- **Snap recall** (Lane A.3, PR `#442` / `1d2d93b`) — `SnapBar.svelte`
  click handlers invoke `recall_snap` / `save_snap` / `clear_snap` through
  `src/lib/ipc/snap.ts` (Cmd-click → save, Alt-click → clear, plain click
  → recall) backed by `app/shell/src/commands/snap.rs`; active slot reflects
  the engine snap store.
- **Preset + playlist** (Lane A.4, PR `#445` / `06d52a8`) — Perform right
  scene column's Previous / Next / Shuffle buttons invoke
  `advance_playlist(step)` and the preset rail invokes `recall_preset(id)`
  via `src/lib/ipc/playlist.ts` + `src/lib/ipc/preset.ts`. Engine
  `PresetStore` / `PlaylistStore` semantics from Phase-3 are unchanged;
  Shuffle advances by +1 like Next (engine has no shuffle command —
  see synthetic-surfaces below).
- **Perform variant** (Lane B.1, PR `#435` / `95d25cb`) — top bar
  STANDARD / CONSOLE toggle ported from the bundle's variant-switcher into
  `src/lib/TopBar.svelte`; `src/lib/stores/performVariant.svelte.ts`
  carries the choice; classic 2-column layout is the default (matches the
  canonical wireframe), dense 3-column is opt-in.
- **Top bar HUD** (Lane B.2, PR `#438` / `838f1ed`) —
  `src/lib/stores/transport.svelte.ts` ticks at 30 Hz from
  `performance.now()` driving the transport timecode display; BPM + VU +
  beat dots read `audioSnapshotStore.snapshot`; Camera / Record buttons
  ship as visual stubs noting "wiring in Wave 2.2"; FPS readout stays
  placeholder pending ADR-011.
- **Settings persistence** (Lane C.3, PR `#440` / `56ea703`) —
  `engine/runtime/src/settings.rs` `SettingsStore` serialises a TOML
  document under Tauri's `app_data_dir`; shell commands `get_settings` /
  `set_settings` in `app/shell/src/commands/settings.rs`; bridge
  `SettingsPayload` round-trip-tested; `src/lib/views/Settings.svelte`
  hydrates on mount, invokes `set_settings` on every form change,
  preferences survive restart.
- **Audio bootstrap doc cleanup** (Lane C.1, PR `#437` / `41a389c`) —
  empirical re-verification confirmed the Wave 1c-α-2 producer + emitter
  chain is live in production; smoke-checklist mic note added covering
  cpal default-input behaviour + macOS loopback path; no code fix
  required.
- **MIDI device listing** (Lane D.1, PR `#439` / `ed0df45`) —
  `app/shell/src/commands/midi.rs` `list_midi_devices` invokes
  `integrations/midi-adapter` `enumerate_devices` (midir 0.10
  cross-platform CoreMIDI / WinMM / ALSA); bridge `MidiDevicePayload`
  round-trip-tested; `src/lib/stores/midiStore.svelte.ts` +
  `src/lib/views/MIDI.svelte` Devices tab consume the real device list.
- **MIDI learn-mode capture** (Lane D.2, PR `#443` / `11839c7`) —
  `start_midi_learn(target)` Tauri command + `midi_learn_event` channel
  surface single-message capture from the midir input listener;
  `src/lib/stores/midiLearn.svelte.ts` drives the Learn button + per-
  message decode (NoteOn / Off / CC / PitchBend / ProgramChange).
  Multi-message capture + binding-to-control wire deferred (see
  synthetic-surfaces).
- **OSC route inspection + IP/port persistence** (Lane D.3, PR `#444` /
  `c40e63e`) — `osc_routes_snapshot` / `bind_osc_route` /
  `unbind_osc_route` Tauri commands in `app/shell/src/commands/osc.rs`
  wrap the `integrations/osc-adapter` route registry; bridge
  `OscRoutePayload` round-trip-tested; `src/lib/views/Osc.svelte` Routes
  table consumes the real store; IP / port inputs persist through the
  Settings TOML store (C.3 dep) under an OSC section that survives
  restart.

**Wave 2.2 scene-renderer wiring + shader sourcing research (2026-05-11
through 2026-05-12).** Two PRs landed under Wave 2.2:

- **SceneCanvas + SceneStill wired into Perform / Library / Playlist /
  Author preview panes** (Lane R.1, PR `#448` / `c4cb080`) — the 16
  canvas-2d procedural scene renderers under
  `src/lib/widgets/scene-renderers.ts` now drive four production preview
  surfaces. Perform's `previewHero` snippet swaps the 5-blob gradient
  placeholder for `<SceneCanvas>` keyed off the loaded scene's
  metadata; Library scene-card thumbnails read `<SceneStill>`;
  Playlist's cue table gains a thumbnail column; Author's gradient-blob
  live-preview tile renders `<SceneCanvas>` keyed off
  `AUTHOR_SCENE_META.title`. Cross-surface consistency comes from a
  new `src/lib/widgets/scene-kind-from-metadata.ts` priority pipeline
  resolving any metadata-like shape to one of the 16 SceneKinds. This
  does NOT bridge the live GPU render pipeline — it's canvas-2d
  procedural visuals only; ADR-011 Phase-2+ remains the gating gap.
- **External shader source research catalog** (Lane R.2, PR `#447` /
  `91d3c80`) — research-only; new `docs/56-shader-sourcing-research.md`
  surveys 13 candidate sources against `docs/29` distribution policy +
  `deny.toml` allowlist. Lygia recommendation: continue Phase-3 trial,
  pre-stage glslify alternatives, AO commercial dialogue at Phase-4
  EXIT. Final commercial-use election deferred to Phase-5
  `EV-LICENSE-AUDIT`. No code / vendoring / `deny.toml` edits.

**Wave 2.3 renderer pipeline fixes + Perform layout fidelity
(2026-05-12).** Two fix PRs surfaced from AO smoke testing of Wave 2.2
R.1:

- **Renderer signature + composition fixes** (Lane Q.1, PR `#449` /
  `2dd9c36`) — 11 of 16 renderers had silently dropped the bundle's
  6th `lvl` parameter from their declarations (TypeScript structural
  typing accepted the under-arity but left the `lvl` axis
  unobservable); all 16 now declare `(ctx, t, seed, w, h, lvl)`. Every
  renderer reads `lvl` for at least one visual axis with coupling
  envelope `pulse(lvl) = 0.6 + lvl * 0.4` (60% intensity at silent
  audio). Four SceneCanvas composition narrowings fixed: ResizeObserver
  no longer races rAF (clear-on-resize debounced), DPR-aware sizing,
  audio gate against the loaded-scene store, vignette overlay
  retained.
- **Perform STANDARD layout fidelity** (Lane Q.2, PR `#450` /
  `f5154a5`) — 14-gap layout reconciliation against the bundle's
  `views/perform.jsx` classic variant. Sidebar collapsed to 52 px
  icon-only rail; integrated-GPU banner default-off; VuStereo
  duplication removed; A/B CrossFader removed; plus two follow-on
  niggles (COLOR group overflow + media tile preview clipping).

**Wave 2.4 Perform-page wiring solidification (2026-05-12).** Six PRs
in parallel closed every remaining cosmetic-chrome surface on Perform:

- **Action buttons** (Lane PERF.A, PR `#453` / `8c7351d`) — six
  previously-stub buttons wired (Default / Random / Lock-all on both
  Scene Controls and Meta Controls panels); new
  `src/lib/stores/controlLock.svelte.ts` `SvelteSet`-backed singleton
  drives the per-row lock store; `dispatchControl` short-circuits on
  locked IDs so the engine never sees suppressed values; per-row dim
  affordance via `data-locked="true"` + scoped CSS.
- **Scene-list click-to-load** (Lane PERF.B, PR `#455` / `917e7b3`) —
  Perform right-column scene list dispatches `load_scene` on click; the
  local `activeScene` index gives way to a derived `loadedSceneId`
  read from `loadedSceneStore.snapshot?.id`; new
  `src/lib/data/perform-setlist-resolver.ts` maps the bundle's
  `SETLIST_FIXTURE` row titles to the Wave 1d-β clean-room corpus (4
  of 12 rows resolve to closest-kind pairings; the remaining 8 return
  `null` and stay display-only pending a future-wave corpus widening).
- **Preset save engine widening** (Lane PERF.C, PR `#456` / `b3eb001`)
  — closes the "scene-author preset capture" axis the Phase-3
  `EV-PRESET-PLAYLIST` narrowing deferred. New `Command::PresetSave` +
  `EventPayload::PresetSaved` variants; `PresetStore` carries a
  `HashMap`-backed `banks` field keyed by `PresetId`; `State::apply_event`
  for `PresetSaved` clones `control_store.values` into the bank. Cmd-
  click on a preset slot saves; click recalls. **In-process snapshot
  only** — Settings-TOML disk persistence deferred (carries forward as
  a future-wave concern if user feedback warrants).
- **Adapter pill liveness** (Lane PERF.D, PR `#454` / `aca28c3`) — the
  TopBar adapter pills (OUT / MIDI / OSC) wire to live cross-seam state,
  replacing the hard-coded `STATUS_PILLS` fixture. OUT pill reads
  `loadedSceneStore.snapshot` (`ok` when a scene is loaded); MIDI pill
  reads `midiStore.devices` (`ok` when ≥1 device enumerated); OSC pill
  reads a new `oscHealthStore` polling `osc_health_snapshot` every 5 s.
- **Media filter + dead-control drop** (Lane PERF.E, PR `#452` /
  `5e2a1f9`) — Perform's media-tab pills filter the media grid; three
  dead controls dropped (Setlist dropdown / Play Setlist button / media
  Refresh). Tabs map `all` / `live` / `media` against the `MediaCardKind`
  union (`none` / `webcam` / `video` / `test`).
- **SnapBar slot labels** (Lane PERF.F, PR `#451` / `e0276c5`) —
  right-click any SnapBar slot to set a custom label ("Intro" / "Drop"
  / "Outro" etc.). Labels persist in localStorage across restarts via
  the new `src/lib/stores/snapLabels.svelte.ts` class-instance
  singleton. **Engine snap state stays binary saved/unsaved** —
  Settings-TOML disk persistence deferred.

**Wave 2.5 Library-page wiring solidification (2026-05-12 through
2026-05-13).** Five PRs in parallel closed the remaining drift on the
Library view:

- **Sidebar preview + metadata + Author handoff** (Lane LIB.A, PR
  `#461` / `c6f55cc`) — the carry-over `SETLIST_FIXTURE` "Your Setlist"
  sidebar (a W9 placeholder absent from the canonical wireframe) gives
  way to the bundle-faithful right pane: a `<SceneCanvas>` preview
  keyed off scene metadata, a metadata detail column (title / source
  pill / credit / description / category + tag chips / controls-count
  badge), and a pair of action buttons that round-trip through the
  Wave 2.1 A.1 `load_scene` IPC seam. Includes an Open-in-Author
  handoff.
- **Tag badges + favorites** (Lane LIB.B, PR `#459` / `3729e6c`) —
  tag chips overlay each scene card thumbnail (top-left, max 3 visible
  with a `+N more` overflow pill); heart-icon button overlays top-right
  toggling localStorage-backed favorite membership via a new
  `src/lib/stores/libraryFavorites.svelte.ts` `SvelteSet`-of-id
  singleton; new FAVORITES filter chip narrows the grid to favorited
  cards only. **Frontend-only** — engine support intentionally
  deferred per the clean-room scene-write narrowing.
- **Source filter + recent + sort** (Lane LIB.C, PR `#458` /
  `e337373`) — three filter / sort axes ported from the bundle's
  panel-head + toolbar: source `<select>` (`all` / `built-in` /
  `my-scenes`); new `libraryRecent.svelte.ts` localStorage store
  (max 20 entries, prepend-on-push with de-dup); `RECENT` chip; sort
  `<select>` (`id` / `title` / `recent` / `favorites`). **Frontend-
  only** — engine support intentionally deferred per the clean-room
  scene-write narrowing.
- **Card thumbnails fade-in + 50-card budget** (Lane LIB.D, PR `#457`
  / `85b88f0`) — note: the structural `SceneStill` wiring already
  shipped in Wave 2.2 R.1 via `sceneKindFromMetadata` /
  `sceneSeedFromMetadata` against each `SceneCard`; this lane closes
  the two remaining items — first-paint fade-in CSS animation (240 ms
  ease-out, suppressed under `prefers-reduced-motion: reduce`) + two
  synthetic-corpus regression guards (kind-resolution spot-check +
  50-card mount under 100 ms budget).
- **Empty state polish + double-click handoff** (Lane LIB.E, PR
  `#460` / `c67767f`) — `EmptyState.svelte` gains an optional `meta`
  prop for the supported-formats footnote (".chroma · .chromaPack ·
  .synScene conversion · Shadertoy URL · ISF reject"); a second copy
  entry `EMPTY_LIBRARY_FILTERED` covers the search-filter-exclude-all
  branch; per-card `loadingPath` state renders `aria-busy="true"` on
  the actively-loading tile (sibling tiles stay unaffected).

**DESIGN.md alignment (PR `#462` / `bd9eb96`, 2026-05-13).** Routine
token-contract reconciliation against the rendered shell per
`DESIGN.md`'s own "the shell wins" rule. Four divergences corrected:
`rail.width` `48px → 76px`; `status-bar.height` `22px → 28px`; ASCII
layout block updated; Snap panel anatomy "16-slot grid → 8-slot grid"
(engine has 16, SnapBar trims to 8 per `SNAP_SLOT_COUNT` in
`snap.svelte.ts`). Snap panel paragraph also gains a one-sentence note
about the Wave 2.4 PERF.F slot-label affordance.

**Wave 2.6 Audio-page solidification (2026-05-13).** Five PRs
(`#463` AUD.E snapshot button, `#464` AUD.D band tiles live 8 → 4
reduction, `#465` AUD.C uniforms peak / rms live, `#467` AUD.B
Settings monitoring meter live, `#469` AUD.A audio device
enumeration) closed live audio-snapshot consumption through the
Audio view widgets.

**Wave 2.7 Settings-page solidification (2026-05-13).** Four PRs
(`#471` SET.A tempo + theme TOML; `#472` SET.D reset + about; `#473`
SET.C output device enum; `#474` SET.B storage + livesources picker)
closed the Settings view. Drift-cleanup PR `#470` removed stale
Phase-3+ comments.

**Wave 2.8 cross-cutting fixes (2026-05-15 through 2026-05-16).** Six
PRs (`#476` LIB.LOCAL scenes-local + media-local folders; `#477` Q.1
meter EMA smoothing; `#478` AUD.SRC operator input-device preference
honoured at startup; `#479` SC.B renderer default-state speed bump;
`#480` SC.A SceneCanvas control pre-multiplier for `speed` /
`intensity` / `tint`; `#481` SC.C AudioReactivity band-pct label
hysteresis). 16 seed `.chroma` packages dropped under `scenes-local/`
(gitignored).

**Pre-alpha fix passes round 1 → round 3 (2026-05-17 through
2026-05-18).** Three parallel rounds against a mega-review
(commissioned via `/ultrareview`) and a follow-on clean-room
reference comparison sweep. **Round 1** (6 PRs, `#482`-`#487`) closed
the mega-review's BLOCKER / HIGH / MEDIUM cohort: IPC wrappers for
audio + syphon (`#482`); command bus ordered commit so a dispatched
command drains through its own commit (`#483`); cpal `Pending`
polling + FFT / BPM accumulator (`#484`); Perform scene-scoped
`controlValues` + welcome bundle mapped through `tauri.conf.json`
`bundle.resources` + Editor reconciled mount via compartments
(`#485`); atomic settings write + quarantine + hydration gate +
crash-resume save / recover wired into Tauri launch + OSC default IP
loopback alignment (`#486`); SceneCanvas rAF reactivity untrack +
renderer control coverage widening + `data-preview-wired` affordance
scaffold (`#487`). **Round 2** amended `#484` / `#486` / `#487` to
close partial-closures called out by the round-1 reviewer fan-out:
`AudioAxisAvailability::{Warming, Ready}` discriminator + `degraded`
flag wired cross-seam + AUDIO adapter pill in TopBar; crash-resume
`discard_crash_resume_snapshot` inverted to preserve snapshot
across clean exits (recovery is idempotent) + `CrashResumeStatusPayload`
migrated to `app/ui-bridge`; SceneCanvas reactivity untracked across
`kind` / `seed` / `controls` / `dpr` + renderer control coverage
widened to rings / orbit / tunnel / fluid / plasma + universal
`invert` / `mirror` post-pass + `data-preview-wired="false"`
affordance shipped for unmapped controls in Perform. **Round 3** (2
PRs, `#488`-`#489`) closed the clean-room comparison sweep's
pre-alpha-blocking findings: audio `syn_*` → `chroma_*` rename for
the four operator-invented symbols (`syn_RMS` / `syn_PEAK` /
`syn_FFT_LO` / `syn_FFT_HI` → `chroma_AudioRMS` / `chroma_AudioPeak`
/ `chroma_AudioFFT[8]`) + RMS / peak unified with the FFT / BPM
analysis window (`#488`); OSC bind-to-action dispatch via typed
`BindTarget` enum + prefix-parser taxonomy widened to all seven
registry families (Control / Scene / Preset / Snap / Playlist /
Audio / Reserved) with distinct `PrefixReserved` diagnostic +
default OSC ports `DEFAULT_OSC_IN_PORT = 9000` /
`DEFAULT_OSC_OUT_PORT = 9001` pinned in `defaults.rs` (`#489`).

**What remains synthetic-only.**

- **Live preview render pipeline** — ADR-011 Phase-2+ deferred. Dragging a
  control fires `set_control` through to the engine but the Perform preview
  pane stays a gradient placeholder; the scene's loaded GLSL is parsed and
  the control schema honored, just not rasterised. The canvas-2d procedural
  preview adapter (`src/lib/widgets/scene-renderers.ts`) was widened in Round 2
  Lane C to honour `rotation_speed` / `warp_amount` on rings / orbit / tunnel
  / fluid / plasma + universal `invert` / `mirror` post-pass; unmapped controls
  render dimmed via `data-preview-wired="false"` affordance in Perform so the
  silent-no-op feel of round-2-prior UX is closed at the operator surface.
- **Camera real bindings + Syphon `publish_syphon_output` real-impl** —
  Wave 1c-β follow-ups; per-platform nokhwa concrete bindings + Syphon
  `Send`-trait reconciliation + `list_syphon_servers`
  `NSDistributedNotificationCenter` observer all carry forward.
- **AudioReactivity 48-bar FFT visualiser + AudioUniforms 64-bar Spectrum**
  — post-MVP UX per the Wave 1c-α-2 narrowing record.
- **Dual-GPU validation + crash-injection drill** — hardware-gated per
  the Phase-4d narrowings.
- **MIDI multi-message learn capture** — Wave 2.1 D.2 ships single-message
  MVP only; multi-message capture (debounce + abort + composite binding)
  carries forward as a future-wave follow-up (Wave 2.2 ended up doing
  scene-renderer wiring + shader sourcing research, not MIDI work).
- **MIDI binding-to-control assignment** — D.2 captures the learned event +
  stores to localStorage but does not yet apply the binding to a specific
  control id; the wire from learn-store → control-store carries forward
  as a future-wave follow-up.
- **Shuffle advance** — engine has no shuffle command; the Perform right-
  column "Shuffle" button advances by +1 like Next (visual-only label).
- **Top bar Camera / Record buttons** — visual stubs only; real camera +
  record wiring rides the Wave 1c-β follow-ups (per-platform `nokhwa`
  bindings + Syphon `publish_syphon_output`) rather than the Wave 2.x
  page-by-page series.
- **Perform preset save disk persistence** — Wave 2.4 PERF.C widened the
  engine with `Command::PresetSave` for in-process snapshot capture, but
  Settings-TOML disk persistence is deferred.
- **SnapBar slot labels disk persistence** — Wave 2.4 PERF.F persists
  custom slot labels through localStorage only; engine snap state stays
  binary saved/unsaved with no Settings-TOML disk persistence yet.
- **Library favorites + recent + source-filter engine support** — Wave 2.5
  LIB.B / LIB.C ship frontend-only stores (`libraryFavorites.svelte.ts` +
  `libraryRecent.svelte.ts`) backed by localStorage; engine-side support
  intentionally deferred per the clean-room scene-write narrowing.

## Fast commands

Frontend, from repo root:

```
npm ci                     # once, after clone
npm run format:check       # prettier
npm run lint               # eslint .    (config is authoritative scope)
npm run tsc                # tsc --noEmit (app + tsconfig.node.json)
npm run check              # svelte-check
npm run dev                # vite dev server on localhost:1420
```

Rust, from repo root:

```
cargo check   --workspace --all-targets
cargo clippy  --workspace --all-targets -- -D warnings
cargo test    --workspace --all-targets --no-fail-fast
cargo fmt --all -- --check
cargo deny   check {advisories,licenses,bans,sources}   # needs cargo-deny
cargo hakari generate --diff                            # needs cargo-hakari
```

Policy / evidence:

```
bash scripts/evidence-tag-lint.sh    # strict on tier tokens, evidence-tag coverage (engine/**, integrations/**), and EV-ID validity against docs/26
bash scripts/adr-lint.sh             # Phase-0 grace mode currently reports 0 warnings
bash scripts/trailer-check.sh HEAD   # DCO + Profile-Impact trailers
npm run commitlint:last              # Conventional Commits check
```

The cargo target directory is shared at the workspace root `/target/`; there is no
per-crate `target/` to clean.

## Workspace layout

```
chromaticity/
├── app/
│   ├── shell/            # Tauri 2 binary (chromaticity-app crate)
│   └── ui-bridge/        # ts-rs IPC type generator (Rust → TypeScript bindings)
├── engine/               # runtime/, scene/, audio/, authoring/, observability/, render/, script/, entitlement/
├── integrations/         # osc-adapter/, midi-adapter/, camera-adapter/, syphon-adapter/
├── third_party/          # vendored snapshots — lygia/ (Lygia v1.1.4 trial vendor) + Syphon-Framework/ (BSD-3-Clause clean-room)
├── workspace-hack/       # cargo-hakari managed crate (at root by convention)
├── .config/hakari.toml   # cargo-hakari config; platforms mirror deny.toml graph.targets
├── src/                  # Svelte web root; may migrate under app/ui/ later
├── tests/                # workspace-level cross-cutting tests
├── dist/                 # Vite build output — .gitkeep is load-bearing; bundled mkdocs help under dist/help/
├── docs/                 # normative planning corpus + ADRs
├── docs-corpus/          # in-app help mkdocs source tree (Wave 1d-α; built into dist/help/)
├── release-eng/          # signing/notarization/staging-appcast stub scripts (Wave 1e-α; cert-gated)
├── evidence-runs/        # evidence-run capture artifacts (per-row anchor bundles)
├── scripts/              # policy / CI shell helpers
├── .github/workflows/    # rust, frontend, sbom, evidence-tag-lint, policy, labels
├── Cargo.toml            # workspace root (edition 2024, MSRV 1.85.0)
├── deny.toml             # cargo-deny advisories / licenses / bans / sources
├── lefthook.yml          # pre-commit / pre-push hooks
├── vitest.config.ts      # Vitest runner (jsdom + Svelte plugin)
└── package.json          # @chromaticity/app
```

`Contents/` (clean-room reference material) and `chromaticity-planning/` (planning
archive) live on disk but are `.gitignored`, `.prettierignored`, and excluded from ESLint.
Source tooling MUST NOT parse them.

## Commits — every commit carries two mandatory trailers

Conventional Commits with Chromaticity vocabulary. Every commit ends with:

```
Signed-off-by: <Name> <email>
Profile-Impact: none | compat-syn-v1-{minor,major} | native-chroma-v1-{minor,major}
```

Missing trailers fail `scripts/trailer-check.sh` and block merge. `compat-syn-*` = impact
on the Synesthesia-compat API surface; `native-chroma-*` = impact on the
Chromaticity-native surface; `none` = no user-visible API / behavior change.

**Types** (commitlint-enforced): `feat` `fix` `perf` `refactor` `docs` `test` `build` `ci`
`chore` `revert`.

**Scopes** (commitlint-enforced; additions require a `policy`-scoped PR editing
`commitlint.config.cjs`): `runtime` `scene` `audio` `integration` `authoring` `library`
`ui` `docs` `ux` `entitlement` `observability` `release-eng` `policy` `repo`.

**Shape:** header ≤ 100 chars; subject ≤ 72 chars, lower-case, no trailing period;
body / footer ≤ 100 chars per line. Scope is mandatory on `feat` / `fix` / `perf` /
`refactor` per `docs/43` §4.3 (enforced by the `scope-required-for-semver-types`
plugin rule in `commitlint.config.cjs`); optional on `docs` / `test` / `build` / `ci` /
`chore` / `revert`.

## Unique conventions and constraints

**Rust lints (workspace-wide):**

- `unsafe_code = "forbid"` at the workspace root.
- `clippy::unwrap_used = "deny"`, `clippy::expect_used = "deny"`. Tests opt out with
  `#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]` at module level.
- MSRV pinned to 1.85.0 in `rust-toolchain.toml`. Bumping needs a `policy`-scoped PR.

**TypeScript / Svelte:**

- `@typescript-eslint/no-explicit-any: "error"`.
- `tsconfig.json` sets `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`,
  `verbatimModuleSyntax`, and `moduleResolution: "bundler"`.
- Svelte 5 runes mode (`compilerOptions.runes: true`). Use `$state`, `$derived`, `$effect` —
  not Svelte 4 stores.
- When a rune is needed in a `.ts` module (e.g., reactive state in a store), use the
  `.svelte.ts` extension; ESLint has a parser override for this glob (per
  `feedback_eslint_svelte_dot_ts_parser.md`).

**REUSE 3.2:**

- Every new source file carries inline `SPDX-License-Identifier` and
  `SPDX-FileCopyrightText` headers. Binary / JSON / markdown files use HTML-comment or
  path-rule coverage via `REUSE.toml`.
- First-party Chromaticity code: `LicenseRef-Chromaticity-Proprietary`. Docs under `docs/`
  are `CC-BY-4.0` (override rule in `REUSE.toml`).

**Lefthook:**

- Requires `evilmartians/lefthook` v2.1.6 or later. `brew install lefthook` ships the
  right package. Do NOT use the npm package `@arkweid/lefthook` — it is a separate
  abandoned project whose v0.7.7 binary blocks commits against modern `lefthook.yml`
  syntax.
- `--no-verify` is for emergencies only. Fix the underlying issue instead.

## Never touch without explicit direction

- `Contents/` — clean-room reference material; source tooling must not parse it (see
  `docs/29-cleanroom-legal-policy-baseline.md`).
- `chromaticity-planning/` — read-only design + planning references (`.gitignored`,
  source tooling must not parse them); the `ui-predev-bundle*/` subtrees are active port
  references for UI/UX work — see "UI design references" below.
- `LICENSES/` — canonical third-party license text; preserve upstream formatting.
- `docs/adr/*` and `docs/29-cleanroom-legal-policy-baseline.md` — non-delegable; AO-only
  approval.
- Anything under `**/*.synScene` or `Contents/Resources/release/`.

## Load-bearing files that look optional

- `dist/.gitkeep` — Tauri's `generate_context!()` macro expands at `cargo check` time and
  reads `frontendDist` via `build.rs`. A missing `dist/` triggers a compile-time panic.
  `.gitignore` tracks only the sentinel and ignores everything else inside. **Note:**
  `npm run build` (vite build) clobbers the sentinel when emitting fresh artifacts;
  restore with `git checkout -- dist/.gitkeep` before staging, or you commit the
  deletion and break `cargo check` on fresh clones. `npm run dev` does not clobber.
- `app/shell/icons/icon.png` — Tauri config requires a window icon even when
  `bundle.active = false`. The 83-byte transparent placeholder is intentional.
- `Cargo.lock` — tracked (required for reproducible builds).
- `rust-toolchain.toml` — honored by CI and local builds. Do not override with
  `+stable` / `+nightly` without a work-package requirement.

## Anti-patterns we already learned

- Don't add features, refactor, or introduce abstractions beyond what the task requires.
  Three similar lines beats a premature abstraction. No half-finished implementations.
- Don't run `npx eslint .` through tooling that bypasses `eslint.config.js`. The config's
  `ignores` protect against the `Contents/` 23k-error landmine; third-party integrations
  that don't load the config will blow up.
- Don't bypass lefthook with `--no-verify`; fix the underlying issue.
- Don't use `.expect()` / `.unwrap()` in non-test Rust code — clippy will deny.
- Don't widen clippy or ESLint rules without a `policy`-scoped PR.
- Don't update `CODEOWNERS` outside a `policy`-scoped commit.
- Don't touch `docs/adr/ADR-*.md` without AO sign-off.
- Don't hand-edit `CHANGELOG.md` — `release-please` populates per-release sections on
  merges to `main`.

## GitHub operations — known gotchas

Surfaced landing the first protected-branch PR in this repo; none are obvious from the
tree.

- **Bare hash-number references in commit bodies break commitlint.** Patterns like
  `PR #2` or `fixes #17` are treated by `conventional-commits-parser` as GitHub
  references, and the parser then mis-identifies the footer boundary and fails
  `footer-leading-blank`. Write "the present PR", "PR number 2", or "this PR" in body
  prose instead. Subject lines are unaffected.
- **`gh pr edit --title` and `gh pr edit --body-file` may silently no-op** due to a
  GraphQL Projects-classic deprecation error hitting the underlying PR-edit mutation —
  both commands return zero plus a warning but the title / body does not change. Do
  not trust the zero exit; verify with `gh pr view <N> --json title` (or `--json body`)
  after the edit. Workaround uses the REST PATCH endpoint:
  `gh api -X PATCH repos/OWNER/REPO/pulls/<N> -f title='<new title>'` for the title,
  and `gh api -X PATCH repos/OWNER/REPO/pulls/<N> --field body=@/tmp/pr-body.md` for
  the body.
- **`pull_request` workflows do not retrigger on title / body edits by default** — the
  trigger types are `[opened, synchronize, reopened]`. To re-run CI after a title edit
  (e.g., to clear a commitlint title failure), close + reopen the PR:
  `gh pr close <N>; gh pr reopen <N>`. `gh run rerun` does not work here — it replays
  the cached event payload with the old title.
- **On `pull_request` events, `HEAD` in a checked-out workflow is the synthetic merge
  commit** at `refs/pull/<N>/merge`, which carries GitHub's auto-generated message with
  no DCO / Profile-Impact trailers. Any CI step that runs against `HEAD` misses the PR's
  actual commits. Walk `$BASE_SHA..$HEAD_SHA` via `git rev-list` instead. The
  `trailer-check` job was fixed after this surfaced on the first PR.
- **`dorny/paths-filter@v3` needs `pull-requests: read` on the workflow token.** On
  `pull_request` events the action calls
  `GET /repos/{owner}/{repo}/pulls/{number}/files` to enumerate changed paths; with
  only the default `contents: read`, it fails with "Resource not accessible by
  integration" and silently degrades to "no matches" on fork PRs. Grant
  `pull-requests: read` workflow-wide alongside `contents: read` whenever a workflow
  uses `dorny/paths-filter` (or any action that hits the pulls REST API) in an in-job
  filter. The three per-workflow `*-summary` aggregators in `rust.yml` /
  `frontend.yml` / `sbom.yml` depend on this scope.
- **Merge queue is NOT active (plan-gated).** The five required-check workflows
  (`rust.yml`, `sbom.yml`, `frontend.yml`, `policy.yml`, `evidence-tag-lint.yml`)
  already trigger on `merge_group` and plumb `merge_group.base_sha` /
  `merge_group.head_sha` through the range-walking jobs, so they're
  forward-compatible. The ruleset that would actually enable the queue
  (`merge_queue` rule type on `refs/heads/main`) is rejected by the API
  because merge queue on private repositories requires GitHub Team or
  Enterprise Cloud; the current plan (Pro) does not include it. Consequences
  for today:
  - Merges on `main` still go through the normal squash-merge path under
    branch protection's `strict: true` + `required_linear_history: true`.
    Slice-style parallel lanes still pay the O(N²) rebase cascade.
  - Workflow surface is prepared: if the plan is ever upgraded to Team or
    higher, creating the ruleset via
    `POST /repos/.../rulesets` (with a `merge_queue` rule targeting
    `refs/heads/main`, `merge_method=SQUASH`, `grouping_strategy=ALLGREEN`)
    is the only remaining step; no workflow changes needed at that point.
  - The parallel-dep-adder recipe below remains the manual-rebase path that
    the queue would otherwise absorb.
    Compat patterns already in place for eventual queue activation:
  - Range-walking jobs (`dco`, `trailer-check`, `commit-convention-lint`'s
    PR-range step) read base/head from the null-coalescing expression
    `${{ github.event.pull_request.base.sha || github.event.merge_group.base_sha }}`
    so the same job body works for both event types.
  - `dorny/paths-filter` jobs gate the filter on
    `github.event_name == 'pull_request'` and combine with a "non-PR bypass"
    step that forces `output=true` on push or merge_group events.
  - `commit-convention-lint`'s `Lint head commit subject` step handles
    `push` and `merge_group` events by linting subject only, not body or
    footer — PR-body-composed squash commits contain markdown that
    regularly violates body/footer length rules written for hand-authored
    commit messages.
- **Squash-merge commits compose from the PR title + PR body, NOT from concatenated
  commit messages.** The repo setting (Settings → General → Squash merging) is
  "Default to pull request title and description". Consequences for PR authors:
  - The PR body becomes the main-side commit body verbatim.
    `scripts/trailer-check.sh` validates `Signed-off-by` and `Profile-Impact`
    on the main-side commit, so both trailers MUST live at the end of the PR
    body (the `.github/pull_request_template.md` stub seeds them).
  - `git interpret-trailers` only recognises a trailer block as the LAST
    paragraph. Do not add prose after the Trailers section of a PR body; any
    post-trailer content terminates the trailer zone and re-breaks
    `trailer-check`.
  - Branch commits are no longer constrained to single-commit-per-PR — the
    old `---------` divider failure mode only arose when the squash template
    concatenated commit messages, which is no longer the case. Branches can
    carry however many commits are useful for review; only the merge commit's
    shape matters for trailer-check.
  - The branch-level commits themselves still need `Signed-off-by` (the `dco`
    required-context validates every commit in the PR range, not just the
    merge commit). Lefthook's pre-push `trailer-check` validates the branch
    HEAD commit message.
  - **`trailer-check` has three firing points, not two.** The PR-event job in
    `.github/workflows/policy.yml` (a) range-walks branch commits AND (b)
    reconstructs the synthesised squash body from `PR_TITLE + "\n\n" + PR_BODY`
    and pipes it through `scripts/trailer-check.sh --stdin`. The (b) step is
    the one that catches a PR body that bypassed `.github/pull_request_template.md`
    and omits Profile-Impact — branch-commit trailers DO NOT survive squash-
    merge, so (a) alone will pass on a bad-body PR. The third firing point is
    the `push`-to-`main` backstop on HEAD; a red status there indicates either
    a parser regression or a PR that bypassed (b), and is remediated with a
    forward-looking fix commit (branch protection disallows history rewrite of
    `main`). PR #83 / merge `09a067d` motivated adding step (b).
- **`gh pr update-branch` breaks DCO and `trailer-check`.** The command asks GitHub to
  update the PR branch from its base, which creates a SERVER-side merge commit signed
  only by GitHub (`GitHub <noreply@github.com>`, no `Signed-off-by`, no
  `Profile-Impact`). That commit fails the `dco` and `trailer-check` required contexts
  on the next CI run and blocks merge. When a laggard PR goes out of date under
  `strict: true` branch protection, use `git rebase origin/main` in a local worktree
  followed by `git push --force-with-lease=<branch>:<old-sha>`; never the GitHub web
  button, never `gh pr update-branch`. Applies equally to the "Update branch" button
  in the PR UI.
- **Architecture-labeled PRs require `ADR:` + `Evidence:` trailers; solo-merge PRs
  require `Succession-Waiver: T1`/`T2`.** Two required branch-protection contexts on
  `main` enforce this: `architecture-trailer-check` (gated on the `architecture`
  label, validates the trailers exist plus reference real
  `docs/adr/ADR-NNN-*.md` files plus valid registry EV-IDs) and
  `succession-waiver-check` (validates `Succession-Waiver: T1`/`T2` format
  whenever the trailer is present). The PR_TITLE plus PR_BODY are reconstructed
  by the workflow into the synthesized squash-merge body and piped to
  `scripts/architecture-trailer-check.sh` / `scripts/succession-waiver-check.sh` —
  meaning the trailers must live in the **PR body**, not just on individual
  branch commits, since GitHub's squash-merge composes the merge commit from the
  PR title plus body. If your architecture PR fails the architecture-trailer-check
  required context, append the missing trailers to the PR body (via
  `gh pr edit --body-file` or the REST PATCH fallback per the `gh pr edit --title`
  gotcha above) — do NOT add them as a separate commit on the branch. Both
  checks are no-ops on PRs without the matching label / trailer, so
  non-architecture / non-solo-merge PRs are unaffected. Rollback path: a
  misbehaving context can be removed with
  `gh api -X DELETE repos/PhantasticUniverse/chromaticity/branches/main/protection/required_status_checks/contexts/<context-name>`,
  which reverts the check to advisory non-required without touching the
  workflow file.

## Agent parallelism playbook

Use sub-agents in parallel when work bundles are branch-independent; keep serial
execution when they share choke points. Cheap rule set, derived from the PR #4 → #5 → #6
re-harden sequence post-mortem:

**Parallel-safe** (spawn one sub-agent per bundle, each on its own branch off latest
`main`):

- Per-subtree implementation work under a single CODEOWNERS entry (`engine/<x>/`,
  `app/<x>/`, `integrations/<x>/`). Each subtree's PR has a self-contained diff.
- Per-subtree dep-adding work — introducing new external crates in parallel subtree
  PRs is viable under strict-true branch protection. Each lane commits its own
  `cargo hakari generate` output as part of its squash-clean PR. When a sibling
  merges first, remaining lanes rebase onto advanced `main`; if the rebase
  conflicts on `workspace-hack/Cargo.toml` or `Cargo.lock`, resolve with either
  `git checkout --ours` or `--theirs` (both converge — `cargo` re-resolves the
  lockfile against the combined manifests on the next `cargo metadata` call)
  and re-run `cargo hakari generate`. Hakari emits the unified superset;
  `cargo hakari manage-deps --dry-run` reporting "no operations to perform" is
  the settlement signal. Slice-2 validated this with three parallel dep-adders
  (serde, cpal, rosc) plus one dep-free lane. The older rule ("dep-adds must
  be serialized") proved too strict. Merge queue, if ever enabled at the plan
  level, would absorb this rebase pass automatically; see the GitHub-
  operations section for the queue-activation follow-up.
- Independent ADR drafts in `docs/adr/` (one ADR per agent; the AO-only approval
  constraint still applies at merge time).
- Independent evidence-tag-producing test suites, once the crate they live in exists.
- Context gathering, CI watchers, read-only searches. Always parallelize these.

**Serial only** (single-owner, rebase-then-push; no sub-agent fan-out):

- Any edit to `Cargo.toml [workspace] members`, `commitlint.config.cjs`, `deny.toml`,
  `CODEOWNERS`, `REUSE.toml`, `.github/workflows/**`,
  `docs/26-evidence-artifact-registry.md`, or `docs/00-phase-0-placeholder-inventory.md`.
  These files are global choke points; concurrent edits produce either text-conflict
  merges or semantic conflicts the linter won't catch.
- Any PR whose merge-state is a prerequisite for another PR's CI to be meaningful —
  e.g. the PR #4 → #5 → #6 re-harden sequence (each PR needed the previous branch-
  protection transition), or the PR-A workspace-members scaffold before parallel
  PR-B..E per-crate implementations.
- Anything touching branch-protection required contexts, squash vs merge-commit policy,
  or GitHub repo settings (fork policy, default branch, signed-commit enforcement).

**Spawning rule:** if a bundle touches _any_ file on the serial list, do that work
first in the main context, land it, then fan out sub-agents for the parallel follow-
ups. Never let a sub-agent own both a serial-list touch and a parallel-safe subtree in
the same PR — split the bundle.

**Branch hygiene after sub-agent fan-out:** each sub-agent rebases onto latest `main`
before push, not onto a sibling sub-agent's branch. Sibling branches are only visible
via origin after their respective merges; local stacking produces dependency chains
that the squash-merge workflow silently breaks.

## Adjudicating briefs against the tree

Briefs describing concrete file paths, test tiers, or slice scope sometimes conflict
with in-source invariants or normative docs. Default posture: **the tree wins**, same
as the "When this file is wrong" rule at the bottom of this doc. Three heuristics,
surfaced from the PR #47 `EV-COMPAT-FIXTURE-COVERAGE` scope-correction:

- **Crate-private visibility is a slice-boundary signal, not friction to route around.**
  `pub(crate)` on a schema type whose module-level prose documents a slice-N widening
  trigger means the briefer cannot route around it with a `serde_json` dev-dep,
  `#[doc(hidden)]` test constructors, or struct-literal fixtures in a separate
  integration-test crate. The visibility invariant IS the slice scope declaration.
  Example: PR #50's `engine/scene/src/schema.rs` module-level prose makes this
  load-bearing explicit — "every field is `pub` (the slice-2 `pub(crate)` visibility
  was the slice-boundary signal this slice realizes)" — confirming the slice-2
  visibility was the boundary marker that the slice-3a deserializer honored when it
  opened the schema. A brief proposing external construction in slice-2 was the
  weaker document and the work correctly relocated in-crate at T1.
- **Docstring invariants in schema / contract files are normative.** Module-level
  prose in files whose job is to declare a contract (`engine/*/src/schema.rs`,
  `docs/26`, `docs/34`, `docs/43`) is the slice boundary encoded in-source. When a
  brief conflicts with an in-source invariant that cites ADRs or slice scope, the
  brief is the weaker document.
- **Re-derive test tier from the registry `Generation Source` column, not the brief.**
  A brief's `T<n>` label is inherited data; the canonical axis is `docs/26` row
  `Generation Source` (e.g. `EV-SCENE-IMPORT`'s "Scene import harness + golden corpus fixtures"
  as the T2 target for `EV-SCENE-IMPORT`) read against `docs/43` §15.5 T-tier
  definitions. A tier-mislabel produces a test module that can't satisfy its row — a
  rollup inconsistency `evidence-tag-lint` surfaces much later than the PR that
  introduced it.

## UI design references — wireframes drive UI/UX work

Two read-only assets in `chromaticity-planning/` define the canonical visual targets for
every UI/UX decision (`.gitignored` and `.prettierignored`; source tooling must not parse
them, but `Read` and review are the entire point of the bundle):

- **`chromaticity-planning/ui-predev-bundle-v0.2-beta/`** — design-surface-complete
  React/JSX bundle (stamped 2026-04-23f, marked partial: design-surface only;
  engineering-mechanics deferred to dev queue). Per-view JSX under `views/` (perform,
  library, scene, playlist, media, midi, osc, import, settings, audio, patch, onboard;
  the bundle's `scene.jsx` is the full Author port source) plus shared primitives —
  `chroma_inspector.jsx` (input controls palette, the W10 port source), `cue_curve.jsx`
  (cubic bezier transition curve, the W13 PR2 port source), `hotkeys_data.jsx`
  (canonical 57-row hotkey vocabulary with the `cap` entitlement field, mirrored in
  `src/lib/data/hotkeys.ts` for the `?`-toggle overlay),
  `empty_states.jsx` (per-view copy), `v02_supplement.jsx` (ChromaCtx + AdapterPill /
  DegradedBanner / OutputRoutingTile / WatermarkOverlay / LockedRow / CrashResumeBanner
  chrome), `extras.jsx` (ThemeSwitcher / FavslotsBar / SnapBar / ControlHotkeysHint),
  `components.jsx` (Icon / SceneStill / Preview / Spectrum primitives), `shaders.jsx`
  (GLSL shader demo patterns), `screenshot_modal.jsx`, plus `views/_data.jsx` fixture
  corpus, `styles.css` with all three themes (Pulse / Atelier / Console), `lib/` and
  `styles/` subdirs, and provenance markers (`BUNDLE-PROVENANCE-v0.2-beta.md`,
  `HANDOFF.md`, `DESIGN.md`, `FONTS.md`). The older `ui-predev-bundle/` (v0.1-alpha)
  sibling exists for historical comparison; the v0.2-beta bundle is canonical.
- **`chromaticity-planning/ui-predev-bundle/screenshots/`** — raw wireframe captures
  (timestamp-named PNGs from 2026-04-22). The W9 port pass copied them to
  `.playwright-mcp/wireframe-reference/` under descriptive filenames
  (`01-library-scene-grid`, `02-author-scene-editor`, `03-playlist-cue-table`,
  `04-media-library`, `05-mappings-midi-pads`, `06-osc-link-settings`,
  `07-import-shadertoy-isf`) plus an index `README.md` for easier reference during port
  work — that's the directory to read first, not the timestamped originals.

When scoping a UI/UX wave or PR, read `.playwright-mcp/wireframe-reference/README.md`
first, then the relevant numbered screenshot, then the corresponding bundle JSX.
Wireframe-faithful ports are the default; divergences need explicit AO approval. The
bundle JSX is portable as a structural reference — React patterns (Context, hooks)
translate to Svelte 5 idioms (stores, runes); the bezier math, hotkey vocabulary, OSC
route taxonomy, and empty-state copy are pure data and port nearly directly. Per the
auto-memory `feedback_wireframes_normative_for_ui_ux.md`, this is a load-bearing
principle, not a stylistic preference — failing to consult the wireframes leads to UX
work that needs to be redone when shell architecture lands (avoided by W12) or when
entitlement infrastructure gates components (deferred for W14+).

UI ports already in production: W6 ThemeSwitcher (3-theme cascade); W8 widget primitives
(Fader / Knob / Toggle / Swatch / ControlGroup) + HotkeyOverlay + EmptyState; W9 Library
backbone; W10 Inputs Inspector; W11 ErrorMessage primitive + Vitest scaffold + Tauri
ship-time bundling; W12 app shell (AppShell + 13-icon SidebarRail + StatusBar +
RouteStore + Perform / Library / Author / Settings views); W13 CueTransitionCurve widget

- Playlist v2 view (first un-disabled bundle-port routed view in the W12 shell). W14-W34
  waves shipped MIDI view, OSC view, Audio view + canvas Spectrum / BpmRing / BeatDots
  visualizers, Author full port (with CodeMirror integration), and the Adapter UI suite
  (AdapterPill + DegradedBanner + OutputRoutingTile + LockedRow + WatermarkOverlay +
  CrashResumeBanner) wired via the `capabilitiesStore` that replaced the bundle's
  React `ChromaCtx`. The store is currently fixture-seeded from the W33b
  `LICENSE_FIXTURE`'s capability matrix per `src/lib/stores/capabilities.svelte.ts`
  lines 19-28; the live `EntitlementBitmask` IPC swap-in is Phase-3+/4a
  `EV-ENTITLEMENT-ENFORCEMENT` scope, so chrome elements render as if a fully-
  activated license were installed until that lands. **Wireframe-faithful chrome
  ported, then engine-wired (audio).** The BpmRing / BeatDots / AudioReactivity
  widgets retrofitted to dual-mode in Wave 1c-α (PR `#340`): live snapshot drives
  visuals when present, synthetic time-domain fallback when absent (per the
  "live snapshot present" gate in each widget). Wave 1c-α-2 (PRs `#343` + `#344` +
  this Bootstrap PR 3) closed the production wiring loop — `engine/audio`
  `AudioRuntime` spawns the producer thread; `app/shell` `TauriIpcEmitter` is the
  Tauri concrete `IpcEmit` adapter; `app/shell/src/lib.rs::run` setup spawns the
  drift-corrected 60 Hz scheduler-tick thread that pulls from `MutexAudioPull` and
  emits via `TauriIpcEmitter`. **Live IPC dispatch in production**: visualizers now
  receive live audio snapshots end-to-end via `app.emit('audio_uniform_snapshot_changed', ...)`,
  with the 4 named band tiles + BPM strip in `AudioReactivity` reading the live
  snapshot via the documented 8 → 4 reduction; the synthetic fallback is retained
  for the no-snapshot startup window only. Spectrum (a standalone widget that
  doesn't exist — the bundle's Spectrum was inlined into
  `src/lib/views/AudioUniforms.svelte` with 64 synthetic bars) and the
  `AudioReactivity` 48-bar FFT visualiser remain synthetic-driven as post-MVP UX
  enhancements. `engine/render::audio_adapter` continues to marshal the producer-
  side snapshot to GPU uniforms feature-gated under `audio-binding`.

## Where authoritative answers live

- **Dev standards**: `docs/43-development-standards.md`.
- **Evidence registry**: `docs/26-evidence-artifact-registry.md`.
- **ADRs**: `docs/adr/` (ADR-018 for UI stack, ADR-014 for release signing, ADR-013 for
  network defaults, ADR-011 for rendering backend).
- **Phase-0 placeholder inventory**: `docs/00-phase-0-placeholder-inventory.md`.
- **Risk register**: `docs/13-risk-register.md`.
- **Clean-room / legal baseline**: `docs/29-cleanroom-legal-policy-baseline.md`.
- **Scope-vocabulary source of truth**: `commitlint.config.cjs`.
- **CI tree**: `.github/workflows/` (rust, frontend, sbom, evidence-tag-lint, policy).

## When this file is wrong

Do not trust memory over the tree. If a command, path, or invariant here disagrees with
the code or git log, the tree wins — edit this file in a follow-up `docs(repo)` commit.
Stale project memory is worse than none.
