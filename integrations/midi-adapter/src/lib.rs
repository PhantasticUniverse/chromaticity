// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity MIDI integration adapter.
//!
//! Phase-4a Wave 1b-A surface: a midir-backed [`adapter::MidirInputAdapter`]
//! that opens an inbound MIDI port, queues incoming messages from
//! midir's callback into an in-memory FIFO, drains the queue on
//! non-blocking [`adapter::MidiAdapter::poll`], and accepts /
//! drops messages per the ADR-013-aligned packet-outcome counters.
//! Route dispatch (turning a parsed [`messages::MidiMessage`] into a
//! runtime command on the bus) remains slice-3+ scope; this Wave
//! parses, classifies, and validates only.
//!
//! The crate intentionally mirrors the OSC adapter shape — trait,
//! defaults module, message-taxonomy module — so the IntegrationHub
//! translation layer (ADR-005 "Decision" bullet 3) can consume both
//! adapters through the same protocol-abstract boundary. There is
//! no MIDI-specific ADR being introduced; ADR-005 already covers the
//! adapter boundary and ADR-013 already pins the networking-default
//! constants this Wave inherits via the default-value module.
//!
//! Conformance gate: `EV-MIDI-EXTENDED`
//! (`docs/26-evidence-artifact-registry.md` row 80) covers extended
//! MIDI surface — pitch bend, program change, MIDI Clock, 14-bit CC
//! pairs, NRPN/RPN, MTC, and MIDI-learn save-quit-relaunch
//! persistence. The corpus lives at
//! `tests/midi_extended_corpus/main.rs` and is dual-tagged with
//! `EV-INTEGRATION-MATRIX` (row 37 MIDI arm) so the Phase-4a Wave 1a
//! OSC-only narrowing is widened to include MIDI conformance.
//!
//! Governing ADRs:
//!
//! - ADR-005 — integration adapter boundary
//!   (`docs/adr/ADR-005-integration-adapter-boundary.md`).
//! - ADR-013 — networking defaults and security
//!   (`docs/adr/ADR-013-networking-defaults-and-security.md`).

pub mod adapter;
pub mod defaults;
pub mod messages;
