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
//!   scalar / vector value occupies a 16-byte slot regardless of
//!   underlying width (a `f32` lives in the first 4 bytes of its
//!   slot, the remaining 12 bytes are pad). [`UniformValue::FloatArray`]
//!   expands into N consecutive 16-byte slots (one per element),
//!   matching std140's `uniform float arr[N]` layout where every
//!   array element is vec4-aligned. This keeps std140 vec4-alignment
//!   satisfied without a per-type packing rulebook; the cost is at
//!   most 12 bytes of pad per scalar / per array element, which is
//!   negligible at the uniform counts we expect. The slice-5
//!   follow-up to slice-5a added [`UniformValue::FloatArray`] to
//!   close the slice-5a transitional `chroma_AudioFFTLow` +
//!   `chroma_AudioFFTHigh` split-vec4 spelling into a single
//!   `chroma_AudioFFT[8]` array binding (`docs/36` Sec.2.11; catalog
//!   v1.3.0); the `syn_RES`-style vec2-batched widening remains
//!   deferred until a concrete consumer surfaces it.
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
//! # Slice-4 deltas (this PR)
//!
//! Slice 4 widens the slice-2 surface in three additive directions
//! while preserving every slice-2 byte-hash:
//!
//! 1. **Feedback-texture sampling.** A pass with
//!    [`PassDef::feedback_self`] = `true` samples its OWN previous-
//!    frame output via implicitly-wired ping-pong textures. The
//!    sampler binds at the LAST `1 + 2*i` slot in the pass's input
//!    binding table, AFTER any explicit prior-pass bindings. Two
//!    textures alternate per frame: on frame `K` the pass writes
//!    target `K % 2` and reads texture `(K + 1) % 2`. Frame `0`'s
//!    read sees a black-cleared texture (the `LoadOp::Clear` invoked
//!    on first allocation), which is the well-defined "cold start"
//!    state. Self-reference via [`PassInputBinding`] still rejects
//!    with [`RenderError::PassInputOutOfRange`]; feedback wiring
//!    goes through the dedicated `feedback_self` flag instead.
//!
//! 2. **Float targets.** [`PassDef::target_format`] selects the
//!    per-pass color format from [`TargetFormat`]: the slice-2
//!    default `Rgba8Unorm`, plus `Rgba16Float` and `Rgba32Float`
//!    per `docs/adr/ADR-011-rendering-backend.md` §Decision. The
//!    final-pass readback resolves to a stable byte representation
//!    regardless of format; the byte-hash captures the precision
//!    delta. Adapters lacking `RENDER_ATTACHMENT` for the requested
//!    float format surface as
//!    [`RenderError::FloatTargetUnsupported`].
//!
//! 3. **Multi-frame execution.** [`MultipassSpec::frame_count`]
//!    drives the scheduler over N consecutive frames before
//!    returning the final frame's readback. With `frame_count == 1`
//!    (the slice-2 default) the behavior is bit-identical to the
//!    slice-2 corpus. The multi-frame loop is the entry-point's
//!    feedback driver; callers wanting per-frame inspection drive
//!    [`compile_and_render_multipass`] manually with
//!    `frame_count = 1` and external state (out of slice-4 scope).
//!
//! Plus the optional `scene-adapter` feature
//! ([`crate::scene_adapter`]) producing a [`MultipassSpec`] from
//! `chromaticity_scene::Scene`.
//!
//! # Still deferred (not in this slice)
//!
//! - Transition orchestration (warm-switch between scene graphs) —
//!   ADR-007 follow-up slice.
//! - Audio uniform binding (`syn_BPM`, `syn_LEVEL`, `audio[]` 1D
//!   sampler) — `engine/audio` integration slice.
//! - DX12 backend (Tauri 2 + windows-rs version conflict; tracked in
//!   `engine/render/Cargo.toml`).
//! - Per-pass `WIDTH` / `HEIGHT` extension overrides — slice-4 keeps
//!   one shared target size; the `scene_adapter` reads the per-pass
//!   `WIDTH` / `HEIGHT` extensions but folds them into `target_size`
//!   conservatively (see `scene_adapter.rs` doc).
//! - Per-pass sampler `WRAP` / `FILTER` extension overrides —
//!   slice-4 still uses the shared nearest / clamp sampler.

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::mpsc::{TryRecvError, channel};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::single_pass::{FULLSCREEN_TRIANGLE_GLSL, RenderError, compile_glsl_module};

/// Plain-data scheduler input.
///
/// Constructed by callers (or, with the `scene-adapter` feature
/// enabled, by [`crate::scene_adapter`] over
/// `chromaticity_scene::Scene`). Slice-2 kept this surface free of
/// a `PassSpec` dependency; slice-4 preserves that invariant — the
/// adapter is in a separate module gated behind a cargo feature.
#[derive(Debug, Clone)]
pub struct MultipassSpec {
    /// Ordered pass list. Pass `i` may sample any pass at index `< i`
    /// via [`PassInputBinding`], and may additionally sample its OWN
    /// previous-frame output via [`PassDef::feedback_self`].
    pub passes: Vec<PassDef>,
    /// Final-pass render target dimensions (width, height). All
    /// intermediate passes also use this size in slice 4; per-pass
    /// `WIDTH` / `HEIGHT` extension overrides remain deferred (see
    /// module-level "Still deferred" list).
    pub target_size: (u32, u32),
    /// Number of consecutive frames to execute before returning the
    /// FINAL frame's readback. `0` rejects with
    /// [`RenderError::EmptyPassList`] (treated as a structural error
    /// — there is nothing to render). `1` reproduces the slice-2
    /// single-frame behavior bit-identically. Higher values drive
    /// feedback passes; their ping-pong textures alternate per
    /// frame so frame `K` reads texture `(K + 1) % 2` and writes
    /// texture `K % 2`.
    pub frame_count: u32,
}

impl Default for MultipassSpec {
    /// Slice-4 default mirrors the slice-2 expectations: empty pass
    /// list, zero target size (which the scheduler rejects), and
    /// `frame_count = 1` so a default-constructed-then-populated
    /// spec behaves like the slice-2 single-frame semantics until
    /// the caller explicitly opts into multi-frame feedback.
    fn default() -> Self {
        Self {
            passes: Vec::new(),
            target_size: (0, 0),
            frame_count: 1,
        }
    }
}

/// Per-pass color-attachment format for the multipass scheduler.
///
/// Slice-4 widens the slice-2 hardcoded `Rgba8Unorm` to cover the
/// ADR-011 §Decision float targets. The slice-2 corpus continuity
/// invariant is preserved by keeping `Rgba8Unorm` as the
/// [`PassDef::target_format`] default — every existing slice-2
/// fixture renders against the same backend bytes it always did.
///
/// The float variants require the wgpu adapter to advertise the
/// `RENDER_ATTACHMENT` flag for the chosen format; on adapters that
/// don't, [`compile_and_render_multipass`] returns
/// [`RenderError::FloatTargetUnsupported`] without ever entering the
/// pipeline-build stage.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum TargetFormat {
    /// 8-bit unsigned-normalized per channel. Default, matches the
    /// slice-1 and slice-2 corpus byte-format.
    #[default]
    Rgba8Unorm,
    /// 16-bit half-float per channel. Quad memory of `Rgba8Unorm`,
    /// extended dynamic range, byte-deterministic across runs of
    /// the same backend (16-bit float bit patterns are stable).
    Rgba16Float,
    /// 32-bit float per channel. Highest precision; corpus tests
    /// using `Rgba32Float` MUST quantize floats before hashing OR
    /// rely on corner-sentinel-only invariants — adapter rounding
    /// of f32 trailing bits is not bit-stable across backends.
    Rgba32Float,
}

impl TargetFormat {
    /// Map to the wgpu enum.
    pub(crate) fn to_wgpu(self) -> wgpu::TextureFormat {
        match self {
            TargetFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
            TargetFormat::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
            TargetFormat::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
        }
    }

    /// Bytes per pixel for the readback row stride. RGBA at 1 / 2 / 4
    /// bytes per channel.
    ///
    /// Slice-5b.3 retired the production callers (the readback path
    /// now derives the row stride from `wgpu::TextureFormat::block_copy_size`
    /// directly, which keeps the readback shape locked to the
    /// returned texture rather than the slice). The accessor is kept
    /// for the T1 drift pin
    /// [`multipass_drift_pins::target_format_bytes_per_pixel_layout`]
    /// which guards the slice-5b.2 byte-shape contract.
    #[allow(dead_code)]
    pub(crate) fn bytes_per_pixel(self) -> u32 {
        match self {
            TargetFormat::Rgba8Unorm => 4,
            TargetFormat::Rgba16Float => 8,
            TargetFormat::Rgba32Float => 16,
        }
    }

    /// Human-readable label for diagnostics.
    pub(crate) fn label(self) -> &'static str {
        match self {
            TargetFormat::Rgba8Unorm => "Rgba8Unorm",
            TargetFormat::Rgba16Float => "Rgba16Float",
            TargetFormat::Rgba32Float => "Rgba32Float",
        }
    }

    /// `true` if a `TextureSampleType::Float { filterable: true }`
    /// is acceptable for sampling. `Rgba32Float` is non-filterable
    /// on most adapters by default; we always use the non-filterable
    /// sample type for safety.
    pub(crate) fn _is_float(self) -> bool {
        matches!(self, TargetFormat::Rgba16Float | TargetFormat::Rgba32Float)
    }
}

/// Single-pass definition inside a [`MultipassSpec`].
#[derive(Debug, Default, Clone)]
pub struct PassDef {
    /// Optional debug label propagated into wgpu object labels.
    pub label: Option<String>,
    /// User-authored GLSL 4.50 fragment shader source. The shader's
    /// uniform / texture / sampler binding layout MUST match the
    /// per-pass scheme described in the module-level doc (uniforms
    /// at `set = 0, binding = 0`, input textures at `1 + 2*i`,
    /// input samplers at `1 + 2*i + 1`).
    ///
    /// When [`Self::feedback_self`] is `true`, the previous-frame
    /// self-feedback texture binds at the slot AFTER the explicit
    /// `input_textures`, i.e. at `1 + 2 * input_textures.len()` /
    /// `1 + 2 * input_textures.len() + 1`.
    pub fragment_glsl: String,
    /// Scalar / vector uniforms packed into the pass's uniform buffer
    /// in declaration order. Skip if the pass has no uniforms.
    pub uniforms: Vec<UniformInput>,
    /// References to PRIOR passes (`source_pass_index < this pass's
    /// index`) whose output texture should be exposed to the
    /// fragment shader as a `texture2D` + `sampler` pair.
    pub input_textures: Vec<PassInputBinding>,
    /// Color-attachment format for this pass's render target. Default
    /// is [`TargetFormat::Rgba8Unorm`] for slice-2 corpus continuity.
    /// See [`TargetFormat`] for the float-target options.
    pub target_format: TargetFormat,
    /// `true` if this pass samples its OWN previous-frame output
    /// (the "feedback" pattern for live-visuals trails / decay /
    /// recursive convolution). Slice-4 wires the ping-pong texture
    /// implicitly: callers do NOT add a [`PassInputBinding`] for
    /// self-reference (which would still reject as
    /// [`RenderError::PassInputOutOfRange`]); they set this flag
    /// instead. Frame 0 reads a black-cleared texture; subsequent
    /// frames read the texture this pass wrote on the previous
    /// frame.
    pub feedback_self: bool,
}

/// One named uniform value.
#[derive(Debug, Clone)]
pub struct UniformInput {
    /// Diagnostic name only — the binding position is determined by
    /// declaration order in [`PassDef::uniforms`], not by name.
    pub name: String,
    pub value: UniformValue,
}

/// Scalar / vector / array uniform value.
///
/// Most variants occupy one 16-byte slot in the per-pass uniform
/// buffer regardless of the underlying width (see module-level doc
/// for the std140-friendly packing choice). The [`UniformValue::FloatArray`]
/// variant is an exception: it expands into N consecutive 16-byte
/// slots (one per element), matching the std140 layout of
/// `uniform float arr[N]` where every array element is vec4-aligned.
#[derive(Debug, Clone)]
pub enum UniformValue {
    Scalar(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Int(i32),
    UInt(u32),
    /// std140-packed float array. Each element occupies one 16-byte
    /// slot in the per-pass uniform buffer (the first 4 bytes hold
    /// the f32, the remaining 12 bytes are zero pad). Authors declare
    /// the binding in GLSL as `uniform float arr[N]`; the per-pass
    /// uniform buffer's slot count grows by `N` per FloatArray
    /// binding. Introduced by the slice-5 follow-up to slice-5a to
    /// consolidate the transitional `chroma_AudioFFTLow` +
    /// `chroma_AudioFFTHigh` split-vec4 binding into a single
    /// `chroma_AudioFFT[8]` array uniform per `docs/36` Sec.2.11.
    FloatArray(Vec<f32>),
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

/// Long-lived render context owning the wgpu Instance / Adapter /
/// Device / Queue plus the cached fullscreen-triangle vertex shader
/// module.
///
/// The slice-5b.2+ refactor (this struct) extracts the device-
/// acquisition block from the prior per-call entry point so callers
/// that render multiple frames can amortize Instance / Adapter /
/// Device creation across calls. The companion shim
/// [`compile_and_render_multipass`] keeps the prior free-function
/// signature working for callers (e.g.
/// [`crate::composition::compose_transition_frame`]) that have not
/// yet migrated to the long-lived shape; that shim builds a fresh
/// `MultipassRenderer` per call so its behavior is bit-identical to
/// the pre-refactor function.
///
/// # Why this exists
///
/// The slice-5b.2 [`crate::composition`] module documents the
/// "renderer is its own GPU lifetime" invariant — the orchestrator
/// could not share a device with the multipass scheduler because the
/// scheduler acquired one per call. Slice 5b.3 (a separate follow-up
/// PR that USES this refactor) lifts the CPU-side composition byte
/// blend onto the GPU; that requires a long-lived device that both
/// the current-spec render and the next-spec render can submit work
/// to without re-acquisition. This struct is the architectural
/// prerequisite — it widens the device-ownership model the slice-5b.2
/// caveat described.
///
/// # Bit-identity invariant
///
/// Calling [`MultipassRenderer::new`] then [`MultipassRenderer::render`]
/// produces the exact same bytes as the prior per-call entry point
/// for any [`MultipassSpec`]. Two consecutive `render` calls with the
/// same spec produce byte-identical outputs (the device is reused;
/// per-spec textures, bind groups, pipelines, and the readback buffer
/// are rebuilt each call). All slice-1 / slice-2 / slice-4 / slice-5
/// `T2` byte-hash corpora pass without modification.
///
/// # Resource caching policy
///
/// - **Cached for the renderer's lifetime**: the wgpu Instance,
///   Adapter, Device, Queue, and the compiled fullscreen-triangle
///   vertex shader module (the `FULLSCREEN_TRIANGLE_GLSL` constant
///   never changes — recompiling it per render was waste).
/// - **Rebuilt per render() call**: every per-spec resource — render
///   targets, samplers, bind group layouts, fragment shaders, pipeline
///   layouts, render pipelines, uniform buffers, bind groups,
///   readback buffer. This keeps the slice-2 byte-identity invariant
///   (the pipeline order is unchanged) and avoids carrying a per-spec
///   cache that would need invalidation logic; pipeline-cache
///   amortization is its own future slice.
pub struct MultipassRenderer {
    /// wgpu Instance owning the backend dispatch table. Held for the
    /// renderer's lifetime so the Adapter remains valid.
    _instance: wgpu::Instance,
    /// wgpu Adapter for capability queries (float-target
    /// `RENDER_ATTACHMENT` checks per render).
    adapter: wgpu::Adapter,
    /// Long-lived `wgpu::Device`. Slice 5b.3 will share this with the
    /// composition pass via [`MultipassRenderer::device`].
    device: Arc<wgpu::Device>,
    /// Long-lived `wgpu::Queue`. Slice 5b.3 will submit composition
    /// command encoders against this queue rather than the per-call
    /// queue the shim builds.
    queue: wgpu::Queue,
    /// Cached fullscreen-triangle vertex shader module. Compiled
    /// once at `new()` time from `FULLSCREEN_TRIANGLE_GLSL`; reused
    /// across every `render()` call. The GLSL source is a runtime
    /// constant so the compiled module is always equivalent — no
    /// invalidation needed.
    vertex_shader: wgpu::ShaderModule,
}

impl MultipassRenderer {
    /// Build a renderer with the chromaticity-default Instance /
    /// Adapter / Device. Mirrors the wgpu descriptors the prior
    /// per-call entry point used so rendered output is bit-identical
    /// across the refactor.
    ///
    /// # Errors
    ///
    /// - [`RenderError::AdapterUnavailable`] — no usable wgpu adapter
    ///   was found (headless CI without a backend, missing GPU
    ///   driver, etc.).
    /// - [`RenderError::DeviceRequest`] — the adapter rejected the
    ///   device descriptor.
    /// - [`RenderError::FragmentParse`] / [`RenderError::FragmentValidate`] —
    ///   the `FULLSCREEN_TRIANGLE_GLSL` vertex shader (a runtime
    ///   constant) failed the naga frontend; surfaces as an
    ///   in-source-only failure if a future workspace upgrade breaks
    ///   the shader. Not reachable on the current source.
    pub fn new() -> Result<Self, RenderError> {
        // Adapter + device acquisition. See `single_pass.rs` lines
        // 193-205 for the in-source explanation of why each leaf
        // future is wrapped with `pollster::block_on` rather than the
        // body being declared `async`.
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
        });

        // Adapter acquisition routes through the deterministic
        // [`crate::adapter_selection`] policy per ADR-011 §7.
        // Wave 4d.2 Lane B PR 1 wires the default (high-performance,
        // no operator override, no battery flag) policy here; a
        // future widening that exposes a `MultipassRenderer::with_policy`
        // builder method threading the caller's persisted override is
        // post-PR-2 scope. The byte-identity contract on existing T2
        // corpora is preserved by routing through the same
        // `Backends::PRIMARY` filter the prior `request_adapter` call
        // used and by the fact that single-adapter hosts (the only
        // class the existing T2 corpora exercise) yield the same
        // selection under either path.
        let policy = crate::adapter_selection::GpuSelectionPolicy::new();
        let (_chosen_summary, adapter) =
            crate::adapter_selection::select_adapter(&instance, wgpu::Backends::PRIMARY, &policy)
                .ok_or(RenderError::AdapterUnavailable)?;

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("chromaticity-render multipass device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::default(),
        }))
        .map_err(|e| RenderError::DeviceRequest(format!("{e}")))?;
        let device = Arc::new(device);

        // Compile the shared full-screen-triangle vertex shader once
        // at construction. The GLSL source is a runtime constant so
        // the resulting module is reusable across every `render()`
        // call — caching it here is a pure performance win with no
        // bit-identity cost (the same naga IR / wgpu module is what
        // the per-call form would have produced).
        let vertex_module =
            compile_glsl_module(FULLSCREEN_TRIANGLE_GLSL, naga::ShaderStage::Vertex)?;
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chromaticity-render multipass vertex (shared fullscreen triangle)"),
            source: wgpu::ShaderSource::Naga(Cow::Owned(vertex_module)),
        });

        Ok(Self {
            _instance: instance,
            adapter,
            device,
            queue,
            vertex_shader,
        })
    }

    /// Long-lived `Arc<wgpu::Device>` shared by every
    /// [`MultipassRenderer::render`] call this renderer makes.
    /// Slice 5b.3 will use this accessor to submit a GPU-side
    /// composition pass against the same device that just rendered
    /// the source frames, eliminating the slice-5b.2 CPU-side blend.
    pub fn device(&self) -> &Arc<wgpu::Device> {
        &self.device
    }

    /// Long-lived `wgpu::Queue` shared by every
    /// [`MultipassRenderer::render`] call this renderer makes.
    /// Slice 5b.3 will submit composition command encoders against
    /// this queue rather than building a fresh one per composition
    /// call.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Compile every pass in `spec` and render them sequentially
    /// across `spec.frame_count` frames, returning the FINAL frame's
    /// FINAL pass readback bytes
    /// (`target_format.bytes_per_pixel()` per pixel, row-major,
    /// padding stripped — same shape as
    /// [`crate::compile_and_render_single_pass`] for `Rgba8Unorm`).
    ///
    /// Single-frame `frame_count = 1` plus all-`Rgba8Unorm` passes
    /// plus no `feedback_self` passes is bit-identical to slice-2
    /// behavior. Calling `render` twice with the same spec produces
    /// byte-identical outputs (the device is reused; only per-spec
    /// resources are rebuilt).
    ///
    /// # Errors
    ///
    /// Same error variants as the prior per-call entry point — see
    /// [`compile_and_render_multipass`] for the exhaustive list.
    pub fn render(&self, spec: &MultipassSpec) -> Result<Vec<u8>, RenderError> {
        let final_target = render_multipass_to_texture_with(
            &self.adapter,
            &self.device,
            &self.queue,
            &self.vertex_shader,
            spec,
        )?;
        readback_texture(&self.device, &self.queue, &final_target)
    }

    /// Compile every pass in `spec` and render them sequentially
    /// across `spec.frame_count` frames, returning the FINAL frame's
    /// FINAL-pass GPU texture WITHOUT a CPU readback. The returned
    /// texture is owned by the caller, sized to `spec.target_size`,
    /// formatted to the final pass's `target_format`, with usage flags
    /// `RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_SRC` so the caller
    /// may sample it in a downstream pipeline (slice-5b.3 GPU-side
    /// composition) or copy it back to a buffer.
    ///
    /// `render_to_texture` plus a caller-side readback round-trip is
    /// byte-identical to [`MultipassRenderer::render`]. Slice-5b.3's
    /// composition path uses `render_to_texture` for current + next
    /// scenes and samples both textures in a third blend pass without
    /// the per-spec readback round-trip the slice-5b.2 CPU path
    /// required.
    ///
    /// # Errors
    ///
    /// Same error variants as [`MultipassRenderer::render`] except
    /// the readback failures — see [`compile_and_render_multipass`]
    /// for the exhaustive list. Readback-stage variants
    /// ([`RenderError::DeviceLoss`], [`RenderError::BufferMap`])
    /// cannot fire on this entry point; they live in the
    /// caller-side readback (or in [`MultipassRenderer::render`]).
    pub fn render_to_texture(&self, spec: &MultipassSpec) -> Result<wgpu::Texture, RenderError> {
        render_multipass_to_texture_with(
            &self.adapter,
            &self.device,
            &self.queue,
            &self.vertex_shader,
            spec,
        )
    }

    /// Compile a single-pass GLSL 4.50 fragment shader once into a
    /// [`CachedSinglePass`] whose pipeline, uniform buffer, offscreen
    /// `Rgba8Unorm` target, and staging readback buffer are reused
    /// across frames.
    ///
    /// # Errors
    ///
    /// - [`RenderError::TargetSizeInvalid`] — width or height is zero.
    /// - [`RenderError::UniformBindingFailed`] — uniform encoding failed.
    /// - [`RenderError::PassRenderFailed`] — GLSL parse / validation failed.
    pub fn compile_cached_single_pass(
        &self,
        label: &str,
        fragment_glsl: &str,
        initial_uniforms: &[UniformInput],
        target_size: (u32, u32),
    ) -> Result<CachedSinglePass, RenderError> {
        if target_size.0 == 0 || target_size.1 == 0 {
            return Err(RenderError::TargetSizeInvalid(target_size));
        }

        let mut bgl_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();
        let (uniform_buffer, uniform_slot_count) = if initial_uniforms.is_empty() {
            (None, 0)
        } else {
            let slots = encode_uniform_slots(initial_uniforms, label)?;
            let count = slots.len();
            let buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("chromaticity-render cached uniform buffer"),
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
            (Some(buffer), count)
        };

        let bgl = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("chromaticity-render cached bind group layout"),
                entries: &bgl_entries,
            });

        let fragment_module = compile_glsl_module(fragment_glsl, naga::ShaderStage::Fragment)
            .map_err(|e| RenderError::PassRenderFailed {
                pass_index: 0,
                source: Box::new(e),
            })?;
        let fragment_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("chromaticity-render cached fragment (user GLSL)"),
                source: wgpu::ShaderSource::Naga(Cow::Owned(fragment_module)),
            });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("chromaticity-render cached pipeline layout"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("chromaticity-render cached pipeline"),
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
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

        let mut bg_entries: Vec<wgpu::BindGroupEntry> = Vec::new();
        if let Some(buffer) = uniform_buffer.as_ref() {
            bg_entries.push(wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            });
        }
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("chromaticity-render cached bind group"),
            layout: &bgl,
            entries: &bg_entries,
        });

        let (target_texture, target_view, readback_buffer, padded_bytes_per_row) =
            allocate_cached_targets(&self.device, target_size);

        Ok(CachedSinglePass {
            label: label.to_string(),
            pipeline,
            bind_group,
            uniform_buffer,
            uniform_slot_count,
            target_size,
            target_texture,
            target_view,
            readback_buffer,
            padded_bytes_per_row,
        })
    }
}

/// Pre-compiled single-pass GPU pipeline with persistent uniform buffer,
/// offscreen `Rgba8Unorm` render target, and staging readback buffer.
///
/// Created via [`MultipassRenderer::compile_cached_single_pass`] so
/// per-frame rendering only uploads the uniform block (`queue.write_buffer`)
/// and submits one render + copy encoder without recompiling GLSL or
/// reallocating GPU targets.
pub struct CachedSinglePass {
    label: String,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: Option<wgpu::Buffer>,
    uniform_slot_count: usize,
    target_size: (u32, u32),
    target_texture: wgpu::Texture,
    target_view: wgpu::TextureView,
    readback_buffer: wgpu::Buffer,
    padded_bytes_per_row: u32,
}

impl CachedSinglePass {
    /// Render one frame into the cached offscreen target (resizing only
    /// when `target_size` changes) and return tightly packed RGBA8 bytes.
    ///
    /// # Errors
    ///
    /// - [`RenderError::TargetSizeInvalid`] — width or height is zero.
    /// - [`RenderError::UniformBindingFailed`] — uniform layout mismatch.
    /// - [`RenderError::DeviceLoss`] / [`RenderError::BufferMap`] — GPU
    ///   readback failure.
    pub fn render(
        &mut self,
        renderer: &MultipassRenderer,
        target_size: (u32, u32),
        uniforms: &[UniformInput],
    ) -> Result<Vec<u8>, RenderError> {
        if target_size.0 == 0 || target_size.1 == 0 {
            return Err(RenderError::TargetSizeInvalid(target_size));
        }
        if target_size != self.target_size {
            let (tex, view, readback, padded) =
                allocate_cached_targets(&renderer.device, target_size);
            self.target_texture = tex;
            self.target_view = view;
            self.readback_buffer = readback;
            self.padded_bytes_per_row = padded;
            self.target_size = target_size;
        }

        if let Some(buffer) = self.uniform_buffer.as_ref() {
            let slots = encode_uniform_slots(uniforms, &self.label)?;
            if slots.len() != self.uniform_slot_count {
                return Err(RenderError::UniformBindingFailed {
                    pass_label: self.label.clone(),
                    name: String::from("<block>"),
                    reason: format!(
                        "uniform slot count changed from {} to {}",
                        self.uniform_slot_count,
                        slots.len()
                    ),
                });
            }
            renderer
                .queue
                .write_buffer(buffer, 0, bytemuck::cast_slice(&slots));
        }

        let (width, height) = self.target_size;
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("chromaticity-render cached frame encoder"),
            });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("chromaticity-render cached pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.target_view,
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
            rpass.set_bind_group(0, &self.bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        renderer.queue.submit(Some(encoder.finish()));

        let buffer_slice = self.readback_buffer.slice(..);
        let (sender, receiver) = channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        loop {
            match renderer.device.poll(wgpu::PollType::Wait) {
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

        let unpadded_bytes_per_row = width * 4;
        let mapped = buffer_slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((unpadded_bytes_per_row as usize) * (height as usize));
        for row in 0..height {
            let start = (row * self.padded_bytes_per_row) as usize;
            let end = start + unpadded_bytes_per_row as usize;
            pixels.extend_from_slice(&mapped[start..end]);
        }
        drop(mapped);
        self.readback_buffer.unmap();
        Ok(pixels)
    }
}

fn allocate_cached_targets(
    device: &wgpu::Device,
    target_size: (u32, u32),
) -> (wgpu::Texture, wgpu::TextureView, wgpu::Buffer, u32) {
    let (width, height) = target_size;
    let target_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("chromaticity-render cached target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let unpadded_bytes_per_row = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
    let readback_size = u64::from(padded_bytes_per_row) * u64::from(height);
    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("chromaticity-render cached readback"),
        size: readback_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    (
        target_texture,
        target_view,
        readback_buffer,
        padded_bytes_per_row,
    )
}

/// Compile every pass in `spec` and render them sequentially across
/// `spec.frame_count` frames, returning the FINAL frame's FINAL
/// pass readback bytes (`target_format.bytes_per_pixel()` per pixel,
/// row-major, padding stripped — same shape as
/// [`crate::compile_and_render_single_pass`] for `Rgba8Unorm`).
///
/// Single-frame `frame_count = 1` plus all-`Rgba8Unorm` passes plus
/// no `feedback_self` passes is bit-identical to slice-2 behavior.
///
/// # Refactor compatibility shim
///
/// This free function builds a fresh [`MultipassRenderer`] per call
/// and immediately runs [`MultipassRenderer::render`], reproducing
/// the pre-refactor "device acquired per render" behavior
/// bit-identically. Callers that benefit from amortized device
/// creation across multiple renders (in particular slice 5b.3's GPU-
/// side composition path) should construct a [`MultipassRenderer`]
/// once and reuse it across `render` calls.
///
/// # Errors
///
/// - [`RenderError::EmptyPassList`] — `spec.passes` is empty OR
///   `spec.frame_count == 0`.
/// - [`RenderError::TargetSizeInvalid`] — width or height is zero.
/// - [`RenderError::PassInputOutOfRange`] — a pass references a
///   later or self pass index via [`PassInputBinding`].
/// - [`RenderError::FeedbackAliasError`] — a pass with
///   [`PassDef::feedback_self`] = `true` ALSO listed itself via a
///   manual [`PassInputBinding`] (the feedback ping-pong texture is
///   wired implicitly).
/// - [`RenderError::FloatTargetUnsupported`] — a pass requested a
///   float [`TargetFormat`] that the active wgpu adapter cannot
///   render to.
/// - [`RenderError::FragmentParse`] / [`RenderError::FragmentValidate`] —
///   any pass's GLSL fails the naga frontend.
/// - [`RenderError::UniformBindingFailed`] — uniform marshalling
///   error.
/// - [`RenderError::PassRenderFailed`] — any per-pass wgpu submission
///   / readback failure, wrapping the original [`RenderError`] from
///   the inner pass with the offending pass index for triage.
/// - [`RenderError::AdapterUnavailable`] / [`RenderError::DeviceRequest`] /
///   [`RenderError::DeviceLoss`] / [`RenderError::BufferMap`] — wgpu
///   device acquisition or readback failure.
pub fn compile_and_render_multipass(spec: &MultipassSpec) -> Result<Vec<u8>, RenderError> {
    // 1. Up-front structural validation BEFORE building the renderer
    // so a structural error on a headless CI runner without a usable
    // adapter still returns the expected variant rather than masking
    // it with `AdapterUnavailable` from `MultipassRenderer::new`.
    validate_spec_shape(spec)?;
    let renderer = MultipassRenderer::new()?;
    renderer.render(spec)
}

/// Up-front structural validation extracted so the shim can run it
/// before adapter acquisition. The shim path needs this for the
/// "structural error fires on adapter-less CI" invariant from the
/// slice-2 drift pins; `MultipassRenderer::render` re-runs the same
/// checks because callers that build their own renderer still expect
/// structural rejection to take priority over GPU-state errors.
fn validate_spec_shape(spec: &MultipassSpec) -> Result<(), RenderError> {
    if spec.passes.is_empty() {
        return Err(RenderError::EmptyPassList);
    }
    if spec.frame_count == 0 {
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
            if pass.feedback_self && binding.source_pass_index == pass_index {
                // Defense-in-depth: source_pass_index == pass_index
                // is already rejected by the previous check (>=);
                // this branch is unreachable but keeps the feedback
                // alias contract explicit at read-time.
                return Err(RenderError::FeedbackAliasError {
                    pass_index,
                    reason: "feedback pass must not also list self in input_textures \
                             (self-feedback is wired implicitly via the feedback_self flag)"
                        .to_string(),
                });
            }
        }
    }
    Ok(())
}

/// Inner render driver shared by [`MultipassRenderer::render`],
/// [`MultipassRenderer::render_to_texture`], and (transitively, via
/// the shim) [`compile_and_render_multipass`]. Takes the long-lived
/// adapter / device / queue / cached vertex shader by reference so
/// the renderer's resources are reused across calls. Returns the
/// FINAL frame's FINAL-pass GPU texture without performing the CPU
/// readback; the callers that need bytes (`render`,
/// `compile_and_render_multipass`) layer [`readback_texture`] on
/// top.
///
/// Slice-5b.3 split: the slice-5b.2+ refactor's
/// `render_multipass_with` body went render → copy → map → bytes in
/// one function. The split moves the copy + map into
/// [`readback_texture`] so the GPU composition path
/// ([`crate::composition::compose_transition_frame_gpu`]) can sample
/// the returned texture directly without round-tripping bytes.
fn render_multipass_to_texture_with(
    adapter: &wgpu::Adapter,
    device: &Arc<wgpu::Device>,
    queue: &wgpu::Queue,
    vertex_shader: &wgpu::ShaderModule,
    spec: &MultipassSpec,
) -> Result<wgpu::Texture, RenderError> {
    // 1. Up-front structural validation. These checks intentionally
    // run BEFORE any GPU work so a structural error returns the
    // expected variant rather than masking it with a downstream
    // wgpu failure. The shim path (`compile_and_render_multipass`)
    // already runs this BEFORE adapter acquisition; running it again
    // here is cheap and keeps the invariant intact for callers that
    // build their own renderer.
    validate_spec_shape(spec)?;
    let num_passes = spec.passes.len();

    // 4. Float-target adapter capability check. Run BEFORE texture
    // allocation so an unsupported format surfaces as
    // `FloatTargetUnsupported` rather than as a downstream wgpu
    // `TextureFormat unsupported` panic. We require
    // `TextureUsages::RENDER_ATTACHMENT` because every pass renders
    // to its target; sampling capability is checked implicitly when
    // the bind group is created against a `TextureSampleType::Float
    // { filterable: false }` layout entry.
    let adapter_features = adapter.features();
    let _ = adapter_features; // kept for future feature gating
    for (pass_index, pass) in spec.passes.iter().enumerate() {
        if matches!(
            pass.target_format,
            TargetFormat::Rgba16Float | TargetFormat::Rgba32Float
        ) {
            let format = pass.target_format.to_wgpu();
            let features = adapter.get_texture_format_features(format);
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

    // 5. Allocate render targets per pass. The slice-2 pattern was
    // ONE texture per pass; slice-4 widens to TWO textures for
    // feedback passes (ping-pong) so a pass can sample its own
    // previous-frame output.
    //
    // Storage shape: a flat `Vec<wgpu::Texture>` indexed by a
    // `pass_texture_layout: Vec<PassTextureSlots>` indirection that
    // records, per pass, the index range covering that pass's
    // textures. Non-feedback passes occupy 1 slot; feedback passes
    // occupy 2 (slot 0 is the "frame K%2 == 0" buffer, slot 1 the
    // "K%2 == 1" buffer). The ordering of the flat vec matches the
    // index range so per-pass-frame view selection is just an
    // arithmetic index.
    //
    // The FINAL pass also needs `COPY_SRC` because we read back from
    // it (whether or not it is a feedback pass).
    struct PassTextureSlots {
        first: usize,
        feedback: bool,
    }
    let mut pass_textures: Vec<wgpu::Texture> = Vec::with_capacity(num_passes + 2);
    let mut pass_views: Vec<wgpu::TextureView> = Vec::with_capacity(num_passes + 2);
    let mut pass_texture_layout: Vec<PassTextureSlots> = Vec::with_capacity(num_passes);
    for (pass_index, pass) in spec.passes.iter().enumerate() {
        let is_final = pass_index + 1 == num_passes;
        let mut usage =
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        if is_final {
            usage |= wgpu::TextureUsages::COPY_SRC;
        }
        let format = pass.target_format.to_wgpu();
        let slot_count = if pass.feedback_self { 2 } else { 1 };
        let first = pass_textures.len();
        for slot in 0..slot_count {
            let label = if pass.feedback_self {
                format!(
                    "chromaticity-render multipass target (pass {pass_index}, ping-pong slot {slot})"
                )
            } else {
                format!("chromaticity-render multipass target (pass {pass_index})")
            };
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&label),
                size: wgpu::Extent3d {
                    width: spec.target_size.0,
                    height: spec.target_size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            });
            let view = target.create_view(&wgpu::TextureViewDescriptor::default());
            pass_textures.push(target);
            pass_views.push(view);
        }
        pass_texture_layout.push(PassTextureSlots {
            first,
            feedback: pass.feedback_self,
        });
    }

    /// Index into `pass_views` / `pass_textures` for the texture
    /// that pass `pass_index` WROTE on frame `frame`.
    fn write_slot(layout: &[PassTextureSlots], pass_index: usize, frame: u32) -> usize {
        let slots = &layout[pass_index];
        if slots.feedback {
            slots.first + (frame as usize % 2)
        } else {
            slots.first
        }
    }

    /// Index into `pass_views` for the texture that pass
    /// `pass_index` READS on frame `frame` for its self-feedback
    /// binding (only meaningful when `slots.feedback == true`).
    fn feedback_read_slot(layout: &[PassTextureSlots], pass_index: usize, frame: u32) -> usize {
        let slots = &layout[pass_index];
        // Reads texture (frame + 1) % 2; on frame 0 reads slot 1
        // (the texture not yet written this run, which is black-
        // cleared on first allocation per wgpu's
        // `mapped_at_creation: false` default). Subsequent frames
        // read the texture this pass wrote on the previous frame.
        slots.first + ((frame as usize + 1) % 2)
    }

    // 6. A single shared sampler for ALL inter-pass texture lookups.
    // Slice-2's sampler is nearest / clamp-to-edge — the deterministic
    // choice that makes the readback hash bit-stable. Slice-4 widens
    // the sampler-binding-type to `NonFiltering` (already the slice-2
    // default), which is also the only sample type compatible with
    // `Rgba32Float` on most adapters without enabling extra wgpu
    // features.
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

    // 7. Build per-pass STATIC resources (layout, pipeline, uniform
    // buffer, vertex buffer, fragment shader). Bind groups are NOT
    // built here because feedback passes need a per-frame bind
    // group (the read texture flips with frame parity). Non-feedback
    // passes get a stable bind group, but we rebuild them per-frame
    // anyway for code simplicity — the cost is negligible at the
    // frame counts the corpus uses (≤ 16 frames).
    struct PassStatics {
        bind_group_layout: wgpu::BindGroupLayout,
        pipeline: wgpu::RenderPipeline,
        uniform_buffer: Option<wgpu::Buffer>,
        _pipeline_layout: wgpu::PipelineLayout,
        _vertex_buffer: wgpu::Buffer,
    }
    let mut pass_statics: Vec<PassStatics> = Vec::with_capacity(num_passes);

    for (pass_index, pass) in spec.passes.iter().enumerate() {
        let pass_label = pass.label.as_deref().unwrap_or("(unlabelled)");

        // 7a. Bind group LAYOUT — uniform block at binding 0 (if any),
        // explicit input textures at `1 + 2*i`, then (if feedback) a
        // self-feedback texture at the slot AFTER the explicit
        // bindings.
        let mut bgl_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();
        let uniform_buffer = if pass.uniforms.is_empty() {
            None
        } else {
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

        let total_input_count = pass.input_textures.len() + if pass.feedback_self { 1 } else { 0 };
        for i in 0..total_input_count {
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

        // 7b. Compile fragment shader.
        let fragment_module = compile_glsl_module(&pass.fragment_glsl, naga::ShaderStage::Fragment)
            .map_err(|e| RenderError::PassRenderFailed {
                pass_index,
                source: Box::new(e),
            })?;
        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chromaticity-render multipass fragment (user GLSL)"),
            source: wgpu::ShaderSource::Naga(Cow::Owned(fragment_module)),
        });

        // 7c. Pipeline layout + pipeline.
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chromaticity-render multipass pipeline layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("chromaticity-render multipass pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: vertex_shader,
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

        // 7d. Reserved vertex buffer (slice-1 inheritance).
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chromaticity-render multipass reserved vertex buffer"),
            contents: bytemuck::cast_slice(&[UniformSlot::default(); 3]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        pass_statics.push(PassStatics {
            bind_group_layout: bgl,
            pipeline,
            uniform_buffer,
            _pipeline_layout: pipeline_layout,
            _vertex_buffer: vertex_buffer,
        });
    }

    // 8. Frame loop. Each frame builds per-pass bind groups (texture
    // views may flip on feedback passes), records all passes into
    // a single command encoder, and submits.
    let final_index = num_passes - 1;
    let final_format = spec.passes[final_index].target_format;
    for frame in 0..spec.frame_count {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("chromaticity-render multipass encoder"),
        });

        // 8a. Per-pass bind groups for THIS frame.
        let mut frame_bind_groups: Vec<wgpu::BindGroup> = Vec::with_capacity(num_passes);
        for (pass_index, pass) in spec.passes.iter().enumerate() {
            let statics = &pass_statics[pass_index];
            let mut bg_entries: Vec<wgpu::BindGroupEntry> = Vec::new();
            if let Some(buffer) = statics.uniform_buffer.as_ref() {
                bg_entries.push(wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                });
            }
            // Explicit prior-pass bindings: read each prior pass's
            // CURRENT-frame write target. (Earlier passes already
            // wrote it earlier in this frame's loop, so the texture
            // contents are this frame's pass output.)
            for (i, binding) in pass.input_textures.iter().enumerate() {
                let tex_binding: u32 = 1 + 2 * (i as u32);
                let smp_binding: u32 = tex_binding + 1;
                let view_idx = write_slot(&pass_texture_layout, binding.source_pass_index, frame);
                let view =
                    pass_views
                        .get(view_idx)
                        .ok_or_else(|| RenderError::PassRenderFailed {
                            pass_index,
                            source: Box::new(RenderError::PassInputOutOfRange {
                                pass_index,
                                source_pass_index: binding.source_pass_index,
                                num_passes,
                            }),
                        })?;
                bg_entries.push(wgpu::BindGroupEntry {
                    binding: tex_binding,
                    resource: wgpu::BindingResource::TextureView(view),
                });
                bg_entries.push(wgpu::BindGroupEntry {
                    binding: smp_binding,
                    resource: wgpu::BindingResource::Sampler(&shared_sampler),
                });
            }
            // Self-feedback binding (after the explicit bindings).
            if pass.feedback_self {
                let i = pass.input_textures.len();
                let tex_binding: u32 = 1 + 2 * (i as u32);
                let smp_binding: u32 = tex_binding + 1;
                let view_idx = feedback_read_slot(&pass_texture_layout, pass_index, frame);
                let view =
                    pass_views
                        .get(view_idx)
                        .ok_or_else(|| RenderError::FeedbackAliasError {
                            pass_index,
                            reason: format!(
                                "feedback read slot {view_idx} out of range \
                                 (pass_views.len() = {})",
                                pass_views.len()
                            ),
                        })?;
                bg_entries.push(wgpu::BindGroupEntry {
                    binding: tex_binding,
                    resource: wgpu::BindingResource::TextureView(view),
                });
                bg_entries.push(wgpu::BindGroupEntry {
                    binding: smp_binding,
                    resource: wgpu::BindingResource::Sampler(&shared_sampler),
                });
            }

            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("chromaticity-render multipass bind group"),
                layout: &statics.bind_group_layout,
                entries: &bg_entries,
            });
            frame_bind_groups.push(bg);
        }

        // 8b. Record all passes for this frame. Each writes to its
        // current write-slot view.
        for (pass_index, statics) in pass_statics.iter().enumerate() {
            let view_idx = write_slot(&pass_texture_layout, pass_index, frame);
            let view = pass_views
                .get(view_idx)
                .ok_or(RenderError::PassRenderFailed {
                    pass_index,
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
            rpass.set_pipeline(&statics.pipeline);
            rpass.set_bind_group(0, &frame_bind_groups[pass_index], &[]);
            rpass.draw(0..3, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    }

    // 9. Extract the FINAL pass's FINAL-frame texture by removing it
    // from `pass_textures` so we own it for return. The
    // `pass_textures` Vec is dropped on function exit, releasing the
    // intermediate textures; the FINAL-pass texture's wgpu RAII
    // handle stays alive in the returned value.
    //
    // The final-frame index is `frame_count - 1`. The texture the
    // final pass wrote on that frame is `write_slot(layout,
    // final_index, frame_count - 1)`.
    let final_frame = spec.frame_count - 1;
    let final_view_idx = write_slot(&pass_texture_layout, final_index, final_frame);
    if final_view_idx >= pass_textures.len() {
        return Err(RenderError::PassRenderFailed {
            pass_index: final_index,
            source: Box::new(RenderError::EmptyPassList),
        });
    }
    // `swap_remove` is O(1) and avoids a clone of the wgpu::Texture
    // handle; we want unique ownership of the final target so the
    // caller (read-back path or composition pass) can rely on RAII.
    let final_target = pass_textures.swap_remove(final_view_idx);
    let _ = final_format;
    Ok(final_target)
}

/// Read back a wgpu texture into a `Vec<u8>` matching the
/// [`MultipassRenderer::render`] byte format (row-major, padding
/// stripped, `bytes_per_pixel * width * height` bytes total).
///
/// Slice-5b.3 split this out of `render_multipass_to_texture_with`
/// so [`MultipassRenderer::render_to_texture`] can return the GPU
/// texture without paying for the readback round-trip; the
/// composition pass needs the texture, not the bytes.
///
/// `texture` MUST be 2D, single mip, single array layer, with usage
/// `COPY_SRC` set on the descriptor (every texture this crate
/// produces via [`render_multipass_to_texture_with`] satisfies these
/// invariants — the function builds them with
/// `RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_SRC` for the final
/// pass).
///
/// # Errors
///
/// - [`RenderError::DeviceLoss`] — wgpu device lost mid-poll.
/// - [`RenderError::BufferMap`] — `map_async` returned an error or
///   the completion channel was dropped before the future resolved.
fn readback_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
) -> Result<Vec<u8>, RenderError> {
    let format = texture.format();
    let bytes_per_pixel = format.block_copy_size(None).ok_or_else(|| {
        RenderError::BufferMap(format!(
            "texture format {format:?} has no fixed block size for readback"
        ))
    })?;
    let width = texture.width();
    let height = texture.height();
    let unpadded_bytes_per_row = width * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
    let readback_size = u64::from(padded_bytes_per_row) * u64::from(height);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("chromaticity-render multipass readback"),
        size: readback_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut copy_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("chromaticity-render multipass copy encoder"),
    });
    copy_encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(copy_encoder.finish()));

    // Map readback buffer and strip row padding. Identical to
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
    let mut pixels = Vec::with_capacity((unpadded_bytes_per_row as usize) * (height as usize));
    for row in 0..height {
        let start = (row * padded_bytes_per_row) as usize;
        let end = start + unpadded_bytes_per_row as usize;
        pixels.extend_from_slice(&mapped[start..end]);
    }
    drop(mapped);
    readback.unmap();

    Ok(pixels)
}

/// Crate-internal accessor for [`readback_texture`] so
/// [`crate::composition`] can map a composition output texture back
/// to bytes without re-implementing the row-padding strip.
pub(crate) fn readback_texture_to_bytes(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
) -> Result<Vec<u8>, RenderError> {
    readback_texture(device, queue, texture)
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
        if input.name.is_empty() {
            return Err(RenderError::UniformBindingFailed {
                pass_label: pass_label.to_string(),
                name: input.name.clone(),
                reason: "uniform name must not be empty".to_string(),
            });
        }
        match &input.value {
            UniformValue::Scalar(v) => {
                let mut slot = UniformSlot::default();
                slot.bytes[0..4].copy_from_slice(&v.to_le_bytes());
                slots.push(slot);
            }
            UniformValue::Vec2(v) => {
                let mut slot = UniformSlot::default();
                slot.bytes[0..4].copy_from_slice(&v[0].to_le_bytes());
                slot.bytes[4..8].copy_from_slice(&v[1].to_le_bytes());
                slots.push(slot);
            }
            UniformValue::Vec3(v) => {
                let mut slot = UniformSlot::default();
                slot.bytes[0..4].copy_from_slice(&v[0].to_le_bytes());
                slot.bytes[4..8].copy_from_slice(&v[1].to_le_bytes());
                slot.bytes[8..12].copy_from_slice(&v[2].to_le_bytes());
                slots.push(slot);
            }
            UniformValue::Vec4(v) => {
                let mut slot = UniformSlot::default();
                slot.bytes[0..4].copy_from_slice(&v[0].to_le_bytes());
                slot.bytes[4..8].copy_from_slice(&v[1].to_le_bytes());
                slot.bytes[8..12].copy_from_slice(&v[2].to_le_bytes());
                slot.bytes[12..16].copy_from_slice(&v[3].to_le_bytes());
                slots.push(slot);
            }
            UniformValue::Int(v) => {
                let mut slot = UniformSlot::default();
                slot.bytes[0..4].copy_from_slice(&v.to_le_bytes());
                slots.push(slot);
            }
            UniformValue::UInt(v) => {
                let mut slot = UniformSlot::default();
                slot.bytes[0..4].copy_from_slice(&v.to_le_bytes());
                slots.push(slot);
            }
            UniformValue::FloatArray(arr) => {
                // std140 array-of-float: each element is vec4-aligned
                // to 16 bytes with the f32 in the first 4 bytes of
                // its slot. Emit `arr.len()` consecutive slots; the
                // shader-side `uniform float chroma_AudioFFT[N]`
                // declaration reads element K from offset `K * 16`.
                for v in arr {
                    let mut slot = UniformSlot::default();
                    slot.bytes[0..4].copy_from_slice(&v.to_le_bytes());
                    slots.push(slot);
                }
            }
        }
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
    //! - Slice-4 additions:
    //!   - `frame_count == 0` rejects with
    //!     [`RenderError::EmptyPassList`].
    //!   - [`TargetFormat`] enum maps to the expected wgpu format
    //!     and bytes-per-pixel pair.
    //!   - [`PassDef`] / [`MultipassSpec`] `Default` produce the
    //!     slice-2-compatible single-frame `Rgba8Unorm` shape.
    //! - Slice-5b.2+ refactor additions:
    //!   - [`MultipassRenderer::new`] succeeds on a working wgpu
    //!     adapter and returns accessor-reachable device + queue
    //!     handles.
    //!   - [`MultipassRenderer::render`] is byte-identical across
    //!     two consecutive calls with the same spec (long-lived
    //!     device reuse does not perturb output).
    //!   - [`compile_and_render_multipass`] (the shim) is
    //!     byte-identical to
    //!     [`MultipassRenderer::new`] + [`MultipassRenderer::render`]
    //!     for the same spec (shim equivalence).
    //!   - [`MultipassRenderer::device`] returns the SAME
    //!     `Arc<wgpu::Device>` across calls
    //!     (`Arc::ptr_eq` identity), proving the device is owned by
    //!     the renderer rather than rebuilt per `render`.

    use std::sync::Arc;

    use super::{
        MultipassRenderer, MultipassSpec, PassDef, PassInputBinding, RenderError, TargetFormat,
        UniformInput, UniformValue, compile_and_render_multipass, encode_uniform_slots,
    };

    #[test]
    fn empty_pass_list_returns_empty_pass_list_error() {
        let spec = MultipassSpec {
            passes: vec![],
            target_size: (16, 16),
            ..MultipassSpec::default()
        };
        let err = compile_and_render_multipass(&spec)
            .expect_err("empty pass list must reject before adapter acquisition");
        assert!(
            matches!(err, RenderError::EmptyPassList),
            "expected EmptyPassList, got {err:?}",
        );
    }

    #[test]
    fn zero_frame_count_returns_empty_pass_list_error() {
        // Drift pin: `frame_count == 0` is a structural error
        // (no rendering can happen) and surfaces as
        // `EmptyPassList`. Slice-4 addition.
        let trivial = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() { fragColor = vec4(uv, 0.0, 1.0); }
"#
        .to_string();
        let spec = MultipassSpec {
            passes: vec![PassDef {
                fragment_glsl: trivial,
                ..PassDef::default()
            }],
            target_size: (16, 16),
            frame_count: 0,
        };
        let err = compile_and_render_multipass(&spec)
            .expect_err("frame_count == 0 must reject before adapter acquisition");
        assert!(
            matches!(err, RenderError::EmptyPassList),
            "expected EmptyPassList for frame_count=0, got {err:?}",
        );
    }

    #[test]
    fn target_format_default_is_rgba8unorm() {
        // Drift pin: the slice-4 default for `PassDef::target_format`
        // MUST remain `Rgba8Unorm` to preserve the slice-1 / slice-2
        // corpus continuity. Any future widening of the default
        // must accompany a regeneration of every existing T2 hash.
        assert_eq!(TargetFormat::default(), TargetFormat::Rgba8Unorm);
        assert_eq!(PassDef::default().target_format, TargetFormat::Rgba8Unorm);
        assert!(!PassDef::default().feedback_self);
        assert_eq!(MultipassSpec::default().frame_count, 1);
    }

    #[test]
    fn target_format_bytes_per_pixel_layout() {
        // Drift pin: the bytes-per-pixel mapping is what the
        // readback path uses to compute the row stride. RGBA = 4
        // channels at 1 / 2 / 4 bytes per channel.
        assert_eq!(TargetFormat::Rgba8Unorm.bytes_per_pixel(), 4);
        assert_eq!(TargetFormat::Rgba16Float.bytes_per_pixel(), 8);
        assert_eq!(TargetFormat::Rgba32Float.bytes_per_pixel(), 16);
    }

    #[test]
    fn target_format_to_wgpu_mapping() {
        // Drift pin: wgpu enum mapping. If wgpu renames /
        // reorganizes its TextureFormat enum, this surfaces.
        assert_eq!(
            TargetFormat::Rgba8Unorm.to_wgpu(),
            wgpu::TextureFormat::Rgba8Unorm
        );
        assert_eq!(
            TargetFormat::Rgba16Float.to_wgpu(),
            wgpu::TextureFormat::Rgba16Float
        );
        assert_eq!(
            TargetFormat::Rgba32Float.to_wgpu(),
            wgpu::TextureFormat::Rgba32Float
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
                    input_textures: vec![PassInputBinding {
                        source_pass_index: 1,
                        sampler_uniform_name: "pass1".to_string(),
                    }],
                    ..PassDef::default()
                },
                PassDef {
                    label: Some("forward-ref-pass-1".to_string()),
                    fragment_glsl: trivial,
                    ..PassDef::default()
                },
            ],
            target_size: (16, 16),
            ..MultipassSpec::default()
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
                input_textures: vec![PassInputBinding {
                    source_pass_index: 0,
                    sampler_uniform_name: "self".to_string(),
                }],
                ..PassDef::default()
            }],
            target_size: (16, 16),
            ..MultipassSpec::default()
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
                fragment_glsl: trivial,
                ..PassDef::default()
            }],
            target_size: (0, 16),
            ..MultipassSpec::default()
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
    fn uniform_marshalling_float_array_layout() {
        // Drift pin: `UniformValue::FloatArray(N)` emits N consecutive
        // 16-byte slots, each holding the f32 in its first 4 bytes
        // with 12 bytes of zero pad. Matches std140's `uniform float
        // arr[N]` layout where every array element is vec4-aligned.
        // Slice-5 follow-up addition unlocking single-name array
        // bindings (`chroma_AudioFFT[8]` per docs/36 Sec.2.11).
        let inputs = vec![UniformInput {
            name: "u_array".to_string(),
            value: UniformValue::FloatArray(vec![0.10, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80]),
        }];
        let slots = encode_uniform_slots(&inputs, "drift-pin")
            .expect("float-array marshalling must succeed");
        assert_eq!(
            slots.len(),
            8,
            "FloatArray(N) must emit N slots (one per element)",
        );
        let expected = [
            0.10_f32, 0.20_f32, 0.30_f32, 0.40_f32, 0.50_f32, 0.60_f32, 0.70_f32, 0.80_f32,
        ];
        for (i, exp) in expected.iter().enumerate() {
            assert_eq!(
                &slots[i].bytes[0..4],
                &exp.to_le_bytes(),
                "FloatArray slot {i} f32 mismatch",
            );
            assert!(
                slots[i].bytes[4..16].iter().all(|b| *b == 0),
                "FloatArray slot {i} trailing pad must be zero",
            );
        }
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

    /// Trivial single-pass spec helper for the `MultipassRenderer`
    /// drift pins. Renders a UV-gradient at 16x16 — enough to exercise
    /// the full pipeline (vertex + fragment + readback) without
    /// pulling in a corpus dependency.
    fn trivial_uv_spec() -> MultipassSpec {
        let trivial = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() { fragColor = vec4(uv, 0.0, 1.0); }
"#
        .to_string();
        MultipassSpec {
            passes: vec![PassDef {
                label: Some("drift-pin-trivial".to_string()),
                fragment_glsl: trivial,
                ..PassDef::default()
            }],
            target_size: (16, 16),
            ..MultipassSpec::default()
        }
    }

    #[test]
    fn renderer_new_returns_reachable_device_and_queue() {
        // Drift pin: `MultipassRenderer::new` succeeds on a working
        // wgpu adapter and the device + queue accessors return
        // non-null handles. Slice-5b.2+ refactor addition.
        let renderer = MultipassRenderer::new()
            .expect("MultipassRenderer::new must succeed on a working wgpu adapter");
        // `device()` returns `&Arc<wgpu::Device>` — dereferencing
        // the Arc proves the inner pointer is reachable.
        let device: &Arc<wgpu::Device> = renderer.device();
        // wgpu::Device has no observable getter we can call without
        // GPU work; the existence of an Arc<Device> with refcount
        // >= 1 is the reachability proof.
        assert!(Arc::strong_count(device) >= 1);
        // `queue()` returns `&wgpu::Queue`; the address is valid
        // (no segfault on access) — the test passes if construction
        // completed.
        let _queue: &wgpu::Queue = renderer.queue();
    }

    #[test]
    fn renderer_render_byte_identical_across_two_calls() {
        // Drift pin: two consecutive `render` calls with the same
        // spec produce byte-identical output. The renderer reuses
        // its device + cached vertex shader; per-spec resources are
        // rebuilt each call. Slice-5b.2+ refactor addition —
        // protects against a future regression where per-call state
        // leaks across `render` invocations.
        let renderer = MultipassRenderer::new()
            .expect("MultipassRenderer::new must succeed on a working wgpu adapter");
        let spec = trivial_uv_spec();
        let first = renderer
            .render(&spec)
            .expect("first render must succeed on a working wgpu adapter");
        let second = renderer
            .render(&spec)
            .expect("second render must succeed on a working wgpu adapter");
        assert_eq!(
            first, second,
            "two consecutive renders with the same spec must produce byte-identical output",
        );
    }

    #[test]
    fn shim_byte_identical_to_renderer_new_then_render() {
        // Drift pin: the backwards-compat shim
        // (`compile_and_render_multipass`) is byte-identical to the
        // explicit `MultipassRenderer::new()?.render(...)` form. The
        // shim is the migration bridge for callers that have not
        // moved to the long-lived shape; this pin guards that the
        // bridge stays semantically transparent. Slice-5b.2+ refactor
        // addition.
        let spec = trivial_uv_spec();
        let shim = compile_and_render_multipass(&spec)
            .expect("shim path must succeed on a working wgpu adapter");
        let renderer = MultipassRenderer::new()
            .expect("MultipassRenderer::new must succeed on a working wgpu adapter");
        let direct = renderer
            .render(&spec)
            .expect("direct render must succeed on a working wgpu adapter");
        assert_eq!(
            shim, direct,
            "shim output must equal direct MultipassRenderer::new()?.render() output",
        );
    }

    #[test]
    fn renderer_device_arc_stable_across_render_calls() {
        // Drift pin: `MultipassRenderer::device()` returns the SAME
        // `Arc<wgpu::Device>` reference across multiple `render`
        // calls, proving the device is owned by the renderer rather
        // than rebuilt per call. `Arc::ptr_eq` is the strict
        // identity check — Arc clones share the inner allocation, so
        // ptr_eq is true iff the underlying allocation is the same.
        // Slice-5b.2+ refactor addition.
        let renderer = MultipassRenderer::new()
            .expect("MultipassRenderer::new must succeed on a working wgpu adapter");
        let device_before: Arc<wgpu::Device> = Arc::clone(renderer.device());
        let spec = trivial_uv_spec();
        let _ = renderer
            .render(&spec)
            .expect("render must succeed on a working wgpu adapter");
        let device_after: Arc<wgpu::Device> = Arc::clone(renderer.device());
        assert!(
            Arc::ptr_eq(&device_before, &device_after),
            "renderer.device() must return the same Arc<Device> allocation across render calls",
        );
    }
}
