// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-AUDIO-UNIFORM-IPC (T2) — engine→UI audio-uniform IPC
//! payload round-trip conformance.
//!
//! T2 per docs/43 §15.5: this corpus drives the producer-side
//! [`chromaticity_ui_bridge::AudioUniformSnapshotPayload`] across the
//! ts-rs newtype + bidirectional `From<chromaticity_audio::AudioUniformSnapshot>`
//! conversion + serde JSON wire shape per
//! `docs/adr/ADR-019-audio-uniform-ipc-bridge.md` (widened for the
//! ADR-024 relative reactive bands). Field-by-field equality across
//! the 17-numeric-field surface (one `bpm`, one `rms`, one `peak`,
//! eight `fft_bands`, and the six MilkDrop-class relative bands —
//! `bass`/`mid`/`treb` + their `*_att` variants) is the load-bearing
//! assertion: any future payload-shape divergence between the engine
//! type and the bridge newtype surfaces here before the committed
//! ts-rs binding under
//! `src/lib/ipc/bindings/AudioUniformSnapshotPayload.ts` drifts.
//!
//! The Wave 1c-α PR 1 closure registers this corpus against the
//! `Planned` row in `docs/26-evidence-artifact-registry.md`; the
//! `Pass` transition lands in the Wave 1c-α closing docs PR after
//! the consumer-side Svelte-listener wiring (PR 2) demonstrates the
//! end-to-end Tauri-event subscription path.
//!
//! Lives under `app/ui-bridge/tests/` (outside the
//! `engine/**`/`integrations/**` evidence-tag-lint coverage scope per
//! `scripts/evidence-tag-lint.sh`); the in-source T1 drift pins under
//! `engine/runtime/src/scheduler/ipc.rs` carry the registry-anchored
//! `Evidence:` tag the lint enforces.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_audio::{AudioAxisAvailability as EngineAxisAvailability, AudioUniformSnapshot};
use chromaticity_ui_bridge::{AudioAxisAvailability, AudioUniformSnapshotPayload};
use ts_rs::TS;

/// Helper: a non-default snapshot exercising distinct values across
/// every field so a copy-paste field-mapping bug surfaces (e.g. swapping
/// `rms` and `peak` in a hand-rolled `From` impl).
fn nonzero_snapshot() -> AudioUniformSnapshot {
    AudioUniformSnapshot {
        bpm: 137.5,
        bpm_availability: EngineAxisAvailability::Ready,
        rms: 0.42,
        peak: 0.81,
        fft_bands: [0.10, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80],
        fft_availability: EngineAxisAvailability::Ready,
        // Distinct relative-band values so a copy-paste field-mapping bug
        // (e.g. swapping `bass`/`treb` or `mid`/`mid_att`) surfaces.
        bass: 1.10,
        mid: 1.21,
        treb: 1.32,
        bass_att: 0.93,
        mid_att: 0.84,
        treb_att: 0.75,
        degraded: false,
    }
}

fn nonzero_payload() -> AudioUniformSnapshotPayload {
    AudioUniformSnapshotPayload {
        bpm: 137.5,
        bpm_availability: AudioAxisAvailability::Ready,
        rms: 0.42,
        peak: 0.81,
        fft_bands: [0.10, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80],
        fft_availability: AudioAxisAvailability::Ready,
        bass: 1.10,
        mid: 1.21,
        treb: 1.32,
        bass_att: 0.93,
        mid_att: 0.84,
        treb_att: 0.75,
        degraded: false,
    }
}

#[test]
fn engine_to_payload_field_by_field_equality() {
    // Producer-side conversion: the engine type's value flows through
    // the `From<AudioUniformSnapshot>` impl. Field-by-field assertion
    // pins the 11-field surface so a future field-mapping bug
    // surfaces here.
    let snapshot = nonzero_snapshot();
    let payload: AudioUniformSnapshotPayload = snapshot.into();

    assert_eq!(payload.bpm, snapshot.bpm);
    assert_eq!(payload.bpm_availability, AudioAxisAvailability::Ready);
    assert_eq!(payload.rms, snapshot.rms);
    assert_eq!(payload.peak, snapshot.peak);
    for (i, (lhs, rhs)) in payload
        .fft_bands
        .iter()
        .zip(snapshot.fft_bands.iter())
        .enumerate()
    {
        assert_eq!(
            lhs, rhs,
            "fft_bands[{i}] mismatch: payload={lhs}, snapshot={rhs}"
        );
    }
    assert_eq!(payload.fft_availability, AudioAxisAvailability::Ready);
    assert_eq!(payload.bass, snapshot.bass);
    assert_eq!(payload.mid, snapshot.mid);
    assert_eq!(payload.treb, snapshot.treb);
    assert_eq!(payload.bass_att, snapshot.bass_att);
    assert_eq!(payload.mid_att, snapshot.mid_att);
    assert_eq!(payload.treb_att, snapshot.treb_att);
    assert_eq!(payload.degraded, snapshot.degraded);
}

#[test]
fn payload_to_engine_round_trip_field_by_field_equality() {
    // Reverse direction: a payload constructed on the consumer side
    // (e.g. by deserialising an event payload) converts back into the
    // engine type with field-identity preserved.
    let payload = nonzero_payload();
    let snapshot: AudioUniformSnapshot = payload.into();

    assert_eq!(snapshot.bpm, payload.bpm);
    assert_eq!(snapshot.bpm_availability, EngineAxisAvailability::Ready);
    assert_eq!(snapshot.rms, payload.rms);
    assert_eq!(snapshot.peak, payload.peak);
    for (i, (lhs, rhs)) in snapshot
        .fft_bands
        .iter()
        .zip(payload.fft_bands.iter())
        .enumerate()
    {
        assert_eq!(
            lhs, rhs,
            "fft_bands[{i}] mismatch: snapshot={lhs}, payload={rhs}"
        );
    }
    assert_eq!(snapshot.fft_availability, EngineAxisAvailability::Ready);
    assert_eq!(snapshot.bass, payload.bass);
    assert_eq!(snapshot.mid, payload.mid);
    assert_eq!(snapshot.treb, payload.treb);
    assert_eq!(snapshot.bass_att, payload.bass_att);
    assert_eq!(snapshot.mid_att, payload.mid_att);
    assert_eq!(snapshot.treb_att, payload.treb_att);
    assert_eq!(snapshot.degraded, payload.degraded);
}

#[test]
fn payload_json_round_trip_field_by_field_equality() {
    // ADR-019 §3 specifies the producer marshals the payload through
    // a Tauri event channel; that channel serialises to JSON. Pin
    // serde JSON round-trip identity so the wire shape stays stable
    // across the engine→Tauri→Svelte path. A future serde-tagged
    // attribute or rename would surface here.
    let payload = nonzero_payload();
    let json = serde_json::to_string(&payload).expect("serialize payload");
    let decoded: AudioUniformSnapshotPayload =
        serde_json::from_str(&json).expect("deserialize payload");

    assert_eq!(decoded.bpm, payload.bpm);
    assert_eq!(decoded.bpm_availability, payload.bpm_availability);
    assert_eq!(decoded.rms, payload.rms);
    assert_eq!(decoded.peak, payload.peak);
    for (i, (lhs, rhs)) in decoded
        .fft_bands
        .iter()
        .zip(payload.fft_bands.iter())
        .enumerate()
    {
        assert_eq!(
            lhs, rhs,
            "fft_bands[{i}] mismatch after JSON round-trip: decoded={lhs}, payload={rhs}"
        );
    }
    assert_eq!(decoded.fft_availability, payload.fft_availability);
    assert_eq!(decoded.bass, payload.bass);
    assert_eq!(decoded.mid, payload.mid);
    assert_eq!(decoded.treb, payload.treb);
    assert_eq!(decoded.bass_att, payload.bass_att);
    assert_eq!(decoded.mid_att, payload.mid_att);
    assert_eq!(decoded.treb_att, payload.treb_att);
    assert_eq!(decoded.degraded, payload.degraded);
    assert_eq!(decoded, payload);
}

#[test]
fn payload_default_round_trip_uses_zero_snapshot() {
    // The engine-side `AudioUniformSnapshot::default()` is the
    // canonical "no audio yet" baseline: the raw-energy axes read zero
    // (bpm=0, rms=0, peak=0, fft_bands=[0; 8]) but the MilkDrop-class
    // relative reactive bands centre on the neutral 1.0 per ADR-024. It
    // must round-trip through the payload to itself unchanged so the
    // consumer cannot observe a non-zero pre-mix frame.
    let snapshot = AudioUniformSnapshot::default();
    let payload: AudioUniformSnapshotPayload = snapshot.into();
    let back: AudioUniformSnapshot = payload.into();

    assert_eq!(back, snapshot);
    assert_eq!(payload.bpm, 0.0);
    assert_eq!(payload.bpm_availability, AudioAxisAvailability::Warming);
    assert_eq!(payload.rms, 0.0);
    assert_eq!(payload.peak, 0.0);
    assert_eq!(payload.fft_bands, [0.0_f32; 8]);
    assert_eq!(payload.fft_availability, AudioAxisAvailability::Warming);
    // Relative reactive bands centre on the neutral 1.0, not zero.
    assert_eq!(payload.bass, 1.0);
    assert_eq!(payload.mid, 1.0);
    assert_eq!(payload.treb, 1.0);
    assert_eq!(payload.bass_att, 1.0);
    assert_eq!(payload.mid_att, 1.0);
    assert_eq!(payload.treb_att, 1.0);
    assert!(!payload.degraded);
}

#[test]
fn payload_extreme_values_round_trip() {
    // Values at the engine clamp bounds round-trip preserved. Per
    // engine/audio/src/uniforms.rs the BPM clamp is `[50, 220]` and
    // RMS / peak / fft_bands are normalised to `[0, 1]`. The bridge
    // does NOT re-clamp at the boundary (clamp is a producer-side
    // invariant in the engine extractor); the round-trip is pure
    // value-preservation.
    let upper = AudioUniformSnapshot {
        bpm: 220.0,
        bpm_availability: EngineAxisAvailability::Ready,
        rms: 1.0,
        peak: 1.0,
        fft_bands: [1.0; 8],
        fft_availability: EngineAxisAvailability::Ready,
        // Reactive bands are unbounded above (a hit after silence spikes);
        // a large value pins that the bridge does not re-clamp them.
        bass: 8.0,
        mid: 6.0,
        treb: 4.0,
        bass_att: 3.0,
        mid_att: 2.5,
        treb_att: 2.0,
        degraded: false,
    };
    let lower = AudioUniformSnapshot {
        bpm: 50.0,
        bpm_availability: EngineAxisAvailability::Ready,
        rms: 0.0,
        peak: 0.0,
        fft_bands: [0.0; 8],
        fft_availability: EngineAxisAvailability::Ready,
        bass: 0.0,
        mid: 0.0,
        treb: 0.0,
        bass_att: 0.0,
        mid_att: 0.0,
        treb_att: 0.0,
        degraded: false,
    };

    for snapshot in [upper, lower] {
        let payload: AudioUniformSnapshotPayload = snapshot.into();
        let back: AudioUniformSnapshot = payload.into();
        assert_eq!(back, snapshot);
        assert_eq!(payload.bpm, snapshot.bpm);
        assert_eq!(payload.bpm_availability, AudioAxisAvailability::Ready);
        assert_eq!(payload.rms, snapshot.rms);
        assert_eq!(payload.peak, snapshot.peak);
        assert_eq!(payload.fft_bands, snapshot.fft_bands);
        assert_eq!(payload.fft_availability, AudioAxisAvailability::Ready);
        assert!(!payload.degraded);
    }
}

#[test]
fn ts_rs_exported_name_matches() {
    // The committed binding lives at
    // `src/lib/ipc/bindings/AudioUniformSnapshotPayload.ts`. ts-rs's
    // `name()` is the source of truth for the export filename;
    // an accidental rename of the Rust newtype (or a `#[ts(rename = ...)]`
    // attribute) would surface here before the frontend index drifts.
    assert_eq!(
        <AudioUniformSnapshotPayload as TS>::name(),
        "AudioUniformSnapshotPayload"
    );
}
