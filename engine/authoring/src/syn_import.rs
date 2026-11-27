// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Pure-Rust `.synScene` → `.chroma` compatibility importer.
//!
//! Converts an on-disk Synesthesia `.synScene` directory (`scene.json`,
//! `main.glsl`, optional `script.js`, and optional image/media assets)
//! into a non-destructive `.chroma` package under a caller-supplied
//! target parent directory.
//!
//! Key compatibility transforms applied to `main.glsl`:
//!
//! 1. **Vulkan GLSL 4.50 / `naga` compat preamble** declaring
//!    `RENDERSIZE`, `TIME`, `PASSINDEX`, `_uv`, `_uvc`, `_xy`, `PI`,
//!    `TAU`, `syn_*` reactive uniforms, Synesthesia helper functions
//!    (`_rotate`, `_scale`, `_fbm`, `_pixelate`, `_nclamp`,
//!    `_isMediaActive`, `_loadMedia`, `_loadMediaAsMask`), and
//!    auto-declared uniforms for every control in `scene.json`.
//! 2. **Metal-safe `pow` guard** (`_chroma_safe_pow`) rewriting
//!    standalone `pow(` calls in the scene body so negative bases never
//!    produce Metal `NaN`s.
//! 3. **Collision-free fragment output** (`layout(location = 0) out
//!    vec4 _chroma_fragColor;`) wrapping `void main() { ...
//!    _chroma_fragColor = renderMain(); }` so scenes that declare a
//!    global `vec4 fragColor;` compile without symbol collision.

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chromaticity_scene::loader::parse_scene_json;
use chromaticity_scene::schema::{ControlDecl, ControlKind, ControlValue, Scene};
use chromaticity_scene::validator::{ValidationMode, validate};

/// Summary returned on a successful `.synScene` → `.chroma` import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynImportReport {
    /// Sanitized package slug used for `<slug>.chroma`.
    pub slug: String,
    /// Full path to the created `<slug>.chroma` directory.
    pub output_dir: PathBuf,
    /// Path to the emitted `scene.json`.
    pub scene_json_path: PathBuf,
    /// Path to the emitted `main.glsl`.
    pub main_glsl_path: PathBuf,
    /// Whether `script.js` was present in the source and copied over.
    pub script_js_copied: bool,
    /// Relative paths of copied image/media or auxiliary files.
    pub copied_assets: Vec<PathBuf>,
    /// Control uniform names auto-declared in the preamble.
    pub declared_control_uniforms: Vec<String>,
    /// Number of standalone `pow(` call sites rewritten to `_chroma_safe_pow(`.
    pub pow_calls_rewritten: usize,
    /// Non-fatal compatibility warnings collected during import.
    pub warnings: Vec<String>,
}

/// Errors returned by [`import_syn_scene_to_chroma`].
#[derive(Debug)]
#[non_exhaustive]
pub enum SynImportError {
    /// Source path does not exist or is not a directory.
    SourceNotDirectory(PathBuf),
    /// Source directory is missing `scene.json`.
    MissingSceneJson(PathBuf),
    /// Source directory is missing `main.glsl`.
    MissingMainGlsl(PathBuf),
    /// Target `<slug>.chroma` directory already exists (non-destructive guard).
    TargetAlreadyExists(PathBuf),
    /// Failed to parse `scene.json`.
    SceneParse(String),
    /// `scene.json` failed `Compat` validation.
    SceneValidation(String),
    /// Rewritten `main.glsl` failed `naga` GLSL 4.50 parse or validation.
    GlslCompileError(String),
    /// Filesystem I/O failure.
    Io(io::Error),
}

impl fmt::Display for SynImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceNotDirectory(p) => {
                write!(
                    f,
                    "source .synScene path is not a directory: {}",
                    p.display()
                )
            }
            Self::MissingSceneJson(p) => {
                write!(f, "source .synScene missing scene.json: {}", p.display())
            }
            Self::MissingMainGlsl(p) => {
                write!(f, "source .synScene missing main.glsl: {}", p.display())
            }
            Self::TargetAlreadyExists(p) => {
                write!(
                    f,
                    "target .chroma directory already exists: {}",
                    p.display()
                )
            }
            Self::SceneParse(msg) => write!(f, "scene.json parse error: {msg}"),
            Self::SceneValidation(msg) => write!(f, "scene.json validation error: {msg}"),
            Self::GlslCompileError(msg) => write!(f, "GLSL compile/validation error: {msg}"),
            Self::Io(err) => write!(f, "I/O error during .synScene import: {err}"),
        }
    }
}

impl std::error::Error for SynImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for SynImportError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// Built-in Synesthesia uniform names declared by the compat preamble.
/// Used to avoid duplicate uniform declarations when a scene's `CONTROLS`
/// or shader body references the same name.
const BUILTIN_UNIFORM_NAMES: &[&str] = &[
    "RENDERSIZE",
    "TIME",
    "PASSINDEX",
    "FRAMECOUNT",
    "syn_Time",
    "syn_BeatTime",
    "syn_BassTime",
    "syn_MidTime",
    "syn_MidHighTime",
    "syn_HighTime",
    "syn_CurvedTime",
    "syn_Level",
    "syn_BassLevel",
    "syn_MidLevel",
    "syn_MidHighLevel",
    "syn_HighLevel",
    "syn_Hits",
    "syn_BassHits",
    "syn_MidHits",
    "syn_MidHighHits",
    "syn_HighHits",
    "syn_Presence",
    "syn_BassPresence",
    "syn_MidPresence",
    "syn_MidHighPresence",
    "syn_HighPresence",
    "syn_Intensity",
    "syn_OnBeat",
    "syn_ToggleOnBeat",
    "syn_RandomOnBeat",
    "syn_FadeInOut",
    "syn_BPM",
    "syn_BPMConfidence",
    "syn_BPMTwitcher",
    "syn_BPMSin",
    "syn_BPMSin2",
    "syn_BPMSin4",
    "syn_BPMSin8",
    "syn_BPMTri",
    "syn_BPMTri2",
    "syn_BPMTri4",
    "syn_BPMTri8",
    "syn_MediaActive",
    "syn_UserImage",
    "syn_Spectrum",
];

/// Static Vulkan GLSL 4.50 / `naga`-compatible Synesthesia preamble header.
const SYN_PREAMBLE_HEADER: &str = r#"#version 450

// Auto-generated by Chromaticity .synScene compatibility importer.
layout(location = 0) in vec2 _chroma_in_uv;
layout(location = 0) out vec4 _chroma_fragColor;

layout(set = 0, binding = 0, std140) uniform _ChromaSynBuiltins {
    vec2 RENDERSIZE;
    float TIME;
    float PASSINDEX;
    float FRAMECOUNT;
    float syn_Time;
    float syn_BeatTime;
    float syn_BassTime;
    float syn_MidTime;
    float syn_MidHighTime;
    float syn_HighTime;
    float syn_CurvedTime;
    float syn_Level;
    float syn_BassLevel;
    float syn_MidLevel;
    float syn_MidHighLevel;
    float syn_HighLevel;
    float syn_Hits;
    float syn_BassHits;
    float syn_MidHits;
    float syn_MidHighHits;
    float syn_HighHits;
    float syn_Presence;
    float syn_BassPresence;
    float syn_MidPresence;
    float syn_MidHighPresence;
    float syn_HighPresence;
    float syn_Intensity;
    float syn_OnBeat;
    float syn_ToggleOnBeat;
    float syn_RandomOnBeat;
    float syn_FadeInOut;
    float syn_BPM;
    float syn_BPMConfidence;
    float syn_BPMTwitcher;
    float syn_BPMSin;
    float syn_BPMSin2;
    float syn_BPMSin4;
    float syn_BPMSin8;
    float syn_BPMTri;
    float syn_BPMTri2;
    float syn_BPMTri4;
    float syn_BPMTri8;
    float syn_MediaActive;
};

layout(set = 0, binding = 1) uniform texture2D _chroma_media_tex;
layout(set = 0, binding = 2) uniform sampler _chroma_media_samp;
layout(set = 0, binding = 3) uniform texture2D _chroma_spectrum_tex;
layout(set = 0, binding = 4) uniform sampler _chroma_spectrum_samp;

#define syn_UserImage sampler2D(_chroma_media_tex, _chroma_media_samp)
#define syn_Spectrum sampler2D(_chroma_spectrum_tex, _chroma_spectrum_samp)
"#;

/// Static Synesthesia constants, coordinate globals, helper functions,
/// and Metal-safe `_chroma_safe_pow` overloads.
const SYN_PREAMBLE_HELPERS: &str = r#"
#ifndef PI
#define PI 3.14159265358979323846
#endif
#ifndef TAU
#define TAU 6.28318530717958647692
#endif

vec2 _xy;
vec2 _uv;
vec2 _uvc;

// Metal-safe pow overloads: avoids MSL NaN when base <= 0.0.
float _chroma_safe_pow(float a, float b) {
    return pow(max(abs(a), 1e-6), b);
}

vec2 _chroma_safe_pow(vec2 a, vec2 b) {
    return pow(max(abs(a), vec2(1e-6)), b);
}

vec3 _chroma_safe_pow(vec3 a, vec3 b) {
    return pow(max(abs(a), vec3(1e-6)), b);
}

vec4 _chroma_safe_pow(vec4 a, vec4 b) {
    return pow(max(abs(a), vec4(1e-6)), b);
}

vec2 _rotate(vec2 v, float a) {
    float c = cos(a);
    float s = sin(a);
    return mat2(c, -s, s, c) * v;
}

vec2 _scale(vec2 v, float s) {
    return v * s;
}

vec2 _scale(vec2 v, vec2 s) {
    return v * s;
}

float _pixelate(float x, float amount) {
    float a = max(amount, 1.0);
    return floor(x * a) / a;
}

vec2 _pixelate(vec2 v, float amount) {
    float a = max(amount, 1.0);
    return floor(v * a) / a;
}

vec2 _pixelate(vec2 v, vec2 amount) {
    vec2 a = max(amount, vec2(1.0));
    return floor(v * a) / a;
}

float _nclamp(float x) {
    return clamp(x, 0.0, 1.0);
}

vec2 _nclamp(vec2 x) {
    return clamp(x, vec2(0.0), vec2(1.0));
}

vec3 _nclamp(vec3 x) {
    return clamp(x, vec3(0.0), vec3(1.0));
}

vec4 _nclamp(vec4 x) {
    return clamp(x, vec4(0.0), vec4(1.0));
}

float _chroma_hash21(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

float _chroma_noise2(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    float a = _chroma_hash21(i);
    float b = _chroma_hash21(i + vec2(1.0, 0.0));
    float c = _chroma_hash21(i + vec2(0.0, 1.0));
    float d = _chroma_hash21(i + vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float _fbm(vec2 p) {
    float value = 0.0;
    float amp = 0.5;
    for (int i = 0; i < 5; i++) {
        value += amp * _chroma_noise2(p);
        p *= 2.0;
        amp *= 0.5;
    }
    return value;
}

float _fbm(vec3 p) {
    return _fbm(p.xy + vec2(p.z * 17.3, p.z * 31.7));
}

bool _isMediaActive() {
    return syn_MediaActive > 0.5;
}

vec4 _loadMedia(vec2 uv) {
    return texture(syn_UserImage, uv);
}

vec4 _loadMedia() {
    return _loadMedia(_uv);
}

vec4 _loadMediaAsMask(vec2 uv) {
    vec4 c = _loadMedia(uv);
    float lum = dot(c.rgb, vec3(0.299, 0.587, 0.114));
    return vec4(vec3(lum), c.a);
}

vec4 _loadMediaAsMask() {
    return _loadMediaAsMask(_uv);
}
"#;

/// Wrapper trailer invoking Synesthesia's `renderMain()` into `_chroma_fragColor`.
const SYN_TRAILER: &str = r#"
void main() {
    _xy = gl_FragCoord.xy;
    vec2 _chroma_res = max(RENDERSIZE, vec2(1.0));
    _uv = _xy / _chroma_res;
    _uvc = (_xy - 0.5 * _chroma_res) / _chroma_res.y;
    _chroma_fragColor = renderMain();
}
"#;

/// Media/asset extensions copied from the root of a `.synScene` directory.
const ROOT_ASSET_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "gif", "bmp", "svg", "tga", "hdr", "mp4", "mov", "webm",
];

/// Import a Synesthesia `.synScene` directory into a non-destructive
/// `<target_parent_dir>/<slug>.chroma/` package.
pub fn import_syn_scene_to_chroma(
    source_dir: &Path,
    target_parent_dir: &Path,
) -> Result<SynImportReport, SynImportError> {
    if !source_dir.is_dir() {
        return Err(SynImportError::SourceNotDirectory(source_dir.to_path_buf()));
    }

    let src_scene_json = source_dir.join("scene.json");
    if !src_scene_json.is_file() {
        return Err(SynImportError::MissingSceneJson(src_scene_json));
    }

    let src_main_glsl = source_dir.join("main.glsl");
    if !src_main_glsl.is_file() {
        return Err(SynImportError::MissingMainGlsl(src_main_glsl));
    }

    let raw_scene_json = fs::read_to_string(&src_scene_json)?;
    let scene = parse_scene_json(&raw_scene_json)
        .map_err(|err| SynImportError::SceneParse(err.to_string()))?;
    let lint_warnings = validate(&scene, ValidationMode::Compat)
        .map_err(|err| SynImportError::SceneValidation(format!("{err:?}")))?;

    let mut warnings: Vec<String> = lint_warnings
        .into_iter()
        .map(|w| format!("{w:?}"))
        .collect();

    let raw_main_glsl = fs::read_to_string(&src_main_glsl)?;
    let (rewritten_glsl, declared_control_uniforms, pow_calls_rewritten, glsl_warnings) =
        rewrite_syn_glsl(&raw_main_glsl, &scene);
    warnings.extend(glsl_warnings);

    // Validate the generated GLSL through naga before touching disk.
    naga_validate_fragment(&rewritten_glsl)?;

    let normalized_scene_json = normalize_scene_json(&raw_scene_json)?;

    let slug = derive_slug(source_dir);
    let output_dir = target_parent_dir.join(format!("{slug}.chroma"));
    if output_dir.exists() {
        return Err(SynImportError::TargetAlreadyExists(output_dir));
    }

    fs::create_dir_all(&output_dir)?;

    let result = write_package_contents(
        source_dir,
        &output_dir,
        &normalized_scene_json,
        &rewritten_glsl,
    );

    match result {
        Ok((scene_json_path, main_glsl_path, script_js_copied, copied_assets)) => {
            Ok(SynImportReport {
                slug,
                output_dir,
                scene_json_path,
                main_glsl_path,
                script_js_copied,
                copied_assets,
                declared_control_uniforms,
                pow_calls_rewritten,
                warnings,
            })
        }
        Err(err) => {
            let _ = fs::remove_dir_all(&output_dir);
            Err(err)
        }
    }
}

fn write_package_contents(
    source_dir: &Path,
    output_dir: &Path,
    scene_json: &str,
    main_glsl: &str,
) -> Result<(PathBuf, PathBuf, bool, Vec<PathBuf>), SynImportError> {
    let scene_json_path = output_dir.join("scene.json");
    fs::write(&scene_json_path, scene_json)?;

    let main_glsl_path = output_dir.join("main.glsl");
    fs::write(&main_glsl_path, main_glsl)?;

    let mut script_js_copied = false;
    let src_script = source_dir.join("script.js");
    if src_script.is_file() {
        fs::copy(&src_script, output_dir.join("script.js"))?;
        script_js_copied = true;
    }

    let mut copied_assets = Vec::new();
    let src_images = source_dir.join("images");
    if src_images.is_dir() {
        copy_dir_recursive(
            &src_images,
            &output_dir.join("images"),
            Path::new("images"),
            &mut copied_assets,
        )?;
    }

    // Copy root-level media/image assets (e.g. scene thumbnail or textures)
    // and auxiliary .glsl files (excluding main.glsl).
    let mut entries: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        entries.push(entry.path());
    }
    entries.sort();

    for path in entries {
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if file_name == "scene.json" || file_name == "main.glsl" || file_name == "script.js" {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        let should_copy = match ext.as_deref() {
            Some("glsl") => true,
            Some(e) => ROOT_ASSET_EXTENSIONS.contains(&e),
            None => false,
        };
        if should_copy {
            fs::copy(&path, output_dir.join(file_name))?;
            copied_assets.push(PathBuf::from(file_name));
        }
    }

    copied_assets.sort();
    Ok((
        scene_json_path,
        main_glsl_path,
        script_js_copied,
        copied_assets,
    ))
}

fn copy_dir_recursive(
    src: &Path,
    dst: &Path,
    rel_prefix: &Path,
    copied: &mut Vec<PathBuf>,
) -> Result<(), SynImportError> {
    fs::create_dir_all(dst)?;
    let mut entries: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(src)? {
        entries.push(entry?.path());
    }
    entries.sort();

    for path in entries {
        let Some(name) = path.file_name() else {
            continue;
        };
        let next_dst = dst.join(name);
        let next_rel = rel_prefix.join(name);
        if path.is_dir() {
            copy_dir_recursive(&path, &next_dst, &next_rel, copied)?;
        } else if path.is_file() {
            fs::copy(&path, &next_dst)?;
            copied.push(next_rel);
        }
    }
    Ok(())
}

/// Derive a clean filesystem slug from `source_dir`.
fn derive_slug(source_dir: &Path) -> String {
    let raw_name = source_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("imported_syn_scene");

    let stripped = if raw_name.len() > ".synScene".len()
        && raw_name[raw_name.len() - ".synScene".len()..].eq_ignore_ascii_case(".synScene")
    {
        &raw_name[..raw_name.len() - ".synScene".len()]
    } else {
        raw_name
    };

    let mut slug = String::with_capacity(stripped.len());
    let mut prev_underscore = false;
    for ch in stripped.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_underscore = false;
        } else if (ch == '-' || ch == '_' || ch.is_ascii_whitespace())
            && !prev_underscore
            && !slug.is_empty()
        {
            slug.push('_');
            prev_underscore = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    if slug.is_empty() {
        "imported_syn_scene".to_string()
    } else {
        slug
    }
}

/// Preserve all keys from the source `scene.json` while ensuring
/// `SCHEMA_VERSION` and `FORMAT_TAG` are set for compat-syn-v1.
fn normalize_scene_json(raw_json: &str) -> Result<String, SynImportError> {
    let mut value: serde_json::Value = serde_json::from_str(raw_json)
        .map_err(|err| SynImportError::SceneParse(err.to_string()))?;
    if let Some(obj) = value.as_object_mut() {
        obj.entry("SCHEMA_VERSION".to_string())
            .or_insert_with(|| serde_json::Value::from(1_u32));
        obj.entry("FORMAT_TAG".to_string())
            .or_insert_with(|| serde_json::Value::String("compat-syn-v1".to_string()));
    }
    let mut formatted = serde_json::to_string_pretty(&value)
        .map_err(|err| SynImportError::SceneParse(err.to_string()))?;
    formatted.push('\n');
    Ok(formatted)
}

/// Determine the GLSL type for a `scene.json` control declaration.
fn control_glsl_type(decl: &ControlDecl) -> &'static str {
    match decl.kind {
        Some(ControlKind::Xy) => "vec2",
        Some(ControlKind::Color) => match &decl.default {
            ControlValue::Vec4(_) => "vec4",
            ControlValue::Vec3(_) => "vec3",
            _ => {
                if decl.def_color.is_some() {
                    "vec4"
                } else {
                    "vec3"
                }
            }
        },
        Some(_) => "float",
        None => match &decl.default {
            ControlValue::Vec2(_) => "vec2",
            ControlValue::Vec3(_) => "vec3",
            ControlValue::Vec4(_) => "vec4",
            _ => {
                if decl.def_color.is_some() {
                    "vec4"
                } else {
                    "float"
                }
            }
        },
    }
}

/// Check whether `name` is a valid GLSL identifier.
fn is_valid_glsl_ident(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Rewrite a Synesthesia `main.glsl` into a Vulkan GLSL 4.50 / `naga`-compatible
/// fragment shader.
pub fn rewrite_syn_glsl(
    raw_glsl: &str,
    scene: &Scene,
) -> (String, Vec<String>, usize, Vec<String>) {
    let mut warnings = Vec::new();
    let mut declared_controls = Vec::new();

    // Strip any existing `#version` line from the source body.
    let mut stripped_body = String::with_capacity(raw_glsl.len());
    for line in raw_glsl.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#version") {
            warnings.push(format!("stripped source directive: {trimmed}"));
            continue;
        }
        stripped_body.push_str(line);
        stripped_body.push('\n');
    }

    // Auto-correct legacy `texture2D(` -> `texture(`.
    if stripped_body.contains("texture2D(") {
        stripped_body = stripped_body.replace("texture2D(", "texture(");
        warnings.push("rewrote legacy texture2D( calls to texture(".to_string());
    }

    // Rewrite standalone `pow(` -> `_chroma_safe_pow(` for Metal NaN safety.
    let (body_after_pow, pow_calls_rewritten) = rewrite_pow_calls(&stripped_body);

    // Build auto-declared controls uniform block.
    let builtins: HashSet<&str> = BUILTIN_UNIFORM_NAMES.iter().copied().collect();
    let mut seen_controls: HashSet<String> = HashSet::new();
    let mut controls_block = String::new();
    let mut control_fields: Vec<(&'static str, String)> = Vec::new();

    for decl in &scene.controls {
        if !is_valid_glsl_ident(&decl.name) {
            warnings.push(format!(
                "skipped control with non-identifier name: {:?}",
                decl.name
            ));
            continue;
        }
        if builtins.contains(decl.name.as_str()) {
            continue;
        }
        if !seen_controls.insert(decl.name.clone()) {
            continue;
        }
        let glsl_ty = control_glsl_type(decl);
        declared_controls.push(decl.name.clone());
        control_fields.push((glsl_ty, decl.name.clone()));
    }

    if !control_fields.is_empty() {
        controls_block
            .push_str("\nlayout(set = 0, binding = 5, std140) uniform _ChromaSynControls {\n");
        for (glsl_ty, name) in &control_fields {
            controls_block.push_str(&format!("    {glsl_ty} {name};\n"));
        }
        controls_block.push_str("};\n");
    }

    // Declare multipass target samplers if `PASSES` has named targets.
    let mut pass_block = String::new();
    let mut next_binding: u32 = 6;
    let mut seen_passes: HashSet<String> = HashSet::new();
    for pass in &scene.passes {
        let target = pass.node.0.trim();
        if target.is_empty()
            || !is_valid_glsl_ident(target)
            || builtins.contains(target)
            || seen_controls.contains(target)
            || !seen_passes.insert(target.to_string())
        {
            continue;
        }
        let tex_binding = next_binding;
        let samp_binding = next_binding + 1;
        next_binding += 2;
        pass_block.push_str(&format!(
            "layout(set = 0, binding = {tex_binding}) uniform texture2D _chroma_pass_tex_{target};\n\
             layout(set = 0, binding = {samp_binding}) uniform sampler _chroma_pass_samp_{target};\n\
             #define {target} sampler2D(_chroma_pass_tex_{target}, _chroma_pass_samp_{target})\n"
        ));
    }

    let has_explicit_main = contains_void_main(&body_after_pow);

    let mut out = String::with_capacity(
        SYN_PREAMBLE_HEADER.len()
            + controls_block.len()
            + pass_block.len()
            + SYN_PREAMBLE_HELPERS.len()
            + body_after_pow.len()
            + SYN_TRAILER.len()
            + 64,
    );
    out.push_str(SYN_PREAMBLE_HEADER);
    out.push_str(&controls_block);
    if !pass_block.is_empty() {
        out.push('\n');
        out.push_str(&pass_block);
    }
    out.push_str(SYN_PREAMBLE_HELPERS);
    out.push_str("\n// --- Synesthesia scene body ---\n");
    out.push_str(&body_after_pow);
    if !body_after_pow.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("// --- End Synesthesia scene body ---\n");
    if !has_explicit_main {
        out.push_str(SYN_TRAILER);
    }

    (out, declared_controls, pow_calls_rewritten, warnings)
}

/// Detect whether the shader body already defines `void main(`.
fn contains_void_main(source: &str) -> bool {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i + 4 <= len {
        // Skip line comments.
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // Skip block comments.
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(len);
            continue;
        }
        if bytes[i..].starts_with(b"void")
            && (i == 0 || !is_ident_byte(bytes[i - 1]))
            && (i + 4 < len && bytes[i + 4].is_ascii_whitespace())
        {
            let mut j = i + 4;
            while j < len && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if bytes[j..].starts_with(b"main") {
                let mut k = j + 4;
                while k < len && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                if k < len && bytes[k] == b'(' {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Rewrite standalone `pow(` calls to `_chroma_safe_pow(` while preserving
/// comments and identifiers like `mypow(` or `pow2(`.
pub fn rewrite_pow_calls(source: &str) -> (String, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + 64);
    let mut count = 0usize;
    let mut i = 0usize;

    while i < len {
        // Preserve line comments verbatim.
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            out.push_str(&source[start..i]);
            continue;
        }
        // Preserve block comments verbatim.
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(len);
            out.push_str(&source[start..i]);
            continue;
        }

        // Match standalone `pow` followed by optional whitespace and `(`.
        if bytes[i..].starts_with(b"pow") && (i == 0 || !is_ident_byte(bytes[i - 1])) {
            let after_pow = i + 3;
            let mut j = after_pow;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            if j < len && bytes[j] == b'(' {
                out.push_str("_chroma_safe_pow");
                out.push_str(&source[after_pow..=j]);
                count += 1;
                i = j + 1;
                continue;
            }
        }

        // Advance one UTF-8 character safely.
        let ch = source[i..].chars().next();
        match ch {
            Some(c) => {
                out.push(c);
                i += c.len_utf8();
            }
            None => break,
        }
    }

    (out, count)
}

/// Parse and validate a Vulkan GLSL 4.50 fragment shader through `naga`.
fn naga_validate_fragment(glsl_source: &str) -> Result<(), SynImportError> {
    let mut frontend = naga::front::glsl::Frontend::default();
    let module = frontend
        .parse(
            &naga::front::glsl::Options {
                stage: naga::ShaderStage::Fragment,
                defines: Default::default(),
            },
            glsl_source,
        )
        .map_err(|errors| {
            SynImportError::GlslCompileError(format!("naga GLSL parse failed: {errors:?}"))
        })?;

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator.validate(&module).map_err(|err| {
        SynImportError::GlslCompileError(format!("naga IR validation failed: {err:?}"))
    })?;

    Ok(())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-SCENE-IMPORT (T1) — pure-Rust `.synScene` to `.chroma`
    //! compatibility importer pins: Vulkan GLSL 4.50 / naga preamble,
    //! array `MIN`/`MAX` controls, `fragColor` global collision avoidance,
    //! Metal-safe `_chroma_safe_pow` rewrite, and `script.js` passthrough.

    use std::sync::atomic::{AtomicU64, Ordering};

    use chromaticity_scene::loader::load_chroma_dir;
    use chromaticity_scene::schema::ControlBound;

    use super::{SynImportError, import_syn_scene_to_chroma, rewrite_pow_calls};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_test_dir(label: &str) -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "chromaticity_syn_import_{label}_{}_{id}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    #[test]
    fn imports_realistic_syn_scene_with_array_bounds_script_and_assets() {
        let root = temp_test_dir("realistic");
        let src = root.join("beats_in_4d.synScene");
        let out_parent = root.join("out");
        std::fs::create_dir_all(src.join("images")).unwrap();
        std::fs::create_dir_all(&out_parent).unwrap();

        let scene_json = r#"{
            "TITLE": "Beats in 4D",
            "CREDIT": "Test Author",
            "CONTROLS": [
                {
                    "NAME": "xy_shift",
                    "TYPE": "xy",
                    "DEFAULT": [0.5, 0.5],
                    "MIN": [0, 0],
                    "MAX": [1, 1]
                },
                {
                    "NAME": "tint_rgb",
                    "TYPE": "color",
                    "DEFAULT": [0.2, 0.6, 0.9],
                    "MIN": [0, 0, 0],
                    "MAX": [1, 1, 1]
                },
                {
                    "NAME": "zoom",
                    "TYPE": "slider smooth",
                    "DEFAULT": 1.0,
                    "MIN": 0.25,
                    "MAX": 4.0
                }
            ]
        }"#;

        let main_glsl = r#"
vec4 renderMain() {
    vec2 p = _rotate(_uvc, TIME * 0.5);
    p = _scale(p, zoom) + xy_shift;
    float n = _fbm(p * 3.0);
    vec2 pix = _pixelate(_uv, 64.0);
    vec4 media = _isMediaActive() ? _loadMedia(pix) : _loadMediaAsMask();
    float glow = _nclamp(sin(n * TAU + PI * syn_Level) + PASSINDEX * 0.0);
    vec3 col = tint_rgb * glow + media.rgb * 0.1;
    return vec4(col, 1.0);
}
"#;

        let script_js = "function update(dt) { uniforms.zoom = 1.0 + dt * 0.0; }\n";

        std::fs::write(src.join("scene.json"), scene_json).unwrap();
        std::fs::write(src.join("main.glsl"), main_glsl).unwrap();
        std::fs::write(src.join("script.js"), script_js).unwrap();
        std::fs::write(src.join("images").join("noise.png"), b"PNGDATA").unwrap();
        std::fs::write(src.join("beats_in_4d.png"), b"THUMB").unwrap();

        let report = import_syn_scene_to_chroma(&src, &out_parent).expect("import must succeed");

        assert_eq!(report.slug, "beats_in_4d");
        assert_eq!(report.output_dir, out_parent.join("beats_in_4d.chroma"));
        assert!(report.script_js_copied);
        assert_eq!(
            report.declared_control_uniforms,
            vec!["xy_shift", "tint_rgb", "zoom"]
        );
        assert!(
            report
                .copied_assets
                .iter()
                .any(|p| p == std::path::Path::new("images/noise.png"))
        );
        assert!(
            report
                .copied_assets
                .iter()
                .any(|p| p == std::path::Path::new("beats_in_4d.png"))
        );

        // Verify script.js passthrough verbatim.
        let copied_script = std::fs::read_to_string(report.output_dir.join("script.js")).unwrap();
        assert_eq!(copied_script, script_js);

        // Verify source directory was not modified.
        assert_eq!(
            std::fs::read_to_string(src.join("main.glsl")).unwrap(),
            main_glsl
        );

        // Verify resulting .chroma loads cleanly through chromaticity_scene::loader.
        let loaded = load_chroma_dir(&report.output_dir).expect("load_chroma_dir must succeed");
        assert_eq!(loaded.controls.len(), 3);
        assert_eq!(
            loaded.controls[0].min,
            Some(ControlBound::Vec(vec![0.0, 0.0]))
        );
        assert_eq!(
            loaded.controls[0].max,
            Some(ControlBound::Vec(vec![1.0, 1.0]))
        );
        assert_eq!(
            loaded.controls[1].min,
            Some(ControlBound::Vec(vec![0.0, 0.0, 0.0]))
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn avoids_collision_with_scene_global_frag_color() {
        let root = temp_test_dir("fragcolor_collision");
        let src = root.join("glob.synScene");
        let out_parent = root.join("out");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&out_parent).unwrap();

        std::fs::write(src.join("scene.json"), r#"{"TITLE": "Glob"}"#).unwrap();
        std::fs::write(
            src.join("main.glsl"),
            r#"
vec4 fragColor = vec4(0.0);

vec4 renderMain() {
    fragColor = vec4(_uv, 0.5 + 0.5 * sin(TIME), 1.0);
    return fragColor;
}
"#,
        )
        .unwrap();

        let report = import_syn_scene_to_chroma(&src, &out_parent)
            .expect("scene with global fragColor must compile cleanly");
        let emitted = std::fs::read_to_string(&report.main_glsl_path).unwrap();
        assert!(emitted.contains("layout(location = 0) out vec4 _chroma_fragColor;"));
        assert!(emitted.contains("_chroma_fragColor = renderMain();"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rewrites_pow_calls_for_metal_nan_safety_and_compiles() {
        let root = temp_test_dir("safe_pow");
        let src = root.join("circles5.synScene");
        let out_parent = root.join("out");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&out_parent).unwrap();

        std::fs::write(src.join("scene.json"), r#"{"TITLE": "Circles5"}"#).unwrap();
        std::fs::write(
            src.join("main.glsl"),
            r#"
float mypow(float a, float b) {
    return a * b;
}

vec4 renderMain() {
    // pow(commented_out, 2.0) should stay untouched
    float x = _xy.x / max(RENDERSIZE.x, 1.0);
    float v = pow(-0.5 + x, 1.0);
    vec3 c = pow(vec3(-0.2 + _uv.x, _uv.y, 0.5), vec3(2.2));
    float m = mypow(v, 0.5);
    return vec4(c + vec3(m * 0.1), 1.0);
}
"#,
        )
        .unwrap();

        let report = import_syn_scene_to_chroma(&src, &out_parent)
            .expect("scene using pow(-0.5 + x, 1.0) must compile cleanly");
        assert_eq!(report.pow_calls_rewritten, 2);

        let emitted = std::fs::read_to_string(&report.main_glsl_path).unwrap();
        assert!(emitted.contains("_chroma_safe_pow(-0.5 + x, 1.0)"));
        assert!(emitted.contains("_chroma_safe_pow(vec3(-0.2 + _uv.x, _uv.y, 0.5), vec3(2.2))"));
        assert!(emitted.contains("float mypow(float a, float b)"));
        assert!(emitted.contains("// pow(commented_out, 2.0) should stay untouched"));

        // Direct unit check on rewrite_pow_calls.
        let (rewritten, count) =
            rewrite_pow_calls("/* pow(a, b) */ float z = pow(x, y) + npow(x, y);");
        assert_eq!(count, 1);
        assert_eq!(
            rewritten,
            "/* pow(a, b) */ float z = _chroma_safe_pow(x, y) + npow(x, y);"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_invalid_glsl_without_leaving_partial_output_dir() {
        let root = temp_test_dir("invalid_glsl");
        let src = root.join("broken.synScene");
        let out_parent = root.join("out");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&out_parent).unwrap();

        std::fs::write(src.join("scene.json"), r#"{"TITLE": "Broken"}"#).unwrap();
        std::fs::write(
            src.join("main.glsl"),
            "vec4 renderMain() { return definitely_undefined_symbol; }\n",
        )
        .unwrap();

        let err = import_syn_scene_to_chroma(&src, &out_parent)
            .expect_err("invalid GLSL must fail import");
        assert!(
            matches!(err, SynImportError::GlslCompileError(_)),
            "expected GlslCompileError, got {err:?}"
        );
        assert!(
            !out_parent.join("broken.chroma").exists(),
            "failed import must not leave a partial .chroma directory on disk"
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}
