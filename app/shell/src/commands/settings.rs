// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Operator settings IPC handlers — Wave 2.1 Lane C.3 settings
//! persistence (`/Users/xaviermac/.claude/plans/inherited-percolating-quill.md`).
//!
//! Frontend → [`get_settings`] reads the current
//! [`chromaticity_runtime::settings::Settings`] document held behind
//! the [`crate::state::AppState::settings`] mutex and returns it as a
//! ts-rs [`SettingsPayload`]. Frontend → [`set_settings`] replaces
//! the in-memory document AND writes it through to the on-disk TOML
//! file at [`crate::state::AppState::settings_path`].
//!
//! Mirrors the `set_control` / `get_control` pattern from
//! [`crate::commands::control`]: thin Tauri wrappers over inner
//! `*_impl(...)` functions taking a borrowed [`AppState`] directly so
//! the round-trip glue is unit-testable without a Tauri runtime.
//!
//! Errors collapse to `Result<_, String>` so the Tauri serializer
//! carries a human-readable failure across the IPC boundary. The
//! engine-side [`chromaticity_runtime::settings::SettingsError`]
//! variants feed `Display` into the wrapper.

use chromaticity_ui_bridge::SettingsPayload;
use tauri::State;

use crate::state::AppState;

/// Inner implementation invoked by both the Tauri wrapper and unit
/// tests; takes a borrowed [`AppState`] directly.
fn get_settings_impl(app: &AppState) -> Result<SettingsPayload, String> {
    let guard = match app.settings.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    Ok(guard.clone().into())
}

/// Inner implementation invoked by both the Tauri wrapper and unit
/// tests; takes a borrowed [`AppState`] directly. Replaces the
/// in-memory settings document AND writes the on-disk TOML file
/// when [`AppState::settings_path`] is set. When the path is
/// unset (test contexts that don't bootstrap persistence), the
/// in-memory update still applies so unit tests don't need a
/// temp-file scaffold.
fn set_settings_impl(app: &AppState, settings: SettingsPayload) -> Result<(), String> {
    let engine: chromaticity_runtime::settings::Settings = settings.into();
    // Resolve the persistence target BEFORE acquiring the mutex so
    // the file I/O (which can be slow) doesn't hold the lock.
    let path_guard = match app.settings_path.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let path = path_guard.clone();
    drop(path_guard);
    if let Some(path) = path.as_ref() {
        engine.save_to_path(path).map_err(|err| err.to_string())?;
    }
    let mut guard = match app.settings.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    *guard = engine;
    Ok(())
}

/// Inner implementation invoked by both the Tauri wrapper and unit
/// tests; takes a borrowed [`AppState`] directly. Replaces the
/// in-memory settings document with
/// [`chromaticity_runtime::settings::Settings::default`] AND writes
/// the default document through to the on-disk TOML file (when
/// [`AppState::settings_path`] is set). Returns the resulting
/// [`SettingsPayload`] so the frontend can hydrate its store from
/// the same canonical shape the engine just persisted.
fn reset_settings_impl(app: &AppState) -> Result<SettingsPayload, String> {
    let defaults = chromaticity_runtime::settings::Settings::default();
    // Resolve the persistence target BEFORE acquiring the mutex so
    // the file I/O (which can be slow) doesn't hold the lock — same
    // ordering as `set_settings_impl` above.
    let path_guard = match app.settings_path.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let path = path_guard.clone();
    drop(path_guard);
    if let Some(path) = path.as_ref() {
        defaults.save_to_path(path).map_err(|err| err.to_string())?;
    }
    let mut guard = match app.settings.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    *guard = defaults.clone();
    Ok(defaults.into())
}

/// Read the current persisted operator settings document.
///
/// Wave 2.1 Lane C.3: returns the in-memory
/// [`chromaticity_runtime::settings::Settings`] held on
/// [`AppState::settings`] as a ts-rs [`SettingsPayload`]. The
/// document is loaded from disk at app bootstrap (or falls back to
/// [`chromaticity_runtime::settings::Settings::default`] when no
/// file yet exists).
#[tauri::command]
pub fn get_settings(app_state: State<'_, AppState>) -> Result<SettingsPayload, String> {
    get_settings_impl(app_state.inner())
}

/// Replace the persisted operator settings document.
///
/// Wave 2.1 Lane C.3: write-through pattern — the in-memory mutex
/// is updated AND the file at [`AppState::settings_path`] is
/// overwritten. The frontend `settingsStore` debounces calls to this
/// handler so rapid form edits don't thrash disk I/O.
#[tauri::command]
pub fn set_settings(
    app_state: State<'_, AppState>,
    settings: SettingsPayload,
) -> Result<(), String> {
    set_settings_impl(app_state.inner(), settings)
}

/// Reset persisted operator settings to
/// [`chromaticity_runtime::settings::Settings::default`].
///
/// Wave 2.7 Lane SET.D: surfaced through the Settings view's
/// "Reset to Defaults" button (with operator-confirmation modal).
/// Same write-through semantics as [`set_settings`]: the in-memory
/// mutex is swapped to the default document AND the on-disk TOML
/// file at [`AppState::settings_path`] is rewritten with the
/// default document so the reset survives restart. Returns the
/// resulting [`SettingsPayload`] so the frontend store can hydrate
/// without a follow-on `get_settings` round-trip.
#[tauri::command]
pub fn reset_settings(app_state: State<'_, AppState>) -> Result<SettingsPayload, String> {
    reset_settings_impl(app_state.inner())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Unit tests for the Wave 2.1 Lane C.3 settings-persistence
    //! handler glue. The engine-side
    //! `Settings::load_from_path` / `save_to_path` round-trip is
    //! covered by the `engine/runtime/src/settings.rs` T1 pins;
    //! these tests pin the SHELL-side
    //! `Mutex::lock → save_to_path → in-memory swap` chain that is
    //! otherwise only exercised end-to-end through Tauri.

    use super::{get_settings_impl, reset_settings_impl, set_settings_impl};
    use crate::state::AppState;
    use chromaticity_runtime::settings::Theme;
    use chromaticity_ui_bridge::{
        AudioSettingsPayload, GeneralSettingsPayload, GraphicsSettingsPayload, OscSettingsPayload,
        SettingsPayload, StorageSettingsPayload, ThemePayload,
    };

    fn nondefault_payload() -> SettingsPayload {
        SettingsPayload {
            general: GeneralSettingsPayload {
                theme: ThemePayload::Console,
            },
            audio: AudioSettingsPayload {
                input_device: "iface".to_string(),
                output_device: "loopback".to_string(),
                sample_rate_hz: 88_200,
                latency_hint: "low".to_string(),
                channel_layout: "mono".to_string(),
                channel_left: "L5".to_string(),
                channel_right: "R6".to_string(),
                input_gain: 0.42,
            },
            graphics: GraphicsSettingsPayload {
                resolution_preset: "1440p".to_string(),
                custom_resolution_enabled: false,
                custom_width: 1920,
                custom_height: 1080,
                fps_target: "144".to_string(),
                vsync: false,
                hdr_target: "sdr".to_string(),
                display: "display-1".to_string(),
                color_space: "rec709".to_string(),
                multi_display_mode: "mirror".to_string(),
                render_scale: 0.75,
                screen_fitting: "stretch".to_string(),
                fps_slider_enabled: true,
            },
            storage: StorageSettingsPayload {
                media_folders: vec!["/tmp/media".to_string()],
                scene_folders: vec![],
                backup_path: "/tmp/backups".to_string(),
                backup_retention: "retain-5".to_string(),
            },
            osc: OscSettingsPayload {
                local_ip: "192.168.1.42".to_string(),
                port: 9100,
                tx_enabled: false,
                rx_enabled: true,
            },
        }
    }

    #[test]
    fn get_settings_returns_default_for_fresh_app_state() {
        let app = AppState::new();
        let payload = get_settings_impl(&app).unwrap();
        // Fresh AppState defaults to Theme::Pulse via Settings::default().
        assert_eq!(payload.general.theme, ThemePayload::Pulse);
        assert_eq!(payload.audio.sample_rate_hz, 48_000);
        assert_eq!(payload.graphics.resolution_preset, "1080p");
    }

    #[test]
    fn set_settings_then_get_settings_round_trip_in_memory() {
        // No persistence path installed: the write-through file
        // I/O is skipped, but the in-memory mutex still updates.
        let app = AppState::new();
        let payload = nondefault_payload();
        set_settings_impl(&app, payload.clone()).unwrap();
        let back = get_settings_impl(&app).unwrap();
        assert_eq!(back, payload);
    }

    #[test]
    fn set_settings_writes_through_to_disk_when_path_is_installed() {
        let app = AppState::new();
        // Use a temp file unique to this test invocation.
        let mut path = std::env::temp_dir();
        path.push(format!(
            "chromaticity-set-settings-handler-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        app.install_settings_path(path.clone());

        let payload = nondefault_payload();
        set_settings_impl(&app, payload.clone()).unwrap();

        // File now exists on disk; reload through the engine and
        // confirm the contents match.
        let loaded = chromaticity_runtime::settings::Settings::load_from_path(&path).unwrap();
        let loaded_payload: SettingsPayload = loaded.into();
        assert_eq!(loaded_payload, payload);
        // Spot-check a couple of fields through the engine enum
        // discriminant directly to pin the ThemePayload ↔ Theme
        // translation.
        let reload = chromaticity_runtime::settings::Settings::load_from_path(&path).unwrap();
        assert_eq!(reload.general.theme, Theme::Console);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn reset_settings_swaps_in_memory_document_to_default() {
        // Wave 2.7 SET.D: with no on-disk path installed (test
        // context) the reset is in-memory-only — the mutex is
        // swapped to `Settings::default()` and the returned payload
        // matches the default shape.
        let app = AppState::new();
        // Seed a non-default document so the swap is observable.
        let mut document = chromaticity_runtime::settings::Settings::default();
        document.general.theme = Theme::Console;
        document.audio.sample_rate_hz = 96_000;
        app.install_settings(document);

        let returned = reset_settings_impl(&app).unwrap();
        assert_eq!(returned.general.theme, ThemePayload::Pulse);
        assert_eq!(returned.audio.sample_rate_hz, 48_000);
        // And the in-memory mutex now reads back as default.
        let after = get_settings_impl(&app).unwrap();
        assert_eq!(after, returned);
    }

    #[test]
    fn reset_settings_writes_default_through_to_disk_when_path_is_installed() {
        // Wave 2.7 SET.D: when a path is installed, the reset
        // overwrites the on-disk TOML file with the default
        // document so the reset survives restart. Reload from disk
        // through the engine and confirm the file contents match
        // `Settings::default()`.
        let app = AppState::new();
        let mut path = std::env::temp_dir();
        path.push(format!(
            "chromaticity-reset-settings-handler-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        app.install_settings_path(path.clone());

        // First, persist a non-default document so the file exists
        // with operator state.
        let payload = nondefault_payload();
        set_settings_impl(&app, payload).unwrap();

        // Now reset and confirm the file contents flipped back to
        // the default shape.
        let returned = reset_settings_impl(&app).unwrap();
        let loaded = chromaticity_runtime::settings::Settings::load_from_path(&path).unwrap();
        assert_eq!(loaded, chromaticity_runtime::settings::Settings::default());
        let loaded_payload: SettingsPayload = loaded.into();
        assert_eq!(loaded_payload, returned);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn get_settings_after_external_write_through_install_settings() {
        // Models the bootstrap flow: shell installs an already-loaded
        // settings document (e.g. from `Settings::load_or_default`)
        // before any IPC fires. `get_settings` returns whatever is in
        // the mutex without doing any I/O.
        let app = AppState::new();
        let mut document = chromaticity_runtime::settings::Settings::default();
        document.general.theme = Theme::Atelier;
        document.audio.sample_rate_hz = 96_000;
        app.install_settings(document);

        let payload = get_settings_impl(&app).unwrap();
        assert_eq!(payload.general.theme, ThemePayload::Atelier);
        assert_eq!(payload.audio.sample_rate_hz, 96_000);
    }
}
