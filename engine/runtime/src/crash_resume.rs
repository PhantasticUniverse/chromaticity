// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4d Wave 4d.3 — `EV-CRASH-RESUME` partial-advance
//! state-restoration adapter (row 80).
//!
//! Composes the Phase-3 EV-STATE-MIGRATION versioned snapshot
//! envelope (sibling [`crate::state_migration`]) with a disk-backed
//! save / recover cycle plus per-class restoration helpers. The
//! partial-advance shape mirrors the Phase-3 narrowing precedent
//! (rows 33-36): subsequent PRs in this wave cluster widen the
//! restoration surface across the row-80 seven-class taxonomy.
//!
//! # Slice progression
//!
//! - **PR 1**: class-a (active scene + preset channel values) +
//!   class-b (Snap slot selection + scene-specific preset history)
//!   per their current canonical-state mappings.
//! - **PR 2**: class-c (playlist position + iteration
//!   mode) per the current canonical playlist-store mapping. Class-d
//!   (OSC/MIDI session bindings) is DEFERRED: the canonical [`State`]
//!   does not yet carry session / device-id state — the
//!   [`crate::state::MappingStore`] surface today is a per-mapping
//!   [`crate::command_bus::MappingSource`] enum (`Osc` / `Midi`),
//!   not a session-binding map keyed by device id. Adding class-d
//!   restoration would require a State-shape extension (new
//!   sub-store + commit-bus event family + canonical-binary encoder
//!   widening), which carries broader review implications and is
//!   sequenced behind that work.
//! - **PR 3**: class-e (audio input device + `syn_Threshold`)
//!   helper [`restore_class_e`] lands as a documented-deferred no-op.
//!   The canonical [`State`] does not carry an audio-input-device
//!   field nor a `syn_Threshold` control as first-class state — the
//!   transient
//!   [`crate::state::State::latest_audio_snapshot`] is the only
//!   audio-touching field today and is intentionally NOT serialized
//!   (the scheduler-tick repopulates it on the next pull, matching
//!   the row-80 "audio trailing reactivity state resets on resume by
//!   design" note). The audio-input-device selection lives at the
//!   producer side ([`chromaticity_audio::runtime::AudioRuntime`])
//!   rather than the canonical state core, and `syn_Threshold` is a
//!   future audio-uniform control reserved by [`chromaticity_audio`]
//!   per the `engine/audio/src/uniforms.rs` module-level prose. Class-
//!   e restoration therefore ships as the partial-advance no-op
//!   PR 1 / PR 2 deferral pattern established for class-d: the helper
//!   is callable, [`restore_all_supported_classes`] does NOT yet
//!   invoke it, and the widening trigger is the State-shape extension
//!   that promotes the audio-input-device + `syn_Threshold` selections
//!   into canonical sub-stores.
//! - **PR 4** (this PR): class-f (operator meta-controls) helper
//!   [`restore_class_f`] AND class-g (render resolution + FPS target)
//!   helper [`restore_class_g`] both land as documented-deferred
//!   no-ops mirroring the PR 3 class-e pattern. The canonical
//!   [`State`] does not carry operator meta-control fields (no
//!   `meta_controls` / `global_controls` sub-store; the existing
//!   [`crate::state::ControlStore`] holds per-channel preset values
//!   covered by class-a, NOT operator-scope global parameters) nor
//!   render-config fields (no `render_resolution` / `fps_target`
//!   sub-store; the scheduler-tick frame-rate is configured at
//!   [`crate::scheduler::Scheduler`] construction time and the render
//!   surface dimensions live at the host / Tauri shell side rather
//!   than the canonical state core). Both helpers ship as no-op-shaped
//!   functions: callable, accepting both arguments, reading neither,
//!   mutating nothing. The widening trigger for each is the State-
//!   shape extension that promotes the corresponding selections into
//!   canonical sub-stores, at which point each helper gains the
//!   corresponding sub-store assignments and joins
//!   [`restore_all_supported_classes`]. As of the current launch
//!   wiring, [`restore_all_supported_classes`] covers class-a +
//!   class-b + class-c + class-d; class-e / -f / -g remain
//!   documented-deferred no-ops because their canonical State
//!   sub-stores do not exist yet.
//! - **PR 5** (this PR): class-d (OSC/MIDI session bindings) ships
//!   as a real sub-store assignment via the additive [`State`]
//!   extension introduced in
//!   [`crate::state::SessionBindings`]. The V2 snapshot envelope
//!   ([`crate::state_migration::encode_snapshot`]) gains a tagged
//!   `EXT_ID_V2_SESSION_BINDINGS` extension carrying the device-id
//!   list per OSC and MIDI; the canonical-binary segment
//!   ([`crate::state_replay::encode_state_canonical`]) does NOT
//!   observe `session_bindings`, so EV-STATE-REPLAY (row 32) /
//!   EV-PRESET-PLAYLIST (row 34) / EV-STATE-MIGRATION (row 33)
//!   canonical hashes are preserved. [`restore_class_d`] joins
//!   [`restore_all_supported_classes`]. Adapter-side rebinding
//!   (re-enumerating OSC / MIDI ports against the persisted device
//!   IDs per MIDI-CONF-09) is the responsibility of the integration
//!   crates and runs at adapter startup-time, not this restore step.
//! - **PR 6+**: class-e once the audio-input-device + `syn_Threshold`
//!   sub-stores land, class-f once the operator meta-controls sub-store
//!   lands, class-g once the render-config sub-store lands (at which
//!   point each respective helper widens from a no-op to a sub-store
//!   assignment and gets added to [`restore_all_supported_classes`]).
//!
//! # Class-a + class-b + class-c mapping onto current canonical state
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
//! - **Class-c — playlist position + iteration mode.** Today the
//!   canonical [`State`] carries the playlist cursor position
//!   ([`State::playlist_position`]) via the `playlist_store.position`
//!   `i64`. The "iteration mode" concept (forward / reverse / loop /
//!   shuffle) is reserved as a future addition — when it lands,
//!   [`restore_class_c`] gains the corresponding sub-store assignment
//!   and the corpus widens. The current canonical-binary encoder
//!   ([`crate::state_replay::encode_state_canonical`]) round-trips
//!   the position field losslessly, so PR 2 ships the position-only
//!   restoration surface.
//!
//! Class-by-class restoration is the partial-advance signal the wave
//! cluster relies on: class-c through class-g land in subsequent PRs
//! by widening the per-class helpers without changing the on-disk
//! envelope shape.
//!
//! # Crash-injection harness scope (PR 1)
//!
//! The on-disk path writes through a same-dir temporary file,
//! `flush` / `sync_all`, then atomic rename, and reads through the
//! configured save path. The
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
//!    [`restore_class_a`] / [`restore_class_b`] / [`restore_class_c`]
//!    selectively) to project the restored values onto the fresh
//!    state, and
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
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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

    /// Serialize `state` into an opaque snapshot blob without
    /// touching the filesystem.
    ///
    /// Host integrations use this to capture the state while holding
    /// their canonical-state lock, then drop the lock before calling
    /// [`Self::save_blob`] for the slower disk write.
    pub fn snapshot(&self, state: &State) -> Result<StateSnapshotBlob, CrashResumeError> {
        self.snapshotter.snapshot(state)
    }

    /// Persist an already-serialized snapshot blob to the configured
    /// save path via same-dir temporary file, `flush` / `sync_all`,
    /// then atomic rename. Overwrites any existing snapshot.
    pub fn save_blob(&mut self, blob: &StateSnapshotBlob) -> Result<(), CrashResumeError> {
        atomic_write(&self.save_path, blob.as_bytes())?;
        Ok(())
    }

    /// Serialize `state` and persist the blob to the configured save
    /// path. Overwrites any existing snapshot.
    pub fn save(&mut self, state: &State) -> Result<(), CrashResumeError> {
        let blob = self.snapshot(state)?;
        self.save_blob(&blob)
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

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), io::Error> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let tmp_path = temporary_sibling_path(path);
    let mut tmp_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)?;
    let write_result = (|| -> Result<(), io::Error> {
        tmp_file.write_all(bytes)?;
        tmp_file.flush()?;
        tmp_file.sync_all()?;
        Ok(())
    })();
    if let Err(err) = write_result {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(err);
    }
    drop(tmp_file);
    if let Err(err) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(err);
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        let _ = std::fs::File::open(parent).and_then(|dir| dir.sync_all());
    }
    Ok(())
}

fn temporary_sibling_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "crash-resume.bin".to_string());
    let pid = std::process::id();
    let timestamp = timestamp_nanos();
    path.with_file_name(format!("{file_name}.tmp-{pid}-{timestamp}"))
}

fn timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
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

/// Apply class-c fields from `restored` onto `target`.
///
/// Class-c per `docs/26` row 80: playlist position + iteration mode.
/// Today this maps onto:
///
/// - `playlist_store.position` — copied wholesale from `restored` to
///   `target`.
///
/// Iteration mode (forward / reverse / loop / shuffle) is reserved
/// as a future canonical-state addition; the assignment widens here
/// when the sub-store field lands (alongside the corpus and the
/// `state_replay` encoder extension).
///
/// Does NOT touch class-a or class-b fields. The orthogonality is
/// asserted at T1.
pub fn restore_class_c(restored: &State, target: &mut State) {
    target.playlist_store.position = restored.playlist_store.position;
}

/// Apply class-d fields from `restored` onto `target`.
///
/// Class-d per `docs/26-evidence-artifact-registry.md` row 80
/// description column: "OSC/MIDI session bindings (device IDs
/// persist; bindings rebind on re-enumeration per MIDI-CONF-09)".
///
/// Wave 4d.3 PR 5 ships the additive
/// [`crate::state::SessionBindings`] sub-store on [`State`]
/// recording the OSC + MIDI device IDs that were bound at session
/// start. This helper overwrites `target.session_bindings` with
/// `restored.session_bindings` verbatim. Adapter-side rebinding
/// (re-enumerating the OSC / MIDI ports against the persisted
/// device IDs per MIDI-CONF-09) is the responsibility of the
/// integration crates and runs at adapter startup-time, not this
/// restore step — class-d's contribution is the device-id
/// PERSISTENCE shape, not the rebind protocol.
///
/// Does NOT touch class-a, class-b, class-c fields, the
/// per-mapping [`crate::state::MappingStore`] (which carries the
/// OSC / MIDI tag for each `MappingId` and is class-d-adjacent but
/// distinct), the deferred class-e / class-f / class-g surfaces, or
/// the snapshot envelope metadata. The orthogonality is asserted at
/// T1.
pub fn restore_class_d(target: &mut State, recovered: &State) {
    let recovered_bindings = recovered.session_bindings().clone();
    *target.session_bindings_mut() = recovered_bindings;
}

/// Apply class-e fields from `restored` onto `target`.
///
/// Class-e per `docs/26` row 80: audio input device + `syn_Threshold`.
///
/// **PR 3 status: documented-deferred no-op.** The canonical [`State`]
/// does not carry an audio-input-device field nor a `syn_Threshold`
/// control as first-class sub-store state. The
/// transient [`State::latest_audio_snapshot`] is the only audio-
/// touching field today and is intentionally NOT serialized (the
/// scheduler-tick repopulates it on the next pull, matching the row-80
/// "audio trailing reactivity state resets on resume by design" note).
/// The audio-input-device selection lives at the producer side
/// ([`chromaticity_audio::runtime::AudioRuntime`]) rather than the
/// canonical state core, and `syn_Threshold` is a future audio-uniform
/// control reserved by [`chromaticity_audio`] per the
/// `engine/audio/src/uniforms.rs` module-level prose.
///
/// This helper is shipped as a no-op (it accepts both arguments, reads
/// neither, mutates nothing) so call sites can be wired ahead of the
/// widening trigger. The widening trigger is the State-shape extension
/// that promotes the audio-input-device + `syn_Threshold` selections
/// into canonical sub-stores, at which point this helper gains the
/// corresponding sub-store assignments and joins
/// [`restore_all_supported_classes`].
///
/// Does NOT touch class-a, class-b, or class-c fields — and does not
/// touch ANY field, since the helper is a no-op until class-e State
/// lands. The orthogonality is asserted at T1.
pub fn restore_class_e(_restored: &State, _target: &mut State) {
    // Documented-deferred no-op: see the doc comment above for the
    // class-e State-shape extension trigger.
}

/// Apply class-f fields from `restored` onto `target`.
///
/// Class-f per `docs/26` row 80: operator meta-controls.
///
/// **PR 4 status: documented-deferred no-op.** The canonical [`State`]
/// does not carry operator meta-control fields as first-class sub-store
/// state. The existing [`crate::state::ControlStore`] holds per-channel
/// preset values that are covered by class-a (active scene + preset
/// channel values), NOT operator-scope global / meta parameters that
/// persist across scene changes. There is no `meta_controls` or
/// `global_controls` sub-store on [`State`] today.
///
/// This helper is shipped as a no-op (it accepts both arguments, reads
/// neither, mutates nothing) so call sites can be wired ahead of the
/// widening trigger. The widening trigger is the State-shape extension
/// that promotes operator meta-controls into a canonical sub-store, at
/// which point this helper gains the corresponding sub-store assignment
/// and joins [`restore_all_supported_classes`].
///
/// Does NOT touch class-a, class-b, class-c, or class-e fields — and
/// does not touch ANY field, since the helper is a no-op until class-f
/// State lands. The orthogonality is asserted at T1.
pub fn restore_class_f(_restored: &State, _target: &mut State) {
    // Documented-deferred no-op: see the doc comment above for the
    // class-f State-shape extension trigger.
}

/// Apply class-g fields from `restored` onto `target`.
///
/// Class-g per `docs/26` row 80: render resolution + FPS target.
///
/// **PR 4 status: documented-deferred no-op.** The canonical [`State`]
/// does not carry render-config fields as first-class sub-store state.
/// There is no `render_resolution` or `fps_target` sub-store on
/// [`State`] today — the scheduler-tick frame-rate is configured at
/// [`crate::scheduler::Scheduler`] construction time (see the
/// drift-corrected 60 Hz scheduler-tick thread spawned in
/// `app/shell/src/lib.rs::run`) and the render surface dimensions live
/// at the host / Tauri shell side rather than the canonical state
/// core.
///
/// This helper is shipped as a no-op (it accepts both arguments, reads
/// neither, mutates nothing) so call sites can be wired ahead of the
/// widening trigger. The widening trigger is the State-shape extension
/// that promotes the render-resolution + FPS-target selections into a
/// canonical sub-store, at which point this helper gains the
/// corresponding sub-store assignment and joins
/// [`restore_all_supported_classes`].
///
/// Does NOT touch class-a, class-b, class-c, class-e, or class-f
/// fields — and does not touch ANY field, since the helper is a no-op
/// until class-g State lands. The orthogonality is asserted at T1.
pub fn restore_class_g(_restored: &State, _target: &mut State) {
    // Documented-deferred no-op: see the doc comment above for the
    // class-g State-shape extension trigger.
}

/// Apply every class supported by the current adapter (class-a +
/// class-b + class-c + class-d).
///
/// Class-e, class-f, and class-g are NOT invoked here as of PR 5 —
/// [`restore_class_e`], [`restore_class_f`], and [`restore_class_g`]
/// remain documented-deferred no-ops preserved for PR 6+ widening
/// once the canonical [`State`] grows the corresponding sub-stores.
/// Class-d (OSC/MIDI session bindings) joined this helper in Wave
/// 4d.3 PR 5 alongside the additive
/// [`crate::state::SessionBindings`] sub-store extension on
/// [`State`]. Subsequent PRs widen this convenience helper as
/// class-e through class-g land. Existing call sites pick up the
/// broader restoration without an API break.
pub fn restore_all_supported_classes(restored: &State, target: &mut State) {
    restore_class_a(restored, target);
    restore_class_b(restored, target);
    restore_class_c(restored, target);
    restore_class_d(target, restored);
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
        restore_class_c, restore_class_d, restore_class_e, restore_class_f, restore_class_g,
    };
    use crate::command_bus::{
        Command, CommandBus, ControlId, ControlValue, InMemoryBus, MappingId, MappingSource,
        PresetId, SnapSlotId,
    };
    use crate::state::{MidiDeviceBinding, OscDeviceBinding, State};
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

    /// Build a non-default state with ONLY class-c mutations
    /// (playlist position only). Used by the class-c orthogonality
    /// drift pin and the round-trip pin.
    fn seeded_class_c_only() -> State {
        let bus = InMemoryBus::in_memory();
        let mut state = State::default();
        bus.dispatch(Command::PlaylistAdvance { step: 11 })
            .expect("playlist command must dispatch");
        bus.dispatch(Command::PlaylistAdvance { step: 4 })
            .expect("playlist command must dispatch");
        bus.commit_all_into(&mut state);
        state
    }

    /// Build a non-default state with mapping mutations only (NOT in
    /// class-a / -b / -c / -d). Used to assert restoration does not
    /// accidentally touch the per-mapping store while class-d restores
    /// only the session-bindings sub-store.
    fn seeded_deferred_classes() -> State {
        let bus = InMemoryBus::in_memory();
        let mut state = State::default();
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
    fn restore_class_a_does_not_touch_class_b_or_class_c_fields() {
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
        // Also pin class-c (playlist) + still-deferred-class
        // (mapping) fields to assert restore_class_a leaves them
        // untouched.
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
        // Class-c untouched (orthogonality with PR 2's new helper):
        assert_eq!(target.playlist_position(), target_playlist_pre);
    }

    #[test]
    fn restore_class_b_does_not_touch_class_a_or_class_c_fields() {
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
        // Class-c untouched (orthogonality with PR 2's new helper):
        assert_eq!(target.playlist_position(), target_playlist_pre);
    }

    #[test]
    fn restore_class_c_state_preserves_playlist_position_and_iteration_mode() {
        // Pin: class-c restoration projects the canonical playlist
        // position from `restored` onto `target`. Iteration mode is
        // not yet a State field; this pin stays load-bearing for the
        // position-only mapping today and widens to assert the
        // iteration-mode field when it lands.
        let restored_source = seeded_class_c_only();
        let mut target = State::default();
        target.playlist_store.position = -7;

        restore_class_c(&restored_source, &mut target);

        assert_eq!(
            target.playlist_position(),
            restored_source.playlist_position(),
            "class-c restoration must project playlist position",
        );
        // Iteration mode: deferred sub-store; documented in
        // module-level prose. When the field lands, the assertion
        // widens here.
    }

    #[test]
    fn restore_class_c_does_not_touch_class_a_or_class_b_fields() {
        // Build a `target` carrying class-a + class-b values;
        // restore class-c from a `restored` source carrying
        // DIFFERENT class-a + class-b values; assert `target`'s
        // class-a + class-b values are unchanged.
        let restored_source = seeded_state();
        let mut target = seeded_class_a_only();
        let target_active_preset_pre = target.active_preset();
        let target_control_pre = target.control_value(ControlId(2));
        // Also seed class-b on `target` to pin its untouched-ness.
        let snap = SnapSlotId::new(15).expect("valid snap slot");
        target.snap_store.saved.insert(snap);
        target.snap_store.active = Some(snap);
        let target_snap_active_pre = target.active_snap_slot();

        restore_class_c(&restored_source, &mut target);

        // Class-c applied:
        assert_eq!(
            target.playlist_position(),
            restored_source.playlist_position(),
        );
        // Class-a untouched:
        assert_eq!(target.active_preset(), target_active_preset_pre);
        assert_eq!(target.control_value(ControlId(2)), target_control_pre);
        // Class-b untouched:
        assert_eq!(target.active_snap_slot(), target_snap_active_pre);
        assert!(target.snap_slot_saved(snap));
    }

    /// Build a non-default state with class-d session bindings (one
    /// OSC device + one MIDI device, each with one mapping). Used by
    /// the class-d round-trip + orthogonality drift pins.
    fn seeded_class_d_only() -> State {
        let mut state = State::default();
        let bindings = state.session_bindings_mut();
        bindings.osc_devices.push(OscDeviceBinding {
            device_id: "osc.example.local:9000".to_owned(),
            mappings: vec![MappingId(11), MappingId(12)],
        });
        bindings.midi_devices.push(MidiDeviceBinding {
            device_id: "iac-bus-0".to_owned(),
            mappings: vec![MappingId(21)],
        });
        state
    }

    #[test]
    fn class_d_status_no_longer_deferred_pending_state_shape() {
        // Documentation pin (Wave 4d.3 PR 5): class-d (OSC/MIDI
        // session bindings) ships in this PR via the additive
        // `crate::state::SessionBindings` sub-store extension on
        // `State`. The widening trigger documented by PR 1-4 has
        // landed: this pin replaces the previous `class_d_deferred_*`
        // pin and asserts the helper is wired into
        // `restore_all_supported_classes` for class-d round-trip.
        //
        // Class-e / class-f / class-g remain documented-deferred no-
        // ops pending their respective State-shape extensions.
        let restored_source = seeded_class_d_only();
        let mut target = State::default();
        // Pre-condition: target has empty session_bindings.
        assert!(target.session_bindings().osc_devices.is_empty());
        assert!(target.session_bindings().midi_devices.is_empty());

        restore_all_supported_classes(&restored_source, &mut target);

        // Post-condition: class-d state has been projected.
        assert_eq!(
            target.session_bindings().osc_devices.len(),
            1,
            "class-d OSC device must be projected",
        );
        assert_eq!(
            target.session_bindings().midi_devices.len(),
            1,
            "class-d MIDI device must be projected",
        );
        assert_eq!(
            target.session_bindings().osc_devices[0].device_id,
            "osc.example.local:9000",
        );
    }

    #[test]
    fn restore_class_d_overwrites_target_session_bindings_with_recovered() {
        // Pin: class-d restoration projects the canonical
        // `SessionBindings` sub-store from `recovered` onto `target`,
        // overwriting any prior bindings on `target`.
        let recovered = seeded_class_d_only();
        let mut target = State::default();
        // Seed `target` with a different binding shape that must be
        // overwritten.
        target
            .session_bindings_mut()
            .osc_devices
            .push(OscDeviceBinding {
                device_id: "stale-osc-device".to_owned(),
                mappings: vec![MappingId(99)],
            });

        restore_class_d(&mut target, &recovered);

        assert_eq!(target.session_bindings(), recovered.session_bindings());
    }

    #[test]
    fn restore_class_d_handles_empty_recovered_session_bindings() {
        // Pin: class-d restoration with empty `recovered.session_bindings`
        // clears any state the target was carrying. The helper is a
        // wholesale replace, not a merge.
        let recovered = State::default();
        let mut target = seeded_class_d_only();

        restore_class_d(&mut target, &recovered);

        assert!(target.session_bindings().osc_devices.is_empty());
        assert!(target.session_bindings().midi_devices.is_empty());
    }

    #[test]
    fn restore_class_d_does_not_touch_other_state_fields() {
        // Pin: class-d restoration only touches `session_bindings`.
        // Class-a (preset / control), class-b (Snap), class-c
        // (playlist), class-d-adjacent `mapping_store`, and the
        // snapshot envelope metadata are all preserved.
        let recovered = seeded_class_d_only();
        let mut target = seeded_class_a_only();
        let snap = SnapSlotId::new(15).expect("valid snap slot");
        target.snap_store.saved.insert(snap);
        target.snap_store.active = Some(snap);
        target.playlist_store.position = -42;
        target
            .mapping_store
            .bindings
            .insert(MappingId(99), MappingSource::Osc);

        let target_active_preset_pre = target.active_preset();
        let target_control_pre = target.control_value(ControlId(2));
        let target_snap_active_pre = target.active_snap_slot();
        let target_playlist_pre = target.playlist_position();
        let target_mapping_pre = target.mapping_source(MappingId(99));

        restore_class_d(&mut target, &recovered);

        // Class-a / b / c / mapping store untouched:
        assert_eq!(target.active_preset(), target_active_preset_pre);
        assert_eq!(target.control_value(ControlId(2)), target_control_pre);
        assert_eq!(target.active_snap_slot(), target_snap_active_pre);
        assert!(target.snap_slot_saved(snap));
        assert_eq!(target.playlist_position(), target_playlist_pre);
        assert_eq!(target.mapping_source(MappingId(99)), target_mapping_pre);
        // Class-d applied:
        assert_eq!(target.session_bindings(), recovered.session_bindings());
    }

    #[test]
    fn restore_all_supported_classes_invokes_class_d() {
        // Pin: PR 5 widens the convenience helper to invoke
        // `restore_class_d`. A target lacking session bindings whose
        // restored source carries them must end up with the bindings
        // applied alongside the class-a / class-b / class-c values.
        let restored_source = seeded_class_d_only();
        let mut target = State::default();

        restore_all_supported_classes(&restored_source, &mut target);

        assert_eq!(
            target.session_bindings(),
            restored_source.session_bindings()
        );
    }

    #[test]
    fn supported_classes_restoration_does_not_touch_per_mapping_store() {
        // Pin: although class-d is now shipped, the per-mapping
        // `mapping_store` (which carries the OSC / MIDI tag for each
        // `MappingId` and is class-d-adjacent but distinct) is
        // intentionally NOT touched by `restore_all_supported_classes`
        // — its restoration would belong to a future class-d-extension
        // PR or a follow-on class assignment.
        let restored_source = seeded_deferred_classes();
        let mut target = State::default();
        restore_all_supported_classes(&restored_source, &mut target);
        assert_eq!(
            target.mapping_source(MappingId(4)),
            None,
            "per-mapping mapping_store must not be touched by class-a/b/c/d restoration",
        );
    }

    #[test]
    fn class_e_deferred_pending_state_shape_extension() {
        // Documentation pin (PR 3): class-e (audio input device +
        // `syn_Threshold`) ships as a documented-deferred no-op
        // helper. The canonical [`State`] does not carry an audio-
        // input-device field nor a `syn_Threshold` control as first-
        // class sub-store state today. The transient
        // `latest_audio_snapshot` field is intentionally NOT
        // serialized (audio trailing reactivity state resets on
        // resume by design per `docs/26` row 80).
        //
        // The helper is no-op-shaped: it accepts both arguments,
        // reads neither, mutates nothing. The widening trigger is
        // the State-shape extension that promotes the audio-input-
        // device + `syn_Threshold` selections into canonical sub-
        // stores — at which point the helper gains the corresponding
        // sub-store assignments and joins
        // `restore_all_supported_classes`.
        //
        // Pin shape: build a `target` with class-a + class-b +
        // class-c values; invoke `restore_class_e` from a `restored`
        // source that differs across every class; assert NOTHING on
        // `target` changed. This is the orthogonality + no-op
        // contract surfaced together: the helper does not touch
        // class-a, class-b, class-c, the deferred-class-d mapping
        // store, OR any future class-e State (because there is none
        // to touch).
        let restored_source = seeded_state();
        let mut target = seeded_class_a_only();
        // Seed `target` with class-b, class-c, class-d-adjacent
        // values that would surface as drift if `restore_class_e`
        // accidentally became non-no-op.
        let snap = SnapSlotId::new(15).expect("valid snap slot");
        target.snap_store.saved.insert(snap);
        target.snap_store.active = Some(snap);
        target.playlist_store.position = -42;
        target
            .mapping_store
            .bindings
            .insert(MappingId(99), MappingSource::Osc);

        let target_active_preset_pre = target.active_preset();
        let target_control_pre = target.control_value(ControlId(2));
        let target_snap_active_pre = target.active_snap_slot();
        let target_playlist_pre = target.playlist_position();
        let target_mapping_pre = target.mapping_source(MappingId(99));

        restore_class_e(&restored_source, &mut target);

        // No-op contract: every observable on `target` is unchanged.
        assert_eq!(target.active_preset(), target_active_preset_pre);
        assert_eq!(target.control_value(ControlId(2)), target_control_pre);
        assert_eq!(target.active_snap_slot(), target_snap_active_pre);
        assert!(target.snap_slot_saved(snap));
        assert_eq!(target.playlist_position(), target_playlist_pre);
        assert_eq!(target.mapping_source(MappingId(99)), target_mapping_pre);
    }

    #[test]
    fn restore_class_e_idempotent_on_equal_target() {
        // Repeated invocation of the no-op helper must remain a no-
        // op (no panic, no flap). When the helper widens into a
        // sub-store assignment, this pin ensures idempotence on a
        // converged target — matching the class-c idempotence pin
        // shape from PR 2.
        let restored_source = seeded_state();
        let mut target = State::default();
        restore_class_e(&restored_source, &mut target);
        let snapshot_after_first = (
            target.active_preset(),
            target.active_snap_slot(),
            target.playlist_position(),
            target.frame(),
        );
        restore_class_e(&restored_source, &mut target);
        let snapshot_after_second = (
            target.active_preset(),
            target.active_snap_slot(),
            target.playlist_position(),
            target.frame(),
        );
        assert_eq!(snapshot_after_first, snapshot_after_second);
    }

    #[test]
    fn class_f_deferred_pending_state_shape_extension() {
        // Documentation pin (PR 4): class-f (operator meta-controls)
        // ships as a documented-deferred no-op helper. The canonical
        // [`State`] does not carry operator meta-control fields as
        // first-class sub-store state today — the existing
        // [`crate::state::ControlStore`] holds per-channel preset
        // values that are covered by class-a, NOT operator-scope
        // global / meta parameters that persist across scene changes.
        //
        // The helper is no-op-shaped: it accepts both arguments,
        // reads neither, mutates nothing. The widening trigger is
        // the State-shape extension that promotes operator meta-
        // controls into a canonical sub-store — at which point the
        // helper gains the corresponding sub-store assignment and
        // joins `restore_all_supported_classes`.
        //
        // Pin shape: build a `target` with class-a + class-b +
        // class-c values plus a class-d-adjacent mapping value;
        // invoke `restore_class_f` from a `restored` source that
        // differs across every class; assert NOTHING on `target`
        // changed. This is the orthogonality + no-op contract
        // surfaced together: the helper does not touch class-a,
        // class-b, class-c, the deferred-class-d mapping store,
        // class-e (which is itself a no-op), OR any future class-f
        // State (because there is none to touch).
        let restored_source = seeded_state();
        let mut target = seeded_class_a_only();
        let snap = SnapSlotId::new(15).expect("valid snap slot");
        target.snap_store.saved.insert(snap);
        target.snap_store.active = Some(snap);
        target.playlist_store.position = -42;
        target
            .mapping_store
            .bindings
            .insert(MappingId(99), MappingSource::Osc);

        let target_active_preset_pre = target.active_preset();
        let target_control_pre = target.control_value(ControlId(2));
        let target_snap_active_pre = target.active_snap_slot();
        let target_playlist_pre = target.playlist_position();
        let target_mapping_pre = target.mapping_source(MappingId(99));

        restore_class_f(&restored_source, &mut target);

        // No-op contract: every observable on `target` is unchanged.
        assert_eq!(target.active_preset(), target_active_preset_pre);
        assert_eq!(target.control_value(ControlId(2)), target_control_pre);
        assert_eq!(target.active_snap_slot(), target_snap_active_pre);
        assert!(target.snap_slot_saved(snap));
        assert_eq!(target.playlist_position(), target_playlist_pre);
        assert_eq!(target.mapping_source(MappingId(99)), target_mapping_pre);
    }

    #[test]
    fn restore_class_f_idempotent_on_equal_target() {
        // Repeated invocation of the no-op helper must remain a no-
        // op (no panic, no flap). When the helper widens into a
        // sub-store assignment, this pin ensures idempotence on a
        // converged target — matching the class-c / class-e
        // idempotence pin shape from PR 2 / PR 3.
        let restored_source = seeded_state();
        let mut target = State::default();
        restore_class_f(&restored_source, &mut target);
        let snapshot_after_first = (
            target.active_preset(),
            target.active_snap_slot(),
            target.playlist_position(),
            target.frame(),
        );
        restore_class_f(&restored_source, &mut target);
        let snapshot_after_second = (
            target.active_preset(),
            target.active_snap_slot(),
            target.playlist_position(),
            target.frame(),
        );
        assert_eq!(snapshot_after_first, snapshot_after_second);
    }

    #[test]
    fn class_g_deferred_pending_state_shape_extension() {
        // Documentation pin (PR 4): class-g (render resolution + FPS
        // target) ships as a documented-deferred no-op helper. The
        // canonical [`State`] does not carry render-config fields as
        // first-class sub-store state today — the scheduler-tick
        // frame-rate is configured at
        // `crate::scheduler::Scheduler` construction time and the
        // render surface dimensions live at the host / Tauri shell
        // side rather than the canonical state core.
        //
        // The helper is no-op-shaped: it accepts both arguments,
        // reads neither, mutates nothing. The widening trigger is
        // the State-shape extension that promotes the render-
        // resolution + FPS-target selections into a canonical sub-
        // store — at which point the helper gains the corresponding
        // sub-store assignment and joins
        // `restore_all_supported_classes`.
        //
        // Pin shape: same orthogonality + no-op contract as the
        // class-e / class-f pins.
        let restored_source = seeded_state();
        let mut target = seeded_class_a_only();
        let snap = SnapSlotId::new(15).expect("valid snap slot");
        target.snap_store.saved.insert(snap);
        target.snap_store.active = Some(snap);
        target.playlist_store.position = -42;
        target
            .mapping_store
            .bindings
            .insert(MappingId(99), MappingSource::Midi);

        let target_active_preset_pre = target.active_preset();
        let target_control_pre = target.control_value(ControlId(2));
        let target_snap_active_pre = target.active_snap_slot();
        let target_playlist_pre = target.playlist_position();
        let target_mapping_pre = target.mapping_source(MappingId(99));

        restore_class_g(&restored_source, &mut target);

        // No-op contract: every observable on `target` is unchanged.
        assert_eq!(target.active_preset(), target_active_preset_pre);
        assert_eq!(target.control_value(ControlId(2)), target_control_pre);
        assert_eq!(target.active_snap_slot(), target_snap_active_pre);
        assert!(target.snap_slot_saved(snap));
        assert_eq!(target.playlist_position(), target_playlist_pre);
        assert_eq!(target.mapping_source(MappingId(99)), target_mapping_pre);
    }

    #[test]
    fn restore_class_g_idempotent_on_equal_target() {
        // Repeated invocation of the no-op helper must remain a no-
        // op (no panic, no flap). Matches the class-c / class-e /
        // class-f idempotence pin shape.
        let restored_source = seeded_state();
        let mut target = State::default();
        restore_class_g(&restored_source, &mut target);
        let snapshot_after_first = (
            target.active_preset(),
            target.active_snap_slot(),
            target.playlist_position(),
            target.frame(),
        );
        restore_class_g(&restored_source, &mut target);
        let snapshot_after_second = (
            target.active_preset(),
            target.active_snap_slot(),
            target.playlist_position(),
            target.frame(),
        );
        assert_eq!(snapshot_after_first, snapshot_after_second);
    }

    #[test]
    fn restore_all_supported_classes_does_not_yet_invoke_class_e_f_or_g() {
        // Pin: PR 3 + PR 4 ship `restore_class_e`, `restore_class_f`,
        // and `restore_class_g` as documented-deferred no-op helpers
        // but explicitly DO NOT add any of them to the convenience
        // helper. PR 5 added `restore_class_d` to the convenience
        // helper alongside the additive `SessionBindings` sub-store
        // extension. When each remaining State-shape extension lands
        // and the corresponding helper widens to a real sub-store
        // assignment, this pin's name updates alongside.
        //
        // The pin shape exercises the convenience helper with a
        // `restored_source` that carries non-default class-a +
        // class-b + class-c mutations, against a `target` that is
        // `State::default`. Class-a, class-b, class-c MUST be
        // applied; class-d operates on empty `session_bindings`
        // (which round-trips as a clear-target no-op for this
        // fixture); class-e/f/g have no State to project.
        let restored_source = seeded_state();
        let mut target = State::default();

        restore_all_supported_classes(&restored_source, &mut target);

        // Class-a applied:
        assert_eq!(target.active_preset(), restored_source.active_preset());
        // Class-b applied:
        assert_eq!(
            target.active_snap_slot(),
            restored_source.active_snap_slot(),
        );
        // Class-c applied:
        assert_eq!(
            target.playlist_position(),
            restored_source.playlist_position(),
        );
        // Class-d applied (empty bindings round-trip):
        assert_eq!(
            target.session_bindings(),
            restored_source.session_bindings()
        );
    }

    #[test]
    fn restore_all_supported_classes_now_includes_class_c() {
        // Extends the PR 1 pin: class-c is added to the convenience
        // helper in PR 2. Class-d is now included via the
        // session-bindings sub-store; class-e through class-g remain
        // documented-deferred no-ops.
        let restored_source = seeded_state();
        // Seed `restored_source` with a class-c mutation so the
        // helper has something to project.
        let bus = InMemoryBus::in_memory();
        let mut restored_source = restored_source;
        bus.dispatch(Command::PlaylistAdvance { step: 23 })
            .expect("playlist command must dispatch");
        bus.commit_all_into(&mut restored_source);

        let mut target = State::default();
        // Seed still-deferred-class fields on the target so we can
        // assert they survive the restoration.
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
        // Class-c applied (new in PR 2):
        assert_eq!(
            target.playlist_position(),
            restored_source.playlist_position(),
        );
        // Per-mapping store remains outside class-a/b/c/d restoration:
        assert_eq!(
            target.mapping_source(MappingId(99)),
            Some(MappingSource::Midi),
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
