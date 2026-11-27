// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Syphon source-picker + output-publish IPC handlers — Wave 1c-β
//! Phase 4 Lane B.
//!
//! Surfaces three Tauri commands consumed by the Syphon section of
//! `src/lib/views/Sources.svelte` (via `src/lib/stores/syphonStore.svelte.ts`):
//!
//! - [`list_syphon_servers`] — enumerate currently-advertised servers
//!   on the local-machine Syphon bus.
//! - [`publish_syphon_output`] — register a chromaticity output server
//!   on the bus under the operator-supplied name.
//! - [`unpublish_syphon_output`] — drop the registered output server.
//!
//! ## macOS-only feature gate
//!
//! Syphon is a macOS-only protocol per ADR-021 §1. The
//! `chromaticity_syphon_adapter` crate body is gated by
//! `#![cfg(target_os = "macos")]`; mirroring that gate at the
//! command-handler level keeps non-macOS workspace builds free of the
//! Objective-C binding stack. Each command compiles in two flavours:
//! the macOS arm constructs the real adapter scaffold; the non-macOS
//! arm returns `Err("syphon: macOS-only feature")` so the frontend
//! store renders the "Unavailable on this platform" empty-state
//! branch.
//!
//! ## Phase 4 Lane B scope narrowing — stub posture
//!
//! Phase 3 PR 2 of Wave 1c-β scaffolded the `SyphonServerDirectory`
//! singleton + the `SyphonInputAdapter` / `SyphonOutputAdapter`
//! structs but explicitly deferred two pieces of production wiring:
//!
//!   * `SyphonServerDirectory::current_servers` returns an empty list
//!     because the `NSDistributedNotificationCenter` observer
//!     (which requires the Tauri main-thread runloop per
//!     `docs/54-wave-1c-beta-research-lessons.md` Phase-3 lesson 4)
//!     lives at the engine-side bridge in Phase 3 PR 3 — and PR 3
//!     ships only the wgpu interop bridge type surface, not the live
//!     observer wiring. The live observer follows in a separate
//!     follow-up wave.
//!   * `SyphonInputAdapter::with_handle` accepts a
//!     `Box<dyn SyphonClientHandle>` whose `SyphonClientHandle: Send`
//!     bound conflicts with the `objc2::rc::Retained<>: !Send` posture
//!     of the real backing handle. The `Send`-vs-`!Send` tension
//!     resolves in Phase 5 alongside the production publish loop.
//!
//! Until those land, the three commands ship under the documented
//! stub posture below — the IPC surface itself is in place so the
//! follow-up PRs only need to flip the underlying adapter
//! implementation; no command-handler / store / view churn.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm, Phase-4a Wave
//! 1c-β Phase 4 Lane B — operator-facing UI surface in place; live
//! enumeration + publish loop follow in subsequent PRs).

use chromaticity_ui_bridge::{SyphonOutputStatusPayload, SyphonServerPayload};

/// Enumerate currently-advertised Syphon servers on the local-machine
/// bus.
///
/// Returns one [`SyphonServerPayload`] per server surfaced by the
/// process-global [`chromaticity_syphon_adapter::SyphonServerDirectory`]
/// singleton. The current implementation surfaces an empty vec under
/// the documented Phase 3 PR 2 stub posture — see the module-level
/// prose for the narrowing rationale. The frontend store treats an
/// empty vec as the "no Syphon sources found" empty-state branch
/// with the producer-app guidance copy.
///
/// # Errors
///
/// macOS arm: never errors today (the empty-vec stub is infallible).
/// Non-macOS arm: returns `Err("syphon: macOS-only feature")` so the
/// frontend renders the "Unavailable on this platform" branch.
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn list_syphon_servers() -> Result<Vec<SyphonServerPayload>, String> {
    // Phase 3 PR 2 SyphonServerDirectory ships current_servers() as
    // an empty-list stub until the engine-side
    // NSDistributedNotificationCenter observer (Phase 3 PR 3 + a
    // subsequent live-wire follow-up wave) populates the cache.
    // The IPC handler surfaces that verbatim — no error injection,
    // no fabricated server.
    let directory = chromaticity_syphon_adapter::SyphonServerDirectory::shared();
    Ok(directory
        .current_servers()
        .into_iter()
        .map(SyphonServerPayload::from)
        .collect())
}

/// Non-macOS stub: surfaces the platform-unavailability error so the
/// frontend store renders the "Unavailable on this platform"
/// empty-state branch.
///
/// # Errors
///
/// Always returns `Err("syphon: macOS-only feature")`.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn list_syphon_servers() -> Result<Vec<SyphonServerPayload>, String> {
    Err(String::from("syphon: macOS-only feature"))
}

/// Register a chromaticity output server on the Syphon bus under the
/// operator-supplied name.
///
/// **CRITICAL operator-facing trust-boundary contract** per ADR-021
/// §10 + `docs/29-cleanroom-legal-policy-baseline.md` Wave 1c-β
/// threat-model addendum: Syphon's IOSurface-backed protocol means
/// published GPU surfaces are visible to ANY macOS application on the
/// same machine. The frontend MUST display a non-dismissible warning
/// while output is active; the warning copy is canonical: "Syphon
/// output is visible to any application on this machine. Do not
/// publish surfaces containing PII or secrets."
///
/// Default-off output posture per ADR-013 + ADR-016 default-off
/// telemetry precedent: this command is invoked only on explicit
/// operator opt-in via the publish toggle in `Sources.svelte`.
///
/// # Errors
///
/// macOS arm: today returns `Ok(SyphonOutputStatusPayload::active(name))`
/// unconditionally under the Phase 4 Lane B stub posture (the real
/// `SyphonMetalServer` registration loop ships in Phase 5 alongside
/// the `SyphonClientHandle: Send` resolution per the module-level
/// prose). Non-macOS arm: returns `Err("syphon: macOS-only feature")`.
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn publish_syphon_output(name: String) -> Result<SyphonOutputStatusPayload, String> {
    // Phase 4 Lane B documented stub: synthesise the active status
    // payload until the real SyphonOutputAdapter publish loop wires
    // in Phase 5. The frontend trust-boundary warning fires off the
    // returned `active: true` flag, so the operator-facing safety
    // surface is in place independent of the underlying registration.
    Ok(SyphonOutputStatusPayload::active(name))
}

/// Non-macOS stub: surfaces the platform-unavailability error.
///
/// # Errors
///
/// Always returns `Err("syphon: macOS-only feature")`.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn publish_syphon_output(_name: String) -> Result<SyphonOutputStatusPayload, String> {
    Err(String::from("syphon: macOS-only feature"))
}

/// Unregister the chromaticity output server from the Syphon bus.
///
/// Safe to invoke when output is already inactive — returns the
/// inactive status payload either way (idempotent).
///
/// # Errors
///
/// macOS arm: today returns `Ok(SyphonOutputStatusPayload::inactive())`
/// unconditionally under the Phase 4 Lane B stub posture (the real
/// adapter Drop impl ships in Phase 5). Non-macOS arm: returns
/// `Err("syphon: macOS-only feature")`.
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn unpublish_syphon_output() -> Result<SyphonOutputStatusPayload, String> {
    // Phase 4 Lane B documented stub: surface the inactive status
    // payload immediately. Once the real SyphonOutputAdapter ships
    // in Phase 5 its Drop impl calls `[server stop]` defensively per
    // ADR-021 §8; this command then drops the held adapter and the
    // teardown follows the framework-level destructor path.
    Ok(SyphonOutputStatusPayload::inactive())
}

/// Non-macOS stub: surfaces the platform-unavailability error.
///
/// # Errors
///
/// Always returns `Err("syphon: macOS-only feature")`.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn unpublish_syphon_output() -> Result<SyphonOutputStatusPayload, String> {
    Err(String::from("syphon: macOS-only feature"))
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — pins the Phase 4 Lane B
    //! stub-posture surface for all three Syphon commands.
    //!
    //! Both the macOS and non-macOS arms are exercised by the
    //! workspace test runner — only the active arm fires per
    //! conditional compilation.

    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn list_syphon_servers_returns_empty_vec_under_scaffold_stub() {
        // The Phase 3 PR 2 SyphonServerDirectory documents an empty
        // current_servers() result until the engine-side
        // NSDistributedNotificationCenter observer wires
        // end-to-end. The IPC handler must surface that verbatim.
        let result = list_syphon_servers().expect("stub must not error");
        assert!(
            result.is_empty(),
            "expected empty vec under scaffold stub, got {result:?}",
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn publish_syphon_output_returns_active_payload_with_supplied_name() {
        let name = String::from("Chromaticity Performance");
        let status =
            publish_syphon_output(name.clone()).expect("publish stub must not error on macOS");
        assert!(status.active, "publish must surface active=true");
        assert_eq!(status.server_name.as_deref(), Some(name.as_str()));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn unpublish_syphon_output_returns_inactive_payload() {
        let status = unpublish_syphon_output().expect("unpublish stub must not error on macOS");
        assert!(!status.active, "unpublish must surface active=false");
        assert_eq!(status.server_name, None);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn list_syphon_servers_errors_on_non_macos() {
        let err = list_syphon_servers().expect_err("non-macOS arm must error");
        assert_eq!(err, "syphon: macOS-only feature");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn publish_syphon_output_errors_on_non_macos() {
        let err =
            publish_syphon_output(String::from("Test")).expect_err("non-macOS publish must error");
        assert_eq!(err, "syphon: macOS-only feature");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn unpublish_syphon_output_errors_on_non_macos() {
        let err = unpublish_syphon_output().expect_err("non-macOS unpublish must error");
        assert_eq!(err, "syphon: macOS-only feature");
    }
}
