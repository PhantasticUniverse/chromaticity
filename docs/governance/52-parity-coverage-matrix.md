# Parity Coverage Matrix

This matrix is a read-only claim-hygiene companion to `26-evidence-artifact-registry.md`. It maps observed reference-behavior families to the current Chromaticity coverage posture so reviews can see what is implemented, what is planned under an evidence row, what is post-MVP, and what is intentionally rejected. Status vocabulary is `implemented`, `planned:<EV-ID>`, `post-MVP`, `intentionally-rejected`, and `n/a`; if no valid evidence row exists yet, use `planned:gap:<description>` so the gap can be triaged without inventing an EV-ID. Every `planned:<EV-ID>` row must cite an EV-ID that exists in `26-evidence-artifact-registry.md`; run `bash scripts/evidence-tag-lint.sh` after edits to keep the EV namespace valid. This matrix is not a gate. It is a coverage index; gate authority stays with `26-evidence-artifact-registry.md` for the release cumulative gate and `11-roadmap-phases.md` for phase-exit gates.

## Scene schema and package

| Feature | Status | Reference | Notes |
|---|---|---|---|
| `scene.json` deserialization | implemented | `02-synesthesia-contracts.md` §Observed root keys; `05-scene-system.md` §Loader Modes | `engine/scene/src/loader.rs` parses in-memory JSON into the canonical schema; this does not imply package-level loading. |
| `.chroma` directory entry point | implemented | `05-scene-system.md` §Scene Package Model; `38-scene-library-and-import-spec.md` §7 Packaging and UTI | `engine/scene/src/loader.rs::load_chroma_dir` validates the `.chroma` directory contract and deserializes `scene.json`; shader, script, asset, and zip-package interpretation remain deferred per the module's slice-boundary comment. |
| `.chromaPack` archive | planned:EV-SCENE-IMPORT | `22-compatibility-versioning-policy.md` §Package Extensions and File-Type Ownership Policy; `38-scene-library-and-import-spec.md` §7 Packaging and UTI | `EV-SCENE-IMPORT` closed at Phase-1 EXIT (W12 PR #121, commit `2b4bffe`) for the canonical-model conversion contract; zip-transport (`.chromaPack`) remains release-cumulative scope and gates this row independently. Extension ownership is separately validated by `EV-EXTENSION-OWNERSHIP`. |
| `script.js` interpretation | implemented | `02-synesthesia-contracts.md` §3 Script Lifecycle; `28-implementation-contract-pack.md` §CP-07 | `app/shell/src/script_host.rs` (commit `30ca792`) wires a sandboxed QuickJS script host for scene `script.js` lifecycle execution; full contract parity against `EV-SCRIPT-CONTRACT` remains tracked in `docs/26`. |
| `main.glsl` pairing | planned:EV-SCENE-IMPORT | `02-synesthesia-contracts.md` §Scene Package Contract; `05-scene-system.md` §Scene Package Model | `EV-SCENE-IMPORT` closed at Phase-1 EXIT (W12 PR #121, commit `2b4bffe`) for canonical-model coverage of the seven reactive-symbol fixtures; package-to-shader entry pairing is Phase-2 render-conformance scope (gated by `EV-RENDER-CONFORMANCE`) and remains deferred under this row. |
| Multipass lifecycle | implemented | `02-synesthesia-contracts.md` §2 Multipass; `34-golden-corpus-definition.md` §Corpus Size Targets | `engine/render/src/multipass.rs` + `multipass_pingpong.rs` implement multi-pass scheduling and zero-CPU-round-trip double-buffered GPU feedback (`EV-RENDER-CONFORMANCE` Pass). |
| Media/image loading | planned:EV-INTEGRATION-MATRIX | `02-synesthesia-contracts.md` §Observed root keys; `08-io-integrations.md` §Camera/Media Contract | Compositor local video/image layers (`Mix.svelte` + `liveMedia.svelte.ts`) and YouTube stream-in are shipped; scene-manifest `MEDIA` / `IMAGES` shader texture binding remains planned. |
| Transition lifecycle | implemented | `02-synesthesia-contracts.md` §15 Transitions; `28-implementation-contract-pack.md` §CP-09 | `engine/render/src/transitions.rs` (ADR-007 4-phase warm-switch) + `composition.rs` (GPU `mix()` blend pass) shipped (`EV-RENDER-CONFORMANCE` Pass). |
| Pass execution | implemented | `05-scene-system.md` §Canonical Internal Scene Model; `adr/ADR-011-rendering-backend.md` | Live `wgpu`/Metal pass execution via `engine/render/src/{single_pass,multipass,multipass_pingpong,compositor}.rs` and `app/shell/src/commands/render.rs::render_gpu_frame`. |
| `.synScene` conversion input | implemented | `22-compatibility-versioning-policy.md` §Reference-app extension handling; `38-scene-library-and-import-spec.md` §7 Packaging and UTI | `app/shell/src/commands/import.rs` (commits `30ca792`, `7216984`) implements non-destructive `.synScene -> .chroma` conversion IPC and UI import workflow; `EV-SCENE-IMPORT` closed at Phase-1 EXIT (W12 PR #121, commit `2b4bffe`) for reactive-symbol canonical-model coverage. |
| `.synScene` OS association / extension claiming | intentionally-rejected | `22-compatibility-versioning-policy.md` §Reference Extension Handling | Chromaticity must not claim, register, or re-register the reference-owned extension. |
| Control metadata preservation (`UI_GROUP`, `DESCRIPTION`, `PARAMS`, `DEF_COLOR`, `LABELS`, `VALUES`, `IS_META`, `LABEL`) | implemented | `02-synesthesia-contracts.md` §Observed control keys; `28-implementation-contract-pack.md` §CP-01 | `engine/scene/src/schema.rs::ControlDecl` preserves all eight fields as explicit `serde(rename = ...)` bindings; unknown keys flatten non-lossily through the `extensions` bag. |

## Control taxonomy

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Base kinds: `slider`, `toggle`, `color`, `xy` | implemented | `28-implementation-contract-pack.md` §CP-01; `02-synesthesia-contracts.md` §Observed control types | The parser accepts these base kinds and the validator exercises CP-01 fallback paths for their current shapes. |
| Base kinds: `bang`, `dropdown`, `knob` | implemented | `02-synesthesia-contracts.md` §Observed control types; `08-io-integrations.md` §OSC Discovery and Announcement Routes | `engine/scene/src/schema.rs::ControlKind` accepts all three first tokens; these kinds require explicit `TYPE` and do not trigger CP-01 coercion. Runtime semantics (bang edge, dropdown discrete-option exposure, knob scalar normalization) and UI / OSC wiring are tracked under their own rows. |
| Slider variants: `smooth`, `traveler`, `counter`, `speed` | planned:EV-RENDER-CONFORMANCE | `02-synesthesia-contracts.md` §Traveler Controls; `adr/ADR-008-transition-and-traveler-semantics.md` | First-token parsing accepts some compound spellings, but variant semantics are not preserved or executed. |
| Missing-`TYPE` compat coercion | implemented | `28-implementation-contract-pack.md` §CP-01; `34-golden-corpus-definition.md` §Coverage Dimensions | `EV-COMPAT-FIXTURE-COVERAGE` closed at W11 PR #118 (commit `3a3b09b`) — T1 validator unit tests + T2 corpus coverage via `engine/scene/tests/cp01_compat.rs` against the 5 CP-01 fallback fixtures (slider/toggle/color/xy/reject) under `engine/scene/tests/fixtures/scenes/cp01-*.chroma/` plus 10 compat-vs-strict tests. |
| CP-01 `DEF_COLOR` fallback | implemented | `28-implementation-contract-pack.md` §CP-01 | `ControlDecl.def_color` is preserved at the schema and `engine/scene/src/validator.rs::coerce_missing_type` consumes it under CP-01 step 3; the `compat_fixture_coverage` T1 `cp01_step3_def_color_coerces_in_compat_rejects_in_strict` test pins the compat-vs-strict branch. |
| Dropdown labels / values exposure | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §OSC Discovery and Announcement Routes; `02-synesthesia-contracts.md` §Observed control keys | `LABELS` / `VALUES` are preserved structurally on `ControlDecl`; OSC discovery routes and MIDI mapping surface remain planned under this row. |

## Library and import

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Bundled scenes under `Resources/scenes/` | planned:EV-LIBRARY-BUNDLE-LAYOUT | `38-scene-library-and-import-spec.md` §2.1 | W11 PR #120 (commit `a18ba20`) implements the bundled-resources path with merge-on-fetch semantics: three demo `.chroma` packages are bundled at link time via `bundle.resources` in `app/shell/tauri.conf.json` and surfaced via the `bundled_scene_root` / `list_bundled_scene_packages` IPC commands; cards carry `SceneSource::Bundled` so the Library view renders a BUILT-IN chip. Full Phase-4b sub-directory layout (`welcome / library / examples / shadertoy-fixtures`) per `38-scene-library-and-import-spec.md` §2.1 and the bundle-walk assertion remain Phase-4b scope. |
| User-folder scanning | implemented | `38-scene-library-and-import-spec.md` §2 | `app/shell/src/commands/library.rs` (`list_scene_packages` + `default_scene_root`); `src/lib/views/Library.svelte`; `src/lib/stores/library.ts`. |
| Thumbnails + fallback | implemented | `38-scene-library-and-import-spec.md` §3 | `app/shell/src/commands/render.rs::render_scene_thumbnail` renders live Metal GPU thumbnails into `SceneStill.svelte` (in-memory LRU cache; `commands/thumbnail.rs` retains the deterministic byte-hash fallback). |
| Search / filter / tag | implemented | `38-scene-library-and-import-spec.md` §2; §4 | `src/lib/views/Library.svelte` (client-side substring search across title/credit/description/tags, category chips, and size selector; Rust `commands/library_search.rs` also exists). |
| Drag-to-playlist | planned:EV-UX-LIBRARY-CONTRACT | `38-scene-library-and-import-spec.md` §2; `03-product-requirements.md` FR-14 | UX contract coverage exists as a planned gate; runtime playlist mutation is covered separately by `EV-PRESET-PLAYLIST`. |
| Preset launch from library | planned:EV-UX-LIBRARY-CONTRACT | `38-scene-library-and-import-spec.md` §2; `03-product-requirements.md` FR-6 / FR-14 | Launch-with-preset is a library UX and runtime state handshake; neither is complete. |
| Shadertoy conversion + diagnostics | implemented | `38-scene-library-and-import-spec.md` §5.1 | `app/shell/src/commands/import.rs` (commits `30ca792`, `7216984`) implements Shadertoy-to-`.chroma` conversion with actionable diagnostics and UI import wiring. |
| V1 ISF rejection | implemented | `38-scene-library-and-import-spec.md` §6 ISF Import | `app/shell/src/commands/import.rs` (commits `30ca792`, `7216984`) enforces V1 ISF reject-with-diagnostic policy rather than native import. |
| Author workspace surface | implemented | `37-authoring-environment-spec.md` §3 Authoring Environment Components | `src/lib/views/Author.svelte` and `app/shell/src/commands/author.rs` deliver the live Author IDE surface with real-time `naga`/`wgpu` preview, auto-repair diagnostics, and `.chroma` save workflow. |
| Reference show / user-data import | planned:EV-IMPORT-REFERENCE-SHOW | `03-product-requirements.md` FR-24; `adr/ADR-015-reference-app-user-data-import.md` | Presets, mappings, playlists, favslots, and settings import are not implemented yet. |

## Runtime state

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Controls (per-node control values) | implemented | `04-runtime-architecture.md` §Core Components; `28-implementation-contract-pack.md` §CP-01 | `engine/runtime/src/state.rs::State::control_value` reads the post-commit value; `app/shell/src/commands/control.rs` exposes the `set_control` / `get_control` Tauri round-trip (PR #94). |
| Presets (snapshot + recall) | implemented | `03-product-requirements.md` FR-6; `07-control-state-presets.md` | `app/shell/src/commands/preset.rs` (`save_preset` / `recall_preset` / `active_preset`) + `src/lib/stores/preset.svelte.ts` (`EV-PRESET-PLAYLIST` Pass; TOML disk persistence remains a follow-up). |
| Playlists (advance / rewind / bounds) | implemented | `03-product-requirements.md` FR-6; `08-io-integrations.md` §OSC Conformance Test IDs | `src/lib/stores/playlist.svelte.ts` implements timed cue playback with auto-advance, loop, scene loading via `loadedSceneStore`, advance/rewind, and persistence alongside `app/shell/src/commands/playlist.rs`. |
| Snap / favslots (16-slot surface) | implemented | `04-runtime-architecture.md` §Core Components; `38-scene-library-and-import-spec.md` §8 Evidence | `engine/runtime/src/command_bus.rs` defines `Command::SnapRecall` / `SnapSave` / `SnapClear` and their `EventPayload` mirrors (lines 155-159, 185-193); `engine/runtime/src/state.rs::State` applies them via `apply_event` (lines 77-89) and exposes `active_snap_slot` / `snap_slot_saved` accessors (lines 114-122); `app/shell/src/commands/snap.rs` exposes the full Tauri round-trip (PR #98). The OSC discovery / announcement surface remains planned and is not part of the implemented rollup here. |
| MIDI mappings | implemented | `08-io-integrations.md` §MIDI Planning Contract | `app/shell/src/commands/midi.rs` (`start_midi_learn`, `start_midi_performance_listener`) + `src/lib/stores/midiBindings.svelte.ts` implement MIDI learn, persistent bindings, and live background CC/Note control dispatch. |
| OSC mappings | implemented | `08-io-integrations.md` §OSC Planning Contract; `adr/ADR-013-networking-defaults-and-security.md` | `app/shell/src/commands/osc.rs` (`bind_osc_route` / `unbind_osc_route` / `osc_routes_snapshot` / `set_audio_threshold`), `integrations/osc-adapter/src/sender.rs`, and `src/lib/views/Osc.svelte` are live. |
| Show bundles | planned:EV-SHOW-BUNDLE-RECOVERY | `22-compatibility-versioning-policy.md` §Package Extensions; `03-product-requirements.md` FR-12 | `.chromaShow` is reserved and not implemented. |
| Rollback | implemented | `03-product-requirements.md` FR-12 / FR-24; `32-state-preset-playlist-migration-policy.md` | `engine/runtime/src/state_migration.rs` implements versioned snapshot rollback and migration (`EV-STATE-MIGRATION` Pass). |
| Crash-resume | implemented | `03-product-requirements.md` FR-21; `31-supportability-privacy-ops-policy.md` §Support Bundle Policy | `engine/runtime/src/crash_resume.rs` + 1 Hz `crash-resume.bin` snapshots + startup recovery + `get_crash_resume_status` IPC are wired in `app/shell/src/lib.rs` (classes a/b/c/d live; classes e/f/g deferred). |
| Command/event deterministic replay baseline | implemented | `04-runtime-architecture.md` §Command Flow; `26-evidence-artifact-registry.md` `EV-DETERMINISM` | The current runtime has deterministic command-bus replay coverage; this is not full state reconciliation. |

## Integrations

| Feature | Status | Reference | Notes |
|---|---|---|---|
| OSC adapter (bind, poll, validate, rate-limit, allowlist) | implemented | `08-io-integrations.md` §OSC Planning Contract; `adr/ADR-013-networking-defaults-and-security.md` | `integrations/osc-adapter/src/{adapter,sender}.rs` + 60 Hz scheduler poll/drain in `app/shell/src/lib.rs` (`EV-INTEGRATION-MATRIX` & `EV-NET-HARDENING` Pass). |
| MIDI adapter | implemented | `08-io-integrations.md` §MIDI Planning Contract | `integrations/midi-adapter/` (`midir 0.10`) + `app/shell/src/commands/midi.rs` (`list_midi_devices`, `start_midi_learn`, `start_midi_performance_listener`) (`EV-MIDI-EXTENDED` Pass). |
| Media / camera | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §Camera/Media Contract; `30-mvp-support-matrix.md` §Media Path Classes; `26-evidence-artifact-registry.md` `EV-INTEGRATION-MATRIX` | Live browser WebRTC `getUserMedia` camera capture is wired in `Mix.svelte` / `liveMedia.svelte.ts`; Rust `app/shell/src/commands/camera.rs` (`nokhwa`) remains a scaffold stub returning `Vec::new()`. |
| System audio ingest | implemented | `06-audio-engine.md` §Pipeline; `30-mvp-support-matrix.md` §Audio Device Classes; `adr/ADR-022-system-audio-capture.md` | `third_party/cpal-0.17.1-patched` delivers zero-install macOS Core Audio Process Tap system loopback capture, paired with 21 ms live RMS/peak/FFT metering, cached FFT autocorrelation BPM, and live `AudioInputGainHandle` IPC. |
| Audio uniform trace parity | implemented | `06-audio-engine.md` §7 Numeric Audio Contract; `10-performance-latency-budgets.md` | `engine/audio/src/{uniforms,bpm,metering,spectrum,runtime}.rs` + `EV-AUDIO-TRACE` Pass. |
| Syphon (macOS) | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §External Texture I/O Contract; `adr/ADR-011-rendering-backend.md` §Syphon / Spout interop; `26-evidence-artifact-registry.md` `EV-INTEGRATION-MATRIX` | `integrations/syphon-adapter/` ships `objc2` bindings scaffold; `list_syphon_servers` returns empty until `NSDistributedNotificationCenter` observer lands and `publish_syphon_output` honestly returns an unlinked-framework error unless `SYPHON_FRAMEWORK_PATH` is built. |
| Spout (Windows) | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §External Texture I/O Contract; `adr/ADR-011-rendering-backend.md` §Syphon / Spout interop | Platform adapter remains planned. |
| Output routing | implemented | `08-io-integrations.md` §Display/Output Surface Workflow; `30-mvp-support-matrix.md` §Output Surface Classes | Dedicated multi-monitor Stage Output Window (`app/shell/src/commands/output.rs`, `OutputStage.svelte`, TopBar `GO LIVE`) shipped; Windows/Spout platform conformance remains tracked under `EV-PLATFORM-CONFORMANCE`. |
| NDI | post-MVP | `30-mvp-support-matrix.md` §Media Path Classes; `adr/ADR-011-rendering-backend.md` §Texture interop | NDI is design-only at MVP freeze and not an MVP gate. |
| Open network OSC bind by default | intentionally-rejected | `adr/ADR-013-networking-defaults-and-security.md` §Rejected Alternatives | Loopback-only is the default; `0.0.0.0` requires explicit operator opt-in. |

## Observability

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Metric schema conformance | implemented | `31-supportability-privacy-ops-policy.md` §Scheme and Metric Catalog; `26-evidence-artifact-registry.md` `EV-OBSERVABILITY` | `engine/observability` mirrors the 14-metric catalog with conformance tests. |
| Metric emission sites (per-metric) | implemented | `31-supportability-privacy-ops-policy.md` §Evidence Mapping | Emission-site wiring is complete across all five subsystems per `EV-OBSERVABILITY` Pass in `docs/26`: observability meta (`engine/observability/src/emit.rs`), runtime (`engine/runtime/src/command_bus.rs` per-`CommandKind` / per-`EventKind` / lifecycle transition counters), scene (`engine/scene/src/observability.rs` + `validator.rs` validation and lint counters), audio (`engine/audio/src/observability.rs::record_pull_outcome` from every `IngestBoundary::try_pull`), and OSC (`integrations/osc-adapter/src/adapter.rs::UdpOscAdapter` via `record_outcome` / `enter_state` / `record_bind_error`). |
| Observability inventory (ADR-016 transparency artifact) | planned:EV-OBSERVABILITY-INVENTORY | `adr/ADR-016-observability-and-telemetry-posture.md` §Transparency artifact | The shipped `OBSERVABILITY-INVENTORY.md` artifact is not present yet. |
| Support bundle | planned:EV-SUPPORT-OPS-READINESS | `31-supportability-privacy-ops-policy.md` §Support Bundle Policy | Support bundle export and inclusion controls remain planned. |
| Loopback health metrics endpoint | planned:EV-HEALTH-METRICS-LOOPBACK-ONLY | `adr/ADR-016-observability-and-telemetry-posture.md` §Health metrics / telemetry | Transport is Phase 4d; Phase 1 has no network I/O. |
| Crash payload redaction | planned:EV-CRASH-PAYLOAD-REDACTION | `adr/ADR-016-observability-and-telemetry-posture.md` §Evidence | Crash-report payload inspection remains planned. |
| Analytics zero-default network posture | planned:EV-ANALYTICS-ZERO-DEFAULT | `adr/ADR-016-observability-and-telemetry-posture.md` §Decision | Fresh-install network observation remains planned. |
| Outbound HTTPS telemetry by default | intentionally-rejected | `adr/ADR-016-observability-and-telemetry-posture.md` §Decision | Health metrics must be opt-in loopback-only; no default outbound telemetry. |

## Release-eng

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Signing (macOS Developer ID, Windows Authenticode) | planned:EV-RELEASE-PIPELINE | `adr/ADR-014-signing-notarization-auto-update.md` §Code signing | Design is accepted; release pipeline implementation remains Phase 5. |
| Notarization | planned:EV-RELEASE-PIPELINE | `adr/ADR-014-signing-notarization-auto-update.md` §Code signing | macOS notarization / stapling remains planned. |
| Updater (Sparkle-EdDSA) | planned:EV-RELEASE-PIPELINE | `adr/ADR-014-signing-notarization-auto-update.md` §Auto-update | Sparkle 2 / EdDSA appcast signing is specified but not implemented. |
| SBOM (CycloneDX 1.5) | planned:EV-SBOM-COVERAGE | `43-development-standards.md` §10.4; `.github/workflows/sbom.yml` | SBOM workflow scaffolding exists; release artifact coverage remains planned. |
| Release attestation | planned:EV-RELEASE-ATTESTATION | `43-development-standards.md` §11.2 | Attested release artifacts remain planned. |
| Bundle hygiene / signing integrity | planned:EV-SIGNING-INTEGRITY | `adr/ADR-014-signing-notarization-auto-update.md` §Bundle hygiene deny-list | No-legacy-key and shipped-bundle denial checks remain planned. |
| Sparkle DSA fallback / legacy DSA key shipping | intentionally-rejected | `adr/ADR-014-signing-notarization-auto-update.md` §Auto-update; §Rejected Alternatives | Chromaticity ships EdDSA-only updater verification and never ships a legacy DSA key path. |
| ATS no third-party exceptions | planned:EV-ATS-NO-THIRD-PARTY-EXCEPTIONS | `adr/ADR-016-observability-and-telemetry-posture.md` §Evidence | Bundle-level ATS inspection remains planned. |
| Platform conformance for release support classes | planned:EV-PLATFORM-CONFORMANCE | `30-mvp-support-matrix.md` §Evidence Binding | Cross-platform release support matrix validation remains planned. |

## Broadcast, compositing & OBS parity (Phase 7 tracking)

Tracks the full OBS Studio 33.0.1 broadcast surface (plus native Twitch interactivity) so Chromaticity can track both active Phase-7 targets and deferred/optional broadcast capabilities in one place.

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Multi-layer GPU scene compositor (blend modes, opacity, 2D transform, Luma/Chroma Key) | implemented | `engine/render/src/compositor.rs`; `src/lib/views/Mix.svelte`; `src/lib/views/OutputStage.svelte` | Engine `wgpu` compositor (`compositor.rs`) + Mix live compositor UI (`Mix.svelte` + `videoMixer.svelte.ts`: 10 blend modes, live Chroma/Luma Key, text overlay compositing, and `BroadcastChannel` Program mirror to `OutputStage.svelte`); direct `wgpu` `compositor_stack` pass inside `render_gpu_frame` remains a follow-up. |
| Nested scenes & layer groups | planned:gap:phase7-nested-scenes | Phase 7 compositor follow-up | Hierarchical scene-as-layer composition and grouped transforms/visibility. |
| Interactive canvas transform / crop / snap handles | implemented | `src/lib/views/Mix.svelte` | Direct-manipulation move/resize handles, edge crop clipping, rotation, and center snap guides (`snapGuideX`/`snapGuideY`) shipped in `Mix.svelte`. |
| Studio Mode (dual Preview / Program canvases) | implemented | `src/lib/views/Mix.svelte`; `src/lib/stores/videoMixer.svelte.ts` | Dual Preview / Program monitors with explicit Cut-to-Program, live Studio Program Twitch broadcast, and `OutputStage.svelte` Program mirror shipped. |
| Shader & alpha-matte Stinger transitions | planned:gap:phase7-stinger-transitions | `engine/render/src/transitions.rs` | Extends `TransitionOrchestrator` with custom GLSL transition shaders and alpha-matte stinger clips. |
| Multiview (live grid of all scenes) | post-MVP | Phase 7+ broadcast ergonomics | Multi-scene live thumbnail monitoring grid for large stage shows. |
| Hardware-encoded local recording (VideoToolbox / NVENC / AMF / QSV → fMP4/MKV/MOV) | planned:gap:phase7a-hw-record | `engine/render/src/nv12_convert.rs`; `app/shell/src/commands/recorder.rs` | Engine GPU BGRA→NV12 pass (`nv12_convert.rs`), crash-safe fMP4 muxer + disk writer, and H.264-validated `push_recorder_frame` IPC (`recorder.rs`) shipped; native VideoToolbox/NVENC H.264 NAL encoder remains Phase 7a. |
| Rolling Replay Buffer ("save last N seconds" hotkey) | planned:gap:phase7a-replay-buffer | `engine/runtime/src/recorder.rs`; `app/shell/src/commands/recorder.rs` | `ReplayBuffer` + H.264-validated `push_recorder_frame` + `save_replay_buffer` IPC + `Mix.svelte` Save Replay button shipped; native hardware-encoder GOP feed remains Phase 7a. |
| Multi-track audio recording | post-MVP | Phase 7a+ | Separate isolated audio stems (Music / Mic / Alerts) inside recorded MKV/MP4 containers. |
| Live streaming: RTMP / RTMPS (H.264; Enhanced RTMP HEVC/AV1 planned) | implemented | `app/shell/src/commands/stream.rs`; `src/lib/stores/liveBroadcast.svelte.ts` | End-to-end Twitch RTMPS/RTMP live publisher (`ffmpeg` `h264_videotoolbox`/`libx264` + `avfoundation` audio muxing, redaction, `liveBroadcastStore` explicit state machine); pure-Rust `rml_rtmp` and Enhanced RTMP HEVC/AV1 remain follow-ups. |
| Live streaming: WHIP (WebRTC ultra-low-latency) | planned:gap:phase7b-whip | Phase 7b | Pure-Rust `str0m` WebRTC WHIP ingest for sub-second broadcast latency. |
| Live streaming: SRT / RIST | post-MVP | Phase 7b+ | Reliable UDP contribution transport for venue/studio links. |
| Display & Window Capture (`ScreenCaptureKit` macOS / WGC DXGI Windows) | planned:gap:phase7d-screen-capture | `src/lib/stores/liveMedia.svelte.ts` | Live screen capture via `getDisplayMedia` is shipped in `Mix.svelte` / `liveMediaStore`; zero-copy `ScreenCaptureKit` / WGC `IOSurface` capture remains Phase 7d. |
| Media / Video Source (MP4/WebM/GIF playback with loop/seek) | implemented | `src/lib/stores/liveMedia.svelte.ts`; `src/lib/views/Mix.svelte` | Local video & image layer playback with deterministic blob-URL lifecycle (`LocalMediaMachine`) shipped in `Mix.svelte`. |
| Direct YouTube URL Stream-In Source (muted video layer by default + opt-in audio-reactive feed — exceeds stock OBS) | implemented | `app/shell/src/commands/mix.rs`; `src/lib/stores/youtubeLayer.svelte.ts`; `src/lib/stores/liveMedia.svelte.ts` | Shipped end-to-end: `resolve_youtube_stream` + SSRF/CORS-hardened 4 MiB streaming proxy + HLS passthrough, DJ-safe muted-by-default `youtubeLayerStore` & `liveMediaStore`, transport controls, and `Perform.svelte` media strip card. |
| Text / Dynamic Typography overlay layer | implemented | `src/lib/stores/videoMixer.svelte.ts`; `src/lib/views/Mix.svelte` | `'text'` overlay layer source kind wired in `videoMixerStore` and rendered into `Mix.svelte` `compositeStageToCanvas` (visible on Twitch streams). |
| Browser Source (embedded HTML/CSS/JS overlay) | post-MVP | Phase 7+ | Embedded webview layer for third-party stream alerts/widgets (optional; native Twitch/Data overlays reduce need). |
| Image Slideshow / VLC Playlist source | post-MVP | Phase 7+ | Timed image/media carousel source layer. |
| Video filters: 3D `.cube` LUT, Color Correction, Crop/Pad, Lanczos/CAS sharpen | planned:gap:phase7c-video-filters | `src/lib/stores/videoMixer.svelte.ts`; `src/lib/utils/liveVideoFx.ts` | Chroma Key, Luma Key, Color Grade, Stylize/Mirror, Crop, and Audio-Reactive Link shipped in `Mix.svelte`; 3D `.cube` LUT and Lanczos/CAS sharpen remain Phase 7c. |
| Multi-bus broadcast audio mixer (System + Mic + Per-App + Media, gain/mute/pan/sync-offset) | implemented | `engine/audio/src/mixer.rs`; `app/shell/src/commands/subsystems.rs`; `src/lib/views/Mix.svelte` | Zero-alloc `BroadcastMixer` wired into `AppState` (fed on the 60 Hz scheduler loop) + `set_broadcast_bus_config` IPC called from `Mix.svelte` faders/mute/solo + OBS-grade ballistics (`meterBallistics.ts`); dedicated Mic/Media/Alerts hardware capture analysers remain a follow-up. |
| Broadcast audio DSP filters (RNNoise suppression, Noise Gate, Compressor, Limiter, Expander, 3-band EQ, Ducking) | planned:gap:phase7c-audio-dsp | Phase 7c | Zero-alloc audio-thread DSP (pure-Rust `nnnoiseless` RNNoise + dynamics/EQ). |
| Third-party VST2 / VST3 / AU audio plugin hosting | post-MVP | Phase 7+ | External plugin host; tracked for future evaluation if built-in DSP is insufficient. |
| Virtual Camera output (macOS `CMIOExtension` / Windows `MFCreateVirtualCamera`) | planned:gap:phase7e-virtual-cam | Phase 7e | Expose Chromaticity live output as a system webcam for Zoom, Discord, Meet, and browser apps (macOS gated on Apple Developer ID signing). |
| WebSocket remote-control API (`obs-websocket`-class JSON-RPC for Stream Deck / Touch Portal) | post-MVP | Phase 7+ | Loopback-default WebSocket control surface complementing OSC/MIDI. |
| Profiles & Scene Collections (switchable full-app broadcast rigs) | post-MVP | Phase 7+ | Named collections bundling scenes, layer stacks, audio routing, and stream profiles. |
| Native Twitch Integration (EventSub WebSocket + Chat + Channel Points → `chroma_Data_*` & triggers) | planned:gap:phase7-twitch | `engine/runtime/src/twitch_bridge.rs` (Phase 7) | Engine-level EventSub, Chat, and Channel Points bridge implemented in `engine/runtime/src/twitch_bridge.rs`; live WebSocket transport, OAuth token flow, and shell UI wiring remain planned. |

## DJ, Rekordbox 7 & Beatmatch Sync (Phase 8 tracking)

Tracks live DJ hardware integration, zero-driver Rekordbox 7 capture, and beatmatch synchronization for club and livestream performance workflows.

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Multi-channel Master vs. Cue stereo pair extractor & Pioneer/AlphaTheta profiles (incl. `DDJ-FLX10`) | implemented | `engine/runtime/src/rekordbox_bridge.rs`; `app/shell/src/commands/audio.rs` | Isolates Master vs. Cue stereo pairs from multi-channel DJ controller interfaces with built-in Pioneer/AlphaTheta device profiles including `DDJ-FLX10` (commit `10769c0`). |
| Rekordbox 7 `PC MASTER OUT` zero-install Core Audio Process Tap | implemented | `adr/ADR-022-system-audio-capture.md`; `third_party/cpal-0.17.1-patched` | Captures Rekordbox 7 `PC MASTER OUT` directly on macOS via zero-install Core Audio Process Tap loopback without BlackHole or third-party virtual audio drivers (commit `f2187da`). |
| DJ BPM `AUTO / LOCK / OFF` & Beatmatch Phase Scrub (`-0.5..+0.5` beats, platter nudge, resync) | implemented | `app/shell/src/commands/audio.rs`; TopBar BPM & beatmatch UI | Live `AUTO / LOCK / OFF` BPM modes, manual BPM override, `-0.5..+0.5` beat phase scrub, platter nudge, and beat-1 resync wired directly to Metal GPU uniforms (commits `ad343cb`, `257ab95`, `10769c0`). |
| Pro DJ Link UDP beat/tempo packet bridge | planned:gap:phase8-pro-dj-link | `engine/runtime/src/pro_dj_link_listener.rs`; `engine/runtime/src/clock_sync.rs` | Engine-level UDP port `50001` listener, multi-deck Master election, and `AutoTransitionPlanner` implemented and tested; spawning the live UDP listener in `app/shell` remains Phase 8. |
| Ableton Link peer tempo/phase sync | planned:gap:phase8-ableton-link | Phase 8 DJ/ensemble sync | Peer-to-peer Ableton Link tempo, quantum, and phase synchronization for live sets and multi-machine rigs. |

## Summary tally

These counts are informational only; this matrix is not a gate.

| implemented | planned:* | post-MVP | intentionally-rejected | n/a |
|---:|---:|---:|---:|---:|
| 46 | 38 | 9 | 4 | 0 |
