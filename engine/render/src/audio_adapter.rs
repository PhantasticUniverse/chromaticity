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
//! # `docs/36-reactive-symbol-catalog.md` v1.1.0 binding contract
//!
//! Slice-5a maps the four primary `AudioUniformSnapshot` fields
//! onto the v1.1.0 reactive-symbol vocabulary:
//!
//! | snapshot field        | uniform name | docs/36 anchor          | `UniformValue` variant            |
//! |-----------------------|--------------|-------------------------|-----------------------------------|
//! | [`bpm`]               | `syn_BPM`    | §2.7 (clamped `[50, 220]`) | [`UniformValue::Scalar`]       |
//! | [`rms`]               | `syn_RMS`    | §2.10 (`syn_InputLevel` family raw RMS) | [`UniformValue::Scalar`] |
//! | [`peak`]              | `syn_PEAK`   | §2.10 (`syn_Level*` family normaliser) | [`UniformValue::Scalar`]  |
//! | [`fft_bands[0..3]`]   | `syn_FFT_LO` | §2.6 `syn_Spectrum` (low-band aggregate) | [`UniformValue::Vec4`]  |
//! | [`fft_bands[4..7]`]   | `syn_FFT_HI` | §2.6 `syn_Spectrum` (high-band aggregate) | [`UniformValue::Vec4`] |
//!
//! Note on the FFT mapping: the slice-1 snapshot exposes 8
//! log-spaced FFT bands as a `[f32; 8]` array. The slice-2
//! [`UniformValue`] enum's widest scalar/vector variant is
//! [`UniformValue::Vec4`]; there is NO 8-element array variant.
//! Per slice-4's `multipass.rs` line 50-53 (the std140 packing doc
//! comment: "Slice 3 will revisit packing when audio uniforms (an
//! `audio[]` block) and `syn_RES` style vec2 batched uniforms enter
//! the surface"), the production scheduler will eventually grow an
//! `audio[]` block / array uniform variant. Slice-5a does NOT
//! widen [`UniformValue`] — that's a scheduler-shaped decision out
//! of scope for the cross-crate adapter slice. Instead, the 8-band
//! FFT array is split into two `vec4`s (low-bands [0..3], high-bands
//! [4..7]) under stable names (`syn_FFT_LO` / `syn_FFT_HI`).
//! Authors who need single-band access can index the `vec4`
//! components directly from GLSL (`syn_FFT_LO.x` is the lowest
//! band, `syn_FFT_HI.w` is the highest).
//!
//! When the future array-uniform variant lands, the adapter can
//! widen to a single `syn_FFT[8]` binding without breaking the
//! four primary `syn_BPM` / `syn_RMS` / `syn_PEAK` scalar names
//! (which match docs/36 verbatim). The split-vec4 spelling is
//! documented in this module so a scene author who hand-writes
//! the binding name can read what shape they get.
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
/// `docs/36` §2.10 (`syn_InputLevel` raw RMS).
pub const UNIFORM_NAME_RMS: &str = "syn_RMS";

/// Symbol name for the peak instantaneous-magnitude scalar uniform
/// per `docs/36` §2.10 (`syn_Level*` family normaliser).
pub const UNIFORM_NAME_PEAK: &str = "syn_PEAK";

/// Symbol name for the low-band FFT vec4 uniform (band indices
/// 0..=3, log-spaced per `chromaticity_audio::FFT_BAND_MIN_HZ`).
pub const UNIFORM_NAME_FFT_LOW: &str = "syn_FFT_LO";

/// Symbol name for the high-band FFT vec4 uniform (band indices
/// 4..=7).
pub const UNIFORM_NAME_FFT_HIGH: &str = "syn_FFT_HI";

/// Marshal an [`AudioUniformSnapshot`] into a stable-ordered
/// [`Vec<UniformInput>`] keyed by the `docs/36` v1.1.0 reactive-
/// symbol vocabulary.
///
/// Output order (same as the binding-slot order in the per-pass
/// uniform buffer):
///
/// 1. [`UNIFORM_NAME_BPM`] = [`UniformValue::Scalar`] (snapshot.bpm)
/// 2. [`UNIFORM_NAME_RMS`] = [`UniformValue::Scalar`] (snapshot.rms)
/// 3. [`UNIFORM_NAME_PEAK`] = [`UniformValue::Scalar`] (snapshot.peak)
/// 4. [`UNIFORM_NAME_FFT_LOW`] = [`UniformValue::Vec4`] (bands [0..=3])
/// 5. [`UNIFORM_NAME_FFT_HIGH`] = [`UniformValue::Vec4`] (bands [4..=7])
///
/// See module-level prose for the FFT-split rationale (no
/// 8-element variant on slice-2 [`UniformValue`]).
#[must_use]
pub fn uniform_input_from_audio_snapshot(snapshot: &AudioUniformSnapshot) -> Vec<UniformInput> {
    let bands = snapshot.fft_bands;
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
            name: UNIFORM_NAME_FFT_LOW.to_string(),
            value: UniformValue::Vec4([bands[0], bands[1], bands[2], bands[3]]),
        },
        UniformInput {
            name: UNIFORM_NAME_FFT_HIGH.to_string(),
            value: UniformValue::Vec4([bands[4], bands[5], bands[6], bands[7]]),
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
    //! - `docs/36-reactive-symbol-catalog.md` §2.6, §2.7, §2.10.

    use chromaticity_audio::AudioUniformSnapshot;
    use chromaticity_audio::uniforms::FFT_BAND_COUNT;

    use super::{
        UNIFORM_NAME_BPM, UNIFORM_NAME_FFT_HIGH, UNIFORM_NAME_FFT_LOW, UNIFORM_NAME_PEAK,
        UNIFORM_NAME_RMS, uniform_input_from_audio_snapshot,
    };
    use crate::multipass::UniformValue;

    #[test]
    fn symbol_names_match_docs_36_vocabulary() {
        // Drift pin: the four primary `syn_*` names match the
        // `docs/36` v1.1.0 catalog spelling verbatim. A catalog edit
        // that renames any of these surfaces here at the next test
        // run.
        assert_eq!(UNIFORM_NAME_BPM, "syn_BPM");
        assert_eq!(UNIFORM_NAME_RMS, "syn_RMS");
        assert_eq!(UNIFORM_NAME_PEAK, "syn_PEAK");
        // The split-vec4 FFT spelling is module-level documented;
        // the constants pin the chosen names so a downstream
        // refactor doesn't silently shift them.
        assert_eq!(UNIFORM_NAME_FFT_LOW, "syn_FFT_LO");
        assert_eq!(UNIFORM_NAME_FFT_HIGH, "syn_FFT_HI");
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
    fn adapter_returns_five_uniforms_in_documented_order() {
        // Drift pin: the binding-slot order is the contract per
        // module prose — std140 buffer slots are positional, so
        // any reordering would silently shuffle which scalar lives
        // at which slot.
        let snapshot = AudioUniformSnapshot::default();
        let uniforms = uniform_input_from_audio_snapshot(&snapshot);
        assert_eq!(uniforms.len(), 5, "adapter MUST return 5 uniforms");
        assert_eq!(uniforms[0].name, UNIFORM_NAME_BPM);
        assert_eq!(uniforms[1].name, UNIFORM_NAME_RMS);
        assert_eq!(uniforms[2].name, UNIFORM_NAME_PEAK);
        assert_eq!(uniforms[3].name, UNIFORM_NAME_FFT_LOW);
        assert_eq!(uniforms[4].name, UNIFORM_NAME_FFT_HIGH);
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
        match uniforms[0].value {
            UniformValue::Scalar(v) => assert!(
                (v - 128.0_f32).abs() < 1e-6,
                "BPM scalar must equal 128.0, got {v}",
            ),
            other => panic!("BPM must be Scalar, got {other:?}"),
        }
        match uniforms[1].value {
            UniformValue::Scalar(v) => assert!(
                (v - 0.6_f32).abs() < 1e-6,
                "RMS scalar must equal 0.6, got {v}"
            ),
            other => panic!("RMS must be Scalar, got {other:?}"),
        }
        match uniforms[2].value {
            UniformValue::Scalar(v) => assert!(
                (v - 0.95_f32).abs() < 1e-6,
                "PEAK scalar must equal 0.95, got {v}",
            ),
            other => panic!("PEAK must be Scalar, got {other:?}"),
        }
    }

    #[test]
    fn adapter_splits_fft_bands_into_two_vec4s_low_then_high() {
        // Drift pin: the band index split is `[0..=3]` -> low,
        // `[4..=7]` -> high (ascending in frequency, lowest band
        // at `.x`, highest at `.w`). A reversed split or a swap
        // of low/high uniform names would land the wrong band at
        // the wrong GLSL component.
        let snapshot = AudioUniformSnapshot {
            bpm: 0.0,
            rms: 0.0,
            peak: 0.0,
            // Distinct values per band so the split + ordering is
            // observable.
            fft_bands: [0.10, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80],
        };
        let uniforms = uniform_input_from_audio_snapshot(&snapshot);
        match uniforms[3].value {
            UniformValue::Vec4(v) => {
                assert!((v[0] - 0.10).abs() < 1e-6, "FFT_LO[0] must be band 0");
                assert!((v[1] - 0.20).abs() < 1e-6, "FFT_LO[1] must be band 1");
                assert!((v[2] - 0.30).abs() < 1e-6, "FFT_LO[2] must be band 2");
                assert!((v[3] - 0.40).abs() < 1e-6, "FFT_LO[3] must be band 3");
            }
            other => panic!("FFT_LO must be Vec4, got {other:?}"),
        }
        match uniforms[4].value {
            UniformValue::Vec4(v) => {
                assert!((v[0] - 0.50).abs() < 1e-6, "FFT_HI[0] must be band 4");
                assert!((v[1] - 0.60).abs() < 1e-6, "FFT_HI[1] must be band 5");
                assert!((v[2] - 0.70).abs() < 1e-6, "FFT_HI[2] must be band 6");
                assert!((v[3] - 0.80).abs() < 1e-6, "FFT_HI[3] must be band 7");
            }
            other => panic!("FFT_HI must be Vec4, got {other:?}"),
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
            // `UniformValue` is `Copy + Debug`; compare the debug
            // strings as a structural equality proxy (the enum
            // does not derive PartialEq because its variants carry
            // float arrays).
            assert_eq!(format!("{:?}", x.value), format!("{:?}", y.value));
        }
    }

    #[test]
    fn adapter_default_snapshot_yields_zeros_in_every_slot() {
        // Drift pin: the default-constructed snapshot (all zeros)
        // marshals to all-zero scalars and all-zero vec4s. This
        // is the silence baseline; downstream T2 GPU corpora rely
        // on it.
        let uniforms = uniform_input_from_audio_snapshot(&AudioUniformSnapshot::default());
        for u in &uniforms {
            match u.value {
                UniformValue::Scalar(v) => assert_eq!(v, 0.0, "{} scalar must be 0", u.name),
                UniformValue::Vec4(v) => {
                    for c in v {
                        assert_eq!(c, 0.0, "{} vec4 component must be 0", u.name);
                    }
                }
                other => panic!("default snapshot variant unexpected: {other:?}"),
            }
        }
    }
}
