// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Cross-platform camera-input adapter per ADR-020.
//!
//! Surface: [`CameraAdapter`] trait + [`NokhwaInputAdapter`] impl,
//! mirroring the established `integrations/osc-adapter` /
//! `integrations/midi-adapter` shape. Cross-platform via the
//! `nokhwa = "0.10"` abstraction (CoreMediaIO on macOS,
//! Media Foundation on Windows; macOS + Windows feature flags
//! ship in subsequent Phase 2 PRs).
//!
//! Texture binding namespace: `chroma_CameraInput[N]` (N = 0..=3
//! per ADR-020 §Decision; mirrors Shadertoy iChannel range).
//! Render-side wiring (Phase 2 PR 5) marshals [`CameraFrame`] to
//! the wgpu texture binding.
//!
//! ## Phase-2 PR-1 scope
//!
//! This PR ships the surface-only scaffold. [`NokhwaInputAdapter`]
//! returns [`AdapterError::DeviceNotFound`] from [`CameraAdapter::bind`]
//! and an empty vec from [`CameraAdapter::enumerate`] until the
//! per-platform backends land:
//!
//! * Phase 2 PR 2 wires the macOS CoreMediaIO backend (nokhwa
//!   `input-avfoundation` feature) + the `NSCameraUsageDescription`
//!   Info.plist override + the async `AVCaptureDevice.requestAccess`
//!   permission flow per ADR-020 §5 / `docs/54` Phase-2 lesson 2.
//! * Phase 2 PR 3 wires the Windows Media Foundation backend
//!   (nokhwa `input-msmf` feature) + the Privacy Settings
//!   permission-denial routing.
//! * Phase 2 PR 4 ships the synthetic T2 corpus (row 78 / row 40 /
//!   row 93 narrowing precedent).
//! * Phase 2 PR 5 wires `engine/render/src/camera_adapter.rs` and
//!   closes the `EV-INTEGRATION-MATRIX` camera-arm widening with
//!   the Wave 1c-β anchor run id.
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
//! - ADR-020 — camera adapter (locked contract this PR scaffolds).

pub mod adapter;
pub mod defaults;
pub mod frames;

/// macOS-specific permission-flow surface per ADR-020 §5. Compiles
/// to a no-op (empty module) on non-macOS targets; the gating lives
/// at the module level rather than at the crate root so the cross-
/// platform [`adapter`] surface stays uniform.
#[cfg(target_os = "macos")]
pub mod macos;

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
