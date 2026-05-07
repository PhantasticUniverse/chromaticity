// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Camera adapter trait + `NokhwaInputAdapter` impl per ADR-020.
//!
//! Phase-4a Wave 1c-β Phase 2 PR 1 surface: the [`CameraAdapter`]
//! trait + [`NokhwaInputAdapter`] scaffold. The shell impl returns
//! [`AdapterError::DeviceNotFound`] from [`CameraAdapter::bind`] and
//! an empty vec from [`CameraAdapter::enumerate`] until the per-
//! platform backends land in Phase 2 PR 2 (macOS) and Phase 2 PR 3
//! (Windows).
//!
//! The trait shape mirrors `integrations/{osc,midi}-adapter/` per
//! ADR-005 "Decision" bullet 1, specialised to the camera
//! enumeration / bind / poll / health surface ADR-020 §2 names.
//!
//! ## Drop-on-full backpressure (per ADR-020 §4)
//!
//! The `poll(max_frames)` method drains up to `max_frames` from the
//! internal queue. The producer-side drop-policy (drop oldest on
//! full per ADR-020 §4 + `docs/54` Phase-2 lesson 1 deque-swap
//! drain semantics) is exercised by the per-platform backends in
//! Phase 2 PR 2 / PR 3; the scaffold queue is empty in PR 1.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (camera arm, Phase-4a
//! Wave 1c-β).

use std::borrow::Cow;
use std::collections::VecDeque;

use thiserror::Error;

use crate::defaults::{
    DEFAULT_FRAME_RATE, DEFAULT_PIXEL_FORMAT_PREFERENCE, DEFAULT_QUEUE_DEPTH,
    DEFAULT_RECONNECT_ATTEMPTS, DEFAULT_RECONNECT_INTERVAL_MS, DEFAULT_RESOLUTION,
};
use crate::frames::{
    BACKEND_ERROR, CameraFrame, DEVICE_BUSY, DEVICE_NOT_FOUND, FORMAT_NOT_SUPPORTED,
    PERMISSION_DENIED, PixelFormat, RECONNECT_EXHAUSTED,
};

/// Coarse classification of a camera device, used at enumeration
/// time to drive UI grouping (built-in vs external vs Continuity vs
/// virtual). Per ADR-020 §Decision §2 + `docs/54` Phase-2 lesson 9.
///
/// `#[non_exhaustive]` because future platform additions (e.g.
/// macOS DeskView) may add discriminators without API break.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum CameraDeviceKind {
    /// Built-in camera (e.g. MacBook FaceTime HD, integrated laptop
    /// webcams). `AVCaptureDeviceTypeBuiltInWideAngleCamera` on
    /// macOS.
    BuiltIn,
    /// External USB / capture-card device.
    /// `AVCaptureDeviceTypeExternalUnknown` on macOS.
    External,
    /// Software-virtual camera (OBS Virtual Camera, Snap Camera,
    /// etc.).
    Virtual,
    /// Apple Continuity Camera (iPhone-as-webcam, macOS-only).
    /// Distinct discriminator per `docs/54` Phase-2 lesson 9 —
    /// Continuity devices appear / disappear as the iPhone connects.
    Continuity,
    /// Unrecognised device-kind. Acts as the catch-all to keep
    /// enumeration moving when nokhwa surfaces a kind the adapter
    /// does not yet model.
    Other,
}

/// Device enumeration summary. Returned from
/// [`CameraAdapter::enumerate`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceSummary {
    /// Platform-stable nokhwa device identifier. Persists across
    /// sessions in app settings per ADR-020 §6; reconnect uses
    /// this string verbatim.
    pub device_id: String,
    /// Human-readable display name surfaced to the operator UI.
    pub display_name: String,
    /// Coarse device-kind for UI grouping.
    pub kind: CameraDeviceKind,
}

/// Camera adapter binding configuration. Per ADR-020 §2.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CameraConfig {
    /// Platform-stable nokhwa device identifier the adapter binds
    /// against.
    pub device_id: String,
    /// Negotiated frame rate hint (frames per second). The
    /// hardware may or may not honour this; the realised cadence
    /// surfaces via [`CameraFrame::timestamp_ns`].
    pub target_fps: u32,
    /// Negotiated capture resolution hint (width, height).
    pub target_resolution: (u32, u32),
    /// Pixel format preference. `None` means "let the hardware
    /// pick". The realised format surfaces via
    /// [`CameraFrame::pixel_format`].
    pub pixel_format_preference: Option<PixelFormat>,
    /// In-adapter frame queue depth. Drop-oldest-on-full per
    /// ADR-020 §4.
    pub queue_depth: usize,
    /// Reconnect attempts on `Disconnected → Bound` recovery.
    /// Per ADR-020 §6.
    pub reconnect_attempts: u32,
    /// Reconnect interval (ms) per attempt. Per ADR-020 §6.
    pub reconnect_interval_ms: u32,
}

impl CameraConfig {
    /// Construct a [`CameraConfig`] for the given device id with
    /// every other field at the [`crate::defaults`] value. The
    /// preference field is set to [`Some(DEFAULT_PIXEL_FORMAT_PREFERENCE)`]
    /// so a fresh config drives the canonical NV12 zero-copy path
    /// per ADR-020 §3 / §7; callers can clear the preference to opt
    /// in to "let the hardware pick".
    ///
    /// [`Some(DEFAULT_PIXEL_FORMAT_PREFERENCE)`]: crate::defaults::DEFAULT_PIXEL_FORMAT_PREFERENCE
    #[must_use]
    pub fn defaults_for(device_id: impl Into<String>) -> Self {
        Self {
            device_id: device_id.into(),
            target_fps: DEFAULT_FRAME_RATE,
            target_resolution: DEFAULT_RESOLUTION,
            pixel_format_preference: Some(DEFAULT_PIXEL_FORMAT_PREFERENCE),
            queue_depth: DEFAULT_QUEUE_DEPTH,
            reconnect_attempts: DEFAULT_RECONNECT_ATTEMPTS,
            reconnect_interval_ms: DEFAULT_RECONNECT_INTERVAL_MS,
        }
    }
}

/// Operational health of a camera adapter, mirroring the OSC / MIDI
/// adapter state machines. Per ADR-020 §Decision §2.
///
/// `Faulted(reason)` carries a stable diagnostic string drawn from
/// the canonical fault-reason vocabulary in [`crate::frames`]
/// (`PERMISSION_DENIED`, `DEVICE_BUSY`, `DEVICE_NOT_FOUND`,
/// `FORMAT_NOT_SUPPORTED`, `RECONNECT_EXHAUSTED`, `BACKEND_ERROR`).
/// Reasons live as `pub const` items rather than free string
/// literals so the conformance corpus matches against the constants
/// and the operator-facing UX surface localizes off them.
///
/// `#[non_exhaustive]` because future health states (e.g. a
/// `Recovering` discriminator orthogonal to `Bound` à la the MIDI
/// adapter's `MidiHealthState::Recovering`) may land additively.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AdapterHealth {
    /// Adapter is bound to a device and the capture path is
    /// healthy.
    Bound,
    /// Adapter is not bound. Either initial state or post-`unbind`.
    Disconnected,
    /// Adapter is in the middle of a bind handshake — either
    /// negotiating format with the hardware, or awaiting an OS-level
    /// permission grant per the async permission flow contract
    /// (ADR-020 §5 + `docs/54` Phase-2 lesson 2). The Tauri shell
    /// surfaces a non-modal "awaiting permission" state during this
    /// phase and re-renders to the granted / denied state when the
    /// adapter health transitions.
    Negotiating,
    /// Adapter has entered a terminal fault state. The reason is one
    /// of the canonical kebab-case strings from [`crate::frames`].
    Faulted(Cow<'static, str>),
}

/// Errors surfaced by camera adapter operations.
///
/// Each variant maps to one of the canonical fault-reason
/// `pub const` strings in [`crate::frames`] via
/// [`AdapterError::canonical_reason`]. Pinned per ADR-020 §Decision
/// §2 "Canonical fault reason vocabulary".
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AdapterError {
    /// OS denied camera access via TCC (macOS) / Privacy Settings
    /// (Windows). Maps to [`PERMISSION_DENIED`].
    #[error("permission-denied: {0}")]
    PermissionDenied(String),
    /// Device is held by another application
    /// (`AVCaptureDevice.isInUseByAnotherApplication == YES` on
    /// macOS; `MF_E_DEVICE_INVALIDATED` mid-stream on Windows).
    /// Maps to [`DEVICE_BUSY`].
    #[error("device-busy: {0}")]
    DeviceBusy(String),
    /// Persisted device-id no longer enumerable. Maps to
    /// [`DEVICE_NOT_FOUND`].
    #[error("device-not-found: {0}")]
    DeviceNotFound(String),
    /// No overlap between requested and offered formats, OR the
    /// negotiated format is `Mjpeg` and JPEG decode is not yet
    /// wired at the render side. Maps to [`FORMAT_NOT_SUPPORTED`].
    #[error("format-not-supported: {0}")]
    FormatNotSupported(String),
    /// `reconnect_attempts × reconnect_interval_ms` window
    /// exhausted without re-enumerating the persisted device. Maps
    /// to [`RECONNECT_EXHAUSTED`].
    #[error("reconnect-exhausted: {0}")]
    ReconnectExhausted(String),
    /// Generic nokhwa-internal failure. Maps to [`BACKEND_ERROR`].
    #[error("backend-error: {0}")]
    BackendError(String),
}

impl AdapterError {
    /// Maps each variant to its canonical fault-reason `pub const`
    /// string per ADR-020 §Decision §2. Used by the per-platform
    /// backends (Phase 2 PR 2 / PR 3) to route an error into the
    /// `AdapterHealth::Faulted(reason)` health state without string
    /// duplication.
    #[must_use]
    pub fn canonical_reason(&self) -> &'static str {
        match self {
            Self::PermissionDenied(_) => PERMISSION_DENIED,
            Self::DeviceBusy(_) => DEVICE_BUSY,
            Self::DeviceNotFound(_) => DEVICE_NOT_FOUND,
            Self::FormatNotSupported(_) => FORMAT_NOT_SUPPORTED,
            Self::ReconnectExhausted(_) => RECONNECT_EXHAUSTED,
            Self::BackendError(_) => BACKEND_ERROR,
        }
    }
}

/// Camera adapter — protocol-abstract trait per ADR-005, specialised
/// to the camera enumeration / bind / poll / health surface ADR-020
/// §2 names.
///
/// `bind` opens the device and starts the capture thread; `unbind`
/// joins it; `poll(max_frames)` drains up to `max_frames` from the
/// internal queue without blocking the caller; `health` reports the
/// current operational state; `enumerate` returns the platform-
/// available device list.
pub trait CameraAdapter {
    /// Enumerate available camera devices.
    ///
    /// On macOS the first call may trigger an asynchronous
    /// `AVCaptureDevice.requestAccess` prompt; the adapter returns
    /// immediately with the cached device list (possibly empty) and
    /// the OS completion handler transitions the adapter health
    /// asynchronously per ADR-020 §5 + `docs/54` Phase-2 lesson 2.
    ///
    /// On Windows the device list enumerates regardless of Privacy
    /// Settings state; the bind-time `MFCreateSourceReader` failure
    /// surfaces the permission denial.
    fn enumerate(&self) -> Result<Vec<DeviceSummary>, AdapterError>;

    /// Bind to the camera identified by `config.device_id` with the
    /// given hints. Idempotent on success: re-binding to the same
    /// device with the same config is a no-op.
    fn bind(&mut self, config: CameraConfig) -> Result<(), AdapterError>;

    /// Release any held capture resources.
    ///
    /// Idempotent: calling `unbind` on an already-disconnected
    /// adapter is not an error. Implementations MUST NOT leak the
    /// nokhwa resources across `unbind` boundaries.
    fn unbind(&mut self) -> Result<(), AdapterError>;

    /// Drain up to `max_frames` queued frames non-blockingly.
    ///
    /// The producer-side drop-on-full policy (drop oldest, never
    /// the newest) is enforced by the per-platform backend per
    /// ADR-020 §4 + `docs/54` Phase-2 lesson 1.
    ///
    /// Returns an empty `Vec` when the adapter is unbound, when
    /// `max_frames == 0`, or when no frames are queued.
    fn poll(&mut self, max_frames: usize) -> Vec<CameraFrame>;

    /// Current operational health state.
    fn health(&self) -> AdapterHealth;
}

/// nokhwa-backed camera adapter scaffold.
///
/// Phase 2 PR 1 ships the trait surface + state-machine shape only;
/// the per-platform nokhwa backends land in Phase 2 PR 2 (macOS
/// CoreMediaIO / AVFoundation) and Phase 2 PR 3 (Windows Media
/// Foundation). The PR 1 shell:
///
/// * `enumerate` returns an empty vec.
/// * `bind` returns [`AdapterError::DeviceNotFound`] (the persisted
///   device is unreachable because no backend is enumerating).
/// * `unbind` is a no-op idempotent transition.
/// * `poll` returns an empty vec.
/// * `health` starts at [`AdapterHealth::Disconnected`].
///
/// This shape is intentionally non-functional at the OS layer so PR
/// 1 can land without macOS / Windows feature flags or platform
/// permission wiring; the shell reserves the trait surface and the
/// internal queue + state-machine fields the per-platform backends
/// will populate.
pub struct NokhwaInputAdapter {
    /// Currently-bound config, when in `Bound`. Cleared on `unbind`.
    bound_config: Option<CameraConfig>,
    /// Internal frame queue. Producer side is the per-platform
    /// nokhwa thread (Phase 2 PR 2 / PR 3); consumer side is
    /// `poll(max_frames)`. Empty in PR 1.
    frame_queue: VecDeque<CameraFrame>,
    /// Current health-state. Starts at `Disconnected`; `bind`
    /// drives `Negotiating → Bound` (or `Faulted(reason)` on
    /// failure).
    health: AdapterHealth,
}

impl NokhwaInputAdapter {
    /// Construct a fresh, unbound adapter.
    ///
    /// The nokhwa client is created lazily on `bind` rather than at
    /// construction so a fresh adapter can be instantiated in
    /// environments without camera-subsystem access (e.g. CI
    /// sandboxes for the synthetic T2 corpus that ships with Phase
    /// 2 PR 4).
    #[must_use]
    pub fn new() -> Self {
        Self {
            bound_config: None,
            frame_queue: VecDeque::new(),
            health: AdapterHealth::Disconnected,
        }
    }

    /// Borrowed access to the currently-bound config, when bound.
    #[must_use]
    pub fn bound_config(&self) -> Option<&CameraConfig> {
        self.bound_config.as_ref()
    }
}

impl Default for NokhwaInputAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraAdapter for NokhwaInputAdapter {
    fn enumerate(&self) -> Result<Vec<DeviceSummary>, AdapterError> {
        // PR 1 scaffold: no backend is enumerating, so the device
        // list is empty. Phase 2 PR 2 / PR 3 wire the per-platform
        // nokhwa enumeration; the trait contract permits an empty
        // vec on a system with no cameras attached.
        Ok(Vec::new())
    }

    fn bind(&mut self, config: CameraConfig) -> Result<(), AdapterError> {
        // PR 1 scaffold: no backend is enumerating, so any
        // persisted device-id is unreachable. The error variant
        // surfaces the device id so test fixtures can match against
        // the request without poking adapter internals. Health
        // moves Disconnected -> Negotiating -> Disconnected to
        // match the eventual real-bind state-machine shape and
        // reset the bound-config cache.
        self.health = AdapterHealth::Negotiating;
        let device_id = config.device_id.clone();
        self.bound_config = None;
        self.health = AdapterHealth::Disconnected;
        Err(AdapterError::DeviceNotFound(format!(
            "camera-adapter PR 1 scaffold: backend not yet wired (requested device {device_id:?}); \
             macOS backend lands in Phase 2 PR 2 + Windows backend in Phase 2 PR 3"
        )))
    }

    fn unbind(&mut self) -> Result<(), AdapterError> {
        // Idempotent: unbind on an already-disconnected adapter is
        // not an error. Mirrors the OSC / MIDI adapter's
        // disconnect-idempotency posture.
        self.bound_config = None;
        self.frame_queue.clear();
        self.health = AdapterHealth::Disconnected;
        Ok(())
    }

    fn poll(&mut self, max_frames: usize) -> Vec<CameraFrame> {
        if max_frames == 0 {
            return Vec::new();
        }
        // PR 1 scaffold: the producer side is unwired so the queue
        // is always empty. Per ADR-020 §4 the eventual drop-on-full
        // policy lives at the producer (per-platform backend); the
        // consumer-side drain is bounded by `max_frames`.
        let drain_count = max_frames.min(self.frame_queue.len());
        self.frame_queue.drain(..drain_count).collect()
    }

    fn health(&self) -> AdapterHealth {
        self.health.clone()
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod adapter_pins {
    //! Evidence: `EV-INTEGRATION-MATRIX` (T1) — adapter trait
    //! surface + state-machine + canonical-error-routing invariants
    //! for the Phase 2 PR 1 nokhwa-backed scaffold.
    //!
    //! Per `docs/43` §15.5 T1: each test pins a specific ADR-020
    //! contract invariant against the implementation. Live camera
    //! coverage requires platform camera-subsystem access and is
    //! not run in CI; the live nokhwa path is exercised by the
    //! Phase 2 PR 4 synthetic T2 corpus per the row 78 / row 40 /
    //! row 93 narrowing precedent.
    //!
    //! The PR 1 scaffold's error routing (`bind` always returns
    //! `DeviceNotFound`, `enumerate` always returns empty) is the
    //! intentional surface-only posture; the per-platform backends
    //! land in Phase 2 PR 2 + PR 3.

    use super::*;

    /// `EV-INTEGRATION-MATRIX` T1: a fresh adapter starts at
    /// `Disconnected`, not at `Bound` or `Negotiating`. Per
    /// ADR-020 §Decision §2 health state machine.
    #[test]
    fn nokhwa_input_adapter_new_starts_disconnected() {
        let adapter = NokhwaInputAdapter::new();
        assert!(matches!(adapter.health(), AdapterHealth::Disconnected));
        assert!(adapter.bound_config().is_none());
    }

    /// `EV-INTEGRATION-MATRIX` T1: PR 1 scaffold's `enumerate`
    /// returns an empty vec rather than panicking — the trait
    /// contract permits empty enumeration, and PR 2 / PR 3 wire
    /// real device lists.
    #[test]
    fn enumerate_on_pr1_scaffold_returns_empty_vec() {
        let adapter = NokhwaInputAdapter::new();
        let devices = adapter.enumerate().expect("enumerate scaffold succeeds");
        assert!(devices.is_empty());
    }

    /// `EV-INTEGRATION-MATRIX` T1: `bind` against any device id in
    /// the PR 1 scaffold routes through `DeviceNotFound` (the
    /// persisted-id-unreachable path) rather than `BackendError`
    /// or `PermissionDenied`. The fault-reason mapping must be
    /// stable across PR 2 / PR 3 wiring even when the actual error
    /// path differs at runtime.
    #[test]
    fn bind_with_unknown_device_returns_devicenotfound() {
        let mut adapter = NokhwaInputAdapter::new();
        let config = CameraConfig::defaults_for("nonexistent-device-id");
        let err = adapter.bind(config).expect_err("PR 1 bind always errors");
        assert!(matches!(err, AdapterError::DeviceNotFound(_)));
        // Health resets to Disconnected on bind failure rather
        // than leaving the adapter in Negotiating limbo.
        assert!(matches!(adapter.health(), AdapterHealth::Disconnected));
        assert!(adapter.bound_config().is_none());
    }

    /// `EV-INTEGRATION-MATRIX` T1: `unbind` is idempotent. Mirrors
    /// the OSC / MIDI adapter's disconnect-idempotency posture so
    /// the trait shape across the three adapters stays uniform for
    /// the IntegrationHub translation layer (ADR-005 "Decision"
    /// bullet 3).
    #[test]
    fn unbind_after_unbind_is_idempotent() {
        let mut adapter = NokhwaInputAdapter::new();
        adapter.unbind().expect("first unbind succeeds");
        adapter.unbind().expect("second unbind also succeeds");
        assert!(matches!(adapter.health(), AdapterHealth::Disconnected));
    }

    /// `EV-INTEGRATION-MATRIX` T1: each `AdapterError` variant maps
    /// to its canonical kebab-case fault-reason `pub const` per
    /// ADR-020 §Decision §2. Pinned because the per-platform
    /// backends (Phase 2 PR 2 / PR 3) route errors into
    /// `AdapterHealth::Faulted(reason)` via this mapping; any
    /// drift would silently break the operator-facing health
    /// surface.
    #[test]
    fn adapter_error_canonical_reason_uses_kebab_case() {
        let cases: &[(AdapterError, &str)] = &[
            (
                AdapterError::PermissionDenied(String::from("x")),
                PERMISSION_DENIED,
            ),
            (AdapterError::DeviceBusy(String::from("x")), DEVICE_BUSY),
            (
                AdapterError::DeviceNotFound(String::from("x")),
                DEVICE_NOT_FOUND,
            ),
            (
                AdapterError::FormatNotSupported(String::from("x")),
                FORMAT_NOT_SUPPORTED,
            ),
            (
                AdapterError::ReconnectExhausted(String::from("x")),
                RECONNECT_EXHAUSTED,
            ),
            (AdapterError::BackendError(String::from("x")), BACKEND_ERROR),
        ];
        for (err, expected) in cases {
            assert_eq!(
                err.canonical_reason(),
                *expected,
                "canonical reason mismatch for {err:?}",
            );
        }
    }

    /// `EV-INTEGRATION-MATRIX` T1: `AdapterError` Display surface
    /// (provided by `thiserror`) carries the canonical reason as
    /// the prefix. Pinned because `tracing` / log scrapes filter
    /// off the prefix; any drift would silently break operator
    /// triage of camera errors in production logs.
    #[test]
    fn adapter_error_display_prefix_matches_canonical_reason() {
        let err = AdapterError::PermissionDenied(String::from("camera blocked"));
        let rendered = format!("{err}");
        assert!(
            rendered.starts_with("permission-denied:"),
            "expected canonical-reason prefix, got {rendered:?}",
        );
    }

    /// `EV-INTEGRATION-MATRIX` T1: `CameraConfig::defaults_for`
    /// pulls each non-id field from `crate::defaults`. Pinned so
    /// any drift in the constants module's value set re-fires the
    /// `defaults_pins::defaults_match_adr_020` test alongside this
    /// one — the constructor and the constants travel together.
    #[test]
    fn camera_config_defaults_for_uses_defaults_module() {
        let config = CameraConfig::defaults_for("device-x");
        assert_eq!(config.device_id, "device-x");
        assert_eq!(config.target_fps, DEFAULT_FRAME_RATE);
        assert_eq!(config.target_resolution, DEFAULT_RESOLUTION);
        assert_eq!(
            config.pixel_format_preference,
            Some(DEFAULT_PIXEL_FORMAT_PREFERENCE),
        );
        assert_eq!(config.queue_depth, DEFAULT_QUEUE_DEPTH);
        assert_eq!(config.reconnect_attempts, DEFAULT_RECONNECT_ATTEMPTS);
        assert_eq!(config.reconnect_interval_ms, DEFAULT_RECONNECT_INTERVAL_MS);
    }

    /// `EV-INTEGRATION-MATRIX` T1: `poll(0)` is a no-op even on a
    /// scaffold adapter — the bound `max_frames` upper-cap is the
    /// API contract, not a property of the producer.
    #[test]
    fn poll_zero_max_frames_returns_empty_without_side_effects() {
        let mut adapter = NokhwaInputAdapter::new();
        let frames = adapter.poll(0);
        assert!(frames.is_empty());
        assert!(matches!(adapter.health(), AdapterHealth::Disconnected));
    }

    /// `EV-INTEGRATION-MATRIX` T1: the `AdapterHealth::Faulted`
    /// variant carries the canonical reason as a `Cow<'static, str>`
    /// — pinned because the operator-facing UX surface borrows the
    /// reason without allocation when it comes from the `pub const`
    /// table. A `String`-only variant would force a needless
    /// allocation per fault dispatch.
    #[test]
    fn adapter_health_faulted_borrows_canonical_reason() {
        let health = AdapterHealth::Faulted(Cow::Borrowed(PERMISSION_DENIED));
        match health {
            AdapterHealth::Faulted(reason) => {
                assert_eq!(reason.as_ref(), PERMISSION_DENIED);
                assert!(matches!(reason, Cow::Borrowed(_)));
            }
            other => panic!("expected Faulted, got {other:?}"),
        }
    }
}
