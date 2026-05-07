<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# ADR-020: Camera Adapter

## Status

Proposed

## Date

2026-05-07

## Deciders

- Accountable Owner: Xavier Butler (@PhantasticUniverse)

## Context

Phase-4a `EV-INTEGRATION-MATRIX` shipped at Pass under the OSC routing-matrix arm (Wave 1a, merge `77f174c` via `#323`) plus the MIDI input arm (Wave 1b, merge `bba49c1` via `#333`); the camera, Syphon, and audio-uniform-IPC arms were explicitly deferred to a Wave 1c follow-on per `docs/11-roadmap-phases.md` and the Phase-4a EXIT closure section of `docs/EV-GATE-MODEL-AUDIT-checklist.md`. Wave 1c was subsequently restructured into independently shippable slices: 1c-α closed the audio-uniform-IPC arm under ADR-019, and 1c-β covers the camera + Syphon arms.

ADR-019 §Out of Scope (lines 99-107) explicitly defers camera-input IPC to Wave 1c-β with a future-ADR pointer. This ADR is that future ADR for the camera arm. Syphon (macOS-only output IPC) is the parallel sibling under ADR-021; the two adapters are independent shipping vehicles and need not co-land.

The decision-problem this ADR locks is the chromaticity-native camera-adapter contract: cross-platform device enumeration, frame capture, texture binding, permission flow, and device-id persistence. Camera ships on **both macOS + Windows** through a single cross-platform crate boundary; per-platform unsafe surfaces (CoreMediaIO + MediaFoundation) are abstracted by an upstream wrapper crate so the workspace `unsafe_code = "forbid"` lint stays in force inside `integrations/camera-adapter/` itself. The decision is structurally analogous to ADR-005's `MediaInputAdapter` slot and follows the `integrations/{osc,midi}-adapter/` shape established by Wave 1a/1b.

Spout (Windows Syphon analog) is explicitly out of scope here and out of scope of ADR-021; it is reserved for a Phase-5+ task. `IO_CAMERA` capability per ADR-009 is free at MVP (`docs/adr/ADR-009-activation-and-entitlements.md` line 50: "camera ingestion capability (free; listed so post-MVP gating remains a data change, not a code change)") and remains free for the duration of this wave.

## Decision Drivers

- **Cross-platform parity at MVP.** Camera is a Parity-Core deliverable per `docs/30-mvp-support-matrix.md`; the adapter contract MUST present an identical surface on macOS and Windows so scenes that bind a camera input render identically across both platforms.
- **Workspace `unsafe_code = "forbid"` floor preserved.** Per `docs/43-development-standards.md` §12.1, subtree-tightening-only overrides are the rule and `unsafe_code = "forbid"` is the floor. The adapter MUST consume per-platform camera APIs through a wrapper crate that already encapsulates the unsafe boundary; loosening the workspace floor is not on the table for this wave.
- **Adapter-shape conformance with ADR-005 + Wave 1a/1b precedent.** The osc-adapter and midi-adapter crates established the canonical `bind/unbind/poll`, `defaults.rs` T1 drift pin, and per-crate `lib.rs` + module-split shape; the camera adapter mirrors that shape so reviewers and the conformance matrix recognise it.
- **Graceful permission-denial UX.** Camera access requires explicit user grant on both macOS (TCC) and Windows 10+ (Privacy Settings). The adapter MUST surface denial as an observable adapter health state, not panic or silently fail enumeration.
- **Bounded backpressure under variable hardware cadence.** Cameras emit frames at hardware-determined rates (typically 30 fps, with degraded performance under low-light auto-exposure pumping to 15 fps). The adapter MUST cap memory growth under a slow consumer; visualizers tolerate frame drops but not unbounded queue growth.
- **Synthetic-narrowing CI conformance.** Real camera hardware is unavailable on shared CI runners. The Wave 1c-β T2 corpus follows the row 78 / row 40 / row 93 synthetic-narrowing precedent: deterministic synthetic frame producers exercise the adapter contract; live hardware validation is manual on AO dev rigs.
- **Frame-buffer privacy.** Camera frames are user-PII. Crash payloads MUST exclude live frame buffers; the redaction surface is already shipped (`engine/observability/src/crash_redaction.rs::RedactCategory::MediaBuffer` per `EV-CRASH-PAYLOAD-REDACTION`) and this ADR reaffirms its posture for the camera path.

## Considered Options

1. **Cross-platform wrapper crate (`nokhwa 0.10`).** A single Rust crate abstracts CoreMediaIO (macOS) and MediaFoundation (Windows) behind a unified threaded-camera surface; chromaticity wraps the threaded surface in the canonical `bind/unbind/poll` adapter. (Chosen.)
2. **Per-platform direct integration via `coremediaio-sys` + `windows` crate Media Foundation bindings.** Two `#[cfg(target_os = ...)]`-gated implementations live inside `integrations/camera-adapter/` directly.
3. **Per-platform direct integration with `unsafe_code` allowed in `integrations/camera-adapter/`.** Same as option 2 but the unsafe blocks live in our crate rather than behind a third-party wrapper.

## Decision

Adopt **Option 1**: a single `integrations/camera-adapter/` crate wrapping `nokhwa 0.10` for cross-platform device enumeration + frame capture; CoreMediaIO on macOS, MediaFoundation on Windows; the workspace `unsafe_code = "forbid"` lint stays in force inside the adapter crate because nokhwa encapsulates the unsafe boundary.

The locked contract follows.

### 1. Crate placement and shape

A new workspace member `integrations/camera-adapter/` mirrors the `integrations/{osc,midi}-adapter/` shape established by Wave 1a/1b:

```
integrations/camera-adapter/
├── Cargo.toml
└── src/
    ├── lib.rs        # crate surface; re-exports + evidence-tag pin module
    ├── adapter.rs    # CameraAdapter trait + NokhwaInputAdapter impl + state machine
    ├── defaults.rs   # DEFAULT_TARGET_FPS / DEFAULT_TARGET_RESOLUTION / DEFAULT_QUEUE_DEPTH / DEFAULT_PIXEL_FORMAT_PREFERENCE + T1 drift pins
    └── frames.rs     # CameraFrame + PixelFormat + DeviceSummary + CameraConfig + AdapterHealth + CameraDeviceKind
```

The `[lints]` table inherits the workspace defaults (`unsafe_code = forbid`, `clippy::unwrap_used = deny`, `clippy::expect_used = deny`) per §12.6. No subtree loosening.

### 2. Public surface

`chromaticity-camera-adapter` exports:

- **Trait** `CameraAdapter` with the methods:
  - `bind(&mut self, config: CameraConfig) -> Result<(), CameraError>`
  - `unbind(&mut self) -> Result<(), CameraError>`
  - `poll(&mut self, timeout: Duration) -> Vec<CameraFrame>`
  - `health(&self) -> AdapterHealth`
  - `enumerate(&self) -> Result<Vec<DeviceSummary>, CameraError>`
- **Impl** `NokhwaInputAdapter` over the `nokhwa` threaded camera source. The threaded surface aligns with the established `bind/unbind/poll` state machine: `bind` opens the device + spawns the capture thread; `unbind` joins it; `poll` drains the bounded internal channel non-blockingly.
- **Frame value type** `CameraFrame { timestamp_ns: u64, width: u32, height: u32, pixel_format: PixelFormat, payload: Bytes }`. `Bytes` is the `bytes::Bytes` ref-counted byte buffer (already in the workspace dep graph via Tauri); the cheap-clone semantics let the render adapter (Phase 2 PR 5) take ownership without copy.
- **Pixel-format enum** `PixelFormat` with the MVP variants `Nv12`, `Yuyv`, `Rgb24`, `Bgra32`. The enum is `#[non_exhaustive]`; widening criteria are documented as a module-level prose comment (added when it becomes useful, e.g. for HDR or 10-bit YCbCr surfaces). Negotiation: the adapter requests `pixel_format_preference` from nokhwa and falls back to whatever the hardware actually offers; the realised format is reported on each `CameraFrame`.
- **Config value type** `CameraConfig { device_id: String, target_fps: u32, target_resolution: (u32, u32), pixel_format_preference: Option<PixelFormat>, queue_depth: usize }`. `device_id` is the platform-stable nokhwa device identifier; `target_fps` and `target_resolution` are negotiation hints; `queue_depth` bounds the in-adapter frame channel.
- **Health enum** `AdapterHealth { Bound, Disconnected, Negotiating, Faulted(Cow<'static, str>) }`, mirroring the MIDI / OSC adapter state machines. The `Faulted(reason)` variant carries a stable diagnostic string (`"permission-denied"`, `"device-not-found"`, `"format-negotiation-failed"`, …).
- **Device summary** `DeviceSummary { device_id: String, display_name: String, kind: CameraDeviceKind }` for the enumeration surface. `CameraDeviceKind` covers `BuiltIn`, `External`, `Virtual` to drive UI grouping.

### 3. Defaults

Per the ADR-013 default-policy precedent (`integrations/osc-adapter/src/defaults.rs`, `integrations/midi-adapter/src/defaults.rs`), the camera adapter pins its defaults in `defaults.rs` with T1 drift-pin tests:

- `DEFAULT_TARGET_FPS: u32 = 30`
- `DEFAULT_TARGET_RESOLUTION: (u32, u32) = (1280, 720)`
- `DEFAULT_QUEUE_DEPTH: usize = 4` (frames; bounded; see backpressure below)
- `DEFAULT_PIXEL_FORMAT_PREFERENCE: PixelFormat = PixelFormat::Nv12`

Rationale: 30 fps × 720p is the MVP support-matrix baseline; 4-frame queue depth balances latency against frame-drop risk under a 60 Hz consumer at 30 Hz producer cadence; NV12 is the most efficient native format on both backends and avoids a colour-space conversion in the producer thread.

### 4. Backpressure — bounded drop-on-full

The internal frame channel between the capture thread and `poll` is a bounded MPSC of `queue_depth` capacity. On full, the producer drops the **oldest** frame (replacing it with the newest), not the newest. Visualizers prefer fresh frames; a stale 4-frame backlog adds 133 ms of latency at 30 fps and is worse than a single dropped frame.

The drop-policy is documented as a module-level invariant in `adapter.rs` and exercised by a T2 fixture (Phase 2 PR 4). Mirrors the ADR-019 §Decision §4 drop-on-saturation posture for the audio-uniform IPC bridge.

### 5. Permission flow

#### macOS

- `app/shell/tauri.conf.json` Info.plist override adds `NSCameraUsageDescription` with copy that satisfies App Store review and clearly identifies camera use.
- First call to `enumerate()` triggers `AVCaptureDevice.requestAccess(for: .video)`; nokhwa's macOS backend already invokes this on first device-list call. Operator denial → `AdapterHealth::Faulted("permission-denied")`; the adapter records the denial and does not retry on subsequent enumerate calls within the process lifetime (the OS-level grant is process-bound).
- The Tauri shell shows a single non-modal banner: "Camera permission required to enumerate devices" with an "Open System Settings" deep-link (`x-apple.systempreferences:com.apple.preference.security?Privacy_Camera`). The banner is dismissable; re-enumeration after granting requires app restart per macOS TCC semantics.

#### Windows

- Media Foundation requires no manifest changes. Windows 10+ Privacy Settings camera-allow-list governs access at OS level.
- `MFCreateAttributes` device-enumeration succeeds even with camera privacy blocked (the device list is visible) but `MFCreateSourceReader` fails with `MF_E_DEVICE_INVALIDATED`. nokhwa surfaces this as an open error; the adapter catches it and routes to `AdapterHealth::Faulted("permission-denied")`.
- The same Tauri shell banner pattern applies; the Windows deep-link is `ms-settings:privacy-webcam`.

The permission-UX banner is a chrome surface that lands with the Phase 2 PR 2 / PR 3 platform wiring; this ADR specifies the contract, not the presentation.

### 6. Device-id persistence and reconnect

`CameraConfig::device_id` persists across sessions in app settings. On startup, if the persisted device is present in `enumerate()` output, `bind` runs against it. If absent, the adapter enters `AdapterHealth::Disconnected` and a re-enumeration on hardware change (USB hotplug detection on both platforms) triggers an auto-reconnect attempt.

Reconnect parameters are configurable per `CameraConfig` extension (added in Phase 2 PR 1):

- `reconnect_attempts: u32` — default `5`
- `reconnect_interval_ms: u32` — default `1000`

After exhausting attempts, the adapter stays at `AdapterHealth::Disconnected` until the user manually re-binds. This mirrors the MIDI-CONF-09 device-rebind precedent (`docs/26-evidence-artifact-registry.md` `EV-MIDI-EXTENDED` row, MIDI Learn save-quit-relaunch persistence fixture).

### 7. Texture-binding namespace

The render-side adapter (Phase 2 PR 5, `engine/render/src/camera_adapter.rs`) marshals `CameraFrame` payloads to `wgpu::Texture` resources exposed to the shader uniform layer under the `chroma_CameraInput[N]` symbol family, where `N ∈ 0..=3` (max 4 cameras per scene; mirrors the Shadertoy `iChannel0..3` range and aligns with the existing `chroma_*` chromaticity-native namespace per the user-memory `feedback_syn_prefix_synesthesia_compat.md` convention).

Format-to-`wgpu::TextureFormat` mapping:

- `PixelFormat::Bgra32` → `wgpu::TextureFormat::Bgra8UnormSrgb` (preferred GPU-side; one-pass sRGB upload)
- `PixelFormat::Rgb24` → `wgpu::TextureFormat::Rgba8UnormSrgb` (with an alpha-fill at upload)
- `PixelFormat::Yuyv` → `wgpu::TextureFormat::Rgba8UnormSrgb` after a CPU-side YUYV→RGBA conversion (or a future GPU-side compute conversion; the CPU path is the MVP baseline)
- `PixelFormat::Nv12` → planar pair: luma to `wgpu::TextureFormat::R8Unorm`, chroma to `wgpu::TextureFormat::Rg8Unorm`, with a fragment-shader-side YUV→RGB conversion uniform-side (the canonical zero-copy path)

Format negotiation lives entirely in `engine/render/src/camera_adapter.rs`; the integration crate stays format-agnostic.

### 8. Entitlement posture

`IO_CAMERA` capability stays **free at MVP** per ADR-009 line 50. The post-Wave-1b `HONORED_TOKEN_CAPABILITIES` widening (1 → 13 entries) intentionally excluded `IO_CAMERA`; that exclusion stays in force for the duration of Wave 1c-β. Tier-gating activation is deferred to a future post-MVP wave per the ADR-009 "post-MVP gating remains a data change, not a code change" comment.

Operationally: scene authors and operators bind cameras at MVP without a paid-tier license; the watermark policy that gates `render.watermark.off` does not touch the camera path.

### 9. Privacy posture — frame redaction in crash payloads

Camera frames are user-PII. The crash-payload redaction surface (`engine/observability/src/crash_redaction.rs::RedactCategory::MediaBuffer`) already covers media buffers per `EV-CRASH-PAYLOAD-REDACTION`. ADR-020 reaffirms the posture for the camera path: `CameraFrame::payload` is tagged `RedactCategory::MediaBuffer` at the Phase 2 PR 5 render-adapter boundary; the redaction surface is not re-implemented inside `integrations/camera-adapter/`.

The reaffirmation is normative: any future change that would route camera frame bytes around the redaction surface (e.g. embedding a thumbnail in a diagnostic payload) requires a new ADR or an amendment to ADR-016.

## Consequences

### Positive

- Camera input is available cross-platform under a single chromaticity-native adapter contract, closing the camera arm of `EV-INTEGRATION-MATRIX` deferred from Phase-4a.
- Workspace `unsafe_code = "forbid"` floor stays in force; the per-platform unsafe surfaces are encapsulated by nokhwa, which carries its own audit posture as a maintained third-party crate.
- Adapter shape mirrors `integrations/{osc,midi}-adapter/` precedent; reviewers and the conformance matrix recognise the surface.
- Frame-buffer privacy is wired from day one: the redaction posture from `EV-CRASH-PAYLOAD-REDACTION` extends to camera frames without per-camera-path implementation work.
- The `chroma_CameraInput[N]` namespace establishes a clean chromaticity-native surface for downstream scene authors; no `syn_*` (Synesthesia-compat) namespace contamination per the user-memory `feedback_syn_prefix_synesthesia_compat.md` convention.

### Negative

- Adds a new third-party dependency (nokhwa 0.10) with a transitive graph (image, mozjpeg-sys on some build configurations); cargo-deny licenses + bans + sources require validation at Phase 2 PR 1 land time. Any deny.toml widening lands as a `policy`-scoped commit per the workspace serial-list rule.
- The adapter's bound state on a single `CameraAdapter` instance is single-camera-at-a-time per nokhwa's threaded-camera shape; binding two cameras from a single scene requires two adapter instances. The Phase 2 PR 5 render wiring will compose multiple `NokhwaInputAdapter` instances behind the `chroma_CameraInput[0..=3]` surface.
- Format negotiation surfaces as observable variation: a scene that requests NV12 may receive YUYV on hardware that does not advertise NV12. Authors must read the realised format off `CameraFrame::pixel_format` if they care; defaults handle the common case.
- Synthetic-narrowing CI does not exercise live hardware. Drift between the contract and real-camera behaviour is caught only on manual AO dev-rig validation; the same posture as rows 78 / 40 / 93 (Phase-4a `EV-INTEGRATION-MATRIX` MIDI arm + `EV-NET-HARDENING` + `EV-PERF-BUDGETS`).

### Neutral / trade-offs

- nokhwa's threaded-camera surface dictates a thread-per-bound-camera model; the chromaticity scheduler thread (ADR-006) is unaffected because the camera frame path lives entirely outside the state-commit thread, but workspace thread count grows by one per bound camera.
- Choosing nokhwa over per-platform direct integration trades third-party dependency surface for unsafe-code containment; the trade aligns with the workspace `unsafe_code = "forbid"` floor and matches the established pattern of consuming midir / rosc through wrapper crates rather than wiring CoreMIDI / Berkeley sockets directly.
- The decision does not commit to nokhwa for any future GPU-IO surface (e.g. NDI, Spout, Syphon); Syphon is ADR-021's scope and Spout is reserved for Phase-5+. Each adapter chooses its own per-platform abstraction.

## Alternatives Considered

- **Per-platform direct integration via `coremediaio-sys` + `windows` Media Foundation bindings (Option 2).** Rejected. Two `#[cfg(target_os = ...)]`-gated implementations double the integration test surface, fork the bug-fix path, and require us to track CoreMediaIO + MediaFoundation idiom drift directly. The maintenance burden does not pay off when nokhwa already abstracts both backends cleanly. Fails the cross-platform-parity-at-MVP and adapter-shape-conformance drivers.
- **Per-platform direct integration with `unsafe_code` allowed in `integrations/camera-adapter/` (Option 3).** Rejected. Loosening the workspace `unsafe_code = "forbid"` floor for one adapter crate violates the §12.1 subtree-tightening-only rule and would require a `policy`-scoped ADR amendment to the workspace lint baseline. The cost is unjustified when nokhwa exists. Fails the `unsafe_code = forbid` driver.

## Out of Scope

- **Spout (Windows Syphon analog).** Reserved for a Phase-5+ task per the Wave 1c-β scoping plan. Not addressed by ADR-020 and not addressed by ADR-021.
- **Camera tier-gating activation.** `IO_CAMERA` stays excluded from the post-Wave-1b `HONORED_TOKEN_CAPABILITIES` widening for the duration of this wave; a future post-MVP wave revisits, at which point the gating is a `HONORED_TOKEN_CAPABILITIES` data change per ADR-009 line 50, not a code change.
- **Hardware-rig CI validation.** Synthetic narrowing per the row 78 / row 40 / row 93 precedent. Real camera hardware is manual on AO dev rigs; the Phase 2 PR 4 T2 corpus exercises the adapter contract with deterministic synthetic frame producers.
- **Live frame redaction in crash payloads — implementation.** Already shipped via `RedactCategory::MediaBuffer` per `EV-CRASH-PAYLOAD-REDACTION`; ADR-020 reaffirms the posture without re-implementing it in `integrations/camera-adapter/`.
- **Streaming output (camera-as-OSC-source equivalent).** Out of scope; cameras are input-only at MVP. No `chroma_CameraOutput[N]` surface is introduced.
- **Audio-from-camera microphone.** Out of scope; cameras' integrated microphones flow through `engine/audio` per ADR-006's audio-thread model, not through the camera adapter. Routing camera-microphone audio through this adapter would conflict with the ADR-006 single-audio-analysis-thread invariant.
- **HDR / 10-bit YCbCr / video-timecode metadata.** The MVP `PixelFormat` set is `Nv12` / `Yuyv` / `Rgb24` / `Bgra32`. The enum is `#[non_exhaustive]`; widening criteria are reserved for a future ADR amendment.

## Evidence

`EV-*` IDs whose status is produced, advanced, or blocked by this decision.

- **`EV-INTEGRATION-MATRIX`** — primary tagged evidence row. Phase-4a EXIT pass under the OSC-routing-matrix + MIDI-input narrowing remains unchanged at this ADR's acceptance time. The camera-arm widening of the row lands when Wave 1c-β PR 5 closes the render-side wiring; the closure narrows the row's "Camera, Syphon (macOS GPU IO), and audio-uniform-IPC arms remain deferred to Wave 1c per AO direction" clause along the camera axis. Per the `EV-AUDIO-UNIFORM-IPC` Wave 1c-α pattern, dual-tagging the camera T2 fixtures into `EV-INTEGRATION-MATRIX` is appropriate; the per-row anchor run id lands with Phase 2 PR 5.
- **`EV-CRASH-PAYLOAD-REDACTION`** — reaffirmed posture for camera frame buffers. ADR-020 does not widen the row this PR; the tagging is normative declaration that camera frames flow through `RedactCategory::MediaBuffer`.
- **`EV-PLATFORM-CONFORMANCE`** — Phase-5 cumulative gate. The cross-platform parity property of this ADR is a precondition for the Phase-5 row's per-platform conformance assertion against the camera adapter.
- **`EV-AUDIT-CRATE-SOURCES` / `EV-LICENSE-AUDIT`** — Phase 2 PR 1 widens the workspace dep graph by adding nokhwa (and any first-order transitives not already present); the cargo-deny + REUSE posture of the adapter crate is validated at land time. ADR-020 does not pre-commit a license election but flags nokhwa as MIT/Apache-2.0 dual which is on the existing allowlist.

## Traceability

- Planning: `docs/11-roadmap-phases.md` Phase-4a Wave 1c sequencing this ADR opens for the camera arm.
- Planning: `docs/30-mvp-support-matrix.md` Camera as Parity-Core deliverable.
- Planning: `docs/EV-GATE-MODEL-AUDIT-checklist.md` Phase-4a EXIT closure section — AO direction deferring camera / Syphon / audio-uniform-IPC arms to Wave 1c that ADR-020 partially closes for the camera slice.
- Risk: `docs/13-risk-register.md` R-37 (cross-platform integration scope risk).
- ADR: `adr/ADR-005-integration-adapter-boundary.md` — the `MediaInputAdapter` slot this ADR realises and the adapter-boundary contract layer it conforms to.
- ADR: `adr/ADR-006-thread-and-process-model.md` — thread allocation respected by the per-camera capture thread (camera-frame path lives outside the state-commit thread; no §single-writer-`StateCore` invariant impact).
- ADR: `adr/ADR-009-activation-and-entitlements.md` line 50 — `IO_CAMERA` MVP-free posture this ADR carries forward.
- ADR: `adr/ADR-013-networking-defaults-and-security.md` §3 — defaults-policy precedent for `defaults.rs` T1 drift pins.
- ADR: `adr/ADR-016-observability-and-telemetry-posture.md` — frame-redaction posture this ADR reaffirms via `RedactCategory::MediaBuffer`.
- ADR: `adr/ADR-018-non-scene-ui-toolkit.md` — Tauri 2 + Svelte 5 stack hosting the permission-UX banner surfaces.
- ADR: `adr/ADR-019-audio-uniform-ipc-bridge.md` §Out of Scope lines 99-107 — Wave 1c-β camera-arm deferral that this ADR closes.
- ADR: `adr/ADR-021-syphon-adapter.md` (parallel sibling Phase-1 PR) — macOS-only Syphon I/O; co-resident sibling adapter.
- Registry: `26-evidence-artifact-registry.md` `EV-INTEGRATION-MATRIX` row updated when Wave 1c-β PR 5 closes; row pre-allocation may extend `Description` via a same-PR docs change at ADR acceptance time.
- Crate precedent: `integrations/osc-adapter/src/{lib,adapter,defaults,routes}.rs` and `integrations/midi-adapter/src/{lib,adapter,defaults,messages}.rs` — the adapter shape the new `integrations/camera-adapter/src/{lib,adapter,defaults,frames}.rs` mirrors.
- User-memory convention: `feedback_syn_prefix_synesthesia_compat.md` — chromaticity-native uses `chroma_*`; the texture-binding namespace `chroma_CameraInput[N]` honours this.

## Rollout and Review

### Rollout

Wave 1c-β engineering plan, gated on this ADR's `Status: Accepted` transition. Estimated 1,500–2,500 LoC across 5 PRs:

- **PR 1 — `integrations/camera-adapter/` scaffold.** New workspace member; `lib.rs` + `adapter.rs` + `defaults.rs` + `frames.rs`; `CameraAdapter` trait + `NokhwaInputAdapter` impl over the nokhwa threaded source; defaults module with T1 drift pins; 8–10 T1 pins covering trait shape + state-machine transitions + defaults values + namespace constants. New top-level dep `nokhwa = "0.10"`; cargo-deny + REUSE + workspace-hack regen. No platform-specific permission wiring yet.
- **PR 2 — macOS permission flow.** `app/shell/tauri.conf.json` Info.plist override (`NSCameraUsageDescription`); `AVCaptureDevice.requestAccess` triggered on first `enumerate()` call (relies on nokhwa's macOS backend); operator denial routed to `AdapterHealth::Faulted("permission-denied")`; tcc.plist coverage check for the test-time codepath. Permission-UX banner copy + deep-link handler in the Tauri shell.
- **PR 3 — Windows Media Foundation flow.** Privacy-Settings-aware enumeration; `MF_E_DEVICE_INVALIDATED` caught and routed to `AdapterHealth::Faulted("permission-denied")`; permission-UX banner deep-link to `ms-settings:privacy-webcam`.
- **PR 4 — T2 corpus.** Synthetic-narrowing per row 78 / row 40 / row 93 precedent; deterministic synthetic-frame producers exercise the adapter contract end-to-end; 12–18 fixtures × 100 trials covering enumeration / bind / unbind / state-machine transitions / queue-depth-bounded drop-on-full / pixel-format negotiation / reconnect attempts × interval / permission-denied routing on both platforms (synthetic). No live hardware on CI.
- **PR 5 — Render-side wiring + closing-docs.** New `engine/render/src/camera_adapter.rs` mirroring `audio_adapter.rs`; texture-binding namespace `chroma_CameraInput[N]` with the per-pixel-format `wgpu::TextureFormat` mapping; multi-camera composition (max 4 per scene); closing docs(repo) PR that widens `EV-INTEGRATION-MATRIX` along the camera axis with the Wave 1c-β anchor run id, dual-tags the row's camera fixtures, and transitions the camera-arm narrowing clause from "deferred to Wave 1c" to "Pass under the synthetic-narrowing scope".

The 5-PR sequence lands as a directed engineering wave; PR 1 is a serial-list touch (workspace member addition + new `Cargo.toml` + nokhwa as new top-level dep — concurrent edits to `Cargo.toml [workspace] members` and `workspace-hack` produce semantic conflicts the linter does not catch per the `Agent parallelism playbook`); PRs 2 / 3 / 4 are parallel-safe per-subtree work; PR 5 is the closing serial touch.

Phase-4 / 4b / 4c / 4d EXIT decisions remain AO-held per `docs/11-roadmap-phases.md`; this ADR's acceptance does not pre-empt the sub-phase gate decisions, only opens Wave 1c-β engineering against the camera arm of `EV-INTEGRATION-MATRIX`.

### Review cadence

ADR-020 is revisited if (a) nokhwa upstream surfaces a maintenance gap that pushes the chromaticity dep graph onto a fork or vendored snapshot; (b) the synthetic-narrowing T2 corpus surfaces a contract gap when validated against live hardware on AO dev rigs that requires widening the trait surface; (c) a future post-MVP wave activates `IO_CAMERA` tier-gating, at which point the entitlement posture section is amended; (d) the Wave 1c-β render-side wiring (PR 5) surfaces a multi-camera composition constraint that requires the `chroma_CameraInput[N]` upper bound (4) to widen; or (e) a future HDR / 10-bit / video-timecode use case requires the `PixelFormat` `#[non_exhaustive]` enum to widen.

---

> This ADR is binding once `Status: Accepted`. Modification requires a new ADR or a superseding ADR; do not edit the Decision section of an Accepted ADR in place.
