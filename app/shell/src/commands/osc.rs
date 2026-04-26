// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! OSC adapter IPC surface.
//!
//! W14 PR4 introduces the `osc_health_snapshot` IPC command consumed by
//! the W14 OSC view (`src/lib/views/Osc.svelte`) to render the
//! adapter-health pill + listen-address readout. The W4 hardening pass
//! defined the underlying [`OscHealthState`] state machine
//! (`Healthy / Recovering / Degraded`) on the
//! `chromaticity-osc-adapter` crate; this module surfaces it across the
//! Tauri command boundary via the IPC types in
//! `app/ui-bridge/src/lib.rs`.
//!
//! W14 scope: STUB. [`AppState`] does not yet own a `UdpOscAdapter`
//! instance — wiring it in (with a real `bind()` call, async poll
//! loop, and packet-outcome telemetry) is Phase-3 scope per
//! `docs/26-evidence-artifact-registry.md` `EV-INTEGRATION-MATRIX`.
//! Until then, this command returns a placeholder snapshot
//! representing "adapter not yet bound" so the UI surface ships and
//! the IPC contract is locked in. The Phase-3 follow-up replaces the
//! stub body with a read-through to the live adapter.
//!
//! Locked design choices:
//!
//! - Stub returns [`OscHealthState::Healthy`] with `attempted_binds: 0`
//!   and `local_addr: None`. This matches the "fresh adapter, never
//!   tried to bind" state — operator-correct because no bind has been
//!   attempted yet, not because a bind succeeded.
//! - `local_addr` is `Option<String>` (not `SocketAddr`) at the IPC
//!   boundary because ts-rs doesn't bind `std::net::SocketAddr`. The
//!   shell stringifies once when emitting the snapshot.
//! - Returns `Result<OscHealthSnapshot, String>` for symmetry with the
//!   other shell commands, even though the W14 stub never errors.
//!   Phase-3 wiring may surface `Err` for adapter-mutex-poisoned or
//!   adapter-not-initialised conditions.

use chromaticity_ui_bridge::{OscHealthSnapshot, OscHealthState};

/// Read the OSC adapter's health snapshot.
///
/// **W14 stub.** Phase-3 will replace the stub body with a
/// read-through to a `UdpOscAdapter` owned by [`AppState`]. See
/// module prose for scope rationale.
#[tauri::command]
pub fn osc_health_snapshot() -> Result<OscHealthSnapshot, String> {
    osc_health_snapshot_impl()
}

/// Inner stub returning the placeholder snapshot. Pulled out as a
/// pure function so the W14 unit tests can exercise the same body
/// the Tauri command shell wraps.
fn osc_health_snapshot_impl() -> Result<OscHealthSnapshot, String> {
    Ok(OscHealthSnapshot {
        state: OscHealthState::Healthy,
        attempted_binds: 0,
        local_addr: None,
    })
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! W14 PR4 IPC contract pins for the stub. Phase-3 wiring will
    //! widen these to behavioural assertions over a live adapter.

    use super::*;

    #[test]
    fn stub_returns_healthy_unbound_snapshot() {
        let snapshot = osc_health_snapshot_impl().expect("stub never errors");
        assert!(matches!(snapshot.state, OscHealthState::Healthy));
        assert_eq!(snapshot.attempted_binds, 0);
        assert_eq!(snapshot.local_addr, None);
    }

    #[test]
    fn snapshot_serializes_round_trip() {
        let snapshot = osc_health_snapshot_impl().expect("stub never errors");
        let json = serde_json::to_string(&snapshot).expect("serializes");
        let parsed: OscHealthSnapshot = serde_json::from_str(&json).expect("round-trips");
        assert_eq!(parsed, snapshot);
    }
}
