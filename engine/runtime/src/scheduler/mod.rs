// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Scheduler-tick path per ADR-001 (runtime model) §Decision and
//! ADR-006 (thread and process model) §Decision.
//!
//! The [`Scheduler`] owns the canonical frame-tick loop that
//! orchestrates the engine's command-bus drain, audio uniform pull,
//! render dispatch, and observability emission. ADR-006 names the
//! state-commit thread as the sole writer to the canonical
//! [`crate::state::State`]; the scheduler runs on that thread and is
//! the canonical entity that advances [`crate::state::State::frame`].
//!
//! ## Slice-3 part 1 scope (predecessor PR)
//!
//! Slice-3 PR 1 landed the foundational module surface only:
//!
//! - The [`Scheduler`] type plus a [`Scheduler::tick`] method that
//!   drains pending commands via [`CommandBus::commit_all_into`] and
//!   advances [`crate::state::State::frame`] by exactly one.
//! - [`TickReport`] and [`SchedulerError`] for tick outcome /
//!   failure reporting (currently `FrameOverflow` only;
//!   [`SchedulerError`] is `#[non_exhaustive]` so subsequent
//!   slice-3 PRs can widen the variant set non-breakingly).
//! - T1 drift pins under [`scheduler_pins`] covering the four
//!   scheduler-tick invariants the slice-3 wave's downstream PRs
//!   build on.
//!
//! ## Slice-3 part 5 scope (predecessor PR — observability emit wiring)
//!
//! Slice-3 PR 5 was the FIRST of slice-3 PRs 3 / 4 / 5 (audio
//! snapshot pull, render dispatch, observability emit) to land. It
//! established the sub-module + builder + tick-step extension
//! pattern that PRs 3 and 4 mirror, by:
//!
//! - Migrating the single-file `scheduler.rs` module into a module
//!   directory (`scheduler/mod.rs` for the foundational surface +
//!   `scheduler/observability.rs` for the per-tick metric-emit
//!   adapter).
//! - Introducing the [`observability::TickEmitter`] trait + the
//!   default [`observability::MetricEmitterAdapter`] wrapping any
//!   `Box<dyn chromaticity_observability::emit::EmitMetric>`.
//! - Widening the [`Scheduler`] struct with an
//!   `observability_handler:
//!   Option<Box<dyn observability::TickEmitter>>` field plus the
//!   [`Scheduler::with_metric_emitter`] builder.
//! - Inserting a per-tick emit step into [`Scheduler::tick`] AFTER
//!   the command-bus drain and frame advancement so the emitter
//!   observes the post-tick state.
//!
//! When no emitter is configured the tick path is a no-op for
//! observability; the slice-3 PR 1 invariants hold verbatim. The
//! [`scheduler_pins`] T1 module preserves all four pre-existing
//! invariants unchanged; new observability-specific T1 pins live
//! co-located with the adapter under [`observability`].
//!
//! ## Slice-3 part 3 scope (this PR — audio snapshot pull wiring)
//!
//! This PR mirrors the slice-3 PR 5 pattern for audio:
//!
//! - Adds the [`audio`] sub-module with the [`audio::AudioPull`]
//!   trait + the default [`audio::MutexAudioPull`] adapter wrapping
//!   any `Arc<Mutex<chromaticity_audio::AudioUniformSnapshot>>`.
//! - Widens the [`Scheduler`] struct with an
//!   `audio_handler: Option<Box<dyn audio::AudioPull>>` field plus
//!   the [`Scheduler::with_audio_pull`] builder.
//! - Inserts a per-tick pull step into [`Scheduler::tick`] BEFORE
//!   the command-bus drain AND BEFORE the frame counter
//!   advancement, so the snapshot represents the audio state
//!   observed at the START of the tick.
//!
//! When no handler is configured the tick path leaves
//! [`crate::state::State::latest_audio_snapshot`] unchanged — the
//! slice-3 PR 1 invariants hold verbatim, and the EV-STATE-REPLAY
//! (row 32) / EV-PRESET-PLAYLIST (row 34) T2 corpora preserve their
//! goldens because the canonical-binary encoder does not observe
//! the new field. Audio-specific T1 pins live co-located with the
//! adapter under [`audio`].
//!
//! Subsequent slice-3 PR wires the tick into:
//!
//! - render dispatch (PR 4).
//!
//! Per-PR scope is intentionally narrow: a single foundational
//! module landing per PR keeps the integration surface reviewable,
//! lets each downstream wiring PR carry its own evidence-tag, and
//! avoids the choke-point trap where a single PR rewrites multiple
//! crate boundaries.
//!
//! ## Additivity invariant — existing goldens preserved
//!
//! The scheduler is purely **additive** in this wave. The existing
//! [`crate::command_bus::InMemoryBus::commit_all_into`] path is
//! unchanged; the slice-3 scheduler-tick is a *new* code path that
//! composes [`CommandBus::commit_all_into`] with an explicit
//! `state.frame += 1`. Consequence: the EV-STATE-REPLAY (row 32)
//! and EV-PRESET-PLAYLIST (row 34) T2 corpora — both of which drive
//! state mutation through [`crate::command_bus::InMemoryBus::commit_all_into`]
//! directly without scheduler involvement — observe identical
//! behaviour, and their golden hashes are preserved verbatim by
//! this PR. The scheduler-tick path will get its own corpus +
//! goldens in slice-3 PR 2 once the audio-uniform pull seam lands.
//!
//! ## Frame-advancement ordering
//!
//! [`Scheduler::tick`] commits pending commands via
//! [`CommandBus::commit_all_into`] **before** advancing the frame
//! counter. The ordering is load-bearing: events committed during
//! tick `N` describe state mutations dispatched **during the
//! frame** (typically by integration-IO and audio-analysis producers
//! whose loops were sampled before the commit-thread woke up). The
//! frame counter increment after the commit captures the "this
//! frame's mutations have been applied; we are now in the next
//! frame" semantics and matches the canonical timing diagram in
//! ADR-001 §Decision.
//!
//! Consequence for [`TickReport`]: `report.frame` is the
//! post-increment value, i.e. the frame the scheduler is now
//! entering, not the frame whose commands were just drained.
//!
//! ## Single-writer invariant
//!
//! Per ADR-006 §Decision, only the state-commit thread mutates the
//! canonical state. Slice-3 part 1 preserves that invariant:
//! [`Scheduler::tick`] mutates `state` in exactly two places —
//! the [`CommandBus::commit_all_into`] call (which routes through
//! [`crate::state::State::apply_event`] internally) and the
//! `state.frame += 1` increment. No other write site is introduced
//! by this PR.

use crate::command_bus::InMemoryBus;
use crate::state::State;
use std::fmt;

pub mod audio;
pub mod observability;

/// Scheduler ownership of the canonical frame tick per ADR-001
/// §Decision and ADR-006 §Decision.
///
/// The scheduler is the sole entity that advances
/// [`crate::state::State::frame`] and is expected to run on the
/// state commit thread. Slice-3 part 1 (this PR) lands the
/// foundational module + tick API; subsequent slice-3 PRs wire
/// the tick into command-bus observability, audio snapshot
/// extraction, render dispatch, and observability emission per
/// the module-level prose above.
#[derive(Default)]
pub struct Scheduler {
    // Crate-private internal state expanded incrementally across
    // slice-3 PRs. PR 1 reserved the shape via a single unit-typed
    // private field; PR 5 adds the optional observability handler
    // (this field). PRs 3 and 4 will mirror the same
    // `Option<Box<dyn ...Trait>>` shape for the audio-snapshot pull
    // and render-dispatch seams respectively.
    _private: (),
    /// Optional metric-emit handler invoked after every tick.
    /// Phase-1 stub uses the
    /// [`chromaticity_observability::emit::InMemoryEmitter`];
    /// Phase-3+ wires a real metric sink. Set via
    /// [`Scheduler::with_metric_emitter`]; absent by default — the
    /// tick path is a no-op for observability when `None`.
    observability_handler: Option<Box<dyn observability::TickEmitter>>,
    /// Optional audio-snapshot pull handler invoked at the START of
    /// every tick (BEFORE the command-bus drain and frame counter
    /// advancement). Phase-1 stub wraps an
    /// `Arc<Mutex<chromaticity_audio::AudioUniformSnapshot>>` shared
    /// with the audio analysis thread per ADR-006; Phase-3+ may
    /// surface freshness guards or replay strategies through the
    /// same [`audio::AudioPull`] seam. Set via
    /// [`Scheduler::with_audio_pull`]; absent by default — the tick
    /// path leaves [`crate::state::State::latest_audio_snapshot`]
    /// unchanged when `None` and the slice-3 PR 1 invariants hold
    /// verbatim.
    audio_handler: Option<Box<dyn audio::AudioPull>>,
}

impl fmt::Debug for Scheduler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The trait object inside `observability_handler` is not
        // `Debug`; render only whether a handler is configured, not
        // its inner state. Mirrors the pattern PRs 3 / 4 will reuse
        // for their own non-`Debug` trait-object fields.
        f.debug_struct("Scheduler")
            .field(
                "observability_handler",
                &self.observability_handler.is_some(),
            )
            .field("audio_handler", &self.audio_handler.is_some())
            .finish()
    }
}

impl Scheduler {
    /// Construct a new scheduler.
    ///
    /// Construction is pure: no frame counter advancement happens
    /// here. The first frame tick is gated on the first
    /// [`Scheduler::tick`] call.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the scheduler to emit a [`chromaticity_observability::catalog::MetricEvent`]
    /// after every tick to the provided emitter.
    ///
    /// Typical Phase-1 usage wraps a
    /// [`chromaticity_observability::emit::InMemoryEmitter`] in the
    /// default [`observability::MetricEmitterAdapter`]; Phase-3+
    /// usage will swap in a runtime-bus emitter implementation. When
    /// no emitter is configured, the tick path is a no-op for
    /// observability — slice-3 PR 1 invariants hold verbatim.
    ///
    /// Mirrors the builder pattern slice-3 PRs 3 (audio snapshot
    /// pull) and 4 (render dispatch) will reuse for their own
    /// per-tick subsystem handlers.
    #[must_use]
    pub fn with_metric_emitter(mut self, emitter: Box<dyn observability::TickEmitter>) -> Self {
        self.observability_handler = Some(emitter);
        self
    }

    /// Configure the scheduler to pull a
    /// [`chromaticity_audio::AudioUniformSnapshot`] at the START of
    /// every tick (BEFORE the command-bus drain and frame counter
    /// advancement) via the provided [`audio::AudioPull`] handler.
    ///
    /// Typical Phase-1 usage wraps an
    /// `Arc<Mutex<AudioUniformSnapshot>>` shared with the audio
    /// analysis thread in the default [`audio::MutexAudioPull`];
    /// Phase-3+ may swap in freshness-guarded or replay-aware
    /// implementations. When no handler is configured, the tick
    /// path leaves [`crate::state::State::latest_audio_snapshot`]
    /// unchanged — slice-3 PR 1 invariants hold verbatim and the
    /// EV-STATE-REPLAY (row 32) / EV-PRESET-PLAYLIST (row 34) T2
    /// corpora preserve their goldens because the canonical-binary
    /// encoder does not observe the audio snapshot field.
    ///
    /// Mirrors the builder pattern slice-3 PR 5
    /// ([`Scheduler::with_metric_emitter`]) established; slice-3 PR 4
    /// (render dispatch) will reuse the same shape for its own
    /// per-tick subsystem handler.
    #[must_use]
    pub fn with_audio_pull(mut self, handler: Box<dyn audio::AudioPull>) -> Self {
        self.audio_handler = Some(handler);
        self
    }

    /// Advance the frame by one tick.
    ///
    /// Drains every pending command on `bus` into `state` via
    /// [`InMemoryBus::commit_all_into`], then increments
    /// [`crate::state::State::frame`] by exactly one. Returns a
    /// [`TickReport`] carrying the post-increment frame value and
    /// the count of events committed during this tick.
    ///
    /// ## Slice-3 part 1 implementation scope
    ///
    /// This PR implements the command-bus drain plus frame
    /// counter advancement. Audio snapshot pull, render dispatch,
    /// and observability emission land in slice-3 PRs 2-5
    /// respectively per the module-level prose. The bus parameter
    /// is the concrete [`InMemoryBus`] rather than a generic
    /// [`crate::command_bus::CommandBus`] trait bound: the trait
    /// declares only the producer-side `dispatch` method, while
    /// the commit-side drain ([`InMemoryBus::commit_all_into`])
    /// lives on the concrete type. Hoisting commit into the
    /// trait would widen its surface area beyond what slice-2
    /// scoped, and slice-3 part 1 keeps the bus seam unchanged
    /// per the additivity invariant in the module-level prose.
    ///
    /// ## Errors
    ///
    /// Returns [`SchedulerError::FrameOverflow`] if the frame
    /// counter would overflow `u64::MAX` on this tick. The state's
    /// frame value remains at `u64::MAX` (no wrap, no command
    /// drain) so the caller can decide the recovery strategy. In
    /// normal operation overflow is unreachable: at 60 FPS the
    /// counter would take ~9.7 billion years to saturate.
    pub fn tick(
        &mut self,
        state: &mut State,
        bus: &InMemoryBus,
    ) -> Result<TickReport, SchedulerError> {
        // Overflow guard runs BEFORE the command drain so an
        // overflow tick is a no-op on the bus as well as on the
        // counter. Otherwise the bus would commit events under a
        // state whose frame value the scheduler then refuses to
        // advance, producing a half-applied tick the caller
        // cannot reason about.
        if state.frame == u64::MAX {
            return Err(SchedulerError::FrameOverflow {
                current: state.frame,
            });
        }

        // Slice-3 PR 3: pull the latest audio uniform snapshot if a
        // handler is configured. Ordering is load-bearing — the pull
        // happens BEFORE the command-bus drain AND BEFORE the frame
        // counter advancement, so the snapshot represents the audio
        // state observed at the START of this tick. Commands
        // committed during the drain may reference the snapshot
        // (e.g. audio-reactive control writes routed through script
        // in slice-5+); the scheduler does not reorder. When the
        // handler is `None` the tick path leaves
        // `state.latest_audio_snapshot` unchanged and the slice-3
        // PR 1 invariants hold verbatim.
        if let Some(handler) = &self.audio_handler {
            state.latest_audio_snapshot = Some(handler.pull_snapshot());
        }

        let events = bus.commit_all_into(state);
        let events_committed = events.len();
        state.frame += 1;

        // Slice-3 PR 5: emit a per-tick MetricEvent when an
        // observability handler is configured. Ordering is load-
        // bearing — the emit happens AFTER `commit_all_into`
        // returned the events AND AFTER the frame counter
        // increment, so the emitter observes the post-tick state
        // (the frame the scheduler is now entering, plus the count
        // of events drained during this tick). When the handler is
        // `None` the tick path is a no-op for observability and
        // the slice-3 PR 1 invariants hold verbatim.
        if let Some(handler) = &self.observability_handler {
            handler.emit_tick_event(state.frame, events_committed);
        }

        Ok(TickReport {
            frame: state.frame,
            events_committed,
        })
    }
}

/// Outcome of a single scheduler tick.
///
/// `frame` is the post-increment frame value (the frame the
/// scheduler is now entering); `events_committed` is the number of
/// events drained from the command bus during this tick.
/// Consumers can inspect both for observability and regression
/// assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickReport {
    /// Post-tick frame counter value.
    pub frame: u64,
    /// Count of events committed during this tick.
    pub events_committed: usize,
}

/// Failures observed during a scheduler tick.
///
/// Slice-3 part 1 introduces this type with a single variant.
/// The enum is `#[non_exhaustive]` so subsequent slice-3 PRs
/// can widen the failure set (e.g. command-bus dispatch failures,
/// audio snapshot pull errors) without breaking downstream
/// callers.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerError {
    /// The frame counter would overflow `u64::MAX` on this tick.
    ///
    /// Saturation is not an option for the canonical state
    /// hash — overflow is a catastrophic state-of-the-runtime
    /// invariant violation. In normal operation this is
    /// unreachable: at 60 FPS the counter would take
    /// ~9.7 billion years to overflow.
    FrameOverflow {
        /// The frame counter value that triggered the overflow
        /// guard. Always `u64::MAX` in slice-3 part 1; carried
        /// in the variant payload so callers don't have to
        /// re-derive it from the state.
        current: u64,
    },
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrameOverflow { current } => {
                write!(f, "scheduler frame counter overflow at {current}")
            }
        }
    }
}

impl std::error::Error for SchedulerError {}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod scheduler_pins {
    //! Evidence: EV-SCHEDULER-TICK (T1) — scheduler-tick frame
    //! advancement + command-drain composition baseline.
    //!
    //! T1 per docs/43 §15.5: each test pins a state-machine
    //! invariant the [`super::Scheduler`] must honour for the
    //! slice-3 wave's downstream PRs (audio uniform pull, render
    //! dispatch, observability emission) to compose correctly.
    //! `EV-SCHEDULER-TICK` is registered in `docs/26` as `Planned`
    //! at this PR; the full T2 scheduler-tick corpus and the
    //! `Pass` transition land once the audio-uniform pull seam
    //! exists in slice-3 PR 2. The scheduler is purely additive —
    //! `InMemoryBus::commit_all_into` itself is unchanged — so the
    //! EV-STATE-REPLAY (row 32) and EV-PRESET-PLAYLIST (row 34) T2
    //! corpora preserve their goldens verbatim under this PR.
    //!
    //! Invariants pinned here:
    //!
    //! - `tick_advances_frame_by_exactly_one` — single tick on a
    //!   fresh state lands the frame counter at exactly 1, not 0
    //!   and not >1.
    //! - `tick_drains_command_bus` — pending commands on the bus
    //!   are committed during the tick and observable via the
    //!   public state-accessor surface afterward; the
    //!   [`super::TickReport::events_committed`] field reports
    //!   the drained count exactly.
    //! - `tick_is_pure_same_inputs_same_report` — two
    //!   independent (state, bus) pairs ticked with identical
    //!   command sequences yield identical reports. This pins
    //!   the determinism invariant the scheduler-tick path
    //!   inherits from the EV-DETERMINISM bus surface.
    //! - `tick_returns_frame_overflow_at_u64_max` — the overflow
    //!   guard fires before the command drain and leaves the
    //!   state's frame value unchanged at `u64::MAX`. The bus
    //!   is also not drained on overflow, so the caller's
    //!   commands stay enqueued for a recovery tick.

    use super::{Scheduler, SchedulerError, TickReport};
    use crate::command_bus::CommandBus;
    use crate::command_bus::{Command, ControlId, ControlValue, InMemoryBus, PresetId};
    use crate::state::State;

    #[test]
    fn tick_advances_frame_by_exactly_one() {
        let mut scheduler = Scheduler::new();
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        assert_eq!(state.frame(), 0);
        let report = scheduler.tick(&mut state, &bus).expect("tick must succeed");
        assert_eq!(state.frame(), 1);
        assert_eq!(
            report,
            TickReport {
                frame: 1,
                events_committed: 0,
            }
        );
    }

    #[test]
    fn tick_drains_command_bus() {
        let mut scheduler = Scheduler::new();
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        bus.dispatch(Command::ControlSet {
            id: ControlId(7),
            value: ControlValue(0.25),
        })
        .expect("dispatch must succeed");
        bus.dispatch(Command::ControlSet {
            id: ControlId(8),
            value: ControlValue(0.5),
        })
        .expect("dispatch must succeed");
        bus.dispatch(Command::PresetRecall { id: PresetId(3) })
            .expect("dispatch must succeed");

        let report = scheduler.tick(&mut state, &bus).expect("tick must succeed");
        assert_eq!(report.events_committed, 3);
        assert_eq!(report.frame, 1);
        assert_eq!(state.control_value(ControlId(7)), Some(ControlValue(0.25)));
        assert_eq!(state.control_value(ControlId(8)), Some(ControlValue(0.5)));
        assert_eq!(state.active_preset(), Some(PresetId(3)));
    }

    #[test]
    fn tick_is_pure_same_inputs_same_report() {
        // Build two independent (state, bus) pairs, dispatch the
        // same command sequence onto each, tick both, and assert
        // the resulting reports are equal. Pins the determinism
        // invariant the scheduler-tick path inherits from the
        // EV-DETERMINISM bus surface.
        fn run() -> (TickReport, u64) {
            let mut scheduler = Scheduler::new();
            let mut state = State::default();
            let bus = InMemoryBus::in_memory();
            bus.dispatch(Command::PresetRecall { id: PresetId(1) })
                .expect("dispatch must succeed");
            bus.dispatch(Command::PlaylistAdvance { step: 5 })
                .expect("dispatch must succeed");
            bus.dispatch(Command::ControlSet {
                id: ControlId(11),
                value: ControlValue(0.125),
            })
            .expect("dispatch must succeed");
            let report = scheduler.tick(&mut state, &bus).expect("tick must succeed");
            (report, state.frame())
        }

        let (report_a, frame_a) = run();
        let (report_b, frame_b) = run();
        assert_eq!(report_a, report_b);
        assert_eq!(frame_a, frame_b);
        assert_eq!(report_a.events_committed, 3);
    }

    #[test]
    fn tick_returns_frame_overflow_at_u64_max() {
        // Pre-load `state.frame` to `u64::MAX` via the crate-
        // private field — the same access path
        // `state_replay`/`state_migration` rely on. The overflow
        // guard MUST fire before the bus is drained, leaving both
        // the frame counter AND the dispatched-but-uncommitted
        // command on the bus unchanged so the caller can choose
        // a recovery strategy.
        let mut scheduler = Scheduler::new();
        let mut state = State {
            frame: u64::MAX,
            ..State::default()
        };
        let bus = InMemoryBus::in_memory();
        bus.dispatch(Command::PresetRecall { id: PresetId(42) })
            .expect("dispatch must succeed");

        let result = scheduler.tick(&mut state, &bus);
        assert_eq!(
            result,
            Err(SchedulerError::FrameOverflow { current: u64::MAX })
        );
        assert_eq!(state.frame(), u64::MAX);
        // Preset MUST NOT be applied — the bus drain is gated on
        // the overflow check.
        assert_eq!(state.active_preset(), None);
    }
}
