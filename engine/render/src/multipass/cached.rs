// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Persistent multi-pass pipeline ([`CachedMultipass`]).
//!
//! [`super::MultipassRenderer::render`] rebuilds every per-spec
//! resource (GLSL compile, pipelines, targets, bind groups) on each
//! call and restarts `feedback_self` ping-pong buffers from black, so a
//! live caller that renders one frame per call can never accumulate
//! feedback beyond `spec.frame_count` iterations.
//!
//! [`CachedMultipass`] compiles the spec ONCE and keeps the pipelines,
//! uniform buffers, render targets (including both ping-pong slots per
//! feedback pass), and both frame-parity bind-group sets alive across
//! calls. Each [`CachedMultipass::render_frames`] call uploads the
//! uniform block, records `n` consecutive frames continuing from the
//! persistent frame counter, and reads back the final pass.
//!
//! # Equivalence invariant
//!
//! With constant uniforms, `k` sequential `render_frames(.., 1)` calls
//! (or one `render_frames(.., k)` call) on a freshly compiled cache
//! produce bytes identical to the one-shot
//! [`super::MultipassRenderer::render`] with `frame_count = k`: the
//! pass order, ping-pong slot arithmetic, sampler, clear ops, and
//! readback are the same. Pinned by the
//! `cached_multipass_equivalence` tests in the parent module.
//!
//! # Lifecycle
//!
//! The cache is size-bound. A different `target_size` is a different
//! cache (the caller keys instances by size) so alternating consumers
//! at different resolutions do not wipe each other's feedback
//! history. [`CachedMultipass::reset`] restarts the frame counter and
//! clears every target back to black (the cold-start state).

use std::borrow::Cow;

use wgpu::util::DeviceExt;

use super::{
    MultipassRenderer, MultipassSpec, TargetFormat, UniformInput, UniformSlot,
    encode_uniform_slots, readback_texture, validate_spec_shape,
};
use crate::single_pass::{RenderError, compile_glsl_module};

/// Per-pass static GPU state.
struct CachedPass {
    label: String,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: Option<wgpu::Buffer>,
    uniform_slot_count: usize,
    /// Index of this pass's first texture in [`CachedMultipass::textures`].
    first_slot: usize,
    feedback: bool,
    /// Bind groups indexed by frame parity (`frame % 2`).
    bind_groups: [wgpu::BindGroup; 2],
}

/// Pre-compiled multi-pass pipeline with persistent render targets and
/// feedback ping-pong state. Built by
/// [`MultipassRenderer::compile_cached_multipass`].
pub struct CachedMultipass {
    passes: Vec<CachedPass>,
    textures: Vec<wgpu::Texture>,
    views: Vec<wgpu::TextureView>,
    target_size: (u32, u32),
    has_feedback: bool,
    /// Number of frames recorded since compile / last [`Self::reset`].
    frame: u64,
}

impl CachedMultipass {
    /// Target size this cache was compiled for.
    #[must_use]
    pub fn target_size(&self) -> (u32, u32) {
        self.target_size
    }

    /// `true` when any pass samples its own previous-frame output.
    #[must_use]
    pub fn has_feedback(&self) -> bool {
        self.has_feedback
    }

    /// Frames recorded since compile or the last [`Self::reset`].
    #[must_use]
    pub fn frames_recorded(&self) -> u64 {
        self.frame
    }

    /// Restart from the cold-start state: frame counter 0 and every
    /// target (both ping-pong slots included) cleared to black.
    pub fn reset(&mut self, renderer: &MultipassRenderer) {
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("chromaticity-render cached multipass reset"),
            });
        for view in &self.views {
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("chromaticity-render cached multipass clear"),
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
        }
        renderer.queue.submit(Some(encoder.finish()));
        self.frame = 0;
    }

    /// Upload `uniforms` to every pass that declared a uniform block at
    /// compile time, record `frames` consecutive frames continuing from
    /// the persistent frame counter, and return the final pass's
    /// readback bytes from the last recorded frame.
    ///
    /// The same uniform list is shared by every pass (the shell's
    /// std140 ABI); passes compiled without uniforms ignore it.
    ///
    /// # Errors
    ///
    /// - [`RenderError::EmptyPassList`] — `frames == 0`.
    /// - [`RenderError::UniformBindingFailed`] — the encoded slot count
    ///   differs from the compile-time layout.
    /// - [`RenderError::DeviceLoss`] / [`RenderError::BufferMap`] —
    ///   readback failure.
    pub fn render_frames(
        &mut self,
        renderer: &MultipassRenderer,
        uniforms: &[UniformInput],
        frames: u32,
    ) -> Result<Vec<u8>, RenderError> {
        if frames == 0 {
            return Err(RenderError::EmptyPassList);
        }
        for pass in &self.passes {
            let Some(buffer) = pass.uniform_buffer.as_ref() else {
                continue;
            };
            let slots = encode_uniform_slots(uniforms, &pass.label)?;
            if slots.len() != pass.uniform_slot_count {
                return Err(RenderError::UniformBindingFailed {
                    pass_label: pass.label.clone(),
                    name: String::from("<block>"),
                    reason: format!(
                        "uniform slot count changed from {} to {}",
                        pass.uniform_slot_count,
                        slots.len()
                    ),
                });
            }
            renderer
                .queue
                .write_buffer(buffer, 0, bytemuck::cast_slice(&slots));
        }

        let mut last_parity = 0usize;
        for _ in 0..frames {
            let parity = (self.frame % 2) as usize;
            let mut encoder =
                renderer
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("chromaticity-render cached multipass encoder"),
                    });
            for pass in &self.passes {
                let view = &self.views[write_slot(pass, parity)];
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("chromaticity-render cached multipass pass"),
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
                rpass.set_pipeline(&pass.pipeline);
                rpass.set_bind_group(0, &pass.bind_groups[parity], &[]);
                rpass.draw(0..3, 0..1);
            }
            renderer.queue.submit(Some(encoder.finish()));
            last_parity = parity;
            self.frame = self.frame.wrapping_add(1);
        }

        let Some(final_pass) = self.passes.last() else {
            return Err(RenderError::EmptyPassList);
        };
        let final_texture = &self.textures[write_slot(final_pass, last_parity)];
        readback_texture(&renderer.device, &renderer.queue, final_texture)
    }
}

/// Texture slot a pass writes on a frame of the given parity
/// (mirrors `write_slot` in the one-shot scheduler).
fn write_slot(pass: &CachedPass, parity: usize) -> usize {
    if pass.feedback {
        pass.first_slot + parity
    } else {
        pass.first_slot
    }
}

/// Texture slot a feedback pass reads on a frame of the given parity
/// (mirrors `feedback_read_slot` in the one-shot scheduler).
fn feedback_read_slot(first_slot: usize, parity: usize) -> usize {
    first_slot + ((parity + 1) % 2)
}

impl MultipassRenderer {
    /// Compile `spec` once into a [`CachedMultipass`] whose pipelines,
    /// targets, and feedback ping-pong buffers persist across frames.
    ///
    /// `spec.frame_count` is ignored (frames are driven per call by
    /// [`CachedMultipass::render_frames`]); `spec.passes[*].uniforms`
    /// define each pass's uniform-block layout (empty = no block).
    ///
    /// # Errors
    ///
    /// Same structural / compile variants as
    /// [`MultipassRenderer::render`]: `EmptyPassList`,
    /// `TargetSizeInvalid`, `PassInputOutOfRange`,
    /// `FeedbackAliasError`, `FloatTargetUnsupported`,
    /// `UniformBindingFailed`, `PassRenderFailed`.
    pub fn compile_cached_multipass(
        &self,
        spec: &MultipassSpec,
    ) -> Result<CachedMultipass, RenderError> {
        // `frame_count` is irrelevant here (frames are driven per call),
        // so validate only the pass graph + size; a caller passing
        // `frame_count = 0` must not be rejected for it.
        if spec.frame_count == 0 {
            let mut shape_only = spec.clone();
            shape_only.frame_count = 1;
            validate_spec_shape(&shape_only)?;
        } else {
            validate_spec_shape(spec)?;
        }

        for (pass_index, pass) in spec.passes.iter().enumerate() {
            if matches!(
                pass.target_format,
                TargetFormat::Rgba16Float | TargetFormat::Rgba32Float
            ) {
                let format = pass.target_format.to_wgpu();
                let features = self.adapter.get_texture_format_features(format);
                if !features
                    .allowed_usages
                    .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
                {
                    return Err(RenderError::FloatTargetUnsupported {
                        format: pass.target_format.label().to_string(),
                        reason: format!(
                            "adapter does not advertise RENDER_ATTACHMENT for {format:?} \
                             (pass {pass_index})"
                        ),
                    });
                }
            }
        }

        let device = &self.device;
        let num_passes = spec.passes.len();

        // Targets: one per pass, two for feedback passes. Every target
        // gets COPY_SRC (not only the final one) so `reset` and future
        // GPU-side consumers can treat them uniformly.
        let mut textures: Vec<wgpu::Texture> = Vec::with_capacity(num_passes + 2);
        let mut views: Vec<wgpu::TextureView> = Vec::with_capacity(num_passes + 2);
        let mut first_slots: Vec<usize> = Vec::with_capacity(num_passes);
        for (pass_index, pass) in spec.passes.iter().enumerate() {
            first_slots.push(textures.len());
            let slot_count = if pass.feedback_self { 2 } else { 1 };
            for slot in 0..slot_count {
                let label = format!(
                    "chromaticity-render cached multipass target (pass {pass_index}, slot {slot})"
                );
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(&label),
                    size: wgpu::Extent3d {
                        width: spec.target_size.0,
                        height: spec.target_size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: pass.target_format.to_wgpu(),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                views.push(texture.create_view(&wgpu::TextureViewDescriptor::default()));
                textures.push(texture);
            }
        }

        // Same nearest / clamp sampler as the one-shot scheduler
        // (required for byte equivalence).
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("chromaticity-render cached multipass sampler (nearest/clamp)"),
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

        let mut passes: Vec<CachedPass> = Vec::with_capacity(num_passes);
        for (pass_index, pass) in spec.passes.iter().enumerate() {
            let pass_label = pass.label.as_deref().unwrap_or("(unlabelled)").to_string();

            let mut bgl_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();
            let (uniform_buffer, uniform_slot_count) = if pass.uniforms.is_empty() {
                (None, 0)
            } else {
                let slots = encode_uniform_slots(&pass.uniforms, &pass_label)?;
                let count = slots.len();
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("chromaticity-render cached multipass uniform buffer"),
                    contents: bytemuck::cast_slice::<UniformSlot, u8>(&slots),
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
                (Some(buffer), count)
            };

            let total_inputs = pass.input_textures.len() + usize::from(pass.feedback_self);
            for i in 0..total_inputs {
                let tex_binding = 1 + 2 * (i as u32);
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
                    binding: tex_binding + 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                });
            }
            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("chromaticity-render cached multipass bind group layout"),
                entries: &bgl_entries,
            });

            let fragment_module =
                compile_glsl_module(&pass.fragment_glsl, naga::ShaderStage::Fragment).map_err(
                    |e| RenderError::PassRenderFailed {
                        pass_index,
                        source: Box::new(e),
                    },
                )?;
            let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("chromaticity-render cached multipass fragment (user GLSL)"),
                source: wgpu::ShaderSource::Naga(Cow::Owned(fragment_module)),
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("chromaticity-render cached multipass pipeline layout"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("chromaticity-render cached multipass pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.vertex_shader,
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
                        format: pass.target_format.to_wgpu(),
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

            // Both parity bind-group sets, built once.
            let build_bind_group = |parity: usize| -> wgpu::BindGroup {
                let mut entries: Vec<wgpu::BindGroupEntry> = Vec::new();
                if let Some(buffer) = uniform_buffer.as_ref() {
                    entries.push(wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    });
                }
                // Explicit prior-pass inputs read that pass's
                // CURRENT-frame write slot; the self-feedback input
                // (if any) follows them and reads the other slot.
                let mut input_slots: Vec<usize> = pass
                    .input_textures
                    .iter()
                    .map(|binding| {
                        let src = binding.source_pass_index;
                        if spec.passes[src].feedback_self {
                            first_slots[src] + parity
                        } else {
                            first_slots[src]
                        }
                    })
                    .collect();
                if pass.feedback_self {
                    input_slots.push(feedback_read_slot(first_slots[pass_index], parity));
                }
                for (i, slot) in input_slots.iter().enumerate() {
                    let tex_binding = 1 + 2 * (i as u32);
                    entries.push(wgpu::BindGroupEntry {
                        binding: tex_binding,
                        resource: wgpu::BindingResource::TextureView(&views[*slot]),
                    });
                    entries.push(wgpu::BindGroupEntry {
                        binding: tex_binding + 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    });
                }
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("chromaticity-render cached multipass bind group"),
                    layout: &bgl,
                    entries: &entries,
                })
            };
            let bind_groups = [build_bind_group(0), build_bind_group(1)];

            passes.push(CachedPass {
                label: pass_label,
                pipeline,
                uniform_buffer,
                uniform_slot_count,
                first_slot: first_slots[pass_index],
                feedback: pass.feedback_self,
                bind_groups,
            });
        }

        Ok(CachedMultipass {
            has_feedback: spec.passes.iter().any(|p| p.feedback_self),
            passes,
            textures,
            views,
            target_size: spec.target_size,
            frame: 0,
        })
    }
}
