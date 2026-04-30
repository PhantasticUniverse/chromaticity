// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! MVP capability namespace constants per ADR-009 §Capability namespace.
//!
//! The strings exposed here are a parsing target for the fixture pack:
//! they let test code populate [`crate::token::Token::capabilities`]
//! from a known-good source and pin the namespace alongside the
//! `entitlement-v2` schema. None of these strings are wired to a
//! runtime entitlement evaluator — that work is Phase 4a
//! (`EV-ENTITLEMENT-ENFORCEMENT`).
//!
//! # Categories (ADR-009)
//!
//! - **Watermark and distribution.** [`RENDER_WATERMARK_OFF`].
//! - **I/O adapters (reference-parity capability split).**
//!   [`IO_OSC_INPUT`], [`IO_OSC_OUTPUT`], [`IO_SYPHON_INPUT`],
//!   [`IO_SYPHON_OUTPUT`], [`IO_SPOUT_INPUT`], [`IO_SPOUT_OUTPUT`],
//!   [`IO_NDI_INPUT`], [`IO_NDI_OUTPUT`], [`IO_CAMERA`].
//! - **Authoring.** [`AUTHORING_IDE`], [`AUTHORING_IMPORT`],
//!   [`AUTHORING_EDIT`], [`AUTHORING_BUILTIN_SCENES`].
//! - **Rendering.** [`RENDER_CUSTOM_RESOLUTION`].
//! - **Meta and audio.** [`META_REACTIVITY`], [`META_AUDIO_SPEED`].
//! - **Reserved (post-MVP).** [`DATA_INGEST_NETWORK`],
//!   [`DATA_INGEST_GEOSPATIAL`], [`DATA_INGEST_SDR`].
//!
//! Per ADR-009 §External data ingestion, the `data.ingest.*`
//! capabilities are reserved for V2/V3; an MVP-era token MAY carry
//! them, the runtime parses them, but every entitlement query against
//! them resolves to `false` until the data-ingest subsystem ships.
//! This crate (Phase 0) only pins the namespace strings.

// --- Watermark and distribution -------------------------------------------

/// Removes the watermark overlay. Any non-empty, valid entitlement
/// token implies this in MVP; post-MVP paid SKUs require it
/// explicitly. See ADR-009 §Capability namespace, "Watermark and
/// distribution".
pub const RENDER_WATERMARK_OFF: &str = "render.watermark.off";

// --- I/O adapters ---------------------------------------------------------

/// OSC input adapter capability.
pub const IO_OSC_INPUT: &str = "io.osc.input";

/// OSC output adapter capability.
pub const IO_OSC_OUTPUT: &str = "io.osc.output";

/// Syphon input adapter capability (macOS).
pub const IO_SYPHON_INPUT: &str = "io.syphon.input";

/// Syphon output adapter capability (macOS).
pub const IO_SYPHON_OUTPUT: &str = "io.syphon.output";

/// Spout input adapter capability (Windows).
pub const IO_SPOUT_INPUT: &str = "io.spout.input";

/// Spout output adapter capability (Windows).
pub const IO_SPOUT_OUTPUT: &str = "io.spout.output";

/// NDI input adapter capability.
pub const IO_NDI_INPUT: &str = "io.ndi.input";

/// NDI output adapter capability.
pub const IO_NDI_OUTPUT: &str = "io.ndi.output";

/// Camera ingestion capability (free at MVP per ADR-009; listed so
/// post-MVP gating is a data change, not a code change).
pub const IO_CAMERA: &str = "io.camera";

// --- Authoring -----------------------------------------------------------

/// In-app authoring environment ("IDE" in the reference).
pub const AUTHORING_IDE: &str = "authoring.ide";

/// Shadertoy / ISF import capability.
pub const AUTHORING_IMPORT: &str = "authoring.import";

/// Scene editing capability.
pub const AUTHORING_EDIT: &str = "authoring.edit";

/// Access to bundled built-in scenes.
pub const AUTHORING_BUILTIN_SCENES: &str = "authoring.builtin-scenes";

// --- Rendering -----------------------------------------------------------

/// Operator-settable render resolution.
pub const RENDER_CUSTOM_RESOLUTION: &str = "render.custom-resolution";

// --- Meta and audio ------------------------------------------------------

/// Reactivity meta-control (Differentiation profile, docs/36 §3.6).
pub const META_REACTIVITY: &str = "meta.reactivity";

/// Audio-speed meta-control (Differentiation profile, docs/36 §3.6).
pub const META_AUDIO_SPEED: &str = "meta.audio-speed";

// --- Reserved (post-MVP) -------------------------------------------------

/// Network-transported external data sources. Reserved for V2;
/// inactive at MVP per ADR-009.
pub const DATA_INGEST_NETWORK: &str = "data.ingest.network";

/// Geospatial basemap / tile providers. Reserved for V2 per ADR-009;
/// split from generic network because per-tile commercial licensing
/// differs materially.
pub const DATA_INGEST_GEOSPATIAL: &str = "data.ingest.geospatial";

/// Software-defined-radio / USB-device-backed RF ingestion. Reserved
/// for V3 per ADR-009 / `41-data-ingestion-differentiator.md` §Scope.
pub const DATA_INGEST_SDR: &str = "data.ingest.sdr";

/// Every MVP-era capability string declared in ADR-009 §Capability
/// namespace, in declaration order. Used by fixture tests to assert
/// that the namespace stays in sync with the ADR.
pub const ALL: &[&str] = &[
    RENDER_WATERMARK_OFF,
    IO_OSC_INPUT,
    IO_OSC_OUTPUT,
    IO_SYPHON_INPUT,
    IO_SYPHON_OUTPUT,
    IO_SPOUT_INPUT,
    IO_SPOUT_OUTPUT,
    IO_NDI_INPUT,
    IO_NDI_OUTPUT,
    IO_CAMERA,
    AUTHORING_IDE,
    AUTHORING_IMPORT,
    AUTHORING_EDIT,
    AUTHORING_BUILTIN_SCENES,
    RENDER_CUSTOM_RESOLUTION,
    META_REACTIVITY,
    META_AUDIO_SPEED,
    DATA_INGEST_NETWORK,
    DATA_INGEST_GEOSPATIAL,
    DATA_INGEST_SDR,
];

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod drift_pins {
    //! Evidence: EV-ENTITLEMENT-TOKEN-FORMAT (T1) — drift pins on the
    //! ADR-009 capability namespace declarations.
    //!
    //! T1 per docs/43 §15.5: each pin asserts an external-spec
    //! constant (the dotted capability strings as enumerated in
    //! `docs/adr/ADR-009-activation-and-entitlements.md` §Capability
    //! namespace). Any drift in spelling, ordering, or set membership
    //! fires the moment the ADR's namespace shifts.
    //!
    //! Capability strings cited:
    //!
    //! - `render.watermark.off` (ADR-009 line 44).
    //! - `io.osc.input`, `io.osc.output` (line 47).
    //! - `io.syphon.input`, `io.syphon.output` (line 48).
    //! - `io.spout.input`, `io.spout.output` (line 48).
    //! - `io.ndi.input`, `io.ndi.output` (line 49).
    //! - `io.camera` (line 50).
    //! - `authoring.ide`, `authoring.import`, `authoring.edit`,
    //!   `authoring.builtin-scenes` (lines 53-56).
    //! - `render.custom-resolution` (line 59).
    //! - `meta.reactivity`, `meta.audio-speed` (line 62).
    //! - `data.ingest.network`, `data.ingest.geospatial`,
    //!   `data.ingest.sdr` (lines 65-67).

    use super::*;

    #[test]
    fn watermark_capability_spells_dotted_path() {
        assert_eq!(RENDER_WATERMARK_OFF, "render.watermark.off");
    }

    #[test]
    fn io_capabilities_use_input_output_split() {
        assert_eq!(IO_OSC_INPUT, "io.osc.input");
        assert_eq!(IO_OSC_OUTPUT, "io.osc.output");
        assert_eq!(IO_NDI_INPUT, "io.ndi.input");
        assert_eq!(IO_NDI_OUTPUT, "io.ndi.output");
        assert_eq!(IO_SYPHON_INPUT, "io.syphon.input");
        assert_eq!(IO_SYPHON_OUTPUT, "io.syphon.output");
        assert_eq!(IO_SPOUT_INPUT, "io.spout.input");
        assert_eq!(IO_SPOUT_OUTPUT, "io.spout.output");
        assert_eq!(IO_CAMERA, "io.camera");
    }

    #[test]
    fn authoring_capabilities_use_dot_separator_and_kebab_leaves() {
        assert_eq!(AUTHORING_IDE, "authoring.ide");
        assert_eq!(AUTHORING_IMPORT, "authoring.import");
        assert_eq!(AUTHORING_EDIT, "authoring.edit");
        assert_eq!(AUTHORING_BUILTIN_SCENES, "authoring.builtin-scenes");
    }

    #[test]
    fn render_custom_resolution_uses_kebab_leaf() {
        assert_eq!(RENDER_CUSTOM_RESOLUTION, "render.custom-resolution");
    }

    #[test]
    fn meta_capabilities_use_kebab_audio_speed() {
        assert_eq!(META_REACTIVITY, "meta.reactivity");
        assert_eq!(META_AUDIO_SPEED, "meta.audio-speed");
    }

    #[test]
    fn data_ingest_capabilities_use_three_segment_path() {
        assert_eq!(DATA_INGEST_NETWORK, "data.ingest.network");
        assert_eq!(DATA_INGEST_GEOSPATIAL, "data.ingest.geospatial");
        assert_eq!(DATA_INGEST_SDR, "data.ingest.sdr");
    }

    #[test]
    fn all_includes_every_declared_capability() {
        // Sanity: ADR-009 §Capability namespace lists exactly 20
        // dotted strings across the seven categories above. Pin the
        // count so a silent insertion or deletion fires.
        assert_eq!(ALL.len(), 20);
    }

    #[test]
    fn all_entries_are_unique() {
        // No duplicate or aliased entries; the namespace is a set.
        let mut sorted: Vec<&'static str> = ALL.to_vec();
        sorted.sort_unstable();
        let mut deduped = sorted.clone();
        deduped.dedup();
        assert_eq!(sorted.len(), deduped.len());
    }
}
