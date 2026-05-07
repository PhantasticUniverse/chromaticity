// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Reference-app user-data import per ADR-015.
//!
//! Phase-4b Wave 4b.7 PR 1 of N — foundation crate-internal module
//! delivering the discovery surface plus the smallest two artifact
//! classes (settings + scene-specific presets). Subsequent PRs add
//! Snap slots / playlists / MIDI / OSC / hotkey-remap importers and
//! the dry-run + commit + rollback drill harness composing the
//! per-class importers per `docs/26-evidence-artifact-registry.md`
//! row 85 (`EV-IMPORT-REFERENCE-SHOW`) and ADR-015 §Mechanism.
//!
//! # Clean-room posture
//!
//! Per ADR-015 line 20 plus
//! `docs/29-cleanroom-legal-policy-baseline.md` Brand/Trademark
//! Review Checklist, no reference-app product name surfaces in any
//! shipped Chromaticity field name, log message, doc comment, JSON
//! key, or operator-facing copy. The neutral wording posture is
//! "reference-app" / "reference user data". The importer reads the
//! reference JSON as data; no reference source code is consulted
//! for transform logic. Transform table entries are derived from
//! observable shape + the Chromaticity schema.
//!
//! # Pattern: dry-run / commit / rollback
//!
//! Every per-class importer follows the same three-phase pattern
//! per ADR-015 §Mechanism §2-§6:
//!
//! 1. **Dry run** — parse reference JSON into an internal diff model;
//!    produce per-class counts and diagnostic lists; never mutate
//!    Chromaticity user data. Diagnostics carry `dry_run = true`.
//! 2. **Commit** — apply the diff atomically against the relevant
//!    Chromaticity store after operator confirmation; emit any
//!    fired-at-commit-time diagnostics with `dry_run = false`.
//!    (PR 4 of this wave introduces the commit harness; PR 1
//!    ships dry-run only.)
//! 3. **Rollback** — restore the pre-commit snapshot byte-identical
//!    per `docs/32-state-preset-playlist-migration-policy.md`. (PR
//!    4 of this wave introduces the rollback drill; out of scope
//!    here.)
//!
//! The importer never mutates the source reference-app user-data
//! folder per ADR-015 §Mechanism §7 (read-only).
//!
//! # Per-class diagnostic surface
//!
//! [`ImportDiagnostic`] carries (source path, class, code,
//! recommended action, dry-run flag) per ADR-015 §Diagnostics
//! lines 54-70. The eleven diagnostic codes ([`ImportDiagnosticCode`])
//! cover unknown-scene / unknown-control / outlier-control-type /
//! coercion-applied / unsupported-schema-version / hotkey-conflict
//! plus the V1-locked / control-focus-locked posture for hotkeys
//! plus unknown-action / osc-alias-collision / midi-mapping-device-
//! missing.
//!
//! T1 evidence: EV-IMPORT-REFERENCE-SHOW (Wave 4b.7 PR 1 of N).

pub mod diagnostics;
pub mod discovery;
pub mod drill;
pub mod hotkey;
pub mod meta_media;
pub mod midi;
pub mod osc;
pub mod playlists;
pub mod presets;
pub mod settings;
pub mod snap;

pub use diagnostics::{ArtifactClass, ImportDiagnostic, ImportDiagnosticCode};
pub use discovery::{
    DiscoveryError, ReferenceUserDataRoot, discover_reference_user_data_root,
    override_reference_user_data_root,
};
pub use drill::{
    CommitOutcome, DryRunReport, ImportClassCounts, ImportSelection, ReferenceImportDriver,
    RollbackSnapshot,
};
pub use hotkey::{HotkeyImporter, ImportedHotkey, ImportedHotkeyRemap, ImportedHotkeyStatus};
pub use meta_media::{
    ImportedMediaPreset, ImportedMetaMedia, ImportedMetaPreset, MetaMediaImporter,
};
pub use midi::{ImportedMidi, ImportedMidiBinding, ImportedMidiSource, MidiImporter};
pub use osc::{ImportedOsc, ImportedOscAlias, OscImporter};
pub use playlists::{
    ImportedIterationMode, ImportedPlaylist, ImportedPlaylists, ImportedTransition,
    PlaylistsImporter,
};
pub use presets::{ImportedPreset, ImportedPresets, PresetsImporter, PresetsImporterError};
pub use settings::{ImportedSettings, SettingsImporter, SettingsImporterError};
pub use snap::{ImportedSceneSnap, ImportedSlot, ImportedSnap, SnapImporter};
