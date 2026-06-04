// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! In-engine test-pattern [`MultipassSpec`] for the ADR-011 Phase-2
//! Slice R-1a output window.
//!
//! Slice R-1a proves the wgpu ↔ Tauri surface pipeline IN ISOLATION:
//! no scene loading, no scene adapter, no scheduler wiring, no live
//! audio (those are R-1b / R-1c). The pattern is a single-pass GLSL
//! fragment producing a time-animated gradient driven by a `u_time`
//! scalar uniform — enough to make motion visible on screen and to
//! exercise the full per-frame
//! [`chromaticity_render::MultipassRenderer::render_frame_live`] →
//! blit → present loop.
//!
//! The fragment mirrors the binding shape of the engine's slice-2
//! `single_pass_uniform_binding` conformance fixture (one
//! `uniform float` at `set = 0, binding = 0`), so the spec compiles
//! through the same naga GLSL frontend the engine's render corpus
//! already pins as known-good.

use chromaticity_render::{MultipassSpec, PassDef, UniformInput, UniformValue};

/// Stable scene id folded into the
/// [`chromaticity_render::live_feedback_spec_key`] hash for the test
/// pattern. R-1a's pattern has no feedback pass, so the key only
/// drives the renderer's reallocation bookkeeping; it stays constant
/// across the run so the renderer never needlessly reallocates.
pub const PATTERN_SCENE_ID: &str = "chroma-r1a-test-pattern";

/// Single-pass animated-gradient fragment.
///
/// `u_time` advances each frame (seconds since the render loop
/// started). The output is a UV gradient with a time-driven hue
/// sweep in the blue channel plus a slow diagonal sine band, so both
/// position (the gradient) and time (the sweep / band motion) are
/// visible — a clear "this is live, not a frozen frame" signal for the
/// runtime-verified slice. The binding layout matches the engine's
/// `single_pass_uniform_binding` corpus fixture: one scalar uniform in
/// a `Uniforms` block at `set = 0, binding = 0`.
const ANIMATED_GRADIENT_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0) uniform Uniforms {
    float u_time;
};
void main() {
    float sweep = 0.5 + 0.5 * sin(u_time);
    float band = 0.5 + 0.5 * sin((uv.x + uv.y) * 6.2831853 + u_time * 2.0);
    fragColor = vec4(uv.x, uv.y, sweep * band, 1.0);
}
"#;

/// Build the R-1a single-pass test-pattern spec for `(width, height)`
/// at the supplied animation time (seconds).
///
/// `frame_count` is `1`: the live render loop drives one frame per
/// `render_frame_live` call and advances `u_time` itself, rather than
/// letting the engine's internal multi-frame loop run (which is the
/// offscreen-feedback driver, not the on-screen present driver).
#[must_use]
pub fn test_pattern_spec(width: u32, height: u32, time_secs: f32) -> MultipassSpec {
    MultipassSpec {
        passes: vec![PassDef {
            label: Some("r1a/pass-0/animated-gradient".to_string()),
            fragment_glsl: ANIMATED_GRADIENT_FRAGMENT.to_string(),
            uniforms: vec![UniformInput {
                name: "u_time".to_string(),
                value: UniformValue::Scalar(time_secs),
            }],
            ..PassDef::default()
        }],
        target_size: (width.max(1), height.max(1)),
        frame_count: 1,
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Slice R-1a spec-builder pins. The render itself requires a GPU
    //! (orchestrator-verified by running the app); the spec SHAPE is
    //! testable headlessly and pinned here.

    use super::{PATTERN_SCENE_ID, test_pattern_spec};
    use chromaticity_render::{UniformValue, live_feedback_spec_key};

    #[test]
    fn spec_is_single_pass_single_frame_with_time_uniform() {
        let spec = test_pattern_spec(1280, 720, 0.0);
        assert_eq!(spec.passes.len(), 1, "R-1a pattern is single-pass");
        assert_eq!(
            spec.frame_count, 1,
            "the live loop drives frames, not frame_count"
        );
        assert_eq!(spec.target_size, (1280, 720));
        let pass = &spec.passes[0];
        assert!(!pass.feedback_self, "R-1a pattern has no feedback");
        assert!(
            pass.input_textures.is_empty(),
            "R-1a pattern samples no prior pass"
        );
        assert_eq!(pass.uniforms.len(), 1, "one u_time scalar uniform");
        assert_eq!(pass.uniforms[0].name, "u_time");
    }

    #[test]
    fn time_uniform_tracks_the_supplied_seconds() {
        let spec = test_pattern_spec(64, 64, 3.5);
        match spec.passes[0].uniforms[0].value {
            UniformValue::Scalar(t) => assert!((t - 3.5).abs() < f32::EPSILON),
            ref other => panic!("expected a scalar u_time uniform, got {other:?}"),
        }
    }

    #[test]
    fn zero_dimensions_are_clamped_to_one() {
        // A degenerate (0, 0) window size must not produce a zero
        // target — the engine rejects zero target sizes with
        // TargetSizeInvalid. Clamp to 1 so the very first frame at a
        // not-yet-sized window still renders.
        let spec = test_pattern_spec(0, 0, 0.0);
        assert_eq!(spec.target_size, (1, 1));
    }

    #[test]
    fn spec_key_is_stable_for_the_pattern_scene_id() {
        let a = live_feedback_spec_key(PATTERN_SCENE_ID, (1280, 720));
        let b = live_feedback_spec_key(PATTERN_SCENE_ID, (1280, 720));
        assert_eq!(
            a, b,
            "the pattern key must be stable across frames at a fixed size"
        );
    }
}
