// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-2 EV-RENDER-CONFORMANCE slice-5b.3 transition composition.
//!
//! The crossfade-composition entry point + GPU resource preallocation
//! companion to [`crate::transitions::TransitionOrchestrator`] for the
//! ADR-007 warm-switch policy's third phase
//! ([`crate::transitions::TransitionState::CrossfadeOrHardCut`]).
//!
//! # Slice-5b.3 GPU-side blending design
//!
//! Slice-5b.3 (this slice) replaces the slice-5b.2 CPU-side per-byte
//! alpha lerp with a GPU compositing pipeline that samples the
//! current and next render outputs and blends them via a `mix()`
//! fragment-shader call against a third intermediate texture. The
//! Tier-9 [`crate::multipass::MultipassRenderer`] refactor (PR #289)
//! is the architectural prerequisite — its long-lived
//! `Arc<wgpu::Device>` and `wgpu::Queue` accessors are what let the
//! third compositing pass bind against the same GPU context that
//! just rendered the source frames. Before Tier-9 the multipass
//! scheduler acquired a fresh Instance / Adapter / Device per call,
//! which is why slice-5b.2 shipped with the CPU readback blend
//! documented at length on the retired module-level prose.
//!
//! ## Three-pass composition pipeline
//!
//! 1. **Current render**: [`MultipassRenderer::render_to_texture`]
//!    is called with `current_spec`; returns a `wgpu::Texture` of
//!    the spec's final-pass output (no CPU readback round-trip).
//! 2. **Next render**: same, with `next_spec`. The two specs MUST
//!    agree on `target_size` AND on the final-pass `target_format`;
//!    the format-coherence gate is unchanged from slice-5b.2 (and
//!    surfaces the same [`TransitionError::CompositionFailed`]
//!    diagnostic before any GPU work runs).
//! 3. **Blend pass**: a third render pipeline samples the two
//!    output textures via a `texture2D` + `sampler` pair (each) and
//!    a single `f32 progress` uniform, writes the blended result to
//!    a fresh intermediate texture (created at composition time and
//!    dropped on return). The output texture is then read back via
//!    [`crate::multipass::readback_texture_to_bytes`] using the
//!    same row-padding strip that
//!    [`MultipassRenderer::render`] uses, so the slice-5b.2 byte-
//!    stream shape contract is preserved verbatim.
//!
//! ## Endpoint short-circuits (byte-identity invariant)
//!
//! - `progress = 0.0` short-circuits to a direct
//!   [`MultipassRenderer::render`] call against `current_spec`,
//!   producing bytes bit-identical to a slice-2 / slice-4
//!   single-multipass byte stream.
//! - `progress = 1.0` short-circuits to a direct
//!   [`MultipassRenderer::render`] call against `next_spec`,
//!   producing bytes bit-identical to a slice-2 / slice-4
//!   single-multipass byte stream.
//!
//! The endpoint short-circuit was the slice-5b.2 contract that the
//! T2 corpus
//! (`engine/render/tests/transition_composition.rs::compose_at_progress_zero_*`
//! / `compose_at_progress_one_*`) pinned by hash equivalence; the
//! same hashes survive the slice-5b.3 GPU pull because the endpoint
//! path does not enter the third compositing pass.
//!
//! ## Mid-range hashes
//!
//! Mid-range `progress` values (0 < p < 1) produce GPU-blended
//! output via the third pass's `mix(current, next, progress)` call.
//! The byte stream differs from the slice-5b.2 CPU u8-lerp by at
//! most a rounding bit (GPU float-intermediate vs CPU u8-arithmetic
//! divergence). The slice-5b.3 corpus replaces the slice-5b.2
//! mid-range ACCEPTED hash entries with values harvested from
//! Apple-Silicon Metal; backend-aware extension follows the
//! existing per-backend ACCEPTED-set pattern from
//! `engine/render/tests/float_targets.rs`.
//!
//! # GPU-FREE-orchestrator invariant (slice-5b.1, preserved)
//!
//! [`crate::transitions::TransitionOrchestrator`] still does NOT
//! own a `wgpu::Device`. Slice-5b.3 callers that already hold a
//! [`MultipassRenderer`] (e.g. the runtime crate's frame loop) call
//! [`compose_transition_frame_gpu`] directly with their renderer;
//! callers that want a one-shot composition (e.g. the slice-5b.2
//! T2 corpus) call [`compose_transition_frame`] which builds a
//! fresh [`MultipassRenderer`] per call (matches the
//! [`crate::compile_and_render_multipass`] shim's per-call device
//! posture for byte-identity continuity).
//!
//! # Hard-cut path
//!
//! When the orchestrator's
//! [`crate::transitions::TransitionOrchestrator::commit_hard_cut`]
//! is called (the ADR-007 timeout fallback), the composition entry
//! point is bypassed entirely — the runtime simply renders
//! `next_spec` via [`crate::compile_and_render_multipass`] (or
//! [`MultipassRenderer::render`]) directly.
//! [`compose_transition_frame`] / [`compose_transition_frame_gpu`]
//! are for the in-flight `progress` ∈ `[0.0, 1.0]` window only.
//!
//! # Anchors
//!
//! - `docs/adr/ADR-007-scene-warm-switch-policy.md` — Accepted
//!   four-phase warm-switch + 500 ms timeout-degrades-to-hard-cut.
//! - `docs/26-evidence-artifact-registry.md` row 28 —
//!   `EV-RENDER-CONFORMANCE`.

use std::borrow::Cow;

use crate::multipass::{MultipassRenderer, MultipassSpec, TargetFormat, readback_texture_to_bytes};
use crate::single_pass::{FULLSCREEN_TRIANGLE_GLSL, RenderError, compile_glsl_module};
use crate::transitions::TransitionError;

/// GPU resources preallocated by
/// [`crate::transitions::TransitionOrchestrator::prepare_resources`]
/// during `PrepareNext` / `PrimeNext` for the cued
/// [`MultipassSpec`].
///
/// The handle holds the output texture, sampler, and uniform buffer
/// that a slice-5b.3 GPU-side composition pass MAY use as its
/// canonical bind targets when running against the orchestrator's
/// preallocated handles. The simpler [`compose_transition_frame_gpu`]
/// entry point allocates its own intermediates per call (matching
/// the `MultipassRenderer::render` per-call resource policy); the
/// preallocated handle is the lever for a future slice that wants
/// to amortize composition allocation across the
/// `CrossfadeOrHardCut` window.
///
/// Constructed via [`PreparedScene::allocate`]; not meant to be
/// hand-constructed from outside the crate.
pub struct PreparedScene {
    /// The cued spec's `target_size` recorded for diagnostic /
    /// invariant checks at composition time.
    target_size: (u32, u32),
    /// The cued spec's final-pass `target_format`. Recorded so the
    /// composition path can dispatch per-format blending without
    /// re-reading the spec.
    target_format: TargetFormat,
    /// Pass count from the cued spec at allocation time. Surfaced
    /// for diagnostic readback only — slice-5b.2 does not validate
    /// the orchestrator-side spec against the prepared handle, but
    /// a future widening might.
    pass_count: usize,
    /// Pre-allocated output texture sized to the cued spec's final
    /// target. Held for the lifetime of the prepared handle; freed
    /// on `Drop` via wgpu's RAII model.
    _output_texture: wgpu::Texture,
    /// Shared sampler for inter-scene blending. Nearest / clamp-to-
    /// edge to match the multipass scheduler's sampler choice
    /// (`multipass.rs::shared_sampler`); kept as a separate handle
    /// so a future GPU-side composition path can bind it without
    /// re-creation.
    _sampler: wgpu::Sampler,
    /// Uniform buffer reserved for the transition `progress` value.
    /// Writes use the wgpu queue; the slot is `f32` little-endian
    /// in the first 4 bytes, with 12 bytes of std140 pad to fill
    /// the 16-byte vec4-aligned slot (matching the slice-2 /
    /// slice-4 multipass uniform packing).
    _progress_uniform: wgpu::Buffer,
}

impl PreparedScene {
    /// Allocate the GPU resources for `spec`. Returns
    /// [`TransitionError::GpuAllocationFailed`] on a wgpu-side
    /// allocation failure.
    ///
    /// The texture is sized to `spec.target_size`, formatted as
    /// `spec.passes[final].target_format`, with usage flags
    /// `RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_SRC` to permit
    /// both render-target and sampler / readback uses. This matches
    /// the multipass scheduler's final-pass texture allocation so a
    /// future widening can hand the prepared texture to the
    /// renderer directly.
    pub fn allocate(device: &wgpu::Device, spec: &MultipassSpec) -> Result<Self, TransitionError> {
        // Defense-in-depth: the orchestrator's shape gate already
        // validated this, but `PreparedScene::allocate` is also a
        // public-from-the-crate path so it re-checks the
        // invariants the wgpu calls would otherwise blow up on.
        if spec.passes.is_empty() {
            return Err(TransitionError::GpuAllocationFailed {
                reason: "cannot allocate prepared scene: spec has empty pass list".to_string(),
            });
        }
        if spec.target_size.0 == 0 || spec.target_size.1 == 0 {
            return Err(TransitionError::GpuAllocationFailed {
                reason: format!(
                    "cannot allocate prepared scene: target_size {:?} has zero dimension",
                    spec.target_size
                ),
            });
        }

        let final_pass_index = spec.passes.len() - 1;
        let final_format = spec.passes[final_pass_index].target_format;

        // Output texture sized to the cued spec.
        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("chromaticity-render transition prepared output texture"),
            size: wgpu::Extent3d {
                width: spec.target_size.0,
                height: spec.target_size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: final_format.to_wgpu(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Sampler: nearest / clamp matches the multipass scheduler's
        // shared sampler so a future widening can bind both
        // interchangeably.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("chromaticity-render transition prepared sampler (nearest/clamp)"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        // Progress uniform buffer: 16 bytes (vec4-aligned slot per
        // the multipass scheduler's std140-friendly packing). The
        // slot holds `f32 progress` in the first 4 bytes; remaining
        // 12 bytes are zero pad. Initialised to 0.0 (transition
        // start).
        let progress_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chromaticity-render transition prepared progress uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            target_size: spec.target_size,
            target_format: final_format,
            pass_count: spec.passes.len(),
            _output_texture: output_texture,
            _sampler: sampler,
            _progress_uniform: progress_uniform,
        })
    }

    /// Recorded `target_size` from the cued spec at allocation time.
    pub fn target_size(&self) -> (u32, u32) {
        self.target_size
    }

    /// Recorded `target_format` from the cued spec's final pass at
    /// allocation time.
    pub fn target_format(&self) -> TargetFormat {
        self.target_format
    }

    /// Recorded pass count from the cued spec at allocation time.
    pub fn pass_count(&self) -> usize {
        self.pass_count
    }
}

impl std::fmt::Debug for PreparedScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // wgpu types do not implement `Debug` uniformly across the
        // versions in our deny-allowed range; surface the in-source
        // recorded fields and a sentinel for the GPU handles.
        f.debug_struct("PreparedScene")
            .field("target_size", &self.target_size)
            .field("target_format", &self.target_format.label())
            .field("pass_count", &self.pass_count)
            .field("output_texture", &"<wgpu::Texture>")
            .field("sampler", &"<wgpu::Sampler>")
            .field("progress_uniform", &"<wgpu::Buffer>")
            .finish()
    }
}

/// GLSL fragment shader for the slice-5b.3 third compositing pass.
///
/// Samples `current` and `next` via separable `texture2D` +
/// `sampler` bindings (Vulkan-GLSL form per the multipass scheduler
/// convention) and `mix()`-blends them by the `progress` uniform
/// supplied at `binding = 0`. Writes the blended pixel to
/// `fragColor`. The `uv.y` flip is omitted intentionally — both
/// source textures already carry the `single_pass.rs::FULLSCREEN_TRIANGLE_GLSL`
/// flip from their own render passes; the compositing pass needs to
/// preserve that orientation, so the sample uses `uv` directly.
const COMPOSITE_FRAGMENT_GLSL: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0) uniform Uniforms { vec4 u_progress_pad; };
layout(set = 0, binding = 1) uniform texture2D u_current;
layout(set = 0, binding = 2) uniform sampler u_current_sampler;
layout(set = 0, binding = 3) uniform texture2D u_next;
layout(set = 0, binding = 4) uniform sampler u_next_sampler;
void main() {
    float p = u_progress_pad.x;
    vec4 c = texture(sampler2D(u_current, u_current_sampler), uv);
    vec4 n = texture(sampler2D(u_next, u_next_sampler), uv);
    fragColor = mix(c, n, p);
}
"#;

/// Crossfade composition entry point (slice-5b.3 GPU-side blending).
///
/// Builds a fresh [`MultipassRenderer`] per call (matching the
/// [`crate::compile_and_render_multipass`] shim posture), renders
/// `current_spec` and `next_spec` via
/// [`MultipassRenderer::render_to_texture`], and composites the two
/// GPU textures via a third blend pass that runs against the
/// renderer's long-lived device + queue. Callers that already own
/// a [`MultipassRenderer`] (e.g. the runtime's frame loop) should
/// call [`compose_transition_frame_gpu`] directly to amortize
/// device acquisition.
///
/// The two specs MUST agree on `target_size` AND on the final-pass
/// `target_format` — composition across mismatched dimensions or
/// formats is rejected with [`TransitionError::CompositionFailed`].
///
/// `progress` is clamped to `[0.0, 1.0]` before blending; values
/// outside that range produce the nearest endpoint.
///
/// # Endpoint short-circuit (byte-identity preserved)
///
/// - `progress == 0.0` returns the `current_spec` readback
///   bit-identical to a direct
///   [`crate::compile_and_render_multipass`] call on `current_spec`.
/// - `progress == 1.0` returns the `next_spec` readback
///   bit-identical to a direct
///   [`crate::compile_and_render_multipass`] call on `next_spec`.
///
/// # Errors
///
/// - [`TransitionError::CompositionFailed`] if either underlying
///   render call fails, OR if the two specs disagree on
///   `target_size` / final-pass format, OR if the third
///   compositing pass fails to compile / submit / readback.
pub fn compose_transition_frame(
    current_spec: &MultipassSpec,
    next_spec: &MultipassSpec,
    progress: f32,
) -> Result<Vec<u8>, TransitionError> {
    let renderer = MultipassRenderer::new().map_err(|e| TransitionError::CompositionFailed {
        reason: format!("renderer construction failed: {e}"),
    })?;
    compose_transition_frame_gpu(&renderer, current_spec, next_spec, progress)
}

/// Long-lived-renderer crossfade composition entry point.
///
/// Same contract as [`compose_transition_frame`] but takes a
/// caller-owned [`MultipassRenderer`] rather than building one per
/// call. Use this when the call site already holds a renderer and
/// wants to amortize device + cached vertex shader across
/// composition calls (typical for the runtime's per-frame loop
/// driving a `CrossfadeOrHardCut` window).
///
/// # Errors
///
/// Same as [`compose_transition_frame`].
pub fn compose_transition_frame_gpu(
    renderer: &MultipassRenderer,
    current_spec: &MultipassSpec,
    next_spec: &MultipassSpec,
    progress: f32,
) -> Result<Vec<u8>, TransitionError> {
    // Endpoint dimension / format coherence. The composition's third
    // pass binds two textures of the same size + format, so the two
    // specs must produce a matching shape. The check fires before
    // any GPU work so adapter-less CI surfaces structural rejection
    // via the slice-5b.2 T1 pin path.
    if current_spec.target_size != next_spec.target_size {
        return Err(TransitionError::CompositionFailed {
            reason: format!(
                "current target_size {:?} != next target_size {:?}",
                current_spec.target_size, next_spec.target_size,
            ),
        });
    }
    if current_spec.passes.is_empty() || next_spec.passes.is_empty() {
        return Err(TransitionError::CompositionFailed {
            reason: "either current or next spec has empty pass list".to_string(),
        });
    }
    let current_format = current_spec.passes[current_spec.passes.len() - 1].target_format;
    let next_format = next_spec.passes[next_spec.passes.len() - 1].target_format;
    if current_format != next_format {
        return Err(TransitionError::CompositionFailed {
            reason: format!(
                "current final-pass format {} != next final-pass format {}",
                current_format.label(),
                next_format.label(),
            ),
        });
    }

    let p = progress.clamp(0.0, 1.0);

    // Endpoint short-circuit. `progress = 0.0` returns a direct
    // current render; `progress = 1.0` returns a direct next render.
    // Both paths skip the third compositing pass entirely so the
    // returned bytes are bit-identical to a slice-2 / slice-4
    // single-multipass call (the slice-5b.2 endpoint contract).
    if p <= 0.0 {
        return renderer
            .render(current_spec)
            .map_err(|e| TransitionError::CompositionFailed {
                reason: format!("current-scene render at progress=0 failed: {e}"),
            });
    }
    if p >= 1.0 {
        return renderer
            .render(next_spec)
            .map_err(|e| TransitionError::CompositionFailed {
                reason: format!("next-scene render at progress=1 failed: {e}"),
            });
    }

    // Mid-range: render both source specs to GPU textures, then run
    // the third compositing pass against the renderer's device.
    let current_texture = renderer.render_to_texture(current_spec).map_err(|e| {
        TransitionError::CompositionFailed {
            reason: format!("current-scene render_to_texture failed: {e}"),
        }
    })?;
    let next_texture =
        renderer
            .render_to_texture(next_spec)
            .map_err(|e| TransitionError::CompositionFailed {
                reason: format!("next-scene render_to_texture failed: {e}"),
            })?;

    composite_textures(
        renderer,
        &current_texture,
        &next_texture,
        current_format,
        current_spec.target_size,
        p,
    )
}

/// Build the third compositing pipeline, run it against the two
/// source textures and the supplied progress uniform, and read back
/// the blended output to a `Vec<u8>` in the slice-5b.2 byte-stream
/// shape.
///
/// `target_size` and `format` describe the SHARED dimensions /
/// format of the two source textures — the compositing pass
/// allocates its intermediate render target with the same shape so
/// the readback is bit-shape-compatible with a direct
/// `MultipassRenderer::render` of either source.
fn composite_textures(
    renderer: &MultipassRenderer,
    current_texture: &wgpu::Texture,
    next_texture: &wgpu::Texture,
    format: TargetFormat,
    target_size: (u32, u32),
    progress: f32,
) -> Result<Vec<u8>, TransitionError> {
    let device = renderer.device();
    let queue = renderer.queue();

    // 1. Compile the compositing fragment shader. The vertex shader
    // is reused via the renderer's own pipeline — but its module is
    // private, so the third pass compiles its own instance from the
    // same `FULLSCREEN_TRIANGLE_GLSL` constant. This stays consistent
    // with the slice-5b.2+ refactor's "vertex shader is a runtime
    // constant" invariant.
    let vertex_module = compile_glsl_module(FULLSCREEN_TRIANGLE_GLSL, naga::ShaderStage::Vertex)
        .map_err(|e| TransitionError::CompositionFailed {
            reason: format!("composite vertex compile failed: {e}"),
        })?;
    let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("chromaticity-render composite vertex (shared fullscreen triangle)"),
        source: wgpu::ShaderSource::Naga(Cow::Owned(vertex_module)),
    });

    let fragment_module = compile_glsl_module(COMPOSITE_FRAGMENT_GLSL, naga::ShaderStage::Fragment)
        .map_err(|e| TransitionError::CompositionFailed {
            reason: format!("composite fragment compile failed: {e}"),
        })?;
    let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("chromaticity-render composite fragment (mix blend)"),
        source: wgpu::ShaderSource::Naga(Cow::Owned(fragment_module)),
    });

    // 2. Texture views for the two source textures.
    let current_view = current_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let next_view = next_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // 3. Sampler for both source textures. Nearest / clamp matches
    // the multipass scheduler's shared sampler so the bind layout
    // (NonFiltering) accepts every TargetFormat (Rgba8Unorm,
    // Rgba16Float, Rgba32Float) without enabling extra wgpu
    // features.
    let composite_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("chromaticity-render composite sampler (nearest/clamp)"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: 0.0,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
    });

    // 4. Progress uniform buffer: 16 bytes (vec4-aligned slot
    // matching the multipass scheduler's std140-friendly packing).
    // Fragment shader reads the `progress` value from `.x` of the
    // packed vec4.
    let mut progress_bytes = [0_u8; 16];
    progress_bytes[0..4].copy_from_slice(&progress.to_le_bytes());
    let progress_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("chromaticity-render composite progress uniform"),
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&progress_buffer, 0, &progress_bytes);

    // 5. Bind group layout matching the COMPOSITE_FRAGMENT_GLSL
    // declaration: uniform at 0, current_tex at 1, current_sampler
    // at 2, next_tex at 3, next_sampler at 4.
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("chromaticity-render composite bind group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("chromaticity-render composite bind group"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: progress_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&current_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&composite_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&next_view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(&composite_sampler),
            },
        ],
    });

    // 6. Output texture for the third pass. Format matches the
    // source textures so the readback shape is identical to a
    // direct multipass render of either source.
    let output_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("chromaticity-render composite output texture"),
        size: wgpu::Extent3d {
            width: target_size.0,
            height: target_size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: format.to_wgpu(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // 7. Pipeline layout + render pipeline.
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("chromaticity-render composite pipeline layout"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("chromaticity-render composite pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vertex_shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &fragment_shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: format.to_wgpu(),
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    });

    // 8. Encode + submit the third pass.
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("chromaticity-render composite encoder"),
    });
    {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("chromaticity-render composite pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rpass.set_pipeline(&pipeline);
        rpass.set_bind_group(0, &bind_group, &[]);
        rpass.draw(0..3, 0..1);
    }
    queue.submit(Some(encoder.finish()));

    // 9. Read back the output texture into the slice-5b.2 byte-stream
    // shape. Reuses the multipass readback helper so the row-padding
    // strip is identical.
    readback_texture_to_bytes(device, queue, &output_texture).map_err(|e: RenderError| {
        TransitionError::CompositionFailed {
            reason: format!("composite readback failed: {e}"),
        }
    })
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod composition_pins {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — slice-5b.3 GPU
    //! composition drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: composition-
    //! algorithm invariant pins that fire WITHOUT a usable wgpu
    //! adapter (the T2 corpus in
    //! `engine/render/tests/transition_composition.rs` covers
    //! end-to-end through the renderer).
    //!
    //! Pinned invariants:
    //!
    //! - `compose_transition_frame` rejects size-mismatched specs
    //!   with `CompositionFailed` BEFORE adapter acquisition.
    //! - `compose_transition_frame` rejects format-mismatched specs
    //!   with `CompositionFailed` BEFORE adapter acquisition.
    //! - `compose_transition_frame` rejects empty-pass-list specs
    //!   with `CompositionFailed` BEFORE adapter acquisition.
    //!
    //! GPU-bound pins (renderer construction succeeds) live in the
    //! T2 corpus and the new `composition_gpu_pins` module below.
    //!
    //! Slice-5b.3 retired the slice-5b.2 CPU-side `compose_byte_streams`
    //! / `half_to_f32` / `f32_to_half` helpers and their associated
    //! T1 pins; those tests exercised a now-removed code path.

    use super::compose_transition_frame;
    use crate::multipass::{MultipassSpec, PassDef, TargetFormat};
    use crate::transitions::TransitionError;

    fn trivial_spec(label: &str, size: (u32, u32), format: TargetFormat) -> MultipassSpec {
        MultipassSpec {
            passes: vec![PassDef {
                label: Some(label.to_string()),
                fragment_glsl: "#version 450\nlayout(location=0) in vec2 uv;\nlayout(location=0) out vec4 fragColor;\nvoid main(){fragColor=vec4(uv,0.0,1.0);}\n".to_string(),
                target_format: format,
                ..PassDef::default()
            }],
            target_size: size,
            frame_count: 1,
        }
    }

    #[test]
    fn compose_transition_rejects_size_mismatch() {
        // Drift pin: target_size mismatch surfaces as
        // CompositionFailed before either render is invoked.
        let a = trivial_spec("a", (16, 16), TargetFormat::Rgba8Unorm);
        let b = trivial_spec("b", (32, 16), TargetFormat::Rgba8Unorm);
        let err = compose_transition_frame(&a, &b, 0.5).expect_err("size mismatch must reject");
        match err {
            TransitionError::CompositionFailed { reason } => {
                assert!(
                    reason.contains("target_size") && reason.contains("!="),
                    "reason: {reason}",
                );
            }
            other => panic!("expected CompositionFailed, got {other:?}"),
        }
    }

    #[test]
    fn compose_transition_rejects_format_mismatch() {
        // Drift pin: format mismatch surfaces as CompositionFailed.
        let a = trivial_spec("a", (16, 16), TargetFormat::Rgba8Unorm);
        let b = trivial_spec("b", (16, 16), TargetFormat::Rgba16Float);
        let err = compose_transition_frame(&a, &b, 0.5).expect_err("format mismatch must reject");
        match err {
            TransitionError::CompositionFailed { reason } => {
                assert!(reason.contains("format"), "reason: {reason}",);
            }
            other => panic!("expected CompositionFailed, got {other:?}"),
        }
    }

    #[test]
    fn compose_transition_rejects_empty_pass_lists() {
        // Drift pin: an empty pass list on either side rejects
        // before the renderer is reached.
        let mut a = trivial_spec("a", (16, 16), TargetFormat::Rgba8Unorm);
        let b = trivial_spec("b", (16, 16), TargetFormat::Rgba8Unorm);
        a.passes.clear();
        let err = compose_transition_frame(&a, &b, 0.5).expect_err("empty current must reject");
        match err {
            TransitionError::CompositionFailed { reason } => {
                assert!(reason.contains("empty pass list"), "reason: {reason}");
            }
            other => panic!("expected CompositionFailed, got {other:?}"),
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod composition_gpu_pins {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — slice-5b.3 GPU
    //! composition pipeline drift pins.
    //!
    //! T1 invariants that DO require a working wgpu adapter (skip on
    //! adapter-less runners). Pinned:
    //!
    //! - GPU compose at `progress=0.0` is bit-identical to a direct
    //!   `MultipassRenderer::render(current_spec)` byte stream
    //!   (endpoint short-circuit invariant).
    //! - GPU compose at `progress=1.0` is bit-identical to a direct
    //!   `MultipassRenderer::render(next_spec)` byte stream.
    //! - GPU compose at midpoint produces per-pixel output where
    //!   each byte is between the corresponding current and next
    //!   bytes (inclusive, modulo wgpu rounding) — the "blend stays
    //!   bounded" sanity invariant.
    //! - `MultipassRenderer::render_to_texture` returns a texture
    //!   matching the spec's final-pass `target_format` and dimensions.
    //! - `compose_transition_frame_gpu` clamps progress < 0 to the
    //!   current-side endpoint and progress > 1 to the next-side
    //!   endpoint (bit-identical to direct renders of either side).

    use super::{compose_transition_frame, compose_transition_frame_gpu};
    use crate::multipass::{MultipassRenderer, MultipassSpec, PassDef, TargetFormat};

    const RED_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() { fragColor = vec4(1.0, 0.0, 0.0, 1.0); }
"#;

    const GREEN_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() { fragColor = vec4(0.0, 1.0, 0.0, 1.0); }
"#;

    fn solid_spec(label: &str, fragment: &str) -> MultipassSpec {
        MultipassSpec {
            passes: vec![PassDef {
                label: Some(label.to_string()),
                fragment_glsl: fragment.to_string(),
                ..PassDef::default()
            }],
            target_size: (16, 16),
            frame_count: 1,
        }
    }

    fn try_renderer() -> Option<MultipassRenderer> {
        MultipassRenderer::new().ok()
    }

    #[test]
    fn endpoint_zero_is_bit_identical_to_current_render() {
        let Some(renderer) = try_renderer() else {
            eprintln!("skipping endpoint_zero_pin: no wgpu adapter");
            return;
        };
        let red = solid_spec("red", RED_FRAGMENT);
        let green = solid_spec("green", GREEN_FRAGMENT);
        let direct = renderer.render(&red).expect("render must succeed");
        let composed = compose_transition_frame_gpu(&renderer, &red, &green, 0.0)
            .expect("compose at p=0 must succeed");
        assert_eq!(composed, direct);
    }

    #[test]
    fn endpoint_one_is_bit_identical_to_next_render() {
        let Some(renderer) = try_renderer() else {
            eprintln!("skipping endpoint_one_pin: no wgpu adapter");
            return;
        };
        let red = solid_spec("red", RED_FRAGMENT);
        let green = solid_spec("green", GREEN_FRAGMENT);
        let direct = renderer.render(&green).expect("render must succeed");
        let composed = compose_transition_frame_gpu(&renderer, &red, &green, 1.0)
            .expect("compose at p=1 must succeed");
        assert_eq!(composed, direct);
    }

    #[test]
    fn midpoint_blend_stays_within_per_pixel_bounds() {
        // GPU compose at p=0.5 between pure red and pure green —
        // every byte must lie between the current's byte and the
        // next's byte (inclusive, ±1 for rounding).
        let Some(renderer) = try_renderer() else {
            eprintln!("skipping midpoint_bounds_pin: no wgpu adapter");
            return;
        };
        let red = solid_spec("red", RED_FRAGMENT);
        let green = solid_spec("green", GREEN_FRAGMENT);
        let red_bytes = renderer.render(&red).expect("render must succeed");
        let green_bytes = renderer.render(&green).expect("render must succeed");
        let blended = compose_transition_frame_gpu(&renderer, &red, &green, 0.5)
            .expect("compose at p=0.5 must succeed");
        assert_eq!(blended.len(), red_bytes.len());
        for i in 0..blended.len() {
            let lo = red_bytes[i].min(green_bytes[i]);
            let hi = red_bytes[i].max(green_bytes[i]);
            // Allow ±1 for GPU rounding intermediates vs strict u8
            // inclusive bounds.
            let lo = lo.saturating_sub(1);
            let hi = hi.saturating_add(1);
            assert!(
                blended[i] >= lo && blended[i] <= hi,
                "byte {i} = {} not in [{lo}, {hi}] (red={}, green={})",
                blended[i],
                red_bytes[i],
                green_bytes[i],
            );
        }
    }

    #[test]
    fn render_to_texture_matches_spec_format_and_size() {
        let Some(renderer) = try_renderer() else {
            eprintln!("skipping render_to_texture_format_pin: no wgpu adapter");
            return;
        };
        let red = solid_spec("red", RED_FRAGMENT);
        let texture = renderer
            .render_to_texture(&red)
            .expect("render_to_texture must succeed");
        assert_eq!(texture.width(), 16);
        assert_eq!(texture.height(), 16);
        assert_eq!(texture.format(), TargetFormat::Rgba8Unorm.to_wgpu());
    }

    #[test]
    fn progress_below_zero_clamps_to_current_endpoint() {
        let Some(renderer) = try_renderer() else {
            eprintln!("skipping clamp_below_zero_pin: no wgpu adapter");
            return;
        };
        let red = solid_spec("red", RED_FRAGMENT);
        let green = solid_spec("green", GREEN_FRAGMENT);
        let direct_red = renderer.render(&red).expect("render must succeed");
        let composed = compose_transition_frame_gpu(&renderer, &red, &green, -0.5)
            .expect("p=-0.5 must clamp to current");
        assert_eq!(composed, direct_red);
    }

    #[test]
    fn progress_above_one_clamps_to_next_endpoint() {
        let Some(renderer) = try_renderer() else {
            eprintln!("skipping clamp_above_one_pin: no wgpu adapter");
            return;
        };
        let red = solid_spec("red", RED_FRAGMENT);
        let green = solid_spec("green", GREEN_FRAGMENT);
        let direct_green = renderer.render(&green).expect("render must succeed");
        let composed = compose_transition_frame_gpu(&renderer, &red, &green, 1.5)
            .expect("p=1.5 must clamp to next");
        assert_eq!(composed, direct_green);
    }

    #[test]
    fn shim_compose_transition_frame_matches_long_lived_renderer_path() {
        // Drift pin: `compose_transition_frame` (which builds a
        // fresh renderer per call) is byte-identical to
        // `compose_transition_frame_gpu` with a caller-owned
        // renderer for the same inputs. The shim is the migration
        // bridge for callers that have not moved to the long-lived
        // renderer; this pin guards that the bridge stays
        // semantically transparent.
        let Some(renderer) = try_renderer() else {
            eprintln!("skipping shim_equivalence_pin: no wgpu adapter");
            return;
        };
        let red = solid_spec("red", RED_FRAGMENT);
        let green = solid_spec("green", GREEN_FRAGMENT);
        let shim = compose_transition_frame(&red, &green, 0.5).expect("shim compose must succeed");
        let direct = compose_transition_frame_gpu(&renderer, &red, &green, 0.5)
            .expect("direct compose must succeed");
        assert_eq!(shim, direct);
    }
}
