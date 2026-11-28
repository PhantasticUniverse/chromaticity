// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Renderer-global scene media input (`chroma_Media`).
//!
//! One RGBA8 texture, a linear / clamp sampler, and a small std140 info
//! block, bound at **bind group 1** of every cached pipeline
//! ([`crate::CachedSinglePass`], [`crate::CachedMultipass`]) so the media
//! binding never collides with the per-pass uniform + prior-pass input
//! bindings in group 0.
//!
//! GLSL view (injected by the shell's media prelude):
//!
//! ```glsl
//! layout(set = 1, binding = 0) uniform texture2D _chroma_media_tex;
//! layout(set = 1, binding = 1) uniform sampler _chroma_media_samp;
//! layout(set = 1, binding = 2, std140) uniform _ChromaMediaInfo {
//!     vec4 _chroma_media_info;   // x = active (0/1), y = chroma_MediaType, z = w, w = h
//!     vec4 _chroma_media_target; // x = render target w, y = render target h
//! };
//! ```
//!
//! Texel row 0 is the TOP of the uploaded image, matching the fullscreen
//! triangle's `uv` (`uv.y == 0` at the top of the output), so
//! `texture(chroma_Media, uv)` is upright.
//!
//! When no media is selected the texture is a single opaque black texel
//! and `_chroma_media_info.x == 0`, which is exactly what the previous
//! black stub returned.

use crate::single_pass::RenderError;

/// Scene media category, published to shaders as `chroma_MediaType`
/// (`docs/architecture/36-reactive-symbol-catalog.md` §1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    /// Still image (`chroma_MediaType == 1`).
    Image,
    /// Video file or screen capture (`chroma_MediaType == 2`).
    Video,
    /// Live camera (`chroma_MediaType == 3`).
    Webcam,
}

impl MediaKind {
    /// Catalog `chroma_MediaType` value.
    #[must_use]
    pub fn type_code(self) -> f32 {
        match self {
            Self::Image => 1.0,
            Self::Video => 2.0,
            Self::Webcam => 3.0,
        }
    }
}

/// Largest accepted media frame edge (pixels).
pub const MAX_MEDIA_DIM: u32 = 4096;

/// GPU-side scene media binding (bind group 1). Owned by
/// [`crate::MultipassRenderer`].
pub struct MediaInput {
    layout: wgpu::BindGroupLayout,
    texture: wgpu::Texture,
    sampler: wgpu::Sampler,
    info_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    size: (u32, u32),
    /// `[active, type, w, h, target_w, target_h, 0, 0]`.
    info: [f32; 8],
}

impl MediaInput {
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("chromaticity-render media bind group layout (set 1)"),
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("chromaticity-render media sampler (linear/clamp)"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let info_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chromaticity-render media info"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let texture = create_media_texture(device, (1, 1));
        let bind_group = create_bind_group(device, &layout, &texture, &sampler, &info_buffer);
        let media = Self {
            layout,
            texture,
            sampler,
            info_buffer,
            bind_group,
            size: (1, 1),
            info: [0.0; 8],
        };
        media.write_black(queue);
        media.write_info(queue);
        media
    }

    /// `true` while an uploaded frame is bound.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.info[0] > 0.5
    }

    /// Dimensions of the bound texture (`(1, 1)` when inactive).
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Upload one tightly packed RGBA8 frame (row 0 = top) and mark media
    /// active. Reallocates the texture only when the dimensions change.
    ///
    /// # Errors
    ///
    /// [`RenderError::TargetSizeInvalid`] when a dimension is zero or
    /// exceeds [`MAX_MEDIA_DIM`], or when `rgba.len() != width * height * 4`.
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
        kind: MediaKind,
    ) -> Result<(), RenderError> {
        let expected = u64::from(width) * u64::from(height) * 4;
        if width == 0
            || height == 0
            || width > MAX_MEDIA_DIM
            || height > MAX_MEDIA_DIM
            || rgba.len() as u64 != expected
        {
            return Err(RenderError::TargetSizeInvalid((width, height)));
        }
        if self.size != (width, height) {
            self.texture = create_media_texture(device, (width, height));
            self.bind_group = create_bind_group(
                device,
                &self.layout,
                &self.texture,
                &self.sampler,
                &self.info_buffer,
            );
            self.size = (width, height);
        }
        write_texture(queue, &self.texture, (width, height), rgba);
        self.info[0] = 1.0;
        self.info[1] = kind.type_code();
        self.info[2] = width as f32;
        self.info[3] = height as f32;
        self.write_info(queue);
        Ok(())
    }

    /// Unbind media: back to the inactive single black texel.
    pub fn clear(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.size != (1, 1) {
            self.texture = create_media_texture(device, (1, 1));
            self.bind_group = create_bind_group(
                device,
                &self.layout,
                &self.texture,
                &self.sampler,
                &self.info_buffer,
            );
            self.size = (1, 1);
        }
        self.write_black(queue);
        self.info[0..4].copy_from_slice(&[0.0; 4]);
        self.write_info(queue);
    }

    pub(crate) fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Publish the current render-target size (`_chroma_media_target`) so
    /// `_loadMedia()` can map `gl_FragCoord` to output UV. Called by the
    /// cached pipelines before each submit.
    pub(crate) fn write_target_size(&self, queue: &wgpu::Queue, target: (u32, u32)) {
        let bytes: Vec<u8> = [target.0 as f32, target.1 as f32, 0.0, 0.0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        queue.write_buffer(&self.info_buffer, 16, &bytes);
    }

    fn write_info(&self, queue: &wgpu::Queue) {
        let bytes: Vec<u8> = self.info[0..4]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        queue.write_buffer(&self.info_buffer, 0, &bytes);
    }

    fn write_black(&self, queue: &wgpu::Queue) {
        write_texture(queue, &self.texture, (1, 1), &[0, 0, 0, 255]);
    }
}

fn create_media_texture(device: &wgpu::Device, size: (u32, u32)) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("chromaticity-render media texture"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    texture: &wgpu::Texture,
    sampler: &wgpu::Sampler,
    info_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("chromaticity-render media bind group (set 1)"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: info_buffer.as_entire_binding(),
            },
        ],
    })
}

fn write_texture(queue: &wgpu::Queue, texture: &wgpu::Texture, size: (u32, u32), rgba: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size.0 * 4),
            rows_per_image: Some(size.1),
        },
        wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
    );
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod media_input_pins {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — scene media binding (group 1) orientation, channel order, lifecycle.

    use super::MediaKind;
    use crate::{
        MultipassRenderer, MultipassSpec, PassDef, TargetFormat, UniformInput, UniformValue,
    };

    const MEDIA_DECLS: &str = r"
layout(set = 1, binding = 0) uniform texture2D _chroma_media_tex;
layout(set = 1, binding = 1) uniform sampler _chroma_media_samp;
layout(set = 1, binding = 2, std140) uniform _ChromaMediaInfo {
    vec4 _chroma_media_info;
    vec4 _chroma_media_target;
};
";

    fn sample_shader() -> String {
        format!(
            "#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0, std140) uniform U {{ vec4 dummy; }};
{MEDIA_DECLS}
void main() {{
    fragColor = texture(sampler2D(_chroma_media_tex, _chroma_media_samp), uv) + 0.0 * dummy;
}}
"
        )
    }

    fn info_shader() -> String {
        format!(
            "#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0, std140) uniform U {{ vec4 dummy; }};
{MEDIA_DECLS}
void main() {{
    fragColor = vec4(_chroma_media_info.x, _chroma_media_info.y / 4.0,
                     _chroma_media_target.x / 255.0, _chroma_media_target.y / 255.0) + 0.0 * dummy;
}}
"
        )
    }

    fn dummy() -> Vec<UniformInput> {
        vec![UniformInput {
            name: "dummy".into(),
            value: UniformValue::Vec4([0.0; 4]),
        }]
    }

    /// 2x2 quadrants: TL red, TR green, BL blue, BR white.
    const QUAD: [u8; 16] = [
        255, 0, 0, 255, 0, 255, 0, 255, // top row
        0, 0, 255, 255, 255, 255, 255, 255, // bottom row
    ];

    #[test]
    fn uploaded_media_is_upright_with_rgba_channel_order() {
        let mut renderer = MultipassRenderer::new().expect("wgpu renderer");
        renderer
            .upload_media(2, 2, &QUAD, MediaKind::Image)
            .expect("upload");
        let mut pass = renderer
            .compile_cached_single_pass("media", &sample_shader(), &dummy(), (2, 2))
            .expect("compile");
        // Target pixel centres sit exactly on texel centres, so the
        // linear sampler returns the texels unchanged.
        let out = pass.render(&renderer, (2, 2), &dummy()).expect("render");
        assert_eq!(
            out,
            QUAD.to_vec(),
            "row 0 of the output must be the top row of the media"
        );
    }

    #[test]
    fn inactive_media_samples_opaque_black_and_reports_inactive() {
        let renderer = MultipassRenderer::new().expect("wgpu renderer");
        assert!(!renderer.media().is_active());
        let mut pass = renderer
            .compile_cached_single_pass("media", &sample_shader(), &dummy(), (4, 4))
            .expect("compile");
        let out = pass.render(&renderer, (4, 4), &dummy()).expect("render");
        assert!(out.chunks(4).all(|p| p == [0, 0, 0, 255]));
    }

    #[test]
    fn info_block_reports_activity_type_and_target_size_then_clears() {
        let mut renderer = MultipassRenderer::new().expect("wgpu renderer");
        renderer
            .upload_media(2, 2, &QUAD, MediaKind::Webcam)
            .expect("upload");
        let mut pass = renderer
            .compile_cached_single_pass("info", &info_shader(), &dummy(), (8, 4))
            .expect("compile");
        let out = pass.render(&renderer, (8, 4), &dummy()).expect("render");
        // active = 1, type 3 / 4 = 0.75, target 8 / 255, 4 / 255.
        assert_eq!(&out[0..4], &[255, 191, 8, 4]);

        renderer.clear_media();
        assert!(!renderer.media().is_active());
        assert_eq!(renderer.media().size(), (1, 1));
        let out = pass.render(&renderer, (8, 4), &dummy()).expect("render");
        assert_eq!(
            &out[0..2],
            &[0, 0],
            "cleared media reports inactive, type 0"
        );
    }

    #[test]
    fn upload_rejects_bad_dimensions_and_lengths() {
        let mut renderer = MultipassRenderer::new().expect("wgpu renderer");
        assert!(
            renderer
                .upload_media(2, 2, &QUAD[..12], MediaKind::Image)
                .is_err()
        );
        assert!(renderer.upload_media(0, 2, &[], MediaKind::Image).is_err());
        let too_wide = super::MAX_MEDIA_DIM + 1;
        assert!(
            renderer
                .upload_media(
                    too_wide,
                    1,
                    &vec![0; too_wide as usize * 4],
                    MediaKind::Image
                )
                .is_err()
        );
        assert!(
            !renderer.media().is_active(),
            "rejected uploads leave media inactive"
        );
    }

    #[test]
    fn cached_multipass_passes_see_media() {
        let mut renderer = MultipassRenderer::new().expect("wgpu renderer");
        renderer
            .upload_media(2, 2, &QUAD, MediaKind::Video)
            .expect("upload");
        let spec = MultipassSpec {
            passes: vec![PassDef {
                label: Some("final".into()),
                fragment_glsl: sample_shader(),
                uniforms: dummy(),
                input_textures: Vec::new(),
                target_format: TargetFormat::Rgba8Unorm,
                feedback_self: false,
            }],
            target_size: (2, 2),
            frame_count: 1,
        };
        let mut cached = renderer.compile_cached_multipass(&spec).expect("compile");
        let out = cached
            .render_frames(&renderer, &dummy(), 1)
            .expect("render");
        assert_eq!(out, QUAD.to_vec());
    }
}
