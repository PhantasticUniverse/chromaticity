// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Tauri concrete [`IpcEmit`] adapter for the audio-uniform IPC bridge.
//!
//! Wave 1c-α PR 1 (`#339`) introduced the
//! [`chromaticity_runtime::scheduler::ipc::IpcEmit`] trait but left
//! the concrete production adapter unimplemented because
//! `engine/runtime` MUST NOT depend on Tauri. This module ships the
//! Tauri-event adapter that wraps a [`tauri::AppHandle`] and emits
//! the [`AudioUniformSnapshotPayload`] on every scheduler tick where
//! a fresh [`AudioUniformSnapshot`] is present on
//! [`chromaticity_runtime::state::State::latest_audio_snapshot`].
//!
//! ## Emit semantics per ADR-019
//!
//! - **Emit-only, no subscriber-count gate.** ADR-019 §7 requires the
//!   producer to fire on every applicable tick whether or not any
//!   Svelte listener is attached. Tauri's
//!   [`tauri::Emitter::emit`] honours this directly: with zero
//!   listeners the call returns `Ok(())` and the event is dropped on
//!   the consumer side.
//! - **Silent drop on saturation.** ADR-019 §3 + §4 specify the
//!   producer drops frames if the Tauri event queue saturates rather
//!   than back-pressuring the scheduler tick. The
//!   [`tauri::Emitter::emit`] result is intentionally ignored: a
//!   future T1 widening may surface a `Dropped` outcome through
//!   observability without changing this seam.
//! - **No panic.** [`IpcEmit::emit_snapshot`]'s contract requires
//!   implementations not to panic; the silent drop above is the
//!   panic-free posture.
//!
//! ## Threading
//!
//! The state-commit thread (where the scheduler-tick runner lives,
//! per ADR-006) calls [`Self::emit_snapshot`]; Tauri's emitter is
//! [`Send + Sync`] and routes the event onto its own dispatcher
//! thread. The trait's `Send + Sync` bound therefore matches.
//!
//! Evidence: EV-AUDIO-UNIFORM-IPC (T1) — Tauri-side adapter T1 pins
//! co-located with the implementation. The cross-seam payload-shape
//! contract is pinned T1 by `engine/runtime/src/scheduler/ipc.rs`
//! and T2 by `app/ui-bridge/tests/audio_uniform_payload_round_trip.rs`;
//! this module pins the Tauri-adapter side of the composition.

use chromaticity_audio::AudioUniformSnapshot;
use chromaticity_runtime::scheduler::ipc::IpcEmit;
use chromaticity_ui_bridge::AudioUniformSnapshotPayload;
use tauri::{AppHandle, Emitter};

/// Event name on the Tauri event channel. The Svelte listener wired
/// in Wave 1c-α PR 2 (`src/lib/stores/audioSnapshot.svelte.ts`) calls
/// `listen('audio_uniform_snapshot_changed', ...)` against this
/// literal; keeping the constant in one place pins the contract.
pub const AUDIO_UNIFORM_SNAPSHOT_EVENT: &str = "audio_uniform_snapshot_changed";

/// Tauri-event concrete adapter for
/// [`chromaticity_runtime::scheduler::ipc::IpcEmit`].
///
/// Holds an owned [`AppHandle`] (cheap to clone — internally an
/// `Arc`) and emits the
/// [`chromaticity_audio::AudioUniformSnapshot`] payload via
/// [`tauri::Emitter::emit`] on every scheduler tick where a fresh
/// snapshot is present.
pub struct TauriIpcEmitter {
    app: AppHandle,
}

impl TauriIpcEmitter {
    /// Wrap a [`tauri::AppHandle`] for use with
    /// [`chromaticity_runtime::scheduler::Scheduler::with_ipc_emit`].
    #[must_use]
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl IpcEmit for TauriIpcEmitter {
    fn emit_snapshot(&self, snapshot: AudioUniformSnapshot) {
        let payload = AudioUniformSnapshotPayload::from(snapshot);
        // Silent drop on emit failure per ADR-019 §3 + §4. Tauri's
        // emit can fail if the event queue is full or the app is
        // shutting down; neither failure mode warrants a panic and
        // the scheduler tick must continue advancing regardless.
        let _ = self.app.emit(AUDIO_UNIFORM_SNAPSHOT_EVENT, &payload);
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod ipc_emit_pins {
    //! Evidence: EV-AUDIO-UNIFORM-IPC (T1) — Tauri-side adapter
    //! drift pins.
    //!
    //! T1 per `docs/43 §15.5`: each test pins a documented
    //! invariant the [`super::TauriIpcEmitter`] must honour for the
    //! production-bootstrap wave's downstream Svelte listener
    //! (`src/lib/stores/audioSnapshot.svelte.ts`) to compose
    //! correctly. The Tauri emit path itself is hard to unit-test
    //! without a Tauri app context — that scope belongs to the
    //! production-bootstrap wave's PR 3 e2e harness — so the pins
    //! here cover the panic-free / payload-conversion contracts
    //! that don't require a live `AppHandle`.
    //!
    //! Coverage:
    //! - `payload_conversion_preserves_field_by_field_equality`
    //!   pins the bidirectional [`AudioUniformSnapshotPayload`] /
    //!   [`AudioUniformSnapshot`] mapping the trait method relies
    //!   on, ensuring the 11-field surface (1 × bpm + 1 × rms + 1 ×
    //!   peak + 8 × fft_bands) crosses the conversion edge
    //!   intact.
    //! - `event_name_constant_matches_consumer_listener` pins the
    //!   Svelte-side listener contract (the consumer listens for
    //!   `'audio_uniform_snapshot_changed'`); a typo here is a
    //!   silent end-to-end break.

    use super::AUDIO_UNIFORM_SNAPSHOT_EVENT;
    use chromaticity_audio::AudioUniformSnapshot;
    use chromaticity_ui_bridge::AudioUniformSnapshotPayload;

    #[test]
    fn payload_conversion_preserves_field_by_field_equality() {
        let snapshot = AudioUniformSnapshot {
            bpm: 145.0,
            rms: 0.7,
            peak: 0.9,
            fft_bands: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        };
        let payload = AudioUniformSnapshotPayload::from(snapshot);
        assert_eq!(payload.bpm, snapshot.bpm);
        assert_eq!(payload.rms, snapshot.rms);
        assert_eq!(payload.peak, snapshot.peak);
        assert_eq!(payload.fft_bands, snapshot.fft_bands);
        // Reverse direction round-trip preserves the snapshot too;
        // mirrors the producer-side `From` symmetry the cross-seam
        // contract relies on.
        let back = AudioUniformSnapshot::from(payload);
        assert_eq!(back, snapshot);
    }

    #[test]
    fn event_name_constant_matches_consumer_listener() {
        // The Svelte consumer in `src/lib/stores/audioSnapshot.svelte.ts`
        // calls `listen('audio_uniform_snapshot_changed', ...)`. A
        // drift in the event name is a silent break that the e2e
        // harness in Bootstrap PR 3 catches end-to-end; this T1
        // pin catches it at unit-test time.
        assert_eq!(
            AUDIO_UNIFORM_SNAPSHOT_EVENT,
            "audio_uniform_snapshot_changed"
        );
    }

    #[test]
    fn payload_conversion_preserves_default_snapshot() {
        // Pre-first-write snapshot is the
        // `AudioUniformSnapshot::default()` zero value; the producer
        // gates emit on snapshot presence (per the `engine/runtime`
        // `IpcEmit` module-level prose) so this path is reached only
        // once a real snapshot lands. Pin the conversion still works
        // for the zero value as a defensive boundary check.
        let snapshot = AudioUniformSnapshot::default();
        let payload = AudioUniformSnapshotPayload::from(snapshot);
        assert_eq!(payload.bpm, 0.0);
        assert_eq!(payload.rms, 0.0);
        assert_eq!(payload.peak, 0.0);
        for band in payload.fft_bands {
            assert_eq!(band, 0.0);
        }
    }
}
