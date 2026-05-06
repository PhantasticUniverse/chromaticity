// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4b Wave 4b.5 Lane B `EV-IMPORT-ISF-REJECT` corpus. Tier
//! declaration and evidence tag live on the `fixtures` inner module
//! below, matching the pattern established by
//! `engine/authoring/tests/scaffolder_corpus.rs` so
//! `scripts/evidence-tag-lint.sh`'s tier-distribution summary counts
//! this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-IMPORT-ISF-REJECT (T2) — Phase-4b Wave 4b.5 Lane B
    //! ISF input reject corpus.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises the
    //! evidence-producing behaviour (ISF detection + structured reject
    //! diagnostic + actionable escape hatch per
    //! `docs/38-scene-library-and-import-spec.md` §6) against a
    //! representative fixture corpus covering the canonical ISF
    //! shapes plus negative-control non-ISF inputs.
    //!
    //! Fixture coverage:
    //!
    //! 1. `minimal_isf_with_isfvsn_marker_rejects` — minimal ISF with
    //!    just the JSON metadata header and a trivial fragment body;
    //!    `ISFVSN` marker triggers the canonical reject path.
    //! 2. `isf_with_inputs_array_rejects_with_inputs_feature_label`
    //!    — ISF declaring `INPUTS` (sliders + colors); `INPUTS array`
    //!    appears in the auto-derived feature labels folded into the
    //!    diagnostic message.
    //! 3. `isf_with_passes_array_rejects_with_passes_feature_label`
    //!    — multi-pass ISF declaring `PASSES`; `PASSES array` appears
    //!    in the auto-derived feature labels.
    //! 4. `pure_shadertoy_input_does_not_reject` — negative control:
    //!    a pure Shadertoy `mainImage` body has no ISF metadata
    //!    header so the detector returns false and the importer
    //!    proceeds.
    //! 5. `pure_glsl_input_does_not_reject` — negative control: a
    //!    plain GLSL fragment without any header comment is not ISF.
    //!
    //! Each rejecting fixture asserts the full diagnostic shape per
    //! the row 65 contract: `Severity::Error`,
    //! `DiagnosticCode::Import`, message references ISF + the
    //! `CHR-ISF-V2` tracking issue, and the hint carries the
    //! Shadertoy-converter + blank-scaffold escape hatches.

    use chromaticity_authoring::{
        Diagnostic, DiagnosticCode, Severity, detect_isf, try_reject_isf_input,
    };
    use std::path::PathBuf;

    /// Fixture A: minimal ISF — JSON metadata header + trivial GLSL.
    const FIXTURE_MINIMAL_ISF: &str = r#"/*{
    "ISFVSN": "2",
    "DESCRIPTION": "minimal ISF corpus fixture",
    "CREDIT": "chromaticity test corpus",
    "CATEGORIES": ["TEST"]
}*/

void main() {
    gl_FragColor = vec4(0.5, 0.0, 1.0, 1.0);
}
"#;

    /// Fixture B: ISF declaring `INPUTS` (per-control sliders +
    /// colors). Canonical `INPUTS != CONTROLS` divergence per
    /// `docs/38` §6.2.
    const FIXTURE_ISF_WITH_INPUTS: &str = r#"/*{
    "ISFVSN": "2",
    "CATEGORIES": ["TEST"],
    "INPUTS": [
        {
            "NAME": "level",
            "TYPE": "float",
            "DEFAULT": 0.5,
            "MIN": 0.0,
            "MAX": 1.0
        },
        {
            "NAME": "tint",
            "TYPE": "color",
            "DEFAULT": [1.0, 0.5, 0.2, 1.0]
        },
        {
            "NAME": "mode",
            "TYPE": "long",
            "VALUES": [0, 1, 2],
            "LABELS": ["A", "B", "C"],
            "DEFAULT": 0
        }
    ]
}*/

void main() {
    gl_FragColor = vec4(level * tint.rgb, 1.0);
}
"#;

    /// Fixture C: multi-pass ISF declaring `PASSES`. ISF's persistent
    /// buffer model (per `docs/38` §6.2) is the second concrete
    /// divergence justifying V1 reject.
    const FIXTURE_ISF_WITH_PASSES: &str = r#"/*{
    "ISFVSN": "2",
    "DESCRIPTION": "multi-pass ISF fixture",
    "INPUTS": [
        { "NAME": "decay", "TYPE": "float", "DEFAULT": 0.95 }
    ],
    "PASSES": [
        {
            "TARGET": "feedbackBuffer",
            "PERSISTENT": true,
            "WIDTH": "$WIDTH",
            "HEIGHT": "$HEIGHT"
        },
        { }
    ]
}*/

void main() {
    gl_FragColor = vec4(0.0);
}
"#;

    /// Negative control: pure Shadertoy paste (no JSON header).
    const FIXTURE_SHADERTOY: &str = r#"void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    fragColor = vec4(uv, 0.5 + 0.5 * sin(iTime), 1.0);
}
"#;

    /// Negative control: plain GLSL fragment, no header comment.
    const FIXTURE_PLAIN_GLSL: &str = r#"void main() {
    gl_FragColor = vec4(0.5, 0.5, 0.5, 1.0);
}
"#;

    fn assert_canonical_reject_shape(d: &Diagnostic, expected_path: &str) {
        assert_eq!(d.severity, Severity::Error, "severity must be Error");
        assert_eq!(d.code, DiagnosticCode::Import, "code must be Import");
        assert!(
            d.message.contains("ISF"),
            "message must reference ISF, got {}",
            d.message,
        );
        assert!(
            d.message.contains("CHR-ISF-V2"),
            "message must reference CHR-ISF-V2 tracking issue, got {}",
            d.message,
        );
        assert_eq!(d.source_path.as_deref(), Some(expected_path));
        let hint = d
            .hint
            .as_deref()
            .expect("reject must carry escape-hatch hint");
        assert!(
            hint.contains("Shadertoy"),
            "hint must reference Shadertoy escape path, got {hint}",
        );
        assert!(
            hint.contains("scaffold") || hint.contains("New Scene"),
            "hint must reference blank-scaffold escape path, got {hint}",
        );
    }

    #[test]
    fn minimal_isf_with_isfvsn_marker_rejects() {
        assert!(detect_isf(FIXTURE_MINIMAL_ISF));

        let path = PathBuf::from("/tmp/corpus/minimal.fs");
        let d = try_reject_isf_input(&path, FIXTURE_MINIMAL_ISF)
            .expect("minimal ISF must produce reject diagnostic");
        assert_canonical_reject_shape(&d, "/tmp/corpus/minimal.fs");
        // ISFVSN feature label folded into the message.
        assert!(d.message.contains("ISFVSN"));
    }

    #[test]
    fn isf_with_inputs_array_rejects_with_inputs_feature_label() {
        assert!(detect_isf(FIXTURE_ISF_WITH_INPUTS));

        let path = PathBuf::from("/tmp/corpus/inputs.fs");
        let d = try_reject_isf_input(&path, FIXTURE_ISF_WITH_INPUTS)
            .expect("ISF with INPUTS must produce reject diagnostic");
        assert_canonical_reject_shape(&d, "/tmp/corpus/inputs.fs");
        assert!(
            d.message.contains("INPUTS array"),
            "INPUTS array feature label must appear, got {}",
            d.message,
        );
    }

    #[test]
    fn isf_with_passes_array_rejects_with_passes_feature_label() {
        assert!(detect_isf(FIXTURE_ISF_WITH_PASSES));

        let path = PathBuf::from("/tmp/corpus/passes.fs");
        let d = try_reject_isf_input(&path, FIXTURE_ISF_WITH_PASSES)
            .expect("ISF with PASSES must produce reject diagnostic");
        assert_canonical_reject_shape(&d, "/tmp/corpus/passes.fs");
        assert!(
            d.message.contains("PASSES array"),
            "PASSES array feature label must appear, got {}",
            d.message,
        );
        assert!(
            d.message.contains("INPUTS array"),
            "co-occurring INPUTS array feature label must also appear, got {}",
            d.message,
        );
    }

    #[test]
    fn pure_shadertoy_input_does_not_reject() {
        assert!(!detect_isf(FIXTURE_SHADERTOY));

        let path = PathBuf::from("/tmp/corpus/shadertoy.glsl");
        assert!(
            try_reject_isf_input(&path, FIXTURE_SHADERTOY).is_none(),
            "pure Shadertoy paste must not be classified as ISF",
        );
    }

    #[test]
    fn pure_glsl_input_does_not_reject() {
        assert!(!detect_isf(FIXTURE_PLAIN_GLSL));

        let path = PathBuf::from("/tmp/corpus/plain.glsl");
        assert!(
            try_reject_isf_input(&path, FIXTURE_PLAIN_GLSL).is_none(),
            "plain GLSL must not be classified as ISF",
        );
    }
}
