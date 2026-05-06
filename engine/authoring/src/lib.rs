// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity authoring environment crate.
//!
//! Phase-4b foundational crate hosting authoring-side concerns that
//! cannot live inside any single engine subsystem. Slice 1 (PR #353)
//! shipped the [`diagnostics`] module — a UI-friendly structured
//! diagnostic surface that bridges engine-side errors from
//! `chromaticity-render`, `chromaticity-script`, and `chromaticity-scene`
//! into the four canonical authoring-console categories
//! (compile, script, JSON, coercion) per
//! `docs/37-authoring-environment-spec.md` §3.4 + §3.5.
//!
//! The diagnostic surface is producer-side: it composes existing
//! engine error types without modifying them. The Tauri / UI
//! consumer-side wiring (a `#[tauri::command]` adapter and the
//! Svelte console-panel renderer) lands in a follow-up wave once the
//! authoring view's port surface stabilises.
//!
//! Slice 2 (PR #355) shipped the [`scaffolder`] module — a New-Scene
//! scaffolder that produces a minimal `.chroma` package skeleton
//! consumable by `chromaticity_scene::loader::load_chroma_dir` and
//! whose embedded fragment shader compiles + validates through the
//! same naga GLSL frontend + IR validator pair the engine/render
//! single-pass entry point uses internally. Slice scope per
//! `docs/26-evidence-artifact-registry.md` row 63
//! (`EV-AUTHORING-SCAFFOLDER`): scaffolder template + golden-output
//! corpus, single PR.
//!
//! Slice 3 (this PR) ships the [`live_reload`] module — a cross-
//! platform file-watcher around the `notify` crate plus a reload
//! pipeline composing the [`diagnostics`] surface to report load
//! failures as structured diagnostics. PR 1 of 2-3 for row 61
//! (`EV-AUTHORING-LIVE-RELOAD`): file-watcher + reload pipeline + T1
//! pins. State preservation is PR 2; the latency-budget T2 corpus is
//! PR 3.
//!
//! Future authoring concerns expected to land here include the
//! hover-docs registry (per `docs/37` §3.3 §3.5) and the latency
//! harness instrumentation (per `docs/37` §3.3 / row 83
//! `EV-AUTHORING-UX-LATENCY`).

pub mod diagnostics;
pub mod live_reload;
pub mod scaffolder;

pub use diagnostics::{
    Diagnostic, DiagnosticCode, IntoDiagnostic, Severity, Span, diagnostic_from_compile_error,
    diagnostic_from_json_error, diagnostic_from_lint_warning, diagnostic_from_script_error,
    diagnostic_from_validation_error,
};
pub use live_reload::{
    LiveReloadConfig, LiveReloadError, LiveReloadHandle, ReloadEvent, ReloadOutcome,
    perform_reload, start_watching,
};
pub use scaffolder::{
    ScaffolderConfig, ScaffolderError, ScaffolderReport, load_scaffolded_scene, scaffold_to_path,
};
