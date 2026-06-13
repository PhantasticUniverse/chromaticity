// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Wave 1c-α PR 1 scheduler-tick audio-uniform IPC wiring.
//!
//! Realises the producer side of the engine→UI audio-uniform IPC
//! bridge per `docs/adr/ADR-019-audio-uniform-ipc-bridge.md`. The
//! [`IpcEmit`] trait declares the scheduler-side seam invoked once
//! per [`super::Scheduler::tick`] AFTER the slice-3 PR 5 observability
//! emit: when configured AND the tick produced a fresh
//! [`chromaticity_audio::AudioUniformSnapshot`] (i.e. the slice-3 PR 3
//! audio-pull handler is configured AND ran successfully on this
//! tick), the trait fires with the same snapshot value the scheduler
//! stored on [`crate::state::State::latest_audio_snapshot`]. The
//! Tauri-event concrete adapter that implements this trait lives in
//! `app/shell/src/state/` (Wave 1c-α PR 2) — `engine/runtime` MUST
//! NOT depend on Tauri, so this module ships only the trait + test
//! stubs.
//!
//! ## Sub-module + builder + tick-step pattern
//!
//! Mirrors the slice-3 PR 5 ([`super::observability`]) / PR 3
//! ([`super::audio`]) / PR 4 ([`super::render`]) shape verbatim:
//!
//! - Module-level prose declaring the tick-ordering contract +
//!   thread-model rationale + the per-tick semantics the trait
//!   captures.
//! - A `pub trait` with a single `Send + Sync` bound and one method
//!   that the scheduler-tick path calls under an `if let Some(...)`
//!   guard against the optional handler field.
//! - In-source T1 drift pins co-located with the trait under a
//!   `cfg(test)` `scheduler_ipc_pins` module pinning the four
//!   invariants the Wave 1c-α corpus + the production Tauri adapter
//!   compose against.
//!
//! No production adapter ships here: the only `IpcEmit` impl in this
//! PR is a test stub. PR 2 lands the Tauri-event adapter under
//! `app/shell/src/state/` where the `tauri::AppHandle` dependency
//! lives.
//!
//! ## Tick-ordering contract
//!
//! [`super::Scheduler::tick`] invokes [`IpcEmit::emit_snapshot`]
//! AFTER:
//!
//! - the slice-3 PR 3 audio snapshot pull (which writes
//!   [`crate::state::State::latest_audio_snapshot`]),
//! - the [`crate::command_bus::InMemoryBus::commit_all_into`] drain,
//! - the `state.frame += 1` increment,
//! - the slice-3 PR 4 render dispatch,
//! - the slice-3 PR 5 observability emit.
//!
//! The IPC emit step therefore observes the post-tick state once
//! every other per-tick subsystem has fired. This ordering is load-
//! bearing: it lets the visualizer-side payload subscriber observe
//! the same canonical state-snapshot the render pipeline already
//! consumed for the same frame, so a Spectrum widget animating the
//! payload arrives at the same `state.frame` value the on-screen
//! pixels were produced for.
//!
//! ## Unconditional emit, but only when a snapshot exists
//!
//! ADR-019 §7 specifies the producer does not maintain a subscriber
//! count: emit fires unconditionally whether or not any Svelte
//! listener is attached. In this implementation "unconditional" is
//! gated on the existence of a snapshot to emit, NOT on subscriber
//! presence. Specifically the tick-step pattern is:
//!
//! ```text
//! if let Some(handler) = &self.ipc_handler {
//!     if let Some(snapshot) = state.latest_audio_snapshot {
//!         handler.emit_snapshot(snapshot);
//!     }
//! }
//! ```
//!
//! A scheduler configured with `with_ipc_emit` but NOT
//! `with_audio_pull` will never observe a fresh audio snapshot on
//! `state.latest_audio_snapshot` (the slice-3 PR 3 invariant: the
//! field stays `None` when no audio handler is configured) and the
//! emit step is therefore a no-op for that configuration. This
//! avoids emitting stale [`chromaticity_audio::AudioUniformSnapshot::default`]
//! zero snapshots at startup before the audio analysis thread has
//! produced its first reading — which, while harmless from a
//! correctness standpoint (zero is a valid snapshot value), would
//! pollute the visualizer with a spurious pre-mix frame the
//! operator never asked for.
//!
//! When the subsequent tick's audio pull provides a snapshot, the
//! IPC emit step fires with that snapshot. Subsequent ticks
//! continue emitting on every tick where a snapshot is present
//! regardless of whether the value changed across ticks (per
//! ADR-019 §6 the producer does not de-duplicate; the visualizer is
//! free to coalesce on the consumer side).
//!
//! ## Drop-frames-on-saturation
//!
//! ADR-019 §3 specifies the producer drops frames if a Tauri-event
//! buffer is full rather than blocking the scheduler tick. The
//! [`IpcEmit::emit_snapshot`] trait contract delegates this
//! behaviour to the implementation: a `&self` method returning `()`
//! captures the "fire-and-forget; never panic; never block more
//! than a bounded amount" semantics the production Tauri adapter
//! must honour. The scheduler does not introspect the result; a
//! future T1 widening may surface a `Dropped` outcome through
//! observability without changing this seam.
//!
//! ## Thread-model rationale
//!
//! Per ADR-006 the state-commit thread (where the scheduler runs) is
//! separate from any Tauri event-loop thread. `Send + Sync` on the
//! trait matches the
//! [`std::sync::Arc<tauri::AppHandle>`]-shaped value a production
//! adapter would naturally hold (PR 2 implementation detail);
//! `&self` matches the trait's no-mutate contract and lets the
//! scheduler share the handler across ticks without serialising on
//! a per-tick lock.

use chromaticity_audio::AudioUniformSnapshot;

/// Evidence: EV-AUDIO-UNIFORM-IPC (T1) — scheduler-tick audio-uniform
/// IPC emit baseline.
///
/// Scheduler-side adapter for emitting an
/// [`AudioUniformSnapshot`] once per tick to the Tauri-event channel
/// per `docs/adr/ADR-019-audio-uniform-ipc-bridge.md`.
///
/// Implemented by:
///
/// - The Wave 1c-α PR 2 production adapter under
///   `app/shell/src/state/` wrapping a
///   [`std::sync::Arc<tauri::AppHandle>`] (NOT in this PR — the
///   engine/runtime crate must not depend on Tauri).
/// - Test stubs that record the per-tick snapshot for assertion
///   (see the in-module T1 drift pins).
///
/// `Send + Sync` is required because the scheduler is owned on the
/// state-commit thread per ADR-006 §Decision but the Tauri event
/// dispatcher fans out to a separate thread; the trait bound matches
/// the [`std::sync::Arc<tauri::AppHandle>`]-shaped value a production
/// adapter would naturally hold.
pub trait IpcEmit: Send + Sync {
    /// Called once per [`super::Scheduler::tick`], AFTER every other
    /// per-tick subsystem (audio pull, command-bus drain, frame
    /// increment, render dispatch, observability emit) has fired
    /// AND only when a fresh
    /// [`chromaticity_audio::AudioUniformSnapshot`] is present on
    /// [`crate::state::State::latest_audio_snapshot`]. The handler
    /// receives the snapshot by value (it is `Copy`) so the
    /// implementation may marshal it into a Tauri-event payload
    /// without borrowing across the dispatch boundary.
    ///
    /// Implementations MUST NOT panic — the scheduler-tick path must
    /// continue advancing even when the Tauri-event channel has
    /// faulted. Implementations SHOULD NOT block: per ADR-019 §3 the
    /// producer drops frames on saturation rather than back-pressuring
    /// the scheduler.
    fn emit_snapshot(&self, snapshot: AudioUniformSnapshot);
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod scheduler_ipc_pins {
    //! Evidence: EV-AUDIO-UNIFORM-IPC (T1) — scheduler-tick audio-
    //! uniform IPC emit baseline.
    //!
    //! T1 per docs/43 §15.5: each test pins a state-machine
    //! invariant the [`super::IpcEmit`] / [`super::super::Scheduler`]
    //! composition must honour for the Wave 1c-α downstream surface
    //! (the Tauri-event adapter in PR 2 + the Svelte listener in
    //! PR 2) to compose correctly. EV-AUDIO-UNIFORM-IPC is registered
    //! in `docs/26` as `Planned`; the `Pass` transition lands in the
    //! Wave 1c-α closing docs PR after the consumer-side wiring
    //! demonstrates the end-to-end round trip.
    //!
    //! Invariants pinned here:
    //!
    //! - `tick_with_ipc_emit_invokes_handler_with_pulled_snapshot` —
    //!   when both an audio-pull handler AND an IPC-emit handler are
    //!   configured, the IPC handler observes the same snapshot the
    //!   audio pull produced for this tick (value-equal across the
    //!   four-field surface).
    //! - `tick_without_ipc_emit_is_noop` — a scheduler with `None`
    //!   IPC handler ticks successfully and the slice-3 PR 1
    //!   invariants hold verbatim (frame advances, no panic, audio
    //!   snapshot still stored on state per the slice-3 PR 3
    //!   contract).
    //! - `tick_emits_after_observability_emit` — observability emit
    //!   fires BEFORE IPC emit on the same tick. The recording
    //!   stubs share a Mutex-guarded sequence counter; the test
    //!   asserts the IPC stub's recorded sequence value is strictly
    //!   greater than the observability stub's.
    //! - `tick_with_ipc_emit_is_noop_when_no_audio_snapshot` — when
    //!   IPC emit is configured but audio pull is NOT, no snapshot
    //!   is present on `state.latest_audio_snapshot`, so the IPC
    //!   handler is never invoked. Documents the "unconditional emit
    //!   only when a snapshot exists" gating per the module-level
    //!   prose.

    use super::IpcEmit;
    use crate::command_bus::CommandBus;
    use crate::command_bus::{Command, ControlId, ControlValue, InMemoryBus, PresetId};
    use crate::scheduler::Scheduler;
    use crate::scheduler::audio::AudioPull;
    use crate::scheduler::observability::TickEmitter;
    use crate::state::State;
    use chromaticity_audio::{AudioAxisAvailability, AudioUniformSnapshot};
    use std::sync::{Arc, Mutex};

    /// Mock [`AudioPull`] returning a fixed snapshot for assertion.
    /// Mirrors the `RecordingAudioPull` shape in the sibling slice-3
    /// PR 3 audio-tick T1 pins.
    struct FixedAudioPull {
        snapshot: AudioUniformSnapshot,
    }

    impl AudioPull for FixedAudioPull {
        fn pull_snapshot(&self) -> AudioUniformSnapshot {
            self.snapshot
        }
    }

    /// Mock [`IpcEmit`] recording the per-tick snapshot AND a global
    /// sequence-counter value for ordering assertions.
    struct RecordingIpcEmit {
        observations: Mutex<Vec<AudioUniformSnapshot>>,
        sequence: Arc<Mutex<u64>>,
        recorded_sequence: Mutex<Vec<u64>>,
    }

    impl IpcEmit for RecordingIpcEmit {
        fn emit_snapshot(&self, snapshot: AudioUniformSnapshot) {
            // Bump the shared sequence counter and record it AND the
            // snapshot. The sequence counter exists so the
            // `tick_emits_after_observability_emit` ordering pin can
            // compare relative emit times across two stubs sharing
            // the same counter.
            let mut seq_guard = self.sequence.lock().unwrap();
            *seq_guard += 1;
            let stamped = *seq_guard;
            drop(seq_guard);
            self.observations.lock().unwrap().push(snapshot);
            self.recorded_sequence.lock().unwrap().push(stamped);
        }
    }

    /// `Arc`-wrapping adapter that lets a test reach the recording
    /// emitter through the `Box<dyn IpcEmit>` indirection. Mirrors
    /// the `ArcRecorder` shape in the sibling observability /
    /// audio-tick T1 pins.
    struct ArcRecorder(Arc<RecordingIpcEmit>);

    impl IpcEmit for ArcRecorder {
        fn emit_snapshot(&self, snapshot: AudioUniformSnapshot) {
            self.0.emit_snapshot(snapshot);
        }
    }

    /// Mock [`TickEmitter`] recording its sequence-counter value.
    /// Used only by the ordering pin.
    struct RecordingObsEmit {
        sequence: Arc<Mutex<u64>>,
        recorded_sequence: Mutex<Vec<u64>>,
    }

    impl TickEmitter for RecordingObsEmit {
        fn emit_tick_event(&self, _frame: u64, _events_committed: usize) {
            let mut seq_guard = self.sequence.lock().unwrap();
            *seq_guard += 1;
            let stamped = *seq_guard;
            drop(seq_guard);
            self.recorded_sequence.lock().unwrap().push(stamped);
        }
    }

    struct ArcObsRecorder(Arc<RecordingObsEmit>);

    impl TickEmitter for ArcObsRecorder {
        fn emit_tick_event(&self, frame: u64, events_committed: usize) {
            self.0.emit_tick_event(frame, events_committed);
        }
    }

    fn nonzero_snapshot() -> AudioUniformSnapshot {
        AudioUniformSnapshot {
            bpm: 128.0,
            bpm_availability: AudioAxisAvailability::Ready,
            rms: 0.4,
            peak: 0.9,
            fft_bands: [0.05, 0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75],
            fft_availability: AudioAxisAvailability::Ready,
            // ADR-024 reactive bands default to neutral 1.0; these IPC
            // pins exercise emit transport, not band values.
            ..Default::default()
        }
    }

    fn fresh_recorder() -> Arc<RecordingIpcEmit> {
        Arc::new(RecordingIpcEmit {
            observations: Mutex::new(Vec::new()),
            sequence: Arc::new(Mutex::new(0)),
            recorded_sequence: Mutex::new(Vec::new()),
        })
    }

    #[test]
    fn tick_with_ipc_emit_invokes_handler_with_pulled_snapshot() {
        let target = nonzero_snapshot();
        let recorder = fresh_recorder();
        let mut scheduler = Scheduler::new()
            .with_audio_pull(Box::new(FixedAudioPull { snapshot: target }))
            .with_ipc_emit(Box::new(ArcRecorder(recorder.clone())));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        // Dispatch a couple of commands so the tick has a non-trivial
        // event count too, pinning that the IPC step does not stall
        // on a non-empty bus drain.
        bus.dispatch(Command::PresetRecall { id: PresetId(7) })
            .expect("dispatch must succeed");
        bus.dispatch(Command::ControlSet {
            id: ControlId(2),
            value: ControlValue(0.6),
        })
        .expect("dispatch must succeed");

        let report = scheduler.tick(&mut state, &bus).expect("tick must succeed");
        assert_eq!(report.frame, 1);
        assert_eq!(report.events_committed, 2);
        assert_eq!(state.latest_audio_snapshot(), Some(&target));

        let observations = recorder.observations.lock().unwrap().clone();
        assert_eq!(observations.len(), 1);
        // Field-by-field equality across the 11-field surface:
        // 1 × bpm + 1 × rms + 1 × peak + 8 × fft_bands.
        assert_eq!(observations[0].bpm, target.bpm);
        assert_eq!(observations[0].rms, target.rms);
        assert_eq!(observations[0].peak, target.peak);
        assert_eq!(observations[0].fft_bands, target.fft_bands);
        assert_eq!(observations[0], target);
    }

    #[test]
    fn tick_without_ipc_emit_is_noop() {
        // Slice-3 PR 1 + PR 3 invariants must hold verbatim when no
        // IPC handler is configured: tick succeeds, frame advances,
        // and the audio snapshot is still stored on state per the
        // slice-3 PR 3 contract. This pin defends the "additive,
        // not replacing" property of the Wave 1c-α tick step.
        let target = nonzero_snapshot();
        let mut scheduler =
            Scheduler::new().with_audio_pull(Box::new(FixedAudioPull { snapshot: target }));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        let report = scheduler
            .tick(&mut state, &bus)
            .expect("tick must succeed without ipc emit");
        assert_eq!(report.frame, 1);
        assert_eq!(report.events_committed, 0);
        assert_eq!(state.frame(), 1);
        assert_eq!(state.latest_audio_snapshot(), Some(&target));
    }

    #[test]
    fn tick_emits_after_observability_emit() {
        // Observability emit MUST fire BEFORE the IPC emit on the
        // same tick. Both stubs share a sequence counter; the
        // recorded sequence values pin the relative ordering.
        let target = nonzero_snapshot();
        let shared_sequence: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
        let obs_recorder = Arc::new(RecordingObsEmit {
            sequence: shared_sequence.clone(),
            recorded_sequence: Mutex::new(Vec::new()),
        });
        let ipc_recorder = Arc::new(RecordingIpcEmit {
            observations: Mutex::new(Vec::new()),
            sequence: shared_sequence.clone(),
            recorded_sequence: Mutex::new(Vec::new()),
        });
        let mut scheduler = Scheduler::new()
            .with_audio_pull(Box::new(FixedAudioPull { snapshot: target }))
            .with_metric_emitter(Box::new(ArcObsRecorder(obs_recorder.clone())))
            .with_ipc_emit(Box::new(ArcRecorder(ipc_recorder.clone())));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        let _ = scheduler.tick(&mut state, &bus).expect("tick must succeed");

        let obs_seq = obs_recorder.recorded_sequence.lock().unwrap().clone();
        let ipc_seq = ipc_recorder.recorded_sequence.lock().unwrap().clone();
        assert_eq!(obs_seq.len(), 1);
        assert_eq!(ipc_seq.len(), 1);
        assert!(
            ipc_seq[0] > obs_seq[0],
            "ipc emit (seq={}) must fire AFTER observability emit (seq={})",
            ipc_seq[0],
            obs_seq[0],
        );
    }

    #[test]
    fn tick_with_ipc_emit_is_noop_when_no_audio_snapshot() {
        // ADR-019 §7 says "emit unconditional", but the Wave 1c-α
        // implementation gates emit on snapshot presence to avoid
        // emitting stale `AudioUniformSnapshot::default()` zero
        // snapshots at startup before any audio handler has run.
        // Configure IPC emit WITHOUT audio pull; tick; assert the
        // IPC handler is never invoked.
        let recorder = fresh_recorder();
        let mut scheduler = Scheduler::new().with_ipc_emit(Box::new(ArcRecorder(recorder.clone())));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        let report = scheduler.tick(&mut state, &bus).expect("tick must succeed");
        assert_eq!(report.frame, 1);
        assert_eq!(state.latest_audio_snapshot(), None);

        let observations = recorder.observations.lock().unwrap().clone();
        assert!(
            observations.is_empty(),
            "ipc emit must not fire when no audio snapshot is present; got {observations:?}"
        );
    }
}
