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
    /// state commit thread per ADR-006. The scheduler-tick path lands in
    /// a later slice.
    #[allow(dead_code)]
    pub(crate) frame: u64,
    /// Control-value store.
    #[allow(dead_code)]
    pub(crate) control_store: ControlStore,
    /// Preset store.
    #[allow(dead_code)]
    pub(crate) preset_store: PresetStore,
    /// Playlist store.
    #[allow(dead_code)]
    pub(crate) playlist_store: PlaylistStore,
    /// Snap / favslot store.
    #[allow(dead_code)]
    pub(crate) snap_store: SnapStore,
    /// Integration-mapping store.
    #[allow(dead_code)]
    pub(crate) mapping_store: MappingStore,
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
}

/// Control-value store.
#[derive(Debug, Default)]
pub(crate) struct ControlStore {
    values: HashMap<ControlId, ControlValue>,
}

/// Preset store. See [`ControlStore`].
#[derive(Debug, Default)]
pub(crate) struct PresetStore {
    active: Option<PresetId>,
}

/// Playlist store. See [`ControlStore`].
#[derive(Debug, Default)]
pub(crate) struct PlaylistStore {
    position: i64,
}

/// Snap / favslot store. See [`ControlStore`].
#[derive(Debug, Default)]
pub(crate) struct SnapStore {
    active: Option<SnapSlotId>,
    saved: HashSet<SnapSlotId>,
}

/// Integration-mapping store. See [`ControlStore`].
#[derive(Debug, Default)]
pub(crate) struct MappingStore {
    bindings: HashMap<MappingId, MappingSource>,
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
