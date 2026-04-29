// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Single-pass GLSL → SPIR-V → wgpu render entry point.
//!
//! Slice-1 ships [`compile_and_render_single_pass`]: take a hand-written
//! GLSL 4.50 fragment-shader source, validate it through naga's GLSL
//! frontend, lower to a wgpu shader module, mount it against a
//! hardcoded full-screen-triangle vertex shader, render to an
//! `RGBA8Unorm` offscreen target at the caller's chosen size, and
//! return the readback bytes (one row at a time, padding stripped).
//!
//! Design choices that may surprise a code reader:
//!
//! - **No surface, no window, no compositor.** The wgpu adapter is
//!   requested with `power_preference = LowPower` and a software
//!   fallback permitted (`force_fallback_adapter: true` is *not* set,
//!   but the slice does not require a discrete GPU). The slice-1
//!   harness runs in CI on Ubuntu under llvmpipe (Mesa software
//!   rasterizer over Vulkan); on macOS it runs against Metal; on
//!   Windows under Vulkan or DX12 — backend selection is left to
//!   wgpu's auto-discovery (`Backends::PRIMARY`).
//! - **RGBA8Unorm, not RGBA32F.** ADR-011 §Decision pins float targets
//!   for production rendering, but the slice-1 corpus asserts a stable
//!   FNV-1a hash over the readback bytes; an integer-format target
//!   keeps the hash bit-exact across backends without forcing the
//!   harness to compare floating-point pixel values for tolerance.
//!   Float-target coverage lands with the multi-pass scheduler.
//! - **`fullscreen_triangle` vertex shader is hardcoded GLSL.** Both
//!   stages are GLSL so the naga frontend produces matching
//!   interpolation-sampling on the linkage (a WGSL vertex stage's
//!   `@interpolate(perspective)` defaults to `center`, which mismatches
//!   the GLSL fragment's `sampling: None` and is rejected by the wgpu
//!   pipeline validator). See `FULLSCREEN_TRIANGLE_GLSL`'s doc comment
//!   for the failure mode that motivated the choice.
//! - **Sync wrapper around the wgpu async API.** [`pollster::block_on`]
//!   blocks the current thread on each of `Adapter::request_adapter`
//!   and `Adapter::request_device` independently (rather than a
//!   single `async {}` over the whole body). The map_async readback
//!   uses a `channel` + `device.poll(PollType::Wait)` loop so the
//!   crate itself never holds an `async` block — keeping the source
//!   well-formed under the standalone-rustfmt invocation in
//!   `lefthook.yml`'s pre-commit hook (which does not forward the
//!   workspace edition flag). The runtime-bus integration in a
//!   later slice will switch to a real executor; the slice-1
//!   surface is sync because the corpus harness drives it from
//!   `#[test]`.

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::mpsc::{TryRecvError, channel};

use bytemuck::{Pod, Zeroable};
use thiserror::Error;
use wgpu::util::DeviceExt;

/// Errors returned by [`compile_and_render_single_pass`].
///
/// Variants are ordered roughly by pipeline stage:
///
/// 1. Adapter / device acquisition ([`RenderError::AdapterUnavailable`],
///    [`RenderError::DeviceRequest`]).
/// 2. Shader compilation ([`RenderError::FragmentParse`],
///    [`RenderError::FragmentValidate`]).
/// 3. Buffer / texture allocation
///    ([`RenderError::TargetSizeInvalid`]).
/// 4. Submission and readback ([`RenderError::DeviceLoss`],
///    [`RenderError::BufferMap`]).
///
/// `#[non_exhaustive]` keeps slice-2+ free to add variants (e.g.
/// `MultipassDependencyCycle`) without a major-version bump on the
/// crate's error surface.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RenderError {
    /// `Instance::request_adapter` returned `None`. On Linux CI this
    /// usually means Mesa / Vulkan ICD is not installed; on macOS /
    /// Windows it indicates the system has no usable GPU at all.
    #[error("no wgpu adapter available (instance returned no candidates)")]
    AdapterUnavailable,

    /// `Adapter::request_device` failed. The wrapped string carries
    /// the wgpu-side diagnostic for triage.
    #[error("wgpu device request failed: {0}")]
    DeviceRequest(String),

    /// naga's GLSL frontend rejected the supplied fragment source
    /// (syntax, type, or version error). The wrapped string is the
    /// formatted parser diagnostic.
    #[error("GLSL fragment shader parse failed: {0}")]
    FragmentParse(String),

    /// naga validation rejected the parsed fragment IR (e.g. dangling
    /// resource binding, unsupported expression for our capability
    /// flags). The wrapped string is the formatted validator
    /// diagnostic.
    #[error("GLSL fragment shader validation failed: {0}")]
    FragmentValidate(String),

    /// Caller asked for a render target with zero width or zero
    /// height. The wrapped tuple is the offending `(width, height)`.
    #[error("render target size {0:?} invalid (width and height must both be > 0)")]
    TargetSizeInvalid((u32, u32)),

    /// Device was lost between submission and readback. The wrapped
    /// string carries wgpu's reason code if any was supplied.
    #[error("wgpu device lost during single-pass render: {0}")]
    DeviceLoss(String),

    /// `Buffer::map_async` failed or returned an error to the
    /// completion callback. The wrapped string is the wgpu
    /// `BufferAsyncError` rendering.
    #[error("readback buffer map failed: {0}")]
    BufferMap(String),

    // ----- Slice 2: multipass scheduler variants. -----
    /// [`crate::multipass::compile_and_render_multipass`] was called
    /// with an empty `passes` list. The scheduler refuses to allocate
    /// a zero-pass spec rather than returning an empty readback.
    #[error("multipass spec has no passes")]
    EmptyPassList,

    /// A multipass [`crate::multipass::PassDef`] referenced a source
    /// pass index that is at or beyond the current pass's index.
    /// Forward references and self-references are rejected to
    /// prevent feedback-edge cycles in slice 2 (full feedback
    /// support lands in slice 3).
    #[error(
        "multipass pass {pass_index} references source pass {source_pass_index} \
         (must be < {pass_index}; spec has {num_passes} passes total)"
    )]
    PassInputOutOfRange {
        /// Index of the pass declaring the offending input binding.
        pass_index: usize,
        /// Source pass index supplied in the binding (>= pass_index).
        source_pass_index: usize,
        /// Total number of passes in the spec, surfaced for triage.
        num_passes: usize,
    },

    /// A uniform value supplied via
    /// [`crate::multipass::UniformInput`] could not be marshalled
    /// into the per-pass uniform buffer. Slice 2's surface fires
    /// this on empty uniform names; slice 3 expands it to cover
    /// audio uniform binding edge cases.
    #[error("multipass uniform binding failed (pass={pass_label}, name={name}): {reason}")]
    UniformBindingFailed {
        /// Caller-supplied pass label for triage; `(unlabelled)`
        /// when the [`crate::multipass::PassDef::label`] is `None`.
        pass_label: String,
        /// Name of the uniform that failed to marshal.
        name: String,
        /// Diagnostic explanation.
        reason: String,
    },

    /// A per-pass render step inside the multipass scheduler failed.
    /// Wraps the original [`RenderError`] from the inner pass with
    /// the offending pass index for triage.
    #[error("multipass pass {pass_index} render failed: {source}")]
    PassRenderFailed {
        /// Index of the pass that failed.
        pass_index: usize,
        /// Original error from the inner pass.
        #[source]
        source: Box<RenderError>,
    },
}

/// GLSL source for the hardcoded full-screen triangle vertex shader.
///
/// Authored in GLSL 4.50 (rather than WGSL) so the naga frontend
/// produces the SAME interpolation-sampling shape on both stages —
/// `sampling: None` for an unqualified `out vec2 uv;` / `in vec2 uv;`.
/// WGSL's `@interpolate(perspective)` defaults to `center`, which
/// the wgpu pipeline validator rejects against the fragment shader's
/// `sampling: None` input (`Input sampling doesn't match provided
/// Some(Center)`). Authoring both stages in GLSL keeps the
/// interpolation contract consistent without requiring the user-
/// supplied fragment shader to add a non-default sampling qualifier.
///
/// One oversized triangle covers the entire (-1..1, -1..1) clip-space
/// quad. Vertex IDs map to UV coordinates (0,0), (2,0), (0,2); the
/// vertex shader projects UV to NDC via `pos = uv * 2 - 1`. The
/// off-screen vertices at (2,0) and (0,2) sit outside the viewport
/// so the rasterizer clips them. Pixels inside the texture see a
/// linear UV gradient in [0, 1].
///
/// `gl_Position.y` is negated so that uv = (0, 0) lands at the
/// upper-left of the rendered texture (matching the row-major
/// readback layout the caller observes after
/// `copy_texture_to_buffer`). Without the flip, uv = (0, 0) lands
/// at the lower-left and the corner-sentinel pin in the T2 corpus
/// reads inverted.
pub(crate) const FULLSCREEN_TRIANGLE_GLSL: &str = r#"
#version 450
layout(location = 0) out vec2 uv;
void main() {
    uint vid = uint(gl_VertexIndex);
    uv = vec2(float((vid << 1) & 2u), float(vid & 2u));
    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
    gl_Position.y = -gl_Position.y;
}
"#;

/// `Pod` bridge type for the readback buffer's row stride. Not
/// actually written to GPU memory; reserved for slice-2's vertex /
/// uniform buffer plumbing.
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)]
struct ReservedVertexLayout {
    _reserved: [f32; 4],
}

/// Compile `glsl_fragment_source` and render a single-pass full-screen
/// triangle into an offscreen `RGBA8Unorm` texture of `target_size`,
/// returning the readback bytes (4 bytes per pixel, row-major,
/// padding stripped).
///
/// The fragment shader's entry point MUST be named `main` and MUST
/// accept the interpolated `uv` varying spelled either as a free
/// `in vec2 uv;` declaration (GLSL 4.50 separable shader interface) or
/// as the location-qualified `layout(location = 0) in vec2 uv;` form.
/// The vertex shader writes to `@location(0)` (UV); naga's GLSL→IR
/// frontend resolves this to the matching IR location.
///
/// The fragment shader MUST write a single 4-component vector to
/// `out vec4 fragColor;` (any name; `layout(location = 0)`-qualified
/// or plain). The sole color attachment is at location 0.
///
/// # Errors
///
/// See [`RenderError`]. The naga frontend rejects unsupported GLSL
/// constructs (e.g. compute / geometry stages, `image` sampler types
/// outside ADR-011's 2D-sampler scope) at the parse / validate
/// stages with structured diagnostics rather than crashing the
/// adapter.
pub fn compile_and_render_single_pass(
    glsl_fragment_source: &str,
    target_size: (u32, u32),
) -> Result<Vec<u8>, RenderError> {
    if target_size.0 == 0 || target_size.1 == 0 {
        return Err(RenderError::TargetSizeInvalid(target_size));
    }

    // wgpu's request_adapter / map_async are async; pollster blocks
    // the current thread for the slice-1 sync surface. The body is
    // structured as a synchronous function with `pollster::block_on`
    // applied at each leaf future (rather than wrapping the whole
    // body in `async move {}` or factoring out an `async fn` helper)
    // because `lefthook.yml`'s pre-commit `rustfmt --style-edition=2024
    // --check` call does not forward `--edition=2024`, which makes
    // standalone rustfmt reject `async fn` and `async move {}` as
    // Rust-2015 syntax. Avoiding both keeps the lefthook hook
    // happy without bumping the edition flag in lefthook.yml (which
    // is on the serial-list per CLAUDE.md). `cargo fmt` would handle
    // either shape correctly, but we keep the slice friendly to the
    // local-hook path.
    // 1. Adapter + device acquisition.
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
        label: Some("chromaticity-render single-pass device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::default(),
    }))
    .map_err(|e| RenderError::DeviceRequest(format!("{e}")))?;

    let device = Arc::new(device);

    // 2. Compile the user's GLSL fragment shader through naga.
    let fragment_module = compile_glsl_module(glsl_fragment_source, naga::ShaderStage::Fragment)?;
    let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("chromaticity-render fragment (user GLSL)"),
        source: wgpu::ShaderSource::Naga(Cow::Owned(fragment_module)),
    });

    // 3. Hardcoded GLSL vertex shader (see FULLSCREEN_TRIANGLE_GLSL
    // doc comment for the GLSL-vs-WGSL choice rationale).
    let vertex_module = compile_glsl_module(FULLSCREEN_TRIANGLE_GLSL, naga::ShaderStage::Vertex)?;
    let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("chromaticity-render vertex (fullscreen triangle)"),
        source: wgpu::ShaderSource::Naga(Cow::Owned(vertex_module)),
    });

    // 4. Render target + readback buffer allocation.
    let color_format = wgpu::TextureFormat::Rgba8Unorm;
    let bytes_per_pixel: u32 = 4;
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("chromaticity-render single-pass target"),
        size: wgpu::Extent3d {
            width: target_size.0,
            height: target_size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: color_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    // wgpu requires the readback buffer's row stride be a multiple of
    // COPY_BYTES_PER_ROW_ALIGNMENT (256). `padded_bytes_per_row` is
    // the on-buffer stride; `unpadded_bytes_per_row` is the actual
    // per-row payload that the caller receives.
    let unpadded_bytes_per_row = target_size.0 * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
    let readback_size = u64::from(padded_bytes_per_row) * u64::from(target_size.1);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("chromaticity-render readback"),
        size: readback_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Reserved vertex buffer to prove the bytemuck pipeline compiles
    // — slice-2 will replace this with a real per-vertex layout.
    let _reserved_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("chromaticity-render reserved vertex buffer"),
        contents: bytemuck::cast_slice(&[ReservedVertexLayout::default(); 3]),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // 5. Pipeline layout (no bind groups in slice-1) and pipeline.
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("chromaticity-render single-pass layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("chromaticity-render single-pass pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vertex_shader,
            // Both stages are GLSL; naga lowers `void main()` to an
            // entry point named `main` regardless of `ShaderStage`.
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

    // 6. Encode + submit.
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("chromaticity-render single-pass encoder"),
    });
    {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("chromaticity-render single-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
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
        rpass.draw(0..3, 0..1);
    }

    // Texture → buffer copy. Row stride must be 256-aligned per wgpu.
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(target_size.1),
            },
        },
        wgpu::Extent3d {
            width: target_size.0,
            height: target_size.1,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(Some(encoder.finish()));

    // 7. Map readback buffer and strip row padding.
    let buffer_slice = readback.slice(..);
    let (sender, receiver) = channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });

    // Poll until either the map callback fires or the device is lost.
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
        Vec::with_capacity((unpadded_bytes_per_row as usize) * (target_size.1 as usize));
    for row in 0..target_size.1 {
        let start = (row * padded_bytes_per_row) as usize;
        let end = start + unpadded_bytes_per_row as usize;
        pixels.extend_from_slice(&mapped[start..end]);
    }
    drop(mapped);
    readback.unmap();

    Ok(pixels)
}

/// Compile a GLSL 4.50 source through naga at `stage`, returning
/// the validated IR module ready to feed
/// [`wgpu::ShaderSource::Naga`].
///
/// The validator runs with `ValidationFlags::all()` and shader
/// capabilities matching wgpu's downlevel defaults — slice-1 does
/// not mount any non-default capability (no compute, no float64, no
/// non-2D samplers per ADR-011 §Decision). The fragment-vs-vertex
/// stage is the only knob the slice-1 surface exposes; later slices
/// will widen capabilities for feedback samplers and audio uniform
/// buffers.
///
/// Only the [`naga::ShaderStage::Fragment`] error variants
/// ([`RenderError::FragmentParse`] / [`RenderError::FragmentValidate`])
/// are populated for now. The hardcoded vertex shader is authored
/// in-crate (`FULLSCREEN_TRIANGLE_GLSL`) and is therefore not
/// expected to fail; if it does, the failure surfaces as a fragment
/// error string with `(stage=Vertex)` prefix to aid triage. Slice 2
/// adds a [`RenderError::VertexParse`] variant when user-supplied
/// vertex shaders enter the surface.
///
/// Visibility is `pub(crate)` so the slice-2 multipass scheduler in
/// `crate::multipass` can reuse the same compile path (uniform
/// per-pass GLSL toolchain across slice-1 and slice-2 entry points).
pub(crate) fn compile_glsl_module(
    source: &str,
    stage: naga::ShaderStage,
) -> Result<naga::Module, RenderError> {
    let stage_label = match stage {
        naga::ShaderStage::Vertex => "Vertex",
        naga::ShaderStage::Fragment => "Fragment",
        naga::ShaderStage::Compute => "Compute",
        naga::ShaderStage::Task => "Task",
        naga::ShaderStage::Mesh => "Mesh",
    };

    let mut frontend = naga::front::glsl::Frontend::default();
    let module = frontend
        .parse(
            &naga::front::glsl::Options {
                stage,
                defines: Default::default(),
            },
            source,
        )
        .map_err(|e| RenderError::FragmentParse(format!("(stage={stage_label}) {e:?}")))?;

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator
        .validate(&module)
        .map_err(|e| RenderError::FragmentValidate(format!("(stage={stage_label}) {e:?}")))?;

    Ok(module)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod compile_drift_pins {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — naga GLSL frontend drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins the
    //! naga compile path against the slice-1 entry-point contract
    //! ([`super::compile_glsl_module`]):
    //!
    //! - a minimal valid 4.50 fragment shader compiles and validates,
    //! - a syntactically broken fragment shader rejects with
    //!   [`RenderError::FragmentParse`] (NOT a panic, NOT a wgpu-stage
    //!   error),
    //! - a `target_size` with a zero dimension rejects with
    //!   [`RenderError::TargetSizeInvalid`] before any wgpu state is
    //!   touched (the corpus T2 in `engine/render/tests/single_pass.rs`
    //!   would otherwise mask this with a wgpu-stage failure on
    //!   adapter-less CI runners).
    //!
    //! The full single-pass pipeline (adapter + device + GPU work) is
    //! covered by the T2 corpus suite under
    //! `engine/render/tests/single_pass.rs`; T1 here is naga-only so
    //! the drift pin survives even on a runner without a usable wgpu
    //! adapter.

    use super::{RenderError, compile_and_render_single_pass, compile_glsl_module};

    const TRIVIAL_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() {
    fragColor = vec4(uv, 0.0, 1.0);
}
"#;

    const BROKEN_FRAGMENT: &str = r#"
#version 450
layout(location = 0) out vec4 fragColor;
void main() {
    fragColor = NOT_A_VALID_TOKEN;
}
"#;

    #[test]
    fn trivial_fragment_compiles() {
        let module = compile_glsl_module(TRIVIAL_FRAGMENT, naga::ShaderStage::Fragment)
            .expect("valid GLSL 4.50 UV-passthrough must compile");
        assert!(
            !module.entry_points.is_empty(),
            "naga module should expose at least the fragment entry point",
        );
    }

    #[test]
    fn broken_fragment_returns_parse_error() {
        let err = compile_glsl_module(BROKEN_FRAGMENT, naga::ShaderStage::Fragment)
            .expect_err("undefined identifier must fail naga frontend");
        assert!(
            matches!(err, RenderError::FragmentParse(_)),
            "broken GLSL must surface as FragmentParse, got {err:?}",
        );
    }

    #[test]
    fn fullscreen_triangle_vertex_compiles() {
        let module =
            compile_glsl_module(super::FULLSCREEN_TRIANGLE_GLSL, naga::ShaderStage::Vertex)
                .expect("hardcoded fullscreen-triangle vertex must compile");
        assert!(
            !module.entry_points.is_empty(),
            "vertex module should expose at least one entry point",
        );
    }

    #[test]
    fn zero_target_size_rejects_before_wgpu() {
        let err = compile_and_render_single_pass(TRIVIAL_FRAGMENT, (0, 16))
            .expect_err("zero width must reject");
        assert!(
            matches!(err, RenderError::TargetSizeInvalid((0, 16))),
            "zero width must surface as TargetSizeInvalid((0, 16)), got {err:?}",
        );

        let err = compile_and_render_single_pass(TRIVIAL_FRAGMENT, (16, 0))
            .expect_err("zero height must reject");
        assert!(
            matches!(err, RenderError::TargetSizeInvalid((16, 0))),
            "zero height must surface as TargetSizeInvalid((16, 0)), got {err:?}",
        );
    }
}
