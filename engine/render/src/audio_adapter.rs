// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-2 EV-RENDER-CONFORMANCE / EV-AUDIO-TRACE slice-5a audio →
//! render uniform-binding adapter.
//!
//! # Cross-crate boundary
//!
//! `engine/audio` publishes a plain-data
//! [`chromaticity_audio::AudioUniformSnapshot`] from
//! `extract_uniform_snapshot` (slice-1 producer side) and from
//! [`chromaticity_audio::BpmEstimator::observe`] (slice-2 history-
//! window BPM). That crate intentionally does NOT depend on
//! `engine/render` — see `engine/audio/src/lib.rs` lines 16-31
//! ("`engine/audio` does NOT depend on `engine/render`; consumers
//! marshal the plain-data shape into `UniformInput` at the
//! boundary"). This module is the consumer-side marshalling step
//! that satisfies that boundary.
//!
//! # Binding namespace
//!
//! Slice-5a binds the four primary `AudioUniformSnapshot` fields
//! across two namespaces per `docs/36-reactive-symbol-catalog.md`
//! Sec.Namespace governance: `syn_BPM` is a canonical compat-syn-v1
//! retention (docs/36 line 117) and stays under that name for
//! reference-scene parity continuity; the raw-RMS / peak / 8-band
//! FFT outputs are net-new chromaticity-native primitives under the
//! `chroma_*` namespace, since docs/36 has no native equivalents at
//! this abstraction level (docs/36's `syn_Level` is normalised +
//! smoothed per `docs/06` §7.4; `syn_Spectrum` is a sampler1D with
//! packed Raw/Juiced/Smooth/Waveform channels — slice-5a's outputs
//! are raw scalar / array analysis primitives, a complementary
//! surface).
//!
//! | snapshot field   | uniform name        | namespace                                  | `UniformValue` variant            |
//! |------------------|---------------------|--------------------------------------------|-----------------------------------|
//! | [`bpm`]          | `syn_BPM`           | compat-syn-v1 (docs/36 line 117 retention) | [`UniformValue::Scalar`]          |
//! | [`rms`]          | `chroma_AudioRMS`   | chromaticity-native (slice-5a-introduced)  | [`UniformValue::Scalar`]          |
//! | [`peak`]         | `chroma_AudioPeak`  | chromaticity-native (slice-5a-introduced)  | [`UniformValue::Scalar`]          |
//! | [`fft_bands`]    | `chroma_AudioFFT`   | chromaticity-native (slice-5a-introduced)  | [`UniformValue::FloatArray`] (8 elements) |
//!
//! Note on the FFT mapping: the slice-1 snapshot exposes 8
//! log-spaced FFT bands as a `[f32; 8]` array. The slice-5a initial
//! release (catalog v1.2.0) used a transitional split-vec4 spelling
//! (`chroma_AudioFFTLow` for bands `[0..=3]` + `chroma_AudioFFTHigh`
//! for bands `[4..=7]`) because [`UniformValue`] had no array variant.
//! The slice-5 follow-up landed [`UniformValue::FloatArray`] in
//! `multipass.rs` and consolidated the FFT into a single
//! `chroma_AudioFFT[8]` array binding (catalog v1.3.0). The two
//! `syn_BPM` / `chroma_AudioRMS` / `chroma_AudioPeak` scalar names
//! are unchanged across the consolidation; only the FFT shape moved.
//! GLSL authors index the array directly (`chroma_AudioFFT[0]` is
//! the lowest band, `chroma_AudioFFT[7]` is the highest). The
//! `FloatArray` packing is std140-friendly per the `multipass.rs`
//! module-level doc — each element occupies one 16-byte slot
//! (vec4-aligned), so the per-pass uniform buffer grows by 8 slots
//! for each `FloatArray(8)` binding.
//!
//! # Stability and ordering
//!
//! [`uniform_input_from_audio_snapshot`] returns the uniforms in a
//! stable, documented order (BPM, RMS, PEAK, FFT_LO, FFT_HI). The
//! [`MultipassSpec`] scheduler binds uniforms in declaration order
//! (per `multipass.rs::PassDef::uniforms` doc comment), so this
//! ordering is the binding contract for slot indices into the
//! per-pass uniform buffer. The names themselves are diagnostic
//! per `multipass.rs::UniformInput::name`; the slot index is what
//! the std140 buffer reads.
//!
//! # Pure function
//!
//! Same input snapshot -> same output `Vec<UniformInput>`. No
//! state, no allocation reuse, no global side effects. The adapter
//! is intentionally a free function rather than an inherent impl
//! on [`UniformInput`] — keeping the audio-side coupling out of
//! `multipass.rs` lets the feature gate cover one module instead of
//! requiring `#[cfg]` on an impl block.
//!
//! # Evidence anchors
//!
//! - `docs/26-evidence-artifact-registry.md` row 28 — `EV-RENDER-CONFORMANCE`.
//! - `docs/26-evidence-artifact-registry.md` row 30 — `EV-AUDIO-TRACE`.
//! - `docs/36-reactive-symbol-catalog.md` v1.1.0 — symbol vocabulary.
//! - `docs/43-development-standards.md` §15.5 — T-tier definitions.
//!
//! [`bpm`]: chromaticity_audio::AudioUniformSnapshot::bpm
//! [`rms`]: chromaticity_audio::AudioUniformSnapshot::rms
//! [`peak`]: chromaticity_audio::AudioUniformSnapshot::peak
//! [`fft_bands[0..3]`]: chromaticity_audio::AudioUniformSnapshot::fft_bands
//! [`fft_bands[4..7]`]: chromaticity_audio::AudioUniformSnapshot::fft_bands

use chromaticity_audio::AudioUniformSnapshot;

use crate::multipass::{UniformInput, UniformValue};

/// Symbol name for the BPM scalar uniform per `docs/36` §2.7.
pub const UNIFORM_NAME_BPM: &str = "syn_BPM";

/// Symbol name for the RMS instantaneous-energy scalar uniform per
/// `docs/36` §2.11.
pub const UNIFORM_NAME_RMS: &str = "chroma_AudioRMS";

/// Symbol name for the peak instantaneous-magnitude scalar uniform
/// per `docs/36` §2.11.
pub const UNIFORM_NAME_PEAK: &str = "chroma_AudioPeak";

/// Symbol name for the 8-band log-spaced FFT array uniform per
/// `docs/36` §2.11. Authors declare `uniform float chroma_AudioFFT[8];`
/// in their GLSL block; element K (0..=7) is the K-th log-spaced
/// band per `chromaticity_audio::FFT_BAND_MIN_HZ`. The slice-5
/// follow-up to slice-5a consolidated the transitional split-vec4
/// spelling (`chroma_AudioFFTLow` + `chroma_AudioFFTHigh`) into this
/// single array binding via [`UniformValue::FloatArray`] (catalog
/// v1.2.0 → v1.3.0 transition).
pub const UNIFORM_NAME_FFT: &str = "chroma_AudioFFT";

/// Marshal an [`AudioUniformSnapshot`] into a stable-ordered
/// [`Vec<UniformInput>`] keyed by the `docs/36` v1.3.0 reactive-
/// symbol vocabulary.
///
/// Output order (same as the binding-slot order in the per-pass
/// uniform buffer):
///
/// 1. [`UNIFORM_NAME_BPM`] = [`UniformValue::Scalar`] (snapshot.bpm)
/// 2. [`UNIFORM_NAME_RMS`] = [`UniformValue::Scalar`] (snapshot.rms)
/// 3. [`UNIFORM_NAME_PEAK`] = [`UniformValue::Scalar`] (snapshot.peak)
/// 4. [`UNIFORM_NAME_FFT`] = [`UniformValue::FloatArray`] (8 elements,
///    bands `[0..=7]`; expands into 8 consecutive 16-byte slots in
///    the uniform buffer per the std140 layout for `float arr[N]`).
#[must_use]
pub fn uniform_input_from_audio_snapshot(snapshot: &AudioUniformSnapshot) -> Vec<UniformInput> {
    vec![
        UniformInput {
            name: UNIFORM_NAME_BPM.to_string(),
            value: UniformValue::Scalar(snapshot.bpm),
        },
        UniformInput {
            name: UNIFORM_NAME_RMS.to_string(),
            value: UniformValue::Scalar(snapshot.rms),
        },
        UniformInput {
            name: UNIFORM_NAME_PEAK.to_string(),
            value: UniformValue::Scalar(snapshot.peak),
        },
        UniformInput {
            name: UNIFORM_NAME_FFT.to_string(),
            value: UniformValue::FloatArray(snapshot.fft_bands.to_vec()),
        },
    ]
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod audio_adapter_drift_pins {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) + EV-AUDIO-TRACE (T1) —
    //! cross-crate audio → render uniform-binding adapter drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins on the
    //! `docs/36` v1.1.0 binding contract — symbol names, slot order,
    //! and `UniformValue` variant mapping. Catalog drift on any of
    //! these surfaces fires the moment `cargo test` runs, before the
    //! T2 GPU-roundtrip corpus in `engine/render/tests/audio_adapter.rs`
    //! would catch the same regression as a noisier multi-test
    //! failure. Dual-tagged because slice-5a is the cross-crate
    //! widening that touches both rows simultaneously; the same
    //! pattern is established by
    //! `engine/script/src/runtime.rs` (`EV-SANDBOX-SAFETY (T1) +
    //! EV-SCRIPT-CONTRACT (T1)`).
    //!
    //! Anchors:
    //! - `docs/26-evidence-artifact-registry.md` row 28 — `EV-RENDER-CONFORMANCE`.
    //! - `docs/26-evidence-artifact-registry.md` row 30 — `EV-AUDIO-TRACE`.
    //! - `docs/36-reactive-symbol-catalog.md` §2.7, §2.11.

    use chromaticity_audio::AudioUniformSnapshot;
    use chromaticity_audio::uniforms::FFT_BAND_COUNT;

    use super::{
        UNIFORM_NAME_BPM, UNIFORM_NAME_FFT, UNIFORM_NAME_PEAK, UNIFORM_NAME_RMS,
        uniform_input_from_audio_snapshot,
    };
    use crate::multipass::UniformValue;

    #[test]
    fn symbol_names_match_docs_36_vocabulary() {
        // Drift pin: the four binding names match the `docs/36`
        // v1.3.0 catalog spelling verbatim. A catalog edit that
        // renames any of these surfaces here at the next test run.
        assert_eq!(UNIFORM_NAME_BPM, "syn_BPM");
        assert_eq!(UNIFORM_NAME_RMS, "chroma_AudioRMS");
        assert_eq!(UNIFORM_NAME_PEAK, "chroma_AudioPeak");
        // FFT array binding (catalog v1.3.0 consolidation of the
        // v1.2.0 split-vec4 `chroma_AudioFFTLow` + `chroma_AudioFFTHigh`
        // shape into a single `chroma_AudioFFT[8]` array uniform).
        assert_eq!(UNIFORM_NAME_FFT, "chroma_AudioFFT");
    }

    #[test]
    fn snapshot_extension_invariant_eight_band_count() {
        // Drift pin: the adapter relies on the slice-1 snapshot
        // exposing exactly 8 FFT bands. If `engine/audio` ever
        // widens `FFT_BAND_COUNT`, the slice-5a adapter must be
        // re-shaped (and the docs/36 binding contract revisited).
        // This pin trips the moment that constant moves and
        // surfaces the slice-5a adapter as a follow-up call site.
        assert_eq!(FFT_BAND_COUNT, 8);
    }

    #[test]
    fn adapter_returns_four_uniforms_in_documented_order() {
        // Drift pin: the binding-slot order is the contract per
        // module prose — std140 buffer slots are positional, so
        // any reordering would silently shuffle which scalar lives
        // at which slot. The catalog v1.3.0 consolidation reduced
        // the binding count from 5 (BPM + RMS + PEAK + FFTLow +
        // FFTHigh) to 4 (BPM + RMS + PEAK + FFT[8]); the FFT array
        // expands into 8 consecutive slots in the uniform buffer.
        let snapshot = AudioUniformSnapshot::default();
        let uniforms = uniform_input_from_audio_snapshot(&snapshot);
        assert_eq!(uniforms.len(), 4, "adapter MUST return 4 uniforms");
        assert_eq!(uniforms[0].name, UNIFORM_NAME_BPM);
        assert_eq!(uniforms[1].name, UNIFORM_NAME_RMS);
        assert_eq!(uniforms[2].name, UNIFORM_NAME_PEAK);
        assert_eq!(uniforms[3].name, UNIFORM_NAME_FFT);
    }

    #[test]
    fn adapter_maps_scalar_fields_to_scalar_variant() {
        // Drift pin: BPM / RMS / PEAK MUST land in
        // `UniformValue::Scalar`. Any future widening that picks a
        // different variant (e.g. `Float` if the enum is renamed)
        // would silently re-pack the std140 buffer slot.
        let snapshot = AudioUniformSnapshot {
            bpm: 128.0,
            rms: 0.6,
            peak: 0.95,
            fft_bands: [0.0; 8],
        };
        let uniforms = uniform_input_from_audio_snapshot(&snapshot);
        match &uniforms[0].value {
            UniformValue::Scalar(v) => assert!(
                (v - 128.0_f32).abs() < 1e-6,
                "BPM scalar must equal 128.0, got {v}",
            ),
            other => panic!("BPM must be Scalar, got {other:?}"),
        }
        match &uniforms[1].value {
            UniformValue::Scalar(v) => assert!(
                (v - 0.6_f32).abs() < 1e-6,
                "RMS scalar must equal 0.6, got {v}",
            ),
            other => panic!("RMS must be Scalar, got {other:?}"),
        }
        match &uniforms[2].value {
            UniformValue::Scalar(v) => assert!(
                (v - 0.95_f32).abs() < 1e-6,
                "PEAK scalar must equal 0.95, got {v}",
            ),
            other => panic!("PEAK must be Scalar, got {other:?}"),
        }
    }

    #[test]
    fn adapter_packs_fft_bands_into_float_array_in_band_order() {
        // Drift pin: the 8-band FFT lands in `UniformValue::FloatArray`
        // ascending from band 0 (lowest log-spaced bin) at index 0
        // to band 7 (highest) at index 7. A reversed iteration or a
        // wrong slice would land the wrong band at the wrong array
        // element index. The catalog v1.3.0 consolidation replaces
        // the v1.2.0 split-vec4 `(FFTLow.xyzw, FFTHigh.xyzw)` shape
        // with a single 8-element float array.
        let snapshot = AudioUniformSnapshot {
            bpm: 0.0,
            rms: 0.0,
            peak: 0.0,
            // Distinct values per band so the ordering is observable.
            fft_bands: [0.10, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80],
        };
        let uniforms = uniform_input_from_audio_snapshot(&snapshot);
        match &uniforms[3].value {
            UniformValue::FloatArray(arr) => {
                assert_eq!(arr.len(), 8, "FFT array must be 8 elements");
                let expected = [0.10_f32, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80];
                for (i, exp) in expected.iter().enumerate() {
                    assert!(
                        (arr[i] - exp).abs() < 1e-6,
                        "FFT band {i} must be {exp}, got {}",
                        arr[i],
                    );
                }
            }
            other => panic!("FFT must be FloatArray, got {other:?}"),
        }
    }

    #[test]
    fn adapter_is_pure_same_input_same_output() {
        // Drift pin: the adapter MUST be deterministic. Two calls
        // with the same snapshot produce two `Vec<UniformInput>`
        // that compare equal name-by-name and value-by-value.
        let snapshot = AudioUniformSnapshot {
            bpm: 90.0,
            rms: 0.3,
            peak: 0.5,
            fft_bands: [0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40],
        };
        let a = uniform_input_from_audio_snapshot(&snapshot);
        let b = uniform_input_from_audio_snapshot(&snapshot);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.name, y.name);
            // `UniformValue` is `Clone + Debug` (the catalog v1.3.0
            // FloatArray addition dropped the prior `Copy`); compare
            // the debug strings as a structural equality proxy (the
            // enum does not derive PartialEq because its variants
            // carry float arrays / vecs).
            assert_eq!(format!("{:?}", x.value), format!("{:?}", y.value));
        }
    }

    #[test]
    fn adapter_default_snapshot_yields_zeros_in_every_slot() {
        // Drift pin: the default-constructed snapshot (all zeros)
        // marshals to all-zero scalars and all-zero array elements.
        // This is the silence baseline; downstream T2 GPU corpora
        // rely on it.
        let uniforms = uniform_input_from_audio_snapshot(&AudioUniformSnapshot::default());
        for u in &uniforms {
            match &u.value {
                UniformValue::Scalar(v) => assert_eq!(*v, 0.0, "{} scalar must be 0", u.name),
                UniformValue::FloatArray(arr) => {
                    for (i, v) in arr.iter().enumerate() {
                        assert_eq!(*v, 0.0, "{} float-array element {i} must be 0", u.name);
                    }
                }
                other => panic!("default snapshot variant unexpected: {other:?}"),
            }
        }
    }
}
