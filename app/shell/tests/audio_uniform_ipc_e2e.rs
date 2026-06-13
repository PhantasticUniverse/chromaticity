// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Cross-seam end-to-end wire test for the audio-uniform IPC bridge.
//!
//! Bootstrap PR 3 of the post-Wave-1c-α production audio bootstrap
//! wave drops the row 83 (`EV-AUDIO-UNIFORM-IPC`) deferred-axis
//! "live end-to-end producer→Tauri-emit→consumer wiring" by
//! standing up a `tauri::test::mock_app()` harness that registers a
//! [`tauri::Listener`] against the production
//! [`chromaticity_app_lib::state::ipc_emit::AUDIO_UNIFORM_SNAPSHOT_EVENT`]
//! channel, drives a Tauri-side emit with a known
//! [`AudioUniformSnapshot`], and asserts the listener receives a
//! payload field-by-field equal to
//! [`AudioUniformSnapshotPayload::from(snapshot)`].
//!
//! The test calls the real
//! [`chromaticity_app_lib::state::ipc_emit::TauriIpcEmitter`]
//! adapter against Tauri's `MockRuntime`, so the payload conversion,
//! event name, and `Emitter::emit` call all execute through the same
//! production `IpcEmit` implementation used by the app shell.
//!
//! Wave 1c-α's `app/ui-bridge/tests/audio_uniform_payload_round_trip.rs`
//! T2 corpus covers the pure ts-rs payload shape; this test extends
//! the T2 corpus with the cross-seam wire-level coverage that the
//! previous wave's row 83 Description column flagged as deferred.
//!
//! Evidence: EV-AUDIO-UNIFORM-IPC (T2)

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chromaticity_app_lib::state::ipc_emit::{AUDIO_UNIFORM_SNAPSHOT_EVENT, TauriIpcEmitter};
use chromaticity_audio::{AudioAxisAvailability, AudioUniformSnapshot};
use chromaticity_runtime::scheduler::ipc::IpcEmit;
use chromaticity_ui_bridge::AudioUniformSnapshotPayload;
use tauri::Listener;

/// Spin until either the predicate is satisfied or the deadline
/// elapses. Tauri delivers events asynchronously on its own
/// dispatcher thread, so the assertion side polls rather than
/// blocking on a channel — keeps the test failure mode "deadline
/// exceeded" rather than "blocked indefinitely".
fn wait_until(deadline: Duration, predicate: impl Fn() -> bool) -> bool {
    let started = Instant::now();
    while started.elapsed() < deadline {
        if predicate() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    predicate()
}

#[test]
fn producer_to_listener_round_trip_preserves_payload_identity() {
    let app = tauri::test::mock_app();
    let received: Arc<Mutex<Vec<AudioUniformSnapshotPayload>>> = Arc::new(Mutex::new(Vec::new()));
    let received_handle = Arc::clone(&received);

    app.listen(AUDIO_UNIFORM_SNAPSHOT_EVENT, move |event| {
        let payload: AudioUniformSnapshotPayload = serde_json::from_str(event.payload()).unwrap();
        received_handle.lock().unwrap().push(payload);
    });

    let snapshot = AudioUniformSnapshot {
        bpm: 132.5,
        bpm_availability: AudioAxisAvailability::Ready,
        rms: 0.42,
        peak: 0.88,
        fft_bands: [0.05, 0.10, 0.20, 0.35, 0.50, 0.65, 0.80, 0.95],
        fft_availability: AudioAxisAvailability::Ready,
        // Relative reactive bands (ADR-024) ride the round-trip at their
        // neutral default; this test pins delivery, not band mapping (that
        // is the ui-bridge round-trip corpus).
        ..Default::default()
    };
    let emitter = TauriIpcEmitter::new(app.handle().clone());
    emitter.emit_snapshot(snapshot);

    assert!(
        wait_until(Duration::from_secs(2), || {
            received.lock().unwrap().len() == 1
        }),
        "listener did not receive emitted snapshot within deadline"
    );

    let received = received.lock().unwrap();
    assert_eq!(received.len(), 1);
    let expected = AudioUniformSnapshotPayload::from(snapshot);
    assert_eq!(received[0].bpm, expected.bpm);
    assert_eq!(received[0].rms, expected.rms);
    assert_eq!(received[0].peak, expected.peak);
    assert_eq!(received[0].fft_bands, expected.fft_bands);
}

#[test]
fn multiple_emits_deliver_in_order() {
    let app = tauri::test::mock_app();
    let received: Arc<Mutex<Vec<AudioUniformSnapshotPayload>>> = Arc::new(Mutex::new(Vec::new()));
    let received_handle = Arc::clone(&received);

    app.listen(AUDIO_UNIFORM_SNAPSHOT_EVENT, move |event| {
        let payload: AudioUniformSnapshotPayload = serde_json::from_str(event.payload()).unwrap();
        received_handle.lock().unwrap().push(payload);
    });

    let emitter = TauriIpcEmitter::new(app.handle().clone());
    let snapshots = [
        AudioUniformSnapshot {
            bpm: 100.0,
            bpm_availability: AudioAxisAvailability::Ready,
            rms: 0.10,
            peak: 0.20,
            fft_bands: [0.0; 8],
            fft_availability: AudioAxisAvailability::Ready,
            ..Default::default()
        },
        AudioUniformSnapshot {
            bpm: 110.0,
            bpm_availability: AudioAxisAvailability::Ready,
            rms: 0.20,
            peak: 0.30,
            fft_bands: [0.1; 8],
            fft_availability: AudioAxisAvailability::Ready,
            ..Default::default()
        },
        AudioUniformSnapshot {
            bpm: 120.0,
            bpm_availability: AudioAxisAvailability::Ready,
            rms: 0.30,
            peak: 0.40,
            fft_bands: [0.2; 8],
            fft_availability: AudioAxisAvailability::Ready,
            ..Default::default()
        },
    ];
    for snapshot in snapshots {
        emitter.emit_snapshot(snapshot);
    }

    assert!(
        wait_until(Duration::from_secs(2), || {
            received.lock().unwrap().len() == 3
        }),
        "listener did not receive all three snapshots within deadline"
    );

    let received = received.lock().unwrap();
    assert_eq!(received.len(), 3);
    for (idx, expected) in snapshots.iter().enumerate() {
        assert_eq!(received[idx].bpm, expected.bpm, "bpm mismatch at idx {idx}");
        assert_eq!(received[idx].rms, expected.rms, "rms mismatch at idx {idx}");
        assert_eq!(
            received[idx].peak, expected.peak,
            "peak mismatch at idx {idx}"
        );
        assert_eq!(
            received[idx].fft_bands, expected.fft_bands,
            "fft_bands mismatch at idx {idx}"
        );
    }
}

#[test]
fn emit_after_listener_unlisten_does_not_panic() {
    // ADR-019 §4 silent-drop posture: with no live subscriber the
    // producer continues advancing the scheduler tick. This pin
    // exercises the drop-frames-on-saturation kin path — listener
    // detached, producer keeps emitting, no panic, no error
    // surfaced into the scheduler tick.
    let app = tauri::test::mock_app();
    let received: Arc<Mutex<Vec<AudioUniformSnapshotPayload>>> = Arc::new(Mutex::new(Vec::new()));
    let received_handle = Arc::clone(&received);

    let listener_id = app.listen(AUDIO_UNIFORM_SNAPSHOT_EVENT, move |event| {
        let payload: AudioUniformSnapshotPayload = serde_json::from_str(event.payload()).unwrap();
        received_handle.lock().unwrap().push(payload);
    });

    let emitter = TauriIpcEmitter::new(app.handle().clone());

    // Drop the listener before any emit fires.
    app.unlisten(listener_id);

    // Emit several snapshots; none should panic and the previously
    // detached listener must not observe any of them.
    let snapshot = AudioUniformSnapshot::default();
    for _ in 0..5 {
        emitter.emit_snapshot(snapshot);
    }

    // Tauri's emit is best-effort; give the dispatcher a chance to
    // confirm the no-listener path delivers no payload to the
    // detached subscriber.
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(
        received.lock().unwrap().len(),
        0,
        "detached listener should not have observed any snapshots"
    );
}
