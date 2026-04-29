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
//! - **Slice 1 (this PR)** — single-pass entry point
//!   ([`single_pass::compile_and_render_single_pass`]) consuming
//!   hand-written GLSL fragment source, hardcoded full-screen-triangle
//!   vertex shader, RGBA8Unorm offscreen target with pixel readback,
//!   structured [`single_pass::RenderError`] for naga / wgpu /
//!   device-loss failures.
//! - **Slice 2 (next)** — `engine/scene::PassSpec` consumer and the
//!   first multi-pass scheduler.
//! - **Slice 3** — feedback-texture sampling and `RGBA32F` /
//!   `RGBA16F` float-target conformance per ADR-011 §Decision.
//! - **Slice 4+** — transition orchestration (ADR-007 warm-switch),
//!   audio-uniform binding (`engine/audio` integration), Metal /
//!   DX12 backend-matrix conformance.
//!
//! The Lygia GLSL trial decision (`feedback_lygia_trial_deferred_legal`
//! auto-memory) does NOT apply to slice 1. Slice 1 ships hand-written
//! minimal GLSL only; vendoring Lygia is a follow-up wave with its
//! own REUSE.toml carve-out.
//!
//! Produces evidence for `EV-RENDER-CONFORMANCE` (T1 entry-point
//! drift pins in `single_pass::compile_drift_pins`; T2 deterministic
//! UV-gradient corpus in `engine/render/tests/single_pass.rs`). Per
//! `docs/26-evidence-artifact-registry.md` row 28's Phase-2-exit
//! definition, full conformance — multipass, feedback, transition
//! fixtures, backend matrix — requires successive slices; slice 1's
//! status remains `Planned` while the harness path tightens.
//!
//! Governing ADRs:
//!
//! - ADR-011 — rendering backend
//!   (`docs/adr/ADR-011-rendering-backend.md`).
//! - ADR-018 — non-scene UI toolkit
//!   (`docs/adr/ADR-018-non-scene-ui-toolkit.md`).

pub mod single_pass;

pub use single_pass::{RenderError, compile_and_render_single_pass};
