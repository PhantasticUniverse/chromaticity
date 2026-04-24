// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity engine runtime.
//!
//! Phase-1 slice-3a. Exposes the canonical state core
//! ([`state`]) with minimal ADR-004 §Decision sub-stores and
//! the command-bus surface ([`command_bus`]): typed [`Command`] and
//! [`Event`] envelopes, the [`CommandChannel`] pluggable transport
//! trait, a crossbeam-channel-backed [`CrossbeamChannel`]
//! implementation, and the [`InMemoryBus`] `CommandBus` that composes
//! a channel with a monotonic [`EventSeq`] counter. Persistent
//! reconciliation, validation, migration, and a cross-process channel
//! implementation land in later slices. The
//! render-graph implementation remains reserved as a future module in
//! this crate, to be extracted only when incremental-compile savings
//! or license-boundary concerns justify.
//!
//! Internal state fields and sub-stores remain crate-private
//! (`pub(crate)`) so downstream crates cannot bypass the command bus;
//! only the public command / event / trait surface is exported for
//! sibling crates to consume in later slices.
//!
//! Governing ADRs:
//!
//! - ADR-001 — runtime model (`docs/adr/ADR-001-runtime-model.md`).
//! - ADR-004 — canonical state model
//!   (`docs/adr/ADR-004-state-model.md`).
//! - ADR-006 — thread and process model
//!   (`docs/adr/ADR-006-thread-and-process-model.md`).

// Convenience re-exports of the slice-2 public surface. Sibling
// engine crates depend on `chromaticity_runtime::Command` and
// `chromaticity_runtime::Event` shapes from slice-3 onward.
pub use command_bus::{
    Command, CommandBus, CommandBusError, CommandChannel, ControlId, ControlValue,
    CrossbeamChannel, Event, EventPayload, EventSeq, InMemoryBus, MappingId, MappingSource,
    PresetId, SnapSlotId,
};

pub mod command_bus;
pub mod state;
