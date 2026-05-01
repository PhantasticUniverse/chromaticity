// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! MIDI-adapter default-value baseline. Produces evidence for
//! `EV-MIDI-EXTENDED` and re-anchors the cross-tagged
//! `EV-INTEGRATION-MATRIX` MIDI arm via the drift pins in the
//! nested `tests` module.
//!
//! The MIDI adapter inherits ADR-013's posture for the
//! networking-default surface (per-route rate ceiling; per-adapter
//! floor; payload cap) even though MIDI is not a networked transport
//! per se — the rate-limit and payload-cap defaults still bound
//! resource use under hostile (ill-behaved controller) inputs the
//! same way ADR-013 §3 / §5 caps bound OSC traffic. Where MIDI
//! diverges from OSC (e.g. SysEx is logged-only this Wave; full
//! SysEx routing is V2 scope), the corresponding default is chosen
//! to match the conservative posture and is documented inline.
//!
//! Operator-facing overrides do not live here: per ADR-013 §6 each
//! default is overridable in a Settings panel at runtime; this
//! module pins the out-of-the-box posture the adapter starts from.

/// Default in-memory inbound queue depth, in messages.
///
/// midir's callback fires on the audio thread; the Wave 1b-A adapter
/// pushes each parsed message into a `Mutex<VecDeque<MidiMessage>>`
/// so the trait surface stays non-blocking poll(). When the queue is
/// full the producer drops the oldest message (ring-buffer
/// semantics) and increments the malformed-dropped counter — back
/// pressure surfaces as a throttled diagnostic rather than blocking
/// the producer thread.
///
/// 256 entries is sized to absorb a worst-case MIDI Clock burst
/// (24 ticks/quarter at 240 BPM = 96 ticks/sec) plus typical CC
/// activity over a full second of poll latency.
pub const DEFAULT_QUEUE_DEPTH: usize = 256;

/// Default per-route inbound rate ceiling, in messages per second.
///
/// Mirrors the ADR-013 §3 OSC default verbatim — the per-route
/// ceiling for MIDI controller traffic is the same 100 msg/s the
/// OSC adapter applies to per-route traffic. Operator-configurable.
/// Over-rate messages are dropped with an `midi-rate-limited`
/// diagnostic emitted at a throttled cadence (no more than 1/s per
/// route) per the same bullet of ADR-013 §3.
pub const DEFAULT_RATE_LIMIT_MSGS_PER_SEC: u32 = 100;

/// Default per-adapter inbound rate ceiling floor, in messages per
/// second.
///
/// Higher than the OSC equivalent (1000 vs 100): the MIDI Clock
/// stream alone produces 24 ticks/quarter, which at 240 BPM is
/// 96 ticks/sec — well within 1000 — but a busy session combining
/// Clock + dense CC + MTC quarter-frames can approach the floor
/// without operator-visible misbehaviour. The 1000 msg/s floor
/// preserves the Clock burst envelope while still bounding hostile
/// adapter-level flooding.
pub const DEFAULT_ADAPTER_RATE_LIMIT_FLOOR_MSGS_PER_SEC: u32 = 1000;

/// Maximum accepted SysEx payload size in bytes (64 KiB).
///
/// Mirrors the ADR-013 §5 OSC packet cap. SysEx larger than this
/// limit is logged with the manufacturer-id prefix retained but the
/// payload truncated; the Wave 1b-A adapter does not route SysEx to
/// the runtime bus, so the cap is the resource fence rather than a
/// dispatch boundary. Full SysEx routing is V2 scope per
/// `docs/08-io-integrations.md` MIDI section.
pub const DEFAULT_MAX_SYSEX_BYTES: u32 = 65_536;

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-MIDI-EXTENDED (T1) — MIDI-adapter default-value
    //! pins for the queue-depth, rate-limit, and SysEx-cap baseline.
    //!
    //! T1 per docs/43 §15.5: each test pins an external-spec
    //! constant against the governing ADR / brief. Any drift in the
    //! queue-depth ceiling, rate-limit defaults, or SysEx cap fires
    //! the corresponding test, so review can route the change to
    //! the `EV-MIDI-EXTENDED` gate before merge. T1 counts toward
    //! `EV-EVIDENCE-TAG-COVERAGE`; Phase-4a exit requires T2 over
    //! the full midi-extended fixture corpus, which lives at
    //! `tests/midi_extended_corpus/main.rs`.

    use super::*;

    /// Evidence: EV-MIDI-EXTENDED — Wave 1b-A queue-depth default
    /// is 256 entries (sized for worst-case MIDI Clock burst plus
    /// CC activity over a full second of poll latency).
    #[test]
    fn queue_depth_default_is_256() {
        assert_eq!(DEFAULT_QUEUE_DEPTH, 256);
    }

    /// Evidence: EV-MIDI-EXTENDED — per-route rate-limit default
    /// matches the ADR-013 §3 OSC default (100 msg/s).
    #[test]
    fn rate_limit_default_matches_osc() {
        assert_eq!(DEFAULT_RATE_LIMIT_MSGS_PER_SEC, 100);
    }

    /// Evidence: EV-MIDI-EXTENDED — per-adapter rate-limit floor
    /// is 1000 msg/s, sized to absorb MIDI Clock + dense CC traffic
    /// without flagging well-behaved controllers as flooding.
    #[test]
    fn adapter_rate_limit_floor_default_is_1000() {
        assert_eq!(DEFAULT_ADAPTER_RATE_LIMIT_FLOOR_MSGS_PER_SEC, 1000);
    }

    /// Evidence: EV-MIDI-EXTENDED — SysEx cap mirrors the ADR-013
    /// §5 OSC packet cap (64 KiB).
    #[test]
    fn sysex_cap_default_is_64_kib() {
        assert_eq!(DEFAULT_MAX_SYSEX_BYTES, 65_536);
    }

    /// Evidence: EV-MIDI-EXTENDED — adapter-floor relationship pin:
    /// the per-adapter floor MUST be at least as permissive as the
    /// per-route ceiling. Otherwise a single route configured at the
    /// per-route default could starve other routes under the
    /// adapter floor.
    #[test]
    fn adapter_floor_is_at_least_per_route_default() {
        // Read constants through `let` bindings so clippy does not
        // fold the comparison into `assert!(true)` and reject the
        // pin under `assertions_on_constants`. The constant values
        // are the load-bearing invariant; the local bindings are
        // an optimisation barrier the compiler cannot evaluate at
        // const-check time without const_evaluatable_checked.
        let floor: u32 = DEFAULT_ADAPTER_RATE_LIMIT_FLOOR_MSGS_PER_SEC;
        let route: u32 = DEFAULT_RATE_LIMIT_MSGS_PER_SEC;
        assert!(
            floor >= route,
            "adapter floor ({floor}) must be >= per-route ceiling ({route})",
        );
    }
}
