// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#![forbid(unsafe_code)]

//! Phase 7a GPU BGRA/RGBA → NV12 (BT.709) compute pass (`wgpu 26`).
//!
//! Converts an even-dimensioned `Bgra8Unorm` or `Rgba8Unorm` GPU texture into
//! bi-planar 4:2:0 NV12 (`Y` luma plane + 2×2 box-averaged interleaved `UV`
//! chroma plane) using cached WGSL compute pipelines:
//!
//! - **Luma `Y` plane:** `width * height` bytes, ITU-R BT.709 coefficients
//!   (`Kr = 0.2126`, `Kg = 0.7152`, `Kb = 0.0722`), selectable between
//!   [`ColorRange::Limited`] (`Y: 16..=235`) and [`ColorRange::Full`]
//!   (`Y: 0..=255`).
//! - **Chroma `UV` plane:** `(width / 2) * (height / 2) * 2` bytes (`[U, V]`
//!   interleaved per 2×2 pixel block), 2×2 box-averaged in RGB before
//!   BT.709 `Cb/Cr` projection, quantized to `16..=240` (limited) or
//!   `0..=255` (full).
//!
//! Outputs are written into tightly packed `storage` buffers (`array<u32>`,
//! 4 bytes per word) so the compute pass runs on every WebGPU/wgpu backend
//! (Metal, Vulkan, DX12, GLES, software adapters) without requiring optional
//! `R8Unorm`/`Rg8Unorm` storage-texture feature flags.

use std::borrow::Cow;
use std::sync::mpsc::{TryRecvError, channel};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::single_pass::RenderError;

/// BT.709 luma/chroma quantization range for NV12 conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorRange {
    /// Broadcast / video limited range (`Y: 16..=235`, `U/V: 16..=240`).
    /// Matches macOS `kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange` and
    /// Windows Media Foundation `MFVideoNominalRange_16_235`.
    #[default]
    Limited,
    /// Full 8-bit range (`Y/U/V: 0..=255`).
    /// Matches macOS `kCVPixelFormatType_420YpCbCr8BiPlanarFullRange` and
    /// Windows Media Foundation `MFVideoNominalRange_0_255`.
    Full,
}

impl ColorRange {
    const fn as_u32(self) -> u32 {
        match self {
            Self::Limited => 0,
            Self::Full => 1,
        }
    }
}

/// CPU-accessible NV12 bi-planar frame (`Y` + interleaved `UV`) produced by
/// [`Nv12Converter`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nv12Frame {
    /// Frame width in pixels (even, `> 0`).
    pub width: u32,
    /// Frame height in pixels (even, `> 0`).
    pub height: u32,
    /// Quantization range used to encode the planes.
    pub color_range: ColorRange,
    /// Tightly packed 8-bit luma plane (`width * height` bytes).
    pub y_plane: Vec<u8>,
    /// Tightly packed interleaved chroma plane (`(width / 2) * (height / 2) * 2`
    /// bytes, `[U0, V0, U1, V1, ...]`).
    pub uv_plane: Vec<u8>,
}

impl Nv12Frame {
    /// Concatenate `y_plane` followed immediately by `uv_plane` into a single
    /// contiguous NV12 byte buffer (`width * height * 3 / 2` bytes).
    #[must_use]
    pub fn into_contiguous_bytes(mut self) -> Vec<u8> {
        self.y_plane.extend_from_slice(&self.uv_plane);
        self.y_plane
    }
}

/// Uniform parameters passed to [`NV12_CONVERT_WGSL`].
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
struct Nv12Params {
    width: u32,
    height: u32,
    /// `0` = BT.709 Limited (`16..=235` / `16..=240`), `1` = Full (`0..=255`).
    color_range: u32,
    /// `0` = standard `textureLoad` RGB order (`Rgba8Unorm` or hardware-swizzled
    /// `Bgra8Unorm`), `1` = swap R and B channels in shader.
    swap_rb: u32,
}

/// Workgroup size used by both `cs_y_plane` and `cs_uv_plane`.
const WORKGROUP_SIZE: u32 = 256;

/// WGSL compute shader performing BT.709 RGB/BGRA → NV12 bi-planar conversion.
const NV12_CONVERT_WGSL: &str = r#"
struct Nv12Params {
    width: u32,
    height: u32,
    color_range: u32,
    swap_rb: u32,
};

@group(0) @binding(0) var<uniform> params: Nv12Params;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var<storage, read_write> y_plane: array<u32>;
@group(0) @binding(3) var<storage, read_write> uv_plane: array<u32>;

fn load_rgb(x: u32, y: u32) -> vec3<f32> {
    let raw = textureLoad(src_tex, vec2<i32>(i32(x), i32(y)), 0);
    if (params.swap_rb != 0u) {
        return vec3<f32>(raw.b, raw.g, raw.r);
    }
    return raw.rgb;
}

fn rgb_to_y_u8(rgb: vec3<f32>) -> u32 {
    let y_prime = 0.2126 * rgb.r + 0.7152 * rgb.g + 0.0722 * rgb.b;
    var y_val: f32;
    if (params.color_range == 0u) {
        y_val = 16.0 + 219.0 * y_prime;
        return u32(clamp(round(y_val), 16.0, 235.0));
    } else {
        y_val = 255.0 * y_prime;
        return u32(clamp(round(y_val), 0.0, 255.0));
    }
}

fn rgb_to_uv_u8(rgb: vec3<f32>) -> vec2<u32> {
    let y_prime = 0.2126 * rgb.r + 0.7152 * rgb.g + 0.0722 * rgb.b;
    let cb = (rgb.b - y_prime) / 1.8556;
    let cr = (rgb.r - y_prime) / 1.5748;
    if (params.color_range == 0u) {
        let u_val = clamp(round(128.0 + 224.0 * cb), 16.0, 240.0);
        let v_val = clamp(round(128.0 + 224.0 * cr), 16.0, 240.0);
        return vec2<u32>(u32(u_val), u32(v_val));
    } else {
        let u_val = clamp(round(128.0 + 255.0 * cb), 0.0, 255.0);
        let v_val = clamp(round(128.0 + 255.0 * cr), 0.0, 255.0);
        return vec2<u32>(u32(u_val), u32(v_val));
    }
}

@compute @workgroup_size(256, 1, 1)
fn cs_y_plane(@builtin(global_invocation_id) gid: vec3<u32>) {
    let total_pixels = params.width * params.height;
    let total_words = total_pixels / 4u;
    let word_idx = gid.x;
    if (word_idx >= total_words) {
        return;
    }

    let base_idx = word_idx * 4u;
    var packed: u32 = 0u;
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let p = base_idx + i;
        let px = p % params.width;
        let py = p / params.width;
        let y_byte = rgb_to_y_u8(load_rgb(px, py));
        packed = packed | ((y_byte & 0xFFu) << (i * 8u));
    }
    y_plane[word_idx] = packed;
}

@compute @workgroup_size(256, 1, 1)
fn cs_uv_plane(@builtin(global_invocation_id) gid: vec3<u32>) {
    let half_w = params.width / 2u;
    let half_h = params.height / 2u;
    let total_pairs = half_w * half_h;
    let total_words = (total_pairs + 1u) / 2u;
    let word_idx = gid.x;
    if (word_idx >= total_words) {
        return;
    }

    var packed: u32 = 0u;
    for (var i: u32 = 0u; i < 2u; i = i + 1u) {
        let pair_idx = word_idx * 2u + i;
        if (pair_idx < total_pairs) {
            let cx = pair_idx % half_w;
            let cy = pair_idx / half_w;
            let x0 = cx * 2u;
            let y0 = cy * 2u;
            let c00 = load_rgb(x0, y0);
            let c10 = load_rgb(x0 + 1u, y0);
            let c01 = load_rgb(x0, y0 + 1u);
            let c11 = load_rgb(x0 + 1u, y0 + 1u);
            let avg_rgb = 0.25 * (c00 + c10 + c01 + c11);
            let uv = rgb_to_uv_u8(avg_rgb);
            let shift = i * 16u;
            packed = packed | ((uv.x & 0xFFu) << shift) | ((uv.y & 0xFFu) << (shift + 8u));
        }
    }
    uv_plane[word_idx] = packed;
}
"#;

/// Cached GPU compute pipelines for converting `Bgra8Unorm` / `Rgba8Unorm`
/// frames into BT.709 NV12 bi-planar output.
#[derive(Debug)]
pub struct Nv12Converter {
    bind_group_layout: wgpu::BindGroupLayout,
    y_pipeline: wgpu::ComputePipeline,
    uv_pipeline: wgpu::ComputePipeline,
}

impl Nv12Converter {
    /// Compile the WGSL compute shader and cache the bind group layout and
    /// luma/chroma compute pipelines on `device`.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chromaticity-render nv12_convert wgsl"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(NV12_CONVERT_WGSL)),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("chromaticity-render nv12_convert bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chromaticity-render nv12_convert pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let y_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("chromaticity-render nv12_convert y_plane pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_y_plane"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let uv_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("chromaticity-render nv12_convert uv_plane pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_uv_plane"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self {
            bind_group_layout,
            y_pipeline,
            uv_pipeline,
        }
    }

    /// Convert a GPU `Bgra8Unorm` or `Rgba8Unorm` texture (with even dimensions)
    /// into CPU-accessible NV12 `Y` and `UV` byte planes.
    ///
    /// Note that `wgpu` hardware `textureLoad` automatically swizzles
    /// `Bgra8Unorm` textures into RGB order; use
    /// [`Self::convert_to_nv12_bytes_with_swap`] if raw BGRA bytes were
    /// uploaded into an `Rgba8Unorm` texture.
    ///
    /// # Errors
    ///
    /// - [`RenderError::TargetSizeInvalid`] if `width` or `height` is zero or odd.
    /// - [`RenderError::CameraFormatNotSupported`] if `src_texture.format()` is
    ///   not `Bgra8Unorm`, `Rgba8Unorm`, `Bgra8UnormSrgb`, or `Rgba8UnormSrgb`.
    /// - [`RenderError::DeviceLoss`] / [`RenderError::BufferMap`] on GPU readback failure.
    pub fn convert_to_nv12_bytes(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        src_texture: &wgpu::Texture,
        color_range: ColorRange,
    ) -> Result<Nv12Frame, RenderError> {
        self.convert_to_nv12_bytes_with_swap(device, queue, src_texture, color_range, false)
    }

    /// Convert a GPU texture into NV12 byte planes with explicit control over
    /// the shader's `swap_rb` uniform flag.
    ///
    /// # Errors
    ///
    /// Returns the same validation and GPU readback errors as
    /// [`Self::convert_to_nv12_bytes`].
    pub fn convert_to_nv12_bytes_with_swap(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        src_texture: &wgpu::Texture,
        color_range: ColorRange,
        swap_rb: bool,
    ) -> Result<Nv12Frame, RenderError> {
        let size = src_texture.size();
        let width = size.width;
        let height = size.height;

        validate_even_dimensions(width, height)?;
        validate_source_format(src_texture.format())?;

        let y_byte_len = (width as usize)
            .checked_mul(height as usize)
            .ok_or(RenderError::TargetSizeInvalid((width, height)))?;
        let uv_byte_len = ((width / 2) as usize)
            .checked_mul((height / 2) as usize)
            .and_then(|pairs| pairs.checked_mul(2))
            .ok_or(RenderError::TargetSizeInvalid((width, height)))?;

        // Storage buffers are u32-aligned; pad uv_byte_len up to a multiple of 4 bytes.
        let y_buf_size = y_byte_len as u64;
        let uv_buf_size = (uv_byte_len.div_ceil(4) * 4) as u64;

        let params = Nv12Params {
            width,
            height,
            color_range: color_range.as_u32(),
            swap_rb: u32::from(swap_rb),
        };
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chromaticity-render nv12_convert params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let y_storage = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chromaticity-render nv12_convert y_storage"),
            size: y_buf_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let uv_storage = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chromaticity-render nv12_convert uv_storage"),
            size: uv_buf_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let y_readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chromaticity-render nv12_convert y_readback"),
            size: y_buf_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let uv_readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chromaticity-render nv12_convert uv_readback"),
            size: uv_buf_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let src_view = src_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("chromaticity-render nv12_convert bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&src_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: y_storage.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: uv_storage.as_entire_binding(),
                },
            ],
        });

        let y_words = (y_byte_len / 4) as u32;
        let uv_pairs = (width / 2) * (height / 2);
        let uv_words = uv_pairs.div_ceil(2);

        let y_groups = y_words.div_ceil(WORKGROUP_SIZE);
        let uv_groups = uv_words.div_ceil(WORKGROUP_SIZE);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("chromaticity-render nv12_convert encoder"),
        });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("chromaticity-render nv12_convert compute pass"),
                timestamp_writes: None,
            });
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.set_pipeline(&self.y_pipeline);
            cpass.dispatch_workgroups(y_groups, 1, 1);
            cpass.set_pipeline(&self.uv_pipeline);
            cpass.dispatch_workgroups(uv_groups, 1, 1);
        }

        encoder.copy_buffer_to_buffer(&y_storage, 0, &y_readback, 0, y_buf_size);
        encoder.copy_buffer_to_buffer(&uv_storage, 0, &uv_readback, 0, uv_buf_size);

        queue.submit(Some(encoder.finish()));

        let y_plane = readback_buffer_bytes(device, &y_readback, y_byte_len)?;
        let uv_plane = readback_buffer_bytes(device, &uv_readback, uv_byte_len)?;

        Ok(Nv12Frame {
            width,
            height,
            color_range,
            y_plane,
            uv_plane,
        })
    }

    /// Upload tightly packed 4-bpp BGRA8 or RGBA8 pixels to a temporary GPU
    /// texture and convert them into NV12 byte planes.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::TargetSizeInvalid`] for zero/odd dimensions,
    /// [`RenderError::UniformBindingFailed`] if `pixels.len() != width * height * 4`,
    /// or any error from [`Self::convert_to_nv12_bytes`].
    pub fn convert_pixels_to_nv12_bytes(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pixels: &[u8],
        size: (u32, u32),
        format: wgpu::TextureFormat,
        color_range: ColorRange,
    ) -> Result<Nv12Frame, RenderError> {
        let (width, height) = size;
        validate_even_dimensions(width, height)?;
        validate_source_format(format)?;

        let expected_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|px| px.checked_mul(4))
            .ok_or(RenderError::TargetSizeInvalid((width, height)))?;
        if pixels.len() != expected_len {
            return Err(RenderError::UniformBindingFailed {
                pass_label: "nv12_convert".to_string(),
                name: "pixels".to_string(),
                reason: format!(
                    "pixel buffer length {} != expected {} bytes for {}x{} 4-bpp frame",
                    pixels.len(),
                    expected_len,
                    width,
                    height
                ),
            });
        }

        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("chromaticity-render nv12_convert source texture"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            extent,
        );

        self.convert_to_nv12_bytes(device, queue, &texture, color_range)
    }
}

fn validate_even_dimensions(width: u32, height: u32) -> Result<(), RenderError> {
    if width == 0 || height == 0 || width % 2 != 0 || height % 2 != 0 {
        return Err(RenderError::TargetSizeInvalid((width, height)));
    }
    Ok(())
}

fn validate_source_format(format: wgpu::TextureFormat) -> Result<(), RenderError> {
    match format {
        wgpu::TextureFormat::Bgra8Unorm
        | wgpu::TextureFormat::Rgba8Unorm
        | wgpu::TextureFormat::Bgra8UnormSrgb
        | wgpu::TextureFormat::Rgba8UnormSrgb => Ok(()),
        other => Err(RenderError::CameraFormatNotSupported {
            format: format!("{other:?}"),
            reason: "NV12 converter requires Bgra8Unorm or Rgba8Unorm source texture".to_string(),
        }),
    }
}

fn readback_buffer_bytes(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    byte_len: usize,
) -> Result<Vec<u8>, RenderError> {
    let slice = buffer.slice(..);
    let (tx, rx) = channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });

    loop {
        match device.poll(wgpu::PollType::Wait) {
            Ok(_) => {}
            Err(e) => return Err(RenderError::DeviceLoss(format!("{e:?}"))),
        }
        match rx.try_recv() {
            Ok(Ok(())) => break,
            Ok(Err(e)) => return Err(RenderError::BufferMap(format!("{e}"))),
            Err(TryRecvError::Empty) => continue,
            Err(TryRecvError::Disconnected) => {
                return Err(RenderError::BufferMap(
                    "NV12 readback map_async sender dropped before completion".to_string(),
                ));
            }
        }
    }

    let mapped = slice.get_mapped_range();
    let bytes = mapped[..byte_len].to_vec();
    drop(mapped);
    buffer.unmap();
    Ok(bytes)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod nv12_convert_tests {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — Phase 7a GPU BGRA/RGBA to BT.709
    //! NV12 compute pass verification (RENDER-NV12-001).

    use super::*;
    use crate::multipass::MultipassRenderer;

    /// Exact ITU-R BT.709 reference Y/U/V for normalized `(r, g, b)` in `[0, 1]`.
    fn bt709_reference(r: f32, g: f32, b: f32, range: ColorRange) -> (u8, u8, u8) {
        let y_prime = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let cb = (b - y_prime) / 1.8556;
        let cr = (r - y_prime) / 1.5748;
        match range {
            ColorRange::Limited => {
                let y = (16.0 + 219.0 * y_prime).round().clamp(16.0, 235.0) as u8;
                let u = (128.0 + 224.0 * cb).round().clamp(16.0, 240.0) as u8;
                let v = (128.0 + 224.0 * cr).round().clamp(16.0, 240.0) as u8;
                (y, u, v)
            }
            ColorRange::Full => {
                let y = (255.0 * y_prime).round().clamp(0.0, 255.0) as u8;
                let u = (128.0 + 255.0 * cb).round().clamp(0.0, 255.0) as u8;
                let v = (128.0 + 255.0 * cr).round().clamp(0.0, 255.0) as u8;
                (y, u, v)
            }
        }
    }

    fn solid_bgra(width: u32, height: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
        let count = (width * height) as usize;
        let mut out = Vec::with_capacity(count * 4);
        for _ in 0..count {
            out.extend_from_slice(&[b, g, r, 255]);
        }
        out
    }

    fn solid_rgba(width: u32, height: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
        let count = (width * height) as usize;
        let mut out = Vec::with_capacity(count * 4);
        for _ in 0..count {
            out.extend_from_slice(&[r, g, b, 255]);
        }
        out
    }

    fn assert_yuv_within_tol(label: &str, frame: &Nv12Frame, expected: (u8, u8, u8), tol: u8) {
        let expected_y_len = (frame.width * frame.height) as usize;
        let expected_uv_len = ((frame.width / 2) * (frame.height / 2) * 2) as usize;
        assert_eq!(frame.y_plane.len(), expected_y_len);
        assert_eq!(frame.uv_plane.len(), expected_uv_len);

        for (i, &y) in frame.y_plane.iter().enumerate() {
            assert!(
                y.abs_diff(expected.0) <= tol,
                "{label}: Y[{i}]={y}, expected {} ± {tol}",
                expected.0
            );
        }
        for (pair_idx, uv) in frame.uv_plane.chunks_exact(2).enumerate() {
            assert!(
                uv[0].abs_diff(expected.1) <= tol,
                "{label}: U[{pair_idx}]={}, expected {} ± {tol}",
                uv[0],
                expected.1
            );
            assert!(
                uv[1].abs_diff(expected.2) <= tol,
                "{label}: V[{pair_idx}]={}, expected {} ± {tol}",
                uv[1],
                expected.2
            );
        }
    }

    #[test]
    fn bt709_limited_and_full_primaries_bgra_within_2_lsb() {
        let renderer = MultipassRenderer::new().expect("wgpu Metal/Vulkan adapter required");
        let converter = Nv12Converter::new(renderer.device());
        let (w, h) = (16, 12);

        let primaries = [
            ("black", 0_u8, 0_u8, 0_u8),
            ("white", 255, 255, 255),
            ("red", 255, 0, 0),
            ("green", 0, 255, 0),
            ("blue", 0, 0, 255),
        ];

        for range in [ColorRange::Limited, ColorRange::Full] {
            for (name, r, g, b) in primaries {
                let bgra = solid_bgra(w, h, r, g, b);
                let nv12 = converter
                    .convert_pixels_to_nv12_bytes(
                        renderer.device(),
                        renderer.queue(),
                        &bgra,
                        (w, h),
                        wgpu::TextureFormat::Bgra8Unorm,
                        range,
                    )
                    .unwrap();

                let expected = bt709_reference(
                    f32::from(r) / 255.0,
                    f32::from(g) / 255.0,
                    f32::from(b) / 255.0,
                    range,
                );
                assert_yuv_within_tol(&format!("{name} ({range:?}) BGRA"), &nv12, expected, 2);
            }
        }
    }

    #[test]
    fn rgba8_and_swap_rb_uniform_flag_match_bt709_reference() {
        let renderer = MultipassRenderer::new().expect("wgpu Metal/Vulkan adapter required");
        let converter = Nv12Converter::new(renderer.device());
        let (w, h) = (8, 8);

        // 1. Native Rgba8Unorm pure red.
        let rgba_red = solid_rgba(w, h, 255, 0, 0);
        let nv12_rgba = converter
            .convert_pixels_to_nv12_bytes(
                renderer.device(),
                renderer.queue(),
                &rgba_red,
                (w, h),
                wgpu::TextureFormat::Rgba8Unorm,
                ColorRange::Limited,
            )
            .unwrap();
        assert_yuv_within_tol(
            "red RGBA8 Limited",
            &nv12_rgba,
            bt709_reference(1.0, 0.0, 0.0, ColorRange::Limited),
            2,
        );

        // 2. BGRA bytes uploaded into an Rgba8Unorm texture with swap_rb = true.
        let bgra_blue = solid_bgra(w, h, 0, 0, 255);
        let extent = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };
        let tex = renderer.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("nv12 swap_rb test"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        renderer.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bgra_blue,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            extent,
        );
        let nv12_swapped = converter
            .convert_to_nv12_bytes_with_swap(
                renderer.device(),
                renderer.queue(),
                &tex,
                ColorRange::Limited,
                true,
            )
            .unwrap();
        assert_yuv_within_tol(
            "blue swap_rb=true Limited",
            &nv12_swapped,
            bt709_reference(0.0, 0.0, 1.0, ColorRange::Limited),
            2,
        );
    }

    #[test]
    fn chroma_2x2_box_average_and_minimal_2x2_frame() {
        let renderer = MultipassRenderer::new().expect("wgpu Metal/Vulkan adapter required");
        let converter = Nv12Converter::new(renderer.device());

        // 2x2 frame: top-left & bottom-right pure red, top-right & bottom-left pure blue.
        // Box average RGB = (0.5, 0.0, 0.5).
        let pixels = vec![
            0, 0, 255, 255, // (0,0) red in BGRA
            255, 0, 0, 255, // (1,0) blue in BGRA
            255, 0, 0, 255, // (0,1) blue in BGRA
            0, 0, 255, 255, // (1,1) red in BGRA
        ];
        let nv12 = converter
            .convert_pixels_to_nv12_bytes(
                renderer.device(),
                renderer.queue(),
                &pixels,
                (2, 2),
                wgpu::TextureFormat::Bgra8Unorm,
                ColorRange::Limited,
            )
            .unwrap();

        let (y_red, _, _) = bt709_reference(1.0, 0.0, 0.0, ColorRange::Limited);
        let (y_blue, _, _) = bt709_reference(0.0, 0.0, 1.0, ColorRange::Limited);
        let (_, u_avg, v_avg) = bt709_reference(0.5, 0.0, 0.5, ColorRange::Limited);

        assert_eq!(nv12.y_plane.len(), 4);
        assert_eq!(nv12.uv_plane.len(), 2);
        assert!(nv12.y_plane[0].abs_diff(y_red) <= 2);
        assert!(nv12.y_plane[1].abs_diff(y_blue) <= 2);
        assert!(nv12.y_plane[2].abs_diff(y_blue) <= 2);
        assert!(nv12.y_plane[3].abs_diff(y_red) <= 2);
        assert!(nv12.uv_plane[0].abs_diff(u_avg) <= 2);
        assert!(nv12.uv_plane[1].abs_diff(v_avg) <= 2);

        let contiguous = nv12.into_contiguous_bytes();
        assert_eq!(contiguous.len(), 6);
    }

    #[test]
    fn rejects_odd_or_zero_dimensions_and_short_buffers() {
        let renderer = MultipassRenderer::new().expect("wgpu Metal/Vulkan adapter required");
        let converter = Nv12Converter::new(renderer.device());

        for bad in [(0, 4), (4, 0), (3, 4), (4, 5)] {
            let err = converter
                .convert_pixels_to_nv12_bytes(
                    renderer.device(),
                    renderer.queue(),
                    &[],
                    bad,
                    wgpu::TextureFormat::Bgra8Unorm,
                    ColorRange::Limited,
                )
                .unwrap_err();
            assert!(matches!(err, RenderError::TargetSizeInvalid(sz) if sz == bad));
        }

        let err_short = converter
            .convert_pixels_to_nv12_bytes(
                renderer.device(),
                renderer.queue(),
                &[0_u8; 8],
                (4, 4),
                wgpu::TextureFormat::Bgra8Unorm,
                ColorRange::Limited,
            )
            .unwrap_err();
        assert!(matches!(
            err_short,
            RenderError::UniformBindingFailed { .. }
        ));
    }
}
