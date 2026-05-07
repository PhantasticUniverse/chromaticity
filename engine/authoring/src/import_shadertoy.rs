// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Shadertoy-to-Chromaticity import converter.
//!
//! Slice scope per `docs/26-evidence-artifact-registry.md` row 64
//! (`EV-IMPORT-SHADERTOY`): the converter surface plus the 3 PR-1
//! starter fixtures plus the PR-2 broader corpus + iChannel-fixture-
//! pack-resolution path called out by `docs/38` §5.2a. Mechanics per
//! `docs/38-scene-library-and-import-spec.md` §5.2 + §5.2a:
//!
//! - The user supplies a Shadertoy GLSL body containing
//!   `void mainImage(out vec4 fragColor, in vec2 fragCoord) { ... }`.
//! - Chromaticity emits a `.chroma` package (the same on-disk shape
//!   the [`crate::scaffolder`] produces) whose `main.glsl` wraps the
//!   Shadertoy body in a `void main()` entry compatible with the naga
//!   GLSL frontend the engine/render single-pass entry point uses.
//! - The wrapper declares the canonical Shadertoy-spelled uniforms
//!   (`iTime`, `iResolution`, `iTimeDelta`, `iFrame`, `iMouse`) so
//!   the user's body code compiles unchanged. `iDate` and
//!   `iChannelTime` are intentionally omitted — they are rare in the
//!   Shadertoy Top-100 and naga's GLSL frontend rejects `float[4]`
//!   uniforms outside a uniform-block (uniform-array stride must be
//!   16-byte-aligned). The runtime binding to chromaticity-native
//!   reactive symbols (`TIME`, `RENDERSIZE`, …) per `docs/38` §5.2
//!   is a downstream binding-layer concern; this slice ships the
//!   GLSL-text + `.chroma`-package surface only.
//! - `iChannel0..3` are resolved against the
//!   [`ShadertoyImportConfig::ichannel_mappings`] builder per PR 2's
//!   widening: each declared `IChannelMapping` causes the wrapper
//!   preamble to declare a Chromaticity-native binding-slot uniform
//!   (`chroma_input_<channel>`) and the body's `iChannelN` references
//!   are rewritten to that slot symbol pre-naga. Channels that the
//!   body references but the config does not map emit a
//!   [`Warning::MissingIChannelMapping`] (the body still references
//!   the symbol so authors can edit the `.chroma` package post-import
//!   to add the binding). [`IChannelKind::Webcam`] mappings emit
//!   [`Warning::WebcamIChannelDeferred`] (Shadertoy permits webcam
//!   binding; Chromaticity defers to post-MVP).
//! - VR-mode (`mainVR`) and cubemap-entry (`mainCubemap`) patterns
//!   are rejected with [`ImportError::UnsupportedShadertoyFeature`]
//!   and [`Warning::CubemapEntrypointDeferred`] respectively because
//!   neither has a plausible auto-mapping into the single-pass
//!   `mainImage` shape; deprecated `texture2D(...)` calls are auto-
//!   corrected to `texture(...)` with a
//!   [`Warning::DeprecatedTexture2DAutoCorrected`] surfaced for the
//!   author's audit trail.
//!
//! # Atomicity
//!
//! [`import_shadertoy_to_chroma`] mirrors the
//! [`crate::scaffolder::scaffold_to_path`] eager-create-then-cleanup-
//! on-error contract: callers see either a fully-populated
//! `<scene_id>.chroma` package or no package at all. Cleanup errors
//! are swallowed so the originating I/O cause stays surfaced.
//!
//! # GLSL-compile validation
//!
//! The converter routes the wrapped fragment through the same naga
//! GLSL frontend + IR validator pair that
//! [`chromaticity_render::single_pass::compile_and_render_single_pass`]
//! drives internally. A naga-rejected fragment surfaces as
//! [`ImportError::GlslCompileError`] and the partially-staged
//! package is removed before the error returns; callers observe
//! either a successful import or no on-disk side effect.
//!
//! `chromaticity_render::single_pass::compile_glsl_module` is
//! `pub(crate)` (a slice-boundary signal per `CLAUDE.md`), so this
//! module reproduces the parse + validate seam directly against
//! `naga::front::glsl::Frontend` — the same shape the
//! [`crate::scaffolder`] T1 pin uses to assert the scaffolded
//! fragment compiles.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Scene-author-supplied configuration for [`import_shadertoy_to_chroma`].
///
/// `shadertoy_id` is optional; when `None`, the import is anonymized
/// (no `SHADERTOY_ID` field is written into `scene.json`). When
/// `Some`, the value is preserved verbatim so the library catalog
/// can route back to the source URL.
///
/// `title` / `credit` / `description` map to the corresponding
/// `scene.json` wire keys per the slice-3a frozen schema. `credit`
/// is expected to carry the per-`docs/38` §5.3 attribution stub
/// (default Shadertoy license is CC-BY-NC-SA 3.0); the converter
/// does not synthesize it from the URL — the caller is responsible.
///
/// `source_glsl` is the raw Shadertoy fragment body. It must contain
/// a `mainImage(out vec4 fragColor, in vec2 fragCoord)` entry point
/// per Shadertoy's editor contract. Whitespace + comment shapes
/// observed in the Shadertoy editor are preserved through the
/// converter; only the wrapper preamble is added.
///
/// `ichannel_mappings` resolves `iChannel0..3` references in the
/// body to Chromaticity-native binding slots per `docs/38` §5.2a.
/// Default: empty Vec — any `iChannelN` reference in the body emits
/// a [`Warning::MissingIChannelMapping`]. Use
/// [`Self::with_ichannel_mappings`] to populate.
#[derive(Debug, Clone)]
pub struct ShadertoyImportConfig {
    /// Optional Shadertoy `id` (the URL fragment). Preserved verbatim
    /// when present; absent fields are stripped.
    pub shadertoy_id: Option<String>,
    /// `TITLE` wire key.
    pub title: String,
    /// `CREDIT` wire key. Per `docs/38` §5.3 the import UI populates
    /// this with the source URL plus an attribution stub.
    pub credit: String,
    /// `DESCRIPTION` wire key.
    pub description: String,
    /// Raw Shadertoy GLSL fragment containing
    /// `mainImage(out vec4 fragColor, in vec2 fragCoord)`.
    pub source_glsl: String,
    /// Per `docs/38` §5.2a: caller-supplied iChannel resolution table.
    /// Empty Vec is the default; the converter still functions but
    /// any `iChannelN` reference in the body surfaces as a
    /// [`Warning::MissingIChannelMapping`].
    pub ichannel_mappings: Vec<IChannelMapping>,
}

impl ShadertoyImportConfig {
    /// Builder-style setter for the iChannel resolution table.
    ///
    /// Each [`IChannelMapping`] in `mappings` declares which channel
    /// index (0..=3) the entry resolves and what kind of source it
    /// is. The converter rewrites `iChannelN` references in the body
    /// to the canonical Chromaticity-native binding slot uniform
    /// (`chroma_input_<index>`) and emits the matching sampler /
    /// texture preamble declaration.
    ///
    /// Channels referenced by the body but absent from `mappings`
    /// produce a [`Warning::MissingIChannelMapping`]; the rewrite
    /// still fires (so the body compiles against the
    /// `chroma_input_<index>` slot) but the slot has no source
    /// declared in `scene.json` until the author edits the imported
    /// package.
    pub fn with_ichannel_mappings(mut self, mappings: Vec<IChannelMapping>) -> Self {
        self.ichannel_mappings = mappings;
        self
    }
}

/// Per-channel iChannel resolution entry per `docs/38` §5.2a.
#[derive(Debug, Clone)]
pub struct IChannelMapping {
    /// Channel index 0..=3 in the Shadertoy body.
    pub channel_index: u8,
    /// What kind of binding source the channel resolves to.
    pub kind: IChannelKind,
    /// Optional asset-source path (relative to the bundled fixture
    /// pack at `<AppBundle>/Resources/scenes/shadertoy-fixtures/`,
    /// or absolute for a `custom-asset` user-supplied file). The
    /// converter records this in `scene.json` under
    /// `ICHANNELS[<index>]` so the authoring UI can re-establish the
    /// binding on subsequent loads.
    pub source_path: Option<String>,
}

/// Categorical kind of a Shadertoy iChannel binding.
///
/// `Texture2D` / `Buffer` / `Video` / `Audio` map to a `sampler2D`
/// in the wrapper preamble (Shadertoy's audio iChannel is a 512×2
/// texture in the reference editor, so `sampler2D` is the correct
/// shape for naga compile-validation). `Cubemap` maps to a
/// `samplerCube`. `Keyboard` maps to a `sampler2D` with its
/// standard 256×3 layout. `Webcam` is rejected as a deferral
/// per `Warning::WebcamIChannelDeferred` — the slot still gets a
/// `sampler2D` declaration so the body compiles, but the warning
/// surfaces post-MVP-deferral context to the author.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IChannelKind {
    /// 2D static texture (the most common Shadertoy iChannel).
    Texture2D,
    /// Cubemap from the bundled fixture pack.
    Cubemap,
    /// Audio iChannel — Shadertoy delivers this as a 512x2 sampler2D
    /// (FFT row 0, time-domain row 1). Chromaticity wires the
    /// audio-uniform-IPC snapshot through this slot; see
    /// `engine/render::audio_adapter`.
    Audio,
    /// Video texture (file-backed media).
    Video,
    /// Buffer (Shadertoy multipass A/B/C/D output read as a 2D
    /// texture). Chromaticity emits the binding slot but the
    /// multipass evaluation order is a downstream authoring concern.
    Buffer,
    /// Shadertoy's keyboard sampler (256x3 layout — pressed / held /
    /// toggled).
    Keyboard,
    /// Shadertoy's webcam iChannel — Chromaticity defers webcam
    /// integration to post-MVP per the Phase-4a Wave 1c-β plan; this
    /// entry exists so the converter can emit a structured deferral
    /// warning rather than a coarse "unsupported" error.
    Webcam,
}

impl IChannelKind {
    /// Underlying GLSL `textureNN` / `textureCube` type the wrapper
    /// preamble declares for this kind, per the separated-texture +
    /// `sampler` shape naga's GLSL frontend accepts.
    fn texture_type(self) -> &'static str {
        match self {
            IChannelKind::Cubemap => "textureCube",
            IChannelKind::Texture2D
            | IChannelKind::Audio
            | IChannelKind::Video
            | IChannelKind::Buffer
            | IChannelKind::Keyboard
            | IChannelKind::Webcam => "texture2D",
        }
    }

    /// Combined-sampler GLSL constructor type the wrapper preamble's
    /// `#define chroma_input_<N> ...` macro instantiates so the
    /// rewritten body's `texture(chroma_input_<N>, uv)` call site
    /// compiles against the separated-texture + `sampler` declaration.
    fn combined_sampler_type(self) -> &'static str {
        match self {
            IChannelKind::Cubemap => "samplerCube",
            IChannelKind::Texture2D
            | IChannelKind::Audio
            | IChannelKind::Video
            | IChannelKind::Buffer
            | IChannelKind::Keyboard
            | IChannelKind::Webcam => "sampler2D",
        }
    }

    /// Wire-format tag the converter writes into `scene.json`'s
    /// `ICHANNELS[<index>].KIND` field so the authoring UI can
    /// distinguish kinds on re-load.
    fn wire_tag(self) -> &'static str {
        match self {
            IChannelKind::Texture2D => "texture2d",
            IChannelKind::Cubemap => "cubemap",
            IChannelKind::Audio => "audio",
            IChannelKind::Video => "video",
            IChannelKind::Buffer => "buffer",
            IChannelKind::Keyboard => "keyboard",
            IChannelKind::Webcam => "webcam",
        }
    }
}

/// Structured conversion warnings — soft observations a caller
/// surfaces in the import-result dialog. Hard incompatibilities
/// reject as [`ImportError::UnsupportedShadertoyFeature`] instead.
///
/// PR 2 widens this from the freeform `Vec<String>` shape PR 1 used
/// for `iChannel*` / `iMouse` warnings into a structured enum so
/// the import-UI can branch on warning kind. The legacy
/// [`ImportReport::conversion_warnings`] string surface continues to
/// exist (each `Warning` has a canonical `Display` impl); callers
/// that want structured access read [`ImportReport::warnings`]
/// directly.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    /// The Shadertoy body references `iChannelN` but the config did
    /// not declare an [`IChannelMapping`] for index `N`. The wrapper
    /// still declares a `chroma_input_<N>` slot so the body compiles,
    /// but `scene.json` carries no source binding until the author
    /// edits the imported package.
    MissingIChannelMapping {
        /// The unresolved channel index 0..=3.
        channel_index: u8,
    },
    /// A declared mapping uses [`IChannelKind::Webcam`]; Chromaticity
    /// defers webcam integration to post-MVP per the Wave 1c-β plan.
    /// The slot still emits a `sampler2D` declaration so the body
    /// compiles against the eventual binding.
    WebcamIChannelDeferred {
        /// The channel index that resolves to webcam.
        channel_index: u8,
    },
    /// The Shadertoy source declares a `mainCubemap` entry-point;
    /// Chromaticity does not auto-map cubemap render-target outputs
    /// in the single-pass MVP shape. The converter rejects this
    /// pattern at import time with this warning surfacing the
    /// deferral rather than a coarse `Unsupported` error so the
    /// import UI can carry actionable copy.
    CubemapEntrypointDeferred,
    /// The body uses the deprecated `texture2D(...)` call form.
    /// Naga's GLSL frontend rejects `texture2D` outside legacy
    /// `#version 110` profiles (the wrapper declares
    /// `#version 450`), so the converter auto-corrects to
    /// `texture(...)` and surfaces this warning so the author
    /// knows the source-of-truth body now differs from the original
    /// paste.
    DeprecatedTexture2DAutoCorrected,
    /// The body references `iMouse`. Mouse uniforms are wired
    /// through `chroma_Mouse` in a later wave; the raw `iMouse`
    /// uniform is declared in the preamble so the body compiles.
    /// (PR 1 surfaced this as a freeform string; PR 2 promotes it
    /// to a structured variant.)
    MouseUniformDeferred,
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Warning::MissingIChannelMapping { channel_index } => {
                write!(
                    f,
                    "iChannel{channel_index} referenced but no IChannelMapping declared — \
                     edit scene.json post-import to add a source binding.",
                )
            }
            Warning::WebcamIChannelDeferred { channel_index } => {
                write!(
                    f,
                    "iChannel{channel_index} mapped to webcam — Chromaticity defers webcam \
                     integration to post-MVP (Phase-4a Wave 1c-β).",
                )
            }
            Warning::CubemapEntrypointDeferred => write!(
                f,
                "mainCubemap entry-point detected — Chromaticity defers cubemap render-target \
                 outputs to post-MVP. The import was rejected.",
            ),
            Warning::DeprecatedTexture2DAutoCorrected => write!(
                f,
                "deprecated texture2D(...) calls auto-corrected to texture(...) — the imported \
                 main.glsl differs from the original Shadertoy paste at these call sites.",
            ),
            Warning::MouseUniformDeferred => write!(
                f,
                "iMouse referenced — mouse binding wires through chroma_Mouse in a later wave.",
            ),
        }
    }
}

/// Successful import report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReport {
    /// Path to the created `<scene_id>.chroma` package directory.
    pub package_root: PathBuf,
    /// Files written by the converter, in order.
    pub files_written: Vec<PathBuf>,
    /// Legacy freeform per-warning string surface — every entry in
    /// [`Self::warnings`] is `Display`-rendered into the matching
    /// position in this Vec. Retained at PR 2 for back-compat with
    /// the PR 1 callers that string-matched on the warning text.
    pub conversion_warnings: Vec<String>,
    /// Structured warnings per the PR 2 widening — the import UI
    /// branches on `Warning` variant for actionable copy / icons.
    pub warnings: Vec<Warning>,
}

/// Errors returned by [`import_shadertoy_to_chroma`].
#[non_exhaustive]
#[derive(Debug)]
pub enum ImportError {
    /// The supplied `source_glsl` does not contain a recognizable
    /// `mainImage(out vec4 fragColor, in vec2 fragCoord)` entry
    /// point. Payload describes the offending input.
    InvalidShadertoyGlsl(String),
    /// The Shadertoy source uses a feature the V1 converter cannot
    /// auto-map (VR-mode, multipass `Buffer A/B/C/D`, keyboard input,
    /// soundcloud-audio iChannels). Soft cases — `iChannel`, `iMouse`
    /// — surface as [`ImportReport::conversion_warnings`] instead.
    UnsupportedShadertoyFeature(String),
    /// I/O failure during package emission. The converter attempts to
    /// remove the partially-populated package directory before
    /// returning; cleanup errors are swallowed so the original cause
    /// is what surfaces.
    IoError(io::Error),
    /// The target `<scene_id>.chroma` package directory already
    /// exists. The converter refuses to overwrite or merge — callers
    /// re-import by removing the directory first.
    TargetExists(PathBuf),
    /// The wrapped fragment failed naga GLSL parse / validation. The
    /// payload carries the underlying naga diagnostic text. The
    /// staged package directory is removed before the error returns.
    GlslCompileError(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::InvalidShadertoyGlsl(msg) => {
                write!(f, "shadertoy import: invalid Shadertoy GLSL: {msg}")
            }
            ImportError::UnsupportedShadertoyFeature(msg) => {
                write!(f, "shadertoy import: unsupported feature: {msg}")
            }
            ImportError::IoError(err) => write!(f, "shadertoy import: I/O error: {err}"),
            ImportError::TargetExists(path) => write!(
                f,
                "shadertoy import: target package already exists: {}",
                path.display()
            ),
            ImportError::GlslCompileError(msg) => {
                write!(f, "shadertoy import: GLSL compile failed: {msg}")
            }
        }
    }
}

impl std::error::Error for ImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ImportError::IoError(err) => Some(err),
            ImportError::InvalidShadertoyGlsl(_)
            | ImportError::UnsupportedShadertoyFeature(_)
            | ImportError::TargetExists(_)
            | ImportError::GlslCompileError(_) => None,
        }
    }
}

impl From<io::Error> for ImportError {
    fn from(err: io::Error) -> Self {
        ImportError::IoError(err)
    }
}

/// Slice-3a `SCHEMA_VERSION` baseline (mirrors
/// `engine/scene::schema::default_schema_version`).
const SHADERTOY_SCHEMA_VERSION: u32 = 1;

/// `FORMAT_TAG` per `docs/38` §3 (line 34): a Shadertoy-imported
/// scene carries the catalog metadata tag `shadertoy-v1-import` so
/// the library browser can render the format-family fallback icon.
const SHADERTOY_FORMAT_TAG: &str = "shadertoy-v1-import";

/// Wrapper preamble.
///
/// Declares the Shadertoy uniforms the converter exposes verbatim so
/// the user's `mainImage` body compiles unchanged. The runtime
/// binding from chromaticity-native reactive symbols
/// (`TIME`, `RENDERSIZE`, ...) per `docs/38` §5.2 is a downstream
/// binding-layer concern beyond this PR's scope.
///
/// Naga's GLSL frontend accepts these uniforms as ordinary scoped
/// declarations; the validator exercises them once `mainImage` reads
/// them.
const PREAMBLE: &str = r#"#version 450
// Chromaticity Shadertoy import wrapper.
//
// Generated by engine/authoring::import_shadertoy. The original
// Shadertoy fragment body is preserved verbatim below; this preamble
// declares the Shadertoy-spelled uniforms the body references and
// the `void main()` entry the chromaticity render-pipeline expects.
//
// Chromaticity runtime binds the host-side reactive symbols (TIME,
// RENDERSIZE, ...) onto these uniforms in a downstream slice per
// docs/38 §5.2; this file's contract is "compiles + validates
// through naga today".
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 frag_color;

// Naga's GLSL frontend requires explicit `layout(binding=...)` on
// every uniform; engine/render's render-pipeline layout supplies the
// runtime descriptor-set binding numbers in a downstream slice.
//
// `iDate` and `iChannelTime` are intentionally omitted from the V1
// converter preamble: they are rarely-used in the Shadertoy Top-100
// corpus and naga's GLSL frontend rejects `float[4]` uniforms in a
// uniform block (uniform-array stride must be a multiple of 16). A
// later wave that resolves the iChannel fixture pack also widens
// the preamble with `iDate` (vec4) and a `vec4[1]` packed
// `iChannelTime` substitute when the body actually references
// either symbol.
layout(binding = 0) uniform float iTime;
layout(binding = 1) uniform vec3  iResolution;
layout(binding = 2) uniform float iTimeDelta;
layout(binding = 3) uniform int   iFrame;
layout(binding = 4) uniform vec4  iMouse;
"#;

/// Wrapper trailer.
///
/// Forwards the chromaticity-native fragment entry into the inlined
/// Shadertoy `mainImage`. `gl_FragCoord` is the canonical fragment-
/// coordinate built-in across GLSL versions; Shadertoy's `fragCoord`
/// has identical semantics (origin at lower-left, units in pixels)
/// when the host supplies `iResolution` consistently.
const TRAILER: &str = r#"
void main() {
    vec4 _chromaticity_color = vec4(0.0);
    mainImage(_chromaticity_color, gl_FragCoord.xy);
    frag_color = _chromaticity_color;
}
"#;

/// Heuristic: the converter rejects a clearly VR-only Shadertoy
/// pattern at parse time because there is no plausible auto-mapping
/// for stereo eye projection, and writing a "compiles but renders
/// monocularly" output silently degrades the user's intent.
const VR_REJECT_NEEDLES: &[&str] = &[
    "mainVR(", "mainVR ", // tolerate the rare space-before-paren spelling
];

/// Heuristic: cubemap-entry rejection. Same posture as VR-mode —
/// Shadertoy's `mainCubemap(out vec4 fragColor, in vec3 rayOrigin,
/// in vec3 rayDir)` shape outputs to a 6-face cubemap render target,
/// which has no auto-mapping into Chromaticity's single-pass
/// `mainImage`-equivalent shape. Surfaced as
/// [`Warning::CubemapEntrypointDeferred`] inside the
/// [`ImportError::UnsupportedShadertoyFeature`] payload so the
/// import UI carries actionable copy.
const CUBEMAP_REJECT_NEEDLES: &[&str] = &["mainCubemap(", "mainCubemap "];

/// Validate that `source_glsl` carries a well-formed Shadertoy
/// `mainImage` entry point.
///
/// The check is deliberately light — a substring scan rather than a
/// full GLSL parse, because the wrapped fragment is round-tripped
/// through naga later in the pipeline and any misshapen body is
/// caught there. We just need enough confidence that wrapping a
/// `void main()` around it isn't immediately nonsensical.
fn validate_shadertoy_source(source: &str) -> Result<(), ImportError> {
    if source.trim().is_empty() {
        return Err(ImportError::InvalidShadertoyGlsl(
            "source_glsl is empty".to_string(),
        ));
    }
    // Tolerate Shadertoy's `void mainImage(out vec4 fragColor, in
    // vec2 fragCoord)` plus a few common whitespace variants. We
    // require the literal `mainImage` plus the `out vec4` /
    // `in vec2` phrases.
    if !source.contains("mainImage") {
        return Err(ImportError::InvalidShadertoyGlsl(
            "source_glsl does not contain a `mainImage` entry point".to_string(),
        ));
    }
    if !source.contains("out vec4") {
        return Err(ImportError::InvalidShadertoyGlsl(
            "source_glsl `mainImage` signature missing `out vec4` parameter".to_string(),
        ));
    }
    if !source.contains("in vec2") {
        return Err(ImportError::InvalidShadertoyGlsl(
            "source_glsl `mainImage` signature missing `in vec2` parameter".to_string(),
        ));
    }
    Ok(())
}

/// Reject hard-incompatible features before any I/O fires.
fn reject_unsupported(source: &str) -> Result<(), ImportError> {
    for needle in VR_REJECT_NEEDLES {
        if source.contains(needle) {
            return Err(ImportError::UnsupportedShadertoyFeature(format!(
                "VR-mode Shadertoy patterns (matched `{}`) cannot be auto-mapped",
                needle.trim()
            )));
        }
    }
    for needle in CUBEMAP_REJECT_NEEDLES {
        if source.contains(needle) {
            return Err(ImportError::UnsupportedShadertoyFeature(
                Warning::CubemapEntrypointDeferred.to_string(),
            ));
        }
    }
    Ok(())
}

/// Detect which `iChannelN` indices the body references.
///
/// Returns a sorted unique Vec of channel indices in `[0, 4)` whose
/// `iChannelN` symbol appears in `source`.
fn detect_referenced_ichannels(source: &str) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(4);
    for index in 0..4u8 {
        let needle = format!("iChannel{index}");
        if source.contains(&needle) {
            out.push(index);
        }
    }
    out
}

/// Auto-correct deprecated `texture2D(` and `textureCube(` call
/// forms to the modern `texture(` form.
///
/// Returns the rewritten body plus a flag indicating whether the
/// auto-correction fired (so the converter can emit a single
/// [`Warning::DeprecatedTexture2DAutoCorrected`]).
fn autocorrect_deprecated_texture(source: &str) -> (String, bool) {
    let mut out = source.to_string();
    let mut fired = false;
    if out.contains("texture2D(") {
        out = out.replace("texture2D(", "texture(");
        fired = true;
    }
    if out.contains("textureCube(") {
        out = out.replace("textureCube(", "texture(");
        fired = true;
    }
    (out, fired)
}

/// Rewrite `iChannelN` → `chroma_input_<N>` in `source` for every
/// channel index referenced by the body.
///
/// The wrapper preamble declares one sampler per referenced channel
/// (see [`build_ichannel_preamble`]); rewriting the body keeps the
/// converter's text-munging contract local to this module without a
/// full GLSL parse.
fn rewrite_ichannel_references(source: &str, referenced: &[u8]) -> String {
    let mut out = source.to_string();
    for &index in referenced {
        let from = format!("iChannel{index}");
        let to = format!("chroma_input_{index}");
        out = out.replace(&from, &to);
    }
    out
}

/// Collect structured conversion warnings for the given source plus
/// caller-supplied iChannel-mapping table.
///
/// `texture2d_autocorrected` is `true` iff
/// [`autocorrect_deprecated_texture`] fired (the warning belongs in
/// the `ImportReport`, not the body rewrite).
fn collect_warnings(
    source: &str,
    referenced_channels: &[u8],
    mappings: &[IChannelMapping],
    texture2d_autocorrected: bool,
) -> Vec<Warning> {
    let mut out = Vec::new();

    // Per-channel mapping warnings.
    for &index in referenced_channels {
        let mapping = mappings.iter().find(|m| m.channel_index == index);
        match mapping {
            None => out.push(Warning::MissingIChannelMapping {
                channel_index: index,
            }),
            Some(m) if m.kind == IChannelKind::Webcam => {
                out.push(Warning::WebcamIChannelDeferred {
                    channel_index: index,
                });
            }
            Some(_) => {}
        }
    }

    // iMouse continues to surface as a warning per PR 1's contract,
    // promoted to a structured variant.
    if source.contains("iMouse") {
        out.push(Warning::MouseUniformDeferred);
    }

    if texture2d_autocorrected {
        out.push(Warning::DeprecatedTexture2DAutoCorrected);
    }

    out
}

/// Build the per-iChannel sampler-declaration block the wrapper
/// preamble appends below the standard Shadertoy uniforms.
///
/// Naga's GLSL frontend rejects the combined `uniform sampler2D`
/// shape outside legacy compatibility profiles; per the same
/// `engine/render::multipass` rationale (`sampler2D(tex, samp)`
/// constructor pattern), we declare separated `texture2D` /
/// `samplerCube` + `sampler` pairs and expose a `#define
/// chroma_input_<N> sampler2D(_chroma_tex_<N>, _chroma_samp_<N>)`
/// alias so the rewritten body's `texture(chroma_input_<N>, uv)`
/// call site compiles unchanged. The iChannel slots use bindings
/// 5..=12 (each channel consumes 2 binding slots — texture +
/// sampler) so they sit after the 5 standard Shadertoy uniforms
/// (`iTime` / `iResolution` / `iTimeDelta` / `iFrame` / `iMouse`).
fn build_ichannel_preamble(referenced: &[u8], mappings: &[IChannelMapping]) -> String {
    if referenced.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(referenced.len() * 192);
    out.push_str("\n// --- iChannel resolution preamble (PR 2 widening) ---\n");
    for &index in referenced {
        let kind = mappings
            .iter()
            .find(|m| m.channel_index == index)
            .map(|m| m.kind)
            // No mapping declared: default to Texture2D so the body
            // compiles. The MissingIChannelMapping warning surfaces
            // the missing binding to the author.
            .unwrap_or(IChannelKind::Texture2D);
        let texture_binding = 5_u32 + u32::from(index) * 2;
        let sampler_binding = texture_binding + 1;
        let texture_type = kind.texture_type();
        let sampler_type = kind.combined_sampler_type();
        out.push_str(&format!(
            "layout(set = 0, binding = {texture_binding}) uniform {texture_type} \
             _chroma_tex_{index};\n",
        ));
        out.push_str(&format!(
            "layout(set = 0, binding = {sampler_binding}) uniform sampler \
             _chroma_samp_{index};\n",
        ));
        out.push_str(&format!(
            "#define chroma_input_{index} {sampler_type}(_chroma_tex_{index}, _chroma_samp_{index})\n",
        ));
    }
    out
}

/// Compose the wrapped fragment body the converter writes to
/// `main.glsl`.
///
/// `referenced_channels` + `mappings` drive the iChannel-preamble
/// declarations; `body_after_rewrites` is the user's body after both
/// `iChannelN → chroma_input_<N>` rewriting and any `texture2D`
/// auto-correction have fired upstream.
fn build_main_glsl(
    body_after_rewrites: &str,
    referenced_channels: &[u8],
    mappings: &[IChannelMapping],
) -> String {
    let ichannel_preamble = build_ichannel_preamble(referenced_channels, mappings);
    let mut out = String::with_capacity(
        PREAMBLE.len() + ichannel_preamble.len() + body_after_rewrites.len() + TRAILER.len() + 8,
    );
    out.push_str(PREAMBLE);
    out.push_str(&ichannel_preamble);
    out.push_str("\n// --- Shadertoy body (rewritten if iChannel/texture2D used) ---\n");
    out.push_str(body_after_rewrites);
    if !body_after_rewrites.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("// --- End Shadertoy body ---\n");
    out.push_str(TRAILER);
    out
}

/// Verify the wrapped fragment compiles + validates through the
/// naga GLSL frontend the engine/render single-pass entry point
/// uses internally.
///
/// Returns the underlying naga diagnostic shape on failure, formatted
/// the same way `engine/render::RenderError::FragmentParse` /
/// `FragmentValidate` would render it through their `Display` impls.
fn naga_compile_check(glsl_source: &str) -> Result<(), String> {
    let mut frontend = naga::front::glsl::Frontend::default();
    let module = match frontend.parse(
        &naga::front::glsl::Options {
            stage: naga::ShaderStage::Fragment,
            defines: Default::default(),
        },
        glsl_source,
    ) {
        Ok(m) => m,
        Err(errors) => {
            return Err(format!("naga GLSL parse rejected the wrapper: {errors:?}"));
        }
    };
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    if let Err(err) = validator.validate(&module) {
        return Err(format!("naga validator rejected the wrapper: {err:?}"));
    }
    Ok(())
}

/// Compose the `scene.json` body for an imported scene.
fn build_scene_json(config: &ShadertoyImportConfig, scene_id: &str) -> Result<String, ImportError> {
    let mut root = serde_json::Map::new();
    root.insert(
        "SCHEMA_VERSION".to_string(),
        serde_json::Value::from(SHADERTOY_SCHEMA_VERSION),
    );
    root.insert(
        "FORMAT_TAG".to_string(),
        serde_json::Value::String(SHADERTOY_FORMAT_TAG.to_string()),
    );
    root.insert(
        "TITLE".to_string(),
        serde_json::Value::String(config.title.clone()),
    );
    root.insert(
        "CREDIT".to_string(),
        serde_json::Value::String(config.credit.clone()),
    );
    root.insert(
        "DESCRIPTION".to_string(),
        serde_json::Value::String(config.description.clone()),
    );
    if let Some(ref id) = config.shadertoy_id {
        root.insert(
            "SHADERTOY_ID".to_string(),
            serde_json::Value::String(id.clone()),
        );
    }
    root.insert(
        "TAGS".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::String("shadertoy".to_string())]),
    );
    root.insert(
        "CATEGORIES".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::String("imported".to_string())]),
    );
    root.insert("CONTROLS".to_string(), serde_json::Value::Array(Vec::new()));

    // PR 2 widening: serialize the iChannel mapping table per
    // docs/38 §5.2a so the authoring UI can re-establish bindings
    // on subsequent loads. Empty Vec → omit the key entirely.
    if !config.ichannel_mappings.is_empty() {
        let mut entries = Vec::with_capacity(config.ichannel_mappings.len());
        for mapping in &config.ichannel_mappings {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "INDEX".to_string(),
                serde_json::Value::from(mapping.channel_index),
            );
            obj.insert(
                "KIND".to_string(),
                serde_json::Value::String(mapping.kind.wire_tag().to_string()),
            );
            if let Some(ref path) = mapping.source_path {
                obj.insert(
                    "SOURCE_PATH".to_string(),
                    serde_json::Value::String(path.clone()),
                );
            }
            entries.push(serde_json::Value::Object(obj));
        }
        root.insert("ICHANNELS".to_string(), serde_json::Value::Array(entries));
    }

    // Single-pass `backbuff` per docs/38 §5.2; TARGET names the
    // package's primary node-id. Slice-3a flatten-preserves
    // unknown per-pass keys, so `WIDTH` / `HEIGHT` are not stamped
    // here — caller can edit post-import.
    let mut pass = serde_json::Map::new();
    pass.insert(
        "TARGET".to_string(),
        serde_json::Value::String(format!("{scene_id}-backbuff")),
    );
    root.insert(
        "PASSES".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::Object(pass)]),
    );

    serde_json::to_string_pretty(&serde_json::Value::Object(root)).map_err(|err| {
        ImportError::InvalidShadertoyGlsl(format!("scene.json render failed: {err}"))
    })
}

/// Convert a slug-y package id from `config.title`.
///
/// Conservative ASCII-lowercase + `-` runs; non-alphanumeric
/// collapses to `-`, leading/trailing `-` strip, and an empty
/// result falls back to `shadertoy-import`. The converter does not
/// claim user-visible globally-unique ids — collisions with an
/// existing package surface as [`ImportError::TargetExists`].
fn slug_from_title(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut prev_dash = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "shadertoy-import".to_string()
    } else {
        trimmed
    }
}

/// Convert a Shadertoy fragment to a Chromaticity-native `.chroma`
/// package under `target_dir`.
///
/// On success the package lives at
/// `<target_dir>/<slug(config.title)>.chroma/`, populated with
/// `scene.json` and `main.glsl`. On any failure mid-write — including
/// the [`naga_compile_check`] pass — the partially-populated package
/// directory is removed.
///
/// The converter does NOT create `target_dir` itself; callers are
/// expected to point at an existing library root. If `target_dir`
/// is absent, the underlying `fs::create_dir` call surfaces an
/// `io::Error` of kind `NotFound`.
pub fn import_shadertoy_to_chroma(
    target_dir: &Path,
    config: &ShadertoyImportConfig,
) -> Result<ImportReport, ImportError> {
    validate_shadertoy_source(&config.source_glsl)?;
    reject_unsupported(&config.source_glsl)?;

    let scene_id = slug_from_title(&config.title);
    let package_dirname = format!("{scene_id}.chroma");
    let package_root = target_dir.join(&package_dirname);

    if package_root.exists() {
        return Err(ImportError::TargetExists(package_root));
    }

    // PR 2 pipeline: detect referenced iChannels → auto-correct
    // deprecated texture2D → rewrite iChannelN → chroma_input_<N> →
    // build wrapped GLSL with per-channel sampler preamble. The
    // structured warning vec is computed against the original
    // source so callers see warnings even when the rewrite consumes
    // the literal `iChannelN` symbol.
    let referenced_channels = detect_referenced_ichannels(&config.source_glsl);
    let (after_texture_rewrite, texture2d_autocorrected) =
        autocorrect_deprecated_texture(&config.source_glsl);
    let body_after_rewrites =
        rewrite_ichannel_references(&after_texture_rewrite, &referenced_channels);

    let main_glsl_body = build_main_glsl(
        &body_after_rewrites,
        &referenced_channels,
        &config.ichannel_mappings,
    );
    // Compile-check BEFORE we touch the filesystem so a naga-rejected
    // import never leaves a half-written package on disk.
    naga_compile_check(&main_glsl_body).map_err(ImportError::GlslCompileError)?;

    let scene_json_body = build_scene_json(config, &scene_id)?;
    let warnings = collect_warnings(
        &config.source_glsl,
        &referenced_channels,
        &config.ichannel_mappings,
        texture2d_autocorrected,
    );
    let conversion_warnings: Vec<String> = warnings.iter().map(ToString::to_string).collect();

    fs::create_dir(&package_root)?;

    let scene_json_path = package_root.join("scene.json");
    let main_glsl_path = package_root.join("main.glsl");

    if let Err(err) = write_files(
        &scene_json_path,
        &main_glsl_path,
        &scene_json_body,
        &main_glsl_body,
    ) {
        let _ = fs::remove_dir_all(&package_root);
        return Err(err);
    }

    Ok(ImportReport {
        package_root,
        files_written: vec![scene_json_path, main_glsl_path],
        conversion_warnings,
        warnings,
    })
}

fn write_files(
    scene_json_path: &Path,
    main_glsl_path: &Path,
    scene_json_body: &str,
    main_glsl_body: &str,
) -> Result<(), ImportError> {
    fs::write(scene_json_path, scene_json_body)?;
    fs::write(main_glsl_path, main_glsl_body)?;
    Ok(())
}

/// Convenience: route an imported package through the scene loader
/// to confirm the produced `scene.json` deserializes.
pub fn load_imported_scene(
    report: &ImportReport,
) -> Result<chromaticity_scene::schema::Scene, chromaticity_scene::loader::LoadError> {
    chromaticity_scene::loader::load_chroma_dir(&report.package_root)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-IMPORT-SHADERTOY (T1) — Shadertoy-import-converter
    //! drift pins per `docs/26-evidence-artifact-registry.md` row 64.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins the
    //! converter entry-point contract (mainImage validation,
    //! VR-rejection, iChannel/iMouse warning surface, target-exists
    //! rejection, naga-compile gating, slug derivation, on-disk file
    //! set, round-trip through the scene loader). The exhaustive
    //! 3+ golden-fixture corpus required by row 64 lives in
    //! `engine/authoring/tests/import_shadertoy_corpus.rs` (T2).
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_target_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let pid = std::process::id();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!("chromaticity-shadertoy-{label}-{pid}-{nanos}-{n}"));
        fs::create_dir_all(&path).expect("create unique target dir");
        path
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    fn config_for(title: &str, source: &str) -> ShadertoyImportConfig {
        ShadertoyImportConfig {
            shadertoy_id: Some("XsXXDn".to_string()),
            title: title.to_string(),
            credit: "Imported test fixture (CC-BY-NC-SA 3.0)".to_string(),
            description: "T1 fixture — Shadertoy import converter pin.".to_string(),
            source_glsl: source.to_string(),
            ichannel_mappings: Vec::new(),
        }
    }

    /// Minimal Shadertoy that exercises iTime — produces a horizontal
    /// stripe whose phase animates over time.
    const SHADERTOY_ITIME_ANIMATION: &str = r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    float t = fract(iTime * 0.25);
    fragColor = vec4(t, t, t, 1.0);
}
"#;

    /// Minimal Shadertoy that exercises iResolution to produce a
    /// circle whose radius accounts for aspect ratio.
    const SHADERTOY_IRESOLUTION_ASPECT: &str = r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = (fragCoord - 0.5 * iResolution.xy) / iResolution.y;
    float r = length(uv);
    float c = smoothstep(0.30, 0.28, r);
    fragColor = vec4(vec3(c), 1.0);
}
"#;

    #[test]
    fn shadertoy_simple_itime_animation_imports() {
        let target = unique_target_dir("itime");
        let config = config_for("Simple iTime Animation", SHADERTOY_ITIME_ANIMATION);

        let report =
            import_shadertoy_to_chroma(&target, &config).expect("simple iTime body imports");
        assert_eq!(
            report.package_root,
            target.join("simple-itime-animation.chroma"),
        );
        assert!(report.package_root.is_dir());
        assert_eq!(report.files_written.len(), 2);
        assert!(report.conversion_warnings.is_empty());

        cleanup(&target);
    }

    #[test]
    fn shadertoy_iresolution_aspect_ratio_imports() {
        let target = unique_target_dir("iresolution");
        let config = config_for("Aspect Circle", SHADERTOY_IRESOLUTION_ASPECT);

        let report =
            import_shadertoy_to_chroma(&target, &config).expect("iResolution body imports");
        assert!(report.package_root.is_dir());
        assert_eq!(report.files_written.len(), 2);

        cleanup(&target);
    }

    #[test]
    fn shadertoy_with_unmapped_ichannel_imports_with_missing_mapping_warning() {
        // PR 2 widening: an iChannel-referencing body with no
        // declared mapping now imports successfully (the wrapper
        // declares a fallback `sampler2D chroma_input_0`) and
        // surfaces a structured `MissingIChannelMapping { 0 }`
        // warning so the import UI prompts the author to bind it.
        let target = unique_target_dir("ichannel-missing");
        let source = r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    vec4 tex = texture(iChannel0, uv);
    fragColor = tex;
}
"#;
        let config = config_for("iChannel Missing", source);
        let report =
            import_shadertoy_to_chroma(&target, &config).expect("import succeeds with fallback");

        assert!(
            report
                .warnings
                .iter()
                .any(|w| matches!(w, Warning::MissingIChannelMapping { channel_index: 0 })),
            "missing-mapping warning fired; got {:?}",
            report.warnings,
        );

        // Verify the body rewrite consumed the literal `iChannel0`
        // symbol — main.glsl on disk no longer references it.
        let main_glsl =
            fs::read_to_string(report.package_root.join("main.glsl")).expect("read main.glsl");
        assert!(
            !main_glsl.contains("iChannel0"),
            "body rewrite consumed iChannel0",
        );
        assert!(
            main_glsl.contains("chroma_input_0"),
            "wrapper declares chroma_input_0 slot",
        );
        cleanup(&target);
    }

    #[test]
    fn shadertoy_with_audio_ichannel_mapping_imports_clean() {
        // PR 2 widening: explicit audio mapping → no warning fires.
        let target = unique_target_dir("ichannel-audio");
        let source = r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    float fft = texture(iChannel0, vec2(uv.x, 0.25)).r;
    fragColor = vec4(vec3(fft), 1.0);
}
"#;
        let config =
            config_for("iChannel Audio", source).with_ichannel_mappings(vec![IChannelMapping {
                channel_index: 0,
                kind: IChannelKind::Audio,
                source_path: Some("audio:///input".to_string()),
            }]);
        let report = import_shadertoy_to_chroma(&target, &config)
            .expect("audio-mapped iChannel imports successfully");
        assert!(
            !report
                .warnings
                .iter()
                .any(|w| matches!(w, Warning::MissingIChannelMapping { .. })),
            "no MissingIChannelMapping warning when mapping is supplied; got {:?}",
            report.warnings,
        );
        // Verify the scene.json carries the ICHANNELS table.
        let scene_json =
            fs::read_to_string(report.package_root.join("scene.json")).expect("read scene.json");
        assert!(
            scene_json.contains("\"ICHANNELS\""),
            "scene.json carries ICHANNELS",
        );
        assert!(
            scene_json.contains("\"audio\""),
            "scene.json records the audio kind tag",
        );
        cleanup(&target);
    }

    #[test]
    fn shadertoy_with_webcam_mapping_emits_deferred_warning() {
        let target = unique_target_dir("ichannel-webcam");
        let source = r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    fragColor = texture(iChannel0, uv);
}
"#;
        let config =
            config_for("iChannel Webcam", source).with_ichannel_mappings(vec![IChannelMapping {
                channel_index: 0,
                kind: IChannelKind::Webcam,
                source_path: None,
            }]);
        let report =
            import_shadertoy_to_chroma(&target, &config).expect("webcam mapping still imports");
        assert!(
            report
                .warnings
                .iter()
                .any(|w| matches!(w, Warning::WebcamIChannelDeferred { channel_index: 0 })),
            "webcam-deferral warning fired; got {:?}",
            report.warnings,
        );
        cleanup(&target);
    }

    #[test]
    fn shadertoy_with_deprecated_texture2d_auto_corrects() {
        let target = unique_target_dir("texture2d");
        let source = r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    vec4 tex = texture2D(iChannel0, uv);
    fragColor = tex;
}
"#;
        let config = config_for("Texture2D Deprecated", source).with_ichannel_mappings(vec![
            IChannelMapping {
                channel_index: 0,
                kind: IChannelKind::Texture2D,
                source_path: Some("textures/noise.png".to_string()),
            },
        ]);
        let report =
            import_shadertoy_to_chroma(&target, &config).expect("auto-corrected import succeeds");
        assert!(
            report
                .warnings
                .iter()
                .any(|w| matches!(w, Warning::DeprecatedTexture2DAutoCorrected)),
            "auto-correction warning fired; got {:?}",
            report.warnings,
        );
        let main_glsl =
            fs::read_to_string(report.package_root.join("main.glsl")).expect("read main.glsl");
        assert!(
            !main_glsl.contains("texture2D("),
            "deprecated texture2D consumed by auto-correction",
        );
        cleanup(&target);
    }

    #[test]
    fn shadertoy_main_cubemap_rejects_with_deferral_warning_text() {
        let target = unique_target_dir("cubemap-reject");
        let source = r#"
void mainCubemap(out vec4 fragColor, in vec3 rayOrigin, in vec3 rayDir) {
    fragColor = vec4(rayDir, 1.0);
}
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    fragColor = vec4(0.5);
}
"#;
        let config = config_for("Cubemap Pattern", source);
        let err = import_shadertoy_to_chroma(&target, &config)
            .expect_err("mainCubemap pattern must reject");
        match err {
            ImportError::UnsupportedShadertoyFeature(msg) => {
                assert!(
                    msg.contains("mainCubemap"),
                    "rejection message names the feature; got {msg:?}",
                );
            }
            other => panic!("expected UnsupportedShadertoyFeature, got {other:?}"),
        }
        cleanup(&target);
    }

    #[test]
    fn shadertoy_invalid_glsl_returns_glsl_compile_error() {
        let target = unique_target_dir("bad-glsl");
        // Has the right mainImage signature so it passes the
        // shape-check, but the body references an undeclared
        // identifier so naga rejects it.
        let source = r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    fragColor = totally_undefined_function(fragCoord);
}
"#;
        let config = config_for("Broken Body", source);
        let err =
            import_shadertoy_to_chroma(&target, &config).expect_err("broken body must reject");
        match err {
            ImportError::GlslCompileError(_) => {}
            other => panic!("expected GlslCompileError, got {other:?}"),
        }
        cleanup(&target);
    }

    #[test]
    fn shadertoy_missing_mainimage_returns_invalid() {
        let target = unique_target_dir("no-mainimage");
        let source = r#"
void main() { gl_FragColor = vec4(1.0); }
"#;
        let config = config_for("Not Shadertoy", source);
        let err = import_shadertoy_to_chroma(&target, &config)
            .expect_err("non-Shadertoy source must reject");
        assert!(matches!(err, ImportError::InvalidShadertoyGlsl(_)));
        cleanup(&target);
    }

    #[test]
    fn shadertoy_vr_pattern_returns_unsupported() {
        let target = unique_target_dir("vr-reject");
        let source = r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    fragColor = vec4(0.5);
}
void mainVR(out vec4 fragColor, in vec2 fragCoord, in vec3 ro, in vec3 rd) {
    fragColor = vec4(rd, 1.0);
}
"#;
        let config = config_for("VR Pattern", source);
        let err = import_shadertoy_to_chroma(&target, &config).expect_err("VR pattern must reject");
        assert!(matches!(err, ImportError::UnsupportedShadertoyFeature(_)));
        cleanup(&target);
    }

    #[test]
    fn import_to_existing_target_returns_target_exists() {
        let target = unique_target_dir("exists");
        let config = config_for("Simple iTime Animation", SHADERTOY_ITIME_ANIMATION);

        let _first = import_shadertoy_to_chroma(&target, &config).expect("first import succeeds");
        let err =
            import_shadertoy_to_chroma(&target, &config).expect_err("second import must reject");
        match err {
            ImportError::TargetExists(path) => {
                assert_eq!(path, target.join("simple-itime-animation.chroma"));
            }
            other => panic!("expected TargetExists, got {other:?}"),
        }
        cleanup(&target);
    }

    #[test]
    fn imported_chroma_loads_via_chromaticity_scene() {
        let target = unique_target_dir("loads");
        let config = config_for("Simple iTime Animation", SHADERTOY_ITIME_ANIMATION);
        let report = import_shadertoy_to_chroma(&target, &config).expect("import succeeds");

        let scene = load_imported_scene(&report).expect("loader accepts imported scene");
        assert_eq!(scene.header.schema_version, SHADERTOY_SCHEMA_VERSION);
        let format_tag = scene
            .extensions
            .get("FORMAT_TAG")
            .and_then(|v| v.as_str())
            .expect("FORMAT_TAG survives load");
        assert_eq!(format_tag, SHADERTOY_FORMAT_TAG);
        let title = scene
            .extensions
            .get("TITLE")
            .and_then(|v| v.as_str())
            .expect("TITLE survives load");
        assert_eq!(title, config.title);
        assert_eq!(scene.passes.len(), 1);
        cleanup(&target);
    }

    #[test]
    fn imported_chroma_compiles_via_naga_directly() {
        // Re-validate post-write that the on-disk main.glsl (not just
        // the in-memory wrapper) still parses + validates. This
        // catches any I/O-time mutation (newline normalization, BOM
        // stripping, etc.) that would otherwise drift between the
        // pre-write compile gate and the on-disk artifact authors
        // edit.
        let target = unique_target_dir("post-write-compile");
        let config = config_for("Aspect Circle", SHADERTOY_IRESOLUTION_ASPECT);
        let report = import_shadertoy_to_chroma(&target, &config).expect("import succeeds");

        let main_glsl_path = report.package_root.join("main.glsl");
        let on_disk = fs::read_to_string(&main_glsl_path).expect("read main.glsl");
        naga_compile_check(&on_disk).expect("on-disk main.glsl parses + validates");
        cleanup(&target);
    }

    #[test]
    fn slug_derivation_handles_punctuation_and_unicode() {
        assert_eq!(slug_from_title("Hello World"), "hello-world");
        assert_eq!(slug_from_title("  --- "), "shadertoy-import");
        assert_eq!(slug_from_title("ABC123"), "abc123");
        assert_eq!(slug_from_title("foo!!bar"), "foo-bar");
    }
}
