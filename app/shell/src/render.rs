// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Shell-side live GPU preview renderer backed by `chromaticity-render`
//! (`wgpu 26` + `naga 26` GLSL 4.50 → SPIR-V).
//!
//! Uses an explicit [`GpuRenderState`] discriminated enum (`Idle` |
//! `Active` | `Error`) and lazily acquires a long-lived
//! [`MultipassRenderer`] on first frame render so unit tests without a
//! GPU or scenes without `main.glsl` incur zero GPU startup cost.

use chromaticity_audio::AudioUniformSnapshot;
use chromaticity_render::{
    CachedSinglePass, MultipassRenderer, MultipassSpec, PassDef, TargetFormat, UniformInput,
    UniformValue,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Minimum preview dimension (pixels) accepted by [`GpuPreviewEngine::render_frame`].
const MIN_PREVIEW_DIM: u32 = 16;

/// Maximum preview width (pixels) accepted by [`GpuPreviewEngine::render_frame`].
const MAX_PREVIEW_WIDTH: u32 = 1920;

/// Maximum preview height (pixels) accepted by [`GpuPreviewEngine::render_frame`].
const MAX_PREVIEW_HEIGHT: u32 = 1080;

/// Explicit lifecycle state of the shell GPU preview engine.
#[derive(Debug, Clone)]
pub enum GpuRenderState {
    /// No scene loaded, or the loaded scene carries no `main.glsl` shader.
    Idle,
    /// A `.chroma` scene's `main.glsl` is armed and ready to render on GPU.
    Active {
        scene_id: String,
        shader_path: PathBuf,
        fragment_source: String,
        frames_rendered: u64,
        last_frame_ms: f32,
    },
    /// The armed shader failed to load, compile, or render on the GPU.
    Error { scene_id: String, message: String },
}

/// Serializable status snapshot surfaced to the UI HUD and
/// `inspect_state_machines`.
#[derive(Debug, Clone, Serialize)]
pub struct GpuRenderStatus {
    pub state: &'static str,
    pub scene_id: Option<String>,
    pub frames_rendered: u64,
    pub last_frame_ms: f32,
    pub error: Option<String>,
}

/// Long-lived GPU preview engine owned by [`crate::state::AppState`].
pub struct GpuPreviewEngine {
    state: GpuRenderState,
    renderer: Option<MultipassRenderer>,
    cached_pass: Option<CachedSinglePass>,
}

impl Default for GpuPreviewEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuPreviewEngine {
    /// Construct an idle GPU preview engine (zero GPU allocation until first render).
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: GpuRenderState::Idle,
            renderer: None,
            cached_pass: None,
        }
    }

    /// Arm the GPU preview engine from a loaded `.chroma` scene directory.
    ///
    /// If `<scene_dir>/main.glsl` exists, reads it and transitions to
    /// [`GpuRenderState::Active`]. If no `main.glsl` exists, transitions
    /// cleanly to [`GpuRenderState::Idle`] so the UI falls back to the
    /// procedural Canvas2D preview.
    pub fn set_scene(&mut self, scene_id: String, scene_dir: &Path) {
        self.cached_pass = None;
        let shader_path = scene_dir.join("main.glsl");
        if !shader_path.is_file() {
            self.state = GpuRenderState::Idle;
            return;
        }
        match std::fs::read_to_string(&shader_path) {
            Ok(fragment_source) => {
                self.state = GpuRenderState::Active {
                    scene_id,
                    shader_path,
                    fragment_source,
                    frames_rendered: 0,
                    last_frame_ms: 0.0,
                };
            }
            Err(err) => {
                self.state = GpuRenderState::Error {
                    scene_id,
                    message: format!("failed to read {}: {err}", shader_path.display()),
                };
            }
        }
    }

    /// Arm an in-memory GLSL 4.50 fragment shader directly (used by live
    /// Author preview and unit tests).
    pub fn set_inline_shader(&mut self, scene_id: String, fragment_source: String) {
        self.cached_pass = None;
        self.state = GpuRenderState::Active {
            scene_id,
            shader_path: PathBuf::from("<inline>"),
            fragment_source,
            frames_rendered: 0,
            last_frame_ms: 0.0,
        };
    }

    /// Disarm the GPU preview engine back to [`GpuRenderState::Idle`].
    pub fn clear_scene(&mut self) {
        self.cached_pass = None;
        self.state = GpuRenderState::Idle;
    }

    /// Read-only status snapshot for the UI HUD and state-machine diagnostics.
    #[must_use]
    pub fn status(&self) -> GpuRenderStatus {
        match &self.state {
            GpuRenderState::Idle => GpuRenderStatus {
                state: "idle",
                scene_id: None,
                frames_rendered: 0,
                last_frame_ms: 0.0,
                error: None,
            },
            GpuRenderState::Active {
                scene_id,
                frames_rendered,
                last_frame_ms,
                ..
            } => GpuRenderStatus {
                state: "active",
                scene_id: Some(scene_id.clone()),
                frames_rendered: *frames_rendered,
                last_frame_ms: *last_frame_ms,
                error: None,
            },
            GpuRenderState::Error { scene_id, message } => GpuRenderStatus {
                state: "error",
                scene_id: Some(scene_id.clone()),
                frames_rendered: 0,
                last_frame_ms: 0.0,
                error: Some(message.clone()),
            },
        }
    }

    /// Short state tag for `inspect_state_machines`.
    #[must_use]
    pub fn state_name(&self) -> &'static str {
        match &self.state {
            GpuRenderState::Idle => "idle",
            GpuRenderState::Active { .. } => "active",
            GpuRenderState::Error { .. } => "error",
        }
    }

    /// Render a single RGBA8 frame of the active shader at `(width, height)`.
    ///
    /// Packs the standard Chromaticity std140 uniform block:
    /// - `chroma_Time` (`vec4`: elapsed_s, dt, frame, aspect)
    /// - `chroma_Resolution` (`vec4`: w, h, 1/w, 1/h)
    /// - `syn_BPM` (`vec4`: bpm, 0, 0, 0)
    /// - `chroma_AudioRMS` (`vec4`: rms, 0, 0, 0)
    /// - `chroma_AudioPeak` (`vec4`: peak, 0, 0, 0)
    /// - `chroma_AudioFFT` (`float[8]`: 8 log-spaced FFT bands)
    /// - `chroma_Ctrl0` (`vec4`: controls 0..3)
    /// - `chroma_Ctrl1` (`vec4`: controls 4..7)
    pub fn render_frame(
        &mut self,
        width: u32,
        height: u32,
        elapsed_secs: f32,
        frame: u64,
        audio: &AudioUniformSnapshot,
        controls: [f32; 8],
    ) -> Result<Vec<u8>, String> {
        let (scene_id, fragment_source) = match &self.state {
            GpuRenderState::Idle => return Err("gpu renderer is idle (no active main.glsl)".into()),
            GpuRenderState::Error { message, .. } => return Err(message.clone()),
            GpuRenderState::Active {
                scene_id,
                fragment_source,
                ..
            } => (scene_id.clone(), fragment_source.clone()),
        };

        let w = width.clamp(MIN_PREVIEW_DIM, MAX_PREVIEW_WIDTH);
        let h = height.clamp(MIN_PREVIEW_DIM, MAX_PREVIEW_HEIGHT);

        if self.renderer.is_none() {
            match MultipassRenderer::new() {
                Ok(r) => self.renderer = Some(r),
                Err(err) => {
                    let msg = format!("wgpu init failed: {err}");
                    self.state = GpuRenderState::Error {
                        scene_id,
                        message: msg.clone(),
                    };
                    return Err(msg);
                }
            }
        }

        let Some(renderer) = self.renderer.as_ref() else {
            return Err("wgpu renderer unavailable".into());
        };

        let uniforms = build_standard_uniforms((w, h), elapsed_secs, frame, audio, controls);

        let started = Instant::now();
        if self.cached_pass.is_none() {
            match renderer.compile_cached_single_pass(
                &scene_id,
                &fragment_source,
                &uniforms,
                (w, h),
            ) {
                Ok(pass) => self.cached_pass = Some(pass),
                Err(err) => {
                    let msg = format!("shader compile failed: {err}");
                    self.state = GpuRenderState::Error {
                        scene_id,
                        message: msg.clone(),
                    };
                    return Err(msg);
                }
            }
        }

        let Some(cached) = self.cached_pass.as_mut() else {
            return Err("cached GPU pipeline unavailable".into());
        };

        match cached.render(renderer, (w, h), &uniforms) {
            Ok(bytes) => {
                let elapsed_ms = started.elapsed().as_secs_f32() * 1000.0;
                if let GpuRenderState::Active {
                    frames_rendered,
                    last_frame_ms,
                    ..
                } = &mut self.state
                {
                    *frames_rendered = frames_rendered.saturating_add(1);
                    *last_frame_ms = elapsed_ms;
                }
                Ok(bytes)
            }
            Err(err) => {
                self.cached_pass = None;
                let msg = format!("shader render failed: {err}");
                self.state = GpuRenderState::Error {
                    scene_id,
                    message: msg.clone(),
                };
                Err(msg)
            }
        }
    }
}

/// Construct the standard Chromaticity std140 uniform vector for a frame.
#[must_use]
pub fn build_standard_uniforms(
    target_size: (u32, u32),
    elapsed_secs: f32,
    frame: u64,
    audio: &AudioUniformSnapshot,
    controls: [f32; 8],
) -> Vec<UniformInput> {
    let w = target_size.0 as f32;
    let h = target_size.1 as f32;
    let aspect = if h > 0.0 { w / h } else { 1.0 };

    vec![
        UniformInput {
            name: "chroma_Time".to_string(),
            value: UniformValue::Vec4([elapsed_secs, 1.0 / 60.0, frame as f32, aspect]),
        },
        UniformInput {
            name: "chroma_Resolution".to_string(),
            value: UniformValue::Vec4([w, h, 1.0 / w.max(1.0), 1.0 / h.max(1.0)]),
        },
        UniformInput {
            name: "syn_BPM".to_string(),
            value: UniformValue::Vec4([audio.bpm, 0.0, 0.0, 0.0]),
        },
        UniformInput {
            name: "chroma_AudioRMS".to_string(),
            value: UniformValue::Vec4([audio.rms, 0.0, 0.0, 0.0]),
        },
        UniformInput {
            name: "chroma_AudioPeak".to_string(),
            value: UniformValue::Vec4([audio.peak, 0.0, 0.0, 0.0]),
        },
        UniformInput {
            name: "chroma_AudioFFT".to_string(),
            value: UniformValue::FloatArray(audio.fft_bands.to_vec()),
        },
        UniformInput {
            name: "chroma_Ctrl0".to_string(),
            value: UniformValue::Vec4([controls[0], controls[1], controls[2], controls[3]]),
        },
        UniformInput {
            name: "chroma_Ctrl1".to_string(),
            value: UniformValue::Vec4([controls[4], controls[5], controls[6], controls[7]]),
        },
    ]
}

/// Construct the standard single-pass [`MultipassSpec`] carrying the
/// Chromaticity std140 uniform block.
#[must_use]
pub fn build_standard_spec(
    label: &str,
    fragment_glsl: String,
    target_size: (u32, u32),
    elapsed_secs: f32,
    frame: u64,
    audio: &AudioUniformSnapshot,
    controls: [f32; 8],
) -> MultipassSpec {
    let uniforms = build_standard_uniforms(target_size, elapsed_secs, frame, audio, controls);

    MultipassSpec {
        passes: vec![PassDef {
            label: Some(label.to_string()),
            fragment_glsl,
            uniforms,
            input_textures: Vec::new(),
            target_format: TargetFormat::Rgba8Unorm,
            feedback_self: false,
        }],
        target_size,
        frame_count: 1,
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod gpu_render_pins {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — shell GPU preview engine
    //! state-machine and uniform-packing pins.

    use super::*;

    const SAMPLE_SHADER: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 0) uniform Uniforms {
    vec4 chroma_Time;
    vec4 chroma_Resolution;
    vec4 syn_BPM;
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    float chroma_AudioFFT[8];
    vec4 chroma_Ctrl0;
    vec4 chroma_Ctrl1;
};

void main() {
    float r = clamp(uv.x * 0.5 + chroma_AudioRMS.x * 0.5, 0.0, 1.0);
    float g = clamp(uv.y * 0.5 + chroma_Ctrl0.x * 0.5, 0.0, 1.0);
    float b = clamp(0.25 + chroma_AudioFFT[0] * 0.5, 0.0, 1.0);
    fragColor = vec4(r, g, b, 1.0);
}
"#;

    #[test]
    fn gpu_preview_engine_starts_idle_and_renders_inline_shader() {
        let mut engine = GpuPreviewEngine::new();
        assert_eq!(engine.state_name(), "idle");
        assert!(
            engine
                .render_frame(32, 32, 0.0, 1, &AudioUniformSnapshot::default(), [0.5; 8])
                .is_err()
        );

        engine.set_inline_shader("test-scene".into(), SAMPLE_SHADER.into());
        assert_eq!(engine.state_name(), "active");

        let bytes = engine
            .render_frame(32, 32, 1.25, 10, &AudioUniformSnapshot::default(), [0.5; 8])
            .expect("inline shader should render via wgpu");
        assert_eq!(bytes.len(), 32 * 32 * 4);
        assert_eq!(engine.status().frames_rendered, 1);
        // Verify alpha channel is opaque and non-zero RGB was written.
        assert_eq!(bytes[3], 255);
        assert!(bytes.iter().step_by(4).any(|&r| r > 0));

        // Second frame reuses cached pipeline and handles dynamic resize.
        let bytes2 = engine
            .render_frame(
                64,
                36,
                1.26,
                11,
                &AudioUniformSnapshot::default(),
                [0.75; 8],
            )
            .expect("cached pipeline should handle resize and uniform update");
        assert_eq!(bytes2.len(), 64 * 36 * 4);
        assert_eq!(engine.status().frames_rendered, 2);

        // Byte-identity pin: cached single-pass output equals MultipassRenderer::render.
        let renderer = MultipassRenderer::new().expect("wgpu renderer");
        let spec = build_standard_spec(
            "test-scene",
            SAMPLE_SHADER.into(),
            (64, 36),
            1.26,
            11,
            &AudioUniformSnapshot::default(),
            [0.75; 8],
        );
        let reference = renderer.render(&spec).expect("reference multipass render");
        assert_eq!(
            bytes2, reference,
            "CachedSinglePass must produce bit-identical bytes to MultipassRenderer::render"
        );
    }

    #[test]
    fn invalid_shader_transitions_to_error_state_without_panicking() {
        let mut engine = GpuPreviewEngine::new();
        engine.set_inline_shader("broken".into(), "not valid glsl".into());
        let err = engine.render_frame(16, 16, 0.0, 1, &AudioUniformSnapshot::default(), [0.5; 8]);
        assert!(err.is_err());
        assert_eq!(engine.state_name(), "error");
        assert!(engine.status().error.is_some());
    }

    #[test]
    fn all_scenes_local_shaders_compile_and_render_on_gpu() {
        let scenes_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("scenes-local");
        if !scenes_root.is_dir() {
            return;
        }
        let mut entries: Vec<_> = std::fs::read_dir(&scenes_root)
            .expect("read scenes-local")
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("chroma"))
            .collect();
        entries.sort();

        let mut engine = GpuPreviewEngine::new();
        let audio = AudioUniformSnapshot {
            bpm: 128.0,
            rms: 0.45,
            peak: 0.72,
            fft_bands: [0.6, 0.5, 0.4, 0.35, 0.3, 0.25, 0.2, 0.15],
            ..AudioUniformSnapshot::default()
        };

        let export_dir = std::env::var_os("CHROMATICITY_EXPORT_GALLERY_DIR").map(PathBuf::from);
        let bench_mode = std::env::var_os("CHROMATICITY_BENCH_SHADERS").is_some();
        let (w, h) = if bench_mode {
            (640u32, 360u32)
        } else if export_dir.is_some() {
            (320u32, 180u32)
        } else {
            (32u32, 32u32)
        };

        let mut rendered_count = 0usize;
        for dir in entries {
            if !dir.join("main.glsl").is_file() {
                continue;
            }
            let id = dir
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("scene")
                .to_string();
            engine.set_scene(id.clone(), &dir);
            assert_eq!(engine.state_name(), "active", "scene {id} should arm");
            let bytes = engine
                .render_frame(w, h, 2.5, 150, &audio, [0.5; 8])
                .unwrap_or_else(|e| panic!("shader {id} failed to compile/render: {e}"));
            assert_eq!(bytes.len(), (w * h * 4) as usize);
            assert!(
                bytes
                    .chunks_exact(4)
                    .any(|px| px[0] > 0 || px[1] > 0 || px[2] > 0),
                "shader {id} rendered all-black output"
            );
            if bench_mode {
                let mut total_ms = 0.0_f64;
                let samples = 10u64;
                for i in 0..samples {
                    let t0 = Instant::now();
                    let _ = engine
                        .render_frame(w, h, 2.5 + (i as f32) * 0.016, 151 + i, &audio, [0.5; 8])
                        .unwrap_or_else(|e| panic!("bench frame failed for {id}: {e}"));
                    total_ms += t0.elapsed().as_secs_f64() * 1000.0;
                }
                let avg_ms = total_ms / (samples as f64);
                let fps = 1000.0 / avg_ms.max(0.001);
                println!("BENCH {id}: {avg_ms:.2} ms/frame ({fps:.0} FPS) @ {w}x{h}");
            }
            if let Some(ref out_dir) = export_dir {
                let _ = std::fs::create_dir_all(out_dir);
                let _ = std::fs::write(out_dir.join(format!("{id}.rgba")), &bytes);
            }
            rendered_count += 1;
        }
        assert!(
            rendered_count >= 20,
            "expected at least 20 live shaders in scenes-local, rendered {rendered_count}"
        );
    }
}
