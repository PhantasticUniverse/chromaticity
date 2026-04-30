// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Typed reactive-symbol-catalog binding for slice 5.2.
//!
//! Slice 5.1 mounted the foundational producer-side seam: a JS-side
//! `__chromaticityEmit(name, value)` call lowers to
//! [`crate::ScriptCommand::ReactiveWrite { name, value }`] — a
//! string-keyed plain-data shape. Slice 5.2 closes the
//! `docs/36-reactive-symbol-catalog.md` v1.1.0 binding gap by
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
//! v1.1.0 §1 (Standard Uniforms — frame/timing + mouse) and §2
//! (Audio Uniforms — full level/hit/time/presence/beat/BPM
//! families + large features + PrimeBang + reference-surface
//! uniforms + the slice-5a render-adapter split FFT spelling
//! `chroma_AudioFFTLow` / `chroma_AudioFFTHigh` + `chroma_AudioRMS` / `chroma_AudioPeak`). Pass-only
//! and fragment-only symbols (`_uv`, `_uvc`, `_xy`, `RENDERSIZE`,
//! `PASSINDEX`) are excluded per `docs/36` §5 (the catalog's own
//! Script Host Mirror Rules). Sampler symbols (`syn_FinalPass`,
//! `syn_Media`, `syn_UserImage`, `syn_Spectrum`, `syn_LevelTrail`)
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
//!   keys `x`/`y`/`z`/`w` or `r`/`g`/`b`/`a`. The slice-5a
//!   render-adapter `chroma_AudioFFTLow` / `chroma_AudioFFTHigh` split lands here.
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
//! - `docs/36-reactive-symbol-catalog.md` v1.1.0 — symbol
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
/// subset of `docs/36-reactive-symbol-catalog.md` v1.1.0.
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
    /// `syn_Level`.
    Level,
    /// `syn_BassLevel`.
    BassLevel,
    /// `syn_MidLevel`.
    MidLevel,
    /// `syn_MidHighLevel`.
    MidHighLevel,
    /// `syn_HighLevel`.
    HighLevel,

    // §2.3 Hits — Scalar, family tolerance `HIT`
    /// `syn_Hits`.
    Hits,
    /// `syn_BassHits`.
    BassHits,
    /// `syn_MidHits`.
    MidHits,
    /// `syn_MidHighHits`.
    MidHighHits,
    /// `syn_HighHits`.
    HighHits,

    // §2.4 Time clocks — Scalar, family tolerance `TIM`
    /// `syn_Time`.
    SynTime,
    /// `syn_BassTime`.
    BassTime,
    /// `syn_MidTime`.
    MidTime,
    /// `syn_MidHighTime`.
    MidHighTime,
    /// `syn_HighTime`.
    HighTime,
    /// `syn_CurvedTime`.
    CurvedTime,

    // §2.5 Presence — Scalar, family tolerance `PRS`
    /// `syn_Presence`.
    Presence,
    /// `syn_BassPresence`.
    BassPresence,
    /// `syn_MidPresence`.
    MidPresence,
    /// `syn_MidHighPresence`.
    MidHighPresence,
    /// `syn_HighPresence`.
    HighPresence,

    // §2.6 Beat — Scalar, family tolerance `BEAT`
    /// `syn_OnBeat`.
    OnBeat,
    /// `syn_ToggleOnBeat`.
    ToggleOnBeat,
    /// `syn_RandomOnBeat`.
    RandomOnBeat,
    /// `syn_BeatTime`.
    BeatTime,

    // §2.7 BPM — Scalar, family tolerance `BPM`
    /// `syn_BPM` — clamped `[50, 220]`. Slice-5a render-adapter
    /// also publishes this name.
    Bpm,
    /// `syn_BPMConfidence`.
    BpmConfidence,
    /// `syn_BPMTwitcher`.
    BpmTwitcher,
    /// `syn_BPMSin`.
    BpmSin,
    /// `syn_BPMSin2`.
    BpmSin2,
    /// `syn_BPMSin4`.
    BpmSin4,
    /// `syn_BPMTri`.
    BpmTri,
    /// `syn_BPMTri2`.
    BpmTri2,
    /// `syn_BPMTri4`.
    BpmTri4,

    // §2.8 Large features — Scalar, family tolerance `LRG`
    /// `syn_FadeInOut`.
    FadeInOut,
    /// `syn_Intensity`.
    Intensity,

    // §2.9 PrimeBang — Scalar (Profile D)
    /// `chroma_PrimeBang` — native canonical name.
    ChromaPrimeBang,
    /// `syn_PrimeBang` — compat alias for `chroma_PrimeBang`.
    SynPrimeBang,

    // §2.10 Reference-surface uniforms — Scalar
    /// `syn_InputLevel` — pre-AGC instantaneous input level.
    InputLevel,
    /// `syn_RawLevel` — alias of `syn_InputLevel`.
    RawLevel,
    /// `syn_BassExpoCurve` — `pow(syn_BassLevel, 3.0)`.
    BassExpoCurve,
    /// `syn_Threshold` — operator-settable noise-floor gate.
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
    /// `chroma_AudioFFTLow` — low-band FFT vec4 `[0..=3]` (slice-5a
    /// render-adapter spelling).
    FftLow,
    /// `chroma_AudioFFTHigh` — high-band FFT vec4 `[4..=7]` (slice-5a
    /// render-adapter spelling).
    FftHigh,
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
    /// `x`/`y`/`z`/`w` or `r`/`g`/`b`/`a` key sets. The slice-5a
    /// render-adapter `chroma_AudioFFTLow` / `chroma_AudioFFTHigh` split lands here.
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
    /// `docs/36` v1.1.0 spellings the slice-5.2 catalog covers;
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
            "syn_Level" => Self::Level,
            "syn_BassLevel" => Self::BassLevel,
            "syn_MidLevel" => Self::MidLevel,
            "syn_MidHighLevel" => Self::MidHighLevel,
            "syn_HighLevel" => Self::HighLevel,
            // §2.3
            "syn_Hits" => Self::Hits,
            "syn_BassHits" => Self::BassHits,
            "syn_MidHits" => Self::MidHits,
            "syn_MidHighHits" => Self::MidHighHits,
            "syn_HighHits" => Self::HighHits,
            // §2.4
            "syn_Time" => Self::SynTime,
            "syn_BassTime" => Self::BassTime,
            "syn_MidTime" => Self::MidTime,
            "syn_MidHighTime" => Self::MidHighTime,
            "syn_HighTime" => Self::HighTime,
            "syn_CurvedTime" => Self::CurvedTime,
            // §2.5
            "syn_Presence" => Self::Presence,
            "syn_BassPresence" => Self::BassPresence,
            "syn_MidPresence" => Self::MidPresence,
            "syn_MidHighPresence" => Self::MidHighPresence,
            "syn_HighPresence" => Self::HighPresence,
            // §2.6
            "syn_OnBeat" => Self::OnBeat,
            "syn_ToggleOnBeat" => Self::ToggleOnBeat,
            "syn_RandomOnBeat" => Self::RandomOnBeat,
            "syn_BeatTime" => Self::BeatTime,
            // §2.7
            "syn_BPM" => Self::Bpm,
            "syn_BPMConfidence" => Self::BpmConfidence,
            "syn_BPMTwitcher" => Self::BpmTwitcher,
            "syn_BPMSin" => Self::BpmSin,
            "syn_BPMSin2" => Self::BpmSin2,
            "syn_BPMSin4" => Self::BpmSin4,
            "syn_BPMTri" => Self::BpmTri,
            "syn_BPMTri2" => Self::BpmTri2,
            "syn_BPMTri4" => Self::BpmTri4,
            // §2.8
            "syn_FadeInOut" => Self::FadeInOut,
            "syn_Intensity" => Self::Intensity,
            // §2.9
            "chroma_PrimeBang" => Self::ChromaPrimeBang,
            "syn_PrimeBang" => Self::SynPrimeBang,
            // §2.10
            "syn_InputLevel" => Self::InputLevel,
            "syn_RawLevel" => Self::RawLevel,
            "syn_BassExpoCurve" => Self::BassExpoCurve,
            "syn_Threshold" => Self::Threshold,
            // Slice-5a render-adapter aggregates
            "chroma_AudioRMS" => Self::Rms,
            "chroma_AudioPeak" => Self::Peak,
            "chroma_AudioFFTLow" => Self::FftLow,
            "chroma_AudioFFTHigh" => Self::FftHigh,
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
            Self::Level => "syn_Level",
            Self::BassLevel => "syn_BassLevel",
            Self::MidLevel => "syn_MidLevel",
            Self::MidHighLevel => "syn_MidHighLevel",
            Self::HighLevel => "syn_HighLevel",
            // §2.3
            Self::Hits => "syn_Hits",
            Self::BassHits => "syn_BassHits",
            Self::MidHits => "syn_MidHits",
            Self::MidHighHits => "syn_MidHighHits",
            Self::HighHits => "syn_HighHits",
            // §2.4
            Self::SynTime => "syn_Time",
            Self::BassTime => "syn_BassTime",
            Self::MidTime => "syn_MidTime",
            Self::MidHighTime => "syn_MidHighTime",
            Self::HighTime => "syn_HighTime",
            Self::CurvedTime => "syn_CurvedTime",
            // §2.5
            Self::Presence => "syn_Presence",
            Self::BassPresence => "syn_BassPresence",
            Self::MidPresence => "syn_MidPresence",
            Self::MidHighPresence => "syn_MidHighPresence",
            Self::HighPresence => "syn_HighPresence",
            // §2.6
            Self::OnBeat => "syn_OnBeat",
            Self::ToggleOnBeat => "syn_ToggleOnBeat",
            Self::RandomOnBeat => "syn_RandomOnBeat",
            Self::BeatTime => "syn_BeatTime",
            // §2.7
            Self::Bpm => "syn_BPM",
            Self::BpmConfidence => "syn_BPMConfidence",
            Self::BpmTwitcher => "syn_BPMTwitcher",
            Self::BpmSin => "syn_BPMSin",
            Self::BpmSin2 => "syn_BPMSin2",
            Self::BpmSin4 => "syn_BPMSin4",
            Self::BpmTri => "syn_BPMTri",
            Self::BpmTri2 => "syn_BPMTri2",
            Self::BpmTri4 => "syn_BPMTri4",
            // §2.8
            Self::FadeInOut => "syn_FadeInOut",
            Self::Intensity => "syn_Intensity",
            // §2.9
            Self::ChromaPrimeBang => "chroma_PrimeBang",
            Self::SynPrimeBang => "syn_PrimeBang",
            // §2.10
            Self::InputLevel => "syn_InputLevel",
            Self::RawLevel => "syn_RawLevel",
            Self::BassExpoCurve => "syn_BassExpoCurve",
            Self::Threshold => "syn_Threshold",
            // Slice-5a render-adapter aggregates
            Self::Rms => "chroma_AudioRMS",
            Self::Peak => "chroma_AudioPeak",
            Self::FftLow => "chroma_AudioFFTLow",
            Self::FftHigh => "chroma_AudioFFTHigh",
        }
    }

    /// Dimensionality slot for the symbol. The carrier
    /// [`ScriptValue`] in a typed write must satisfy
    /// [`ReactiveValueKind::validate`] against this slot.
    ///
    /// Per `docs/36` v1.1.0:
    /// - All audio-uniform scalars (§2.2 / §2.3 / §2.4 / §2.5 /
    ///   §2.6 / §2.7 / §2.8 / §2.9 / §2.10) are `Scalar`.
    /// - Frame/timing (§1.1) values exposed to the script host
    ///   are `Scalar`.
    /// - Mouse symbols (§1.3) follow the catalog's typed shape:
    ///   `_mouse` is `Vec4`, `_click` is `Vec3`, `_muv` / `_muvc`
    ///   are `Vec2`.
    /// - Slice-5a render-adapter aggregates: `chroma_AudioRMS` /
    ///   `chroma_AudioPeak` are `Scalar`; `chroma_AudioFFTLow` / `chroma_AudioFFTHigh` are
    ///   `Vec4`.
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
            Self::SynTime
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
            Self::ChromaPrimeBang | Self::SynPrimeBang => ReactiveValueKind::Scalar,
            // §2.10 — all Scalar
            Self::InputLevel | Self::RawLevel | Self::BassExpoCurve | Self::Threshold => {
                ReactiveValueKind::Scalar
            }
            // Slice-5a render-adapter aggregates
            Self::Rms | Self::Peak => ReactiveValueKind::Scalar,
            Self::FftLow | Self::FftHigh => ReactiveValueKind::Vec4,
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
    //! - `docs/36-reactive-symbol-catalog.md` v1.1.0.

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
        ReactiveSymbol::SynTime,
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
        ReactiveSymbol::ChromaPrimeBang,
        ReactiveSymbol::SynPrimeBang,
        // §2.10
        ReactiveSymbol::InputLevel,
        ReactiveSymbol::RawLevel,
        ReactiveSymbol::BassExpoCurve,
        ReactiveSymbol::Threshold,
        // Slice-5a render-adapter aggregates
        ReactiveSymbol::Rms,
        ReactiveSymbol::Peak,
        ReactiveSymbol::FftLow,
        ReactiveSymbol::FftHigh,
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
        assert!(ReactiveSymbol::from_name("syn_bpm").is_none());
        assert!(ReactiveSymbol::from_name("BPM").is_none());
        assert!(ReactiveSymbol::from_name("not_a_symbol").is_none());
        assert!(ReactiveSymbol::from_name("").is_none());
    }

    #[test]
    fn slice_5a_render_adapter_names_match_audio_adapter_constants() {
        // Drift pin: the slice-5a render-adapter chose the
        // names `syn_BPM`, `chroma_AudioRMS`, `chroma_AudioPeak`, `chroma_AudioFFTLow`,
        // `chroma_AudioFFTHigh` for cross-crate audio uniforms (see
        // `engine/render/src/audio_adapter.rs`). The slice-5.2
        // catalog MUST surface the same spellings so a script
        // emitting against these names lands at a typed variant
        // matching the render-side binding contract.
        assert_eq!(ReactiveSymbol::Bpm.name(), "syn_BPM");
        assert_eq!(ReactiveSymbol::Rms.name(), "chroma_AudioRMS");
        assert_eq!(ReactiveSymbol::Peak.name(), "chroma_AudioPeak");
        assert_eq!(ReactiveSymbol::FftLow.name(), "chroma_AudioFFTLow");
        assert_eq!(ReactiveSymbol::FftHigh.name(), "chroma_AudioFFTHigh");
    }

    #[test]
    fn dimensionality_assignment_matches_catalog_typed_shape() {
        // Drift pin: per-symbol dimensionality matches the
        // `docs/36` v1.1.0 catalog's typed-shape column. Spot
        // checks across the four dimensionality slots; full
        // coverage by the `every_variant_has_known_kind` pin.
        assert_eq!(
            ReactiveSymbol::Bpm.expected_kind(),
            ReactiveValueKind::Scalar
        );
        assert_eq!(
            ReactiveSymbol::FftLow.expected_kind(),
            ReactiveValueKind::Vec4
        );
        assert_eq!(
            ReactiveSymbol::FftHigh.expected_kind(),
            ReactiveValueKind::Vec4
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
