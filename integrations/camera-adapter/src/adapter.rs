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
#[cfg(target_os = "macos")]
use crate::macos::{AuthorizationProbe, AuthorizationStatus, InUsePrecheck};
#[cfg(target_os = "windows")]
use crate::windows::WindowsCameraProbe;

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
    /// macOS-only authorization-status probe. Trait-object DI seam
    /// per ADR-020 §5 + `docs/54-wave-1c-beta-research-lessons.md`
    /// Phase-2 lesson 2; defaults to
    /// [`crate::macos::PlaceholderProbe`] returning
    /// `NotDetermined` until the policy-scoped follow-up PR ships
    /// the real `AVCaptureDevice` binding. Tests inject mock
    /// probes via [`Self::with_authorization_probe`].
    #[cfg(target_os = "macos")]
    authorization_probe: Box<dyn AuthorizationProbe>,
    /// macOS-only in-use precheck. Trait-object DI seam per
    /// ADR-020 §5 + `docs/54` Phase-2 lesson 10. Defaults to
    /// [`crate::macos::PlaceholderProbe`]; tests inject mocks via
    /// [`Self::with_in_use_precheck`].
    #[cfg(target_os = "macos")]
    in_use_precheck: Box<dyn InUsePrecheck>,
    /// Windows-only Privacy-Settings probe + HRESULT-mapper.
    /// Trait-object DI seam per ADR-020 §5 + §6 "Per-platform
    /// notes — Windows"; defaults to
    /// [`crate::windows::PlaceholderProbe`] returning `Unknown`
    /// until the policy-scoped follow-up PR ships the real
    /// consent-store + Media Foundation HRESULT binding. Tests
    /// inject mock probes via
    /// [`Self::with_windows_camera_probe`].
    #[cfg(target_os = "windows")]
    windows_probe: Box<dyn WindowsCameraProbe>,
}

impl NokhwaInputAdapter {
    /// Construct a fresh, unbound adapter.
    ///
    /// The nokhwa client is created lazily on `bind` rather than at
    /// construction so a fresh adapter can be instantiated in
    /// environments without camera-subsystem access (e.g. CI
    /// sandboxes for the synthetic T2 corpus that ships with Phase
    /// 2 PR 4).
    ///
    /// On macOS the adapter holds default probes
    /// ([`crate::macos::PlaceholderProbe`]) for the authorization-
    /// status query + in-use precheck per ADR-020 §5; tests
    /// override with [`Self::with_authorization_probe`] /
    /// [`Self::with_in_use_precheck`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            bound_config: None,
            frame_queue: VecDeque::new(),
            health: AdapterHealth::Disconnected,
            #[cfg(target_os = "macos")]
            authorization_probe: Box::new(crate::macos::PlaceholderProbe),
            #[cfg(target_os = "macos")]
            in_use_precheck: Box::new(crate::macos::PlaceholderProbe),
            #[cfg(target_os = "windows")]
            windows_probe: Box::new(crate::windows::PlaceholderProbe),
        }
    }

    /// Borrowed access to the currently-bound config, when bound.
    #[must_use]
    pub fn bound_config(&self) -> Option<&CameraConfig> {
        self.bound_config.as_ref()
    }

    /// Override the macOS authorization-status probe. Tests inject
    /// mock probes through this seam to drive the adapter state
    /// machine through every [`crate::macos::AuthorizationStatus`]
    /// arm without depending on the OS TCC subsystem.
    #[cfg(target_os = "macos")]
    #[must_use]
    pub fn with_authorization_probe(mut self, probe: Box<dyn AuthorizationProbe>) -> Self {
        self.authorization_probe = probe;
        self
    }

    /// Override the macOS in-use precheck. Tests inject mocks
    /// through this seam to exercise the
    /// `Faulted("device-busy")` transition deterministically.
    #[cfg(target_os = "macos")]
    #[must_use]
    pub fn with_in_use_precheck(mut self, precheck: Box<dyn InUsePrecheck>) -> Self {
        self.in_use_precheck = precheck;
        self
    }

    /// Override the Windows Privacy-Settings probe + HRESULT-
    /// mapper. Tests inject mock probes through this seam to drive
    /// the adapter state machine through every
    /// [`crate::windows::WindowsPermissionStatus`] arm + the
    /// canonical HRESULT-mapping taxonomy without depending on a
    /// real Media Foundation runtime.
    #[cfg(target_os = "windows")]
    #[must_use]
    pub fn with_windows_camera_probe(mut self, probe: Box<dyn WindowsCameraProbe>) -> Self {
        self.windows_probe = probe;
        self
    }
}

impl Default for NokhwaInputAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraAdapter for NokhwaInputAdapter {
    fn enumerate(&self) -> Result<Vec<DeviceSummary>, AdapterError> {
        // On macOS, consult the authorization-status probe before
        // surfacing the device list. Per ADR-020 §5: if the operator
        // has explicitly denied access (or MDM has restricted it),
        // return an empty vec rather than tantalizing them with a
        // device they cannot use; if `NotDetermined`, also return
        // empty — the OS-level prompt is kicked from `bind`, not
        // from `enumerate`. Only the `Authorized` arm proceeds to
        // the (still-stubbed) nokhwa enumeration.
        #[cfg(target_os = "macos")]
        {
            match self.authorization_probe.current() {
                AuthorizationStatus::Authorized => {
                    // Fall through to the cross-platform enumeration
                    // path below.
                }
                AuthorizationStatus::NotDetermined
                | AuthorizationStatus::Denied
                | AuthorizationStatus::Restricted => {
                    return Ok(Vec::new());
                }
            }
        }

        // On Windows, consult the Privacy-Settings probe before
        // surfacing the device list. Per ADR-020 §6: Media
        // Foundation device-enumeration succeeds even with camera
        // privacy blocked (the device list is visible) but
        // `MFCreateSourceReader` fails; surfacing devices the
        // operator cannot bind would tantalize them. Per the
        // brief direction for this PR, the adapter pre-empts that
        // by routing an explicitly-`Denied` consent-store status
        // to an empty enumeration vec. `Granted` and `Unknown`
        // both proceed (the open-call HRESULT is the arbiter when
        // the consent-store query is indeterminate, and the
        // bind-time mapping surfaces the eventual fault).
        #[cfg(target_os = "windows")]
        {
            if self.windows_probe.current().is_terminal_denial() {
                return Ok(Vec::new());
            }
        }

        // Phase 2 PR 2 scaffold: no backend is enumerating yet on
        // any platform, so the device list is empty. Phase 2 PR 3
        // (Windows) and the follow-up policy-scoped PR (macOS
        // AVFoundation binding) wire the per-platform nokhwa
        // enumeration; the trait contract permits an empty vec on
        // a system with no cameras attached.
        Ok(Vec::new())
    }

    fn bind(&mut self, config: CameraConfig) -> Result<(), AdapterError> {
        // macOS permission state machine per ADR-020 §5 +
        // `docs/54-wave-1c-beta-research-lessons.md` Phase-2 lessons
        // 2 + 10:
        //
        //   1. Query authorization status.
        //      * `Denied` / `Restricted` → terminal
        //        `Faulted("permission-denied")` immediately.
        //      * `NotDetermined` → transition to `Negotiating`
        //        (the OS prompt is kicked by the concrete
        //        `RequestAccess` impl in the follow-up policy-
        //        scoped PR; the placeholder leaves the adapter in
        //        `Negotiating` until a real probe transitions it).
        //      * `Authorized` → fall through to in-use precheck.
        //   2. In-use precheck. If another macOS app holds the
        //      device, transition to `Faulted("device-busy")` —
        //      distinct from `permission-denied` per the canonical
        //      reason vocabulary in ADR-020 §2.
        //   3. nokhwa-bind (deferred to the follow-up policy-scoped
        //      PR + Phase 2 PR 3 for Windows).
        #[cfg(target_os = "macos")]
        {
            let status = self.authorization_probe.current();
            if status.is_terminal_denial() {
                self.bound_config = None;
                self.health = AdapterHealth::Faulted(Cow::Borrowed(PERMISSION_DENIED));
                return Err(AdapterError::PermissionDenied(format!(
                    "camera authorization status is terminal-denial ({status:?}); \
                     operator must grant access via System Settings \
                     before re-binding"
                )));
            }
            if !status.permits_bind() {
                // NotDetermined: kick the async OS prompt path. The
                // placeholder `RequestAccess` impl is a no-op; the
                // adapter stays in `Negotiating` until the concrete
                // impl (follow-up policy-scoped PR) transitions it
                // via the OS completion handler. Returning a
                // `PermissionDenied` error here would short-circuit
                // the async contract; instead we surface the
                // negotiating state to the caller and let them
                // re-enumerate / re-bind once the operator responds
                // to the prompt.
                self.health = AdapterHealth::Negotiating;
                self.bound_config = None;
                return Err(AdapterError::PermissionDenied(format!(
                    "camera authorization status is {status:?}; \
                     awaiting operator response to OS prompt \
                     (async permission-flow contract per ADR-020 §5)"
                )));
            }
            if self.in_use_precheck.is_in_use(&config.device_id) {
                self.bound_config = None;
                self.health = AdapterHealth::Faulted(Cow::Borrowed(DEVICE_BUSY));
                return Err(AdapterError::DeviceBusy(format!(
                    "camera device {:?} is in use by another macOS \
                     application; operator must close the holding \
                     application before re-binding",
                    config.device_id
                )));
            }
        }

        // Windows permission state machine per ADR-020 §5 + §6
        // "Per-platform notes — Windows" + the brief direction for
        // Wave 1c-β Phase 2 PR 3:
        //
        //   1. Query the Privacy-Settings consent-store status.
        //      * `Denied` → terminal
        //        `Faulted("permission-denied")` immediately. Unlike
        //        macOS, Windows has no in-app permission prompt;
        //        the operator-facing UX surface deep-links to
        //        `ms-settings:privacy-webcam` per ADR-020 §6 line
        //        156, and the adapter's recovery path is a manual
        //        re-bind once the operator grants access.
        //      * `Granted` / `Unknown` → fall through to the
        //        nokhwa-bind path; the eventual HRESULT from the
        //        Media Foundation open call is the authoritative
        //        arbiter via [`crate::windows::map_hresult_default`].
        //   2. nokhwa-bind (deferred to the follow-up policy-scoped
        //      PR; on a real bind, `MF_E_DEVICE_INVALIDATED` →
        //      `Faulted("device-busy")`, `E_ACCESSDENIED` →
        //      `Faulted("permission-denied")`, success → `Bound`).
        #[cfg(target_os = "windows")]
        {
            let status = self.windows_probe.current();
            if status.is_terminal_denial() {
                self.bound_config = None;
                self.health = AdapterHealth::Faulted(Cow::Borrowed(PERMISSION_DENIED));
                return Err(AdapterError::PermissionDenied(format!(
                    "Windows Privacy-Settings camera access is denied \
                     ({status:?}); operator must grant access via \
                     ms-settings:privacy-webcam before re-binding"
                )));
            }
        }

        // Phase 2 PR 2 scaffold: no backend is enumerating, so any
        // persisted device-id is unreachable from the (still-stubbed)
        // nokhwa-bind path. The error variant surfaces the device id
        // so test fixtures can match against the request without
        // poking adapter internals. Health moves
        // Disconnected -> Negotiating -> Disconnected to match the
        // eventual real-bind state-machine shape and reset the
        // bound-config cache.
        self.health = AdapterHealth::Negotiating;
        let device_id = config.device_id.clone();
        self.bound_config = None;
        self.health = AdapterHealth::Disconnected;
        Err(AdapterError::DeviceNotFound(format!(
            "camera-adapter Phase 2 PR 2 scaffold: nokhwa backend not yet \
             wired (requested device {device_id:?}); macOS AVFoundation \
             binding lands in the follow-up policy-scoped PR + Windows \
             Media Foundation backend lands in Phase 2 PR 3"
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

    /// `EV-INTEGRATION-MATRIX` T1: `bind` with an `Authorized`
    /// authorization probe + a not-in-use device routes through
    /// `DeviceNotFound` (the persisted-id-unreachable path) rather
    /// than `BackendError` or `PermissionDenied`. The fault-reason
    /// mapping must be stable across PR 2 / PR 3 wiring even when
    /// the actual error path differs at runtime. The test injects
    /// the `Authorized` probe explicitly because the default
    /// placeholder returns `NotDetermined` which now routes
    /// through the negotiating-permission path per ADR-020 §5.
    #[test]
    fn bind_with_unknown_device_returns_devicenotfound() {
        #[cfg(target_os = "macos")]
        let adapter = {
            use crate::macos::AuthorizationStatus;
            use crate::macos::test_support::{MockAuthorizationProbe, MockInUsePrecheck};
            NokhwaInputAdapter::new()
                .with_authorization_probe(Box::new(MockAuthorizationProbe {
                    status: AuthorizationStatus::Authorized,
                }))
                .with_in_use_precheck(Box::new(MockInUsePrecheck { in_use: false }))
        };
        #[cfg(target_os = "windows")]
        let adapter = {
            use crate::windows::WindowsPermissionStatus;
            use crate::windows::test_support::MockWindowsCameraProbe;
            NokhwaInputAdapter::new().with_windows_camera_probe(Box::new(MockWindowsCameraProbe {
                status: WindowsPermissionStatus::Granted,
            }))
        };
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let adapter = NokhwaInputAdapter::new();

        let mut adapter = adapter;
        let config = CameraConfig::defaults_for("nonexistent-device-id");
        let err = adapter.bind(config).expect_err("PR 2 bind always errors");
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

    // -----------------------------------------------------------------
    // macOS permission-flow state-machine pins per ADR-020 §5.
    //
    // The four arms below cover the cross product of authorization
    // status (Denied / Restricted / Authorized + in-use / Authorized
    // + not-in-use) under DI-injected mocks. The `NotDetermined`
    // arm is exercised by the `bind_with_notdetermined_authorization_negotiating`
    // pin which verifies the negotiating-permission contract per the
    // async permission-flow contract in `docs/54` Phase-2 lesson 2.
    // -----------------------------------------------------------------

    /// `EV-INTEGRATION-MATRIX` T1: `bind` with `Denied` authorization
    /// transitions to `Faulted("permission-denied")` immediately
    /// without firing an OS-level prompt — operator must
    /// re-authorize via System Settings. Pinned because routing
    /// `Denied` through the negotiating-async-prompt path would
    /// silently re-fire the prompt on every bind attempt within a
    /// process, which violates macOS TCC's process-bound grant
    /// semantics.
    #[cfg(target_os = "macos")]
    #[test]
    fn bind_with_denied_authorization_transitions_to_faulted_permission_denied() {
        use crate::macos::AuthorizationStatus;
        use crate::macos::test_support::MockAuthorizationProbe;
        let mut adapter =
            NokhwaInputAdapter::new().with_authorization_probe(Box::new(MockAuthorizationProbe {
                status: AuthorizationStatus::Denied,
            }));
        let err = adapter
            .bind(CameraConfig::defaults_for("any-device"))
            .expect_err("Denied authorization always errors");
        assert!(matches!(err, AdapterError::PermissionDenied(_)));
        match adapter.health() {
            AdapterHealth::Faulted(reason) => {
                assert_eq!(reason.as_ref(), PERMISSION_DENIED);
            }
            other => panic!("expected Faulted(permission-denied), got {other:?}"),
        }
        assert!(adapter.bound_config().is_none());
    }

    /// `EV-INTEGRATION-MATRIX` T1: `Restricted` authorization
    /// (Parental Controls / MDM) routes through the same terminal
    /// `Faulted("permission-denied")` transition as `Denied`. Pinned
    /// because the operator-facing UX surface conflates the two
    /// (both surface the "permission required" banner); the
    /// distinction matters only in the System Settings deep-link copy
    /// which is non-blocking.
    #[cfg(target_os = "macos")]
    #[test]
    fn bind_with_restricted_authorization_transitions_to_faulted_permission_denied() {
        use crate::macos::AuthorizationStatus;
        use crate::macos::test_support::MockAuthorizationProbe;
        let mut adapter =
            NokhwaInputAdapter::new().with_authorization_probe(Box::new(MockAuthorizationProbe {
                status: AuthorizationStatus::Restricted,
            }));
        let err = adapter
            .bind(CameraConfig::defaults_for("any-device"))
            .expect_err("Restricted authorization always errors");
        assert!(matches!(err, AdapterError::PermissionDenied(_)));
        match adapter.health() {
            AdapterHealth::Faulted(reason) => {
                assert_eq!(reason.as_ref(), PERMISSION_DENIED);
            }
            other => panic!("expected Faulted(permission-denied), got {other:?}"),
        }
    }

    /// `EV-INTEGRATION-MATRIX` T1: `bind` with `NotDetermined`
    /// authorization transitions to `AdapterHealth::Negotiating` and
    /// returns a `PermissionDenied` error code — but does NOT
    /// terminal-fault. The async permission-flow contract per
    /// ADR-020 §5 + `docs/54` Phase-2 lesson 2 requires the adapter
    /// to surface "awaiting OS prompt" rather than block on the
    /// completion handler. Pinned because routing `NotDetermined`
    /// through the terminal-fault path would silently break the
    /// async contract — the operator-facing UI would render
    /// "permission denied" before the operator has even seen the
    /// system prompt.
    #[cfg(target_os = "macos")]
    #[test]
    fn bind_with_notdetermined_authorization_negotiating() {
        use crate::macos::AuthorizationStatus;
        use crate::macos::test_support::MockAuthorizationProbe;
        let mut adapter =
            NokhwaInputAdapter::new().with_authorization_probe(Box::new(MockAuthorizationProbe {
                status: AuthorizationStatus::NotDetermined,
            }));
        let err = adapter
            .bind(CameraConfig::defaults_for("any-device"))
            .expect_err("NotDetermined authorization defers via err");
        assert!(matches!(err, AdapterError::PermissionDenied(_)));
        // Health stays at Negotiating rather than terminal-faulting,
        // so the operator-facing UI surfaces "awaiting permission"
        // not "permission denied". Caller re-binds once the OS
        // completion handler transitions the probe to `Authorized`
        // (or to terminal `Denied` on operator decline).
        assert!(matches!(adapter.health(), AdapterHealth::Negotiating));
    }

    /// `EV-INTEGRATION-MATRIX` T1: `bind` with `Authorized`
    /// authorization but an in-use device transitions to
    /// `Faulted("device-busy")` — distinct from the
    /// `permission-denied` reason. Pinned because the canonical
    /// reason vocabulary in ADR-020 §2 explicitly separates the two
    /// per `docs/54` Phase-2 lesson 10; the operator-facing UI
    /// shows "Camera is in use by <App Name>" rather than the
    /// permission-denied banner.
    #[cfg(target_os = "macos")]
    #[test]
    fn bind_with_in_use_device_transitions_to_faulted_device_busy() {
        use crate::macos::AuthorizationStatus;
        use crate::macos::test_support::{MockAuthorizationProbe, MockInUsePrecheck};
        let mut adapter = NokhwaInputAdapter::new()
            .with_authorization_probe(Box::new(MockAuthorizationProbe {
                status: AuthorizationStatus::Authorized,
            }))
            .with_in_use_precheck(Box::new(MockInUsePrecheck { in_use: true }));
        let err = adapter
            .bind(CameraConfig::defaults_for("held-device"))
            .expect_err("in-use device always errors");
        assert!(matches!(err, AdapterError::DeviceBusy(_)));
        match adapter.health() {
            AdapterHealth::Faulted(reason) => {
                assert_eq!(reason.as_ref(), DEVICE_BUSY);
                // device-busy and permission-denied must remain
                // distinct strings — the UX surface routes off this.
                assert_ne!(reason.as_ref(), PERMISSION_DENIED);
            }
            other => panic!("expected Faulted(device-busy), got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Windows permission-flow state-machine pins per ADR-020 §5 + §6.
    //
    // The arms below cover the Privacy-Settings consent-store status
    // taxonomy under DI-injected mocks. The `Unknown` arm exercises
    // the open-call-as-arbiter contract (the consent-store query is
    // indeterminate, so the eventual HRESULT drives the
    // state-machine transition); the `Denied` arm exercises the
    // terminal-fault path; the `Granted` arm composes with
    // device-busy mapping to verify the canonical HRESULT taxonomy
    // routes correctly.
    // -----------------------------------------------------------------

    /// `EV-INTEGRATION-MATRIX` T1: `bind` with `Denied` Privacy-
    /// Settings status transitions to
    /// `Faulted("permission-denied")` immediately without firing an
    /// open call — the Privacy-Settings deep-link
    /// (`ms-settings:privacy-webcam`) is the operator's recovery
    /// path. Pinned because routing `Denied` through the open-call
    /// path would silently swallow the user-visible operator
    /// guidance.
    #[cfg(target_os = "windows")]
    #[test]
    fn bind_with_windows_denied_permission_transitions_to_faulted_permission_denied() {
        use crate::windows::WindowsPermissionStatus;
        use crate::windows::test_support::MockWindowsCameraProbe;
        let mut adapter =
            NokhwaInputAdapter::new().with_windows_camera_probe(Box::new(MockWindowsCameraProbe {
                status: WindowsPermissionStatus::Denied,
            }));
        let err = adapter
            .bind(CameraConfig::defaults_for("any-device"))
            .expect_err("Denied Privacy-Settings always errors");
        assert!(matches!(err, AdapterError::PermissionDenied(_)));
        match adapter.health() {
            AdapterHealth::Faulted(reason) => {
                assert_eq!(reason.as_ref(), PERMISSION_DENIED);
            }
            other => panic!("expected Faulted(permission-denied), got {other:?}"),
        }
        assert!(adapter.bound_config().is_none());
    }

    /// `EV-INTEGRATION-MATRIX` T1: `bind` with the `MF_E_DEVICE_INVALIDATED`
    /// HRESULT route surfaces as
    /// `AdapterError::DeviceBusy` via the canonical mapping in
    /// [`crate::windows::map_hresult_default`]. The PR 3 scaffold
    /// does not yet wire the open-call HRESULT through the bind
    /// path (the nokhwa-bind path lands in a follow-up policy-
    /// scoped PR), so this pin exercises the mapping function
    /// directly + verifies the corresponding `AdapterError`
    /// `canonical_reason` round-trips to `device-busy` distinct
    /// from `permission-denied`.
    #[cfg(target_os = "windows")]
    #[test]
    fn bind_with_windows_in_use_device_transitions_to_faulted_device_busy() {
        use crate::windows::map_hresult_default;
        // MF_E_DEVICE_INVALIDATED → device-busy via canonical map.
        let reason = map_hresult_default(0xC00D_36B4);
        assert_eq!(reason, DEVICE_BUSY);
        // The reverse round-trip: an `AdapterError::DeviceBusy`
        // surfaces the same canonical reason, locking the
        // mapping symmetry the operator UX surface depends on.
        let err = AdapterError::DeviceBusy(String::from(
            "MF_E_DEVICE_INVALIDATED on MFCreateSourceReader",
        ));
        assert_eq!(err.canonical_reason(), DEVICE_BUSY);
        assert_ne!(err.canonical_reason(), PERMISSION_DENIED);
    }

    /// `EV-INTEGRATION-MATRIX` T1: `enumerate` with `Denied`
    /// Windows Privacy-Settings status returns an empty vec rather
    /// than enumerating devices the operator cannot bind. Pinned
    /// because the operator-facing device-picker UI iterates the
    /// enumeration vec; surfacing devices under a denied probe
    /// would tantalize the operator with cameras they cannot use.
    /// `Granted` and `Unknown` both proceed to the cross-platform
    /// (still-stubbed) enumeration path.
    #[cfg(target_os = "windows")]
    #[test]
    fn enumerate_with_windows_denied_returns_empty_vec() {
        use crate::windows::WindowsPermissionStatus;
        use crate::windows::test_support::MockWindowsCameraProbe;
        let adapter =
            NokhwaInputAdapter::new().with_windows_camera_probe(Box::new(MockWindowsCameraProbe {
                status: WindowsPermissionStatus::Denied,
            }));
        let devices = adapter
            .enumerate()
            .expect("enumerate succeeds under Denied status");
        assert!(devices.is_empty());
    }

    /// `EV-INTEGRATION-MATRIX` T1: `enumerate` with the placeholder
    /// `Unknown` probe falls through to the cross-platform
    /// scaffold (empty vec under PR 3, since no backend
    /// enumerates yet). Pinned because routing `Unknown` through
    /// the empty-vec short-circuit would silently break the
    /// open-call-as-arbiter contract — `Unknown` MUST proceed so
    /// the eventual nokhwa enumeration sees the device list.
    #[cfg(target_os = "windows")]
    #[test]
    fn enumerate_with_windows_unknown_proceeds() {
        use crate::windows::WindowsPermissionStatus;
        use crate::windows::test_support::MockWindowsCameraProbe;
        let adapter =
            NokhwaInputAdapter::new().with_windows_camera_probe(Box::new(MockWindowsCameraProbe {
                status: WindowsPermissionStatus::Unknown,
            }));
        let devices = adapter
            .enumerate()
            .expect("enumerate succeeds under Unknown status");
        // PR 3 scaffold: enumeration is empty across the board
        // because no backend is wired; the pin's contract is the
        // fall-through path is reached, not that devices are
        // surfaced.
        assert!(devices.is_empty());
    }

    /// `EV-INTEGRATION-MATRIX` T1: `enumerate` with `Denied`
    /// authorization returns an empty vec rather than enumerating
    /// devices the operator cannot use. Pinned because the
    /// operator-facing device-picker UI iterates the enumeration
    /// vec; surfacing devices under a denied-authorization probe
    /// would tantalize the operator with cameras they cannot bind.
    /// `Restricted` and `NotDetermined` route through the same
    /// empty-vec path.
    #[cfg(target_os = "macos")]
    #[test]
    fn enumerate_with_denied_authorization_returns_empty_vec() {
        use crate::macos::AuthorizationStatus;
        use crate::macos::test_support::MockAuthorizationProbe;
        for status in [
            AuthorizationStatus::Denied,
            AuthorizationStatus::Restricted,
            AuthorizationStatus::NotDetermined,
        ] {
            let adapter = NokhwaInputAdapter::new()
                .with_authorization_probe(Box::new(MockAuthorizationProbe { status }));
            let devices = adapter
                .enumerate()
                .expect("enumerate succeeds under non-Authorized status");
            assert!(
                devices.is_empty(),
                "enumerate must return empty vec under {status:?}; got {devices:?}",
            );
        }
    }
}
