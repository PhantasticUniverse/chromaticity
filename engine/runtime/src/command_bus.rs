// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Command bus trait skeleton.
//!
//! Per ADR-001 (runtime model) all external and internal state
//! updates become typed commands validated before commit. Per ADR-006
//! (thread and process model) dispatch crosses thread boundaries:
//! producers run on the scheduler, audio analysis, and integration
//! IO threads, while commit happens on the state commit thread. The
//! trait defined here is the seam between producers and the commit
//! thread; concrete queue-backed implementations, typed command
//! variants, and validation arrive in later slices.

/// Placeholder command envelope.
///
/// First-slice shape: a zero-sized marker so the trait signature
/// compiles without committing to a command taxonomy. The typed
/// variants (ControlSet, PresetRecall, PlaylistAdvance, MappingBind,
/// …) land alongside the event-sourced stores from ADR-004.
#[derive(Debug)]
pub struct Command;

/// Errors returned from [`CommandBus::dispatch`].
///
/// The first slice exposes only a shape; validation-failure and
/// queue-backpressure variants land with the concrete implementation.
#[derive(Debug)]
#[non_exhaustive]
pub enum CommandBusError {}

/// Command bus seam between producer subsystems (scheduler, audio,
/// integration IO) and the state commit thread.
///
/// Implementations MUST uphold the ADR-006 thread-affinity rule:
/// `dispatch` may be called from any producer thread, but the commit
/// that applies the command to the canonical state runs on the state
/// commit thread. `Send + Sync` bounds on the trait object are left
/// to concrete implementations (they are implementation detail, not
/// part of this first-slice contract).
pub trait CommandBus {
    /// Enqueue `command` for validation and commit. Returns once the
    /// command has been accepted by the bus; actual commit is
    /// asynchronous on the commit thread.
    fn dispatch(&self, command: Command) -> Result<(), CommandBusError>;
}
