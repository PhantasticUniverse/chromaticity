// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-OBSERVABILITY (T1) — adapter-path coverage for the
//! `chromaticity_osc_inbound_packets_total` outcome counter wire-up.
//!
//! T1 per `docs/43-development-standards.md` §15.5: each test pins the
//! slice-2 adapter-side emission contract for one
//! [`OscPacketOutcome`] variant against a live
//! `std::net::UdpSocket` round-trip. The free-function drift pins in
//! `integrations/osc-adapter/src/adapter.rs` cover `validate_packet`
//! in isolation; this file is additive and covers the
//! `UdpOscAdapter::poll` -> `record_outcome` -> `packet_outcome_count`
//! chain that slice-3 will rebase onto the runtime-bus
//! [`MetricEvent::OscInboundPackets`] emission
//! (`engine/observability/src/catalog.rs`).
//!
//! Coverage is intentionally limited to the two outcomes reachable
//! through a live UDP socket in slice-2:
//!
//! - [`OscPacketOutcome::Accepted`]: well-formed OSC message sent
//!   from a peer `UdpSocket` to the adapter's bound ephemeral port.
//! - [`OscPacketOutcome::ParseError`]: bytes that are neither a
//!   valid OSC message (no leading `/`) nor a `#bundle`; rosc's
//!   decoder returns `OscAdapterError::MalformedPacket`, which the
//!   adapter folds into `ParseError` per the catalog's five-outcome
//!   vocabulary (docs/31).
//!
//! Unreachable outcomes in this slice:
//!
//! - [`OscPacketOutcome::OversizedDropped`] is not observable via
//!   live UDP. UDP-over-IPv4 caps the per-datagram payload at 65,507
//!   bytes (RFC 768 UDP length + RFC 791 IP total-length, each
//!   16-bit), strictly below `DEFAULT_MAX_PACKET_BYTES` (65,536).
//!   The size-gate branch is covered at the free-function tier by
//!   `oversized_packet_rejected_at_parse_not_at_allocation` in
//!   `adapter.rs` and exposed by the adapter-side counter as a
//!   never-incremented sentinel (asserted at 0 below).
//! - [`OscPacketOutcome::AllowlistRejected`] and
//!   [`OscPacketOutcome::RateLimitedDropped`] remain reserved for
//!   the allowlist / rate-limit slice; slice-2 never produces them,
//!   so the adapter-side counter stays pinned at 0.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::net::{SocketAddr, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};

use chromaticity_observability::catalog::OscPacketOutcome;
use chromaticity_osc_adapter::adapter::{OscAdapter, UdpOscAdapter};

/// Upper bound on how long we wait for a UDP packet to traverse
/// loopback and become visible to `poll()`. Loopback typically
/// completes in microseconds; this is a generous ceiling so a
/// scheduler stall cannot flake the suite.
const POLL_DEADLINE: Duration = Duration::from_secs(1);

/// Gap between successive `poll()` spins while waiting for an
/// expected counter advancement. Keeps CPU usage bounded without
/// materially slowing the happy path.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Spin-poll the adapter until `outcome`'s counter advances past
/// `baseline` or [`POLL_DEADLINE`] elapses. Returns the last
/// observed counter value; equals `baseline` iff the deadline fired
/// without a packet arriving.
fn poll_until(adapter: &mut UdpOscAdapter, outcome: OscPacketOutcome, baseline: u64) -> u64 {
    let start = Instant::now();
    loop {
        adapter.poll().expect("poll on bound adapter succeeds");
        let current = adapter.packet_outcome_count(outcome);
        if current != baseline {
            return current;
        }
        if start.elapsed() >= POLL_DEADLINE {
            return current;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn bind_loopback_adapter() -> (UdpOscAdapter, SocketAddr) {
    let mut adapter = UdpOscAdapter::new();
    let bind_addr: SocketAddr = "127.0.0.1:0"
        .parse()
        .expect("loopback:0 parses as SocketAddr");
    adapter.bind(bind_addr).expect("bind ephemeral loopback");
    let local = adapter
        .local_addr()
        .expect("bound adapter exposes its local_addr");
    (adapter, local)
}

#[test]
fn poll_increments_accepted_counter_on_well_formed_message() {
    let (mut adapter, bound) = bind_loopback_adapter();
    let sender = UdpSocket::bind("127.0.0.1:0").expect("bind sender socket");

    // Minimal well-formed OSC 1.0 message. Per the OSC 1.0 spec the
    // wire shape is an address string + type-tag string, each null-
    // terminated and padded to a 4-byte boundary. `/t` at 2 bytes
    // pads to 4; `,` (empty arg list) at 1 byte pads to 4. Total 8
    // bytes, which rosc's `decode_udp` accepts as `OscMessage`.
    let valid_osc: [u8; 8] = [b'/', b't', 0, 0, b',', 0, 0, 0];
    sender
        .send_to(&valid_osc, bound)
        .expect("send well-formed osc to adapter");

    let accepted = poll_until(&mut adapter, OscPacketOutcome::Accepted, 0);
    assert_eq!(
        accepted, 1,
        "poll() should record one Accepted outcome for a well-formed packet",
    );
    assert_eq!(
        adapter.packet_outcome_count(OscPacketOutcome::ParseError),
        0,
        "well-formed packet must not advance the ParseError counter",
    );
    assert_eq!(
        adapter.packet_outcome_count(OscPacketOutcome::OversizedDropped),
        0,
        "size-gate branch is unreachable via live UDP; counter stays at 0",
    );
    assert_eq!(
        adapter.packet_outcome_count(OscPacketOutcome::AllowlistRejected),
        0,
        "allowlist outcome is reserved for a future slice",
    );
    assert_eq!(
        adapter.packet_outcome_count(OscPacketOutcome::RateLimitedDropped),
        0,
        "rate-limit outcome is reserved for a future slice",
    );
}

#[test]
fn poll_increments_parse_error_counter_on_malformed_bytes() {
    let (mut adapter, bound) = bind_loopback_adapter();
    let sender = UdpSocket::bind("127.0.0.1:0").expect("bind sender socket");

    // Bytes that are neither a valid OSC message (no leading `/`
    // address prefix) nor a bundle (no `#bundle\0` prefix). rosc's
    // `decode_udp` returns a decode error, which `validate_packet`
    // wraps as `OscAdapterError::MalformedPacket`. `poll()` maps it
    // to [`OscPacketOutcome::ParseError`] per the slice-2 outcome
    // table in `adapter.rs`.
    let garbage: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];
    sender
        .send_to(&garbage, bound)
        .expect("send malformed bytes to adapter");

    let parse_errors = poll_until(&mut adapter, OscPacketOutcome::ParseError, 0);
    assert_eq!(
        parse_errors, 1,
        "poll() should record one ParseError outcome for a malformed packet",
    );
    assert_eq!(
        adapter.packet_outcome_count(OscPacketOutcome::Accepted),
        0,
        "malformed packet must not advance the Accepted counter",
    );
}

#[test]
fn packet_outcome_counters_start_zero_and_unbound_poll_is_noop() {
    // Pins the default initial state: every counter reads 0 on a
    // freshly-constructed adapter, and `poll()` on an unbound
    // adapter returns Ok without mutating state. This guards the
    // "additive observable" property: unrelated lifecycle
    // transitions must not spuriously advance any outcome counter.
    let mut adapter = UdpOscAdapter::new();
    for outcome in [
        OscPacketOutcome::Accepted,
        OscPacketOutcome::OversizedDropped,
        OscPacketOutcome::AllowlistRejected,
        OscPacketOutcome::RateLimitedDropped,
        OscPacketOutcome::ParseError,
    ] {
        assert_eq!(
            adapter.packet_outcome_count(outcome),
            0,
            "fresh adapter counter for {outcome:?} must be 0",
        );
    }

    adapter.poll().expect("unbound poll returns Ok");

    for outcome in [
        OscPacketOutcome::Accepted,
        OscPacketOutcome::OversizedDropped,
        OscPacketOutcome::AllowlistRejected,
        OscPacketOutcome::RateLimitedDropped,
        OscPacketOutcome::ParseError,
    ] {
        assert_eq!(
            adapter.packet_outcome_count(outcome),
            0,
            "unbound poll must not advance counter for {outcome:?}",
        );
    }
}
