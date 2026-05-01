// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Slice-3 PR 4 scheduler-tick render-dispatch wiring.
//!
//! After the canonical state has advanced (frame counter incremented,
//! events committed, audio snapshot pulled per slice-3 PRs 1+3), the
//! scheduler signals a render-dispatch handler to dispatch a new render
//! frame. Per ADR-001 §Decision the render thread samples the immutable
//! state snapshot; per ADR-006 the render thread is separate from the
//! state-commit thread. The scheduler's role here is purely the
//! dispatch signal; the actual rendering happens out-of-process on the
//! render thread which the handler abstracts.
//!
//! ## Tick-ordering contract
//!
//! [`super::Scheduler::tick`] invokes [`RenderDispatch::dispatch_frame`]
//! AFTER the [`crate::command_bus::InMemoryBus::commit_all_into`] drain
//! AND AFTER the `state.frame += 1` increment, so the render handler
//! observes the post-tick state (the frame the scheduler is now
//! entering, plus any commands committed during this tick AND the
//! audio snapshot pulled at the START of this tick per slice-3 PR 3).
//! The render-dispatch step runs BEFORE the slice-3 PR 5 observability
//! emit so observability — when both seams are configured — closes the
//! tick after every per-tick subsystem has fired.
//!
//! When no handler is configured the tick path is a no-op for render
//! dispatch — the slice-3 PR 1 invariants hold verbatim, and the
//! EV-STATE-REPLAY (row 32) / EV-PRESET-PLAYLIST (row 34) T2 corpora
//! preserve their goldens because the render-dispatch step does not
//! mutate state.
//!
//! ## Production adapter is post-Phase-3 scope
//!
//! The slice-3 PR 4 surface is intentionally the scheduler-side trait
//! only. A production [`RenderDispatch`] implementation that bridges
//! [`crate::state::State`] → `chromaticity_render::MultipassSpec` and
//! invokes the `chromaticity_render::compile_and_render_multipass`
//! pipeline depends on the scene-adapter
//! (`scene_adapter::AdapterError` plus `MultipassSpec::from_scene`)
//! which is itself a post-Phase-3 scene-infrastructure slice. This PR
//! establishes the dispatch hook so future scene + render integration
//! plugs in cleanly without touching the scheduler.
//!
//! ## Dispatch outcome semantics
//!
//! [`RenderDispatchOutcome`] captures the three reportable states of a
//! single dispatch invocation:
//!
//! - [`RenderDispatchOutcome::Queued`] — the typical path; the render
//!   thread accepted the dispatch.
//! - [`RenderDispatchOutcome::Skipped`] — the render thread is back-
//!   pressured and the handler chose to drop the frame. Per ADR-006
//!   the scheduler keeps advancing state regardless; frame skip is
//!   acceptable.
//! - [`RenderDispatchOutcome::Errored`] — the render thread reported
//!   an error. The scheduler does NOT abort the tick — operators
//!   observe the error via the slice-3 PR 5 observability emit. A
//!   future T1 widening may surface the outcome on
//!   [`super::TickReport`] without changing this seam.

use crate::state::State;

/// Scheduler-side adapter for dispatching a render frame after the tick
/// has advanced the canonical state.
///
/// Implemented by:
///
/// - Production handlers wrapping the `chromaticity_render`
///   `compile_and_render_multipass` pipeline (post-Phase-3 scope; the
///   adapter bridges [`State`] → `MultipassSpec`).
/// - Test stubs that record the dispatch frame counter for assertion
///   (see the in-module T1 drift pins).
///
/// `Send + Sync` is required because the scheduler is owned on the
/// state-commit thread per ADR-006 §Decision but the render thread runs
/// separately; a real handler typically wraps a cross-thread channel
/// or a shared snapshot ring whose synchronisation primitive already
/// satisfies these bounds.
pub trait RenderDispatch: Send + Sync {
    /// Called once per [`super::Scheduler::tick`], AFTER
    /// [`crate::state::State::frame`] has advanced AND
    /// [`crate::command_bus::InMemoryBus::commit_all_into`] has
    /// returned AND the audio snapshot has been pulled per slice-3
    /// PR 3. The handler receives an immutable view of the state for
    /// snapshotting; it MUST NOT mutate.
    ///
    /// Returns a [`RenderDispatchOutcome`] describing whether the
    /// dispatch was queued, skipped, or errored. Implementations MUST
    /// NOT panic — the scheduler-tick path must continue advancing
    /// even when the render thread has faulted.
    fn dispatch_frame(&self, state: &State) -> RenderDispatchOutcome;
}

/// Outcome of a render-dispatch invocation.
///
/// `#[non_exhaustive]` is deliberately omitted: the three reportable
/// states are exhaustive for slice-3 PR 4, and any Phase-3+ widening
/// (e.g. a `Throttled` variant carrying a target-FPS cap) is a
/// breaking trait widening that needs an ADR + registry row anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderDispatchOutcome {
    /// Frame dispatch was queued for the render thread. The typical
    /// path on a healthy system.
    Queued,
    /// Frame dispatch was skipped (e.g. the render thread is
    /// backpressured; the handler chose to drop this frame). Per
    /// ADR-006 the scheduler keeps advancing state since frame skip
    /// is acceptable in the deterministic-state / best-effort-render
    /// thread model.
    Skipped,
    /// The handler reported a render-side error. Does NOT abort the
    /// tick — the scheduler keeps state advancement going. Operators
    /// see the error via the slice-3 PR 5 observability emit; a
    /// future T1 widening may surface the outcome on
    /// [`super::TickReport`] without changing the trait surface.
    Errored,
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod scheduler_render_pins {
    //! Evidence: EV-SCHEDULER-TICK (T1) — scheduler-tick render
    //! dispatch baseline.
    //!
    //! T1 per docs/43 §15.5: each test pins a state-machine
    //! invariant the [`super::RenderDispatch`] / [`super::super::Scheduler`]
    //! composition must honour for the slice-3 wave's downstream
    //! render surface (post-Phase-3 scene-infrastructure adapter
    //! that bridges [`crate::state::State`] →
    //! `chromaticity_render::MultipassSpec`) to compose correctly.
    //! `EV-SCHEDULER-TICK` is registered in `docs/26` as `Planned`;
    //! the `Pass` transition lands in the slice-3 closing docs PR
    //! after PRs 3 / 4 / 5 / 6 land.
    //!
    //! Invariants pinned here:
    //!
    //! - `tick_with_render_dispatch_invokes_handler_once_per_tick` —
    //!   a single [`super::super::Scheduler::tick`] call invokes the
    //!   configured handler exactly once.
    //! - `tick_without_render_dispatch_is_noop` — a scheduler with
    //!   `None` handler ticks successfully and dispatches nothing
    //!   (the slice-3 PR 1 invariants hold verbatim).
    //! - `tick_render_handler_receives_post_tick_frame` — the
    //!   handler's observed `state.frame()` value equals the
    //!   post-increment value, matching the
    //!   [`super::super::TickReport::frame`] semantics.
    //! - `tick_render_handler_receives_post_tick_audio_snapshot` —
    //!   when both [`super::super::Scheduler::with_audio_pull`] and
    //!   [`super::super::Scheduler::with_render_dispatch`] are
    //!   configured, the render handler observes the audio snapshot
    //!   pulled at the START of this same tick. Pins the slice-3
    //!   PR 3 → PR 4 ordering (audio runs BEFORE render in a tick).
    //! - `tick_render_handler_skipped_outcome_does_not_abort_tick` —
    //!   a handler returning [`super::RenderDispatchOutcome::Skipped`]
    //!   does not abort the tick; subsequent ticks still advance the
    //!   frame counter correctly.

    use super::{RenderDispatch, RenderDispatchOutcome};
    use crate::command_bus::CommandBus;
    use crate::command_bus::{Command, ControlId, ControlValue, InMemoryBus, PresetId};
    use crate::scheduler::Scheduler;
    use crate::scheduler::audio::AudioPull;
    use crate::state::State;
    use chromaticity_audio::AudioUniformSnapshot;
    use std::sync::{Arc, Mutex};

    /// Mock [`RenderDispatch`] recording the post-tick `state.frame()`
    /// observed at each dispatch plus the count of audio-snapshot-
    /// present observations. Mirrors the recording-stub shape used by
    /// the sibling observability + audio pin modules.
    struct RecordingRenderDispatch {
        frames: Mutex<Vec<u64>>,
        audio_snapshots: Mutex<Vec<Option<AudioUniformSnapshot>>>,
        outcome: RenderDispatchOutcome,
    }

    impl RecordingRenderDispatch {
        fn new(outcome: RenderDispatchOutcome) -> Self {
            Self {
                frames: Mutex::new(Vec::new()),
                audio_snapshots: Mutex::new(Vec::new()),
                outcome,
            }
        }

        fn frames(&self) -> Vec<u64> {
            self.frames.lock().unwrap().clone()
        }

        fn audio_snapshots(&self) -> Vec<Option<AudioUniformSnapshot>> {
            self.audio_snapshots.lock().unwrap().clone()
        }
    }

    impl RenderDispatch for RecordingRenderDispatch {
        fn dispatch_frame(&self, state: &State) -> RenderDispatchOutcome {
            self.frames.lock().unwrap().push(state.frame());
            self.audio_snapshots
                .lock()
                .unwrap()
                .push(state.latest_audio_snapshot().copied());
            self.outcome
        }
    }

    /// Adapter routing the `Box<dyn RenderDispatch>` indirection back
    /// to the test-owned `Arc<RecordingRenderDispatch>` so assertions
    /// can read the recorded observations after the tick. Mirrors the
    /// `ArcRecorder` shape in the sibling observability + audio pins.
    struct ArcRenderRecorder(Arc<RecordingRenderDispatch>);

    impl RenderDispatch for ArcRenderRecorder {
        fn dispatch_frame(&self, state: &State) -> RenderDispatchOutcome {
            self.0.dispatch_frame(state)
        }
    }

    /// Mock [`AudioPull`] returning a fixed snapshot for the audio /
    /// render ordering pin.
    struct FixedAudioPull(AudioUniformSnapshot);

    impl AudioPull for FixedAudioPull {
        fn pull_snapshot(&self) -> AudioUniformSnapshot {
            self.0
        }
    }

    fn nonzero_snapshot() -> AudioUniformSnapshot {
        AudioUniformSnapshot {
            bpm: 132.5,
            rms: 0.42,
            peak: 0.91,
            fft_bands: [0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7],
        }
    }

    #[test]
    fn tick_with_render_dispatch_invokes_handler_once_per_tick() {
        let recorder = Arc::new(RecordingRenderDispatch::new(RenderDispatchOutcome::Queued));
        let mut scheduler =
            Scheduler::new().with_render_dispatch(Box::new(ArcRenderRecorder(recorder.clone())));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        let _ = scheduler.tick(&mut state, &bus).expect("tick must succeed");

        let frames = recorder.frames();
        assert_eq!(
            frames.len(),
            1,
            "exactly one dispatch per tick; got {frames:?}"
        );
    }

    #[test]
    fn tick_without_render_dispatch_is_noop() {
        // Slice-3 PR 1 invariants must hold verbatim when no render
        // handler is configured: tick succeeds, frame advances, no
        // panic, no dispatch side-effects.
        let mut scheduler = Scheduler::new();
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        let report = scheduler
            .tick(&mut state, &bus)
            .expect("tick must succeed without render dispatch");
        assert_eq!(report.frame, 1);
        assert_eq!(report.events_committed, 0);
        assert_eq!(state.frame(), 1);
    }

    #[test]
    fn tick_render_handler_receives_post_tick_frame() {
        // The render handler must observe the post-increment frame
        // value (1 on the first tick, 2 on the second, etc.) — the
        // dispatch hook fires AFTER the frame counter advances.
        let recorder = Arc::new(RecordingRenderDispatch::new(RenderDispatchOutcome::Queued));
        let mut scheduler =
            Scheduler::new().with_render_dispatch(Box::new(ArcRenderRecorder(recorder.clone())));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        let report_a = scheduler
            .tick(&mut state, &bus)
            .expect("tick A must succeed");
        let report_b = scheduler
            .tick(&mut state, &bus)
            .expect("tick B must succeed");

        let frames = recorder.frames();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0], report_a.frame);
        assert_eq!(frames[0], 1);
        assert_eq!(frames[1], report_b.frame);
        assert_eq!(frames[1], 2);
        assert_eq!(state.frame(), 2);
    }

    #[test]
    fn tick_render_handler_receives_post_tick_audio_snapshot() {
        // Pin the slice-3 PR 3 → PR 4 tick-step ordering: the audio
        // pull runs at the START of the tick (BEFORE the bus drain
        // and frame advance), and the render dispatch runs AFTER the
        // bus drain and frame advance. So when both handlers are
        // configured, the render handler observes the audio snapshot
        // pulled at the START of this same tick on
        // `state.latest_audio_snapshot()`.
        let target = nonzero_snapshot();
        let render_recorder = Arc::new(RecordingRenderDispatch::new(RenderDispatchOutcome::Queued));
        let mut scheduler = Scheduler::new()
            .with_audio_pull(Box::new(FixedAudioPull(target)))
            .with_render_dispatch(Box::new(ArcRenderRecorder(render_recorder.clone())));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        // Sanity: pre-tick the audio snapshot is None.
        assert_eq!(state.latest_audio_snapshot(), None);

        let _ = scheduler.tick(&mut state, &bus).expect("tick must succeed");

        // Post-tick the state carries the pulled snapshot and the
        // render handler observed it during dispatch.
        assert_eq!(state.latest_audio_snapshot(), Some(&target));
        let observed = render_recorder.audio_snapshots();
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0], Some(target));
    }

    #[test]
    fn tick_render_handler_skipped_outcome_does_not_abort_tick() {
        // A handler returning `Skipped` is a routine backpressure
        // signal per ADR-006 — the scheduler keeps advancing state
        // regardless. Drive three ticks against a always-Skipped
        // handler, dispatch a few commands across the run, and pin
        // that the frame counter and command-bus drain still observe
        // the slice-3 PR 1 baseline.
        let recorder = Arc::new(RecordingRenderDispatch::new(RenderDispatchOutcome::Skipped));
        let mut scheduler =
            Scheduler::new().with_render_dispatch(Box::new(ArcRenderRecorder(recorder.clone())));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        let _ = scheduler
            .tick(&mut state, &bus)
            .expect("tick A must succeed");

        bus.dispatch(Command::ControlSet {
            id: ControlId(2),
            value: ControlValue(0.4),
        })
        .expect("dispatch must succeed");
        bus.dispatch(Command::PresetRecall { id: PresetId(9) })
            .expect("dispatch must succeed");

        let report_b = scheduler
            .tick(&mut state, &bus)
            .expect("tick B must succeed despite Skipped outcome");
        let report_c = scheduler
            .tick(&mut state, &bus)
            .expect("tick C must succeed despite Skipped outcome");

        assert_eq!(report_b.events_committed, 2);
        assert_eq!(report_c.events_committed, 0);
        assert_eq!(state.frame(), 3);
        assert_eq!(state.active_preset(), Some(PresetId(9)));
        assert_eq!(state.control_value(ControlId(2)), Some(ControlValue(0.4)));
        assert_eq!(recorder.frames(), vec![1, 2, 3]);
    }
}
