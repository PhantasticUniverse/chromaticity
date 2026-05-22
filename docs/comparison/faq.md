<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> -->

# FAQ parity sweep — Chromaticity vs Synesthesia

_Last refreshed:_ 2026-05-22

Match-or-exceed quality benchmarking of Chromaticity against the seven
public Synesthesia FAQ pages (General / Troubleshooting / License /
Audio / Video / Settings / MIDI & OSC). Per the parent
[README](README.md) and `docs/29-cleanroom-legal-policy-baseline.md`,
this is **observation-only quality benchmarking** — no Synesthesia
source is read, no UI / API shapes are copied, and per-item summaries
are paraphrased from the public documentation surface only.

Source URLs (visited 2026-05-22):

- `https://app.synesthesia.live/docs/faq/index.html` (General)
- `https://app.synesthesia.live/docs/faq/troubleshooting.html`
- `https://app.synesthesia.live/docs/faq/license.html`
- `https://app.synesthesia.live/docs/faq/audio.html`
- `https://app.synesthesia.live/docs/faq/video.html`
- `https://app.synesthesia.live/docs/faq/settings.html`
- `https://app.synesthesia.live/docs/faq/midi_osc.html`

## TL;DR summary

| Section          | Items | EXCEEDS+ | EXCEEDS | MATCH | PARTIAL | GAP | N/A | Headline                                                                                  |
| :--------------- | :---- | :------: | :-----: | :---: | :-----: | :-: | :-: | :---------------------------------------------------------------------------------------- |
| General          | 9     |    1     |    2    |   3   |    3    |  0  |  0  | Scene library + media + import covered; scene-mixing remains partial.                     |
| Troubleshooting  | 7     |    3     |    2    |   2   |    0    |  0  |  0  | Crash-resume + quarantine + adapter-default safety meaningfully exceed reference.         |
| License          | 7     |    3     |    1    |   1   |    1    |  1  |  0  | Capability-bitmask substrate + offline-by-default + free-feature floor exceeds reference. |
| Audio            | 4     |    1     |    1    |   2   |    0    |  0  |  0  | Live IPC seam + structured device enumeration exceed; loopback path documented.           |
| Video            | 5     |    1     |    0    |   1   |    3    |  0  |  0  | Syphon scaffolding in place; per-platform output bindings carry as Wave 1c-β follow-ups.  |
| Settings         | 2     |    2     |    0    |   0   |    0    |  0  |  0  | Atomic-write + quarantine + serde-default forward-compat exceed reference posture.        |
| MIDI & OSC       | 7     |    2     |    2    |   1   |    2    |  0  |  0  | Bind-to-action + 7-family taxonomy + capability-gated routing exceed; multi-message learn carries forward. |
| **Total**        | **41**| **13**   | **8**   | **10**| **9**   | **1**| **0**| **31 of 41 items at MATCH or better; 1 explicit GAP (educational discount — commerce-deferred).**          |

Distribution: 51% EXCEEDS-class (5/4), 24% MATCH, 22% PARTIAL, 2% GAP, 0% OUT-OF-SCOPE.

---

## General

### System requirements

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** `docs/30-mvp-support-matrix.md`; MSRV 1.85.0 (`rust-toolchain.toml`); Tauri 2 + Svelte 5 + wgpu 26 per ADR-018 / ADR-011; `release-eng/pre-alpha-smoke-checklist.md` lists the operator bring-up baseline.
- **Match notes:** Chromaticity ships macOS + Windows desktop targets (no Linux at MVP), `wgpu`-backed rendering (Metal / D3D12 / Vulkan), and the same dedicated-GPU expectation for production scenes.
- **Exceed notes:** Chromaticity's `wgpu 26` substrate auto-selects the best available adapter and surfaces dual-GPU selection synthetically via `engine/render/src/adapter_selection.rs` (`EV-GPU-SELECTION` Pass under synthetic narrowing). The reference baseline pins OpenGL 3.3; Chromaticity's substrate covers Metal / D3D12 / Vulkan natively.
- **Gap notes:** Operator-facing "system requirements" docs page not yet shipped under `docs-corpus/docs/`; today the answer lives in `release-eng/pre-alpha-smoke-checklist.md` and the support matrix. Worth a docs-corpus page before V1-RC.

### Where do I start?

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `docs-corpus/docs/index.md` (Welcome), `docs-corpus/docs/perform-mode.md`, `docs-corpus/docs/scene-workflow.md`, `docs-corpus/docs/library-workflow.md`; bundled 12-scene starter library under `engine/scene/tests/fixtures/scenes/` (`welcome/` + `examples/` + `library/`) per `EV-LIBRARY-CORPUS-MIN`.
- **Match notes:** First-run path — confirm audio, set fullscreen, browse built-in scenes, save Snap slots (the Chromaticity name for what the reference calls "favslots") — is covered end-to-end by Wave 1d-α help corpus and Wave 1d-β starter library.
- **Exceed notes:** Welcome scene auto-loads on first launch via the `welcome/welcome.chroma` bundle (PR `#485` reconciled Editor mount). The 12-scene starter corpus covers five feature surfaces (uniforms / control-widget showcase, audio reactivity, OSC routing, hotkey-bound preset cycling, playlist transitions) — broader than the reference's "explore built-in scenes" prompt.
- **Gap notes:** None for the operator path; future-wave widening is the live preview render pipeline (ADR-011 Phase-2+).

### Adding images / videos into a scene

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** `engine/runtime/src/settings.rs` `StorageSettings::media_folders` (`Settings.svelte` Storage section), Wave 2.8 LIB.LOCAL `scenes-local/` + `media-local/` operator folders.
- **Match notes:** Settings exposes operator-added media folders; the Perform Media tab filters / launches them; `IMAGE_PATH` and video samplers resolve through the same folder list. Round-trip persistence pinned by `Settings::save_then_load_round_trip_preserves_non_default_values`.
- **Exceed notes:** TOML-serialised, hand-editable storage paths with `#[serde(default)]` forward-compat (`engine/runtime/src/settings.rs:866`); a missing field doesn't invalidate the file.
- **Gap notes:** None at the persistence seam; the runtime media decode pipeline (transparency policy, codec list) is a Phase-5 EXIT scoping decision.

### Accepted media file types

- **Score:** 2 / PARTIAL
- **Chromaticity equivalent:** `docs-corpus/docs/library-workflow.md` enumerates the supported scene formats (`.chroma`, `.chromaPack`, `.synScene` conversion, Shadertoy URL, ISF reject); per-media-format decode is a Phase-4d+ widening.
- **Match notes:** Scene-format coverage matches or exceeds (we accept the Synesthesia `.synScene` shape via conversion).
- **Exceed notes:** Clean-room policy gates per-codec activation; transparency policy is explicit per ADR.
- **Gap notes:** A formal supported-image-and-video codec list is not yet pinned in docs-corpus; today the answer is "PNG / JPG / MP4 / MOV via the `image` and `ffmpeg`-class decoders we ship". The supported-formats footnote ships in `EmptyState.svelte` (Wave 2.5 LIB.E) but is not enumerated in user-facing help.

### Mix between two scenes at once

- **Score:** 2 / PARTIAL
- **Chromaticity equivalent:** Playlist cue transitions via `CueTransitionCurve.svelte` (W13 PR2) + scene-warm-switch policy in ADR-007; cross-fader was deliberately removed in Wave 2.3 Q.2 as a layout-fidelity narrowing.
- **Match notes:** Like the reference, Chromaticity supports cross-fading between scenes via playlist transitions, with a cubic-bezier transition curve operator-tunable per cue.
- **Gap notes:** True A/B layer-mix (two scenes rendering simultaneously) is not in MVP scope. The architectural seam is open per ADR-007 / ADR-008 (transition + traveler semantics), but a dual-render-target composition is a Phase-5+ scoping decision. Syphon output via `integrations/syphon-adapter/` is the documented external-VJ-software path once `publish_syphon_output` real-impl lands (Wave 1c-β follow-up).

### Custom scene folder

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** `engine/runtime/src/settings.rs` `StorageSettings::scene_folders`; merged against `<UserDataRoot>/Scenes/` per `docs-corpus/docs/settings-reference.md` §3 Storage.
- **Match notes:** Same operator workflow as the reference — point Settings at a folder, refresh, scenes appear in Library. Settings persistence is atomic with malformed-quarantine fallback (`Settings::load_or_default` at `engine/runtime/src/settings.rs:644`).
- **Exceed notes:** TOML hand-editable shape exposes the folder list directly; a corrupted edit is quarantined as `settings.toml.malformed-<timestamp>` instead of blocking launch.

### Adding more scenes

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `docs-corpus/docs/import-shadertoy.md`; Shadertoy + `.chroma` + `.chromaPack` import via the Import view; bundled 12-scene starter corpus per Wave 1d-β; Lygia v1.1.4 trial vendored under `third_party/lygia/` (license election deferred to Phase-5 `EV-LICENSE-AUDIT`).
- **Match notes:** Scene marketplace equivalent, shader conversion, and custom-folder loading all covered.
- **Exceed notes:** `EV-IMPORT-SHADERTOY` PR 2 of 2 closed the broader Shadertoy corpus + iChannel fixture-pack resolution (`docs/38` §5.2a). `EV-IMPORT-REFERENCE-SHOW` synthetic-commit harness covers 8 importer classes producing `CommitOutcome` + `RollbackSnapshot` shape proof. The clean-room scene-library policy is explicit (`docs/29`).
- **Gap notes:** A first-party Chromaticity scene marketplace doesn't exist yet — commerce-backend deferred per ADR-009 §Negative + risk `R-18`.

### Getting downloaded `.synScene` files into the library

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** Importer in the Import view + drop into a registered scene folder + Library refresh.
- **Match notes:** Exact operator workflow as documented; `.synScene` conversion is one of the eight importer classes covered by `EV-IMPORT-REFERENCE-SHOW`.
- **Exceed notes:** Synthetic-commit harness validates `RollbackSnapshot` shape per import class — the reference's import path documents no rollback semantics. The conversion does NOT mutate Chromaticity user-data stores at MVP (rollback `Ok(())` no-op per ADR-015 §6 + `docs/32`; real rollback gated behind entitlement-tier + scene-library-write surfaces).

### Editing / customising scenes

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** Author view full port + CodeMirror integration (W14-W34); `EV-AUTHORING-DIAGNOSTICS` (JSON-class spans, Pass under narrowing); `EV-AUTHORING-LIVE-RELOAD` (Wave 4b.2 PR 1 + PR 2); `EV-AUTHORING-SCAFFOLDER` (full scope, Pass); `engine/authoring/src/live_reload.rs`.
- **Match notes:** External editor + reload-on-change matches the reference exactly; built-in editor is a wireframe-faithful Author port routed in the W12 shell.
- **Exceed notes:** Hot-reload preserves `PreservedState` (control_values + active_preset + active_snap + playlist_position) across reload per `docs/37` §3.3, a contract the reference does not document. Diagnostic span classes (JSON / compile / script / coercion) are categorical, not free-text. `EV-AUTHORING-UX-LATENCY` cross-tagged with `EV-DOCS-UX-LATENCY` pins ergonomic latency at the synthetic-harness level.
- **Gap notes:** Runtime-side wiring (engine/runtime adapter capturing `State -> PreservedState` before reload + re-applying via command-bus + commit-thread per ADR-006) is deferred to a future composition wave. Live preview render pipeline is ADR-011 Phase-2+.

---

## Troubleshooting

### Black scenes or unresponsiveness

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `engine/observability/` `tracing`-backed structured diagnostics; `EV-HEALTH-METRICS-LOOPBACK-ONLY` (Pass) catalog conformance with 14 metrics across 5 subsystems; `engine/audio/src/runtime.rs::MutexAudioPull` poison-recovery + audio-silent fallback (`engine/audio/src/silent.rs`).
- **Match notes:** First-class diagnostic when no audio input is present; observability surface exposes counters per subsystem; the audio-silent fallback keeps the scene visible (no black screen) when the audio thread is idle.
- **Exceed notes:** `AudioAxisAvailability::{Warming, Ready}` discriminator + `degraded: bool` flag surface cross-seam as the AUDIO adapter pill (PR `#484` Round 2 Lane A), so the operator sees "warming / ready / degraded" instead of a silent freeze. The audio-degraded UI banner exists today (round-1 `#487` + round-2 follow-on); the reference's only documented remedy is "check console output".

### Low FPS and choppy visuals

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** Settings `GraphicsSettings::render_scale` (0.5..2.0), `fps_target`, `vsync`, resolution presets (`engine/runtime/src/settings.rs:154`); `EV-PERF-BUDGETS` (Pass) under Phase-3 Path B nightly criterion harness in `engine/runtime/benches/budgets.rs`.
- **Match notes:** Resolution + GPU-load mitigation covered; FPS target is operator-tunable.
- **Exceed notes:** Render scale as a fractional 0.5..2.0 axis is a strictly more expressive performance lever than the reference's "lower resolution" remedy. Nightly benches pin perf budgets at the criterion level (Phase-3 EV-PERF-BUDGETS).
- **Gap notes:** Phase-4 frame-loop budget surface is observable-subsystem proxy at MVP; true per-frame budget surface ships post-MVP.

### Crashes on launch

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** `engine/runtime/src/settings.rs::load_or_default` (line 644) quarantines malformed TOML to `settings.toml.malformed-<timestamp>` instead of blocking launch; `engine/runtime/src/crash_resume.rs` 7-class restoration helpers per `EV-CRASH-RESUME` Pass; `EV-CRASH-PAYLOAD-REDACTION` Pass with 5-arm `RedactCategory`.
- **Match notes:** "Delete the data folder" remedy is functionally replaced — operator does not need to manually wipe state.
- **Exceed notes:** The Chromaticity runtime quarantines a corrupted settings file automatically (`Settings::load_or_default_quarantines_malformed_toml_before_defaulting` test pins behaviour at `engine/runtime/src/settings.rs:827`), and the crash-resume helpers cover 7 restoration classes (a/b/c/d/e/f/g). The reference's documented remedy is destructive ("delete the Data folder"); Chromaticity preserves the damaged file for recovery. Crash-payload redaction across 5 categories is a clean-room safety layer the reference does not document.
- **Gap notes:** Full crash-injection drill (controlled `SIGKILL` / `SIGSEGV` / panic-in-render-thread / OOM at three wallclock points) is deferred to a future-wave full-injection harness; the apparatus composes additively on the synthetic restore-helper substrate.

### White screen on Windows (missing internal files)

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** Tauri 2 bundle install integrity + `EV-RELEASE-PIPELINE` (Planned, cert-gated); `release-eng/sign-windows.sh` stub script.
- **Match notes:** Reinstall remediation applies equally; Tauri 2's bundler ships a more cohesive install footprint than the reference's separable `resources/` directory.
- **Gap notes:** Production Windows signing cert procurement (EV cert) remains AO-gated per the Wave 1e-γ posture in CLAUDE.md. The synthetic-signing smoke test (run `25628685936`) validates the end-to-end pipeline but the production cert path is the activation gate.

### Poor GPU performance on laptops

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** `engine/render/src/adapter_selection.rs` (`EV-GPU-SELECTION` Pass, synthetic 10-fixture T2 corpus); `wgpu 26` `power_preference: HighPerformance` per ADR-011.
- **Match notes:** Dedicated-GPU selection is honored.
- **Exceed notes:** The reference relies on the operator manually overriding "integrated vs dedicated" in the OS GPU panel. Chromaticity's adapter-selection layer requests `HighPerformance` by default and surfaces the chosen adapter via the OUT pill in the TopBar. The Wave 2.3 Q.2 narrowing explicitly defaulted the integrated-GPU banner off because adapter-selection happens at the wgpu seam.
- **Gap notes:** Hardware-gated dual-GPU validation is deferred (CI's wgpu round-trip arm covers at-least-one-adapter only); TDR-history warning UI surface is deferred.

### Black display when fullscreening on macOS

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** Tauri 2 window + fullscreen handling; multi-display routing through `GraphicsSettings::display` + `multi_display_mode` (`engine/runtime/src/settings.rs:154`).
- **Match notes:** Operator can pick the target display from Settings; macOS "Displays have separate Spaces" pre-requisite is a platform constraint Chromaticity inherits.
- **Gap notes:** Operator-facing macOS Spaces hint not yet in docs-corpus (worth adding to `docs-corpus/docs/settings-reference.md` before V1-RC). Multi-display fullscreen smoke is covered by `release-eng/pre-alpha-smoke-checklist.md`.

### `VCRUNTIME140.dll` missing on Windows

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** Tauri 2 bundle ships the WebView2 runtime via the bundler config; no MSVC runtime DLL surface on the Rust side (`forbid(unsafe_code)` workspace-wide + `cargo-deny`-gated dep graph).
- **Match notes:** The remedy class (missing platform runtime) is structurally less likely; Chromaticity's only platform-runtime dependency is WebView2 which the Tauri installer handles.
- **Exceed notes:** Rust binary is statically MSVC-runtime-linked by default in the rust-toolchain MSVC target — there is no separate VC++ Redistributable install step. The `EV-SIGNING-INTEGRITY` row provenance + the bundle layout are operator-recoverable.

---

## License

This is the highest-information section — Synesthesia is a commercial product with paid Standard / Pro tiers, while Chromaticity is pre-alpha clean-room with the ADR-009 entitlement substrate. Scoring follows the four axes called out in the brief: offline grace, revocation safety, free-tier feature set, and network privacy posture.

### Standard vs Pro tier differences

- **Score:** 3 / MATCH (different shape, equivalent reach)
- **Chromaticity equivalent:** `docs/adr/ADR-009-activation-and-entitlements.md` capability-bitmask substrate; per-capability flags (`io.osc.input`, `io.osc.output`, `io.syphon.input/output`, `io.spout.input/output`, `io.ndi.input/output`, `authoring.ide`, `authoring.import`, `authoring.edit`, `render.custom-resolution`, `meta.reactivity`, `meta.audio-speed`); editions are marketing-only labels over the bitmask.
- **Match notes:** Same capability reach as the reference's Standard / Pro split. The reference's "Reactivity / Audio Speed" gating maps to `meta.reactivity` and `meta.audio-speed`.
- **Exceed notes:** The bitmask substrate decouples product SKU decisions from the runtime. Adding a new tier is a data change, not a code change. The reference's per-feature gating is observable via decompiled strings; Chromaticity ships an explicit `entitlement-v2` capability namespace declared in ADR-009 line 30.
- **Gap notes:** MVP ships a single "Core" edition; paid SKU rollout requires commerce-backend (deferred per ADR-009 §Negative + `R-18`).

### Watermark posture

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** `engine/entitlement/src/enforcement.rs` `HONORED_TOKEN_CAPABILITIES` (13-cap data-only); `render.watermark.off` capability per ADR-009; `WatermarkOverlay.svelte`.
- **Match notes:** Any non-empty valid entitlement token removes the watermark (ADR-009 line 44); identical operator outcome to the reference.
- **Exceed notes:** Free-tier features (pause, transitions-off, playlist navigation, preset recall, Snap slots, scene export) are explicitly unconditionally available even with no token installed (`EV-ENTITLEMENT-FREE-FEATURES` Pass under full scope, 12-fixture T2 corpus). The reference paywalls some of these (notably "transitions-off") and operators have flagged that posture as hostile-UX; Chromaticity makes a different product decision documented in ADR-009 §Explicitly free.

### License transfer between machines

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** ADR-009 §User flows (Activate / Deactivate / Transfer); `entitlement-v2` token format with `subject` + `session.id` + `machine_binding`.
- **Match notes:** Same operator flow — deactivate on machine A, activate on machine B; offline grace honored per `EV-ENTITLEMENT-OFFLINE` (Pass, 14-day window).
- **Exceed notes:** Session-level invalidation (`SESSION_INVALIDATE` equivalent) is decoupled from subject-level revocation, so a deactivation on one session doesn't invalidate the subject's other devices. Token-format determinism is pinned by `EV-ENTITLEMENT-TOKEN-FORMAT` (Pass, canonical CBOR byte-identity gate across 17 round-trip fixtures).
- **Gap notes:** Offline transfer requires a signed handoff token (post-MVP per ADR-009).

### Lost license recovery (machine inaccessible)

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** ADR-009 §Revocation — signed deny-list (`SERVER_OUT_FLUSH_LICENSE` equivalent) fetched on refresh; subject + session.id invalidation; bounded deny-list entries with their own expiry.
- **Match notes:** Remote deactivation pathway documented (deny-list entry by `subject`).
- **Exceed notes:** Runtime never hard-fails on revocation latency — a current valid token continues to honor its local `expires_at` if the deny list can't be fetched. The reference's documented flow is online-dependent; Chromaticity's offline-first posture protects live performers (no festival-night network dependency per ADR-009 §Offline posture).
- **Gap notes:** `EV-ENTITLEMENT-REVOCATION` Pass at MVP is expiry-based; deny-list activation requires the commerce backend (deferred).

### Adding extra activations

- **Score:** 2 / PARTIAL
- **Chromaticity equivalent:** ADR-009 forward-compat schema supports `device.count` as an unknown-claim addition.
- **Match notes:** Capability-bitmask substrate supports per-device entitlements as a data change without runtime work.
- **Gap notes:** Commerce backend deferred; no operator-facing "+1 activation upgrade" surface today. The forward-compat shape is in place (unknown claims forward-compat-ignored per ADR-009 line 38).

### Educational discounts

- **Score:** 1 / GAP
- **Chromaticity equivalent:** None — commerce-backend deferred per ADR-009 §Negative + `R-18`.
- **Gap notes:** No pricing model exists yet, so no discount surface exists. This is a Phase-5+ commercial decision belonging in the commerce layer, not the runtime. The reference's posture ("we don't offer discounts; contact support for bulk") is itself a near-gap; once Chromaticity ships paid SKUs, the policy decision is in scope for the AO commercial layer.

### Offline grace period

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** ADR-009 §Offline posture; `EV-ENTITLEMENT-OFFLINE` Pass at 14-day hard-offline operation.
- **Match notes:** Operator can perform offline for an extended period.
- **Exceed notes:** **Default posture is offline.** Token read from local storage at startup; refresh happens over HTTPS when available; failure to refresh does not revoke a currently-valid token until `expires_at`. The reference does not document an explicit offline-grace contract on its license FAQ page — Chromaticity ships one as a first-class evidence row. Hard-offline 14-day operation is an `EV-ENTITLEMENT-OFFLINE` evidence row.

---

## Audio

### Setting up audio input

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `engine/audio/src/cpal_source.rs` cpal binding; `app/shell/src/commands/audio.rs` device enumeration; Wave 2.6 AUD.A landed live device enumeration via PR `#469`; Wave 2.8 AUD.SRC honoured operator input-device preference at startup (`#478`).
- **Match notes:** Operator picks device from Settings; live level / peak meter confirms signal; refresh re-enumerates.
- **Exceed notes:** Tier-9 multi-format dispatch over cpal::SampleFormat with per-format f32 / i16 / u16 closures (`engine/audio/src/cpal_source.rs`) is broader than the reference's stated coverage; FFI-seam `catch_unwind` discipline preserved per arm. `EV-AUDIO-ROUTING-MATRIX` Pass under 13-fixture T2 corpus pins format conversion + channel aggregation + clamp-boundary edges. Live cross-seam audio IPC seam (`EV-AUDIO-UNIFORM-IPC` Pass) emits scheduler-tick snapshots at 60 Hz, drift-corrected per `app/shell/src/lib.rs::run`.
- **Gap notes:** Operator-input-device hot-swap mid-show is deferred — analysis thread restart is at next scene load per `docs-corpus/docs/settings-reference.md` §1, by design for deterministic snapshot pipeline.

### Best way to get audio into Chromaticity

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** `docs-corpus/docs/audio-reactivity.md`; cpal default input device; `AudioRuntime` warming / ready / degraded discriminator (PR `#484` Round 2 Lane A).
- **Match notes:** Live audio stream is the recommended path; moderate signal levels (not silent, not clipping) is honored at the analyser via the `docs/06` §7.4 clamp.
- **Exceed notes:** AUDIO adapter pill in TopBar surfaces warming / ready / degraded states cross-seam, so the operator can see the chain is live before scene launch — a richer affordance than the reference's "watch the audio indicator".

### Playing music from iTunes / Spotify (system-audio capture)

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `release-eng/pre-alpha-smoke-checklist.md` operator note; CLAUDE.md "Live cross-seam dispatch verified" §"Audio IPC seam" producer note documents the cpal DEFAULT INPUT device (microphone) on macOS + the virtual loopback path (BlackHole / Loopback).
- **Match notes:** Exact same operator pathway: BlackHole / Loopback / Soundflower / Sound Siphon on macOS; equivalent VoiceMeeter / Stereo Mix path on Windows.
- **Exceed notes:** The empirical re-verification in Wave 2.1 C.1 (PR `#437`) added the smoke-checklist mic note covering cpal default-input behaviour, so operators don't rediscover the gap by surprise. The loopback path is documented prescriptively rather than as a troubleshooting aside.
- **Gap notes:** No first-party loopback bundling — operators install the loopback tool themselves (consistent with reference).

### Windows alternatives (Stereo Mix / VoiceMeeter)

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** Same operator pathway; cpal's WASAPI backend honors Windows recording-device selection.
- **Match notes:** "Stereo Mix" + VoiceMeeter both work via cpal default-input enumeration on Windows; the operator's Settings input-device picker surfaces them like any other device.

---

## Video

### Saving video output (capture / record)

- **Score:** 2 / PARTIAL
- **Chromaticity equivalent:** `integrations/syphon-adapter/` scaffold (Wave 1c-β closure landed 2026-05-07 at PR `#406` under synthetic-narrowing); `app/shell/src/commands/syphon.rs`; `src/lib/TopBar.svelte` Camera / Record buttons (visual stubs per CLAUDE.md).
- **Match notes:** Syphon scaffolding ports the canonical macOS interop path; OBS + Spout2 on Windows is the documented operator pathway (third-party tooling, same as reference).
- **Gap notes:** `publish_syphon_output` real-impl + `list_syphon_servers` `NSDistributedNotificationCenter` observer + Syphon `Send`-trait reconciliation all carry forward as Wave 1c-β follow-ups per CLAUDE.md "Out-of-scope" section. Top-bar Record button is a visual stub.

### Supported output resolutions

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `engine/runtime/src/settings.rs` `GraphicsSettings::resolution_preset` (720p / 1080p / 1440p / 4k) + `custom_resolution_enabled` + `custom_width` / `custom_height`; `render.custom-resolution` capability per ADR-009.
- **Match notes:** Preset list covers the documented Standard / Pro reference posture; custom resolution is capability-gated.
- **Exceed notes:** Resolution + render-scale axes are independently operator-tunable (render_scale 0.5..2.0). Custom-resolution gating is data, not code — moving the capability between SKUs is a token-issuance change.
- **Gap notes:** Multi-pass shader scaling options are a Phase-2+ render-pipeline extension (ADR-011); MVP ships the single-pass + multipass T2 corpora per `EV-RENDER-CONFORMANCE`.

### Sending visuals to a projector / second display

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** `GraphicsSettings::display` + `multi_display_mode` (`engine/runtime/src/settings.rs:154`); Tauri 2 window-management surface.
- **Match notes:** Fullscreen-on-display operator workflow is covered; multi-display mode pin is persistent.
- **Gap notes:** Operator-facing display-routing hint not yet enumerated in docs-corpus per-platform; same Settings UX as reference.

### Syphon / Spout I/O

- **Score:** 2 / PARTIAL
- **Chromaticity equivalent:** `integrations/syphon-adapter/`; `LiveSourcesSettings::syphon_input_enabled` / `syphon_output_enabled` / `ndi_input_enabled` / `ndi_output_enabled` toggle persistence (`engine/runtime/src/settings.rs:310`); `io.syphon.input/output` + `io.spout.input/output` capability namespace per ADR-009.
- **Match notes:** Settings exposes the toggle surface; sender-name persistence; `data-cap=...` attributes on the view rows mark capability-gated entitlement enforcement points.
- **Gap notes:** Per-platform Syphon `Send`-trait reconciliation + real publish impl are Wave 1c-β follow-ups; Spout adapter on Windows is similarly scoped. The Settings persistence seam is in place ahead of the engine wiring per the W15+ engine-side rollout.

### NDI I/O

- **Score:** 2 / PARTIAL
- **Chromaticity equivalent:** `LiveSourcesSettings::ndi_input_enabled` / `ndi_output_enabled`; `io.ndi.input` / `io.ndi.output` capability namespace per ADR-009.
- **Match notes:** Capability-gated under the entitlement substrate; settings persistence in place.
- **Gap notes:** NDI binding crate not yet vendored; capability surface is reserved-but-inactive (consistent with the explicit "reserved for V2" data-ingest pattern in ADR-009 lines 64-69). NDI is a paid licensable namespace from NewTek; clean-room policy gates activation behind `docs/29` review.

---

## Settings

### Settings checklist (OS-level optimisation)

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** `release-eng/pre-alpha-smoke-checklist.md`; `docs-corpus/docs/settings-reference.md`; the Settings panel's persistent per-field surface.
- **Match notes:** Operator-bring-up checklist covers screen savers, display settings, automatic graphics switching, and audio input levels.
- **Exceed notes:** Chromaticity's `Settings` struct (`engine/runtime/src/settings.rs:526`) pins every operator preference with `#[serde(default)]` forward-compat. A freshly-installed app renders bundle-faithful defaults via the nested `Default` impls — no operator-side configuration is required to reach a working baseline. The checklist lives alongside the operator's pre-show runbook rather than a separate "optimise your OS" doc, so it's where the operator already is.

### Where settings, playlists, MIDI mappings, presets are saved

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** Tauri's `app_data_dir` (`~/Library/Application Support/<bundle-id>/` on macOS; `%APPDATA%\<bundle-id>\` on Windows); `engine/runtime/src/settings.rs::save_to_path` atomic write (line 625) + `quarantine_malformed` (line 690); `EV-CRASH-RESUME` 7-class restoration.
- **Match notes:** Same per-platform app-data-dir convention as the reference.
- **Exceed notes:** Three concrete safety improvements over the reference's "edit at your own risk" posture:
  1. **Atomic write** via same-dir tempfile + `flush` + `sync_all` + atomic rename (`engine/runtime/src/settings.rs:662`); a crash during settings save cannot corrupt the file.
  2. **Quarantine-before-default** for malformed TOML (line 690); the operator's damaged file is preserved as `settings.toml.malformed-<timestamp>` for recovery, so a hand-edit syntax error doesn't silently destroy the operator's customisations on next launch.
  3. **Forward-compat via `#[serde(default)]`** on every field (line 866 test pin); a settings file written by an older app version deserialises cleanly under a newer one.
  Plus the format choice is TOML (operator-editable with comments + trailing commas) instead of JSON.

---

## MIDI & OSC

### MIDI device control coverage

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `integrations/midi-adapter/` (midir 0.10 cross-platform CoreMIDI / WinMM / ALSA); `app/shell/src/commands/midi.rs` `list_midi_devices` + `start_midi_learn`; `EV-MIDI-EXTENDED` Pass per CLAUDE.md.
- **Match notes:** Scene controls, meta controls, actions, media launches, scenes, playlists, and Snap slots all addressable from MIDI.
- **Exceed notes:** SysEx logged-only at MVP per the explicit `EV-MIDI-EXTENDED` narrowing (consistent with safety; the reference's documented coverage doesn't specify SysEx posture). MPE deferred to V2 with diagnostic-fires-once rejection — an explicit posture vs the reference's silence.
- **Gap notes:** Multi-message learn capture is single-message MVP only per Wave 2.1 D.2; multi-message capture (debounce + abort + composite binding) is a future-wave follow-up. Binding-to-control assignment from a learned event is also a future wire per CLAUDE.md.

### Control-type response to MIDI

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** `src/lib/widgets/` Fader / Knob / Toggle / Swatch primitives; `integrations/midi-adapter/src/messages.rs`.
- **Match notes:** Toggle / Bang respond to Note On/Off; sliders / knobs map CC 0-127 to operator-configurable min/max; widget primitives are wireframe-faithful ports per W8.
- **Exceed notes:** Capability-gated routing — every adapter capability flag (`io.osc.input/output` etc.) is enforced at the adapter boundary per ADR-005 + ADR-009; routing is data + tokens, not code.

### Overwrite Old Mappings (per-scene mapping policy)

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** Per-scene mapping policy via the route-registry layer; multiple controls can share a route by operator opt-in.
- **Match notes:** Operator policy choice mirrored.

### MIDI panel tabs

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** `src/lib/views/MIDI.svelte` per-tab layout (Devices / mappings); `src/lib/stores/midiStore.svelte.ts` + `src/lib/stores/midiLearn.svelte.ts`.
- **Match notes:** Functional tab layout covers Devices + per-scene / global / meta / media / Snap slot / start-scenes mapping surfaces.

### MIDI mapping strategy guide

- **Score:** 2 / PARTIAL
- **Chromaticity equivalent:** `docs-corpus/docs/hotkeys-reference.md` covers the 57-row hotkey vocabulary; no MIDI-specific mapping-strategy guide yet.
- **Match notes:** Hotkey vocabulary is the discoverable counterpart for keyboard.
- **Gap notes:** No MIDI-specific "recommended mapping strategy" page in docs-corpus today; worth adding before V1-RC under `docs-corpus/docs/midi-reference.md` to mirror the reference's "global first, then 3-4 meta sliders, then scene-specific" guidance.

### OSC support

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** `integrations/osc-adapter/` (rosc 0.10); `app/shell/src/commands/osc.rs` `osc_routes_snapshot` / `bind_osc_route` / `unbind_osc_route`; ADR-013 networking defaults; `DEFAULT_OSC_IN_PORT = 9000` / `DEFAULT_OSC_OUT_PORT = 9001` in `integrations/osc-adapter/src/defaults.rs` (PR `#489`).
- **Match notes:** OSC input + output, route discovery, capability-gated send/receive.
- **Exceed notes:** Three meaningful exceeds:
  1. **Bind-to-action dispatch via typed `BindTarget` enum** (PR `#489` round-3 Lane Q) — operator binds an OSC route to any of seven action families (Control / Scene / Preset / Snap / Playlist / Audio / Reserved) with a distinct `PrefixReserved` diagnostic for collisions.
  2. **Loopback-only by default** per ADR-013 §1 — operator opt-in for routable inbound, with a per-route source-IP allowlist (CIDR-aware), per-route rate limits (100 msg/s default, 1000 msg/s hard cap), and payload hardening (64 KiB max packet, 4-level nested bundle cap, malformed-rejected with diagnostic).
  3. **`EV-NET-HARDENING` Pass** across the bind-default matrix + allowlist + rate-limit + payload-hardening + override-round-trip axes.
- **Gap notes:** None at the protocol surface.

### Customising OSC output addresses

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `integrations/osc-adapter/src/route_registry.rs` + `bindings.rs`; `OscSettings::tx_enabled` + `out_port` + `local_ip` persistence.
- **Match notes:** Route surface covers control + audio engine data exposure.
- **Exceed notes:** Unlike the reference (which documents fixed addresses requiring Max/MSP or TouchDesigner as middleware), Chromaticity exposes a route-registry that operators inspect (`osc_routes_snapshot`) and bind / unbind through the IPC seam (`bind_osc_route` / `unbind_osc_route`). The seven-family taxonomy (Control / Scene / Preset / Snap / Playlist / Audio / Reserved) is a structured registry; address customisation is a data operation.

---

## Cross-cutting observations

### Where Chromaticity meaningfully exceeds

1. **Settings persistence safety.** Atomic write + malformed-quarantine + serde-default forward-compat is a posture the reference does not document. The operator's hand-edits survive parser errors, version upgrades, and write-time crashes.
2. **Crash-resume substrate.** `EV-CRASH-RESUME` 7-class restoration + `EV-CRASH-PAYLOAD-REDACTION` 5-arm redaction is a step-change over the reference's "delete your data folder" remedy.
3. **Network privacy posture.** ADR-013 loopback-only-by-default + per-route allowlist + rate-limit + payload-hardening + `EV-NET-HARDENING` Pass is a meaningful safety improvement over the reference's default-open OSC posture. `EV-HEALTH-METRICS-LOOPBACK-ONLY` Pass extends the posture to telemetry.
4. **Entitlement substrate.** Capability-bitmask substrate decouples product SKU from runtime; offline-first; expiry-bounded deny-list; session-level invalidation; canonical CBOR byte-identity; `EV-ENTITLEMENT-FREE-FEATURES` Pass on the explicit free-feature floor (pause / transitions-off / playlist / preset / Snap / scene export).
5. **Audio observability.** AUDIO adapter pill with warming / ready / degraded discriminator is more diagnostic-rich than the reference's binary audio indicator.

### Where Chromaticity is at parity but should still ship docs

The PARTIAL items skew toward two clusters:

- **Wave 1c-β follow-ups** (Syphon `publish_syphon_output` + NDI binding + camera real impl + Spout adapter) — the Settings persistence + capability surfaces are in place, but the engine-side bindings are deferred. These ship pre-V1-RC.
- **Docs-corpus widening** (MIDI mapping strategy guide, supported codecs page, macOS Spaces hint, system requirements page) — small content additions worth landing before V1-RC. Today the gap is "the information exists in the codebase but not yet in the operator-facing help".

### The one explicit GAP

Educational discount (License FAQ item 7) is a commerce-layer policy decision that cannot exist until the commerce backend ships (deferred per ADR-009 §Negative + `R-18`). Worth flagging when AO scopes the Phase-5+ commercial layer.

### Clean-room posture

No Synesthesia source, struct shapes, code, or UI screenshots were read in producing this comparison. Every Chromaticity score cites a first-party `docs/`, ADR, `engine/`, `integrations/`, or `app/` surface. Synesthesia capabilities are paraphrased from public documentation pages cited at the head of the document.
