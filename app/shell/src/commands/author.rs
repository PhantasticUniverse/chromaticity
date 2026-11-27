// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Safe `.chroma` scene package file read/write Tauri commands for `Author.svelte`.
//!
//! Strictly restricts file names to the canonical allowlist (`main.glsl`,
//! `scene.json`, `script.js`) inside a verified `.chroma` directory so path
//! traversal or arbitrary filesystem writes are impossible by construction.

use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Allowlisted editable filenames inside a `.chroma` package.
const ALLOWED_SCENE_FILES: &[&str] = &["main.glsl", "scene.json", "script.js"];

/// Bundle of editable text files read from a `.chroma` directory.
#[derive(Debug, Clone, Serialize)]
pub struct SceneFilesBundle {
    pub main_glsl: Option<String>,
    pub scene_json: Option<String>,
    pub script_js: Option<String>,
}

/// Read `main.glsl`, `scene.json`, and `script.js` from a `.chroma` directory.
#[tauri::command]
pub fn read_scene_files(scene_path: String) -> Result<SceneFilesBundle, String> {
    read_scene_files_impl(Path::new(&scene_path))
}

/// Atomically write an allowlisted file (`main.glsl`, `scene.json`, or
/// `script.js`) inside a `.chroma` directory.
#[tauri::command]
pub fn write_scene_file(
    scene_path: String,
    file_name: String,
    content: String,
) -> Result<String, String> {
    write_scene_file_impl(Path::new(&scene_path), &file_name, &content)
}

pub(crate) fn read_scene_files_impl(scene_dir: &Path) -> Result<SceneFilesBundle, String> {
    validate_chroma_dir(scene_dir)?;
    Ok(SceneFilesBundle {
        main_glsl: read_optional_file(&scene_dir.join("main.glsl"))?,
        scene_json: read_optional_file(&scene_dir.join("scene.json"))?,
        script_js: read_optional_file(&scene_dir.join("script.js"))?,
    })
}

pub(crate) fn write_scene_file_impl(
    scene_dir: &Path,
    file_name: &str,
    content: &str,
) -> Result<String, String> {
    validate_chroma_dir(scene_dir)?;
    if !ALLOWED_SCENE_FILES.contains(&file_name) {
        return Err(format!(
            "unsupported scene file '{file_name}' (allowed: main.glsl, scene.json, script.js)"
        ));
    }
    if file_name == "scene.json" {
        serde_json::from_str::<serde_json::Value>(content)
            .map_err(|err| format!("refusing to write invalid scene.json: {err}"))?;
    }

    let target: PathBuf = scene_dir.join(file_name);
    let tmp: PathBuf = scene_dir.join(format!(".{file_name}.tmp"));
    let normalized = content.replace("\r\n", "\n");

    let mut file = std::fs::File::create(&tmp)
        .map_err(|err| format!("failed to create temp file {}: {err}", tmp.display()))?;
    file.write_all(normalized.as_bytes())
        .map_err(|err| format!("failed to write temp file {}: {err}", tmp.display()))?;
    file.sync_all()
        .map_err(|err| format!("failed to sync temp file {}: {err}", tmp.display()))?;
    drop(file);

    std::fs::rename(&tmp, &target).map_err(|err| {
        let _ = std::fs::remove_file(&tmp);
        format!("failed to commit {}: {err}", target.display())
    })?;

    Ok(target.to_string_lossy().into_owned())
}

fn validate_chroma_dir(scene_dir: &Path) -> Result<(), String> {
    if !scene_dir.is_dir() {
        return Err(format!(
            "scene directory does not exist: {}",
            scene_dir.display()
        ));
    }
    let ext = scene_dir.extension().and_then(|s| s.to_str());
    if ext != Some("chroma") {
        return Err(format!(
            "target path must be a .chroma package directory: {}",
            scene_dir.display()
        ));
    }
    Ok(())
}

fn read_optional_file(path: &Path) -> Result<Option<String>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    std::fs::read_to_string(path)
        .map(Some)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod author_command_pins {
    use super::*;

    #[test]
    fn write_and_read_scene_files_round_trip_and_reject_traversal() {
        let dir = std::env::temp_dir().join("chromaticity-author-io-test.chroma");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let written = write_scene_file_impl(&dir, "main.glsl", "#version 450\nvoid main(){}\n")
            .expect("main.glsl write should succeed");
        assert!(written.ends_with("main.glsl"));

        // Reject invalid scene.json
        assert!(write_scene_file_impl(&dir, "scene.json", "{not json").is_err());
        // Reject path traversal / disallowed file
        assert!(write_scene_file_impl(&dir, "../escape.txt", "nope").is_err());

        let bundle = read_scene_files_impl(&dir).expect("read should succeed");
        assert!(bundle.main_glsl.unwrap().contains("#version 450"));
        assert!(bundle.scene_json.is_none());
        assert!(bundle.script_js.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
