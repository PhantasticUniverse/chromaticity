// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! OSC adapter trait skeleton.
//!
//! Defines the adapter-boundary shape only; the wire-level OSC
//! implementation (datagram decode, bundle expansion, route
//! dispatch) lands in a later slice alongside the first dependency
//! on an OSC codec crate. No `pub` surface yet beyond the trait
//! itself.
//!
//! Governing ADRs:
//!
//! - ADR-005 — integration adapter boundary
//!   (`docs/adr/ADR-005-integration-adapter-boundary.md`). The
//!   trait below is the `ProtocolAdapter` shape named in ADR-005
//!   "Decision" bullet 1, specialised to OSC. The IntegrationHub
//!   translation layer (ADR-005 "Decision" bullet 3) is out of
//!   scope for this crate.
//! - ADR-013 — networking defaults and security
//!   (`docs/adr/ADR-013-networking-defaults-and-security.md`).
//!   Implementations MUST honour `crate::defaults` for bind scope,
//!   packet caps, bundle depth, and per-route rate ceilings unless
//!   an operator override is installed per ADR-013 §6.

use core::net::SocketAddr;

/// OSC protocol adapter.
///
/// The trait is intentionally minimal: bind, unbind, poll. The
/// hub-facing event shape is deferred until `engine/runtime` lands
/// the canonical command/event type set; this keeps ADR-005's
/// "no protocol-specific logic in core rendering or state stores"
/// invariant cheap to hold.
///
/// All methods return `Result` with an associated error type so a
/// later slice can substitute a concrete error enum (malformed
/// payload, oversize packet, rate-limit drop, bind refused, ...)
/// without touching call sites.
pub trait OscAdapter {
    /// Adapter-specific error type.
    type Error;

    /// Bind the inbound listener.
    ///
    /// Implementations MUST default to
    /// [`crate::defaults::DEFAULT_INBOUND_BIND`] when the caller
    /// does not supply an operator-configured address. Callers that
    /// expand the bind scope beyond loopback are asserting the
    /// operator opt-in contract in ADR-013 §1.
    fn bind(&mut self, addr: SocketAddr) -> Result<(), Self::Error>;

    /// Release any held listener and source it may own.
    ///
    /// Idempotent: calling `unbind` on an already-unbound adapter
    /// is not an error. Implementations MUST NOT leak the bound
    /// socket across `unbind` boundaries.
    fn unbind(&mut self) -> Result<(), Self::Error>;

    /// Poll the adapter once for work.
    ///
    /// Non-blocking. The poll cadence is owned by the
    /// IntegrationHub (ADR-005 bullet 3); the adapter itself does
    /// not spawn threads or timers. Implementations MUST enforce
    /// [`crate::defaults::DEFAULT_MAX_PACKET_BYTES`] and
    /// [`crate::defaults::DEFAULT_MAX_BUNDLE_DEPTH`] on every
    /// payload before accepting it, and MUST apply the per-route
    /// rate ceiling in
    /// [`crate::defaults::DEFAULT_RATE_LIMIT_MSGS_PER_SEC`] before
    /// emitting work to the hub.
    fn poll(&mut self) -> Result<(), Self::Error>;
}
