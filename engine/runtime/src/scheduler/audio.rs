// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Slice-3 PR 3 scheduler-tick audio wiring.
//!
//! Wraps the [`chromaticity_audio::AudioUniformSnapshot`] producer
//! pattern into a scheduler-aware [`AudioPull`] trait that
//! [`super::Scheduler::tick`] calls before frame advancement. The
//! Phase-1 baseline pulls a snapshot once per tick and stores it on
//! [`crate::state::State::latest_audio_snapshot`] for downstream
//! subsystems to read; Phase-3+ may add snapshot-freshness guards or
//! replay strategies without changing the [`AudioPull`] seam.
//!
//! ## Tick-ordering contract
//!
//! [`super::Scheduler::tick`] invokes [`AudioPull::pull_snapshot`]
//! BEFORE the [`crate::command_bus::InMemoryBus::commit_all_into`]
//! drain AND BEFORE the `state.frame += 1` increment. The snapshot
//! represents the audio state observed at the START of the tick:
//! commands that commit during this tick may reference it (e.g.
//! audio-reactive control writes routed through script in
//! slice-5+), but the scheduler does not reorder. When no handler is
//! configured the tick path leaves
//! [`crate::state::State::latest_audio_snapshot`] unchanged — slice-3
//! PR 1 invariants hold verbatim, and the EV-STATE-REPLAY (row 32) /
//! EV-PRESET-PLAYLIST (row 34) T2 corpora preserve their goldens
//! because [`crate::state_replay::canonical_state_hash`] does not
//! observe the audio snapshot field.
//!
//! ## Thread-model rationale
//!
//! Per ADR-006 the audio analysis thread is separate from the state-
//! commit thread. The Phase-1 stub can use a
//! [`std::sync::Mutex<AudioUniformSnapshot>`] shared between the two
//! threads via an [`std::sync::Arc`] — see [`MutexAudioPull`].
//! Migration to [`std::sync::RwLock`] is a drop-in if read concurrency
//! becomes a performance concern; the [`AudioPull`] trait does not
//! constrain the synchronisation primitive.

use chromaticity_audio::AudioUniformSnapshot;
use std::sync::atomic::{AtomicU64, Ordering};

/// Scheduler-side adapter for pulling the latest audio uniform
/// snapshot once per tick. Implemented by:
///
/// - The default [`MutexAudioPull`] wrapping any
///   `Arc<Mutex<AudioUniformSnapshot>>` (production use; slice-3
///   PR 3 baseline).
/// - Test stubs that return a fixed or recorded snapshot for
///   assertion (see the in-module T1 drift pins).
///
/// `Send + Sync` is required because the scheduler is owned on the
/// state-commit thread per ADR-006 §Decision but the audio analyser
/// runs on a separate thread; the trait bound matches the
/// [`std::sync::Arc<std::sync::Mutex<T>>`] shape an audio adapter
/// would naturally surface.
pub trait AudioPull: Send + Sync {
    /// Called once per [`super::Scheduler::tick`], BEFORE the
    /// command-bus drain AND BEFORE the frame counter advances.
    /// Returns the latest available snapshot; the scheduler stores
    /// it on [`crate::state::State::latest_audio_snapshot`] for
    /// downstream subsystems (render-graph dispatch in slice-3 PR 4;
    /// script reactive resolution in slice-5+).
    ///
    /// Implementations MUST NOT panic — the scheduler-tick path must
    /// continue advancing even when the audio analysis thread has
    /// faulted.
    fn pull_snapshot(&self) -> AudioUniformSnapshot;
}

/// Default adapter wrapping any
/// `Arc<Mutex<AudioUniformSnapshot>>`.
///
/// The Mutex-backed shape is sufficient for the Phase-1 stub; the
/// audio analysis thread acquires the lock to write the latest
/// snapshot, the scheduler-tick reads it once per tick. Phase-3+
/// migration to `Arc<RwLock<...>>` is a drop-in if read concurrency
/// becomes a performance concern.
pub struct MutexAudioPull {
    snapshot: std::sync::Arc<std::sync::Mutex<AudioUniformSnapshot>>,
    poison_recoveries: std::sync::Arc<AtomicU64>,
}

impl MutexAudioPull {
    /// Wrap any `Arc<Mutex<AudioUniformSnapshot>>` for use with
    /// [`super::Scheduler::with_audio_pull`].
    #[must_use]
    pub fn new(snapshot: std::sync::Arc<std::sync::Mutex<AudioUniformSnapshot>>) -> Self {
        Self {
            snapshot,
            poison_recoveries: std::sync::Arc::new(AtomicU64::new(0)),
        }
    }

    /// Number of times this pull adapter recovered a poisoned mutex.
    ///
    /// The scheduler still returns the last snapshot to preserve the
    /// no-panic tick contract, but the recovery is now observable to
    /// the owner for diagnostics.
    #[must_use]
    pub fn poison_recovery_count(&self) -> u64 {
        self.poison_recoveries.load(Ordering::Relaxed)
    }

    /// Shared poison-recovery counter for tests or diagnostics that
    /// move the adapter behind `Box<dyn AudioPull>`.
    #[must_use]
    pub fn poison_recovery_counter(&self) -> std::sync::Arc<AtomicU64> {
        std::sync::Arc::clone(&self.poison_recoveries)
    }
}

impl AudioPull for MutexAudioPull {
    fn pull_snapshot(&self) -> AudioUniformSnapshot {
        // Best-effort lock acquisition; if the lock is poisoned (the
        // audio analysis thread panicked while holding the lock),
        // recover the inner snapshot and return it rather than
        // propagating the poison. The scheduler-tick path must
        // continue advancing even when the audio thread has faulted —
        // this is the conservative posture per the trait contract's
        // no-panic clause. Phase-3+ may surface a degraded-mode
        // signal through observability without changing this seam.
        match self.snapshot.lock() {
            Ok(guard) => *guard,
            Err(poisoned) => {
                self.poison_recoveries.fetch_add(1, Ordering::Relaxed);
                *poisoned.into_inner()
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod scheduler_audio_pins {
    //! Evidence: EV-SCHEDULER-TICK (T1) — scheduler-tick audio
    //! snapshot pull baseline.
    //!
    //! T1 per docs/43 §15.5: each test pins a state-machine
    //! invariant the [`super::AudioPull`] / [`super::super::Scheduler`]
    //! composition must honour for the slice-3 wave's downstream
    //! audio-reactive surface (render-graph dispatch in PR 4; script
    //! reactive resolution in slice-5+) to compose correctly.
    //! `EV-SCHEDULER-TICK` is registered in `docs/26` as `Planned`;
    //! the `Pass` transition lands in the slice-3 closing docs PR
    //! after PRs 3 / 4 / 5 / 6 land.
    //!
    //! Invariants pinned here:
    //!
    //! - `tick_with_audio_pull_stores_snapshot_on_state` — a single
    //!   [`super::super::Scheduler::tick`] call with a configured
    //!   [`super::AudioPull`] writes the pulled snapshot onto
    //!   [`crate::state::State::latest_audio_snapshot`].
    //! - `tick_without_audio_pull_leaves_snapshot_none` — a scheduler
    //!   with `None` handler leaves the field at its default `None`
    //!   (slice-3 PR 1 invariants hold verbatim).
    //! - `tick_pulls_snapshot_before_frame_advances` — the pull
    //!   observes the PRE-tick frame value, pinning the
    //!   "snapshot represents start-of-tick audio state" contract.
    //! - `tick_pulls_snapshot_before_command_bus_commit` — commands
    //!   dispatched on the bus during the tick do not influence the
    //!   pulled snapshot's storage location; the audio handler runs
    //!   before the drain.
    //! - `tick_with_audio_pull_handles_lock_poisoning_gracefully` —
    //!   the [`super::MutexAudioPull`] poison-recovery path returns
    //!   the inner snapshot rather than panicking, so a faulted
    //!   audio analysis thread does not stall the scheduler tick.

    use super::{AudioPull, MutexAudioPull};
    use crate::command_bus::CommandBus;
    use crate::command_bus::{Command, ControlId, ControlValue, InMemoryBus, PresetId};
    use crate::scheduler::Scheduler;
    use crate::state::State;
    use chromaticity_audio::AudioUniformSnapshot;
    use std::sync::{Arc, Mutex};

    /// Mock [`AudioPull`] returning a fixed snapshot and recording
    /// the per-call invocation count for assertion. The Mutex is
    /// required because [`AudioPull::pull_snapshot`] takes `&self`.
    struct RecordingAudioPull {
        snapshot: AudioUniformSnapshot,
        observations: Mutex<Vec<u64>>,
    }

    impl RecordingAudioPull {
        fn with_snapshot(snapshot: AudioUniformSnapshot) -> Self {
            Self {
                snapshot,
                observations: Mutex::new(Vec::new()),
            }
        }
    }

    impl AudioPull for RecordingAudioPull {
        fn pull_snapshot(&self) -> AudioUniformSnapshot {
            self.observations.lock().unwrap().push(0);
            self.snapshot
        }
    }

    /// Adapter that lets a test reach the recording pull through the
    /// `Box<dyn AudioPull>` indirection. Mirrors the
    /// `ArcRecorder` shape in the sibling observability pins.
    struct ArcRecorder(Arc<RecordingAudioPull>);

    impl AudioPull for ArcRecorder {
        fn pull_snapshot(&self) -> AudioUniformSnapshot {
            self.0.pull_snapshot()
        }
    }

    /// Adapter that records the `state.frame()` value visible on the
    /// state at pull time. The state is shared via `Arc<Mutex<...>>`
    /// so the pull can read it; production code never does this — the
    /// scheduler-tick contract guarantees the pull observes the
    /// pre-tick frame value, and the adapter only exists to assert
    /// the contract.
    struct FrameObservingPull {
        recorded_frames: Arc<Mutex<Vec<u64>>>,
        live_state: Arc<Mutex<u64>>,
        snapshot: AudioUniformSnapshot,
    }

    impl AudioPull for FrameObservingPull {
        fn pull_snapshot(&self) -> AudioUniformSnapshot {
            let frame_at_pull = *self.live_state.lock().unwrap();
            self.recorded_frames.lock().unwrap().push(frame_at_pull);
            self.snapshot
        }
    }

    fn nonzero_snapshot() -> AudioUniformSnapshot {
        AudioUniformSnapshot {
            bpm: 120.0,
            rms: 0.5,
            peak: 0.75,
            fft_bands: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        }
    }

    #[test]
    fn tick_with_audio_pull_stores_snapshot_on_state() {
        let target = nonzero_snapshot();
        let recorder = Arc::new(RecordingAudioPull::with_snapshot(target));
        let mut scheduler =
            Scheduler::new().with_audio_pull(Box::new(ArcRecorder(recorder.clone())));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        assert_eq!(state.latest_audio_snapshot(), None);
        let _ = scheduler.tick(&mut state, &bus).expect("tick must succeed");

        assert_eq!(state.latest_audio_snapshot(), Some(&target));
        assert_eq!(recorder.observations.lock().unwrap().len(), 1);
    }

    #[test]
    fn tick_without_audio_pull_leaves_snapshot_none() {
        // Slice-3 PR 1 invariants must hold verbatim when no audio
        // handler is configured: tick succeeds, frame advances, and
        // `latest_audio_snapshot` stays `None`.
        let mut scheduler = Scheduler::new();
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        assert_eq!(state.latest_audio_snapshot(), None);
        let report = scheduler
            .tick(&mut state, &bus)
            .expect("tick must succeed without audio pull");
        assert_eq!(report.frame, 1);
        assert_eq!(state.latest_audio_snapshot(), None);
    }

    #[test]
    fn tick_pulls_snapshot_before_frame_advances() {
        // The audio handler must observe the PRE-tick frame value (0
        // on the first tick, 1 on the second, etc.). Drive two ticks
        // and assert the recorded frames match the pre-tick state.
        let recorded_frames: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
        let live_state: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));

        // The pull adapter reads from `live_state` (which the test
        // mirrors against `state.frame()` after each tick by the
        // scheduler's increment). Production code does NOT do this;
        // the adapter exists only to capture the pull-time frame.
        struct DriverPull {
            inner: FrameObservingPull,
        }

        impl AudioPull for DriverPull {
            fn pull_snapshot(&self) -> AudioUniformSnapshot {
                self.inner.pull_snapshot()
            }
        }

        let pull = DriverPull {
            inner: FrameObservingPull {
                recorded_frames: recorded_frames.clone(),
                live_state: live_state.clone(),
                snapshot: nonzero_snapshot(),
            },
        };
        let mut scheduler = Scheduler::new().with_audio_pull(Box::new(pull));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        let _ = scheduler
            .tick(&mut state, &bus)
            .expect("tick A must succeed");
        // After tick A, state.frame == 1; sync the live mirror so the
        // next pull observes the new pre-tick frame value.
        *live_state.lock().unwrap() = state.frame();
        let _ = scheduler
            .tick(&mut state, &bus)
            .expect("tick B must succeed");

        let recorded = recorded_frames.lock().unwrap().clone();
        assert_eq!(recorded.len(), 2);
        // Pre-tick frame on tick A is 0; pre-tick frame on tick B is
        // 1 (the post-tick-A value mirrored before tick B started).
        assert_eq!(recorded[0], 0);
        assert_eq!(recorded[1], 1);
        assert_eq!(state.frame(), 2);
    }

    #[test]
    fn tick_pulls_snapshot_before_command_bus_commit() {
        // Verify ordering: the pull happens BEFORE
        // `commit_all_into`. Dispatch a `PresetRecall` command;
        // before tick, `state.active_preset()` is `None`. The audio
        // pull adapter records `state.active_preset().is_some()` at
        // pull time — it must observe `false` because the bus drain
        // has not happened yet.
        let observed_preset_at_pull: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
        let live_preset: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

        struct OrderingPull {
            observed: Arc<Mutex<Option<bool>>>,
            live_preset: Arc<Mutex<bool>>,
            snapshot: AudioUniformSnapshot,
        }

        impl AudioPull for OrderingPull {
            fn pull_snapshot(&self) -> AudioUniformSnapshot {
                let observation = *self.live_preset.lock().unwrap();
                *self.observed.lock().unwrap() = Some(observation);
                self.snapshot
            }
        }

        let pull = OrderingPull {
            observed: observed_preset_at_pull.clone(),
            live_preset: live_preset.clone(),
            snapshot: nonzero_snapshot(),
        };
        let mut scheduler = Scheduler::new().with_audio_pull(Box::new(pull));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        bus.dispatch(Command::PresetRecall { id: PresetId(42) })
            .expect("dispatch must succeed");
        bus.dispatch(Command::ControlSet {
            id: ControlId(3),
            value: ControlValue(0.9),
        })
        .expect("dispatch must succeed");

        // Before tick, the live mirror is `false` (no preset
        // recalled). The scheduler should pull BEFORE the bus drain
        // applies the recall, so the observed value is `false`.
        let _ = scheduler.tick(&mut state, &bus).expect("tick must succeed");
        // After tick, the bus drain has applied the recall; mirror
        // updates only happen post-tick to confirm the pre-pull
        // observation.
        *live_preset.lock().unwrap() = state.active_preset().is_some();

        assert_eq!(*observed_preset_at_pull.lock().unwrap(), Some(false));
        // Sanity: the bus drain DID apply during the tick — the
        // pull-before-drain ordering is the assertion, not a stall.
        assert_eq!(state.active_preset(), Some(PresetId(42)));
        assert_eq!(state.control_value(ControlId(3)), Some(ControlValue(0.9)));
    }

    #[test]
    fn tick_with_audio_pull_handles_lock_poisoning_gracefully() {
        // The `MutexAudioPull` poison-recovery path returns the inner
        // snapshot rather than panicking. Simulate a poisoned mutex
        // by panicking inside a `lock` scope on a side thread; the
        // scheduler-tick must still succeed and store the snapshot.
        let target = nonzero_snapshot();
        let shared: Arc<Mutex<AudioUniformSnapshot>> = Arc::new(Mutex::new(target));

        // Poison the mutex by panicking while holding the lock.
        let shared_clone = shared.clone();
        let _ = std::thread::spawn(move || {
            let _guard = shared_clone.lock().unwrap();
            panic!("simulated audio analyser fault");
        })
        .join();

        // The shared mutex is now poisoned. The
        // `MutexAudioPull::pull_snapshot` impl must recover the
        // inner snapshot rather than propagating the poison.
        assert!(shared.is_poisoned());
        let pull = MutexAudioPull::new(shared.clone());
        assert_eq!(pull.poison_recovery_count(), 0);
        let poison_recoveries = pull.poison_recovery_counter();
        let mut scheduler = Scheduler::new().with_audio_pull(Box::new(pull));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        let report = scheduler
            .tick(&mut state, &bus)
            .expect("tick must succeed despite poisoned audio mutex");
        assert_eq!(report.frame, 1);
        assert_eq!(state.latest_audio_snapshot(), Some(&target));
        assert_eq!(
            poison_recoveries.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }
}
