// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Swapchain blit pass for the ADR-011 Phase-2 Slice R-1a output window.
//!
//! [`BlitPass`] is a two-triangle (fullscreen-triangle) `wgpu`
//! [`wgpu::RenderPipeline`] that samples an offscreen render target
//! (the texture returned by
//! [`chromaticity_render::MultipassRenderer::render_frame_live`]) and
//! writes it onto the presentation surface's swapchain texture. It is
//! the on-screen counterpart to the engine's offscreen-only conformance
//! path: R-0 proved the multipass scheduler renders into a texture; the
//! blit proves that texture reaches a window.
//!
//! ## Why a dedicated pass and not a texture copy
//!
//! The offscreen target is `Rgba8Unorm` (the engine's
//! [`chromaticity_render::TargetFormat::Rgba8Unorm`] default), while the
//! swapchain's preferred format is typically an sRGB format
//! (`Bgra8UnormSrgb` on macOS Metal). A `copy_texture_to_texture` would
//! require identical formats; a sampling pass lets the swapchain apply
//! its own sRGB-encode on write and decouples the two formats. The pass
//! samples with a nearest / clamp-to-edge sampler (same posture as the
//! engine's multipass shared sampler).
//!
//! ## Shaders
//!
//! Both shaders are WGSL runtime constants compiled once at
//! construction. The vertex shader synthesises a fullscreen triangle
//! from `vertex_index` (no vertex buffer); the fragment shader samples
//! the bound source texture at the interpolated UV. WGSL (not GLSL) is
//! used here because the blit is shell-local plumbing with no naga GLSL
//! frontend dependency — wgpu's built-in WGSL frontend compiles it
//! directly.
//!
//! ## No `unsafe`
//!
//! Everything here is safe `wgpu` 26 API. The workspace
//! `unsafe_code = "forbid"` lint is inherited; this module adds none.

use std::borrow::Cow;

/// Fullscreen-triangle + texture-sample WGSL for the swapchain blit.
///
/// The vertex stage emits a single oversized triangle covering the
/// viewport (the standard 3-vertex fullscreen trick) and forwards a
/// `[0, 1]` UV. The fragment stage samples the source texture at that
/// UV. The UV's V axis is flipped (`1.0 - uv.y`) so the offscreen
/// texture's top-left origin maps to the swapchain's top-left on
/// present (wgpu clip-space Y points up; framebuffer rows count down).
const BLIT_WGSL: &str = r#"
struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vidx: u32) -> VsOut {
    // Fullscreen triangle: clip-space corners (-1,-1), (3,-1), (-1,3).
    var out: VsOut;
    let x = f32(i32((vidx << 1u) & 2u));
    let y = f32(i32(vidx & 2u));
    out.clip_pos = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    // Map to UV in [0, 1]; flip V so the offscreen top row presents on top.
    out.uv = vec2<f32>(x, 1.0 - y);
    return out;
}

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(src_tex, src_sampler, in.uv);
}
"#;

/// A reusable swapchain blit pipeline.
///
/// Owns the WGSL shader module, bind-group layout, render pipeline, and
/// the shared nearest / clamp-to-edge sampler. The per-frame source
/// texture view + swapchain view are supplied to [`BlitPass::blit`];
/// the bind group is rebuilt each frame because the source texture view
/// changes every frame (a fresh
/// [`chromaticity_render::MultipassRenderer::render_frame_live`]
/// output).
pub struct BlitPass {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl BlitPass {
    /// Build the blit pipeline targeting `swapchain_format`.
    ///
    /// `swapchain_format` MUST equal the configured surface format
    /// (see [`preferred_surface_format`]) — the fragment color target
    /// format must match the swapchain texture the pass writes to, or
    /// `wgpu` rejects the render pass at submit time.
    #[must_use]
    pub fn new(device: &wgpu::Device, swapchain_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chromaticity-output blit shader (wgsl)"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(BLIT_WGSL)),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("chromaticity-output blit bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        // The offscreen target is Rgba8Unorm; sample it
                        // as a non-filterable float texture to match the
                        // NonFiltering sampler binding below (the engine's
                        // multipass path uses the same posture).
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chromaticity-output blit pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("chromaticity-output blit pipeline"),
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
                    format: swapchain_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("chromaticity-output blit sampler (nearest/clamp)"),
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

        Self {
            pipeline,
            bind_group_layout,
            sampler,
        }
    }

    /// Encode + submit a blit of `source` onto `swapchain_view`.
    ///
    /// `source` is the offscreen texture view to sample (the
    /// `render_frame_live` output's default view); `swapchain_view` is
    /// the current frame's swapchain texture view. The bind group is
    /// rebuilt each call because `source` changes every frame.
    pub fn blit(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &wgpu::TextureView,
        swapchain_view: &wgpu::TextureView,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("chromaticity-output blit bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("chromaticity-output blit encoder"),
        });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("chromaticity-output blit pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: swapchain_view,
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
            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }
        queue.submit(std::iter::once(encoder.finish()));
    }
}

/// Choose the swapchain texture format for the surface configuration.
///
/// Prefers the surface's first advertised format (which
/// [`wgpu::Surface::get_default_config`] also uses); on Metal /
/// DXGI this is typically an sRGB format so the swapchain applies the
/// linear→sRGB encode on present. The returned format MUST be the one
/// passed to both [`wgpu::Surface::configure`] (via the
/// [`wgpu::SurfaceConfiguration::format`]) and [`BlitPass::new`] so the
/// blit's color target matches the swapchain it writes to.
///
/// Falls back to `Bgra8UnormSrgb` (the universal Metal / DXGI
/// presentation format) if the surface advertises no formats — a
/// defensive path that should never fire on a real adapter.
#[must_use]
pub fn preferred_surface_format(caps: &wgpu::SurfaceCapabilities) -> wgpu::TextureFormat {
    caps.formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
        .or_else(|| caps.formats.first().copied())
        .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Slice R-1a unit pins for the swapchain format selection.
    //!
    //! The pipeline construction + blit submit require a real
    //! `wgpu::Device` + `wgpu::Surface`, which the headless shell test
    //! runner does not have (and which the orchestrator verifies by
    //! running the app). What IS testable without a GPU is the
    //! format-selection policy, pinned here.

    use super::preferred_surface_format;

    fn caps_with(formats: Vec<wgpu::TextureFormat>) -> wgpu::SurfaceCapabilities {
        wgpu::SurfaceCapabilities {
            formats,
            present_modes: vec![],
            alpha_modes: vec![],
            usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
        }
    }

    #[test]
    fn prefers_first_srgb_format_when_present() {
        let caps = caps_with(vec![
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ]);
        assert_eq!(
            preferred_surface_format(&caps),
            wgpu::TextureFormat::Bgra8UnormSrgb,
            "an sRGB format must win over a non-sRGB one so the swapchain encodes on present",
        );
    }

    #[test]
    fn falls_back_to_first_format_when_no_srgb() {
        let caps = caps_with(vec![
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8Unorm,
        ]);
        assert_eq!(
            preferred_surface_format(&caps),
            wgpu::TextureFormat::Rgba8Unorm,
            "with no sRGB format the first advertised format is used",
        );
    }

    #[test]
    fn falls_back_to_bgra_srgb_when_empty() {
        let caps = caps_with(vec![]);
        assert_eq!(
            preferred_surface_format(&caps),
            wgpu::TextureFormat::Bgra8UnormSrgb,
            "an empty format list (never real) defaults to the universal presentation format",
        );
    }
}
