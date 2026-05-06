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
//! Slice 2 (PR #355) shipped the [`scaffolder`] module — a New-Scene
//! scaffolder that produces a minimal `.chroma` package skeleton
//! consumable by `chromaticity_scene::loader::load_chroma_dir` per
//! `docs/26` row 63 (`EV-AUTHORING-SCAFFOLDER`).
//!
//! Slice 3 (PR #356) shipped the [`live_reload`] module — a cross-
//! platform file-watcher around the `notify` crate plus a reload
//! pipeline composing the [`diagnostics`] surface to report load
//! failures as structured diagnostics. PR 1 of 2-3 for row 61
//! (`EV-AUTHORING-LIVE-RELOAD`).
//!
//! Slice 4 (PR #358) shipped the [`import_isf`] module — the V1
//! actionable-reject diagnostic + escape hatch for ISF (Interactive
//! Shader Format) inputs per `docs/26` row 65 (`EV-IMPORT-ISF-REJECT`)
//! and `docs/38-scene-library-and-import-spec.md` §6. ISF native import
//! is a V2 differentiation bet (`CHR-ISF-V2`); V1 detects the ISF
//! fingerprint and rejects with a structured `Diagnostic` pointing the
//! author at the Shadertoy converter (V1) or a blank-scene-scaffold
//! hand-port path.
//!
//! Slice 5 (this PR) ships the [`import_shadertoy`] module — the V1
//! Shadertoy `.chroma` import converter per `docs/26` row 64
//! (`EV-IMPORT-SHADERTOY`) and `docs/38` §5.2. PR 1 of 2 parses the
//! Shadertoy `mainImage` entry point and maps Shadertoy uniforms to
//! chromaticity preamble uniforms via `layout(binding=N)` declarations,
//! validates through naga GLSL frontend, and writes a `.chroma`
//! package at the target path. The runtime binding to
//! chromaticity-native reactive symbols (`TIME`, `RENDERSIZE`) is a
//! downstream binding-layer concern; PR 2 or a later wave wires
//! bindings 0 through 4 to the appropriate reactive symbols without
//! touching the converter's text-rewrite surface.
//!
//! Future authoring concerns expected to land here include the
//! hover-docs registry (per `docs/37` §3.3 §3.5) and the latency
//! harness instrumentation (per `docs/37` §3.3 / row 83
//! `EV-AUTHORING-UX-LATENCY`).

pub mod diagnostics;
pub mod import_isf;
pub mod import_shadertoy;
pub mod live_reload;
pub mod scaffolder;

pub use diagnostics::{
    Diagnostic, DiagnosticCode, IntoDiagnostic, Severity, Span, diagnostic_from_compile_error,
    diagnostic_from_json_error, diagnostic_from_lint_warning, diagnostic_from_script_error,
    diagnostic_from_validation_error,
};
pub use import_isf::{build_isf_reject_diagnostic, detect_isf, try_reject_isf_input};
pub use import_shadertoy::{
    ImportError, ImportReport, ShadertoyImportConfig, import_shadertoy_to_chroma,
    load_imported_scene,
};
pub use live_reload::{
    LiveReloadConfig, LiveReloadError, LiveReloadHandle, ReloadEvent, ReloadOutcome,
    perform_reload, start_watching,
};
pub use scaffolder::{
    ScaffolderConfig, ScaffolderError, ScaffolderReport, load_scaffolded_scene, scaffold_to_path,
};
