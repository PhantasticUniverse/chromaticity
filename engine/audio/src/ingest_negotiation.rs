// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-2 EV-AUDIO-TRACE slice-3a ingest-mode capability negotiation.
//!
//! Slice-2 (PR number 267) wired the [`crate::ingest::IngestMode`]
//! Normal/Degraded state machine and the process-wide observability
//! mirror via `record_pull_outcome`. A single ingest source has no
//! decision point, though: when the primary degrades it keeps
//! returning `WouldBlock`/`Closed` and the analyser has nothing to
//! fall back to.
//!
//! Slice-3a closes that gap on the *negotiation side*: an
//! [`IngestNegotiator`] wraps a primary [`IngestBoundary`] plus an
//! internal [`crate::silent::SilentSource`] fallback and a
//! configurable [`IngestModeCriteria`] policy. When the primary
//! crosses the policy's failure threshold, the negotiator flips the
//! active source to the fallback; when the primary recovers, it flips
//! back.
//!
//! # Scope boundary (slice-3a vs slice-3b)
//!
//! Slice-3a operates entirely on the existing
//! [`IngestBoundary::try_pull`] surface. **Slice-3a does NOT add cpal
//! device-capture integration.** The [`IngestSource`] enum reserves a
//! `#[non_exhaustive]` [`IngestSource::Cpal`] slot for slice-3b but
//! never selects it from production code.
//!
//! # Counter semantics
//!
//! Slice-2 process-wide observability counters retain their meaning
//! unchanged: every underlying source's `try_pull` still calls
//! `record_pull_outcome`. The negotiator's per-instance counters are
//! layered above and never replace the global tracker.
//!
//! Governing docs: `docs/06-audio-engine.md` "Reliability/Fallback
//! Policy" (fall back to silent on device failure) and ADR-016 (audio-
//! ingest counter family).

use crate::error::AudioError;
use crate::frame::AudioFrame;
use crate::ingest::{IngestBoundary, IngestStatus};
use crate::silent::SilentSource;

/// Default consecutive-failure threshold before the negotiator flips
/// to the fallback. Matches the Reliability/Fallback Policy "two consecutive non-Ready"
/// baseline that [`crate::ingest::IngestState`] pins.
pub const DEFAULT_DEGRADED_AFTER_FAILURES: u32 = 2;

/// Default consecutive-success threshold before the negotiator flips
/// back to the primary. Matches the Reliability/Fallback Policy "first Ready returns to
/// Normal" baseline.
pub const DEFAULT_NORMAL_AFTER_SUCCESSES: u32 = 1;

/// Sanity ceiling on either threshold; values above this would
/// effectively disable the path the threshold gates.
pub const MAX_THRESHOLD: u32 = 1_024;

/// Identity tag for the active ingest source the negotiator drives.
///
/// Production-path variants in slice-3a: [`IngestSource::Primary`]
/// (the analyser-supplied source) and [`IngestSource::SilentFallback`]
/// (the slice-2 graduated [`crate::silent::SilentSource`]).
/// [`IngestSource::Cpal`] is a slice-3b reserved slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IngestSource {
    /// Caller-supplied primary [`IngestBoundary`].
    Primary,
    /// Negotiator's silent fallback ([`crate::silent::SilentSource`]).
    SilentFallback,
    /// Slice-3b reserved slot for the cpal-driven device-capture
    /// source. Slice-3a never selects this variant in production
    /// code.
    Cpal,
}

/// Configurable transition criteria for [`IngestNegotiator`].
///
/// Slice-3a transition rule:
///
/// - Negotiator starts on the primary source.
/// - Each non-`Ready` outcome from the primary increments the
///   primary failure counter; each `Ready` resets it (and increments
///   the primary success counter).
/// - When `primary_consecutive_failures >= degraded_after`, demote
///   to the fallback.
/// - While on the fallback, the negotiator probes the primary on
///   every pull. When `primary_consecutive_successes >= normal_after`
///   the negotiator promotes back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IngestModeCriteria {
    /// Failure-count threshold on the primary before demotion.
    /// Validated `1..=MAX_THRESHOLD`.
    pub degraded_after: u32,
    /// Success-count threshold on the primary before promotion.
    /// Validated `1..=MAX_THRESHOLD`.
    pub normal_after: u32,
}

impl Default for IngestModeCriteria {
    fn default() -> Self {
        Self {
            degraded_after: DEFAULT_DEGRADED_AFTER_FAILURES,
            normal_after: DEFAULT_NORMAL_AFTER_SUCCESSES,
        }
    }
}

impl IngestModeCriteria {
    /// Validate a criteria pair. Both fields must satisfy
    /// `1..=MAX_THRESHOLD`.
    pub fn validate(&self) -> Result<(), AudioError> {
        if self.degraded_after == 0 {
            return Err(AudioError::IngestNegotiation {
                reason: "degraded_after must be at least 1; 0 would flip on every pull".into(),
            });
        }
        if self.normal_after == 0 {
            return Err(AudioError::IngestNegotiation {
                reason: "normal_after must be at least 1; 0 would flip on every pull".into(),
            });
        }
        if self.degraded_after > MAX_THRESHOLD {
            return Err(AudioError::IngestNegotiation {
                reason: format!(
                    "degraded_after must be at most {MAX_THRESHOLD}; larger values disable the fallback"
                ),
            });
        }
        if self.normal_after > MAX_THRESHOLD {
            return Err(AudioError::IngestNegotiation {
                reason: format!(
                    "normal_after must be at most {MAX_THRESHOLD}; larger values disable the recovery"
                ),
            });
        }
        Ok(())
    }
}

/// Capability-negotiating ingest source.
///
/// Wraps a primary [`IngestBoundary`] plus an internal silent
/// fallback. On each [`IngestBoundary::try_pull`] the negotiator
/// pulls from the active source, applies the configured
/// [`IngestModeCriteria`], and may flip the active source for the
/// next pull. Implements [`IngestBoundary`] so the analyser can
/// store it as `Box<dyn IngestBoundary>`.
///
/// # Recovery probing
///
/// While the fallback is active, the negotiator additionally probes
/// the primary on every pull (writing the probe into a scratch
/// frame) so the recovery counter can fire. The analyser-visible
/// frame written to `out` always comes from the source that
/// successfully served the hop.
pub struct IngestNegotiator {
    primary: Box<dyn IngestBoundary + Send>,
    fallback: SilentSource,
    criteria: IngestModeCriteria,
    active: IngestSource,
    primary_consecutive_failures: u32,
    primary_consecutive_successes: u32,
}

impl core::fmt::Debug for IngestNegotiator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("IngestNegotiator")
            .field("active", &self.active)
            .field("criteria", &self.criteria)
            .field("primary_failures", &self.primary_consecutive_failures)
            .field("primary_successes", &self.primary_consecutive_successes)
            .finish_non_exhaustive()
    }
}

impl IngestNegotiator {
    /// Build a negotiator over `primary` with default criteria
    /// ([`IngestModeCriteria::default`]) and an effectively unbounded
    /// silent fallback (`u32::MAX` hops).
    pub fn new<P>(primary: P) -> Self
    where
        P: IngestBoundary + Send + 'static,
    {
        Self {
            primary: Box::new(primary),
            fallback: SilentSource::new(u32::MAX),
            criteria: IngestModeCriteria::default(),
            active: IngestSource::Primary,
            primary_consecutive_failures: 0,
            primary_consecutive_successes: 0,
        }
    }

    /// Override the failure-count threshold for demotion.
    pub fn with_degraded_after(mut self, failures: u32) -> Result<Self, AudioError> {
        self.criteria.degraded_after = failures;
        self.criteria.validate()?;
        Ok(self)
    }

    /// Override the success-count threshold for promotion.
    pub fn with_normal_after(mut self, successes: u32) -> Result<Self, AudioError> {
        self.criteria.normal_after = successes;
        self.criteria.validate()?;
        Ok(self)
    }

    /// Override the fallback's hop budget (default `u32::MAX`).
    /// Useful for harnesses exercising the `Closed`-on-exhaustion
    /// path.
    #[must_use]
    pub fn with_fallback_budget(mut self, hops: u32) -> Self {
        self.fallback = SilentSource::new(hops);
        self
    }

    /// Replace the criteria wholesale; validates before applying.
    pub fn with_criteria(mut self, criteria: IngestModeCriteria) -> Result<Self, AudioError> {
        criteria.validate()?;
        self.criteria = criteria;
        Ok(self)
    }

    /// Source the negotiator will pull from on the next call.
    #[must_use]
    pub fn active_source(&self) -> IngestSource {
        self.active
    }

    /// Current criteria.
    #[must_use]
    pub fn criteria(&self) -> IngestModeCriteria {
        self.criteria
    }

    /// Consecutive-failure count on the primary since its last
    /// `Ready` (or since construction / promotion).
    #[must_use]
    pub fn primary_consecutive_failures(&self) -> u32 {
        self.primary_consecutive_failures
    }

    /// Consecutive-success count on the primary since its last
    /// non-`Ready` (or since construction).
    #[must_use]
    pub fn primary_consecutive_successes(&self) -> u32 {
        self.primary_consecutive_successes
    }

    fn record_primary(&mut self, status: &IngestStatus) {
        match status {
            IngestStatus::Ready => {
                self.primary_consecutive_successes =
                    self.primary_consecutive_successes.saturating_add(1);
                self.primary_consecutive_failures = 0;
            }
            IngestStatus::WouldBlock | IngestStatus::Closed => {
                self.primary_consecutive_failures =
                    self.primary_consecutive_failures.saturating_add(1);
                self.primary_consecutive_successes = 0;
            }
        }
    }

    fn maybe_demote(&mut self) {
        if matches!(self.active, IngestSource::Primary)
            && self.primary_consecutive_failures >= self.criteria.degraded_after
        {
            self.active = IngestSource::SilentFallback;
        }
    }

    fn maybe_promote(&mut self) {
        if matches!(self.active, IngestSource::SilentFallback)
            && self.primary_consecutive_successes >= self.criteria.normal_after
        {
            self.active = IngestSource::Primary;
            // Reset the failure counter on promotion so the next
            // demotion needs the full degraded_after threshold again.
            self.primary_consecutive_failures = 0;
        }
    }
}

impl IngestBoundary for IngestNegotiator {
    fn try_pull(&mut self, out: &mut AudioFrame) -> IngestStatus {
        match self.active {
            IngestSource::Primary => {
                let status = self.primary.try_pull(out);
                self.record_primary(&status);
                self.maybe_demote();
                if matches!(status, IngestStatus::Ready) {
                    return status;
                }
                // Primary failed; if we just demoted, serve the
                // fallback this hop so the analyser still gets a
                // frame. Otherwise propagate the primary's
                // non-Ready status (still on the primary,
                // pre-threshold).
                if matches!(self.active, IngestSource::SilentFallback) {
                    self.fallback.try_pull(out)
                } else {
                    status
                }
            }
            IngestSource::SilentFallback => {
                // Probe the primary into a scratch frame so the
                // probe never overwrites the fallback's frame
                // contents the analyser actually consumes.
                let mut probe = AudioFrame::default();
                let probe_status = self.primary.try_pull(&mut probe);
                self.record_primary(&probe_status);
                self.maybe_promote();
                if matches!(self.active, IngestSource::Primary)
                    && matches!(probe_status, IngestStatus::Ready)
                {
                    *out = probe;
                    return IngestStatus::Ready;
                }
                self.fallback.try_pull(out)
            }
            IngestSource::Cpal => {
                // Slice-3a never selects Cpal; safe-default to the
                // fallback so future slice-3b code that flips the
                // tag through an ill-formed path still serves a
                // frame.
                self.fallback.try_pull(out)
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod negotiation_pins {
    //! Evidence: EV-AUDIO-TRACE (T1) — drift pins on the slice-3a
    //! capability-negotiation surface.
    //!
    //! T1 per `docs/43 §15.5`: each test pins either a builder-
    //! validation invariant ([`IngestModeCriteria::validate`] reject
    //! paths) or a documented state-machine invariant on
    //! [`IngestNegotiator`] (default criteria match Reliability/Fallback Policy baseline,
    //! demotion threshold, promotion threshold, fallback serves a
    //! frame on demotion, criteria getters round-trip).

    use super::*;

    /// Programmable [`IngestBoundary`] yielding a deterministic
    /// status sequence; pure helper, doesn't touch global counters.
    #[derive(Debug)]
    struct ScriptedSource(std::collections::VecDeque<IngestStatus>);

    impl ScriptedSource {
        fn new(v: Vec<IngestStatus>) -> Self {
            Self(v.into())
        }
    }

    impl IngestBoundary for ScriptedSource {
        fn try_pull(&mut self, out: &mut AudioFrame) -> IngestStatus {
            let status = self.0.pop_front().unwrap_or(IngestStatus::Closed);
            if matches!(status, IngestStatus::Ready) {
                *out = AudioFrame::new(48_000, 1, vec![0.0; 128]);
            }
            status
        }
    }

    #[test]
    fn default_criteria_match_section_7_11_baseline() {
        let crit = IngestModeCriteria::default();
        assert_eq!(crit.degraded_after, 2);
        assert_eq!(crit.normal_after, 1);
        crit.validate().unwrap();
    }

    #[test]
    fn criteria_rejects_zero_thresholds() {
        let err = IngestModeCriteria {
            degraded_after: 0,
            normal_after: 1,
        }
        .validate()
        .unwrap_err();
        assert!(matches!(err, AudioError::IngestNegotiation { .. }));
        assert!(
            IngestModeCriteria {
                degraded_after: 1,
                normal_after: 0,
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn criteria_rejects_threshold_above_ceiling() {
        assert!(
            IngestModeCriteria {
                degraded_after: MAX_THRESHOLD + 1,
                normal_after: 1,
            }
            .validate()
            .is_err()
        );
        assert!(
            IngestModeCriteria {
                degraded_after: 1,
                normal_after: MAX_THRESHOLD + 1,
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn negotiator_starts_on_primary() {
        let neg = IngestNegotiator::new(ScriptedSource::new(vec![]));
        assert_eq!(neg.active_source(), IngestSource::Primary);
        assert_eq!(neg.primary_consecutive_failures(), 0);
        assert_eq!(neg.primary_consecutive_successes(), 0);
    }

    #[test]
    fn negotiator_demotes_after_threshold_failures() {
        let mut neg = IngestNegotiator::new(ScriptedSource::new(vec![
            IngestStatus::WouldBlock,
            IngestStatus::WouldBlock,
        ]))
        .with_degraded_after(2)
        .unwrap();
        let mut frame = AudioFrame::default();
        let _ = neg.try_pull(&mut frame);
        assert_eq!(neg.active_source(), IngestSource::Primary);
        let s = neg.try_pull(&mut frame);
        assert_eq!(neg.active_source(), IngestSource::SilentFallback);
        assert_eq!(s, IngestStatus::Ready);
    }

    #[test]
    fn negotiator_promotes_after_threshold_successes() {
        let mut neg = IngestNegotiator::new(ScriptedSource::new(vec![
            IngestStatus::WouldBlock,
            IngestStatus::WouldBlock,
            IngestStatus::Ready,
            IngestStatus::Ready,
            IngestStatus::Ready,
        ]))
        .with_degraded_after(2)
        .unwrap()
        .with_normal_after(3)
        .unwrap();
        let mut frame = AudioFrame::default();
        let _ = neg.try_pull(&mut frame);
        let _ = neg.try_pull(&mut frame);
        assert_eq!(neg.active_source(), IngestSource::SilentFallback);
        let _ = neg.try_pull(&mut frame);
        let _ = neg.try_pull(&mut frame);
        let _ = neg.try_pull(&mut frame);
        assert_eq!(neg.active_source(), IngestSource::Primary);
    }

    #[test]
    fn negotiator_first_ready_recovery_with_default_criteria() {
        let mut neg = IngestNegotiator::new(ScriptedSource::new(vec![
            IngestStatus::Closed,
            IngestStatus::Closed,
            IngestStatus::Ready,
        ]));
        let mut frame = AudioFrame::default();
        neg.try_pull(&mut frame);
        neg.try_pull(&mut frame);
        assert_eq!(neg.active_source(), IngestSource::SilentFallback);
        let s = neg.try_pull(&mut frame);
        assert_eq!(neg.active_source(), IngestSource::Primary);
        assert_eq!(s, IngestStatus::Ready);
    }

    #[test]
    fn negotiator_failure_resets_success_counter() {
        let mut neg = IngestNegotiator::new(ScriptedSource::new(vec![
            IngestStatus::Ready,
            IngestStatus::Ready,
            IngestStatus::WouldBlock,
        ]));
        let mut frame = AudioFrame::default();
        neg.try_pull(&mut frame);
        neg.try_pull(&mut frame);
        assert_eq!(neg.primary_consecutive_successes(), 2);
        neg.try_pull(&mut frame);
        assert_eq!(neg.primary_consecutive_successes(), 0);
        assert_eq!(neg.primary_consecutive_failures(), 1);
    }

    #[test]
    fn negotiator_with_criteria_round_trips() {
        let crit = IngestModeCriteria {
            degraded_after: 5,
            normal_after: 3,
        };
        let neg = IngestNegotiator::new(ScriptedSource::new(vec![]))
            .with_criteria(crit)
            .unwrap();
        assert_eq!(neg.criteria(), crit);
    }

    #[test]
    fn negotiator_with_degraded_after_validates() {
        assert!(matches!(
            IngestNegotiator::new(ScriptedSource::new(vec![])).with_degraded_after(0),
            Err(AudioError::IngestNegotiation { .. }),
        ));
        assert!(matches!(
            IngestNegotiator::new(ScriptedSource::new(vec![]))
                .with_degraded_after(MAX_THRESHOLD + 1),
            Err(AudioError::IngestNegotiation { .. }),
        ));
    }

    #[test]
    fn negotiator_cpal_variant_is_reserved_not_constructed_in_production() {
        let neg = IngestNegotiator::new(ScriptedSource::new(vec![]));
        assert_ne!(neg.active_source(), IngestSource::Cpal);
    }
}
