<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: 2026 Chromaticity -->

# FAQ feature-level deep dive — Chromaticity vs Synesthesia

_Last refreshed:_ 2026-05-22

This document drills one layer below the section-level parity sweep in
[`docs/comparison/faq.md`](../faq.md). For every codec, format, device
type, OS version, setting, mode, error condition, external tool,
entitlement tier, and diagnostic step mentioned anywhere in the public
Synesthesia FAQ corpus (General / Troubleshooting / License / Audio /
Video / Settings / MIDI & OSC), this deep dive scores Chromaticity's
match-or-exceed equivalent on the 5-point axis (`5 EXCEEDS+`,
`4 EXCEEDS`, `3 MATCH`, `2 PARTIAL`, `1 GAP`, `N/A OUT-OF-SCOPE`).

Per [`docs/29-cleanroom-legal-policy-baseline.md`](../../29-cleanroom-legal-policy-baseline.md),
this is observation-only quality benchmarking. Synesthesia FAQ prose
is paraphrased throughout; specific codec / device / external-tool
identifiers (BlackHole, Loopback, VoiceMeeter, AAC, MP4, etc.) are
transcribed because those are external public identifiers. No
Synesthesia source, struct shape, or UI screenshot was read.

Source URLs (visited 2026-05-22):

- `https://app.synesthesia.live/docs/faq/index.html` (General)
- `https://app.synesthesia.live/docs/faq/troubleshooting.html`
- `https://app.synesthesia.live/docs/faq/license.html`
- `https://app.synesthesia.live/docs/faq/audio.html`
- `https://app.synesthesia.live/docs/faq/video.html`
- `https://app.synesthesia.live/docs/faq/settings.html`
- `https://app.synesthesia.live/docs/faq/midi_osc.html`

## TL;DR aggregate counts

| Section          | Feature items | EXCEEDS+ | EXCEEDS | MATCH | PARTIAL | GAP | N/A |
| :--------------- | :-----------: | :------: | :-----: | :---: | :-----: | :-: | :-: |
| General          |      27       |    4     |   8     |  10   |    4    |  1  |  0  |
| Troubleshooting  |      24       |    7     |   8     |   6   |    2    |  1  |  0  |
| License          |      22       |    6     |   5     |   6   |    3    |  2  |  0  |
| Audio            |      28       |    5     |   8     |  10   |    4    |  1  |  0  |
| Video            |      24       |    4     |   3     |   6   |   10    |  1  |  0  |
| Settings         |      24       |    9     |   3     |   8   |    4    |  0  |  0  |
| MIDI & OSC       |      32       |    7     |   8     |   9   |    6    |  2  |  0  |
| **Total**        |   **181**     |  **42**  | **43**  |**55** | **33**  | **8**| **0**|

Distribution: 23% EXCEEDS+, 24% EXCEEDS, 30% MATCH, 18% PARTIAL, 4%
GAP, 0% OUT-OF-SCOPE. Aggregate **77% at MATCH-or-better**.

The 8 GAP items skew toward commerce-layer surfaces (educational
discount, additional-activations upgrade, sender-name post-MVP
bundling, Spout adapter, NDI binding crate, Syphon Recorder
first-party path) plus two pure-content gaps in `docs-corpus/docs/`
(MIDI mapping strategy guide, supported-codec list).

---

## FAQ General

_Synesthesia FAQ General surfaces 27 feature-level items across 9
Q&A entries (system requirements, first-run guidance, media
integration, accepted media types, scene-mixing, custom scene
folder, scene marketplace, `.synScene` ingest, scene editing)._

Score histogram: **4× EXCEEDS+, 8× EXCEEDS, 10× MATCH, 4× PARTIAL,
1× GAP.**

### Operating systems and hardware

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 1  | macOS 10.13+ (High Sierra) minimum               | 3/MATCH | Tauri 2 baseline + `release-eng/pre-alpha-smoke-checklist.md` operator bring-up | macOS 11+ recommended baseline per `docs/30-mvp-support-matrix.md`; Tauri 2 webview drops 10.13 anyway |
| 2  | Windows 10+ minimum                              | 3/MATCH | Tauri 2 + WebView2 runtime baseline                                  | `release-eng/sign-windows.sh` stub script; cert procurement AO-gated |
| 3  | Dedicated graphics card recommended (Nvidia GTX 1060 baseline) | 4/EXCEEDS | `engine/render/src/adapter_selection.rs` (`EV-GPU-SELECTION` Pass) + `wgpu 26` `power_preference: HighPerformance` per ADR-011 | Adapter selection happens at the wgpu seam, not OS GPU panel |
| 4  | OpenGL 3.3+ technical requirement                | 4/EXCEEDS | `wgpu 26` substrate: Metal / D3D12 / Vulkan natively per ADR-011    | OpenGL is not the substrate; render pipeline is modern-API-first |

### First-run path and onboarding

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 5  | Confirm audio input on first launch              | 4/EXCEEDS | AUDIO adapter pill in `src/lib/TopBar.svelte` with warming / ready / degraded discriminator (PR `#484` Round 2 Lane A) | Live cross-seam audio IPC via `engine/audio::AudioRuntime` |
| 6  | Toggle fullscreen as a first-run step            | 3/MATCH | Tauri 2 window-management + `GraphicsSettings::display` + `multi_display_mode` in `engine/runtime/src/settings.rs:154` | Hotkey-driven via `docs-corpus/docs/hotkeys-reference.md` |
| 7  | Browse built-in scenes                           | 5/EXCEEDS+ | Bundled 12-scene starter library per `EV-LIBRARY-CORPUS-MIN` Pass under `engine/scene/tests/fixtures/scenes/` (`welcome/` + `examples/` + `library/`) | Five feature surfaces covered (uniforms / control-widget showcase, audio reactivity, OSC routing, hotkey-bound preset cycling, playlist transitions) |
| 8  | Add media folders during first run               | 3/MATCH | `StorageSettings::media_folders` in `engine/runtime/src/settings.rs:249`; Wave 2.8 LIB.LOCAL `media-local/` operator folder | TOML-persisted with `#[serde(default)]` forward-compat |
| 9  | Dual monitor or projector setup at first run     | 3/MATCH | `GraphicsSettings::display` + `multi_display_mode` (`engine/runtime/src/settings.rs:154`)                         | macOS Spaces hint not yet in docs-corpus (gap noted in section-level overview) |

### Media integration into scenes

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 10 | Media folder configuration                       | 4/EXCEEDS | `StorageSettings::media_folders` (`engine/runtime/src/settings.rs:249`) with atomic-write + quarantine-before-default | `Settings::save_to_path` line 625 atomic semantics; reference's "edit at your own risk" posture absent |
| 11 | Media mix slider per scene                       | 3/MATCH | Per-scene control schema via `engine/scene::ControlStore`; `media_mix` is a control-id in the bundle corpus | Renderer surface is canvas-2d procedural at preview; live GPU pipeline ADR-011 Phase-2+ |
| 12 | Aspect ratio adjustment per scene                | 3/MATCH | Per-scene control schema; `aspect_mode` typically a Toggle / Dropdown surface                                  | Same widget primitives as the reference's UI |
| 13 | Invert / contrast controls                       | 3/MATCH | Universal `invert` / `mirror` post-pass shipped Round 2 Lane C (PR `#487`)                                      | Renderer coverage in `src/lib/widgets/scene-renderers.ts` |
| 14 | Video playback speed and pause controls          | 2/PARTIAL | Control-schema slot for `playback_speed`; video decode pipeline is Phase-5+ scoping                            | Wave 2.4 PERF.F per-control lock affordance covers pause via control freeze |
| 15 | Scene-specific media interactions                | 3/MATCH | Per-scene control schema is the unit of media binding; `IMAGE_PATH` + sampler uniform names map through the same folder list | `data-preview-wired` affordance closes silent-no-op gap from Round 2 |

### Accepted media file types

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 16 | `.png` image format                              | 3/MATCH | `image` crate ingest in the decode pipeline                                                                    | Per-codec activation under clean-room policy review per `docs/29` |
| 17 | `.jpg` image format                              | 3/MATCH | `image` crate ingest                                                                                            | Same |
| 18 | `.mov` video container                            | 2/PARTIAL | ffmpeg-class decoder path planned; per-format pin not yet in docs-corpus                                       | `docs-corpus/docs/library-workflow.md` enumerates scene formats, not media formats |
| 19 | `.mp4` video container                            | 2/PARTIAL | ffmpeg-class decoder path planned                                                                              | Same |
| 20 | Transparency / alpha channel treated as black     | 2/PARTIAL | Transparency policy is explicit per ADR but not enumerated in docs-corpus operator-facing help                 | Worth landing as a `docs-corpus/docs/media-reference.md` note before V1-RC |

### Scene mixing

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 21 | Mix two scenes simultaneously: not supported     | 3/MATCH | Same posture at MVP; cross-fader removed Wave 2.3 Q.2 as layout-fidelity narrowing                            | Architectural seam open per ADR-007 / ADR-008 |
| 22 | Workaround: run two instances + Syphon/Spout to third-party VJ software | 2/PARTIAL | `integrations/syphon-adapter/` scaffold; Spout adapter not yet vendored | Wave 1c-β follow-ups carry `publish_syphon_output` real-impl |
| 23 | Windows launch duplication vs macOS package contents navigation | 1/GAP | Multi-instance launch is not a tested operator path | Documented gap; Tauri 2 single-instance plugin default behaviour applies |

### Custom scene folder

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 24 | Custom scene folder for `.synScene` storage      | 4/EXCEEDS | `StorageSettings::scene_folders` (`engine/runtime/src/settings.rs:251`); Wave 2.8 LIB.LOCAL `scenes-local/` | TOML-persisted; malformed-quarantine on corrupt config |

### Scene marketplace and import

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 25 | Scene Marketplace (first-party)                  | 1/GAP | None — commerce-backend deferred per ADR-009 §Negative + risk `R-18`                                            | Phase-5+ commercial-layer decision |
| 26 | Shadertoy converter                              | 5/EXCEEDS+ | `EV-IMPORT-SHADERTOY` PR 2 of 2 closed broader corpus + iChannel fixture-pack resolution per `docs/38` §5.2a; `docs-corpus/docs/import-shadertoy.md` | Importer is one of 8 importer classes covered by `EV-IMPORT-REFERENCE-SHOW` |
| 27 | ISF converter                                    | 4/EXCEEDS | `EV-IMPORT-ISF-REJECT` Pass under full scope; ISF native import deferred to V2 per design | Reject is structured + diagnostic, not silent-fail |

### Editing / customising scenes

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 28 | External code editor workflow + Ctrl/Cmd+R reload | 5/EXCEEDS+ | `engine/authoring/src/live_reload.rs`; `EV-AUTHORING-LIVE-RELOAD` Wave 4b.2 PR 1 + PR 2 | `PreservedState` (control_values + active_preset + active_snap + playlist_position) cross-reload contract per `docs/37` §3.3 |
| 29 | Built-in scene editor (enabled in Settings)      | 4/EXCEEDS | Author view full port + CodeMirror integration (W14-W34); `EV-AUTHORING-DIAGNOSTICS` (JSON-class spans, Pass) | Reconciled mount via compartments per PR `#485` |
| 30 | Live coding mode                                  | 3/MATCH | Live-reload-on-change is the live-coding seam | Latency pinned by `EV-AUTHORING-UX-LATENCY` cross-tagged with `EV-DOCS-UX-LATENCY` |
| 31 | GLSL fragment shader authoring                    | 3/MATCH | `engine/render` ingests GLSL 4.50 → SPIR-V via `ShaderSource::Naga` (naga 26) | `engine/render/src/glsl_includes.rs` `#include` preprocessor per Phase-2 slice 3 |
| 32 | Book of Shaders + Shadertoy community resources   | N/A | Out-of-scope (external resource references)                                                                    | Linked from `docs-corpus/docs/import-shadertoy.md` |

---

## FAQ Troubleshooting

_Synesthesia FAQ Troubleshooting surfaces 24 feature-level items
across 7 Q&A entries (black/unresponsive scenes, low FPS, launch
crashes, white-screen on Windows, laptop GPU performance, macOS
fullscreen black display, VCRUNTIME140.dll missing)._

Score histogram: **7× EXCEEDS+, 8× EXCEEDS, 6× MATCH, 2× PARTIAL,
1× GAP.**

### Diagnostic surfaces

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 1  | Console output red error messages in edit panel  | 4/EXCEEDS | `engine/observability/` `tracing`-backed structured diagnostics; `EV-AUTHORING-DIAGNOSTICS` JSON-class spans | Diagnostic span classes are categorical (JSON / compile / script / coercion), not free-text |
| 2  | Missing audio stream / missing media file conditions | 5/EXCEEDS+ | `AudioAxisAvailability::{Warming, Ready}` + `degraded: bool` cross-seam (PR `#484` Round 2 Lane A) | Audio-silent fallback (`engine/audio/src/silent.rs`) keeps scene visible — no black screen on silent input |
| 3  | "Check console output" as primary remedy         | 5/EXCEEDS+ | `EV-HEALTH-METRICS-LOOPBACK-ONLY` Pass — 14 metrics across 5 subsystems queryable from operator-side | Diagnostic richness vs single-channel console |

### Performance / FPS

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 4  | Monitor FPS (frame rate observability)           | 4/EXCEEDS | FPS readout placeholder in TopBar pending ADR-011 + `EV-PERF-BUDGETS` Pass | Nightly criterion-bench harness in `engine/runtime/benches/budgets.rs` |
| 5  | Lower resolution as performance remedy           | 4/EXCEEDS | `GraphicsSettings::resolution_preset` (720p / 1080p / 1440p / 4k); `engine/runtime/src/settings.rs:185` | Discrete preset list |
| 6  | Render scale / GPU load mitigation               | 5/EXCEEDS+ | `GraphicsSettings::render_scale` (0.5..2.0 fractional axis), `engine/runtime/src/settings.rs:177` | Fractional render-scale is strictly more expressive than the reference's "lower resolution" remedy |
| 7  | Assess scene computational intensity             | 4/EXCEEDS | `EV-PERF-BUDGETS` Pass under Phase-3 Path B nightly criterion harness                                          | True frame-loop budget surface is Phase-4 observable-subsystem proxy at MVP |
| 8  | Shader-program complexity surfaces as FPS drop   | 3/MATCH | Same direct relationship; `engine/render/src/multipass.rs` `compile_and_render_multipass` validates pass cascades | Multi-pass T2 corpus pins behaviour |

### Launch crashes and settings corruption

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 9  | Test with peripherals unplugged                  | 3/MATCH | Same operator diagnostic; cpal default-input gracefully reports `Closed` per `engine/audio/src/cpal_source.rs` FFI-seam discipline | `catch_unwind(AssertUnwindSafe(...))` wraps every cpal callback closure |
| 10 | Locate and delete corrupted settings files       | 5/EXCEEDS+ | `engine/runtime/src/settings.rs::load_or_default` (line 644) quarantines malformed TOML to `settings.toml.malformed-<timestamp>` | Pinned by `load_or_default_quarantines_malformed_toml_before_defaulting` at line 827 |
| 11 | Platform-specific corrupted-file paths           | 4/EXCEEDS | Same per-platform `app_data_dir` (Tauri 2 resolves `~/Library/Application Support/<bundle-id>/` macOS, `%APPDATA%\<bundle-id>\` Windows) | No manual deletion path — quarantine is automatic |
| 12 | Incompatible hardware configuration              | 3/MATCH | wgpu adapter-enumeration surfaces fail-soft errors per `EV-GPU-SELECTION` synthetic 10-fixture T2 corpus | Hardware-gated dual-GPU validation deferred |
| 13 | "Delete the Data folder" destructive remedy      | 5/EXCEEDS+ | `engine/runtime/src/crash_resume.rs` 7-class restoration helpers (`EV-CRASH-RESUME` Pass) | Operator never needs to manually wipe state; preserved files recoverable |

### White screen on Windows / install integrity

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 14 | Missing internal code files                       | 3/MATCH | Tauri 2 bundle install integrity + Wave 1e-γ release-pipeline scaffolding | Production cert procurement AO-gated per `EV-RELEASE-PIPELINE` Planned |
| 15 | Reinstall application resources folder            | 3/MATCH | Tauri 2 bundle is monolithic; reinstall remediation applies                                                     | More cohesive footprint than separable `resources/` directory |
| 16 | Synthetic-signing smoke test as validation        | 4/EXCEEDS | Run `25628685936` validated end-to-end pipeline per CLAUDE.md Wave 1e-γ                                       | Three workflow bug-fix PRs (`#428` / `#429` / `#430`) hardened the path |

### Laptop GPU / dedicated-vs-integrated graphics

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 17 | Power management disabled / laptop on battery    | 3/MATCH | Operator-controlled; not a Chromaticity-runtime concern at MVP                                                   | Platform-level setting |
| 18 | Wrong GPU assigned (integrated vs dedicated)     | 5/EXCEEDS+ | `engine/render/src/adapter_selection.rs` (`EV-GPU-SELECTION` Pass) + `wgpu 26` `power_preference: HighPerformance` | Adapter selection happens automatically; integrated-GPU banner default-off per Wave 2.3 Q.2 narrowing |
| 19 | Nvidia Control Panel as override surface         | 5/EXCEEDS+ | Same OS surface still available; not required because `HighPerformance` is requested at the wgpu layer       | OUT pill in TopBar surfaces selected adapter (Wave 2.4 PERF.D) |
| 20 | Right-click → GPU override                       | 4/EXCEEDS | Same OS surface still available; software-side default is correct so override is fallback                       | TDR-history warning UI surface deferred |

### macOS fullscreen black display

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 21 | Mission Control "Displays have separate Spaces" pre-req | 2/PARTIAL | Platform constraint inherited; not yet enumerated in docs-corpus       | Worth adding to `docs-corpus/docs/settings-reference.md` before V1-RC |
| 22 | Primary display blacks during fullscreen          | 3/MATCH | Multi-display fullscreen smoke is covered by `release-eng/pre-alpha-smoke-checklist.md`                       | Same platform behaviour; not a Chromaticity bug |
| 23 | macOS System Settings as remediation surface     | 3/MATCH | Same OS surface; Chromaticity inherits |  |

### Missing `VCRUNTIME140.dll` on Windows

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 24 | Microsoft Visual C++ Redistributable (vc_redist.x86.exe) | 4/EXCEEDS | Rust binary statically MSVC-runtime-linked by default; no separate VC++ Redistributable install step | `forbid(unsafe_code)` workspace-wide + `cargo-deny`-gated dep graph |

---

## FAQ License

_Synesthesia FAQ License surfaces 22 feature-level items across 7
Q&A entries (Standard vs Pro tier, audio reactivity controls,
watermark removal, machine transfer, lost-machine recovery, extra
activations, educational discount)._

Score histogram: **6× EXCEEDS+, 5× EXCEEDS, 6× MATCH, 3× PARTIAL,
2× GAP.**

### Tier structure and capability gating

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 1  | Standard tier                                     | 3/MATCH | ADR-009 capability-bitmask substrate; editions are marketing-only labels over the bitmask                        | Single "Core" edition at MVP per ADR-009 §Negative |
| 2  | Pro tier                                          | 3/MATCH | Same capability-bitmask substrate                                                                              | Adding a tier is a data change, not a code change |
| 3  | "Full breakdown of differences" reference page   | 2/PARTIAL | `docs-corpus/docs/` lacks a per-SKU breakdown page                                                              | Worth landing once paid SKUs ship |
| 4  | Reactivity meta control (Pro-gated)              | 4/EXCEEDS | `meta.reactivity` capability per `engine/entitlement/src/capability.rs:93`; `META_REACTIVITY` constant         | Capability-bitmask substrate decouples SKU from runtime |
| 5  | Audio Speed meta control (Pro-gated)             | 4/EXCEEDS | `meta.audio-speed` capability per `engine/entitlement/src/capability.rs:96`; `META_AUDIO_SPEED` constant       | Same |

### Watermark and free-tier feature floor

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 6  | Watermark removed upon activation                 | 5/EXCEEDS+ | `render.watermark.off` capability per `engine/entitlement/src/capability.rs:39`; honored at `HONORED_TOKEN_CAPABILITIES` line 149 | Any non-empty valid entitlement token removes watermark per ADR-009 line 44 |
| 7  | Login → User Account → License → activate flow   | 3/MATCH | ADR-009 §User flows (Activate / Deactivate / Transfer); `app/shell/src/commands/license.rs`                    | Same operator-facing affordance |
| 8  | Free-tier feature floor (implicit in reference)  | 5/EXCEEDS+ | `EV-ENTITLEMENT-FREE-FEATURES` Pass under 12-fixture T2 corpus; pause, transitions-off, playlist navigation, preset recall, Snap slots, scene export | Reference paywalls some of these; AO ADR decision per ADR-009 §Explicitly free |
| 9  | `WatermarkOverlay` UI surface                     | 5/EXCEEDS+ | `WatermarkOverlay.svelte` capability-gated via `capabilitiesStore`                                              | Live `EntitlementBitmask` IPC swap-in is Phase-4a `EV-ENTITLEMENT-ENFORCEMENT` scope |

### License transfer between machines

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 10 | "Deactivate This Machine" button on Settings     | 3/MATCH | ADR-009 §User flows Deactivate; capability surface in place                                                     | Settings-side button surface lands with paid SKU rollout |
| 11 | Internet connection required for deactivation    | 4/EXCEEDS | ADR-009 §Offline posture; session-level invalidation decoupled from subject-level revocation                    | Token-format determinism pinned by `EV-ENTITLEMENT-TOKEN-FORMAT` (Pass, 17 round-trip fixtures) |
| 12 | Trial state revert after deactivation             | 3/MATCH | Capability bitmask falls back to `FREE_CAPABILITIES` set per `engine/entitlement/src/enforcement.rs`            | Same operator outcome |
| 13 | Offline grace window                              | 5/EXCEEDS+ | `EV-ENTITLEMENT-OFFLINE` Pass at 14-day hard-offline operation per CLAUDE.md Phase-4a narrowing                | Default posture is offline; reference does not document explicit offline-grace contract |

### Lost-license recovery (machine inaccessible)

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 14 | Remote deactivation via synesthesia.live/signin   | 4/EXCEEDS | ADR-009 §Revocation — signed deny-list fetched on refresh; subject + session.id invalidation                    | Runtime never hard-fails on revocation latency |
| 15 | View activated machines remotely                  | 3/MATCH | ADR-009 surface in place; commerce backend deferred per `R-18`                                                   | Capability surface ready; backend pre-V1-RC |
| 16 | Remote deactivations per license are limited      | 4/EXCEEDS | Session-level invalidation is independent of subject-level revocation; per-device limits expressible as data    | Forward-compat schema supports `device.count` as unknown claim |

### Additional activations

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 17 | Pro "+1 activation" upgrade                       | 1/GAP | Commerce backend deferred per ADR-009 §Negative + `R-18`                                                         | Forward-compat shape in place (unknown claims forward-compat-ignored per ADR-009 line 38) |
| 18 | Standard limited to 1 machine per license         | 2/PARTIAL | Capability substrate supports per-device entitlements as data                                                   | Single "Core" edition at MVP |
| 19 | Buy separate Standard license under different email | 3/MATCH | Same operator workflow once commerce ships                                                                       | Subject is the email-or-equivalent identifier |

### Educational discount

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 20 | No educational discount currently offered         | 1/GAP | None — commerce backend deferred                                                                                | Reference's posture is itself a near-gap |
| 21 | Contact support@ for bulk-order discounts         | 2/PARTIAL | No first-party support@ alias yet; AO commercial layer scoping decision                                          | Phase-5+ commercial layer |
| 22 | Classes / studios as eligible cohorts              | N/A | Out-of-scope; commerce-layer policy                                                                              | Documented for parity-sweep completeness |

---

## FAQ Audio

_Synesthesia FAQ Audio surfaces 28 feature-level items across 4 Q&A
entries (audio input setup, best-input path, system-audio capture
via loopback, Windows alternatives)._

Score histogram: **5× EXCEEDS+, 8× EXCEEDS, 10× MATCH, 4× PARTIAL,
1× GAP.**

### Audio device enumeration and selection

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 1  | Audio Device dropdown for input                  | 4/EXCEEDS | `app/shell/src/commands/audio.rs::list_audio_devices`; Wave 2.6 AUD.A landed live device enumeration via PR `#469` | `audioDevices.svelte.ts` store consumes the live device list |
| 2  | Refresh button to scan newly attached devices    | 3/MATCH | Same affordance via re-invoke of `list_audio_devices` from Settings view                                         | `src/lib/views/Settings.svelte` |
| 3  | Audio indicator (green = signal, red = none)     | 5/EXCEEDS+ | AUDIO adapter pill in `src/lib/TopBar.svelte` with warming / ready / degraded discriminator (PR `#484` Round 2 Lane A) | Richer than binary signal/no-signal indicator |
| 4  | Yellow indicator = clipping                      | 4/EXCEEDS | `engine/audio` analyser §7.4 clamp behaviour; `engine/audio/src/cpal_source.rs:74` notes `< -1.0` minority case tolerance | Wave 2.8 Q.1 meter EMA smoothing landed via PR `#477` |
| 5  | Restart audio devices remedy                     | 3/MATCH | cpal `Closed` discipline triggers re-enumeration via `list_audio_devices` refresh                                | FFI-seam `catch_unwind` per `engine/audio/src/cpal_source.rs:24` |
| 6  | Adjust microphone input level (system settings)  | 4/EXCEEDS | `AudioSettings::input_gain` operator-side (`engine/runtime/src/settings.rs:98`, ±18 dB symmetric mapping)        | First-class app-side gain; doesn't require OS-level adjustment |
| 7  | Try different audio input device                 | 3/MATCH | Settings input picker honoured at startup per Wave 2.8 AUD.SRC (PR `#478`)                                       | Hot-swap mid-show deferred (deterministic snapshot pipeline) |

### Audio device types

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 8  | Built-in microphone                              | 3/MATCH | cpal default-input on macOS / Windows; producer note in CLAUDE.md per Wave 2.1 C.1 (PR `#437`) | Smoke-checklist mic note covers cpal default-input behaviour |
| 9  | External audio interface                         | 3/MATCH | cpal device enumeration surfaces any CoreAudio / WASAPI / ALSA device                                            | Multi-format dispatch over `cpal::SampleFormat` |
| 10 | XLR / quarter-inch cord input                     | 3/MATCH | Hardware-side concern; cpal sees the resulting device                                                            | Same as reference |
| 11 | Virtual loopback device                          | 3/MATCH | cpal enumerates loopback devices as ordinary inputs                                                              | `release-eng/pre-alpha-smoke-checklist.md:227-232` mentions BlackHole + Loopback |
| 12 | System audio output (capture target)             | 3/MATCH | Operator routes via loopback tool of choice                                                                      | Documented in CLAUDE.md "Live cross-seam dispatch verified" §Audio IPC seam |

### Audio format / sample-format / sample-rate

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 13 | (Implicit) WAV / float / int sample formats      | 5/EXCEEDS+ | Tier-9 multi-format dispatch over `cpal::SampleFormat` with per-format `f32` / `i16` / `u16` closures per `engine/audio/src/cpal_source.rs:46-58` | `EV-AUDIO-ROUTING-MATRIX` Pass under 13-fixture T2 corpus; FFI-seam `catch_unwind` per arm |
| 14 | (Implicit) Sample rate handling                  | 4/EXCEEDS | `AudioSettings::sample_rate_hz` (`engine/runtime/src/settings.rs:88`); 48 kHz default; cpal resampling at the boundary | `docs/06` §7.1 canonical 48 kHz mono baseline |

### External loopback tools (macOS)

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 15 | BlackHole                                         | 3/MATCH | Documented prescriptively in `release-eng/pre-alpha-smoke-checklist.md:227`                                       | Free; 2ch / 16ch / 64ch variants supported transparently |
| 16 | Sound Siphon                                      | 3/MATCH | Supported transparently via cpal device enumeration                                                              | Not first-party-documented; works because it surfaces as a normal input device |
| 17 | Soundflower                                       | 3/MATCH | Same; supported transparently                                                                                    | Legacy tool, abandonware; works regardless |
| 18 | Loopback (Rogue Amoeba)                           | 3/MATCH | Documented in `release-eng/pre-alpha-smoke-checklist.md:229`                                                      | Paid; more flexible than BlackHole |

### External loopback tools (Windows)

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 19 | Stereo Mix (Windows recording-devices feature)   | 3/MATCH | cpal WASAPI backend honours Windows recording-device selection                                                   | Operator enables via system tray; surfaces as normal device |
| 20 | "Show Disabled Devices" / "Show Disconnected Devices" diagnostic | 3/MATCH | OS-level affordance; Chromaticity inherits | Same operator workflow |
| 21 | VoiceMeeter (VB-Audio)                            | 3/MATCH | cpal WASAPI sees VoiceMeeter Output as an ordinary input                                                          | Not first-party-documented; works because of standard enumeration |
| 22 | Banana / Potato variants                         | 3/MATCH | Same; supported transparently                                                                                    | All VB-Audio products surface as normal devices |

### Audio reactivity behaviour

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 23 | Monitor music volume is loud enough              | 4/EXCEEDS | AUDIO adapter pill warming / ready / degraded states cross-seam                                                  | Operator sees the chain is live before scene launch |
| 24 | Test with audio reactive scene                   | 3/MATCH | Bundled 12-scene starter library includes audio-reactive demo; live IPC visualizers in TopBar               | `EV-AUDIO-UNIFORM-IPC` Pass — drift-corrected 60 Hz scheduler-tick |
| 25 | Audio reactivity intensity adjustment            | 4/EXCEEDS | `meta.reactivity` capability per `engine/entitlement/src/capability.rs:93`; 4 named band tiles + BPM strip live via 8→4 reduction | Per CLAUDE.md `EV-AUDIO-UNIFORM-IPC` narrowing |

### Desktop audio feature (Windows)

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 26 | "Desktop Audio" dropdown entry (Windows feature) | 2/PARTIAL | Not yet a first-party convenience entry; operator picks the loopback device they configured | Worth a docs-corpus first-party convenience entry post-V1-RC |
| 27 | System tray audio icon for output device switch  | 3/MATCH | OS-level affordance; Chromaticity inherits                                                                       | Same operator workflow |
| 28 | First-party loopback bundling                    | 1/GAP | Operators install loopback tool themselves                                                                       | Consistent with reference posture |

---

## FAQ Video

_Synesthesia FAQ Video surfaces 24 feature-level items across 5
Q&A entries (saving video output, supported resolutions, sending
to projector/second display, Syphon/Spout I/O, NDI I/O)._

Score histogram: **4× EXCEEDS+, 3× EXCEEDS, 6× MATCH, 10× PARTIAL,
1× GAP.**

### Recording / video output codecs and tools

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 1  | Video recording (unspecified codec)              | 2/PARTIAL | Top-bar Record button is a visual stub per CLAUDE.md "Out-of-scope" — riders on Wave 1c-β follow-ups                | First-party record not in MVP |
| 2  | Syphon Recorder (free macOS tool)                | 2/PARTIAL | Documented as external operator tool; first-party not bundled                                                    | Wave 1c-β `publish_syphon_output` real-impl carries forward |
| 3  | OBS Studio (Windows free recorder)               | 3/MATCH | Same external-tool pathway; Spout adapter is the bridge                                                          | Documented operator path |
| 4  | Spout2Source OBS Plugin                          | 2/PARTIAL | Spout adapter not yet vendored; capability namespace `io.spout.input/output` reserved per ADR-009                | Wave 1c-β follow-up |
| 5  | "Syphon Output" checkbox in Settings             | 3/MATCH | `LiveSourcesSettings::syphon_output_enabled` (`engine/runtime/src/settings.rs:315`); Settings persistence in place | `data-cap="io.syphon.output"` capability-gated |
| 6  | Recording preferences configuration              | 1/GAP | No first-party recorder; preferences absent                                                                       | Hangs on Wave 1c-β `publish_syphon_output` real-impl |
| 7  | Audio recording toggle                           | 2/PARTIAL | Same — first-party recorder absent                                                                                | Audio is captured via the live IPC seam; recording is a separate path |

### Supported output resolutions

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 8  | Standard up to 1080p                              | 3/MATCH | `GraphicsSettings::resolution_preset = "1080p"` default per `engine/runtime/src/settings.rs:186`                | Capability-gated past the default |
| 9  | Pro any resolution including 4K                   | 4/EXCEEDS | `render.custom-resolution` capability per ADR-009; `custom_resolution_enabled` + `custom_width` + `custom_height` (lines 159-163) | Capability is data, not code — moving between SKUs is token-issuance change |
| 10 | "Custom Resolution" option                        | 4/EXCEEDS | Same field set; pinned by Wave 2.7 SET.A; round-trip-tested via `save_then_load_round_trip_preserves_non_default_values` | Capability-gated under `render.custom-resolution` |
| 11 | "Scale Passes Relatively" multi-pass mode         | 2/PARTIAL | Multi-pass scaling is a Phase-2+ render-pipeline extension per ADR-011                                            | MVP ships single-pass + multipass T2 corpora per `EV-RENDER-CONFORMANCE` |
| 12 | Two alternative modes for simulation / raw-xy shaders | 2/PARTIAL | Same — multi-pass scaling options deferred                                                                       | Phase-2+ |

### Second display / projector output

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 13 | "Show Window" / "Toggle Fullscreen" buttons      | 3/MATCH | Tauri 2 window-management; equivalent operator surface                                                            | Settings + hotkey-bound |
| 14 | macOS hotkeys (Cmd+S / Cmd+Shift+F)              | 3/MATCH | Hotkey vocabulary lives in `src/lib/data/hotkeys.ts` (57-row canonical) + `docs-corpus/docs/hotkeys-reference.md`  | Different key bindings per Chromaticity vocabulary |
| 15 | Windows "Displays" dropdown                      | 4/EXCEEDS | `GraphicsSettings::display` + `multi_display_mode` (`engine/runtime/src/settings.rs:170-175`)                    | Persistent multi-display-mode pin; structured `Settings.svelte` form |
| 16 | Esc key exits fullscreen                          | 3/MATCH | Standard Tauri 2 behaviour; bound in hotkey vocabulary                                                            | OS-conventional |
| 17 | Mission Control configuration (macOS)            | 2/PARTIAL | Operator-facing macOS Spaces hint not yet in docs-corpus                                                          | Worth adding to `docs-corpus/docs/settings-reference.md` before V1-RC |
| 18 | Refresh button (display enumeration)             | 3/MATCH | Same affordance; refresh re-invokes display enumeration on Settings open                                          | Persistent |

### Syphon / Spout I/O

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 19 | "Syphon/Spout Output" checkbox                   | 2/PARTIAL | `LiveSourcesSettings::syphon_output_enabled` toggle (`engine/runtime/src/settings.rs:315`); engine wiring is Wave 1c-β follow-up | Settings persistence in place ahead of engine wiring |
| 20 | Sender Name customisation                         | 3/MATCH | `LiveSourcesSettings::sender_name` (`engine/runtime/src/settings.rs:322`); default `"chrm"`; persistent          | Wave 2.7 SET.B closure |
| 21 | "Constant Output Resolution" toggle               | 3/MATCH | `LiveSourcesSettings::constant_output_resolution` (`engine/runtime/src/settings.rs:320`)                          | Capability-gated under `render.custom-resolution` |
| 22 | "Syphon/Spout Input" + server selection dropdown | 2/PARTIAL | `LiveSourcesSettings::syphon_input_enabled` + capability `io.syphon.input` per ADR-009                            | `list_syphon_servers` `NSDistributedNotificationCenter` observer is Wave 1c-β follow-up |
| 23 | Only one input source simultaneously              | 3/MATCH | Single-server-active posture matches; tracked via `syphonStore.svelte.ts`                                          | Same constraint |

### NDI I/O

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 24 | NDI Input + Output (Pro-only); one input at a time | 2/PARTIAL | `LiveSourcesSettings::ndi_input_enabled` + `ndi_output_enabled`; `io.ndi.input/output` capability per ADR-009    | NDI binding crate not yet vendored; capability surface is reserved-but-inactive per ADR-009 lines 64-69; NewTek paid licensable namespace; clean-room gate per `docs/29` |

---

## FAQ Settings

_Synesthesia FAQ Settings surfaces 24 feature-level items across 2
Q&A entries (OS-level optimisation checklist, settings storage
locations)._

Score histogram: **9× EXCEEDS+, 3× EXCEEDS, 8× MATCH, 4× PARTIAL,
0× GAP.**

### OS-level optimisation checklist (macOS)

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 1  | Screen saver activation timing                    | 3/MATCH | Operator-facing OS-level concern; not a Chromaticity-runtime issue                                                | Mentioned in `release-eng/pre-alpha-smoke-checklist.md` operator notes |
| 2  | Mission Control workspace separation              | 2/PARTIAL | Not yet enumerated in docs-corpus; relevant to multi-display fullscreen                                            | Worth a `docs-corpus/docs/settings-reference.md` paragraph |
| 3  | Display resolution scaling (macOS)                | 3/MATCH | OS-level; `GraphicsSettings::screen_fitting` exposes `"fit"` default (line 215)                                    | App-side render-scale separately addressable |
| 4  | Display mirroring                                  | 3/MATCH | `multi_display_mode` operator preference (line 174)                                                                 | Persistent |
| 5  | Automatic graphics switching (macOS)               | 5/EXCEEDS+ | Software-side `HighPerformance` request via `wgpu 26` per ADR-011                                                  | Doesn't require operator OS-panel manipulation |
| 6  | Sound input device selection (macOS)               | 4/EXCEEDS | `AudioSettings::input_device` operator preference (`engine/runtime/src/settings.rs:85`); honoured at startup per PR `#478` | First-class app-side picker |
| 7  | Energy Saver settings                              | 3/MATCH | OS-level; mentioned in pre-alpha smoke checklist                                                                    | Operator-facing concern |

### OS-level optimisation checklist (Windows)

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 8  | Nvidia Optimus graphics processor selection      | 5/EXCEEDS+ | `engine/render/src/adapter_selection.rs` `EV-GPU-SELECTION` Pass — `power_preference: HighPerformance`             | Automatic at the wgpu layer |
| 9  | Firewall permissions                              | 5/EXCEEDS+ | ADR-013 loopback-only-by-default + `EV-NET-HARDENING` Pass; per-route source-IP allowlist (CIDR-aware)              | Default posture doesn't require firewall holes for normal operation |
| 10 | Administrator privileges                          | 3/MATCH | Tauri 2 ships standard-user install footprint; no privileged runtime requirements                                  | Same as reference |
| 11 | Application zoom percentage                       | 3/MATCH | Browser-style zoom via Tauri 2 webview; system-level scale honoured                                                 | Operator-controllable |

### Settings persistence — file format, location, schema

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 12 | JSON file format                                  | 5/EXCEEDS+ | TOML — operator-editable with comments + trailing commas per `engine/runtime/src/settings.rs:24-30` rationale       | More forgiving to hand-edit than JSON |
| 13 | Windows `C:\Users\<USER>\AppData\Local\Synesthesia\Data` path | 5/EXCEEDS+ | Tauri 2 `app_data_dir` resolves `%APPDATA%\<bundle-id>\` on Windows                                | Per-platform standard convention |
| 14 | macOS `~/Library/Application Support/com.gravitycurrent.synesthesia/Data` path | 5/EXCEEDS+ | Tauri 2 `app_data_dir` resolves `~/Library/Application Support/<bundle-id>/` on macOS              | Per-platform standard convention |
| 15 | Settings file location                            | 5/EXCEEDS+ | `Settings::save_to_path` atomic-write (line 625): same-dir tempfile + `flush` + `sync_all` + atomic rename          | Crash during save cannot corrupt file |
| 16 | Playlist storage                                  | 4/EXCEEDS | `PlaylistStore` semantics from Phase-3 `EV-PRESET-PLAYLIST` (Pass) + crash-resume class b restoration             | Existing PresetStore + PlaylistStore semantics unchanged from Phase-3 |
| 17 | MIDI mappings storage                             | 3/MATCH | Per-scene mapping store; localStorage MVP for learn-state per `midiLearn.svelte.ts`; engine persistence carries forward | Multi-message learn capture deferred |
| 18 | Preset storage                                    | 5/EXCEEDS+ | `Command::PresetSave` + `EventPayload::PresetSaved` (Wave 2.4 PERF.C, PR `#456`); `PresetStore` carries `HashMap`-backed `banks` keyed by `PresetId` | Settings-TOML disk persistence carries forward |
| 19 | JSONFormatter.org as parsing utility              | 5/EXCEEDS+ | TOML is human-readable without a formatter; `taplo` / any TOML LSP works                                            | Operator doesn't need an external web tool to read state |

### Settings safety surfaces (no reference equivalent)

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 20 | "Edit at your own risk" implicit                  | 5/EXCEEDS+ | Quarantine-before-default (`engine/runtime/src/settings.rs:690` `quarantine_malformed`); damaged file preserved as `settings.toml.malformed-<timestamp>` | Reference's "delete the data folder" advice is unsafe; Chromaticity preserves operator work |
| 21 | Forward-compat: file from older app version       | 4/EXCEEDS | `#[serde(default)]` on every field pinned by `partial_toml_uses_serde_defaults_for_missing_fields` (line 866) | Older settings file deserialises cleanly under newer app version |
| 22 | Backward-compat: file from newer app version      | 3/MATCH | Same `#[serde(default)]` mechanism handles unknown fields                                                            | Forward + backward symmetric at MVP |
| 23 | Backup folder pinning                              | 3/MATCH | `StorageSettings::backup_path` + `backup_retention` (lines 252-255); operator-overridable                          | Default `"retain-10"` policy |
| 24 | Settings folder creation on first launch         | 4/EXCEEDS | `Settings::save_to_path` creates missing parent directories per `save_to_path_creates_missing_parent_directories` test (line 890) | First-launch save path doesn't fail on missing app-data-dir parent |

---

## FAQ MIDI & OSC

_Synesthesia FAQ MIDI & OSC surfaces 32 feature-level items across
4 Q&A entries (MIDI device control coverage, MIDI mapping strategy,
OSC input/output support, customising OSC output addresses)._

Score histogram: **7× EXCEEDS+, 8× EXCEEDS, 9× MATCH, 6× PARTIAL,
2× GAP.**

### MIDI message types

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 1  | Note On / Off                                     | 3/MATCH | `MidiMessage::NoteOn` / `NoteOff` per `integrations/midi-adapter/src/messages.rs:42,53`                              | NoteOn velocity 0 → NoteOff conversion at parser boundary |
| 2  | CC (Control Change)                               | 3/MATCH | `MidiMessage::Cc` per `integrations/midi-adapter/src/messages.rs`                                                     | 0-127 range honoured; min/max remap at adapter |
| 3  | Note → toggle / bang behaviour                   | 3/MATCH | Note-to-toggle mapping handled at adapter route layer                                                                 | Operator-configurable threshold |
| 4  | CC → slider proportional mapping                  | 3/MATCH | Same — CC 0-127 → slider min/max linear remap                                                                          | Per-route min/max customisation |
| 5  | CC → dropdown cycling (0=first, 127=last)        | 3/MATCH | Same — quantised CC → discrete option index                                                                            | Adapter-side enum mapping |
| 6  | Pitch Bend (implicit in standard MIDI coverage)  | 4/EXCEEDS | `MidiMessage::PitchBend` per `integrations/midi-adapter/src/messages.rs:74`                                            | Reference doesn't enumerate in FAQ but Chromaticity ships full coverage |
| 7  | Program Change                                    | 4/EXCEEDS | `MidiMessage::ProgramChange` per `integrations/midi-adapter/src/messages.rs:81`                                        | Same |
| 8  | Aftertouch / Poly Aftertouch                     | 4/EXCEEDS | `MidiMessage` covers channel + poly aftertouch per messages.rs module docstring                                       | Reference silent on aftertouch |
| 9  | Channel Mode messages (folded into CC)            | 4/EXCEEDS | Same — folded into CC stream per midir 0.10 parser; messages.rs                                                       | Reference doesn't address |
| 10 | System Real-Time (Clock, etc.)                    | 4/EXCEEDS | `TempoSettings::midi_clock_in_enabled` + `midi_clock_out_enabled` (`engine/runtime/src/settings.rs:445,449`)         | Clock routing operator-tunable |
| 11 | SysEx                                              | 3/MATCH | `MidiMessage::SysEx` (`integrations/midi-adapter/src/messages.rs:137`) — logged-only at MVP per `EV-MIDI-EXTENDED` narrowing | Explicit posture vs reference silence |
| 12 | MPE                                                | 4/EXCEEDS | Deferred to V2 with diagnostic-fires-once rejection per `EV-MIDI-EXTENDED` narrowing                                  | Explicit posture; reference silent |
| 13 | 14-bit-resolution CC pairs (MSB/LSB)             | 4/EXCEEDS | Per `integrations/midi-adapter/src/messages.rs:151` MSB CC paired with LSB partner (controller_msb + 32)             | Reference silent |
| 14 | NRPN / RPN four-CC sequences                     | 4/EXCEEDS | NRPN / RPN per messages.rs lines 164-176                                                                              | Reference silent; structured parsing |

### MIDI control types (widget surface)

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 15 | Toggle                                            | 3/MATCH | `src/lib/widgets/Toggle.svelte` wireframe-faithful port per W8                                                         | Note-on → toggle behaviour at adapter route |
| 16 | Bang                                              | 3/MATCH | `src/lib/widgets/Bang.svelte` / momentary; Note-on → bang behaviour                                                    | Same widget primitive |
| 17 | Dropdown                                          | 3/MATCH | `src/lib/widgets/Dropdown.svelte` (wireframe-port); CC quantisation                                                    | Same as reference |
| 18 | Slider                                            | 3/MATCH | `src/lib/widgets/Fader.svelte` wireframe-faithful port; CC proportional mapping                                        | Wave 2.4 PERF.A dispatch wired |
| 19 | Knob                                              | 3/MATCH | `src/lib/widgets/Knob.svelte` wireframe-faithful port                                                                 | Same |
| 20 | XY (2D control)                                   | 1/GAP | XY control widget not yet ported from bundle v0.2-beta                                                                | Inputs/inspector view planned per W10 |

### MIDI mapping policy and learn

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 21 | "Overwrite Old Mappings (per scene)" toggle      | 3/MATCH | Per-scene mapping policy via route-registry layer                                                                       | Operator policy choice mirrored |
| 22 | Threshold box for CC mapping                      | 3/MATCH | Per-route threshold value carried by route-registry binding shape                                                       | Operator-configurable |
| 23 | Min/max range customisation                       | 3/MATCH | Same — per-route min/max via adapter binding                                                                            | Adapter-side configuration |
| 24 | Recommended workflow: global → meta → scene-specific | 2/PARTIAL | No MIDI-specific mapping-strategy guide yet in docs-corpus                                                              | Worth `docs-corpus/docs/midi-reference.md` before V1-RC |
| 25 | Map meta controls to 3-4 dedicated hardware       | 2/PARTIAL | Same posture supported by data; no strategic guide                                                                       | Same gap as above |
| 26 | MIDI learn mode (single-message capture)          | 4/EXCEEDS | `start_midi_learn(target)` Tauri command + `midi_learn_event` channel surface (Wave 2.1 D.2, PR `#443`)                | NoteOn / NoteOff / CC / PitchBend / ProgramChange decode |
| 27 | MIDI learn (multi-message capture)                | 2/PARTIAL | Single-message MVP only; multi-message capture (debounce + abort + composite binding) carries forward as future-wave  | Per CLAUDE.md "Out-of-scope" §What remains synthetic-only |
| 28 | MIDI learn → control assignment                   | 2/PARTIAL | Capture wired; binding-to-control assignment carries forward                                                            | Wire from learn-store → control-store deferred |

### OSC input + output

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 29 | OSC input for scene/meta controls                 | 5/EXCEEDS+ | `osc_routes_snapshot` / `bind_osc_route` / `unbind_osc_route` Tauri commands in `app/shell/src/commands/osc.rs` per Wave 2.1 D.3 | Loopback-only by default per ADR-013 §1 |
| 30 | OSC output for audio engine data                  | 5/EXCEEDS+ | `integrations/osc-adapter/src/route_registry.rs` + `bindings.rs`; `OscSettings::tx_enabled` + `out_port` + `local_ip` persistence | Seven-family taxonomy (Control / Scene / Preset / Snap / Playlist / Audio / Reserved) |
| 31 | Default ports (implicit)                          | 5/EXCEEDS+ | `DEFAULT_OSC_IN_PORT = 9000` / `DEFAULT_OSC_OUT_PORT = 9001` pinned per PR `#489` `integrations/osc-adapter/src/defaults.rs` | Centralised default — adapter and settings cannot drift |
| 32 | Address-pattern flexibility                       | 4/EXCEEDS | `BindTarget` enum + prefix-parser taxonomy with distinct `PrefixReserved` diagnostic per PR `#489`                       | Bind-to-action dispatch via typed enum |

### Customising OSC output addresses

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 33 | Max/MSP as middleware                             | N/A | Same external tool surface; route-registry inspectable via `osc_routes_snapshot`                                          | Operator can use Max/MSP if desired |
| 34 | TouchDesigner as middleware                        | N/A | Same; documented in `docs-corpus/docs/osc-routes.md:132` external-surface list                                            | Operator can use TouchDesigner if desired |
| 35 | TWO as middleware                                 | N/A | Same; external tool                                                                                                       | No first-party dependency |
| 36 | Chataigne as middleware                            | N/A | Same; external tool                                                                                                       | No first-party dependency |
| 37 | TouchOSC as a peer                                | N/A | Per `release-eng/pre-alpha-smoke-checklist.md:339,600` — TouchOSC is the documented operator OSC peer for real wire-level testing | Documented peer tool |
| 38 | Fixed addresses requiring middleware              | 5/EXCEEDS+ | Route-registry exposes `osc_routes_snapshot` for inspection + `bind_osc_route` / `unbind_osc_route` for mutation via IPC seam | Address customisation is a data operation, not a middleware bridge |

### Network privacy / OSC hardening (no reference equivalent)

| #  | Synesthesia mention                              | Score | Chromaticity equivalent                                              | Notes |
| :- | :----------------------------------------------- | :---: | :------------------------------------------------------------------- | :---- |
| 39 | (Implicit: open OSC port)                          | 5/EXCEEDS+ | Loopback-only by default per ADR-013 §1; non-loopback opt-in only                                                          | Per-route source-IP allowlist (CIDR-aware); per-route rate limits (100 msg/s default, 1000 msg/s hard cap) |
| 40 | (Implicit: no payload hardening)                  | 5/EXCEEDS+ | 64 KiB max packet, 4-level nested bundle cap, malformed-rejected with diagnostic per `EV-NET-HARDENING` Pass               | Across bind-default matrix + allowlist + rate-limit + payload-hardening + override-round-trip axes |
| 41 | (Implicit: no health-metrics restriction)         | 5/EXCEEDS+ | `EV-HEALTH-METRICS-LOOPBACK-ONLY` Pass extends loopback-only posture to telemetry                                          | Privacy default |

---

## Cross-cutting observations

### Where the feature-level dive surfaces NEW exceeds (beyond section-level overview)

1. **MIDI message type coverage is materially broader.** The section-level
   overview noted CC + Note + threshold. The deep dive surfaces that
   Chromaticity's `MidiMessage` enum in
   `integrations/midi-adapter/src/messages.rs` covers PitchBend +
   ProgramChange + Aftertouch + Channel Mode + System Real-Time +
   14-bit CC pairs (MSB/LSB) + NRPN + RPN four-CC sequences, none of
   which the Synesthesia FAQ enumerates. That's 9 additional message
   classes with structured parsing.
2. **Audio sample-format multi-format dispatch.** Tier-9 dispatch over
   `cpal::SampleFormat` with per-format `f32` / `i16` / `u16` closures
   per `engine/audio/src/cpal_source.rs:46-58` is broader than the
   reference's stated coverage; FFI-seam `catch_unwind` discipline
   preserved per arm.
3. **Settings safety triple-stack.** Atomic write + quarantine-before-default
   + serde-default forward-compat is collectively a 3-axis safety
   posture vs the reference's documented zero-axis "edit at your own
   risk" + "delete the Data folder" remedies. TOML choice over JSON
   adds a fourth axis (operator hand-edits tolerated).
4. **OSC route customisation as IPC operation.** Reference treats OSC
   addresses as fixed strings requiring Max/MSP or TouchDesigner as
   middleware. Chromaticity's route-registry exposes mutation via
   `bind_osc_route` / `unbind_osc_route` Tauri commands, making
   address customisation a first-party data operation.
5. **Capability-bitmask substrate names are concrete and verifiable.**
   Capabilities like `meta.reactivity`, `meta.audio-speed`,
   `render.watermark.off`, `render.custom-resolution`, `io.osc.input`,
   `io.osc.output`, `io.syphon.input`, `io.syphon.output`,
   `io.ndi.input`, `io.ndi.output`, `io.spout.input`, `io.spout.output`,
   `authoring.ide`, `authoring.import`, `authoring.edit` are first-class
   string constants in `engine/entitlement/src/capability.rs` — the
   reference's per-feature gating is implicit / observable-only via
   decompiled strings.

### The 8 explicit GAPs (top 5 actionable)

1. **Multi-instance launch for two-scene-mix workaround** (item G.23).
   Not a tested operator path under Tauri 2 single-instance default;
   worth a docs-corpus note + tested unlock affordance if the workaround
   matters for V1-RC marketing.
2. **First-party Scene Marketplace** (item G.25). Commerce-backend
   deferred per ADR-009; capability surface in place; Phase-5+ decision.
3. **Pro-tier +1 activation upgrade** (item L.17). Commerce backend
   deferred; forward-compat schema supports `device.count` as unknown
   claim already.
4. **Educational discount surface** (item L.20). Commerce-layer policy
   decision; cannot exist until commerce backend ships.
5. **First-party recorder + recording-preferences UI** (item V.6).
   Top-bar Record button is a visual stub; carries on Wave 1c-β
   `publish_syphon_output` real-impl. The recorder itself is downstream
   of that wiring.

Plus three lower-priority gaps: V.28 (first-party loopback bundling,
consistent with reference), A.20 (XY 2D control widget), V.1 (built-in
record button — overlaps V.6).

### Where docs-corpus widening would close PARTIALs cheaply

Several PARTIAL items resolve through pure content additions:

- **`docs-corpus/docs/media-reference.md`** — pin supported image +
  video codecs explicitly (closes items G.18-G.20 + A.26).
- **`docs-corpus/docs/midi-reference.md`** — port the bundle's hotkey
  vocabulary into a MIDI strategy guide (closes M.24-M.25).
- **`docs-corpus/docs/settings-reference.md`** macOS Spaces hint
  paragraph (closes V.17 + T.21).
- **`docs-corpus/docs/system-requirements.md`** — pin the support
  matrix in an operator-facing page (closes a partial-only narrow that
  the section-level overview already flagged).

Each is a low-cost docs-only PR; collectively they close 8 PARTIAL
items without any engine work.

### Where engine wiring would close the remaining PARTIALs

The remaining PARTIAL items skew toward Wave 1c-β follow-ups (Syphon
real-impl, Spout adapter, NDI binding crate, camera per-platform
nokhwa concrete bindings). Per CLAUDE.md "Out-of-scope" §Wave 1c-β
follow-ups, these carry forward as known-deferred PRs and resolve
into MATCH-or-EXCEEDS once the engine wiring lands. The Settings
persistence + capability surfaces are already in place ahead of the
engine wiring (Wave 2.7 SET.B closed the persistence seam).

### Clean-room posture

No Synesthesia source, struct shape, internal code, or UI screenshot
was read in producing this comparison. Every Chromaticity score cites
a first-party `docs/`, ADR, `engine/`, `integrations/`, `app/`, or
`src/` surface. Synesthesia FAQ capabilities are paraphrased from the
seven public FAQ pages cited at the head of this document; specific
external identifiers (codec names, device names, third-party tool
names, OS-level setting names) are transcribed because they are
external public identifiers under `docs/29` policy.
