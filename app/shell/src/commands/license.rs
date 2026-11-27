// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! License & entitlement IPC handlers wired to `chromaticity-entitlement`
//! (ADR-009 capability bitmask + expiry enforcement).

use crate::state::AppState;
use chromaticity_entitlement::{
    Clock, FREE_CAPABILITIES, HONORED_TOKEN_CAPABILITIES, SystemClock, Token, entitled, is_valid_at,
};
use serde::Serialize;
use tauri::State;

/// Serializable entitlement status snapshot for the UI License view and
/// `capabilitiesStore`.
#[derive(Debug, Clone, Serialize)]
pub struct EntitlementStatusPayload {
    pub state: &'static str,
    pub watermark_off: bool,
    pub granted_capabilities: Vec<String>,
}

/// Install a JSON-encoded `entitlement-v2` token into `AppState`.
///
/// Validates JSON shape and expiry against `SystemClock` before storing.
/// When invoked with `None` (legacy stub caller without a token string),
/// returns a descriptive error instead of silently succeeding.
#[tauri::command]
pub fn install_license(
    app_state: State<'_, AppState>,
    token_json: Option<String>,
) -> Result<(), String> {
    let raw = token_json.ok_or_else(|| "no entitlement token supplied".to_string())?;
    let token: Token =
        serde_json::from_str(&raw).map_err(|err| format!("invalid entitlement token: {err}"))?;
    let now = SystemClock.now();
    if !is_valid_at(&token, now) {
        return Err("entitlement token is expired or not yet valid".to_string());
    }
    let mut guard = match app_state.entitlement_token.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    *guard = Some(token);
    Ok(())
}

/// Remove the active entitlement token, reverting the runtime to free-tier
/// capabilities (watermark-on).
#[tauri::command]
pub fn remove_license(app_state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = match app_state.entitlement_token.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    *guard = None;
    Ok(())
}

/// Request a trial token (commerce endpoint deferred post-MVP per ADR-009).
#[tauri::command]
pub fn request_trial() -> Result<(), String> {
    Err("online trial issuance requires the post-MVP activation service".to_string())
}

/// Query the live evaluated entitlement status and granted capabilities
/// against `chromaticity-entitlement` (`HONORED_TOKEN_CAPABILITIES` +
/// `FREE_CAPABILITIES`).
#[tauri::command]
pub fn get_entitlement_status(
    app_state: State<'_, AppState>,
) -> Result<EntitlementStatusPayload, String> {
    let guard = match app_state.entitlement_token.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let now = SystemClock.now();
    let token_ref = guard.as_ref();
    let mut granted: Vec<String> = FREE_CAPABILITIES.iter().map(|c| (*c).to_string()).collect();
    for cap in HONORED_TOKEN_CAPABILITIES {
        if entitled(token_ref, cap, now) && !granted.iter().any(|g| g == cap) {
            granted.push((*cap).to_string());
        }
    }
    granted.sort();
    let watermark_off = entitled(token_ref, "render.watermark.off", now);
    let state = match token_ref {
        Some(t) if is_valid_at(t, now) => "licensed",
        Some(_) => "expired",
        None => "unlicensed",
    };
    Ok(EntitlementStatusPayload {
        state,
        watermark_off,
        granted_capabilities: granted,
    })
}
