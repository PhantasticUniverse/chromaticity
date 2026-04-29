// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Multi-pass GLSL → SPIR-V → wgpu render scheduler (slice 2).
//!
//! Slice 2 builds on the slice-1 single-pass entry point by adding
//! sequential multi-pass scheduling: each pass renders into its own
//! offscreen `RGBA8Unorm` texture, prior-pass textures are exposed to
//! later passes as `texture2D` + `sampler` bindings, and per-pass
//! scalar / vector uniforms are marshalled into a single std140
//! uniform buffer per pass. Only the FINAL pass's readback is
//! returned to the caller, matching the slice-1 byte format so the
//! `EV-RENDER-CONFORMANCE` corpus can hash-pin the output.
//!
//! # Architectural choices that may surprise a code reader
//!
//! - **Local `MultipassSpec` rather than `engine/scene::PassSpec`.** The
//!   render scheduler intentionally does NOT depend on `engine/scene`.
//!   The scheduler's input ([`MultipassSpec`]) is plain data: a flat
//!   list of per-pass shader source + uniform values + input-texture
//!   references. A future small adapter PR can produce a
//!   [`MultipassSpec`] from `engine/scene::SceneIr` (or expose a
//!   `to_multipass()` on `PassSpec`); that crate-coupling decision is
//!   its own slice. The principle preserved here is that the render
//!   backend is scene-agnostic — it doesn't know what JSON
//!   `engine/scene::PassSpec` deserialized from, only what frame data
//!   it needs to draw.
//!
//! - **Forward references are forbidden, not topologically sorted.** A
//!   pass at index `i` may sample any pass at index `j < i`. A pass
//!   referencing index `>= i` returns
//!   [`RenderError::PassInputOutOfRange`]. Slice-2 passes are linear /
//!   non-cyclic by construction; full cycle detection across feedback
//!   edges is a slice-3 concern.
//!
//! - **Shared hardcoded GLSL vertex shader.** Every pass uses the
//!   slice-1 `FULLSCREEN_TRIANGLE_GLSL` vertex shader. See
//!   `single_pass.rs::FULLSCREEN_TRIANGLE_GLSL` doc comment for the
//!   GLSL-vs-WGSL choice rationale (interpolation-sampling parity
//!   with the user fragment).
//!
//! - **One uniform buffer per pass, std140-friendly.** All scalar /
//!   vector uniforms supplied via [`UniformInput`] for a single pass
//!   are packed into ONE uniform buffer at group 0, binding 0. Each
//!   value occupies a 16-byte slot regardless of underlying width
//!   (a `f32` lives in the first 4 bytes of its slot, the remaining
//!   12 bytes are pad). This keeps std140 vec4-alignment satisfied
//!   without a per-type packing rulebook; the cost is at most 12
//!   bytes of pad per scalar uniform, which is negligible at the
//!   uniform-count we expect (single-digit per pass for slice 2's
//!   evidence corpus). Slice 3 will revisit packing when audio
//!   uniforms (an `audio[]` block) and `syn_RES` style vec2 batched
//!   uniforms enter the surface.
//!
//! - **Texture / sampler bind layout is naga-GLSL-compliant Vulkan
//!   shape.** naga's GLSL frontend rejects combined `uniform sampler2D`
//!   declarations as top-level resources; the Vulkan-GLSL form with a
//!   separate `texture2D` and `sampler` is required, combined at use
//!   site via the `sampler2D(tex, samp)` constructor. Slice-2's binding
//!   numbering, per pass, is:
//!
//!   ```text
//!   layout(set = 0, binding = 0)            uniform Uniforms { ... };
//!   layout(set = 0, binding = 1)            uniform texture2D pass0;
//!   layout(set = 0, binding = 2)            uniform sampler   pass0_sampler;
//!   layout(set = 0, binding = 3)            uniform texture2D pass1;
//!   layout(set = 0, binding = 4)            uniform sampler   pass1_sampler;
//!   ```
//!
//!   i.e. the uniform block (binding 0, omitted if no uniforms),
//!   then each input pass at `1 + 2*i` (texture) and `1 + 2*i + 1`
//!   (sampler). The exposed sampler is a standard wgpu nearest /
//!   clamp-to-edge sampler — slice 3 widens this to honor wrap /
//!   filter per-pass extension keys from `engine/scene::PassSpec`.
//!
//! - **Sync wrapper around wgpu async.** Same rationale as
//!   `single_pass.rs`: [`pollster::block_on`] applied at each leaf
//!   future (rather than `async fn` / `async move {}`) keeps the
//!   source well-formed under lefthook's standalone
//!   `rustfmt --style-edition=2024 --check`.
//!
//! # Slice 2 vs Slice 1 deltas
//!
//! - Slice 1: single offscreen pass, no textures, no uniforms.
//! - Slice 2: N passes, output texture per non-final pass, prior-pass
//!   sampling, std140 uniform buffer. Final readback path is
//!   bit-identical to slice 1's (same wgpu format, same row-padding
//!   strip).
//!
//! # Still deferred (not in this slice)
//!
//! - Feedback-texture sampling (a pass reading its OWN previous-frame
//!   output) — slice 3.
//! - Transition orchestration (warm-switch between scene graphs) —
//!   slice 4 (ADR-007).
//! - Audio uniform binding (`syn_BPM`, `syn_LEVEL`, `audio[]` 1D
//!   sampler) — slice 5 (`engine/audio` integration).
//! - `RGBA32F` / `RGBA16F` float-target conformance — slice 3 (per
//!   ADR-011 §Decision).
//! - DX12 backend (Tauri 2 + windows-rs version conflict; tracked in
//!   `engine/render/Cargo.toml`).
//! - Adapter-driven `MultipassSpec` from `engine/scene::PassSpec` —
//!   own slice with the crate-coupling decision.

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::mpsc::{TryRecvError, channel};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::single_pass::{FULLSCREEN_TRIANGLE_GLSL, RenderError, compile_glsl_module};

/// Plain-data scheduler input.
///
/// Constructed by callers (or, in a follow-up slice, by an adapter
/// over `engine/scene::PassSpec`). Slice-2 keeps this surface free of
/// a `PassSpec` dependency on purpose — see module-level doc.
#[derive(Debug, Clone)]
pub struct MultipassSpec {
    /// Ordered pass list. Pass `i` may sample any pass at index `< i`.
    pub passes: Vec<PassDef>,
    /// Final-pass render target dimensions (width, height). All
    /// intermediate passes also use this size in slice 2; per-pass
    /// `WIDTH` / `HEIGHT` extension overrides land with slice 3's
    /// `PassSpec` adapter.
    pub target_size: (u32, u32),
}

/// Single-pass definition inside a [`MultipassSpec`].
#[derive(Debug, Clone)]
pub struct PassDef {
    /// Optional debug label propagated into wgpu object labels.
    pub label: Option<String>,
    /// User-authored GLSL 4.50 fragment shader source. The shader's
    /// uniform / texture / sampler binding layout MUST match the
    /// per-pass scheme described in the module-level doc (uniforms
    /// at `set = 0, binding = 0`, input textures at `1 + 2*i`,
    /// input samplers at `1 + 2*i + 1`).
    pub fragment_glsl: String,
    /// Scalar / vector uniforms packed into the pass's uniform buffer
    /// in declaration order. Skip if the pass has no uniforms.
    pub uniforms: Vec<UniformInput>,
    /// References to PRIOR passes (`source_pass_index < this pass's
    /// index`) whose output texture should be exposed to the
    /// fragment shader as a `texture2D` + `sampler` pair.
    pub input_textures: Vec<PassInputBinding>,
}

/// One named uniform value.
#[derive(Debug, Clone)]
pub struct UniformInput {
    /// Diagnostic name only — the binding position is determined by
    /// declaration order in [`PassDef::uniforms`], not by name.
    pub name: String,
    pub value: UniformValue,
}

/// Scalar / vector uniform value.
///
/// Each variant occupies one 16-byte slot in the per-pass uniform
/// buffer regardless of the underlying width (see module-level doc
/// for the std140-friendly packing choice).
#[derive(Debug, Clone, Copy)]
pub enum UniformValue {
    Scalar(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Int(i32),
    UInt(u32),
}

/// Reference into [`MultipassSpec::passes`] for a prior pass whose
/// output should be sampled by the current pass.
#[derive(Debug, Clone)]
pub struct PassInputBinding {
    /// Index of the prior pass (`< current pass's index`). Forward
    /// or self references return [`RenderError::PassInputOutOfRange`].
    pub source_pass_index: usize,
    /// Diagnostic name only — the binding positions are determined by
    /// declaration order in [`PassDef::input_textures`] (textures at
    /// `1 + 2*i`, samplers at `1 + 2*i + 1`).
    pub sampler_uniform_name: String,
}

/// Each [`UniformValue`] occupies a vec4 slot in the per-pass uniform
/// buffer. `Pod`+`Zeroable` so we can use `bytemuck::cast_slice` on
/// `&[UniformSlot]` for the buffer-init contents. The derive macros
/// emit the unsafe `Pod` / `Zeroable` impls under their own
/// `#[allow(unsafe_code)]` shim, which is the only path this slice
/// can reach `Pod` at the workspace's `unsafe_code = forbid` lint
/// (matches `single_pass.rs::ReservedVertexLayout`).
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)]
struct UniformSlot {
    /// 16 raw bytes covering scalar (4 + 12 pad), vec2 (8 + 8 pad),
    /// vec3 (12 + 4 pad), vec4 (16), int / uint (4 + 12 pad).
    bytes: [u8; 16],
}

/// Compile every pass in `spec` and render them sequentially, returning
/// the FINAL pass's readback bytes (4 bytes per pixel, row-major,
/// padding stripped — same shape as
/// [`crate::compile_and_render_single_pass`]).
///
/// # Errors
///
/// - [`RenderError::EmptyPassList`] — `spec.passes` is empty.
/// - [`RenderError::TargetSizeInvalid`] — width or height is zero.
/// - [`RenderError::PassInputOutOfRange`] — a pass references a
///   later or self pass index.
/// - [`RenderError::FragmentParse`] / [`RenderError::FragmentValidate`] —
///   any pass's GLSL fails the naga frontend.
/// - [`RenderError::UniformBindingFailed`] — uniform marshalling
///   error (currently never fires on valid input; reserved for
///   slice-3 audio uniform plumbing).
/// - [`RenderError::PassRenderFailed`] — any per-pass wgpu submission
///   / readback failure, wrapping the original [`RenderError`] from
///   the inner pass with the offending pass index for triage.
/// - [`RenderError::AdapterUnavailable`] / [`RenderError::DeviceRequest`] /
///   [`RenderError::DeviceLoss`] / [`RenderError::BufferMap`] — wgpu
///   device acquisition or readback failure.
pub fn compile_and_render_multipass(spec: &MultipassSpec) -> Result<Vec<u8>, RenderError> {
    // 1. Up-front structural validation. These checks intentionally
    // run BEFORE wgpu adapter acquisition so a structural error on a
    // headless CI runner without a usable adapter still returns the
    // expected variant rather than masking it with
    // `AdapterUnavailable`.
    if spec.passes.is_empty() {
        return Err(RenderError::EmptyPassList);
    }
    if spec.target_size.0 == 0 || spec.target_size.1 == 0 {
        return Err(RenderError::TargetSizeInvalid(spec.target_size));
    }
    let num_passes = spec.passes.len();
    for (pass_index, pass) in spec.passes.iter().enumerate() {
        for binding in &pass.input_textures {
            if binding.source_pass_index >= pass_index {
                return Err(RenderError::PassInputOutOfRange {
                    pass_index,
                    source_pass_index: binding.source_pass_index,
                    num_passes,
                });
            }
        }
    }

    // 2. Adapter + device acquisition. See `single_pass.rs` lines
    // 193-205 for the in-source explanation of why each leaf future
    // is wrapped with `pollster::block_on` rather than the body
    // being declared `async`.
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        flags: wgpu::InstanceFlags::default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        backend_options: wgpu::BackendOptions::default(),
    });

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        force_fallback_adapter: false,
        compatible_surface: None,
    }))
    .map_err(|_| RenderError::AdapterUnavailable)?;

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("chromaticity-render multipass device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::default(),
    }))
    .map_err(|e| RenderError::DeviceRequest(format!("{e}")))?;
    let device = Arc::new(device);

    // 3. Compile the shared full-screen-triangle vertex shader once.
    let vertex_module = compile_glsl_module(FULLSCREEN_TRIANGLE_GLSL, naga::ShaderStage::Vertex)?;
    let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("chromaticity-render multipass vertex (shared fullscreen triangle)"),
        source: wgpu::ShaderSource::Naga(Cow::Owned(vertex_module)),
    });

    // 4. Allocate one render target per pass. Each pass's target is
    // also `COPY_SRC` so the FINAL pass can be read back via
    // `copy_texture_to_buffer`; intermediate targets are
    // `TEXTURE_BINDING` so later passes can sample them.
    let color_format = wgpu::TextureFormat::Rgba8Unorm;
    let bytes_per_pixel: u32 = 4;
    let mut pass_targets: Vec<wgpu::Texture> = Vec::with_capacity(num_passes);
    let mut pass_views: Vec<wgpu::TextureView> = Vec::with_capacity(num_passes);
    for pass_index in 0..num_passes {
        let is_final = pass_index + 1 == num_passes;
        let usage = if is_final {
            // Final pass: render to + read back.
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING
        } else {
            // Intermediate pass: render to + later passes sample it.
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING
        };
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("chromaticity-render multipass target"),
            size: wgpu::Extent3d {
                width: spec.target_size.0,
                height: spec.target_size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: color_format,
            usage,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        pass_targets.push(target);
        pass_views.push(view);
    }

    // 5. A single shared sampler for ALL inter-pass texture lookups.
    // Slice-2's sampler is nearest / clamp-to-edge — the deterministic
    // choice that makes the readback hash bit-stable. Slice-3's
    // adapter over `PassSpec::extensions["WRAP"]` /
    // `extensions["FILTER"]` will widen this surface.
    let shared_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("chromaticity-render multipass shared sampler (nearest/clamp)"),
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

    // 6. Encode all passes into one command buffer; submit at the end.
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("chromaticity-render multipass encoder"),
    });

    // The pipeline / bind-group / uniform-buffer state for each pass
    // must outlive `begin_render_pass` (the borrow reaches into them
    // through the bind groups) and must not be dropped before
    // `encoder.finish()`. We collect them into a `Vec` keyed by pass
    // index and let the loop borrow into it.
    struct PassResources {
        _uniform_buffer: Option<wgpu::Buffer>,
        _bind_group_layout: wgpu::BindGroupLayout,
        bind_group: wgpu::BindGroup,
        _pipeline_layout: wgpu::PipelineLayout,
        pipeline: wgpu::RenderPipeline,
        _vertex_buffer: wgpu::Buffer,
    }
    let mut pass_resources: Vec<PassResources> = Vec::with_capacity(num_passes);

    for (pass_index, pass) in spec.passes.iter().enumerate() {
        let pass_label = pass.label.as_deref().unwrap_or("(unlabelled)");

        // 6a. Build bind group layout entries.
        let mut bgl_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();
        let uniform_buffer = if pass.uniforms.is_empty() {
            None
        } else {
            // Pack uniforms into 16-byte slots (see UniformSlot doc).
            let slots = encode_uniform_slots(&pass.uniforms, pass_label)?;
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("chromaticity-render multipass uniform buffer"),
                contents: bytemuck::cast_slice(&slots),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            bgl_entries.push(wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });
            Some(buffer)
        };

        for (i, _binding) in pass.input_textures.iter().enumerate() {
            // Texture at 1 + 2*i, sampler at 1 + 2*i + 1.
            // u32 cast: i is bounded by per-pass input count, well
            // below u32::MAX in any realistic scene.
            let tex_binding: u32 = 1 + 2 * (i as u32);
            let smp_binding: u32 = tex_binding + 1;
            bgl_entries.push(wgpu::BindGroupLayoutEntry {
                binding: tex_binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
            bgl_entries.push(wgpu::BindGroupLayoutEntry {
                binding: smp_binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                count: None,
            });
        }

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("chromaticity-render multipass bind group layout"),
            entries: &bgl_entries,
        });

        // 6b. Build bind group entries — must mirror the layout's
        // bindings exactly. The texture / sampler refs borrow into
        // `pass_views` and `shared_sampler`.
        let mut bg_entries: Vec<wgpu::BindGroupEntry> = Vec::new();
        if let Some(buffer) = uniform_buffer.as_ref() {
            bg_entries.push(wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            });
        }
        for (i, binding) in pass.input_textures.iter().enumerate() {
            let tex_binding: u32 = 1 + 2 * (i as u32);
            let smp_binding: u32 = tex_binding + 1;
            // Validation already ensured source_pass_index < pass_index.
            // Defense-in-depth: would only fire if validation was
            // bypassed; not reachable on valid input.
            let view = pass_views.get(binding.source_pass_index).ok_or(
                RenderError::PassInputOutOfRange {
                    pass_index,
                    source_pass_index: binding.source_pass_index,
                    num_passes,
                },
            )?;
            bg_entries.push(wgpu::BindGroupEntry {
                binding: tex_binding,
                resource: wgpu::BindingResource::TextureView(view),
            });
            bg_entries.push(wgpu::BindGroupEntry {
                binding: smp_binding,
                resource: wgpu::BindingResource::Sampler(&shared_sampler),
            });
        }
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("chromaticity-render multipass bind group"),
            layout: &bgl,
            entries: &bg_entries,
        });

        // 6c. Compile the pass's fragment shader.
        let fragment_module = compile_glsl_module(&pass.fragment_glsl, naga::ShaderStage::Fragment)
            .map_err(|e| RenderError::PassRenderFailed {
                pass_index,
                source: Box::new(e),
            })?;
        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chromaticity-render multipass fragment (user GLSL)"),
            source: wgpu::ShaderSource::Naga(Cow::Owned(fragment_module)),
        });

        // 6d. Pipeline layout + pipeline.
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chromaticity-render multipass pipeline layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("chromaticity-render multipass pipeline"),
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
                    format: color_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        // 6e. Reserved vertex buffer to keep the bytemuck pipeline
        // observable from this slice's binary; mirrors slice-1.
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chromaticity-render multipass reserved vertex buffer"),
            contents: bytemuck::cast_slice(&[UniformSlot::default(); 3]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        pass_resources.push(PassResources {
            _uniform_buffer: uniform_buffer,
            _bind_group_layout: bgl,
            bind_group,
            _pipeline_layout: pipeline_layout,
            pipeline,
            _vertex_buffer: vertex_buffer,
        });
    }

    // 7. Record one render pass per spec pass. Each writes into its
    // own target view (`pass_views[pass_index]`) and binds the
    // bind group built above.
    for (pass_index, resources) in pass_resources.iter().enumerate() {
        let view = pass_views
            .get(pass_index)
            .ok_or(RenderError::PassRenderFailed {
                pass_index,
                // Defense-in-depth; not reachable on valid input.
                source: Box::new(RenderError::EmptyPassList),
            })?;
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("chromaticity-render multipass pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
        rpass.set_pipeline(&resources.pipeline);
        rpass.set_bind_group(0, &resources.bind_group, &[]);
        rpass.draw(0..3, 0..1);
    }

    // 8. Allocate the readback buffer for the FINAL pass and copy.
    let unpadded_bytes_per_row = spec.target_size.0 * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
    let readback_size = u64::from(padded_bytes_per_row) * u64::from(spec.target_size.1);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("chromaticity-render multipass readback"),
        size: readback_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let final_index = num_passes - 1;
    let final_target = pass_targets
        .get(final_index)
        .ok_or(RenderError::PassRenderFailed {
            pass_index: final_index,
            source: Box::new(RenderError::EmptyPassList),
        })?;
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: final_target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(spec.target_size.1),
            },
        },
        wgpu::Extent3d {
            width: spec.target_size.0,
            height: spec.target_size.1,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(Some(encoder.finish()));

    // 9. Map readback buffer and strip row padding. Identical to
    // single_pass.rs's readback path.
    let buffer_slice = readback.slice(..);
    let (sender, receiver) = channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });

    loop {
        match device.poll(wgpu::PollType::Wait) {
            Ok(_) => {}
            Err(e) => return Err(RenderError::DeviceLoss(format!("{e:?}"))),
        }
        match receiver.try_recv() {
            Ok(Ok(())) => break,
            Ok(Err(e)) => return Err(RenderError::BufferMap(format!("{e}"))),
            Err(TryRecvError::Empty) => continue,
            Err(TryRecvError::Disconnected) => {
                return Err(RenderError::BufferMap(
                    "map_async sender dropped before completion".to_string(),
                ));
            }
        }
    }

    let mapped = buffer_slice.get_mapped_range();
    let mut pixels =
        Vec::with_capacity((unpadded_bytes_per_row as usize) * (spec.target_size.1 as usize));
    for row in 0..spec.target_size.1 {
        let start = (row * padded_bytes_per_row) as usize;
        let end = start + unpadded_bytes_per_row as usize;
        pixels.extend_from_slice(&mapped[start..end]);
    }
    drop(mapped);
    readback.unmap();

    Ok(pixels)
}

/// Encode a slice of [`UniformInput`] into vec4-aligned 16-byte
/// slots. Each value lives at the start of its slot; remaining bytes
/// are zero pad. See module-level doc for the std140-friendly
/// rationale.
fn encode_uniform_slots(
    uniforms: &[UniformInput],
    pass_label: &str,
) -> Result<Vec<UniformSlot>, RenderError> {
    let mut slots = Vec::with_capacity(uniforms.len());
    for input in uniforms {
        let mut slot = UniformSlot::default();
        match input.value {
            UniformValue::Scalar(v) => {
                let bytes = v.to_le_bytes();
                slot.bytes[0..4].copy_from_slice(&bytes);
            }
            UniformValue::Vec2(v) => {
                slot.bytes[0..4].copy_from_slice(&v[0].to_le_bytes());
                slot.bytes[4..8].copy_from_slice(&v[1].to_le_bytes());
            }
            UniformValue::Vec3(v) => {
                slot.bytes[0..4].copy_from_slice(&v[0].to_le_bytes());
                slot.bytes[4..8].copy_from_slice(&v[1].to_le_bytes());
                slot.bytes[8..12].copy_from_slice(&v[2].to_le_bytes());
            }
            UniformValue::Vec4(v) => {
                slot.bytes[0..4].copy_from_slice(&v[0].to_le_bytes());
                slot.bytes[4..8].copy_from_slice(&v[1].to_le_bytes());
                slot.bytes[8..12].copy_from_slice(&v[2].to_le_bytes());
                slot.bytes[12..16].copy_from_slice(&v[3].to_le_bytes());
            }
            UniformValue::Int(v) => {
                slot.bytes[0..4].copy_from_slice(&v.to_le_bytes());
            }
            UniformValue::UInt(v) => {
                slot.bytes[0..4].copy_from_slice(&v.to_le_bytes());
            }
        }
        // Sanity hook for slice-3 audio uniforms — not reachable on
        // the slice-2 enum's variants but keeps the marshalling
        // surface explicit so a future widening of `UniformValue`
        // can return a structured error rather than producing a
        // silent garbage value.
        if input.name.is_empty() {
            return Err(RenderError::UniformBindingFailed {
                pass_label: pass_label.to_string(),
                name: input.name.clone(),
                reason: "uniform name must not be empty".to_string(),
            });
        }
        slots.push(slot);
    }
    Ok(slots)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod multipass_drift_pins {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — multipass scheduler drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: scheduler-
    //! input validation pins that fire WITHOUT a usable wgpu adapter
    //! (the T2 corpus in `engine/render/tests/multipass.rs` covers
    //! the GPU-roundtrip path).
    //!
    //! Pinned invariants:
    //!
    //! - Empty pass list rejects with [`RenderError::EmptyPassList`].
    //! - Forward-reference input rejects with
    //!   [`RenderError::PassInputOutOfRange`].
    //! - Self-reference input rejects with
    //!   [`RenderError::PassInputOutOfRange`] (cycle prevention).
    //! - Zero target size rejects with
    //!   [`RenderError::TargetSizeInvalid`].
    //! - Uniform marshalling produces the expected byte layout for
    //!   each [`UniformValue`] variant (drift pin against future
    //!   std140 widening).

    use super::{
        MultipassSpec, PassDef, PassInputBinding, RenderError, UniformInput, UniformValue,
        compile_and_render_multipass, encode_uniform_slots,
    };

    #[test]
    fn empty_pass_list_returns_empty_pass_list_error() {
        let spec = MultipassSpec {
            passes: vec![],
            target_size: (16, 16),
        };
        let err = compile_and_render_multipass(&spec)
            .expect_err("empty pass list must reject before adapter acquisition");
        assert!(
            matches!(err, RenderError::EmptyPassList),
            "expected EmptyPassList, got {err:?}",
        );
    }

    #[test]
    fn forward_reference_returns_pass_input_out_of_range() {
        // Pass 0 attempts to sample pass 1 (index 1 >= current 0).
        let trivial = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() { fragColor = vec4(uv, 0.0, 1.0); }
"#
        .to_string();
        let spec = MultipassSpec {
            passes: vec![
                PassDef {
                    label: Some("forward-ref-pass-0".to_string()),
                    fragment_glsl: trivial.clone(),
                    uniforms: vec![],
                    input_textures: vec![PassInputBinding {
                        source_pass_index: 1,
                        sampler_uniform_name: "pass1".to_string(),
                    }],
                },
                PassDef {
                    label: Some("forward-ref-pass-1".to_string()),
                    fragment_glsl: trivial,
                    uniforms: vec![],
                    input_textures: vec![],
                },
            ],
            target_size: (16, 16),
        };
        let err = compile_and_render_multipass(&spec)
            .expect_err("forward reference must reject before adapter acquisition");
        match err {
            RenderError::PassInputOutOfRange {
                pass_index,
                source_pass_index,
                num_passes,
            } => {
                assert_eq!(pass_index, 0);
                assert_eq!(source_pass_index, 1);
                assert_eq!(num_passes, 2);
            }
            other => panic!("expected PassInputOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn self_reference_returns_pass_input_out_of_range() {
        // Pass 0 attempts to sample itself (index 0 >= current 0).
        let trivial = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() { fragColor = vec4(uv, 0.0, 1.0); }
"#
        .to_string();
        let spec = MultipassSpec {
            passes: vec![PassDef {
                label: Some("self-ref-pass-0".to_string()),
                fragment_glsl: trivial,
                uniforms: vec![],
                input_textures: vec![PassInputBinding {
                    source_pass_index: 0,
                    sampler_uniform_name: "self".to_string(),
                }],
            }],
            target_size: (16, 16),
        };
        let err = compile_and_render_multipass(&spec)
            .expect_err("self reference must reject before adapter acquisition");
        match err {
            RenderError::PassInputOutOfRange {
                pass_index,
                source_pass_index,
                num_passes,
            } => {
                assert_eq!(pass_index, 0);
                assert_eq!(source_pass_index, 0);
                assert_eq!(num_passes, 1);
            }
            other => panic!("expected PassInputOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn zero_target_size_rejects_before_wgpu() {
        let trivial = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() { fragColor = vec4(uv, 0.0, 1.0); }
"#
        .to_string();
        let spec = MultipassSpec {
            passes: vec![PassDef {
                label: None,
                fragment_glsl: trivial,
                uniforms: vec![],
                input_textures: vec![],
            }],
            target_size: (0, 16),
        };
        let err = compile_and_render_multipass(&spec).expect_err("zero target width must reject");
        assert!(
            matches!(err, RenderError::TargetSizeInvalid((0, 16))),
            "expected TargetSizeInvalid((0, 16)), got {err:?}",
        );
    }

    #[test]
    fn uniform_marshalling_scalar_layout() {
        // Drift pin: a `f32` uniform must land in the first 4 bytes
        // of its 16-byte slot, little-endian, with the trailing 12
        // bytes zero pad. Slice-3 widening of std140 packing must
        // either preserve this layout or revise this pin.
        let inputs = vec![UniformInput {
            name: "u_value".to_string(),
            value: UniformValue::Scalar(0.5),
        }];
        let slots =
            encode_uniform_slots(&inputs, "drift-pin").expect("scalar marshalling must succeed");
        assert_eq!(slots.len(), 1);
        let bytes = slots[0].bytes;
        assert_eq!(&bytes[0..4], &0.5_f32.to_le_bytes());
        assert!(
            bytes[4..16].iter().all(|b| *b == 0),
            "trailing pad must be zero"
        );
    }

    #[test]
    fn uniform_marshalling_vec3_layout() {
        // Drift pin: a `vec3` uniform packs the three floats into
        // the first 12 bytes (LE) with the trailing 4 bytes zero pad.
        let inputs = vec![UniformInput {
            name: "u_color".to_string(),
            value: UniformValue::Vec3([0.25, 0.5, 0.75]),
        }];
        let slots =
            encode_uniform_slots(&inputs, "drift-pin").expect("vec3 marshalling must succeed");
        assert_eq!(slots.len(), 1);
        let bytes = slots[0].bytes;
        assert_eq!(&bytes[0..4], &0.25_f32.to_le_bytes());
        assert_eq!(&bytes[4..8], &0.5_f32.to_le_bytes());
        assert_eq!(&bytes[8..12], &0.75_f32.to_le_bytes());
        assert!(
            bytes[12..16].iter().all(|b| *b == 0),
            "trailing pad must be zero"
        );
    }

    #[test]
    fn uniform_marshalling_int_uint_layout() {
        let inputs = vec![
            UniformInput {
                name: "u_signed".to_string(),
                value: UniformValue::Int(-7),
            },
            UniformInput {
                name: "u_unsigned".to_string(),
                value: UniformValue::UInt(42),
            },
        ];
        let slots =
            encode_uniform_slots(&inputs, "drift-pin").expect("int/uint marshalling must succeed");
        assert_eq!(slots.len(), 2);
        assert_eq!(&slots[0].bytes[0..4], &(-7_i32).to_le_bytes());
        assert!(slots[0].bytes[4..16].iter().all(|b| *b == 0));
        assert_eq!(&slots[1].bytes[0..4], &42_u32.to_le_bytes());
        assert!(slots[1].bytes[4..16].iter().all(|b| *b == 0));
    }

    #[test]
    fn empty_uniform_name_rejects() {
        // Drift pin: empty uniform name surfaces as
        // UniformBindingFailed rather than producing a silent
        // unnamed slot — keeps the marshalling surface explicit
        // for slice-3 audio uniforms.
        let inputs = vec![UniformInput {
            name: String::new(),
            value: UniformValue::Scalar(1.0),
        }];
        let err =
            encode_uniform_slots(&inputs, "drift-pin").expect_err("empty uniform name must reject");
        match err {
            RenderError::UniformBindingFailed {
                pass_label,
                name,
                reason,
            } => {
                assert_eq!(pass_label, "drift-pin");
                assert_eq!(name, "");
                assert!(reason.contains("must not be empty"), "reason: {reason}");
            }
            other => panic!("expected UniformBindingFailed, got {other:?}"),
        }
    }
}
