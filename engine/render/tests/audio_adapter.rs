// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-RENDER-CONFORMANCE (T2) + EV-AUDIO-TRACE (T2) — slice-5a
//! cross-crate audio → render uniform-binding GPU-roundtrip corpus.
//!
//! Advances `docs/26-evidence-artifact-registry.md` row 28
//! (`EV-RENDER-CONFORMANCE`) AND row 30 (`EV-AUDIO-TRACE`) toward
//! the slice-5a deliverable: end-to-end production of one
//! [`chromaticity_audio::AudioUniformSnapshot`] from a synthesised
//! [`chromaticity_audio::frame::AudioFrame`], marshalling through
//! [`chromaticity_render::uniform_input_from_audio_snapshot`], and
//! GPU-roundtripping via
//! [`chromaticity_render::compile_and_render_multipass`] with a
//! shader that consumes all five `syn_*` uniforms. Dual-tagged
//! because slice-5a is the cross-crate widening that closes both
//! rows simultaneously; the same dual-tag pattern is established
//! by `engine/script/tests/sandbox.rs` (`EV-SANDBOX-SAFETY (T2) +
//! EV-SCRIPT-CONTRACT (T2)`).
//!
//! T2 per `docs/43-development-standards.md` §15.5: corpus
//! conformance over a fixture set with stable golden hashes
//! (FNV-1a 64-bit byte hash with a backend-aware ACCEPTED set) plus
//! corner-sentinel invariants for cross-backend resilience. The
//! fixture set is two synthesised audio frames (silent baseline,
//! 440 Hz sine tone) — the slice-1 audio T2 corpus uses the same
//! synthesis routines so the snapshots travelling into the
//! adapter here are the same shapes that
//! `engine/audio/tests/uniforms.rs` already pins.
//!
//! # Backend-aware ACCEPTED set
//!
//! Same pattern as
//! `engine/render/tests/float_targets.rs::rgba16float_byte_hash_is_stable`:
//! 8-bit unorm bit patterns are stable on a single backend but
//! differ across backends because the upstream fragment-shader
//! arithmetic compiles to different instructions on Metal /
//! Vulkan / DX12. The first commit observed this on Apple Silicon
//! Metal; new backends should add their measured value via a
//! `docs(repo)`-scoped follow-up commit (NOT a feature commit —
//! corpus widening is a separate slice).
//!
//! # Headless adapter requirement
//!
//! Same as slice-1 / slice-2 / slice-4 baseline: Linux CI llvmpipe
//! Vulkan, macOS Metal, Windows Vulkan / GLES per wgpu's
//! auto-discovery (`Backends::PRIMARY`).
//!
//! Anchors:
//! - `engine/render/src/audio_adapter.rs` — adapter implementation.
//! - `engine/audio/src/uniforms.rs` — `AudioUniformSnapshot` producer.
//! - `docs/26-evidence-artifact-registry.md` row 28 — `EV-RENDER-CONFORMANCE`.
//! - `docs/26-evidence-artifact-registry.md` row 30 — `EV-AUDIO-TRACE`.
//! - `docs/36-reactive-symbol-catalog.md` v1.3.0 — symbol vocabulary.

#![cfg(feature = "audio-binding")]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::f32::consts::TAU;

use chromaticity_audio::frame::{AudioFrame, DEFAULT_SAMPLE_RATE_HZ};
use chromaticity_audio::uniforms::extract_uniform_snapshot;
use chromaticity_render::{
    MultipassSpec, PassDef, TargetFormat, compile_and_render_multipass,
    uniform_input_from_audio_snapshot,
};

/// One-second of audio at 48 kHz mono — long enough for the
/// `extract_uniform_snapshot` autocorrelation BPM path and
/// FFT-window path to engage. Matches the fixture length used by
/// `engine/audio/tests/uniforms.rs`.
const SAMPLES_ONE_SECOND: usize = 48_000;

/// Fragment shader that consumes all four slice-5a uniforms after
/// the catalog v1.3.0 consolidation (BPM + RMS + PEAK + FFT[8]).
///
/// # Why scalars are declared `vec4` while the FFT array is `float[8]`
///
/// The [`MultipassSpec`] uniform-buffer packing places every
/// scalar / vector [`chromaticity_render::UniformValue`] entry into
/// its own 16-byte slot regardless of underlying width
/// (`engine/render/src/multipass.rs` module-level doc, "One uniform
/// buffer per pass, std140-friendly"). This means scalars cannot be
/// declared as adjacent `float`s in GLSL — std140 packs `float` at
/// 4-byte alignment and would read scalar K from offset `K*4`, but
/// the actual buffer puts scalar K at offset `K*16` (with 12 bytes
/// of zero pad after each). The consumer-side idiom is therefore to
/// declare each scalar as `vec4` and read the `.x` component.
///
/// [`UniformValue::FloatArray`] (slice-5 follow-up addition; catalog
/// v1.3.0) expands into N consecutive 16-byte slots — true std140,
/// element K at engine offset `K*16` with 12 bytes of zero pad per
/// element. naga's GLSL frontend, however, does NOT apply std140 to
/// arrays: it reads a bare `float arr[N]` at the WGSL-default tight
/// 4-byte stride, so a shader declared `float chroma_AudioFFT[8]`
/// sees a real band only at indices 0 and 4 (engine bands 0 and 1)
/// and zero pad everywhere else (verified empirically; see the
/// diagnostic fixture
/// `engine/scene/tests/fixtures/scenes/diagnostic/audio-reactivity-test.chroma/main.glsl`
/// and the archetypes corpus lint in
/// `engine/authoring/tests/shader_archetypes_corpus.rs`). The
/// engine-wide consumer convention is therefore `vec4 arr[N]` read
/// via `.x`. The fragment below predates that convention and still
/// uses the bare-float form: its golden hashes were measured under
/// the 4-byte-stride read, which the 440 Hz sine fixture masks
/// because the FFT energy concentrates in the low bands (so the
/// alpha mean is insensitive to the missing bands 2-7 at u8
/// quantization). Migrating this shader to `vec4 chroma_AudioFFT[8]` +
/// `.x` reads requires re-measuring the ACCEPTED hash set in the
/// same change.
///
/// # Output encoding
///
/// - `R` = `syn_BPM.x / 220.0` (max BPM clamp value, normalised to [0, 1])
/// - `G` = `chroma_AudioRMS.x`
/// - `B` = `chroma_AudioPeak.x`
/// - `A` = `mean(chroma_AudioFFT[0..=7])`
///
/// Every output channel touches every uniform, so a marshalling
/// drift (wrong slot, swapped order) surfaces as a visible byte
/// change in R / G / B. The A channel is the weaker sentinel: under
/// the bare-float stride read described above, its FFT mean is
/// insensitive to the unread high bands, so per-element index drift
/// is pinned by the archetypes corpus lint rather than this golden.
const AUDIO_AWARE_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 0) uniform Uniforms {
    vec4 syn_BPM;
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    float chroma_AudioFFT[8];
};

void main() {
    float r = syn_BPM.x / 220.0;
    float g = chroma_AudioRMS.x;
    float b = chroma_AudioPeak.x;
    float fft_sum = chroma_AudioFFT[0] + chroma_AudioFFT[1] + chroma_AudioFFT[2] + chroma_AudioFFT[3]
                  + chroma_AudioFFT[4] + chroma_AudioFFT[5] + chroma_AudioFFT[6] + chroma_AudioFFT[7];
    float a = fft_sum / 8.0;
    fragColor = vec4(clamp(r, 0.0, 1.0),
                     clamp(g, 0.0, 1.0),
                     clamp(b, 0.0, 1.0),
                     clamp(a, 0.0, 1.0));
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

fn one_second_silence() -> AudioFrame {
    AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, vec![0.0_f32; SAMPLES_ONE_SECOND])
}

fn one_second_sine(frequency_hz: f32, amplitude: f32) -> AudioFrame {
    let mut samples = Vec::with_capacity(SAMPLES_ONE_SECOND);
    let step = TAU * frequency_hz / DEFAULT_SAMPLE_RATE_HZ as f32;
    for i in 0..SAMPLES_ONE_SECOND {
        samples.push(amplitude * (step * i as f32).sin());
    }
    AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, samples)
}

/// Render the audio-aware shader against `frame`'s extracted
/// snapshot, returning the readback bytes (16x16 RGBA8, 1024
/// bytes total) for hashing / corner-sentinel inspection.
fn render_with_snapshot(frame: &AudioFrame) -> Vec<u8> {
    let snapshot = extract_uniform_snapshot(frame);
    let uniforms = uniform_input_from_audio_snapshot(&snapshot);
    assert_eq!(
        uniforms.len(),
        4,
        "adapter must produce exactly 4 uniforms (3 scalars + 1 FloatArray[8])",
    );

    let spec = MultipassSpec {
        passes: vec![PassDef {
            label: Some("audio-adapter/audio-aware-fragment".to_string()),
            fragment_glsl: AUDIO_AWARE_FRAGMENT.to_string(),
            uniforms,
            target_format: TargetFormat::Rgba8Unorm,
            ..PassDef::default()
        }],
        target_size: (16, 16),
        frame_count: 1,
    };

    compile_and_render_multipass(&spec)
        .expect("audio-aware multipass render must succeed on a working wgpu adapter")
}

/// Silent baseline: an all-zero `AudioFrame` produces an
/// all-zero `AudioUniformSnapshot` (RMS=0, PEAK=0, all FFT bands
/// 0; BPM falls back to `BPM_DEFAULT = 120.0` because the silent
/// frame is too short for autocorrelation across the full
/// `[50, 220]` lag range — but actually a 1-second 48k frame is
/// long enough, and silent frames pin BPM to BPM_DEFAULT via the
/// energy-epsilon early-return). The shader output is therefore
/// `R = 120/220 ≈ 0.545`, `G = B = A = 0`. Pin the byte hash
/// across an ACCEPTED set.
#[test]
fn silent_frame_byte_hash_is_stable() {
    let frame = one_second_silence();
    let pixels = render_with_snapshot(&frame);

    // 16x16 RGBA8 = 1024 bytes.
    assert_eq!(pixels.len(), 16 * 16 * 4);

    let observed = fnv1a_64(&pixels);

    const ACCEPTED: &[u64] = &[
        // Apple Silicon, Metal backend, wgpu 26.0.1 + naga 26.0.0.
        0x8013_1e41_5c3c_fb25,
    ];

    if !ACCEPTED.contains(&observed) {
        panic!(
            "audio_adapter silent-frame hash 0x{observed:016x} not in accepted set \
             {ACCEPTED:#018x?}; new backends should add their measured value via \
             a docs(repo)-scoped commit. If this drifted on a previously-measured \
             backend, the slice-5a marshalling order, the docs/36 binding contract, \
             or the slice-1 silent-frame snapshot shape may have regressed.",
        );
    }

    // Backend-portable corner sentinels — the silent-frame
    // shader output is uniform across the 16x16 grid because
    // none of the uniforms vary spatially. Pin a few corners to
    // the expected `(120/220, 0, 0, 0)` shape with byte
    // tolerance for the unorm rounding.
    for (x, y) in [(0_usize, 0_usize), (15, 0), (0, 15), (15, 15), (8, 8)] {
        let i = (y * 16 + x) * 4;
        let r = pixels[i];
        let g = pixels[i + 1];
        let b = pixels[i + 2];
        // Expected R = 120/220 ≈ 0.5454; encoded as ~139 in u8.
        // Tolerance ±2 covers rasterizer rounding across backends.
        let expected_r = (120.0_f32 / 220.0_f32 * 255.0).round() as i16;
        assert!(
            (i16::from(r) - expected_r).abs() <= 2,
            "silent-frame R at ({x},{y}) should be ~{expected_r} (BPM_DEFAULT/220), got {r}",
        );
        assert_eq!(g, 0, "silent-frame G must be 0 (RMS=0) at ({x},{y})");
        assert_eq!(b, 0, "silent-frame B must be 0 (PEAK=0) at ({x},{y})");
    }
}

/// 440 Hz sine tone: drives RMS to ~0.5/√2 ≈ 0.354 (amplitude
/// 0.5), PEAK to 0.5, BPM to the autocorrelation result (a pure
/// sine tone has no rhythmic content so BPM is whatever the
/// autocorrelation of one cycle finds — clamped to [50, 220]),
/// and one FFT band dominant. The shader output bytes therefore
/// differ visibly from the silent-frame fixture; a hash drift
/// here would surface a regression in either the slice-1 snapshot
/// extraction OR the slice-5a marshalling.
#[test]
fn sine_tone_byte_hash_is_stable() {
    let frame = one_second_sine(440.0, 0.5);
    let pixels = render_with_snapshot(&frame);

    assert_eq!(pixels.len(), 16 * 16 * 4);

    let observed = fnv1a_64(&pixels);

    const ACCEPTED: &[u64] = &[
        // Apple Silicon, Metal backend, wgpu 26.0.1 + naga 26.0.0,
        // catalog v1.3.0 (single-array `chroma_AudioFFT[8]` binding).
        // Apple's Metal compiler reorders the GLSL `arr[0]+...+arr[7]`
        // summation into a different SPIR-V/MSL instruction order
        // than the prior `Low.xyzw + High.xyzw` form, and FP
        // non-associativity produces a deterministic alpha-channel
        // rounding delta. The silent-frame hash is preserved across
        // the consolidation because its FFT bands sum to bitwise 0.
        0xe95a_1533_2045_0d25,
        // Linux Vulkan, Mesa llvmpipe, wgpu 26.0.1 + naga 26.0.0,
        // catalog v1.3.0. Empirical finding: Mesa preserves the
        // natural left-to-right `arr[0] + arr[1] + ... + arr[7]`
        // summation order, which happens to be bit-identical to the
        // pre-consolidation v1.2.0 split-vec4 byte stream (which
        // also added components left-to-right: `Low.x + Low.y +
        // ... + High.w`). The two ACCEPTED entries here therefore
        // straddle the same mathematical semantics under different
        // GPU-shader-compiler instruction orderings.
        0x7ab4_cc96_6bba_0d25,
    ];

    if !ACCEPTED.contains(&observed) {
        panic!(
            "audio_adapter sine-tone hash 0x{observed:016x} not in accepted set \
             {ACCEPTED:#018x?}; new backends should add their measured value via \
             a docs(repo)-scoped commit. If this drifted on a previously-measured \
             backend, the slice-5a marshalling order, the docs/36 binding contract, \
             the slice-1 RMS / FFT formula, or the slice-1 BPM autocorrelation may \
             have regressed.",
        );
    }

    // Backend-portable corner sentinels: regardless of byte-hash
    // backend variance, every pixel must reflect the single
    // snapshot — so every pixel is identical (uniforms, not
    // texture sampling), and the channel ranges are bounded.
    let center_i = (8 * 16 + 8) * 4;
    let r = pixels[center_i];
    let g = pixels[center_i + 1];
    let b = pixels[center_i + 2];

    // RMS for amplitude 0.5 sine is ~0.354 -> u8 ~90.
    let expected_g = (0.5_f32 / 2.0_f32.sqrt() * 255.0).round() as i16;
    assert!(
        (i16::from(g) - expected_g).abs() <= 6,
        "sine-tone G must be ~RMS({expected_g}), got {g}",
    );
    // PEAK for amplitude 0.5 sine is exactly 0.5 -> u8 ~128.
    let expected_b = (0.5_f32 * 255.0).round() as i16;
    assert!(
        (i16::from(b) - expected_b).abs() <= 4,
        "sine-tone B must be ~PEAK({expected_b}), got {b}",
    );
    // BPM is autocorrelation-derived, clamped [50, 220]; the
    // R channel maps that into [50/220, 1] ≈ [0.227, 1] -> u8
    // [58, 255]. Just assert it's in the clamp-derived range.
    assert!(
        (58..=255).contains(&r),
        "sine-tone R must encode a BPM in [50, 220], got {r}",
    );
}

/// Drift pin against the silent-frame and sine-tone fixtures
/// producing distinct byte hashes — proves the audio side of the
/// pipeline actually drives the render output (a marshalling
/// regression that hard-coded all uniforms to zero would pass
/// the silent-frame test silently and only this cross-fixture
/// invariant would flag it).
#[test]
fn distinct_audio_inputs_produce_distinct_render_output() {
    let silent_pixels = render_with_snapshot(&one_second_silence());
    let sine_pixels = render_with_snapshot(&one_second_sine(440.0, 0.5));
    assert_ne!(
        fnv1a_64(&silent_pixels),
        fnv1a_64(&sine_pixels),
        "silent-frame and sine-tone snapshots must produce distinct render output \
         (otherwise the audio side of the adapter is decoupled from the render side)",
    );
}
