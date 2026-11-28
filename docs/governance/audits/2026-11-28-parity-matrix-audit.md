<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Parity Coverage Matrix Audit — 2026-11-28

Audit of `docs/governance/52-parity-coverage-matrix.md` against the tree at
`feat/phase0-phase1-state-machines` HEAD `7413ba1`. The matrix itself was not
changed by this audit.

## Method and limits

- All 96 data rows were checked in five lanes (scene schema + controls;
  library + runtime state; integrations + observability; release-eng + DJ;
  broadcast/OBS). Each row's badge and every cited path, symbol, command,
  test and `EV-*` ID was compared with the code, including whether the
  feature is reachable in the running app (Tauri command registered *and*
  called from the frontend).
- Static reading only: nothing was built, run, or exercised on hardware.
  "Tested" means a test exists, not that it passed in this audit.
- Every status-changing finding and every code defect below was re-checked
  against the files after the lane reports came back.
  One lane claim failed that re-check and is excluded: recording does *not*
  depend on the Mix view (`SceneCanvas` pushes recorder frames from the
  Perform hero, output window and Author whenever Mix is not mounted).
- Mechanical checks (scripted): status vocabulary, badge/status agreement,
  `planned:<EV-ID>` existence in `26-evidence-artifact-registry.md`, summary
  tally.

## Results

| Verdict | Rows |
| --- | ---: |
| Accurate | 53 |
| Partly wrong (status right, notes or evidence wrong) | 20 |
| Stale reference (claim right, cited name/anchor wrong) | 10 |
| Overstated (claims more than the code does) | 8 |
| Understated (code exists that the row does not credit) | 5 |
| **Total** | **96** |

Mechanical checks: every status token is valid, every badge matches its
status, and every cited `planned:<EV-ID>` exists. The summary tally is stale:
it lists 45 implemented / 38 planned; the rows contain **47 / 36** (post-MVP 9
and rejected 4 match).

## Status corrections (badge is wrong today)

| Line | Row | Today | What the code does | Proposed |
| --- | --- | --- | --- | --- |
| L18 | Transition lifecycle | 🟢 | Engine orchestrator and GPU blend are tested, but the live engine only ever calls `commit_hard_cut()` (`app/shell/src/render.rs:754`); nothing calls the crossfade. | 🟡 `planned:gap:live-scene-crossfade` |
| L31 | Missing-`TYPE` coercion | 🟢 | Coercion only produces validator warnings; `load_scene` never validates, so a control without `TYPE` reaches the UI as `kind: null` and renders as a fader. | 🟡 `planned:gap:apply-cp01-coercion-on-load` (same caveat on L32) |
| L61 | Rollback | 🟢 | `state_migration.rs` is forward V1→V2 only; its own doc defers rollback to Phase 4+; the shell never uses the module. | 🟡 `planned:gap:state-rollback` |
| L69 | OSC adapter (… rate-limit, allowlist) | 🟢 | Bind/poll/validate work; the source-IP allowlist and rate limits are not enforced (`poll()` discards the source address). | 🟢 for bind/poll/validate; allowlist + rate limit 🟡 |
| L113 | Multi-layer GPU compositor | 🟢 | Live Mix compositing is Canvas2D with CPU keying; the wgpu `render_layer_stack` has no caller and `set_mix_layer_stack` is never invoked from the frontend. | Split: Canvas2D Mix compositor 🟢, GPU compositor 🟡 |
| L132 | Multi-bus broadcast audio mixer | 🟢 | The scheduler feeds the mixer a constant block of the RMS value on the master bus only; output is discarded; Mic/Media/Alerts get nothing; stream and recording bypass it. | 🟡 (engine library complete, not wired) |
| L145 | Rekordbox 7 PC MASTER OUT tap | 🟢 | The Core Audio tap captures **all** system audio (empty exclusion list); nothing targets Rekordbox; the profile note claiming it targets `com.pioneerdj.rekordboxdj` is false. | Retitle as a generic system-audio tap (🟢); Rekordbox-specific claim 🟡 |
| L127 | YouTube stream-in (audio-reactive part) | 🟢 | The "opt-in audio-reactive feed" is a `routeAudio` flag that only styles a chip; no audio path exists. | Drop the audio-reactive claim (video layer stays 🟢) |

Understated rows that should credit shipped code:

- **L71 Media / camera** — omits live scene media (`chroma_Media`, milestone
  2a). Split into "scene media input (webview-sourced)" 🟢 and "native camera
  capture" 🟡 (the nokhwa adapter's `enumerate()` returns an empty list).
- **L48 Reference show import / L60 Show bundles** — engine importers (8
  classes + drill) and the tar + SHA-256 bundle format exist with Pass EVs but
  have no IPC/UI; keep 🟡 for the user-facing feature, fix the notes.
- **L89 Crash payload redaction** — redactor and harness shipped (EV Pass) but
  no crash reporter uses them.
- **L133 Broadcast audio DSP** — gate, compressor and limiter exist in
  `engine/audio/src/mixer.rs` (library only).

## Code defects surfaced by the audit

These are behaviour problems, not documentation drift. All re-checked.

1. **OSC inbound throughput cap** — `OscAdapter::poll()` reads at most one
   datagram per 60 Hz scheduler tick, so inbound OSC is capped near 60
   packets/s and bursts queue in the socket buffer
   (`integrations/osc-adapter/src/adapter.rs:735`, caller `app/shell/src/lib.rs`).
2. **Replay buffer timing** — the ring holds 450 frames
   (`app/shell/src/commands/recorder.rs:29`); Mix pushes every 100 ms when not
   recording (~45 s, not the documented 15 s), and `save_replay_buffer` always
   encodes at 10 fps, so a replay saved during an active recording (20–30 fps
   pushes) plays back slow. Inferred from code, not observed.
3. **Unbounded metrics buffer** — the shell `InMemoryEmitter` receives one
   `RuntimeUptime` event per second and is never drained.
4. **Missing-`TYPE`, dropdown and bang controls render as faders** — control
   metadata (`PARAMS`, `LABELS`, `VALUES`, `DEF_COLOR`, `IS_META`) is dropped at
   `ControlSchemaPayload` (`app/ui-bridge/src/lib.rs:419`).
5. **Camera list always empty** — `NokhwaInputAdapter::enumerate()` is a
   scaffold returning no devices, so the Sources camera list is always empty
   (webview capture is unaffected).
6. **Inert controls** — `set_audio_threshold` is registered but never called
   (the Shift+T hotkey does nothing); the Open/Save `.chromaShow` hotkeys do
   nothing; YouTube's audio toggle does nothing; the output window always goes
   to the second monitor (`listOutputMonitors` is never called).
7. **Settings hardcodes a 4-channel input** — `Settings.svelte:322` for the
   Master/Cue channel-pair picker.

## Evidence-linkage problems

Rows marked 🟡 `planned:<EV-ID>` whose EV row is already **Pass** and does not
cover the gap (so the gap has no open tracker): L13 (`.chromaPack`), L17
(scene `IMAGES` binding), L30 (slider variants), L33 (dropdown options), L48,
L75 (Spout), L88 (loopback health endpoint), L89. Each should move to
`planned:gap:<…>` or to an open EV row (e.g. L13 → `EV-EXTENSION-OWNERSHIP`).
L102 (bundle hygiene) should cite `EV-BUNDLE-HYGIENE` per ADR-014 §7.
L62 (crash-resume) cites `EV-CRASH-RESUME`, which is still InProgress
(synthetic only); the note should say so. L90's analytics soak was due around
2026-05-20 and is still open; the registry's claim of a default-off telemetry
setting has no matching setting in code.

Release-eng divergence worth an ADR decision: ADR-014 specifies Sparkle 2
EdDSA, but the app registers `tauri-plugin-updater` (with a placeholder public
key), and the release scripts disagree with each other (L99).

## Reference hygiene

- Stale names: L15 `load_scene_from_path` (does not exist → `load_scene` /
  `load_scene_impl` + `GpuPreviewEngine::set_scene`); L39 test renamed to
  `all_26_bundled_scenes_compile_and_render_on_gpu`; L57 line numbers out of
  date; L41 cites `commands/thumbnail.rs`, which is test-only (real path:
  SceneStill renderer + `/scene-thumbs/*.jpg`, FIFO cache); L47 attributes live
  preview to `author.rs` (it is `compile_and_preview_shader` +
  `glslRepair.ts`); L56 playlists persist only in webview localStorage.
- Requirement IDs: `03-product-requirements.md` stops at FR-22. FR-24 (L48,
  L61) does not exist; FR-14 is import adapters (L43/L44 mean FR-15); FR-6 is
  controls (L55/L56 mean FR-7); L54 cites CP-01 where CP-05 is meant.
- Missing anchors: ADR-013 and ADR-014 "§Rejected Alternatives" →
  "Alternatives Considered" (L78, L103); ADR-011 "§Syphon / Spout interop" →
  "4. Texture interop" (L74, L75); `06-audio-engine.md` §7 is "Normative
  Analysis Spec (MVP)" (L73); `31-…` "§Evidence Mapping" → "Evidence binding"
  (L85); `04-runtime-architecture.md` has no "§Command Flow" (L63).
- Bare spec filenames in the Reference column live in `docs/architecture/` or
  `docs/adr/`, not beside the matrix.
- Stale module doc: `engine/scene/src/loader.rs:14-16` still says `script.js`
  and `main.glsl` handling is deferred.

## Accurate rows

53 rows were confirmed as written, including all of release-eng signing,
notarization, SBOM and attestation (as planned), NDI/virtual camera/WHIP/SRT
and the other post-MVP and rejected rows, scene schema parsing, multipass,
script.js, library search, Shadertoy conversion, ISF rejection, MIDI, presets,
snaps, Studio Mode, Multiview, text overlays and the BPM AUTO/LOCK/OFF row.

## Suggested follow-up

1. Apply the status corrections and reference fixes to the matrix and
   recompute the summary tally (documentation only).
2. Triage the code defects; items 1, 2 and 3 are small, contained fixes.

## Resolution (2026-11-28)

Code defects fixed on `feat/phase0-phase1-state-machines`, each with a
regression test written to fail on the old behaviour:

| # | Defect | Fix | Commit |
| --- | --- | --- | --- |
| 1 | OSC inbound cap | `poll()` drains every queued datagram (bounded at 256 per call) | `4e82a52` |
| 2 | Replay timing | Ring trimmed to a true 15 s window; saves resampled to 30 fps from push timestamps (ffprobe: 3 s capture → 3.03 s file) | `b8f4e75` |
| 3 | Unbounded metrics buffer | `InMemoryEmitter::with_capacity_limit`; shell keeps 4096 events | `2b8011b` |
| 4 | Controls rendered as faders | CP-01 `effective_kind` and `LABELS`/`VALUES` reach the UI; Perform renders bang triggers and dropdown option rows | `83240ad` |
| 5 | Empty camera list | Sources lists the webview cameras live capture uses and selects one (native adapter still a scaffold) | `be0a70b` |
| 6 | Inert controls | Noise-gate threshold now gates live audio and Shift+T sets it (`7ea6b71`); 41 of 57 hotkey rows marked planned in the overlay and user reference, YouTube route-audio chip disabled, Settings lists real monitors and Go Live opens the output window on the chosen one (`556790b`) | `7ea6b71`, `556790b` |
| 7 | 4-channel assumption | Device enumeration reports each device's channel count; Settings offers only the pairs it has | `ed77c60` |

Also corrected: the Rekordbox profile note no longer claims the tap targets
Rekordbox (`bdeed72`).

Not fixed (feature work, now tracked as planned rows): live scene
crossfades, GPU compositor in the live Mix path, broadcast-mixer PCM wiring,
OSC allowlist and rate limiting, state rollback, Rekordbox-scoped capture,
native camera capture, Syphon publish, show-bundle and reference-import app
wiring, the 41 unwired hotkeys.

Matrix updated in the same change: every correction above applied, four rows
split (OSC, scene media vs native camera, Mix compositor, DJ tap), new tally
45 implemented / 42 planned / 9 post-MVP / 4 rejected over 100 rows. Every
planned row whose evidence entry was already Pass now points at an open gap.
Verified by script: status vocabulary, badge/status agreement, `planned:<EV-ID>`
existence, tally.

Still unverified: output-window placement on a real second monitor (single
display test host); real webcam capture through the Sources selection;
`EV-NET-HARDENING`'s registry text, which still claims allowlist and rate-limit
enforcement.
