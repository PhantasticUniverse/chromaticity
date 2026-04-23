// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! OSC adapter trait and UDP-backed implementation.
//!
//! Slice-2 ships the first non-stub `OscAdapter` implementation:
//! [`UdpOscAdapter`] binds a `std::net::UdpSocket` to the ADR-013
//! loopback default, polls non-blocking, and applies the ADR-013
//! §5 payload-hardening checks (packet-size cap; bundle-depth cap)
//! before accepting a parsed packet. Route dispatch — turning a
//! parsed message into a command on the runtime bus — remains
//! slice-3 scope; slice-2 parses and validates only.
//!
//! Governing ADRs:
//!
//! - ADR-005 — integration adapter boundary
//!   (`docs/adr/ADR-005-integration-adapter-boundary.md`). The
//!   [`OscAdapter`] trait is the `ProtocolAdapter` shape named in
//!   ADR-005 "Decision" bullet 1, specialised to OSC. The
//!   IntegrationHub translation layer (ADR-005 "Decision" bullet 3)
//!   is out of scope for this crate.
//! - ADR-013 — networking defaults and security
//!   (`docs/adr/ADR-013-networking-defaults-and-security.md`).
//!   Implementations MUST honour `crate::defaults` for bind scope,
//!   packet caps, bundle depth, and per-route rate ceilings unless
//!   an operator override is installed per ADR-013 §6.
//!
//! Slice-2 pins the ADR-013 §3 per-route rate-limit precedence
//! rule as a pure function, [`resolve_rate_limit`]. The enforcing
//! state machine that counts inbound messages per route and emits
//! `osc-rate-limited` diagnostics lands with slice-3; the
//! precedence rule is pinned now so the slice-3 implementation
//! inherits a regression fence on the ordering.

use core::net::{IpAddr, SocketAddr};
use std::net::UdpSocket;

use rosc::{OscPacket, decoder};

use crate::defaults::{DEFAULT_INBOUND_BIND, DEFAULT_MAX_BUNDLE_DEPTH, DEFAULT_MAX_PACKET_BYTES};

/// Errors surfaced by [`UdpOscAdapter`] and by the packet-validation
/// free function [`validate_packet`].
///
/// Slice-2 surfaces diagnostics without wiring them to a runtime
/// event channel. Slice-3 routes these through the `chromaticity-
/// runtime` bus as validator-rejection events.
#[derive(Debug)]
#[non_exhaustive]
pub enum OscAdapterError {
    /// `bind` could not acquire the requested socket.
    BindFailed(std::io::Error),
    /// `poll` hit a non-`WouldBlock` receive error.
    RecvFailed(std::io::Error),
    /// Inbound packet exceeded [`DEFAULT_MAX_PACKET_BYTES`]. Size
    /// check fires before rosc decoder allocation per ADR-013 §5
    /// bullet 1; the decoder never sees the oversized buffer.
    OversizedPacket {
        /// Actual packet size in bytes.
        actual: usize,
        /// Configured cap (ADR-013 §5 bullet 1 default is 64 KiB).
        max: usize,
    },
    /// Inbound packet nested bundles beyond
    /// [`DEFAULT_MAX_BUNDLE_DEPTH`] per ADR-013 §5 bullet 2.
    BundleDepthExceeded {
        /// Observed depth.
        actual: u8,
        /// Configured cap (ADR-013 §5 bullet 2 default is 4).
        max: u8,
    },
    /// rosc decoder rejected the packet shape.
    MalformedPacket(String),
}

impl core::fmt::Display for OscAdapterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BindFailed(e) => write!(f, "osc bind failed: {e}"),
            Self::RecvFailed(e) => write!(f, "osc recv failed: {e}"),
            Self::OversizedPacket { actual, max } => {
                write!(f, "osc packet {actual} bytes exceeds cap {max}")
            }
            Self::BundleDepthExceeded { actual, max } => {
                write!(f, "osc bundle depth {actual} exceeds cap {max}")
            }
            Self::MalformedPacket(msg) => write!(f, "osc packet malformed: {msg}"),
        }
    }
}

impl std::error::Error for OscAdapterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BindFailed(e) | Self::RecvFailed(e) => Some(e),
            _ => None,
        }
    }
}

/// OSC protocol adapter.
///
/// The trait surface is bind / unbind / poll. Slice-3 will introduce
/// the hub-facing event channel (validator-rejection + route
/// dispatch events) once `chromaticity-runtime::Event` grows the
/// integration-adapter variants.
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
    /// payload before accepting it.
    fn poll(&mut self) -> Result<(), Self::Error>;
}

/// UDP-backed OSC adapter.
///
/// Slice-2 implementation: bind a non-blocking
/// `std::net::UdpSocket` to the ADR-013 loopback default, poll for
/// one packet at a time, run the packet through [`validate_packet`],
/// drop the packet on hardening failure. No route dispatch happens
/// in slice-2 — slice-3 wires the parsed packet to the runtime bus
/// via the event channel the `OscAdapter` trait will grow.
///
/// The `allowlist` field is the structural placeholder for
/// ADR-013 §2 "accept from bound interface only (empty
/// allowlist)": slice-2 ships the empty default; slice-3
/// implements the actual `recv_from` source-IP filter against the
/// configured list. The T1 pin [`empty_allowlist_blocks_all`]
/// asserts the default-constructed adapter carries no allowlist
/// entries.
pub struct UdpOscAdapter {
    socket: Option<UdpSocket>,
    recv_buf: Vec<u8>,
    allowlist: Vec<IpAddr>,
}

impl UdpOscAdapter {
    /// Construct a fresh, unbound adapter with the ADR-013 defaults.
    ///
    /// `allowlist` starts empty per ADR-013 §2. The `recv_buf`
    /// pre-allocates [`DEFAULT_MAX_PACKET_BYTES`] so the
    /// non-blocking receive path never allocates during poll.
    #[must_use]
    pub fn new() -> Self {
        Self {
            socket: None,
            recv_buf: vec![0u8; DEFAULT_MAX_PACKET_BYTES],
            allowlist: Vec::new(),
        }
    }

    /// Number of entries currently on the allowlist.
    #[must_use]
    pub fn allowlist_len(&self) -> usize {
        self.allowlist.len()
    }

    /// `true` iff the allowlist is empty (the ADR-013 §2 default).
    #[must_use]
    pub fn allowlist_is_empty(&self) -> bool {
        self.allowlist.is_empty()
    }
}

impl Default for UdpOscAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl OscAdapter for UdpOscAdapter {
    type Error = OscAdapterError;

    fn bind(&mut self, addr: SocketAddr) -> Result<(), Self::Error> {
        let socket = UdpSocket::bind(addr).map_err(OscAdapterError::BindFailed)?;
        socket
            .set_nonblocking(true)
            .map_err(OscAdapterError::BindFailed)?;
        self.socket = Some(socket);
        Ok(())
    }

    fn unbind(&mut self) -> Result<(), Self::Error> {
        // Dropping the socket releases the OS-level binding.
        self.socket = None;
        Ok(())
    }

    fn poll(&mut self) -> Result<(), Self::Error> {
        let Some(socket) = self.socket.as_ref() else {
            return Ok(());
        };
        match socket.recv_from(&mut self.recv_buf) {
            Ok((len, _src_addr)) => {
                // Slice-2 parses and validates but drops the packet;
                // slice-3 routes it via the hub-facing event channel.
                let _ = validate_packet(&self.recv_buf[..len]);
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
            Err(e) => Err(OscAdapterError::RecvFailed(e)),
        }
    }
}

/// Validate an inbound OSC packet against the ADR-013 §5
/// payload-hardening caps.
///
/// Slice-2 pin: the size gate runs BEFORE any rosc allocation.
/// A buffer larger than [`DEFAULT_MAX_PACKET_BYTES`] returns
/// [`OscAdapterError::OversizedPacket`] without invoking the
/// decoder. The bundle-depth walk runs on the successfully-parsed
/// packet and rejects nesting beyond [`DEFAULT_MAX_BUNDLE_DEPTH`].
///
/// Kept as a free function so the drift pins can exercise the
/// hardening path with a synthesised `&[u8]` and no live socket.
pub fn validate_packet(bytes: &[u8]) -> Result<OscPacket, OscAdapterError> {
    if bytes.len() > DEFAULT_MAX_PACKET_BYTES {
        return Err(OscAdapterError::OversizedPacket {
            actual: bytes.len(),
            max: DEFAULT_MAX_PACKET_BYTES,
        });
    }

    let (_remaining, packet) = decoder::decode_udp(bytes)
        .map_err(|e| OscAdapterError::MalformedPacket(format!("{e:?}")))?;

    let depth = bundle_depth(&packet);
    if depth > DEFAULT_MAX_BUNDLE_DEPTH {
        return Err(OscAdapterError::BundleDepthExceeded {
            actual: depth,
            max: DEFAULT_MAX_BUNDLE_DEPTH,
        });
    }

    Ok(packet)
}

/// Walk an [`OscPacket`] tree and return the maximum bundle-nesting
/// depth. A bare message has depth 0; a top-level bundle of
/// messages has depth 1; a bundle of bundles-of-messages has depth 2.
fn bundle_depth(packet: &OscPacket) -> u8 {
    match packet {
        OscPacket::Message(_) => 0,
        OscPacket::Bundle(bundle) => {
            let max_child = bundle.content.iter().map(bundle_depth).max().unwrap_or(0);
            max_child.saturating_add(1)
        }
    }
}

/// Resolve the effective per-route rate-limit per ADR-013 §3
/// precedence: the per-route ceiling wins over the per-adapter
/// floor when both are configured.
///
/// Slice-2 pins this as a pure function so the slice-3 state
/// machine (actual inbound counters, `osc-rate-limited` diagnostic
/// emission) inherits a regression fence on the precedence
/// ordering. The `_adapter_floor` argument is intentionally unused
/// in slice-2's return value — the test
/// [`per_route_rate_limit_precedes_adapter_floor`] pins this as
/// the expected shape.
#[must_use]
pub fn resolve_rate_limit(per_route: u32, _adapter_floor: u32) -> u32 {
    per_route
}

/// Construct the default inbound bind address from
/// [`DEFAULT_INBOUND_BIND`] with `port` applied.
#[must_use]
pub fn default_inbound_addr(port: u16) -> SocketAddr {
    SocketAddr::new(DEFAULT_INBOUND_BIND, port)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod drift_pins {
    //! Evidence: EV-NET-HARDENING (T1) — ADR-013 §2 empty-allowlist
    //! default, §5 bullet 1 size-gate-before-allocation ordering,
    //! and §3 per-route rate-limit precedence.
    //!
    //! T1 per docs/43 §15.5: each test pins a specific ADR-013
    //! clause against the slice-2 implementation. Any drift fires
    //! the corresponding pin at the exact ADR-section boundary,
    //! routing review to the `EV-NET-HARDENING` gate before merge.
    //! T1 counts toward `EV-EVIDENCE-TAG-COVERAGE`; Phase-4a exit
    //! requires T2 over the net-hardening fixture corpus.

    use super::{OscAdapterError, UdpOscAdapter, resolve_rate_limit, validate_packet};
    use crate::defaults::DEFAULT_MAX_PACKET_BYTES;

    #[test]
    fn empty_allowlist_blocks_all() {
        let adapter = UdpOscAdapter::new();
        // ADR-013 §2 default: an adapter with an empty allowlist
        // accepts from the bound interface only (slice-3 wires the
        // recv_from source-IP filter; slice-2 pins the structural
        // default).
        assert!(adapter.allowlist_is_empty());
        assert_eq!(adapter.allowlist_len(), 0);
    }

    #[test]
    fn oversized_packet_rejected_at_parse_not_at_allocation() {
        // Buffer one byte above the cap. If the size gate did not
        // run before rosc, the decoder would be fed an
        // unsanitized-length buffer. The assertion is: the error
        // variant is OversizedPacket (size-gate fired), NOT
        // MalformedPacket (rosc parsed and rejected).
        let oversized = vec![0u8; DEFAULT_MAX_PACKET_BYTES + 1];
        match validate_packet(&oversized) {
            Err(OscAdapterError::OversizedPacket { actual, max }) => {
                assert_eq!(actual, DEFAULT_MAX_PACKET_BYTES + 1);
                assert_eq!(max, DEFAULT_MAX_PACKET_BYTES);
            }
            other => panic!("expected OversizedPacket, got {other:?}"),
        }
    }

    #[test]
    fn per_route_rate_limit_precedes_adapter_floor() {
        // ADR-013 §3: per-route rate ceiling takes precedence over
        // the per-adapter floor when both are configured.
        assert_eq!(resolve_rate_limit(100, 200), 100);
        assert_eq!(resolve_rate_limit(50, 100), 50);
        // Precedence holds even when per_route is larger than the
        // floor; slice-3 will cap against the §3 hard 1000 msg/s
        // ceiling independently.
        assert_eq!(resolve_rate_limit(500, 100), 500);
    }
}
