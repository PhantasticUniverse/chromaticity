// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4a Wave 1a EV-NET-HARDENING corpus. Tier declaration and
//! evidence tag live on the `fixtures` inner module below, matching
//! the pattern established by
//! `engine/runtime/tests/state_replay_corpus/main.rs` so
//! `scripts/evidence-tag-lint.sh`'s tier-distribution summary
//! counts this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-NET-HARDENING (T2) — ADR-013 networking-defaults
    //! conformance corpus for the Phase-4a network-hardening gate
    //! (`docs/26-evidence-artifact-registry.md` row 81).
    //!
    //! AO-approved Wave 1a scope: full ADR-013 surface for the OSC
    //! adapter — bind defaults (§1), accept-from-bound-interface
    //! allowlist boundary (§2), per-route rate-limit precedence
    //! (§3), and payload-hardening caps (§5). The slice-2 in-source
    //! drift pins under `integrations/osc-adapter/src/defaults.rs`
    //! and `integrations/osc-adapter/src/adapter.rs` already cover
    //! the constants at T1; this T2 corpus exercises the same
    //! contracts against the live [`UdpOscAdapter`] surface and
    //! the per-outcome counter table the matrix gate keys off.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! the evidence-producing behaviour against a fixture corpus.
    //! Each fixture targets a distinct ADR-013 clause; a regression
    //! fires the relevant fixture at the exact clause boundary so
    //! review routes the change to the network-hardening gate.
    //!
    //! Out-of-scope deliberately:
    //!
    //! - **Live `OscPacketOutcome::AllowlistRejected`** advancement
    //!   via a non-loopback source IP. Slice-2 ships the empty
    //!   allowlist (ADR-013 §2 default) and pins the structural
    //!   counter surface; the live source-IP `recv_from` filter
    //!   that would advance the counter against a non-matching
    //!   source IP is slice-3+ scope per the slice-2 adapter
    //!   prose. The fixture below asserts the counter stays at 0
    //!   under the empty-allowlist default; a sibling boundary
    //!   pin asserts the structural counter shape stays
    //!   monotonic-non-negative.
    //! - **Live `OscPacketOutcome::RateLimitedDropped`** counter
    //!   advancement. Same slice-3+ deferral rationale; the §3
    //!   precedence pin below covers the resolver function which
    //!   is the matrix gate's hand-off point.

    use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

    use chromaticity_observability::catalog::OscPacketOutcome;
    use chromaticity_osc_adapter::adapter::{
        OscAdapter, OscAdapterError, UdpOscAdapter, default_inbound_addr, resolve_rate_limit,
        validate_packet,
    };
    use chromaticity_osc_adapter::defaults::{
        DEFAULT_ADAPTER_RATE_LIMIT_FLOOR_MSGS_PER_SEC, DEFAULT_INBOUND_BIND,
        DEFAULT_MAX_BUNDLE_DEPTH, DEFAULT_MAX_PACKET_BYTES, DEFAULT_OUTBOUND_TARGET,
        DEFAULT_RATE_LIMIT_MSGS_PER_SEC,
    };

    /// Skip-on-CI sentinel — matches the integration-matrix corpus
    /// shape. Returns `Some` when a UDP loopback bind succeeds and
    /// `None` (with stderr notice) when the runtime restricts raw
    /// socket access.
    fn try_bind_loopback() -> Option<(UdpOscAdapter, SocketAddr)> {
        let mut adapter = UdpOscAdapter::new();
        let bind_addr: SocketAddr = "127.0.0.1:0"
            .parse()
            .expect("loopback:0 parses as SocketAddr");
        match adapter.bind(bind_addr) {
            Ok(()) => {
                let local = adapter
                    .local_addr()
                    .expect("bound adapter exposes its local_addr");
                Some((adapter, local))
            }
            Err(OscAdapterError::BindFailed(io_err)) => {
                eprintln!(
                    "network_hardening_corpus: skipping fixture; UDP loopback bind unavailable: {io_err}"
                );
                None
            }
            Err(other) => panic!("unexpected bind failure shape: {other:?}"),
        }
    }

    // -----------------------------------------------------------
    // ADR-013 §1 — bind defaults
    // -----------------------------------------------------------

    #[test]
    fn default_inbound_bind_is_loopback() {
        // ADR-013 §1: OSC inbound default bind is loopback. The T1
        // drift pin in `defaults.rs` covers the constant; this T2
        // fixture re-asserts the same invariant against the
        // operator-facing helper [`default_inbound_addr`] that
        // adapters actually call.
        assert_eq!(
            DEFAULT_INBOUND_BIND,
            std::net::IpAddr::V4(Ipv4Addr::LOCALHOST)
        );
        let helper_addr = default_inbound_addr(0);
        assert!(
            helper_addr.ip().is_loopback(),
            "default_inbound_addr() must return a loopback address",
        );
        assert_eq!(helper_addr.port(), 0, "ephemeral-port marker preserved");
    }

    #[test]
    fn default_outbound_target_is_loopback() {
        // ADR-013 §1: OSC outbound default target is loopback
        // until operator configures otherwise. The matrix gate
        // pins both directions of the default posture so a
        // refactor that flips outbound-default to non-loopback
        // surfaces here, not at first-customer-impact.
        assert_eq!(
            DEFAULT_OUTBOUND_TARGET,
            std::net::IpAddr::V4(Ipv4Addr::LOCALHOST)
        );
    }

    // -----------------------------------------------------------
    // ADR-013 §5 — payload-hardening caps
    // -----------------------------------------------------------

    #[test]
    fn default_packet_cap_is_64_kib() {
        // ADR-013 §5 bullet 1: 64 KiB packet cap. T2 re-pin so a
        // refactor that tightens or loosens the cap surfaces
        // against the matrix gate as well as the in-source T1 pin.
        assert_eq!(DEFAULT_MAX_PACKET_BYTES, 65_536);
    }

    #[test]
    fn default_bundle_depth_is_four() {
        // ADR-013 §5 bullet 2: bundle-nesting cap default 4.
        assert_eq!(DEFAULT_MAX_BUNDLE_DEPTH, 4);
    }

    #[test]
    fn oversized_packet_outcome_recorded_in_counter() {
        // ADR-013 §5 bullet 1 conformance: an oversized packet on
        // the wire MUST surface as
        // [`OscPacketOutcome::OversizedDropped`] when routed
        // through `validate_packet`. This is the
        // adapter-internal observable the slice-3 runtime bus
        // will rebase onto `MetricEvent::OscInboundPackets`. We
        // can't send a >64 KiB UDP datagram on loopback (UDP/IPv4
        // tops out at 65,507 payload bytes), so the test invokes
        // [`validate_packet`] directly with an oversized buffer
        // and asserts the error variant is the size-gated one
        // (NOT the rosc-decoder MalformedPacket).
        let oversized = vec![0u8; DEFAULT_MAX_PACKET_BYTES + 1];
        match validate_packet(&oversized) {
            Err(OscAdapterError::OversizedPacket { actual, max }) => {
                assert_eq!(actual, DEFAULT_MAX_PACKET_BYTES + 1);
                assert_eq!(max, DEFAULT_MAX_PACKET_BYTES);
            }
            other => panic!("expected OversizedPacket, got {other:?}"),
        }
    }

    // -----------------------------------------------------------
    // ADR-013 §3 — rate-limit precedence
    // -----------------------------------------------------------

    #[test]
    fn default_rate_limit_is_100_msg_per_sec() {
        // ADR-013 §3 bullets 1+2: per-route default 100 msg/s and
        // adapter floor default 100 msg/s.
        assert_eq!(DEFAULT_RATE_LIMIT_MSGS_PER_SEC, 100);
        assert_eq!(DEFAULT_ADAPTER_RATE_LIMIT_FLOOR_MSGS_PER_SEC, 100);
    }

    #[test]
    fn rate_limit_resolution_picks_per_route_over_adapter_floor() {
        // ADR-013 §3 precedence pin: the per-route ceiling wins
        // over the per-adapter floor when both are configured.
        // Mirrors the in-source `per_route_rate_limit_precedes_adapter_floor`
        // T1 drift pin and re-asserts at T2 since the matrix gate
        // keys off the same precedence rule for slice-3
        // route-handler configuration. The "most restrictive"
        // intuition only applies when per-route is the smaller
        // value (1000 adapter-floor vs 100 per-route -> 100
        // wins). The brief scenario:
        assert_eq!(
            resolve_rate_limit(100, 1000),
            100,
            "per-route 100 msg/s wins over adapter-floor 1000 msg/s",
        );
        // Symmetric edge: per-route can be larger than the floor
        // (slice-3 caps against the §3 hard 1000 msg/s ceiling
        // independently; the resolver itself returns per-route
        // verbatim).
        assert_eq!(resolve_rate_limit(500, 100), 500);
        // Equal values converge.
        assert_eq!(resolve_rate_limit(100, 100), 100);
    }

    // -----------------------------------------------------------
    // ADR-013 §2 — allowlist boundary
    // -----------------------------------------------------------

    #[test]
    fn allowlist_default_is_empty() {
        // ADR-013 §2 default posture: a fresh adapter ships with
        // an empty allowlist (which slice-3 will interpret as
        // accept-from-bound-interface-only per the catalog's
        // `OscPacketOutcome::AllowlistRejected` slice-3 wiring).
        let adapter = UdpOscAdapter::new();
        assert!(
            adapter.allowlist_is_empty(),
            "default allowlist must be empty"
        );
        assert_eq!(adapter.allowlist_len(), 0);
    }

    #[test]
    fn allowlist_default_accepts_loopback_round_trip() {
        // With the empty allowlist (ADR-013 §2 default), a
        // loopback packet round-trip lands on `Accepted`, not
        // `AllowlistRejected`. Slice-2 does not yet enforce a
        // source-IP filter — the empty-allowlist default-accept
        // posture is the structural pin; a non-empty allowlist
        // rejecting a non-matching source IP is slice-3+ scope.
        let Some((mut adapter, bound)) = try_bind_loopback() else {
            return;
        };
        let Ok(sender) = UdpSocket::bind("127.0.0.1:0") else {
            eprintln!("network_hardening_corpus: skipping round-trip; sender bind unavailable");
            return;
        };
        let valid_osc: [u8; 8] = [b'/', b't', 0, 0, b',', 0, 0, 0];
        sender
            .send_to(&valid_osc, bound)
            .expect("send well-formed osc from loopback peer");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while std::time::Instant::now() < deadline
            && adapter.packet_outcome_count(OscPacketOutcome::Accepted) == 0
        {
            adapter.poll().expect("poll on bound adapter succeeds");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        assert_eq!(
            adapter.packet_outcome_count(OscPacketOutcome::Accepted),
            1,
            "loopback round-trip with empty allowlist must Accept",
        );
        assert_eq!(
            adapter.packet_outcome_count(OscPacketOutcome::AllowlistRejected),
            0,
            "empty-allowlist default-accept must NOT advance AllowlistRejected",
        );
    }

    #[test]
    fn allowlist_rejection_counter_is_structurally_addressable() {
        // Structural pin for the slice-3+ allowlist-enforcement
        // wiring: the counter shape MUST already address the
        // `AllowlistRejected` outcome at zero on a fresh adapter,
        // so the slice-3 enforcement wiring inherits a
        // monotonic-non-negative starting point. A regression
        // that loses the counter slot (or aliases it onto a
        // different outcome) fires here at the structural
        // boundary, not at first-customer-impact.
        let adapter = UdpOscAdapter::new();
        assert_eq!(
            adapter.packet_outcome_count(OscPacketOutcome::AllowlistRejected),
            0,
        );
        assert_eq!(
            adapter.packet_outcome_count(OscPacketOutcome::RateLimitedDropped),
            0,
        );
    }
}
