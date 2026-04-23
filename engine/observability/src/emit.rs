// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Metric emission trait and in-process stub implementation.
//!
//! Slice-2 ships the object-safe [`EmitMetric`] trait plus an
//! [`InMemoryEmitter`] stub that round-trips [`crate::catalog::MetricEvent`]
//! values through an internal `Mutex<Vec<_>>`. The stub honours
//! ADR-016 posture: no network I/O, no filesystem I/O, no panic
//! paths on lock poisoning.
//!
//! Slice-3 wires the runtime bus against `Box<dyn EmitMetric>` so
//! subsystem events feed into whatever sink the app shell configures
//! (the stub for tests, a Prometheus exposition endpoint for the
//! Phase 4d loopback-only health surface, a support-bundle collector
//! for the opt-in export path, and so on).

use std::sync::{Mutex, PoisonError};

use crate::catalog::MetricEvent;

/// Object-safe sink for metric events.
///
/// The trait surface is intentionally narrow: one method accepting
/// a [`MetricEvent`]. Implementations decide what to do with it
/// (buffer in-process, emit to a transport, redact and forward,
/// drop on the floor). ADR-016 permits only local in-process sinks
/// at Phase 1; a transport-backed implementation lands with the
/// Phase 4d loopback-only Prometheus endpoint.
pub trait EmitMetric: Send + Sync {
    /// Emit one metric event.
    ///
    /// Implementations MUST NOT panic on emit. The Phase-1 stub
    /// coerces Mutex poison into a best-effort append and keeps
    /// going; transport-backed implementations MUST similarly
    /// coerce any I/O failure into a silent drop (ADR-016 §Failure
    /// posture).
    fn emit(&self, event: MetricEvent);
}

/// In-process, no-I/O stub emitter. Appends events to an internal
/// vector protected by a `Mutex`; [`Self::drain`] returns and clears
/// the buffer.
///
/// Used directly by the slice-2 drift pins. Slice-3 tests that need
/// to observe telemetry use this implementation as the default test
/// sink.
#[derive(Debug, Default)]
pub struct InMemoryEmitter {
    events: Mutex<Vec<MetricEvent>>,
}

impl InMemoryEmitter {
    /// Construct an empty emitter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    /// Drain every buffered event in emission order.
    ///
    /// On Mutex poison the function returns the inner vector via
    /// [`PoisonError::into_inner`] rather than panicking, matching
    /// the trait-level "no panic on emit" contract.
    pub fn drain(&self) -> Vec<MetricEvent> {
        let mut guard = self.events.lock().unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut *guard)
    }

    /// Number of events currently buffered. Useful for readiness
    /// checks without draining.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events
            .lock()
            .map(|g| g.len())
            .unwrap_or_else(|e| e.into_inner().len())
    }

    /// `true` iff no events are buffered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl EmitMetric for InMemoryEmitter {
    fn emit(&self, event: MetricEvent) {
        match self.events.lock() {
            Ok(mut guard) => guard.push(event),
            Err(poison) => poison.into_inner().push(event),
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod ctor_placeholder {
    //! Evidence: EV-OBSERVABILITY (T0) — trait object-safety and
    //! stub-emitter round-trip baseline.
    //!
    //! T0 per docs/43 §15.5: the trait surface is object-safe and
    //! the stub has no non-derive-generated behaviour beyond vec
    //! push / drain. T1 upgrade arrives with slice-3 runtime-bus
    //! wiring; T2 with the Phase-1-exit conformance harness.

    use super::{EmitMetric, InMemoryEmitter};
    use crate::catalog::{CommandKind, EventKind, MetricEvent};

    #[test]
    fn emit_metric_trait_is_object_safe() {
        // Object-safety: the trait MUST be usable behind a trait
        // object so the slice-3 runtime bus can hold a
        // `Box<dyn EmitMetric>` selected at startup time.
        let emitter: Box<dyn EmitMetric> = Box::new(InMemoryEmitter::new());
        emitter.emit(MetricEvent::RuntimeCommandsDispatched {
            kind: CommandKind::ControlSet,
            value: 1,
        });
    }

    #[test]
    fn in_memory_emitter_roundtrips_events_in_order() {
        let emitter = InMemoryEmitter::new();
        assert!(emitter.is_empty());

        emitter.emit(MetricEvent::RuntimeCommandsDispatched {
            kind: CommandKind::ControlSet,
            value: 1,
        });
        emitter.emit(MetricEvent::RuntimeEventsEmitted {
            kind: EventKind::ControlChanged,
            value: 1,
        });
        emitter.emit(MetricEvent::RuntimeCommandsDispatched {
            kind: CommandKind::PresetRecall,
            value: 2,
        });

        assert_eq!(emitter.len(), 3);
        let drained = emitter.drain();
        assert_eq!(drained.len(), 3);

        // Drain preserves emission order.
        assert_eq!(
            drained[0],
            MetricEvent::RuntimeCommandsDispatched {
                kind: CommandKind::ControlSet,
                value: 1,
            }
        );
        assert_eq!(
            drained[2],
            MetricEvent::RuntimeCommandsDispatched {
                kind: CommandKind::PresetRecall,
                value: 2,
            }
        );

        // Drain clears the buffer.
        assert!(emitter.is_empty());
    }
}
