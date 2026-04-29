// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-RENDER-CONFORMANCE (T2) — multipass scheduler deterministic-cascade corpus.
//!
//! Advances `docs/26-evidence-artifact-registry.md` row 28
//! (`EV-RENDER-CONFORMANCE`) past the slice-1 single-pass-only corpus.
//! T2 per `docs/43-development-standards.md` §15.5: corpus
//! conformance over a fixture set with stable golden hashes plus
//! cross-backend orientation invariants.
//!
//! Slice-2 corpus (this file): three deterministic multipass
//! fixtures rendered at 16x16, each asserted against an FNV-1a
//! 64-bit hash of the FINAL pass's readback bytes plus
//! corner-sentinel checks. The hashes are pinned per backend in an
//! `ACCEPTED: &[u64]` slice (see slice-1's
//! `engine/render/tests/single_pass.rs` for the precedent); new
//! backends grow the set, they never replace existing entries.
//!
//! Hash precedent: FNV-1a 64-bit matches slice-1's
//! `engine/render/tests/single_pass.rs` and the
//! `engine/runtime/tests/determinism/main.rs` `EV-DETERMINISM`
//! corpus's hashing convention so reviewers see a single audit
//! pattern across the three evidence rows.
//!
//! # Why three fixtures
//!
//! Each fixture isolates one slice-2 deliverable:
//!
//! 1. `red_to_blue_two_pass_cascade` — exercises inter-pass texture
//!    sampling through a CONSTANT-color first pass, eliminating
//!    rasterizer / interpolation rounding from the equation. If a
//!    backend renders pass-1 red (255, 0, 0, 255) and pass-2 reads
//!    via `texture(sampler2D(...), uv)` and swaps r↔b, the readback
//!    is constant blue (0, 0, 255, 255). Backend-portable as a byte
//!    hash AND as the corner-sentinel test below.
//!
//! 2. `uv_attenuation_three_pass_cascade` — exercises the
//!    multipass scheduler on three sequential passes where each
//!    samples its predecessor and halves intensity. The pass-1 UV
//!    gradient (slice-1 corpus) reaches pass-3 with 0.25× intensity:
//!    corner (0,0) ~= (0, 0, 0.0625, 0.25), corner (15,15) ~= (0.24,
//!    0.24, 0.0625, 0.25). RGBA8Unorm quantization makes the byte
//!    hash backend-sensitive, but the corner-sentinel pin is
//!    backend-portable.
//!
//! 3. `single_pass_uniform_binding` — single-pass spec with one
//!    `uniform float u_brightness = 0.5` bound at the std140 layout
//!    that the slice-2 scheduler emits, validating end-to-end
//!    uniform-buffer marshalling. Output is a UV gradient * 0.5 in
//!    R/G with constant 0.25 in B (matching the slice-1 fixture
//!    shape with brightness applied). Backend-portable corner
//!    sentinels.
//!
//! Each fixture uses 16x16 resolution, matching the slice-1 corpus
//! stride so reviewers can compare hash sums between slice-1 and
//! slice-2 entries on the same hardware.
//!
//! # Headless adapter requirement
//!
//! Same as slice-1: Linux CI llvmpipe Vulkan, macOS Metal, Windows
//! Vulkan / GLES per wgpu's auto-discovery (`Backends::PRIMARY`).
//!
//! Anchors:
//! - `docs/adr/ADR-011-rendering-backend.md` — wgpu + naga + GLSL→SPIR-V pinning.
//! - `docs/26-evidence-artifact-registry.md` row 28 — `EV-RENDER-CONFORMANCE`.
//! - `docs/43-development-standards.md` §15.5 — T-tier definitions.
//! - `engine/render/src/multipass.rs` — scheduler entry point.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_render::{
    MultipassSpec, PassDef, PassInputBinding, UniformInput, UniformValue,
    compile_and_render_multipass,
};

/// FNV-1a 64-bit hash of a byte slice. Matches the precedent in
/// `engine/runtime/tests/determinism/main.rs` and
/// `engine/render/tests/single_pass.rs` so reviewers see one
/// hashing convention across all three evidence rows.
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

/// Common helper: index into a 16x16 RGBA8 readback at row-major
/// `(x, y)` and return the 4-byte pixel. Shared with slice-1
/// corpus's pattern.
fn pixel_at_16x16(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * 16 + x) * 4;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

/// Pass-1 fragment that emits constant red. Used as the "produce
/// something to sample" side of the red→blue cascade.
const CONSTANT_RED_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() {
    fragColor = vec4(1.0, 0.0, 0.0, 1.0);
}
"#;

/// Pass-2 fragment that samples pass-0 and swaps the R and B
/// channels. Pass-0 = constant red ⇒ pass-1 readback = constant
/// blue. The Vulkan-GLSL `texture2D` + `sampler` separation is the
/// only form naga's GLSL frontend accepts as a top-level resource;
/// see `engine/render/src/multipass.rs` module-level doc.
const SAMPLE_AND_SWAP_RB_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 1) uniform texture2D pass0;
layout(set = 0, binding = 2) uniform sampler pass0_sampler;
void main() {
    vec4 c = texture(sampler2D(pass0, pass0_sampler), uv);
    fragColor = vec4(c.b, c.g, c.r, c.a);
}
"#;

/// 2-pass red→blue cascade. Backend-portable byte hash (constant
/// color, no interpolation rounding) plus corner-sentinel
/// orientation pin.
#[test]
fn red_to_blue_two_pass_cascade() {
    let spec = MultipassSpec {
        passes: vec![
            PassDef {
                label: Some("red-to-blue/pass-0/red".to_string()),
                fragment_glsl: CONSTANT_RED_FRAGMENT.to_string(),
                ..PassDef::default()
            },
            PassDef {
                label: Some("red-to-blue/pass-1/swap-rb".to_string()),
                fragment_glsl: SAMPLE_AND_SWAP_RB_FRAGMENT.to_string(),
                input_textures: vec![PassInputBinding {
                    source_pass_index: 0,
                    sampler_uniform_name: "pass0".to_string(),
                }],
                ..PassDef::default()
            },
        ],
        target_size: (16, 16),
        ..MultipassSpec::default()
    };

    let pixels = compile_and_render_multipass(&spec)
        .expect("red→blue cascade must succeed on a working wgpu adapter");

    assert_eq!(
        pixels.len(),
        16 * 16 * 4,
        "readback must return one row per scanline with row padding stripped",
    );

    // Constant blue: every pixel should be (0, 0, 255, 255). FNV-1a
    // over 1024 bytes of `[0, 0, 255, 255]` is deterministic across
    // backends because there is no rasterizer interpolation in
    // play; the sampler reads a constant texture and swaps two
    // channels.
    let observed = fnv1a_64(&pixels);

    // Slice-2 backend-aware accept set. Constant-color cascade is
    // expected to hash identically across backends, but we keep
    // the per-backend accept-set shape so future drift (e.g. a
    // sampler-default-value change in wgpu, an ICD pre-multiplied
    // alpha tweak) surfaces here without forcing a regeneration
    // commit. See slice-1's `single_pass.rs` corpus comment for
    // the regeneration loop.
    const ACCEPTED: &[u64] = &[
        // Apple Silicon, Metal backend, wgpu 26.0.1 + naga 26.0.0.
        0x1f87_3219_3a3f_4d25,
    ];

    if !ACCEPTED.contains(&observed) {
        panic!(
            "red→blue cascade hash 0x{observed:016x} not in accepted set {ACCEPTED:#018x?}; \
             if this is a new backend, add the value to ACCEPTED in a docs(repo)-scoped \
             commit and reference this test path. If the hash drifted on a previously- \
             measured backend, do NOT regenerate blind — confirm the wgpu / naga / sampler \
             defaults change is intentional first.",
        );
    }

    // Backend-portable corner sentinels: every pixel should be
    // constant blue (0, 0, 255, 255). Allow a small ± tolerance for
    // RGBA8Unorm quantization through the texture-sample path.
    for (x, y) in [(0_usize, 0_usize), (15, 0), (0, 15), (15, 15), (8, 8)] {
        let p = pixel_at_16x16(&pixels, x, y);
        assert_eq!(
            p[0], 0,
            "R must be 0 (was swapped from B=0) at ({x},{y}), got {}",
            p[0]
        );
        assert_eq!(p[1], 0, "G must remain 0 at ({x},{y}), got {}", p[1]);
        assert_eq!(
            p[2], 255,
            "B must be 255 (was swapped from R=255) at ({x},{y}), got {}",
            p[2]
        );
        assert_eq!(p[3], 255, "A must be 255 (1.0) at ({x},{y}), got {}", p[3]);
    }
}

/// Pass-1 fragment of the UV-attenuation cascade: emit the slice-1
/// UV gradient with constant 0.25 blue.
const UV_GRADIENT_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() {
    fragColor = vec4(uv.x, uv.y, 0.25, 1.0);
}
"#;

/// Pass-N fragment that samples pass-0 (the previous pass in
/// declaration order) and halves the result. Used twice in the
/// 3-pass cascade to attenuate to 0.25×.
const SAMPLE_AND_HALVE_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 1) uniform texture2D prev;
layout(set = 0, binding = 2) uniform sampler prev_sampler;
void main() {
    vec4 c = texture(sampler2D(prev, prev_sampler), uv);
    fragColor = c * 0.5;
}
"#;

/// 3-pass UV-attenuation cascade. Pass 1 = UV gradient, pass 2 =
/// pass-1 * 0.5, pass 3 = pass-2 * 0.5. Final output is the slice-1
/// gradient at 0.25× intensity.
#[test]
fn uv_attenuation_three_pass_cascade() {
    let spec = MultipassSpec {
        passes: vec![
            PassDef {
                label: Some("uv-attenuation/pass-0/uv-gradient".to_string()),
                fragment_glsl: UV_GRADIENT_FRAGMENT.to_string(),
                ..PassDef::default()
            },
            PassDef {
                label: Some("uv-attenuation/pass-1/halve".to_string()),
                fragment_glsl: SAMPLE_AND_HALVE_FRAGMENT.to_string(),
                input_textures: vec![PassInputBinding {
                    source_pass_index: 0,
                    sampler_uniform_name: "prev".to_string(),
                }],
                ..PassDef::default()
            },
            PassDef {
                label: Some("uv-attenuation/pass-2/halve".to_string()),
                fragment_glsl: SAMPLE_AND_HALVE_FRAGMENT.to_string(),
                input_textures: vec![PassInputBinding {
                    source_pass_index: 1,
                    sampler_uniform_name: "prev".to_string(),
                }],
                ..PassDef::default()
            },
        ],
        target_size: (16, 16),
        ..MultipassSpec::default()
    };

    let pixels = compile_and_render_multipass(&spec)
        .expect("UV-attenuation cascade must succeed on a working wgpu adapter");

    assert_eq!(pixels.len(), 16 * 16 * 4);

    let observed = fnv1a_64(&pixels);

    // Backend-aware accept set. The byte hash IS sensitive to per-
    // backend RGBA8Unorm quantization through two sample / multiply
    // round trips; the cross-backend orientation pin below is the
    // strict invariant.
    const ACCEPTED: &[u64] = &[
        // Apple Silicon, Metal backend, wgpu 26.0.1 + naga 26.0.0.
        0x86bc_e8a3_c6ae_9a25,
    ];

    if !ACCEPTED.contains(&observed) {
        panic!(
            "UV-attenuation cascade hash 0x{observed:016x} not in accepted set {ACCEPTED:#018x?}; \
             new backends should add their measured value via a docs(repo)-scoped commit.",
        );
    }

    // Backend-portable corner sentinels:
    //
    // - corner (0, 0): UV ~= (0.03, 0.03), * 0.25 ⇒ R/G ~= 0.008
    //   (quantized to ~2 in u8); B = 0.25 / 4 = 0.0625 (~16 in u8).
    // - corner (15, 15): UV ~= (0.97, 0.97), * 0.25 ⇒ R/G ~= 0.24
    //   (~62 in u8); B ~= 16 in u8.
    // - alpha = 1.0 / 4 = 0.25 (~64 in u8) at every pixel.
    let top_left = pixel_at_16x16(&pixels, 0, 0);
    assert!(
        top_left[0] < 16,
        "x=0 R must be near 0 (UV.x*0.25 ~= 0.008), got {}",
        top_left[0],
    );
    assert!(
        top_left[1] < 16,
        "y=0 G must be near 0 (UV.y*0.25 ~= 0.008), got {}",
        top_left[1],
    );
    let bottom_right = pixel_at_16x16(&pixels, 15, 15);
    assert!(
        (48..=80).contains(&bottom_right[0]),
        "x=15 R must be near 0.24 (UV.x*0.25 ~= 0.24), got {}",
        bottom_right[0],
    );
    assert!(
        (48..=80).contains(&bottom_right[1]),
        "y=15 G must be near 0.24, got {}",
        bottom_right[1],
    );

    // Blue channel: 0.25 * 0.5 * 0.5 = 0.0625 ⇒ ~16 in u8 (allow
    // ± a few ULP for backend rounding).
    for (x, y) in [(0_usize, 0_usize), (15, 0), (0, 15), (15, 15), (8, 8)] {
        let p = pixel_at_16x16(&pixels, x, y);
        assert!(
            (12..=20).contains(&p[2]),
            "B must be near 16 (0.25 * 0.25 = 0.0625) at ({x},{y}), got {}",
            p[2],
        );
        // Alpha attenuated to 0.25 ⇒ ~64.
        assert!(
            (60..=68).contains(&p[3]),
            "A must be near 64 (1.0 * 0.25 = 0.25) at ({x},{y}), got {}",
            p[3],
        );
    }
}

/// Single-pass with one `uniform float u_brightness` bound to 0.5,
/// applied to the UV gradient. Validates end-to-end uniform buffer
/// marshalling through the slice-2 std140 layout.
const UV_GRADIENT_BRIGHTNESS_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0) uniform Uniforms {
    float u_brightness;
};
void main() {
    fragColor = vec4(uv.x * u_brightness, uv.y * u_brightness, 0.25, 1.0);
}
"#;

/// Uniform binding fixture. Single pass with one float uniform.
#[test]
fn single_pass_uniform_binding() {
    let spec = MultipassSpec {
        passes: vec![PassDef {
            label: Some("uniform-binding/pass-0/uv-brightness".to_string()),
            fragment_glsl: UV_GRADIENT_BRIGHTNESS_FRAGMENT.to_string(),
            uniforms: vec![UniformInput {
                name: "u_brightness".to_string(),
                value: UniformValue::Scalar(0.5),
            }],
            ..PassDef::default()
        }],
        target_size: (16, 16),
        ..MultipassSpec::default()
    };

    let pixels = compile_and_render_multipass(&spec)
        .expect("uniform-binding fixture must succeed on a working wgpu adapter");

    assert_eq!(pixels.len(), 16 * 16 * 4);

    let observed = fnv1a_64(&pixels);

    // Backend-aware accept set.
    const ACCEPTED: &[u64] = &[
        // Apple Silicon, Metal backend, wgpu 26.0.1 + naga 26.0.0.
        0xe6ac_ab35_8e36_9125,
    ];

    if !ACCEPTED.contains(&observed) {
        panic!(
            "uniform-binding hash 0x{observed:016x} not in accepted set {ACCEPTED:#018x?}; \
             new backends should add their measured value via a docs(repo)-scoped commit.",
        );
    }

    // Backend-portable corner sentinels:
    //
    // - x=0 R ~= 0 (UV.x ~= 0.03 * 0.5 ~= 0.015 ⇒ ~4 in u8).
    // - x=15 R ~= 124 (UV.x ~= 0.97 * 0.5 ~= 0.484 ⇒ ~123-125 in u8).
    // - B = 0.25 ⇒ ~64 at every pixel (NOT scaled by u_brightness).
    let top_left = pixel_at_16x16(&pixels, 0, 0);
    let top_right = pixel_at_16x16(&pixels, 15, 0);
    assert!(
        top_left[0] < 16,
        "x=0 R must be near 0 (UV.x*0.5 ~= 0.015), got {}",
        top_left[0],
    );
    assert!(
        (112..=136).contains(&top_right[0]),
        "x=15 R must be near 124 (UV.x*0.5 ~= 0.484), got {}",
        top_right[0],
    );

    // Blue channel: constant 0.25 ⇒ ~64 (NOT attenuated).
    for (x, y) in [(0_usize, 0_usize), (15, 0), (0, 15), (15, 15)] {
        let p = pixel_at_16x16(&pixels, x, y);
        assert!(
            (60..=68).contains(&p[2]),
            "B must be near 64 (constant 0.25, not affected by u_brightness) at ({x},{y}), got {}",
            p[2],
        );
        assert_eq!(p[3], 255, "A must be 255 (1.0) at ({x},{y})");
    }
}

/// Single-pass-only multipass spec that produces an output equivalent
/// to slice-1's `compile_and_render_single_pass` for the same UV
/// gradient. The byte streams are not strictly bit-identical (the
/// multipass path allocates its target with `TEXTURE_BINDING |
/// COPY_SRC | RENDER_ATTACHMENT` even on the final pass to keep the
/// shape uniform across slice-2 single- and multi-pass inputs;
/// that should not perturb the final-pass-readback bytes, but the
/// strict-bit-equality check is left to a future slice that needs it).
/// The slice-2 invariant: corner sentinels MUST agree with the
/// slice-1 corpus's well-defined orientation. Loose-check option (a)
/// from the brief because the strict bit-equality option (b) would
/// require restructuring `compile_and_render_single_pass` to share
/// the multipass codepath, which is out of scope for slice 2.
#[test]
fn single_pass_multipass_consistency_invariant() {
    let spec = MultipassSpec {
        passes: vec![PassDef {
            label: Some("consistency/pass-0/uv-gradient".to_string()),
            fragment_glsl: UV_GRADIENT_FRAGMENT.to_string(),
            ..PassDef::default()
        }],
        target_size: (16, 16),
        ..MultipassSpec::default()
    };
    let pixels =
        compile_and_render_multipass(&spec).expect("single-element multipass spec must succeed");

    assert_eq!(pixels.len(), 16 * 16 * 4);

    // Same orientation pins as slice-1's `uv_gradient_corner_sentinels`.
    let top_left = pixel_at_16x16(&pixels, 0, 0);
    let top_right = pixel_at_16x16(&pixels, 15, 0);
    assert!(
        top_left[0] < 64,
        "x=0 R must be near 0 (UV.x ~= 0.03), got {}",
        top_left[0],
    );
    assert!(
        top_right[0] > 192,
        "x=15 R must be near 255 (UV.x ~= 0.97), got {}",
        top_right[0],
    );

    // Blue channel 0.25 quantized to ~64.
    for (x, y) in [(0_usize, 0_usize), (15, 0), (0, 15), (15, 15)] {
        let p = pixel_at_16x16(&pixels, x, y);
        assert!(
            (60..=68).contains(&p[2]),
            "B must be near 64 (constant 0.25) at ({x},{y}), got {}",
            p[2],
        );
        assert_eq!(p[3], 255, "A must be 255 (1.0) at ({x},{y})");
    }
}
