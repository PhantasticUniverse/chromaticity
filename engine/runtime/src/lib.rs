// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity engine runtime.
//!
//! Phase-1 first slice. Exposes the canonical state core
//! ([`state`]) and the command-bus trait skeleton ([`command_bus`]);
//! concrete command variants, queue-backed bus implementations, and
//! the event-sourced stores from ADR-004 land in subsequent slices.
//! The render-graph implementation will start as a module in this
//! crate and be extracted only when incremental-compile savings or
//! license-boundary concerns justify.
//!
//! Internal types remain crate-private (`pub(crate)`) so downstream
//! crates cannot bypass the command bus; only the module surface is
//! `pub` to let sibling crates in later PRs depend on this one.
//!
//! Governing ADRs:
//!
//! - ADR-001 — runtime model (`docs/adr/ADR-001-runtime-model.md`).
//! - ADR-004 — canonical state model (`docs/adr/ADR-004-state-model.md`).
//! - ADR-006 — thread and process model
//!   (`docs/adr/ADR-006-thread-and-process-model.md`).

pub mod command_bus;
pub mod state;
