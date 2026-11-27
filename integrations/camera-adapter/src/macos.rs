// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! macOS camera permission flow per ADR-020 §5.
//!
//! Phase-4a Wave 1c-β Phase 2 PR 2 wires the macOS-specific permission
//! state machine that the cross-platform [`crate::adapter::CameraAdapter`]
//! consults during `bind` + `enumerate`. The module declares three
//! dependency-injection seams that the adapter consumes through trait
//! objects:
//!
//! * [`AuthorizationProbe`] — synchronous query for the current
//!   [`AuthorizationStatus`] (`AVCaptureDevice.authorizationStatus(for: .video)`).
//! * [`InUsePrecheck`] — synchronous query for whether another macOS
//!   process holds the device (`AVCaptureDevice.isInUseByAnotherApplication`),
//!   per `docs/54-wave-1c-beta-research-lessons.md` Phase-2 lesson 10.
//!   The precheck distinguishes [`crate::frames::DEVICE_BUSY`] from
//!   [`crate::frames::PERMISSION_DENIED`] so the operator UX surfaces
//!   the right banner ("Camera is in use by <App Name>" vs. "Camera
//!   permission required").
//! * [`RequestAccess`] — fire-and-forget kick of the OS-level access
//!   prompt (`AVCaptureDevice.requestAccess(for: .video)`). Returns
//!   immediately per the **async permission-flow contract** in ADR-020
//!   §5 + `docs/54` Phase-2 lesson 2 — the calling thread MUST NOT
//!   block on the OS prompt; the OS completion handler transitions
//!   the adapter health from [`crate::adapter::AdapterHealth::Negotiating`]
//!   to either [`crate::adapter::AdapterHealth::Bound`] (on grant) or
//!   [`crate::adapter::AdapterHealth::Faulted`]
//!   ([`crate::frames::PERMISSION_DENIED`]) (on deny). VLC's
//!   `avcapture.m:420` blocks on the prompt via
//!   `dispatch_semaphore_wait(sema, DISPATCH_TIME_FOREVER)`; that
//!   pattern works for VLC's threaded demux model but would deadlock
//!   the Tauri main-thread-bound IPC handler chromaticity uses.
//!
//! ## Concrete impl deferred to a follow-up PR
//!
//! This PR ships the trait surface + a [`PlaceholderProbe`] concrete
//! impl that returns [`AuthorizationStatus::NotDetermined`] +
//! `false` (not in-use) + a no-op `RequestAccess`. Tests inject mock
//! probes through the trait objects.
//!
//! The real `AVCaptureDevice` Objective-C binding (which requires
//! `unsafe` blocks for `objc2::msg_send!` / `extern_class!` macro
//! expansion) lands in a follow-up policy-scoped PR alongside the
//! workspace-floor widening per `docs/43-development-standards.md`
//! §12.6 (workspace `unsafe_code = "forbid"` is the floor; loosening
//! requires a policy-scoped ADR amending §12.1 + the workspace
//! `Cargo.toml` in the same PR). This mirrors the
//! `integrations/syphon-adapter/` precedent — Syphon PR 1 shipped the
//! type surface only, and PR 2 lands the binding code under the
//! policy-scoped lint widening.
//!
//! Phase 2 PR 2 (this PR) consequently keeps **zero** `unsafe` blocks
//! in the camera-adapter crate; all workspace lints
//! (`unsafe_code = "forbid"`, `clippy::unwrap_used = "deny"`,
//! `clippy::expect_used = "deny"`) apply unrelaxed.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (camera arm, Phase-4a
//! Wave 1c-β).
//!
//! The `cfg(target_os = "macos")` gate lives at the
//! `pub mod macos;` declaration in `lib.rs` rather than as an
//! inner-attribute on this file, so the gate is single-sourced and
//! `clippy::duplicated_attributes` stays clean.

/// AVAuthorizationStatus mapping per `AVCaptureDevice.authorizationStatus(for: .video)`.
///
/// Mirrors the four discriminator values the macOS AVFoundation
/// framework returns from
/// `+[AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo]`:
///
/// * [`AuthorizationStatus::NotDetermined`] — operator has not yet
///   responded to a permission prompt for this bundle id. The OS-level
///   request flow has not been kicked off, OR a prompt is in flight
///   and awaiting a completion-handler invocation.
/// * [`AuthorizationStatus::Authorized`] — operator granted access.
///   The TCC.db cache holds a positive grant for this bundle id.
/// * [`AuthorizationStatus::Denied`] — operator declined the prompt
///   OR revoked an earlier grant via System Settings → Privacy &
///   Security → Camera. Per macOS TCC semantics the grant is
///   process-bound; re-firing the prompt within a process lifetime
///   does not re-trigger the OS dialog.
/// * [`AuthorizationStatus::Restricted`] — Parental Controls or MDM
///   policy disallows camera access for this user. The operator
///   cannot resolve this in System Settings without administrator
///   intervention.
///
/// `#[non_exhaustive]` because future macOS versions may add
/// authorization-status values (e.g. an "expired" discriminator if
/// TCC.db ever surfaces grant TTL); widening reuses the variant
/// taxonomy established here.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum AuthorizationStatus {
    /// Operator has not yet responded; OS prompt has not fired or is
    /// in flight.
    NotDetermined,
    /// Operator granted access.
    Authorized,
    /// Operator declined or revoked.
    Denied,
    /// Parental Controls / MDM policy disallows access.
    Restricted,
}

impl AuthorizationStatus {
    /// Whether the adapter can proceed with `bind` against a device
    /// without first kicking the OS-level access-request flow.
    ///
    /// Only [`AuthorizationStatus::Authorized`] short-circuits the
    /// negotiation path; every other variant requires either an
    /// async OS prompt round-trip ([`AuthorizationStatus::NotDetermined`])
    /// or a terminal `Faulted("permission-denied")` transition
    /// ([`AuthorizationStatus::Denied`] / [`AuthorizationStatus::Restricted`]).
    #[must_use]
    pub fn permits_bind(self) -> bool {
        matches!(self, Self::Authorized)
    }

    /// Whether the variant is a terminal denial (operator-declined or
    /// MDM-restricted) that should route directly to
    /// `AdapterHealth::Faulted("permission-denied")` without firing
    /// an OS-level access-request prompt. `NotDetermined` is NOT a
    /// terminal state — the adapter kicks the async request and waits
    /// for the completion handler.
    #[must_use]
    pub fn is_terminal_denial(self) -> bool {
        matches!(self, Self::Denied | Self::Restricted)
    }
}

/// Synchronous authorization-status probe. The DI seam through which
/// the adapter consults the current
/// `AVCaptureDevice.authorizationStatus(for: .video)` value.
///
/// Tests inject [`MockAuthorizationProbe`] to drive the adapter
/// state machine through every [`AuthorizationStatus`] arm without
/// depending on the OS TCC subsystem.
pub trait AuthorizationProbe: Send + Sync {
    /// Returns the current AVAuthorizationStatus mapped to
    /// [`AuthorizationStatus`].
    ///
    /// Synchronous and immediate — this method is callable from any
    /// thread, including the Tauri main-thread-bound IPC handler.
    /// The OS-level call this wraps
    /// (`+[AVCaptureDevice authorizationStatusForMediaType:]`) is
    /// itself synchronous and returns a cached value from TCC.db; the
    /// async contract per ADR-020 §5 applies only to
    /// [`RequestAccess::request`], not to this probe.
    fn current(&self) -> AuthorizationStatus;
}

/// Synchronous in-use precheck. Wraps
/// `AVCaptureDevice.isInUseByAnotherApplication` per
/// `docs/54-wave-1c-beta-research-lessons.md` Phase-2 lesson 10.
///
/// The precheck distinguishes [`crate::frames::DEVICE_BUSY`] from
/// [`crate::frames::PERMISSION_DENIED`]. Without it, the open-call
/// failure path conflates "another app holds the camera" with
/// "permission denied" — VLC's `avcapture.m` historically suffered
/// from that conflation; the canonical reason vocabulary in ADR-020
/// §2 explicitly separates the two so the operator UI surfaces the
/// busy-device banner ("Camera is in use by <App Name>") rather
/// than the permission-denied banner.
pub trait InUsePrecheck: Send + Sync {
    /// Returns `true` if another macOS application currently holds
    /// the device identified by `device_id`.
    ///
    /// Synchronous and immediate. Implementations MUST NOT block —
    /// the underlying `isInUseByAnotherApplication` property read is
    /// a cached state lookup, not a blocking framework call.
    fn is_in_use(&self, device_id: &str) -> bool;
}

/// Async access-request kicker. Wraps
/// `AVCaptureDevice.requestAccess(for: .video) completionHandler:`
/// per ADR-020 §5 + `docs/54-wave-1c-beta-research-lessons.md`
/// Phase-2 lesson 2.
///
/// The contract is **fire-and-forget**: [`RequestAccess::request`]
/// returns immediately, the OS displays the system permission prompt
/// asynchronously, and the OS completion handler invokes the supplied
/// callback when the operator responds (grant or deny).
///
/// VLC's `avcapture.m:420` blocks on the prompt via
/// `dispatch_semaphore_wait(sema, DISPATCH_TIME_FOREVER)`. That
/// pattern works for VLC's threaded demux model — a dedicated capture
/// thread can block without freezing the UI — but the same code in
/// chromaticity's Tauri main-thread-bound IPC handler would deadlock
/// the entire frontend. Implementations of this trait MUST NOT block
/// the calling thread; the implementation owns the dispatch of the
/// completion-handler callback to whatever async runtime the adapter
/// uses (typically a `std::thread::spawn` for the placeholder, a
/// Tokio runtime task in the Tauri shell concrete impl).
pub trait RequestAccess: Send + Sync {
    /// Kicks the OS-level access-request prompt. Returns immediately;
    /// the supplied `on_complete` closure fires from the OS
    /// completion-handler thread when the operator responds.
    ///
    /// `on_complete` receives `true` on grant, `false` on deny.
    fn request(&self, on_complete: Box<dyn FnOnce(bool) + Send + 'static>);
}

/// Placeholder concrete impl of all three traits.
///
/// Returns conservative defaults that drive the adapter into a
/// safe state without depending on the OS TCC subsystem:
///
/// * [`AuthorizationProbe::current`] returns
///   [`AuthorizationStatus::NotDetermined`].
/// * [`InUsePrecheck::is_in_use`] returns `false`.
/// * [`RequestAccess::request`] is a no-op (drops the closure
///   without invoking it; the adapter stays in
///   [`crate::adapter::AdapterHealth::Negotiating`] until a real
///   probe replaces this placeholder).
///
/// The real `AVCaptureDevice` binding lands in a follow-up
/// policy-scoped PR that widens the workspace `unsafe_code` floor
/// per `docs/43` §12.6. Until that PR lands the placeholder lets the
/// adapter compile on macOS without `unsafe` blocks while preserving
/// the trait-based DI surface tests exercise via [`MockAuthorizationProbe`].
#[derive(Clone, Copy, Debug, Default)]
pub struct PlaceholderProbe;

impl AuthorizationProbe for PlaceholderProbe {
    fn current(&self) -> AuthorizationStatus {
        // The OS-level binding to AVCaptureDevice is deferred to the
        // policy-scoped follow-up PR per the module-level prose
        // above; the placeholder returns NotDetermined so the
        // adapter exercises the negotiating-then-await-completion
        // path that the concrete impl will drive.
        AuthorizationStatus::NotDetermined
    }
}

impl InUsePrecheck for PlaceholderProbe {
    fn is_in_use(&self, _device_id: &str) -> bool {
        // Placeholder: no concurrent-app holds detection until the
        // real `AVCaptureDevice.isInUseByAnotherApplication` binding
        // lands in the follow-up policy-scoped PR.
        false
    }
}

impl RequestAccess for PlaceholderProbe {
    fn request(&self, _on_complete: Box<dyn FnOnce(bool) + Send + 'static>) {
        // No-op: the placeholder does not invoke the completion
        // handler. The adapter remains in `Negotiating` until a real
        // request-access impl replaces this placeholder. Tests use
        // `MockRequestAccess` to drive the completion-handler
        // synchronously.
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

    use std::sync::Mutex;

    use super::{AuthorizationProbe, AuthorizationStatus, InUsePrecheck, RequestAccess};

    /// Boxed completion-handler closure shape captured by
    /// [`MockRequestAccess`]. Mirrors the production
    /// [`super::RequestAccess::request`] parameter type; aliased
    /// so the `Mutex<Option<...>>` field stays under the
    /// `clippy::type_complexity` threshold.
    type PendingCompletion = Box<dyn FnOnce(bool) + Send + 'static>;

    /// Mock authorization probe returning a fixed status.
    pub(crate) struct MockAuthorizationProbe {
        pub(crate) status: AuthorizationStatus,
    }

    impl AuthorizationProbe for MockAuthorizationProbe {
        fn current(&self) -> AuthorizationStatus {
            self.status
        }
    }

    /// Mock in-use precheck returning a fixed boolean.
    pub(crate) struct MockInUsePrecheck {
        pub(crate) in_use: bool,
    }

    impl InUsePrecheck for MockInUsePrecheck {
        fn is_in_use(&self, _device_id: &str) -> bool {
            self.in_use
        }
    }

    /// Mock request-access that synchronously invokes the completion
    /// handler with a fixed grant value when [`Self::fire`] is called.
    pub(crate) struct MockRequestAccess {
        pub(crate) grant: bool,
        pending: Mutex<Option<PendingCompletion>>,
    }

    impl MockRequestAccess {
        pub(crate) fn new(grant: bool) -> Self {
            Self {
                grant,
                pending: Mutex::new(None),
            }
        }

        /// Synchronously invokes the captured completion handler.
        /// Tests use this to drive the async completion deterministically.
        pub(crate) fn fire(&self) {
            let cb = self.pending.lock().unwrap().take();
            if let Some(cb) = cb {
                cb(self.grant);
            }
        }
    }

    impl RequestAccess for MockRequestAccess {
        fn request(&self, on_complete: Box<dyn FnOnce(bool) + Send + 'static>) {
            // Fire-and-forget shape mirroring the production contract:
            // the caller does not block; tests drive the completion
            // handler explicitly via `fire()`.
            *self.pending.lock().unwrap() = Some(on_complete);
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod macos_pins {
    //! Evidence: `EV-INTEGRATION-MATRIX` (T1) — macOS authorization
    //! status taxonomy + DI-trait surface invariants for the camera
    //! adapter Phase 2 PR 2 permission flow.
    //!
    //! Per `docs/43` §15.5 T1: each test pins a specific ADR-020 §5
    //! contract invariant against the implementation. The OS-level
    //! AVFoundation calls the concrete impl wraps are deferred to a
    //! follow-up policy-scoped PR (see module-level prose); these
    //! pins lock the trait surface, status taxonomy, and placeholder
    //! conservative-defaults posture in the meantime.

    use super::test_support::{MockAuthorizationProbe, MockInUsePrecheck, MockRequestAccess};
    use super::{
        AuthorizationProbe, AuthorizationStatus, InUsePrecheck, PlaceholderProbe, RequestAccess,
    };

    /// `EV-INTEGRATION-MATRIX` T1: the [`AuthorizationStatus`] enum
    /// carries exactly the four AVAuthorizationStatus discriminators
    /// per ADR-020 §5. Pinned via an exhaustive match — adding a
    /// fifth variant fails to compile until the test acknowledges it.
    #[test]
    fn authorization_status_query_returns_one_of_four_variants() {
        for status in [
            AuthorizationStatus::NotDetermined,
            AuthorizationStatus::Authorized,
            AuthorizationStatus::Denied,
            AuthorizationStatus::Restricted,
        ] {
            // Exhaustive match locks the variant set.
            let label = match status {
                AuthorizationStatus::NotDetermined => "not-determined",
                AuthorizationStatus::Authorized => "authorized",
                AuthorizationStatus::Denied => "denied",
                AuthorizationStatus::Restricted => "restricted",
            };
            assert!(!label.is_empty());
        }
    }

    /// `EV-INTEGRATION-MATRIX` T1: only `Authorized` short-circuits
    /// the bind path. Pinned because the adapter state machine
    /// switches on this predicate to decide between immediate-bind
    /// and negotiate-via-async-request.
    #[test]
    fn authorization_status_permits_bind_only_when_authorized() {
        assert!(AuthorizationStatus::Authorized.permits_bind());
        assert!(!AuthorizationStatus::NotDetermined.permits_bind());
        assert!(!AuthorizationStatus::Denied.permits_bind());
        assert!(!AuthorizationStatus::Restricted.permits_bind());
    }

    /// `EV-INTEGRATION-MATRIX` T1: `Denied` and `Restricted` route
    /// directly to terminal `Faulted("permission-denied")` without
    /// firing an OS-level prompt; `NotDetermined` does NOT — it
    /// triggers the async request flow per ADR-020 §5.
    #[test]
    fn request_access_with_already_denied_returns_immediately() {
        assert!(AuthorizationStatus::Denied.is_terminal_denial());
        assert!(AuthorizationStatus::Restricted.is_terminal_denial());
        assert!(!AuthorizationStatus::NotDetermined.is_terminal_denial());
        assert!(!AuthorizationStatus::Authorized.is_terminal_denial());
    }

    /// `EV-INTEGRATION-MATRIX` T1: the [`PlaceholderProbe`] returns
    /// the ADR-020 §5 conservative defaults — `NotDetermined`
    /// authorization, no-app-holds-it precheck, no-op request. Pinned
    /// because the placeholder is the production-shipping concrete
    /// impl until the follow-up policy-scoped PR widens the
    /// `unsafe_code` floor.
    #[test]
    fn placeholder_probe_returns_conservative_defaults() {
        let probe = PlaceholderProbe;
        assert_eq!(probe.current(), AuthorizationStatus::NotDetermined);
        assert!(!probe.is_in_use("any-device-id"));
        // request() is no-op; we only verify it accepts the
        // completion-handler closure without panicking.
        probe.request(Box::new(|_granted| {}));
    }

    /// `EV-INTEGRATION-MATRIX` T1: the [`InUsePrecheck`] surface is
    /// `Send + Sync` and returns a plain `bool`. Pinned because the
    /// adapter consults the trait through a `dyn InUsePrecheck`
    /// trait-object stored in the adapter struct; the
    /// `Send + Sync` requirement is a load-bearing posture for
    /// cross-thread sharing once the per-platform capture thread
    /// lands.
    #[test]
    fn device_in_use_precheck_returns_bool() {
        let precheck = MockInUsePrecheck { in_use: true };
        assert!(precheck.is_in_use("device-x"));
        let precheck = MockInUsePrecheck { in_use: false };
        assert!(!precheck.is_in_use("device-y"));
    }

    /// `EV-INTEGRATION-MATRIX` T1: the [`AuthorizationProbe`] mock
    /// covers every status variant under DI; the adapter T1 pins
    /// (in `adapter.rs`) compose this mock with the bind/enumerate
    /// state machine to drive the four AdapterHealth transition
    /// arms.
    #[test]
    fn mock_authorization_probe_returns_injected_status() {
        for status in [
            AuthorizationStatus::NotDetermined,
            AuthorizationStatus::Authorized,
            AuthorizationStatus::Denied,
            AuthorizationStatus::Restricted,
        ] {
            let probe = MockAuthorizationProbe { status };
            assert_eq!(probe.current(), status);
        }
    }

    /// `EV-INTEGRATION-MATRIX` T1: the [`MockRequestAccess`]
    /// fire-and-forget shape — `request()` returns immediately
    /// without blocking, and the completion handler fires
    /// deterministically when the test calls `fire()`. Pinned
    /// because the production concrete impl (deferred to the
    /// follow-up PR) MUST honour the same fire-and-forget contract
    /// per ADR-020 §5; the mock locks the trait shape so the
    /// adapter state-machine pins exercise the negotiating-to-bound
    /// transition deterministically.
    #[test]
    fn request_access_fire_and_forget_shape() {
        let access = MockRequestAccess::new(true);
        // request() returns immediately without blocking.
        let captured: std::sync::Arc<std::sync::Mutex<Option<bool>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));
        let captured_clone = captured.clone();
        access.request(Box::new(move |granted| {
            *captured_clone.lock().unwrap() = Some(granted);
        }));
        // Completion handler has not fired yet.
        assert_eq!(*captured.lock().unwrap(), None);
        // Test drives the completion synchronously.
        access.fire();
        assert_eq!(*captured.lock().unwrap(), Some(true));
    }
}
