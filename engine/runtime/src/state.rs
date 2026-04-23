// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Canonical runtime state core.
//!
//! Evidence: EV-DETERMINISM — determinism-replay baseline.
//!
//! Per ADR-004 (canonical state model) the runtime maintains a single
//! canonical `State` value that is the only source of truth for
//! control values, preset/playlist state, and integration mappings.
//! Durable mutation of this value is restricted to the state commit
//! thread per ADR-006.
//!
//! This first slice introduces only a default-constructible skeleton;
//! concrete stores (controls, presets, playlists, mappings) land in
//! later slices alongside the event-sourced command validator. All
//! fields are crate-private so downstream crates cannot bypass the
//! command bus introduced in [`crate::command_bus`].

/// Canonical runtime state.
///
/// Holds a monotonically non-decreasing `frame` counter as the first
/// determinism-relevant field; subsequent slices extend it with the
/// stores listed in ADR-004 §Decision. The struct is intentionally
/// opaque: all mutation flows through validated commands dispatched
/// on the state commit thread (ADR-006).
#[derive(Debug, Default)]
pub struct State {
    /// Monotonic frame counter advanced by the scheduler tick
    /// (ADR-001 §Decision). Crate-private to keep mutation inside the
    /// state commit thread per ADR-006. `#[allow(dead_code)]` is
    /// scoped to this first slice: the scheduler-tick path that reads
    /// this field lands in a later slice alongside the command-bus
    /// implementation.
    #[allow(dead_code)]
    pub(crate) frame: u64,
}

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
