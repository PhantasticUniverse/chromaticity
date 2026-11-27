// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Cross-platform camera-input adapter per ADR-020.
//!
//! Surface: [`CameraAdapter`] trait + [`NokhwaInputAdapter`] impl,
//! mirroring the established `integrations/osc-adapter` /
//! `integrations/midi-adapter` shape. Cross-platform via the
//! `nokhwa = "0.10"` abstraction (CoreMediaIO on macOS,
//! Media Foundation on Windows). Texture binding namespace:
//! `chroma_CameraInput[N]` (N = 0..=3 per ADR-020 §Decision;
//! mirrors the Shadertoy iChannel range). The render-side
//! marshalling step lives at `engine/render/src/camera_adapter.rs`
//! per ADR-020 §7; the engine crate consumes the plain-data
//! [`CameraFrame`] payloads this crate publishes without taking a
//! reverse dependency.
//!
//! ## Phase-4a Wave 1c-β camera-arm closure
//!
//! Wave 1c-β closed 2026-05-07 (cluster merge `#406`). The
//! camera-arm widening of `EV-INTEGRATION-MATRIX` landed across the
//! following sequenced PRs:
//!
//! * PR `#395` shipped the surface-only adapter scaffold
//!   ([`CameraAdapter`] trait + [`NokhwaInputAdapter`] +
//!   [`AdapterError`] + [`CameraFrame`] + [`PixelFormat`] +
//!   [`defaults`]).
//! * PR `#397` shipped the macOS CoreMediaIO permission-flow
//!   surface ([`macos`]) — `NSCameraUsageDescription` Info.plist
//!   override + `AVCaptureDevice.requestAccess` per ADR-020 §5 /
//!   `docs/54` Phase-2 lesson 2.
//! * PR `#399` shipped the Windows Media Foundation
//!   permission-flow surface ([`windows`]) — Privacy Settings
//!   probe + canonical HRESULT → fault-reason mapping per
//!   ADR-020 §5 + §6 "Per-platform notes — Windows".
//! * PR `#401` shipped the synthetic T2 corpus (row 78 / row 40 /
//!   row 93 narrowing precedent).
//! * PR `#403` wired `engine/render/src/camera_adapter.rs` —
//!   marshalling [`CameraFrame`] payloads into the wgpu texture
//!   binding under the `chroma_CameraInput[N]` namespace.
//!
//! Synthetic-narrowing posture per the row 78 / row 40 / row 93
//! precedent: per-platform concrete nokhwa `input-avfoundation` /
//! `input-msmf` bindings + real hardware validation are documented
//! follow-ups carried forward per
//! `docs/EV-GATE-MODEL-AUDIT-checklist.md` Phase-4a Wave 1c-β
//! closure section. [`NokhwaInputAdapter::bind`] returns
//! [`AdapterError::DeviceNotFound`] and
//! [`NokhwaInputAdapter::enumerate`] returns an empty vec until the
//! deferred concrete-backend wave lands.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (camera arm, Phase-4a
//! Wave 1c-β).
//!
//! Governing ADRs:
//!
//! - ADR-005 — integration adapter boundary
//!   (`docs/adr/ADR-005-integration-adapter-boundary.md`).
//! - ADR-009 — activation and entitlements
//!   (`docs/adr/ADR-009-activation-and-entitlements.md`; `IO_CAMERA`
//!   stays MVP-free per line 50).
//! - ADR-013 — networking defaults and security
//!   (`docs/adr/ADR-013-networking-defaults-and-security.md`).
//! - ADR-020 — camera adapter
//!   (`docs/adr/ADR-020-camera-adapter.md`).

pub mod adapter;
pub mod defaults;
pub mod frames;

/// macOS-specific permission-flow surface per ADR-020 §5. Compiles
/// to a no-op (empty module) on non-macOS targets; the gating lives
/// at the module level rather than at the crate root so the cross-
/// platform [`adapter`] surface stays uniform.
#[cfg(target_os = "macos")]
pub mod macos;

/// Windows Media Foundation permission-flow surface per ADR-020 §5
/// + §6 "Per-platform notes — Windows". Compiles to a no-op
/// (empty module) on non-Windows targets; the gating lives at the
/// module level so the cross-platform [`adapter`] surface stays
/// uniform. PR `#399` shipped the trait surface +
/// [`windows::PlaceholderProbe`] concrete impl + canonical HRESULT
/// → fault-reason mapping; the real Media Foundation / consent-
/// store binding lands in a follow-up policy-scoped PR per the
/// `macos.rs` precedent.
#[cfg(target_os = "windows")]
pub mod windows;

pub use adapter::{
    AdapterError, AdapterHealth, CameraAdapter, CameraConfig, CameraDeviceKind, DeviceSummary,
    NokhwaInputAdapter,
};
pub use defaults::{
    DEFAULT_FRAME_RATE, DEFAULT_PIXEL_FORMAT_PREFERENCE, DEFAULT_QUEUE_DEPTH,
    DEFAULT_RECONNECT_ATTEMPTS, DEFAULT_RECONNECT_INTERVAL_MS, DEFAULT_RESOLUTION,
};
pub use frames::{
    BACKEND_ERROR, CameraFrame, DEVICE_BUSY, DEVICE_NOT_FOUND, FORMAT_NOT_SUPPORTED,
    PERMISSION_DENIED, PixelFormat, RECONNECT_EXHAUSTED,
};
