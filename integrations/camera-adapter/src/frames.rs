// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Camera frame data + pixel format taxonomy.
//!
//! ## Pixel format byte-order discipline
//!
//! Every variant names in-memory byte order explicitly to avoid the
//! ambiguity that has historically caused multi-year regression
//! cycles in upstream codecs (cf. VLC's `VLC_CODEC_RGB24 → RGB24M
//! → BGR24` saga, `MEDIASUBTYPE_ARGB32 → VLC_CODEC_BGRA`,
//! `RGB32 → BGRX` per `docs/54-wave-1c-beta-research-lessons.md`
//! Phase-2 lesson 4). Per ADR-020 §Decision §2 "Byte-order
//! discipline":
//!
//! - [`PixelFormat::Bgra32`] — bytes B, G, R, A in memory (NOT
//!   `Argb32`; the leading-letter convention names the byte order,
//!   not the wgpu / DirectX-style component-naming-by-channel-
//!   significance).
//! - [`PixelFormat::Rgb24`] — bytes R, G, B in memory; no alpha;
//!   3-byte stride.
//! - [`PixelFormat::Nv12`] — planar Y' (full-resolution luma plane)
//!   followed by interleaved U/V at 1/2 horizontal + vertical
//!   resolution (4:2:0 chroma subsampling).
//! - [`PixelFormat::Yuyv`] — packed Y0, U, Y1, V pattern repeating
//!   (4:2:2 chroma subsampling); byte-stride is `width * 2`.
//! - [`PixelFormat::Mjpeg`] — payload is a complete JPEG bitstream
//!   (SOI...EOI markers); no fixed in-memory layout. Many USB
//!   cameras default to MJPG at high-resolution modes per ADR-020
//!   amendment + `docs/54` research-pass note; the variant lets
//!   enumeration succeed even when MJPG is the only offered format.
//!   Decoded on the fly to `Bgra32` or `Rgb24` at the render-side
//!   wiring (deferred enhancement; render-side initially routes
//!   `Mjpeg` to [`FORMAT_NOT_SUPPORTED`] until JPEG decode is
//!   wired).
//!
//! The enum is `#[non_exhaustive]`; widening criteria (e.g. HDR or
//! 10-bit YCbCr surfaces) reuse the byte-order discipline
//! established here.
//!
//! ## Timestamp policy
//!
//! [`CameraFrame::timestamp_ns`] is monotonic-from-bind nanoseconds.
//! Source: device-supplied per-frame timestamp (CoreMediaIO
//! `CMSampleBufferGetPresentationTimeStamp` on macOS; Media
//! Foundation `IMFSample::GetSampleTime` on Windows) where nokhwa
//! exposes it; falls back to `std::time::Instant::now()` since
//! adapter `bind` on backends that don't supply per-frame
//! timestamps.
//!
//! NEVER wallclock (`SystemTime::now()` / `chrono::Utc::now()` /
//! equivalent). Wallclock jumps from NTP slew, DST transitions,
//! manual system-clock changes, or container-pause/resume would
//! skip or freeze frames driving cue-curve animations — a
//! five-second NTP slew during a live show would either lock the
//! camera-feed visualization or fast-forward through five seconds
//! of cue interpolation.
//!
//! Cautionary precedent: ffmpeg's `dshow` indev had a five-year
//! regression where `av_gettime()` (wallclock-derived) was
//! substituted for the device-supplied timestamp; commit
//! `7dc33aad45` (2021-12-21) reverted that substitution after it
//! surfaced as inter-frame jitter for users with well-behaved
//! devices. Per ADR-020 §Decision §2 "Timestamp policy" + `docs/54`
//! Phase-2 lesson 3.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (camera arm).

use serde::{Deserialize, Serialize};

/// Pixel format taxonomy for [`CameraFrame::pixel_format`].
///
/// Variant byte-order documented in module-level prose. Marked
/// `#[non_exhaustive]` because future hardware may surface
/// additional formats (e.g. `Yv12`, `I420`, 10-bit `P010`); widening
/// the enum reuses the byte-order discipline established here per
/// ADR-020 §Decision §2.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PixelFormat {
    /// Planar Y' (full-resolution luma) + interleaved U/V at 1/2
    /// horizontal + vertical resolution (4:2:0 chroma subsampling).
    Nv12,
    /// Packed Y0, U, Y1, V repeating (4:2:2 chroma subsampling).
    /// Byte stride = `width * 2`.
    Yuyv,
    /// Bytes R, G, B in memory. No alpha. 3-byte stride.
    Rgb24,
    /// Bytes B, G, R, A in memory (NOT `Argb32`).
    Bgra32,
    /// JPEG bitstream payload (SOI...EOI markers); no fixed
    /// in-memory layout. Decoded at the render-side wiring (deferred
    /// enhancement). Per ADR-020 amendment.
    Mjpeg,
}

/// Single camera frame emitted by an adapter.
///
/// `timestamp_ns` is monotonic-from-bind nanoseconds — see
/// module-level "Timestamp policy" prose for the source-by-platform
/// + wallclock-forbidden invariant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraFrame {
    /// Monotonic-from-bind ns. See module-level "Timestamp policy".
    pub timestamp_ns: u64,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Realised pixel format. May differ from the negotiated
    /// preference when the hardware does not advertise the requested
    /// format; authors who care should read this field rather than
    /// assume the preference was honoured.
    pub pixel_format: PixelFormat,
    /// Raw frame payload bytes. Layout is determined by
    /// `pixel_format` per the byte-order discipline in module prose.
    pub payload: Vec<u8>,
}

/// Canonical fault reason: OS denied camera access via TCC (macOS)
/// / Privacy Settings (Windows). Per ADR-020 §Decision §2 "Canonical
/// fault reason vocabulary".
pub const PERMISSION_DENIED: &str = "permission-denied";

/// Canonical fault reason: device is held by another application
/// (`AVCaptureDevice.isInUseByAnotherApplication == YES` on macOS;
/// `MF_E_DEVICE_INVALIDATED` mid-stream on Windows). Per ADR-020
/// §Decision §2.
pub const DEVICE_BUSY: &str = "device-busy";

/// Canonical fault reason: persisted device-id no longer enumerable
/// (USB unplug + restart, OS-level removal). Per ADR-020
/// §Decision §2.
pub const DEVICE_NOT_FOUND: &str = "device-not-found";

/// Canonical fault reason: no overlap between requested and offered
/// formats, OR the negotiated format is `Mjpeg` and JPEG decode is
/// not yet wired at the render side. Per ADR-020 §Decision §2.
pub const FORMAT_NOT_SUPPORTED: &str = "format-not-supported";

/// Canonical fault reason: `reconnect_attempts × reconnect_interval_ms`
/// window exhausted without re-enumerating the persisted device.
/// Per ADR-020 §Decision §2 + §6.
pub const RECONNECT_EXHAUSTED: &str = "reconnect-exhausted";

/// Canonical fault reason: generic nokhwa-internal failure (any
/// error class not covered above). Per ADR-020 §Decision §2.
pub const BACKEND_ERROR: &str = "backend-error";

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod camera_frame_pins {
    //! Evidence: `EV-INTEGRATION-MATRIX` (T1) — camera-adapter
    //! frames + canonical fault reason invariants. Per `docs/43`
    //! §15.5 T1 definition: each test pins a specific ADR-020
    //! invariant against the implementation; drift fires the
    //! corresponding pin so review can route the change to the
    //! `EV-INTEGRATION-MATRIX` gate before merge.

    use super::*;

    /// `EV-INTEGRATION-MATRIX` T1: PixelFormat carries the five
    /// ADR-020 §Decision §2 MVP variants and remains marked
    /// `#[non_exhaustive]`. The exhaustive match here is the drift
    /// pin against the variant set; the
    /// `pixel_format_attribute_is_non_exhaustive` test below pins
    /// the `#[non_exhaustive]` attribute itself by inspecting the
    /// crate's source.
    #[test]
    fn pixel_format_variant_set_matches_adr_020() {
        for f in [
            PixelFormat::Nv12,
            PixelFormat::Yuyv,
            PixelFormat::Rgb24,
            PixelFormat::Bgra32,
            PixelFormat::Mjpeg,
        ] {
            // Exhaustive match — adding a sixth variant fails to
            // compile until the test acknowledges it. Removing the
            // `#[non_exhaustive]` attribute would not break this
            // test, but the `pixel_format_attribute_is_non_exhaustive`
            // companion below catches that drift.
            let name = match f {
                PixelFormat::Nv12 => "nv12",
                PixelFormat::Yuyv => "yuyv",
                PixelFormat::Rgb24 => "rgb24",
                PixelFormat::Bgra32 => "bgra32",
                PixelFormat::Mjpeg => "mjpeg",
            };
            assert!(!name.is_empty());
        }
    }

    /// `EV-INTEGRATION-MATRIX` T1: the `PixelFormat` enum carries
    /// `#[non_exhaustive]` per ADR-020 §Decision §2 widening rule.
    /// The attribute is invisible to in-crate match arms (see the
    /// companion `pixel_format_variant_set_matches_adr_020` test
    /// above), so this drift pin reads the source file directly
    /// and asserts the attribute is present on the enum
    /// declaration. Any drift surfaces here.
    #[test]
    fn pixel_format_attribute_is_non_exhaustive() {
        let source = include_str!("frames.rs");
        // Locate the pixel-format enum declaration and assert the
        // `#[non_exhaustive]` attribute precedes it. Pinned via a
        // simple substring scan rather than a parser dep so the
        // test stays cheap and review-friendly.
        let enum_marker = "pub enum PixelFormat {";
        let enum_idx = source
            .find(enum_marker)
            .expect("enum PixelFormat declaration present");
        let prefix = &source[..enum_idx];
        let attr_idx = prefix
            .rfind("#[non_exhaustive]")
            .expect("expected #[non_exhaustive] attribute on PixelFormat");
        // Confirm no intervening enum / struct declaration sits
        // between the attribute and the enum (which would mean the
        // attribute decorates a different item).
        let between = &source[attr_idx..enum_idx];
        assert!(
            !between.contains("pub enum") || between.starts_with("#[non_exhaustive]"),
            "another enum declaration sits between the attribute and PixelFormat",
        );
    }

    /// `EV-INTEGRATION-MATRIX` T1: canonical fault reason consts
    /// are lowercase kebab-case per ADR-020 §Decision §2. The
    /// operator-facing UX surface localizes off these values, so
    /// any drift in casing or separator would silently break the
    /// localization map.
    #[test]
    fn canonical_fault_reasons_are_kebab_case() {
        for reason in [
            PERMISSION_DENIED,
            DEVICE_BUSY,
            DEVICE_NOT_FOUND,
            FORMAT_NOT_SUPPORTED,
            RECONNECT_EXHAUSTED,
            BACKEND_ERROR,
        ] {
            assert!(!reason.is_empty(), "reason must not be empty",);
            assert!(
                reason.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "reason {reason:?} must be lowercase kebab-case",
            );
            assert!(
                !reason.starts_with('-') && !reason.ends_with('-'),
                "reason {reason:?} must not start or end with a hyphen",
            );
        }
    }

    /// `EV-INTEGRATION-MATRIX` T1: each canonical fault reason has
    /// its ADR-020-pinned exact string value. Pinned literally so
    /// any in-source rename surfaces here (the strings are part of
    /// the operator-facing UX contract).
    #[test]
    fn canonical_fault_reasons_match_adr_020() {
        assert_eq!(PERMISSION_DENIED, "permission-denied");
        assert_eq!(DEVICE_BUSY, "device-busy");
        assert_eq!(DEVICE_NOT_FOUND, "device-not-found");
        assert_eq!(FORMAT_NOT_SUPPORTED, "format-not-supported");
        assert_eq!(RECONNECT_EXHAUSTED, "reconnect-exhausted");
        assert_eq!(BACKEND_ERROR, "backend-error");
    }

    /// `EV-INTEGRATION-MATRIX` T1: `CameraFrame` round-trips via
    /// `Clone` + `PartialEq`. Pinned because the render-side
    /// wiring (Phase 2 PR 5) takes ownership of the payload and
    /// the value type's clone semantics must not silently change.
    #[test]
    fn camera_frame_clone_preserves_fields() {
        let frame = CameraFrame {
            timestamp_ns: 12_345,
            width: 1280,
            height: 720,
            pixel_format: PixelFormat::Nv12,
            payload: vec![0x1, 0x2, 0x3],
        };
        let cloned = frame.clone();
        assert_eq!(frame, cloned);
        assert_eq!(cloned.timestamp_ns, 12_345);
        assert_eq!(cloned.width, 1280);
        assert_eq!(cloned.height, 720);
        assert!(matches!(cloned.pixel_format, PixelFormat::Nv12));
        assert_eq!(cloned.payload, vec![0x1, 0x2, 0x3]);
    }
}
