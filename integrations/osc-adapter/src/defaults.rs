// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-NET-HARDENING — bind-scope and packet-hardening
//! default-value baseline.
//!
//! ADR-013 (networking defaults and security) fixes the values in
//! this module. Any change here is a conformance break and must be
//! reviewed against the `EV-NET-HARDENING` Phase-4a gate
//! (`docs/26-evidence-artifact-registry.md` line 77) before merge.
//!
//! Operator-facing overrides do not live here: per ADR-013 §6 each
//! default is overridable in a Settings panel at runtime; this
//! module pins the out-of-the-box posture the runtime starts from.

use core::net::{IpAddr, Ipv4Addr};

/// Default inbound OSC bind address.
///
/// Loopback-only. Exposing the OSC listener on a routable interface
/// is an explicit operator opt-in per ADR-013 §1 "Bind defaults per
/// adapter" (OSC inbound row).
pub const DEFAULT_INBOUND_BIND: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Default outbound OSC target address.
///
/// Loopback unless the operator configures a specific target IP per
/// ADR-013 §1 (OSC outbound row). The runtime must not transmit to a
/// non-loopback target without explicit operator configuration.
pub const DEFAULT_OUTBOUND_TARGET: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Maximum accepted OSC packet size in bytes (64 KiB).
///
/// Oversize packets are rejected with an `osc-malformed`-class
/// diagnostic and no state mutation per ADR-013 §5 "Payload
/// hardening" bullet 1.
pub const DEFAULT_MAX_PACKET_BYTES: usize = 65_536;

/// Maximum nested OSC bundle depth.
///
/// Deeper payloads are rejected per ADR-013 §5 "Payload hardening"
/// bullet 2. The cap exists to bound recursive bundle-expansion cost
/// and to block trivial stack-exhaustion payloads.
pub const DEFAULT_MAX_BUNDLE_DEPTH: u8 = 4;

/// Default per-route inbound rate ceiling, in messages per second.
///
/// Per-route default is 100 msg/s per unique `(adapter, route)` pair
/// per ADR-013 §3 "Rate limits" bullet 2. Operator-configurable.
/// Over-rate messages are dropped with an `osc-rate-limited`
/// diagnostic emitted at a throttled cadence (no more than 1/s per
/// route) per bullet 3 of the same section.
pub const DEFAULT_RATE_LIMIT_MSGS_PER_SEC: u32 = 100;

/// Default per-adapter inbound rate ceiling floor, in messages per
/// second.
///
/// ADR-013 §3 "Rate limits" bullet 1: operator-configurable floor is
/// 100 msg/s; the hard cap (1000 msg/s) is not represented here
/// because it is a non-configurable ceiling, not a default.
pub const DEFAULT_ADAPTER_RATE_LIMIT_FLOOR_MSGS_PER_SEC: u32 = 100;

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    use super::*;

    /// Evidence: EV-NET-HARDENING — ADR-013 §1 bind defaults hold.
    #[test]
    fn inbound_bind_default_is_loopback() {
        assert_eq!(DEFAULT_INBOUND_BIND, IpAddr::V4(Ipv4Addr::LOCALHOST));
    }

    /// Evidence: EV-NET-HARDENING — ADR-013 §1 outbound target
    /// defaults to loopback until operator configures otherwise.
    #[test]
    fn outbound_target_default_is_loopback() {
        assert_eq!(DEFAULT_OUTBOUND_TARGET, IpAddr::V4(Ipv4Addr::LOCALHOST));
    }

    /// Evidence: EV-NET-HARDENING — ADR-013 §5 packet-hardening caps
    /// hold at 64 KiB / depth 4.
    #[test]
    fn packet_hardening_caps_match_adr_013() {
        assert_eq!(DEFAULT_MAX_PACKET_BYTES, 65_536);
        assert_eq!(DEFAULT_MAX_BUNDLE_DEPTH, 4);
    }

    /// Evidence: EV-NET-HARDENING — ADR-013 §3 per-route rate-limit
    /// default is 100 msg/s.
    #[test]
    fn rate_limit_default_matches_adr_013() {
        assert_eq!(DEFAULT_RATE_LIMIT_MSGS_PER_SEC, 100);
        assert_eq!(DEFAULT_ADAPTER_RATE_LIMIT_FLOOR_MSGS_PER_SEC, 100);
    }
}
