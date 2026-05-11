// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! OSC adapter IPC surface.
//!
//! W14 PR4 introduced the [`osc_health_snapshot`] IPC command (still
//! a stub — adapter binding lives on the Phase-3+ wiring queue).
//! Wave 2.1 Lane D.3 (`/Users/xaviermac/.claude/plans/inherited-percolating-quill.md`)
//! adds three operator-facing handlers backing the OSC view's routes
//! table + bind / unbind affordances:
//!
//! - [`osc_routes_snapshot`] — enumerate the canonical-route registry
//!   (the seven UX entries the v0.2-beta bundle's OSC view JSX
//!   fixture surfaces, mirrored in `src/lib/data/osc-routes.ts`).
//! - [`bind_osc_route`] — set the operator's advisory bind target
//!   for a single route. Validation is intentionally minimal (the
//!   target string must be non-empty); semantic checks against real
//!   control ids or scene ids are deferred until the runtime dispatch
//!   surface that consumes the bind target ships.
//! - [`unbind_osc_route`] — clear the bind target on a single route.
//!
//! All three handlers operate on [`crate::state::AppState::osc_routes`]
//! — a `Mutex<OscRouteRegistry>` seeded on [`crate::state::AppState::new`]
//! with the canonical UX subset. The registry is process-local (not
//! persisted to disk in Wave 2.1 Lane D.3); see the field-level prose
//! on `AppState::osc_routes` for the persistence-deferral rationale.
//!
//! ## Error model
//!
//! `bind_osc_route` and `unbind_osc_route` return `Err(String)` for two
//! distinct failure classes:
//!
//! - Unknown canonical route (the supplied address isn't in the
//!   seed table) — surfaces the [`OscRouteError::UnknownRoute`]
//!   `Display` form.
//! - Empty bind target (only relevant on `bind_osc_route`) — surfaces
//!   `"bind target must not be empty"`.
//!
//! `osc_routes_snapshot` is infallible; the `Result<_, String>` return
//! type matches the project's existing handler shape for forward-compat
//! with future wiring that might surface adapter-mutex-poisoned conditions.

use chromaticity_osc_adapter::route_registry::OscRouteError;
use chromaticity_ui_bridge::{OscHealthSnapshot, OscHealthState, OscRoutePayload};
use tauri::State;

use crate::state::AppState;

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

/// Inner implementation for [`osc_routes_snapshot`]; takes a borrowed
/// [`AppState`] directly so unit tests can exercise the seam without
/// a Tauri runtime.
fn osc_routes_snapshot_impl(app: &AppState) -> Result<Vec<OscRoutePayload>, String> {
    let guard = match app.osc_routes.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    Ok(guard
        .snapshot()
        .into_iter()
        .map(OscRoutePayload::from)
        .collect())
}

/// Inner implementation for [`bind_osc_route`]; takes a borrowed
/// [`AppState`] directly so unit tests can exercise the seam without
/// a Tauri runtime.
fn bind_osc_route_impl(app: &AppState, route: &str, target: &str) -> Result<(), String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err("bind target must not be empty".to_string());
    }
    let mut guard = match app.osc_routes.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard
        .bind(route, trimmed)
        .map_err(|err: OscRouteError| err.to_string())
}

/// Inner implementation for [`unbind_osc_route`]; takes a borrowed
/// [`AppState`] directly so unit tests can exercise the seam without
/// a Tauri runtime.
fn unbind_osc_route_impl(app: &AppState, route: &str) -> Result<(), String> {
    let mut guard = match app.osc_routes.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard
        .unbind(route)
        .map_err(|err: OscRouteError| err.to_string())
}

/// Enumerate the OSC route registry as a snapshot of
/// [`OscRoutePayload`]s. Wave 2.1 Lane D.3.
#[tauri::command]
pub fn osc_routes_snapshot(app_state: State<'_, AppState>) -> Result<Vec<OscRoutePayload>, String> {
    osc_routes_snapshot_impl(app_state.inner())
}

/// Bind `target` to the route identified by canonical address `route`.
/// Wave 2.1 Lane D.3.
#[tauri::command]
pub fn bind_osc_route(
    app_state: State<'_, AppState>,
    route: String,
    target: String,
) -> Result<(), String> {
    bind_osc_route_impl(app_state.inner(), &route, &target)
}

/// Clear the bind target on the route identified by canonical
/// address `route`. Wave 2.1 Lane D.3.
#[tauri::command]
pub fn unbind_osc_route(app_state: State<'_, AppState>, route: String) -> Result<(), String> {
    unbind_osc_route_impl(app_state.inner(), &route)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! W14 PR4 IPC contract pins for the health-snapshot stub plus
    //! Wave 2.1 Lane D.3 pins for the route-registry handler glue.
    //! Engine-side seed + bind / unbind invariants live in
    //! `integrations/osc-adapter/src/route_registry.rs`; these tests
    //! pin the SHELL-side `Mutex::lock → registry.method → IPC payload`
    //! chain that is otherwise only exercised end-to-end through
    //! Tauri.

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

    #[test]
    fn routes_snapshot_returns_seeded_routes_unbound() {
        let app = AppState::new();
        let routes = osc_routes_snapshot_impl(&app).expect("infallible");
        assert!(!routes.is_empty(), "default seed must be non-empty");
        assert!(
            routes.iter().all(|r| r.bind_target.is_none()),
            "every seeded entry must start unbound"
        );
    }

    #[test]
    fn bind_then_snapshot_surfaces_the_target() {
        let app = AppState::new();
        bind_osc_route_impl(&app, "/controls/{bank}/{name}", "fader-1")
            .expect("seeded route accepts bind");
        let routes = osc_routes_snapshot_impl(&app).expect("infallible");
        let entry = routes
            .iter()
            .find(|r| r.canonical == "/controls/{bank}/{name}")
            .expect("seeded route in snapshot");
        assert_eq!(entry.bind_target.as_deref(), Some("fader-1"));
    }

    #[test]
    fn bind_rejects_empty_target() {
        let app = AppState::new();
        let err = bind_osc_route_impl(&app, "/controls/{bank}/{name}", "")
            .expect_err("empty target should reject");
        assert_eq!(err, "bind target must not be empty");
        // Whitespace-only is also rejected.
        let err = bind_osc_route_impl(&app, "/controls/{bank}/{name}", "   ")
            .expect_err("whitespace-only target should reject");
        assert_eq!(err, "bind target must not be empty");
    }

    #[test]
    fn bind_rejects_unknown_canonical_route() {
        let app = AppState::new();
        let err = bind_osc_route_impl(&app, "/not/a/real/route", "anything")
            .expect_err("unknown route should reject");
        assert!(
            err.contains("/not/a/real/route"),
            "error should reference the unknown canonical: got {err:?}"
        );
    }

    #[test]
    fn unbind_clears_target_via_handler_glue() {
        let app = AppState::new();
        bind_osc_route_impl(&app, "/scenes/{sceneName}", "s-12")
            .expect("seeded route accepts bind");
        unbind_osc_route_impl(&app, "/scenes/{sceneName}").expect("seeded route accepts unbind");
        let routes = osc_routes_snapshot_impl(&app).expect("infallible");
        let entry = routes
            .iter()
            .find(|r| r.canonical == "/scenes/{sceneName}")
            .expect("seeded route in snapshot");
        assert!(entry.bind_target.is_none(), "unbind should clear target");
    }

    #[test]
    fn unbind_rejects_unknown_canonical_route() {
        let app = AppState::new();
        let err = unbind_osc_route_impl(&app, "/not/a/real/route")
            .expect_err("unknown route should reject");
        assert!(err.contains("/not/a/real/route"));
    }

    #[test]
    fn bind_target_trims_surrounding_whitespace() {
        // The frontend's text input may surface trailing whitespace
        // from operator copy-paste; the handler trims at the seam
        // so the stored target stays clean.
        let app = AppState::new();
        bind_osc_route_impl(&app, "/controls/{bank}/{name}", "  fader-1  ")
            .expect("trim-friendly bind");
        let routes = osc_routes_snapshot_impl(&app).expect("infallible");
        let entry = routes
            .iter()
            .find(|r| r.canonical == "/controls/{bank}/{name}")
            .expect("seeded route in snapshot");
        assert_eq!(entry.bind_target.as_deref(), Some("fader-1"));
    }
}
