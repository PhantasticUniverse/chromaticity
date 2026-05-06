// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4b Wave 4b.3 Lane B `EV-AUTHORING-DIAGNOSTICS` PR-2 multi-class
//! fixture corpus. Tier declaration and evidence tag live on the
//! `fixtures` inner module below, matching the pattern established by
//! `engine/authoring/tests/scaffolder_corpus.rs` so
//! `scripts/evidence-tag-lint.sh`'s tier-distribution summary counts
//! this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-AUTHORING-DIAGNOSTICS (T2) — Phase-4b Wave 4b.3
    //! Lane B PR 2 of 2 multi-class fixture corpus closing out
    //! `docs/26-evidence-artifact-registry.md` row 62.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises the
    //! evidence-producing diagnostic-renderer surface (PR 1 / merge
    //! 1e8b6f1 — `engine/authoring/src/diagnostics.rs`) against a
    //! fixture corpus where each fixture (a) constructs a real
    //! engine-side error of the relevant class — `RenderError`
    //! variants synthesised from real `naga::front::glsl::Frontend` /
    //! `naga::valid::Validator` failures, `ScriptError` variants
    //! produced by the real `chromaticity_script::ScriptRuntime`,
    //! `serde_json::Error` variants raised by `serde_json::from_str`
    //! against representative malformed `scene.json` shapes,
    //! coercion-class `Warning` / `ValidationError` variants emitted
    //! by the real `chromaticity_scene::lint::lint_scene` and
    //! `chromaticity_scene::validator::validate` passes — (b) routes
    //! it through the corresponding per-source mapper or
    //! `IntoDiagnostic` trait impl, and (c) asserts the resulting
    //! `Diagnostic` carries the expected `severity`, `code`,
    //! `message` substring, `span` shape, and (where applicable)
    //! `hint`.
    //!
    //! Fixture roster (12 sub-fixtures across the four classes per
    //! the PR-2 brief):
    //!
    //! 1. Compile class — three sub-fixtures, each driving real
    //!    `naga` against ill-formed GLSL and lowering to
    //!    `RenderError::FragmentParse` / `FragmentValidate`:
    //!    - `compile_missing_semicolon` — parser-stage failure.
    //!    - `compile_undefined_identifier` — parser-stage failure on
    //!      an undeclared symbol reference.
    //!    - `compile_type_mismatch_int_to_vec4` — assigning a scalar
    //!      to a `vec4` declaration; surfaces at parse OR validate
    //!      depending on naga's stage decomposition. Both stages map
    //!      to `DiagnosticCode::Compile` so the assertion only
    //!      requires "compile-class".
    //! 2. Script class — three sub-fixtures, each driving the real
    //!    `ScriptRuntime` and lowering the resulting `ScriptError`:
    //!    - `script_unbalanced_brace_compile_error` —
    //!      `ScriptError::Compile`.
    //!    - `script_uncaught_throw_runtime_error` —
    //!      `ScriptError::Runtime`.
    //!    - `script_undefined_property_access_runtime_error` —
    //!      `TypeError` from accessing a property on `undefined`,
    //!      surfaces as `ScriptError::Runtime`.
    //! 3. JSON class — three sub-fixtures, each driving real
    //!    `serde_json::from_str` against malformed
    //!    `chromaticity_scene::schema::Scene` JSON:
    //!    - `json_trailing_comma_in_array` — serde rejects a
    //!      trailing comma.
    //!    - `json_unexpected_token_in_passes_array` — non-string /
    //!      non-object scalar where the schema expects a
    //!      structured shape.
    //!    - `json_truncated_input_unexpected_eof` — early EOF
    //!      mid-value.
    //! 4. Coercion class — three sub-fixtures driving the real
    //!    scene lint + validator passes:
    //!    - `coercion_legacy_key_rewrite_emits_coerce_severity` —
    //!      `lint_scene` emits `Warning::LegacyKey` on the
    //!      documented `IMAGE` → `IMAGE_PATH` map.
    //!    - `coercion_unknown_control_type_emits_warning_severity`
    //!      — `lint_scene` emits `Warning::UnknownControlType` on a
    //!      control whose `TYPE` is not in the canonical taxonomy.
    //!    - `coercion_validation_strict_missing_type_emits_error_severity`
    //!      — `validator::validate` under `ValidationMode::Strict`
    //!      rejects a control with no `TYPE` and surfaces
    //!      `ValidationError::MissingControlType`.
    //!
    //! Span shape per fixture: `Compile`, `Script`, and the lint /
    //! validation `Coercion` fixtures all assert `span == None`
    //! (engine error sources for these classes do not carry
    //! structured `(line, column)` today; see PR-1's module-level
    //! "Span surface" prose). The `Json` fixtures all assert
    //! `span == Some(Span { ... })` with `start_line >= 1` and
    //! `start_col >= 1`, matching the 1-based serde_json shape PR-1
    //! threads through.
    //!
    //! `Diagnostic` round-trip and `IntoDiagnostic` trait dispatch
    //! invariants are pinned at T1 in
    //! `engine/authoring/src/diagnostics.rs::tests`; this corpus
    //! does not duplicate them.

    use chromaticity_authoring::diagnostics::{
        Diagnostic, DiagnosticCode, IntoDiagnostic, Severity, diagnostic_from_compile_error,
        diagnostic_from_json_error, diagnostic_from_lint_warning, diagnostic_from_script_error,
        diagnostic_from_validation_error,
    };
    use chromaticity_render::RenderError;
    use chromaticity_scene::lint::{Warning, lint_scene};
    use chromaticity_scene::schema::{ControlDecl, Scene};
    use chromaticity_scene::validator::{ValidationError, ValidationMode, validate};
    use chromaticity_script::{ScriptError, ScriptRuntime};

    // -------------------------------------------------------------------
    // Helpers — real engine error producers.
    // -------------------------------------------------------------------

    /// Drive `naga::front::glsl::Frontend::parse` followed by
    /// `naga::valid::Validator::validate`, lowering the first failure
    /// into the `RenderError` variant `engine/render::single_pass`
    /// would surface for the same input. Mirrors the `compile_glsl_module`
    /// seam in `engine/render/src/single_pass.rs` (which is `pub(crate)`
    /// — the slice-boundary signal per CLAUDE.md, so the corpus
    /// reproduces the parse + validate seam directly against `naga`,
    /// matching the pattern already used by `scaffolder_corpus.rs`).
    fn compile_glsl_for_diagnostics(source: &str) -> Result<(), RenderError> {
        let mut frontend = naga::front::glsl::Frontend::default();
        let module = frontend
            .parse(
                &naga::front::glsl::Options {
                    stage: naga::ShaderStage::Fragment,
                    defines: Default::default(),
                },
                source,
            )
            .map_err(|e| RenderError::FragmentParse(format!("{e:?}")))?;

        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        );
        validator
            .validate(&module)
            .map_err(|e| RenderError::FragmentValidate(format!("{e:?}")))?;
        Ok(())
    }

    /// Assert that a freshly produced compile-class `Diagnostic` carries
    /// the canonical shape: `Compile` code at `Error` severity with
    /// the engine source path tag and no span (the GLSL line numbers
    /// are embedded in the formatted naga diagnostic; PR 1 deliberately
    /// does not parse them out — see PR-1's module-level "Span surface"
    /// prose).
    fn assert_compile_class_diagnostic_shape(diag: &Diagnostic) {
        assert_eq!(diag.code, DiagnosticCode::Compile);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.source_path.as_deref(), Some("main.glsl"));
        assert!(diag.span.is_none());
        assert!(!diag.message.is_empty());
        assert!(diag.hint.is_none());
    }

    fn assert_script_class_diagnostic_shape(diag: &Diagnostic) {
        assert_eq!(diag.code, DiagnosticCode::Script);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.source_path.as_deref(), Some("script.js"));
        assert!(diag.span.is_none());
        assert!(!diag.message.is_empty());
    }

    fn assert_json_class_diagnostic_shape(diag: &Diagnostic) {
        assert_eq!(diag.code, DiagnosticCode::Json);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.source_path.as_deref(), Some("scene.json"));
        let span = diag
            .span
            .expect("JSON-class diagnostics thread a serde_json span through verbatim");
        assert!(
            span.start_line >= 1,
            "serde_json position is 1-based; got line {}",
            span.start_line,
        );
        assert!(
            span.start_col >= 1,
            "serde_json position is 1-based; got column {}",
            span.start_col,
        );
        assert_eq!(span.start_line, span.end_line);
        assert_eq!(span.start_col, span.end_col);
    }

    // -------------------------------------------------------------------
    // 1. Compile class — three sub-fixtures.
    // -------------------------------------------------------------------

    #[test]
    fn compile_missing_semicolon() {
        let source = r#"
            #version 450
            layout(location = 0) out vec4 frag_color;
            void main() {
                frag_color = vec4(1.0, 0.0, 0.0, 1.0)
            }
        "#;
        let err = compile_glsl_for_diagnostics(source)
            .expect_err("missing semicolon must surface as a compile-class error");
        let diag = diagnostic_from_compile_error(&err);
        assert_compile_class_diagnostic_shape(&diag);

        // Trait-dispatch parity: `&RenderError::into_diagnostic()` matches
        // the free-function output. Pinned per-fixture because the corpus
        // is the only T2 surface that exercises trait dispatch under real
        // engine inputs.
        let via_trait = (&err).into_diagnostic();
        assert_eq!(diag, via_trait);
    }

    #[test]
    fn compile_undefined_identifier() {
        let source = r#"
            #version 450
            layout(location = 0) out vec4 frag_color;
            void main() {
                frag_color = undeclared_symbol;
            }
        "#;
        let err = compile_glsl_for_diagnostics(source)
            .expect_err("undeclared symbol must surface as a compile-class error");
        let diag = diagnostic_from_compile_error(&err);
        assert_compile_class_diagnostic_shape(&diag);
    }

    #[test]
    fn compile_type_mismatch_int_to_vec4() {
        // Assigning a bare scalar literal to a `vec4` declaration.
        // naga's GLSL frontend rejects this either at parse or validate
        // depending on stage decomposition; both lower to a compile-class
        // diagnostic per `engine/authoring/src/diagnostics.rs::diagnostic_from_compile_error`.
        let source = r#"
            #version 450
            layout(location = 0) out vec4 frag_color;
            void main() {
                vec4 x = 1;
                frag_color = x;
            }
        "#;
        let err = compile_glsl_for_diagnostics(source)
            .expect_err("scalar-to-vec4 assignment must surface as a compile-class error");
        // Either FragmentParse or FragmentValidate is acceptable here —
        // both map to DiagnosticCode::Compile / Severity::Error.
        assert!(
            matches!(
                err,
                RenderError::FragmentParse(_) | RenderError::FragmentValidate(_)
            ),
            "type mismatch must surface as a parse or validate failure, got {err:?}",
        );
        let diag = diagnostic_from_compile_error(&err);
        assert_compile_class_diagnostic_shape(&diag);
    }

    // -------------------------------------------------------------------
    // 2. Script class — three sub-fixtures.
    // -------------------------------------------------------------------

    #[test]
    fn script_unbalanced_brace_compile_error() {
        let rt = ScriptRuntime::new().expect("script runtime constructs");
        let err = rt
            .execute("function broken( { return 1;")
            .expect_err("unbalanced brace must reject at compile");
        assert!(
            matches!(err, ScriptError::Compile(_)),
            "unbalanced brace must surface as Compile, got {err:?}",
        );
        let diag = diagnostic_from_script_error(&err);
        assert_script_class_diagnostic_shape(&diag);
        assert!(
            diag.message.contains("compile failed"),
            "message must carry the upstream compile-failure prefix, got {:?}",
            diag.message,
        );

        let via_trait = (&err).into_diagnostic();
        assert_eq!(diag, via_trait);
    }

    #[test]
    fn script_uncaught_throw_runtime_error() {
        let rt = ScriptRuntime::new().expect("script runtime constructs");
        let err = rt
            .execute("throw new Error('explicit throw');")
            .expect_err("uncaught throw must reject at runtime");
        assert!(
            matches!(err, ScriptError::Runtime(_)),
            "uncaught throw must surface as Runtime, got {err:?}",
        );
        let diag = diagnostic_from_script_error(&err);
        assert_script_class_diagnostic_shape(&diag);
        assert!(
            diag.message.contains("uncaught"),
            "message must carry the upstream uncaught-value prefix, got {:?}",
            diag.message,
        );
    }

    #[test]
    fn script_undefined_property_access_runtime_error() {
        let rt = ScriptRuntime::new().expect("script runtime constructs");
        // `null.x` is a `TypeError` at runtime in QuickJS — it parses,
        // executes, then throws. Confirms the script class catches
        // runtime-stage failures distinct from the compile-stage
        // failure exercised above.
        let err = rt
            .execute("var x = null; x.foo;")
            .expect_err("property access on null must reject at runtime");
        assert!(
            matches!(err, ScriptError::Runtime(_)),
            "null property access must surface as Runtime, got {err:?}",
        );
        let diag = diagnostic_from_script_error(&err);
        assert_script_class_diagnostic_shape(&diag);
    }

    // -------------------------------------------------------------------
    // 3. JSON class — three sub-fixtures.
    // -------------------------------------------------------------------

    #[test]
    fn json_trailing_comma_in_array() {
        // `serde_json` rejects trailing commas by default. The malformed
        // input is shaped like a `scene.json` to keep the fixture
        // representative even though only the trailing comma triggers
        // the failure.
        let raw = r#"{
            "PASSES": [
                "node_a",
                "node_b",
            ]
        }"#;
        let err =
            serde_json::from_str::<serde_json::Value>(raw).expect_err("trailing comma must reject");
        let diag = diagnostic_from_json_error(&err);
        assert_json_class_diagnostic_shape(&diag);
    }

    #[test]
    fn json_unexpected_token_in_passes_array() {
        // A bare bareword inside the `PASSES` array — not a quoted
        // string, not a JSON object — surfaces as a `serde_json`
        // syntax error at the position of the offending token.
        let raw = r#"{
            "PASSES": [
                bareword_not_quoted
            ]
        }"#;
        let err = serde_json::from_str::<serde_json::Value>(raw).expect_err("bareword must reject");
        let diag = diagnostic_from_json_error(&err);
        assert_json_class_diagnostic_shape(&diag);
    }

    #[test]
    fn json_truncated_input_unexpected_eof() {
        // Missing closing brace + missing closing bracket. `serde_json`
        // surfaces an EOF-class error with a 1-based position pointing
        // at the truncation site.
        let raw = r#"{ "PASSES": [ "node_a""#;
        let err = serde_json::from_str::<serde_json::Value>(raw)
            .expect_err("truncated input must reject");
        let diag = diagnostic_from_json_error(&err);
        assert_json_class_diagnostic_shape(&diag);

        let via_trait = (&err).into_diagnostic();
        assert_eq!(diag, via_trait);
    }

    // -------------------------------------------------------------------
    // 4. Coercion class — three sub-fixtures.
    // -------------------------------------------------------------------

    #[test]
    fn coercion_legacy_key_rewrite_emits_coerce_severity() {
        // The documented `IMAGE` → `IMAGE_PATH` map per
        // `engine/scene/src/lint.rs::LEGACY_ROOT_KEY_MAP` fires a
        // `Warning::LegacyKey` on every top-level `IMAGE` key.
        let raw = serde_json::json!({
            "IMAGE": "media/foo.png",
            "PASSES": [],
        });
        let warnings = lint_scene(&raw);
        let legacy = warnings
            .iter()
            .find(|w| matches!(w, Warning::LegacyKey { .. }))
            .expect("lint pass must emit LegacyKey for the documented IMAGE → IMAGE_PATH map");

        let diag = diagnostic_from_lint_warning(legacy);
        assert_eq!(diag.code, DiagnosticCode::Coercion);
        assert_eq!(diag.severity, Severity::Coerce);
        assert!(diag.message.contains("IMAGE"));
        assert!(diag.message.contains("IMAGE_PATH"));
        assert!(diag.span.is_none());
        let hint = diag.hint.as_deref().expect("legacy-key carries a fix hint");
        assert!(hint.contains("IMAGE_PATH"));
    }

    #[test]
    fn coercion_unknown_control_type_emits_warning_severity() {
        // A control whose `TYPE` is outside the canonical taxonomy
        // (per `engine/scene/src/lint.rs::KNOWN_CONTROL_KINDS`) emits
        // `Warning::UnknownControlType` at `Severity::Warning` with no
        // hint (no proposed fix is available — the spelling was
        // rejected, not rewritten).
        let raw = serde_json::json!({
            "PASSES": [],
            "CONTROLS": [
                {
                    "NAME": "mystery_input",
                    "TYPE": "warp_drive",
                },
            ],
        });
        let warnings = lint_scene(&raw);
        let unknown = warnings
            .iter()
            .find(|w| matches!(w, Warning::UnknownControlType { .. }))
            .expect("lint pass must emit UnknownControlType for the unrecognized TYPE");

        let diag = diagnostic_from_lint_warning(unknown);
        assert_eq!(diag.code, DiagnosticCode::Coercion);
        assert_eq!(diag.severity, Severity::Warning);
        assert!(diag.message.contains("mystery_input"));
        assert!(diag.span.is_none());
        assert!(diag.hint.is_none());
    }

    #[test]
    fn coercion_validation_strict_missing_type_emits_error_severity() {
        // A control with no `TYPE` under `ValidationMode::Strict` per
        // `engine/scene/src/validator.rs` rejects with
        // `ValidationError::MissingControlType`. Construct the
        // `Scene` programmatically via the public schema surface
        // (mirroring the in-crate validator T1 helper
        // `scene_with_missing_type_control`) — round-tripping a raw
        // JSON shape would also exercise the slice-3a deserializer's
        // CP-01 fallback rather than isolating the strict-mode
        // rejection. The diagnostic surface lowers this to
        // `Coercion` / `Error` (a strict-mode hard reject distinct
        // from the `Coerce` severity used for compat-mode silent
        // rewrites).
        let mut scene = Scene::default();
        scene.controls.push(ControlDecl {
            name: "mystery_input".to_string(),
            kind: None,
            ..ControlDecl::default()
        });
        let err = validate(&scene, ValidationMode::Strict)
            .expect_err("strict-mode validation must reject a control with no TYPE");
        assert!(
            matches!(err, ValidationError::MissingControlType(_)),
            "strict-mode missing TYPE must surface as MissingControlType, got {err:?}",
        );
        let diag = diagnostic_from_validation_error(&err);
        assert_eq!(diag.code, DiagnosticCode::Coercion);
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("mystery_input"));
        assert!(diag.span.is_none());

        let via_trait = (&err).into_diagnostic();
        assert_eq!(diag, via_trait);
    }
}
