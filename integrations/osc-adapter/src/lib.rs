// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity OSC integration adapter.
//!
//! Phase-1 slice-2 baseline plus Phase-4a Wave 1a closure: a real
//! UDP-backed [`adapter::UdpOscAdapter`] that binds the ADR-013
//! loopback default, non-blocking polls for one packet at a time,
//! applies ADR-013 §5 payload-hardening checks (packet-size cap;
//! bundle-depth cap) via the free function [`adapter::validate_packet`],
//! and exposes a pure-function precedence pin
//! [`adapter::resolve_rate_limit`] for the ADR-013 §3 per-route-over-
//! adapter-floor rule. Route-schema namespace types live in
//! [`routes`]. Phase-4a Wave 1a (anchor run `25213038199`) landed
//! `EV-INTEGRATION-MATRIX` (row 37) at Pass under the OSC-routing-
//! matrix narrowing alongside `EV-NET-HARDENING` (row 82) at full
//! scope; the deferred camera / Syphon arms remain Wave 1c-β scope
//! per the AO-approved narrowing.
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
pub mod bindings;
pub mod defaults;
pub mod route_registry;
pub mod routes;
