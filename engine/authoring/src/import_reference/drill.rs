// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Dry-run / commit / rollback drill harness composing all eight
//! per-class reference user-data importers into a single orchestration
//! surface per ADR-015 §Mechanism §2-§6 and
//! `docs/26-evidence-artifact-registry.md` row 85
//! (`EV-IMPORT-REFERENCE-SHOW`).
//!
//! Phase-4b Wave 4b.7 PR 4 of N — composes:
//!
//! 1. [`SettingsImporter`] (PR 1)
//! 2. [`PresetsImporter`] (PR 1)
//! 3. [`SnapImporter`] (PR 2)
//! 4. [`PlaylistsImporter`] (PR 2)
//! 5. [`MetaMediaImporter`] (PR 2)
//! 6. [`MidiImporter`] (PR 3)
//! 7. [`OscImporter`] (PR 3)
//! 8. [`HotkeyImporter`] (PR 3)
//!
//! into a unified [`DryRunReport`] (per-class imports + flat
//! diagnostic list + per-class counts), a [`CommitOutcome`] that
//! re-emits diagnostics from the operator-selected classes with
//! `dry_run = false`, and a [`RollbackSnapshot`] capturing a
//! synthesized pre-commit JSON payload.
//!
//! # Synthetic commit + rollback semantics in PR 4
//!
//! The commit step in this PR is a **contract-shape harness** — it
//! produces a [`CommitOutcome`] + [`RollbackSnapshot`] proving the
//! orchestration shape compiles + tests cleanly, but does NOT mutate
//! any Chromaticity user-data store (no scene-library write, no
//! [`PresetStore`][^ps] / [`MappingStore`][^ms] / hotkey-store
//! mutation). Wiring the commit into the actual Chromaticity stores
//! is a separate post-MVP PR scoped behind the V1 entitlement-tier-
//! gating + scene-library-write surfaces; that follow-on PR also
//! drops the synthetic-no-op rollback per ADR-015 line 50 ("imports
//! write a pre-migration snapshot per
//! `32-state-preset-playlist-migration-policy.md`. Rollback restores
//! the Chromaticity user-data root to its pre-import state
//! atomically").
//!
//! [^ps]: scene-author-side preset store (V1 surface, separate wave).
//! [^ms]: integrations/midi-adapter MappingStore (V1 surface,
//!        separate wave).
//!
//! [`Self::rollback`][`ReferenceImportDriver::rollback`] is therefore
//! a no-op for this PR — there is no committed state to roll back to
//! — and returns `Ok(())` after recording the synthesized
//! [`RollbackSnapshot`] for shape-test purposes.
//!
//! # Diagnostic ordering invariant
//!
//! [`ReferenceImportDriver::dry_run`] aggregates per-class diagnostics
//! into the report's flat [`DryRunReport::diagnostics`] vector in the
//! canonical artifact-class order:
//!
//! `settings` → `presets` → `snap` → `playlists` → `meta_media` →
//! `midi` → `osc` → `hotkey`
//!
//! Within each class, the per-class importer's own ordering (file-
//! name-sorted for the multi-file classes, source-file-order for the
//! single-file classes) is preserved. The order is part of the public
//! contract: the v0.2-beta UI bundle's import-summary panel groups
//! diagnostics by class and counts on this ordering for stable
//! per-row indexing.
//!
//! # Clean-room neutral wording
//!
//! Per ADR-015 line 20 + `docs/29-cleanroom-legal-policy-baseline.md`
//! Brand/Trademark Review Checklist, no reference-app product name
//! appears in any field name, log message, doc comment, JSON key, or
//! operator-facing copy emitted from this module. The neutral phrasing
//! is "reference-app" / "reference user data".

use std::collections::HashSet;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::diagnostics::{ArtifactClass, ImportDiagnostic};
use super::discovery::ReferenceUserDataRoot;
use super::hotkey::{HotkeyImporter, ImportedHotkey};
use super::meta_media::{ImportedMetaMedia, MetaMediaImporter};
use super::midi::{ImportedMidi, MidiImporter};
use super::osc::{ImportedOsc, OscImporter};
use super::playlists::{ImportedPlaylists, PlaylistsImporter};
use super::presets::{ImportedPresets, PresetsImporter};
use super::settings::{ImportedSettings, SettingsImporter};
use super::snap::{ImportedSnap, SnapImporter};

/// Aggregated dry-run report across all eight artifact-class
/// importers per ADR-015 §Mechanism §2.
#[derive(Debug, Clone)]
pub struct DryRunReport {
    /// Settings imports (PR 1).
    pub settings: ImportedSettings,
    /// Scene-specific presets imports (PR 1).
    pub presets: ImportedPresets,
    /// Snap-slot imports (PR 2).
    pub snap: ImportedSnap,
    /// Playlist imports (PR 2).
    pub playlists: ImportedPlaylists,
    /// Meta + media preset imports (PR 2).
    pub meta_media: ImportedMetaMedia,
    /// MIDI binding imports (PR 3).
    pub midi: ImportedMidi,
    /// OSC alias imports (PR 3).
    pub osc: ImportedOsc,
    /// Hotkey-remap imports (PR 3).
    pub hotkey: ImportedHotkey,
    /// Flat diagnostic list aggregated in artifact-class order
    /// (settings → presets → snap → playlists → meta_media → midi
    /// → osc → hotkey). Each entry carries `dry_run = true` per
    /// ADR-015 §Mechanism §2.
    pub diagnostics: Vec<ImportDiagnostic>,
    /// Per-class counts for the operator-facing summary panel.
    pub class_counts: ImportClassCounts,
}

/// Per-class summary counts populated by
/// [`ReferenceImportDriver::dry_run`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportClassCounts {
    /// Number of known (typed-field-mapped) settings keys present.
    pub settings_known_keys: usize,
    /// Number of unknown settings keys preserved verbatim in
    /// `extensions_legacy`.
    pub settings_extension_keys: usize,
    /// Total number of presets across all imported scenes.
    pub preset_count: usize,
    /// Total number of populated Snap slots across all imported
    /// scenes.
    pub snap_slot_count: usize,
    /// Number of imported playlists.
    pub playlist_count: usize,
    /// Number of imported meta-channel presets.
    pub meta_preset_count: usize,
    /// Number of imported media-channel presets.
    pub media_preset_count: usize,
    /// Total number of MIDI bindings (per-scene + global).
    pub midi_binding_count: usize,
    /// Number of imported OSC aliases.
    pub osc_alias_count: usize,
    /// Number of imported hotkey remaps.
    pub hotkey_remap_count: usize,
}

/// Operator-controlled selection of which artifact classes to
/// commit. PR 4 ships the shape; the post-MVP follow-on PR wires
/// the per-class boolean against the actual Chromaticity user-data
/// stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSelection {
    /// Commit settings.
    pub settings: bool,
    /// Commit scene-specific presets.
    pub presets: bool,
    /// Commit Snap slots.
    pub snap: bool,
    /// Commit playlists.
    pub playlists: bool,
    /// Commit meta + media presets.
    pub meta_media: bool,
    /// Commit MIDI bindings.
    pub midi: bool,
    /// Commit OSC aliases.
    pub osc: bool,
    /// Commit hotkey remaps.
    pub hotkey: bool,
}

impl ImportSelection {
    /// Construct a selection with every artifact class enabled.
    #[must_use]
    pub fn all() -> Self {
        Self {
            settings: true,
            presets: true,
            snap: true,
            playlists: true,
            meta_media: true,
            midi: true,
            osc: true,
            hotkey: true,
        }
    }

    /// Construct a selection with every artifact class disabled.
    #[must_use]
    pub fn none() -> Self {
        Self {
            settings: false,
            presets: false,
            snap: false,
            playlists: false,
            meta_media: false,
            midi: false,
            osc: false,
            hotkey: false,
        }
    }

    /// Returns true iff the artifact `class` is enabled in this
    /// selection.
    #[must_use]
    pub fn includes(&self, class: ArtifactClass) -> bool {
        match class {
            ArtifactClass::Settings => self.settings,
            ArtifactClass::Preset => self.presets,
            ArtifactClass::Snap => self.snap,
            ArtifactClass::Playlist => self.playlists,
            ArtifactClass::Midi => self.midi,
            ArtifactClass::Osc => self.osc,
            ArtifactClass::Hotkey => self.hotkey,
        }
    }
}

/// Result of [`ReferenceImportDriver::commit`].
///
/// PR 4 synthesizes this from the input dry-run report — the actual
/// store-mutation wiring lands in the post-MVP follow-on PR (see
/// module-level prose).
#[derive(Debug, Clone)]
pub struct CommitOutcome {
    /// The selection passed to the commit step.
    pub committed_classes: ImportSelection,
    /// Snapshot capturing the pre-commit state so a future rollback
    /// step can restore it. Per PR 4's synthetic-commit semantics
    /// this is a contract-shape payload, not an actual user-data
    /// snapshot.
    pub rollback_snapshot: RollbackSnapshot,
    /// Diagnostics re-emitted at commit time from the selected
    /// classes only, with `dry_run = false` per ADR-015 §Mechanism
    /// §2.
    pub committed_diagnostics: Vec<ImportDiagnostic>,
}

/// Snapshot capturing the pre-commit state so a future rollback step
/// can restore it atomically per ADR-015 line 50.
///
/// PR 4 synthesizes the payload as a JSON object keyed by class name
/// — see module-level prose for the deferral of the real
/// `32-state-preset-playlist-migration-policy.md` snapshot machinery.
#[derive(Debug, Clone)]
pub struct RollbackSnapshot {
    /// Stable identifier for this snapshot (a process-unique stamp;
    /// not a UUID — the V1 wiring substitutes a content-addressed
    /// identifier scheme).
    pub snapshot_id: String,
    /// Wall-clock instant the snapshot was captured.
    pub captured_at: SystemTime,
    /// Synthesized pre-commit state payload, JSON object keyed by
    /// class name (`settings`, `presets`, `snap`, `playlists`,
    /// `meta_media`, `midi`, `osc`, `hotkey`).
    pub pre_commit_state: serde_json::Value,
}

/// Errors produced by the drill harness.
#[derive(Debug)]
pub enum ImporterError {
    /// Settings importer failed.
    Settings(super::settings::SettingsImporterError),
    /// Presets importer failed.
    Presets(super::presets::PresetsImporterError),
    /// Snap importer failed.
    Snap(super::snap::ImporterError),
    /// Playlists importer failed.
    Playlists(super::playlists::ImporterError),
    /// Meta+Media importer failed.
    MetaMedia(super::meta_media::ImporterError),
    /// MIDI importer failed.
    Midi(super::midi::ImporterError),
    /// OSC importer failed.
    Osc(super::osc::ImporterError),
    /// Hotkey importer failed.
    Hotkey(super::hotkey::ImporterError),
    /// Synthesized rollback-snapshot serialization failed.
    SnapshotSerialization(serde_json::Error),
}

impl std::fmt::Display for ImporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Settings(e) => write!(f, "settings importer failed: {e}"),
            Self::Presets(e) => write!(f, "presets importer failed: {e}"),
            Self::Snap(e) => write!(f, "snap importer failed: {e}"),
            Self::Playlists(e) => write!(f, "playlists importer failed: {e}"),
            Self::MetaMedia(e) => write!(f, "meta+media importer failed: {e}"),
            Self::Midi(e) => write!(f, "midi importer failed: {e}"),
            Self::Osc(e) => write!(f, "osc importer failed: {e}"),
            Self::Hotkey(e) => write!(f, "hotkey importer failed: {e}"),
            Self::SnapshotSerialization(e) => {
                write!(f, "rollback snapshot serialization failed: {e}")
            }
        }
    }
}

impl std::error::Error for ImporterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Settings(e) => Some(e),
            Self::Presets(e) => Some(e),
            Self::Snap(e) => Some(e),
            Self::Playlists(e) => Some(e),
            Self::MetaMedia(e) => Some(e),
            Self::Midi(e) => Some(e),
            Self::Osc(e) => Some(e),
            Self::Hotkey(e) => Some(e),
            Self::SnapshotSerialization(e) => Some(e),
        }
    }
}

/// Drill-harness driver composing all eight per-class importers.
pub struct ReferenceImportDriver;

impl ReferenceImportDriver {
    /// Aggregate every per-class importer's dry-run output into a
    /// single [`DryRunReport`]. Diagnostics are collected in the
    /// canonical artifact-class order (settings → presets → snap →
    /// playlists → meta_media → midi → osc → hotkey). Per ADR-015
    /// §Mechanism §7 the source root is never mutated.
    ///
    /// # Errors
    ///
    /// Returns the first per-class importer error encountered. Per-
    /// class diagnostics that the importer reports inside its `Ok`
    /// payload are aggregated rather than failing the call.
    pub fn dry_run(root: &ReferenceUserDataRoot) -> Result<DryRunReport, ImporterError> {
        let mut diagnostics: Vec<ImportDiagnostic> = Vec::new();

        let (settings, settings_diag) =
            SettingsImporter::dry_run(root).map_err(ImporterError::Settings)?;
        diagnostics.extend(settings_diag);

        // PR 1 module-level prose: the library cross-check accepts a
        // caller-provided `&HashSet<String>` of "known scene names"
        // (empty in PR 1 tests — meaning every scene name is
        // "unknown" — and the diagnostic semantics still exercise
        // correctly). The drill harness in PR 4 carries the same
        // "empty set means every scene name is unknown" posture
        // forward — the V1 follow-on PR substitutes a real
        // SceneIndex-derived set when the commit harness wires
        // against a live library snapshot.
        let known_scenes: HashSet<String> = HashSet::new();
        let (presets, presets_diag) =
            PresetsImporter::dry_run(root, &known_scenes).map_err(ImporterError::Presets)?;
        diagnostics.extend(presets_diag);

        let (snap, snap_diag) = SnapImporter::dry_run(root).map_err(ImporterError::Snap)?;
        diagnostics.extend(snap_diag);

        let (playlists, playlists_diag) =
            PlaylistsImporter::dry_run(root).map_err(ImporterError::Playlists)?;
        diagnostics.extend(playlists_diag);

        let (meta_media, meta_media_diag) =
            MetaMediaImporter::dry_run(root).map_err(ImporterError::MetaMedia)?;
        diagnostics.extend(meta_media_diag);

        let (midi, midi_diag) = MidiImporter::dry_run(root).map_err(ImporterError::Midi)?;
        diagnostics.extend(midi_diag);

        let (osc, osc_diag) = OscImporter::dry_run(root).map_err(ImporterError::Osc)?;
        diagnostics.extend(osc_diag);

        let (hotkey, hotkey_diag) = HotkeyImporter::dry_run(root).map_err(ImporterError::Hotkey)?;
        diagnostics.extend(hotkey_diag);

        let class_counts = compute_class_counts(
            &settings,
            &presets,
            &snap,
            &playlists,
            &meta_media,
            &midi,
            &osc,
            &hotkey,
        );

        Ok(DryRunReport {
            settings,
            presets,
            snap,
            playlists,
            meta_media,
            midi,
            osc,
            hotkey,
            diagnostics,
            class_counts,
        })
    }

    /// Commit the operator-selected subset of a dry-run report and
    /// produce a [`CommitOutcome`] carrying the re-emitted (selected-
    /// classes-only, `dry_run = false`) diagnostics plus a synthesized
    /// [`RollbackSnapshot`].
    ///
    /// Per the module-level prose this is a contract-shape harness;
    /// it does NOT mutate any Chromaticity user-data store.
    ///
    /// # Errors
    ///
    /// Returns [`ImporterError::SnapshotSerialization`] if the
    /// pre-commit-state JSON value cannot be constructed (will not
    /// fire under the synthetic semantics — `serde_json::to_value`
    /// against the importer output types never errors at the surface
    /// PR 4 ships, but the error-arm is plumbed for the V1 wiring).
    pub fn commit(
        report: &DryRunReport,
        selection: &ImportSelection,
    ) -> Result<CommitOutcome, ImporterError> {
        let committed_diagnostics: Vec<ImportDiagnostic> = report
            .diagnostics
            .iter()
            .filter(|d| selection.includes(d.class))
            .map(|d| ImportDiagnostic {
                dry_run: false,
                ..d.clone()
            })
            .collect();

        let rollback_snapshot = build_rollback_snapshot(report, selection)?;

        Ok(CommitOutcome {
            committed_classes: *selection,
            rollback_snapshot,
            committed_diagnostics,
        })
    }

    /// Restore the Chromaticity user-data root to the state captured
    /// in `snapshot` per ADR-015 line 50. PR 4 ships the synthetic
    /// no-op surface — there is no committed state to roll back to —
    /// and returns `Ok(())`. The V1 follow-on PR replaces the body
    /// with the real
    /// `32-state-preset-playlist-migration-policy.md`-aligned
    /// restoration.
    ///
    /// # Errors
    ///
    /// Never fails under PR 4 semantics. Plumbed as a `Result` so the
    /// V1 follow-on can surface restoration errors without changing
    /// the signature.
    pub fn rollback(_snapshot: &RollbackSnapshot) -> Result<(), ImporterError> {
        // PR 4 synthetic-no-op rollback. No state was committed; no
        // state to restore. The V1 follow-on PR replaces this body
        // with the real restoration path.
        Ok(())
    }
}

/// Build a [`RollbackSnapshot`] capturing a synthesized pre-commit
/// JSON payload keyed by class name. Helper extracted for clarity.
fn build_rollback_snapshot(
    report: &DryRunReport,
    selection: &ImportSelection,
) -> Result<RollbackSnapshot, ImporterError> {
    let mut payload = serde_json::Map::new();
    if selection.settings {
        payload.insert(
            "settings".to_string(),
            serde_json::to_value(&report.settings).map_err(ImporterError::SnapshotSerialization)?,
        );
    }
    if selection.presets {
        payload.insert(
            "presets".to_string(),
            serde_json::to_value(&report.presets).map_err(ImporterError::SnapshotSerialization)?,
        );
    }
    if selection.snap {
        payload.insert(
            "snap".to_string(),
            serde_json::to_value(&report.snap).map_err(ImporterError::SnapshotSerialization)?,
        );
    }
    if selection.playlists {
        payload.insert(
            "playlists".to_string(),
            serde_json::to_value(&report.playlists)
                .map_err(ImporterError::SnapshotSerialization)?,
        );
    }
    if selection.meta_media {
        payload.insert(
            "meta_media".to_string(),
            serde_json::to_value(&report.meta_media)
                .map_err(ImporterError::SnapshotSerialization)?,
        );
    }
    if selection.midi {
        payload.insert(
            "midi".to_string(),
            serde_json::to_value(&report.midi).map_err(ImporterError::SnapshotSerialization)?,
        );
    }
    if selection.osc {
        payload.insert(
            "osc".to_string(),
            serde_json::to_value(&report.osc).map_err(ImporterError::SnapshotSerialization)?,
        );
    }
    if selection.hotkey {
        payload.insert(
            "hotkey".to_string(),
            serde_json::to_value(&report.hotkey).map_err(ImporterError::SnapshotSerialization)?,
        );
    }

    let captured_at = SystemTime::now();
    // Process-unique stamp; the V1 wiring substitutes a content-
    // addressed identifier scheme per
    // `32-state-preset-playlist-migration-policy.md`.
    let stamp_nanos = captured_at
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let snapshot_id = format!("import-reference-show-snapshot-{stamp_nanos}");

    Ok(RollbackSnapshot {
        snapshot_id,
        captured_at,
        pre_commit_state: serde_json::Value::Object(payload),
    })
}

/// Compute per-class counts from each importer's output. Helper
/// extracted for clarity.
#[allow(clippy::too_many_arguments)]
fn compute_class_counts(
    settings: &ImportedSettings,
    presets: &ImportedPresets,
    snap: &ImportedSnap,
    playlists: &ImportedPlaylists,
    meta_media: &ImportedMetaMedia,
    midi: &ImportedMidi,
    osc: &ImportedOsc,
    hotkey: &ImportedHotkey,
) -> ImportClassCounts {
    let settings_known_keys = [
        settings.audio_device_id.is_some(),
        settings.output_display.is_some(),
        settings.osc_input_port.is_some(),
        settings.osc_output_port.is_some(),
    ]
    .iter()
    .filter(|b| **b)
    .count();

    let settings_extension_keys = match &settings.extensions_legacy {
        serde_json::Value::Object(map) => map.len(),
        _ => 0,
    };

    let preset_count: usize = presets.per_scene.values().map(Vec::len).sum();

    let snap_slot_count: usize = snap
        .per_scene
        .values()
        .map(|scene| scene.slots.iter().filter(|s| s.is_some()).count())
        .sum();

    let playlist_count = playlists.playlists.len();
    let meta_preset_count = meta_media.meta.len();
    let media_preset_count = meta_media.media.len();

    let midi_binding_count: usize =
        midi.per_scene.values().map(Vec::len).sum::<usize>() + midi.global.len();

    let osc_alias_count = osc.aliases.len();
    let hotkey_remap_count = hotkey.remaps.len();

    ImportClassCounts {
        settings_known_keys,
        settings_extension_keys,
        preset_count,
        snap_slot_count,
        playlist_count,
        meta_preset_count,
        media_preset_count,
        midi_binding_count,
        osc_alias_count,
        hotkey_remap_count,
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-IMPORT-REFERENCE-SHOW (T1) — Phase-4b Wave 4b.7
    //! PR 4 of N drill harness composing all eight per-class
    //! importers per ADR-015 §Mechanism §2-§6 and
    //! `docs/26-evidence-artifact-registry.md` row 85.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: in-source
    //! pins covering aggregation across all eight classes,
    //! diagnostic-ordering invariant, full + partial commit
    //! selection re-emission, synthesized rollback snapshot shape,
    //! synthetic-no-op rollback, count helpers, and selection
    //! helper round-trips. The multi-fixture corpus lives at
    //! `tests/import_reference_drill_harness_corpus.rs` at T2.
    use super::*;
    use crate::import_reference::override_reference_user_data_root;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique_scratch_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let idx = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "chromaticity-import-reference-drill-t1-{label}-{pid}-{nanos}-{idx}"
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn cleanup(dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Seed every artifact class with a single minimal valid
    /// payload. Used by multiple T1 pins to produce a non-empty
    /// dry-run report against which aggregation, counts, and
    /// commit semantics are exercised.
    fn seed_eight_class_root(dir: &Path) {
        std::fs::write(
            dir.join("Settings.json"),
            r#"{ "audioDevice": "device-x", "uiScaleFactor": 1.0 }"#,
        )
        .unwrap();

        let presets_dir = dir.join("Presets");
        std::fs::create_dir_all(&presets_dir).unwrap();
        std::fs::write(
            presets_dir.join("scene-a.json"),
            r#"{
                "scene": "scene-a",
                "presets": [{ "name": "p1", "controls": { "x": 0.5 } }]
            }"#,
        )
        .unwrap();

        let snap_dir = dir.join("Favslots");
        std::fs::create_dir_all(&snap_dir).unwrap();
        std::fs::write(
            snap_dir.join("scene-a.json"),
            r#"{
                "scene": "scene-a",
                "slots": [{ "index": 0, "controls": { "x": 0.25 } }]
            }"#,
        )
        .unwrap();

        let playlists_dir = dir.join("Playlists");
        std::fs::create_dir_all(&playlists_dir).unwrap();
        std::fs::write(
            playlists_dir.join("set.json"),
            r#"{ "name": "set-1", "scenes": ["scene-a"] }"#,
        )
        .unwrap();

        let meta_dir = dir.join("MetaPresets");
        std::fs::create_dir_all(&meta_dir).unwrap();
        std::fs::write(
            meta_dir.join("preset.json"),
            r#"{ "name": "meta-1", "controls": { "y": 0.1 } }"#,
        )
        .unwrap();

        let media_dir = dir.join("MediaPresets");
        std::fs::create_dir_all(&media_dir).unwrap();
        std::fs::write(
            media_dir.join("preset.json"),
            r#"{ "name": "media-1", "settings": { "fade": 0.2 } }"#,
        )
        .unwrap();

        let midi_dir = dir.join("MidiMappings");
        std::fs::create_dir_all(&midi_dir).unwrap();
        std::fs::write(
            midi_dir.join("scene-a.json"),
            r#"{
                "scene": "scene-a",
                "bindings": [{
                    "device_id": "controller-1",
                    "channel": 0,
                    "cc": 1,
                    "target_control": "fader.master"
                }]
            }"#,
        )
        .unwrap();

        std::fs::write(
            dir.join("OscAliases.json"),
            r#"{ "aliases": [{ "source_route": "/legacy/x", "target_route": "/chroma/y" }] }"#,
        )
        .unwrap();

        std::fs::write(
            dir.join("HotkeyRemaps.json"),
            r#"{ "remaps": [{ "source_key": "ctrl+1", "target_action": "scene.next" }] }"#,
        )
        .unwrap();
    }

    // EV-IMPORT-REFERENCE-SHOW T1: dry_run aggregates outputs from
    // all eight per-class importers into a single DryRunReport that
    // carries each class's imports + a per-class count.
    #[test]
    fn dry_run_aggregates_eight_class_imports() {
        let dir = unique_scratch_dir("aggregate");
        seed_eight_class_root(&dir);
        let root = override_reference_user_data_root(dir.clone()).unwrap();

        let report = ReferenceImportDriver::dry_run(&root).unwrap();

        assert_eq!(report.settings.audio_device_id.as_deref(), Some("device-x"));
        assert!(report.presets.per_scene.contains_key("scene-a"));
        assert!(report.snap.per_scene.contains_key("scene-a"));
        assert_eq!(report.playlists.playlists.len(), 1);
        assert!(report.meta_media.meta.contains_key("meta-1"));
        assert!(report.meta_media.media.contains_key("media-1"));
        assert!(report.midi.per_scene.contains_key("scene-a"));
        assert_eq!(report.osc.aliases.len(), 1);
        assert_eq!(report.hotkey.remaps.len(), 1);

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: dry_run aggregates per-class
    // diagnostics into the report's flat `diagnostics` vector in the
    // canonical artifact-class order (settings → presets → snap →
    // playlists → meta_media → midi → osc → hotkey).
    #[test]
    fn dry_run_aggregates_diagnostics_from_each_class_in_order() {
        let dir = unique_scratch_dir("ordering");

        // Settings: malformed -> Settings UnsupportedSchemaVersion.
        std::fs::write(dir.join("Settings.json"), b"not actually json {{").unwrap();

        // Presets: scene-a referenced (unknown to empty known-set) ->
        // Preset UnknownScene.
        let presets_dir = dir.join("Presets");
        std::fs::create_dir_all(&presets_dir).unwrap();
        std::fs::write(
            presets_dir.join("a.json"),
            r#"{ "scene": "s", "presets": [{ "name": "p", "controls": {} }] }"#,
        )
        .unwrap();

        // OSC: chroma-prefix -> Osc OscAliasCollision.
        std::fs::write(
            dir.join("OscAliases.json"),
            r#"{ "aliases": [{ "source_route": "/chroma/foo", "target_route": "/x" }] }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let report = ReferenceImportDriver::dry_run(&root).unwrap();

        let classes: Vec<ArtifactClass> = report.diagnostics.iter().map(|d| d.class).collect();
        // Filter to the three classes we seeded; they MUST appear in
        // the order Settings, Preset, Osc.
        let observed: Vec<ArtifactClass> = classes
            .into_iter()
            .filter(|c| {
                matches!(
                    c,
                    ArtifactClass::Settings | ArtifactClass::Preset | ArtifactClass::Osc
                )
            })
            .collect();
        assert_eq!(
            observed,
            vec![
                ArtifactClass::Settings,
                ArtifactClass::Preset,
                ArtifactClass::Osc,
            ]
        );

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: commit with full selection
    // re-emits every diagnostic from the dry-run report with
    // `dry_run = false`.
    #[test]
    fn commit_with_full_selection_re_emits_all_diagnostics_with_dry_run_false() {
        let dir = unique_scratch_dir("commit-full");

        // Plant a settings malformed-shape diagnostic + a hotkey
        // V1-locked diagnostic so the report carries diagnostics
        // from at least two classes.
        std::fs::write(dir.join("Settings.json"), b"not json").unwrap();
        std::fs::write(
            dir.join("HotkeyRemaps.json"),
            r#"{ "remaps": [{ "source_key": "ctrl+9", "target_action": "chroma.v1_locked.foo" }] }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let report = ReferenceImportDriver::dry_run(&root).unwrap();
        assert!(!report.diagnostics.is_empty());
        assert!(report.diagnostics.iter().all(|d| d.dry_run));

        let outcome = ReferenceImportDriver::commit(&report, &ImportSelection::all()).unwrap();
        assert_eq!(
            outcome.committed_diagnostics.len(),
            report.diagnostics.len()
        );
        assert!(outcome.committed_diagnostics.iter().all(|d| !d.dry_run));

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: commit with a partial selection
    // omits diagnostics from unselected classes.
    #[test]
    fn commit_with_partial_selection_omits_unselected_class_diagnostics() {
        let dir = unique_scratch_dir("commit-partial");
        std::fs::write(dir.join("Settings.json"), b"not json").unwrap();
        std::fs::write(
            dir.join("HotkeyRemaps.json"),
            r#"{ "remaps": [{ "source_key": "ctrl+9", "target_action": "chroma.v1_locked.foo" }] }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let report = ReferenceImportDriver::dry_run(&root).unwrap();

        // Select only Settings; expect only Settings diagnostics.
        let selection = ImportSelection {
            settings: true,
            ..ImportSelection::none()
        };
        let outcome = ReferenceImportDriver::commit(&report, &selection).unwrap();

        assert!(
            outcome
                .committed_diagnostics
                .iter()
                .all(|d| d.class == ArtifactClass::Settings)
        );
        assert!(!outcome.committed_diagnostics.is_empty());

        // No hotkey diagnostic should appear in the committed
        // outcome despite living in the dry-run report.
        let hotkey_in_report = report
            .diagnostics
            .iter()
            .any(|d| d.class == ArtifactClass::Hotkey);
        assert!(hotkey_in_report);
        let hotkey_in_outcome = outcome
            .committed_diagnostics
            .iter()
            .any(|d| d.class == ArtifactClass::Hotkey);
        assert!(!hotkey_in_outcome);

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: commit produces a RollbackSnapshot
    // whose `pre_commit_state` is a JSON object containing keys for
    // every selected class.
    #[test]
    fn commit_produces_rollback_snapshot_with_synthesized_pre_commit_state() {
        let dir = unique_scratch_dir("snapshot");
        seed_eight_class_root(&dir);
        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let report = ReferenceImportDriver::dry_run(&root).unwrap();

        let outcome = ReferenceImportDriver::commit(&report, &ImportSelection::all()).unwrap();
        let snapshot = &outcome.rollback_snapshot;

        assert!(
            snapshot
                .snapshot_id
                .starts_with("import-reference-show-snapshot-")
        );
        let serde_json::Value::Object(ref payload) = snapshot.pre_commit_state else {
            panic!("expected pre_commit_state to be a JSON object");
        };
        for class_key in [
            "settings",
            "presets",
            "snap",
            "playlists",
            "meta_media",
            "midi",
            "osc",
            "hotkey",
        ] {
            assert!(payload.contains_key(class_key), "missing key: {class_key}");
        }

        // Partial selection: only the selected classes appear in the
        // payload.
        let selection = ImportSelection {
            settings: true,
            midi: true,
            ..ImportSelection::none()
        };
        let outcome = ReferenceImportDriver::commit(&report, &selection).unwrap();
        let serde_json::Value::Object(ref payload) = outcome.rollback_snapshot.pre_commit_state
        else {
            panic!("expected pre_commit_state to be a JSON object");
        };
        assert!(payload.contains_key("settings"));
        assert!(payload.contains_key("midi"));
        assert!(!payload.contains_key("hotkey"));
        assert!(!payload.contains_key("presets"));

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: rollback returns Ok(()) under
    // PR 4's documented synthetic-no-op semantics.
    #[test]
    fn rollback_is_synthetic_no_op_per_pr_4_documented_deferral() {
        let dir = unique_scratch_dir("rollback");
        seed_eight_class_root(&dir);
        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let report = ReferenceImportDriver::dry_run(&root).unwrap();
        let outcome = ReferenceImportDriver::commit(&report, &ImportSelection::all()).unwrap();

        let result = ReferenceImportDriver::rollback(&outcome.rollback_snapshot);
        assert!(result.is_ok());

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: ImportClassCounts match the
    // underlying importer outputs (one preset, one slot, one
    // playlist, one meta + one media, one midi binding, one osc
    // alias, one hotkey remap, one known settings key + one
    // extension key against the seeded synthetic root).
    #[test]
    fn import_class_counts_match_underlying_importer_outputs() {
        let dir = unique_scratch_dir("counts");
        seed_eight_class_root(&dir);
        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let report = ReferenceImportDriver::dry_run(&root).unwrap();
        let c = &report.class_counts;

        assert_eq!(c.settings_known_keys, 1, "audioDevice -> 1 known");
        assert_eq!(c.settings_extension_keys, 1, "uiScaleFactor -> 1 extension");
        assert_eq!(c.preset_count, 1);
        assert_eq!(c.snap_slot_count, 1);
        assert_eq!(c.playlist_count, 1);
        assert_eq!(c.meta_preset_count, 1);
        assert_eq!(c.media_preset_count, 1);
        assert_eq!(c.midi_binding_count, 1);
        assert_eq!(c.osc_alias_count, 1);
        assert_eq!(c.hotkey_remap_count, 1);

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: ImportSelection::all() and
    // ::none() produce the expected per-field state and
    // ImportSelection::includes routes by class.
    #[test]
    fn import_selection_all_and_none_helpers_round_trip() {
        let all = ImportSelection::all();
        assert!(all.settings && all.presets && all.snap && all.playlists);
        assert!(all.meta_media && all.midi && all.osc && all.hotkey);
        for class in [
            ArtifactClass::Settings,
            ArtifactClass::Preset,
            ArtifactClass::Snap,
            ArtifactClass::Playlist,
            ArtifactClass::Midi,
            ArtifactClass::Osc,
            ArtifactClass::Hotkey,
        ] {
            assert!(all.includes(class));
        }

        let none = ImportSelection::none();
        assert!(!none.settings && !none.presets && !none.snap && !none.playlists);
        assert!(!none.meta_media && !none.midi && !none.osc && !none.hotkey);
        for class in [
            ArtifactClass::Settings,
            ArtifactClass::Preset,
            ArtifactClass::Snap,
            ArtifactClass::Playlist,
            ArtifactClass::Midi,
            ArtifactClass::Osc,
            ArtifactClass::Hotkey,
        ] {
            assert!(!none.includes(class));
        }
    }
}
