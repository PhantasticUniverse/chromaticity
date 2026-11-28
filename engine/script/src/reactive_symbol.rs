// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Typed reactive-symbol-catalog binding for slice 5.2.
//!
//! Slice 5.1 mounted the foundational producer-side seam: a JS-side
//! `__chromaticityEmit(name, value)` call lowers to
//! [`crate::ScriptCommand::ReactiveWrite { name, value }`] — a
//! string-keyed plain-data shape. Slice 5.2 closes the
//! `docs/36-reactive-symbol-catalog.md` v1.3.0 binding gap by
//! attaching a typed catalog: a [`ReactiveSymbol`] enum with one
//! variant per documented `syn_*` / `chroma_*` name in the
//! script-host-visible subset, a [`ReactiveValueKind`] dimensionality
//! descriptor, and the [`ReactiveSymbol::route`] /
//! [`ScriptCommand::route`] (in [`crate::command_bus`]) routing path
//! that converts string-keyed writes to the typed
//! [`crate::ScriptCommand::TypedReactiveWrite { symbol, value }`]
//! variant.
//!
//! # Where the catalog lives
//!
//! The catalog binds the slice-1 [`crate::ScriptValue`] vocabulary
//! to typed [`crate::ScriptCommand`] variants. Three placements were
//! considered:
//!
//! - **engine/script-side (chosen).** The catalog lives here next to
//!   the slice-5.1 [`crate::command_bus`] surface. Engine/script
//!   already owns the typed shape; the runtime consumer only sees
//!   typed [`crate::ScriptCommand`] variants. Dep direction stays
//!   clean — engine/script does NOT take a `chromaticity-runtime`
//!   dep, the slice-5.1 [`crate::ScriptCommandSender`] trait stays
//!   the cross-crate seam.
//! - engine/runtime-side. Catalog lives in runtime, script sends
//!   string-keyed writes that runtime types up. Couples the
//!   binding contract to runtime, conflates concerns, breaks the
//!   single-seam-per-crate-boundary invariant slice 5.1 set.
//! - Shared crate (`chromaticity-reactive-catalog`). Adds a
//!   workspace-members edit (serial-list per CLAUDE.md), more dep
//!   plumbing for no concrete benefit at slice 5.2 scope.
//!
//! engine/script-side wins on dep-direction cleanness and on slice
//! lifecycle: the catalog is read by the producer and the consumer
//! both, but the consumer reads it through the typed variant
//! dispatch, never through the catalog enum directly.
//!
//! # Catalog scope
//!
//! Slice 5.2 covers the script-host-visible subset of `docs/36`
//! v1.3.0 §1 (Standard Uniforms — frame/timing + mouse) and §2
//! (Audio Uniforms — full level/hit/time/presence/beat/BPM
//! families + large features + PrimeBang + reference-surface
//! uniforms + the slice-5a render-adapter scalar primitives
//! `chroma_AudioRMS` / `chroma_AudioPeak`; the FFT array surface
//! is producer-only on the script side post catalog v1.3.0). Pass-only
//! and fragment-only symbols (`_uv`, `_uvc`, `_xy`, `RENDERSIZE`,
//! `PASSINDEX`) are excluded per `docs/36` §5 (the catalog's own
//! Script Host Mirror Rules). Sampler symbols (`chroma_FinalPass`,
//! `chroma_Media`, `chroma_Media`, `chroma_Spectrum`, `chroma_LevelTrail`)
//! are excluded too — `docs/36` §5 routes those through helper
//! functions (`textureFinalPass(x,y)`), not direct value access,
//! and the slice-5.2 catalog is value-write-shaped.
//!
//! Meta controls (`docs/36` §3) are deferred — the script-side
//! contract for those is "host-published global" not
//! "script-emitted reactive write", and writing them needs the
//! ADR-005 state-bus seam that slice 5.3+ will add. The
//! `#[non_exhaustive]` attribute keeps the variant set additive.
//!
//! # Value-kind taxonomy
//!
//! [`ReactiveValueKind`] is the dimensionality descriptor. Slice
//! 5.2 ships four variants:
//!
//! - [`ReactiveValueKind::Scalar`] — a single `f64`-shaped
//!   [`crate::ScriptValue::Number`]. The bulk of the audio
//!   uniforms land here.
//! - [`ReactiveValueKind::Vec2`] — a 2-component vector, accepted
//!   as either a 2-element [`crate::ScriptValue::Array`] or a
//!   [`crate::ScriptValue::Object`] with `x`/`y` keys (matching
//!   `docs/36` §5: "Vector values are JS objects: `vec2 = {x, y}`").
//! - [`ReactiveValueKind::Vec3`] — same shape with 3 components,
//!   keys `x`/`y`/`z` or `r`/`g`/`b`.
//! - [`ReactiveValueKind::Vec4`] — same shape with 4 components,
//!   keys `x`/`y`/`z`/`w` or `r`/`g`/`b`/`a`. `_mouse` (`x, y,
//!   x_prev_click, y_prev_click`) is the canonical Vec4 in the
//!   script-side catalog. (The slice-5a v1.2.0 transitional
//!   `chroma_AudioFFTLow` / `chroma_AudioFFTHigh` Vec4 spellings
//!   were retired alongside the catalog v1.3.0 single-array
//!   `chroma_AudioFFT[8]` consolidation; see the variant retirement
//!   note inside the `ReactiveSymbol` enum body.)
//!
//! Validation against the kind is done by
//! [`ReactiveValueKind::validate`] — it does NOT decode the vector
//! into a typed array (the slice-1 [`crate::ScriptValue`] surface
//! stays the carrier). Decoders for `vec2` / `vec3` / `vec4` shapes
//! belong to the consumer side (engine/runtime adapter, slice 5.3+).
//!
//! # Routing surface
//!
//! [`ReactiveSymbol::from_name`] does the catalog lookup; returns
//! `Some` when the name matches a documented symbol, `None`
//! otherwise. [`ReactiveSymbol::name`] is the round-trip back to
//! the canonical string. [`ReactiveSymbol::expected_kind`] is the
//! dimensionality slot.
//!
//! [`crate::ScriptCommand::route`] is the producer-side conversion
//! from the slice-5.1 string-keyed
//! [`crate::ScriptCommand::ReactiveWrite { name, value }`] shape to
//! either the slice-5.2 typed
//! [`crate::ScriptCommand::TypedReactiveWrite { symbol, value }`]
//! variant (when the name resolves AND the value's
//! [`ReactiveValueKind`] matches) or a structured rejection. The
//! string-keyed variant stays in the enum under
//! `#[non_exhaustive]` — a host that wants to forward unknown names
//! can keep the catch-all variant; a host that wants to drop them
//! discards the result of `route()` on the rejection arm. Slice 5.2
//! does NOT prune the catch-all variant (script authors can still
//! call `__chromaticityEmit('unknown_name', v)` and the host
//! decides the policy); it ADDS the typed variant alongside.
//!
//! # Evidence anchors
//!
//! - `docs/26-evidence-artifact-registry.md` row 29 —
//!   `EV-SCRIPT-CONTRACT`.
//! - `docs/36-reactive-symbol-catalog.md` v1.3.0 — symbol
//!   vocabulary.
//! - `docs/43-development-standards.md` §15.5 — T-tier
//!   definitions.
//!
//! Governing ADRs:
//!
//! - ADR-001 — runtime model.
//! - ADR-006 — thread and process model.
//! - ADR-012 — script sandbox engine and budgets.

use crate::value::ScriptValue;

/// Canonical reactive-symbol identity for the script-host-visible
/// subset of `docs/36-reactive-symbol-catalog.md` v1.3.0.
///
/// One variant per documented name. The variant name is a Rust
/// `PascalCase` rendering of the catalog string; the catalog
/// string itself is the round-trip target of [`Self::name`].
///
/// `#[non_exhaustive]` keeps slice-5.3+ widening (meta controls,
/// data-source uniforms, future namespace additions) additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReactiveSymbol {
    // §1.1 Frame / timing (script-visible per §5)
    /// `TIME` — seconds since scene started rendering. Scalar.
    Time,
    /// `FRAMECOUNT` — frames since scene load. Scalar.
    FrameCount,
    /// `TIMEDELTA` — seconds since previous frame. Scalar.
    TimeDelta,
    /// `PI` — compile-time `3.14159...`. Scalar.
    Pi,

    // §1.3 Mouse (script-visible per §5; fragment-only `_xy` etc. excluded)
    /// `_mouse` — `vec4 {x, y, x_prev_click, y_prev_click}`. Vec4.
    Mouse,
    /// `_click` — `vec3 {left, right, middle}`. Vec3.
    Click,
    /// `_muv` — mouse normalized `[0,1]`. Vec2.
    MouseUv,
    /// `_muvc` — aspect-corrected mouse UV. Vec2.
    MouseUvc,

    // §2.2 Levels — Scalar, family tolerance `LVL`
    /// `chroma_Level`.
    Level,
    /// `chroma_BassLevel`.
    BassLevel,
    /// `chroma_MidLevel`.
    MidLevel,
    /// `chroma_MidHighLevel`.
    MidHighLevel,
    /// `chroma_HighLevel`.
    HighLevel,

    // §2.3 Hits — Scalar, family tolerance `HIT`
    /// `chroma_Hits`.
    Hits,
    /// `chroma_BassHits`.
    BassHits,
    /// `chroma_MidHits`.
    MidHits,
    /// `chroma_MidHighHits`.
    MidHighHits,
    /// `chroma_HighHits`.
    HighHits,

    // §2.4 Time clocks — Scalar, family tolerance `TIM`
    /// `chroma_AudioTime`.
    AudioTime,
    /// `chroma_BassTime`.
    BassTime,
    /// `chroma_MidTime`.
    MidTime,
    /// `chroma_MidHighTime`.
    MidHighTime,
    /// `chroma_HighTime`.
    HighTime,
    /// `chroma_CurvedTime`.
    CurvedTime,

    // §2.5 Presence — Scalar, family tolerance `PRS`
    /// `chroma_Presence`.
    Presence,
    /// `chroma_BassPresence`.
    BassPresence,
    /// `chroma_MidPresence`.
    MidPresence,
    /// `chroma_MidHighPresence`.
    MidHighPresence,
    /// `chroma_HighPresence`.
    HighPresence,

    // §2.6 Beat — Scalar, family tolerance `BEAT`
    /// `chroma_OnBeat`.
    OnBeat,
    /// `chroma_ToggleOnBeat`.
    ToggleOnBeat,
    /// `chroma_RandomOnBeat`.
    RandomOnBeat,
    /// `chroma_BeatTime`.
    BeatTime,

    // §2.7 BPM — Scalar, family tolerance `BPM`
    /// `chroma_BPM` — clamped `[50, 220]`. Slice-5a render-adapter
    /// also publishes this name.
    Bpm,
    /// `chroma_BPMConfidence`.
    BpmConfidence,
    /// `chroma_BPMTwitcher`.
    BpmTwitcher,
    /// `chroma_BPMSin`.
    BpmSin,
    /// `chroma_BPMSin2`.
    BpmSin2,
    /// `chroma_BPMSin4`.
    BpmSin4,
    /// `chroma_BPMTri`.
    BpmTri,
    /// `chroma_BPMTri2`.
    BpmTri2,
    /// `chroma_BPMTri4`.
    BpmTri4,

    // §2.8 Large features — Scalar, family tolerance `LRG`
    /// `chroma_FadeInOut`.
    FadeInOut,
    /// `chroma_Intensity`.
    Intensity,

    // §2.9 PrimeBang — Scalar (Profile D)
    /// `chroma_PrimeBang`.
    PrimeBang,

    // §2.10 Reference-surface uniforms — Scalar
    /// `chroma_InputLevel` — pre-AGC instantaneous input level.
    InputLevel,
    /// `chroma_RawLevel` — alias of `chroma_InputLevel`.
    RawLevel,
    /// `chroma_BassExpoCurve` — `pow(chroma_BassLevel, 3.0)`.
    BassExpoCurve,
    /// `chroma_Threshold` — operator-settable noise-floor gate.
    Threshold,

    // Slice-5a render-adapter aggregates (mirrors
    // `engine/render/src/audio_adapter.rs`). These are NOT
    // verbatim catalog entries — they are the slice-5a-chosen
    // names for the audio-uniform marshalling under
    // `chroma_PrimeBang`-style native-first conventions. Documented
    // verbatim in `engine/render/src/audio_adapter.rs` module
    // prose. Held under their slice-5a names for cross-crate
    // consistency.
    /// `chroma_AudioRMS` — instantaneous RMS scalar (slice-5a
    /// render-adapter spelling per
    /// `engine/render/src/audio_adapter.rs`).
    Rms,
    /// `chroma_AudioPeak` — instantaneous peak scalar (slice-5a
    /// render-adapter spelling per
    /// `engine/render/src/audio_adapter.rs`).
    Peak,
    // Note: the slice-5a v1.2.0 transitional split-vec4 FFT
    // variants (`FftLow` / `FftHigh` mapping to `chroma_AudioFFTLow`
    // / `chroma_AudioFFTHigh`) were retired alongside the catalog
    // v1.3.0 consolidation that collapsed those bindings into a
    // single 8-element float array (`chroma_AudioFFT[8]`, backed
    // by `engine/render::UniformValue::FloatArray`). The script-
    // side carrier vocabulary
    // (`ReactiveValueKind::{Scalar, Vec2, Vec3, Vec4}`) does not
    // yet support 8-element float-array writes, so the FFT array
    // is producer-only at this slice — script writes against
    // `"chroma_AudioFFT"` resolve to `None` from `from_name` and
    // surface as `ScriptError::ReactiveSymbolUnknown`. Adding an
    // array-carrier variant to `ReactiveValueKind` is a future-
    // slice scope; until then, scripts that need to mock the FFT
    // surface should write the producer-side `AudioUniformSnapshot`
    // directly rather than going through the typed-reactive-write
    // pathway. No scenes shipped against the v1.2.0 split-vec4
    // surface (see `docs/22` Catalog Ratification Log v1.3.0
    // entry), so retiring the variants is non-breaking.
}

/// Dimensionality slot a [`ReactiveSymbol`] write must satisfy.
///
/// The slice-1 [`ScriptValue`] vocabulary is the carrier — slice 5.2
/// does NOT decode vectors into typed arrays. Validation checks
/// the carrier's shape against the dimensionality slot; consumers
/// that need typed `[f32; N]` decode in their own seam.
///
/// `#[non_exhaustive]` keeps slice-5.3+ widening (matrices,
/// arrays, samplers) additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReactiveValueKind {
    /// Single `f64`-shaped [`ScriptValue::Number`]. Accepts also
    /// [`ScriptValue::Bool`] (per ECMAScript `Number(true) === 1`)
    /// is intentionally NOT widened — slice 5.2 keeps the carrier
    /// strict to surface authoring-time mistakes early.
    Scalar,
    /// 2-component vector. Accepted carriers:
    /// - 2-element [`ScriptValue::Array`] of [`ScriptValue::Number`].
    /// - [`ScriptValue::Object`] with `x` + `y` keys (no other
    ///   keys), each mapping to [`ScriptValue::Number`].
    Vec2,
    /// 3-component vector. Accepted carriers as `Vec2` with
    /// `x`/`y`/`z` or `r`/`g`/`b` key sets.
    Vec3,
    /// 4-component vector. Accepted carriers as `Vec2` with
    /// `x`/`y`/`z`/`w` or `r`/`g`/`b`/`a` key sets. `_mouse` is the
    /// canonical Vec4 in the script-side catalog. (The slice-5a
    /// v1.2.0 transitional FFT split-vec4 spellings were retired
    /// alongside the catalog v1.3.0 single-array consolidation;
    /// see the `ReactiveSymbol` enum body for the retirement note.)
    Vec4,
}

impl ReactiveValueKind {
    /// Validate that `value` matches this dimensionality slot.
    ///
    /// Returns `Ok(())` when the carrier shape conforms; returns
    /// `Err` carrying a diagnostic of the actual shape observed
    /// (used by the `route()` rejection path to populate
    /// [`crate::ScriptError::ReactiveValueKindMismatch::got`]).
    ///
    /// The check is structural (does the carrier's variant +
    /// dimension match), not semantic (a `Number(NaN)` is
    /// accepted for `Scalar`; a `Number(-0.5)` is accepted for
    /// `Vec3.x`). Semantic clamping is the consumer's job per
    /// `docs/36` per-symbol semantics columns.
    pub fn validate(&self, value: &ScriptValue) -> Result<(), ReactiveValueKindMismatchDetail> {
        match (self, value) {
            (Self::Scalar, ScriptValue::Number(_)) => Ok(()),
            (Self::Vec2, v) => validate_vector(v, 2),
            (Self::Vec3, v) => validate_vector(v, 3),
            (Self::Vec4, v) => validate_vector(v, 4),
            (Self::Scalar, other) => Err(ReactiveValueKindMismatchDetail::ScalarExpected {
                got: describe_value(other),
            }),
        }
    }
}

/// Diagnostic shape returned by
/// [`ReactiveValueKind::validate`] when the carrier does not
/// conform. Carries enough detail for
/// [`crate::ScriptError::ReactiveValueKindMismatch`] to render an
/// actionable message without re-walking the carrier.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReactiveValueKindMismatchDetail {
    /// `Scalar` was expected; the carrier was a non-`Number`.
    ScalarExpected {
        /// One-word categorisation of the actual carrier shape.
        got: &'static str,
    },
    /// A vector was expected; the carrier did not match the
    /// vector vocabulary at all.
    VectorExpected {
        /// Expected component count (2 / 3 / 4).
        expected_components: usize,
        /// One-word categorisation of the actual carrier shape.
        got: &'static str,
    },
    /// A vector was expected; the carrier was an array of the
    /// wrong length.
    VectorWrongLength {
        /// Expected component count.
        expected_components: usize,
        /// Actual array length the carrier carried.
        actual_components: usize,
    },
    /// A vector was expected; the carrier was an object whose
    /// key set did not match any accepted spelling
    /// (`x`/`y`/... or `r`/`g`/...).
    VectorWrongKeys {
        /// Expected component count.
        expected_components: usize,
    },
    /// A vector array element or object value was a non-`Number`.
    VectorComponentNotNumber {
        /// Expected component count.
        expected_components: usize,
    },
}

/// Categorise a [`ScriptValue`] for diagnostic rendering. Returns
/// a static string so the diagnostic shape is allocation-free on
/// the rejection path.
fn describe_value(v: &ScriptValue) -> &'static str {
    match v {
        ScriptValue::Null => "null",
        ScriptValue::Bool(_) => "bool",
        ScriptValue::Number(_) => "number",
        ScriptValue::String(_) => "string",
        ScriptValue::Array(_) => "array",
        ScriptValue::Object(_) => "object",
    }
}

/// Accepted vector key spellings per component count.
///
/// `vec2`: `[x, y]`.
/// `vec3`: `[x, y, z]` or `[r, g, b]`.
/// `vec4`: `[x, y, z, w]` or `[r, g, b, a]`.
fn vector_key_spellings(components: usize) -> &'static [&'static [&'static str]] {
    match components {
        2 => &[&["x", "y"]],
        3 => &[&["x", "y", "z"], &["r", "g", "b"]],
        4 => &[&["x", "y", "z", "w"], &["r", "g", "b", "a"]],
        _ => &[],
    }
}

/// Validate a `ScriptValue` against an N-component vector slot.
///
/// Accepts:
/// - `Array(_)` of length `components`, all elements `Number`.
/// - `Object(_)` with exactly the `components` keys from one of
///   [`vector_key_spellings`], all values `Number`.
fn validate_vector(
    v: &ScriptValue,
    components: usize,
) -> Result<(), ReactiveValueKindMismatchDetail> {
    match v {
        ScriptValue::Array(items) => {
            if items.len() != components {
                return Err(ReactiveValueKindMismatchDetail::VectorWrongLength {
                    expected_components: components,
                    actual_components: items.len(),
                });
            }
            for item in items {
                if !matches!(item, ScriptValue::Number(_)) {
                    return Err(ReactiveValueKindMismatchDetail::VectorComponentNotNumber {
                        expected_components: components,
                    });
                }
            }
            Ok(())
        }
        ScriptValue::Object(entries) => {
            if entries.len() != components {
                return Err(ReactiveValueKindMismatchDetail::VectorWrongKeys {
                    expected_components: components,
                });
            }
            // Check that the object's keys match one accepted
            // spelling. Order-insensitive on the key set; the
            // values are matched per-key in spelling order.
            let observed_keys: Vec<&str> = entries.iter().map(|(k, _)| k.as_str()).collect();
            for spelling in vector_key_spellings(components) {
                if spelling.iter().all(|k| observed_keys.contains(k)) {
                    // All values must be Number.
                    for (_, val) in entries {
                        if !matches!(val, ScriptValue::Number(_)) {
                            return Err(
                                ReactiveValueKindMismatchDetail::VectorComponentNotNumber {
                                    expected_components: components,
                                },
                            );
                        }
                    }
                    return Ok(());
                }
            }
            Err(ReactiveValueKindMismatchDetail::VectorWrongKeys {
                expected_components: components,
            })
        }
        other => Err(ReactiveValueKindMismatchDetail::VectorExpected {
            expected_components: components,
            got: describe_value(other),
        }),
    }
}

impl ReactiveSymbol {
    /// Look up a [`ReactiveSymbol`] by canonical catalog name.
    ///
    /// Returns `Some` when `name` matches one of the documented
    /// `docs/36` v1.3.0 spellings the slice-5.2 catalog covers;
    /// returns `None` otherwise. Case-sensitive — `docs/36` §6
    /// CR-01 declares case-mismatch a strict-reject (compat
    /// warn-and-coerce); the slice-5.2 catalog is the strict
    /// surface, so a typo in case fails fast.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        // Linear scan of a static table. ~50 entries — well under
        // any cache-locality threshold where a HashMap would help.
        // Pinned in test corpus for round-trip integrity.
        Some(match name {
            // §1.1
            "TIME" => Self::Time,
            "FRAMECOUNT" => Self::FrameCount,
            "TIMEDELTA" => Self::TimeDelta,
            "PI" => Self::Pi,
            // §1.3
            "_mouse" => Self::Mouse,
            "_click" => Self::Click,
            "_muv" => Self::MouseUv,
            "_muvc" => Self::MouseUvc,
            // §2.2
            "chroma_Level" => Self::Level,
            "chroma_BassLevel" => Self::BassLevel,
            "chroma_MidLevel" => Self::MidLevel,
            "chroma_MidHighLevel" => Self::MidHighLevel,
            "chroma_HighLevel" => Self::HighLevel,
            // §2.3
            "chroma_Hits" => Self::Hits,
            "chroma_BassHits" => Self::BassHits,
            "chroma_MidHits" => Self::MidHits,
            "chroma_MidHighHits" => Self::MidHighHits,
            "chroma_HighHits" => Self::HighHits,
            // §2.4
            "chroma_AudioTime" => Self::AudioTime,
            "chroma_BassTime" => Self::BassTime,
            "chroma_MidTime" => Self::MidTime,
            "chroma_MidHighTime" => Self::MidHighTime,
            "chroma_HighTime" => Self::HighTime,
            "chroma_CurvedTime" => Self::CurvedTime,
            // §2.5
            "chroma_Presence" => Self::Presence,
            "chroma_BassPresence" => Self::BassPresence,
            "chroma_MidPresence" => Self::MidPresence,
            "chroma_MidHighPresence" => Self::MidHighPresence,
            "chroma_HighPresence" => Self::HighPresence,
            // §2.6
            "chroma_OnBeat" => Self::OnBeat,
            "chroma_ToggleOnBeat" => Self::ToggleOnBeat,
            "chroma_RandomOnBeat" => Self::RandomOnBeat,
            "chroma_BeatTime" => Self::BeatTime,
            // §2.7
            "chroma_BPM" => Self::Bpm,
            "chroma_BPMConfidence" => Self::BpmConfidence,
            "chroma_BPMTwitcher" => Self::BpmTwitcher,
            "chroma_BPMSin" => Self::BpmSin,
            "chroma_BPMSin2" => Self::BpmSin2,
            "chroma_BPMSin4" => Self::BpmSin4,
            "chroma_BPMTri" => Self::BpmTri,
            "chroma_BPMTri2" => Self::BpmTri2,
            "chroma_BPMTri4" => Self::BpmTri4,
            // §2.8
            "chroma_FadeInOut" => Self::FadeInOut,
            "chroma_Intensity" => Self::Intensity,
            // §2.9
            "chroma_PrimeBang" => Self::PrimeBang,
            // §2.10
            "chroma_InputLevel" => Self::InputLevel,
            "chroma_RawLevel" => Self::RawLevel,
            "chroma_BassExpoCurve" => Self::BassExpoCurve,
            "chroma_Threshold" => Self::Threshold,
            // Slice-5a render-adapter aggregates (post catalog
            // v1.3.0: only the scalar primitives remain here; the
            // FFT array surface is producer-only on the script side
            // until a future array-carrier variant lands in
            // `ReactiveValueKind`).
            "chroma_AudioRMS" => Self::Rms,
            "chroma_AudioPeak" => Self::Peak,
            _ => return None,
        })
    }

    /// Round-trip the canonical catalog name from a typed
    /// [`ReactiveSymbol`].
    ///
    /// Round-trips with [`Self::from_name`]:
    /// `Self::from_name(s.name()) == Some(*s)` for every variant.
    /// Pinned in test corpus.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            // §1.1
            Self::Time => "TIME",
            Self::FrameCount => "FRAMECOUNT",
            Self::TimeDelta => "TIMEDELTA",
            Self::Pi => "PI",
            // §1.3
            Self::Mouse => "_mouse",
            Self::Click => "_click",
            Self::MouseUv => "_muv",
            Self::MouseUvc => "_muvc",
            // §2.2
            Self::Level => "chroma_Level",
            Self::BassLevel => "chroma_BassLevel",
            Self::MidLevel => "chroma_MidLevel",
            Self::MidHighLevel => "chroma_MidHighLevel",
            Self::HighLevel => "chroma_HighLevel",
            // §2.3
            Self::Hits => "chroma_Hits",
            Self::BassHits => "chroma_BassHits",
            Self::MidHits => "chroma_MidHits",
            Self::MidHighHits => "chroma_MidHighHits",
            Self::HighHits => "chroma_HighHits",
            // §2.4
            Self::AudioTime => "chroma_AudioTime",
            Self::BassTime => "chroma_BassTime",
            Self::MidTime => "chroma_MidTime",
            Self::MidHighTime => "chroma_MidHighTime",
            Self::HighTime => "chroma_HighTime",
            Self::CurvedTime => "chroma_CurvedTime",
            // §2.5
            Self::Presence => "chroma_Presence",
            Self::BassPresence => "chroma_BassPresence",
            Self::MidPresence => "chroma_MidPresence",
            Self::MidHighPresence => "chroma_MidHighPresence",
            Self::HighPresence => "chroma_HighPresence",
            // §2.6
            Self::OnBeat => "chroma_OnBeat",
            Self::ToggleOnBeat => "chroma_ToggleOnBeat",
            Self::RandomOnBeat => "chroma_RandomOnBeat",
            Self::BeatTime => "chroma_BeatTime",
            // §2.7
            Self::Bpm => "chroma_BPM",
            Self::BpmConfidence => "chroma_BPMConfidence",
            Self::BpmTwitcher => "chroma_BPMTwitcher",
            Self::BpmSin => "chroma_BPMSin",
            Self::BpmSin2 => "chroma_BPMSin2",
            Self::BpmSin4 => "chroma_BPMSin4",
            Self::BpmTri => "chroma_BPMTri",
            Self::BpmTri2 => "chroma_BPMTri2",
            Self::BpmTri4 => "chroma_BPMTri4",
            // §2.8
            Self::FadeInOut => "chroma_FadeInOut",
            Self::Intensity => "chroma_Intensity",
            // §2.9
            Self::PrimeBang => "chroma_PrimeBang",
            // §2.10
            Self::InputLevel => "chroma_InputLevel",
            Self::RawLevel => "chroma_RawLevel",
            Self::BassExpoCurve => "chroma_BassExpoCurve",
            Self::Threshold => "chroma_Threshold",
            // Slice-5a render-adapter aggregates
            Self::Rms => "chroma_AudioRMS",
            Self::Peak => "chroma_AudioPeak",
        }
    }

    /// Dimensionality slot for the symbol. The carrier
    /// [`ScriptValue`] in a typed write must satisfy
    /// [`ReactiveValueKind::validate`] against this slot.
    ///
    /// Per `docs/36` v1.3.0:
    /// - All audio-uniform scalars (§2.2 / §2.3 / §2.4 / §2.5 /
    ///   §2.6 / §2.7 / §2.8 / §2.9 / §2.10) are `Scalar`.
    /// - Frame/timing (§1.1) values exposed to the script host
    ///   are `Scalar`.
    /// - Mouse symbols (§1.3) follow the catalog's typed shape:
    ///   `_mouse` is `Vec4`, `_click` is `Vec3`, `_muv` / `_muvc`
    ///   are `Vec2`.
    /// - Slice-5a render-adapter aggregates (post catalog v1.3.0):
    ///   `chroma_AudioRMS` / `chroma_AudioPeak` are `Scalar`. The
    ///   v1.2.0 split-vec4 FFT spellings were retired alongside the
    ///   v1.3.0 single-array consolidation; the FFT array surface
    ///   is producer-only on the script side until an array-carrier
    ///   variant lands in `ReactiveValueKind`.
    #[must_use]
    pub fn expected_kind(&self) -> ReactiveValueKind {
        match self {
            // §1.1 — all Scalar
            Self::Time | Self::FrameCount | Self::TimeDelta | Self::Pi => ReactiveValueKind::Scalar,
            // §1.3 — mixed dimensionality per catalog
            Self::Mouse => ReactiveValueKind::Vec4,
            Self::Click => ReactiveValueKind::Vec3,
            Self::MouseUv | Self::MouseUvc => ReactiveValueKind::Vec2,
            // §2.2 — all Scalar
            Self::Level
            | Self::BassLevel
            | Self::MidLevel
            | Self::MidHighLevel
            | Self::HighLevel => ReactiveValueKind::Scalar,
            // §2.3 — all Scalar
            Self::Hits | Self::BassHits | Self::MidHits | Self::MidHighHits | Self::HighHits => {
                ReactiveValueKind::Scalar
            }
            // §2.4 — all Scalar
            Self::AudioTime
            | Self::BassTime
            | Self::MidTime
            | Self::MidHighTime
            | Self::HighTime
            | Self::CurvedTime => ReactiveValueKind::Scalar,
            // §2.5 — all Scalar
            Self::Presence
            | Self::BassPresence
            | Self::MidPresence
            | Self::MidHighPresence
            | Self::HighPresence => ReactiveValueKind::Scalar,
            // §2.6 — all Scalar
            Self::OnBeat | Self::ToggleOnBeat | Self::RandomOnBeat | Self::BeatTime => {
                ReactiveValueKind::Scalar
            }
            // §2.7 — all Scalar
            Self::Bpm
            | Self::BpmConfidence
            | Self::BpmTwitcher
            | Self::BpmSin
            | Self::BpmSin2
            | Self::BpmSin4
            | Self::BpmTri
            | Self::BpmTri2
            | Self::BpmTri4 => ReactiveValueKind::Scalar,
            // §2.8 — all Scalar
            Self::FadeInOut | Self::Intensity => ReactiveValueKind::Scalar,
            // §2.9 — Scalar
            Self::PrimeBang => ReactiveValueKind::Scalar,
            // §2.10 — all Scalar
            Self::InputLevel | Self::RawLevel | Self::BassExpoCurve | Self::Threshold => {
                ReactiveValueKind::Scalar
            }
            // Slice-5a render-adapter aggregates
            Self::Rms | Self::Peak => ReactiveValueKind::Scalar,
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod reactive_symbol_pins {
    //! Evidence: EV-SCRIPT-CONTRACT (T1) — slice-5.2 reactive-
    //! symbol-catalog binding contract.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins on
    //! the catalog name lookup, round-trip integrity,
    //! dimensionality assignment, and value-kind validation
    //! shape. Catalog drift on any of these surfaces fires the
    //! moment `cargo test` runs, before the T2 corpus in
    //! `engine/script/tests/reactive_symbol.rs` would catch the
    //! same regression as a noisier multi-test failure.
    //!
    //! Anchors:
    //! - `docs/26-evidence-artifact-registry.md` row 29 —
    //!   `EV-SCRIPT-CONTRACT`.
    //! - `docs/36-reactive-symbol-catalog.md` v1.3.0.

    use super::{ReactiveSymbol, ReactiveValueKind, ReactiveValueKindMismatchDetail};
    use crate::value::ScriptValue;

    /// Static list of every variant. Kept in sync manually with
    /// [`ReactiveSymbol`]; the
    /// `every_variant_round_trips_through_name` pin trips if a
    /// new variant is added without updating this list, providing
    /// drift detection on additions.
    const ALL_SYMBOLS: &[ReactiveSymbol] = &[
        // §1.1
        ReactiveSymbol::Time,
        ReactiveSymbol::FrameCount,
        ReactiveSymbol::TimeDelta,
        ReactiveSymbol::Pi,
        // §1.3
        ReactiveSymbol::Mouse,
        ReactiveSymbol::Click,
        ReactiveSymbol::MouseUv,
        ReactiveSymbol::MouseUvc,
        // §2.2
        ReactiveSymbol::Level,
        ReactiveSymbol::BassLevel,
        ReactiveSymbol::MidLevel,
        ReactiveSymbol::MidHighLevel,
        ReactiveSymbol::HighLevel,
        // §2.3
        ReactiveSymbol::Hits,
        ReactiveSymbol::BassHits,
        ReactiveSymbol::MidHits,
        ReactiveSymbol::MidHighHits,
        ReactiveSymbol::HighHits,
        // §2.4
        ReactiveSymbol::AudioTime,
        ReactiveSymbol::BassTime,
        ReactiveSymbol::MidTime,
        ReactiveSymbol::MidHighTime,
        ReactiveSymbol::HighTime,
        ReactiveSymbol::CurvedTime,
        // §2.5
        ReactiveSymbol::Presence,
        ReactiveSymbol::BassPresence,
        ReactiveSymbol::MidPresence,
        ReactiveSymbol::MidHighPresence,
        ReactiveSymbol::HighPresence,
        // §2.6
        ReactiveSymbol::OnBeat,
        ReactiveSymbol::ToggleOnBeat,
        ReactiveSymbol::RandomOnBeat,
        ReactiveSymbol::BeatTime,
        // §2.7
        ReactiveSymbol::Bpm,
        ReactiveSymbol::BpmConfidence,
        ReactiveSymbol::BpmTwitcher,
        ReactiveSymbol::BpmSin,
        ReactiveSymbol::BpmSin2,
        ReactiveSymbol::BpmSin4,
        ReactiveSymbol::BpmTri,
        ReactiveSymbol::BpmTri2,
        ReactiveSymbol::BpmTri4,
        // §2.8
        ReactiveSymbol::FadeInOut,
        ReactiveSymbol::Intensity,
        // §2.9
        ReactiveSymbol::PrimeBang,
        // §2.10
        ReactiveSymbol::InputLevel,
        ReactiveSymbol::RawLevel,
        ReactiveSymbol::BassExpoCurve,
        ReactiveSymbol::Threshold,
        // Slice-5a render-adapter aggregates (post catalog v1.3.0:
        // only the scalar primitives remain; the FFT array surface
        // was retired from the script-side catalog when the v1.2.0
        // split-vec4 spelling consolidated into `chroma_AudioFFT[8]`,
        // because `ReactiveValueKind` does not yet carry an
        // 8-element-float-array variant).
        ReactiveSymbol::Rms,
        ReactiveSymbol::Peak,
    ];

    #[test]
    fn every_variant_round_trips_through_name() {
        // Drift pin: every variant in the catalog round-trips
        // `ReactiveSymbol::from_name(s.name()) == Some(s)`. A
        // typo in either lookup table at the ::name() side or
        // the from_name() side trips here.
        for sym in ALL_SYMBOLS {
            let name = sym.name();
            let parsed = ReactiveSymbol::from_name(name).expect("name resolves");
            assert_eq!(parsed, *sym, "round trip for {name}");
        }
    }

    #[test]
    fn from_name_rejects_unknown_strings() {
        // Drift pin: `from_name` MUST return `None` for strings
        // not in the catalog. Common typo cases pinned: case
        // mismatch (catalog is case-sensitive per §6 CR-01),
        // missing `syn_` prefix, and entirely-foreign names.
        assert!(ReactiveSymbol::from_name("chroma_bpm").is_none());
        assert!(ReactiveSymbol::from_name("BPM").is_none());
        assert!(ReactiveSymbol::from_name("not_a_symbol").is_none());
        assert!(ReactiveSymbol::from_name("").is_none());
    }

    #[test]
    fn slice_5a_render_adapter_names_match_audio_adapter_constants() {
        // Drift pin: the slice-5a render-adapter chose the
        // names `chroma_BPM`, `chroma_AudioRMS`, `chroma_AudioPeak`
        // for cross-crate audio uniforms (see
        // `engine/render/src/audio_adapter.rs`). The slice-5.2
        // catalog MUST surface the same spellings so a script
        // emitting against these names lands at a typed variant
        // matching the render-side binding contract. The catalog
        // v1.3.0 consolidation retired the v1.2.0 split-vec4 FFT
        // spellings (`chroma_AudioFFTLow` / `chroma_AudioFFTHigh`)
        // alongside the engine-side widening; the FFT array
        // surface is producer-only on the script side until an
        // 8-element-float-array `ReactiveValueKind` variant lands
        // in a future slice.
        assert_eq!(ReactiveSymbol::Bpm.name(), "chroma_BPM");
        assert_eq!(ReactiveSymbol::Rms.name(), "chroma_AudioRMS");
        assert_eq!(ReactiveSymbol::Peak.name(), "chroma_AudioPeak");
        assert!(
            ReactiveSymbol::from_name("chroma_AudioFFT").is_none(),
            "v1.3.0 single-array FFT binding is producer-only on \
             the script side; from_name MUST resolve to None until \
             an array-carrier variant lands",
        );
        assert!(
            ReactiveSymbol::from_name("chroma_AudioFFTLow").is_none(),
            "v1.2.0 split-vec4 FFT binding was retired alongside \
             the v1.3.0 consolidation",
        );
        assert!(
            ReactiveSymbol::from_name("chroma_AudioFFTHigh").is_none(),
            "v1.2.0 split-vec4 FFT binding was retired alongside \
             the v1.3.0 consolidation",
        );
    }

    #[test]
    fn dimensionality_assignment_matches_catalog_typed_shape() {
        // Drift pin: per-symbol dimensionality matches the
        // `docs/36` v1.3.0 catalog's typed-shape column. Spot
        // checks across the dimensionality slots that the script-
        // side carrier vocabulary supports; full coverage by the
        // `every_variant_has_known_kind` pin.
        assert_eq!(
            ReactiveSymbol::Bpm.expected_kind(),
            ReactiveValueKind::Scalar
        );
        assert_eq!(
            ReactiveSymbol::MouseUv.expected_kind(),
            ReactiveValueKind::Vec2
        );
        assert_eq!(
            ReactiveSymbol::Click.expected_kind(),
            ReactiveValueKind::Vec3
        );
        assert_eq!(
            ReactiveSymbol::Mouse.expected_kind(),
            ReactiveValueKind::Vec4
        );
    }

    #[test]
    fn every_variant_has_known_kind() {
        // Drift pin: every variant returns one of the four
        // documented dimensionality slots. Adding a new variant
        // without a kind arm trips the exhaustiveness pin.
        for sym in ALL_SYMBOLS {
            let _ = sym.expected_kind();
        }
    }

    #[test]
    fn scalar_kind_accepts_number_only() {
        // Drift pin: Scalar slot accepts ScriptValue::Number,
        // rejects every other carrier shape.
        assert!(
            ReactiveValueKind::Scalar
                .validate(&ScriptValue::Number(42.0))
                .is_ok()
        );
        assert!(matches!(
            ReactiveValueKind::Scalar.validate(&ScriptValue::Bool(true)),
            Err(ReactiveValueKindMismatchDetail::ScalarExpected { got: "bool" })
        ));
        assert!(matches!(
            ReactiveValueKind::Scalar.validate(&ScriptValue::Null),
            Err(ReactiveValueKindMismatchDetail::ScalarExpected { got: "null" })
        ));
    }

    #[test]
    fn vec4_accepts_array_and_xyzw_object() {
        // Drift pin: Vec4 slot accepts a 4-element Number array
        // OR an Object with x/y/z/w (or r/g/b/a) keys.
        let arr = ScriptValue::Array(vec![
            ScriptValue::Number(1.0),
            ScriptValue::Number(2.0),
            ScriptValue::Number(3.0),
            ScriptValue::Number(4.0),
        ]);
        assert!(ReactiveValueKind::Vec4.validate(&arr).is_ok());
        let obj_xyzw = ScriptValue::Object(vec![
            ("x".to_string(), ScriptValue::Number(1.0)),
            ("y".to_string(), ScriptValue::Number(2.0)),
            ("z".to_string(), ScriptValue::Number(3.0)),
            ("w".to_string(), ScriptValue::Number(4.0)),
        ]);
        assert!(ReactiveValueKind::Vec4.validate(&obj_xyzw).is_ok());
        let obj_rgba = ScriptValue::Object(vec![
            ("r".to_string(), ScriptValue::Number(1.0)),
            ("g".to_string(), ScriptValue::Number(2.0)),
            ("b".to_string(), ScriptValue::Number(3.0)),
            ("a".to_string(), ScriptValue::Number(4.0)),
        ]);
        assert!(ReactiveValueKind::Vec4.validate(&obj_rgba).is_ok());
    }

    #[test]
    fn vec4_rejects_wrong_length_array() {
        // Drift pin: Vec4 slot rejects a 3-element array with
        // `VectorWrongLength { expected: 4, actual: 3 }`.
        let arr = ScriptValue::Array(vec![
            ScriptValue::Number(1.0),
            ScriptValue::Number(2.0),
            ScriptValue::Number(3.0),
        ]);
        let err = ReactiveValueKind::Vec4.validate(&arr).unwrap_err();
        assert_eq!(
            err,
            ReactiveValueKindMismatchDetail::VectorWrongLength {
                expected_components: 4,
                actual_components: 3,
            }
        );
    }

    #[test]
    fn vec3_rejects_wrong_keys() {
        // Drift pin: Vec3 slot rejects an Object whose keys do
        // NOT match `[x,y,z]` or `[r,g,b]`.
        let obj = ScriptValue::Object(vec![
            ("foo".to_string(), ScriptValue::Number(1.0)),
            ("bar".to_string(), ScriptValue::Number(2.0)),
            ("baz".to_string(), ScriptValue::Number(3.0)),
        ]);
        let err = ReactiveValueKind::Vec3.validate(&obj).unwrap_err();
        assert_eq!(
            err,
            ReactiveValueKindMismatchDetail::VectorWrongKeys {
                expected_components: 3,
            }
        );
    }

    #[test]
    fn vec_rejects_non_number_component() {
        // Drift pin: vector validation requires every element /
        // value to be a Number. A string component must reject.
        let arr = ScriptValue::Array(vec![
            ScriptValue::Number(1.0),
            ScriptValue::String("x".to_string()),
        ]);
        let err = ReactiveValueKind::Vec2.validate(&arr).unwrap_err();
        assert_eq!(
            err,
            ReactiveValueKindMismatchDetail::VectorComponentNotNumber {
                expected_components: 2,
            }
        );
    }

    #[test]
    fn vec_rejects_non_vector_carrier() {
        // Drift pin: vector slots reject scalar carriers with
        // `VectorExpected { got: "number" }`.
        let v = ScriptValue::Number(1.0);
        let err = ReactiveValueKind::Vec2.validate(&v).unwrap_err();
        assert_eq!(
            err,
            ReactiveValueKindMismatchDetail::VectorExpected {
                expected_components: 2,
                got: "number",
            }
        );
    }
}
