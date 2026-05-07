// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Default constants per ADR-020 §3 "Defaults". Per-default rationale
//! documented inline. Drift pins below assert each constant against
//! the ADR-020 published value.
//!
//! Mirrors the ADR-013 default-policy precedent established by
//! `integrations/osc-adapter/src/defaults.rs` and
//! `integrations/midi-adapter/src/defaults.rs`: each constant carries
//! a T1 drift pin whose failure routes review to the
//! `EV-INTEGRATION-MATRIX` gate before the change merges. Operator-
//! facing overrides do not live here; per ADR-013 §6 each default is
//! overridable in a Settings panel at runtime, and this module pins
//! the out-of-the-box posture the adapter starts from.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (camera arm).

use crate::frames::PixelFormat;

/// Default frame rate (frames per second). 30 fps matches the
/// `docs/30-mvp-support-matrix.md` Camera-as-Parity-Core baseline
/// and balances bandwidth against typical live-visuals frame demand.
/// Per ADR-020 §3.
pub const DEFAULT_FRAME_RATE: u32 = 30;

/// Default capture resolution (1280×720). 720p is the highest
/// commonly-supported resolution across consumer USB cameras
/// without forcing MJPG, matching the MVP support-matrix baseline.
/// Per ADR-020 §3.
pub const DEFAULT_RESOLUTION: (u32, u32) = (1280, 720);

/// Default frame queue depth. 4 frames balances latency against
/// frame-drop risk under a 60 Hz consumer at 30 Hz producer cadence:
/// at queue_depth = 1 a single missed poll drops the frame; at
/// queue_depth = 4 the visualizer absorbs up to 133 ms of consumer
/// jitter before drop-on-full kicks in. Drop-oldest-on-full per
/// ADR-020 §4 + `docs/54` Phase-2 lesson 1 (deque-swap drain
/// semantics).
pub const DEFAULT_QUEUE_DEPTH: usize = 4;

/// Default pixel format preference. NV12 is universally supported on
/// macOS + Windows backends and is the most efficient native format
/// (avoids a colour-space conversion in the producer thread; the
/// canonical zero-copy GPU upload path per ADR-020 §7). Per
/// ADR-020 §3.
pub const DEFAULT_PIXEL_FORMAT_PREFERENCE: PixelFormat = PixelFormat::Nv12;

/// Default reconnect attempts on adapter `Disconnected → Bound`
/// recovery after USB hotplug detection. Operator-overridable per
/// `CameraConfig::reconnect_attempts`. Per ADR-020 §6.
pub const DEFAULT_RECONNECT_ATTEMPTS: u32 = 5;

/// Default reconnect interval (ms). 1000 ms × 5 attempts = 5 s
/// total tolerance window. Conservative posture for MVP per
/// ADR-020 §6 "Reconnect-interval clarification" — live-show
/// telemetry to drive a tune-down (e.g. `250ms × 5 = 1.25s`) is a
/// Phase-5+ deliverable. Operator-overridable per
/// `CameraConfig::reconnect_interval_ms`.
pub const DEFAULT_RECONNECT_INTERVAL_MS: u32 = 1000;

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod defaults_pins {
    //! Evidence: `EV-INTEGRATION-MATRIX` (T1) — camera-adapter
    //! default-value pins for the frame-rate, resolution, queue-
    //! depth, pixel-format-preference, and reconnect baseline. Per
    //! `docs/43` §15.5 T1 definition: each test pins an external-
    //! spec constant against the governing ADR (ADR-020 §3 / §6).
    //! Drift in any default fires the corresponding test, so review
    //! can route the change to the `EV-INTEGRATION-MATRIX` gate
    //! before merge. Phase-4a EXIT requires T2 over the synthetic
    //! camera fixture corpus, which lands with Phase 2 PR 4.

    use super::*;

    /// `EV-INTEGRATION-MATRIX` T1: each default constant matches
    /// its ADR-020 §3 published value.
    #[test]
    fn defaults_match_adr_020() {
        assert_eq!(DEFAULT_FRAME_RATE, 30);
        assert_eq!(DEFAULT_RESOLUTION, (1280, 720));
        assert_eq!(DEFAULT_QUEUE_DEPTH, 4);
        assert!(matches!(DEFAULT_PIXEL_FORMAT_PREFERENCE, PixelFormat::Nv12));
        assert_eq!(DEFAULT_RECONNECT_ATTEMPTS, 5);
        assert_eq!(DEFAULT_RECONNECT_INTERVAL_MS, 1000);
    }

    /// `EV-INTEGRATION-MATRIX` T1: queue-depth is non-zero per
    /// ADR-020 §4 — a queue_depth of zero would degenerate the
    /// drop-on-full policy and starve the producer thread. Pinned
    /// because the value is operator-overridable but the floor at
    /// 1 is part of the bounded-backpressure contract.
    #[test]
    fn queue_depth_default_is_non_zero() {
        // Read constant through a `let` binding so clippy does not
        // fold the comparison into `assert!(true)` and reject the
        // pin under `assertions_on_constants`. The constant value
        // is the load-bearing invariant; the local binding is an
        // optimisation barrier the compiler cannot evaluate at
        // const-check time without const_evaluatable_checked.
        // Mirrors the midi-adapter `adapter_floor_is_at_least_per_route_default`
        // pin pattern.
        let depth: usize = DEFAULT_QUEUE_DEPTH;
        assert!(depth > 0, "queue depth ({depth}) must be at least 1");
    }

    /// `EV-INTEGRATION-MATRIX` T1: reconnect window product equals
    /// the ADR-020 §6 stated 5-second tolerance. Pinned literally
    /// because operator-tuning proposals must surface against the
    /// MVP baseline.
    #[test]
    fn reconnect_window_total_is_five_seconds() {
        let attempts = u64::from(DEFAULT_RECONNECT_ATTEMPTS);
        let interval = u64::from(DEFAULT_RECONNECT_INTERVAL_MS);
        assert_eq!(attempts * interval, 5_000);
    }
}
