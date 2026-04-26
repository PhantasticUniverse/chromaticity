// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Folder-picker IPC stub — Wave-21 PR1 Settings expansion.
//!
//! Defines the `pick_folder` Tauri command contract surface that the
//! W21 PR1 Settings → Storage section's "Browse" buttons invoke. The
//! wireframe-aligned UI surface is the deliverable here; the native
//! folder-picker dialog wiring is deferred to the Phase-3+/4a
//! `EV-INTEGRATION-MATRIX` slice per `docs/26` because the dialog
//! plugin (`tauri-plugin-dialog`) is not yet on the dependency
//! graph and the capabilities allowlist hasn't been opened to it.
//!
//! Contract:
//! - `pick_folder(title: String) -> Result<Option<String>, String>`
//! - `Ok(Some(path))` — user accepted the dialog; returns the absolute
//!   folder path.
//! - `Ok(None)` — user dismissed / cancelled the dialog.
//! - `Err(message)` — platform error (permission denied, plugin
//!   unavailable, etc.).
//!
//! W21 PR1 stub returns a deterministic placeholder
//! (`/Users/operator/Movies/Chromaticity/Media`) so the frontend IPC
//! plumbing — `invoke('pick_folder', { title })` → typed
//! `Promise<string | null>` wrapper in `src/lib/ipc/picker.ts` →
//! Storage section button → state push — exercises the full round
//! trip without the platform-dependent dialog. Phase-3+/4a will swap
//! the placeholder for `tauri_plugin_dialog::DialogExt::dialog().pick_folder()`.

/// Phase-3+/4a `EV-INTEGRATION-MATRIX` swap-in target: replace this
/// placeholder with `tauri_plugin_dialog::DialogExt::dialog().pick_folder()`
/// once the dialog plugin lands on the dependency graph and the
/// capabilities allowlist opens to it.
const PLACEHOLDER_PICKED_PATH: &str = "/Users/operator/Movies/Chromaticity/Media";

fn pick_folder_impl(_title: &str) -> Result<Option<String>, String> {
    // TODO(Phase-3+/4a `EV-INTEGRATION-MATRIX`): replace this stub with
    // a `tauri_plugin_dialog`-backed native folder picker. The contract
    // shape (`Result<Option<String>, String>`, `Ok(None)` for cancel)
    // is intentionally final so the swap is a pure backend change with
    // no frontend ripple.
    Ok(Some(PLACEHOLDER_PICKED_PATH.to_string()))
}

/// Open a native folder picker titled `title` and return the chosen
/// path, or `None` if the user cancelled.
///
/// W21 PR1 stub: returns a deterministic placeholder path so the
/// Settings → Storage section's "Browse" buttons exercise the full
/// IPC round-trip. Native-dialog hookup arrives in Phase-3+/4a per
/// the module-level prose.
#[tauri::command]
pub fn pick_folder(title: String) -> Result<Option<String>, String> {
    pick_folder_impl(&title)
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
