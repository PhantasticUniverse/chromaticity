// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4a Wave 1a EV-INTEGRATION-MATRIX corpus (OSC slice). Tier
//! declaration and evidence tag live on the `fixtures` inner module
//! below, matching the pattern established by
//! `engine/runtime/tests/state_replay_corpus/main.rs` so
//! `scripts/evidence-tag-lint.sh`'s tier-distribution summary
//! counts this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-INTEGRATION-MATRIX (T2) — OSC routing-matrix
    //! conformance corpus for the Phase-4a integration gate
    //! (`docs/26-evidence-artifact-registry.md` row 37).
    //!
    //! AO-approved Wave 1a narrowing: this corpus covers the OSC
    //! routing-matrix axis only. MIDI / camera / Syphon /
    //! audio-uniform-IPC axes are deferred to Wave 1b. Within OSC,
    //! the corpus exercises (a) the route-namespace taxonomy
    //! (`/controls`, `/scenes`, `/audio`, `/render`) declared in
    //! `docs/30-mvp-support-matrix.md`, (b) the slice-2
    //! [`UdpOscAdapter`] payload-hardening surface
    //! ([`validate_packet`] size + bundle-depth caps), (c) the
    //! ADR-013 §3 per-route rate-limit precedence pin
    //! ([`resolve_rate_limit`]), and (d) the operational health
    //! state machine that R-05 fragility-mitigation introduced.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! the evidence-producing behaviour against a fixture corpus.
    //! Each fixture targets a distinct conformance contract from
    //! the `EV-INTEGRATION-MATRIX` row's "Generation Source"
    //! ("Adapter conformance matrix"); a regression fires the
    //! relevant fixture at the exact contract boundary so review
    //! routes the change to the right gate.
    //!
    //! Out-of-scope deliberately:
    //!
    //! - **Compat-syn-v1 alias dual-listing** (`/favslot/recall/3`
    //!   ↔ `/snaps/recall/3`). The slice-2 [`crate::routes::parse`]
    //!   accepts only the four canonical prefixes; compat-alias
    //!   expansion is slice-3+ scope per the routes module prose
    //!   and `docs/22-compat-policy.md` §Compat aliases. A T1 pin
    //!   for the alias landing belongs with that slice and is not
    //!   manufactured here. The corresponding wire-name retention
    //!   under `compat-syn-v1` is recorded in
    //!   `docs/30-mvp-support-matrix.md` line 77.
    //! - **Live `OscPacketOutcome::AllowlistRejected`** /
    //!   **`RateLimitedDropped`** counter advancement. Those
    //!   counters are pinned at zero in slice-2 (allowlist /
    //!   rate-limit enforcement is slice-3+ scope per
    //!   `engine/observability/src/catalog.rs` and the slice-2
    //!   adapter prose); the network-hardening corpus
    //!   (`tests/network_hardening_corpus`) covers the structural
    //!   counter surface and the ADR-013 §2 / §3 default posture.

    use std::net::{SocketAddr, UdpSocket};

    use chromaticity_observability::catalog::{OscAdapterState, OscPacketOutcome};
    use chromaticity_osc_adapter::adapter::{
        OscAdapter, OscAdapterError, OscHealthState, UdpOscAdapter, default_inbound_addr,
        resolve_rate_limit, validate_packet,
    };
    use chromaticity_osc_adapter::defaults::{
        DEFAULT_INBOUND_BIND, DEFAULT_MAX_BUNDLE_DEPTH, DEFAULT_MAX_PACKET_BYTES,
    };
    use chromaticity_osc_adapter::routes::{RoutePrefix, parse};

    /// Skip-on-CI sentinel: returns `Some(adapter, addr)` when an
    /// ephemeral loopback bind succeeds, or `None` (with a stderr
    /// notice) when the runtime restricts UDP socket access. The
    /// fixture treats the latter as an environmental skip rather
    /// than a fixture failure, matching the Wave 1a brief.
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
                    "integration_matrix_corpus: skipping fixture; UDP loopback bind unavailable: {io_err}"
                );
                None
            }
            Err(other) => {
                panic!("unexpected bind failure shape: {other:?}");
            }
        }
    }

    // -----------------------------------------------------------
    // Route-namespace conformance (docs/30 §OSC route prefixes)
    // -----------------------------------------------------------

    #[test]
    fn osc_route_prefix_controls_parses_canonical_path() {
        let addr = parse("/controls/scene-a/foo").expect("canonical /controls path parses");
        assert_eq!(addr.prefix, RoutePrefix::Controls);
        assert_eq!(
            addr.segments,
            vec!["scene-a".to_string(), "foo".to_string()]
        );
    }

    #[test]
    fn osc_route_prefix_scenes_parses_canonical_path() {
        let addr = parse("/scenes/select/3").expect("canonical /scenes path parses");
        assert_eq!(addr.prefix, RoutePrefix::Scenes);
        assert_eq!(addr.segments, vec!["select".to_string(), "3".to_string()]);
    }

    #[test]
    fn osc_route_prefix_audio_parses_canonical_path() {
        let addr = parse("/audio/level/master").expect("canonical /audio path parses");
        assert_eq!(addr.prefix, RoutePrefix::Audio);
        assert_eq!(
            addr.segments,
            vec!["level".to_string(), "master".to_string()]
        );
    }

    #[test]
    fn osc_route_prefix_render_parses_canonical_path() {
        let addr = parse("/render/profile/standard").expect("canonical /render path parses");
        assert_eq!(addr.prefix, RoutePrefix::Render);
        assert_eq!(
            addr.segments,
            vec!["profile".to_string(), "standard".to_string()]
        );
    }

    #[test]
    fn osc_route_unknown_prefix_rejects() {
        // ADR-013 §5 strict-parse posture: an unrecognised top-level
        // prefix must NOT yield a partial `RouteAddr`. Slice-3
        // dispatch will rely on the post-parse shape being one of
        // the four canonical prefixes; widening the accept set is a
        // deliberate slice-3+ decision.
        assert!(parse("/unknown/whatever").is_none());
        assert!(parse("/midi/note/on").is_none());
        assert!(parse("/snaps/recall/3").is_none()); // canonical-Snap not yet in parser
    }

    #[test]
    fn osc_route_path_traversal_rejects() {
        // ADR-013 §5 hardening: path-traversal-style segments must
        // not yield a parsed [`RouteAddr`]. The slice-2 parser
        // either rejects the whole address (unknown prefix `..`) or
        // collapses empty segments; in neither case should
        // `/controls/../../etc/passwd` produce a `RouteAddr` whose
        // segments include `..` as a special-cased identity. We
        // assert the parser produces no traversal escape: any
        // `RouteAddr` returned must keep `..` as an opaque segment
        // (slice-3 dispatch then rejects unknown segments at the
        // route-handler boundary).
        let parsed = parse("/controls/../../etc/passwd");
        match parsed {
            Some(addr) => {
                // Traversal segments are NOT collapsed into a
                // shorter path — they remain as opaque strings the
                // slice-3 dispatch will reject.
                assert_eq!(addr.prefix, RoutePrefix::Controls);
                assert!(
                    addr.segments.iter().any(|s| s == ".."),
                    "traversal segments must not be silently absorbed",
                );
            }
            None => {
                // Equally acceptable: parser rejects outright.
            }
        }
        // A path that puts the traversal at the prefix position is
        // an unknown prefix and must reject.
        assert!(parse("/../controls/foo").is_none());
    }

    #[test]
    fn osc_route_empty_segment_rejects_or_collapses() {
        // ADR-013 §5 strict-parse posture: empty segments caused by
        // doubled slashes are stripped (per the slice-2 routes prose
        // "OSC wire conventions tolerate trailing slashes on some
        // clients"). The post-parse `segments` vector must NOT
        // contain empty strings — a slice-3 route handler keying on
        // an empty segment would be undefined.
        let parsed = parse("/controls//empty").expect("doubled-slash collapses, not rejects");
        assert_eq!(parsed.prefix, RoutePrefix::Controls);
        assert!(
            !parsed.segments.iter().any(String::is_empty),
            "empty segments must be stripped from the canonical segment vector",
        );
        assert_eq!(parsed.segments, vec!["empty".to_string()]);
    }

    // -----------------------------------------------------------
    // UDP adapter conformance (ADR-013 §1 / §5 / R-05)
    // -----------------------------------------------------------

    #[test]
    fn udp_adapter_binds_loopback_only_by_default() {
        // ADR-013 §1 "Bind defaults per adapter": the OSC inbound
        // listener defaults to loopback. The matrix conformance
        // pin asserts that the [`default_inbound_addr`] helper
        // produces a 127.0.0.1 address and that a
        // [`UdpOscAdapter::bind`] call against it produces a
        // [`SocketAddr`] whose IP is the loopback default — never
        // 0.0.0.0.
        let default_addr = default_inbound_addr(0);
        assert_eq!(
            default_addr.ip(),
            DEFAULT_INBOUND_BIND,
            "default inbound addr must inherit the loopback default",
        );
        assert!(
            default_addr.ip().is_loopback(),
            "default inbound addr must be loopback (ADR-013 §1)",
        );

        let Some((adapter, local)) = try_bind_loopback() else {
            return;
        };
        assert!(
            local.ip().is_loopback(),
            "bound adapter must surface a loopback local address; saw {local:?}",
        );
        assert_ne!(
            local.ip().to_string(),
            "0.0.0.0",
            "default bind must NOT widen to 0.0.0.0",
        );
        drop(adapter);
    }

    #[test]
    fn udp_adapter_validate_packet_rejects_oversized() {
        // ADR-013 §5 bullet 1: the 64 KiB cap fires before any
        // decoder allocation. A buffer one byte above the cap must
        // surface [`OscAdapterError::OversizedPacket`].
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
    fn udp_adapter_validate_packet_rejects_deep_bundle() {
        // ADR-013 §5 bullet 2: bundle nesting beyond the depth cap
        // (default 4) must surface [`OscAdapterError::BundleDepthExceeded`].
        // We synthesize a depth-5 nested bundle on the wire by
        // hand: rosc's encoder isn't a dev-dep here, but the
        // OSC 1.0 bundle wire format is stable and small.
        let packet = build_nested_bundle(DEFAULT_MAX_BUNDLE_DEPTH + 1);
        match validate_packet(&packet) {
            Err(OscAdapterError::BundleDepthExceeded { actual, max }) => {
                assert!(
                    actual > DEFAULT_MAX_BUNDLE_DEPTH,
                    "synthesized depth {actual} must exceed cap {max}",
                );
                assert_eq!(max, DEFAULT_MAX_BUNDLE_DEPTH);
            }
            other => panic!("expected BundleDepthExceeded, got {other:?}"),
        }
    }

    #[test]
    fn udp_adapter_resolve_rate_limit_per_route_takes_precedence() {
        // ADR-013 §3: the per-route ceiling wins over the
        // per-adapter floor when both are configured. This pin is
        // mirrored from the in-source `per_route_rate_limit_precedes_adapter_floor`
        // T1 drift pin and re-asserted here at T2 because the
        // matrix gate keys off the same precedence rule for slice-3
        // route-handler configuration.
        assert_eq!(resolve_rate_limit(100, 200), 100);
        assert_eq!(resolve_rate_limit(50, 100), 50);
        assert_eq!(resolve_rate_limit(500, 100), 500);
        // Symmetric edge: when per_route equals adapter_floor, the
        // per-route value still wins.
        assert_eq!(resolve_rate_limit(100, 100), 100);
    }

    #[test]
    fn udp_adapter_state_machine_unbound_to_bound_to_unbound() {
        // ADR-005 / ADR-013 lifecycle pin: a bind / unbind round
        // trip lands the catalog state-transition counters on the
        // canonical Unbound -> Binding -> Bound -> Unbinding ->
        // Unbound path with each edge taken exactly once. Slice-3
        // routing dispatch consumes this guarantee when wiring the
        // hub-side observer.
        let Some((mut adapter, _local)) = try_bind_loopback() else {
            return;
        };
        adapter.unbind().expect("unbind from bound");
        for (from, to, expected) in [
            (OscAdapterState::Unbound, OscAdapterState::Binding, 1),
            (OscAdapterState::Binding, OscAdapterState::Bound, 1),
            (OscAdapterState::Bound, OscAdapterState::Unbinding, 1),
            (OscAdapterState::Unbinding, OscAdapterState::Unbound, 1),
        ] {
            assert_eq!(
                adapter.state_transition_count(from, to),
                expected,
                "expected one {from:?} -> {to:?} transition for a bind/unbind round-trip",
            );
        }
    }

    #[test]
    fn udp_adapter_packet_outcome_counters_are_zero_on_fresh_adapter() {
        // Conformance baseline: a freshly-constructed adapter has
        // every [`OscPacketOutcome`] counter at 0. This is the
        // pre-condition slice-3's runtime-bus counter rebase
        // assumes when it sources the initial values.
        let adapter = UdpOscAdapter::new();
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
    }

    #[test]
    fn udp_adapter_health_state_starts_healthy_and_stays_healthy_on_first_shot_bind() {
        // R-05 mitigation conformance: a first-shot bind must NOT
        // flip health to Recovering. Recovering is reserved for
        // the post-retry posture per the
        // `state_machine_tracks_recovery_after_bind_failure` T1 pin
        // in `tests/adapter_bind_path.rs`.
        let Some((adapter, _local)) = try_bind_loopback() else {
            return;
        };
        assert_eq!(
            adapter.health_state(),
            OscHealthState::Healthy,
            "first-shot bind must surface Healthy, not Recovering",
        );
        assert_eq!(
            adapter.attempted_binds(),
            1,
            "first-shot bind records exactly one attempt",
        );
    }

    #[test]
    fn udp_adapter_round_trip_accepts_well_formed_message() {
        // End-to-end matrix pin: a well-formed OSC 1.0 message
        // sent from a peer socket lands on the adapter's
        // [`OscPacketOutcome::Accepted`] counter via the slice-2
        // poll path. Mirrors the live-socket coverage in
        // `tests/adapter_poll_path.rs` but additionally asserts
        // (a) loopback-only bind side-effect, (b) that the
        // ParseError counter is NOT spuriously advanced for a
        // valid packet.
        let Some((mut adapter, bound)) = try_bind_loopback() else {
            return;
        };
        let Ok(sender) = UdpSocket::bind("127.0.0.1:0") else {
            eprintln!("integration_matrix_corpus: skipping round-trip; sender bind unavailable");
            return;
        };
        let valid_osc: [u8; 8] = [b'/', b't', 0, 0, b',', 0, 0, 0];
        sender
            .send_to(&valid_osc, bound)
            .expect("send well-formed osc to adapter");

        // Spin-poll up to one second for the loopback datagram.
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
            "well-formed packet must advance Accepted exactly once",
        );
        assert_eq!(
            adapter.packet_outcome_count(OscPacketOutcome::ParseError),
            0,
            "well-formed packet must not advance ParseError",
        );
    }

    // -----------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------

    /// Synthesize an OSC bundle nested `depth` levels deep on the
    /// OSC 1.0 wire format. Each layer wraps the inner payload in
    /// a `#bundle\0` header + 8-byte timetag + 4-byte size prefix.
    /// At depth 0 returns a minimal well-formed OSC message.
    fn build_nested_bundle(depth: u8) -> Vec<u8> {
        let mut payload: Vec<u8> = vec![b'/', b't', 0, 0, b',', 0, 0, 0];
        for _ in 0..depth {
            let mut layer: Vec<u8> = Vec::with_capacity(payload.len() + 24);
            // OSC bundle header: "#bundle\0" (8 bytes) + 8-byte
            // timetag (immediate = 0x00..0x01).
            layer.extend_from_slice(b"#bundle\0");
            layer.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1]);
            // 4-byte big-endian size prefix for the inner element.
            let inner_len = u32::try_from(payload.len()).expect("payload fits in u32");
            layer.extend_from_slice(&inner_len.to_be_bytes());
            layer.extend_from_slice(&payload);
            payload = layer;
        }
        payload
    }
}
