// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! OSC adapter IPC surface.
//!
//! W14 PR4 introduced the [`osc_health_snapshot`] IPC command; Round 3
//! wires it to the live UDP adapter owned by [`AppState`].
//! Wave 2.1 Lane D.3 (`/Users/xaviermac/.claude/plans/inherited-percolating-quill.md`)
//! adds three operator-facing handlers backing the OSC view's routes
//! table + bind / unbind affordances:
//!
//! - [`osc_routes_snapshot`] — enumerate the canonical-route registry
//!   (the seven UX entries the v0.2-beta bundle's OSC view JSX
//!   fixture surfaces, mirrored in `src/lib/data/osc-routes.ts`).
//! - [`bind_osc_route`] — set the operator's bind target for a single
//!   route. The string stays verbatim on the IPC wire but is parsed at
//!   bind time by `chromaticity_osc_adapter::bindings` so malformed
//!   targets cannot become silent no-ops.
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
//! - Empty or malformed bind target (only relevant on
//!   `bind_osc_route`) — surfaces the adapter bind-target parser's
//!   human-readable error.
//!
//! `osc_routes_snapshot` is infallible; the `Result<_, String>` return
//! type matches the project's existing handler shape for forward-compat
//! with future wiring that might surface adapter-mutex-poisoned conditions.

use chromaticity_osc_adapter::adapter::{OscAdapter, OscHealthState as AdapterOscHealthState};
use chromaticity_osc_adapter::bindings::{OscDispatchAction, OscRuntimeCommand};
use chromaticity_osc_adapter::route_registry::OscRouteError;
use chromaticity_runtime::{
    Command, CommandBus, ControlId as RuntimeControlId, ControlValue as RuntimeControlValue,
    PresetId, SnapSlotId,
};
use chromaticity_scene::loader::load_chroma_dir;
use chromaticity_scene::schema::Scene;
use chromaticity_ui_bridge::{
    LoadedScenePayload, OscHealthSnapshot, OscHealthState, OscRoutePayload,
};
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use tauri::State;

use crate::commands::scene::build_loaded_scene_payload;
use crate::state::{AppState, LoadedSceneRecord};

const BUNDLED_USER_FACING_SCENE_SUBDIRS: &[&str] = &["library", "welcome", "examples"];

/// Read the OSC adapter's health snapshot.
///
/// Read-through to the `UdpOscAdapter` owned by [`AppState`].
#[tauri::command]
pub fn osc_health_snapshot(app_state: State<'_, AppState>) -> Result<OscHealthSnapshot, String> {
    osc_health_snapshot_impl(app_state.inner())
}

/// Inner implementation pulled out so unit tests can exercise the
/// same body the Tauri command shell wraps.
fn osc_health_snapshot_impl(app: &AppState) -> Result<OscHealthSnapshot, String> {
    let adapter = match app.osc_adapter.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    Ok(OscHealthSnapshot {
        state: bridge_health_state(adapter.health_state()),
        attempted_binds: adapter.attempted_binds(),
        local_addr: adapter.local_addr().map(|addr| addr.to_string()),
    })
}

fn bridge_health_state(state: AdapterOscHealthState) -> OscHealthState {
    match state {
        AdapterOscHealthState::Healthy => OscHealthState::Healthy,
        AdapterOscHealthState::Recovering => OscHealthState::Recovering,
        AdapterOscHealthState::Degraded => OscHealthState::Degraded,
        _ => OscHealthState::Degraded,
    }
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
    {
        let mut guard = match app.osc_routes.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard
            .bind(route, trimmed)
            .map_err(|err: OscRouteError| err.to_string())?;
    }

    let adapter_result = {
        let mut adapter = match app.osc_adapter.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        adapter.bind_route(route, trimmed)
    };
    if let Err(err) = adapter_result {
        let mut guard = match app.osc_routes.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let _ = guard.unbind(route);
        return Err(err.to_string());
    }
    Ok(())
}

/// Inner implementation for [`unbind_osc_route`]; takes a borrowed
/// [`AppState`] directly so unit tests can exercise the seam without
/// a Tauri runtime.
fn unbind_osc_route_impl(app: &AppState, route: &str) -> Result<(), String> {
    {
        let mut guard = match app.osc_routes.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard
            .unbind(route)
            .map_err(|err: OscRouteError| err.to_string())?;
    }
    let mut adapter = match app.osc_adapter.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    adapter.unbind_route(route);
    Ok(())
}

fn runtime_command_from_osc(command: OscRuntimeCommand) -> Result<Command, String> {
    match command {
        OscRuntimeCommand::ControlSet { id, value } => Ok(Command::ControlSet {
            id: RuntimeControlId(id),
            value: RuntimeControlValue(value),
        }),
        OscRuntimeCommand::PresetRecall { id } => Ok(Command::PresetRecall { id: PresetId(id) }),
        OscRuntimeCommand::SnapRecall { slot } => {
            let slot = SnapSlotId::new(slot)
                .ok_or_else(|| format!("invalid snap slot {slot}: max is {}", SnapSlotId::MAX))?;
            Ok(Command::SnapRecall { slot })
        }
        OscRuntimeCommand::PlaylistAdvance { step } => Ok(Command::PlaylistAdvance { step }),
        _ => Err("unsupported osc runtime command".to_string()),
    }
}

fn dispatch_osc_action_impl(app: &AppState, action: OscDispatchAction) -> Result<(), String> {
    match action {
        OscDispatchAction::RuntimeCommand(command) => {
            let command = runtime_command_from_osc(command)?;
            let mut state = app
                .state
                .lock()
                .map_err(|err| format!("state mutex poisoned: {err}"))?;
            app.bus
                .dispatch_and_commit_into(command, &mut state)
                .map_err(|err| err.to_string())?;
            Ok(())
        }
        OscDispatchAction::SceneLoad { scene_id } => {
            load_scene_by_id_impl(app, &scene_id).map(|_| ())
        }
        _ => Err("unsupported osc dispatch action".to_string()),
    }
}

/// Configure the live UDP adapter from the current settings snapshot.
///
/// Called during setup and after settings writes. Bind failures are
/// surfaced to the caller, but the adapter remains installed so the
/// health command can report attempted bind count and `local_addr =
/// None` instead of leaving OSC as a silent no-op.
pub(crate) fn configure_live_osc_adapter(app: &AppState) -> Result<(), String> {
    let settings = {
        let guard = match app.settings.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.osc.clone()
    };
    let route_snapshot = {
        let guard = match app.osc_routes.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.snapshot()
    };

    let mut adapter = match app.osc_adapter.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    adapter.clear_route_bindings();
    for route in route_snapshot {
        if let Some(target) = route.bind_target.as_deref() {
            adapter
                .bind_route(&route.canonical, target)
                .map_err(|err| err.to_string())?;
        }
    }

    adapter.unbind().map_err(|err| err.to_string())?;
    if !settings.rx_enabled {
        return Ok(());
    }
    let ip: IpAddr = settings
        .local_ip
        .parse()
        .map_err(|err| format!("invalid OSC listen IP {}: {err}", settings.local_ip))?;
    adapter
        .bind(SocketAddr::new(ip, settings.port))
        .map_err(|err| err.to_string())
}

/// Poll the live UDP adapter once. The caller must not hold
/// `AppState::state`; adapter dispatch only queues actions.
pub(crate) fn poll_live_osc_adapter(app: &AppState) -> Result<(), String> {
    let mut adapter = match app.osc_adapter.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    adapter.poll().map_err(|err| err.to_string())
}

/// Drain OSC actions queued by the adapter sink and apply them through
/// the runtime command bus or shell scene loader.
pub(crate) fn drain_queued_osc_actions(app: &AppState) -> Result<usize, String> {
    let actions = {
        let mut guard = match app.osc_dispatch_queue.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        std::mem::take(&mut *guard)
    };
    let count = actions.len();
    for action in actions {
        dispatch_osc_action_impl(app, action)?;
    }
    Ok(count)
}

fn load_scene_by_id_impl(app: &AppState, scene_id: &str) -> Result<LoadedScenePayload, String> {
    let candidates = scene_candidates(app, scene_id);
    let mut attempted = Vec::new();
    for path in candidates {
        attempted.push(path.display().to_string());
        match load_scene_path(app, &path) {
            Ok(payload) => return Ok(payload),
            Err(_) => continue,
        }
    }
    Err(format!(
        "osc scene target {scene_id} did not resolve to a loadable .chroma package (tried: {})",
        attempted.join(", "),
    ))
}

fn load_scene_path(app: &AppState, path: &Path) -> Result<LoadedScenePayload, String> {
    let scene = load_chroma_dir(path).map_err(|err| err.to_string())?;
    store_loaded_scene(app, scene, path)
}

fn store_loaded_scene(
    app: &AppState,
    scene: Scene,
    path: &Path,
) -> Result<LoadedScenePayload, String> {
    let payload = build_loaded_scene_payload(&scene, path);
    let record = LoadedSceneRecord {
        scene,
        path: path.to_path_buf(),
    };
    match app.loaded_scene.lock() {
        Ok(mut guard) => {
            *guard = Some(record);
        }
        Err(poisoned) => {
            let mut guard = poisoned.into_inner();
            *guard = Some(record);
        }
    }
    Ok(payload)
}

fn scene_candidates(app: &AppState, scene_id: &str) -> Vec<PathBuf> {
    let trimmed = scene_id.trim();
    let mut candidates = Vec::new();
    if trimmed.is_empty() {
        return candidates;
    }

    let direct = PathBuf::from(trimmed);
    if direct.is_absolute() || direct.extension().and_then(|ext| ext.to_str()) == Some("chroma") {
        push_candidate(&mut candidates, direct);
    }

    let mut roots = Vec::new();
    if let Some(root) = match app.bundled_scene_root.lock() {
        Ok(g) => g.clone(),
        Err(p) => p.into_inner().clone(),
    } {
        roots.push(root);
    }
    if let Some(root) = platform_default_scene_root() {
        roots.push(root);
    }
    {
        let settings = match app.settings.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        roots.extend(settings.storage.scene_folders.iter().map(PathBuf::from));
    }

    let package_name = if trimmed.ends_with(".chroma") {
        trimmed.to_string()
    } else {
        format!("{trimmed}.chroma")
    };
    for root in roots {
        push_candidate(&mut candidates, root.join(&package_name));
        for subdir in BUNDLED_USER_FACING_SCENE_SUBDIRS {
            push_candidate(&mut candidates, root.join(subdir).join(&package_name));
        }
    }
    candidates
}

fn push_candidate(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if !candidates.iter().any(|existing| existing == &path) {
        candidates.push(path);
    }
}

fn platform_default_scene_root() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join("Library/Application Support/Chromaticity/Scenes"))
    } else if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .ok()
            .map(|appdata| PathBuf::from(appdata).join("Chromaticity").join("Scenes"))
    } else if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if xdg.is_empty() {
            std::env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join(".local/share/chromaticity/scenes"))
        } else {
            Some(PathBuf::from(xdg).join("chromaticity/scenes"))
        }
    } else {
        std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(".local/share/chromaticity/scenes"))
    }
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
    use chromaticity_osc_adapter::adapter::OscAdapter;
    use chromaticity_osc_adapter::bindings::control_target_id;
    use std::net::{SocketAddr, UdpSocket};
    use std::thread;
    use std::time::{Duration, Instant};

    const POLL_DEADLINE: Duration = Duration::from_secs(1);
    const POLL_INTERVAL: Duration = Duration::from_millis(5);

    fn fixture_scene_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../engine/scene/tests/fixtures/scenes")
    }

    fn bind_app_osc_ephemeral(app: &AppState) -> SocketAddr {
        let mut adapter = app.osc_adapter.lock().expect("osc adapter lock");
        adapter
            .bind("127.0.0.1:0".parse().expect("loopback socket addr"))
            .expect("bind ephemeral osc adapter");
        adapter.local_addr().expect("bound adapter local addr")
    }

    fn encode_osc_message(addr: &str, args: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_osc_string(&mut bytes, addr);
        let mut tags = String::from(",");
        for _ in args {
            tags.push('f');
        }
        push_osc_string(&mut bytes, &tags);
        for value in args {
            bytes.extend(value.to_bits().to_be_bytes());
        }
        bytes
    }

    fn push_osc_string(bytes: &mut Vec<u8>, value: &str) {
        bytes.extend(value.as_bytes());
        bytes.push(0);
        while bytes.len() % 4 != 0 {
            bytes.push(0);
        }
    }

    fn send_osc(sender: &UdpSocket, addr: SocketAddr, path: &str, args: &[f32]) {
        sender
            .send_to(&encode_osc_message(path, args), addr)
            .expect("send osc packet");
    }

    fn poll_and_drain_until<F>(app: &AppState, mut done: F)
    where
        F: FnMut() -> bool,
    {
        let start = Instant::now();
        loop {
            poll_live_osc_adapter(app).expect("osc poll succeeds");
            let _ = drain_queued_osc_actions(app).expect("queued osc actions drain");
            if done() {
                return;
            }
            assert!(
                start.elapsed() < POLL_DEADLINE,
                "timed out waiting for OSC dispatch"
            );
            thread::sleep(POLL_INTERVAL);
        }
    }

    #[test]
    fn health_snapshot_reads_fresh_unbound_adapter() {
        let app = AppState::new();
        let snapshot = osc_health_snapshot_impl(&app).expect("health snapshot reads adapter");
        assert!(matches!(snapshot.state, OscHealthState::Healthy));
        assert_eq!(snapshot.attempted_binds, 0);
        assert_eq!(snapshot.local_addr, None);
    }

    #[test]
    fn snapshot_serializes_round_trip() {
        let app = AppState::new();
        let snapshot = osc_health_snapshot_impl(&app).expect("health snapshot reads adapter");
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
        bind_osc_route_impl(&app, "/controls/{bank}/{name}", "control:scene:fader-1")
            .expect("seeded route accepts bind");
        let routes = osc_routes_snapshot_impl(&app).expect("infallible");
        let entry = routes
            .iter()
            .find(|r| r.canonical == "/controls/{bank}/{name}")
            .expect("seeded route in snapshot");
        assert_eq!(entry.bind_target.as_deref(), Some("control:scene:fader-1"),);
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
    fn bind_rejects_invalid_target_grammar() {
        let app = AppState::new();
        let err = bind_osc_route_impl(&app, "/controls/{bank}/{name}", "fader-1")
            .expect_err("malformed target should reject");
        assert!(
            err.contains("unknown osc bind target family"),
            "error should explain target grammar: got {err:?}",
        );
    }

    #[test]
    fn bind_rejects_unknown_canonical_route() {
        let app = AppState::new();
        let err = bind_osc_route_impl(&app, "/not/a/real/route", "control:scene:fader-1")
            .expect_err("unknown route should reject");
        assert!(
            err.contains("/not/a/real/route"),
            "error should reference the unknown canonical: got {err:?}"
        );
    }

    #[test]
    fn unbind_clears_target_via_handler_glue() {
        let app = AppState::new();
        bind_osc_route_impl(&app, "/scenes/{sceneName}", "scene:s-12")
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
        bind_osc_route_impl(&app, "/controls/{bank}/{name}", "  control:scene:fader-1  ")
            .expect("trim-friendly bind");
        let routes = osc_routes_snapshot_impl(&app).expect("infallible");
        let entry = routes
            .iter()
            .find(|r| r.canonical == "/controls/{bank}/{name}")
            .expect("seeded route in snapshot");
        assert_eq!(entry.bind_target.as_deref(), Some("control:scene:fader-1"),);
    }

    #[test]
    fn health_snapshot_reports_bound_adapter_addr() {
        let app = AppState::new();
        let bound = bind_app_osc_ephemeral(&app);

        let snapshot = osc_health_snapshot_impl(&app).expect("health snapshot reads adapter");

        assert!(matches!(snapshot.state, OscHealthState::Healthy));
        assert_eq!(snapshot.attempted_binds, 1);
        let bound_string = bound.to_string();
        assert_eq!(snapshot.local_addr.as_deref(), Some(bound_string.as_str()));
    }

    #[test]
    fn live_adapter_dispatches_runtime_targets_from_udp() {
        let app = AppState::new();
        let bound = bind_app_osc_ephemeral(&app);
        bind_osc_route_impl(&app, "/controls/{bank}/{name}", "control:fx:speed")
            .expect("control route binds");
        bind_osc_route_impl(&app, "/presets/{name}", "preset:7").expect("preset route binds");
        bind_osc_route_impl(&app, "/favslots/{n}", "snap:3").expect("snap route binds");
        bind_osc_route_impl(&app, "/playlists/next", "playlist:next")
            .expect("playlist route binds");
        let sender = UdpSocket::bind("127.0.0.1:0").expect("sender binds");

        send_osc(&sender, bound, "/controls/fx/speed", &[0.42]);
        send_osc(&sender, bound, "/presets/main", &[]);
        send_osc(&sender, bound, "/favslots/3", &[]);
        send_osc(&sender, bound, "/playlists/next", &[]);

        poll_and_drain_until(&app, || {
            let state = app.state.lock().expect("state lock");
            state.control_value(RuntimeControlId(control_target_id("fx", "speed")))
                == Some(RuntimeControlValue(0.42))
                && state.active_preset() == Some(PresetId(7))
                && state.active_snap_slot() == Some(SnapSlotId::new(3).expect("valid slot"))
                && state.playlist_position() == 1
        });
    }

    #[test]
    fn live_adapter_dispatches_scene_target_from_udp() {
        let app = AppState::new();
        app.install_bundled_scene_root(fixture_scene_root());
        let bound = bind_app_osc_ephemeral(&app);
        bind_osc_route_impl(&app, "/scenes/{sceneName}", "scene:welcome")
            .expect("scene route binds");
        let sender = UdpSocket::bind("127.0.0.1:0").expect("sender binds");

        send_osc(&sender, bound, "/scenes/welcome", &[]);

        poll_and_drain_until(&app, || {
            let guard = app.loaded_scene.lock().expect("loaded scene lock");
            guard
                .as_ref()
                .map(|record| record.path.ends_with("welcome/welcome.chroma"))
                .unwrap_or(false)
        });
    }

    #[test]
    fn dispatch_osc_runtime_actions_commit_through_bus() {
        let app = AppState::new();
        dispatch_osc_action_impl(
            &app,
            OscDispatchAction::RuntimeCommand(OscRuntimeCommand::ControlSet { id: 9, value: 0.5 }),
        )
        .expect("control action dispatches");
        dispatch_osc_action_impl(
            &app,
            OscDispatchAction::RuntimeCommand(OscRuntimeCommand::PresetRecall { id: 4 }),
        )
        .expect("preset action dispatches");
        dispatch_osc_action_impl(
            &app,
            OscDispatchAction::RuntimeCommand(OscRuntimeCommand::SnapRecall { slot: 3 }),
        )
        .expect("snap action dispatches");
        dispatch_osc_action_impl(
            &app,
            OscDispatchAction::RuntimeCommand(OscRuntimeCommand::PlaylistAdvance { step: -1 }),
        )
        .expect("playlist action dispatches");

        let state = app.state.lock().expect("state lock");
        assert_eq!(
            state.control_value(RuntimeControlId(9)),
            Some(RuntimeControlValue(0.5)),
        );
        assert_eq!(state.active_preset(), Some(PresetId(4)));
        assert_eq!(
            state.active_snap_slot(),
            Some(SnapSlotId::new(3).expect("valid slot")),
        );
        assert_eq!(state.playlist_position(), -1);
    }

    #[test]
    fn dispatch_osc_scene_action_loads_scene_into_app_state() {
        let app = AppState::new();
        app.install_bundled_scene_root(fixture_scene_root());

        dispatch_osc_action_impl(
            &app,
            OscDispatchAction::SceneLoad {
                scene_id: "welcome".to_string(),
            },
        )
        .expect("scene target loads");
        let guard = app.loaded_scene.lock().expect("loaded scene lock");
        let record = guard.as_ref().expect("scene loaded");
        assert!(record.path.ends_with("welcome/welcome.chroma"));
    }
}
