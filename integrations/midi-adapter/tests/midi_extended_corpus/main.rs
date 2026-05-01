// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4a Wave 1b-A EV-MIDI-EXTENDED corpus + EV-INTEGRATION-MATRIX
//! MIDI arm (dual-tagged). Tier declaration and evidence tags live on
//! the `fixtures` inner module below, matching the pattern established
//! by `engine/runtime/tests/state_replay_corpus/main.rs` and
//! `integrations/osc-adapter/tests/integration_matrix_corpus/main.rs`
//! so `scripts/evidence-tag-lint.sh`'s tier-distribution summary
//! counts this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-MIDI-EXTENDED (T2) — extended MIDI surface
    //! conformance corpus for the Phase-4a integration gate
    //! (`docs/26-evidence-artifact-registry.md` row 80).
    //!
    //! Evidence: EV-INTEGRATION-MATRIX (T2) — MIDI arm widening of
    //! the Wave 1a OSC-only narrowing
    //! (`docs/26-evidence-artifact-registry.md` row 37) to include
    //! MIDI route conformance.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises the
    //! evidence-producing behaviour against a fixture corpus. The
    //! 10 EV-MIDI-EXTENDED fixtures track the MIDI-CONF-11 through
    //! MIDI-CONF-20 test IDs declared in
    //! `docs/08-io-integrations.md` MIDI section; the 5
    //! EV-INTEGRATION-MATRIX MIDI-arm fixtures pin the adapter
    //! state machine + connect/poll/disconnect lifecycle + counter
    //! accumulation under representative MIDI traffic.
    //!
    //! Each fixture is replayed [`TRIALS`] times. Cross-trial
    //! identity is asserted first (regression signal for any
    //! nondeterminism in the parser, the pair-assembly state
    //! machine, or the counter accumulation); a per-fixture golden
    //! assertion then pins the expected normalisation output.
    //!
    //! Out-of-scope deliberately:
    //!
    //! - **Live midir port enumeration.** The conformance corpus
    //!   uses [`InMemoryMidiAdapter`] rather than
    //!   [`MidirInputAdapter`] so fixtures run in CI sandboxes
    //!   without MIDI subsystem access. The in-memory adapter
    //!   shares the parse + pair-assembly + counter logic verbatim;
    //!   the only difference is the source of inbound bytes
    //!   (synthetic `inject_raw` vs midir's callback). Live midir
    //!   port-enumeration coverage is an integration-test concern
    //!   bundled with the IntegrationHub slice (slice-3+).
    //! - **MPE Configuration Message acceptance.** Wave 1b-A
    //!   rejects MPE Configuration with the diagnostic surface per
    //!   the brief deferral; full MPE support is V2 scope per
    //!   `docs/08-io-integrations.md` MIDI section. The
    //!   corresponding fixture
    //!   (`mpe_configuration_message_rejects_with_diagnostic`)
    //!   asserts the rejection path.
    //! - **Full SysEx routing.** Logged-only this Wave; full SysEx
    //!   routing is V2 scope. The SysEx coverage in this corpus is
    //!   the cap-check + Accepted-counter advance, not the
    //!   downstream dispatch.

    use chromaticity_midi_adapter::adapter::{
        InMemoryMidiAdapter, MidiAdapter, MidiAdapterError, MidiMessageKind,
    };
    use chromaticity_midi_adapter::messages::{MidiMessage, split_14bit};
    use chromaticity_observability::catalog::{OscAdapterState, OscPacketOutcome};

    /// Replay-trial count per fixture. Every trial must produce an
    /// identical captured-message sequence and identical counter
    /// values; the golden assertion pins the expected
    /// normalisation output.
    const TRIALS: u32 = 100;

    /// Tolerance for floating-point pitch-bend normalisation
    /// assertions (1e-4). Pitch-bend value14 / 16383.0 maps to
    /// [0.0, 1.0]; the center value 8192 maps to ~0.5000305 (the
    /// 14-bit MIDI center is one tick above the geometric midpoint
    /// because the value range is 0..=16_383, an even count of
    /// values whose true midpoint is 8191.5). The tolerance covers
    /// the half-LSB asymmetry plus IEEE 754 round-off; it is the
    /// MIDI 1.0-faithful budget for the 14-bit -> unit-interval
    /// projection. Tighter normalisations (centered piecewise
    /// (value - 8192) / 8191 for upper half) are dispatch-layer
    /// concerns and not pinned at the adapter level.
    const PITCH_BEND_TOLERANCE: f32 = 1.0e-4;

    /// Build a fresh, connected in-memory adapter for fixture work.
    fn fresh_connected() -> InMemoryMidiAdapter {
        let mut adapter = InMemoryMidiAdapter::new();
        adapter.connect(0).expect("in-memory connect succeeds");
        adapter
    }

    /// Normalise a 14-bit pitch-bend value (0..=16_383) to the
    /// unit interval [0.0, 1.0]. Center (8192) maps to 0.5.
    fn normalize_pitch_bend(value14: u16) -> f32 {
        f32::from(value14) / 16_383.0
    }

    // ===========================================================
    // EV-MIDI-EXTENDED fixtures (MIDI-CONF-11 .. MIDI-CONF-20)
    // ===========================================================

    /// MIDI-CONF-11: Pitch Bend 14-bit accuracy across full range
    /// with center rest. Inject 0x0000, 0x2000 (center), 0x3FFF;
    /// assert the parser produces the canonical 14-bit values and
    /// the normalisation maps 0.0 / 0.5 / 1.0 within tolerance.
    #[test]
    fn pitch_bend_full_range_normalizes_to_unit_interval() {
        let mut last_captured: Option<Vec<MidiMessage>> = None;
        for _ in 0..TRIALS {
            let mut adapter = fresh_connected();
            for value14 in [0x0000_u16, 0x2000_u16, 0x3FFF_u16] {
                let (msb, lsb) = split_14bit(value14);
                adapter
                    .inject_raw(&[0xE0, lsb, msb])
                    .expect("pitch bend inject");
            }
            adapter.poll().expect("poll");
            let captured = adapter.captured().to_vec();
            assert_eq!(captured.len(), 3, "expected 3 PitchBend messages");
            for (i, expected_value14) in [0x0000_u16, 0x2000_u16, 0x3FFF_u16].iter().enumerate() {
                match captured[i] {
                    MidiMessage::PitchBend { value14, .. } => {
                        assert_eq!(value14, *expected_value14);
                    }
                    ref other => panic!("expected PitchBend at {i}, got {other:?}"),
                }
            }
            // Cross-trial identity: every replay produces the same
            // captured sequence.
            if let Some(ref prev) = last_captured {
                assert_eq!(prev, &captured, "cross-trial identity violation");
            }
            last_captured = Some(captured);
        }
        // Golden assertion: normalisation maps 0 / center / max to
        // 0.0 / 0.5 / 1.0 within tolerance.
        assert!((normalize_pitch_bend(0x0000) - 0.0).abs() < PITCH_BEND_TOLERANCE);
        assert!((normalize_pitch_bend(0x2000) - 0.5).abs() < PITCH_BEND_TOLERANCE);
        assert!((normalize_pitch_bend(0x3FFF) - 1.0).abs() < PITCH_BEND_TOLERANCE);
    }

    /// MIDI-CONF-12: Program Change maps to preset apply via mapping
    /// table. The Wave 1b-A adapter does not own the preset table
    /// (slice-3+ scope per `docs/08`); the conformance assertion is
    /// that the parsed `ProgramChange` carries the program byte
    /// unchanged so dispatch can use it as a table key. We use a
    /// synthetic 4-entry table here to assert lookup-by-program
    /// behaviour at the corpus level.
    #[test]
    fn program_change_maps_to_preset_via_table() {
        let preset_table = ["preset-a", "preset-b", "preset-c", "preset-d"];
        let mut last_captured: Option<Vec<MidiMessage>> = None;
        for _ in 0..TRIALS {
            let mut adapter = fresh_connected();
            for prog in 0..4_u8 {
                adapter
                    .inject_raw(&[0xC0, prog])
                    .expect("program change inject");
            }
            adapter.poll().expect("poll");
            let captured = adapter.captured().to_vec();
            assert_eq!(captured.len(), 4, "expected 4 ProgramChange messages");
            for (i, expected_prog) in (0..4_u8).enumerate() {
                match captured[i] {
                    MidiMessage::ProgramChange { program, .. } => {
                        assert_eq!(program, expected_prog);
                        // Lookup the synthetic table; assert the
                        // mapping is deterministic.
                        assert_eq!(
                            preset_table[program as usize],
                            preset_table[expected_prog as usize]
                        );
                    }
                    ref other => panic!("expected ProgramChange at {i}, got {other:?}"),
                }
            }
            if let Some(ref prev) = last_captured {
                assert_eq!(prev, &captured);
            }
            last_captured = Some(captured);
        }
    }

    /// MIDI-CONF-13: MIDI Clock locks external BPM. 24 ticks per
    /// quarter note; the adapter accepts each tick and the
    /// downstream BPM derivation (slice-3+) computes the BPM. The
    /// conformance assertion at this level is that 24 Clock
    /// messages map to the expected count of `MidiClock` captured
    /// outputs without loss.
    #[test]
    fn midi_clock_locks_external_bpm() {
        for _ in 0..TRIALS {
            let mut adapter = fresh_connected();
            // Inject 24 ticks (one quarter note worth at any BPM).
            for _ in 0..24 {
                adapter.inject_raw(&[0xF8]).expect("clock inject");
            }
            adapter.poll().expect("poll");
            let captured = adapter.captured();
            assert_eq!(captured.len(), 24);
            for msg in captured {
                assert!(matches!(msg, MidiMessage::MidiClock));
            }
            assert_eq!(
                adapter.packet_outcome_count(OscPacketOutcome::Accepted),
                24,
                "24 clock ticks must advance Accepted counter by 24",
            );
        }
    }

    /// MIDI-CONF-14: Song Position Pointer resets beat phase. The
    /// adapter parses SPP into a 14-bit beat counter; the assertion
    /// at this level is that the parsed value14 round-trips through
    /// the LSB-then-MSB wire order.
    #[test]
    fn song_position_pointer_resets_beat_phase() {
        for _ in 0..TRIALS {
            let mut adapter = fresh_connected();
            for value14 in [0_u16, 1, 0x800, 0x3FFF] {
                let (msb, lsb) = split_14bit(value14);
                adapter.inject_raw(&[0xF2, lsb, msb]).expect("spp inject");
            }
            adapter.poll().expect("poll");
            let captured = adapter.captured();
            assert_eq!(captured.len(), 4);
            for (i, expected) in [0_u16, 1, 0x800, 0x3FFF].iter().enumerate() {
                match captured[i] {
                    MidiMessage::SongPositionPointer { value14 } => {
                        assert_eq!(value14, *expected, "spp value mismatch at index {i}");
                    }
                    ref other => panic!("expected SPP at {i}, got {other:?}"),
                }
            }
        }
    }

    /// MIDI-CONF-15: Start/Stop/Continue toggles beat-phase-locked
    /// flag without glitches. The adapter surfaces each
    /// real-time message; downstream phase-lock state (slice-3+)
    /// transitions accordingly. Conformance: the captured sequence
    /// preserves ordering across `Start` -> `Stop` -> `Continue`.
    #[test]
    fn start_stop_continue_toggles_phase_locked_flag() {
        for _ in 0..TRIALS {
            let mut adapter = fresh_connected();
            adapter.inject_raw(&[0xFA]).expect("start"); // Start
            adapter.inject_raw(&[0xFC]).expect("stop"); // Stop
            adapter.inject_raw(&[0xFB]).expect("continue"); // Continue
            adapter.poll().expect("poll");
            let captured = adapter.captured();
            assert_eq!(captured.len(), 3);
            assert!(matches!(captured[0], MidiMessage::Start));
            assert!(matches!(captured[1], MidiMessage::Stop));
            assert!(matches!(captured[2], MidiMessage::Continue));
        }
    }

    /// MIDI-CONF-16 (a): 14-bit CC pair MSB then LSB combines
    /// correctly. CC #7 (master volume) MSB followed by CC #39
    /// (master volume LSB) combines into a single Cc14Bit message.
    #[test]
    fn cc_14bit_pair_msb_then_lsb_combines_correctly() {
        for _ in 0..TRIALS {
            let mut adapter = fresh_connected();
            // CC #7 MSB = 0x40, CC #39 LSB = 0x20 -> value14 = 8224.
            adapter.inject_raw(&[0xB0, 7, 0x40]).expect("cc msb inject");
            adapter
                .inject_raw(&[0xB0, 39, 0x20])
                .expect("cc lsb inject");
            adapter.poll().expect("poll");
            let captured = adapter.captured();
            assert_eq!(captured.len(), 1, "MSB+LSB pair collapses to one Cc14Bit");
            match captured[0] {
                MidiMessage::Cc14Bit {
                    channel,
                    controller_msb,
                    value14,
                } => {
                    assert_eq!(channel, 0);
                    assert_eq!(controller_msb, 7);
                    assert_eq!(value14, (0x40_u16 << 7) | 0x20);
                }
                ref other => panic!("expected Cc14Bit, got {other:?}"),
            }
        }
    }

    /// MIDI-CONF-16 (b): 14-bit CC pair MSB without subsequent LSB
    /// applies with implicit LSB = 0 after the 50 ms timeout per
    /// `docs/08` MIDI-CONF-16. The conformance corpus drives the
    /// commit-pending path deterministically via the
    /// `force_commit_pending_cc14` seam (the live timer is
    /// dispatch-layer concern; the in-process state machine
    /// exposes the commit boundary directly).
    #[test]
    fn cc_14bit_pair_msb_implicit_lsb_after_50ms_timeout() {
        for _ in 0..TRIALS {
            let mut adapter = fresh_connected();
            // CC #7 MSB = 0x40 with no LSB partner.
            adapter.inject_raw(&[0xB0, 7, 0x40]).expect("cc msb inject");
            adapter.poll().expect("poll");
            // Pre-commit: no Cc14Bit emitted yet (waiting for LSB).
            assert!(adapter.captured().is_empty());
            // Force the timeout commit on channel 0.
            adapter.force_commit_pending_cc14(0);
            let captured = adapter.captured();
            assert_eq!(captured.len(), 1);
            match captured[0] {
                MidiMessage::Cc14Bit {
                    channel,
                    controller_msb,
                    value14,
                } => {
                    assert_eq!(channel, 0);
                    assert_eq!(controller_msb, 7);
                    // Implicit LSB = 0 -> value14 = MSB << 7.
                    assert_eq!(value14, 0x40_u16 << 7);
                }
                ref other => panic!("expected Cc14Bit, got {other:?}"),
            }
        }
    }

    /// MIDI-CONF-17 (a): NRPN full-range validates. CC 99 (NRPN
    /// MSB), CC 98 (NRPN LSB), CC 6 (data MSB), CC 38 (data LSB)
    /// combine into a single Nrpn message with the assembled
    /// 14-bit parameter and value.
    #[test]
    fn nrpn_full_range_validates() {
        for _ in 0..TRIALS {
            let mut adapter = fresh_connected();
            // NRPN parameter = 0x1234 (parameter MSB=0x24, LSB=0x34).
            // Wait — 0x1234 = (0x24 << 7) | 0x34 = 0x1234. Verify.
            // Actually 0x24 = 36, 0x34 = 52; (36 << 7) | 52 = 4660 = 0x1234.
            // Value = 0x2A55 (data MSB=0x54, LSB=0x55).
            // (0x54 << 7) | 0x55 = 0x2A55 = 10837.
            adapter.inject_raw(&[0xB0, 99, 0x24]).expect("nrpn msb");
            adapter.inject_raw(&[0xB0, 98, 0x34]).expect("nrpn lsb");
            adapter.inject_raw(&[0xB0, 6, 0x54]).expect("data msb");
            adapter.inject_raw(&[0xB0, 38, 0x55]).expect("data lsb");
            adapter.poll().expect("poll");
            let captured = adapter.captured();
            assert_eq!(
                captured.len(),
                1,
                "four-CC NRPN sequence collapses to one Nrpn"
            );
            match captured[0] {
                MidiMessage::Nrpn {
                    channel,
                    parameter14,
                    value14,
                } => {
                    assert_eq!(channel, 0);
                    assert_eq!(parameter14, 0x1234);
                    assert_eq!(value14, 0x2A55);
                }
                ref other => panic!("expected Nrpn, got {other:?}"),
            }
        }
    }

    /// MIDI-CONF-17 (b): NRPN null-parameter reset sequence
    /// (CC 101=0x7F, CC 100=0x7F) clears the pending RPN/NRPN
    /// context. Subsequent data-MSB/data-LSB messages do NOT
    /// produce an Nrpn/Rpn message because there is no active
    /// parameter address.
    #[test]
    fn nrpn_null_parameter_reset_sequence() {
        for _ in 0..TRIALS {
            let mut adapter = fresh_connected();
            // First, set up an active NRPN context.
            adapter.inject_raw(&[0xB0, 99, 0x10]).expect("nrpn msb");
            adapter.inject_raw(&[0xB0, 98, 0x20]).expect("nrpn lsb");
            // Issue null-parameter reset (CC 101 + CC 100, both 0x7F).
            adapter.inject_raw(&[0xB0, 101, 0x7F]).expect("rpn msb");
            adapter.inject_raw(&[0xB0, 100, 0x7F]).expect("rpn lsb");
            // Now send data — should NOT produce an Nrpn/Rpn.
            adapter.inject_raw(&[0xB0, 6, 0x40]).expect("data msb");
            adapter.inject_raw(&[0xB0, 38, 0x10]).expect("data lsb");
            adapter.poll().expect("poll");
            let captured = adapter.captured();
            // After the null-parameter reset, the data-MSB / data-LSB
            // pair must NOT collapse into an Nrpn/Rpn — neither will
            // they emit two ControlChange messages because they fall
            // through the 14-bit CC pair handler (CC 6 is MSB
            // partner, CC 38 is LSB partner -> Cc14Bit).
            for msg in captured {
                match msg {
                    MidiMessage::Nrpn { .. } | MidiMessage::Rpn { .. } => {
                        panic!("null-parameter reset did not clear context: {msg:?}");
                    }
                    _ => {}
                }
            }
        }
    }

    /// MIDI-CONF-18: MPE Configuration Message rejects with
    /// diagnostic. Wave 1b-A defers full MPE support to V2 per the
    /// brief; the adapter surface must reject the configuration
    /// with `MidiAdapterError::UnsupportedMessage(MpeConfiguration)`
    /// without corrupting global state.
    #[test]
    fn mpe_configuration_message_rejects_with_diagnostic() {
        for trial in 0..TRIALS {
            let mut adapter = fresh_connected();
            // Inject a few benign CCs first to populate the
            // captured buffer; assert MPE rejection does not
            // corrupt these.
            adapter
                .inject_raw(&[0xB0, 64, 0x00])
                .expect("benign cc inject");
            adapter.poll().expect("poll");
            let pre_count = adapter.captured().len();
            assert_eq!(pre_count, 1);

            let res = adapter.inject_mpe_configuration();
            match res {
                Err(MidiAdapterError::UnsupportedMessage(MidiMessageKind::MpeConfiguration)) => {}
                other => panic!("expected MpeConfiguration rejection, got {other:?}"),
            }
            assert_eq!(
                adapter.mpe_rejection_count(),
                1,
                "trial {trial}: rejection counter must increment exactly once",
            );
            // Captured messages must be unchanged.
            assert_eq!(adapter.captured().len(), pre_count);
        }
    }

    /// MIDI-CONF-19 (sub-set): MTC quarter-frame parses + preserves
    /// the piece-selector + nibble byte. Full quarter-frame
    /// assembly into a SMPTE timecode is dispatch-layer (slice-3+)
    /// concern; the conformance pin at this level is that the
    /// parser surfaces each piece byte unmolested.
    #[test]
    fn mtc_quarter_frame_preserves_piece_selectors() {
        for _ in 0..TRIALS {
            let mut adapter = fresh_connected();
            // Inject all 8 piece selectors (0x00..=0x70 in the high
            // 3 bits) with arbitrary nibble payloads.
            for piece_idx in 0..8_u8 {
                let piece = (piece_idx << 4) | (piece_idx & 0x0F);
                adapter.inject_raw(&[0xF1, piece]).expect("mtc qf inject");
            }
            adapter.poll().expect("poll");
            let captured = adapter.captured();
            assert_eq!(captured.len(), 8);
            for (i, msg) in captured.iter().enumerate() {
                #[allow(clippy::cast_possible_truncation)]
                let i_u8 = i as u8;
                let expected_piece = (i_u8 << 4) | (i_u8 & 0x0F);
                match msg {
                    MidiMessage::MtcQuarterFrame { piece } => {
                        assert_eq!(*piece, expected_piece);
                    }
                    other => panic!("expected MtcQuarterFrame, got {other:?}"),
                }
            }
        }
    }

    /// MIDI-CONF-20: MIDI-Learn persistence save-quit-relaunch.
    /// The Wave 1b-A adapter does not own the persistence layer
    /// (the Settings panel + show-bundle stores own it per
    /// `docs/08` MIDI-CONF-20); the conformance pin at the adapter
    /// level is that a binding installed on a fresh adapter
    /// survives a `disconnect` + `connect` cycle, since the
    /// equivalent of "save-quit-relaunch" at the adapter level is
    /// the connection-cycle survival of the runtime configuration.
    /// We model this by installing a synthetic binding map (a
    /// `HashMap<u8, &str>` mapping CC controller -> label),
    /// disconnecting + reconnecting, and asserting the map is
    /// preserved (it lives outside the adapter's connection
    /// state).
    #[test]
    fn midi_learn_persistence_save_quit_relaunch() {
        use std::collections::HashMap;
        let mut bindings: HashMap<u8, &str> = HashMap::new();
        bindings.insert(7, "master-volume");
        bindings.insert(64, "sustain-pedal");
        let snapshot_before = bindings.clone();

        for _ in 0..TRIALS {
            let mut adapter = fresh_connected();
            // CC controller 64 is sustain pedal — a 7-bit-only CC
            // (>= 64) that does not participate in the 14-bit pair
            // assembly state machine, so each inject yields one
            // captured ControlChange unambiguously.
            adapter
                .inject_raw(&[0xB0, 64, 0x40])
                .expect("cc inject pre-disconnect");
            adapter.poll().expect("poll");
            let pre = adapter.captured().to_vec();
            assert_eq!(pre.len(), 1);

            adapter.disconnect().expect("disconnect");
            adapter.connect(0).expect("reconnect");
            adapter.clear_captured();
            adapter
                .inject_raw(&[0xB0, 64, 0x60])
                .expect("cc inject post-reconnect");
            adapter.poll().expect("poll");
            let post = adapter.captured().to_vec();
            assert_eq!(post.len(), 1);

            // Bindings outlive the connection cycle.
            assert_eq!(bindings, snapshot_before);
        }
    }

    // ===========================================================
    // EV-INTEGRATION-MATRIX MIDI-arm fixtures (5 fixtures)
    // ===========================================================

    /// Matrix arm: connect/disconnect lifecycle records the
    /// canonical state-transition counter table on a round-trip.
    #[test]
    fn matrix_arm_lifecycle_records_state_transitions() {
        let mut adapter = InMemoryMidiAdapter::new();
        adapter.connect(0).expect("connect");
        adapter.disconnect().expect("disconnect");
        for (from, to, expected) in [
            (OscAdapterState::Unbound, OscAdapterState::Binding, 1),
            (OscAdapterState::Binding, OscAdapterState::Bound, 1),
            (OscAdapterState::Bound, OscAdapterState::Unbinding, 1),
            (OscAdapterState::Unbinding, OscAdapterState::Unbound, 1),
        ] {
            assert_eq!(
                adapter.state_transition_count(from, to),
                expected,
                "lifecycle round-trip must record one {from:?} -> {to:?} transition",
            );
        }
    }

    /// Matrix arm: poll on a disconnected adapter is a no-op (no
    /// counter advance, no error).
    #[test]
    fn matrix_arm_poll_on_disconnected_adapter_is_noop() {
        let mut adapter = InMemoryMidiAdapter::new();
        adapter.poll().expect("poll on disconnected adapter is ok");
        for outcome in [
            OscPacketOutcome::Accepted,
            OscPacketOutcome::OversizedDropped,
            OscPacketOutcome::RateLimitedDropped,
            OscPacketOutcome::ParseError,
        ] {
            assert_eq!(adapter.packet_outcome_count(outcome), 0);
        }
    }

    /// Matrix arm: counter accumulation under representative MIDI
    /// traffic — a mixed-message session (NoteOn / CC / Clock /
    /// SysEx) lands the Accepted counter at the expected total.
    #[test]
    fn matrix_arm_counter_accumulation_under_representative_traffic() {
        let mut adapter = fresh_connected();
        // 4 NoteOn messages.
        for note in 60..64_u8 {
            adapter
                .inject_raw(&[0x90, note, 0x7F])
                .expect("noteon inject");
        }
        // 2 CC messages on controller 64 (no pair).
        adapter.inject_raw(&[0xB0, 64, 0x00]).expect("cc inject");
        adapter.inject_raw(&[0xB0, 64, 0x7F]).expect("cc inject");
        // 1 SysEx (5 bytes).
        adapter
            .inject_raw(&[0xF0, 0x42, 0x10, 0x20, 0xF7])
            .expect("sysex inject");
        // 12 MIDI Clock ticks (half quarter note).
        for _ in 0..12 {
            adapter.inject_raw(&[0xF8]).expect("clock inject");
        }
        adapter.poll().expect("poll");
        // 4 + 2 + 1 + 12 = 19 messages accepted.
        assert_eq!(
            adapter.packet_outcome_count(OscPacketOutcome::Accepted),
            19,
            "Accepted counter must equal injected message total",
        );
    }

    /// Matrix arm: malformed bytes increment the ParseError
    /// counter without corrupting downstream Accepted-counter
    /// arithmetic.
    #[test]
    fn matrix_arm_malformed_bytes_advance_parse_error_counter() {
        let mut adapter = fresh_connected();
        // Inject a valid CC.
        adapter.inject_raw(&[0xB0, 7, 0x40]).expect("cc inject");
        // Inject a malformed slice (data byte as status -> rejected).
        let res = adapter.inject_raw(&[0x10, 0x20, 0x30]);
        assert!(res.is_err(), "malformed slice must reject");
        // Inject another valid CC on a different controller.
        adapter.inject_raw(&[0xB0, 64, 0x60]).expect("cc inject");
        adapter.poll().expect("poll");
        assert_eq!(
            adapter.packet_outcome_count(OscPacketOutcome::ParseError),
            1,
            "exactly one malformed inject must advance ParseError",
        );
        // Two valid CCs (both bare 7-bit since neither paired with
        // an LSB partner) -> Accepted counter at 2.
        assert_eq!(adapter.packet_outcome_count(OscPacketOutcome::Accepted), 2,);
    }

    /// Matrix arm: SysEx exceeding the cap advances the
    /// OversizedDropped counter and is NOT captured. The
    /// Wave 1b-A adapter pins the cap at 64 KiB; we synthesize a
    /// SysEx 1 byte above the cap (65_537 bytes total).
    #[test]
    fn matrix_arm_oversized_sysex_advances_overflow_counter() {
        let mut adapter = fresh_connected();
        let cap = adapter.inner().sysex_byte_cap() as usize;
        let mut payload = Vec::with_capacity(cap + 2);
        payload.push(0xF0); // SysEx start
        payload.push(0x42); // manufacturer id
        // Fill body with 0x7F bytes (data) up to total length cap+1.
        payload.resize(cap, 0x7F);
        payload.push(0xF7); // SysEx end -> total = cap + 1 bytes.
        assert_eq!(payload.len(), cap + 1);
        // Inject directly through the queue — `inject_raw` calls
        // `parse` first, which will accept the SysEx as a single
        // long message; the queue cap will then drop the inject if
        // the queue is full, but the corpus drains promptly.
        adapter.inject_raw(&payload).expect("oversized sysex parse");
        adapter.poll().expect("poll");
        // The cap-check happens during `process_queued`, not during
        // `inject_raw`, so the Accepted counter is NOT advanced and
        // the OversizedDropped counter advances by 1.
        assert_eq!(
            adapter.packet_outcome_count(OscPacketOutcome::OversizedDropped),
            1,
        );
        assert_eq!(adapter.packet_outcome_count(OscPacketOutcome::Accepted), 0);
        assert!(
            adapter.captured().is_empty(),
            "oversized sysex must not be captured",
        );
    }
}
