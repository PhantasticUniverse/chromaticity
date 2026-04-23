// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity OSC integration adapter.
//!
//! Phase-1 slice-2 surface: a real UDP-backed [`adapter::UdpOscAdapter`]
//! that binds the ADR-013 loopback default, non-blocking polls for
//! one packet at a time, applies ADR-013 §5 payload-hardening
//! checks (packet-size cap; bundle-depth cap) via the free function
//! [`adapter::validate_packet`], and exposes a pure-function
//! precedence pin [`adapter::resolve_rate_limit`] for the ADR-013
//! §3 per-route-over-adapter-floor rule. Route-schema namespace
//! types live in [`routes`]; no dispatch, no evaluation yet —
//! slice-3 wires the parsed address to the runtime bus.
//!
//! The `crate::defaults` module holds the ADR-013 constants the
//! conformance gate `EV-NET-HARDENING`
//! (`docs/26-evidence-artifact-registry.md` line 77) asserts
//! against.
//!
//! Governing ADRs:
//!
//! - ADR-005 — integration adapter boundary
//!   (`docs/adr/ADR-005-integration-adapter-boundary.md`).
//! - ADR-013 — networking defaults and security
//!   (`docs/adr/ADR-013-networking-defaults-and-security.md`).

pub mod adapter;
pub mod defaults;
pub(crate) mod routes;
