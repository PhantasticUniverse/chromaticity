// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! ISF (Interactive Shader Format) input reject surface.
//!
//! Phase-4b Wave 4b.5 Lane B — V1 actionable-reject diagnostic for ISF
//! inputs per `docs/38-scene-library-and-import-spec.md` §6. ISF is the
//! Interactive Shader Format used by VDMX et al; a `.fs` / `.isf` file
//! starts with a `/* ... */` comment block that wraps a JSON metadata
//! header (declaring `ISFVSN`, `INPUTS`, `PASSES`, `DESCRIPTION`,
//! `CREDIT`, `CATEGORIES`) followed by GLSL fragment-shader source.
//!
//! # V1 stance — reject with escape hatch
//!
//! V1 deliberately does NOT import ISF: ISF's `INPUTS` / `PASSES`
//! semantics overlap syntactically with SSF / Chromaticity-native
//! shapes but diverge meaningfully (`INPUTS` ≠ `CONTROLS`, persistent
//! buffer model differs), and a half-working importer would erode
//! trust per `docs/38` §6.2. The surface here detects the ISF
//! fingerprint, builds a structured `Diagnostic` rejecting the input,
//! and provides the escape-hatch hint pointing the author at the
//! Shadertoy converter (V1) or the V2 native ISF importer
//! (`CHR-ISF-V2`).
//!
//! # Detection
//!
//! [`detect_isf`] scans for the canonical ISF fingerprint:
//!
//! 1. A `/* ... */` block comment at the top of the file containing a
//!    JSON object.
//! 2. The JSON object declares at least one of `ISFVSN` (the ISF
//!    version key, unique to ISF) or co-occurring `PASSES` + `INPUTS`
//!    arrays (which together strongly indicate ISF semantics over
//!    pure-Shadertoy or pure-GLSL files).
//!
//! The detector is intentionally lexical (string-search inside the
//! initial comment block) rather than running a full JSON parser:
//! malformed ISF files still need to be detected and rejected with a
//! useful diagnostic, and a parse-then-decide path would mis-classify
//! them as "not ISF" and try other importers.
//!
//! # Diagnostic shape
//!
//! [`build_isf_reject_diagnostic`] emits a [`Diagnostic`] with:
//!
//! - `severity = Severity::Error` — the import is rejected outright;
//!   the file does not reach the scene loader.
//! - `code = DiagnosticCode::Import` — distinct from the engine-side
//!   `Compile` / `Json` / `Coercion` rungs since the rejection fires
//!   pre-load. See the `DiagnosticCode::Import` doc comment for the
//!   rationale on why this is its own variant.
//! - `message` — references the V1-deferred stance plus the
//!   `CHR-ISF-V2` tracking issue per `docs/38` §6.3.
//! - `hint` — the actionable escape hatch: convert via the ISF Editor's
//!   "Export as Shadertoy" option then use Chromaticity's Shadertoy
//!   importer, OR paste the ISF body into a blank Chromaticity scene
//!   scaffold for hand-port.
//! - `source_path` — the rejected file's path when available.
//!
//! When the caller passes detected-feature labels (e.g. `["ISFVSN",
//! "INPUTS array", "PASSES array"]`), they're folded into the message
//! so the author sees which ISF features triggered the rejection. This
//! is non-load-bearing prose; the rejection fires regardless.

use std::path::Path;

use crate::diagnostics::{Diagnostic, DiagnosticCode, Severity};

/// Canonical V1 reject message preface per `docs/38` §6.3. The
/// per-fixture message may append detected-feature labels on the end.
const REJECT_MESSAGE_BASE: &str = "ISF input format detected. Chromaticity does not import ISF directly in V1. \
     ISF scenes are not yet supported; Chromaticity V2 will support ISF natively. \
     Tracking issue: CHR-ISF-V2.";

/// Canonical actionable escape-hatch hint per `docs/38` §6.3 and the
/// Lane B brief. Two paths offered: convert via the ISF Editor's
/// "Export as Shadertoy" option (preferred — round-trips the canonical
/// `mainImage` body) and use Chromaticity's V1 Shadertoy importer; OR
/// paste the ISF body as a comment in a blank Chromaticity scene
/// scaffold for hand-port.
const REJECT_HINT: &str = "Convert to Shadertoy format via the ISF Editor's 'Export as Shadertoy' option, \
     then use Chromaticity's Shadertoy importer; or paste the ISF body into a new \
     blank Chromaticity scene scaffold (File → New Scene) for hand-port.";

/// Detect whether `content` is an ISF input.
///
/// Returns `true` when the source carries an ISF header fingerprint:
/// a top-of-file `/* ... */` block comment containing a JSON object
/// that declares either `ISFVSN` (the ISF Version key, unique to ISF)
/// or co-occurring `PASSES` + `INPUTS` arrays.
///
/// Returns `false` for pure-GLSL inputs, pure-Shadertoy inputs (which
/// don't carry a JSON header), and inputs whose initial comment lacks
/// the ISF marker keys.
///
/// The detection is intentionally lexical and tolerant of malformed
/// JSON inside the comment block: malformed ISF still returns `true`
/// here so the caller surfaces a useful reject diagnostic rather than
/// silently routing the file to a different (and incorrect) importer.
pub fn detect_isf(content: &str) -> bool {
    // Look for a leading block comment. ISF requires the metadata
    // header to be the very first thing in the file (modulo
    // whitespace), so a `/*` after non-whitespace, non-`/*` content
    // disqualifies.
    let trimmed = content.trim_start();
    let Some(after_open) = trimmed.strip_prefix("/*") else {
        return false;
    };
    // Bound the search to the first comment-block close.
    let Some(close_idx) = after_open.find("*/") else {
        return false;
    };
    let header = &after_open[..close_idx];

    // Canonical ISF fingerprint: `"ISFVSN"` is unique to ISF.
    if header.contains("\"ISFVSN\"") {
        return true;
    }

    // Co-occurring `PASSES` + `INPUTS` is strongly indicative even
    // when `ISFVSN` was omitted (some hand-authored ISF files skip
    // it). The presence of both arrays inside the header comment
    // distinguishes ISF from pure-GLSL `/* license header */`
    // comments and from Shadertoy paste-snippets.
    header.contains("\"PASSES\"") && header.contains("\"INPUTS\"")
}

/// Build the canonical ISF reject [`Diagnostic`].
///
/// `source_path` populates `Diagnostic::source_path`; pass `None` when
/// the rejection fires against a paste-buffer or in-memory shader
/// rather than an on-disk file.
///
/// `detected_features` may carry human-readable labels of the ISF
/// features that triggered the detection (e.g. `["ISFVSN"]`,
/// `["INPUTS array", "PASSES array"]`). When non-empty they're folded
/// into the message tail; the rejection fires identically regardless
/// of feature labels.
pub fn build_isf_reject_diagnostic(
    source_path: Option<&Path>,
    detected_features: &[&str],
) -> Diagnostic {
    let mut message = REJECT_MESSAGE_BASE.to_string();
    if !detected_features.is_empty() {
        message.push_str(" Detected ISF features: ");
        message.push_str(&detected_features.join(", "));
        message.push('.');
    }

    Diagnostic {
        severity: Severity::Error,
        code: DiagnosticCode::Import,
        message,
        source_path: source_path.map(|p| p.display().to_string()),
        span: None,
        hint: Some(REJECT_HINT.to_string()),
    }
}

/// Convenience: detect-and-reject in one call.
///
/// Returns `Some(diagnostic)` when [`detect_isf`] fingerprints the
/// content as ISF; the caller short-circuits the rest of the import
/// pipeline and surfaces the diagnostic to the author. Returns `None`
/// when the content is not ISF, leaving the caller free to delegate
/// to the Shadertoy importer or other parsers.
///
/// The detected-features array passed to [`build_isf_reject_diagnostic`]
/// is auto-derived from the content fingerprint: `ISFVSN` and / or
/// `INPUTS` / `PASSES` array markers when present in the header.
pub fn try_reject_isf_input(source_path: &Path, content: &str) -> Option<Diagnostic> {
    if !detect_isf(content) {
        return None;
    }

    // Derive the human-readable feature labels from the same lexical
    // scan `detect_isf` performed. Cheap re-extraction of the header
    // to avoid leaking detector internals through the API surface.
    let trimmed = content.trim_start();
    let header = trimmed
        .strip_prefix("/*")
        .and_then(|after_open| after_open.find("*/").map(|close| &after_open[..close]))
        .unwrap_or("");

    let mut features: Vec<&str> = Vec::new();
    if header.contains("\"ISFVSN\"") {
        features.push("ISFVSN");
    }
    if header.contains("\"INPUTS\"") {
        features.push("INPUTS array");
    }
    if header.contains("\"PASSES\"") {
        features.push("PASSES array");
    }

    Some(build_isf_reject_diagnostic(Some(source_path), &features))
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-IMPORT-ISF-REJECT (T1) — Phase-4b Wave 4b.5 Lane B
    //! actionable-reject diagnostic + escape hatch for ISF inputs per
    //! `docs/26-evidence-artifact-registry.md` row 65 and
    //! `docs/38-scene-library-and-import-spec.md` §6.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: in-source pins
    //! against the producer surface (`detect_isf`,
    //! `build_isf_reject_diagnostic`, `try_reject_isf_input`). The
    //! multi-fixture corpus validating the surface against
    //! representative ISF shapes (minimal header, INPUTS-bearing,
    //! PASSES-bearing) plus negative-control fixtures (pure Shadertoy,
    //! pure GLSL) lives in `tests/import_isf_reject_corpus.rs` at T2.

    use super::*;
    use std::path::PathBuf;

    fn isf_with_metadata_header() -> &'static str {
        r#"/*{
    "ISFVSN": "2",
    "DESCRIPTION": "trivial ISF fixture",
    "CREDIT": "test corpus",
    "CATEGORIES": ["TEST"]
}*/
void main() {
    gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
}
"#
    }

    fn isf_with_passes_no_isfvsn() -> &'static str {
        // ISF without explicit ISFVSN — still recognised via
        // co-occurring INPUTS + PASSES.
        r#"/*{
    "INPUTS": [{ "NAME": "speed", "TYPE": "float" }],
    "PASSES": [
        { "TARGET": "bufferA", "PERSISTENT": true },
        { }
    ]
}*/
void main() { gl_FragColor = vec4(0.0); }
"#
    }

    fn pure_glsl() -> &'static str {
        "void main() {\n    gl_FragColor = vec4(0.5, 0.5, 0.5, 1.0);\n}\n"
    }

    fn pure_shadertoy() -> &'static str {
        "void mainImage(out vec4 fragColor, in vec2 fragCoord) {\n    fragColor = vec4(0.0);\n}\n"
    }

    fn glsl_with_license_comment() -> &'static str {
        // A `/* license */` block at the top must NOT trip the ISF
        // detector — these comments are common in author shader code.
        "/* SPDX-License-Identifier: MIT\n   Copyright (c) 2026 someone */\nvoid main() { gl_FragColor = vec4(0.0); }\n"
    }

    #[test]
    fn detect_isf_returns_true_for_isf_with_metadata_header() {
        assert!(detect_isf(isf_with_metadata_header()));
    }

    #[test]
    fn detect_isf_returns_false_for_pure_glsl() {
        assert!(!detect_isf(pure_glsl()));
    }

    #[test]
    fn detect_isf_returns_false_for_shadertoy_format() {
        assert!(!detect_isf(pure_shadertoy()));
    }

    #[test]
    fn detect_isf_returns_false_for_glsl_license_header() {
        assert!(!detect_isf(glsl_with_license_comment()));
    }

    #[test]
    fn detect_isf_returns_true_for_inputs_plus_passes_without_isfvsn() {
        assert!(detect_isf(isf_with_passes_no_isfvsn()));
    }

    #[test]
    fn build_reject_diagnostic_severity_is_error() {
        let d = build_isf_reject_diagnostic(None, &[]);
        assert_eq!(d.severity, Severity::Error);
    }

    #[test]
    fn build_reject_diagnostic_uses_import_code() {
        let d = build_isf_reject_diagnostic(None, &[]);
        assert_eq!(d.code, DiagnosticCode::Import);
    }

    #[test]
    fn build_reject_diagnostic_message_mentions_isf() {
        let d = build_isf_reject_diagnostic(None, &[]);
        assert!(
            d.message.contains("ISF"),
            "reject message must reference ISF, got {}",
            d.message,
        );
        assert!(
            d.message.contains("CHR-ISF-V2"),
            "reject message must reference the V2 tracking issue, got {}",
            d.message,
        );
    }

    #[test]
    fn build_reject_diagnostic_hint_includes_actionable_escape_hatch() {
        let d = build_isf_reject_diagnostic(None, &[]);
        let hint = d
            .hint
            .expect("reject diagnostic must carry an escape-hatch hint");
        // Two-path escape hatch: Shadertoy converter OR blank scene
        // scaffold.
        assert!(
            hint.contains("Shadertoy"),
            "hint should reference the Shadertoy escape path, got {hint}",
        );
        assert!(
            hint.contains("scaffold") || hint.contains("New Scene"),
            "hint should reference the blank-scaffold escape path, got {hint}",
        );
    }

    #[test]
    fn build_reject_diagnostic_propagates_source_path() {
        let path = PathBuf::from("/tmp/scenes/example.fs");
        let d = build_isf_reject_diagnostic(Some(&path), &[]);
        assert_eq!(d.source_path.as_deref(), Some("/tmp/scenes/example.fs"));
    }

    #[test]
    fn reject_diagnostic_includes_detected_features_in_message_when_provided() {
        let d = build_isf_reject_diagnostic(None, &["ISFVSN", "INPUTS array"]);
        assert!(
            d.message.contains("ISFVSN"),
            "feature label ISFVSN must be folded into message, got {}",
            d.message,
        );
        assert!(
            d.message.contains("INPUTS array"),
            "feature label INPUTS array must be folded into message, got {}",
            d.message,
        );
    }

    #[test]
    fn try_reject_isf_input_with_isf_returns_some_diagnostic() {
        let path = PathBuf::from("/tmp/example.fs");
        let result = try_reject_isf_input(&path, isf_with_metadata_header());
        let d = result.expect("ISF input must produce a reject diagnostic");
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.code, DiagnosticCode::Import);
        assert_eq!(d.source_path.as_deref(), Some("/tmp/example.fs"));
        assert!(d.hint.is_some());
        // Auto-derived feature labels include ISFVSN for this fixture.
        assert!(d.message.contains("ISFVSN"));
    }

    #[test]
    fn try_reject_isf_input_with_non_isf_returns_none() {
        let path = PathBuf::from("/tmp/plain.glsl");
        assert!(try_reject_isf_input(&path, pure_glsl()).is_none());
        assert!(try_reject_isf_input(&path, pure_shadertoy()).is_none());
        assert!(try_reject_isf_input(&path, glsl_with_license_comment()).is_none());
    }

    #[test]
    fn try_reject_isf_input_auto_derives_inputs_passes_features() {
        let path = PathBuf::from("/tmp/multipass.fs");
        let d = try_reject_isf_input(&path, isf_with_passes_no_isfvsn())
            .expect("ISF without ISFVSN but with INPUTS+PASSES still rejects");
        assert!(
            d.message.contains("INPUTS array") && d.message.contains("PASSES array"),
            "auto-derived features should include both INPUTS and PASSES labels, got {}",
            d.message,
        );
    }
}
