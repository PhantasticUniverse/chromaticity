// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Slice-3 PR 5 scheduler-tick observability wiring.
//!
//! Wraps the [`chromaticity_observability::emit::EmitMetric`] trait
//! into a scheduler-aware [`TickEmitter`] that
//! [`super::Scheduler::tick`] calls after every frame advancement.
//! The Phase-1 baseline catalog (`docs/31` §Phase-1 baseline catalog,
//! mirrored by [`chromaticity_observability::catalog::MetricEvent`])
//! does not yet carry a per-frame metric variant; the closest
//! semantic fit is [`MetricEvent::RuntimeUptime`], a label-free
//! counter / gauge keyed off the post-tick
//! [`crate::state::State::frame`] value. Phase-3+ catalog widening
//! that introduces a dedicated `RuntimeFrameAdvanced` (or similar)
//! variant will swap in here without changing the [`TickEmitter`]
//! seam.
//!
//! Why `RuntimeUptime` and not `RuntimeCommandsDispatched`:
//!
//! - `RuntimeCommandsDispatched { kind, value }` requires a
//!   [`chromaticity_observability::catalog::CommandKind`] label.
//!   The per-tick aggregate of N commands does not have a single
//!   `kind` — synthesising one (e.g. always `ControlSet`) would
//!   pollute the catalog's `kind` cardinality with a fake label
//!   value the docs/31 vocabulary would have to retroactively
//!   accommodate.
//! - `RuntimeUptime { value }` is label-free and the variant prose
//!   already describes a monotonic-frame-counter-shaped reading;
//!   emitting `value: state.frame()` is honest about what the
//!   per-tick signal represents (the frame the scheduler is now
//!   entering, after the commit-bus drain). Slice-3 PR 1's overflow
//!   guard ensures the value never wraps inside a successful tick.
//!
//! The chosen mapping is documented in the registry / Phase-3+
//! widening plan; PRs 3 (audio) and 4 (render) follow the same
//! sub-module + builder + tick-step pattern this module establishes.

use chromaticity_observability::catalog::MetricEvent;
use chromaticity_observability::emit::EmitMetric;

/// Scheduler-side adapter for emitting a per-tick metric.
///
/// Implemented by:
///
/// - The default [`MetricEmitterAdapter`] wrapping any
///   `Box<dyn EmitMetric>` (production use).
/// - Test stubs that accumulate events in-memory for assertion.
///
/// `Send + Sync` is required because the scheduler is owned on the
/// state-commit thread per ADR-006 §Decision but the emitter may
/// fan out to a sink consumed on another thread (e.g. the Phase-4d
/// loopback-only Prometheus exposer); the trait bound matches
/// [`chromaticity_observability::emit::EmitMetric`] verbatim so an
/// adapter implementation pays no synchronisation tax beyond what
/// the wrapped emitter already requires.
pub trait TickEmitter: Send + Sync {
    /// Called once per [`super::Scheduler::tick`], AFTER the
    /// command-bus drain completes AND the frame counter has been
    /// advanced. The handler observes the post-tick frame value
    /// and the count of events committed during this tick.
    ///
    /// Implementations MUST NOT panic — the scheduler-tick path
    /// inherits the no-panic-on-emit contract from
    /// [`chromaticity_observability::emit::EmitMetric`].
    fn emit_tick_event(&self, frame: u64, events_committed: usize);
}

/// Default adapter wrapping any
/// [`Box<dyn chromaticity_observability::emit::EmitMetric>`].
///
/// Production code constructs this by wrapping a
/// [`chromaticity_observability::emit::InMemoryEmitter`] (Phase-1
/// stub) or a Phase-3+ runtime-bus emitter, then passes the
/// resulting `Box<dyn TickEmitter>` to
/// [`super::Scheduler::with_metric_emitter`].
///
/// The adapter emits one [`MetricEvent::RuntimeUptime`] per tick
/// carrying the post-tick frame counter as `value`. See module-
/// level prose for the Phase-1 catalog mapping rationale and the
/// Phase-3+ widening swap-in plan.
pub struct MetricEmitterAdapter {
    emitter: Box<dyn EmitMetric>,
}

impl MetricEmitterAdapter {
    /// Wrap any [`Box<dyn EmitMetric>`] for use with
    /// [`super::Scheduler::with_metric_emitter`].
    #[must_use]
    pub fn new(emitter: Box<dyn EmitMetric>) -> Self {
        Self { emitter }
    }
}

impl TickEmitter for MetricEmitterAdapter {
    fn emit_tick_event(&self, frame: u64, _events_committed: usize) {
        // Phase-1 catalog mapping: see module-level prose. The
        // `_events_committed` parameter is part of the trait
        // contract for forward compatibility — Phase-3+ catalog
        // widening (e.g. a dedicated `RuntimeFrameAdvanced` variant
        // carrying both labels) will consume it without touching
        // the trait surface. Slice-3 PR 1's overflow guard ensures
        // `frame` never wraps inside a successful tick, so the
        // gauge reading is monotonic.
        self.emitter
            .emit(MetricEvent::RuntimeUptime { value: frame });
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod scheduler_observability_pins {
    //! Evidence: EV-SCHEDULER-TICK (T1) — scheduler-tick
    //! observability emit baseline.
    //!
    //! T1 per docs/43 §15.5: each test pins a state-machine
    //! invariant the [`super::TickEmitter`] / [`super::super::Scheduler`]
    //! composition must honour for the slice-3 wave's downstream
    //! observability surface (Phase-1 baseline catalog round-trip;
    //! Phase-3+ catalog widening compatibility) to compose
    //! correctly. EV-SCHEDULER-TICK is registered in `docs/26` as
    //! `Planned`; the `Pass` transition lands in the slice-3
    //! closing docs PR after PRs 3 / 4 / 5 / 6 land.
    //!
    //! Invariants pinned here:
    //!
    //! - `tick_with_emitter_invokes_emit_once_per_tick` — a
    //!   single [`super::super::Scheduler::tick`] call invokes the
    //!   configured emitter exactly once.
    //! - `tick_without_emitter_is_noop_for_observability` — a
    //!   scheduler with `None` handler ticks successfully and
    //!   emits nothing (the slice-3 PR 1 invariants hold verbatim).
    //! - `tick_emitter_receives_post_tick_frame` — the emitter's
    //!   observed `frame` argument equals the post-increment
    //!   `state.frame()` value, matching the
    //!   [`super::super::TickReport::frame`] semantics.
    //! - `tick_emitter_receives_correct_events_committed_count` —
    //!   when N commands are dispatched onto the bus before a tick,
    //!   the emitter observes `events_committed = N`.

    use super::{MetricEmitterAdapter, TickEmitter};
    use crate::command_bus::CommandBus;
    use crate::command_bus::{Command, ControlId, ControlValue, InMemoryBus, PresetId};
    use crate::scheduler::Scheduler;
    use crate::state::State;
    use chromaticity_observability::catalog::MetricEvent;
    use chromaticity_observability::emit::InMemoryEmitter;
    use std::sync::Mutex;

    /// Mock emitter accumulating per-tick `(frame, events_committed)`
    /// observations for assertion. The Mutex is required because
    /// [`TickEmitter::emit_tick_event`] takes `&self`, mirroring the
    /// `EmitMetric::emit` shape.
    #[derive(Default)]
    struct RecordingTickEmitter {
        observations: Mutex<Vec<(u64, usize)>>,
    }

    impl TickEmitter for RecordingTickEmitter {
        fn emit_tick_event(&self, frame: u64, events_committed: usize) {
            self.observations
                .lock()
                .unwrap()
                .push((frame, events_committed));
        }
    }

    impl RecordingTickEmitter {
        fn observations(&self) -> Vec<(u64, usize)> {
            self.observations.lock().unwrap().clone()
        }
    }

    /// Adapter shape that lets the test reach into the recording
    /// emitter through the `Box<dyn TickEmitter>` indirection.
    /// `Arc` would be more flexible but the tests only need the
    /// post-tick observation, which is recoverable by leaking a
    /// raw pointer-shaped reference via the recorder's interior
    /// mutability — an `Arc<RecordingTickEmitter>` cannot be boxed
    /// as `Box<dyn TickEmitter>` directly because `Arc<T>: TickEmitter`
    /// is not implemented. The simplest path: own the recorder via
    /// `Arc`, hand a clone to the scheduler wrapped in an adapter
    /// trait impl on the `Arc`.
    struct ArcRecorder(std::sync::Arc<RecordingTickEmitter>);

    impl TickEmitter for ArcRecorder {
        fn emit_tick_event(&self, frame: u64, events_committed: usize) {
            self.0.emit_tick_event(frame, events_committed);
        }
    }

    #[test]
    fn tick_with_emitter_invokes_emit_once_per_tick() {
        let recorder = std::sync::Arc::new(RecordingTickEmitter::default());
        let mut scheduler =
            Scheduler::new().with_metric_emitter(Box::new(ArcRecorder(recorder.clone())));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        let _ = scheduler.tick(&mut state, &bus).expect("tick must succeed");

        let observations = recorder.observations();
        assert_eq!(
            observations.len(),
            1,
            "exactly one emit per tick; got {observations:?}"
        );
    }

    #[test]
    fn tick_without_emitter_is_noop_for_observability() {
        // Slice-3 PR 1 invariants must hold verbatim when no
        // observability handler is configured: tick succeeds, frame
        // advances, no panic.
        let mut scheduler = Scheduler::new();
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        let report = scheduler
            .tick(&mut state, &bus)
            .expect("tick must succeed without emitter");
        assert_eq!(report.frame, 1);
        assert_eq!(report.events_committed, 0);
        assert_eq!(state.frame(), 1);
    }

    #[test]
    fn tick_emitter_receives_post_tick_frame() {
        // The emitter observes the post-increment frame value
        // matching `TickReport::frame`. Drive two ticks and assert
        // both observations match the post-tick state.
        let recorder = std::sync::Arc::new(RecordingTickEmitter::default());
        let mut scheduler =
            Scheduler::new().with_metric_emitter(Box::new(ArcRecorder(recorder.clone())));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        let report_a = scheduler
            .tick(&mut state, &bus)
            .expect("tick A must succeed");
        let report_b = scheduler
            .tick(&mut state, &bus)
            .expect("tick B must succeed");

        let observations = recorder.observations();
        assert_eq!(observations.len(), 2);
        assert_eq!(observations[0].0, report_a.frame);
        assert_eq!(observations[0].0, 1);
        assert_eq!(observations[1].0, report_b.frame);
        assert_eq!(observations[1].0, 2);
        assert_eq!(state.frame(), 2);
    }

    #[test]
    fn tick_emitter_receives_correct_events_committed_count() {
        // Dispatch three commands; tick once; assert the emitter
        // observes events_committed == 3 matching
        // `TickReport::events_committed`.
        let recorder = std::sync::Arc::new(RecordingTickEmitter::default());
        let mut scheduler =
            Scheduler::new().with_metric_emitter(Box::new(ArcRecorder(recorder.clone())));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        bus.dispatch(Command::ControlSet {
            id: ControlId(1),
            value: ControlValue(0.1),
        })
        .expect("dispatch must succeed");
        bus.dispatch(Command::ControlSet {
            id: ControlId(2),
            value: ControlValue(0.2),
        })
        .expect("dispatch must succeed");
        bus.dispatch(Command::PresetRecall { id: PresetId(7) })
            .expect("dispatch must succeed");

        let report = scheduler.tick(&mut state, &bus).expect("tick must succeed");

        let observations = recorder.observations();
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].1, 3);
        assert_eq!(observations[0].1, report.events_committed);
    }

    #[test]
    fn metric_emitter_adapter_emits_runtime_uptime_with_post_tick_frame() {
        // The default `MetricEmitterAdapter` Phase-1 mapping emits
        // `MetricEvent::RuntimeUptime { value: frame }`. Pin the
        // mapping so a Phase-3+ catalog-widening swap-in is a
        // visible, intentional change rather than silent drift.
        let in_memory = std::sync::Arc::new(InMemoryEmitter::new());
        // The adapter takes `Box<dyn EmitMetric>` by value; clone
        // the `Arc` for read-side observation via a small helper.
        struct ArcInMemory(std::sync::Arc<InMemoryEmitter>);
        impl chromaticity_observability::emit::EmitMetric for ArcInMemory {
            fn emit(&self, event: MetricEvent) {
                self.0.emit(event);
            }
        }
        let adapter = MetricEmitterAdapter::new(Box::new(ArcInMemory(in_memory.clone())));
        let mut scheduler = Scheduler::new().with_metric_emitter(Box::new(adapter));
        let mut state = State::default();
        let bus = InMemoryBus::in_memory();

        let _ = scheduler.tick(&mut state, &bus).expect("tick must succeed");

        let drained = in_memory.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0], MetricEvent::RuntimeUptime { value: 1 });
    }
}
