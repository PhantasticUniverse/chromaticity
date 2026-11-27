// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-7 GPU-accelerated multi-layer compositor (`wgpu 26`).
//!
//! Composites an ordered [`LayerStack`] of RGBA8 layers onto an offscreen
//! `Rgba8Unorm` target using a compiled GLSL 4.50 fragment shader over
//! ping-pong textures on a caller-supplied [`MultipassRenderer`].
//!
//! Supported per-layer capabilities:
//!
//! - **2D transforms** ([`LayerTransform`]): normalized UV translation
//!   (`offset_xy`), non-uniform scale (`scale_xy`), rotation around layer
//!   center (`rotation_rad`), and master opacity (`opacity`).
//! - **Blend modes** ([`BlendMode`]): `Normal` (Porter-Duff source-over),
//!   `Additive`, `Screen`, and `Multiply`.
//! - **Keying modes** ([`BlendMode`]): `LumaKey` (Rec. 709 luminance
//!   threshold + softness) and `ChromaKey` (RGB Euclidean color distance
//!   similarity + smoothness for green/blue-screen removal).

use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::multipass::{MultipassRenderer, readback_texture_to_bytes};
use crate::single_pass::{FULLSCREEN_TRIANGLE_GLSL, RenderError, compile_glsl_module};

/// Per-layer compositing blend or keying mode.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum BlendMode {
    /// Standard Porter-Duff source-over alpha compositing.
    #[default]
    Normal,
    /// Linear additive blend clamped to `[0.0, 1.0]`.
    Additive,
    /// Screen blend (`1 - (1 - base) * (1 - top)`).
    Screen,
    /// Multiply blend (`base * top`).
    Multiply,
    /// Rec. 709 luminance key: pixels at or below `threshold` become
    /// transparent, ramping smoothly to opaque over `softness`.
    LumaKey {
        /// Luminance cutoff in `[0.0, 1.0]`.
        threshold: f32,
        /// Transition width above `threshold` for anti-aliased edges.
        softness: f32,
    },
    /// RGB chroma key (green/blue screen): pixels within Euclidean RGB
    /// distance `similarity` of `key_rgb` become transparent, ramping
    /// smoothly to opaque over `smoothness`.
    ChromaKey {
        /// Normalized RGB key color in `[0.0, 1.0]` (e.g. `[0.0, 1.0, 0.0]`).
        key_rgb: [f32; 3],
        /// Distance cutoff within which pixels are fully keyed out.
        similarity: f32,
        /// Feathering width beyond `similarity` for smooth edge falloff.
        smoothness: f32,
    },
}

/// 2D affine transform and master opacity applied to a [`CompositeLayer`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerTransform {
    /// Normalized UV translation `[dx, dy]` relative to the canvas center.
    pub offset_xy: [f32; 2],
    /// Non-uniform scale factors `[sx, sy]` around the layer center.
    pub scale_xy: [f32; 2],
    /// Counter-clockwise rotation angle in radians around the layer center.
    pub rotation_rad: f32,
    /// Master layer opacity multiplier clamped to `[0.0, 1.0]`.
    pub opacity: f32,
}

impl Default for LayerTransform {
    fn default() -> Self {
        Self {
            offset_xy: [0.0, 0.0],
            scale_xy: [1.0, 1.0],
            rotation_rad: 0.0,
            opacity: 1.0,
        }
    }
}

/// A single RGBA8 image layer within a [`LayerStack`].
#[derive(Debug, Clone, PartialEq)]
pub struct CompositeLayer {
    /// Stable identifier used for lookup and mutation on [`LayerStack`].
    pub id: String,
    /// Whether this layer participates in compositing.
    pub visible: bool,
    /// Stacking order (ascending: lower `z_index` renders underneath higher).
    pub z_index: i32,
    /// Blend or keying mode applied when compositing over accumulated lower layers.
    pub blend: BlendMode,
    /// 2D transform and opacity for this layer.
    pub transform: LayerTransform,
    /// Layer source texture width in pixels (must be > 0).
    pub width: u32,
    /// Layer source texture height in pixels (must be > 0).
    pub height: u32,
    /// Tightly packed row-major RGBA8 pixel bytes (`width * height * 4`).
    pub rgba_pixels: Vec<u8>,
}

/// Ordered collection of [`CompositeLayer`] entries.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayerStack {
    layers: Vec<CompositeLayer>,
}

impl LayerStack {
    /// Create an empty layer stack.
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Append a layer to the stack.
    pub fn push_layer(&mut self, layer: CompositeLayer) {
        self.layers.push(layer);
    }

    /// Remove the first layer matching `id`, returning it if found.
    pub fn remove_layer(&mut self, id: &str) -> Option<CompositeLayer> {
        let idx = self.layers.iter().position(|layer| layer.id == id)?;
        Some(self.layers.remove(idx))
    }

    /// Update the visibility flag for the layer matching `id`.
    /// Returns `true` if a matching layer was found.
    pub fn set_visible(&mut self, id: &str, visible: bool) -> bool {
        if let Some(layer) = self.layers.iter_mut().find(|layer| layer.id == id) {
            layer.visible = visible;
            true
        } else {
            false
        }
    }

    /// Update the 2D transform and opacity for the layer matching `id`.
    /// Returns `true` if a matching layer was found.
    pub fn set_transform(&mut self, id: &str, transform: LayerTransform) -> bool {
        if let Some(layer) = self.layers.iter_mut().find(|layer| layer.id == id) {
            layer.transform = transform;
            true
        } else {
            false
        }
    }

    /// Update the [`BlendMode`] for the layer matching `id`.
    /// Returns `true` if a matching layer was found.
    pub fn set_blend(&mut self, id: &str, blend: BlendMode) -> bool {
        if let Some(layer) = self.layers.iter_mut().find(|layer| layer.id == id) {
            layer.blend = blend;
            true
        } else {
            false
        }
    }

    /// Return all visible layers stably sorted by ascending `z_index`.
    pub fn sorted_visible_layers(&self) -> Vec<&CompositeLayer> {
        let mut visible: Vec<&CompositeLayer> =
            self.layers.iter().filter(|layer| layer.visible).collect();
        visible.sort_by_key(|layer| layer.z_index);
        visible
    }

    /// Borrow all layers in insertion order.
    pub fn layers(&self) -> &[CompositeLayer] {
        &self.layers
    }

    /// Total number of layers (visible and hidden).
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Returns `true` if the stack contains no layers.
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

/// std140-aligned uniform block passed to [`LAYER_COMPOSITE_FRAGMENT_GLSL`].
///
/// Four `vec4` slots (64 bytes total) eliminate all std140 padding ambiguity
/// between Rust and GLSL 4.50.
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
struct LayerCompositeUniforms {
    /// `[offset_x, offset_y, scale_x, scale_y]`
    transform: [f32; 4],
    /// `[cos(rotation_rad), sin(rotation_rad), opacity, mode_id]`
    rot_opacity_mode: [f32; 4],
    /// `[key_r, key_g, key_b, param0]` (`threshold` for LumaKey, `similarity` for ChromaKey)
    key_rgb_p0: [f32; 4],
    /// `[param1, 0.0, 0.0, 0.0]` (`softness` for LumaKey, `smoothness` for ChromaKey)
    key_extra: [f32; 4],
}

impl LayerCompositeUniforms {
    fn from_layer(layer: &CompositeLayer) -> Self {
        let (sin_rot, cos_rot) = layer.transform.rotation_rad.sin_cos();
        let opacity = layer.transform.opacity.clamp(0.0, 1.0);

        let (mode_id, key_rgb_p0, key_extra) = match layer.blend {
            BlendMode::Normal => (0.0_f32, [0.0; 4], [0.0; 4]),
            BlendMode::Additive => (1.0_f32, [0.0; 4], [0.0; 4]),
            BlendMode::Screen => (2.0_f32, [0.0; 4], [0.0; 4]),
            BlendMode::Multiply => (3.0_f32, [0.0; 4], [0.0; 4]),
            BlendMode::LumaKey {
                threshold,
                softness,
            } => (
                4.0_f32,
                [0.0, 0.0, 0.0, threshold],
                [softness, 0.0, 0.0, 0.0],
            ),
            BlendMode::ChromaKey {
                key_rgb,
                similarity,
                smoothness,
            } => (
                5.0_f32,
                [key_rgb[0], key_rgb[1], key_rgb[2], similarity],
                [smoothness, 0.0, 0.0, 0.0],
            ),
        };

        Self {
            transform: [
                layer.transform.offset_xy[0],
                layer.transform.offset_xy[1],
                layer.transform.scale_xy[0],
                layer.transform.scale_xy[1],
            ],
            rot_opacity_mode: [cos_rot, sin_rot, opacity, mode_id],
            key_rgb_p0,
            key_extra,
        }
    }
}

/// GLSL 4.50 fragment shader for per-layer GPU compositing (2D transform,
/// blend modes, LumaKey, and ChromaKey).
const LAYER_COMPOSITE_FRAGMENT_GLSL: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 0) uniform Uniforms {
    vec4 u_transform;
    vec4 u_rot_opacity_mode;
    vec4 u_key_rgb_p0;
    vec4 u_key_extra;
};
layout(set = 0, binding = 1) uniform texture2D u_base;
layout(set = 0, binding = 2) uniform sampler u_base_sampler;
layout(set = 0, binding = 3) uniform texture2D u_top;
layout(set = 0, binding = 4) uniform sampler u_top_sampler;

void main() {
    vec4 base = texture(sampler2D(u_base, u_base_sampler), uv);

    vec2 offset_xy = u_transform.xy;
    vec2 scale_xy = u_transform.zw;
    float cos_rot = u_rot_opacity_mode.x;
    float sin_rot = u_rot_opacity_mode.y;
    float opacity = clamp(u_rot_opacity_mode.z, 0.0, 1.0);
    int mode = int(u_rot_opacity_mode.w + 0.5);

    if (abs(scale_xy.x) < 1e-6 || abs(scale_xy.y) < 1e-6) {
        fragColor = base;
        return;
    }

    vec2 d = uv - vec2(0.5) - offset_xy;
    vec2 rotated = vec2(
        d.x * cos_rot + d.y * sin_rot,
        -d.x * sin_rot + d.y * cos_rot
    );
    vec2 src_uv = rotated / scale_xy + vec2(0.5);

    if (src_uv.x < 0.0 || src_uv.x > 1.0 || src_uv.y < 0.0 || src_uv.y > 1.0) {
        fragColor = base;
        return;
    }

    vec4 top = texture(sampler2D(u_top, u_top_sampler), src_uv);
    float eff_alpha = clamp(top.a * opacity, 0.0, 1.0);

    if (mode == 1) {
        // Additive
        vec3 out_rgb = clamp(base.rgb * base.a + top.rgb * eff_alpha, 0.0, 1.0);
        float out_a = clamp(base.a + eff_alpha, 0.0, 1.0);
        fragColor = vec4(out_rgb, out_a);
        return;
    }

    if (mode == 2) {
        // Screen
        vec3 screen_rgb = vec3(1.0) - (vec3(1.0) - base.rgb) * (vec3(1.0) - top.rgb);
        float out_a = eff_alpha + base.a * (1.0 - eff_alpha);
        vec3 out_rgb = (out_a > 1e-6)
            ? (mix(base.rgb, screen_rgb, eff_alpha) * base.a + top.rgb * eff_alpha * (1.0 - base.a)) / out_a
            : vec3(0.0);
        fragColor = vec4(clamp(out_rgb, 0.0, 1.0), clamp(out_a, 0.0, 1.0));
        return;
    }

    if (mode == 3) {
        // Multiply
        vec3 mul_rgb = base.rgb * top.rgb;
        float out_a = eff_alpha + base.a * (1.0 - eff_alpha);
        vec3 out_rgb = (out_a > 1e-6)
            ? (mix(base.rgb, mul_rgb, eff_alpha) * base.a + top.rgb * eff_alpha * (1.0 - base.a)) / out_a
            : vec3(0.0);
        fragColor = vec4(clamp(out_rgb, 0.0, 1.0), clamp(out_a, 0.0, 1.0));
        return;
    }

    if (mode == 4) {
        // LumaKey
        float luma = dot(top.rgb, vec3(0.2126, 0.7152, 0.0722));
        float threshold = u_key_rgb_p0.w;
        float softness = max(u_key_extra.x, 0.0);
        float key_alpha = (softness > 1e-5)
            ? smoothstep(threshold, threshold + softness, luma)
            : (luma <= threshold ? 0.0 : 1.0);
        eff_alpha *= key_alpha;
    } else if (mode == 5) {
        // ChromaKey
        float dist = distance(top.rgb, u_key_rgb_p0.xyz);
        float similarity = max(u_key_rgb_p0.w, 0.0);
        float smoothness = max(u_key_extra.x, 0.0);
        float key_alpha = (smoothness > 1e-5)
            ? smoothstep(similarity, similarity + smoothness, dist)
            : (dist <= similarity ? 0.0 : 1.0);
        eff_alpha *= key_alpha;
    }

    // Normal (Porter-Duff source-over), also shared as final composite for LumaKey/ChromaKey.
    float out_a = eff_alpha + base.a * (1.0 - eff_alpha);
    vec3 out_rgb = (out_a > 1e-6)
        ? (top.rgb * eff_alpha + base.rgb * base.a * (1.0 - eff_alpha)) / out_a
        : vec3(0.0);
    fragColor = vec4(clamp(out_rgb, 0.0, 1.0), clamp(out_a, 0.0, 1.0));
}
"#;

/// Validate `target_size` and every layer's dimensions and RGBA8 byte length.
fn validate_inputs(stack: &LayerStack, target_size: (u32, u32)) -> Result<usize, RenderError> {
    if target_size.0 == 0 || target_size.1 == 0 {
        return Err(RenderError::TargetSizeInvalid(target_size));
    }

    let target_bytes = (target_size.0 as usize)
        .checked_mul(target_size.1 as usize)
        .and_then(|px| px.checked_mul(4))
        .ok_or(RenderError::TargetSizeInvalid(target_size))?;

    for layer in stack.layers() {
        if layer.width == 0 || layer.height == 0 {
            return Err(RenderError::TargetSizeInvalid((layer.width, layer.height)));
        }
        let expected_len = (layer.width as usize)
            .checked_mul(layer.height as usize)
            .and_then(|px| px.checked_mul(4))
            .ok_or(RenderError::TargetSizeInvalid((layer.width, layer.height)))?;
        if layer.rgba_pixels.len() != expected_len {
            return Err(RenderError::UniformBindingFailed {
                pass_label: layer.id.clone(),
                name: "rgba_pixels".to_string(),
                reason: format!(
                    "layer pixel buffer length {} != expected {} bytes for {}x{} RGBA8",
                    layer.rgba_pixels.len(),
                    expected_len,
                    layer.width,
                    layer.height,
                ),
            });
        }
    }

    Ok(target_bytes)
}

/// Composite all visible layers in `stack` in ascending `z_index` order on the
/// GPU via ping-pong `Rgba8Unorm` render passes and return the RGBA8 readback.
///
/// An empty visible stack returns a clean transparent black `(w * h * 4)` byte
/// buffer without erroring.
///
/// # Errors
///
/// - [`RenderError::TargetSizeInvalid`] if `target_size` or any layer has zero
///   width or height.
/// - [`RenderError::UniformBindingFailed`] if any layer's `rgba_pixels` length
///   does not match `width * height * 4`.
/// - Shader compilation or GPU readback errors from [`RenderError`].
pub fn render_layer_stack(
    renderer: &MultipassRenderer,
    stack: &LayerStack,
    target_size: (u32, u32),
) -> Result<Vec<u8>, RenderError> {
    let target_bytes = validate_inputs(stack, target_size)?;

    let visible_layers = stack.sorted_visible_layers();
    if visible_layers.is_empty() {
        return Ok(vec![0_u8; target_bytes]);
    }

    let device = renderer.device();
    let queue = renderer.queue();

    // Compile vertex and fragment shaders via naga GLSL 4.50 frontend.
    let vertex_module = compile_glsl_module(FULLSCREEN_TRIANGLE_GLSL, naga::ShaderStage::Vertex)?;
    let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("chromaticity-render compositor vertex (fullscreen triangle)"),
        source: wgpu::ShaderSource::Naga(Cow::Owned(vertex_module)),
    });

    let fragment_module =
        compile_glsl_module(LAYER_COMPOSITE_FRAGMENT_GLSL, naga::ShaderStage::Fragment)?;
    let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("chromaticity-render compositor fragment (blend/key)"),
        source: wgpu::ShaderSource::Naga(Cow::Owned(fragment_module)),
    });

    // Nearest/clamp sampler matches the multipass scheduler and avoids
    // bilinear fringe bleeding across sharp chroma-key edges.
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("chromaticity-render compositor sampler (nearest/clamp)"),
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

    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("chromaticity-render compositor bind group layout"),
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

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("chromaticity-render compositor pipeline layout"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("chromaticity-render compositor pipeline"),
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
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    });

    // Allocate ping-pong accumulation targets.
    let target_extent = wgpu::Extent3d {
        width: target_size.0,
        height: target_size.1,
        depth_or_array_layers: 1,
    };
    let ping_pong_desc = |label| wgpu::TextureDescriptor {
        label: Some(label),
        size: target_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    };
    let ping_pong = [
        device.create_texture(&ping_pong_desc("chromaticity-render compositor ping")),
        device.create_texture(&ping_pong_desc("chromaticity-render compositor pong")),
    ];
    let ping_pong_views = [
        ping_pong[0].create_view(&wgpu::TextureViewDescriptor::default()),
        ping_pong[1].create_view(&wgpu::TextureViewDescriptor::default()),
    ];

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("chromaticity-render compositor encoder"),
    });

    // Clear initial base target (slot 0) to transparent black (0, 0, 0, 0).
    {
        let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("chromaticity-render compositor clear base"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &ping_pong_views[0],
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }

    // Hold per-layer GPU resources alive until submission completes.
    let mut layer_textures: Vec<wgpu::Texture> = Vec::with_capacity(visible_layers.len());
    let mut layer_uniforms: Vec<wgpu::Buffer> = Vec::with_capacity(visible_layers.len());
    let mut bind_groups: Vec<wgpu::BindGroup> = Vec::with_capacity(visible_layers.len());

    let mut src_idx: usize = 0;

    for layer in &visible_layers {
        let dst_idx = 1 - src_idx;

        let layer_extent = wgpu::Extent3d {
            width: layer.width,
            height: layer.height,
            depth_or_array_layers: 1,
        };
        let layer_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("chromaticity-render compositor layer source"),
            size: layer_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &layer_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &layer.rgba_pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(layer.width * 4),
                rows_per_image: Some(layer.height),
            },
            layer_extent,
        );
        let layer_view = layer_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let uniform_data = LayerCompositeUniforms::from_layer(layer);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chromaticity-render compositor layer uniforms"),
            contents: bytemuck::bytes_of(&uniform_data),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("chromaticity-render compositor bind group"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&ping_pong_views[src_idx]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&layer_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        layer_textures.push(layer_texture);
        layer_uniforms.push(uniform_buffer);
        bind_groups.push(bind_group);

        if let Some(bg) = bind_groups.last() {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("chromaticity-render compositor layer pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &ping_pong_views[dst_idx],
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&pipeline);
            rpass.set_bind_group(0, bg, &[]);
            rpass.draw(0..3, 0..1);
        }

        src_idx = dst_idx;
    }

    queue.submit(Some(encoder.finish()));

    // Keep references alive until after submission.
    drop(bind_groups);
    drop(layer_uniforms);
    drop(layer_textures);

    readback_texture_to_bytes(device, queue, &ping_pong[src_idx])
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod compositor_tests {
    use super::*;

    fn solid_rgba(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
        let count = (width * height) as usize;
        let mut out = Vec::with_capacity(count * 4);
        for _ in 0..count {
            out.extend_from_slice(&rgba);
        }
        out
    }

    fn pixel_at(bytes: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let idx = ((y * width + x) * 4) as usize;
        [bytes[idx], bytes[idx + 1], bytes[idx + 2], bytes[idx + 3]]
    }

    fn assert_rgba_near(actual: [u8; 4], expected: [u8; 4], tol: u8) {
        for ch in 0..4 {
            let diff = actual[ch].abs_diff(expected[ch]);
            assert!(
                diff <= tol,
                "channel {ch} mismatch: actual={actual:?}, expected={expected:?}, tol={tol}"
            );
        }
    }

    #[test]
    fn empty_stack_and_validation_checks() {
        let renderer = MultipassRenderer::new().expect("wgpu Metal/Vulkan adapter required");

        // 1. Empty stack returns clean transparent black frame.
        let empty = LayerStack::new();
        let frame = render_layer_stack(&renderer, &empty, (8, 6)).unwrap();
        assert_eq!(frame.len(), 8 * 6 * 4);
        assert!(frame.iter().all(|&b| b == 0));

        // 2. Zero target_size is rejected before GPU work.
        let err_w = render_layer_stack(&renderer, &empty, (0, 8)).unwrap_err();
        assert!(matches!(err_w, RenderError::TargetSizeInvalid((0, 8))));
        let err_h = render_layer_stack(&renderer, &empty, (8, 0)).unwrap_err();
        assert!(matches!(err_h, RenderError::TargetSizeInvalid((8, 0))));

        // 3. Invalid layer dimensions rejected.
        let mut bad_dim_stack = LayerStack::new();
        bad_dim_stack.push_layer(CompositeLayer {
            id: "zero_dim".to_string(),
            visible: true,
            z_index: 0,
            blend: BlendMode::Normal,
            transform: LayerTransform::default(),
            width: 0,
            height: 4,
            rgba_pixels: Vec::new(),
        });
        let err_dim = render_layer_stack(&renderer, &bad_dim_stack, (8, 8)).unwrap_err();
        assert!(matches!(err_dim, RenderError::TargetSizeInvalid((0, 4))));

        // 4. Mismatched pixel buffer length rejected.
        let mut bad_len_stack = LayerStack::new();
        bad_len_stack.push_layer(CompositeLayer {
            id: "short_buf".to_string(),
            visible: true,
            z_index: 0,
            blend: BlendMode::Normal,
            transform: LayerTransform::default(),
            width: 4,
            height: 4,
            rgba_pixels: vec![255; 12],
        });
        let err_len = render_layer_stack(&renderer, &bad_len_stack, (8, 8)).unwrap_err();
        assert!(matches!(
            err_len,
            RenderError::UniformBindingFailed { ref pass_label, .. } if pass_label == "short_buf"
        ));
    }

    #[test]
    fn normal_alpha_blend_50pct_red_over_blue_yields_purple() {
        let renderer = MultipassRenderer::new().expect("wgpu adapter required");
        let (w, h) = (16, 16);

        let mut stack = LayerStack::new();
        // Push out of z_index order to verify z-sorting.
        stack.push_layer(CompositeLayer {
            id: "red_top".to_string(),
            visible: true,
            z_index: 10,
            blend: BlendMode::Normal,
            transform: LayerTransform {
                opacity: 0.5,
                ..LayerTransform::default()
            },
            width: w,
            height: h,
            rgba_pixels: solid_rgba(w, h, [255, 0, 0, 255]),
        });
        stack.push_layer(CompositeLayer {
            id: "blue_bg".to_string(),
            visible: true,
            z_index: 1,
            blend: BlendMode::Normal,
            transform: LayerTransform::default(),
            width: w,
            height: h,
            rgba_pixels: solid_rgba(w, h, [0, 0, 255, 255]),
        });

        let out = render_layer_stack(&renderer, &stack, (w, h)).unwrap();
        assert_eq!(out.len(), (w * h * 4) as usize);
        assert_rgba_near(pixel_at(&out, w, 8, 8), [128, 0, 128, 255], 2);
    }

    #[test]
    fn additive_blend_green_plus_blue_yields_cyan() {
        let renderer = MultipassRenderer::new().expect("wgpu adapter required");
        let (w, h) = (16, 16);

        let mut stack = LayerStack::new();
        stack.push_layer(CompositeLayer {
            id: "blue_bg".to_string(),
            visible: true,
            z_index: 0,
            blend: BlendMode::Normal,
            transform: LayerTransform::default(),
            width: w,
            height: h,
            rgba_pixels: solid_rgba(w, h, [0, 0, 255, 255]),
        });
        stack.push_layer(CompositeLayer {
            id: "green_add".to_string(),
            visible: true,
            z_index: 1,
            blend: BlendMode::Additive,
            transform: LayerTransform::default(),
            width: w,
            height: h,
            rgba_pixels: solid_rgba(w, h, [0, 255, 0, 255]),
        });

        let out = render_layer_stack(&renderer, &stack, (w, h)).unwrap();
        assert_rgba_near(pixel_at(&out, w, 4, 4), [0, 255, 255, 255], 1);
    }

    #[test]
    fn chroma_key_green_screen_reveals_blue_bg_and_preserves_red_center() {
        let renderer = MultipassRenderer::new().expect("wgpu adapter required");
        let (w, h) = (16, 16);

        // Pure green foreground with a centered 6x6 red square (x,y in 5..11).
        let mut fg_pixels = solid_rgba(w, h, [0, 255, 0, 255]);
        for y in 5..11 {
            for x in 5..11 {
                let idx = ((y * w + x) * 4) as usize;
                fg_pixels[idx..idx + 4].copy_from_slice(&[255, 0, 0, 255]);
            }
        }

        let mut stack = LayerStack::new();
        stack.push_layer(CompositeLayer {
            id: "blue_bg".to_string(),
            visible: true,
            z_index: 0,
            blend: BlendMode::Normal,
            transform: LayerTransform::default(),
            width: w,
            height: h,
            rgba_pixels: solid_rgba(w, h, [0, 0, 255, 255]),
        });
        stack.push_layer(CompositeLayer {
            id: "green_screen_fg".to_string(),
            visible: true,
            z_index: 1,
            blend: BlendMode::ChromaKey {
                key_rgb: [0.0, 1.0, 0.0],
                similarity: 0.4,
                smoothness: 0.1,
            },
            transform: LayerTransform::default(),
            width: w,
            height: h,
            rgba_pixels: fg_pixels,
        });

        let out = render_layer_stack(&renderer, &stack, (w, h)).unwrap();
        // Corners were green -> keyed transparent -> blue background shows through.
        assert_rgba_near(pixel_at(&out, w, 1, 1), [0, 0, 255, 255], 1);
        assert_rgba_near(pixel_at(&out, w, 14, 14), [0, 0, 255, 255], 1);
        // Center square was red -> preserved intact.
        assert_rgba_near(pixel_at(&out, w, 8, 8), [255, 0, 0, 255], 1);
    }

    #[test]
    fn layer_transform_visibility_scale_and_offset() {
        let renderer = MultipassRenderer::new().expect("wgpu adapter required");
        let (w, h) = (16, 16);

        let mut stack = LayerStack::new();
        stack.push_layer(CompositeLayer {
            id: "bg".to_string(),
            visible: true,
            z_index: 0,
            blend: BlendMode::Normal,
            transform: LayerTransform::default(),
            width: w,
            height: h,
            rgba_pixels: solid_rgba(w, h, [0, 0, 200, 255]),
        });
        stack.push_layer(CompositeLayer {
            id: "fg".to_string(),
            visible: true,
            z_index: 1,
            blend: BlendMode::Normal,
            transform: LayerTransform {
                // Scale to 50% width/height and shift right by +0.25 UV:
                // occupies x in [0.5, 1.0], y in [0.25, 0.75].
                offset_xy: [0.25, 0.0],
                scale_xy: [0.5, 0.5],
                rotation_rad: 0.0,
                opacity: 1.0,
            },
            width: w,
            height: h,
            rgba_pixels: solid_rgba(w, h, [255, 200, 0, 255]),
        });

        let out_scaled = render_layer_stack(&renderer, &stack, (w, h)).unwrap();
        // Right-center pixel (x=12 => u≈0.78, y=8 => v≈0.53) is inside transformed fg.
        assert_rgba_near(pixel_at(&out_scaled, w, 12, 8), [255, 200, 0, 255], 1);
        // Left-center pixel (x=2 => u≈0.15, y=8 => v≈0.53) is outside fg -> bg shows.
        assert_rgba_near(pixel_at(&out_scaled, w, 2, 8), [0, 0, 200, 255], 1);

        // Toggle visibility off -> entire frame becomes background blue.
        assert!(stack.set_visible("fg", false));
        let out_hidden = render_layer_stack(&renderer, &stack, (w, h)).unwrap();
        assert_rgba_near(pixel_at(&out_hidden, w, 12, 8), [0, 0, 200, 255], 1);
        assert_rgba_near(pixel_at(&out_hidden, w, 2, 8), [0, 0, 200, 255], 1);

        // Verify set_transform, set_blend, and remove_layer stack mutations.
        assert!(stack.set_transform("fg", LayerTransform::default()));
        assert!(stack.set_blend(
            "fg",
            BlendMode::LumaKey {
                threshold: 0.2,
                softness: 0.05,
            }
        ));
        assert!(stack.remove_layer("fg").is_some());
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn luma_key_screen_and_multiply_modes() {
        let renderer = MultipassRenderer::new().expect("wgpu adapter required");
        let (w, h) = (8, 8);

        // LumaKey: black left half (luma=0) keyed out over red bg; white right half kept.
        let mut luma_fg = solid_rgba(w, h, [0, 0, 0, 255]);
        for y in 0..h {
            for x in (w / 2)..w {
                let idx = ((y * w + x) * 4) as usize;
                luma_fg[idx..idx + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        let mut stack = LayerStack::new();
        stack.push_layer(CompositeLayer {
            id: "bg".to_string(),
            visible: true,
            z_index: 0,
            blend: BlendMode::Normal,
            transform: LayerTransform::default(),
            width: w,
            height: h,
            rgba_pixels: solid_rgba(w, h, [255, 0, 0, 255]),
        });
        stack.push_layer(CompositeLayer {
            id: "luma".to_string(),
            visible: true,
            z_index: 1,
            blend: BlendMode::LumaKey {
                threshold: 0.1,
                softness: 0.05,
            },
            transform: LayerTransform::default(),
            width: w,
            height: h,
            rgba_pixels: luma_fg,
        });
        let out_luma = render_layer_stack(&renderer, &stack, (w, h)).unwrap();
        assert_rgba_near(pixel_at(&out_luma, w, 1, 4), [255, 0, 0, 255], 1);
        assert_rgba_near(pixel_at(&out_luma, w, 6, 4), [255, 255, 255, 255], 1);

        // Multiply: gray(0.5) * white(1.0) = gray(0.5); gray(0.5) * red(1,0,0) = (0.5, 0, 0).
        let mut mul_stack = LayerStack::new();
        mul_stack.push_layer(CompositeLayer {
            id: "bg".to_string(),
            visible: true,
            z_index: 0,
            blend: BlendMode::Normal,
            transform: LayerTransform::default(),
            width: w,
            height: h,
            rgba_pixels: solid_rgba(w, h, [255, 255, 255, 255]),
        });
        mul_stack.push_layer(CompositeLayer {
            id: "top".to_string(),
            visible: true,
            z_index: 1,
            blend: BlendMode::Multiply,
            transform: LayerTransform::default(),
            width: w,
            height: h,
            rgba_pixels: solid_rgba(w, h, [128, 0, 255, 255]),
        });
        let out_mul = render_layer_stack(&renderer, &mul_stack, (w, h)).unwrap();
        assert_rgba_near(pixel_at(&out_mul, w, 4, 4), [128, 0, 255, 255], 2);

        // Screen: red(1,0,0) screen blue(0,0,1) = magenta(1,0,1).
        let mut scr_stack = LayerStack::new();
        scr_stack.push_layer(CompositeLayer {
            id: "bg".to_string(),
            visible: true,
            z_index: 0,
            blend: BlendMode::Normal,
            transform: LayerTransform::default(),
            width: w,
            height: h,
            rgba_pixels: solid_rgba(w, h, [255, 0, 0, 255]),
        });
        scr_stack.push_layer(CompositeLayer {
            id: "top".to_string(),
            visible: true,
            z_index: 1,
            blend: BlendMode::Screen,
            transform: LayerTransform::default(),
            width: w,
            height: h,
            rgba_pixels: solid_rgba(w, h, [0, 0, 255, 255]),
        });
        let out_scr = render_layer_stack(&renderer, &scr_stack, (w, h)).unwrap();
        assert_rgba_near(pixel_at(&out_scr, w, 4, 4), [255, 0, 255, 255], 1);
    }
}
