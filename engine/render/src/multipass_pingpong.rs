// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#![forbid(unsafe_code)]

//! Phase 9 Metal / wgpu multi-pass ping-pong feedback framebuffer pipeline.
//!
//! Provides a zero-CPU-round-trip double-buffered GPU feedback loop where frame
//! `N` samples frame `N - 1`'s GPU texture directly through a cached
//! `wgpu::RenderPipeline`, then swaps the active read/write texture indices in
//! place. Supports optical trails, zoom feedback, reaction-diffusion, and fluid
//! advection shaders via [`DEFAULT_FEEDBACK_WGSL`] or custom WGSL modules.

use std::borrow::Cow;
use std::sync::mpsc::{TryRecvError, channel};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// Uniform parameters consumed by the ping-pong feedback WGSL shader at
/// `@group(0) @binding(2)`.
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct FeedbackUniforms {
    /// Elapsed time in seconds.
    pub time: f32,
    /// Feedback trail retention factor in `[0.0, 1.0]`.
    pub decay: f32,
    /// Injected pulse red channel in `[0.0, 1.0]`.
    pub inject_r: f32,
    /// Injected pulse green channel in `[0.0, 1.0]`.
    pub inject_g: f32,
    /// Injected pulse blue channel in `[0.0, 1.0]`.
    pub inject_b: f32,
    /// Injection mix strength in `[0.0, 1.0]`.
    pub inject_strength: f32,
    /// Radial UV zoom factor around `(0.5, 0.5)` (`1.0` = identity).
    pub zoom: f32,
    /// Explicit 4-byte padding to keep the uniform struct 16-byte aligned (32 bytes total).
    pub _pad: f32,
}

impl Default for FeedbackUniforms {
    fn default() -> Self {
        Self {
            time: 0.0,
            decay: 0.9,
            inject_r: 0.0,
            inject_g: 0.0,
            inject_b: 0.0,
            inject_strength: 0.0,
            zoom: 1.0,
            _pad: 0.0,
        }
    }
}

/// Default WGSL ping-pong feedback shader with fullscreen-triangle vertex entry
/// (`vs_main`) and decaying zoom + color-injection fragment entry (`fs_main`).
pub const DEFAULT_FEEDBACK_WGSL: &str = r#"
struct FeedbackUniforms {
    time: f32,
    decay: f32,
    inject_r: f32,
    inject_g: f32,
    inject_b: f32,
    inject_strength: f32,
    zoom: f32,
    _pad: f32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: VertexOut;
    out.position = vec4<f32>(positions[vid], 0.0, 1.0);
    out.uv = uvs[vid];
    return out;
}

@group(0) @binding(0) var prev_tex: texture_2d<f32>;
@group(0) @binding(1) var prev_sampler: sampler;
@group(0) @binding(2) var<uniform> params: FeedbackUniforms;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let safe_zoom = select(params.zoom, 1.0, params.zoom <= 0.0001);
    let zoomed_uv = (in.uv - vec2<f32>(0.5, 0.5)) / safe_zoom + vec2<f32>(0.5, 0.5);
    let prev = textureSample(prev_tex, prev_sampler, zoomed_uv);
    let decay = clamp(params.decay, 0.0, 1.0);
    let inject = vec3<f32>(params.inject_r, params.inject_g, params.inject_b)
        * clamp(params.inject_strength, 0.0, 1.0);
    let out_rgb = clamp(prev.rgb * decay + inject, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(out_rgb, 1.0);
}
"#;

/// Double-buffered GPU ping-pong feedback pipeline.
///
/// Each [`PingPongFeedbackPipeline::step`] call renders into `textures[write_idx]`
/// while sampling `textures[read_idx]` on the GPU, then swaps `read_idx` and
/// `write_idx` with zero CPU readback.
#[derive(Debug)]
pub struct PingPongFeedbackPipeline {
    width: u32,
    height: u32,
    textures: [wgpu::Texture; 2],
    views: [wgpu::TextureView; 2],
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    bind_groups: [wgpu::BindGroup; 2],
    pipeline: wgpu::RenderPipeline,
    read_idx: usize,
}

impl PingPongFeedbackPipeline {
    /// Allocate a double-buffered `Rgba8Unorm` ping-pong feedback pipeline using
    /// [`DEFAULT_FEEDBACK_WGSL`] and clear both buffers to black.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        Self::with_wgsl(device, queue, width, height, DEFAULT_FEEDBACK_WGSL)
    }

    /// Allocate a double-buffered `Rgba8Unorm` ping-pong feedback pipeline with
    /// a custom WGSL shader module exposing `vs_main` and `fs_main` entry points
    /// bound to:
    /// - `@group(0) @binding(0) var prev_tex: texture_2d<f32>;`
    /// - `@group(0) @binding(1) var prev_sampler: sampler;`
    /// - `@group(0) @binding(2) var<uniform> params: FeedbackUniforms;`
    pub fn with_wgsl(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        wgsl_source: &str,
    ) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err(format!(
                "ping-pong feedback dimensions must be non-zero, got {width}x{height}"
            ));
        }

        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture_usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST;

        let make_texture = |label: &'static str| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: texture_usage,
                view_formats: &[],
            })
        };

        let textures = [
            make_texture("chromaticity-render ping-pong texture 0"),
            make_texture("chromaticity-render ping-pong texture 1"),
        ];
        let views = [
            textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
            textures[1].create_view(&wgpu::TextureViewDescriptor::default()),
        ];

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("chromaticity-render ping-pong linear clamp sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let initial_uniforms = FeedbackUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chromaticity-render ping-pong feedback uniforms"),
            contents: bytemuck::bytes_of(&initial_uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("chromaticity-render ping-pong bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let make_bind_group = |label: &'static str, view: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                ],
            })
        };

        let bind_groups = [
            make_bind_group("chromaticity-render ping-pong bg 0", &views[0]),
            make_bind_group("chromaticity-render ping-pong bg 1", &views[1]),
        ];

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chromaticity-render ping-pong feedback wgsl"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(wgsl_source)),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chromaticity-render ping-pong pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("chromaticity-render ping-pong pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
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
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        let instance = Self {
            width,
            height,
            textures,
            views,
            sampler,
            uniform_buffer,
            bind_groups,
            pipeline,
            read_idx: 0,
        };
        instance.reset(queue);
        Ok(instance)
    }

    /// Texture width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Texture height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Borrow the active output texture holding the most recently rendered frame.
    #[must_use]
    pub fn current_texture(&self) -> &wgpu::Texture {
        &self.textures[self.read_idx]
    }

    /// Borrow the active output texture view holding the most recently rendered frame.
    #[must_use]
    pub fn current_view(&self) -> &wgpu::TextureView {
        &self.views[self.read_idx]
    }

    /// Borrow the linear clamping sampler used by the feedback pass.
    #[must_use]
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Advance the feedback simulation by one GPU pass: renders into
    /// `textures[write_idx]` while sampling `textures[read_idx]`, then swaps
    /// `read_idx <-> write_idx` with zero CPU readback.
    pub fn step(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, uniforms: FeedbackUniforms) {
        let write_idx = 1 - self.read_idx;

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("chromaticity-render ping-pong step encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("chromaticity-render ping-pong step pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.views[write_idx],
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
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_groups[self.read_idx], &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        self.read_idx = write_idx;
    }

    /// Clear both ping-pong textures cleanly to black (`0, 0, 0, 0`).
    pub fn reset(&self, queue: &wgpu::Queue) {
        let extent = wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        };
        let byte_count = (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(4);
        let zeros = vec![0_u8; byte_count];
        let layout = wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(self.width * 4),
            rows_per_image: Some(self.height),
        };

        for texture in &self.textures {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &zeros,
                layout,
                extent,
            );
        }
    }

    /// Read back the current active ping-pong texture (`textures[read_idx]`)
    /// into tightly packed `Rgba8Unorm` bytes (`width * height * 4`).
    pub fn read_current_rgba(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<u8>, String> {
        let unpadded_bytes_per_row = self.width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let buffer_size = u64::from(padded_bytes_per_row) * u64::from(self.height);

        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chromaticity-render ping-pong readback staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("chromaticity-render ping-pong readback encoder"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.textures[self.read_idx],
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });

        loop {
            device
                .poll(wgpu::PollType::Wait)
                .map_err(|err| format!("ping-pong readback device poll failed: {err:?}"))?;
            match rx.try_recv() {
                Ok(Ok(())) => break,
                Ok(Err(err)) => return Err(format!("ping-pong readback buffer map failed: {err}")),
                Err(TryRecvError::Empty) => continue,
                Err(TryRecvError::Disconnected) => {
                    return Err(
                        "ping-pong readback map_async channel disconnected before completion"
                            .to_string(),
                    );
                }
            }
        }

        let mapped = slice.get_mapped_range();
        let unpadded = unpadded_bytes_per_row as usize;
        let padded = padded_bytes_per_row as usize;
        let mut out = Vec::with_capacity(unpadded * (self.height as usize));
        for row in 0..(self.height as usize) {
            let start = row * padded;
            let end = start + unpadded;
            out.extend_from_slice(&mapped[start..end]);
        }
        drop(mapped);
        staging.unmap();
        Ok(out)
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — Phase 9 Metal / wgpu multi-pass
    //! ping-pong feedback framebuffer decay retention and clean reset verification.

    use super::*;
    use crate::multipass::MultipassRenderer;

    fn mean_luma(rgba: &[u8]) -> f32 {
        assert!(!rgba.is_empty());
        assert_eq!(rgba.len() % 4, 0);
        let pixel_count = (rgba.len() / 4) as f32;
        let sum: f32 = rgba
            .chunks_exact(4)
            .map(|px| {
                0.2126 * f32::from(px[0]) + 0.7152 * f32::from(px[1]) + 0.0722 * f32::from(px[2])
            })
            .sum();
        sum / pixel_count
    }

    #[test]
    fn ping_pong_retains_monotonically_decaying_trail_across_five_frames() {
        let renderer = MultipassRenderer::new().expect("wgpu Metal/Vulkan adapter required");
        let mut fb = PingPongFeedbackPipeline::new(renderer.device(), renderer.queue(), 32, 32)
            .expect("ping-pong pipeline allocation should succeed");

        // Frame 0: inject a bright white pulse.
        fb.step(
            renderer.device(),
            renderer.queue(),
            FeedbackUniforms {
                time: 0.0,
                decay: 0.8,
                inject_r: 1.0,
                inject_g: 1.0,
                inject_b: 1.0,
                inject_strength: 1.0,
                zoom: 1.0,
                _pad: 0.0,
            },
        );
        let frame0 = fb
            .read_current_rgba(renderer.device(), renderer.queue())
            .expect("frame 0 readback should succeed");
        let luma0 = mean_luma(&frame0);
        assert!(
            luma0 > 250.0,
            "frame 0 injected pulse should be near full white, got luma={luma0}"
        );

        // Frames 1..=5: zero injection, decay = 0.8 -> strictly monotonically decaying
        // non-zero optical trail energy directly from the previous GPU texture.
        let mut prev_luma = luma0;
        let mut expected_luma = 255.0_f32;
        for frame_idx in 1..=5 {
            fb.step(
                renderer.device(),
                renderer.queue(),
                FeedbackUniforms {
                    time: frame_idx as f32 * (1.0 / 60.0),
                    decay: 0.8,
                    inject_r: 0.0,
                    inject_g: 0.0,
                    inject_b: 0.0,
                    inject_strength: 0.0,
                    zoom: 1.0,
                    _pad: 0.0,
                },
            );
            let bytes = fb
                .read_current_rgba(renderer.device(), renderer.queue())
                .expect("decay frame readback should succeed");
            let luma = mean_luma(&bytes);
            expected_luma *= 0.8;

            assert!(
                luma > 0.0,
                "frame {frame_idx} must retain non-zero feedback energy, got {luma}"
            );
            assert!(
                luma < prev_luma,
                "frame {frame_idx} luma ({luma}) must strictly decay below previous ({prev_luma})"
            );
            assert!(
                (luma - expected_luma).abs() <= 4.0,
                "frame {frame_idx} luma ({luma}) diverged from 0.8^n decay ({expected_luma})"
            );
            prev_luma = luma;
        }
    }

    #[test]
    fn reset_clears_both_ping_pong_buffers_to_zero() {
        let renderer = MultipassRenderer::new().expect("wgpu Metal/Vulkan adapter required");
        let mut fb = PingPongFeedbackPipeline::new(renderer.device(), renderer.queue(), 16, 16)
            .expect("ping-pong pipeline allocation should succeed");

        // Pump energy into both ping-pong slots.
        let pulse = FeedbackUniforms {
            time: 0.0,
            decay: 0.9,
            inject_r: 1.0,
            inject_g: 0.5,
            inject_b: 0.25,
            inject_strength: 1.0,
            zoom: 1.0,
            _pad: 0.0,
        };
        fb.step(renderer.device(), renderer.queue(), pulse);
        fb.step(renderer.device(), renderer.queue(), pulse);
        let lit = fb
            .read_current_rgba(renderer.device(), renderer.queue())
            .expect("lit frame readback should succeed");
        assert!(mean_luma(&lit) > 100.0);

        // Reset both buffers and verify zero energy immediately and after a zero-inject step.
        fb.reset(renderer.queue());
        let cleared = fb
            .read_current_rgba(renderer.device(), renderer.queue())
            .expect("post-reset readback should succeed");
        assert!(
            cleared.iter().all(|&b| b == 0),
            "reset() must zero the active ping-pong texture"
        );

        fb.step(
            renderer.device(),
            renderer.queue(),
            FeedbackUniforms {
                time: 0.1,
                decay: 0.95,
                inject_r: 0.0,
                inject_g: 0.0,
                inject_b: 0.0,
                inject_strength: 0.0,
                zoom: 1.0,
                _pad: 0.0,
            },
        );
        let after_step = fb
            .read_current_rgba(renderer.device(), renderer.queue())
            .expect("post-reset step readback should succeed");
        assert_eq!(
            mean_luma(&after_step),
            0.0,
            "stepping after reset() with inject_strength=0 must remain black"
        );
    }

    #[test]
    fn rejects_zero_dimensions() {
        let renderer = MultipassRenderer::new().expect("wgpu Metal/Vulkan adapter required");
        assert!(PingPongFeedbackPipeline::new(renderer.device(), renderer.queue(), 0, 16).is_err());
        assert!(PingPongFeedbackPipeline::new(renderer.device(), renderer.queue(), 16, 0).is_err());
    }
}
