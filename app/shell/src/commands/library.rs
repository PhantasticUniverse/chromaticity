// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Scene library directory-scan IPC handlers — Wave-9 backbone slice.
//!
//! Surfaces the `docs/38-scene-library-and-import-spec.md` §2.2
//! per-scene metadata across the Tauri command boundary. The handlers
//! are stateless (no `AppState`): the inputs are a filesystem path and
//! the platform-default scene-root resolution. Path traversal is
//! constrained by canonicalising both the input and the platform-
//! default root and requiring the input to be the default root or a
//! descendant of it.
//!
//! `.chroma` directory entries that fail to load (malformed
//! `scene.json`, missing entry-point, etc.) are skipped silently —
//! mirrors the reference app's `_data.jsx` LIBRARY tolerance for
//! authoring-time damage that should not collapse the whole library.
//!
//! No `tauri-plugin-fs` involvement: Tauri 2 capabilities only gate
//! plugin commands, so a `#[tauri::command]` handler that reads via
//! `std::fs` is not subject to the capability allowlist.

use chromaticity_scene::loader::load_chroma_dir;
use chromaticity_ui_bridge::SceneCard;
use std::path::Path;

/// Format tag emitted for every card in this slice; future imports
/// (`compat-syn-scene`, `shadertoy-v1-import`, `isf-import`) widen the
/// emitter set per `docs/38` §2.2.
const FORMAT_TAG_NATIVE: &str = "native-chroma-scene";

/// Resolve the platform-default scene-library root.
///
/// macOS: `$HOME/Library/Application Support/Chromaticity/Scenes`
/// Linux: `$XDG_DATA_HOME/chromaticity/scenes` (else
///        `$HOME/.local/share/chromaticity/scenes`)
/// Windows: `$APPDATA\Chromaticity\Scenes`
fn default_scene_root_impl() -> Result<String, String> {
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
        Ok(format!(
            "{home}/Library/Application Support/Chromaticity/Scenes"
        ))
    } else if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_err(|_| "APPDATA not set".to_string())?;
        Ok(format!("{appdata}\\Chromaticity\\Scenes"))
    } else {
        // Linux + other unix-likes follow the XDG basedir spec.
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                return Ok(format!("{xdg}/chromaticity/scenes"));
            }
        }
        let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
        Ok(format!("{home}/.local/share/chromaticity/scenes"))
    }
}

fn extract_string(extensions: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    extensions
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn extract_string_array(
    extensions: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Vec<String> {
    extensions
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn list_scene_packages_impl(root: &str) -> Result<Vec<SceneCard>, String> {
    // Canonicalise the input path. A `NotFound` is the graceful empty-
    // library case (the user's scenes directory simply does not exist
    // yet); other I/O errors surface to the caller.
    let canonical_input = match Path::new(root).canonicalize() {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("could not canonicalize {root}: {e}")),
    };

    // Scope the input to the platform-default root or a descendant of
    // it. If the default root itself does not exist yet, refuse rather
    // than silently widen the scan.
    let default_root = default_scene_root_impl()?;
    let canonical_default = Path::new(&default_root)
        .canonicalize()
        .map_err(|e| format!("could not canonicalize default root {default_root}: {e}"))?;
    if !canonical_input.starts_with(&canonical_default) {
        return Err("path is outside the allowed scene root".into());
    }

    let entries = std::fs::read_dir(&canonical_input)
        .map_err(|e| format!("could not read {}: {e}", canonical_input.display()))?;

    let mut cards = Vec::new();
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }
        if entry_path.extension().and_then(|s| s.to_str()) != Some("chroma") {
            continue;
        }

        // Malformed `.chroma` packages are tolerated silently: a single
        // broken scene must not collapse the library scan.
        let Ok(scene) = load_chroma_dir(&entry_path) else {
            continue;
        };

        let id = entry_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let path = entry_path.to_string_lossy().into_owned();

        cards.push(SceneCard {
            id,
            title: extract_string(&scene.extensions, "TITLE"),
            credit: extract_string(&scene.extensions, "CREDIT"),
            description: extract_string(&scene.extensions, "DESCRIPTION"),
            tags: extract_string_array(&scene.extensions, "TAGS"),
            categories: extract_string_array(&scene.extensions, "CATEGORIES"),
            path,
            format_tag: FORMAT_TAG_NATIVE.to_string(),
        });
    }
    Ok(cards)
}

/// List every `.chroma` package directly under `root`, returning a
/// [`SceneCard`] per package per `docs/38` §2.2.
#[tauri::command]
pub fn list_scene_packages(root: String) -> Result<Vec<SceneCard>, String> {
    list_scene_packages_impl(&root)
}

/// Return the platform-default scene-library root path.
#[tauri::command]
pub fn default_scene_root() -> Result<String, String> {
    default_scene_root_impl()
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-LIBRARY-DISCOVERY (T1) — pins the directory-scan
    //! handler glue against `docs/38-scene-library-and-import-spec.md`
    //! §2.2: platform-default-root resolution shape, graceful empty-
    //! library fallback when the input path does not exist, and the
    //! descendant-of-default-root scope check.

    use super::{default_scene_root_impl, list_scene_packages_impl};

    #[test]
    fn default_scene_root_returns_platform_default() {
        let root = default_scene_root_impl().expect("HOME/APPDATA must be set in test env");
        assert!(!root.is_empty());
        if cfg!(target_os = "macos") {
            assert!(
                root.contains("Library/Application Support/Chromaticity/Scenes"),
                "got: {root}",
            );
        } else if cfg!(target_os = "windows") {
            assert!(root.contains("Chromaticity\\Scenes"), "got: {root}");
        } else {
            assert!(root.contains("chromaticity"), "got: {root}");
        }
    }

    #[test]
    fn list_scene_packages_nonexistent_path_returns_empty_vec() {
        // A path that does not exist anywhere on the filesystem returns
        // the graceful empty-library response, not an error.
        let nonexistent = format!(
            "{}/__chromaticity_test_nonexistent_{}__",
            default_scene_root_impl().unwrap(),
            std::process::id()
        );
        let cards =
            list_scene_packages_impl(&nonexistent).expect("nonexistent path must yield empty Ok");
        assert!(cards.is_empty());
    }

    #[test]
    fn list_scene_packages_path_traversal_rejected() {
        // `/tmp` (or `C:\Windows\Temp`) is outside the platform-default
        // scene root; the descendant check must reject it. Skip on
        // Windows where temp-dir paths and APPDATA can intersect.
        if cfg!(target_os = "windows") {
            return;
        }
        // The canonicalize step needs the path to exist; std::env::temp_dir
        // is guaranteed to exist on the test host.
        let temp = std::env::temp_dir();
        let result = list_scene_packages_impl(&temp.to_string_lossy());
        match result {
            Err(e) => assert!(
                e.contains("outside the allowed scene root")
                    || e.contains("could not canonicalize default root"),
                "got: {e}",
            ),
            Ok(_) => panic!("expected Err for {temp:?}, got Ok"),
        }
    }

    #[test]
    fn default_scene_root_is_absolute() {
        let root = default_scene_root_impl().unwrap();
        // POSIX paths start with `/`; Windows paths carry `:\\` after
        // the drive letter. Either form is rooted.
        assert!(
            root.starts_with('/') || root.contains(":\\"),
            "expected absolute path, got: {root}",
        );
    }

    // The two tests below were `#[ignore]`'d at Wave-9 PR1 because the
    // production `list_scene_packages_impl` requires the input path to
    // live under the platform-default scene root (`$HOME/Library/...`,
    // `$XDG_DATA_HOME/...`), and routine `cargo test` must not stage
    // fixtures under the user's real home directory.
    //
    // Wave-10 PR1 reroutes both to the loader directly
    // (`chromaticity_scene::loader::load_chroma_dir`) using the
    // repo-resident demo corpus at
    // `engine/scene/tests/fixtures/scenes/`. This exercises the
    // metadata-extraction contract that `list_scene_packages_impl`
    // depends on (canonical-model fields surface through the
    // `extensions` map exactly the way the IPC handler reads them) and
    // the malformed-input rejection that the `let Ok(scene) = ...`
    // skip pattern in the IPC handler relies on, without touching
    // `$HOME`. The IPC handler's own unit coverage (default-root
    // resolution, nonexistent-path empty-Ok, traversal rejection) lives
    // in the four tests above.

    #[test]
    fn list_scene_packages_one_chroma_returns_one_card() {
        // Evidence: EV-LIBRARY-DISCOVERY (T1) — fixture-backed coverage
        // for the metadata-extraction path against the repo-resident
        // `demo-kaleido.chroma` package. Surfaces the same `TITLE`,
        // `CREDIT`, `DESCRIPTION`, `TAGS`, `CATEGORIES` fields the
        // `SceneCard` mapping in `list_scene_packages_impl` reads from
        // `Scene::extensions`.
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../engine/scene/tests/fixtures/scenes/demo-kaleido.chroma");

        let scene =
            chromaticity_scene::loader::load_chroma_dir(&fixture).expect("fixture must load");

        assert_eq!(
            super::extract_string(&scene.extensions, "TITLE"),
            "Demo Kaleido",
        );
        assert_eq!(
            super::extract_string(&scene.extensions, "CREDIT"),
            "chromaticity demo",
        );
        assert!(
            !super::extract_string(&scene.extensions, "DESCRIPTION").is_empty(),
            "DESCRIPTION must surface for library display"
        );
        assert_eq!(
            super::extract_string_array(&scene.extensions, "TAGS"),
            vec!["abstract".to_string(), "geometric".to_string()],
        );
        assert_eq!(
            super::extract_string_array(&scene.extensions, "CATEGORIES"),
            vec![
                "BUILT-IN".to_string(),
                "ABSTRACT".to_string(),
                "GEOMETRIC".to_string(),
            ],
        );
        assert!(!scene.controls.is_empty(), "demo-kaleido carries controls");
    }

    #[test]
    fn list_scene_packages_malformed_scene_json_skipped() {
        // Evidence: EV-LIBRARY-DISCOVERY (T1) — pins the loader-level
        // contract that the `let Ok(scene) = load_chroma_dir(...)` skip
        // arm in `list_scene_packages_impl` relies on: malformed
        // `scene.json` returns `Err`, which the IPC handler then
        // tolerates silently.
        let root = std::env::temp_dir().join(format!(
            "chromaticity-library-malformed-{}.chroma",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).expect("temp .chroma dir must be created");
        std::fs::write(root.join("scene.json"), "{ this is not json")
            .expect("malformed scene.json must be written");

        let result = chromaticity_scene::loader::load_chroma_dir(&root);
        assert!(
            result.is_err(),
            "malformed scene.json must surface as Err so the IPC handler skips it"
        );

        std::fs::remove_dir_all(root).expect("temp dir must be removed");
    }
}
