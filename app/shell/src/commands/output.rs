// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Dedicated secondary-display / fullscreen stage output window commands.

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Stable Tauri window label for the Chromaticity stage output window.
pub const OUTPUT_WINDOW_LABEL: &str = "chromaticity-output";

/// Serializable status snapshot of the stage output window.
#[derive(Debug, Clone, Serialize)]
pub struct OutputWindowStatus {
    pub open: bool,
    pub fullscreen: bool,
    pub width: u32,
    pub height: u32,
}

/// Serializable monitor descriptor for output routing selectors.
#[derive(Debug, Clone, Serialize)]
pub struct MonitorSummary {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub is_primary: bool,
}

/// Enumerate attached OS displays for stage output routing.
#[tauri::command]
pub fn list_output_monitors(app: AppHandle) -> Result<Vec<MonitorSummary>, String> {
    let monitors = app
        .available_monitors()
        .map_err(|err| format!("failed to query monitors: {err}"))?;
    let primary_name = app
        .primary_monitor()
        .ok()
        .flatten()
        .and_then(|m| m.name().cloned());

    Ok(monitors
        .into_iter()
        .enumerate()
        .map(|(index, m)| {
            let name = m
                .name()
                .cloned()
                .unwrap_or_else(|| format!("Display {}", index + 1));
            let is_primary = primary_name.as_deref() == Some(name.as_str()) || index == 0;
            MonitorSummary {
                index,
                name,
                width: m.size().width,
                height: m.size().height,
                scale_factor: m.scale_factor(),
                is_primary,
            }
        })
        .collect())
}

/// Open (or focus) the dedicated Chromaticity stage output window.
///
/// When `monitor_index` is omitted and a secondary display is connected,
/// automatically places the output window on the secondary display.
#[tauri::command]
pub fn open_output_window(
    app: AppHandle,
    fullscreen: Option<bool>,
    monitor_index: Option<usize>,
) -> Result<OutputWindowStatus, String> {
    let want_fullscreen = fullscreen.unwrap_or(false);
    if let Some(existing) = app.get_webview_window(OUTPUT_WINDOW_LABEL) {
        let _ = existing.set_fullscreen(want_fullscreen);
        let _ = existing.set_focus();
        return window_status(&existing);
    }

    let monitors = app.available_monitors().unwrap_or_default();
    let chosen_monitor = monitor_index
        .and_then(|idx| monitors.get(idx))
        .or_else(|| monitors.get(1))
        .or_else(|| monitors.first());

    let mut builder = WebviewWindowBuilder::new(
        &app,
        OUTPUT_WINDOW_LABEL,
        WebviewUrl::App("index.html?mode=output".into()),
    )
    .title("Chromaticity Output")
    .decorations(!want_fullscreen)
    .fullscreen(want_fullscreen)
    .inner_size(1280.0, 720.0);

    if let Some(mon) = chosen_monitor {
        let pos = mon.position();
        builder = builder.position(f64::from(pos.x), f64::from(pos.y));
    }

    let window = builder
        .build()
        .map_err(|err| format!("failed to open output window: {err}"))?;
    window_status(&window)
}

/// Close the stage output window if open.
#[tauri::command]
pub fn close_output_window(app: AppHandle) -> Result<OutputWindowStatus, String> {
    if let Some(window) = app.get_webview_window(OUTPUT_WINDOW_LABEL) {
        window
            .close()
            .map_err(|err| format!("failed to close output window: {err}"))?;
    }
    Ok(OutputWindowStatus {
        open: false,
        fullscreen: false,
        width: 0,
        height: 0,
    })
}

/// Toggle fullscreen state on the open stage output window.
#[tauri::command]
pub fn toggle_output_fullscreen(app: AppHandle) -> Result<OutputWindowStatus, String> {
    let Some(window) = app.get_webview_window(OUTPUT_WINDOW_LABEL) else {
        return open_output_window(app, Some(true), None);
    };
    let current = window.is_fullscreen().unwrap_or(false);
    window
        .set_fullscreen(!current)
        .map_err(|err| format!("failed to toggle output fullscreen: {err}"))?;
    window_status(&window)
}

/// Query current stage output window status.
#[tauri::command]
pub fn output_window_status(app: AppHandle) -> Result<OutputWindowStatus, String> {
    match app.get_webview_window(OUTPUT_WINDOW_LABEL) {
        Some(window) => window_status(&window),
        None => Ok(OutputWindowStatus {
            open: false,
            fullscreen: false,
            width: 0,
            height: 0,
        }),
    }
}

fn window_status(window: &tauri::WebviewWindow) -> Result<OutputWindowStatus, String> {
    let fullscreen = window.is_fullscreen().unwrap_or(false);
    let size = window.inner_size().unwrap_or(tauri::PhysicalSize {
        width: 0,
        height: 0,
    });
    Ok(OutputWindowStatus {
        open: true,
        fullscreen,
        width: size.width,
        height: size.height,
    })
}
