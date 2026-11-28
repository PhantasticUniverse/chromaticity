// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Native OS folder-picker IPC handler backed by `tauri-plugin-dialog`.
//!
//! Contract:
//! - `pick_folder(title: String) -> Result<Option<String>, String>`
//! - `Ok(Some(path))` — user accepted the dialog; returns the absolute folder path.
//! - `Ok(None)` — user dismissed / cancelled the dialog.
//! - `Err(message)` — platform error.

use tauri_plugin_dialog::DialogExt;

#[cfg(test)]
fn fallback_media_dir() -> String {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Movies")
        .join("Chromaticity")
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
fn pick_folder_impl(_title: &str) -> Result<Option<String>, String> {
    Ok(Some(fallback_media_dir()))
}

/// Open a native OS folder picker titled `title` via `tauri-plugin-dialog` and
/// return the chosen directory path, or `None` if the user cancelled.
#[tauri::command]
pub fn pick_folder(app: tauri::AppHandle, title: String) -> Result<Option<String>, String> {
    let picked = app.dialog().file().set_title(&title).blocking_pick_folder();
    Ok(picked
        .and_then(|fp| fp.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned()))
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Pins the W21 PR1 contract shape: the stub must surface a
    //! `Some(path)` result so the frontend IPC wrapper can resolve to a
    //! non-null string for the Settings Storage section's optimistic
    //! state push. The Phase-3+/4a swap-in will widen the test set with
    //! a `None`-return cancel path.
    use super::pick_folder_impl;

    #[test]
    fn pick_folder_stub_returns_some_path() {
        let result = pick_folder_impl("Pick a Media folder").expect("stub must succeed");
        let path = result.expect("stub must return Some(path), not None");
        assert!(!path.is_empty(), "stub path must be non-empty");
        assert!(path.starts_with('/'), "stub path must be absolute");
    }

    #[test]
    fn pick_folder_stub_ignores_title_argument() {
        // The placeholder is title-independent; the contract still
        // takes `title` so the Phase-3+/4a swap-in can plumb it
        // straight into the dialog plugin's `set_title` builder.
        let a = pick_folder_impl("Pick a Media folder").unwrap();
        let b = pick_folder_impl("Pick a Backup folder").unwrap();
        assert_eq!(a, b);
    }
}
