// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Output-scene loading for the ADR-011 Phase-2 Slice R-1b live render
//! path.
//!
//! Loads a `.chroma` scene on the MAIN THREAD (plain file I/O — no GPU,
//! no main-thread constraint, but co-located with the rest of the
//! output-window bootstrap) and exposes a per-frame
//! [`chromaticity_render::MultipassSpec`] builder via the engine's
//! `from_scene_live` resolver. The render thread owns the resulting
//! [`LoadedScene`] and calls [`LoadedScene::frame_spec`] each frame with
//! the current swapchain size + elapsed time at SILENT audio (live audio
//! reactivity is Slice R-1c).
//!
//! The scene is the `CHROMA_RENDER_SCENE` env path, or the default
//! wide-energy-plasma archetype — a reconciled, render-verified scene
//! that animates via `TIME` even at silence. Any failure (missing /
//! unparseable scene, missing `main.glsl`, a multi-pass scene, or a scene
//! that does not resolve) logs the reason and yields `None`, so the
//! caller falls back to the R-1a [`crate::render::pattern::test_pattern_spec`].
//!
//! R-1b scope: SINGLE-PASS scenes only (the shader-archetypes are all
//! single-pass; multi-pass / feedback wiring is a later slice). The
//! default-scene path is resolved relative to the process CWD — launch
//! from the repo root, or set `CHROMA_RENDER_SCENE` to an absolute path.

use std::path::{Path, PathBuf};

use chromaticity_render::MultipassSpec;
use chromaticity_render::scene_adapter::{AdapterError, LiveUniforms, SceneAdapterOptions};
use chromaticity_scene::loader::load_chroma_dir;
use chromaticity_scene::schema::Scene;

/// Env var selecting the output scene `.chroma` directory.
const OUTPUT_SCENE_ENV_VAR: &str = "CHROMA_RENDER_SCENE";

/// Default output scene when `CHROMA_RENDER_SCENE` is unset: a reconciled,
/// render-verified archetype that is full-frame and TIME-animated at
/// silent audio.
const DEFAULT_SCENE: &str =
    "engine/scene/tests/fixtures/scenes/shader-archetypes/wide-energy-plasma.chroma";

/// Stable scene id folded into the engine feedback-cache key for the live
/// output scene (kept constant across frames so the renderer does not
/// needlessly reallocate; the R-1b default scene has no feedback pass).
pub const OUTPUT_SCENE_SPEC_ID: &str = "chroma-r1b-output-scene";

/// A loaded single-pass output scene plus its fragment GLSL, ready for
/// per-frame [`MultipassSpec::from_scene_live`] resolution.
pub struct LoadedScene {
    scene: Scene,
    /// Per-pass fragment GLSL (one entry for the single pass).
    fragment_sources: Vec<Option<String>>,
    /// Display label (the source path) for diagnostics.
    label: String,
}

impl LoadedScene {
    /// Load the configured output scene, or `None` (after logging the
    /// reason) on any failure so the caller falls back to the test
    /// pattern.
    #[must_use]
    pub fn load() -> Option<Self> {
        let path = std::env::var(OUTPUT_SCENE_ENV_VAR)
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_SCENE));
        let label = path.display().to_string();

        let scene = match load_chroma_dir(&path) {
            Ok(scene) => scene,
            Err(err) => {
                eprintln!(
                    "chromaticity-app: output scene '{label}' failed to load ({err}); \
                     falling back to the test pattern"
                );
                return None;
            }
        };

        if scene.passes.len() != 1 {
            eprintln!(
                "chromaticity-app: output scene '{label}' has {} passes; R-1b renders \
                 single-pass only — falling back to the test pattern",
                scene.passes.len()
            );
            return None;
        }

        let main_glsl = match std::fs::read_to_string(Path::new(&path).join("main.glsl")) {
            Ok(src) => src,
            Err(err) => {
                eprintln!(
                    "chromaticity-app: output scene '{label}' main.glsl unreadable ({err}); \
                     falling back to the test pattern"
                );
                return None;
            }
        };

        let loaded = Self {
            scene,
            fragment_sources: vec![Some(main_glsl)],
            label,
        };

        // Validate the scene resolves before committing to it. Structural
        // failures (e.g. a feedback texture mis-listed in UNIFORMS) are
        // size-independent, so one probe confirms every subsequent frame.
        if let Err(err) = loaded.frame_spec(1280, 720, 0.0) {
            eprintln!(
                "chromaticity-app: output scene '{}' does not resolve ({err}); \
                 falling back to the test pattern",
                loaded.label
            );
            return None;
        }

        eprintln!(
            "chromaticity-app: rendering live output scene '{}'",
            loaded.label
        );
        Some(loaded)
    }

    /// Build the per-frame renderable spec at `(width, height)` and
    /// `time_secs`, SILENT audio (live audio is R-1c).
    ///
    /// # Errors
    ///
    /// Propagates [`AdapterError`] from `from_scene_live` (e.g. a feedback
    /// texture mis-listed in `UNIFORMS`). Validated once at [`Self::load`],
    /// so this is effectively infallible for an already-loaded scene.
    pub fn frame_spec(
        &self,
        width: u32,
        height: u32,
        time_secs: f32,
    ) -> Result<MultipassSpec, AdapterError> {
        let width = width.max(1);
        let height = height.max(1);
        let options = SceneAdapterOptions {
            fragment_sources: self.fragment_sources.clone(),
            default_target_size: (width, height),
            ..SceneAdapterOptions::default()
        };
        let live = LiveUniforms::silent(time_secs, (width as f32, height as f32));
        MultipassSpec::from_scene_live(&self.scene, &options, &live)
    }

    /// The scene's display label (source path), for diagnostics.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}
