// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-AUDIO-TRACE (T2) — slice-3a capability-negotiation
//! corpus.
//!
//! T2 per `docs/43 §15.5`: end-to-end exercise of
//! [`chromaticity_audio::IngestNegotiator`] over synthesised
//! [`chromaticity_audio::ingest::IngestStatus`] sequences. Each test
//! pins a complete demote / recover transition path against the
//! configurable [`chromaticity_audio::IngestModeCriteria`] thresholds.
//! The slice-2 process-wide observability counters retain their
//! semantics — the negotiator's per-instance state is layered above
//! `record_pull_outcome`, never replacing it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::VecDeque;

use chromaticity_audio::frame::{AudioFrame, DEFAULT_SAMPLE_RATE_HZ};
use chromaticity_audio::ingest::{IngestBoundary, IngestStatus};
use chromaticity_audio::ingest_negotiation::{
    DEFAULT_DEGRADED_AFTER_FAILURES, DEFAULT_NORMAL_AFTER_SUCCESSES, IngestModeCriteria,
    IngestNegotiator, IngestSource, MAX_THRESHOLD,
};
use chromaticity_audio::{AudioError, BpmEstimator};

/// Programmable [`IngestBoundary`] yielding a deterministic status
/// sequence.
struct ScriptedSource(VecDeque<IngestStatus>);

impl ScriptedSource {
    fn new(statuses: Vec<IngestStatus>) -> Self {
        Self(statuses.into())
    }
}

impl IngestBoundary for ScriptedSource {
    fn try_pull(&mut self, out: &mut AudioFrame) -> IngestStatus {
        let status = self.0.pop_front().unwrap_or(IngestStatus::Closed);
        if matches!(status, IngestStatus::Ready) {
            *out = AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, vec![0.1; 128]);
        }
        status
    }
}

#[test]
fn five_successes_then_three_failures_demotes() {
    let mut statuses = vec![IngestStatus::Ready; 5];
    statuses.extend(vec![IngestStatus::WouldBlock; 3]);
    let mut neg = IngestNegotiator::new(ScriptedSource::new(statuses))
        .with_degraded_after(3)
        .unwrap();
    let mut frame = AudioFrame::default();

    for _ in 0..5 {
        let s = neg.try_pull(&mut frame);
        assert_eq!(s, IngestStatus::Ready);
        assert_eq!(neg.active_source(), IngestSource::Primary);
    }
    assert_eq!(neg.primary_consecutive_successes(), 5);
    let _ = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::Primary);
    let _ = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::Primary);
    let s = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::SilentFallback);
    assert_eq!(s, IngestStatus::Ready);
}

#[test]
fn five_failures_then_first_ready_returns_to_normal() {
    let mut statuses = vec![IngestStatus::WouldBlock; 5];
    statuses.push(IngestStatus::Ready);
    let mut neg = IngestNegotiator::new(ScriptedSource::new(statuses));
    let mut frame = AudioFrame::default();
    for _ in 0..5 {
        let _ = neg.try_pull(&mut frame);
    }
    assert_eq!(neg.active_source(), IngestSource::SilentFallback);
    let s = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::Primary);
    assert_eq!(s, IngestStatus::Ready);
}

#[test]
fn closed_and_would_block_both_count_toward_failure_threshold() {
    let mut neg = IngestNegotiator::new(ScriptedSource::new(vec![
        IngestStatus::Closed,
        IngestStatus::WouldBlock,
    ]));
    let mut frame = AudioFrame::default();
    let _ = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::Primary);
    let _ = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::SilentFallback);
}

#[test]
fn intermittent_success_resets_failure_counter() {
    let mut neg = IngestNegotiator::new(ScriptedSource::new(vec![
        IngestStatus::WouldBlock,
        IngestStatus::WouldBlock,
        IngestStatus::Ready,
        IngestStatus::WouldBlock,
        IngestStatus::WouldBlock,
    ]));
    let mut frame = AudioFrame::default();
    let _ = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::Primary);
    let _ = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::SilentFallback);
    let _ = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::Primary);
    let _ = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::Primary);
    let _ = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::SilentFallback);
}

#[test]
fn fallback_serves_silent_frames_at_canonical_shape() {
    let mut neg = IngestNegotiator::new(ScriptedSource::new(vec![
        IngestStatus::Closed,
        IngestStatus::Closed,
    ]));
    let mut frame = AudioFrame::default();
    let _ = neg.try_pull(&mut frame);
    let s = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::SilentFallback);
    assert_eq!(s, IngestStatus::Ready);
    assert_eq!(frame.sample_rate_hz(), DEFAULT_SAMPLE_RATE_HZ);
    assert_eq!(frame.channels(), 1);
    assert_eq!(frame.samples().len(), 128);
    assert!(frame.samples().iter().all(|&v| v == 0.0));
}

#[test]
fn debounced_recovery_holds_fallback_until_threshold_met() {
    let mut statuses = vec![IngestStatus::WouldBlock; 2];
    statuses.extend(vec![IngestStatus::Ready; 4]);
    let mut neg = IngestNegotiator::new(ScriptedSource::new(statuses))
        .with_normal_after(4)
        .unwrap();
    let mut frame = AudioFrame::default();
    let _ = neg.try_pull(&mut frame);
    let _ = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::SilentFallback);
    for _ in 0..3 {
        let _ = neg.try_pull(&mut frame);
        assert_eq!(neg.active_source(), IngestSource::SilentFallback);
    }
    let _ = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::Primary);
}

#[test]
fn fallback_exhaustion_propagates_closed() {
    let mut statuses = vec![IngestStatus::WouldBlock; 2];
    statuses.extend(vec![IngestStatus::Closed; 4]);
    let mut neg = IngestNegotiator::new(ScriptedSource::new(statuses)).with_fallback_budget(1);
    let mut frame = AudioFrame::default();
    let _ = neg.try_pull(&mut frame);
    let s = neg.try_pull(&mut frame);
    assert_eq!(neg.active_source(), IngestSource::SilentFallback);
    assert_eq!(s, IngestStatus::Ready);
    let s = neg.try_pull(&mut frame);
    assert_eq!(s, IngestStatus::Closed);
}

#[test]
fn negotiator_round_trips_through_dyn_ingest_boundary() {
    let mut neg: Box<dyn IngestBoundary> =
        Box::new(IngestNegotiator::new(ScriptedSource::new(vec![
            IngestStatus::Ready,
            IngestStatus::Ready,
        ])));
    let mut frame = AudioFrame::default();
    assert_eq!(neg.try_pull(&mut frame), IngestStatus::Ready);
    assert_eq!(neg.try_pull(&mut frame), IngestStatus::Ready);
}

#[test]
fn criteria_validation_surfaces_ingest_negotiation_error() {
    let Err(err) = IngestNegotiator::new(ScriptedSource::new(vec![])).with_degraded_after(0) else {
        panic!("with_degraded_after(0) must reject");
    };
    assert!(matches!(err, AudioError::IngestNegotiation { .. }));
    assert!(format!("{err}").contains("ingest-negotiation"));
}

#[test]
fn slice_2_bpm_estimator_unaffected_by_negotiator_addition() {
    // Regression pin: the slice-2 BpmEstimator surface keeps its
    // shape across the slice-3a addition.
    drop(BpmEstimator::new());
    let crit = IngestModeCriteria::default();
    assert_eq!(crit.degraded_after, DEFAULT_DEGRADED_AFTER_FAILURES);
    assert_eq!(crit.normal_after, DEFAULT_NORMAL_AFTER_SUCCESSES);
    const _: () = assert!(MAX_THRESHOLD >= 256);
    const _: () = assert!(MAX_THRESHOLD <= 1 << 16);
}
