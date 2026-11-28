// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Shell-side live GPU preview renderer backed by `chromaticity-render`
//! (`wgpu 26` + `naga 26` GLSL 4.50 → SPIR-V).
//!
//! Uses an explicit [`GpuRenderState`] discriminated enum (`Idle` |
//! `Active` | `Error`) and lazily acquires a long-lived
//! [`MultipassRenderer`] on first frame render so unit tests without a
//! GPU or scenes without `main.glsl` incur zero GPU startup cost.

use chromaticity_audio::{AudioUniformSnapshot, SynAudioEnvelopeBank, SynAudioEnvelopes};
use chromaticity_render::{
    CachedMultipass, CachedSinglePass, MultipassRenderer, MultipassSpec, PassDef, PassInputBinding,
    TargetFormat, TransitionOrchestrator, UniformInput, UniformValue, resolve_includes_multi,
};
use chromaticity_scene::loader::parse_scene_json;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Clean-room GLSL 4.50 implementations of standard SSF helper functions
/// injected automatically when a fragment shader references them without
/// defining its own.
const SSF_GLSL_HELPER_PRELUDE: &str = r#"
// --- Chromaticity SSF GLSL Helper Prelude ---
float _map(float v, float inMin, float inMax, float outMin, float outMax) {
    float denom = max(abs(inMax - inMin), 1e-6);
    float t = (v - inMin) / (inMax >= inMin ? denom : -denom);
    return outMin + t * (outMax - outMin);
}
float _smin(float a, float b, float k) {
    float h = max(k - abs(a - b), 0.0) / max(k, 1e-6);
    return min(a, b) - h * h * k * 0.25;
}
vec2 _rotate(vec2 p, float a) {
    float c = cos(a);
    float s = sin(a);
    return vec2(c * p.x - s * p.y, s * p.x + c * p.y);
}
vec2 _toPolar(vec2 xy) {
    return vec2(length(xy), atan(xy.y, xy.x));
}
vec2 _fromPolar(vec2 rt) {
    return vec2(cos(rt.y), sin(rt.y)) * rt.x;
}
vec2 _toRect(vec2 rt) {
    return _fromPolar(rt);
}
vec3 _rgb2hsv(vec3 c) {
    vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
    vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));
    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}
vec3 _hsv2rgb(vec3 c) {
    vec3 p = abs(fract(c.xxx + vec3(1.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0);
    return c.z * mix(vec3(1.0), clamp(p - 1.0, 0.0, 1.0), c.y);
}
float _hash21(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}
float _noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    float a = _hash21(i);
    float b = _hash21(i + vec2(1.0, 0.0));
    float c = _hash21(i + vec2(0.0, 1.0));
    float d = _hash21(i + vec2(1.0, 1.0));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}
float _fbm(vec2 p) {
    float v = 0.0;
    float amp = 0.5;
    for (int i = 0; i < 4; i++) {
        v += amp * _noise(p);
        p *= 2.02;
        amp *= 0.5;
    }
    return v;
}
// --- End SSF GLSL Helper Prelude ---
"#;

/// Inject [`SSF_GLSL_HELPER_PRELUDE`] after `#version ...` when a shader calls
/// standard SSF helper functions without declaring them locally.
fn inject_ssf_helpers_if_needed(source: &str) -> String {
    const MARKERS: &[&str] = &[
        "_map(",
        "_smin(",
        "_rotate(",
        "_toPolar(",
        "_fromPolar(",
        "_toRect(",
        "_rgb2hsv(",
        "_hsv2rgb(",
        "_noise(",
        "_fbm(",
    ];
    let needs_prelude = MARKERS.iter().any(|m| source.contains(m))
        && !source.contains("Chromaticity SSF GLSL Helper Prelude")
        && !source.contains("_chroma_safe_pow");
    if !needs_prelude {
        return source.to_string();
    }
    if let Some(ver_pos) = source.find("#version") {
        if let Some(nl_rel) = source[ver_pos..].find('\n') {
            let split = ver_pos + nl_rel + 1;
            let mut out = String::with_capacity(source.len() + SSF_GLSL_HELPER_PRELUDE.len());
            out.push_str(&source[..split]);
            out.push_str(SSF_GLSL_HELPER_PRELUDE);
            out.push_str(&source[split..]);
            return out;
        }
    }
    format!("{SSF_GLSL_HELPER_PRELUDE}\n{source}")
}

/// Resolve `#include "lygia/..."` and `#include "hg_sdf/..."` directives
/// and inject SSF helper functions when referenced.
fn preprocess_shader_with_helpers(
    source: &str,
    resource_dir: Option<&Path>,
    pass_index: u32,
) -> Result<String, String> {
    let adapted = crate::syn_compat::adapt_syn_imported_shader(source, pass_index);
    let with_helpers = inject_ssf_helpers_if_needed(&adapted);
    if !with_helpers.contains("#include") {
        return Ok(with_helpers);
    }
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lygia_root = resource_dir
        .map(|d| d.join("third_party/lygia"))
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| repo_root.join("third_party/lygia"));
    let hg_sdf_root = resource_dir
        .map(|d| d.join("third_party/hg_sdf"))
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| repo_root.join("third_party/hg_sdf"));
    let mut namespaces: Vec<(&str, &Path)> = Vec::new();
    if lygia_root.is_dir() {
        namespaces.push(("lygia", lygia_root.as_path()));
    }
    if hg_sdf_root.is_dir() {
        namespaces.push(("hg_sdf", hg_sdf_root.as_path()));
    }
    if namespaces.is_empty() {
        return Err(
            "shader contains #include directives, but neither third_party/lygia nor third_party/hg_sdf could be located"
                .into(),
        );
    }
    resolve_includes_multi(&with_helpers, &namespaces)
        .map_err(|err| format!("shader #include resolution failed: {err}"))
}

/// Load optional intermediate pass fragment shaders declared in `scene.json` `PASSES`.
///
/// For each pass entry, resolves either an explicit `SHADER`/`FRAGMENT` filename
/// under `scene_dir` or `<TARGET>.glsl` (ignoring `main.glsl` which is always the
/// final pass). Passes without an on-disk `.glsl` file are skipped so single-pass
/// scenes with metadata-only `PASSES` stay on the fast `CachedSinglePass` path.
fn load_scene_extra_passes(scene_dir: &Path) -> Vec<ScenePassSource> {
    let scene_json_path = scene_dir.join("scene.json");
    let Ok(raw_json) = std::fs::read_to_string(&scene_json_path) else {
        return Vec::new();
    };
    let Ok(scene) = parse_scene_json(&raw_json) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for pass in &scene.passes {
        let target = pass.node.0.trim();
        let rel_file = pass
            .extensions
            .get("SHADER")
            .or_else(|| pass.extensions.get("FRAGMENT"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{target}.glsl"));
        if rel_file.is_empty() || rel_file == "main.glsl" || rel_file.contains("..") {
            continue;
        }
        let pass_path = scene_dir.join(&rel_file);
        let Ok(fragment_glsl) = std::fs::read_to_string(&pass_path) else {
            continue;
        };
        let is_float = pass
            .extensions
            .get("FLOAT")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let feedback_self = pass
            .extensions
            .get("PERSISTENT")
            .or_else(|| pass.extensions.get("FEEDBACK"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let label = if target.is_empty() {
            rel_file.trim_end_matches(".glsl").to_string()
        } else {
            target.to_string()
        };
        out.push(ScenePassSource {
            label,
            fragment_glsl,
            target_format: if is_float {
                TargetFormat::Rgba16Float
            } else {
                TargetFormat::Rgba8Unorm
            },
            feedback_self,
        });
    }
    out
}

/// Minimum preview dimension (pixels) accepted by [`GpuPreviewEngine::render_frame`].
const MIN_PREVIEW_DIM: u32 = 16;

/// Maximum preview width (pixels) accepted by [`GpuPreviewEngine::render_frame`].
const MAX_PREVIEW_WIDTH: u32 = 1920;

/// Maximum preview height (pixels) accepted by [`GpuPreviewEngine::render_frame`].
const MAX_PREVIEW_HEIGHT: u32 = 1080;

/// Minimum allowed manual BPM override (beats per minute).
pub const MIN_MANUAL_BPM: f32 = 20.0;

/// Maximum allowed manual BPM override (beats per minute).
pub const MAX_MANUAL_BPM: f32 = 300.0;

/// Default manual BPM fallback when an invalid/non-finite value is supplied.
pub const DEFAULT_MANUAL_BPM: f32 = 120.0;

/// Exponential decay factor for `on_beat` pulse within a single beat (`phase ∈ [0, 1)`).
const ON_BEAT_DECAY_RATE: f32 = 6.0;

/// Explicit live BPM synchronization mode for GPU shader uniforms.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum BpmOverrideMode {
    /// Use live audio-detected BPM with optional phase scrub offset.
    Auto,
    /// Lock shader BPM to an explicit manual tempo.
    Manual { bpm: f32 },
    /// Disable BPM sync (`bpm = 0.0`, `beat_time = 0.0`, `on_beat = 0.0`, `bpm_sin = 0.0`).
    Off,
}

/// Live BPM override and beatmatch phase-scrub state applied to GPU frame uniforms.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BpmOverrideState {
    #[serde(flatten)]
    pub mode: BpmOverrideMode,
    /// Beatmatch phase scrub offset in beats (positive advances, negative delays).
    #[serde(default)]
    pub phase_offset_beats: f32,
    /// Monotonic shell-elapsed epoch (milliseconds) anchoring beat 0 after a resync/tap.
    #[serde(default)]
    pub resync_epoch_ms: u64,
}

impl Default for BpmOverrideState {
    fn default() -> Self {
        Self {
            mode: BpmOverrideMode::Auto,
            phase_offset_beats: 0.0,
            resync_epoch_ms: 0,
        }
    }
}

impl BpmOverrideState {
    /// Return a sanitized copy with finite phase offset and clamped manual BPM.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let mode = match self.mode {
            BpmOverrideMode::Auto => BpmOverrideMode::Auto,
            BpmOverrideMode::Manual { bpm } => BpmOverrideMode::Manual {
                bpm: sanitize_manual_bpm(bpm),
            },
            BpmOverrideMode::Off => BpmOverrideMode::Off,
        };
        let phase_offset_beats = if self.phase_offset_beats.is_finite() {
            self.phase_offset_beats
        } else {
            0.0
        };
        Self {
            mode,
            phase_offset_beats,
            resync_epoch_ms: self.resync_epoch_ms,
        }
    }

    /// Short mode label for state-machine diagnostics.
    #[must_use]
    pub const fn mode_name(&self) -> &'static str {
        match self.mode {
            BpmOverrideMode::Auto => "auto",
            BpmOverrideMode::Manual { .. } => "manual",
            BpmOverrideMode::Off => "off",
        }
    }
}

/// Resolved per-frame BPM and beat-phase uniform values packed into `chroma_BPM` (`vec4`).
///
/// Layout in `chroma_BPM`:
/// - `.x` = `bpm` (effective BPM; `0.0` in `Off` mode)
/// - `.y` = `beat_time` (continuous beat counter with phase scrub and resync applied)
/// - `.z` = `on_beat` (exponentially decaying beat pulse in `[0.0, 1.0]`; `0.0` in `Off` mode)
/// - `.w` = `bpm_sin` (beat-locked sine oscillator in `[0.0, 1.0]`; `0.0` in `Off` mode)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedBpmUniforms {
    pub bpm: f32,
    pub beat_time: f32,
    pub on_beat: f32,
    pub bpm_sin: f32,
}

/// Clamp and sanitize a manual BPM value into `[MIN_MANUAL_BPM, MAX_MANUAL_BPM]`.
#[must_use]
pub fn sanitize_manual_bpm(bpm: f32) -> f32 {
    if bpm.is_finite() && bpm > 0.0 {
        bpm.clamp(MIN_MANUAL_BPM, MAX_MANUAL_BPM)
    } else {
        DEFAULT_MANUAL_BPM
    }
}

/// Compute the effective per-frame BPM, beat-time, beat-pulse, and beat-sine
/// uniforms from elapsed shell time, live audio-detected BPM, and the active
/// [`BpmOverrideState`].
#[must_use]
pub fn resolve_bpm_uniforms(
    elapsed_secs: f32,
    audio_bpm: f32,
    override_state: &BpmOverrideState,
) -> ResolvedBpmUniforms {
    let sanitized = override_state.sanitized();
    let effective_bpm = match sanitized.mode {
        BpmOverrideMode::Off => {
            return ResolvedBpmUniforms {
                bpm: 0.0,
                beat_time: 0.0,
                on_beat: 0.0,
                bpm_sin: 0.0,
            };
        }
        BpmOverrideMode::Auto => {
            if audio_bpm.is_finite() && audio_bpm > 0.0 {
                audio_bpm
            } else {
                0.0
            }
        }
        BpmOverrideMode::Manual { bpm } => bpm,
    };

    if effective_bpm <= 0.0 {
        return ResolvedBpmUniforms {
            bpm: 0.0,
            beat_time: 0.0,
            on_beat: 0.0,
            bpm_sin: 0.0,
        };
    }

    let elapsed_s = if elapsed_secs.is_finite() {
        f64::from(elapsed_secs.max(0.0))
    } else {
        0.0
    };
    let resync_secs = (sanitized.resync_epoch_ms as f64) / 1000.0;
    let dt_secs = (elapsed_s - resync_secs).max(0.0);
    let beats_per_sec = f64::from(effective_bpm) / 60.0;
    let beat_time = (dt_secs * beats_per_sec) as f32 + sanitized.phase_offset_beats;
    let phase = beat_time.rem_euclid(1.0);
    let on_beat = (-phase * ON_BEAT_DECAY_RATE).exp().clamp(0.0, 1.0);
    let bpm_sin = (0.5 + 0.5 * (phase * std::f32::consts::TAU).sin()).clamp(0.0, 1.0);

    ResolvedBpmUniforms {
        bpm: effective_bpm,
        beat_time,
        on_beat,
        bpm_sin,
    }
}

/// Pre-loaded intermediate render pass declared in `scene.json` `PASSES`.
#[derive(Debug, Clone)]
pub struct ScenePassSource {
    pub label: String,
    pub fragment_glsl: String,
    pub target_format: TargetFormat,
    pub feedback_self: bool,
}

/// Explicit lifecycle state of the shell GPU preview engine.
#[derive(Debug, Clone)]
pub enum GpuRenderState {
    /// No scene loaded, or the loaded scene carries no `main.glsl` shader.
    Idle,
    /// A `.chroma` scene's `main.glsl` (plus optional `PASSES`) is armed and ready to render on GPU.
    Active {
        scene_id: String,
        shader_path: PathBuf,
        fragment_source: String,
        extra_passes: Vec<ScenePassSource>,
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

/// Preprocessed multi-pass shader sources cached across frames so `#include`
/// resolution and `.synScene` ABI adaptation run once per scene load rather
/// than re-reading disk every frame.
#[derive(Debug, Clone)]
struct CachedMultipassSources {
    passes: Vec<ScenePassSource>,
    final_fragment_glsl: String,
}

/// Build the [`MultipassSpec`] for a scene's cached `PASSES` chain: every
/// intermediate pass may sample all earlier passes, and the scene's
/// `main.glsl` runs last sampling all of them. `uniforms` fixes each
/// pass's uniform-block layout for [`MultipassRenderer::compile_cached_multipass`].
fn build_multipass_spec(
    sources: &CachedMultipassSources,
    scene_id: &str,
    uniforms: &[UniformInput],
    target_size: (u32, u32),
) -> MultipassSpec {
    let bindings_before = |count: usize| -> Vec<PassInputBinding> {
        (0..count)
            .map(|prev| PassInputBinding {
                source_pass_index: prev,
                sampler_uniform_name: sources.passes[prev].label.clone(),
            })
            .collect()
    };
    let mut passes: Vec<PassDef> = Vec::with_capacity(sources.passes.len() + 1);
    for (idx, p) in sources.passes.iter().enumerate() {
        passes.push(PassDef {
            label: Some(p.label.clone()),
            fragment_glsl: p.fragment_glsl.clone(),
            uniforms: uniforms.to_vec(),
            input_textures: bindings_before(idx),
            target_format: p.target_format,
            feedback_self: p.feedback_self,
        });
    }
    passes.push(PassDef {
        label: Some(scene_id.to_string()),
        fragment_glsl: sources.final_fragment_glsl.clone(),
        uniforms: uniforms.to_vec(),
        input_textures: bindings_before(sources.passes.len()),
        target_format: TargetFormat::Rgba8Unorm,
        feedback_self: false,
    });
    MultipassSpec {
        passes,
        target_size,
        frame_count: 1,
    }
}

/// Maximum number of per-resolution persistent multipass pipelines kept
/// alive at once. Several consumers (Perform hero, stage dock, separate
/// output window) can request frames at different sizes from the same
/// engine; keying the persistent pipeline by size keeps each consumer's
/// feedback history intact instead of resetting it on every alternation.
const MAX_CACHED_MULTIPASS_SIZES: usize = 3;

/// Long-lived GPU preview engine owned by [`crate::state::AppState`].
pub struct GpuPreviewEngine {
    state: GpuRenderState,
    renderer: Option<MultipassRenderer>,
    cached_pass: Option<CachedSinglePass>,
    cached_multipass_sources: Option<CachedMultipassSources>,
    /// Compiled multipass pipelines with persistent feedback history,
    /// most-recently-used last, at most [`MAX_CACHED_MULTIPASS_SIZES`].
    cached_multipass: Vec<CachedMultipass>,
    multipass_frame_index: u32,
    envelopes: SynAudioEnvelopeBank,
    last_elapsed_secs: f32,
    transition_orchestrator: TransitionOrchestrator,
    resource_dir: Option<PathBuf>,
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
        let idle_spec = MultipassSpec {
            passes: vec![PassDef {
                label: Some("idle".into()),
                fragment_glsl: String::new(),
                uniforms: Vec::new(),
                input_textures: Vec::new(),
                target_format: TargetFormat::Rgba8Unorm,
                feedback_self: false,
            }],
            target_size: (640, 360),
            frame_count: 1,
        };
        Self {
            state: GpuRenderState::Idle,
            renderer: None,
            cached_pass: None,
            cached_multipass_sources: None,
            cached_multipass: Vec::new(),
            multipass_frame_index: 0,
            envelopes: SynAudioEnvelopeBank::new(),
            last_elapsed_secs: 0.0,
            transition_orchestrator: TransitionOrchestrator::new(idle_spec),
            resource_dir: None,
        }
    }

    /// Install the packaged Tauri `resource_dir` for resolving bundled shader includes.
    pub fn set_resource_dir(&mut self, dir: PathBuf) {
        self.resource_dir = Some(dir);
        self.cached_multipass_sources = None;
        self.cached_multipass.clear();
    }

    /// Current transition-orchestrator phase label for diagnostics.
    #[must_use]
    pub fn transition_phase_name(&self) -> &'static str {
        self.transition_orchestrator.current_state().label()
    }

    /// Resolve `#include "lygia/..."` and `#include "hg_sdf/..."` directives
    /// and inject SSF helper functions when referenced.
    fn preprocess_shader_source(&self, source: &str) -> Result<String, String> {
        self.preprocess_shader_source_for_pass(source, 0)
    }

    /// Same as [`Self::preprocess_shader_source`] with an explicit `PASSINDEX`
    /// for multi-pass `.synScene` pipelines.
    fn preprocess_shader_source_for_pass(
        &self,
        source: &str,
        pass_index: u32,
    ) -> Result<String, String> {
        preprocess_shader_with_helpers(source, self.resource_dir.as_deref(), pass_index)
    }

    /// Arm the GPU preview engine from a loaded `.chroma` scene directory.
    ///
    /// If `<scene_dir>/main.glsl` exists, reads it, cues the ADR-007
    /// [`TransitionOrchestrator`], and transitions to [`GpuRenderState::Active`].
    /// If no `main.glsl` exists, transitions cleanly to [`GpuRenderState::Idle`].
    pub fn set_scene(&mut self, scene_id: String, scene_dir: &Path) {
        self.cached_pass = None;
        self.cached_multipass_sources = None;
        self.cached_multipass.clear();
        self.multipass_frame_index = 0;
        let shader_path = scene_dir.join("main.glsl");
        if !shader_path.is_file() {
            self.state = GpuRenderState::Idle;
            return;
        }
        match std::fs::read_to_string(&shader_path) {
            Ok(fragment_source) => {
                let extra_passes = load_scene_extra_passes(scene_dir);
                let next_spec = MultipassSpec {
                    passes: vec![PassDef {
                        label: Some(scene_id.clone()),
                        fragment_glsl: fragment_source.clone(),
                        uniforms: Vec::new(),
                        input_textures: Vec::new(),
                        target_format: TargetFormat::Rgba8Unorm,
                        feedback_self: false,
                    }],
                    target_size: (640, 360),
                    frame_count: 1,
                };
                if self.transition_orchestrator.cue_next(next_spec).is_ok() {
                    let _ = self.transition_orchestrator.mark_primed();
                    let _ = self.transition_orchestrator.commit_hard_cut();
                }
                self.state = GpuRenderState::Active {
                    scene_id,
                    shader_path,
                    fragment_source,
                    extra_passes,
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
        self.cached_multipass_sources = None;
        self.cached_multipass.clear();
        self.multipass_frame_index = 0;
        self.state = GpuRenderState::Active {
            scene_id,
            shader_path: PathBuf::from("<inline>"),
            fragment_source,
            extra_passes: Vec::new(),
            frames_rendered: 0,
            last_frame_ms: 0.0,
        };
    }

    /// Disarm the GPU preview engine back to [`GpuRenderState::Idle`].
    pub fn clear_scene(&mut self) {
        self.cached_pass = None;
        self.cached_multipass_sources = None;
        self.cached_multipass.clear();
        self.multipass_frame_index = 0;
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

    /// Render a single RGBA8 frame of the active shader at `(width, height)`
    /// using default (`Auto`, zero phase-scrub) BPM synchronization.
    pub fn render_frame(
        &mut self,
        width: u32,
        height: u32,
        elapsed_secs: f32,
        frame: u64,
        audio: &AudioUniformSnapshot,
        controls: [f32; 16],
    ) -> Result<Vec<u8>, String> {
        self.render_frame_with_bpm(
            width,
            height,
            elapsed_secs,
            frame,
            audio,
            controls,
            &BpmOverrideState::default(),
        )
    }

    /// Render a single RGBA8 frame of the active shader at `(width, height)`
    /// applying an explicit [`BpmOverrideState`] (`Auto`, `Manual`, or `Off`
    /// plus beatmatch phase scrub and resync epoch).
    ///
    /// Packs the standard Chromaticity std140 uniform block:
    /// - `chroma_Time` (`vec4`: elapsed_s, dt, frame, aspect)
    /// - `chroma_Resolution` (`vec4`: w, h, 1/w, 1/h)
    /// - `chroma_BPM` (`vec4`: bpm, beat_time, on_beat, bpm_sin)
    /// - `chroma_AudioRMS` (`vec4`: rms, 0, 0, 0)
    /// - `chroma_AudioPeak` (`vec4`: peak, 0, 0, 0)
    /// - `chroma_AudioFFT` (`float[8]`: 8 log-spaced FFT bands)
    /// - `chroma_Ctrl0` (`vec4`: controls 0..3)
    /// - `chroma_Ctrl1` (`vec4`: controls 4..7)
    /// - `chroma_Ctrl2` (`vec4`: controls 8..11)
    /// - `chroma_Ctrl3` (`vec4`: controls 12..15, including XY.y & script.js envelopes)
    #[allow(clippy::too_many_arguments)]
    pub fn render_frame_with_bpm(
        &mut self,
        width: u32,
        height: u32,
        elapsed_secs: f32,
        frame: u64,
        audio: &AudioUniformSnapshot,
        controls: [f32; 16],
        bpm_override: &BpmOverrideState,
    ) -> Result<Vec<u8>, String> {
        let (scene_id, fragment_source, extra_passes) = match &self.state {
            GpuRenderState::Idle => return Err("gpu renderer is idle (no active main.glsl)".into()),
            GpuRenderState::Error { message, .. } => return Err(message.clone()),
            GpuRenderState::Active {
                scene_id,
                fragment_source,
                extra_passes,
                ..
            } => (
                scene_id.clone(),
                fragment_source.clone(),
                extra_passes.clone(),
            ),
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

        let dt = (elapsed_secs - self.last_elapsed_secs).clamp(1.0 / 240.0, 0.1);
        self.last_elapsed_secs = elapsed_secs;
        let resolved_bpm = resolve_bpm_uniforms(elapsed_secs, audio.bpm, bpm_override);
        let envelopes = self.envelopes.advance(audio, dt, resolved_bpm.beat_time);

        let mut uniforms = build_standard_uniforms_with_bpm(
            (w, h),
            elapsed_secs,
            frame,
            audio,
            controls,
            bpm_override,
        );
        let needs_envelopes = fragment_source.contains("chroma_AudioEnv0")
            || fragment_source.contains("uniform _ChromaSynBuiltins")
            || extra_passes.iter().any(|p| {
                p.fragment_glsl.contains("chroma_AudioEnv0")
                    || p.fragment_glsl.contains("uniform _ChromaSynBuiltins")
            });
        if needs_envelopes {
            append_audio_envelope_uniforms(&mut uniforms, &envelopes);
        }

        let started = Instant::now();

        // Multi-pass execution path when `scene.json` declares intermediate `PASSES` shaders.
        if !extra_passes.is_empty() {
            if self.cached_multipass_sources.is_none() {
                let mut resolved_passes: Vec<ScenePassSource> =
                    Vec::with_capacity(extra_passes.len());
                for (idx, p) in extra_passes.iter().enumerate() {
                    let resolved_pass = match self
                        .preprocess_shader_source_for_pass(&p.fragment_glsl, idx as u32)
                    {
                        Ok(src) => src,
                        Err(msg) => {
                            self.state = GpuRenderState::Error {
                                scene_id,
                                message: msg.clone(),
                            };
                            return Err(msg);
                        }
                    };
                    resolved_passes.push(ScenePassSource {
                        label: p.label.clone(),
                        fragment_glsl: resolved_pass,
                        target_format: p.target_format,
                        feedback_self: p.feedback_self,
                    });
                }
                let resolved_main = match self
                    .preprocess_shader_source_for_pass(&fragment_source, extra_passes.len() as u32)
                {
                    Ok(src) => src,
                    Err(msg) => {
                        self.state = GpuRenderState::Error {
                            scene_id,
                            message: msg.clone(),
                        };
                        return Err(msg);
                    }
                };
                self.cached_multipass_sources = Some(CachedMultipassSources {
                    passes: resolved_passes,
                    final_fragment_glsl: resolved_main,
                });
            }

            let Some(cached_sources) = self.cached_multipass_sources.as_ref() else {
                return Err("cached multipass sources unavailable".into());
            };
            let Some(renderer) = self.renderer.as_ref() else {
                return Err("wgpu renderer unavailable".into());
            };

            let cache_pos = self
                .cached_multipass
                .iter()
                .position(|c| c.target_size() == (w, h));
            let freshly_compiled = cache_pos.is_none();
            let mut cache = match cache_pos {
                Some(pos) => self.cached_multipass.remove(pos),
                None => {
                    let spec = build_multipass_spec(cached_sources, &scene_id, &uniforms, (w, h));
                    match renderer.compile_cached_multipass(&spec) {
                        Ok(cache) => cache,
                        Err(err) => {
                            let msg = format!("multipass compile failed: {err}");
                            self.cached_multipass_sources = None;
                            self.cached_multipass.clear();
                            self.state = GpuRenderState::Error {
                                scene_id,
                                message: msg.clone(),
                            };
                            return Err(msg);
                        }
                    }
                }
            };
            // A freshly compiled feedback pipeline records two frames on
            // its first call so the first visible frame already samples
            // real prior-frame output rather than the black cold-start
            // buffer (single-call consumers such as thumbnails rely on
            // this). Every later call advances exactly one frame, so
            // feedback history accumulates across calls without a cap.
            let frames = if freshly_compiled && cache.has_feedback() {
                2
            } else {
                1
            };
            let result = cache
                .render_frames(renderer, &uniforms, frames)
                .map_err(|err| format!("multipass render failed: {err}"));
            if result.is_ok() {
                if self.cached_multipass.len() >= MAX_CACHED_MULTIPASS_SIZES {
                    self.cached_multipass.remove(0);
                }
                self.cached_multipass.push(cache);
            }
            match result {
                Ok(bytes) => {
                    let elapsed_ms = started.elapsed().as_secs_f32() * 1000.0;
                    self.multipass_frame_index = self.multipass_frame_index.saturating_add(1);
                    if let GpuRenderState::Active {
                        frames_rendered,
                        last_frame_ms,
                        ..
                    } = &mut self.state
                    {
                        *frames_rendered = frames_rendered.saturating_add(1);
                        *last_frame_ms = elapsed_ms;
                    }
                    return Ok(bytes);
                }
                Err(msg) => {
                    self.cached_multipass_sources = None;
                    self.cached_multipass.clear();
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

        if self.cached_pass.is_none() {
            let resolved_source = match self.preprocess_shader_source(&fragment_source) {
                Ok(src) => src,
                Err(msg) => {
                    self.state = GpuRenderState::Error {
                        scene_id,
                        message: msg.clone(),
                    };
                    return Err(msg);
                }
            };
            match renderer.compile_cached_single_pass(
                &scene_id,
                &resolved_source,
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

/// Construct the standard Chromaticity std140 uniform vector for a frame
/// using default (`Auto`, zero phase-scrub) BPM synchronization.
#[must_use]
pub fn build_standard_uniforms(
    target_size: (u32, u32),
    elapsed_secs: f32,
    frame: u64,
    audio: &AudioUniformSnapshot,
    controls: [f32; 16],
) -> Vec<UniformInput> {
    build_standard_uniforms_with_bpm(
        target_size,
        elapsed_secs,
        frame,
        audio,
        controls,
        &BpmOverrideState::default(),
    )
}

/// Construct the standard Chromaticity std140 uniform vector for a frame
/// applying an explicit [`BpmOverrideState`] to `chroma_BPM`
/// (`[bpm, beat_time, on_beat, bpm_sin]`).
#[must_use]
pub fn build_standard_uniforms_with_bpm(
    target_size: (u32, u32),
    elapsed_secs: f32,
    frame: u64,
    audio: &AudioUniformSnapshot,
    controls: [f32; 16],
    bpm_override: &BpmOverrideState,
) -> Vec<UniformInput> {
    let w = target_size.0 as f32;
    let h = target_size.1 as f32;
    let aspect = if h > 0.0 { w / h } else { 1.0 };
    let resolved_bpm = resolve_bpm_uniforms(elapsed_secs, audio.bpm, bpm_override);

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
            name: "chroma_BPM".to_string(),
            value: UniformValue::Vec4([
                resolved_bpm.bpm,
                resolved_bpm.beat_time,
                resolved_bpm.on_beat,
                resolved_bpm.bpm_sin,
            ]),
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
        UniformInput {
            name: "chroma_Ctrl2".to_string(),
            value: UniformValue::Vec4([controls[8], controls[9], controls[10], controls[11]]),
        },
        UniformInput {
            name: "chroma_Ctrl3".to_string(),
            value: UniformValue::Vec4([controls[12], controls[13], controls[14], controls[15]]),
        },
    ]
}

/// Append the full [`SynAudioEnvelopes`] std140 uniform vectors (`chroma_AudioEnv0..3`
/// and `chroma_AudioMeta`) for shaders declaring extended reactive envelopes.
pub fn append_audio_envelope_uniforms(
    uniforms: &mut Vec<UniformInput>,
    envelopes: &SynAudioEnvelopes,
) {
    uniforms.push(UniformInput {
        name: "chroma_AudioEnv0".to_string(),
        value: UniformValue::Vec4([
            envelopes.level,
            envelopes.hits,
            envelopes.presence,
            envelopes.time,
        ]),
    });
    uniforms.push(UniformInput {
        name: "chroma_AudioEnv1".to_string(),
        value: UniformValue::Vec4([
            envelopes.bass_level,
            envelopes.bass_hits,
            envelopes.bass_presence,
            envelopes.bass_time,
        ]),
    });
    uniforms.push(UniformInput {
        name: "chroma_AudioEnv2".to_string(),
        value: UniformValue::Vec4([
            envelopes.mid_level,
            envelopes.mid_hits,
            envelopes.mid_presence,
            envelopes.mid_time,
        ]),
    });
    uniforms.push(UniformInput {
        name: "chroma_AudioEnv3".to_string(),
        value: UniformValue::Vec4([
            envelopes.high_level,
            envelopes.high_hits,
            envelopes.high_presence,
            envelopes.high_time,
        ]),
    });
    uniforms.push(UniformInput {
        name: "chroma_AudioMeta".to_string(),
        value: UniformValue::Vec4([
            envelopes.fade_in_out,
            envelopes.intensity,
            envelopes.bpm_confidence,
            envelopes.random_on_beat,
        ]),
    });
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
    controls: [f32; 16],
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
    vec4 chroma_BPM;
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
                .render_frame(32, 32, 0.0, 1, &AudioUniformSnapshot::default(), [0.5; 16])
                .is_err()
        );

        engine.set_inline_shader("test-scene".into(), SAMPLE_SHADER.into());
        assert_eq!(engine.state_name(), "active");

        let bytes = engine
            .render_frame(
                32,
                32,
                1.25,
                10,
                &AudioUniformSnapshot::default(),
                [0.5; 16],
            )
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
                [0.75; 16],
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
            [0.75; 16],
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
        let err = engine.render_frame(16, 16, 0.0, 1, &AudioUniformSnapshot::default(), [0.5; 16]);
        assert!(err.is_err());
        assert_eq!(engine.state_name(), "error");
        assert!(engine.status().error.is_some());
    }

    #[test]
    fn all_scenes_local_shaders_compile_and_render_on_gpu() {
        let scenes_root = std::env::var_os("CHROMATICITY_LOCAL_SCENES_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .join("..")
                    .join(".local")
                    .join("scenes")
            });
        if !scenes_root.is_dir() {
            return;
        }
        let mut entries: Vec<_> = std::fs::read_dir(&scenes_root)
            .expect("read .local/scenes")
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
                .render_frame(w, h, 2.5, 150, &audio, [0.5; 16])
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
                        .render_frame(w, h, 2.5 + (i as f32) * 0.016, 151 + i, &audio, [0.5; 16])
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

    #[test]
    fn bpm_override_auto_uses_audio_bpm_and_applies_phase_scrub() {
        let base = BpmOverrideState::default();
        // At 120 BPM (2 beats/sec), elapsed = 1.0s => beat_time = 2.0, phase = 0.0 => on_beat = 1.0.
        let r0 = resolve_bpm_uniforms(1.0, 120.0, &base);
        assert_eq!(r0.bpm, 120.0);
        assert!((r0.beat_time - 2.0).abs() < 1e-5);
        assert!((r0.on_beat - 1.0).abs() < 1e-5);
        assert!((r0.bpm_sin - 0.5).abs() < 1e-5);

        // Apply +0.25 beat phase scrub => beat_time = 2.25, phase = 0.25 => sin(pi/2) = 1.0 => bpm_sin = 1.0.
        let scrubbed = BpmOverrideState {
            mode: BpmOverrideMode::Auto,
            phase_offset_beats: 0.25,
            resync_epoch_ms: 0,
        };
        let r1 = resolve_bpm_uniforms(1.0, 120.0, &scrubbed);
        assert_eq!(r1.bpm, 120.0);
        assert!((r1.beat_time - 2.25).abs() < 1e-5);
        assert!(r1.on_beat < 0.3 && r1.on_beat > 0.1);
        assert!((r1.bpm_sin - 1.0).abs() < 1e-5);

        // Negative phase scrub wraps cleanly into [0, 1) without NaN/negative phase.
        let neg = BpmOverrideState {
            mode: BpmOverrideMode::Auto,
            phase_offset_beats: -0.25,
            resync_epoch_ms: 0,
        };
        let r2 = resolve_bpm_uniforms(0.0, 120.0, &neg);
        assert!((r2.beat_time - (-0.25)).abs() < 1e-5);
        assert!(r2.on_beat >= 0.0 && r2.on_beat <= 1.0);
        assert!(r2.bpm_sin.abs() < 1e-5);
    }

    #[test]
    fn bpm_override_manual_locks_tempo_and_honors_resync_epoch() {
        let manual = BpmOverrideState {
            mode: BpmOverrideMode::Manual { bpm: 128.0 },
            phase_offset_beats: 0.5,
            resync_epoch_ms: 2000,
        };
        // Audio reports 95 BPM, but Manual locks to 128 BPM; resynced at t=2.0s.
        // At elapsed = 2.0s => dt = 0.0s => beat_time = 0.5, phase = 0.5 => bpm_sin = 0.5.
        let r_at_resync = resolve_bpm_uniforms(2.0, 95.0, &manual);
        assert_eq!(r_at_resync.bpm, 128.0);
        assert!((r_at_resync.beat_time - 0.5).abs() < 1e-5);
        assert!((r_at_resync.bpm_sin - 0.5).abs() < 1e-5);

        // 0.46875s later (= 1 beat at 128 BPM): beat_time advances by exactly 1.0 beat.
        let r_next_beat = resolve_bpm_uniforms(2.468_75, 95.0, &manual);
        assert_eq!(r_next_beat.bpm, 128.0);
        assert!((r_next_beat.beat_time - 1.5).abs() < 1e-4);

        // Verify packing into `chroma_BPM` uniform vec4.
        let audio = AudioUniformSnapshot {
            bpm: 95.0,
            ..AudioUniformSnapshot::default()
        };
        let uniforms =
            build_standard_uniforms_with_bpm((64, 36), 2.0, 120, &audio, [0.5; 16], &manual);
        let bpm_vec = uniforms
            .iter()
            .find(|u| u.name == "chroma_BPM")
            .expect("chroma_BPM uniform present");
        match bpm_vec.value {
            UniformValue::Vec4([bpm, beat_time, on_beat, bpm_sin]) => {
                assert_eq!(bpm, 128.0);
                assert!((beat_time - 0.5).abs() < 1e-5);
                assert!(on_beat > 0.0 && on_beat < 0.1);
                assert!((bpm_sin - 0.5).abs() < 1e-5);
            }
            ref other => panic!("expected Vec4 for chroma_BPM, got {other:?}"),
        }
    }

    #[test]
    fn bpm_override_off_zeroes_bpm_and_disables_beat_pulses() {
        let off = BpmOverrideState {
            mode: BpmOverrideMode::Off,
            phase_offset_beats: 0.75,
            resync_epoch_ms: 500,
        };
        let resolved = resolve_bpm_uniforms(4.0, 140.0, &off);
        assert_eq!(resolved.bpm, 0.0);
        assert_eq!(resolved.beat_time, 0.0);
        assert_eq!(resolved.on_beat, 0.0);
        assert_eq!(resolved.bpm_sin, 0.0);

        let audio = AudioUniformSnapshot {
            bpm: 140.0,
            ..AudioUniformSnapshot::default()
        };
        let uniforms =
            build_standard_uniforms_with_bpm((64, 36), 4.0, 240, &audio, [0.5; 16], &off);
        let bpm_vec = uniforms
            .iter()
            .find(|u| u.name == "chroma_BPM")
            .expect("chroma_BPM uniform present");
        match bpm_vec.value {
            UniformValue::Vec4(v) => assert_eq!(v, [0.0, 0.0, 0.0, 0.0]),
            ref other => panic!("expected Vec4 for chroma_BPM, got {other:?}"),
        }
    }

    #[test]
    fn grandmaster_flagship_scenes_validate_script_and_render_on_metal() {
        use crate::script_host::ScriptHost;
        use chromaticity_scene::loader::load_chroma_dir;
        use chromaticity_scene::validator::{ValidationMode, validate};
        use std::collections::BTreeSet;

        let scenes_root = std::env::var_os("CHROMATICITY_LOCAL_SCENES_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .join("..")
                    .join(".local")
                    .join("scenes")
            });
        if !scenes_root.is_dir() {
            return;
        }
        let flagships = [
            "gm-kerr-hologram-orrery.chroma",
            "gm-dyson-prismatic-lotus.chroma",
            "gm-hyperbolic-opal-cathedral.chroma",
            "gm-quantum-ferro-nebula.chroma",
        ];

        let mut engine = GpuPreviewEngine::new();
        let mut script_host = ScriptHost::new();

        for folder in flagships {
            let dir = scenes_root.join(folder);
            assert!(
                dir.is_dir(),
                "missing flagship scene dir: {}",
                dir.display()
            );

            // 1. Strict schema validation via chromaticity-scene
            let scene =
                load_chroma_dir(&dir).unwrap_or_else(|e| panic!("failed to load {folder}: {e}"));
            let warnings = validate(&scene, ValidationMode::Strict)
                .unwrap_or_else(|e| panic!("strict validation failed for {folder}: {e:?}"));
            assert!(
                warnings.is_empty(),
                "{folder} produced strict warnings: {warnings:?}"
            );
            assert_eq!(
                scene.controls.len(),
                12,
                "{folder} should declare 12 controls"
            );

            let groups: BTreeSet<String> = scene
                .controls
                .iter()
                .map(|c| c.ui_group.clone().unwrap_or_default())
                .collect();
            assert_eq!(
                groups,
                ["Geometry", "Motion", "Optics", "Performance"]
                    .into_iter()
                    .map(String::from)
                    .collect::<BTreeSet<_>>(),
                "{folder} must organize controls into the 4 canonical UI_GROUPs"
            );
            for c in &scene.controls {
                assert!(
                    c.description
                        .as_deref()
                        .is_some_and(|d| !d.trim().is_empty()),
                    "{folder} control {} missing DESCRIPTION",
                    c.name
                );
            }
            let presets = scene
                .extensions
                .get("PRESETS")
                .and_then(|v| v.as_array())
                .unwrap_or_else(|| panic!("{folder} missing PRESETS array"));
            assert_eq!(presets.len(), 5, "{folder} should ship 5 factory presets");

            // 2. QuickJS script.js lifecycle + deterministic drift & transient decay
            script_host.set_scene(folder.to_string(), &dir);
            assert_eq!(
                script_host.state_name(),
                "active",
                "{folder} script.js failed to arm: {:?}",
                script_host.status().error
            );
            let mut controls = [0.5_f32; 16];
            for step in 0..30u64 {
                let peak = if step == 10 { 0.92 } else { 0.25 };
                let audio = AudioUniformSnapshot {
                    bpm: 126.0,
                    rms: if step == 10 { 0.78 } else { 0.35 },
                    peak,
                    fft_bands: [0.6, 0.5, 0.4, 0.35, 0.3, 0.25, 0.2, 0.15],
                    ..AudioUniformSnapshot::default()
                };
                let emitted =
                    script_host.tick(step as f32 * (1.0 / 60.0), 1.0 / 60.0, step, &audio);
                assert_eq!(
                    script_host.state_name(),
                    "active",
                    "{folder} script.js errored on tick {step}"
                );
                for (i, slot) in controls.iter_mut().enumerate().skip(13) {
                    let key = format!("ctrl_{i}");
                    if let Some(&val) = emitted.get(&key) {
                        assert!(val.is_finite(), "{folder} emitted non-finite {key}");
                        *slot = val;
                    }
                }
            }
            assert!(
                (controls[13] - 0.5).abs() > 1e-4 || (controls[14] - 0.5).abs() > 1e-4,
                "{folder} script.js should emit non-zero beat drift"
            );
            assert!(
                controls[15] > 0.5,
                "{folder} script.js should emit positive transient decay envelope after peak"
            );

            // 3. Live Metal wgpu compilation & render with all 16 control floats
            engine.set_scene(folder.to_string(), &dir);
            let audio = AudioUniformSnapshot {
                bpm: 126.0,
                rms: 0.48,
                peak: 0.75,
                fft_bands: [0.65, 0.55, 0.45, 0.38, 0.32, 0.28, 0.22, 0.18],
                ..AudioUniformSnapshot::default()
            };
            let bytes = engine
                .render_frame(640, 360, 2.5, 150, &audio, controls)
                .unwrap_or_else(|e| panic!("{folder} Metal compile/render failed: {e}"));
            assert_eq!(bytes.len(), 640 * 360 * 4);
            assert!(
                bytes
                    .chunks_exact(4)
                    .any(|px| px[0] > 0 || px[1] > 0 || px[2] > 0),
                "{folder} rendered all-black frame"
            );
        }
    }

    #[test]
    fn all_25_bundled_scenes_compile_and_render_on_gpu() {
        let bundled_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources/scenes");
        let mut scene_dirs = Vec::new();
        for sub in ["library", "welcome", "examples"] {
            let dir = bundled_root.join(sub);
            for entry in std::fs::read_dir(&dir).expect("read bundled subdir") {
                let p = entry.expect("dir entry").path();
                if p.extension().and_then(|s| s.to_str()) == Some("chroma") {
                    scene_dirs.push(p);
                }
            }
        }
        scene_dirs.sort();
        assert_eq!(
            scene_dirs.len(),
            25,
            "expected all 25 bundled .chroma scenes"
        );

        let mut engine = GpuPreviewEngine::new();
        let audio = AudioUniformSnapshot {
            bpm: 124.0,
            rms: 0.45,
            peak: 0.72,
            fft_bands: [0.6, 0.5, 0.4, 0.35, 0.3, 0.25, 0.2, 0.15],
            ..AudioUniformSnapshot::default()
        };
        for scene_dir in &scene_dirs {
            let id = scene_dir
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("scene")
                .to_string();
            assert!(
                scene_dir.join("main.glsl").is_file(),
                "bundled scene missing main.glsl: {}",
                scene_dir.display()
            );
            engine.set_scene(id.clone(), scene_dir);
            let bytes = engine
                .render_frame(160, 90, 1.5, 90, &audio, [0.5; 16])
                .unwrap_or_else(|e| panic!("bundled scene {id} failed Metal compile/render: {e}"));
            assert_eq!(bytes.len(), 160 * 90 * 4);
            assert!(
                bytes
                    .chunks_exact(4)
                    .any(|px| px[0] > 0 || px[1] > 0 || px[2] > 0),
                "bundled scene {id} rendered all-black frame"
            );
        }
    }

    #[test]
    fn ssf_helper_prelude_and_audio_envelope_uniforms_render_on_gpu() {
        let mut engine = GpuPreviewEngine::new();
        let shader = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 0) uniform Uniforms {
    vec4 chroma_Time;
    vec4 chroma_Resolution;
    vec4 chroma_BPM;
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    vec4 chroma_AudioFFT[2];
    vec4 chroma_Ctrl0;
    vec4 chroma_Ctrl1;
    vec4 chroma_Ctrl2;
    vec4 chroma_Ctrl3;
    vec4 chroma_AudioEnv0;
    vec4 chroma_AudioEnv1;
    vec4 chroma_AudioEnv2;
    vec4 chroma_AudioEnv3;
    vec4 chroma_AudioMeta;
};

void main() {
    vec2 p = _rotate(uv - 0.5, chroma_Time.x + chroma_AudioEnv1.w);
    float n = _fbm(p * 4.0);
    float h = _map(n + chroma_AudioEnv0.y, 0.0, 2.0, 0.0, 1.0);
    vec3 col = _hsv2rgb(vec3(h, 0.85, 0.6 + 0.4 * chroma_AudioMeta.y));
    fragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
"#;
        engine.set_inline_shader("ssf-env-test".into(), shader.into());
        let audio = AudioUniformSnapshot {
            bpm: 128.0,
            rms: 0.65,
            peak: 0.9,
            fft_bands: [0.85, 0.75, 0.55, 0.45, 0.35, 0.25, 0.2, 0.15],
            ..AudioUniformSnapshot::default()
        };
        let bytes = engine
            .render_frame(160, 90, 1.0, 60, &audio, [0.5; 16])
            .expect("SSF helper prelude + AudioEnv shader must compile and render on Metal");
        assert_eq!(bytes.len(), 160 * 90 * 4);
        assert!(
            bytes
                .chunks_exact(4)
                .any(|px| px[0] > 0 || px[1] > 0 || px[2] > 0),
            "SSF helper + AudioEnv shader rendered all-black"
        );
    }

    #[test]
    fn multipass_scene_passes_compile_and_render_on_gpu() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "chromaticity-multipass-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let scene_json = r#"{
            "TITLE": "Multipass Test",
            "CREDIT": "Chromaticity",
            "DESCRIPTION": "Two-pass GPU test",
            "PASSES": [
                { "TARGET": "pass0", "FLOAT": true }
            ]
        }"#;
        let pass0_glsl = r#"#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0, std140) uniform Uniforms {
    vec4 chroma_Time;
    vec4 chroma_Resolution;
    vec4 chroma_BPM;
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    float chroma_AudioFFT[8];
    vec4 chroma_Ctrl0;
    vec4 chroma_Ctrl1;
    vec4 chroma_Ctrl2;
    vec4 chroma_Ctrl3;
};
void main() {
    fragColor = vec4(0.2 + 0.6 * uv.x, 0.3 + 0.5 * uv.y, 0.8, 1.0);
}
"#;
        let main_glsl = r#"#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0, std140) uniform Uniforms {
    vec4 chroma_Time;
    vec4 chroma_Resolution;
    vec4 chroma_BPM;
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    float chroma_AudioFFT[8];
    vec4 chroma_Ctrl0;
    vec4 chroma_Ctrl1;
    vec4 chroma_Ctrl2;
    vec4 chroma_Ctrl3;
};
layout(set = 0, binding = 1) uniform texture2D pass0_tex;
layout(set = 0, binding = 2) uniform sampler pass0_samp;
void main() {
    vec4 prev = texture(sampler2D(pass0_tex, pass0_samp), uv);
    fragColor = vec4(prev.rgb * (0.8 + 0.2 * sin(chroma_Time.x)), 1.0);
}
"#;
        std::fs::write(tmp_dir.join("scene.json"), scene_json).unwrap();
        std::fs::write(tmp_dir.join("pass0.glsl"), pass0_glsl).unwrap();
        std::fs::write(tmp_dir.join("main.glsl"), main_glsl).unwrap();

        let mut engine = GpuPreviewEngine::new();
        engine.set_scene("multipass-test".into(), &tmp_dir);
        let bytes = engine
            .render_frame(160, 90, 1.0, 1, &AudioUniformSnapshot::default(), [0.5; 16])
            .expect("multipass scene must compile and render on Metal");
        assert_eq!(bytes.len(), 160 * 90 * 4);
        assert!(
            bytes
                .chunks_exact(4)
                .any(|px| px[0] > 20 && px[1] > 20 && px[2] > 20),
            "multipass scene rendered all-black"
        );
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn multipass_include_error_transitions_engine_to_error_state() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "chromaticity-multipass-inc-err-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let scene_json = r#"{
            "TITLE": "Multipass Include Error",
            "CREDIT": "Chromaticity",
            "DESCRIPTION": "Missing include in pass0",
            "PASSES": [
                { "TARGET": "pass0" }
            ]
        }"#;
        let pass0_glsl =
            "#version 450\n#include \"lygia/nonexistent_file_xyz.glsl\"\nvoid main() {}\n";
        let main_glsl = SAMPLE_SHADER;
        std::fs::write(tmp_dir.join("scene.json"), scene_json).unwrap();
        std::fs::write(tmp_dir.join("pass0.glsl"), pass0_glsl).unwrap();
        std::fs::write(tmp_dir.join("main.glsl"), main_glsl).unwrap();

        let mut engine = GpuPreviewEngine::new();
        engine.set_scene("multipass-inc-err".into(), &tmp_dir);
        let err = engine.render_frame(32, 32, 0.0, 1, &AudioUniformSnapshot::default(), [0.5; 16]);
        assert!(err.is_err());
        assert_eq!(engine.state_name(), "error");
        assert!(engine.status().error.is_some());
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn multipass_feedback_self_accumulates_and_caches_preprocessed_sources() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "chromaticity-multipass-feedback-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let scene_json = r#"{
            "TITLE": "Multipass Feedback Test",
            "CREDIT": "Chromaticity",
            "DESCRIPTION": "Ping-pong feedback accumulation test",
            "PASSES": [
                { "TARGET": "trail", "PERSISTENT": true }
            ]
        }"#;
        // Pass 0 adds 0.25 to its previous-frame self-feedback texture on each iteration.
        // With frame_count >= 2, iteration 0 writes 0.25 and iteration 1 reads 0.25 -> writes 0.50.
        let trail_glsl = r#"#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0, std140) uniform Uniforms {
    vec4 chroma_Time;
    vec4 chroma_Resolution;
    vec4 chroma_BPM;
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    float chroma_AudioFFT[8];
    vec4 chroma_Ctrl0;
    vec4 chroma_Ctrl1;
    vec4 chroma_Ctrl2;
    vec4 chroma_Ctrl3;
};
layout(set = 0, binding = 1) uniform texture2D prev_tex;
layout(set = 0, binding = 2) uniform sampler prev_samp;
void main() {
    vec4 prev = texture(sampler2D(prev_tex, prev_samp), uv);
    fragColor = vec4(clamp(prev.rgb + vec3(0.25), 0.0, 1.0), 1.0);
}
"#;
        let main_glsl = r#"#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0, std140) uniform Uniforms {
    vec4 chroma_Time;
    vec4 chroma_Resolution;
    vec4 chroma_BPM;
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    float chroma_AudioFFT[8];
    vec4 chroma_Ctrl0;
    vec4 chroma_Ctrl1;
    vec4 chroma_Ctrl2;
    vec4 chroma_Ctrl3;
};
layout(set = 0, binding = 1) uniform texture2D trail_tex;
layout(set = 0, binding = 2) uniform sampler trail_samp;
void main() {
    fragColor = vec4(texture(sampler2D(trail_tex, trail_samp), uv).rgb, 1.0);
}
"#;
        std::fs::write(tmp_dir.join("scene.json"), scene_json).unwrap();
        std::fs::write(tmp_dir.join("trail.glsl"), trail_glsl).unwrap();
        std::fs::write(tmp_dir.join("main.glsl"), main_glsl).unwrap();

        let mut engine = GpuPreviewEngine::new();
        engine.set_scene("multipass-feedback".into(), &tmp_dir);
        let bytes1 = engine
            .render_frame(32, 32, 0.0, 1, &AudioUniformSnapshot::default(), [0.5; 16])
            .expect("feedback multipass frame 1 must render");
        assert!(engine.cached_multipass_sources.is_some());
        assert_eq!(engine.multipass_frame_index, 1);
        // Frame 1 runs 2 ping-pong iterations => 0.50 (~128), strictly above single-pass 0.25 (~64).
        assert!(
            bytes1[0] >= 115,
            "expected feedback accumulation >= 115 (2 passes), got {}",
            bytes1[0]
        );

        let bytes2 = engine
            .render_frame(
                32,
                32,
                0.016,
                2,
                &AudioUniformSnapshot::default(),
                [0.5; 16],
            )
            .expect("feedback multipass frame 2 must reuse cached sources");
        assert_eq!(engine.multipass_frame_index, 2);
        assert!(bytes2[0] >= 115);
        // Persistent history: call 1 recorded 2 frames (0.50), call 2 one
        // more (0.75). The pre-cache path re-simulated from black every
        // call and could never exceed 4 iterations.
        assert!(
            (191..=193).contains(&bytes2[0]),
            "call 2 must continue feedback history to ~0.75 (192), got {}",
            bytes2[0]
        );

        // A consumer at another size gets its own pipeline (cold start,
        // 2 warm frames => 0.50) without disturbing the 32x32 history.
        let other = engine
            .render_frame(
                48,
                24,
                0.032,
                3,
                &AudioUniformSnapshot::default(),
                [0.5; 16],
            )
            .expect("second-size multipass frame must render");
        assert!(
            (127..=129).contains(&other[0]),
            "fresh 48x24 pipeline must cold-start at ~0.50 (128), got {}",
            other[0]
        );
        assert_eq!(engine.cached_multipass.len(), 2);
        let bytes3 = engine
            .render_frame(
                32,
                32,
                0.048,
                4,
                &AudioUniformSnapshot::default(),
                [0.5; 16],
            )
            .expect("feedback multipass frame 3 must render");
        assert_eq!(
            bytes3[0], 255,
            "32x32 history must survive the interleaved size and reach 1.0"
        );

        // Re-arming the scene drops every persistent pipeline (cold start).
        engine.set_scene("multipass-feedback".into(), &tmp_dir);
        assert!(engine.cached_multipass.is_empty());
        let restarted = engine
            .render_frame(
                32,
                32,
                0.064,
                5,
                &AudioUniformSnapshot::default(),
                [0.5; 16],
            )
            .expect("re-armed scene must render");
        assert_eq!(
            restarted[0], bytes1[0],
            "set_scene must restore the cold start"
        );

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
