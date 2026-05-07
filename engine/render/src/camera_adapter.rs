// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4a Wave 1c-β Phase 2 PR 5 — engine-side camera → render
//! texture-binding adapter per ADR-020 §7.
//!
//! # Cross-crate boundary
//!
//! The `chromaticity_camera_adapter` crate
//! (`integrations/camera-adapter/`) publishes plain-data
//! [`chromaticity_camera_adapter::CameraFrame`] payloads from
//! [`chromaticity_camera_adapter::CameraAdapter::poll`] (per-platform
//! backends: CoreMediaIO on macOS Phase 2 PR 2; Media Foundation on
//! Windows Phase 2 PR 3). That crate intentionally does NOT depend
//! on `engine/render` — see `integrations/camera-adapter/src/lib.rs`
//! Phase-2 PR-1 scope prose ("Phase 2 PR 5 wires
//! `engine/render/src/camera_adapter.rs` and closes the
//! `EV-INTEGRATION-MATRIX` camera-arm widening"). This module is the
//! consumer-side marshalling step that satisfies that boundary,
//! mirroring the [`crate::audio_adapter`] precedent (slice-5a
//! cross-crate audio → render uniform-binding adapter): producer
//! crate publishes plain data; consumer-side engine crate marshals
//! the plain-data shape into the engine seam.
//!
//! Unlike [`crate::audio_adapter`] (which marshals scalar / array
//! audio uniforms into a `Vec<UniformInput>`), the camera surface
//! marshals frame payloads into a [`CameraTextureBinding`] carrier
//! shape — texture data is GPU-resident sampler input, not a
//! per-pass scalar uniform. Authors sample the bound texture in
//! GLSL via `texture2D(chroma_CameraInput0, uv)` (or `[1]` / `[2]`
//! / `[3]` for the additional slots).
//!
//! # Texture-binding namespace per ADR-020 §7
//!
//! Slice-PR-5 binds camera frames under the
//! `chroma_CameraInput[N]` symbol family where `N ∈ 0..=3`
//! ([`MAX_CAMERA_BINDING_SLOTS`] = 4 — mirrors the Shadertoy
//! `iChannel0..3` range and the chromaticity-native `chroma_*`
//! namespace per the user-memory `feedback_syn_prefix_synesthesia_compat.md`
//! convention). The slot-N → symbol-name function is
//! [`camera_uniform_name`]; PR 5 ships the four canonical names as
//! [`CAMERA_UNIFORM_NAMES`] for fast direct lookup.
//!
//! # Pixel-format → `wgpu::TextureFormat` mapping per ADR-020 §7
//!
//! | source `PixelFormat` | wgpu `TextureFormat`     | upload shape                       |
//! |----------------------|--------------------------|------------------------------------|
//! | `Bgra32`             | `Bgra8UnormSrgb`         | direct (4 bpp; no repacking)       |
//! | `Rgb24`              | `Rgba8UnormSrgb`         | repack 3 → 4 bpp w/ alpha = 0xFF   |
//! | `Nv12`               | `R8Unorm` (luma plane)   | planar (chroma plane bound separately in a future widening) |
//! | `Yuyv`               | `Rg8Unorm`               | packed (every 4 source bytes carry 2 luma + 1 chroma pair) |
//! | `Mjpeg`              | (rejected)               | [`RenderError::CameraFormatNotSupported`] until JPEG decode wires |
//!
//! ## Mapping rationale
//!
//! - `Bgra32` lands on `Bgra8UnormSrgb` (the universal HDR-bypass
//!   path) because every wgpu backend Chromaticity ships supports
//!   that format with sampleable + render-attachment usage flags
//!   per the wgpu feature matrix.
//! - `Rgb24` is the awkward one — wgpu has no native `Rgb8` format
//!   for sampler bindings (the Vulkan `VK_FORMAT_R8G8B8_UNORM`
//!   single-byte-per-channel three-component path is optional and
//!   unsupported on most desktop drivers). PR 5's mapping repacks
//!   to `Rgba8UnormSrgb` with alpha = 0xFF; the upload doubles the
//!   per-pixel byte count (3 bpp source → 4 bpp upload) but keeps
//!   the binding shape uniform across the four wgpu backends.
//! - `Nv12` planar mapping returns the LUMA plane only as a
//!   single `R8Unorm` texture in PR 5 (see the
//!   [`CameraTextureBinding::chroma_plane`] field — `None` in PR 5;
//!   future widening adds the `Rg8Unorm`-bound chroma plane).
//!   Authors who sample `chroma_CameraInput[N]` against an `Nv12`
//!   source see grayscale luminance in PR 5 — explicit YUV → RGB
//!   conversion in the fragment shader is the future-wave path.
//! - `Yuyv` packed mapping treats the 4-byte `Y0 U Y1 V` repeat as
//!   a packed `Rg8Unorm` 2-byte-per-pixel texture — the R channel
//!   carries the per-pixel luma, the G channel carries the
//!   alternating chroma. Authors get a 2-channel sampler that
//!   approximates luminance plus a chroma carrier; full YUV → RGB
//!   conversion is the future-wave path. Per the Khronos packed
//!   YUV convention.
//! - `Mjpeg` is rejected at the marshal boundary with
//!   [`RenderError::CameraFormatNotSupported`] until a future wave
//!   wires the JPEG decode path. The `frames.rs` module-level
//!   prose documents this as the deferred enhancement; the camera-
//!   adapter side returns the canonical `format-not-supported`
//!   fault reason during enumeration when MJPG is the only
//!   offered format.
//!
//! # Pure function
//!
//! [`camera_texture_binding_from_frame`] returns the same
//! [`CameraTextureBinding`] for the same `(frame, slot_index)`
//! input. No GPU device is required — the marshal runs on the CPU
//! and the consumer is responsible for the actual `Queue::write_texture`
//! upload (mirrors the [`crate::audio_adapter`] consumer-side
//! upload pattern).
//!
//! The marshal is intentionally a free function rather than an
//! inherent impl on [`crate::multipass::PassDef`] — keeping the
//! camera-side coupling out of `multipass.rs` lets the feature gate
//! cover one module instead of requiring `#[cfg]` on an impl block.
//!
//! # Scope narrowing for PR 5
//!
//! PR 5 ships:
//!
//! - The pure-function marshal
//!   [`camera_texture_binding_from_frame`] — slot index ∈ `0..=3`
//!   plus `&CameraFrame` → `Result<CameraTextureBinding,
//!   RenderError>`.
//! - The [`CameraTextureBinding`] carrier shape — symbol name +
//!   wgpu format + dimensions + repacked upload bytes (the
//!   `Rgb24 → Rgba8UnormSrgb` 3 → 4 bpp expansion runs in the
//!   marshal; `Bgra32` / `Nv12` luma / `Yuyv` packed shapes copy
//!   the source payload through verbatim).
//! - The slot-N → symbol-name lookup
//!   ([`camera_uniform_name`] + [`CAMERA_UNIFORM_NAMES`]).
//! - T1 drift pins covering the symbol-namespace shape + the
//!   pixel-format mapping table + the slot-bound rejection +
//!   the `Mjpeg` `CameraFormatNotSupported` rejection per
//!   docs/43 §15.5 + the canonical EV-ID-not-row addressing
//!   convention recorded in CLAUDE.md.
//!
//! PR 5 explicitly **does NOT ship**, with the narrowing recorded
//! honestly in the closing-docs `EV-INTEGRATION-MATRIX` camera-arm
//! widening:
//!
//! - The actual `wgpu::Queue::write_texture` upload + the
//!   `wgpu::Texture` allocation + the `wgpu::TextureView` /
//!   `wgpu::Sampler` resource creation. The marshal returns the
//!   carrier shape; the GPU-side allocation + upload is the
//!   consumer's job (a future wave that integrates the camera
//!   binding into [`crate::multipass::MultipassRenderer`]'s
//!   per-pass binding loop is the natural target). Mirrors the
//!   [`crate::audio_adapter::uniform_input_from_audio_snapshot`]
//!   pattern: marshal returns plain-data; the multipass scheduler
//!   does the GPU upload.
//! - The `Nv12` chroma-plane bind. PR 5's marshal returns only
//!   the luma plane; the future wave widens
//!   [`CameraTextureBinding`] with a second optional binding for
//!   the chroma plane.
//! - The MJPG JPEG decode. PR 5 rejects `Mjpeg` at the marshal
//!   boundary; a future wave wires `image::codecs::jpeg::JpegDecoder`
//!   (or `mozjpeg` for performance) and re-routes the marshal to
//!   produce a `Bgra32`-shaped binding from a decoded MJPG frame.
//! - Multi-camera composition. PR 5 honors the slot index ∈
//!   `0..=3` per-call but does not aggregate across slots; the
//!   consumer is responsible for marshalling each `CameraFrame`
//!   into its slot independently.
//!
//! # Evidence anchors
//!
//! - `docs/26-evidence-artifact-registry.md` `EV-INTEGRATION-MATRIX`
//!   row — camera-arm widening recorded in the closing-docs
//!   description-column entry per the canonical EV-ID-not-row
//!   addressing convention.
//! - `docs/adr/ADR-020-camera-adapter.md` §3, §7 — adapter contract
//!   + texture-binding namespace.
//! - `docs/54-wave-1c-beta-research-lessons.md` Phase-2 lessons 1
//!   (deque-swap drain), 4 (byte-order discipline), 6 (single-slot
//!   atomic exchange or bounded MPSC).
//! - `docs/adr/ADR-011-rendering-backend.md` §Decision — wgpu
//!   format-support matrix.

use chromaticity_camera_adapter::{CameraFrame, PixelFormat};

use crate::single_pass::RenderError;

/// Maximum number of `chroma_CameraInput[N]` texture bindings a
/// single scene may declare per ADR-020 §7. The upper bound mirrors
/// the Shadertoy `iChannel0..3` convention; widening to 5+ slots
/// requires an ADR-020 amendment per the ADR's "Revisit" rule (e).
pub const MAX_CAMERA_BINDING_SLOTS: usize = 4;

/// The four canonical `chroma_CameraInput[N]` symbol names per
/// ADR-020 §7. Indexed by binding slot (0..=3); use
/// [`camera_uniform_name`] to look up by slot index with bounds
/// checking.
///
/// Authors declare the binding in GLSL as
/// `uniform sampler2D chroma_CameraInput0;` (or `1` / `2` / `3`).
/// The integer suffix (rather than `[N]` array indexing) keeps the
/// per-binding `wgpu::BindGroupLayoutEntry` shape independent of
/// the others — each slot is a separate binding, not an element
/// of a binding-array (which would require the `BINDING_ARRAY`
/// wgpu feature flag that the ADR-011 backend matrix does not
/// universally enable).
pub const CAMERA_UNIFORM_NAMES: [&str; MAX_CAMERA_BINDING_SLOTS] = [
    "chroma_CameraInput0",
    "chroma_CameraInput1",
    "chroma_CameraInput2",
    "chroma_CameraInput3",
];

/// Look up the canonical `chroma_CameraInput[N]` symbol name for a
/// given binding slot index (0-based).
///
/// Returns `None` when `slot_index >= MAX_CAMERA_BINDING_SLOTS`.
/// The marshal entry point [`camera_texture_binding_from_frame`]
/// surfaces an out-of-range slot as a structured
/// [`RenderError`] rather than as a `None` here, so callers that
/// already validated the slot via the marshal can `expect` the
/// `Some` return safely.
#[must_use]
pub fn camera_uniform_name(slot_index: usize) -> Option<&'static str> {
    CAMERA_UNIFORM_NAMES.get(slot_index).copied()
}

/// Engine-side carrier of a marshalled [`CameraFrame`] ready for
/// `wgpu::Queue::write_texture` upload to a sampler-bound texture
/// at the [`Self::uniform_name`] symbol.
///
/// PR 5 ships the carrier shape; the actual `wgpu::Texture` /
/// `wgpu::TextureView` / `wgpu::Sampler` allocation + the
/// `Queue::write_texture` upload land in a follow-up wave (per
/// the module-level "Scope narrowing for PR 5" prose). The marshal
/// shape is plain data — no GPU handles — so callers can pre-
/// stage uploads, batch across slots, or schedule against the
/// [`crate::multipass::MultipassRenderer::queue`] long-lived queue
/// at their discretion.
///
/// Mirrors the audio-adapter `Vec<UniformInput>` shape
/// (per-uniform plain data, marshal precedes upload) — see
/// [`crate::audio_adapter::uniform_input_from_audio_snapshot`].
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (camera arm).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraTextureBinding {
    /// Canonical `chroma_CameraInput[N]` symbol name per ADR-020
    /// §7. Diagnostic + matched against the GLSL `uniform sampler2D`
    /// declaration name during binding-layout assembly.
    pub uniform_name: &'static str,
    /// wgpu texture format the upload bytes are encoded in.
    /// Determined by [`Self::source_pixel_format`] per the
    /// module-level mapping table.
    pub wgpu_format: wgpu::TextureFormat,
    /// Pixel-format the source [`CameraFrame`] declared. Retained
    /// alongside [`Self::wgpu_format`] so the consumer can route
    /// optional fragment-shader-side YUV → RGB conversion based
    /// on the source format (not just the bound wgpu format).
    pub source_pixel_format: PixelFormat,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Repacked upload bytes ready for `Queue::write_texture`.
    ///
    /// - `Bgra32` source: copied through verbatim (4 bpp).
    /// - `Rgb24` source: 3 → 4 bpp expansion with alpha = 0xFF
    ///   per the module-level mapping table.
    /// - `Nv12` source: luma plane only (`width * height` bytes;
    ///   chroma plane deferred per [`Self::chroma_plane`]).
    /// - `Yuyv` source: copied through verbatim (`width * 2 *
    ///   height` bytes; packed `Rg8Unorm` representation).
    /// - `Mjpeg` source: this binding never constructs (the
    ///   marshal returns
    ///   [`RenderError::CameraFormatNotSupported`]).
    pub upload_bytes: Vec<u8>,
    /// Optional secondary upload payload for planar-YUV chroma
    /// data. Always `None` in PR 5 per the module-level "Scope
    /// narrowing for PR 5" prose; the future wave that lands the
    /// `Nv12` chroma plane widens this to `Some(Vec<u8>)`
    /// carrying the interleaved U/V plane bytes (1/2-resolution
    /// per ADR-020 §7 Nv12 mapping note).
    pub chroma_plane: Option<Vec<u8>>,
}

/// Marshal a [`CameraFrame`] into a [`CameraTextureBinding`] for
/// the given slot index ∈ `0..MAX_CAMERA_BINDING_SLOTS`.
///
/// Pure function: same `(frame, slot_index)` input → same
/// [`CameraTextureBinding`] output. No GPU device required.
///
/// # Errors
///
/// - [`RenderError::CameraFormatNotSupported`] when
///   `frame.pixel_format == PixelFormat::Mjpeg` (JPEG decode
///   deferred per the module-level "Scope narrowing for PR 5").
/// - [`RenderError::CameraFormatNotSupported`] when the source
///   pixel format is a future widening that PR 5 does not yet map
///   (the `chromaticity_camera_adapter::PixelFormat` enum is
///   `#[non_exhaustive]`; new variants surface here until the
///   mapping table widens).
/// - [`RenderError::TargetSizeInvalid`] when `frame.width == 0`
///   or `frame.height == 0` (zero-dim textures are rejected by
///   wgpu's texture-creation validator; surfacing the rejection
///   at the marshal boundary lets the consumer skip the upload
///   entirely without consulting the GPU).
/// - [`RenderError::UniformBindingFailed`] when `slot_index >=
///   MAX_CAMERA_BINDING_SLOTS`. Surfaces with the canonical
///   namespace label so triage routes to ADR-020 §7.
pub fn camera_texture_binding_from_frame(
    frame: &CameraFrame,
    slot_index: usize,
) -> Result<CameraTextureBinding, RenderError> {
    if frame.width == 0 || frame.height == 0 {
        return Err(RenderError::TargetSizeInvalid((frame.width, frame.height)));
    }

    let uniform_name =
        camera_uniform_name(slot_index).ok_or_else(|| RenderError::UniformBindingFailed {
            pass_label: "(camera-binding)".to_string(),
            name: format!("chroma_CameraInput{slot_index}"),
            reason: format!(
                "slot index {slot_index} out of range; \
                 ADR-020 §7 caps `chroma_CameraInput[N]` at N ∈ 0..={}",
                MAX_CAMERA_BINDING_SLOTS - 1,
            ),
        })?;

    let (wgpu_format, upload_bytes) = match frame.pixel_format {
        PixelFormat::Bgra32 => (wgpu::TextureFormat::Bgra8UnormSrgb, frame.payload.clone()),
        PixelFormat::Rgb24 => (
            wgpu::TextureFormat::Rgba8UnormSrgb,
            repack_rgb24_to_rgba8(&frame.payload),
        ),
        PixelFormat::Nv12 => {
            // PR 5 narrowing: emit only the luma plane as
            // `R8Unorm`. The chroma plane is documented inline as
            // the future-wave widening (see
            // [`CameraTextureBinding::chroma_plane`]).
            let luma_byte_count = (frame.width as usize) * (frame.height as usize);
            let luma = frame
                .payload
                .get(..luma_byte_count)
                .map(<[u8]>::to_vec)
                .unwrap_or_default();
            (wgpu::TextureFormat::R8Unorm, luma)
        }
        PixelFormat::Yuyv => (wgpu::TextureFormat::Rg8Unorm, frame.payload.clone()),
        PixelFormat::Mjpeg => {
            return Err(RenderError::CameraFormatNotSupported {
                format: "Mjpeg".to_string(),
                reason: "JPEG decode path deferred per ADR-020 §Decision §2 \
                         Mjpeg note + camera_adapter.rs `# Scope narrowing \
                         for PR 5`; future wave wires `image::codecs::jpeg` \
                         decode and re-routes the marshal to the Bgra32 \
                         binding shape"
                    .to_string(),
            });
        }
        // The `chromaticity_camera_adapter::PixelFormat` enum is
        // `#[non_exhaustive]`; future widenings (e.g. `Yv12`,
        // `I420`, 10-bit `P010`) surface here as the "format
        // unmapped" rejection until the mapping table is widened
        // by a follow-up render-side PR.
        other => {
            return Err(RenderError::CameraFormatNotSupported {
                format: format!("{other:?}"),
                reason: "PR 5 mapping table covers Bgra32 / Rgb24 / Nv12 / \
                         Yuyv / Mjpeg per ADR-020 §7; new pixel-format \
                         variants require a render-side mapping widening"
                    .to_string(),
            });
        }
    };

    Ok(CameraTextureBinding {
        uniform_name,
        wgpu_format,
        source_pixel_format: frame.pixel_format,
        width: frame.width,
        height: frame.height,
        upload_bytes,
        chroma_plane: None,
    })
}

/// Repack a 3-byte-per-pixel `Rgb24` payload into a 4-byte-per-
/// pixel `Rgba8UnormSrgb` payload with alpha = 0xFF (opaque).
///
/// Per the module-level mapping table: wgpu has no native
/// `Rgb8` sampler-bind format on the four backends Chromaticity
/// ships (the Vulkan `VK_FORMAT_R8G8B8_UNORM` path is optional and
/// unsupported on most desktop drivers per ADR-011 §Decision +
/// the wgpu format-support matrix), so the marshal repacks at the
/// CPU boundary. The expansion doubles the payload byte count
/// (`width * height * 3` source → `width * height * 4` upload).
fn repack_rgb24_to_rgba8(payload: &[u8]) -> Vec<u8> {
    // Pre-allocate 4/3 of the source size; the source MUST be a
    // multiple of 3 bytes per the `Rgb24` byte-order discipline,
    // but defensively round up if the producer ever supplies a
    // truncated final pixel (which the camera-adapter side
    // shouldn't, but a render-side defensive copy is cheap).
    let pixel_count = payload.len() / 3;
    let mut upload = Vec::with_capacity(pixel_count * 4);
    for px in payload.chunks_exact(3) {
        // Source bytes are R, G, B per
        // `chromaticity_camera_adapter::frames` module-level prose;
        // wgpu's `Rgba8UnormSrgb` expects R, G, B, A in memory.
        upload.push(px[0]);
        upload.push(px[1]);
        upload.push(px[2]);
        upload.push(0xFF);
    }
    upload
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod camera_adapter_drift_pins {
    //! Evidence: `EV-INTEGRATION-MATRIX` (T1) — engine-side
    //! camera → render texture-binding adapter drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: each test
    //! pins an ADR-020 §7 invariant against the implementation —
    //! symbol-namespace shape, pixel-format mapping table,
    //! slot-bound rejection, and the `Mjpeg`
    //! `CameraFormatNotSupported` rejection. Drift in any of these
    //! surfaces fails the corresponding pin at the exact ADR §
    //! boundary so review can route the change to the
    //! `EV-INTEGRATION-MATRIX` gate before merge.
    //!
    //! The deferred GPU-upload wave (per the module-level "Scope
    //! narrowing for PR 5") will widen this module with pins
    //! covering the live `wgpu::Queue::write_texture` upload + the
    //! `wgpu::Texture` / `wgpu::TextureView` / `wgpu::Sampler`
    //! resource lifecycle once the bridge body lands.

    use chromaticity_camera_adapter::{CameraFrame, PixelFormat};

    use super::{
        CAMERA_UNIFORM_NAMES, CameraTextureBinding, MAX_CAMERA_BINDING_SLOTS,
        camera_texture_binding_from_frame, camera_uniform_name,
    };
    use crate::single_pass::RenderError;

    fn make_frame(
        pixel_format: PixelFormat,
        width: u32,
        height: u32,
        payload: Vec<u8>,
    ) -> CameraFrame {
        CameraFrame {
            timestamp_ns: 1_000,
            width,
            height,
            pixel_format,
            payload,
        }
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — slot 0 marshal of a
    /// `Bgra32` frame emits the canonical `chroma_CameraInput0`
    /// symbol name and the `Bgra8UnormSrgb` wgpu format per
    /// ADR-020 §7.
    #[test]
    fn marshal_bgra32_frame_emits_chroma_camera_input_zero() {
        // Distinct per-byte payload so the verbatim-copy invariant
        // is observable. 2x2 frame = 16 bytes BGRA.
        let payload = (0..16u8).collect::<Vec<u8>>();
        let frame = make_frame(PixelFormat::Bgra32, 2, 2, payload.clone());
        let binding = camera_texture_binding_from_frame(&frame, 0).unwrap();
        assert_eq!(binding.uniform_name, "chroma_CameraInput0");
        assert_eq!(binding.wgpu_format, wgpu::TextureFormat::Bgra8UnormSrgb);
        assert_eq!(binding.source_pixel_format, PixelFormat::Bgra32);
        assert_eq!(binding.width, 2);
        assert_eq!(binding.height, 2);
        assert_eq!(
            binding.upload_bytes, payload,
            "Bgra32 upload must be verbatim"
        );
        assert!(binding.chroma_plane.is_none(), "Bgra32 has no chroma plane");
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — successive marshals at
    /// slot 0 / 1 / 2 / 3 produce the four canonical
    /// `chroma_CameraInput[0..=3]` symbol names in order.
    #[test]
    fn marshal_multiple_frames_indexed_correctly() {
        let payload = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let frame = make_frame(PixelFormat::Bgra32, 1, 1, payload);
        for (slot, expected) in CAMERA_UNIFORM_NAMES.iter().enumerate() {
            let binding = camera_texture_binding_from_frame(&frame, slot).unwrap();
            assert_eq!(
                binding.uniform_name, *expected,
                "slot {slot} symbol name must match canonical table"
            );
        }
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — marshal of an `Mjpeg`
    /// frame surfaces [`RenderError::CameraFormatNotSupported`]
    /// per the module-level "Scope narrowing for PR 5" deferral.
    #[test]
    fn marshal_mjpeg_returns_format_not_supported() {
        // Token JPEG SOI bytes; payload contents are immaterial
        // to the rejection.
        let frame = make_frame(PixelFormat::Mjpeg, 320, 240, vec![0xFF, 0xD8, 0xFF, 0xE0]);
        let err = camera_texture_binding_from_frame(&frame, 0).unwrap_err();
        match err {
            RenderError::CameraFormatNotSupported { format, reason } => {
                assert_eq!(format, "Mjpeg");
                assert!(
                    reason.contains("JPEG decode"),
                    "rejection reason must reference JPEG decode deferral, got {reason:?}"
                );
            }
            other => panic!("expected CameraFormatNotSupported, got {other:?}"),
        }
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — the `chroma_CameraInput[N]`
    /// symbol-index range is bounded at `0..=3` per ADR-020 §7.
    /// Out-of-range slot indices surface as
    /// [`RenderError::UniformBindingFailed`] with a triage message
    /// citing the ADR §.
    #[test]
    fn chroma_camera_input_symbol_index_within_zero_to_three() {
        // Confirm the constant matches the ADR-020 §7 cap.
        assert_eq!(
            MAX_CAMERA_BINDING_SLOTS, 4,
            "ADR-020 §7 caps the chroma_CameraInput[N] family at 4 slots"
        );
        // Confirm the table length matches.
        assert_eq!(CAMERA_UNIFORM_NAMES.len(), MAX_CAMERA_BINDING_SLOTS);
        // Confirm the lookup function bounds-checks.
        for slot in 0..MAX_CAMERA_BINDING_SLOTS {
            assert!(
                camera_uniform_name(slot).is_some(),
                "slot {slot} must yield a canonical name"
            );
        }
        assert!(
            camera_uniform_name(MAX_CAMERA_BINDING_SLOTS).is_none(),
            "slot index 4 must yield None"
        );
        // Confirm the marshal surfaces the rejection.
        let frame = make_frame(PixelFormat::Bgra32, 1, 1, vec![0, 0, 0, 0]);
        let err = camera_texture_binding_from_frame(&frame, 4).unwrap_err();
        match err {
            RenderError::UniformBindingFailed {
                pass_label,
                name,
                reason,
            } => {
                assert_eq!(pass_label, "(camera-binding)");
                assert_eq!(name, "chroma_CameraInput4");
                assert!(
                    reason.contains("ADR-020 §7"),
                    "out-of-range rejection must cite ADR-020 §7 in triage \
                     message, got {reason:?}"
                );
            }
            other => panic!("expected UniformBindingFailed, got {other:?}"),
        }
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — the pixel-format →
    /// `wgpu::TextureFormat` mapping table per ADR-020 §7. Drift
    /// in any mapping fails this pin at the exact mapping row.
    #[test]
    fn pixel_format_to_wgpu_texture_format_round_trip() {
        // Bgra32 → Bgra8UnormSrgb (4 bpp verbatim).
        let bgra = make_frame(PixelFormat::Bgra32, 1, 1, vec![1, 2, 3, 4]);
        let bgra_binding = camera_texture_binding_from_frame(&bgra, 0).unwrap();
        assert_eq!(
            bgra_binding.wgpu_format,
            wgpu::TextureFormat::Bgra8UnormSrgb
        );
        assert_eq!(bgra_binding.upload_bytes, vec![1, 2, 3, 4]);

        // Rgb24 → Rgba8UnormSrgb (3 → 4 bpp expansion w/ alpha = 0xFF).
        let rgb = make_frame(PixelFormat::Rgb24, 1, 1, vec![1, 2, 3]);
        let rgb_binding = camera_texture_binding_from_frame(&rgb, 0).unwrap();
        assert_eq!(rgb_binding.wgpu_format, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!(
            rgb_binding.upload_bytes,
            vec![1, 2, 3, 0xFF],
            "Rgb24 → Rgba8UnormSrgb expansion must opaque-fill alpha"
        );

        // Nv12 → R8Unorm (luma plane only, width*height bytes).
        // 2x2 frame = 4 luma bytes + 2 chroma bytes (4:2:0 sub-sample).
        let nv12 = make_frame(PixelFormat::Nv12, 2, 2, vec![10, 20, 30, 40, 128, 128]);
        let nv12_binding = camera_texture_binding_from_frame(&nv12, 0).unwrap();
        assert_eq!(nv12_binding.wgpu_format, wgpu::TextureFormat::R8Unorm);
        assert_eq!(
            nv12_binding.upload_bytes,
            vec![10, 20, 30, 40],
            "Nv12 PR-5 narrowing emits luma plane only"
        );
        assert!(
            nv12_binding.chroma_plane.is_none(),
            "PR 5 chroma_plane is None per scope narrowing"
        );

        // Yuyv → Rg8Unorm (verbatim packed, width*2 bytes per row).
        // 2x1 frame = 4 bytes Y0 U Y1 V.
        let yuyv = make_frame(PixelFormat::Yuyv, 2, 1, vec![100, 128, 200, 128]);
        let yuyv_binding = camera_texture_binding_from_frame(&yuyv, 0).unwrap();
        assert_eq!(yuyv_binding.wgpu_format, wgpu::TextureFormat::Rg8Unorm);
        assert_eq!(yuyv_binding.upload_bytes, vec![100, 128, 200, 128]);
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — zero-dim frames are
    /// rejected at the marshal boundary as
    /// [`RenderError::TargetSizeInvalid`] (mirrors the slice-1
    /// single-pass rejection rule).
    #[test]
    fn marshal_zero_dim_frame_rejects_with_target_size_invalid() {
        let zero_w = make_frame(PixelFormat::Bgra32, 0, 16, vec![]);
        let err = camera_texture_binding_from_frame(&zero_w, 0).unwrap_err();
        assert!(
            matches!(err, RenderError::TargetSizeInvalid((0, 16))),
            "zero-width frame must reject as TargetSizeInvalid((0, 16)), \
             got {err:?}"
        );

        let zero_h = make_frame(PixelFormat::Bgra32, 16, 0, vec![]);
        let err = camera_texture_binding_from_frame(&zero_h, 0).unwrap_err();
        assert!(
            matches!(err, RenderError::TargetSizeInvalid((16, 0))),
            "zero-height frame must reject as TargetSizeInvalid((16, 0)), \
             got {err:?}"
        );
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — the marshal is pure;
    /// same input produces the same output.
    #[test]
    fn marshal_is_pure_same_input_same_output() {
        let frame = make_frame(PixelFormat::Bgra32, 4, 4, (0..64u8).collect::<Vec<u8>>());
        let a = camera_texture_binding_from_frame(&frame, 2).unwrap();
        let b = camera_texture_binding_from_frame(&frame, 2).unwrap();
        assert_eq!(a, b, "marshal MUST be deterministic");
        let _: &CameraTextureBinding = &a; // type-check the carrier shape
    }
}
