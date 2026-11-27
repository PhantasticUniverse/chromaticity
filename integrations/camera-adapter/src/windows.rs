// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Windows Media Foundation camera permission flow per ADR-020 §5 +
//! §6 "Per-platform notes — Windows".
//!
//! Phase-4a Wave 1c-β Phase 2 PR 3 wires the Windows-specific
//! permission state machine that the cross-platform
//! [`crate::adapter::CameraAdapter`] consults during `bind` +
//! `enumerate`. The module mirrors the trait-based dependency-
//! injection seam pattern established by `crate::macos` (Phase 2
//! PR 2) so the adapter state-machine surface stays uniform across
//! platforms:
//!
//! * [`WindowsCameraProbe`] — synchronous query for the current
//!   [`WindowsPermissionStatus`]. Windows 10+ exposes camera-access
//!   policy through the Privacy Settings consent store rather than
//!   an in-process AVFoundation-style cache; the runtime read is
//!   typically a registry-backed query, but the trait surface
//!   abstracts the OS detail so tests inject mock probes through
//!   trait objects.
//! * [`WindowsCameraProbe::map_hresult`] — error-taxonomy entry
//!   point: maps an HRESULT-equivalent into one of the canonical
//!   fault-reason strings (`device-busy`, `permission-denied`,
//!   `backend-error`) per ADR-020 §2 + the brief direction for this
//!   PR. The MF-specific mapping rules:
//!   * `MF_E_DEVICE_INVALIDATED` (`0xC00D36B4u32`) →
//!     [`crate::frames::DEVICE_BUSY`]. The device is held by another
//!     application; the operator-facing UX surface shows the
//!     busy-device banner ("Camera is in use by <App Name>") rather
//!     than the permission-denied banner. This MF error fires both
//!     on initial open against a held device AND mid-stream when an
//!     external process pre-empts the capture handle.
//!   * `E_ACCESSDENIED` (`0x80070005u32`) →
//!     [`crate::frames::PERMISSION_DENIED`]. Privacy-Settings camera
//!     access has been disallowed for this app or globally; the
//!     operator-facing UX surface shows the permission-required
//!     banner with the Privacy-Settings deep-link.
//!   * Any other HRESULT failure → [`crate::frames::BACKEND_ERROR`].
//!     Generic catch-all for nokhwa-internal MF failures the adapter
//!     does not map specifically (e.g. `MF_E_NO_MORE_TYPES` on
//!     unsupported pixel formats, which are surfaced as
//!     [`crate::frames::FORMAT_NOT_SUPPORTED`] from the
//!     format-negotiation path rather than from the HRESULT
//!     mapper).
//!
//! ## No in-app permission prompt on Windows
//!
//! Unlike macOS — where `AVCaptureDevice.requestAccess(for: .video)`
//! triggers an OS-level system permission dialog the first time a
//! given bundle id requests camera access — Windows 10+ has **no
//! in-app prompt**. The Privacy Settings camera-allow-list is
//! managed exclusively through the Settings app
//! (`Settings → Privacy & Security → Camera`); a denied app cannot
//! re-trigger a system prompt by retrying the open call. The
//! adapter's recovery path on a [`WindowsPermissionStatus::Denied`]
//! probe result is therefore:
//!
//! 1. Transition to
//!    [`crate::adapter::AdapterHealth::Faulted`] with the
//!    [`crate::frames::PERMISSION_DENIED`] reason.
//! 2. Surface an operator-facing banner with a deep-link to
//!    `ms-settings:privacy-webcam` (the Settings app's per-app
//!    camera permission page) per ADR-020 §6 "Per-platform notes —
//!    Windows" line 156.
//! 3. Wait for the operator to grant access via the Settings app
//!    and re-bind from a fresh user gesture; the re-bind is
//!    NOT auto-retried — Windows Privacy Settings changes do not
//!    fire any OS-level notification the adapter can observe
//!    cheaply, and the operator-facing recovery flow per ADR-020 §6
//!    is a manual re-bind from the camera-input UI.
//!
//! ## Concrete impl deferred to a follow-up PR
//!
//! This PR ships the trait surface + a [`PlaceholderProbe`]
//! concrete impl that returns
//! [`WindowsPermissionStatus::Unknown`] (the conservative default
//! that drives the adapter into `Negotiating` until a real probe
//! transitions it). Tests inject mock probes through the trait
//! objects via [`crate::adapter::NokhwaInputAdapter::with_windows_camera_probe`].
//!
//! The real Media Foundation / COM binding (which requires
//! `unsafe` blocks for the `windows` crate's `IMFActivate` /
//! `IMFSourceReader` accesses, the consent-store registry read for
//! the per-app Privacy-Settings status, and HRESULT propagation
//! through nokhwa's open-call surface) lands in a follow-up
//! policy-scoped PR alongside the workspace `unsafe_code` floor
//! widening per `docs/43-development-standards.md` §12.6 (workspace
//! `unsafe_code = "forbid"` is the floor; loosening requires a
//! policy-scoped ADR amending §12.1 + the workspace `Cargo.toml`
//! in the same PR). This mirrors the
//! `integrations/camera-adapter/src/macos.rs` precedent — Phase 2
//! PR 2 shipped the trait surface only; the AVFoundation binding
//! lands in a follow-up policy-scoped PR. PR 3 (this PR)
//! consequently keeps **zero** `unsafe` blocks in the camera-
//! adapter crate; all workspace lints (`unsafe_code = "forbid"`,
//! `clippy::unwrap_used = "deny"`, `clippy::expect_used = "deny"`)
//! apply unrelaxed.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (camera arm, Phase-4a
//! Wave 1c-β).
//!
//! The `cfg(target_os = "windows")` gate lives at the
//! `pub mod windows;` declaration in `lib.rs` rather than as an
//! inner-attribute on this file, so the gate is single-sourced and
//! `clippy::duplicated_attributes` stays clean.

use crate::frames::{BACKEND_ERROR, DEVICE_BUSY, PERMISSION_DENIED};

/// Privacy-Settings camera-access status mapping per Windows 10+
/// consent-store semantics.
///
/// Mirrors the three observable states the per-app camera-access
/// query surfaces (the consent store maps a per-app `Allow`,
/// `Deny`, or absent entry; the absent entry is surfaced as
/// [`WindowsPermissionStatus::Unknown`] because Windows does not
/// distinguish "first-time-not-yet-prompted" from "system policy
/// has not yet evaluated"):
///
/// * [`WindowsPermissionStatus::Granted`] — Privacy Settings allows
///   camera access for this app. The adapter proceeds with the
///   nokhwa open call.
/// * [`WindowsPermissionStatus::Denied`] — Privacy Settings has
///   explicitly denied camera access for this app, OR the global
///   "Camera access for this device" toggle is off. The adapter
///   transitions to terminal `Faulted("permission-denied")`; the
///   operator must grant access via the Settings app's
///   `ms-settings:privacy-webcam` page before re-binding.
/// * [`WindowsPermissionStatus::Unknown`] — the consent-store
///   query could not resolve a definitive status (e.g. the
///   registry-backed read failed, the per-app entry is absent and
///   the global toggle is in an indeterminate state, or the
///   placeholder probe is in use because the real binding has not
///   landed yet). The adapter enters `Negotiating` and the open
///   call drives the eventual transition: `MF_E_DEVICE_INVALIDATED`
///   on a held device → `Faulted("device-busy")`; `E_ACCESSDENIED`
///   on a denied app → `Faulted("permission-denied")`; success →
///   `Bound`.
///
/// `#[non_exhaustive]` because future Windows versions may add
/// consent-store discriminators (e.g. an "expiring" status if
/// per-app camera grants ever gain TTL semantics).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum WindowsPermissionStatus {
    /// Privacy Settings allows camera access for this app.
    Granted,
    /// Privacy Settings has explicitly denied camera access (per-app
    /// or globally). Operator must use the Settings app to grant.
    Denied,
    /// Consent-store query could not resolve a definitive status;
    /// the adapter relies on the open-call HRESULT for the eventual
    /// state-machine transition.
    Unknown,
}

impl WindowsPermissionStatus {
    /// Whether the adapter can proceed with `bind` against a device
    /// without first surfacing a terminal-fault transition. Both
    /// [`WindowsPermissionStatus::Granted`] and
    /// [`WindowsPermissionStatus::Unknown`] return `true` because the
    /// open-call HRESULT is the authoritative arbiter when the
    /// consent-store query is indeterminate; only an explicit
    /// [`WindowsPermissionStatus::Denied`] short-circuits to
    /// `Faulted("permission-denied")`.
    #[must_use]
    pub fn permits_bind(self) -> bool {
        matches!(self, Self::Granted | Self::Unknown)
    }

    /// Whether the variant is a terminal denial (operator-declined
    /// or globally-disallowed) that should route directly to
    /// `AdapterHealth::Faulted("permission-denied")`. Pinned because
    /// routing `Unknown` through this path would silently break the
    /// open-call-as-arbiter contract when the consent-store query
    /// is indeterminate.
    #[must_use]
    pub fn is_terminal_denial(self) -> bool {
        matches!(self, Self::Denied)
    }
}

/// Synchronous Privacy-Settings probe + HRESULT-mapper. The DI seam
/// through which the adapter consults the current per-app camera
/// access status AND interprets nokhwa-surfaced HRESULTs from the
/// Media Foundation open-call path.
///
/// Tests inject [`MockWindowsCameraProbe`] (under `#[cfg(test)]`)
/// to drive the adapter state machine through every
/// [`WindowsPermissionStatus`] arm + the canonical HRESULT mapping
/// taxonomy without depending on a real Media Foundation runtime.
pub trait WindowsCameraProbe: Send + Sync {
    /// Returns the current Privacy-Settings camera-access status.
    ///
    /// Synchronous and immediate — this method is callable from any
    /// thread, including the Tauri main-thread-bound IPC handler.
    /// The OS-level call this wraps (the consent-store registry
    /// read) is itself synchronous and returns a cached value; no
    /// async contract applies.
    fn current(&self) -> WindowsPermissionStatus;

    /// Maps an HRESULT-equivalent failure code (as a `u32`,
    /// matching the `windows` crate's `HRESULT::0` raw layout) to
    /// one of the canonical fault-reason `pub const` strings in
    /// [`crate::frames`]. The default impl encodes the MF-specific
    /// taxonomy described in the module-level prose:
    ///
    /// * `MF_E_DEVICE_INVALIDATED` (`0xC00D36B4`) → [`DEVICE_BUSY`].
    /// * `E_ACCESSDENIED` (`0x80070005`) → [`PERMISSION_DENIED`].
    /// * Any other value → [`BACKEND_ERROR`].
    ///
    /// Implementations override this only when a custom probe needs
    /// finer-grained mapping (e.g. routing
    /// `MF_E_NOT_AVAILABLE` to a specific reason once the format-
    /// negotiation path is wired); the default is the canonical
    /// MF mapping the production probe uses.
    #[must_use]
    fn map_hresult(&self, hresult: u32) -> &'static str {
        map_hresult_default(hresult)
    }
}

/// Canonical HRESULT → fault-reason mapping per ADR-020 §2 +
/// the brief direction for Wave 1c-β Phase 2 PR 3.
///
/// Free function exported as the default impl of
/// [`WindowsCameraProbe::map_hresult`]; tests can call it directly
/// to lock the mapping taxonomy without instantiating a probe.
#[must_use]
pub fn map_hresult_default(hresult: u32) -> &'static str {
    // Constants inlined from <mferror.h> + <winerror.h> rather than
    // imported via the `windows` crate; the trait surface in PR 3
    // is binding-free per the module-level deferral prose. The
    // policy-scoped follow-up PR will source these from the
    // `windows::Win32::Foundation::HRESULT` constants directly.
    const MF_E_DEVICE_INVALIDATED: u32 = 0xC00D_36B4;
    const E_ACCESSDENIED: u32 = 0x8007_0005;

    match hresult {
        MF_E_DEVICE_INVALIDATED => DEVICE_BUSY,
        E_ACCESSDENIED => PERMISSION_DENIED,
        _ => BACKEND_ERROR,
    }
}

/// Placeholder concrete impl of [`WindowsCameraProbe`].
///
/// Returns conservative defaults that drive the adapter into a
/// safe state without depending on the OS Media Foundation /
/// consent-store subsystem:
///
/// * [`WindowsCameraProbe::current`] returns
///   [`WindowsPermissionStatus::Unknown`].
/// * [`WindowsCameraProbe::map_hresult`] uses the default impl
///   ([`map_hresult_default`]).
///
/// The real consent-store registry read + Media Foundation HRESULT
/// surfaces land in a follow-up policy-scoped PR that widens the
/// workspace `unsafe_code` floor per `docs/43` §12.6. Until that
/// PR lands the placeholder lets the adapter compile on Windows
/// without `unsafe` blocks while preserving the trait-based DI
/// surface tests exercise via [`MockWindowsCameraProbe`].
#[derive(Clone, Copy, Debug, Default)]
pub struct PlaceholderProbe;

impl WindowsCameraProbe for PlaceholderProbe {
    fn current(&self) -> WindowsPermissionStatus {
        // The OS-level binding to the consent-store registry read
        // is deferred to the policy-scoped follow-up PR per the
        // module-level prose above; the placeholder returns
        // Unknown so the adapter exercises the
        // open-call-as-arbiter path that the concrete impl will
        // drive via the production HRESULT surface.
        WindowsPermissionStatus::Unknown
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
pub(crate) mod test_support {
    //! Mock probes for the adapter-side state-machine T1 pins.
    //!
    //! Lives under `#[cfg(test)]` rather than a `dev-dependencies`
    //! block so the trait DI surface stays exclusively in the public
    //! crate API; the mocks are compiled only when running the test
    //! suite.

    use super::{WindowsCameraProbe, WindowsPermissionStatus};

    /// Mock Windows camera probe returning a fixed status. Uses the
    /// default `map_hresult` impl so adapter-side T1 pins exercise
    /// the canonical taxonomy without a probe-specific override.
    pub(crate) struct MockWindowsCameraProbe {
        pub(crate) status: WindowsPermissionStatus,
    }

    impl WindowsCameraProbe for MockWindowsCameraProbe {
        fn current(&self) -> WindowsPermissionStatus {
            self.status
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod windows_pins {
    //! Evidence: `EV-INTEGRATION-MATRIX` (T1) — Windows
    //! permission-status taxonomy + HRESULT-mapping + DI-trait
    //! surface invariants for the camera adapter Phase 2 PR 3
    //! Media Foundation flow.
    //!
    //! Per `docs/43` §15.5 T1: each test pins a specific ADR-020 §5
    //! / §6 contract invariant against the implementation. The
    //! OS-level Media Foundation + consent-store calls the concrete
    //! impl wraps are deferred to a follow-up policy-scoped PR (see
    //! module-level prose); these pins lock the trait surface,
    //! status taxonomy, HRESULT-mapping vocabulary, and placeholder
    //! conservative-defaults posture in the meantime.

    use super::test_support::MockWindowsCameraProbe;
    use super::{
        PlaceholderProbe, WindowsCameraProbe, WindowsPermissionStatus, map_hresult_default,
    };
    use crate::frames::{BACKEND_ERROR, DEVICE_BUSY, PERMISSION_DENIED};

    /// `EV-INTEGRATION-MATRIX` T1: the [`WindowsPermissionStatus`]
    /// enum carries exactly the three Privacy-Settings consent-store
    /// discriminators per ADR-020 §6 + the brief direction. Pinned
    /// via an exhaustive match — adding a fourth variant fails to
    /// compile until the test acknowledges it.
    #[test]
    fn windows_permission_status_returns_one_of_three_variants() {
        for status in [
            WindowsPermissionStatus::Granted,
            WindowsPermissionStatus::Denied,
            WindowsPermissionStatus::Unknown,
        ] {
            // Exhaustive match locks the variant set.
            let label = match status {
                WindowsPermissionStatus::Granted => "granted",
                WindowsPermissionStatus::Denied => "denied",
                WindowsPermissionStatus::Unknown => "unknown",
            };
            assert!(!label.is_empty());
        }
    }

    /// `EV-INTEGRATION-MATRIX` T1: `Granted` and `Unknown` permit
    /// bind (the open-call HRESULT is the authoritative arbiter
    /// when the consent-store query is indeterminate); only an
    /// explicit `Denied` short-circuits.
    #[test]
    fn windows_permission_status_permits_bind_only_when_not_denied() {
        assert!(WindowsPermissionStatus::Granted.permits_bind());
        assert!(WindowsPermissionStatus::Unknown.permits_bind());
        assert!(!WindowsPermissionStatus::Denied.permits_bind());
    }

    /// `EV-INTEGRATION-MATRIX` T1: only `Denied` is a terminal
    /// denial; `Unknown` is NOT — the adapter relies on the open
    /// call's HRESULT to drive the eventual transition.
    #[test]
    fn only_denied_is_terminal() {
        assert!(WindowsPermissionStatus::Denied.is_terminal_denial());
        assert!(!WindowsPermissionStatus::Granted.is_terminal_denial());
        assert!(!WindowsPermissionStatus::Unknown.is_terminal_denial());
    }

    /// `EV-INTEGRATION-MATRIX` T1: `MF_E_DEVICE_INVALIDATED`
    /// (`0xC00D36B4`) maps to [`DEVICE_BUSY`] — distinct from
    /// `permission-denied` per ADR-020 §2 canonical reason
    /// vocabulary + the brief direction. Pinned because routing
    /// the MF-busy error through `permission-denied` would silently
    /// break the operator-facing UX surface (busy-device banner vs
    /// permission banner).
    #[test]
    fn mf_e_device_invalidated_maps_to_device_busy() {
        assert_eq!(map_hresult_default(0xC00D_36B4), DEVICE_BUSY);
    }

    /// `EV-INTEGRATION-MATRIX` T1: `E_ACCESSDENIED` (`0x80070005`)
    /// maps to [`PERMISSION_DENIED`]. Pinned so the Privacy-Settings
    /// denial path routes to the operator-facing permission-required
    /// banner with the `ms-settings:privacy-webcam` deep-link.
    #[test]
    fn e_access_denied_maps_to_permission_denied() {
        assert_eq!(map_hresult_default(0x8007_0005), PERMISSION_DENIED);
    }

    /// `EV-INTEGRATION-MATRIX` T1: any unmapped HRESULT routes to
    /// [`BACKEND_ERROR`] as a generic catch-all. Pinned because the
    /// adapter MUST NOT silently swallow MF errors — they all need
    /// to surface to the operator UX, and the `backend-error`
    /// reason is the canonical "diagnostic-only, no operator
    /// action" path per ADR-020 §2.
    #[test]
    fn generic_hresult_maps_to_backend_error() {
        // A few representative unmapped HRESULTs:
        // MF_E_NO_MORE_TYPES (0xC00D36B9), MF_E_NOT_AVAILABLE
        // (0xC00D3704), MF_E_INVALIDREQUEST (0xC00D36B2), and a
        // wholly-out-of-band placeholder.
        for hresult in [0xC00D_36B9_u32, 0xC00D_3704, 0xC00D_36B2, 0xDEAD_BEEF] {
            assert_eq!(
                map_hresult_default(hresult),
                BACKEND_ERROR,
                "unmapped HRESULT {hresult:#010x} must route to backend-error",
            );
        }
    }

    /// `EV-INTEGRATION-MATRIX` T1: the [`PlaceholderProbe`] returns
    /// the ADR-020 §6 conservative defaults — `Unknown`
    /// permission-status (which permits bind so the open-call
    /// HRESULT is the arbiter), and the canonical default
    /// HRESULT-mapping. Pinned because the placeholder is the
    /// production-shipping concrete impl until the follow-up
    /// policy-scoped PR widens the `unsafe_code` floor.
    #[test]
    fn placeholder_probe_returns_conservative_defaults() {
        let probe = PlaceholderProbe;
        assert_eq!(probe.current(), WindowsPermissionStatus::Unknown);
        // Permission status `Unknown` permits bind because the
        // open-call HRESULT is the arbiter when the consent-store
        // query is indeterminate.
        assert!(probe.current().permits_bind());
        // The default `map_hresult` impl exposes the canonical
        // taxonomy.
        assert_eq!(probe.map_hresult(0xC00D_36B4), DEVICE_BUSY);
        assert_eq!(probe.map_hresult(0x8007_0005), PERMISSION_DENIED);
        assert_eq!(probe.map_hresult(0xDEAD_BEEF), BACKEND_ERROR);
    }

    /// `EV-INTEGRATION-MATRIX` T1: the [`MockWindowsCameraProbe`]
    /// covers every status variant under DI; the adapter T1 pins
    /// (in `adapter.rs`) compose this mock with the bind/enumerate
    /// state machine to drive the AdapterHealth transition arms.
    #[test]
    fn mock_windows_probe_returns_injected_status() {
        for status in [
            WindowsPermissionStatus::Granted,
            WindowsPermissionStatus::Denied,
            WindowsPermissionStatus::Unknown,
        ] {
            let probe = MockWindowsCameraProbe { status };
            assert_eq!(probe.current(), status);
            // The mock inherits the canonical default HRESULT
            // mapping; pinning that here ensures adapter-side
            // pins driving probe injection see the expected
            // taxonomy without a probe-specific override.
            assert_eq!(probe.map_hresult(0xC00D_36B4), DEVICE_BUSY);
        }
    }
}
