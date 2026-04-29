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
//! - **Slice 2 (this PR)** — multi-pass scheduler entry point
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
//! - **Slice 3** — feedback-texture sampling (a pass reading its
//!   own previous-frame output), `RGBA32F` / `RGBA16F` float-target
//!   conformance per ADR-011 §Decision, and the
//!   `engine/scene::PassSpec` → [`multipass::MultipassSpec`] adapter.
//! - **Slice 4+** — transition orchestration (ADR-007 warm-switch),
//!   audio-uniform binding (`engine/audio` integration), Metal /
//!   DX12 backend-matrix conformance.
//!
//! The Lygia GLSL trial decision (`feedback_lygia_trial_deferred_legal`
//! auto-memory) does NOT apply to slice 1. Slice 1 ships hand-written
//! minimal GLSL only; vendoring Lygia is a follow-up wave with its
//! own REUSE.toml carve-out.
//!
//! Produces evidence for `EV-RENDER-CONFORMANCE`:
//!
//! - T1 entry-point drift pins in `single_pass::compile_drift_pins`
//!   (slice 1) and `multipass::multipass_drift_pins` (slice 2).
//! - T2 deterministic UV-gradient corpus in
//!   `engine/render/tests/single_pass.rs` (slice 1) and per-pass
//!   cascade corpus in `engine/render/tests/multipass.rs` (slice 2).
//!
//! Per `docs/26-evidence-artifact-registry.md` row 28's Phase-2-exit
//! definition, full conformance — feedback, transition fixtures,
//! backend matrix — requires successive slices; the row's status
//! stays `Planned` while the harness path tightens. The row update
//! after slice 2 lands on a separate small follow-up commit (this
//! PR is `engine/render` only and does not touch the registry).
//!
//! Governing ADRs:
//!
//! - ADR-011 — rendering backend
//!   (`docs/adr/ADR-011-rendering-backend.md`).
//! - ADR-018 — non-scene UI toolkit
//!   (`docs/adr/ADR-018-non-scene-ui-toolkit.md`).

pub mod multipass;
pub mod single_pass;

pub use multipass::{
    MultipassSpec, PassDef, PassInputBinding, UniformInput, UniformValue,
    compile_and_render_multipass,
};
pub use single_pass::{RenderError, compile_and_render_single_pass};
