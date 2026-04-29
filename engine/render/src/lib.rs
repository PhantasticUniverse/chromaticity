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
//! - **Slice 4 (this PR)** — feedback-texture sampling
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
//! - **Slice 5+** — transition orchestration (ADR-007 warm-switch),
//!   audio-uniform binding (`engine/audio` integration), Metal /
//!   DX12 backend-matrix conformance.
//!
//! Produces evidence for `EV-RENDER-CONFORMANCE`:
//!
//! - T1 entry-point drift pins in `single_pass::compile_drift_pins`
//!   (slice 1), `multipass::multipass_drift_pins` (slices 2 + 4),
//!   `glsl_includes::include_drift_pins` (slice 3), and
//!   `scene_adapter::adapter_drift_pins` (slice 4, gated behind
//!   the `scene-adapter` feature).
//! - T2 deterministic UV-gradient corpus in
//!   `engine/render/tests/single_pass.rs` (slice 1), per-pass cascade
//!   corpus in `engine/render/tests/multipass.rs` (slice 2),
//!   Lygia-include end-to-end corpus in
//!   `engine/render/tests/glsl_lygia.rs` (slice 3), feedback-decay /
//!   float-target / scene-adapter corpora in
//!   `engine/render/tests/feedback.rs`,
//!   `engine/render/tests/float_targets.rs`, and
//!   `engine/render/tests/scene_adapter.rs` (slice 4).
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
//! fixtures (slice 5+), backend matrix (slice 5+) — requires
//! successive slices; the row's status stays `Planned` while the
//! harness path tightens. Per-slice row updates land on separate
//! small follow-up commits (this PR is `engine/render` only and does
//! not touch the registry).
//!
//! Governing ADRs:
//!
//! - ADR-011 — rendering backend
//!   (`docs/adr/ADR-011-rendering-backend.md`).
//! - ADR-018 — non-scene UI toolkit
//!   (`docs/adr/ADR-018-non-scene-ui-toolkit.md`).

pub mod glsl_includes;
pub mod multipass;
#[cfg(feature = "scene-adapter")]
pub mod scene_adapter;
pub mod single_pass;

pub use glsl_includes::{IncludeError, MAX_INCLUDE_DEPTH, resolve_includes};
pub use multipass::{
    MultipassSpec, PassDef, PassInputBinding, TargetFormat, UniformInput, UniformValue,
    compile_and_render_multipass,
};
#[cfg(feature = "scene-adapter")]
pub use scene_adapter::{AdapterError, SceneAdapterOptions};
pub use single_pass::{RenderError, compile_and_render_single_pass};
