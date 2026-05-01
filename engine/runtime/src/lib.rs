// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity engine runtime.
//!
//! Exposes the canonical state core ([`state`]) with the ADR-004
//! §Decision sub-stores and the command-bus surface ([`command_bus`]):
//! typed [`Command`] and [`Event`] envelopes, the [`CommandChannel`]
//! pluggable transport trait, a crossbeam-channel-backed
//! [`CrossbeamChannel`] implementation, and the [`InMemoryBus`]
//! `CommandBus` that composes a channel with a monotonic [`EventSeq`]
//! counter. Post-Phase-3 (`ec10ee7`) the crate also carries the
//! deterministic state-replay path ([`state_replay`], Phase-3 wave PR 1
//! / `EV-STATE-REPLAY`), the V1↔V2 forward-migration envelope
//! ([`state_migration`], Phase-3 wave PR 4 / `EV-STATE-MIGRATION`),
//! and the observable-subsystem proxy benchmarks under
//! `engine/runtime/benches/budgets.rs` (Phase-3 wave PR 6 /
//! `EV-PERF-BUDGETS` Path B nightly criterion harness). The
//! render-graph implementation remains reserved as a future module in
//! this crate, to be extracted only when incremental-compile savings
//! or license-boundary concerns justify.
//!
//! Internal state fields and sub-stores remain crate-private
//! (`pub(crate)`) so downstream crates cannot bypass the command bus;
//! only the public command / event / trait surface is exported for
//! sibling crates to consume.
//!
//! Slice-5.3 (Phase-2, behind the `script-bridge` Cargo feature) adds
//! the [`script_bridge`] module: an adapter that implements the
//! slice-5.1 `chromaticity_script::ScriptCommandSender` trait against
//! a host-owned bounded `crossbeam-channel` consumer queue. The
//! adapter forwards [`chromaticity_script::ScriptCommand`] values
//! the script emits (post slice-5.2 catalog routing) onto a host
//! drain the runtime owns. The runtime [`Command`] enum is NOT
//! widened by slice 5.3 — typed reactive writes are not part of the
//! Phase-1 [`Command`] vocabulary, and slice 5.3 keeps that vocabulary
//! frozen by routing the script-emitted values onto a sibling
//! consumer channel rather than into the runtime command bus
//! directly. The host integration that decides whether to translate
//! a `ScriptCommand::TypedReactiveWrite` into a runtime state
//! mutation lives at the call site (typically the script tick loop
//! or a per-frame drain on the commit thread); slice 5.3 keeps the
//! seam minimal so that translation policy stays out-of-crate.
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

#[cfg(feature = "script-bridge")]
pub use script_bridge::{
    ScriptBridgeError, ScriptBridgeReceiver, ScriptCommandRouter, script_bridge,
};

pub mod command_bus;
#[cfg(feature = "script-bridge")]
pub mod script_bridge;
pub mod state;
pub mod state_migration;
pub mod state_replay;
