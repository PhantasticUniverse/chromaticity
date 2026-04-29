// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-RENDER-CONFORMANCE (T2) — GLSL #include preprocessor → naga → wgpu end-to-end corpus.
//!
//! Advances `docs/26-evidence-artifact-registry.md` row 28
//! (`EV-RENDER-CONFORMANCE`) past the slice-2 multipass T2 toward the
//! Phase-2 exit shape. T2 per
//! `docs/43-development-standards.md` §15.5: corpus conformance over a
//! fixture set with a stable golden hash.
//!
//! Slice-3 corpus (this file): a small fragment shader issues
//! `#include "lygia/color/space/rgb2luma.glsl"` (a leaf Lygia file with
//! no transitive includes), the preprocessor inlines the file, naga
//! validates the resulting GLSL, and the single-pass entry point
//! renders a 16x16 RGBA8Unorm gradient. The readback bytes are FNV-1a-
//! hashed; the hash is asserted against a backend-aware accept set
//! (seeded with the local Apple Silicon Metal observation), matching
//! the slice-1 / slice-2 corpus convention.
//!
//! The fixture exercises the full Lygia → preprocessor → naga →
//! wgpu path so any drift in any stage surfaces in this single test:
//!
//! - Vendor snapshot version drift (Lygia 1.1.4 → 1.1.x with a
//!   different `rgb2luma` implementation) would change the FNV-1a
//!   hash because the inlined function body produces different
//!   pixels.
//! - Preprocessor drift (cycle guard, parser, namespace-prefix
//!   stripping) would surface as a compile-time naga rejection or as
//!   an `IncludeError`.
//! - naga / wgpu drift behaves identically to the slice-1 single-pass
//!   corpus drift: a per-backend hash bump in the accept set.
//!
//! # Why only one fixture today
//!
//! Slice 3's job is to validate that the preprocessor → existing
//! pipeline integration works. Phase-2-exit-quality conformance still
//! requires feedback texturing, transitions, audio uniforms, and
//! float-target coverage (not in this slice). One fixture is the
//! minimal shape that lets the registry row's `Generation Source`
//! cell start citing the Lygia path.
//!
//! Anchors:
//! - `docs/adr/ADR-011-rendering-backend.md` — wgpu + naga + GLSL→SPIR-V pinning.
//! - `docs/26-evidence-artifact-registry.md` row 28 — `EV-RENDER-CONFORMANCE`.
//! - `docs/26-evidence-artifact-registry.md` `EV-LIBRARY-LICENSE-TRIAL` — Lygia trial vendor.
//! - `docs/29-cleanroom-legal-policy-baseline.md` Sec.Bundled GLSL Library Handling.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::path::PathBuf;

use chromaticity_render::{compile_and_render_single_pass, resolve_includes};

/// A small fragment that pulls Lygia's `rgb2luma` and applies it to
/// the interpolated UV (treated as a faux RGB color) to produce a
/// luminance-weighted gradient. Self-contained: the inlined function
/// is a leaf (no transitive includes) so the corpus does not depend on
/// the recursive resolution path — that path is already pinned by the
/// T1 `nested_includes_resolve` drift pin in
/// `engine/render/src/glsl_includes.rs`.
const LYGIA_LUMA_FRAGMENT: &str = r#"#version 450
#include "lygia/color/space/rgb2luma.glsl"
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() {
    float lum = rgb2luma(vec3(uv.x, uv.y, 0.5));
    fragColor = vec4(vec3(lum), 1.0);
}
"#;

/// Path to the Lygia vendor snapshot relative to the workspace root.
fn vendor_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .join("third_party")
        .join("lygia")
}

/// FNV-1a 64-bit hash matching the slice-1 / slice-2 corpus
/// convention.
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

#[test]
fn lygia_luma_corpus_hash_is_stable() {
    let resolved = resolve_includes(LYGIA_LUMA_FRAGMENT, &vendor_root())
        .expect("Lygia rgb2luma include must resolve when the vendor snapshot is present");

    // Sanity: the inlined function symbol should be present.
    assert!(
        resolved.contains("FNC_RGB2LUMA"),
        "preprocessor must inline rgb2luma.glsl",
    );

    let pixels = compile_and_render_single_pass(&resolved, (16, 16))
        .expect("Lygia luma fragment must compile and render on a working wgpu adapter");

    assert_eq!(
        pixels.len(),
        16 * 16 * 4,
        "readback must return one row per scanline with row padding stripped",
    );

    let observed = fnv1a_64(&pixels);

    // Per the slice-1 / slice-2 convention: the accept set grows as
    // new backends are measured on CI. A new entry lands in a follow-
    // up commit when CI surfaces a not-yet-seen backend hash; the
    // panic message prints the observed value for short regeneration.
    const ACCEPTED: &[u64] = &[
        // Apple Silicon, Metal backend, wgpu 26 + naga 26, Lygia 1.1.4.
        0x8534_bff1_aab5_ddfd,
    ];

    if !ACCEPTED.contains(&observed) {
        panic!(
            "Lygia luma corpus hash 0x{observed:016x} not in accepted set {ACCEPTED:#018x?}; \
             if this is a new backend, add the value to ACCEPTED in a docs(repo)-scoped commit \
             and reference this test path. If the hash drifted on a previously-measured backend, \
             do NOT regenerate blind — confirm the wgpu / naga upgrade or Lygia snapshot bump is \
             intentional first.",
        );
    }
}
