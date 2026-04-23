// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity OSC integration adapter.
//!
//! First-slice implementation: trait skeleton plus the ADR-013
//! networking-defaults constants the runtime conformance gate
//! `EV-NET-HARDENING` (`docs/26-evidence-artifact-registry.md`
//! line 77) asserts against. Wire-level OSC parsing lands in a
//! follow-up slice alongside the first OSC-codec dependency.
//!
//! Governing ADRs:
//!
//! - ADR-005 — integration adapter boundary
//!   (`docs/adr/ADR-005-integration-adapter-boundary.md`).
//! - ADR-013 — networking defaults and security
//!   (`docs/adr/ADR-013-networking-defaults-and-security.md`).
//!   Fixes the bind-scope default, per-route rate-limit default,
//!   and packet-hardening caps this crate must honour.

pub mod adapter;
pub mod defaults;
