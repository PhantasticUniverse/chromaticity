// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Canonical runtime state core.
//!
//! Per ADR-004 (canonical state model) the runtime maintains a single
//! canonical `State` value that is the only source of truth for control
//! values, preset / playlist / Snap state, and integration mappings.
//! Durable mutation of this value is restricted to the state commit
//! thread per ADR-006.
//!
//! Slice-3a expands the state core with minimal in-memory sub-stores
//! mirroring the ADR-004 §Decision topology. Mutation remains
//! crate-private and flows through committed
//! [`crate::command_bus::Event`] values so downstream crates cannot
//! bypass the command bus.

use crate::command_bus::{
    ControlId, ControlValue, Event, EventPayload, MappingId, MappingSource, PresetId, SnapSlotId,
};
use crate::state_migration::V2SchemaMetadata;
use std::collections::{HashMap, HashSet};

/// Canonical runtime state.
///
/// Carries the monotonic `frame` counter plus the ADR-004 §Decision
/// sub-stores. The struct is intentionally opaque: all mutation flows
/// through committed events dispatched on the state commit thread
/// (ADR-006).
#[derive(Debug, Default)]
pub struct State {
    /// Monotonic frame counter advanced by the scheduler tick
    /// (ADR-001 §Decision). Crate-private to keep mutation inside the
    /// state commit thread per ADR-006. Slice-3 part 1 (post-Phase-3)
    /// introduced [`crate::scheduler::Scheduler::tick`] as the
    /// canonical advancement path; the field stays `pub(crate)` so
    /// only [`crate::scheduler`], [`crate::state_replay`]
    /// (canonical-binary encoder reads), and [`crate::state_migration`]
    /// (V1↔V2 envelope decoder writes) can touch it. The public
    /// read surface is [`State::frame`].
    pub(crate) frame: u64,
    /// Control-value store. Mutated by [`State::apply_event`]; read
    /// by [`crate::state_replay`] and [`crate::state_migration`].
    pub(crate) control_store: ControlStore,
    /// Preset store. See [`Self::control_store`].
    pub(crate) preset_store: PresetStore,
    /// Playlist store. See [`Self::control_store`].
    pub(crate) playlist_store: PlaylistStore,
    /// Snap / favslot store. See [`Self::control_store`].
    pub(crate) snap_store: SnapStore,
    /// Integration-mapping store. See [`Self::control_store`].
    pub(crate) mapping_store: MappingStore,
    /// Session-binding device-id state for crash-resume class-d
    /// (OSC/MIDI session bindings per
    /// `docs/26-evidence-artifact-registry.md` row 80 description
    /// column).
    ///
    /// Records the device IDs that were bound at session start so
    /// that crash-resume can rebind on re-enumeration per
    /// MIDI-CONF-09. The per-mapping
    /// [`crate::command_bus::MappingSource`] enum on
    /// [`MappingStore`] continues to carry the OSC / MIDI tag; this
    /// sub-store carries the device-id dimension that was previously
    /// absent.
    ///
    /// Additive: defaults to empty [`SessionBindings`]; existing
    /// tests observe identical [`State`] equality + canonical-binary
    /// encoding because the canonical-binary encoder
    /// ([`crate::state_replay::encode_state_canonical`]) MUST NOT
    /// observe this field (per the same precedent as
    /// [`Self::latest_audio_snapshot`] on row 65, and
    /// [`Self::snapshot_metadata`] on row 32 / 34).
    ///
    /// Wave 4d.3 PR 5 (row 80 class-d) shipping addition.
    pub(crate) session_bindings: SessionBindings,
    /// Phase-3 EV-STATE-MIGRATION envelope metadata.
    ///
    /// Synthetic V1→V2 schema-version scalar carried by the
    /// [`crate::state_migration::encode_snapshot`] envelope only;
    /// the EV-STATE-REPLAY canonical-binary path
    /// ([`crate::state_replay::encode_state_canonical`]) IGNORES
    /// this field, so row 32 / row 34 goldens do not drift when
    /// snapshot_metadata is set. V1 buffers omit the extension
    /// section entirely; decoded V1 → defaults to `None`. V2
    /// buffers carry an explicit `V2SchemaMetadata { schema_version
    /// = 2 }` after the canonical-binary segment.
    pub(crate) snapshot_metadata: Option<V2SchemaMetadata>,
    /// Latest audio uniform snapshot pulled by [`crate::scheduler::Scheduler`]
    /// at the start of each tick (slice-3 PR 3). Read by downstream
    /// subsystems (render-graph dispatch in slice-3 PR 4; script
    /// reactive resolution in slice-5+).
    ///
    /// `None` when the scheduler has not been configured with an
    /// audio pull handler (see
    /// [`crate::scheduler::Scheduler::with_audio_pull`]); the
    /// default Phase-1 [`crate::scheduler::Scheduler::new`] leaves
    /// this unset and the field stays `None` across ticks.
    ///
    /// The EV-STATE-REPLAY (row 32) and EV-PRESET-PLAYLIST (row 34)
    /// T2 corpora drive mutation through
    /// [`crate::command_bus::InMemoryBus::commit_all_into`] directly
    /// without scheduler involvement, so the field is `None` along
    /// those code paths and the canonical-binary encoder
    /// ([`crate::state_replay::encode_state_canonical`]) does not
    /// observe it — golden hashes are preserved verbatim by this PR.
    pub(crate) latest_audio_snapshot: Option<chromaticity_audio::AudioUniformSnapshot>,
}

impl State {
    /// Apply one committed event to the canonical state.
    ///
    /// This remains crate-private: sibling crates dispatch commands and
    /// the commit-thread path calls [`crate::command_bus::InMemoryBus`]
    /// `commit_*_into` helpers.
    pub(crate) fn apply_event(&mut self, event: &Event) {
        match event.payload {
            EventPayload::ControlChanged { id, new_value } => {
                self.control_store.values.insert(id, new_value);
            }
            EventPayload::PresetRecalled { id } => {
                self.preset_store.active = Some(id);
                // Wave 2.4 PERF.C: restore the bank's `ControlStore`
                // snapshot if `id` has been saved. Recalling an
                // unsaved preset leaves the control store untouched
                // (no-op restoration) — that's the Phase-3 baseline
                // behaviour every prior corpus + golden pinned and
                // remains the contract here.
                if let Some(snapshot) = self.preset_store.banks.get(&id) {
                    self.control_store.values = snapshot.clone();
                }
            }
            EventPayload::PresetSaved { id } => {
                // Wave 2.4 PERF.C: capture the current control-store
                // snapshot into the bank keyed by `id`. Saving also
                // activates the preset (mirrors `SnapSaved` which
                // sets `active = Some(slot)` and `recall_preset`
                // already overwriting `active`). The clone is an
                // in-process HashMap copy (no IO); preset-bank
                // durability + on-disk persistence remain Wave 2.5+
                // scope.
                self.preset_store
                    .banks
                    .insert(id, self.control_store.values.clone());
                self.preset_store.active = Some(id);
            }
            EventPayload::PlaylistAdvanced { step } => {
                // Saturate at i64 bounds to prevent silent two's-complement
                // wrap if `position` approaches `i64::MAX` / `i64::MIN`.
                // `apply_event` is currently infallible; promotion to a
                // typed `BusError::PositionOverflow` can land alongside
                // Result-ifying the apply path in a future PR.
                self.playlist_store.position =
                    self.playlist_store.position.saturating_add(i64::from(step));
            }
            EventPayload::SnapRecalled { slot } => {
                self.snap_store.active = Some(slot);
            }
            EventPayload::SnapSaved { slot } => {
                self.snap_store.saved.insert(slot);
                self.snap_store.active = Some(slot);
            }
            EventPayload::SnapCleared { slot } => {
                self.snap_store.saved.remove(&slot);
                if self.snap_store.active == Some(slot) {
                    self.snap_store.active = None;
                }
            }
            EventPayload::MappingBound { id, source } => {
                self.mapping_store.bindings.insert(id, source);
            }
        }
    }

    /// Read the current monotonic frame counter advanced by the
    /// [`crate::scheduler::Scheduler`] tick (ADR-001 §Decision;
    /// ADR-006 single-writer invariant).
    ///
    /// Pre-slice-3 the counter remains at `0` for callers that drive
    /// state mutation via [`crate::command_bus::InMemoryBus::commit_all_into`]
    /// directly; slice-3 part 1 introduced
    /// [`crate::scheduler::Scheduler::tick`] as the canonical
    /// advancement path. The EV-STATE-REPLAY (row 32) and
    /// EV-PRESET-PLAYLIST (row 34) T2 corpora drive mutation through
    /// the commit-all-into path without scheduler involvement, so
    /// their goldens — which encode `frame == 0` — are preserved
    /// verbatim by the slice-3 wave.
    #[must_use]
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Read the current value for a control.
    #[must_use]
    pub fn control_value(&self, id: ControlId) -> Option<ControlValue> {
        self.control_store.values.get(&id).copied()
    }

    /// Read the active preset, if one has been recalled.
    #[must_use]
    pub fn active_preset(&self) -> Option<PresetId> {
        self.preset_store.active
    }

    /// Read the current playlist cursor position.
    #[must_use]
    pub fn playlist_position(&self) -> i64 {
        self.playlist_store.position
    }

    /// Read the active Snap slot, if one has been recalled or saved.
    #[must_use]
    pub fn active_snap_slot(&self) -> Option<SnapSlotId> {
        self.snap_store.active
    }

    /// Return whether a Snap slot currently has saved state.
    #[must_use]
    pub fn snap_slot_saved(&self, slot: SnapSlotId) -> bool {
        self.snap_store.saved.contains(&slot)
    }

    /// Read the integration source bound for a mapping id.
    #[must_use]
    pub fn mapping_source(&self, id: MappingId) -> Option<MappingSource> {
        self.mapping_store.bindings.get(&id).copied()
    }

    /// Read the latest audio uniform snapshot pulled by
    /// [`crate::scheduler::Scheduler`] at the start of the most
    /// recent tick (slice-3 PR 3). `None` when the scheduler was
    /// not configured with an audio pull handler — see
    /// [`crate::scheduler::Scheduler::with_audio_pull`].
    ///
    /// Downstream subsystems (render-graph dispatch in slice-3 PR 4;
    /// script reactive resolution in slice-5+) read through this
    /// accessor. The canonical-binary encoder
    /// ([`crate::state_replay::encode_state_canonical`]) does NOT
    /// observe this field, so EV-STATE-REPLAY (row 32) and
    /// EV-PRESET-PLAYLIST (row 34) goldens are preserved verbatim
    /// by this PR.
    #[must_use]
    pub fn latest_audio_snapshot(&self) -> Option<&chromaticity_audio::AudioUniformSnapshot> {
        self.latest_audio_snapshot.as_ref()
    }

    /// Read the snapshot envelope metadata, if a snapshot has been
    /// decoded into this state. Phase-3 EV-STATE-MIGRATION envelope
    /// surface; production hosts treat this as opaque versioning
    /// metadata. Hashing path
    /// ([`crate::state_replay::canonical_state_hash`]) IGNORES this
    /// field so row 32 / row 34 goldens are not affected by its
    /// presence or value.
    #[must_use]
    pub fn snapshot_metadata(&self) -> Option<V2SchemaMetadata> {
        self.snapshot_metadata
    }

    /// Read the session-binding sub-store (Wave 4d.3 PR 5, row 80
    /// class-d). Returns the OSC + MIDI device-id list captured at
    /// session start so callers (the crash-resume adapter, OSC /
    /// MIDI integration crates, the T2 corpus) can inspect the
    /// recovery-side snapshot. Mutation flows through
    /// [`Self::apply_session_bindings`] — see that method's prose
    /// for the recovery-write-path rationale.
    #[must_use]
    pub fn session_bindings(&self) -> &SessionBindings {
        &self.session_bindings
    }

    /// Apply a [`SessionBindings`] snapshot to this state, replacing
    /// any existing bindings (Wave 4d.3 PR 5, row 80 class-d).
    ///
    /// This is the recovery-side write path used by
    /// [`crate::crash_resume::restore_class_d`] and the integration-
    /// test corpus. Unlike the per-mapping
    /// [`crate::command_bus::Event`] flow, session-binding writes do
    /// NOT route through the canonical command bus — class-d state
    /// is a wholesale snapshot of the device-id list as it was at
    /// session start, captured by the adapter crates at adapter
    /// startup-time and then frozen for the duration of the session.
    /// The recovery exception in ADR-006 covers this:
    /// crash-resume restoration projects the recovered snapshot
    /// directly without re-driving the original committed events.
    pub fn apply_session_bindings(&mut self, bindings: SessionBindings) {
        self.session_bindings = bindings;
    }

    /// Crate-private mutable accessor used by the in-crate T1 pins
    /// to seed fixtures. The sibling
    /// [`crate::state_migration::decode_session_bindings_extension`]
    /// also writes through this path. Public callers use
    /// [`Self::apply_session_bindings`].
    pub(crate) fn session_bindings_mut(&mut self) -> &mut SessionBindings {
        &mut self.session_bindings
    }
}

/// Control-value store.
#[derive(Debug, Default)]
pub(crate) struct ControlStore {
    /// Crate-private; sibling [`crate::state_replay`] reads this for
    /// canonical-binary encoding (sorted by `ControlId.0` for
    /// deterministic FNV-1a-64 hashing per the EV-STATE-REPLAY T2
    /// corpus). Mutation flows through [`State::apply_event`] only.
    pub(crate) values: HashMap<ControlId, ControlValue>,
}

/// Preset store. See [`ControlStore`].
///
/// Wave 2.4 PERF.C extends the store with an in-process preset bank
/// keyed by [`PresetId`]. Each entry holds a snapshot of
/// [`ControlStore::values`] captured at
/// [`EventPayload::PresetSaved`]-apply time. The bank is purely
/// in-memory — preset-bank durability + on-disk persistence remain
/// Wave 2.5+ scope per the Phase-3 `EV-PRESET-PLAYLIST` narrowing.
/// `PresetRecalled` restores controls from the bank IF a matching
/// entry exists; recalling an unsaved preset leaves the control
/// store untouched (matching the existing Phase-3 semantics that
/// every prior corpus + golden hash pins).
///
/// The `banks` field is intentionally NOT observed by
/// [`crate::state_replay::encode_state_canonical`] for the same
/// reason as [`State::latest_audio_snapshot`] and
/// [`State::session_bindings`]: keeping the canonical-binary
/// encoder a frozen-shape function of the Phase-3 baseline so the
/// EV-STATE-REPLAY (row 32) and EV-PRESET-PLAYLIST (row 34)
/// goldens remain invariant under the bank widening.
#[derive(Debug, Default)]
pub(crate) struct PresetStore {
    pub(crate) active: Option<PresetId>,
    pub(crate) banks: HashMap<PresetId, HashMap<ControlId, ControlValue>>,
}

/// Playlist store. See [`ControlStore`].
#[derive(Debug, Default)]
pub(crate) struct PlaylistStore {
    pub(crate) position: i64,
}

/// Snap / favslot store. See [`ControlStore`].
#[derive(Debug, Default)]
pub(crate) struct SnapStore {
    pub(crate) active: Option<SnapSlotId>,
    pub(crate) saved: HashSet<SnapSlotId>,
}

/// Integration-mapping store. See [`ControlStore`].
#[derive(Debug, Default)]
pub(crate) struct MappingStore {
    pub(crate) bindings: HashMap<MappingId, MappingSource>,
}

/// Session-binding state recording the OSC/MIDI device IDs that were
/// active at session start. Crash-resume class-d
/// ([`crate::crash_resume::restore_class_d`]) uses this to rebind on
/// re-enumeration per MIDI-CONF-09.
///
/// Both fields are `Vec<>` to preserve insertion order — which devices
/// were enumerated when in the session — because that ordering matters
/// for the rebind-priority in case of partial-reconnect scenarios.
///
/// Visibility note (Wave 4d.3 PR 5): the struct ships `pub` rather
/// than the broader `pub(crate)` posture used by [`MappingStore`]
/// because (a) crash-resume restoration is a recovery-side write
/// path that does NOT flow through the canonical command-bus
/// commit-thread (the device-id snapshot is repopulated wholesale
/// from the recovered state per ADR-006's recovery exception), and
/// (b) the integration-test corpus + future adapter crates need to
/// construct + observe these values across the snapshot envelope
/// boundary. Field visibility stays `pub` for the same reason.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SessionBindings {
    pub osc_devices: Vec<OscDeviceBinding>,
    pub midi_devices: Vec<MidiDeviceBinding>,
}

/// One OSC device binding (device id plus the mapping ids bound to
/// that device at session-start time). Public so the
/// [`crate::crash_resume::restore_class_d`] helper can construct +
/// read these values when wiring with the OSC adapter crate in a
/// future PR; mirrors the [`MappingId`] visibility posture (`pub`
/// even though [`MappingStore`] itself is `pub(crate)`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OscDeviceBinding {
    pub device_id: String,
    pub mappings: Vec<MappingId>,
}

/// One MIDI device binding. See [`OscDeviceBinding`] for the same
/// visibility + composition posture.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MidiDeviceBinding {
    pub device_id: String,
    pub mappings: Vec<MappingId>,
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-STATE-REPLAY (T1) — state reconciliation baseline.
    //!
    //! T1 per docs/43 §15.5: pins the minimal event-reconciliation
    //! behaviour for control, preset, playlist, Snap, and mapping
    //! stores. The full replay / migration corpus remains a later T2
    //! artifact.

    use super::State;
    use crate::command_bus::{Event, EventPayload, EventSeq};
    use crate::{
        Command, CommandBus, ControlId, ControlValue, InMemoryBus, MappingId, MappingSource,
        PresetId, SnapSlotId,
    };

    #[test]
    fn state_default_is_deterministic() {
        let a = State::default();
        let b = State::default();
        assert_eq!(a.frame, 0);
        assert_eq!(a.frame, b.frame);
    }

    #[test]
    fn committed_events_update_state_stores() {
        let bus = InMemoryBus::in_memory();
        let mut state = State::default();
        let snap = SnapSlotId::new(3).expect("valid snap slot");

        bus.dispatch(Command::ControlSet {
            id: ControlId(7),
            value: ControlValue(0.75),
        })
        .expect("control command must dispatch");
        bus.dispatch(Command::PresetRecall { id: PresetId(2) })
            .expect("preset command must dispatch");
        bus.dispatch(Command::PlaylistAdvance { step: 4 })
            .expect("playlist command must dispatch");
        bus.dispatch(Command::SnapSave { slot: snap })
            .expect("snap save command must dispatch");
        bus.dispatch(Command::SnapRecall { slot: snap })
            .expect("snap recall command must dispatch");
        bus.dispatch(Command::MappingBind {
            id: MappingId(9),
            source: MappingSource::Midi,
        })
        .expect("mapping command must dispatch");

        let events = bus.commit_all_into(&mut state);
        assert_eq!(events.len(), 6);
        assert_eq!(state.control_value(ControlId(7)), Some(ControlValue(0.75)));
        assert_eq!(state.active_preset(), Some(PresetId(2)));
        assert_eq!(state.playlist_position(), 4);
        assert_eq!(state.active_snap_slot(), Some(snap));
        assert!(state.snap_slot_saved(snap));
        assert_eq!(
            state.mapping_source(MappingId(9)),
            Some(MappingSource::Midi)
        );
    }

    #[test]
    fn playlist_advance_saturates_at_i64_max() {
        // Risk 2: silent two's-complement wrap if `position + step`
        // crosses `i64::MAX`. `apply_event` clamps via `saturating_add`
        // until the apply path can be Result-ified end-to-end.
        let mut state = State::default();
        state.playlist_store.position = i64::MAX - 1;
        let event = Event {
            seq: EventSeq(0),
            payload: EventPayload::PlaylistAdvanced { step: i32::MAX },
        };
        state.apply_event(&event);
        assert_eq!(state.playlist_position(), i64::MAX);
    }

    #[test]
    fn save_recall_roundtrip() {
        // Wave 2.4 PERF.C T1 round-trip pin: set a control →
        // dispatch PresetSave(0) → change the control → dispatch
        // PresetRecall(0) → assert the control reverted to the
        // saved value. Closes the "scene-author preset capture"
        // axis the Phase-3 EV-PRESET-PLAYLIST narrowing deferred
        // (preset-bank durability + on-disk persistence remain
        // Wave 2.5+ scope).
        let bus = InMemoryBus::in_memory();
        let mut state = State::default();
        let control = ControlId(11);
        let saved_value = ControlValue(0.25);
        let mutated_value = ControlValue(0.95);

        // Set the control then save preset 0 — snapshot captures
        // `saved_value`.
        bus.dispatch(Command::ControlSet {
            id: control,
            value: saved_value,
        })
        .expect("control command must dispatch");
        bus.dispatch(Command::PresetSave { id: PresetId(0) })
            .expect("preset save command must dispatch");
        bus.commit_all_into(&mut state);
        assert_eq!(state.control_value(control), Some(saved_value));
        assert_eq!(state.active_preset(), Some(PresetId(0)));

        // Mutate the control past the saved value.
        bus.dispatch(Command::ControlSet {
            id: control,
            value: mutated_value,
        })
        .expect("control command must dispatch");
        bus.commit_all_into(&mut state);
        assert_eq!(state.control_value(control), Some(mutated_value));

        // Recall preset 0 — control restores to `saved_value`.
        bus.dispatch(Command::PresetRecall { id: PresetId(0) })
            .expect("preset recall command must dispatch");
        bus.commit_all_into(&mut state);
        assert_eq!(state.control_value(control), Some(saved_value));
        assert_eq!(state.active_preset(), Some(PresetId(0)));
    }

    #[test]
    fn recall_unsaved_preset_leaves_controls_untouched() {
        // Phase-3 baseline pin (re-affirmed under the Wave 2.4
        // PERF.C widening): recalling a preset that has never
        // been saved must NOT mutate the control store. This is
        // the invariant that keeps the EV-PRESET-PLAYLIST (row
        // 34) T2 corpus + EV-STATE-REPLAY (row 32) goldens
        // invariant — every prior corpus dispatches PresetRecall
        // against an empty bank and expects controls to stay at
        // whatever the prior ControlSet sequence left them.
        let bus = InMemoryBus::in_memory();
        let mut state = State::default();
        let control = ControlId(7);
        let value = ControlValue(0.5);

        bus.dispatch(Command::ControlSet { id: control, value })
            .expect("control command must dispatch");
        bus.dispatch(Command::PresetRecall { id: PresetId(99) })
            .expect("preset recall command must dispatch");
        bus.commit_all_into(&mut state);

        assert_eq!(state.control_value(control), Some(value));
        assert_eq!(state.active_preset(), Some(PresetId(99)));
    }

    #[test]
    fn snap_clear_removes_saved_and_active_slot() {
        let bus = InMemoryBus::in_memory();
        let mut state = State::default();
        let snap = SnapSlotId::new(5).expect("valid snap slot");

        bus.dispatch(Command::SnapSave { slot: snap })
            .expect("snap save command must dispatch");
        bus.dispatch(Command::SnapClear { slot: snap })
            .expect("snap clear command must dispatch");
        bus.commit_all_into(&mut state);

        assert!(!state.snap_slot_saved(snap));
        assert_eq!(state.active_snap_slot(), None);
    }
}
