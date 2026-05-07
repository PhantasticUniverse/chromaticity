// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity engine render.
//!
//! Phase-2 slice 1 establishes the wgpu + naga + GLSL 4.50 → SPIR-V
//! toolchain mandated by `docs/adr/ADR-011-rendering-backend.md` and
//! produces the first evidence harness toward `EV-RENDER-CONFORMANCE`
//! (`docs/26-evidence-artifact-registry.md` row 28). The crate also
//! advances the deferred R-33 wgpu / WebView2 composition spike noted
//! in `docs/13-risk-register.md` line 173 — slice-1 stands up the
//! offscreen render path that the spike's surface-attached extension
//! will build on.
//!
//! Slice progression:
//!
//! - **Slice 1** — single-pass entry point
//!   ([`single_pass::compile_and_render_single_pass`]) consuming
//!   hand-written GLSL fragment source, hardcoded full-screen-triangle
//!   vertex shader, RGBA8Unorm offscreen target with pixel readback,
//!   structured [`single_pass::RenderError`] for naga / wgpu /
//!   device-loss failures.
//! - **Slice 2** — multi-pass scheduler entry point
//!   ([`multipass::compile_and_render_multipass`]) consuming a flat
//!   plain-data [`multipass::MultipassSpec`] (intentionally NOT
//!   `engine/scene::PassSpec` — see `multipass.rs` module-level doc),
//!   per-pass std140 uniform buffers, prior-pass texture sampling
//!   via Vulkan-GLSL `texture2D` + `sampler` separation, sequential
//!   pass scheduling with forward-reference / self-reference
//!   rejection. Final-pass readback retains the slice-1 byte format
//!   for corpus-hash continuity. The
//!   [`single_pass::RenderError`] surface widens with multipass
//!   variants (`EmptyPassList`, `PassInputOutOfRange`,
//!   `UniformBindingFailed`, `PassRenderFailed`) under
//!   `#[non_exhaustive]`.
//! - **Slice 3** — GLSL `#include` preprocessor for the Lygia
//!   v1.1.4 trial vendor ([`glsl_includes::resolve_includes`]) consuming
//!   `#include "lygia/<path>.glsl"` directives at line scope, recursively
//!   inlining content under the caller-supplied vendor root with a depth
//!   cap to catch cycles. The single-pass / multi-pass entry points do NOT
//!   auto-call the preprocessor; callers explicitly invoke
//!   [`glsl_includes::resolve_includes`] before passing source to those
//!   entry points. Trial-period scope; the preprocessor stays as a
//!   generic include-resolver if Phase-5 `EV-LICENSE-AUDIT` drops Lygia.
//! - **Slice 4** (PR `#259` / `efd8b63`) — feedback-texture sampling
//!   ([`multipass::PassDef::feedback_self`] + ping-pong texture
//!   pair, multi-frame execution via
//!   [`multipass::MultipassSpec::frame_count`]), `RGBA32F` /
//!   `RGBA16F` float-target conformance per ADR-011 §Decision
//!   ([`multipass::TargetFormat`]), and the
//!   `chromaticity_scene::Scene` → [`multipass::MultipassSpec`]
//!   adapter ([`scene_adapter::AdapterError`] +
//!   `MultipassSpec::from_scene`, gated behind the
//!   `scene-adapter` cargo feature). The
//!   [`single_pass::RenderError`] surface widens with three
//!   variants (`FeedbackAliasError`, `FloatTargetUnsupported`,
//!   `SceneAdapterError`) under `#[non_exhaustive]`. Existing
//!   slice-2 corpus byte-hashes remain bit-identical (single-
//!   frame, `Rgba8Unorm`-only, no feedback paths preserve the
//!   slice-2 byte stream).
//! - **Slice 5a** (PR `#269` / `c724d17`) — cross-crate audio → render uniform-
//!   binding adapter ([`audio_adapter::uniform_input_from_audio_snapshot`],
//!   gated behind the optional `audio-binding` cargo feature).
//!   Marshals a `chromaticity_audio::AudioUniformSnapshot` (slice-1
//!   producer-side, plain data) into a `Vec<UniformInput>` keyed by
//!   the `syn_BPM` compat-syn-v1 retention (docs/36 line 117) plus
//!   three chromaticity-native `chroma_*` audio primitives
//!   (`chroma_AudioRMS`, `chroma_AudioPeak`, `chroma_AudioFFT[8]`).
//!   Slice-5a's initial release (catalog v1.2.0) used a transitional
//!   split-vec4 spelling for the FFT (`chroma_AudioFFTLow` +
//!   `chroma_AudioFFTHigh`) because [`multipass::UniformValue`] had
//!   no array variant; the slice-5 follow-up landed
//!   [`multipass::UniformValue::FloatArray`] and consolidated the
//!   FFT into a single 8-element array binding (catalog v1.3.0).
//!   Crate coupling stays consumer-side: `engine/audio` does NOT
//!   depend on `engine/render` (see `engine/audio/src/lib.rs`
//!   lines 16-31).
//! - **Slice 5b.1** (PR `#276` / `f6d6540`) — transition-orchestration LIFECYCLE
//!   per `docs/adr/ADR-007-scene-warm-switch-policy.md` Accepted
//!   four-phase warm-switch (`PrepareNext` / `PrimeNext` /
//!   `CrossfadeOrHardCut` / `CommitNext`) plus an `Idle`
//!   "no-transition-pending" state. The
//!   [`transitions::TransitionOrchestrator`] is a pure-data state
//!   machine: it owns the current and (optionally) next
//!   [`multipass::MultipassSpec`] and exposes explicit
//!   phase-advance methods (`cue_next` / `mark_primed` /
//!   `begin_crossfade` / `commit_next` / `abort`). Invalid
//!   advances surface as
//!   [`transitions::TransitionError::InvalidStateTransition`]; the
//!   cued-spec shape gate (empty pass list / zero `frame_count` /
//!   zero target size) surfaces as
//!   [`transitions::TransitionError::SpecMismatch`]. The orchestrator
//!   is GPU-device-FREE — every method is sync and runs without
//!   touching wgpu, mirroring the "render scheduler stays scene-
//!   agnostic" invariant from `multipass.rs`. T1 algorithm-drift
//!   pins live in `transitions::transitions_pins`; T2 lifecycle
//!   corpus lives in `engine/render/tests/transitions.rs`. Slice
//!   5b.2 will add the actual crossfade composition + 500 ms
//!   timeout-degrades-to-hard-cut path + GPU resource preallocation
//!   wiring.
//! - **Slice 5b.2** (PR `#283` / `5fb9c03`) — crossfade frame composition with the
//!   ADR-007 §Decision 500 ms timeout-degrades-to-hard-cut fallback
//!   plus GPU resource preallocation wiring on the orchestrator's
//!   `PrepareNext` and `PrimeNext` advance. Adds
//!   [`composition::PreparedScene`] (a GPU resource handle allocated
//!   by [`transitions::TransitionOrchestrator::prepare_resources`]
//!   ahead of the crossfade — holds an output texture, sampler, and
//!   progress-uniform buffer pre-sized to the cued spec),
//!   [`composition::compose_transition_frame`] (the crossfade entry
//!   point; the slice-5b.2 implementation rendered the two specs via
//!   [`compile_and_render_multipass`] and composited the byte streams
//!   on the CPU — slice 5b.3 below replaces the body with a GPU
//!   compositing pipeline while preserving the entry-point signature
//!   and endpoint short-circuit byte-identity contract),
//!   [`transitions::TransitionClock`] (a local trait abstraction
//!   for the timeout-tracking wall-clock — separate from
//!   `engine/script::Clock` which has different semantics for the
//!   JS `Date.now()` shim — with default
//!   [`transitions::MonotonicTransitionClock`] backed by `Instant`
//!   and [`transitions::FixedTransitionClock`] as the test
//!   primitive), and
//!   [`transitions::TransitionOrchestrator::commit_hard_cut`] (the
//!   ADR-007 timeout-fallback path, swaps current/next immediately
//!   without crossfade — legal from any in-flight phase). Plus three
//!   new `TransitionError` variants under existing `#[non_exhaustive]`
//!   (`TimeoutExceeded`, `GpuAllocationFailed`, `CompositionFailed`).
//!   New T1 pins in `transitions::transitions_pins` cover timeout
//!   firing, within-budget happy path, hard-cut from each phase,
//!   clock monotonicity, and elapsed accessor. The T2 GPU corpus in
//!   `engine/render/tests/transition_composition.rs` exercises the
//!   orchestrator + composition end-to-end.
//! - **Slice 5b.2+ refactor** (PR `#289` / `91f421a`) —
//!   [`multipass::MultipassRenderer`] long-lived-device entry
//!   point. The pre-refactor `compile_and_render_multipass` free
//!   function acquired a fresh wgpu Instance / Adapter / Device per
//!   call, which forced slice-5b.2's
//!   [`composition::compose_transition_frame`] to perform the
//!   crossfade as a CPU-side byte blend (the slice-5b.2 module-
//!   level prose documents this constraint). The refactor
//!   introduces [`multipass::MultipassRenderer`] which owns the
//!   wgpu Instance / Adapter / Device / Queue plus the cached
//!   fullscreen-triangle vertex shader module across `render`
//!   calls; the device is exposed via
//!   [`multipass::MultipassRenderer::device`] and
//!   [`multipass::MultipassRenderer::queue`] accessors so a future
//!   slice can submit a GPU composition pass against the same
//!   device that just rendered the source frames. The free
//!   function `compile_and_render_multipass` remains as a thin
//!   backwards-compat shim that builds a fresh
//!   [`multipass::MultipassRenderer`] per call so every existing
//!   slice-1 / slice-2 / slice-4 / slice-5 corpus byte-hash stays
//!   bit-identical and existing callers
//!   ([`composition::compose_transition_frame`],
//!   [`transitions::TransitionOrchestrator::prepare_resources`])
//!   need zero changes. The refactor is purely a device-ownership
//!   widening; slice 5b.3 (a separate follow-up PR that USES this
//!   refactor) will pull the CPU-side blend onto the GPU.
//! - **Slice 5b.3** (PR `#291` / `5263422`) — GPU-side compositing pipeline. The
//!   slice-5b.2 [`composition::compose_transition_frame`] CPU-side
//!   per-byte alpha lerp is retired in favor of a third blend-pass
//!   render pipeline that samples the current and next render
//!   outputs and blends them via `mix()` in the fragment shader.
//!   Tier-9 [`multipass::MultipassRenderer`]'s long-lived
//!   `Arc<wgpu::Device>` + `wgpu::Queue` accessors are the
//!   architectural prerequisite that slice 5b.3 spends —
//!   pre-Tier-9 the multipass scheduler acquired a fresh device
//!   per call, which is what forced slice-5b.2 to read back two
//!   byte streams and blend on the CPU. Adds
//!   [`multipass::MultipassRenderer::render_to_texture`] (renders a
//!   spec to a GPU texture without the readback round-trip;
//!   [`multipass::MultipassRenderer::render`] becomes a wrapper
//!   that calls `render_to_texture` and reads back the bytes) and
//!   [`composition::compose_transition_frame_gpu`] (the long-lived-
//!   renderer composition entry point that callers already holding
//!   a renderer should prefer; the original
//!   [`composition::compose_transition_frame`] keeps its public
//!   signature and now wraps `compose_transition_frame_gpu` with a
//!   per-call renderer). Endpoint short-circuit byte-identity
//!   (`progress = 0.0` and `progress = 1.0`) is preserved — the
//!   short-circuit short-paths to a direct
//!   [`multipass::MultipassRenderer::render`] call without entering
//!   the third pass. Mid-range `progress` byte hashes are
//!   regenerated against the GPU-blended output (Apple Silicon
//!   Metal seed; the slice-5b.2 CPU-blended hashes are retired in
//!   the same PR). T1 GPU pins in
//!   `composition::composition_gpu_pins` cover endpoint identity,
//!   midpoint per-pixel bounds, `render_to_texture` format /
//!   dimensions, and progress clamping. The slice-5b.2
//!   `composition::composition_pins` CPU-helper pins (formerly
//!   covering `compose_byte_streams` and the half-float helpers)
//!   are removed — those helpers no longer exist; structural
//!   validation pins (size / format / empty-pass-list mismatch
//!   rejection) remain.
//! - **Slice 5b.4+ (deferred)** — DX12 backend re-enablement,
//!   Windows Vulkan accept-set expansion, Metal backend-matrix
//!   conformance, `syn_RES`-style vec2-batched uniform widening of
//!   [`multipass::UniformValue`] (the FFT array consolidation that
//!   was previously listed here landed in the slice-5a follow-up
//!   that opened catalog v1.3.0).
//! - **Wave 1c-β Phase 3 PR 3** (this PR) — engine-side Syphon ↔ wgpu
//!   bridge per ADR-021 §6 ([`syphon_adapter::SyphonRenderBridge`] +
//!   [`syphon_adapter::Objc2SyphonClientHandle`] +
//!   [`syphon_adapter::Objc2SyphonServerHandle`], gated behind the
//!   optional `syphon-binding` cargo feature + `cfg(target_os = "macos")`).
//!   Real-backend impls of
//!   `chromaticity_syphon_adapter::adapter::backend::SyphonClientHandle`
//!   / `SyphonServerHandle` wrap `Retained<SyphonMetalClient>` /
//!   `Retained<SyphonMetalServer>` framework handles obtained via
//!   `chromaticity_syphon_adapter::bindings`. Drop posture per
//!   ADR-021 §8 + docs/54 Lesson 11 (defensive `[client stop]` /
//!   `[server stop]`); CFType-as-Drop refcounting per docs/54
//!   Lesson 3.7. PR 3 ships the trait-impl shape + T1 pins under
//!   the wave's synthetic-narrowing posture (no live Syphon bus on
//!   CI per ADR-021 §10); the live `wgpu_hal::api::Metal` escape-
//!   hatch + `CVMetalTextureCache` + `CVPixelBufferPool` pre-warm
//!   ring per docs/54 lessons 3.1 / 3.3 / 3.4 are documented
//!   inline at [`syphon_adapter::SyphonRenderBridge::resolve_iosurface_to_wgpu_texture`]
//!   / [`syphon_adapter::SyphonRenderBridge::prewarm_output_surface_ring`]
//!   as the deferred bridge-body wave's target — both entry points
//!   return [`syphon_adapter::SyphonRenderBridgeError::Unimplemented`]
//!   in PR 3. The narrowing is recorded honestly in the
//!   `EV-INTEGRATION-MATRIX` closing-docs description-column entry
//!   per the wave's synthetic-narrowing posture; format locked to
//!   [`wgpu::TextureFormat::Bgra8UnormSrgb`] ↔ `kCVPixelFormatType_32BGRA`
//!   per docs/54 Lesson 3.2. Crate coupling stays consumer-side:
//!   `integrations/syphon-adapter/` does NOT depend on
//!   `engine/render`.
//! - **Wave 1c-β Phase 2 PR 5** (this PR) — engine-side camera → render
//!   texture-binding adapter per ADR-020 §7
//!   ([`camera_adapter::camera_texture_binding_from_frame`] +
//!   [`camera_adapter::CameraTextureBinding`] carrier shape +
//!   [`camera_adapter::CAMERA_UNIFORM_NAMES`] /
//!   [`camera_adapter::camera_uniform_name`], gated behind the
//!   optional `camera-binding` cargo feature). Cross-platform via
//!   the `chromaticity_camera_adapter` crate
//!   (`integrations/camera-adapter/`, nokhwa 0.10 abstracting
//!   CoreMediaIO on macOS + Media Foundation on Windows; the
//!   adapter crate itself is platform-uniform). Marshals a
//!   plain-data `chromaticity_camera_adapter::CameraFrame` into a
//!   [`camera_adapter::CameraTextureBinding`] keyed by the
//!   `chroma_CameraInput[N]` symbol family (N ∈ 0..=3, max 4
//!   cameras per scene per ADR-020 §7). Pixel-format →
//!   `wgpu::TextureFormat` mapping covers `Bgra32` →
//!   [`wgpu::TextureFormat::Bgra8UnormSrgb`] (verbatim 4-bpp
//!   copy), `Rgb24` → [`wgpu::TextureFormat::Rgba8UnormSrgb`]
//!   (3 → 4 bpp expansion w/ alpha = 0xFF; wgpu has no native
//!   `Rgb8` sampler-bind format), `Nv12` →
//!   [`wgpu::TextureFormat::R8Unorm`] (luma plane only — chroma
//!   plane deferred per the module's "Scope narrowing for PR 5"),
//!   `Yuyv` → [`wgpu::TextureFormat::Rg8Unorm`] (verbatim packed
//!   2-bpp; full YUV → RGB conversion is the future-wave path),
//!   `Mjpeg` → [`single_pass::RenderError::CameraFormatNotSupported`]
//!   (JPEG decode deferred). Crate coupling stays consumer-side:
//!   `integrations/camera-adapter/` does NOT depend on
//!   `engine/render`. PR 5 ships the marshal + carrier shape +
//!   T1 pins (slot-namespace shape, mapping table, slot-bound
//!   rejection, Mjpeg rejection, zero-dim rejection, purity); the
//!   live `wgpu::Queue::write_texture` upload + the
//!   `wgpu::Texture` / `wgpu::TextureView` / `wgpu::Sampler`
//!   resource lifecycle is the consumer's job — a future wave
//!   that integrates the camera binding into
//!   [`multipass::MultipassRenderer`]'s per-pass binding loop is
//!   the natural target.
//!
//! - **Wave 4d.2 Lane B PR 1 of 2** (PR `#351` / `5abc443`) — deterministic GPU
//!   adapter-selection policy per ADR-011 §7
//!   ([`adapter_selection::GpuSelectionPolicy`] +
//!   [`adapter_selection::select_adapter`] +
//!   [`adapter_selection::enumerate_adapter_summaries`]). Adds the
//!   ranking core (adapter-type + per-OS backend preference + name
//!   tie-breaker) and the operator-override input surface; the
//!   per-installation persistence shell + battery / TDR-history
//!   warning surfacing + hot-plug recovery are deferred per the
//!   module-level "scope boundary" prose. The
//!   [`multipass::MultipassRenderer::new`] and
//!   [`single_pass::compile_and_render_single_pass`] wgpu adapter
//!   acquisition sites both route through
//!   [`adapter_selection::select_adapter`] with the default policy
//!   (which preserves the prior `LowPower` / no-override behavior at
//!   the *type* axis only — a Discrete adapter, when present, is now
//!   preferred per the ADR; the slice-1 / slice-2 / slice-4 / slice-5
//!   T2 corpora keep their byte hashes because none of those tests
//!   rely on multi-adapter machines and the selector returns the
//!   same single adapter when only one is enumerated). The pure-data
//!   T1 ranking pins live in
//!   `adapter_selection::adapter_selection_pins`; the dual-adapter
//!   physical / VM validation corpus is PR 2 scope and lives in
//!   `engine/render/tests/gpu_selection.rs` in that follow-up.
//!
//! Produces evidence for `EV-RENDER-CONFORMANCE`:
//!
//! - T1 entry-point drift pins in `single_pass::compile_drift_pins`
//!   (slice 1), `multipass::multipass_drift_pins` (slices 2 + 4 +
//!   5b.2-refactor + 5b.3 `render_to_texture`),
//!   `glsl_includes::include_drift_pins` (slice 3),
//!   `scene_adapter::adapter_drift_pins` (slice 4, gated behind
//!   the `scene-adapter` feature),
//!   `audio_adapter::audio_adapter_drift_pins` (slice 5a, gated
//!   behind the `audio-binding` feature; dual-tagged
//!   `EV-AUDIO-TRACE`), `transitions::transitions_pins` (slice
//!   5b.1, the warm-switch lifecycle state machine), and
//!   `composition::composition_pins` + `composition::composition_gpu_pins`
//!   (slice 5b.3 GPU compositing — structural rejection without an
//!   adapter and end-to-end GPU compose with one).
//! - T2 deterministic UV-gradient corpus in
//!   `engine/render/tests/single_pass.rs` (slice 1), per-pass cascade
//!   corpus in `engine/render/tests/multipass.rs` (slice 2),
//!   Lygia-include end-to-end corpus in
//!   `engine/render/tests/glsl_lygia.rs` (slice 3), feedback-decay /
//!   float-target / scene-adapter corpora in
//!   `engine/render/tests/feedback.rs`,
//!   `engine/render/tests/float_targets.rs`, and
//!   `engine/render/tests/scene_adapter.rs` (slice 4), the
//!   audio-uniform GPU-roundtrip corpus in
//!   `engine/render/tests/audio_adapter.rs` (slice 5a, dual-tagged
//!   `EV-AUDIO-TRACE`), and the warm-switch lifecycle corpus in
//!   `engine/render/tests/transitions.rs` (slice 5b.1).
//!
//! The Lygia trial vendor row is tracked separately as
//! `EV-LIBRARY-LICENSE-TRIAL` in
//! `docs/26-evidence-artifact-registry.md`; final commercial-use
//! license election is deferred to Phase-5 `EV-LICENSE-AUDIT` per
//! `docs/29-cleanroom-legal-policy-baseline.md` Sec.Bundled GLSL
//! Library Handling.
//!
//! Per `docs/26-evidence-artifact-registry.md` row 28's Phase-2-exit
//! definition, full conformance — feedback (slice 4 ✓), transition
//! fixtures (slice 5b.1 / 5b.2 / 5b.3 ✓), backend matrix (Wave 4d.2
//! Lane B PR 2 dual-adapter corpus pending) — was reached
//! incrementally. Per-slice registry-row updates land on separate
//! follow-up `docs(repo)` commits.
//!
//! Governing ADRs:
//!
//! - ADR-011 — rendering backend
//!   (`docs/adr/ADR-011-rendering-backend.md`).
//! - ADR-018 — non-scene UI toolkit
//!   (`docs/adr/ADR-018-non-scene-ui-toolkit.md`).

pub mod adapter_selection;
#[cfg(feature = "audio-binding")]
pub mod audio_adapter;
#[cfg(feature = "camera-binding")]
pub mod camera_adapter;
pub mod composition;
pub mod glsl_includes;
pub mod multipass;
#[cfg(feature = "scene-adapter")]
pub mod scene_adapter;
pub mod shader_cache;
pub mod single_pass;
#[cfg(all(feature = "syphon-binding", target_os = "macos"))]
pub mod syphon_adapter;
pub mod transitions;

pub use adapter_selection::{
    AdapterId, AdapterType, Backend, GpuSelectionPolicy, SelectedAdapter,
    enumerate_adapter_summaries, select_adapter,
};
#[cfg(feature = "audio-binding")]
pub use audio_adapter::{
    UNIFORM_NAME_BPM, UNIFORM_NAME_FFT, UNIFORM_NAME_PEAK, UNIFORM_NAME_RMS,
    uniform_input_from_audio_snapshot,
};
#[cfg(feature = "camera-binding")]
pub use camera_adapter::{
    CAMERA_UNIFORM_NAMES, CameraTextureBinding, MAX_CAMERA_BINDING_SLOTS,
    camera_texture_binding_from_frame, camera_uniform_name,
};
pub use composition::{PreparedScene, compose_transition_frame, compose_transition_frame_gpu};
pub use glsl_includes::{IncludeError, MAX_INCLUDE_DEPTH, resolve_includes};
pub use multipass::{
    MultipassRenderer, MultipassSpec, PassDef, PassInputBinding, TargetFormat, UniformInput,
    UniformValue, compile_and_render_multipass,
};
#[cfg(feature = "scene-adapter")]
pub use scene_adapter::{AdapterError, SceneAdapterOptions};
pub use shader_cache::{ShaderCache, ShaderCompileMetric, ShaderId};
pub use single_pass::{RenderError, compile_and_render_single_pass};
#[cfg(all(feature = "syphon-binding", target_os = "macos"))]
pub use syphon_adapter::{
    Objc2SyphonClientHandle, Objc2SyphonServerHandle, SyphonRenderBridge, SyphonRenderBridgeError,
};
pub use transitions::{
    DEFAULT_TIMEOUT_MS, FixedTransitionClock, MonotonicTransitionClock, TransitionClock,
    TransitionError, TransitionOrchestrator, TransitionState,
};
