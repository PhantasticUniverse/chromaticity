// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Canonical runtime state core.
//!
//! Produces evidence for `EV-DETERMINISM` via the determinism-replay
//! baseline pin in the `tests` module below.
//!
//! Per ADR-004 (canonical state model) the runtime maintains a single
//! canonical `State` value that is the only source of truth for
//! control values, preset / playlist state, and integration mappings.
//! Durable mutation of this value is restricted to the state commit
//! thread per ADR-006.
//!
//! Slice-2 expands the state core with placeholder sub-stores mirroring
//! the ADR-004 §Decision topology (controls, presets, playlists,
//! mappings). Each sub-store is a crate-private empty data-shape
//! declaration; the event-sourced reconciliation path that populates
//! them lands in slice-3 alongside the validator. All fields are
//! crate-private so downstream crates cannot bypass the command bus in
//! [`crate::command_bus`].

/// Canonical runtime state.
///
/// Carries the monotonic `frame` counter plus the four ADR-004 §Decision
/// sub-stores as placeholder topology. The struct is intentionally
/// opaque: all mutation flows through validated commands dispatched on
/// the state commit thread (ADR-006).
#[derive(Debug, Default)]
pub struct State {
    /// Monotonic frame counter advanced by the scheduler tick
    /// (ADR-001 §Decision). Crate-private to keep mutation inside the
    /// state commit thread per ADR-006. `#[allow(dead_code)]` is
    /// slice-2 transitional: the scheduler-tick path that advances this
    /// field lands alongside the event-sourced reconciliation layer in
    /// slice-3.
    #[allow(dead_code)]
    pub(crate) frame: u64,
    /// Control-value store. Populated by the control-set reconciliation
    /// path in slice-3.
    #[allow(dead_code)]
    pub(crate) control_store: ControlStore,
    /// Preset store. Populated by the preset-recall reconciliation path
    /// in slice-3.
    #[allow(dead_code)]
    pub(crate) preset_store: PresetStore,
    /// Playlist store. Populated by the playlist-advance reconciliation
    /// path in slice-3.
    #[allow(dead_code)]
    pub(crate) playlist_store: PlaylistStore,
    /// Integration-mapping store. Populated by the mapping-bind
    /// reconciliation path in slice-3.
    #[allow(dead_code)]
    pub(crate) mapping_store: MappingStore,
}

/// Placeholder for the slice-3 control-value store.
///
/// Empty data-shape declaration reserving the topology slot per
/// ADR-004 §Decision. No runtime behaviour; Slice-3's event-sourced
/// reconciliation layer grows the concrete storage.
#[derive(Debug, Default)]
pub(crate) struct ControlStore {}

/// Placeholder for the slice-3 preset store. See [`ControlStore`].
#[derive(Debug, Default)]
pub(crate) struct PresetStore {}

/// Placeholder for the slice-3 playlist store. See [`ControlStore`].
#[derive(Debug, Default)]
pub(crate) struct PlaylistStore {}

/// Placeholder for the slice-3 integration-mapping store. See
/// [`ControlStore`].
#[derive(Debug, Default)]
pub(crate) struct MappingStore {}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-DETERMINISM (T0) — determinism-replay baseline.
    //!
    //! Phase-1 baseline test claiming the EV-DETERMINISM evidence row
    //! (`docs/26-evidence-artifact-registry.md` line 37). The full
    //! determinism-replay suite arrives with the event-sourced command
    //! validator in a later slice; this test merely asserts the
    //! canonical state core constructs at its deterministic defaults
    //! without panicking. T0 per docs/43 §15.5: the subject module's
    //! public surface is a `#[derive(Default)]` data-shape declaration
    //! with no non-derive-generated behaviour. Upgrade path is T2
    //! once the determinism-replay corpus lands.
    use super::State;

    #[test]
    fn state_default_is_deterministic() {
        let a = State::default();
        let b = State::default();
        assert_eq!(a.frame, 0);
        assert_eq!(a.frame, b.frame);
    }
}
