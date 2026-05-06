// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity authoring environment crate.
//!
//! Phase-4b foundational crate hosting authoring-side concerns that
//! cannot live inside any single engine subsystem. Slice 1 (this PR)
//! ships the [`diagnostics`] module — a UI-friendly structured
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
//! Future authoring concerns expected to land here include the
//! hover-docs registry (per `docs/37` §3.3 §3.5) and the latency
//! harness instrumentation (per `docs/37` §3.3 / row 83
//! `EV-AUTHORING-UX-LATENCY`).

pub mod diagnostics;

pub use diagnostics::{
    Diagnostic, DiagnosticCode, IntoDiagnostic, Severity, Span, diagnostic_from_compile_error,
    diagnostic_from_json_error, diagnostic_from_lint_warning, diagnostic_from_script_error,
    diagnostic_from_validation_error,
};
