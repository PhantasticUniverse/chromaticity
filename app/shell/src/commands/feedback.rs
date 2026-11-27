// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Disk persistence for operator scene and preset feedback (`scene-feedback.json`).
//!
//! Stores a validated, deterministically pretty-printed JSON document under
//! `app.path().app_data_dir()` via atomic temp-file-and-rename writes so
//! concurrent readers or crashes never observe a truncated document.

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::Manager;

/// File name (relative to `app.path().app_data_dir()`) for persisted scene/preset feedback.
const FEEDBACK_FILE_NAME: &str = "scene-feedback.json";

/// Monotonic counter used to disambiguate atomic temporary files within a process.
static TMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Load the raw JSON string from `<data_dir>/scene-feedback.json`.
///
/// Returns `"{}"` when the file does not exist yet.
pub fn load_scene_feedback_impl(data_dir: &Path) -> Result<String, String> {
    let target_path = data_dir.join(FEEDBACK_FILE_NAME);
    match std::fs::read_to_string(&target_path) {
        Ok(contents) => Ok(contents),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok("{}".to_string()),
        Err(err) => Err(format!(
            "failed to read scene feedback from {}: {err}",
            target_path.display()
        )),
    }
}

/// Validate, deterministically pretty-print, and atomically write `json` to
/// `<data_dir>/scene-feedback.json`.
///
/// Returns the written file path as a string so operators and external tools
/// know the exact location on disk.
pub fn save_scene_feedback_impl(data_dir: &Path, json: &str) -> Result<String, String> {
    let parsed: serde_json::Value =
        serde_json::from_str(json).map_err(|err| format!("invalid scene feedback JSON: {err}"))?;
    let mut formatted = serde_json::to_string_pretty(&parsed)
        .map_err(|err| format!("failed to format scene feedback JSON: {err}"))?;
    formatted.push('\n');

    std::fs::create_dir_all(data_dir).map_err(|err| {
        format!(
            "failed to create scene feedback directory {}: {err}",
            data_dir.display()
        )
    })?;

    let target_path = data_dir.join(FEEDBACK_FILE_NAME);
    let seq = TMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_name = format!(".{FEEDBACK_FILE_NAME}.{}.{seq}.tmp", std::process::id());
    let tmp_path = data_dir.join(tmp_name);

    let write_result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(formatted.as_bytes())?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(err) = write_result {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!(
            "failed to write temporary scene feedback file {}: {err}",
            tmp_path.display()
        ));
    }

    if let Err(err) = std::fs::rename(&tmp_path, &target_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!(
            "failed to commit scene feedback file {}: {err}",
            target_path.display()
        ));
    }

    Ok(target_path.to_string_lossy().into_owned())
}

/// Load persisted scene/preset feedback JSON from the application data directory.
#[tauri::command]
pub fn load_scene_feedback(app: tauri::AppHandle) -> Result<String, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("failed to resolve app data directory: {err}"))?;
    load_scene_feedback_impl(&data_dir)
}

/// Validate and atomically save scene/preset feedback JSON to the application
/// data directory, returning the full written file path.
#[tauri::command]
pub fn save_scene_feedback(app: tauri::AppHandle, json: String) -> Result<String, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("failed to resolve app data directory: {err}"))?;
    save_scene_feedback_impl(&data_dir, &json)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_temp_dir(label: &str) -> PathBuf {
        let seq = TMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "chromaticity-feedback-test-{label}-{}-{seq}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn load_returns_empty_object_when_file_missing() {
        let dir = make_temp_dir("missing");
        let loaded = load_scene_feedback_impl(&dir).expect("load missing file");
        assert_eq!(loaded, "{}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_and_load_round_trip_is_deterministic_and_atomic() {
        let dir = make_temp_dir("roundtrip");
        let input = r#"{"z_note":"great bloom","a_rating":5,"tags":["neon","audio"]}"#;

        let written_path = save_scene_feedback_impl(&dir, input).expect("save valid json");
        let expected_path = dir.join(FEEDBACK_FILE_NAME);
        assert_eq!(PathBuf::from(&written_path), expected_path);
        assert!(expected_path.is_file());

        let loaded = load_scene_feedback_impl(&dir).expect("load saved json");
        assert!(loaded.ends_with('\n'));
        assert!(!loaded.contains('\r'));

        let expected_val: serde_json::Value = serde_json::from_str(input).expect("parse input");
        let loaded_val: serde_json::Value = serde_json::from_str(&loaded).expect("parse loaded");
        assert_eq!(loaded_val, expected_val);

        // Second save overwrites cleanly and leaves no lingering temp files.
        let updated = r#"{"a_rating":4}"#;
        save_scene_feedback_impl(&dir, updated).expect("overwrite json");
        let loaded_updated = load_scene_feedback_impl(&dir).expect("load overwritten json");
        let updated_val: serde_json::Value =
            serde_json::from_str(&loaded_updated).expect("parse overwritten");
        assert_eq!(updated_val["a_rating"], 4);

        let entries: Vec<_> = std::fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .collect();
        assert_eq!(entries.len(), 1, "no temporary files should remain");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_rejects_malformed_json_without_writing() {
        let dir = make_temp_dir("malformed");
        let err = save_scene_feedback_impl(&dir, "{ not valid json")
            .expect_err("malformed json must be rejected");
        assert!(err.contains("invalid scene feedback JSON"));
        assert!(!dir.join(FEEDBACK_FILE_NAME).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
