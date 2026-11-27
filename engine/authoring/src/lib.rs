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
//! Slice 5 (PR #361) shipped the [`import_shadertoy`] module — the
//! V1 Shadertoy `.chroma` import converter per `docs/26` row 64
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
//! Slice 6 (PR #363 / `b2156bb`) shipped the [`ux_latency`] module — the
//! co-developed §3.3a UX latency measurement harness satisfying
//! both `EV-AUTHORING-UX-LATENCY` (`docs/26` row 83) and
//! `EV-DOCS-UX-LATENCY` (row 84) per AO direction "co-develop the
//! harness once and use it for both rows". The two rows share the
//! §3.3a budget table verbatim — same hover-docs (80 ms p95),
//! symbol-search / doc-search (150 ms p95), and first-paint
//! (300 ms p95) targets per `docs/37-authoring-environment-spec.md`
//! §3.3a — so a single producer-side harness is the correct shape.
//! The harness is synthetic: it ships the measurement primitives
//! plus the static budget tables ([`ux_latency::AUTHORING_BUDGETS`]
//! / [`ux_latency::DOCS_BUDGETS`]) plus the comparator seam
//! ([`ux_latency::assert_p95_within_budget`]); the UI-driven
//! instrumentation feeding real authoring-view and embedded-docs-
//! view samples lands as post-MVP integration work.
//!
//! Future authoring concerns expected to land here include the
//! hover-docs registry (per `docs/37` §3.3 §3.5).

pub mod diagnostics;
pub mod import_isf;
pub mod import_reference;
pub mod import_shadertoy;
pub mod live_reload;
pub mod scaffolder;
pub mod shadertoy_license;
pub mod ux_latency;

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
    LiveReloadConfig, LiveReloadError, LiveReloadHandle, PreservedState, PreservedStateSnapshot,
    ReloadEvent, ReloadOutcome, capture_preserved_state, perform_reload, start_watching,
};
pub use scaffolder::{
    ScaffolderConfig, ScaffolderError, ScaffolderReport, load_scaffolded_scene, scaffold_to_path,
};
pub use ux_latency::{
    AUTHORING_BUDGETS, BudgetExceeded, DOCS_BUDGETS, LatencyBudget, LatencyMeasurement,
    assert_p95_within_budget, measure_p95,
};
