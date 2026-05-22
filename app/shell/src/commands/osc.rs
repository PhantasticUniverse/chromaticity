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
use chromaticity_osc_adapter::sender::{OscArg, OscSender};
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
    let compat_alias_emits = {
        let sender_guard = match app.osc_sender.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        sender_guard
            .as_ref()
            .map(|s| s.compat_alias_emits())
            .unwrap_or(0)
    };
    Ok(OscHealthSnapshot {
        state: bridge_health_state(adapter.health_state()),
        attempted_binds: adapter.attempted_binds(),
        local_addr: adapter.local_addr().map(|addr| addr.to_string()),
        compat_alias_emits,
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
        OscRuntimeCommand::PlaylistSetPosition { index } => {
            Ok(Command::PlaylistSetPosition { index })
        }
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
        OscDispatchAction::SceneLoad { scene_id, preset } => {
            // Always attempt the scene load first; the preset recall is
            // best-effort — a missing or unresolvable preset id must not
            // abort the scene load.
            load_scene_by_id_impl(app, &scene_id).map(|_| ())?;
            if let Some(preset_id) = preset {
                let recall_result = {
                    let mut state = app
                        .state
                        .lock()
                        .map_err(|err| format!("state mutex poisoned: {err}"))?;
                    app.bus
                        .dispatch_and_commit_into(
                            Command::PresetRecall {
                                id: PresetId(preset_id),
                            },
                            &mut state,
                        )
                        .map_err(|err| err.to_string())
                };
                if let Err(err) = recall_result {
                    // Graceful diagnostic: log and continue; scene load
                    // already succeeded above.
                    eprintln!(
                        "osc scene-load preset recall (id={preset_id}) failed: {err}; \
                         scene load succeeded",
                    );
                }
            }
            Ok(())
        }
        OscDispatchAction::AudioThreshold { value } => {
            // Store + emit outbound announce (OscDirection::Both contract).
            apply_audio_threshold_and_announce(app, value);
            Ok(())
        }
        _ => Err("unsupported osc dispatch action".to_string()),
    }
}

/// Store the audio threshold and emit a single outbound announce to the
/// configured OSC target.
///
/// Called by both the inbound OSC dispatch path and the
/// `set_audio_threshold` Tauri command so both change-points share the
/// same announce logic. The emit is best-effort (non-blocking per the
/// [`OscSender`] posture); a send-buffer-full or socket-closed condition
/// is silently dropped — the threshold is stored regardless.
///
/// Dual-emit: `/audio/threshold` has no `compat_alias` in the route
/// registry (ADR-013 §4 "OSC Direction: Both" surface; singular form
/// is already canonical), so the compat-alias path is a no-op here.
/// The call still flows through [`emit_osc_with_dual`] for consistency
/// and to ensure future alias additions are automatically exercised.
pub(crate) fn apply_audio_threshold_and_announce(app: &AppState, value: f32) {
    let stored = app.store_audio_threshold(value);
    let mut guard = match app.osc_sender.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(sender) = guard.as_mut() {
        emit_osc_with_dual(app, sender, "/audio/threshold", &[OscArg::Float(stored)]);
    }
}

/// Return `true` when the dual-emit window is currently open.
///
/// The window is open when `dual_emit_enabled = true` AND either no
/// deadline is configured or the deadline has not yet passed. Deadline
/// comparison is lexicographic over RFC 3339 UTC strings using a
/// `chrono`-free implementation that reads wall-clock time via
/// [`std::time::SystemTime`].
pub(crate) fn dual_emit_active(settings: &chromaticity_runtime::settings::OscSettings) -> bool {
    if !settings.dual_emit_enabled {
        return false;
    }
    let Some(deadline) = settings.dual_emit_until.as_deref() else {
        return true;
    };
    // Compare lexicographically. RFC 3339 UTC strings sort correctly
    // when the timezone offset is `+00:00` or `Z`.
    let now = chrono_free_now_rfc3339();
    now.as_str() < deadline
}

/// Produce the current wall-clock instant as an RFC 3339 UTC string
/// without pulling in the `chrono` crate.
///
/// Format: `YYYY-MM-DDTHH:MM:SSZ`. Second-level granularity is
/// sufficient for deadline gating (the expiry strings operators
/// configure are date-level: `"2026-07-01T00:00:00Z"`).
///
/// Implementation uses Howard Hinnant's civil calendar algorithm to
/// convert the Unix epoch offset in seconds into a Gregorian
/// year/month/day triple, then formats the HH:MM:SS from the
/// remainder. No platform extensions, no `unsafe`, no external crates.
fn chrono_free_now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Seconds since Unix epoch; clamp to 0 on the unlikely pre-epoch case.
    let epoch_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Civil calendar: days since epoch → year / month / day.
    // Howard Hinnant algorithm (public domain / MIT).
    let days = (epoch_secs / 86_400) as i64;
    let z = days + 719_468;
    let era: i64 = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    let time_in_day = epoch_secs % 86_400;
    let hh = time_in_day / 3_600;
    let mm = (time_in_day % 3_600) / 60;
    let ss = time_in_day % 60;

    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Return the compat alias for a concrete outbound OSC address, if any.
///
/// The route registry stores template-style canonical addresses (e.g.
/// `/scenes/{sceneName}`) while callers operate on concrete addresses
/// (e.g. `/scenes/welcome`). This function walks the registry and
/// matches by static prefix so concrete addresses resolve to their
/// template entry's `compat_alias`.
///
/// Returns `None` when no matching registry entry carries a
/// `compat_alias`, which means `emit_with_compat_alias` will
/// short-circuit to canonical-only emission.
fn resolve_compat_alias(address: &str, app: &AppState) -> Option<String> {
    let registry = match app.osc_routes.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    // Walk all entries; match by static prefix of the template.
    for entry in registry.snapshot() {
        let static_pfx = canonical_static_prefix(&entry.canonical);
        if let Some(suffix) = address.strip_prefix(static_pfx) {
            if let Some(alias_template) = entry.compat_alias {
                // Replace the matching prefix with the alias prefix.
                let alias_pfx = canonical_static_prefix(&alias_template);
                return Some(format!("{alias_pfx}{suffix}"));
            }
        }
    }
    None
}

/// Extract the static (non-parameterized) prefix of a template address.
///
/// For `/scenes/{sceneName}` returns `/scenes/`; for `/playlists/next`
/// (no parameters) returns `/playlists/next`. Used to match a concrete
/// outbound address against a template registry entry.
fn canonical_static_prefix(template: &str) -> &str {
    match template.find('{') {
        Some(brace_pos) => &template[..brace_pos],
        None => template,
    }
}

/// Emit an outbound OSC message, automatically applying the compat-alias
/// dual-emit window from the current settings.
///
/// Resolves the `compat_alias` for `address` from the route registry,
/// evaluates `dual_emit_active`, and delegates to
/// [`OscSender::emit_with_compat_alias`]. Returns `true` if the
/// canonical send succeeded.
///
/// Callers hold the `osc_sender` mutex guard themselves; this function
/// takes a `&mut OscSender` directly to avoid a second lock acquisition.
pub(crate) fn emit_osc_with_dual(
    app: &AppState,
    sender: &mut OscSender,
    address: &str,
    args: &[OscArg],
) -> bool {
    let active = {
        let settings = match app.settings.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        dual_emit_active(&settings.osc)
    };
    let alias = resolve_compat_alias(address, app);
    let (canonical_sent, _alias_sent) =
        sender.emit_with_compat_alias(address, args, alias.as_deref(), active);
    canonical_sent
}

/// Configure the live UDP adapter and outbound sender from the
/// current settings snapshot.
///
/// Called during setup and after settings writes. Inbound bind
/// failures are surfaced to the caller, but the adapter remains
/// installed so the health command can report attempted bind count
/// and `local_addr = None` instead of leaving OSC as a silent no-op.
///
/// Outbound sender configuration is best-effort: a bind failure for
/// the sender's ephemeral socket is logged and discarded (the
/// `osc_sender` slot is set to `None` so subsequent emit calls
/// short-circuit). This mirrors the inbound best-effort posture.
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

    // --- Inbound adapter ---
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
    if settings.rx_enabled {
        let ip: IpAddr = settings
            .local_ip
            .parse()
            .map_err(|err| format!("invalid OSC listen IP {}: {err}", settings.local_ip))?;
        adapter
            .bind(SocketAddr::new(ip, settings.port))
            .map_err(|err| err.to_string())?;
    }
    drop(adapter);

    // --- Outbound sender (ADR-013 §1 / §6) ---
    configure_osc_sender(app, &settings);

    Ok(())
}

/// (Re-)create the outbound [`OscSender`] from the OSC settings
/// snapshot and install it into [`AppState::osc_sender`].
///
/// Best-effort: a bind failure for the sender's ephemeral local
/// socket is logged to stderr and the slot is set to `None`.
/// Callers must not treat a `None` slot as fatal — outbound sends
/// simply no-op until the operator's settings are valid.
fn configure_osc_sender(app: &AppState, settings: &chromaticity_runtime::settings::OscSettings) {
    if !settings.tx_enabled {
        // Operator has disabled outbound transmission. Install None
        // so subsequent emit calls short-circuit immediately.
        let mut guard = match app.osc_sender.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *guard = None;
        return;
    }
    let out_ip: IpAddr = match settings.out_host.parse() {
        Ok(ip) => ip,
        Err(err) => {
            eprintln!(
                "[chromaticity-shell] invalid OSC outbound host '{}': {err}; \
                 outbound OSC disabled",
                settings.out_host,
            );
            let mut guard = match app.osc_sender.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            *guard = None;
            return;
        }
    };
    let target = SocketAddr::new(out_ip, settings.out_port);
    let new_sender = match OscSender::with_target(target) {
        Ok(s) => Some(s),
        Err(err) => {
            eprintln!(
                "[chromaticity-shell] osc outbound sender bind failed for \
                 {target}: {err}; outbound OSC disabled",
            );
            None
        }
    };
    let mut guard = match app.osc_sender.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    *guard = new_sender;
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

/// Set the audio noise-floor gate threshold and emit an outbound OSC
/// announce on `/audio/threshold {f}`.
///
/// This is the direct-set path for the operator `Shift+T` hotkey ("set
/// audio threshold to current RMS level"). The value is clamped to
/// `[0.0, 1.0]` before storage; the clamped value is what gets
/// announced. The announce is best-effort (non-blocking UDP send).
///
/// PR9 Lane D — `OscDirection::Both` announce-on-change wire.
#[tauri::command]
pub fn set_audio_threshold(app_state: State<'_, AppState>, value: f32) -> Result<f32, String> {
    let stored = set_audio_threshold_impl(app_state.inner(), value);
    Ok(stored)
}

/// Inner implementation for [`set_audio_threshold`]; takes a borrowed
/// [`AppState`] directly so unit tests can exercise the seam without a
/// Tauri runtime.
pub(crate) fn set_audio_threshold_impl(app: &AppState, value: f32) -> f32 {
    apply_audio_threshold_and_announce(app, value);
    app.current_audio_threshold()
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
        // PR10: fresh adapter with no sender must report zero alias emits.
        assert_eq!(snapshot.compat_alias_emits, 0);
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

    // Evidence: EV-INTEGRATION-MATRIX (T1) — PR7 playlist position dispatch
    // through the shell command bus.

    #[test]
    fn dispatch_osc_playlist_set_position_commits_through_bus() {
        // `OscRuntimeCommand::PlaylistSetPosition { index }` must be
        // accepted by `runtime_command_from_osc` and committed through the
        // command bus so the runtime playlist cursor lands at the given
        // absolute index.
        let app = AppState::new();

        dispatch_osc_action_impl(
            &app,
            OscDispatchAction::RuntimeCommand(OscRuntimeCommand::PlaylistSetPosition { index: 4 }),
        )
        .expect("PlaylistSetPosition action must dispatch without error");

        let state = app.state.lock().expect("state lock");
        assert_eq!(
            state.playlist_position(),
            4,
            "playlist cursor must be set to absolute index 4",
        );
    }

    #[test]
    fn live_adapter_dispatches_playlist_position_from_udp() {
        // End-to-end: bind the position route, send a UDP packet to the
        // adapter, poll + drain, and assert the runtime cursor moved.
        let app = AppState::new();
        let bound = bind_app_osc_ephemeral(&app);
        bind_osc_route_impl(&app, "/playlists/position {i}", "playlist:position:2")
            .expect("position route binds");
        let sender = UdpSocket::bind("127.0.0.1:0").expect("sender binds");

        // `/playlists/position` takes no float args; trigger value is
        // irrelevant for a no-arg route — the position is encoded in the
        // bind target string.
        send_osc(&sender, bound, "/playlists/position", &[]);

        poll_and_drain_until(&app, || {
            let state = app.state.lock().expect("state lock");
            state.playlist_position() == 2
        });
    }

    #[test]
    fn dispatch_osc_scene_action_loads_scene_into_app_state() {
        let app = AppState::new();
        app.install_bundled_scene_root(fixture_scene_root());

        dispatch_osc_action_impl(
            &app,
            OscDispatchAction::SceneLoad {
                scene_id: "welcome".to_string(),
                preset: None,
            },
        )
        .expect("scene target loads");
        let guard = app.loaded_scene.lock().expect("loaded scene lock");
        let record = guard.as_ref().expect("scene loaded");
        assert!(record.path.ends_with("welcome/welcome.chroma"));
    }

    #[test]
    fn dispatch_osc_scene_action_with_preset_loads_scene_and_recalls_preset() {
        // Evidence: EV-INTEGRATION-MATRIX (T1) — scene two-arg dispatch in
        // the shell: scene loads successfully AND the preset is recalled.
        let app = AppState::new();
        app.install_bundled_scene_root(fixture_scene_root());

        dispatch_osc_action_impl(
            &app,
            OscDispatchAction::SceneLoad {
                scene_id: "welcome".to_string(),
                preset: Some(5),
            },
        )
        .expect("scene+preset action dispatches");

        let guard = app.loaded_scene.lock().expect("loaded scene lock");
        let record = guard.as_ref().expect("scene loaded");
        assert!(record.path.ends_with("welcome/welcome.chroma"));
        drop(guard);

        let state = app.state.lock().expect("state lock");
        assert_eq!(
            state.active_preset(),
            Some(chromaticity_runtime::PresetId(5)),
            "preset must be recalled after scene load",
        );
    }

    #[test]
    fn dispatch_osc_scene_action_scene_load_succeeds_even_if_preset_were_ignored() {
        // Evidence: EV-INTEGRATION-MATRIX (T1) — a scene load with
        // preset=None completes without error (backward-compat one-arg form).
        let app = AppState::new();
        app.install_bundled_scene_root(fixture_scene_root());

        dispatch_osc_action_impl(
            &app,
            OscDispatchAction::SceneLoad {
                scene_id: "welcome".to_string(),
                preset: None,
            },
        )
        .expect("one-arg scene action dispatches");

        let guard = app.loaded_scene.lock().expect("loaded scene lock");
        let record = guard.as_ref().expect("scene loaded");
        assert!(record.path.ends_with("welcome/welcome.chroma"));

        let state = app.state.lock().expect("state lock");
        assert_eq!(
            state.active_preset(),
            None,
            "no preset recalled when preset=None",
        );
    }

    // Evidence: EV-INTEGRATION-MATRIX (T1) — PR9 audio-threshold
    // bidirectionality wire pins (announce-on-change).

    #[test]
    fn dispatch_audio_threshold_action_stores_value_and_returns_ok() {
        // An inbound OscDispatchAction::AudioThreshold must be accepted by
        // dispatch_osc_action_impl without error.
        let app = AppState::new();
        dispatch_osc_action_impl(&app, OscDispatchAction::AudioThreshold { value: 0.25 })
            .expect("AudioThreshold action must dispatch without error");
        assert_eq!(
            app.current_audio_threshold(),
            0.25_f32,
            "threshold must be stored after dispatch",
        );
    }

    #[test]
    fn set_audio_threshold_impl_stores_and_returns_clamped_value() {
        // The Tauri command inner impl must clamp out-of-range inputs and
        // return the stored (clamped) value so callers can verify what was
        // persisted.
        let app = AppState::new();

        // Value in range — stored as-is.
        let returned = set_audio_threshold_impl(&app, 0.42);
        assert_eq!(
            returned.to_bits(),
            0.42_f32.to_bits(),
            "in-range value must round-trip bit-exactly",
        );
        assert_eq!(app.current_audio_threshold().to_bits(), 0.42_f32.to_bits());

        // Above range — clamped to 1.0.
        let returned = set_audio_threshold_impl(&app, 1.5);
        assert_eq!(
            returned, 1.0_f32,
            "above-range value must be clamped to 1.0"
        );
        assert_eq!(app.current_audio_threshold(), 1.0_f32);

        // Below range — clamped to 0.0.
        let returned = set_audio_threshold_impl(&app, -0.1);
        assert_eq!(
            returned, 0.0_f32,
            "below-range value must be clamped to 0.0"
        );
        assert_eq!(app.current_audio_threshold(), 0.0_f32);
    }

    #[test]
    fn inbound_threshold_update_triggers_exactly_one_outbound_announce() {
        // Wire a real receiver, install it as the sender target, dispatch
        // one AudioThreshold action, and assert exactly one UDP datagram
        // arrives carrying the threshold value as a float.
        let receiver = match std::net::UdpSocket::bind("127.0.0.1:0") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("skipping announce test; receiver bind unavailable: {e}");
                return;
            }
        };
        receiver
            .set_read_timeout(Some(std::time::Duration::from_secs(1)))
            .expect("set read timeout");
        let recv_addr = receiver.local_addr().expect("receiver local addr");

        let app = AppState::new();
        // Point the sender at our receiver.
        {
            let mut guard = app.osc_sender.lock().expect("osc sender lock");
            *guard = chromaticity_osc_adapter::sender::OscSender::with_target(recv_addr)
                .map(Some)
                .unwrap_or(None);
        }
        if app.osc_sender.lock().expect("osc sender lock").is_none() {
            eprintln!("skipping announce test; sender bind unavailable");
            return;
        }

        dispatch_osc_action_impl(&app, OscDispatchAction::AudioThreshold { value: 0.37 })
            .expect("dispatch must not error");

        // Receive the single datagram and verify it decodes as
        // `/audio/threshold` with a float argument equal to 0.37.
        let mut buf = [0u8; 256];
        let (len, _) = receiver
            .recv_from(&mut buf)
            .expect("receiver must get exactly one datagram");
        assert!(len > 0, "received datagram must be non-empty");

        // The datagram must decode as an OSC message with address
        // `/audio/threshold` and at least one float argument.
        let (_, packet) =
            rosc::decoder::decode_udp(&buf[..len]).expect("datagram must be a valid OSC packet");
        match packet {
            rosc::OscPacket::Message(msg) => {
                assert_eq!(
                    msg.addr, "/audio/threshold",
                    "announce address must be /audio/threshold"
                );
                assert_eq!(
                    msg.args.len(),
                    1,
                    "announce must carry exactly one argument"
                );
                match msg.args[0] {
                    rosc::OscType::Float(f) => {
                        assert_eq!(
                            f.to_bits(),
                            0.37_f32.to_bits(),
                            "announced float must be bit-identical to the inbound value",
                        );
                    }
                    ref other => panic!("expected float arg, got {other:?}"),
                }
            }
            other => panic!("expected OSC Message, got {other:?}"),
        }

        // No second datagram should arrive (non-blocking check).
        receiver
            .set_read_timeout(Some(std::time::Duration::from_millis(50)))
            .expect("set short timeout");
        let second = receiver.recv_from(&mut buf);
        assert!(
            second.is_err(),
            "exactly one announce must fire per change; got a second datagram: {second:?}",
        );
    }

    // Evidence: EV-NET-HARDENING / EV-INTEGRATION-MATRIX (T1) — PR10
    // compat-alias dual-emit shell plumbing pins.

    #[test]
    fn health_snapshot_compat_alias_emits_reflects_sender_counter() {
        // When an OscSender has been installed and alias emits have
        // occurred, osc_health_snapshot_impl must surface the counter
        // from the sender so the frontend health widget can show it.
        let app = AppState::new();

        // Install a sender pointed at a loopback receiver.
        let receiver = match std::net::UdpSocket::bind("127.0.0.1:0") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("skipping compat_alias_emits test; receiver bind unavailable: {e}");
                return;
            }
        };
        let recv_addr = receiver.local_addr().expect("receiver addr");
        {
            let mut guard = app.osc_sender.lock().expect("osc sender lock");
            match chromaticity_osc_adapter::sender::OscSender::with_target(recv_addr) {
                Ok(s) => *guard = Some(s),
                Err(e) => {
                    eprintln!("skipping compat_alias_emits test; sender bind failed: {e}");
                    return;
                }
            }
        }

        // Before any alias emit the counter must be zero.
        {
            let snapshot =
                osc_health_snapshot_impl(&app).expect("health snapshot reads adapter (pre)");
            assert_eq!(
                snapshot.compat_alias_emits, 0,
                "compat_alias_emits must be 0 before any alias send",
            );
        }

        // Emit one alias send directly through the sender.
        {
            let mut guard = app.osc_sender.lock().expect("osc sender lock");
            if let Some(sender) = guard.as_mut() {
                // Emit with an alias address; dual_emit_active=true so the
                // alias datagram fires and the counter increments.
                sender.emit_with_compat_alias("/scenes/demo", &[], Some("/scene/demo"), true);
            }
        }

        // Counter must now be 1.
        {
            let snapshot =
                osc_health_snapshot_impl(&app).expect("health snapshot reads adapter (post)");
            assert_eq!(
                snapshot.compat_alias_emits, 1,
                "compat_alias_emits must be 1 after one alias send",
            );
        }
    }

    #[test]
    fn dual_emit_active_returns_false_when_disabled() {
        // Evidence: EV-NET-HARDENING (T1) — dual_emit_active must return
        // false when dual_emit_enabled = false regardless of deadline.
        use chromaticity_runtime::settings::OscSettings;
        let settings = OscSettings {
            dual_emit_enabled: false,
            dual_emit_until: None,
            ..OscSettings::default()
        };
        assert!(
            !dual_emit_active(&settings),
            "dual_emit_active must be false when dual_emit_enabled=false",
        );
    }

    #[test]
    fn dual_emit_active_returns_true_when_enabled_no_deadline() {
        // Evidence: EV-NET-HARDENING (T1) — dual_emit_active must return
        // true when dual_emit_enabled = true and no deadline is set.
        use chromaticity_runtime::settings::OscSettings;
        let settings = OscSettings {
            dual_emit_enabled: true,
            dual_emit_until: None,
            ..OscSettings::default()
        };
        assert!(
            dual_emit_active(&settings),
            "dual_emit_active must be true when enabled with no deadline",
        );
    }

    #[test]
    fn dual_emit_active_returns_false_after_deadline_passed() {
        // Evidence: EV-NET-HARDENING (T1) — dual_emit_active must return
        // false when enabled but the deadline string is in the past.
        use chromaticity_runtime::settings::OscSettings;
        // Epoch start is definitely in the past.
        let settings = OscSettings {
            dual_emit_enabled: true,
            dual_emit_until: Some("1970-01-02T00:00:00Z".to_string()),
            ..OscSettings::default()
        };
        assert!(
            !dual_emit_active(&settings),
            "dual_emit_active must be false when deadline has passed",
        );
    }

    #[test]
    fn dual_emit_active_returns_true_before_deadline() {
        // Evidence: EV-NET-HARDENING (T1) — dual_emit_active must return
        // true when enabled and the deadline is far in the future.
        use chromaticity_runtime::settings::OscSettings;
        // Far-future deadline.
        let settings = OscSettings {
            dual_emit_enabled: true,
            dual_emit_until: Some("2099-01-01T00:00:00Z".to_string()),
            ..OscSettings::default()
        };
        assert!(
            dual_emit_active(&settings),
            "dual_emit_active must be true when deadline is far in the future",
        );
    }

    #[test]
    fn resolve_compat_alias_maps_scenes_prefix() {
        // Evidence: EV-INTEGRATION-MATRIX (T1) — concrete outbound
        // address `/scenes/demo` must resolve to the compat alias
        // `/scene/demo` from the seeded registry.
        let app = AppState::new();
        let alias = resolve_compat_alias("/scenes/demo", &app);
        assert_eq!(
            alias.as_deref(),
            Some("/scene/demo"),
            "scenes template must map to /scene/... alias",
        );
    }

    #[test]
    fn resolve_compat_alias_returns_none_for_no_alias_route() {
        // Evidence: EV-INTEGRATION-MATRIX (T1) — routes without a
        // compat_alias in the seed table must resolve to None.
        let app = AppState::new();
        // /playlists/position has no compat_alias in the seed table.
        let alias = resolve_compat_alias("/audio/threshold", &app);
        assert_eq!(
            alias, None,
            "/audio/threshold has no compat alias; must return None",
        );
    }

    #[test]
    fn chrono_free_now_rfc3339_format_is_valid_rfc3339() {
        // Evidence: EV-NET-HARDENING (T1) — the deadline-comparison
        // helper must produce a string in YYYY-MM-DDTHH:MM:SSZ format so
        // lexicographic comparison with operator-supplied deadlines is
        // well-defined.
        let now = chrono_free_now_rfc3339();
        assert_eq!(now.len(), 20, "RFC 3339 string must be 20 chars: {now}");
        assert!(
            now.chars().nth(4) == Some('-')
                && now.chars().nth(7) == Some('-')
                && now.chars().nth(10) == Some('T')
                && now.chars().nth(13) == Some(':')
                && now.chars().nth(16) == Some(':')
                && now.ends_with('Z'),
            "string must match YYYY-MM-DDTHH:MM:SSZ format: {now}",
        );
        // Year must be plausible (>=2026).
        let year: u32 = now[..4].parse().expect("year is numeric");
        assert!(year >= 2026, "year must be >= 2026 in test: {year}");
    }
}
