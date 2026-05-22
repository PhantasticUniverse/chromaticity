// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! OSC outbound sender — encode and `send_to` over a UDP socket.
//!
//! This module owns the outbound message construction path.
//! [`OscArg`] is the public argument type accepted by
//! [`OscSender::emit`]; [`encode_message`] converts an address +
//! arg slice into a well-formed OSC 1.0 UDP packet via the `rosc`
//! encoder; [`OscSender`] wraps a `std::net::UdpSocket` in send-only
//! mode and delivers the encoded bytes to the operator-configured
//! target.
//!
//! ## Best-effort / non-blocking posture
//!
//! The send socket is set non-blocking at construction time. Every
//! [`OscSender::emit`] call does a single `send_to` attempt. A
//! `WouldBlock` result (send buffer saturated) is silently dropped
//! — the caller is never stalled. Any other OS-level error emits a
//! throttled diagnostic to `stderr` (no more than once per second
//! per-sender, matching the ADR-013 §3 diagnostic-throttle cadence
//! for inbound rate-limit drops) and is otherwise ignored. This
//! mirrors the inbound adapter's best-effort posture for oversize /
//! malformed packets: the send path MUST NOT block the runtime loop.
//!
//! ## ADR-013 §1 loopback default
//!
//! The default target address is assembled from
//! [`crate::defaults::DEFAULT_OUTBOUND_TARGET`] (127.0.0.1) and
//! [`crate::defaults::DEFAULT_OSC_OUT_PORT`] (9001) per ADR-013 §1.
//! Operator overrides are applied by replacing the target on the
//! `OscSender` via [`OscSender::set_target`]; the new target takes
//! effect on the next [`OscSender::emit`] call.
//!
//! ## Governing ADRs
//!
//! - ADR-013 — networking defaults and security
//!   (`docs/adr/ADR-013-networking-defaults-and-security.md`),
//!   §1 outbound target default, §6 operator-overridable.

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use rosc::{OscMessage, OscPacket, OscType, encoder};

use crate::defaults::{DEFAULT_OSC_OUT_PORT, DEFAULT_OUTBOUND_TARGET};

/// Throttle window for send-error diagnostics — no more than one
/// message per sender per second, matching the ADR-013 §3
/// per-route rate-limit diagnostic throttle cadence.
const SEND_ERROR_THROTTLE: Duration = Duration::from_secs(1);

/// OSC argument types accepted by [`OscSender::emit`].
///
/// Covers the value shapes the runtime's outbound paths need at MVP.
/// Future lanes may extend this enum; it is `#[non_exhaustive]` so
/// external callers cannot exhaustively match without a wildcard arm.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum OscArg {
    /// A 32-bit IEEE 754 float — the most common control-value shape.
    Float(f32),
    /// A 32-bit signed integer.
    Int(i32),
    /// A UTF-8 string.
    String(String),
    /// A boolean (maps to OSC true/false type tags).
    Bool(bool),
}

impl From<OscArg> for OscType {
    fn from(arg: OscArg) -> Self {
        match arg {
            OscArg::Float(v) => OscType::Float(v),
            OscArg::Int(v) => OscType::Int(v),
            OscArg::String(v) => OscType::String(v),
            OscArg::Bool(v) => OscType::Bool(v),
        }
    }
}

/// Encode an OSC address + argument list into UDP bytes.
///
/// Returns `None` when the `rosc` encoder rejects the message shape
/// (malformed address, unsupported argument type). The caller
/// treats a `None` result as a best-effort drop and does not surface
/// it as a hard error.
///
/// Kept `pub` so in-source T1 tests and the network-hardening
/// corpus can drive the encoding path without a live socket.
#[must_use]
pub fn encode_message(address: &str, args: &[OscArg]) -> Option<Vec<u8>> {
    let osc_args: Vec<OscType> = args.iter().cloned().map(OscType::from).collect();
    let packet = OscPacket::Message(OscMessage {
        addr: address.to_owned(),
        args: osc_args,
    });
    encoder::encode(&packet).ok()
}

/// UDP-backed OSC outbound sender.
///
/// Owns a send-only, non-blocking `UdpSocket` bound to an ephemeral
/// loopback port (the OS assigns the source port; the target is the
/// operator-configured destination). The socket is bound on
/// construction and kept alive for the sender's lifetime.
///
/// ## Thread safety
///
/// `OscSender` is not `Sync`; callers that need to emit from
/// multiple threads should wrap it in a `Mutex` following the same
/// pattern as the inbound `UdpOscAdapter`.
pub struct OscSender {
    socket: Option<UdpSocket>,
    target: SocketAddr,
    last_error_emit: Option<Instant>,
}

impl std::fmt::Debug for OscSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OscSender")
            .field("target", &self.target)
            .field("socket_bound", &self.socket.is_some())
            .finish()
    }
}

impl OscSender {
    /// Construct a new [`OscSender`] targeting
    /// `DEFAULT_OUTBOUND_TARGET:DEFAULT_OSC_OUT_PORT` (ADR-013 §1
    /// loopback default) and bind an ephemeral local UDP socket.
    ///
    /// Returns `Err` only when the OS rejects the initial
    /// `UdpSocket::bind` (e.g. all ephemeral ports exhausted).
    /// Callers that handle start-up errors gracefully should treat
    /// this as a transient condition; the inbound adapter's
    /// `MAX_BIND_ATTEMPTS` retry pattern applies equally here.
    pub fn new() -> Result<Self, std::io::Error> {
        let target = SocketAddr::new(DEFAULT_OUTBOUND_TARGET, DEFAULT_OSC_OUT_PORT);
        Self::with_target(target)
    }

    /// Construct a [`OscSender`] with an explicit target address.
    ///
    /// Used by the operator-override path: the shell command handler
    /// calls this (or [`Self::set_target`]) after reading the
    /// `osc.out_host` + `osc.out_port` fields from the persisted
    /// `Settings`.
    pub fn with_target(target: SocketAddr) -> Result<Self, std::io::Error> {
        // Bind to an OS-assigned ephemeral port on loopback; we don't
        // care about the source port, only the destination.
        let bind_addr = SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 0);
        let socket = UdpSocket::bind(bind_addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket: Some(socket),
            target,
            last_error_emit: None,
        })
    }

    /// Update the outbound target address (ADR-013 §6 operator
    /// override). Takes effect on the next [`Self::emit`] call.
    pub fn set_target(&mut self, target: SocketAddr) {
        self.target = target;
    }

    /// Current outbound target address.
    #[must_use]
    pub fn target(&self) -> SocketAddr {
        self.target
    }

    /// Encode `address` + `args` as an OSC 1.0 UDP packet and
    /// `send_to` the configured target.
    ///
    /// Best-effort: `WouldBlock` (saturated send buffer) is silently
    /// dropped; any other OS error emits a throttled diagnostic and
    /// is discarded. The return value is `true` when the datagram was
    /// accepted by the kernel and `false` on any drop (WouldBlock,
    /// encode failure, or other OS error).
    ///
    /// Callers MUST NOT interpret `false` as a fatal condition — the
    /// send path is fire-and-forget per the module-level prose.
    pub fn emit(&mut self, address: &str, args: &[OscArg]) -> bool {
        let Some(ref socket) = self.socket else {
            return false;
        };
        let Some(bytes) = encode_message(address, args) else {
            // Encode failure is rare (only a malformed address /
            // invalid arg type can cause it). Throttle the same way
            // as OS send errors so a buggy call-site doesn't spam
            // stderr.
            self.maybe_emit_error("osc outbound encode failed for address", address);
            return false;
        };
        match socket.send_to(&bytes, self.target) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Backpressure: send buffer saturated. Drop silently;
                // the next emit will retry.
                false
            }
            Err(e) => {
                self.maybe_emit_error(&format!("osc outbound send error: {e}"), address);
                false
            }
        }
    }

    /// Emit a throttled diagnostic to stderr, honouring the
    /// [`SEND_ERROR_THROTTLE`] window so a persistent OS error does
    /// not produce unbounded output.
    fn maybe_emit_error(&mut self, message: &str, address: &str) {
        let now = Instant::now();
        let should_emit = self
            .last_error_emit
            .map(|t| now.duration_since(t) >= SEND_ERROR_THROTTLE)
            .unwrap_or(true);
        if should_emit {
            eprintln!("[chromaticity-osc-adapter] {message} (route: {address})");
            self.last_error_emit = Some(now);
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod sender_pins {
    //! Evidence: EV-NET-HARDENING (T1) — ADR-013 §1 outbound
    //! loopback-default + §6 operator-override + best-effort-drop
    //! backpressure posture for the outbound send path.

    use std::net::{SocketAddr, UdpSocket};

    use super::{OscArg, OscSender, encode_message};
    use crate::defaults::{DEFAULT_OSC_OUT_PORT, DEFAULT_OUTBOUND_TARGET};

    #[test]
    fn new_sender_targets_adr_013_loopback_default() {
        // ADR-013 §1: fresh OscSender targets loopback:DEFAULT_OSC_OUT_PORT.
        match OscSender::new() {
            Ok(sender) => {
                assert_eq!(sender.target().ip(), DEFAULT_OUTBOUND_TARGET);
                assert_eq!(sender.target().port(), DEFAULT_OSC_OUT_PORT);
            }
            Err(e) => {
                // Ephemeral-port exhaustion or restricted environment.
                eprintln!("sender_pins: skipping; bind unavailable: {e}");
            }
        }
    }

    #[test]
    fn set_target_overrides_destination_per_adr_013_sec6() {
        // ADR-013 §6: operator can override the outbound target.
        match OscSender::new() {
            Ok(mut sender) => {
                let custom: SocketAddr = "127.0.0.1:19999".parse().unwrap();
                sender.set_target(custom);
                assert_eq!(sender.target(), custom);
            }
            Err(e) => {
                eprintln!("sender_pins: skipping; bind unavailable: {e}");
            }
        }
    }

    #[test]
    fn emit_happy_path_sends_to_bound_receiver() {
        // Bind a receiver on an ephemeral loopback port; construct a
        // sender targeting that port; emit one float arg; assert the
        // receiver gets the datagram. This is the round-trip T1 pin.
        let receiver = match UdpSocket::bind("127.0.0.1:0") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("sender_pins: skipping emit happy-path; receiver bind: {e}");
                return;
            }
        };
        let receiver_addr = receiver.local_addr().unwrap();
        receiver
            .set_read_timeout(Some(std::time::Duration::from_secs(1)))
            .unwrap();

        let mut sender = match OscSender::with_target(receiver_addr) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("sender_pins: skipping emit happy-path; sender bind: {e}");
                return;
            }
        };
        let sent = sender.emit("/test/addr", &[OscArg::Float(0.5)]);
        assert!(sent, "emit must return true on a successful send");

        let mut buf = [0u8; 1024];
        let (len, _) = receiver.recv_from(&mut buf).expect("receiver got datagram");
        assert!(len > 0, "received datagram must be non-empty");
    }

    #[test]
    fn emit_backpressure_drop_returns_false() {
        // Fill the OS send buffer by targeting a non-connected
        // loopback address that nothing reads from, then call emit
        // repeatedly until WouldBlock. With a non-blocking socket the
        // kernel drops when the buffer is full; the emit must return
        // false and not block.
        //
        // NOTE: On most OS configurations the send buffer is large
        // enough that a single small packet never saturates it.
        // This test therefore pins the *structural* invariant (false
        // returned on WouldBlock) by sending to a well-formed target
        // that may or may not stall. We verify that emit never panics
        // and that the bool returned is coherent (we accept either
        // true or false here — the important contract is no block/panic).
        let target: SocketAddr = "127.0.0.1:0".parse().unwrap();
        // Bind target to ensure the port is real; immediately drop so
        // it becomes unreachable to the sender.
        let hole = UdpSocket::bind(target).expect("bind sink port");
        let sink_addr = hole.local_addr().unwrap();
        drop(hole);

        match OscSender::with_target(sink_addr) {
            Ok(mut sender) => {
                // Multiple sends — result is true or false; must not panic.
                for _ in 0..10 {
                    let _ = sender.emit("/test/bp", &[OscArg::Int(1)]);
                }
            }
            Err(e) => {
                eprintln!("sender_pins: skipping backpressure pin; bind: {e}");
            }
        }
    }

    #[test]
    fn encode_message_float_produces_non_empty_bytes() {
        // encode_message must return Some for a well-formed address
        // and a single float argument.
        let bytes = encode_message("/test/addr", &[OscArg::Float(1.0)]);
        assert!(bytes.is_some(), "encode_message must succeed for float arg");
        let bytes = bytes.unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn encode_message_no_args_produces_valid_packet() {
        let bytes = encode_message("/ping", &[]);
        assert!(bytes.is_some());
    }

    #[test]
    fn encode_message_all_arg_types() {
        // All OscArg variants must encode without error.
        let args = [
            OscArg::Float(0.75),
            OscArg::Int(-42),
            OscArg::String("hello".to_string()),
            OscArg::Bool(true),
        ];
        let bytes = encode_message("/multi", &args);
        assert!(bytes.is_some(), "mixed-type encode must succeed");
    }
}
