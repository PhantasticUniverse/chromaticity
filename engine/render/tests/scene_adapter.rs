// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-RENDER-CONFORMANCE (T2) — scene-adapter corpus (slice 4).
//!
//! Advances `docs/26-evidence-artifact-registry.md` row 28 toward
//! the slice-4 deliverable: the
//! [`chromaticity_scene::Scene`] → [`chromaticity_render::MultipassSpec`]
//! adapter. T2 per `docs/43-development-standards.md` §15.5: corpus
//! conformance over a fixture set with stable golden hashes.
//!
//! Slice-4 corpus (this file): one round-trip fixture that
//! constructs a synthetic `Scene` carrying the `FLOAT` /
//! `FEEDBACK` per-pass extension keys, runs it through
//! [`chromaticity_render::MultipassSpec::from_scene`], renders the
//! adapted spec via
//! [`chromaticity_render::compile_and_render_multipass`], and
//! asserts the FNV-1a 64-bit byte-hash matches the equivalent
//! hand-built `MultipassSpec` (consistency invariant). The two
//! paths must produce bit-identical output for the same shader +
//! same target_size + same frame_count + same `target_format` +
//! same `feedback_self`; any drift surfaces here.
//!
//! Gated behind the `scene-adapter` cargo feature (enabled by
//! `--all-features` in CI's render-crate test invocation).
//!
//! # Headless adapter requirement
//!
//! Same as slice-1 / slice-2 / slice-4 baseline: Linux CI llvmpipe
//! Vulkan, macOS Metal, Windows Vulkan / GLES per wgpu's
//! auto-discovery (`Backends::PRIMARY`).
//!
//! Anchors:
//! - `engine/render/src/scene_adapter.rs` — adapter implementation.
//! - `engine/scene/src/schema.rs` — source `Scene` / `PassSpec`
//!   / `UniformBinding` types.
//! - `docs/02-synesthesia-contracts.md` §2 Multipass — per-pass
//!   `WIDTH` / `HEIGHT` / `FLOAT` extension keys.
//! - `docs/26-evidence-artifact-registry.md` row 28 —
//!   `EV-RENDER-CONFORMANCE`.

#![cfg(feature = "scene-adapter")]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_render::{
    MultipassSpec, PassDef, SceneAdapterOptions, TargetFormat, compile_and_render_multipass,
};
use chromaticity_scene::schema::{NodeRef, PassSpec, Scene};

const UV_GRADIENT_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() {
    fragColor = vec4(uv.x, uv.y, 0.25, 1.0);
}
"#;

fn fnv1a_64(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Round-trip a synthetic single-pass scene through the adapter
/// and confirm its byte-hash matches an equivalent hand-built
/// `MultipassSpec`. Demonstrates the adapter preserves rendering
/// semantics for the trivial case (no extensions, default
/// target_format, default feedback_self).
#[test]
fn scene_adapter_single_pass_round_trip_consistency() {
    // Build the hand-crafted reference spec.
    let reference = MultipassSpec {
        passes: vec![PassDef {
            label: Some("scene-adapter/pass-0/node:uv-gradient-target".to_string()),
            fragment_glsl: UV_GRADIENT_FRAGMENT.to_string(),
            ..PassDef::default()
        }],
        target_size: (16, 16),
        frame_count: 1,
    };

    // Build the equivalent scene.
    let mut scene = Scene::default();
    scene.passes.push(PassSpec {
        node: NodeRef("uv-gradient-target".to_string()),
        uniform_bindings: Vec::new(),
        extensions: serde_json::Map::new(),
    });
    let adapted = MultipassSpec::from_scene(&scene, vec![Some(UV_GRADIENT_FRAGMENT.to_string())])
        .expect("adapter must accept the synthetic single-pass scene");

    // The adapter's label format includes the node ID; assert
    // the structural contract before rendering.
    assert_eq!(adapted.passes.len(), 1);
    assert_eq!(adapted.target_size, reference.target_size);
    assert_eq!(adapted.frame_count, reference.frame_count);
    assert_eq!(adapted.passes[0].target_format, TargetFormat::Rgba8Unorm);
    assert!(!adapted.passes[0].feedback_self);

    // Render both, assert bit-identical readback bytes.
    let reference_pixels = compile_and_render_multipass(&reference)
        .expect("reference render must succeed on a working wgpu adapter");
    let adapted_pixels = compile_and_render_multipass(&adapted)
        .expect("adapted render must succeed on a working wgpu adapter");

    assert_eq!(
        reference_pixels.len(),
        adapted_pixels.len(),
        "reference and adapted readbacks must be the same length",
    );
    assert_eq!(
        fnv1a_64(&reference_pixels),
        fnv1a_64(&adapted_pixels),
        "round-trip via scene adapter must be byte-identical to a hand-built spec \
         (rendering semantics must be preserved through the adapter)",
    );
}

/// Verify the adapter routes `FLOAT` and `FEEDBACK` extensions
/// into the corresponding [`PassDef`] fields, and that a 4-frame
/// feedback render via the adapter produces the same byte stream
/// as an equivalent hand-built spec.
#[test]
fn scene_adapter_feedback_and_float_extensions_thread_through() {
    // Self-feedback fragment with a uniform-less constant decay
    // (avoiding the slice-4 audio-uniform integration boundary).
    const SELF_FEEDBACK_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 1) uniform texture2D prev_frame;
layout(set = 0, binding = 2) uniform sampler prev_sampler;
void main() {
    vec4 prev = texture(sampler2D(prev_frame, prev_sampler), uv);
    vec4 emit = vec4(uv.x, uv.y, 0.25, 1.0);
    fragColor = prev * 0.7 + emit * 0.3;
}
"#;

    // Build the hand-crafted reference spec.
    let reference = MultipassSpec {
        passes: vec![PassDef {
            label: Some("scene-adapter/pass-0/node:feedback-test".to_string()),
            fragment_glsl: SELF_FEEDBACK_FRAGMENT.to_string(),
            feedback_self: true,
            target_format: TargetFormat::Rgba8Unorm,
            ..PassDef::default()
        }],
        target_size: (16, 16),
        frame_count: 4,
    };

    // Build the equivalent scene with extensions.
    let mut scene = Scene::default();
    let mut extensions = serde_json::Map::new();
    extensions.insert("FEEDBACK".to_string(), serde_json::Value::Bool(true));
    // FLOAT explicitly false to exercise the adapter's bool
    // mapping path (false ⇒ Rgba8Unorm, the same as absent).
    extensions.insert("FLOAT".to_string(), serde_json::Value::Bool(false));
    scene.passes.push(PassSpec {
        node: NodeRef("feedback-test".to_string()),
        uniform_bindings: Vec::new(),
        extensions,
    });
    let options = SceneAdapterOptions {
        fragment_sources: vec![Some(SELF_FEEDBACK_FRAGMENT.to_string())],
        default_target_size: (16, 16),
        frame_count: 4,
        strict_size: false,
    };
    let adapted = MultipassSpec::from_scene_with_options(&scene, &options)
        .expect("adapter must accept the synthetic feedback scene");

    assert_eq!(adapted.passes.len(), 1);
    assert!(adapted.passes[0].feedback_self);
    assert_eq!(adapted.passes[0].target_format, TargetFormat::Rgba8Unorm);
    assert_eq!(adapted.frame_count, 4);

    let reference_pixels =
        compile_and_render_multipass(&reference).expect("reference feedback render must succeed");
    let adapted_pixels =
        compile_and_render_multipass(&adapted).expect("adapted feedback render must succeed");

    assert_eq!(
        fnv1a_64(&reference_pixels),
        fnv1a_64(&adapted_pixels),
        "scene-adapter feedback / float-extension round-trip must be byte-identical \
         to a hand-built spec",
    );
}
