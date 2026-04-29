// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-RENDER-CONFORMANCE (T2) — feedback-decay corpus (slice 4).
//!
//! Advances `docs/26-evidence-artifact-registry.md` row 28
//! (`EV-RENDER-CONFORMANCE`) past the slice-2 multipass-only corpus.
//! T2 per `docs/43-development-standards.md` §15.5: corpus
//! conformance over a fixture set with stable golden hashes plus
//! cross-backend orientation invariants.
//!
//! Slice-4 corpus (this file): two deterministic feedback-decay
//! fixtures rendered at 16×16, each asserted against an FNV-1a
//! 64-bit hash of the FINAL frame's FINAL pass readback bytes plus
//! corner-sentinel checks. The hashes are pinned per backend in an
//! `ACCEPTED: &[u64]` slice (slice-1 / slice-2 precedent); new
//! backends grow the set without replacing existing entries.
//!
//! # Why two fixtures
//!
//! Each fixture isolates one slice-4 feedback property:
//!
//! 1. `cold_start_feedback_first_frame_emitter_only` — single
//!    self-feedback pass running for ONE frame. The previous-frame
//!    texture is the cold-start black-clear (wgpu's zero-init
//!    guarantee for new textures), so the output is the emitter
//!    pattern alone with no decay history. Validates that
//!    frame-0 self-feedback reads black (the expected cold-start
//!    semantic, not garbage).
//!
//! 2. `feedback_decay_converges_after_n_frames` — single
//!    self-feedback pass running for 8 frames with the
//!    `pixel = prev * 0.85 + emitter * 0.15` decay rule. After 8
//!    frames the geometric series has substantially converged.
//!    Validates the per-frame ping-pong texture swap (frame K
//!    reads texture (K+1)%2, writes K%2) by demonstrating that
//!    successive frames CAN read the previous frame's output
//!    rather than only the cold-start black.
//!
//! # Headless adapter requirement
//!
//! Same as slice-1 / slice-2: Linux CI llvmpipe Vulkan, macOS
//! Metal, Windows Vulkan / GLES per wgpu's auto-discovery
//! (`Backends::PRIMARY`).
//!
//! Anchors:
//! - `docs/adr/ADR-011-rendering-backend.md` — wgpu + naga + GLSL→SPIR-V pinning.
//! - `docs/26-evidence-artifact-registry.md` row 28 — `EV-RENDER-CONFORMANCE`.
//! - `docs/43-development-standards.md` §15.5 — T-tier definitions.
//! - `engine/render/src/multipass.rs` — `feedback_self` flag + ping-pong wiring.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_render::{MultipassSpec, PassDef, compile_and_render_multipass};

/// FNV-1a 64-bit hash. Slice-1 / slice-2 precedent.
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

fn pixel_at_16x16(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * 16 + x) * 4;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

/// Self-feedback fragment: `out = prev * decay + emitter * (1 - decay)`.
///
/// `prev` samples THIS pass's previous-frame output (the implicitly-
/// wired feedback texture). The emitter is the slice-1 UV gradient
/// (constant per-pixel), so the steady state would be the UV gradient
/// itself; intermediate frames decay toward it.
///
/// The feedback texture binds at `1 + 2*0` / `1 + 2*0 + 1` because
/// `input_textures` is empty and `feedback_self = true` — the slice-4
/// scheduler places the self-feedback binding at the slot AFTER the
/// explicit input bindings.
const SELF_FEEDBACK_DECAY_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0) uniform Uniforms {
    float u_decay;
};
layout(set = 0, binding = 1) uniform texture2D prev_frame;
layout(set = 0, binding = 2) uniform sampler prev_sampler;
void main() {
    vec4 prev = texture(sampler2D(prev_frame, prev_sampler), uv);
    vec4 emit = vec4(uv.x, uv.y, 0.25, 1.0);
    fragColor = prev * u_decay + emit * (1.0 - u_decay);
}
"#;

/// Frame 0 cold-start: previous-frame texture is black (zero-init).
/// Output should be `0 * 0.85 + emit * 0.15` ≡ `emit * 0.15`.
#[test]
fn cold_start_feedback_first_frame_emitter_only() {
    let spec = MultipassSpec {
        passes: vec![PassDef {
            label: Some("feedback/cold-start/decay-pass".to_string()),
            fragment_glsl: SELF_FEEDBACK_DECAY_FRAGMENT.to_string(),
            uniforms: vec![chromaticity_render::UniformInput {
                name: "u_decay".to_string(),
                value: chromaticity_render::UniformValue::Scalar(0.85),
            }],
            feedback_self: true,
            ..PassDef::default()
        }],
        target_size: (16, 16),
        frame_count: 1,
    };

    let pixels = compile_and_render_multipass(&spec)
        .expect("cold-start feedback fixture must succeed on a working wgpu adapter");

    assert_eq!(pixels.len(), 16 * 16 * 4);

    let observed = fnv1a_64(&pixels);

    const ACCEPTED: &[u64] = &[
        // Apple Silicon, Metal backend, wgpu 26.0.1 + naga 26.0.0.
        // This is the cold-start frame-0 hash; previous-frame
        // texture is black-zero-init so output is `emit * 0.15`.
        0x49e9_9250_6a12_7b8d,
    ];

    if !ACCEPTED.contains(&observed) {
        panic!(
            "cold-start feedback hash 0x{observed:016x} not in accepted set {ACCEPTED:#018x?}; \
             new backends should add their measured value via a docs(repo)-scoped commit. \
             If the hash drifted on a previously-measured backend, the texture-zero-init \
             guarantee or the ping-pong slot wiring may have regressed.",
        );
    }

    // Backend-portable corner sentinels: output ≈ emit * 0.15.
    // - emit at (0,0) ≈ (0.03, 0.03, 0.25, 1.0) ⇒ scaled to
    //   (0.005, 0.005, 0.0375, 0.15) ⇒ ~(1, 1, 10, 38) in u8.
    // - emit at (15,15) ≈ (0.97, 0.97, 0.25, 1.0) ⇒ scaled to
    //   (0.146, 0.146, 0.0375, 0.15) ⇒ ~(37, 37, 10, 38) in u8.
    let top_left = pixel_at_16x16(&pixels, 0, 0);
    let bottom_right = pixel_at_16x16(&pixels, 15, 15);

    assert!(
        top_left[0] < 8,
        "cold-start (0,0) R must be near 0 (emit.x * 0.15 ~= 0.005), got {}",
        top_left[0],
    );
    assert!(
        (28..=48).contains(&bottom_right[0]),
        "cold-start (15,15) R must be near 37 (emit.x * 0.15 ~= 0.146), got {}",
        bottom_right[0],
    );
    // Blue channel: 0.25 * 0.15 = 0.0375 ⇒ ~10 in u8.
    for (x, y) in [(0_usize, 0_usize), (15, 0), (0, 15), (15, 15), (8, 8)] {
        let p = pixel_at_16x16(&pixels, x, y);
        assert!(
            p[2] <= 16,
            "cold-start B must be near 10 (0.25 * 0.15 = 0.0375) at ({x},{y}), got {}",
            p[2],
        );
        // Alpha: 1.0 * 0.15 = 0.15 ⇒ ~38 in u8.
        assert!(
            (32..=44).contains(&p[3]),
            "cold-start A must be near 38 (1.0 * 0.15) at ({x},{y}), got {}",
            p[3],
        );
    }
}

/// 8-frame feedback decay: each frame folds the emitter into the
/// previous frame's accumulator with `decay = 0.85`. The geometric
/// series toward the steady-state emit value substantially converges
/// after 8 frames; this fixture pins the convergence-state hash.
#[test]
fn feedback_decay_converges_after_n_frames() {
    let spec = MultipassSpec {
        passes: vec![PassDef {
            label: Some("feedback/decay/8-frame-converge".to_string()),
            fragment_glsl: SELF_FEEDBACK_DECAY_FRAGMENT.to_string(),
            uniforms: vec![chromaticity_render::UniformInput {
                name: "u_decay".to_string(),
                value: chromaticity_render::UniformValue::Scalar(0.85),
            }],
            feedback_self: true,
            ..PassDef::default()
        }],
        target_size: (16, 16),
        frame_count: 8,
    };

    let pixels = compile_and_render_multipass(&spec)
        .expect("8-frame feedback decay must succeed on a working wgpu adapter");

    assert_eq!(pixels.len(), 16 * 16 * 4);

    let observed = fnv1a_64(&pixels);

    const ACCEPTED: &[u64] = &[
        // Apple Silicon, Metal backend, wgpu 26.0.1 + naga 26.0.0.
        // 8-frame convergence state; each frame K writes
        // texture (K%2) and reads texture ((K+1)%2).
        0x26c3_3bf6_35d2_4495,
    ];

    if !ACCEPTED.contains(&observed) {
        panic!(
            "8-frame feedback hash 0x{observed:016x} not in accepted set {ACCEPTED:#018x?}; \
             new backends should add their measured value via a docs(repo)-scoped commit.",
        );
    }

    // Backend-portable corner sentinels: after 8 frames at decay
    // 0.85 with constant emit, the value approaches:
    //   sum_{k=0..7} (0.85)^k * 0.15 * emit
    //   = 0.15 * emit * (1 - 0.85^8) / (1 - 0.85)
    //   = emit * (1 - 0.272) ≈ emit * 0.728.
    //
    // - top-left (0,0): emit ≈ (0.03, 0.03, 0.25, 1.0) * 0.728
    //   = (0.022, 0.022, 0.182, 0.728) ⇒ ~(6, 6, 46, 186) in u8.
    // - bottom-right (15,15): emit ≈ (0.97, 0.97, 0.25, 1.0)
    //   * 0.728 = (0.706, 0.706, 0.182, 0.728)
    //   ⇒ ~(180, 180, 46, 186) in u8.
    //
    // The byte conversion adds quantization rounding; allow a
    // ± 8-ULP tolerance for backend rounding.
    let top_left = pixel_at_16x16(&pixels, 0, 0);
    let bottom_right = pixel_at_16x16(&pixels, 15, 15);
    assert!(
        top_left[0] < 16,
        "8-frame (0,0) R must be near 6 (emit.x * 0.728 ~= 0.022), got {}",
        top_left[0],
    );
    assert!(
        (160..=200).contains(&bottom_right[0]),
        "8-frame (15,15) R must be near 180 (emit.x * 0.728 ~= 0.706), got {}",
        bottom_right[0],
    );
    // Blue: 0.25 * 0.728 ≈ 0.182 ⇒ ~46 in u8.
    for (x, y) in [(0_usize, 0_usize), (15, 0), (0, 15), (15, 15), (8, 8)] {
        let p = pixel_at_16x16(&pixels, x, y);
        assert!(
            (36..=56).contains(&p[2]),
            "8-frame B must be near 46 (0.25 * 0.728) at ({x},{y}), got {}",
            p[2],
        );
        // Alpha: 1.0 * 0.728 ⇒ ~186 in u8.
        assert!(
            (176..=196).contains(&p[3]),
            "8-frame A must be near 186 (1.0 * 0.728) at ({x},{y}), got {}",
            p[3],
        );
    }
}
