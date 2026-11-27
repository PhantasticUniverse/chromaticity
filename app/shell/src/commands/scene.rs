// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Scene-load IPC handler — Wave 2.1 Lane A.1.
//!
//! The [`load_scene`] handler takes a path to a `.chroma` package,
//! invokes [`chromaticity_scene::loader::load_chroma_dir`] to parse
//! `scene.json`, stores the resulting
//! [`chromaticity_scene::schema::Scene`] + path on
//! [`crate::state::AppState::loaded_scene`], and returns a normalized
//! [`LoadedScenePayload`] the Perform view's Scene Controls header +
//! control-panel surfaces bind against.
//!
//! **Engine-surface posture.** Lane A.1 deliberately does NOT widen
//! the engine [`chromaticity_runtime::Command`] envelope with a
//! `SceneLoad` variant. The runtime command-bus + observability
//! catalog + EV-STATE-REPLAY (row 32) + EV-PRESET-PLAYLIST (row 34) T2
//! corpora are choke-points that a single Lane A.1 PR should not push
//! through. The loaded scene is a shell-side concern until Wave 2.2's
//! render pipeline + control dispatch refactor consumes it through the
//! canonical command bus. The shell side mirrors the
//! [`crate::state::AppState::install_audio_and_scheduler`] / `Mutex<...>`
//! interior-mutability pattern that the audio runtime + scheduler
//! threads already use; the public surface is one Tauri command plus
//! one inner `*_impl(&AppState, ...)` function for unit-test coverage
//! without a Tauri runtime (matching the
//! [`super::control::set_control_impl`] precedent).
//!
//! Errors collapse to `Result<_, String>` so the Tauri serializer
//! carries a human-readable failure across the IPC boundary; the
//! engine-side [`chromaticity_scene::loader::LoadError`] feeds
//! [`std::fmt::Display`] into the wrapper.

use chromaticity_scene::loader::load_chroma_dir;
use chromaticity_scene::schema::Scene;
use chromaticity_ui_bridge::{ControlSchemaPayload, LoadedScenePayload};
use std::path::{Path, PathBuf};
use tauri::State;

use crate::state::{AppState, LoadedSceneRecord};

/// Extract a string-typed `extensions` value for `key`, falling back
/// to an empty string when absent.
///
/// Mirrors the helper of the same name in [`super::library`]: the
/// `.chroma` JSON wire format flattens `TITLE` / `CREDIT` /
/// `DESCRIPTION` into [`Scene::extensions`], so reading them back is
/// a `serde_json::Map<String, serde_json::Value>` lookup.
fn extract_string(extensions: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    extensions
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Build a [`LoadedScenePayload`] from a parsed [`Scene`] + the on-
/// disk path. Pure function so tests can pin the field-mapping
/// without a Tauri runtime.
pub(crate) fn build_loaded_scene_payload(scene: &Scene, path: &Path) -> LoadedScenePayload {
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let title = extract_string(&scene.extensions, "TITLE");
    let name = if title.is_empty() { id.clone() } else { title };

    let controls = scene
        .controls
        .iter()
        .cloned()
        .map(ControlSchemaPayload::from)
        .collect::<Vec<_>>();

    LoadedScenePayload {
        id,
        path: path.to_string_lossy().into_owned(),
        name,
        credit: extract_string(&scene.extensions, "CREDIT"),
        description: extract_string(&scene.extensions, "DESCRIPTION"),
        controls,
        schema_version: scene.header.schema_version,
    }
}

/// Load the `.chroma` directory at `path`, store the parsed scene on
/// `app`, and return the payload describing what was loaded.
///
/// Inner implementation invoked by both the Tauri wrapper and unit
/// tests; takes a borrowed [`AppState`] directly.
fn load_scene_impl(app: &AppState, path: String) -> Result<LoadedScenePayload, String> {
    let path_buf = PathBuf::from(&path);

    // Parse the `.chroma` directory. `load_chroma_dir` validates the
    // `.chroma` extension + the `scene.json` entry-point + the JSON
    // shape before returning; any failure surfaces through Display.
    let scene = load_chroma_dir(&path_buf).map_err(|err| err.to_string())?;

    let payload = build_loaded_scene_payload(&scene, &path_buf);

    // Arm the live GPU preview engine if this `.chroma` package ships `main.glsl`.
    {
        let mut gpu_guard = match app.gpu_renderer.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        gpu_guard.set_scene(payload.id.clone(), &path_buf);
    }

    // Arm the live QuickJS `script.js` host if this `.chroma` package ships `script.js`.
    {
        let mut script_guard = match app.script_host.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        script_guard.set_scene(payload.id.clone(), &path_buf);
    }

    // Store the parsed scene + path on AppState behind the mutex.
    // Mutex poison-recovery follows the workspace pattern: read
    // through `match lock() { Ok(g) => g, Err(p) => p.into_inner() }`
    // rather than panicking on a poisoned mutex.
    let record = LoadedSceneRecord {
        scene,
        path: path_buf,
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

/// Read the most recently loaded scene's payload, or `None` if no
/// scene has been loaded since process start.
///
/// Inner implementation invoked by both the Tauri wrapper and unit
/// tests.
fn loaded_scene_impl(app: &AppState) -> Result<Option<LoadedScenePayload>, String> {
    let guard = match app.loaded_scene.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    Ok(guard
        .as_ref()
        .map(|record| build_loaded_scene_payload(&record.scene, &record.path)))
}

/// Load the `.chroma` package at `path` and return its
/// [`LoadedScenePayload`] description.
///
/// Errors when the path is not a `.chroma` directory, when
/// `scene.json` is missing, or when `scene.json` fails to parse.
/// On success the parsed scene is stored on
/// [`crate::state::AppState::loaded_scene`] for subsequent Perform-
/// view binding.
#[tauri::command]
pub fn load_scene(
    app_state: State<'_, AppState>,
    path: String,
) -> Result<LoadedScenePayload, String> {
    load_scene_impl(app_state.inner(), path)
}

/// Read the most recently loaded scene's payload, or `null` when
/// the runtime has not loaded a scene yet.
///
/// Used by the Perform view's mount-time hydration so the Scene
/// Controls header survives a route navigation that re-mounts the
/// component after a scene was loaded from the Library view.
#[tauri::command]
pub fn loaded_scene(app_state: State<'_, AppState>) -> Result<Option<LoadedScenePayload>, String> {
    loaded_scene_impl(app_state.inner())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — Wave 2.1 Lane A.1
    //! scene-load shell handler glue.
    //!
    //! T1 per docs/43 §15.5: pins the SHELL-side
    //! `parse path → load_chroma_dir → store on AppState → build payload`
    //! chain that is otherwise only exercised end-to-end through
    //! Tauri. Mirrors the [`super::control::tests`] precedent.
    //!
    //! Uses the repo-resident `engine/scene/tests/fixtures/scenes/library/demo-kaleido.chroma`
    //! fixture as the load target because the parsed schema is what
    //! the Perform Scene Controls panel binds against, and the fixture
    //! mirrors a representative six-control set (Slider / Slider /
    //! Knob / Toggle / Color / Xy).

    use super::{build_loaded_scene_payload, load_scene_impl, loaded_scene_impl};
    use crate::state::AppState;
    use chromaticity_scene::loader::load_chroma_dir;
    use chromaticity_ui_bridge::{
        ControlDefaultValuePayload, ControlKindPayload, LoadedScenePayload,
    };
    use std::path::Path;

    fn demo_kaleido_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../engine/scene/tests/fixtures/scenes/library/demo-kaleido.chroma")
    }

    #[test]
    fn build_loaded_scene_payload_extracts_metadata_and_controls() {
        let path = demo_kaleido_path();
        let scene = load_chroma_dir(&path).expect("demo-kaleido fixture must load");

        let payload = build_loaded_scene_payload(&scene, &path);

        assert_eq!(payload.id, "demo-kaleido");
        assert_eq!(payload.name, "Demo Kaleido");
        assert_eq!(payload.credit, "chromaticity demo");
        assert!(payload.description.contains("radial"));
        assert!(payload.path.ends_with("demo-kaleido.chroma"));
        assert_eq!(payload.schema_version, 1);

        // Demo-kaleido fixture declares 6 controls: segments
        // (Slider), rotation_speed (Slider), warp_amount (Knob),
        // invert (Toggle), tint (Color), focus (Xy).
        assert_eq!(payload.controls.len(), 6);
        let names: Vec<&str> = payload.controls.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "segments",
                "rotation_speed",
                "warp_amount",
                "invert",
                "tint",
                "focus",
            ],
        );

        // Pin the kind mapping against the engine ControlKind set.
        assert_eq!(payload.controls[0].kind, Some(ControlKindPayload::Slider));
        assert_eq!(payload.controls[2].kind, Some(ControlKindPayload::Knob));
        assert_eq!(payload.controls[3].kind, Some(ControlKindPayload::Toggle));
        assert_eq!(payload.controls[4].kind, Some(ControlKindPayload::Color));
        assert_eq!(payload.controls[5].kind, Some(ControlKindPayload::Xy));

        // Pin the default-value mapping: Slider segments defaults to
        // 6.0, Toggle invert defaults to `false`, Color tint defaults
        // to RGBA quadruple.
        assert_eq!(
            payload.controls[0].default,
            ControlDefaultValuePayload::Float(6.0),
        );
        assert_eq!(
            payload.controls[3].default,
            ControlDefaultValuePayload::Bool(false),
        );
        assert_eq!(
            payload.controls[4].default,
            ControlDefaultValuePayload::Vec4([0.62, 0.31, 0.88, 1.0]),
        );
        assert_eq!(
            payload.controls[5].default,
            ControlDefaultValuePayload::Vec2([0.5, 0.5]),
        );

        // Slider min/max round-trip.
        assert_eq!(payload.controls[0].min, Some(2.0));
        assert_eq!(payload.controls[0].max, Some(24.0));
    }

    #[test]
    fn build_loaded_scene_payload_falls_back_to_id_when_title_absent() {
        // Synthesize a Scene without a TITLE extension and confirm
        // `name` falls back to the file-stem id rather than emitting
        // an empty string the UI would render as blank chrome.
        let scene = chromaticity_scene::schema::Scene::default();
        let path = std::path::PathBuf::from("/tmp/no-title.chroma");
        let payload = build_loaded_scene_payload(&scene, &path);
        assert_eq!(payload.id, "no-title");
        assert_eq!(payload.name, "no-title");
        assert_eq!(payload.credit, "");
        assert_eq!(payload.description, "");
        assert!(payload.controls.is_empty());
    }

    #[test]
    fn load_scene_impl_then_loaded_scene_impl_round_trip() {
        let app = AppState::new();

        // Initially no scene loaded.
        assert!(loaded_scene_impl(&app).unwrap().is_none());

        let path = demo_kaleido_path().to_string_lossy().into_owned();
        let loaded: LoadedScenePayload = load_scene_impl(&app, path.clone()).unwrap();
        assert_eq!(loaded.id, "demo-kaleido");

        // Read-back via loaded_scene_impl returns the same payload.
        let after = loaded_scene_impl(&app).unwrap().expect("scene loaded");
        assert_eq!(after, loaded);
    }

    #[test]
    fn load_scene_impl_rejects_non_chroma_directory() {
        let app = AppState::new();
        // The crate root itself is a directory, but its extension is
        // not `.chroma`; `load_chroma_dir` must reject it.
        let path = env!("CARGO_MANIFEST_DIR").to_string();
        let result = load_scene_impl(&app, path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains(".chroma") || err.contains("not a directory"),
            "expected .chroma rejection, got: {err}",
        );
    }

    #[test]
    fn load_scene_impl_overwrites_previously_loaded_scene() {
        let app = AppState::new();
        let path = demo_kaleido_path().to_string_lossy().into_owned();
        let _first = load_scene_impl(&app, path.clone()).unwrap();
        let second = load_scene_impl(&app, path.clone()).unwrap();
        // Same fixture → same payload. The test pins the "store
        // replaces previous record" path rather than a no-op append.
        assert_eq!(second.id, "demo-kaleido");
        let after = loaded_scene_impl(&app).unwrap().expect("scene loaded");
        assert_eq!(after, second);
    }
}
