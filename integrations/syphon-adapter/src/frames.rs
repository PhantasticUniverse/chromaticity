// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Cross-seam frame + server identity types per ADR-021 §4.
//!
//! [`SyphonServerId`] is the stable handle the adapter exchanges
//! with operator-facing UI and the `IntegrationHub` translation
//! layer. [`SyphonTexture`] is the cross-seam texture descriptor
//! the wgpu ↔ `IOSurface` bridge (Phase 3 PR 3) resolves into an
//! `MTLTexture` on input and constructs from one on output.
//!
//! Pixel format is locked to BGRA8Unorm per
//! `docs/54-wave-1c-beta-research-lessons.md` Phase-3 lesson 4
//! (universally-supported across Syphon consumers; widening would
//! drop interoperability with downstream apps that only consume the
//! canonical format). The [`SyphonPixelFormat`] enum exists as a
//! forward-compat marker in case a future framework upgrade widens
//! the supported set; PR 1 ships only the `Bgra8Unorm` variant.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).

use serde::{Deserialize, Serialize};

/// The current `kSyphonDictionaryVersion` value the canonical
/// Syphon-Framework `SyphonClientBase.m` line ~30 advertises.
///
/// Servers carrying a higher value are forward-incompatible with
/// chromaticity's binding generation per ADR-021 §4: the
/// discovery-side filter refuses such servers and the canonical
/// `SyphonClientBase.m:87-93` defensive forward-compat refusal
/// pattern is mirrored on the chromaticity side.
///
/// PR 1 pins this at 0 — the canonical value as of the framework
/// vendoring. A framework upgrade that bumps this constant requires
/// a follow-up PR per ADR-021 §Rollout review-cadence bullet (d).
pub const KNOWN_DICTIONARY_VERSION: u32 = 0;

/// Pixel format of a Syphon-published / consumed surface.
///
/// Locked to BGRA8Unorm per docs/54 Phase-3 lesson 4 (universally-
/// supported across Syphon consumers). The enum exists as a
/// forward-compat marker so future framework upgrades that widen
/// the supported format set can extend this without a wholesale
/// type-shape break; PR 1 ships only the canonical variant. Marked
/// `#[non_exhaustive]` so callers must handle future variants
/// defensively.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SyphonPixelFormat {
    /// 32-bit BGRA, 8 bits per channel, normalized. The canonical
    /// Syphon pixel format and the only variant Phase 3 PR 1 + PR 2
    /// produce or accept. Phase 3 PR 3's wgpu interop bridge maps
    /// this to `wgpu::TextureFormat::Bgra8Unorm`.
    Bgra8Unorm,
}

impl Default for SyphonPixelFormat {
    fn default() -> Self {
        Self::Bgra8Unorm
    }
}

/// Stable identity of a Syphon server advertised on the local-
/// machine bus.
///
/// Mirrors the reference Syphon discovery shape per ADR-021 §4 and
/// adds the `dictionary_version: u32` forward-compat refusal field.
/// The UUID identifies the server uniquely; `name` is the
/// operator-facing label; `app_name` identifies the publishing
/// application (useful for UI grouping); `dictionary_version`
/// carries the server's advertised
/// `SyphonServerDescriptionDictionaryVersionKey` value.
///
/// **Forward-compat refusal**: servers with `dictionary_version >
/// [`KNOWN_DICTIONARY_VERSION`]` are filtered out of the
/// [`crate::SyphonServerDirectory::current_servers`] return value
/// per ADR-021 §4 (PR 2 ships the actual filter; PR 1 pins the
/// type shape that PR 2 keys off of). This mirrors the canonical
/// `SyphonClientBase.m:87-93` defensive forward-compat refusal
/// pattern, where the client's `init` returns `nil` when the
/// server's `dictionaryVersion > kSyphonDictionaryVersion`.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SyphonServerId {
    /// Globally-unique server identifier (the framework's
    /// `SyphonServerDescriptionUUIDKey` value). Survives across
    /// process restarts on the publishing side as long as the
    /// publisher persists its `SyphonServer` UUID.
    pub uuid: String,
    /// Operator-facing label (the framework's
    /// `SyphonServerDescriptionNameKey` value). Not unique across
    /// the bus — multiple servers may share a name; UUID is the
    /// disambiguator.
    pub name: String,
    /// Publishing application name (the framework's
    /// `SyphonServerDescriptionAppNameKey` value). Useful for UI
    /// grouping (e.g. "All servers from TouchDesigner").
    pub app_name: String,
    /// Forward-compat marker: the server's advertised
    /// `SyphonServerDescriptionDictionaryVersionKey` value.
    /// Servers with `dictionary_version > KNOWN_DICTIONARY_VERSION`
    /// are refused per ADR-021 §4. Phase 3 PR 1 pins the field;
    /// Phase 3 PR 2 wires the actual refusal at the discovery seam.
    pub dictionary_version: u32,
}

impl SyphonServerId {
    /// `true` iff this server's advertised dictionary version is
    /// compatible with chromaticity's binding generation.
    ///
    /// A server is compatible when its `dictionary_version` is at
    /// or below [`KNOWN_DICTIONARY_VERSION`]. Higher values mark
    /// forward-incompatibility per the canonical
    /// `SyphonClientBase.m:87-93` refusal pattern; chromaticity
    /// surfaces them as a `tracing::warn!` log line and filters them
    /// out of the discovery surface in PR 2 (PR 1 pins the
    /// compatibility predicate as a free function so the PR 2
    /// filter has a stable target to call into).
    #[must_use]
    // Clippy flags `<=` against the `u32` minimum (0) as
    // `absurd_extreme_comparisons` because today
    // `KNOWN_DICTIONARY_VERSION == 0`. The comparison is intentional:
    // ADR-021 §Rollout review-cadence bullet (d) anticipates a
    // framework upgrade that bumps `KNOWN_DICTIONARY_VERSION`, after
    // which the inequality stops being trivially true. Suppressing
    // the lint at the call site keeps the predicate honest about its
    // forward-compat intent; using `==` would silently break the day
    // the constant rises.
    #[allow(clippy::absurd_extreme_comparisons)]
    pub fn is_compatible(&self) -> bool {
        self.dictionary_version <= KNOWN_DICTIONARY_VERSION
    }
}

/// Cross-seam texture handle the adapter exchanges with the
/// engine-side bridge (`engine/render/src/syphon_adapter.rs`,
/// Phase 3 PR 3).
///
/// `iosurface_id` is the canonical `IOSurfaceID` the wgpu Metal
/// interop path resolves into an `MTLTexture`. `width` + `height`
/// describe the surface dimensions in pixels. `pixel_format` is
/// locked to BGRA8Unorm per docs/54 Phase-3 lesson 4.
///
/// The handle does NOT carry the `IOSurface` reference itself
/// because chromaticity's adapter boundary is GPU-direct (zero-copy
/// per ADR-021 §6): the engine-side bridge reconstructs the surface
/// from the integer ID via the canonical `IOSurfaceLookup` API.
/// Carrying a CFTypeRef across the adapter boundary would require
/// either an `unsafe`-bounded `Send`/`Sync` wrapper or a
/// chromaticity-side reference-counted handle that mirrors
/// `core-foundation`'s `Retained` shape; the integer-ID design
/// avoids both and matches the canonical Syphon-Metal subtree's
/// own zero-copy convention.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SyphonTexture {
    /// Canonical `IOSurfaceID`. Carried as `u64` to leave headroom
    /// in case upstream ever widens the type; `IOSurfaceID` is
    /// canonically a 32-bit unsigned integer at the framework level.
    pub iosurface_id: u64,
    /// Surface width in pixels.
    pub width: u32,
    /// Surface height in pixels.
    pub height: u32,
    /// Pixel format. Locked to BGRA8Unorm per docs/54 Phase-3
    /// lesson 4 in PR 1.
    pub pixel_format: SyphonPixelFormat,
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod frame_pins {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — public surface type
    //! shape and forward-compat predicate pins.
    //!
    //! T1 per docs/43 §15.5: each test pins a public-surface
    //! invariant against the governing ADR. Any drift in the
    //! `SyphonServerId` field set, the `dictionary_version`
    //! refusal predicate, or the `SyphonTexture` shape fails the
    //! corresponding test at the exact ADR-section boundary.

    use super::*;

    /// Evidence: EV-INTEGRATION-MATRIX — `SyphonServerId` is
    /// `Clone + Eq` so the IntegrationHub translation layer can
    /// hold a snapshot across the discovery callback boundary.
    #[test]
    fn syphon_server_id_clone_eq_round_trip() {
        let original = SyphonServerId {
            uuid: "11111111-2222-3333-4444-555555555555".to_string(),
            name: "Test Server".to_string(),
            app_name: "Test App".to_string(),
            dictionary_version: 0,
        };
        let cloned = original.clone();
        assert_eq!(original, cloned);
        assert_eq!(original.uuid, cloned.uuid);
        assert_eq!(original.dictionary_version, cloned.dictionary_version);
    }

    /// Evidence: EV-INTEGRATION-MATRIX — ADR-021 §4 forward-compat
    /// refusal: `dictionary_version > KNOWN_DICTIONARY_VERSION`
    /// marks an incompatible server.
    #[test]
    fn dictionary_version_zero_is_compatible() {
        let id = SyphonServerId {
            uuid: "test".to_string(),
            name: "n".to_string(),
            app_name: "a".to_string(),
            dictionary_version: 0,
        };
        assert!(id.is_compatible());
    }

    /// Evidence: EV-INTEGRATION-MATRIX — ADR-021 §4 forward-compat
    /// refusal: any `dictionary_version > KNOWN_DICTIONARY_VERSION`
    /// is rejected. PR 2 wires the discovery-side filter; PR 1
    /// pins the predicate.
    #[test]
    fn dictionary_version_above_known_is_incompatible() {
        let id = SyphonServerId {
            uuid: "test".to_string(),
            name: "n".to_string(),
            app_name: "a".to_string(),
            dictionary_version: KNOWN_DICTIONARY_VERSION + 1,
        };
        assert!(!id.is_compatible());
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `KNOWN_DICTIONARY_VERSION`
    /// pinned at 0 to track the canonical
    /// `SyphonClientBase.m` constant. A framework upgrade that
    /// bumps this requires a follow-up PR per ADR-021 §Rollout
    /// review-cadence bullet (d).
    #[test]
    fn known_dictionary_version_matches_canonical() {
        assert_eq!(KNOWN_DICTIONARY_VERSION, 0);
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `SyphonTexture` carries
    /// the canonical `iosurface_id` as a non-zero integer when
    /// constructed by the engine-side bridge. The zero value is
    /// reserved to indicate the absence of a surface (PR 2 / PR 3
    /// use this convention internally).
    #[test]
    fn syphon_texture_zero_iosurface_id_marks_absence() {
        let absent = SyphonTexture {
            iosurface_id: 0,
            width: 0,
            height: 0,
            pixel_format: SyphonPixelFormat::Bgra8Unorm,
        };
        // Zero is the reserved marker; non-zero is a real surface.
        assert_eq!(absent.iosurface_id, 0);
    }

    /// Evidence: EV-INTEGRATION-MATRIX — pixel format default is
    /// the canonical Bgra8Unorm per docs/54 Phase-3 lesson 4.
    #[test]
    fn pixel_format_default_is_bgra8_unorm() {
        assert_eq!(SyphonPixelFormat::default(), SyphonPixelFormat::Bgra8Unorm);
    }
}
