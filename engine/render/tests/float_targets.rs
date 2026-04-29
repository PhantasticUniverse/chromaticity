// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-RENDER-CONFORMANCE (T2) — float-target corpus (slice 4).
//!
//! Advances `docs/26-evidence-artifact-registry.md` row 28 toward
//! the ADR-011 §Decision float-target conformance requirement.
//! T2 per `docs/43-development-standards.md` §15.5: corpus
//! conformance over a fixture set with stable golden hashes plus
//! cross-backend orientation invariants.
//!
//! Slice-4 corpus (this file): two float-target fixtures rendered
//! at 16×16, each asserted against EITHER an FNV-1a 64-bit hash
//! over a precision-quantized byte stream OR corner-sentinel
//! invariants when the underlying float bits are not bit-stable
//! across backends.
//!
//! # Hashing strategy by format
//!
//! Per the slice-4 brief's "On corpus determinism" section:
//!
//! - **`Rgba8Unorm` baseline** — full byte hash (slice-1 / slice-2
//!   precedent). The slice-1 single-pass UV-gradient corpus already
//!   pins this hash; this fixture re-derives it through the
//!   slice-4 multipass scheduler to confirm the
//!   `frame_count = 1` + `target_format::default()` path remains
//!   bit-identical.
//!
//! - **`Rgba16Float`** — full byte hash. 16-bit float bit patterns
//!   are deterministic when fed deterministic GLSL on identical
//!   wgpu device. The slice-4 hash will differ between Apple
//!   Silicon Metal and Linux Vulkan — the cross-backend matrix is
//!   deferred to a follow-up slice. Within a single backend, the
//!   bytes are reproducible.
//!
//! - **`Rgba32Float`** — corner-sentinel-only. f32 trailing-bit
//!   adapter rounding makes a full byte hash unstable across
//!   backends; the test compares discrete pixel values (read out
//!   as f32 from the readback bytes) against an absolute tolerance.
//!
//! # Headless adapter requirement
//!
//! Float-target rendering requires the adapter to advertise
//! `RENDER_ATTACHMENT` for the requested format. On Apple Silicon
//! Metal (this dev seed) and Linux llvmpipe Vulkan (CI),
//! `Rgba16Float` and `Rgba32Float` both qualify. Adapters that
//! lack the capability surface as
//! `RenderError::FloatTargetUnsupported` and the test prints a
//! diagnostic that this is expected on a non-conforming adapter
//! (rather than failing).
//!
//! Anchors:
//! - `docs/adr/ADR-011-rendering-backend.md` — float-target
//!   §Decision pin.
//! - `docs/26-evidence-artifact-registry.md` row 28 — `EV-RENDER-CONFORMANCE`.
//! - `engine/render/src/multipass.rs` — `TargetFormat` enum +
//!   `PassDef::target_format` field.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_render::{
    MultipassSpec, PassDef, RenderError, TargetFormat, compile_and_render_multipass,
};

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

/// Decode a 16-bit half-float from its IEEE 754 bit pattern.
///
/// Half-float layout: 1 sign bit, 5 exponent bits, 10 mantissa
/// bits. The standard bit-tricks reconstruction.
fn half_to_f32(bits: u16) -> f32 {
    let sign = ((bits >> 15) & 0x0001) as u32;
    let exponent = ((bits >> 10) & 0x001f) as u32;
    let mantissa = (bits & 0x03ff) as u32;
    let f32_bits = if exponent == 0 {
        if mantissa == 0 {
            sign << 31
        } else {
            // Denormalized half ⇒ normalized f32.
            let mut e = 1_u32;
            let mut m = mantissa;
            while m & 0x0400 == 0 {
                m <<= 1;
                e = e.wrapping_sub(1);
            }
            (sign << 31) | ((e + 127 - 15) << 23) | ((m & 0x03ff) << 13)
        }
    } else if exponent == 31 {
        (sign << 31) | (0xff << 23) | (mantissa << 13)
    } else {
        (sign << 31) | ((exponent + 127 - 15) << 23) | (mantissa << 13)
    };
    f32::from_bits(f32_bits)
}

/// Read an `Rgba16Float` pixel as a `[f32; 4]` from row-major
/// readback bytes (8 bytes per pixel: four little-endian u16s).
fn read_rgba16f_pixel(pixels: &[u8], x: usize, y: usize, width: usize) -> [f32; 4] {
    let i = (y * width + x) * 8;
    [
        half_to_f32(u16::from_le_bytes([pixels[i], pixels[i + 1]])),
        half_to_f32(u16::from_le_bytes([pixels[i + 2], pixels[i + 3]])),
        half_to_f32(u16::from_le_bytes([pixels[i + 4], pixels[i + 5]])),
        half_to_f32(u16::from_le_bytes([pixels[i + 6], pixels[i + 7]])),
    ]
}

/// Read an `Rgba32Float` pixel as a `[f32; 4]` from row-major
/// readback bytes (16 bytes per pixel: four little-endian f32s).
fn read_rgba32f_pixel(pixels: &[u8], x: usize, y: usize, width: usize) -> [f32; 4] {
    let i = (y * width + x) * 16;
    let read = |offset: usize| {
        f32::from_le_bytes([
            pixels[offset],
            pixels[offset + 1],
            pixels[offset + 2],
            pixels[offset + 3],
        ])
    };
    [read(i), read(i + 4), read(i + 8), read(i + 12)]
}

/// Baseline: `Rgba8Unorm` UV gradient through the slice-4
/// scheduler with `frame_count = 1`. Pins the slice-4 multipass
/// codepath's byte stream for the trivial single-pass case so the
/// `Rgba16Float` and `Rgba32Float` hashes below have a sibling
/// integer-format anchor to compare against. The byte stream
/// differs from `engine/render/tests/single_pass.rs`'s slice-1
/// single_pass codepath (the multipass scheduler allocates its
/// final-pass target with `TEXTURE_BINDING | RENDER_ATTACHMENT |
/// COPY_SRC`, which is structurally distinct from slice-1's
/// `RENDER_ATTACHMENT | COPY_SRC` allocation; see slice-2's
/// `single_pass_multipass_consistency_invariant` test for the
/// orientation-only equivalence check).
#[test]
fn rgba8unorm_baseline_continuity() {
    let spec = MultipassSpec {
        passes: vec![PassDef {
            label: Some("float-targets/rgba8unorm-baseline".to_string()),
            fragment_glsl: UV_GRADIENT_FRAGMENT.to_string(),
            target_format: TargetFormat::Rgba8Unorm,
            ..PassDef::default()
        }],
        target_size: (16, 16),
        frame_count: 1,
    };

    let pixels = compile_and_render_multipass(&spec)
        .expect("Rgba8Unorm baseline must succeed on a working wgpu adapter");

    assert_eq!(pixels.len(), 16 * 16 * 4);

    let observed = fnv1a_64(&pixels);

    // Same accept set as the slice-2 single-pass-multipass-
    // consistency invariant — slice-4 must preserve this byte
    // stream bit-for-bit.
    const ACCEPTED: &[u64] = &[
        // Apple Silicon, Metal backend, wgpu 26.0.1 + naga 26.0.0.
        0xc1b2_b7c4_7c8f_f6a5,
    ];

    if !ACCEPTED.contains(&observed) {
        panic!(
            "Rgba8Unorm baseline hash 0x{observed:016x} not in accepted set {ACCEPTED:#018x?}; \
             new backends should add their measured value via a docs(repo)-scoped commit. \
             If this drifted on a previously-measured backend, the slice-4 multipass \
             texture allocation usage flags or copy-encoder split may have regressed.",
        );
    }
}

/// `Rgba16Float` UV gradient. Full byte-hash since 16-bit float
/// bit patterns are deterministic on a single backend.
#[test]
fn rgba16float_byte_hash_is_stable() {
    let spec = MultipassSpec {
        passes: vec![PassDef {
            label: Some("float-targets/rgba16f-uv-gradient".to_string()),
            fragment_glsl: UV_GRADIENT_FRAGMENT.to_string(),
            target_format: TargetFormat::Rgba16Float,
            ..PassDef::default()
        }],
        target_size: (16, 16),
        frame_count: 1,
    };

    let pixels = match compile_and_render_multipass(&spec) {
        Ok(p) => p,
        Err(RenderError::FloatTargetUnsupported { format, reason }) => {
            // Some backends may lack RGBA16F render-attachment
            // support; skip the test with a diagnostic rather
            // than failing.
            eprintln!(
                "skipping rgba16float test: adapter lacks RENDER_ATTACHMENT for {format} \
                 ({reason})"
            );
            return;
        }
        Err(e) => panic!("rgba16f fixture failed unexpectedly: {e:?}"),
    };

    // 16x16 pixels * 8 bytes per pixel.
    assert_eq!(pixels.len(), 16 * 16 * 8);

    let observed = fnv1a_64(&pixels);

    const ACCEPTED: &[u64] = &[
        // Apple Silicon, Metal backend, wgpu 26.0.1 + naga 26.0.0.
        // 16-bit float bit patterns are stable on a single
        // backend; cross-backend variance is captured by adding
        // the observed hash here.
        0xcf7d_53c0_0144_c805,
        // Linux x86_64, Vulkan backend (Mesa llvmpipe), wgpu 26 +
        // naga 26. Different rasterization fixed-function path
        // produces a distinct bit-pattern for the same UV-gradient
        // GLSL — both hashes are correct outputs of their backend.
        0x970b_557b_e790_3805,
    ];

    if !ACCEPTED.contains(&observed) {
        panic!(
            "Rgba16Float hash 0x{observed:016x} not in accepted set {ACCEPTED:#018x?}; \
             new backends should add their measured value via a docs(repo)-scoped commit.",
        );
    }

    // Backend-portable corner sentinels: the half-float pixel at
    // (0,0) should decode to roughly (0.03, 0.03, 0.25, 1.0); at
    // (15,15) roughly (0.97, 0.97, 0.25, 1.0).
    let top_left = read_rgba16f_pixel(&pixels, 0, 0, 16);
    let bottom_right = read_rgba16f_pixel(&pixels, 15, 15, 16);
    let tol = 0.05_f32;
    assert!(
        (top_left[0] - 0.03).abs() < tol,
        "rgba16f (0,0) R must be ~0.03, got {}",
        top_left[0],
    );
    assert!(
        (bottom_right[0] - 0.97).abs() < tol,
        "rgba16f (15,15) R must be ~0.97, got {}",
        bottom_right[0],
    );
    for (x, y) in [(0_usize, 0_usize), (15, 0), (0, 15), (15, 15), (8, 8)] {
        let p = read_rgba16f_pixel(&pixels, x, y, 16);
        assert!(
            (p[2] - 0.25).abs() < tol,
            "rgba16f B must be ~0.25 at ({x},{y}), got {}",
            p[2],
        );
        assert!(
            (p[3] - 1.0).abs() < tol,
            "rgba16f A must be ~1.0 at ({x},{y}), got {}",
            p[3],
        );
    }
}

/// `Rgba32Float` UV gradient. Corner-sentinel-only — f32
/// trailing-bit rounding is not bit-stable across backends.
#[test]
fn rgba32float_corner_sentinels() {
    let spec = MultipassSpec {
        passes: vec![PassDef {
            label: Some("float-targets/rgba32f-uv-gradient".to_string()),
            fragment_glsl: UV_GRADIENT_FRAGMENT.to_string(),
            target_format: TargetFormat::Rgba32Float,
            ..PassDef::default()
        }],
        target_size: (16, 16),
        frame_count: 1,
    };

    let pixels = match compile_and_render_multipass(&spec) {
        Ok(p) => p,
        Err(RenderError::FloatTargetUnsupported { format, reason }) => {
            eprintln!(
                "skipping rgba32float test: adapter lacks RENDER_ATTACHMENT for {format} \
                 ({reason})"
            );
            return;
        }
        Err(e) => panic!("rgba32f fixture failed unexpectedly: {e:?}"),
    };

    // 16x16 pixels * 16 bytes per pixel.
    assert_eq!(pixels.len(), 16 * 16 * 16);

    // Corner sentinels read out as f32 directly. f32 has more
    // mantissa bits than the 1/16 = 0.0625 grid of UV centers
    // produces, so the read-side tolerance is tighter than the
    // half-float test.
    let top_left = read_rgba32f_pixel(&pixels, 0, 0, 16);
    let bottom_right = read_rgba32f_pixel(&pixels, 15, 15, 16);
    let tol = 0.005_f32;
    let expected_tl_uv = 0.5 / 16.0; // pixel-center UV at row/col 0.
    let expected_br_uv = 15.5 / 16.0; // pixel-center UV at row/col 15.
    assert!(
        (top_left[0] - expected_tl_uv).abs() < tol,
        "rgba32f (0,0) R must be ~{expected_tl_uv}, got {}",
        top_left[0],
    );
    assert!(
        (bottom_right[0] - expected_br_uv).abs() < tol,
        "rgba32f (15,15) R must be ~{expected_br_uv}, got {}",
        bottom_right[0],
    );
    for (x, y) in [(0_usize, 0_usize), (15, 0), (0, 15), (15, 15), (8, 8)] {
        let p = read_rgba32f_pixel(&pixels, x, y, 16);
        assert!(
            (p[2] - 0.25).abs() < tol,
            "rgba32f B must be exactly 0.25 at ({x},{y}), got {}",
            p[2],
        );
        assert!(
            (p[3] - 1.0).abs() < tol,
            "rgba32f A must be exactly 1.0 at ({x},{y}), got {}",
            p[3],
        );
    }
}
