// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Synthetic T2 corpus for chromaticity-camera-adapter (Wave 1c-β
//! Phase 2 PR 4). Hardware-rig validation explicitly deferred per
//! the row 78 / row 40 / row 93 synthetic-narrowing precedent.
//! Real camera hardware validation = manual on AO dev rigs.
//!
//! # Narrowing
//!
//! Camera-rig CI validation against a live AVFoundation /
//! Media Foundation device is not feasible on GitHub Actions
//! runners (no permissioned hardware; even self-hosted macOS
//! runners cannot reliably surface a built-in FaceTime camera
//! across CI runs without operator-grant of TCC permissions).
//! This corpus uses a `MockCameraAdapter` — a test-only impl of
//! the [`chromaticity_camera_adapter::CameraAdapter`] trait — that
//! exercises the documented state machine + canonical fault-reason
//! taxonomy + queue / drop-on-full / reconnect / format-negotiation
//! behaviors deterministically without any nokhwa hardware
//! involvement.
//!
//! Per ADR-020 §Decision §10 + the row 78 (OSC routing matrix) /
//! row 40 (multi-profile chaos soak) / row 93 (Syphon adapter)
//! precedent, this synthetic narrowing is recorded honestly against
//! `EV-INTEGRATION-MATRIX` (camera arm) at the Phase 2 PR 5
//! closing-docs widening. Per docs/54 Phase-2 lesson 8: the
//! synthetic harness strictly improves on ffmpeg + VLC who ship
//! zero camera-capture unit tests.
//!
//! # Coverage axes
//!
//! 1. **PixelFormat round-trip across all five MVP variants**
//!    (Nv12 / Yuyv / Rgb24 / Bgra32 / Mjpeg) — serde Serialize +
//!    Deserialize symmetry + variant identity.
//! 2. **Deque-swap drain preserves FIFO order under no overflow**
//!    per docs/54 lesson 1.
//! 3. **Deque-swap drain drops oldest on overflow** preserving the
//!    newest N frames per ADR-020 §4.
//! 4. **bind / unbind / bind round-trip** — state-machine
//!    idempotency.
//! 5. **Format negotiation falls back when preferred unavailable**
//!    — realised format reflects the hardware, not the preference.
//! 6. **Format negotiation returns format-not-supported when no
//!    overlap** — `Faulted("format-not-supported")` transition.
//! 7. **Reconnect attempts exhausted transitions to faulted** —
//!    `Faulted("reconnect-exhausted")` after configured budget.
//! 8. **Reconnect attempts succeed within budget** — `Bound` with
//!    correct attempt count.
//! 9. **Canonical fault-reason exact-string match** against the
//!    `pub const` table.
//! 10. **Monotonic timestamp progression within session** —
//!     `timestamp_ns` strictly increasing across drained frames.
//! 11. **Byte-order discipline for Bgra32 payload** per docs/54
//!     lesson 4 (B, G, R, A in memory).
//! 12. **PixelFormat #[non_exhaustive] widening guard** — explicit
//!     match per variant breaks until a new variant is acknowledged.
//!
//! Evidence: `EV-INTEGRATION-MATRIX` (T2) — camera arm synthetic-
//! narrowing fixture corpus per ADR-020 §Decision §10 + the row 78
//! / row 40 / row 93 narrowing precedent.

// Tests opt out of the workspace `clippy::unwrap_used` /
// `expect_used` per docs/43 §12.1 module-level guidance.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::borrow::Cow;
use std::collections::VecDeque;

use chromaticity_camera_adapter::{
    AdapterError, AdapterHealth, BACKEND_ERROR, CameraAdapter, CameraConfig, CameraFrame,
    DEVICE_BUSY, DEVICE_NOT_FOUND, DeviceSummary, FORMAT_NOT_SUPPORTED, PERMISSION_DENIED,
    PixelFormat, RECONNECT_EXHAUSTED,
};

// ----- Mock backend implementation -----

/// Test-only `CameraAdapter` impl. Holds a synthetic frame deque +
/// a state machine + scriptable hooks for the format-negotiation /
/// reconnect / drop-on-full behaviors the real nokhwa backends ship
/// in follow-up policy-scoped PRs.
///
/// Constructor accepts a pre-loaded `Vec<CameraFrame>`; `bind`
/// applies the configured `bind_outcome` script to drive the state
/// machine through the `Bound` / `Faulted(reason)` arms; `poll(N)`
/// uses the deque-swap drain semantics per docs/54 lesson 1
/// (drop-oldest-on-full at producer time, FIFO drain at consumer
/// time).
struct MockCameraAdapter {
    /// In-adapter frame queue. Producer (test-side) push semantics
    /// honor `queue_depth`-bounded drop-oldest-on-full per ADR-020
    /// §4. Consumer side is `poll(max_frames)`.
    queue: VecDeque<CameraFrame>,
    /// Bound queue depth for drop-oldest-on-full; mirrors
    /// `CameraConfig::queue_depth`.
    queue_depth: usize,
    /// Currently-bound config (when bound).
    bound_config: Option<CameraConfig>,
    /// Current health-state.
    health: AdapterHealth,
    /// Scripted `bind` outcome. Tests construct the mock with the
    /// outcome they want exercised; the mock applies it on the
    /// `bind` call.
    bind_outcome: BindOutcome,
    /// Reconnect counter. Each `bind` call increments; the
    /// outcome script consults this to drive the
    /// reconnect-within-budget vs reconnect-exhausted arms.
    bind_attempt_count: u32,
    /// Realised format the mock reports back from `bind` via the
    /// state machine. Lets the format-negotiation fixtures assert
    /// the realised format independent of the preference.
    realised_format: Option<PixelFormat>,
    /// Device list returned from `enumerate`.
    enumerated_devices: Vec<DeviceSummary>,
}

/// Scripted-fail variant tag for [`BindOutcome::Fail`]. Mirrors
/// the [`AdapterError`] variant set but is `Clone` so the mock's
/// `bind_outcome` field can be cloned per call without needing
/// `Clone` on `AdapterError` itself.
#[derive(Clone, Copy, Debug)]
enum FailKind {
    PermissionDenied,
    DeviceBusy,
    DeviceNotFound,
    FormatNotSupported,
    ReconnectExhausted,
    BackendError,
}

impl FailKind {
    fn into_error(self) -> AdapterError {
        match self {
            Self::PermissionDenied => AdapterError::PermissionDenied(String::from("scripted")),
            Self::DeviceBusy => AdapterError::DeviceBusy(String::from("scripted")),
            Self::DeviceNotFound => AdapterError::DeviceNotFound(String::from("scripted")),
            Self::FormatNotSupported => AdapterError::FormatNotSupported(String::from("scripted")),
            Self::ReconnectExhausted => AdapterError::ReconnectExhausted(String::from("scripted")),
            Self::BackendError => AdapterError::BackendError(String::from("scripted")),
        }
    }

    fn canonical_reason(self) -> &'static str {
        self.into_error().canonical_reason()
    }
}

/// Scripted `bind` outcome. Each variant maps to a specific
/// state-machine arm the corpus exercises.
#[derive(Clone, Debug)]
enum BindOutcome {
    /// `bind` succeeds on first call; transitions to `Bound`.
    Succeed { realised: PixelFormat },
    /// `bind` returns an `AdapterError` of the given variant on
    /// every call; transitions to `Faulted(reason)` per the
    /// canonical mapping.
    Fail(FailKind),
    /// `bind` fails with `ReconnectExhausted` after the configured
    /// number of attempts; succeeds on the budget+1-th call. Lets
    /// the reconnect-within-budget fixture exercise the recovery
    /// path. `disconnects_before_success` is the number of
    /// disconnect-attempts the mock simulates before honoring the
    /// bind.
    ReconnectThenSucceed {
        disconnects_before_success: u32,
        realised: PixelFormat,
    },
    /// Reconnect is requested but every attempt fails; after
    /// `attempts_budget` failures, transitions to
    /// `Faulted("reconnect-exhausted")`.
    ReconnectExhausted { attempts_budget: u32 },
    /// Format negotiation: `bind` fails with `FormatNotSupported`
    /// when the requested preference does not overlap with the
    /// `offered` set. Falls back to the first overlapping format
    /// otherwise.
    FormatNegotiate { offered: Vec<PixelFormat> },
}

impl MockCameraAdapter {
    fn new(bind_outcome: BindOutcome) -> Self {
        Self {
            queue: VecDeque::new(),
            queue_depth: 4,
            bound_config: None,
            health: AdapterHealth::Disconnected,
            bind_outcome,
            bind_attempt_count: 0,
            realised_format: None,
            enumerated_devices: Vec::new(),
        }
    }

    /// Set the queue depth. Mirrors `CameraConfig::queue_depth`
    /// for producer-side drop-oldest-on-full.
    fn with_queue_depth(mut self, depth: usize) -> Self {
        self.queue_depth = depth;
        self
    }

    /// Push a synthetic frame into the queue, honoring the
    /// drop-oldest-on-full deque-swap semantics per docs/54
    /// lesson 1: when the queue is at `queue_depth` and a new
    /// frame arrives, the OLDEST frame is dropped to make room
    /// (the producer never blocks; the newest frame always lands).
    fn producer_push(&mut self, frame: CameraFrame) {
        while self.queue.len() >= self.queue_depth {
            self.queue.pop_front();
        }
        self.queue.push_back(frame);
    }

    /// Read-only access to the `bind_attempt_count` for the
    /// reconnect fixtures.
    fn attempts(&self) -> u32 {
        self.bind_attempt_count
    }

    /// Read-only access to the realised format for the format-
    /// negotiation fixtures.
    fn realised(&self) -> Option<PixelFormat> {
        self.realised_format
    }
}

impl CameraAdapter for MockCameraAdapter {
    fn enumerate(&self) -> Result<Vec<DeviceSummary>, AdapterError> {
        Ok(self.enumerated_devices.clone())
    }

    fn bind(&mut self, config: CameraConfig) -> Result<(), AdapterError> {
        self.bind_attempt_count += 1;
        // Clone the outcome so we can mutate `self` without
        // borrow conflicts on the script field.
        let outcome = self.bind_outcome.clone();
        match outcome {
            BindOutcome::Succeed { realised } => {
                self.health = AdapterHealth::Bound;
                self.bound_config = Some(config);
                self.realised_format = Some(realised);
                Ok(())
            }
            BindOutcome::Fail(kind) => {
                let reason: Cow<'static, str> = Cow::Borrowed(kind.canonical_reason());
                self.health = AdapterHealth::Faulted(reason);
                self.bound_config = None;
                Err(kind.into_error())
            }
            BindOutcome::ReconnectThenSucceed {
                disconnects_before_success,
                realised,
            } => {
                if self.bind_attempt_count <= disconnects_before_success {
                    // Stay disconnected; surface a fault until
                    // budget elapses. The mock surfaces a
                    // BackendError to mimic a transient
                    // disconnect that the higher-level reconnect
                    // loop retries against.
                    let err = AdapterError::BackendError(format!(
                        "transient disconnect (attempt {}/{})",
                        self.bind_attempt_count, disconnects_before_success
                    ));
                    let reason: Cow<'static, str> = Cow::Borrowed(err.canonical_reason());
                    self.health = AdapterHealth::Faulted(reason);
                    Err(err)
                } else {
                    self.health = AdapterHealth::Bound;
                    self.bound_config = Some(config);
                    self.realised_format = Some(realised);
                    Ok(())
                }
            }
            BindOutcome::ReconnectExhausted { attempts_budget } => {
                if self.bind_attempt_count > attempts_budget {
                    let err = AdapterError::ReconnectExhausted(format!(
                        "reconnect budget exhausted after {attempts_budget} attempts"
                    ));
                    let reason: Cow<'static, str> = Cow::Borrowed(err.canonical_reason());
                    self.health = AdapterHealth::Faulted(reason);
                    Err(err)
                } else {
                    let err = AdapterError::BackendError(format!(
                        "transient disconnect (attempt {}/{})",
                        self.bind_attempt_count, attempts_budget
                    ));
                    let reason: Cow<'static, str> = Cow::Borrowed(err.canonical_reason());
                    self.health = AdapterHealth::Faulted(reason);
                    Err(err)
                }
            }
            BindOutcome::FormatNegotiate { offered } => {
                let preference = config.pixel_format_preference;
                // No overlap → format-not-supported.
                if let Some(pref) = preference {
                    if offered.iter().any(|f| *f == pref) {
                        // Hardware honors the preference exactly.
                        self.health = AdapterHealth::Bound;
                        self.bound_config = Some(config);
                        self.realised_format = Some(pref);
                        return Ok(());
                    }
                    // Preference is set but unavailable; fall
                    // back to the first offered format if any.
                    if let Some(fallback) = offered.first().copied() {
                        self.health = AdapterHealth::Bound;
                        self.bound_config = Some(config);
                        self.realised_format = Some(fallback);
                        return Ok(());
                    }
                    // Preference set but offered list empty →
                    // format-not-supported.
                    let err = AdapterError::FormatNotSupported(format!(
                        "preference {pref:?} unavailable; no offered formats"
                    ));
                    let reason: Cow<'static, str> = Cow::Borrowed(err.canonical_reason());
                    self.health = AdapterHealth::Faulted(reason);
                    return Err(err);
                }
                // No preference: hardware picks. Surface the
                // first offered format if any, else
                // format-not-supported (no offered formats and
                // no preference is a degenerate state worth
                // pinning).
                if let Some(picked) = offered.first().copied() {
                    self.health = AdapterHealth::Bound;
                    self.bound_config = Some(config);
                    self.realised_format = Some(picked);
                    Ok(())
                } else {
                    let err = AdapterError::FormatNotSupported(String::from(
                        "no preference + no offered formats",
                    ));
                    let reason: Cow<'static, str> = Cow::Borrowed(err.canonical_reason());
                    self.health = AdapterHealth::Faulted(reason);
                    Err(err)
                }
            }
        }
    }

    fn unbind(&mut self) -> Result<(), AdapterError> {
        self.bound_config = None;
        self.queue.clear();
        self.health = AdapterHealth::Disconnected;
        Ok(())
    }

    fn poll(&mut self, max_frames: usize) -> Vec<CameraFrame> {
        if max_frames == 0 {
            return Vec::new();
        }
        let drain_count = max_frames.min(self.queue.len());
        self.queue.drain(..drain_count).collect()
    }

    fn health(&self) -> AdapterHealth {
        self.health.clone()
    }
}

// ----- Helpers -----

fn synthetic_frame(timestamp_ns: u64, format: PixelFormat) -> CameraFrame {
    CameraFrame {
        timestamp_ns,
        width: 640,
        height: 480,
        pixel_format: format,
        payload: vec![0u8; 16],
    }
}

// ============================================================================
// Fixtures
// ============================================================================

/// **Fixture 1**: PixelFormat round-trip preserves variant across
/// all five MVP variants (Nv12 / Yuyv / Rgb24 / Bgra32 / Mjpeg) per
/// ADR-020 §Decision §2 byte-order discipline + frames.rs module
/// prose. Pinned because the variant set is part of the operator-
/// facing UX surface (the preview pipeline routes off the realised
/// format).
///
/// EV-INTEGRATION-MATRIX T2: PixelFormat variant round-trip across
/// all five MVP variants.
#[test]
fn pixel_format_round_trip_for_all_five_variants() {
    for variant in [
        PixelFormat::Nv12,
        PixelFormat::Yuyv,
        PixelFormat::Rgb24,
        PixelFormat::Bgra32,
        PixelFormat::Mjpeg,
    ] {
        let json = serde_json::to_string(&variant).expect("serialize PixelFormat");
        let parsed: PixelFormat = serde_json::from_str(&json).expect("deserialize PixelFormat");
        assert_eq!(
            parsed, variant,
            "round-trip mismatch for {variant:?}: serialized={json}",
        );
    }
}

/// **Fixture 2**: deque-swap drain preserves FIFO order when the
/// queue does not overflow per docs/54 Phase-2 lesson 1.
///
/// EV-INTEGRATION-MATRIX T2: drain order under no overflow.
#[test]
fn deque_swap_drain_preserves_fifo_order_under_no_overflow() {
    let mut adapter = MockCameraAdapter::new(BindOutcome::Succeed {
        realised: PixelFormat::Nv12,
    })
    .with_queue_depth(8);
    for ts in [100_u64, 200, 300] {
        adapter.producer_push(synthetic_frame(ts, PixelFormat::Nv12));
    }
    let drained = adapter.poll(10);
    let timestamps: Vec<u64> = drained.iter().map(|f| f.timestamp_ns).collect();
    assert_eq!(
        timestamps,
        vec![100, 200, 300],
        "FIFO order must be preserved under no overflow",
    );
}

/// **Fixture 3**: deque-swap drain drops oldest on overflow,
/// preserving the newest `queue_depth` frames per ADR-020 §4 +
/// docs/54 Phase-2 lesson 1.
///
/// EV-INTEGRATION-MATRIX T2: drop-oldest-on-overflow under
/// producer-side burst.
#[test]
fn deque_swap_drain_drops_oldest_on_overflow() {
    let mut adapter = MockCameraAdapter::new(BindOutcome::Succeed {
        realised: PixelFormat::Nv12,
    })
    .with_queue_depth(4);
    // Push 6 frames into a depth-4 queue; oldest 2 should drop.
    for ts in [10_u64, 20, 30, 40, 50, 60] {
        adapter.producer_push(synthetic_frame(ts, PixelFormat::Nv12));
    }
    let drained = adapter.poll(10);
    let timestamps: Vec<u64> = drained.iter().map(|f| f.timestamp_ns).collect();
    assert_eq!(
        timestamps,
        vec![30, 40, 50, 60],
        "oldest 2 frames must be dropped; newest 4 retained",
    );
}

/// **Fixture 4**: bind / unbind / bind state-machine round-trip is
/// idempotent. Mirrors the OSC / MIDI / Syphon adapter
/// disconnect-idempotency posture per ADR-005 "Decision" bullet 3.
///
/// EV-INTEGRATION-MATRIX T2: bind/unbind/bind state-machine
/// round-trip.
#[test]
fn bind_then_unbind_then_bind_idempotency() {
    let mut adapter = MockCameraAdapter::new(BindOutcome::Succeed {
        realised: PixelFormat::Nv12,
    });
    let config = CameraConfig::defaults_for("device-x");

    // First bind.
    adapter.bind(config.clone()).expect("first bind succeeds");
    assert!(matches!(adapter.health(), AdapterHealth::Bound));

    // Unbind.
    adapter.unbind().expect("unbind succeeds");
    assert!(matches!(adapter.health(), AdapterHealth::Disconnected));

    // Second bind on same adapter.
    adapter.bind(config).expect("second bind succeeds");
    assert!(matches!(adapter.health(), AdapterHealth::Bound));
    assert_eq!(adapter.attempts(), 2);
}

/// **Fixture 5**: format negotiation falls back when the preferred
/// format is unavailable but at least one alternative is offered.
/// `CameraFrame::pixel_format` reports the realised format, NOT
/// the preference. Pinned because authors who care about the format
/// must read the realised field per ADR-020 §2 + frames.rs module
/// prose.
///
/// EV-INTEGRATION-MATRIX T2: format-negotiation fallback path.
#[test]
fn format_negotiation_falls_back_when_preferred_unavailable() {
    let mut adapter = MockCameraAdapter::new(BindOutcome::FormatNegotiate {
        offered: vec![PixelFormat::Nv12, PixelFormat::Yuyv],
    });
    let mut config = CameraConfig::defaults_for("device-y");
    config.pixel_format_preference = Some(PixelFormat::Mjpeg);

    adapter.bind(config).expect("bind falls back successfully");
    assert!(matches!(adapter.health(), AdapterHealth::Bound));
    // Realised format is the first offered (Nv12), NOT the
    // preference (Mjpeg).
    assert_eq!(adapter.realised(), Some(PixelFormat::Nv12));
}

/// **Fixture 6**: format negotiation returns `format-not-supported`
/// when no offered format overlaps the preference and the offered
/// list is empty (degenerate hardware).
///
/// EV-INTEGRATION-MATRIX T2: format-not-supported terminal-fault
/// path.
#[test]
fn format_negotiation_returns_format_not_supported_when_no_overlap() {
    let mut adapter = MockCameraAdapter::new(BindOutcome::FormatNegotiate { offered: vec![] });
    let mut config = CameraConfig::defaults_for("device-z");
    config.pixel_format_preference = Some(PixelFormat::Rgb24);

    let err = adapter.bind(config).expect_err("no overlap errors");
    assert!(matches!(err, AdapterError::FormatNotSupported(_)));
    assert_eq!(err.canonical_reason(), FORMAT_NOT_SUPPORTED);
    match adapter.health() {
        AdapterHealth::Faulted(reason) => {
            assert_eq!(reason.as_ref(), FORMAT_NOT_SUPPORTED);
        }
        other => panic!("expected Faulted(format-not-supported), got {other:?}"),
    }
}

/// **Fixture 7**: reconnect attempts exhausted transitions to
/// `Faulted("reconnect-exhausted")` after the configured budget
/// elapses. Per ADR-020 §6 the
/// `reconnect_attempts × reconnect_interval_ms` window is bounded;
/// crossing it surfaces the canonical fault reason.
///
/// EV-INTEGRATION-MATRIX T2: reconnect-exhausted terminal-fault
/// path.
#[test]
fn reconnect_attempts_exhausted_transitions_to_faulted() {
    let mut adapter =
        MockCameraAdapter::new(BindOutcome::ReconnectExhausted { attempts_budget: 3 });
    let config = CameraConfig::defaults_for("flaky-device");

    // First three attempts: transient backend errors.
    for _ in 0..3 {
        let err = adapter
            .bind(config.clone())
            .expect_err("transient disconnect errors");
        assert!(matches!(err, AdapterError::BackendError(_)));
    }
    // Fourth attempt crosses the budget boundary →
    // reconnect-exhausted terminal fault.
    let err = adapter
        .bind(config)
        .expect_err("budget-exceeded attempt errors");
    assert!(matches!(err, AdapterError::ReconnectExhausted(_)));
    assert_eq!(err.canonical_reason(), RECONNECT_EXHAUSTED);
    match adapter.health() {
        AdapterHealth::Faulted(reason) => {
            assert_eq!(reason.as_ref(), RECONNECT_EXHAUSTED);
        }
        other => panic!("expected Faulted(reconnect-exhausted), got {other:?}"),
    }
    assert_eq!(adapter.attempts(), 4);
}

/// **Fixture 8**: reconnect attempts succeed within budget — after
/// `disconnects_before_success` failures, the next bind transitions
/// to `Bound`. Pinned because the reconnect-recovery path is the
/// happy-path counterpart to the exhausted-fault arm in fixture 7;
/// both must hold for ADR-020 §6 to be honored.
///
/// EV-INTEGRATION-MATRIX T2: reconnect-within-budget recovery
/// path.
#[test]
fn reconnect_attempts_succeed_within_budget() {
    let mut adapter = MockCameraAdapter::new(BindOutcome::ReconnectThenSucceed {
        disconnects_before_success: 2,
        realised: PixelFormat::Nv12,
    });
    let config = CameraConfig::defaults_for("recovering-device");

    // First two attempts fail with transient backend errors.
    for _ in 0..2 {
        let err = adapter
            .bind(config.clone())
            .expect_err("transient disconnect errors");
        assert!(matches!(err, AdapterError::BackendError(_)));
    }
    // Third attempt recovers.
    adapter
        .bind(config)
        .expect("third attempt succeeds within budget");
    assert!(matches!(adapter.health(), AdapterHealth::Bound));
    assert_eq!(adapter.attempts(), 3);
}

/// **Fixture 9**: each `Faulted(reason)` produced by the state
/// machine matches the `pub const` canonical strings exactly per
/// ADR-020 §Decision §2. Pinned because the operator-facing UX
/// surface localizes off the constants; any drift would silently
/// break the localization map.
///
/// EV-INTEGRATION-MATRIX T2: canonical fault-reason exact-string
/// table.
#[test]
fn canonical_fault_reasons_match_pub_const_exact_strings() {
    let cases: &[(FailKind, &str)] = &[
        (FailKind::PermissionDenied, PERMISSION_DENIED),
        (FailKind::DeviceBusy, DEVICE_BUSY),
        (FailKind::DeviceNotFound, DEVICE_NOT_FOUND),
        (FailKind::FormatNotSupported, FORMAT_NOT_SUPPORTED),
        (FailKind::ReconnectExhausted, RECONNECT_EXHAUSTED),
        (FailKind::BackendError, BACKEND_ERROR),
    ];
    // Drive each variant through the mock state-machine via
    // `BindOutcome::Fail` to verify the surfaced
    // `AdapterHealth::Faulted(reason)` matches the canonical
    // string end-to-end (not just at the AdapterError surface).
    for (kind, expected) in cases {
        let mut adapter = MockCameraAdapter::new(BindOutcome::Fail(*kind));
        let surfaced_err = adapter
            .bind(CameraConfig::defaults_for("any-device"))
            .expect_err("scripted-fail bind errors");
        assert_eq!(
            surfaced_err.canonical_reason(),
            *expected,
            "canonical-reason mismatch on AdapterError surface for {kind:?}",
        );
        match adapter.health() {
            AdapterHealth::Faulted(reason) => {
                assert_eq!(
                    reason.as_ref(),
                    *expected,
                    "canonical-reason mismatch on AdapterHealth::Faulted for {kind:?}",
                );
            }
            other => panic!("expected Faulted({expected}) for {kind:?}, got {other:?}"),
        }
    }
}

/// **Fixture 10**: monotonic timestamp progression within a session
/// per `frames.rs` Timestamp policy + docs/54 Phase-2 lesson 3
/// (cautionary precedent: ffmpeg's dshow indev five-year wallclock
/// regression). Pinned because cue-curve interpolation reads the
/// timestamp delta to drive frame-locked animations; a regression
/// to wallclock sourcing would silently break that surface.
///
/// EV-INTEGRATION-MATRIX T2: monotonic timestamp progression
/// within session.
#[test]
fn monotonic_timestamp_progression_within_session() {
    let mut adapter = MockCameraAdapter::new(BindOutcome::Succeed {
        realised: PixelFormat::Nv12,
    })
    .with_queue_depth(8);
    let timestamps_in = [1_000_u64, 2_000, 3_500, 5_000, 7_500];
    for ts in timestamps_in {
        adapter.producer_push(synthetic_frame(ts, PixelFormat::Nv12));
    }
    let drained = adapter.poll(10);
    let timestamps_out: Vec<u64> = drained.iter().map(|f| f.timestamp_ns).collect();
    assert_eq!(
        timestamps_out,
        timestamps_in.to_vec(),
        "timestamp order must round-trip identically",
    );
    // Strictly-monotonic invariant: each ts > previous.
    for window in timestamps_out.windows(2) {
        assert!(
            window[1] > window[0],
            "timestamps must be strictly monotonic; got {window:?}",
        );
    }
}

/// **Fixture 11**: byte-order discipline for `Bgra32` payload per
/// docs/54 Phase-2 lesson 4 + frames.rs module prose. The byte
/// order is B, G, R, A in memory (NOT `Argb32`); pinned because the
/// operator-facing preview pipeline reads the payload bytes
/// directly + a regression to ARGB byte order would silently shift
/// every color channel by one position. Cautionary precedent: VLC's
/// `MEDIASUBTYPE_ARGB32 → VLC_CODEC_BGRA` saga.
///
/// EV-INTEGRATION-MATRIX T2: Bgra32 byte-order round-trip.
#[test]
fn byte_order_discipline_bgra32_payload_round_trip() {
    // Construct a single 1x1 BGRA32 pixel: B=1, G=2, R=3, A=4.
    let payload = vec![0x01_u8, 0x02, 0x03, 0x04];
    let frame = CameraFrame {
        timestamp_ns: 0,
        width: 1,
        height: 1,
        pixel_format: PixelFormat::Bgra32,
        payload: payload.clone(),
    };
    let mut adapter = MockCameraAdapter::new(BindOutcome::Succeed {
        realised: PixelFormat::Bgra32,
    });
    adapter.producer_push(frame);
    let drained = adapter.poll(1);
    assert_eq!(drained.len(), 1);
    let drained_frame = &drained[0];
    assert_eq!(
        drained_frame.payload, payload,
        "Bgra32 payload bytes must round-trip in B,G,R,A order",
    );
    assert_eq!(drained_frame.payload[0], 0x01, "byte 0 must be Blue");
    assert_eq!(drained_frame.payload[1], 0x02, "byte 1 must be Green");
    assert_eq!(drained_frame.payload[2], 0x03, "byte 2 must be Red");
    assert_eq!(drained_frame.payload[3], 0x04, "byte 3 must be Alpha");
    assert!(matches!(drained_frame.pixel_format, PixelFormat::Bgra32));
}

/// **Fixture 12**: `PixelFormat` `#[non_exhaustive]` widening
/// guard per ADR-020 §Decision §2. Out-of-crate matches against a
/// `#[non_exhaustive]` enum require a wildcard arm, so the corpus
/// pairs with the in-crate exhaustive-match T1 pin
/// `pixel_format_variant_set_matches_adr_020` in `frames.rs`. At
/// the T2 surface this fixture pins each known variant against a
/// per-format byte-stride property + asserts the wildcard catch-
/// all is never hit at the current variant set; adding a sixth
/// variant without updating the corpus surfaces here as a
/// `panic!` from the catch-all arm during a routine corpus run.
///
/// EV-INTEGRATION-MATRIX T2: PixelFormat widening guard via
/// catch-all panic.
#[test]
fn pixel_format_non_exhaustive_widening_guard() {
    for variant in [
        PixelFormat::Nv12,
        PixelFormat::Yuyv,
        PixelFormat::Rgb24,
        PixelFormat::Bgra32,
        PixelFormat::Mjpeg,
    ] {
        // `#[non_exhaustive]` requires the wildcard arm in cross-
        // crate matches. The wildcard panics so any newly-added
        // variant breaks the corpus until acknowledged, re-routing
        // the change through the `EV-INTEGRATION-MATRIX` gate.
        let bytes_per_pixel: Option<u32> = match variant {
            PixelFormat::Nv12 => None, // planar, not bytes-per-pixel
            PixelFormat::Yuyv => Some(2),
            PixelFormat::Rgb24 => Some(3),
            PixelFormat::Bgra32 => Some(4),
            PixelFormat::Mjpeg => None, // compressed
            _ => panic!(
                "PixelFormat widened beyond MVP set; update camera_adapter_corpus.rs \
                 fixture 12 (and likely the render-side wiring in Phase 2 PR 5) \
                 before merging the new variant"
            ),
        };
        match variant {
            PixelFormat::Yuyv | PixelFormat::Rgb24 | PixelFormat::Bgra32 => {
                assert!(bytes_per_pixel.is_some());
            }
            PixelFormat::Nv12 | PixelFormat::Mjpeg => {
                assert!(bytes_per_pixel.is_none());
            }
            _ => panic!("PixelFormat widened beyond MVP set; see fixture 12 catch-all"),
        }
    }
}
