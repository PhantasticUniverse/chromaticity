// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Observability emission site for the audio ingest boundary.
//!
//! Slice-2 wires three `docs/31` §Phase-1 baseline catalog counters
//! behind [`crate::ingest::IngestBoundary::try_pull`]:
//!
//! - `chromaticity_audio_ingest_frames_total` (`status` label)
//! - `chromaticity_audio_ingest_samples_total` (no labels)
//! - `chromaticity_audio_ingest_state_transitions_total`
//!   (`from`, `to` labels)
//!
//! The counter vocabulary keys off the `chromaticity-observability`
//! catalog enums ([`AudioIngestFrameStatus`], [`AudioIngestState`])
//! so the slice-3 runtime-bus wiring — which will replace the
//! in-memory table with `MetricEvent::AudioIngestFrames` /
//! `MetricEvent::AudioIngestSamples` /
//! `MetricEvent::AudioIngestStateTransition` emission through
//! [`chromaticity_observability::emit::EmitMetric`] — stays
//! observable-equivalent. This mirrors the slice-2 pattern seeded
//! by `integrations/osc-adapter/src/adapter.rs` (PR number 70) for
//! `OscPacketOutcome`.
//!
//! Slice-2 divergence from the osc-adapter pattern: osc keeps its
//! counts instance-bound on
//! `chromaticity_osc_adapter::UdpOscAdapter`; audio routes through
//! a process-wide `LazyLock<Mutex<AudioObservability>>`. Rationale:
//! [`crate::silent::SilentSource`] is the Phase-1 fallback ingest,
//! but the analyser will eventually select between multiple
//! [`crate::ingest::IngestBoundary`] implementors at runtime
//! (cpal-backed, file-backed, silent-fallback) via a
//! `Box<dyn IngestBoundary>`. Keeping the state-transition tracker
//! at module scope lets any implementor route through the same
//! emission site without the trait or its implementors growing an
//! observability parameter. Mutex (not atomics) because every
//! `try_pull` outcome mutates the tuple `(state,
//! consecutive_underruns, TransitionCounts, FrameStatusCounts,
//! samples_total)` atomically; an atomics variant would need a CAS
//! loop over the tracker state for larger contention cost than the
//! mutex at the 375 Hz hop cadence the §7.1 pipeline targets.
//!
//! TODO(slice-3): reconcile against the concrete audio-pipeline
//! lifecycle once the scheduler lands; the slice-2 tracker is a
//! best-effort synthesis, not the canonical lifecycle. This TODO
//! mirrors `engine/observability/src/catalog.rs` lines 121-126
//! verbatim in spirit so the two sides move together at the
//! slice-3 reconciliation.

use std::sync::{LazyLock, Mutex, MutexGuard};

use chromaticity_observability::catalog::{AudioIngestFrameStatus, AudioIngestState};

use crate::ingest::IngestStatus;

/// Per-[`AudioIngestFrameStatus`] try_pull-outcome counters keying
/// off `chromaticity_audio_ingest_frames_total`.
///
/// Named `u64` fields (not a map) so the compiler sees the full key
/// set at declaration time and the public getter below can keep a
/// wildcard arm for any future `#[non_exhaustive]`
/// [`AudioIngestFrameStatus`] variant without shifting the storage
/// shape.
#[derive(Debug, Default)]
struct FrameStatusCounts {
    ready: u64,
    would_block: u64,
    closed: u64,
}

/// Per-`(from, to)` transition counters keying off
/// `chromaticity_audio_ingest_state_transitions_total`.
///
/// Slice-2 reachable set under the brief's transition mapping:
///
/// - `Starting -> Running` (first `Ready`).
/// - `Running -> Draining` (two consecutive non-`Ready`; §7.11
///   rule in lockstep with `IngestState::Normal -> Degraded`).
/// - `Draining -> Running` (first `Ready` while Draining; §7.11
///   rule in lockstep with `IngestState::Degraded -> Normal`).
/// - `Draining -> Closed` (explicit `Closed` while Draining;
///   terminal).
///
/// Other catalog pairs (`Starting -> Closed`, `Starting ->
/// Draining`, `Running -> Closed`) are unreachable under the
/// slice-2 mapping — every terminal path runs through `Draining`
/// first. Slice-3 reconciliation against the concrete
/// audio-pipeline lifecycle (TODO at module top) may reify more
/// transitions; the `_ => 0` wildcard arm on
/// [`AudioIngestStateTracker::transition_count`] and the no-op
/// wildcard arm on [`AudioIngestStateTracker::record_transition`]
/// fence that future widening without breaking today's callers.
#[derive(Debug, Default)]
struct TransitionCounts {
    starting_to_running: u64,
    running_to_draining: u64,
    draining_to_running: u64,
    draining_to_closed: u64,
}

/// Slice-2 state-transition tracker for the audio ingest boundary.
///
/// Distinct from [`crate::ingest::IngestState`]: `IngestState` is
/// the two-mode (`Normal`/`Degraded`) §7.11 degraded-mode accounting
/// machine; this tracker is the four-state
/// `Starting -> Running -> Draining -> Closed` lifecycle the
/// `docs/31` §Phase-1 baseline catalog
/// `chromaticity_audio_ingest_state_transitions_total` counter
/// keys off. The two machines share the §7.11 "two consecutive
/// non-`Ready`" rule: that rule drives `Normal -> Degraded` in
/// [`crate::ingest::IngestState`] and `Running -> Draining` here.
///
/// Slice-2 mapping (verbatim from the emission-site brief):
///
/// - Tracker starts in [`AudioIngestState::Starting`].
/// - First `Ready` -> transition `Starting -> Running`.
/// - Two consecutive non-`Ready` -> transition `Running ->
///   Draining` (§7.11 rule).
/// - First `Ready` while `Draining` -> transition `Draining ->
///   Running` (matches `IngestState`'s `Degraded -> Normal`
///   recovery).
/// - `Closed` while `Draining` -> transition `Draining ->
///   Closed`.
/// - `Closed` is terminal: once in [`AudioIngestState::Closed`],
///   no further transitions (subsequent
///   [`AudioIngestStateTracker::observe`] calls are no-ops).
///
/// Pre-first-`Ready` non-`Ready` outcomes (`WouldBlock` / `Closed`
/// received while the tracker is still in `Starting`) do NOT count
/// toward the §7.11 threshold; `Starting` models the "not yet
/// producing" baseline, and the two-consecutive rule only fires
/// once the source has produced at least one `Ready`.
///
/// TODO(slice-3): reconcile against the concrete audio-pipeline
/// lifecycle once the scheduler lands; the slice-2 tracker is a
/// best-effort synthesis, not the canonical lifecycle. Mirrors
/// `engine/observability/src/catalog.rs::AudioIngestState` lines
/// 121-126.
#[derive(Debug)]
pub struct AudioIngestStateTracker {
    state: AudioIngestState,
    consecutive_underruns: u32,
    transitions: TransitionCounts,
}

impl Default for AudioIngestStateTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioIngestStateTracker {
    /// Construct a fresh tracker in [`AudioIngestState::Starting`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: AudioIngestState::Starting,
            consecutive_underruns: 0,
            transitions: TransitionCounts::default(),
        }
    }

    /// Current state, without consuming an outcome.
    #[must_use]
    pub fn current_state(&self) -> AudioIngestState {
        self.state
    }

    /// Count of recorded `(from -> to)` transitions since
    /// construction.
    ///
    /// Returns 0 for pairs unreachable under the slice-2 mapping
    /// (see [`TransitionCounts`] prose). Slice-3 widening adds
    /// fields + arms here without breaking existing callers.
    #[must_use]
    pub fn transition_count(&self, from: AudioIngestState, to: AudioIngestState) -> u64 {
        match (from, to) {
            (AudioIngestState::Starting, AudioIngestState::Running) => {
                self.transitions.starting_to_running
            }
            (AudioIngestState::Running, AudioIngestState::Draining) => {
                self.transitions.running_to_draining
            }
            (AudioIngestState::Draining, AudioIngestState::Running) => {
                self.transitions.draining_to_running
            }
            (AudioIngestState::Draining, AudioIngestState::Closed) => {
                self.transitions.draining_to_closed
            }
            _ => 0,
        }
    }

    /// Feed a `try_pull` outcome into the tracker. Returns the
    /// post-transition state (unchanged on a self-loop).
    pub fn observe(&mut self, status: AudioIngestFrameStatus) -> AudioIngestState {
        if matches!(self.state, AudioIngestState::Closed) {
            return self.state;
        }
        match self.state {
            AudioIngestState::Starting => {
                if matches!(status, AudioIngestFrameStatus::Ready) {
                    self.record_transition(AudioIngestState::Starting, AudioIngestState::Running);
                    self.state = AudioIngestState::Running;
                    self.consecutive_underruns = 0;
                }
                // Pre-first-`Ready` non-`Ready` outcomes do not
                // count toward §7.11 — see the module-level prose.
            }
            AudioIngestState::Running => match status {
                AudioIngestFrameStatus::Ready => {
                    self.consecutive_underruns = 0;
                }
                AudioIngestFrameStatus::WouldBlock | AudioIngestFrameStatus::Closed => {
                    self.consecutive_underruns = self.consecutive_underruns.saturating_add(1);
                    if self.consecutive_underruns >= 2 {
                        self.record_transition(
                            AudioIngestState::Running,
                            AudioIngestState::Draining,
                        );
                        self.state = AudioIngestState::Draining;
                        self.consecutive_underruns = 0;
                    }
                }
                _ => {
                    // Future `#[non_exhaustive]`
                    // AudioIngestFrameStatus variant; slice-3
                    // extends this arm explicitly.
                }
            },
            AudioIngestState::Draining => match status {
                AudioIngestFrameStatus::Ready => {
                    self.record_transition(AudioIngestState::Draining, AudioIngestState::Running);
                    self.state = AudioIngestState::Running;
                    self.consecutive_underruns = 0;
                }
                AudioIngestFrameStatus::Closed => {
                    self.record_transition(AudioIngestState::Draining, AudioIngestState::Closed);
                    self.state = AudioIngestState::Closed;
                    self.consecutive_underruns = 0;
                }
                AudioIngestFrameStatus::WouldBlock => {
                    // WouldBlock while Draining keeps us Draining;
                    // the terminal transition requires an explicit
                    // `Closed` per the slice-2 mapping.
                }
                _ => {
                    // Future `#[non_exhaustive]` variant.
                }
            },
            AudioIngestState::Closed => {
                // Guarded by the early return above; kept as an
                // explicit arm so adding an AudioIngestState
                // variant upstream triggers a non-exhaustive-match
                // error here, not a silent fall-through.
            }
            _ => {
                // Future AudioIngestState variant; no-op until
                // slice-3 reconciliation reifies the mapping.
            }
        }
        self.state
    }

    fn record_transition(&mut self, from: AudioIngestState, to: AudioIngestState) {
        match (from, to) {
            (AudioIngestState::Starting, AudioIngestState::Running) => {
                self.transitions.starting_to_running =
                    self.transitions.starting_to_running.saturating_add(1);
            }
            (AudioIngestState::Running, AudioIngestState::Draining) => {
                self.transitions.running_to_draining =
                    self.transitions.running_to_draining.saturating_add(1);
            }
            (AudioIngestState::Draining, AudioIngestState::Running) => {
                self.transitions.draining_to_running =
                    self.transitions.draining_to_running.saturating_add(1);
            }
            (AudioIngestState::Draining, AudioIngestState::Closed) => {
                self.transitions.draining_to_closed =
                    self.transitions.draining_to_closed.saturating_add(1);
            }
            _ => {
                // Unreachable under the slice-2 mapping. Slice-3
                // widening adds TransitionCounts fields + arms.
            }
        }
    }
}

/// Process-wide audio observability state.
struct AudioObservability {
    frames: FrameStatusCounts,
    samples: u64,
    tracker: AudioIngestStateTracker,
}

static AUDIO_OBS: LazyLock<Mutex<AudioObservability>> = LazyLock::new(|| {
    Mutex::new(AudioObservability {
        frames: FrameStatusCounts::default(),
        samples: 0,
        tracker: AudioIngestStateTracker::new(),
    })
});

fn lock_obs() -> MutexGuard<'static, AudioObservability> {
    AUDIO_OBS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Map a [`IngestStatus`] to the catalog-side
/// [`AudioIngestFrameStatus`] label value.
///
/// The brief pins the mapping 1:1; the variant names match
/// exactly. [`IngestStatus`] is defined in this crate and never
/// grows without a paired catalog update, so the exhaustive match
/// here is deliberate.
fn map_status(status: &IngestStatus) -> AudioIngestFrameStatus {
    match status {
        IngestStatus::Ready => AudioIngestFrameStatus::Ready,
        IngestStatus::WouldBlock => AudioIngestFrameStatus::WouldBlock,
        IngestStatus::Closed => AudioIngestFrameStatus::Closed,
    }
}

/// Record one `try_pull` outcome against the process-wide
/// emission-site counters and feed it into the singleton tracker.
///
/// Callers pass the samples-count from the [`crate::frame::AudioFrame`]
/// populated by `try_pull`; only the `Ready` branch advances the
/// samples counter. Every [`crate::ingest::IngestBoundary`]
/// implementor MUST call this at the outcome boundary of
/// `try_pull` so the three audio catalog metrics accumulate for
/// the `EV-OBSERVABILITY` exit gate.
pub(crate) fn record_pull_outcome(status: &IngestStatus, samples_len: usize) {
    let frame_status = map_status(status);
    let mut obs = lock_obs();
    match frame_status {
        AudioIngestFrameStatus::Ready => {
            obs.frames.ready = obs.frames.ready.saturating_add(1);
            obs.samples = obs.samples.saturating_add(samples_len as u64);
        }
        AudioIngestFrameStatus::WouldBlock => {
            obs.frames.would_block = obs.frames.would_block.saturating_add(1);
        }
        AudioIngestFrameStatus::Closed => {
            obs.frames.closed = obs.frames.closed.saturating_add(1);
        }
        _ => {
            // Future `#[non_exhaustive]` catalog variant; no
            // counter surface to update until slice-3 widens
            // FrameStatusCounts.
        }
    }
    obs.tracker.observe(frame_status);
}

/// Current count of `try_pull` outcomes mapped to `status`.
///
/// Slice-2 interim surface for
/// `chromaticity_audio_ingest_frames_total`; slice-3 replaces the
/// in-memory counter with `MetricEvent::AudioIngestFrames` routed
/// through the runtime bus. Counts are monotonic across the
/// process lifetime.
///
/// The wildcard arm reads 0 for future `#[non_exhaustive]`
/// [`AudioIngestFrameStatus`] variants that slice-2 has not been
/// taught to produce.
#[must_use]
pub fn frames_count(status: AudioIngestFrameStatus) -> u64 {
    let obs = lock_obs();
    match status {
        AudioIngestFrameStatus::Ready => obs.frames.ready,
        AudioIngestFrameStatus::WouldBlock => obs.frames.would_block,
        AudioIngestFrameStatus::Closed => obs.frames.closed,
        _ => 0,
    }
}

/// Total PCM samples delivered through the ingest boundary since
/// process start.
///
/// Slice-2 interim surface for
/// `chromaticity_audio_ingest_samples_total`.
#[must_use]
pub fn samples_count() -> u64 {
    lock_obs().samples
}

/// Count of recorded `(from -> to)` transitions on the
/// process-wide tracker.
///
/// Slice-2 interim surface for
/// `chromaticity_audio_ingest_state_transitions_total`. Returns 0
/// for pairs unreachable under the slice-2 mapping.
#[must_use]
pub fn transition_count(from: AudioIngestState, to: AudioIngestState) -> u64 {
    lock_obs().tracker.transition_count(from, to)
}

/// Test-only serialization lock.
///
/// The observability emission site is a process-wide
/// [`LazyLock<Mutex<AudioObservability>>`]. Tests that read the
/// resulting counters via [`frames_count`] / [`samples_count`] /
/// [`transition_count`] need to serialize against concurrent
/// `try_pull`-path writers so that before/after delta semantics
/// the brief pins are deterministic under cargo-test's default
/// parallel execution.
///
/// Every test that either (a) reads the three getters above or
/// (b) calls an [`crate::ingest::IngestBoundary::try_pull`]
/// implementor MUST hold this guard across the test body.
#[cfg(test)]
pub(crate) fn test_serialize_lock() -> MutexGuard<'static, ()> {
    static TEST_LOCK: Mutex<()> = Mutex::new(());
    TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
