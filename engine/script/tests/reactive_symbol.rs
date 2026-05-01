// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-SCRIPT-CONTRACT (T2) — slice-5.2 reactive-symbol
//! catalog binding corpus.
//!
//! Widens `docs/26-evidence-artifact-registry.md` row 29
//! (`EV-SCRIPT-CONTRACT`) past the slice-5.1 producer-side seam
//! corpus with the slice-5.2 typed catalog binding. T2 per
//! `docs/43-development-standards.md` §15.5: corpus conformance
//! over the public catalog surface.
//!
//! Slice-5.2 corpus per ADR-001 (runtime model) + ADR-006 (thread
//! and process model) + ADR-012 (script sandbox engine):
//!
//! - `js_emit_routes_to_typed_variant_for_known_audio_uniform` —
//!   end-to-end script-mutation: a JS call to
//!   `__chromaticityEmit('syn_BPM', 120)` lands on the host
//!   sender as the slice-5.1 catch-all variant; the host's
//!   subsequent `ScriptCommand::route()` call resolves it to
//!   `TypedReactiveWrite { symbol: ReactiveSymbol::Bpm, value:
//!   Number(120) }`.
//! - `route_resolves_every_documented_audio_scalar` — every
//!   documented audio-scalar name in the slice-5.2 catalog
//!   resolves through `route()` to the typed variant carrying
//!   the canonical [`ReactiveSymbol`].
//! - `route_resolves_vec4_via_array_carrier` — a JS-side emit of
//!   a 4-element array under a `Vec4`-shape symbol resolves
//!   through `route()` to the typed variant.
//! - `route_resolves_vec4_via_object_carrier` — a JS-side emit
//!   of an `{x,y,z,w}` object under a `Vec4`-shape symbol
//!   resolves the same way (matching `docs/36` §5 vector
//!   spelling).
//! - `route_rejects_unknown_name` — `route()` on an unknown name
//!   returns [`ScriptError::ReactiveSymbolUnknown`] verbatim.
//! - `route_rejects_kind_mismatch_scalar_got_array` — `route()`
//!   on a known scalar symbol with an array carrier returns
//!   [`ScriptError::ReactiveValueKindMismatch`] carrying the
//!   resolved symbol's name + expected kind.
//! - `route_passes_typed_variant_through_unchanged` — calling
//!   `route()` on an already-typed variant is the identity. Pin
//!   for idempotency.
//! - `slice_5a_render_adapter_split_fft_names_resolve` — the
//!   slice-5a `chroma_AudioFFTLow` / `chroma_AudioFFTHigh` split spelling
//!   resolves through the catalog to the typed variants
//!   reserved for it.
//! - `multiple_runtimes_route_independently` — two separate
//!   [`chromaticity_script::ScriptRuntime`] instances each
//!   producing a sender-bound emit route through the catalog
//!   without cross-contamination. Mirrors slice-5.1's per-runtime
//!   isolate contract for the typed routing surface.
//!
//! The slice-5.3+ engine/runtime adapter that bridges
//! [`chromaticity_script::ScriptCommandSender`] onto
//! `chromaticity_runtime::CommandBus` is deferred. The slice-5.2
//! catalog is the typed contract that adapter consumes, but the
//! routing is producer-side and lives entirely under the
//! [`chromaticity_script::ScriptCommand::route`] API surface.
//!
//! Anchors:
//! - `docs/adr/ADR-001-runtime-model.md` — runtime model.
//! - `docs/adr/ADR-006-thread-and-process-model.md` — thread + process model.
//! - `docs/adr/ADR-012-script-sandbox-engine.md` — engine choice + budgets.
//! - `docs/26-evidence-artifact-registry.md` row 29 — `EV-SCRIPT-CONTRACT`.
//! - `docs/36-reactive-symbol-catalog.md` v1.1.0 — symbol vocabulary.
//! - `docs/43-development-standards.md` §15.5 — T-tier definitions.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::sync::{Arc, Mutex};

use chromaticity_script::{
    ReactiveSymbol, ReactiveValueKind, ScriptCommand, ScriptCommandError, ScriptCommandSender,
    ScriptError, ScriptRuntime, ScriptValue,
};

/// External (out-of-crate) buffering sender mirroring the slice-
/// 5.1 corpus's `BufferingSender`. The slice-5.2 corpus exercises
/// only the public surface, so we re-implement here rather than
/// re-using crate-private helpers.
struct BufferingSender {
    commands: Mutex<Vec<ScriptCommand>>,
}

impl BufferingSender {
    fn new() -> Self {
        Self {
            commands: Mutex::new(Vec::new()),
        }
    }

    fn drain(&self) -> Vec<ScriptCommand> {
        std::mem::take(&mut self.commands.lock().unwrap())
    }
}

impl ScriptCommandSender for BufferingSender {
    fn send(&self, command: ScriptCommand) -> Result<(), ScriptCommandError> {
        self.commands.lock().unwrap().push(command);
        Ok(())
    }
}

#[test]
fn js_emit_routes_to_typed_variant_for_known_audio_uniform() {
    let rt = ScriptRuntime::new().expect("runtime new");
    let sender = Arc::new(BufferingSender::new());
    let rt = rt
        .with_command_sender(sender.clone())
        .expect("with_command_sender");
    rt.execute("__chromaticityEmit('syn_BPM', 120)")
        .expect("emit succeeds");
    let drained = sender.drain();
    assert_eq!(drained.len(), 1, "exactly one emit");
    // Slice-5.1 install path produces the catch-all variant.
    let routed = drained[0].clone().route().expect("route resolves");
    match routed {
        ScriptCommand::TypedReactiveWrite { symbol, value } => {
            assert_eq!(symbol, ReactiveSymbol::Bpm);
            assert_eq!(value, ScriptValue::Number(120.0));
            assert_eq!(symbol.name(), "syn_BPM");
            assert_eq!(symbol.expected_kind(), ReactiveValueKind::Scalar);
        }
        other => panic!("expected TypedReactiveWrite, got {other:?}"),
    }
}

#[test]
fn route_resolves_every_documented_audio_scalar() {
    // A representative spread across the audio-scalar families
    // (§2.2 / §2.3 / §2.4 / §2.5 / §2.6 / §2.7 / §2.8 / §2.9 /
    // §2.10 + slice-5a aggregates). Each name resolves through
    // `route()` and round-trips back to the same canonical
    // string via `ReactiveSymbol::name()`.
    let cases: &[(&str, ReactiveSymbol)] = &[
        ("syn_Level", ReactiveSymbol::Level),
        ("syn_BassLevel", ReactiveSymbol::BassLevel),
        ("syn_Hits", ReactiveSymbol::Hits),
        ("syn_BassTime", ReactiveSymbol::BassTime),
        ("syn_Presence", ReactiveSymbol::Presence),
        ("syn_OnBeat", ReactiveSymbol::OnBeat),
        ("syn_BPM", ReactiveSymbol::Bpm),
        ("syn_BPMConfidence", ReactiveSymbol::BpmConfidence),
        ("syn_FadeInOut", ReactiveSymbol::FadeInOut),
        ("chroma_PrimeBang", ReactiveSymbol::ChromaPrimeBang),
        ("syn_PrimeBang", ReactiveSymbol::SynPrimeBang),
        ("syn_InputLevel", ReactiveSymbol::InputLevel),
        ("syn_RawLevel", ReactiveSymbol::RawLevel),
        ("syn_Threshold", ReactiveSymbol::Threshold),
        ("chroma_AudioRMS", ReactiveSymbol::Rms),
        ("chroma_AudioPeak", ReactiveSymbol::Peak),
    ];
    for (name, expected_symbol) in cases {
        let cmd = ScriptCommand::ReactiveWrite {
            name: (*name).to_string(),
            value: ScriptValue::Number(0.5),
        };
        let routed = cmd.route().unwrap_or_else(|e| {
            panic!("route '{name}' must succeed; got error {e:?}");
        });
        match routed {
            ScriptCommand::TypedReactiveWrite { symbol, value } => {
                assert_eq!(symbol, *expected_symbol, "symbol mismatch for '{name}'");
                assert_eq!(symbol.name(), *name, "round-trip name mismatch");
                assert_eq!(value, ScriptValue::Number(0.5));
            }
            other => panic!("expected TypedReactiveWrite for '{name}', got {other:?}"),
        }
    }
}

#[test]
fn route_resolves_vec4_via_array_carrier() {
    // `_mouse` is the canonical Vec4-shape symbol on the script-
    // side catalog (`x, y, x_prev_click, y_prev_click` per docs/36
    // §1.3). JS `[a,b,c,d]` array maps to ScriptValue::Array via
    // the slice-1 lowering. (The slice-5a v1.2.0 transitional
    // `chroma_AudioFFTLow` Vec4 spelling was retired alongside the
    // catalog v1.3.0 single-array consolidation.)
    let rt = ScriptRuntime::new().expect("runtime new");
    let sender = Arc::new(BufferingSender::new());
    let rt = rt
        .with_command_sender(sender.clone())
        .expect("with_command_sender");
    rt.execute("__chromaticityEmit('_mouse', [0.1, 0.2, 0.3, 0.4])")
        .expect("emit succeeds");
    let drained = sender.drain();
    let routed = drained[0].clone().route().expect("route resolves");
    match routed {
        ScriptCommand::TypedReactiveWrite { symbol, value } => {
            assert_eq!(symbol, ReactiveSymbol::Mouse);
            assert_eq!(symbol.expected_kind(), ReactiveValueKind::Vec4);
            match value {
                ScriptValue::Array(items) => {
                    assert_eq!(items.len(), 4);
                    assert_eq!(items[0], ScriptValue::Number(0.1));
                    assert_eq!(items[3], ScriptValue::Number(0.4));
                }
                other => panic!("expected Array carrier, got {other:?}"),
            }
        }
        other => panic!("expected TypedReactiveWrite, got {other:?}"),
    }
}

#[test]
fn route_resolves_vec4_via_object_carrier() {
    // `docs/36` §5: vec4 may be `{x,y,z,w}` or `{r,g,b,a}`. Both
    // shapes route. JS `{x:1,y:2,z:3,w:4}` lowers to
    // ScriptValue::Object preserving insertion order. (Same Vec4
    // canonical-symbol substitution rationale as the array-carrier
    // pin above; the v1.2.0 split-vec4 FFT spellings were retired
    // post catalog v1.3.0.)
    let rt = ScriptRuntime::new().expect("runtime new");
    let sender = Arc::new(BufferingSender::new());
    let rt = rt
        .with_command_sender(sender.clone())
        .expect("with_command_sender");
    rt.execute("__chromaticityEmit('_mouse', { x: 0.5, y: 0.6, z: 0.7, w: 0.8 })")
        .expect("emit succeeds");
    let drained = sender.drain();
    let routed = drained[0].clone().route().expect("route resolves");
    match routed {
        ScriptCommand::TypedReactiveWrite { symbol, value } => {
            assert_eq!(symbol, ReactiveSymbol::Mouse);
            assert_eq!(symbol.expected_kind(), ReactiveValueKind::Vec4);
            match value {
                ScriptValue::Object(entries) => {
                    assert_eq!(entries.len(), 4);
                    let keys: Vec<&str> = entries.iter().map(|(k, _)| k.as_str()).collect();
                    assert!(keys.contains(&"x"));
                    assert!(keys.contains(&"w"));
                }
                other => panic!("expected Object carrier, got {other:?}"),
            }
        }
        other => panic!("expected TypedReactiveWrite, got {other:?}"),
    }
}

#[test]
fn route_rejects_unknown_name() {
    let cmd = ScriptCommand::ReactiveWrite {
        name: "definitely_not_a_symbol".to_string(),
        value: ScriptValue::Number(1.0),
    };
    let err = cmd.route().expect_err("unknown name must fail");
    match err {
        ScriptError::ReactiveSymbolUnknown { name } => {
            assert_eq!(name, "definitely_not_a_symbol");
        }
        other => panic!("expected ReactiveSymbolUnknown, got {other:?}"),
    }
}

#[test]
fn route_rejects_kind_mismatch_scalar_got_array() {
    // `syn_BPM` is `Scalar`. An array carrier MUST reject.
    let cmd = ScriptCommand::ReactiveWrite {
        name: "syn_BPM".to_string(),
        value: ScriptValue::Array(vec![ScriptValue::Number(120.0)]),
    };
    let err = cmd.route().expect_err("scalar+array must fail");
    match err {
        ScriptError::ReactiveValueKindMismatch {
            symbol_name,
            expected,
            ..
        } => {
            assert_eq!(symbol_name, "syn_BPM");
            assert_eq!(expected, ReactiveValueKind::Scalar);
        }
        other => panic!("expected ReactiveValueKindMismatch, got {other:?}"),
    }
}

#[test]
fn route_passes_typed_variant_through_unchanged() {
    // Idempotency pin: routing an already-typed variant is the
    // identity.
    let original = ScriptCommand::TypedReactiveWrite {
        symbol: ReactiveSymbol::Bpm,
        value: ScriptValue::Number(140.0),
    };
    let routed = original.clone().route().expect("typed passes through");
    assert_eq!(routed, original);
}

#[test]
fn slice_5a_v1_2_0_split_fft_names_no_longer_route() {
    // The slice-5a v1.2.0 transitional split-vec4 FFT spellings
    // (`chroma_AudioFFTLow` / `chroma_AudioFFTHigh`) were retired
    // alongside the catalog v1.3.0 single-array consolidation
    // (`chroma_AudioFFT[8]` backed by
    // `engine/render::UniformValue::FloatArray`). The script-side
    // carrier vocabulary (`ReactiveValueKind`) does not yet support
    // 8-element float-array writes, so the FFT array surface is
    // producer-only on the script side until a future array-
    // carrier slice. Per `docs/22` Catalog Ratification Log v1.3.0
    // entry, no scenes shipped against the v1.2.0 split-vec4
    // surface, so retiring the script-side mirror is non-breaking.
    // Both retired names + the consolidated `chroma_AudioFFT` name
    // MUST resolve to `ReactiveSymbolUnknown` at the route surface.
    let retired_names = [
        "chroma_AudioFFTLow",
        "chroma_AudioFFTHigh",
        "chroma_AudioFFT",
    ];
    for retired in retired_names {
        let cmd = ScriptCommand::ReactiveWrite {
            name: retired.to_string(),
            value: ScriptValue::Array(vec![
                ScriptValue::Number(0.0),
                ScriptValue::Number(0.0),
                ScriptValue::Number(0.0),
                ScriptValue::Number(0.0),
            ]),
        };
        let err = cmd.route().expect_err("retired FFT names must reject");
        match err {
            ScriptError::ReactiveSymbolUnknown { name: observed } => {
                assert_eq!(observed, retired, "echo-back catalog-miss name");
            }
            other => panic!("expected ReactiveSymbolUnknown for '{retired}', got {other:?}"),
        }
    }
}

#[test]
fn multiple_runtimes_route_independently() {
    // Two independent runtimes, each with their own sender.
    // Routing the captured command on each side yields the same
    // typed variant — runtimes don't cross-contaminate the
    // catalog surface.
    let sender_a = Arc::new(BufferingSender::new());
    let sender_b = Arc::new(BufferingSender::new());
    let rt_a = ScriptRuntime::new()
        .and_then(|r| r.with_command_sender(sender_a.clone()))
        .expect("rt a");
    let rt_b = ScriptRuntime::new()
        .and_then(|r| r.with_command_sender(sender_b.clone()))
        .expect("rt b");
    rt_a.execute("__chromaticityEmit('syn_BassLevel', 0.25)")
        .expect("a emit");
    rt_b.execute("__chromaticityEmit('syn_HighLevel', 0.75)")
        .expect("b emit");
    let drained_a = sender_a.drain();
    let drained_b = sender_b.drain();
    assert_eq!(drained_a.len(), 1);
    assert_eq!(drained_b.len(), 1);
    let routed_a = drained_a[0].clone().route().expect("route a");
    let routed_b = drained_b[0].clone().route().expect("route b");
    match routed_a {
        ScriptCommand::TypedReactiveWrite { symbol, value } => {
            assert_eq!(symbol, ReactiveSymbol::BassLevel);
            assert_eq!(value, ScriptValue::Number(0.25));
        }
        other => panic!("a: expected TypedReactiveWrite, got {other:?}"),
    }
    match routed_b {
        ScriptCommand::TypedReactiveWrite { symbol, value } => {
            assert_eq!(symbol, ReactiveSymbol::HighLevel);
            assert_eq!(value, ScriptValue::Number(0.75));
        }
        other => panic!("b: expected TypedReactiveWrite, got {other:?}"),
    }
}
