// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Authoring diagnostic-renderer surface.
//!
//! Bridges engine-side error types into a UI-friendly
//! [`Diagnostic`] shape consumable by the authoring console
//! ([`docs/37-authoring-environment-spec.md` §3.4]) and the inline
//! diagnostics surface ([`§3.5`]). Four canonical source categories
//! are recognized today, mirroring the
//! `EV-AUTHORING-DIAGNOSTICS` row scope in
//! `docs/26-evidence-artifact-registry.md` row 62: GLSL compile
//! errors, JS-host (script) errors, JSON deserialization errors,
//! and CP-01 coercion warnings. The taxonomy is intentionally
//! `#[non_exhaustive]` so future categories (audio-adapter,
//! OSC/MIDI adapter, media-adapter — see `docs/37` §3.4 line 64)
//! can join without a compat bump.
//!
//! # Surface choice — free functions, not a `From` web
//!
//! Engine error types live across three crates
//! (`chromaticity-render`, `chromaticity-script`,
//! `chromaticity-scene`) that the authoring crate composes but does
//! not modify. A `From<ScriptError> for Diagnostic` impl scattered
//! across those crates would couple every engine subsystem to the
//! authoring surface; an `IntoDiagnostic` trait on the authoring
//! side would couple the same engine types via blanket impls.
//! Slice-1 ships per-source free functions
//! ([`diagnostic_from_compile_error`],
//! [`diagnostic_from_script_error`],
//! [`diagnostic_from_json_error`],
//! [`diagnostic_from_lint_warning`],
//! [`diagnostic_from_validation_error`]) so the conversion lives
//! exclusively in this crate. An [`IntoDiagnostic`] trait is also
//! exposed for callers that prefer trait-method dispatch; it is
//! impl-ed on the same engine error types as a thin wrapper around
//! the free functions. Future categories add a new free function +
//! a new trait impl in lockstep here.
//!
//! # Span surface
//!
//! Today's engine error types do not all carry structured
//! `(line, column)` positions. The [`Span`] surface is widened
//! conservatively:
//!
//! - `serde_json::Error` exposes `line` + `column` directly via
//!   [`serde_json::Error::line`] / [`serde_json::Error::column`];
//!   the JSON mapper threads them through verbatim.
//! - `ScriptError::Compile` and `ScriptError::Runtime` carry the
//!   QuickJS diagnostic verbatim with position embedded inside the
//!   message string ([`engine/script/src/error.rs` line 49]); the
//!   slice-1 surface intentionally does NOT parse line / column out
//!   of the diagnostic — that's a slice-2+ widening when the
//!   author-tooling surface stabilises (see the `error.rs` doc
//!   comment, and `docs/37` §3.5 line 71 on the post-prepend
//!   mapping table).
//! - `RenderError::FragmentParse` / `FragmentValidate` carry the
//!   naga formatted diagnostic; same deferral — the GLSL line
//!   number is embedded in the formatted message string.
//! - `Warning` / `ValidationError` are pre-deserialization /
//!   post-deserialization symbolic diagnostics that point at named
//!   author-supplied identifiers (control names, symbol names,
//!   legacy keys) rather than file positions; their span is
//!   `None` and the `source_path` carries the named identifier
//!   shape via the `hint` field.
//!
//! When the engine error surface widens to carry structured spans,
//! the per-source mapper here updates in lockstep.
//!
//! # Test discipline
//!
//! Module-level test pins exercise one mapping per source category
//! (Compile / Script / JSON / Coercion) plus a handful of
//! invariants on the [`Diagnostic`] shape itself (severity
//! classification, span propagation, code stability). The
//! exhaustive multi-class fixture corpus required by the
//! `EV-AUTHORING-DIAGNOSTICS` registry row is the scope of PR 2
//! of this wave; PR 1 (this module) ships the producer-side
//! surface plus minimal in-source pins demonstrating each error
//! class can be mapped.

use serde::{Deserialize, Serialize};

use chromaticity_render::{IncludeError, RenderError};
use chromaticity_scene::lint::Warning as LintWarning;
use chromaticity_scene::loader::LoadError;
use chromaticity_scene::validator::ValidationError;
use chromaticity_script::ScriptError;

/// Severity ladder mirroring `docs/37` §3.4 line 65: `info`, `warn`,
/// `error`, `coerce`. Coercion is its own ladder rung distinct from
/// `warn` because the authoring-console UI filters and styles the
/// CP-01 coercion class differently from generic warnings (per
/// `docs/37` §3.4 line 67–68).
///
/// `#[non_exhaustive]` keeps the ladder open to future severity
/// classes without a compat bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Severity {
    /// Informational diagnostic. Per `docs/37` §3.8 line 137, the
    /// performance-feedback HUD writes its periodic numbers to the
    /// console at this severity.
    Info,
    /// Warning that does not block the scene from rendering.
    Warning,
    /// Hard failure. Compile / parse / runtime errors that blocked
    /// the operation surface here.
    Error,
    /// CP-01 coercion fired and the source spelling was rewritten.
    /// `docs/37` §3.4 line 67 calls out this as a distinct severity
    /// covering all `CR-01..CR-04` coercions.
    Coerce,
}

/// Source-category code for a [`Diagnostic`].
///
/// Mirrors the four `EV-AUTHORING-DIAGNOSTICS` source classes per
/// `docs/26` row 62 ("compile, script, JSON, and coercion errors").
/// `#[non_exhaustive]` keeps the taxonomy open to the future
/// adapter / runtime classes that `docs/37` §3.4 line 64 enumerates
/// (`OSC/MIDI adapters, audio adapter, media adapter`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum DiagnosticCode {
    /// GLSL fragment-shader compile / validate / include failure.
    /// Source: [`RenderError::FragmentParse`],
    /// [`RenderError::FragmentValidate`], [`IncludeError`].
    Compile,
    /// JS-host (script) error. Source: [`ScriptError`] family —
    /// `Compile`, `Runtime`, `TimeoutExceeded`, `MemoryCapExceeded`,
    /// and the watchdog / determinism / command-bus install variants.
    Script,
    /// `scene.json` deserialization failure. Source:
    /// [`serde_json::Error`] (typically wrapped by
    /// [`LoadError::Json`]).
    Json,
    /// CP-01 coercion / lint diagnostic. Source: scene
    /// [`LintWarning`] and [`ValidationError`].
    Coercion,
    /// Scene-import surface diagnostic. Source: the import-pipeline
    /// surfaces under [`crate::import_isf`] (V1 actionable-reject for
    /// ISF inputs per `docs/38-scene-library-and-import-spec.md` §6) and
    /// future V1 Shadertoy / V2 ISF importers. Distinct from
    /// `DiagnosticCode::Compile` (which surfaces engine-side GLSL parse
    /// failures) because import-time diagnostics fire BEFORE the source
    /// reaches the render pipeline; the file shape itself is the
    /// rejected artefact, not the GLSL it might have produced. Distinct
    /// from `DiagnosticCode::Coercion` because no source spelling is
    /// rewritten — the import is rejected outright with an escape
    /// hatch.
    Import,
}

/// One-based source position. Mirrors the
/// [`serde_json::Error::line`] / [`serde_json::Error::column`]
/// shape (positions are 1-based in serde_json's surface).
///
/// Half-open ranges are not used at slice-1: the scene loader's
/// JSON error and the future GLSL / script error parsers all
/// surface a single `(line, column)` point. When a future engine
/// widening surfaces a real range, [`Span::end_line`] and
/// [`Span::end_col`] become the offset end-point and the span
/// becomes a point span when `end_line == start_line && end_col ==
/// start_col`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    /// One-based source line of the start of the span.
    pub start_line: u32,
    /// One-based source column of the start of the span.
    pub start_col: u32,
    /// One-based source line of the end of the span (inclusive).
    /// Equal to [`Span::start_line`] for point spans.
    pub end_line: u32,
    /// One-based source column of the end of the span (inclusive).
    /// Equal to [`Span::start_col`] for point spans.
    pub end_col: u32,
}

impl Span {
    /// Construct a point span from a single 1-based `(line,
    /// column)` pair. `serde_json::Error` is the canonical caller.
    pub fn point(line: u32, col: u32) -> Self {
        Self {
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col,
        }
    }
}

/// UI-friendly structured authoring diagnostic.
///
/// Composed by per-source mapper functions
/// ([`diagnostic_from_compile_error`] etc.) from the engine error
/// types. The Tauri IPC adapter that exposes this to the Svelte
/// console-panel UI lands in a later wave; the type is `Serialize +
/// Deserialize` today so the serialization shape can be exercised
/// by the PR-2 fixture corpus before the IPC surface is wired.
///
/// `#[non_exhaustive]` is NOT applied to the struct because every
/// field has a stable shape under the slice-1 contract; future
/// fields (e.g. a structured `quick_fix` payload for the inline
/// diagnostic surface per `docs/37` §3.5 line 72) will be added
/// behind feature flags or as `Option<…>` fields that don't break
/// existing readers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity classification per [`Severity`].
    pub severity: Severity,
    /// Source-category code per [`DiagnosticCode`].
    pub code: DiagnosticCode,
    /// Human-readable diagnostic message. Carries the upstream
    /// engine diagnostic verbatim (e.g. the QuickJS `Error.toString`
    /// rendering or the naga formatted parse-failure message).
    pub message: String,
    /// Optional source path (e.g. `main.glsl`, `script.js`,
    /// `scene.json`) the diagnostic originated against. `None`
    /// when the upstream error does not carry path context.
    pub source_path: Option<String>,
    /// Optional source-position span. Populated when the upstream
    /// engine error type carries a structured `(line, column)` —
    /// today only [`serde_json::Error`] does (see the module
    /// docstring's "Span surface" section).
    pub span: Option<Span>,
    /// Optional actionable hint (e.g. the proposed fix for a
    /// `CR-01..CR-04` coercion per `docs/37` §3.5 line 72, or the
    /// rewritten canonical key for a `legacy-key` warning).
    pub hint: Option<String>,
}

/// Convert an engine-side error type into a [`Diagnostic`].
///
/// Implemented by the authoring crate for each known engine error
/// source as a thin wrapper around the per-source free function. A
/// trait-method dispatch is offered for callers that prefer it over
/// matching against the source type explicitly; the free functions
/// remain the canonical surface and are usable independently.
pub trait IntoDiagnostic {
    /// Render `self` as a UI-friendly [`Diagnostic`].
    fn into_diagnostic(self) -> Diagnostic;
}

// -------------------------------------------------------------------
// Per-source free functions.
// -------------------------------------------------------------------

/// Map a [`RenderError`] (GLSL compile / validate / include path)
/// into a `Compile`-class [`Diagnostic`].
///
/// Variants outside the GLSL compile path
/// ([`RenderError::AdapterUnavailable`], [`RenderError::DeviceLoss`],
/// etc.) are still mapped to a `Compile`-class diagnostic at
/// `Severity::Error` because they surface during the same author
/// "save → recompile → render" cycle and the operator console treats
/// them as a render-pipeline failure category. Future widenings may
/// split these out into a dedicated `DiagnosticCode::Render` rung.
pub fn diagnostic_from_compile_error(err: &RenderError) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        code: DiagnosticCode::Compile,
        message: err.to_string(),
        source_path: Some("main.glsl".to_string()),
        span: None,
        hint: None,
    }
}

/// Map a GLSL include-resolution [`IncludeError`] into a
/// `Compile`-class [`Diagnostic`]. Surfaces the `#include` failure
/// in the same console category as the downstream parse / validate
/// failures it would have produced if the include had resolved.
pub fn diagnostic_from_include_error(err: &IncludeError) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        code: DiagnosticCode::Compile,
        message: err.to_string(),
        source_path: Some("main.glsl".to_string()),
        span: None,
        hint: None,
    }
}

/// Map a [`ScriptError`] into a `Script`-class [`Diagnostic`].
///
/// Severity defaults to `Error` for every variant. The QuickJS
/// position (line / column) is embedded inside the diagnostic
/// string verbatim; slice-1 deliberately does NOT parse a span out
/// of the message. See the module docstring's "Span surface"
/// section.
pub fn diagnostic_from_script_error(err: &ScriptError) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        code: DiagnosticCode::Script,
        message: err.to_string(),
        source_path: Some("script.js".to_string()),
        span: None,
        hint: None,
    }
}

/// Map a [`serde_json::Error`] into a `Json`-class [`Diagnostic`].
///
/// `serde_json` exposes 1-based [`serde_json::Error::line`] /
/// [`serde_json::Error::column`] for every parse / data error;
/// they're threaded through to a [`Span::point`] verbatim. A `(0,
/// 0)` reading from serde_json (its sentinel for "no position
/// available") collapses to `span: None` so downstream UI
/// consumers don't render a phantom line-1-col-0 marker.
pub fn diagnostic_from_json_error(err: &serde_json::Error) -> Diagnostic {
    let line = u32::try_from(err.line()).unwrap_or(0);
    let col = u32::try_from(err.column()).unwrap_or(0);
    let span = if line == 0 && col == 0 {
        None
    } else {
        Some(Span::point(line, col))
    };
    Diagnostic {
        severity: Severity::Error,
        code: DiagnosticCode::Json,
        message: err.to_string(),
        source_path: Some("scene.json".to_string()),
        span,
        hint: None,
    }
}

/// Map a scene [`LoadError`] into a [`Diagnostic`].
///
/// `LoadError::Json` rounds through [`diagnostic_from_json_error`]
/// (the underlying `serde_json::Error` carries the position).
/// Other variants (I/O failure, missing `scene.json`, unsupported
/// package extension) map to a `Json`-class error since they all
/// surface against the `scene.json` package-loader contract — they
/// share the JSON-class console rung today; a future widening may
/// introduce a `Package` rung for the directory-shape failures.
pub fn diagnostic_from_load_error(err: &LoadError) -> Diagnostic {
    match err {
        LoadError::Json(json_err) => diagnostic_from_json_error(json_err),
        other => Diagnostic {
            severity: Severity::Error,
            code: DiagnosticCode::Json,
            message: other.to_string(),
            source_path: Some("scene.json".to_string()),
            span: None,
            hint: None,
        },
    }
}

/// Map a scene [`LintWarning`] (CP-01 / pre-deserialization
/// compatibility warning) into a `Coercion`-class [`Diagnostic`].
///
/// Severity is [`Severity::Coerce`] for the CP-01
/// `coercion-applied` and `legacy-key` variants — the cases where
/// a source spelling was silently rewritten — and
/// [`Severity::Warning`] for the diagnostic-only variants
/// (`unknown-control-type`, `deprecated-script-api`).
///
/// The `hint` field carries the proposed fix where one is
/// available (the rewritten canonical key for `legacy-key`, the
/// coerced control kind for `coercion-applied`); the inline
/// diagnostics surface in `docs/37` §3.5 line 72 reads it.
pub fn diagnostic_from_lint_warning(warning: &LintWarning) -> Diagnostic {
    let (severity, message, source_path, hint) = match warning {
        LintWarning::LegacyKey { canonical, legacy } => (
            Severity::Coerce,
            format!("legacy key '{legacy}' rewritten to canonical '{canonical}'"),
            Some(format!("scene.json:{legacy}")),
            Some(format!("rename to '{canonical}'")),
        ),
        LintWarning::UnknownControlType { control_name } => (
            Severity::Warning,
            format!("unknown control type for control '{control_name}'"),
            Some(format!("scene.json:CONTROLS[{control_name}]")),
            None,
        ),
        LintWarning::DeprecatedScriptApi { api } => (
            Severity::Warning,
            format!("deprecated script API: {api}"),
            Some("script.js".to_string()),
            Some(format!("avoid '{api}' in native-chroma-v1 scenes")),
        ),
        LintWarning::CoercionApplied {
            control_name,
            from,
            to,
        } => (
            Severity::Coerce,
            format!("coercion applied to control '{control_name}': {from} → {to}"),
            Some(format!("scene.json:CONTROLS[{control_name}]")),
            Some(format!("declare TYPE: '{to}' explicitly")),
        ),
        // `Warning` is `#[non_exhaustive]`; future variants land
        // here as a generic `Coercion` mapping until this match
        // arm is widened in lockstep.
        other => (
            Severity::Warning,
            format!("{other:?}"),
            Some("scene.json".to_string()),
            None,
        ),
    };
    Diagnostic {
        severity,
        code: DiagnosticCode::Coercion,
        message,
        source_path,
        span: None,
        hint,
    }
}

/// Map a scene [`ValidationError`] into a `Coercion`-class
/// [`Diagnostic`] at [`Severity::Error`].
///
/// `ValidationError` is the post-deserialization strict-mode
/// rejection counterpart to the pre-deserialization
/// [`LintWarning`] taxonomy; both share the `Coercion` console
/// rung because they're both authored against CP-01 contract
/// language. The strict-mode rejections are hard errors rather
/// than coercions — the source spelling was NOT rewritten — so
/// severity is `Error`, not `Coerce`.
pub fn diagnostic_from_validation_error(err: &ValidationError) -> Diagnostic {
    let (message, source_path) = match err {
        ValidationError::UnknownSymbol(symbol) => (
            format!("unknown reactive symbol '{symbol}'"),
            Some(format!("scene.json:UNIFORM_BINDINGS[{symbol}]")),
        ),
        ValidationError::MissingControlType(name) => (
            format!("control '{name}' missing TYPE under strict mode"),
            Some(format!("scene.json:CONTROLS[{name}]")),
        ),
        ValidationError::DuplicateNode(name) => (
            format!("duplicate scene node '{name}'"),
            Some(format!("scene.json:PASSES[{name}]")),
        ),
        // `ValidationError` is `#[non_exhaustive]`; future variants
        // fall through to a generic mapping until widened here.
        other => (format!("{other:?}"), Some("scene.json".to_string())),
    };
    Diagnostic {
        severity: Severity::Error,
        code: DiagnosticCode::Coercion,
        message,
        source_path,
        span: None,
        hint: None,
    }
}

// -------------------------------------------------------------------
// IntoDiagnostic trait impls — thin wrappers around the free
// functions above.
// -------------------------------------------------------------------

impl IntoDiagnostic for &RenderError {
    fn into_diagnostic(self) -> Diagnostic {
        diagnostic_from_compile_error(self)
    }
}

impl IntoDiagnostic for &IncludeError {
    fn into_diagnostic(self) -> Diagnostic {
        diagnostic_from_include_error(self)
    }
}

impl IntoDiagnostic for &ScriptError {
    fn into_diagnostic(self) -> Diagnostic {
        diagnostic_from_script_error(self)
    }
}

impl IntoDiagnostic for &serde_json::Error {
    fn into_diagnostic(self) -> Diagnostic {
        diagnostic_from_json_error(self)
    }
}

impl IntoDiagnostic for &LoadError {
    fn into_diagnostic(self) -> Diagnostic {
        diagnostic_from_load_error(self)
    }
}

impl IntoDiagnostic for &LintWarning {
    fn into_diagnostic(self) -> Diagnostic {
        diagnostic_from_lint_warning(self)
    }
}

impl IntoDiagnostic for &ValidationError {
    fn into_diagnostic(self) -> Diagnostic {
        diagnostic_from_validation_error(self)
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-AUTHORING-DIAGNOSTICS (T1) — slice-1 producer-side
    //! diagnostic-renderer surface. One mapping pin per source category
    //! (Compile / Script / JSON / Coercion) plus invariants on severity
    //! classification, span propagation, source-path coverage, and
    //! `IntoDiagnostic` trait dispatch matching the free-function
    //! surface. The exhaustive multi-class fixture corpus required by
    //! the registry row 62 is the scope of PR 2 of this wave; this
    //! module establishes the surface plus a minimal coverage floor.
    //!
    //! Tier rationale per `docs/43` §15.5: T1 = "in-source pin against
    //! a producer surface; no external corpus or replay harness". The
    //! fixture corpus is T2 and lands separately.
    //!
    //! Per-test mappings:
    //! - Compile class: synthesised `RenderError::FragmentParse` →
    //!   `Compile` / `Error`.
    //! - Script class: synthesised `ScriptError::Compile` →
    //!   `Script` / `Error`.
    //! - JSON class: real `serde_json::from_str` failure on malformed
    //!   input → `Json` / `Error` with non-`None` span (line / column
    //!   threaded through).
    //! - Coercion class — `legacy-key`: `LintWarning::LegacyKey` →
    //!   `Coercion` / `Coerce` with `hint`.
    //! - Coercion class — `coercion-applied`: `LintWarning::CoercionApplied`
    //!   → `Coercion` / `Coerce` with `hint` carrying the target kind.
    //! - Coercion class — `validation-error`: `ValidationError::UnknownSymbol`
    //!   → `Coercion` / `Error`.
    //! - Round-trip: `Diagnostic` Serialize / Deserialize round-trip
    //!   preserves equality.
    //! - Trait dispatch: `(&ScriptError).into_diagnostic()` matches
    //!   the free-function output.

    use super::*;

    #[test]
    fn compile_class_maps_render_error_to_error_severity() {
        let err = RenderError::FragmentParse("unexpected token at line 5 col 12".to_string());
        let d = diagnostic_from_compile_error(&err);
        assert_eq!(d.code, DiagnosticCode::Compile);
        assert_eq!(d.severity, Severity::Error);
        assert!(d.message.contains("unexpected token"));
        assert_eq!(d.source_path.as_deref(), Some("main.glsl"));
        assert!(d.span.is_none());
    }

    #[test]
    fn script_class_maps_script_error_to_error_severity() {
        let err = ScriptError::Compile("SyntaxError: incomplete statement".to_string());
        let d = diagnostic_from_script_error(&err);
        assert_eq!(d.code, DiagnosticCode::Script);
        assert_eq!(d.severity, Severity::Error);
        assert!(d.message.contains("SyntaxError"));
        assert_eq!(d.source_path.as_deref(), Some("script.js"));
    }

    #[test]
    fn json_class_threads_serde_json_line_column_through_span() {
        let raw = "{\n  \"PASSES\": [\n    invalid\n  ]\n}";
        let err = serde_json::from_str::<serde_json::Value>(raw).unwrap_err();
        let d = diagnostic_from_json_error(&err);
        assert_eq!(d.code, DiagnosticCode::Json);
        assert_eq!(d.severity, Severity::Error);
        let span = d.span.expect("serde_json error carries position");
        assert!(span.start_line > 0, "expected 1-based line number");
        assert!(span.start_col > 0, "expected 1-based column number");
        assert_eq!(span.start_line, span.end_line);
        assert_eq!(span.start_col, span.end_col);
        assert_eq!(d.source_path.as_deref(), Some("scene.json"));
    }

    #[test]
    fn coercion_class_legacy_key_emits_coerce_severity_with_hint() {
        let warning = LintWarning::LegacyKey {
            canonical: "IMAGE_PATH".to_string(),
            legacy: "IMAGE".to_string(),
        };
        let d = diagnostic_from_lint_warning(&warning);
        assert_eq!(d.code, DiagnosticCode::Coercion);
        assert_eq!(d.severity, Severity::Coerce);
        assert!(d.message.contains("IMAGE"));
        assert!(d.message.contains("IMAGE_PATH"));
        let hint = d.hint.expect("legacy-key carries a fix hint");
        assert!(hint.contains("IMAGE_PATH"));
    }

    #[test]
    fn coercion_class_coercion_applied_carries_target_kind_in_hint() {
        let warning = LintWarning::CoercionApplied {
            control_name: "intensity".to_string(),
            from: "missing-type".to_string(),
            to: "slider".to_string(),
        };
        let d = diagnostic_from_lint_warning(&warning);
        assert_eq!(d.code, DiagnosticCode::Coercion);
        assert_eq!(d.severity, Severity::Coerce);
        let hint = d.hint.expect("coercion-applied carries a fix hint");
        assert!(hint.contains("slider"));
    }

    #[test]
    fn coercion_class_validation_error_emits_error_severity() {
        let err = ValidationError::UnknownSymbol("chroma_NotASymbol".to_string());
        let d = diagnostic_from_validation_error(&err);
        assert_eq!(d.code, DiagnosticCode::Coercion);
        assert_eq!(d.severity, Severity::Error);
        assert!(d.message.contains("chroma_NotASymbol"));
    }

    #[test]
    fn coercion_class_unknown_control_type_emits_warning_severity() {
        let warning = LintWarning::UnknownControlType {
            control_name: "phantom".to_string(),
        };
        let d = diagnostic_from_lint_warning(&warning);
        assert_eq!(d.code, DiagnosticCode::Coercion);
        assert_eq!(d.severity, Severity::Warning);
        assert!(d.hint.is_none());
    }

    #[test]
    fn diagnostic_round_trips_through_json_serialization() {
        let original = Diagnostic {
            severity: Severity::Coerce,
            code: DiagnosticCode::Coercion,
            message: "round-trip pin".to_string(),
            source_path: Some("scene.json".to_string()),
            span: Some(Span::point(3, 7)),
            hint: Some("apply suggested fix".to_string()),
        };
        let wire = serde_json::to_string(&original).unwrap();
        let restored: Diagnostic = serde_json::from_str(&wire).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn into_diagnostic_trait_matches_free_function_for_script_error() {
        let err = ScriptError::Runtime("threw uncaught Error: boom".to_string());
        let direct = diagnostic_from_script_error(&err);
        let via_trait = (&err).into_diagnostic();
        assert_eq!(direct, via_trait);
    }

    #[test]
    fn load_error_json_variant_routes_through_serde_json_mapper() {
        let raw = "{ not even close }";
        let json_err = serde_json::from_str::<serde_json::Value>(raw).unwrap_err();
        let load_err = LoadError::Json(json_err);
        let d = diagnostic_from_load_error(&load_err);
        assert_eq!(d.code, DiagnosticCode::Json);
        assert!(d.span.is_some(), "JSON variant must thread span through");
    }
}
