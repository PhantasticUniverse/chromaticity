// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Per-class import diagnostic surface.
//!
//! Phase-4b Wave 4b.7 PR 1 of N — diagnostic shape per ADR-015
//! §Diagnostics lines 54-70. Every artifact that cannot be imported
//! cleanly emits an [`ImportDiagnostic`] carrying the source path,
//! artifact class, reason code, recommended operator action, and a
//! dry-run / committed flag. The eleven [`ImportDiagnosticCode`]
//! variants cover the diagnostic vocabulary of ADR-015 §Diagnostics.
//!
//! Distinct from the four-rung
//! [`crate::diagnostics::Diagnostic`] (compile / script / JSON /
//! coercion) used by the authoring console: the import diagnostic
//! is reported in the dry-run / commit summary surface, not the
//! authoring-time error pane, and per-class semantics carry
//! information (e.g. `MidiMappingDeviceMissing` is meaningful only
//! to the MIDI class) that does not fit the four-rung mapping.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Artifact classes supported by the reference-app importer per
/// ADR-015 §Scope (lines 17-29) and §Diagnostics line 56.
///
/// All seven variants are individually selectable in the dry-run
/// UI; operators may import any subset atomically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactClass {
    /// Audio device, output display, OSC ports, et al per ADR-015
    /// line 20.
    Settings,
    /// Per-scene named preset collections + startup pointers per
    /// ADR-015 line 21.
    Preset,
    /// Per-scene Snap slots 0-15 + gold-slot-0 pointer per ADR-015
    /// line 22 (reference literal: `favslots`).
    Snap,
    /// Scene lists with iteration mode + transitions per ADR-015
    /// line 24.
    Playlist,
    /// Per-scene + global CC/note->control bindings per ADR-015
    /// line 25.
    Midi,
    /// Route alias tables + source-IP allowlists per ADR-015 line
    /// 26.
    Osc,
    /// Per-user operator-hotkey rebindings per ADR-015 line 27.
    Hotkey,
}

impl std::fmt::Display for ArtifactClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Settings => "settings",
            Self::Preset => "preset",
            Self::Snap => "snap",
            Self::Playlist => "playlist",
            Self::Midi => "midi",
            Self::Osc => "osc",
            Self::Hotkey => "hotkey",
        };
        f.write_str(s)
    }
}

/// Reason codes per ADR-015 §Diagnostics lines 57-68.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImportDiagnosticCode {
    /// Preset or Snap references a scene name not present in the
    /// Chromaticity library.
    UnknownScene,
    /// A control `NAME` does not exist on the target scene.
    UnknownControl,
    /// A control type is not in the CP-01 canonical set.
    OutlierControlType,
    /// A CR-01..04 coercion was used.
    CoercionApplied,
    /// A reference artifact shape does not match any supported
    /// schema heuristic.
    UnsupportedSchemaVersion,
    /// Imported hotkey collides with another imported binding.
    HotkeyConflict,
    /// Imported hotkey differs from the V1 locked binding and is
    /// stored dormant pending V2.
    HotkeyV1Locked,
    /// Import attempted to rebind a control-focused D/R/P/L hotkey
    /// (never rebindable per FR-17).
    HotkeyControlFocusLocked,
    /// Imported hotkey references an action that has no
    /// Chromaticity equivalent.
    UnknownAction,
    /// Imported OSC alias would shadow a canonical Chromaticity
    /// route.
    OscAliasCollision,
    /// Imported MIDI mapping references a device ID not present on
    /// the importing machine.
    MidiMappingDeviceMissing,
}

impl std::fmt::Display for ImportDiagnosticCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::UnknownScene => "unknown-scene",
            Self::UnknownControl => "unknown-control",
            Self::OutlierControlType => "outlier-control-type",
            Self::CoercionApplied => "coercion-applied",
            Self::UnsupportedSchemaVersion => "unsupported-schema-version",
            Self::HotkeyConflict => "hotkey-conflict",
            Self::HotkeyV1Locked => "hotkey-v1-locked",
            Self::HotkeyControlFocusLocked => "hotkey-control-focus-locked",
            Self::UnknownAction => "unknown-action",
            Self::OscAliasCollision => "osc-alias-collision",
            Self::MidiMappingDeviceMissing => "midi-mapping-device-missing",
        };
        f.write_str(s)
    }
}

/// Single diagnostic entry per ADR-015 §Diagnostics lines 54-70.
///
/// Carries (source path, class, code, recommended operator action,
/// dry-run / committed flag). Importers append to a `Vec<_>` of
/// these in their dry-run output; the commit harness re-emits at
/// commit time with `dry_run = false` when the diagnostic still
/// applies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportDiagnostic {
    /// Source path of the offending reference-app artifact.
    pub source_path: PathBuf,
    /// Artifact class (`settings` / `preset` / `snap` / `playlist`
    /// / `midi` / `osc` / `hotkey`).
    pub class: ArtifactClass,
    /// Reason code per [`ImportDiagnosticCode`].
    pub code: ImportDiagnosticCode,
    /// Recommended operator action — Chromaticity-original copy;
    /// no reference-app product name appears here per ADR-015
    /// line 20 + `docs/29-cleanroom-legal-policy-baseline.md`.
    pub recommended_action: String,
    /// `true` while produced by the dry-run pass; `false` once the
    /// commit harness re-emits a still-applicable diagnostic at
    /// commit time.
    pub dry_run: bool,
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-IMPORT-REFERENCE-SHOW (T1) — Phase-4b Wave 4b.7
    //! PR 1 of N foundation diagnostic surface per ADR-015
    //! §Diagnostics and `docs/26-evidence-artifact-registry.md` row
    //! 85.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: in-source
    //! pins on the diagnostic vocabulary (every variant's Display
    //! formatting + dry-run / committed flag round-trip). The
    //! multi-fixture corpus pairing diagnostics with real importer
    //! emissions lives in
    //! `tests/import_reference_foundation_corpus.rs` at T2.

    use super::*;

    // EV-IMPORT-REFERENCE-SHOW T1: every ArtifactClass variant has
    // a stable lower-case Display formatting that matches the
    // ADR-015 §Diagnostics line 56 reason set.
    #[test]
    fn display_round_trip_artifact_class() {
        assert_eq!(ArtifactClass::Settings.to_string(), "settings");
        assert_eq!(ArtifactClass::Preset.to_string(), "preset");
        assert_eq!(ArtifactClass::Snap.to_string(), "snap");
        assert_eq!(ArtifactClass::Playlist.to_string(), "playlist");
        assert_eq!(ArtifactClass::Midi.to_string(), "midi");
        assert_eq!(ArtifactClass::Osc.to_string(), "osc");
        assert_eq!(ArtifactClass::Hotkey.to_string(), "hotkey");
    }

    // EV-IMPORT-REFERENCE-SHOW T1: every ImportDiagnosticCode
    // variant Display-formats to its kebab-case ADR-015 reason
    // string verbatim (matched against ADR-015 lines 57-68).
    #[test]
    fn display_round_trip_diagnostic_codes() {
        assert_eq!(
            ImportDiagnosticCode::UnknownScene.to_string(),
            "unknown-scene"
        );
        assert_eq!(
            ImportDiagnosticCode::UnknownControl.to_string(),
            "unknown-control"
        );
        assert_eq!(
            ImportDiagnosticCode::OutlierControlType.to_string(),
            "outlier-control-type"
        );
        assert_eq!(
            ImportDiagnosticCode::CoercionApplied.to_string(),
            "coercion-applied"
        );
        assert_eq!(
            ImportDiagnosticCode::UnsupportedSchemaVersion.to_string(),
            "unsupported-schema-version"
        );
        assert_eq!(
            ImportDiagnosticCode::HotkeyConflict.to_string(),
            "hotkey-conflict"
        );
        assert_eq!(
            ImportDiagnosticCode::HotkeyV1Locked.to_string(),
            "hotkey-v1-locked"
        );
        assert_eq!(
            ImportDiagnosticCode::HotkeyControlFocusLocked.to_string(),
            "hotkey-control-focus-locked"
        );
        assert_eq!(
            ImportDiagnosticCode::UnknownAction.to_string(),
            "unknown-action"
        );
        assert_eq!(
            ImportDiagnosticCode::OscAliasCollision.to_string(),
            "osc-alias-collision"
        );
        assert_eq!(
            ImportDiagnosticCode::MidiMappingDeviceMissing.to_string(),
            "midi-mapping-device-missing"
        );
    }

    // EV-IMPORT-REFERENCE-SHOW T1: a Diagnostic carries dry_run vs
    // committed correctly across clone-with-flag-toggle, and
    // round-trips through serde JSON byte-identically.
    #[test]
    fn diagnostic_marks_dry_run_vs_committed() {
        let dry = ImportDiagnostic {
            source_path: PathBuf::from("/synthetic/Settings.json"),
            class: ArtifactClass::Settings,
            code: ImportDiagnosticCode::UnsupportedSchemaVersion,
            recommended_action: "review the reference user-data layout".to_string(),
            dry_run: true,
        };
        let committed = ImportDiagnostic {
            dry_run: false,
            ..dry.clone()
        };
        assert!(dry.dry_run);
        assert!(!committed.dry_run);
        assert_eq!(dry.source_path, committed.source_path);
        assert_eq!(dry.class, committed.class);
        assert_eq!(dry.code, committed.code);

        let serialized = serde_json::to_string(&dry).unwrap();
        let parsed: ImportDiagnostic = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed, dry);
    }
}
