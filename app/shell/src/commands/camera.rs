// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Camera device-enumeration IPC handler — Wave 1c-β Phase 4 Lane A.
//!
//! Surfaces the [`chromaticity_camera_adapter::CameraAdapter::enumerate`]
//! result across the Tauri command boundary so the operator-facing
//! camera device-picker in `src/lib/views/Sources.svelte` can render a
//! real device list (rather than the W17 PR1 fixture-only collection
//! row that read `cap: "io.camera"` and was visually disabled).
//!
//! ## Phase 4 Lane A scope narrowing — empty-vec stub posture
//!
//! Phase 2 PR 1 of Wave 1c-β scaffolded
//! [`chromaticity_camera_adapter::NokhwaInputAdapter`] with
//! [`CameraAdapter::enumerate`] returning [`Ok(Vec::new())`] until the
//! per-platform nokhwa enumeration arms wire end-to-end:
//!
//!   * macOS: Phase 2 PR 2 shipped the `AuthorizationProbe` /
//!     `InUsePrecheck` trait surfaces but the concrete
//!     `AVCaptureDevice` Objective-C binding (which requires a
//!     policy-scoped widening of the workspace `unsafe_code = "forbid"`
//!     floor per `docs/43` §12.6) lands in a follow-up PR.
//!   * Windows: Phase 2 PR 3 shipped the `WindowsCameraProbe` trait
//!     surface + canonical HRESULT → fault-reason mapping but the real
//!     consent-store registry read + Media Foundation HRESULT
//!     propagation lands in a follow-up policy-scoped PR.
//!
//! Until those land, [`list_camera_devices`] returns an empty vec — the
//! frontend store renders the "No cameras found" empty state with the
//! permission-grant guidance copy. The IPC surface itself is in place
//! so the per-platform follow-up PRs only need to flip the underlying
//! adapter implementation; no command-handler / store / view churn.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (camera arm, Phase-4a Wave 1c-β
//! Phase 4 Lane A — operator-facing UI surface in place; live
//! enumeration follows the per-platform unsafe-binding follow-up PRs).

use chromaticity_camera_adapter::{CameraAdapter, NokhwaInputAdapter};
use chromaticity_ui_bridge::CameraDevicePayload;

/// Enumerate available camera devices.
///
/// Returns one [`CameraDevicePayload`] per device surfaced by the
/// platform-default [`NokhwaInputAdapter`]. The current implementation
/// returns an empty vec (the adapter scaffold's documented stub posture)
/// until the per-platform nokhwa enumeration arms land in follow-up
/// policy-scoped PRs — see the module-level prose for the narrowing
/// rationale. The frontend store treats an empty vec as the "no cameras
/// available" empty-state branch with the permission-grant guidance
/// copy.
///
/// Errors surface as `Err(String)` so the frontend store can render an
/// operator-facing error banner without leaking adapter internals; the
/// stub path never errors today.
///
/// Sync (not `async`) per the project's existing Tauri command shape —
/// the underlying [`CameraAdapter::enumerate`] call is a non-blocking
/// stub today and per ADR-005 will remain a fast adapter-side method
/// once the per-platform backends wire (the heavy lifting lives on the
/// per-frame poll path, not the device-list path). Tauri 2 dispatches
/// sync commands on a worker pool, so this does not block the UI
/// thread.
#[tauri::command]
pub fn list_camera_devices() -> Result<Vec<CameraDevicePayload>, String> {
    let adapter = NokhwaInputAdapter::new();
    let summaries = adapter
        .enumerate()
        .map_err(|e| format!("camera enumeration failed: {e:?}"))?;
    Ok(summaries
        .into_iter()
        .map(CameraDevicePayload::from)
        .collect())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — pins the empty-vec stub
    //! posture documented in the module-level prose for the camera
    //! arm: with the per-platform enumeration arms not yet wired, the
    //! command surface returns an empty vec rather than erroring, so
    //! the frontend's "no cameras found" empty-state branch fires.

    use super::*;

    #[test]
    fn list_camera_devices_returns_empty_vec_under_scaffold_stub() {
        // The Phase-2-PR-1 NokhwaInputAdapter scaffold documents an
        // empty enumeration as the contract until the per-platform
        // backends wire end-to-end. The IPC handler must surface that
        // verbatim — no error injection, no fabricated device.
        let result = list_camera_devices().expect("stub must not error");
        assert!(
            result.is_empty(),
            "expected empty vec under scaffold stub, got {result:?}",
        );
    }
}
