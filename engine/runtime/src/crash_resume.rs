// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4d Wave 4d.3 PR 1 — `EV-CRASH-RESUME` partial-advance
//! state-restoration adapter for class-a + class-b (row 80).
//!
//! Composes the Phase-3 EV-STATE-MIGRATION versioned snapshot
//! envelope (sibling [`crate::state_migration`]) with a disk-backed
//! save / recover cycle plus per-class restoration helpers. The
//! partial-advance shape mirrors the Phase-3 narrowing precedent
//! (rows 33-36): subsequent PRs in this wave cluster widen the
//! restoration surface to class-c (playlist position + iteration
//! mode), class-d (OSC/MIDI session bindings), class-e (audio input
//! device + `syn_Threshold`), class-f (operator meta-controls), and
//! class-g (render resolution + FPS target).
//!
//! # Class-a + class-b mapping onto current canonical state
//!
//! Per `docs/26-evidence-artifact-registry.md` row 80:
//!
//! - **Class-a — active scene + preset channel values.** Today the
//!   canonical [`State`] carries the preset selection
//!   ([`State::active_preset`]) and the control-value store
//!   ([`State::control_value`]) that holds the per-channel preset
//!   values. The "active scene" concept is reserved as a future
//!   addition — when it lands, [`restore_class_a`] gains the
//!   corresponding sub-store assignment and the corpus widens.
//! - **Class-b — Snap slot selection + scene-specific preset
//!   history.** Today the canonical [`State`] carries Snap slot
//!   selection ([`State::active_snap_slot`] +
//!   [`State::snap_slot_saved`]). Scene-specific preset history is
//!   reserved as a future addition — when it lands,
//!   [`restore_class_b`] gains the corresponding sub-store
//!   assignment.
//!
//! Class-by-class restoration is the partial-advance signal the wave
//! cluster relies on: class-c through class-g land in subsequent PRs
//! by widening the per-class helpers without changing the on-disk
//! envelope shape.
//!
//! # Crash-injection harness scope (PR 1)
//!
//! The on-disk path uses an ordinary `std::fs::write` /
//! `std::fs::read` cycle through the configured save path. The
//! "crash-injection" harness in [`tests`] and the sibling
//! `engine/runtime/tests/crash_resume_corpus/main.rs` T2 corpus
//! simulate a crash IN-PROCESS by:
//!
//! 1. building a [`State`] and mutating it through the canonical
//!    command-bus path,
//! 2. invoking [`CrashResumeAdapter::save`] to write the snapshot
//!    blob,
//! 3. dropping the original adapter + state (the in-process analogue
//!    of process termination),
//! 4. constructing a fresh [`CrashResumeAdapter`] pointing at the
//!    same save path and a fresh [`State::default`] (the in-process
//!    analogue of the host re-launching),
//! 5. invoking [`CrashResumeAdapter::try_recover`] to deserialize
//!    the saved snapshot, then [`restore_all_supported_classes`] (or
//!    [`restore_class_a`] / [`restore_class_b`] selectively) to
//!    project the restored values onto the fresh state, and
//! 6. asserting the per-class observable values match the pre-crash
//!    state.
//!
//! Real cross-process `SIGKILL` / `SIGSEGV` / panic-in-render-thread
//! / OOM injection (the row-80 Generation Source) lands in a
//! subsequent PR alongside the time-to-resume + visible-artifact
//! window p50/p95 measurement budgets. PR 1 ships the correctness
//! seam: the adapter, the per-class restoration helpers, and the
//! in-process harness scaffold.
//!
//! # Composition with Phase-3 surfaces
//!
//! The on-disk encoder is
//! [`crate::state_migration::encode_snapshot`] (V2 envelope); the
//! decoder is [`crate::state_migration::decode_snapshot`]. This
//! choice (a) pulls forward the EV-STATE-MIGRATION
//! `V2SchemaMetadata` extension trailer so a future schema bump
//! transparently fires `DecodeError::UnknownVersion`, and (b)
//! preserves the EV-STATE-REPLAY (row 32) + EV-PRESET-PLAYLIST (row
//! 34) goldens — the canonical hash path
//! ([`crate::state_replay::canonical_state_hash`]) ignores the
//! envelope trailer.
//!
//! # Anchors
//!
//! - `docs/26-evidence-artifact-registry.md` row 80 — `EV-CRASH-RESUME`.
//! - `docs/adr/ADR-004-state-model.md` — canonical state model.
//! - `docs/adr/ADR-006-thread-and-process-model.md` — single-writer
//!   commit-thread invariant.

use crate::state::State;
use crate::state_migration::{DecodeError, decode_snapshot, encode_snapshot};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// Opaque byte container for a serialized state snapshot.
///
/// The internal shape is the [`crate::state_migration`] V2 envelope
/// when produced by [`CanonicalBinarySnapshotter`]; alternative
/// snapshotter implementations (JSON / msgpack / etc.) wrap their
/// own bytes. Callers treat the contents as opaque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSnapshotBlob(pub Vec<u8>);

impl StateSnapshotBlob {
    /// Borrow the underlying byte slice.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Errors surfaced by the crash-resume adapter and snapshotter
/// implementations.
///
/// `#[non_exhaustive]` so future PRs in the wave cluster widen the
/// surface (e.g. class-c playlist-iteration-mode-mismatch,
/// class-d device-id-collision) without an SemVer-major break.
#[derive(Debug)]
#[non_exhaustive]
pub enum CrashResumeError {
    /// The snapshotter could not serialize / deserialize the blob.
    /// Wraps the [`crate::state_migration::DecodeError`] surfaced by
    /// the canonical-binary decoder; alternative snapshotter impls
    /// surface their own diagnostic strings via this variant's
    /// payload.
    SerializationError(SerializationKind),
    /// The save / recover path failed at the filesystem boundary.
    Io(io::Error),
    /// The snapshot envelope's schema version did not match a known
    /// version. Reserved for forward-compatibility: today the
    /// `state_migration` decoder surfaces this as
    /// `DecodeError::UnknownVersion` wrapped in
    /// `SerializationError`; this variant exists so callers can
    /// match on a typed reason without inspecting the wrapped
    /// `DecodeError`.
    VersionMismatch,
}

/// Concrete serialization-kind payload carried by
/// [`CrashResumeError::SerializationError`]. Distinguishes between
/// canonical-binary envelope drift and a future alternative
/// snapshotter's diagnostic string.
#[derive(Debug)]
#[non_exhaustive]
pub enum SerializationKind {
    /// The Phase-3 EV-STATE-MIGRATION canonical-binary envelope
    /// decoder rejected the buffer. Wraps the typed reason so
    /// fixtures can match on the precise variant.
    CanonicalBinary(DecodeError),
}

impl fmt::Display for CrashResumeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SerializationError(kind) => match kind {
                SerializationKind::CanonicalBinary(e) => {
                    write!(f, "snapshot serialization error: {e}")
                }
            },
            Self::Io(e) => write!(f, "snapshot io error: {e}"),
            Self::VersionMismatch => f.write_str("snapshot schema version mismatch"),
        }
    }
}

impl std::error::Error for CrashResumeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SerializationError(SerializationKind::CanonicalBinary(e)) => Some(e),
            Self::Io(e) => Some(e),
            Self::VersionMismatch => None,
        }
    }
}

impl From<io::Error> for CrashResumeError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<DecodeError> for CrashResumeError {
    fn from(value: DecodeError) -> Self {
        Self::SerializationError(SerializationKind::CanonicalBinary(value))
    }
}

/// Abstract snapshot serializer.
///
/// `pub` so subsequent PRs in the wave cluster can swap in
/// alternative serialization formats (JSON / canonical-binary /
/// msgpack) without changing [`CrashResumeAdapter`]'s on-disk
/// path. The default implementation
/// [`CanonicalBinarySnapshotter`] uses the Phase-3 V2 envelope.
pub trait StateSnapshotter {
    /// Serialize `state` into a [`StateSnapshotBlob`].
    fn snapshot(&self, state: &State) -> Result<StateSnapshotBlob, CrashResumeError>;

    /// Deserialize `blob` into a fresh [`State`].
    fn restore(&self, blob: &StateSnapshotBlob) -> Result<State, CrashResumeError>;
}

/// Canonical-binary snapshotter using the Phase-3
/// EV-STATE-MIGRATION V2 envelope.
///
/// Round-trips `State` losslessly across the canonical-binary
/// sub-store contents (control / preset / playlist / Snap /
/// mapping). The `latest_audio_snapshot` field on `State` is
/// transient ephemeral (the scheduler-tick repopulates it on the
/// next pull) and is intentionally NOT serialized — matching the
/// row-80 design note that "audio trailing reactivity state
/// resets on resume by design".
#[derive(Debug, Default, Clone, Copy)]
pub struct CanonicalBinarySnapshotter;

impl StateSnapshotter for CanonicalBinarySnapshotter {
    fn snapshot(&self, state: &State) -> Result<StateSnapshotBlob, CrashResumeError> {
        // `encode_snapshot` is infallible at the producer side; the
        // V2 envelope writer cannot fail on a well-formed `State`.
        Ok(StateSnapshotBlob(encode_snapshot(state)))
    }

    fn restore(&self, blob: &StateSnapshotBlob) -> Result<State, CrashResumeError> {
        decode_snapshot(blob.as_bytes()).map_err(CrashResumeError::from)
    }
}

/// Disk-backed crash-resume adapter.
///
/// Composes a [`StateSnapshotter`] with a file-system save path.
/// [`Self::save`] writes the snapshot blob; [`Self::try_recover`]
/// reads it back if present. The adapter does NOT implicitly
/// truncate or remove the save file on recovery — host integrations
/// decide when to rotate or delete the snapshot (e.g. on graceful
/// shutdown). Subsequent PRs in the wave cluster may add a
/// `discard_snapshot` helper alongside the broader class-c..g
/// surface.
#[derive(Debug)]
pub struct CrashResumeAdapter<S: StateSnapshotter> {
    snapshotter: S,
    save_path: PathBuf,
}

impl<S: StateSnapshotter> CrashResumeAdapter<S> {
    /// Construct an adapter writing snapshots to `save_path`.
    pub fn new(snapshotter: S, save_path: impl Into<PathBuf>) -> Self {
        Self {
            snapshotter,
            save_path: save_path.into(),
        }
    }

    /// Borrow the configured save path.
    #[must_use]
    pub fn save_path(&self) -> &Path {
        &self.save_path
    }

    /// Serialize `state` and persist the blob to the configured save
    /// path. Overwrites any existing snapshot.
    pub fn save(&mut self, state: &State) -> Result<(), CrashResumeError> {
        let blob = self.snapshotter.snapshot(state)?;
        std::fs::write(&self.save_path, blob.as_bytes())?;
        Ok(())
    }

    /// Attempt to read a previously-saved snapshot from the save
    /// path. Returns `Ok(None)` when the file does not exist —
    /// crash-on-first-launch surfaces as a clean no-op rather than
    /// an error. Read or decode failures surface as the matching
    /// [`CrashResumeError`] variant.
    pub fn try_recover(&self) -> Result<Option<State>, CrashResumeError> {
        match std::fs::read(&self.save_path) {
            Ok(bytes) => {
                let blob = StateSnapshotBlob(bytes);
                let state = self.snapshotter.restore(&blob)?;
                Ok(Some(state))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(CrashResumeError::Io(e)),
        }
    }
}

/// Apply class-a fields from `restored` onto `target`.
///
/// Class-a per `docs/26` row 80: active scene + preset channel
/// values. Today this maps onto:
///
/// - `preset_store.active` — copied from `restored` to `target`.
/// - `control_store.values` — replaced wholesale on `target` with the
///   restored map. Control values ARE the preset channel values
///   under the slice-3a state model.
///
/// Does NOT touch class-b fields (Snap selection, mapping bindings,
/// playlist position, frame counter, snapshot metadata, or
/// `latest_audio_snapshot`). The orthogonality is asserted at T1.
pub fn restore_class_a(restored: &State, target: &mut State) {
    target.preset_store.active = restored.preset_store.active;
    target.control_store.values = restored.control_store.values.clone();
}

/// Apply class-b fields from `restored` onto `target`.
///
/// Class-b per `docs/26` row 80: Snap slot selection + scene-
/// specific preset history. Today this maps onto:
///
/// - `snap_store.active` + `snap_store.saved` — copied wholesale
///   from `restored` to `target`.
///
/// Scene-specific preset history is reserved as a future
/// canonical-state addition; the assignment widens here when the
/// sub-store lands (alongside the corpus and the
/// `state_replay` encoder extension).
///
/// Does NOT touch class-a fields. The orthogonality is asserted at
/// T1.
pub fn restore_class_b(restored: &State, target: &mut State) {
    target.snap_store.active = restored.snap_store.active;
    target.snap_store.saved = restored.snap_store.saved.clone();
}

/// Apply every class supported by the current adapter (class-a +
/// class-b in PR 1).
///
/// Subsequent PRs widen this convenience helper as class-c through
/// class-g land. Existing call sites pick up the broader
/// restoration without an API break.
pub fn restore_all_supported_classes(restored: &State, target: &mut State) {
    restore_class_a(restored, target);
    restore_class_b(restored, target);
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-CRASH-RESUME (T1) — Phase-4d Wave 4d.3 PR 1
    //! crash-resume class-a + class-b drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins on the
    //! adapter's save / recover round-trip, the per-class
    //! restoration orthogonality, and the
    //! [`CrashResumeError`] surface. The T2 corpus in
    //! `engine/runtime/tests/crash_resume_corpus/main.rs` exercises
    //! the simulated-crash harness across the canonical-binary
    //! envelope round-trip.
    //!
    //! Anchors:
    //! - `docs/26-evidence-artifact-registry.md` row 80 —
    //!   `EV-CRASH-RESUME`.
    //! - `docs/adr/ADR-004-state-model.md` — canonical state model.
    //! - `docs/adr/ADR-006-thread-and-process-model.md` — state
    //!   commit thread.
    //! - `docs/43-development-standards.md` §15.5 — T-tier definitions.

    use super::{
        CanonicalBinarySnapshotter, CrashResumeAdapter, CrashResumeError, SerializationKind,
        StateSnapshotter, restore_all_supported_classes, restore_class_a, restore_class_b,
    };
    use crate::command_bus::{
        Command, CommandBus, ControlId, ControlValue, InMemoryBus, MappingId, MappingSource,
        PresetId, SnapSlotId,
    };
    use crate::state::State;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Per-test unique save path under the OS temp dir. Avoids a
    /// `tempfile` dev-dep — the path is composed from process id,
    /// nanos since epoch, and a per-process counter so concurrent
    /// `cargo test` runs do not collide.
    fn unique_save_path(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let pid = std::process::id();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "chromaticity-crash-resume-{label}-{pid}-{nanos}-{n}.bin"
        ));
        path
    }

    /// Cleanup helper: remove the snapshot file if present. Errors
    /// are swallowed — the test outcome is what's load-bearing.
    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
    }

    /// Build a non-default state with class-a + class-b mutations
    /// committed through the canonical command-bus path. Keeps the
    /// drift pins exercising the same code-path the production
    /// commit thread drives.
    fn seeded_state() -> State {
        let bus = InMemoryBus::in_memory();
        let mut state = State::default();
        let snap = SnapSlotId::new(7).expect("valid snap slot");
        bus.dispatch(Command::ControlSet {
            id: ControlId(11),
            value: ControlValue(0.25),
        })
        .expect("control command must dispatch");
        bus.dispatch(Command::ControlSet {
            id: ControlId(13),
            value: ControlValue(0.875),
        })
        .expect("control command must dispatch");
        bus.dispatch(Command::PresetRecall { id: PresetId(5) })
            .expect("preset command must dispatch");
        bus.dispatch(Command::SnapSave { slot: snap })
            .expect("snap save command must dispatch");
        bus.dispatch(Command::SnapRecall { slot: snap })
            .expect("snap recall command must dispatch");
        bus.commit_all_into(&mut state);
        state
    }

    /// Build a non-default state with ONLY class-a mutations (no
    /// Snap activity). Used by the orthogonality drift pin.
    fn seeded_class_a_only() -> State {
        let bus = InMemoryBus::in_memory();
        let mut state = State::default();
        bus.dispatch(Command::ControlSet {
            id: ControlId(2),
            value: ControlValue(0.5),
        })
        .expect("control command must dispatch");
        bus.dispatch(Command::PresetRecall { id: PresetId(9) })
            .expect("preset command must dispatch");
        bus.commit_all_into(&mut state);
        state
    }

    /// Build a non-default state with ONLY class-b mutations (no
    /// preset / control activity). Used by the orthogonality drift
    /// pin.
    fn seeded_class_b_only() -> State {
        let bus = InMemoryBus::in_memory();
        let mut state = State::default();
        let snap = SnapSlotId::new(3).expect("valid snap slot");
        bus.dispatch(Command::SnapSave { slot: snap })
            .expect("snap save command must dispatch");
        bus.dispatch(Command::SnapRecall { slot: snap })
            .expect("snap recall command must dispatch");
        bus.commit_all_into(&mut state);
        state
    }

    /// Build a non-default state with mapping + playlist mutations
    /// (NOT in class-a or class-b). Used to assert restoration does
    /// not accidentally touch deferred classes.
    fn seeded_deferred_classes() -> State {
        let bus = InMemoryBus::in_memory();
        let mut state = State::default();
        bus.dispatch(Command::PlaylistAdvance { step: 7 })
            .expect("playlist command must dispatch");
        bus.dispatch(Command::MappingBind {
            id: MappingId(4),
            source: MappingSource::Osc,
        })
        .expect("mapping command must dispatch");
        bus.commit_all_into(&mut state);
        state
    }

    #[test]
    fn snapshotter_round_trip_class_a_state_preserves_active_scene_and_presets() {
        let original = seeded_class_a_only();
        let snapshotter = CanonicalBinarySnapshotter;
        let blob = snapshotter
            .snapshot(&original)
            .expect("class-a state must serialize");
        let restored = snapshotter
            .restore(&blob)
            .expect("class-a state must deserialize");
        assert_eq!(restored.active_preset(), original.active_preset());
        assert_eq!(
            restored.control_value(ControlId(2)),
            original.control_value(ControlId(2)),
        );
    }

    #[test]
    fn snapshotter_round_trip_class_b_state_preserves_snap_slot_and_preset_history() {
        let original = seeded_class_b_only();
        let snapshotter = CanonicalBinarySnapshotter;
        let blob = snapshotter
            .snapshot(&original)
            .expect("class-b state must serialize");
        let restored = snapshotter
            .restore(&blob)
            .expect("class-b state must deserialize");
        assert_eq!(restored.active_snap_slot(), original.active_snap_slot());
        let slot = SnapSlotId::new(3).expect("valid snap slot");
        assert_eq!(
            restored.snap_slot_saved(slot),
            original.snap_slot_saved(slot),
        );
    }

    #[test]
    fn adapter_save_then_recover_yields_byte_identical_state_for_classes_a_and_b() {
        let path = unique_save_path("save-then-recover");
        let original = seeded_state();
        let blob_baseline = CanonicalBinarySnapshotter
            .snapshot(&original)
            .expect("baseline snapshot must serialize");
        {
            let mut adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
            adapter.save(&original).expect("save must succeed");
        }
        let adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
        let recovered = adapter
            .try_recover()
            .expect("recover must succeed")
            .expect("recover must return Some(state)");
        let blob_recovered = CanonicalBinarySnapshotter
            .snapshot(&recovered)
            .expect("recovered state must reserialize");
        assert_eq!(
            blob_baseline.as_bytes(),
            blob_recovered.as_bytes(),
            "save -> recover -> reserialize must byte-equal the baseline serialization",
        );
        cleanup(&path);
    }

    #[test]
    fn adapter_recover_returns_none_when_save_path_missing() {
        let path = unique_save_path("recover-missing");
        // Belt-and-braces: ensure path does not exist.
        cleanup(&path);
        let adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
        let outcome = adapter
            .try_recover()
            .expect("missing-file recover must not error");
        assert!(
            outcome.is_none(),
            "missing save path must surface as Ok(None), got {outcome:?}",
        );
    }

    #[test]
    fn restore_class_a_does_not_touch_class_b_fields() {
        // Build a `target` carrying class-b values; restore class-a
        // from a `restored` source carrying DIFFERENT class-b
        // values; assert `target`'s class-b values are unchanged.
        let restored_source = seeded_state();
        let mut target = seeded_class_b_only();
        let target_snap_active_pre = target.active_snap_slot();
        let target_snap_saved_pre: Vec<u8> = {
            let mut v: Vec<u8> = target.snap_store.saved.iter().map(|s| s.get()).collect();
            v.sort_unstable();
            v
        };
        // Also pin deferred-class fields (playlist, mapping) to
        // assert restore_class_a leaves them untouched.
        target.playlist_store.position = 42;
        let target_playlist_pre = target.playlist_position();

        restore_class_a(&restored_source, &mut target);

        // Class-a has been applied:
        assert_eq!(target.active_preset(), restored_source.active_preset());
        // Class-b is untouched:
        assert_eq!(target.active_snap_slot(), target_snap_active_pre);
        let target_snap_saved_post: Vec<u8> = {
            let mut v: Vec<u8> = target.snap_store.saved.iter().map(|s| s.get()).collect();
            v.sort_unstable();
            v
        };
        assert_eq!(target_snap_saved_post, target_snap_saved_pre);
        // Deferred classes untouched:
        assert_eq!(target.playlist_position(), target_playlist_pre);
    }

    #[test]
    fn restore_class_b_does_not_touch_class_a_fields() {
        let restored_source = seeded_state();
        let mut target = seeded_class_a_only();
        let target_active_preset_pre = target.active_preset();
        let target_control_pre = target.control_value(ControlId(2));
        target.playlist_store.position = 99;
        let target_playlist_pre = target.playlist_position();

        restore_class_b(&restored_source, &mut target);

        // Class-b has been applied:
        assert_eq!(
            target.active_snap_slot(),
            restored_source.active_snap_slot(),
        );
        // Class-a untouched:
        assert_eq!(target.active_preset(), target_active_preset_pre);
        assert_eq!(target.control_value(ControlId(2)), target_control_pre);
        // Deferred classes untouched:
        assert_eq!(target.playlist_position(), target_playlist_pre);
    }

    #[test]
    fn restore_all_supported_classes_applies_both_class_a_and_class_b() {
        let restored_source = seeded_state();
        let mut target = State::default();
        // Seed deferred-class fields on the target so we can assert
        // they survive the restoration.
        target.playlist_store.position = 17;
        target
            .mapping_store
            .bindings
            .insert(MappingId(99), MappingSource::Midi);

        restore_all_supported_classes(&restored_source, &mut target);

        // Class-a applied:
        assert_eq!(target.active_preset(), restored_source.active_preset());
        assert_eq!(
            target.control_value(ControlId(11)),
            restored_source.control_value(ControlId(11)),
        );
        // Class-b applied:
        assert_eq!(
            target.active_snap_slot(),
            restored_source.active_snap_slot(),
        );
        // Deferred classes (c-g) preserved on target:
        assert_eq!(target.playlist_position(), 17);
        assert_eq!(
            target.mapping_source(MappingId(99)),
            Some(MappingSource::Midi),
        );
    }

    #[test]
    fn restore_helpers_ignore_deferred_class_state_on_source() {
        // Even when the `restored` source carries deferred-class
        // mutations (playlist position, mapping bindings), the
        // class-a + class-b helpers MUST NOT project them onto
        // `target`. Pins the partial-advance scope: class-c..g land
        // by widening the helpers, not by widening what already-
        // shipped helpers touch.
        let restored_source = seeded_deferred_classes();
        let mut target = State::default();
        restore_all_supported_classes(&restored_source, &mut target);
        assert_eq!(
            target.playlist_position(),
            0,
            "playlist (class-c) must not be touched by class-a/b restoration",
        );
        assert_eq!(
            target.mapping_source(MappingId(4)),
            None,
            "mapping (class-d) must not be touched by class-a/b restoration",
        );
    }

    #[test]
    fn crash_resume_error_serialization_kind_documented() {
        // Sanity: the `#[non_exhaustive]` enum must be matchable on
        // every shipped variant without panicking. A future variant
        // would force the wildcard arm and surface this pin as the
        // load-bearing reminder to widen the corpus.
        let err = CrashResumeError::SerializationError(SerializationKind::CanonicalBinary(
            crate::state_migration::DecodeError::Truncated,
        ));
        let msg = format!("{err}");
        assert!(msg.contains("serialization"), "display surface: {msg}");
        let err_io = CrashResumeError::Io(std::io::Error::other("synthetic"));
        let msg_io = format!("{err_io}");
        assert!(msg_io.contains("io"), "io display surface: {msg_io}");
        let err_ver = CrashResumeError::VersionMismatch;
        let msg_ver = format!("{err_ver}");
        assert!(
            msg_ver.contains("version") || msg_ver.contains("schema"),
            "version display surface: {msg_ver}",
        );
    }

    #[test]
    fn snapshot_blob_borrow_is_stable_across_clones() {
        // Pins the opaque-byte-container contract: `as_bytes` is the
        // canonical read path.
        let original = seeded_state();
        let blob = CanonicalBinarySnapshotter
            .snapshot(&original)
            .expect("snapshot must serialize");
        let cloned = blob.clone();
        assert_eq!(blob.as_bytes(), cloned.as_bytes());
        assert!(
            !blob.as_bytes().is_empty(),
            "non-default state blob is non-empty"
        );
    }
}
