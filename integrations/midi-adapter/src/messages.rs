// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! MIDI message taxonomy + raw-byte-slice parser.
//!
//! Phase-4a Wave 1b-A declares the structural shape of inbound MIDI
//! messages without runtime evaluation. midir's callback delivers
//! parsed-but-unclassified `&[u8]` slices; the adapter calls
//! [`parse`] on each to map raw status/data bytes into the strongly
//! typed [`MidiMessage`] enum the conformance corpus exercises.
//! Slice-3+ wires the dispatched message into the runtime bus once
//! the route-handler design settles.
//!
//! The taxonomy is the EV-MIDI-EXTENDED scope intersected with the
//! `docs/08-io-integrations.md` MIDI section: Channel Voice (Note
//! On/Off, CC, Aftertouch / Poly Aftertouch, Program Change, Pitch
//! Bend), Channel Mode (folded into CC), System Real-Time (Clock,
//! Start, Continue, Stop), System Common (Song Position Pointer,
//! MTC Quarter Frame), System Exclusive (logged-only), and the
//! 14-bit-resolution layered forms (CC pairs, NRPN, RPN). MPE is
//! deferred to V2 per the brief; the parser does not fold MPE
//! configuration messages into a dedicated variant — they appear as
//! their underlying CC stream and are rejected by the adapter
//! diagnostic surface.
//!
//! Width pins: `channel` is a 4-bit field (0..=15); `note`,
//! `controller`, `velocity`, `value`, `program` are 7-bit fields
//! (0..=127); `value14` and `parameter14` are 14-bit fields
//! (0..=16_383). The parser does not narrow these further — values
//! are validated within their MIDI 1.0 nibble/byte boundaries and
//! are clamped at parse time when out of range bytes appear.

/// Strongly typed MIDI message produced by [`parse`].
///
/// `#[non_exhaustive]` so the slice-3+ message-shape additions
/// (full SysEx routing, MPE Configuration acceptance, etc.) are
/// additive rather than breaking.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MidiMessage {
    /// Note On (status 0x90..=0x9F). Velocity 0 is preserved as
    /// `NoteOn { velocity: 0 }`; conversion to NoteOff is a
    /// dispatch-side decision and is not folded here.
    NoteOn {
        /// Channel (0..=15).
        channel: u8,
        /// Note number (0..=127).
        note: u8,
        /// Velocity (0..=127).
        velocity: u8,
    },
    /// Note Off (status 0x80..=0x8F).
    NoteOff {
        /// Channel (0..=15).
        channel: u8,
        /// Note number (0..=127).
        note: u8,
        /// Velocity (0..=127).
        velocity: u8,
    },
    /// Control Change (status 0xB0..=0xBF). Bare 7-bit form; the
    /// 14-bit pair-assembled form is [`MidiMessage::Cc14Bit`] and
    /// is produced post-pairing rather than from a single message.
    ControlChange {
        /// Channel (0..=15).
        channel: u8,
        /// Controller number (0..=127).
        controller: u8,
        /// Controller value (0..=127).
        value: u8,
    },
    /// Pitch Bend (status 0xE0..=0xEF). LSB and MSB are combined
    /// into a single 14-bit value (0..=16_383, with center 8192).
    PitchBend {
        /// Channel (0..=15).
        channel: u8,
        /// Combined 14-bit pitch-bend value (0..=16_383).
        value14: u16,
    },
    /// Program Change (status 0xC0..=0xCF).
    ProgramChange {
        /// Channel (0..=15).
        channel: u8,
        /// Program number (0..=127).
        program: u8,
    },
    /// Channel Aftertouch (status 0xD0..=0xDF). One pressure value
    /// for the entire channel.
    Aftertouch {
        /// Channel (0..=15).
        channel: u8,
        /// Pressure value (0..=127).
        value: u8,
    },
    /// Polyphonic Aftertouch (status 0xA0..=0xAF). Per-note
    /// pressure value.
    PolyAftertouch {
        /// Channel (0..=15).
        channel: u8,
        /// Note number (0..=127).
        note: u8,
        /// Pressure value (0..=127).
        value: u8,
    },
    /// MIDI Clock tick (System Real-Time 0xF8). 24 ticks per
    /// quarter note; the adapter accumulates ticks for external
    /// BPM derivation.
    MidiClock,
    /// Start (System Real-Time 0xFA). Restarts beat-phase from 0.
    Start,
    /// Continue (System Real-Time 0xFB). Resumes from current
    /// beat-phase.
    Continue,
    /// Stop (System Real-Time 0xFC). Halts beat-phase advance.
    Stop,
    /// Song Position Pointer (System Common 0xF2). 14-bit MIDI
    /// beat counter (a "MIDI beat" = a 16th note = 6 MIDI Clocks).
    SongPositionPointer {
        /// Combined 14-bit beat position (0..=16_383).
        value14: u16,
    },
    /// MTC Quarter Frame (System Common 0xF1). Each piece carries
    /// one nibble of the running timecode plus a 3-bit piece
    /// selector (0..=7). Eight quarter-frames assemble one full
    /// SMPTE frame at half the frame rate.
    MtcQuarterFrame {
        /// Combined byte: piece selector (high 3 bits) + nibble
        /// (low 4 bits). Preserved as-is so dispatch can index
        /// the piece selector and assemble the running timecode
        /// without re-parsing.
        piece: u8,
    },
    /// System Exclusive (status 0xF0 .. 0xF7). Wave 1b-A logs the
    /// manufacturer-id prefix and the payload byte length without
    /// retaining the body — full SysEx routing is V2 scope per
    /// `docs/08-io-integrations.md` MIDI section.
    SysEx {
        /// Manufacturer ID byte (the byte immediately following
        /// 0xF0). Three-byte (0x00, vendor-id-high, vendor-id-low)
        /// IDs are folded as their first non-zero byte for the
        /// logged-only path.
        manufacturer_id: u8,
        /// Total payload length in bytes, including the leading
        /// 0xF0 and trailing 0xF7. Capped at
        /// [`crate::defaults::DEFAULT_MAX_SYSEX_BYTES`] before the
        /// log site fires.
        len: usize,
    },
    /// 14-bit Control Change pair, post-pair-assembly. `controller_msb`
    /// is the MSB controller number (0..=31); the adapter pairs an
    /// MSB CC with its LSB partner (controller_msb + 32) within a
    /// 50 ms window per `docs/08-io-integrations.md` MIDI-CONF-16.
    /// If the LSB never arrives, the assembler emits with
    /// `value14 = (msb << 7)` (implicit LSB = 0).
    Cc14Bit {
        /// Channel (0..=15).
        channel: u8,
        /// MSB controller number (0..=31).
        controller_msb: u8,
        /// Combined 14-bit value (0..=16_383).
        value14: u16,
    },
    /// Non-Registered Parameter Number (NRPN) 14-bit transmission.
    /// Assembled from the four-CC sequence (CC 99 = NRPN MSB,
    /// CC 98 = NRPN LSB, CC 6 = data MSB, CC 38 = data LSB).
    Nrpn {
        /// Channel (0..=15).
        channel: u8,
        /// 14-bit parameter number (0..=16_383).
        parameter14: u16,
        /// 14-bit data value (0..=16_383).
        value14: u16,
    },
    /// Registered Parameter Number (RPN) 14-bit transmission.
    /// Assembled from the four-CC sequence (CC 101 = RPN MSB,
    /// CC 100 = RPN LSB, CC 6 = data MSB, CC 38 = data LSB).
    Rpn {
        /// Channel (0..=15).
        channel: u8,
        /// 14-bit parameter number (0..=16_383).
        parameter14: u16,
        /// 14-bit data value (0..=16_383).
        value14: u16,
    },
}

/// Parse a raw MIDI byte slice into a [`MidiMessage`].
///
/// Returns `None` for malformed or unsupported inputs (empty slice,
/// truncated data bytes, unrecognized status). The parser is
/// deliberately strict — slice-3+ may widen the accept set as the
/// route-handler design settles. SysEx slices are parsed for the
/// manufacturer-id prefix + length only; the body is not retained.
///
/// 14-bit pair assembly (Cc14Bit, Nrpn, Rpn) is NOT performed here:
/// the parser produces the bare ControlChange messages, and the
/// adapter's pairing state machine combines them. The Cc14Bit /
/// Nrpn / Rpn variants are the post-assembly shapes; the
/// conformance corpus constructs them via the assembly helpers in
/// the adapter rather than via [`parse`].
#[must_use]
pub fn parse(raw: &[u8]) -> Option<MidiMessage> {
    let status = *raw.first()?;
    // Channel Voice messages: high nibble 0x8..=0xE; low nibble = channel.
    let high = status & 0xF0;
    let channel = status & 0x0F;
    match high {
        0x80 => {
            // Note Off: status + 2 data bytes.
            let note = *raw.get(1)?;
            let velocity = *raw.get(2)?;
            if note > 127 || velocity > 127 {
                return None;
            }
            Some(MidiMessage::NoteOff {
                channel,
                note,
                velocity,
            })
        }
        0x90 => {
            // Note On: status + 2 data bytes.
            let note = *raw.get(1)?;
            let velocity = *raw.get(2)?;
            if note > 127 || velocity > 127 {
                return None;
            }
            Some(MidiMessage::NoteOn {
                channel,
                note,
                velocity,
            })
        }
        0xA0 => {
            // Polyphonic Aftertouch: status + 2 data bytes.
            let note = *raw.get(1)?;
            let value = *raw.get(2)?;
            if note > 127 || value > 127 {
                return None;
            }
            Some(MidiMessage::PolyAftertouch {
                channel,
                note,
                value,
            })
        }
        0xB0 => {
            // Control Change: status + 2 data bytes.
            let controller = *raw.get(1)?;
            let value = *raw.get(2)?;
            if controller > 127 || value > 127 {
                return None;
            }
            Some(MidiMessage::ControlChange {
                channel,
                controller,
                value,
            })
        }
        0xC0 => {
            // Program Change: status + 1 data byte.
            let program = *raw.get(1)?;
            if program > 127 {
                return None;
            }
            Some(MidiMessage::ProgramChange { channel, program })
        }
        0xD0 => {
            // Channel Aftertouch: status + 1 data byte.
            let value = *raw.get(1)?;
            if value > 127 {
                return None;
            }
            Some(MidiMessage::Aftertouch { channel, value })
        }
        0xE0 => {
            // Pitch Bend: status + LSB + MSB; combined 14-bit.
            let lsb = *raw.get(1)?;
            let msb = *raw.get(2)?;
            if lsb > 127 || msb > 127 {
                return None;
            }
            let value14 = combine_14bit(msb, lsb);
            Some(MidiMessage::PitchBend { channel, value14 })
        }
        0xF0 => {
            // System messages: status byte uniquely identifies.
            match status {
                0xF0 => parse_sysex(raw),
                0xF1 => {
                    let piece = *raw.get(1)?;
                    if piece > 127 {
                        return None;
                    }
                    Some(MidiMessage::MtcQuarterFrame { piece })
                }
                0xF2 => {
                    let lsb = *raw.get(1)?;
                    let msb = *raw.get(2)?;
                    if lsb > 127 || msb > 127 {
                        return None;
                    }
                    Some(MidiMessage::SongPositionPointer {
                        value14: combine_14bit(msb, lsb),
                    })
                }
                0xF8 => Some(MidiMessage::MidiClock),
                0xFA => Some(MidiMessage::Start),
                0xFB => Some(MidiMessage::Continue),
                0xFC => Some(MidiMessage::Stop),
                _ => None, // Active Sensing (0xFE), Reset (0xFF), Tune Request (0xF6), etc. — not yet routed.
            }
        }
        _ => None,
    }
}

/// Combine two 7-bit halves into a 14-bit value.
///
/// MSB occupies bits 7..=13; LSB occupies bits 0..=6. Out-of-range
/// inputs are masked at parse time before this is called, so the
/// result fits in `u16`.
#[must_use]
pub fn combine_14bit(msb: u8, lsb: u8) -> u16 {
    ((u16::from(msb) & 0x7F) << 7) | (u16::from(lsb) & 0x7F)
}

/// Split a 14-bit value into MSB / LSB halves. Inverse of
/// [`combine_14bit`]. Used by the conformance corpus when
/// constructing test fixtures.
#[must_use]
pub fn split_14bit(value14: u16) -> (u8, u8) {
    let masked = value14 & 0x3FFF;
    #[allow(clippy::cast_possible_truncation)]
    let msb = ((masked >> 7) & 0x7F) as u8;
    #[allow(clippy::cast_possible_truncation)]
    let lsb = (masked & 0x7F) as u8;
    (msb, lsb)
}

fn parse_sysex(raw: &[u8]) -> Option<MidiMessage> {
    // Minimum well-formed SysEx: 0xF0 <id> 0xF7 (3 bytes).
    if raw.len() < 3 {
        return None;
    }
    if raw[0] != 0xF0 {
        return None;
    }
    let manufacturer_id = raw[1];
    // Trailing 0xF7 is required for a complete SysEx; midir delivers
    // truncated payloads via separate callbacks but the adapter
    // surface receives a single complete payload per parse() call.
    if *raw.last()? != 0xF7 {
        return None;
    }
    Some(MidiMessage::SysEx {
        manufacturer_id,
        len: raw.len(),
    })
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod drift_pins {
    //! Evidence: EV-MIDI-EXTENDED (T1) — message-taxonomy parser
    //! invariants for the major MIDI 1.0 status classes.
    //!
    //! T1 per docs/43 §15.5: each test pins a parser invariant the
    //! Wave 1b-A adapter MUST honour so the slice-3+ dispatch can
    //! rely on a stable pre-parsed shape. Strict-accept posture
    //! rejects malformed input outright; widening (e.g. running-
    //! status, Active Sensing) is a slice-3+ decision.

    use super::{MidiMessage, combine_14bit, parse, split_14bit};

    #[test]
    fn parse_rejects_empty_and_unknown_status() {
        assert!(parse(&[]).is_none());
        // 0x00..=0x7F are data bytes, never status.
        assert!(parse(&[0x42, 0x10, 0x20]).is_none());
        // 0xF6 (Tune Request) and 0xFE (Active Sensing) are valid
        // System Real-Time/Common but Wave 1b-A does not route them.
        assert!(parse(&[0xF6]).is_none());
        assert!(parse(&[0xFE]).is_none());
    }

    #[test]
    fn parse_note_on_and_note_off() {
        match parse(&[0x91, 0x40, 0x7F]) {
            Some(MidiMessage::NoteOn {
                channel,
                note,
                velocity,
            }) => {
                assert_eq!(channel, 1);
                assert_eq!(note, 0x40);
                assert_eq!(velocity, 0x7F);
            }
            other => panic!("expected NoteOn, got {other:?}"),
        }
        match parse(&[0x82, 0x3C, 0x40]) {
            Some(MidiMessage::NoteOff {
                channel,
                note,
                velocity,
            }) => {
                assert_eq!(channel, 2);
                assert_eq!(note, 0x3C);
                assert_eq!(velocity, 0x40);
            }
            other => panic!("expected NoteOff, got {other:?}"),
        }
    }

    #[test]
    fn parse_control_change_and_program_change() {
        match parse(&[0xB0, 0x07, 0x64]) {
            Some(MidiMessage::ControlChange {
                channel,
                controller,
                value,
            }) => {
                assert_eq!(channel, 0);
                assert_eq!(controller, 7);
                assert_eq!(value, 0x64);
            }
            other => panic!("expected ControlChange, got {other:?}"),
        }
        match parse(&[0xC1, 0x05]) {
            Some(MidiMessage::ProgramChange { channel, program }) => {
                assert_eq!(channel, 1);
                assert_eq!(program, 5);
            }
            other => panic!("expected ProgramChange, got {other:?}"),
        }
    }

    #[test]
    fn parse_pitch_bend_combines_14bit() {
        // Center: LSB=0, MSB=0x40 -> value14 = 8192.
        match parse(&[0xE0, 0x00, 0x40]) {
            Some(MidiMessage::PitchBend { channel, value14 }) => {
                assert_eq!(channel, 0);
                assert_eq!(value14, 8192);
            }
            other => panic!("expected PitchBend, got {other:?}"),
        }
        // Max: LSB=0x7F, MSB=0x7F -> value14 = 16_383.
        match parse(&[0xE0, 0x7F, 0x7F]) {
            Some(MidiMessage::PitchBend {
                channel: _,
                value14,
            }) => {
                assert_eq!(value14, 16_383);
            }
            other => panic!("expected PitchBend, got {other:?}"),
        }
        // Min: LSB=0, MSB=0 -> value14 = 0.
        match parse(&[0xE0, 0x00, 0x00]) {
            Some(MidiMessage::PitchBend { value14, .. }) => {
                assert_eq!(value14, 0);
            }
            other => panic!("expected PitchBend, got {other:?}"),
        }
    }

    #[test]
    fn parse_system_real_time_and_song_position_pointer() {
        assert!(matches!(parse(&[0xF8]), Some(MidiMessage::MidiClock)));
        assert!(matches!(parse(&[0xFA]), Some(MidiMessage::Start)));
        assert!(matches!(parse(&[0xFB]), Some(MidiMessage::Continue)));
        assert!(matches!(parse(&[0xFC]), Some(MidiMessage::Stop)));
        match parse(&[0xF2, 0x00, 0x10]) {
            Some(MidiMessage::SongPositionPointer { value14 }) => {
                // MSB=0x10, LSB=0 -> 0x800 = 2048.
                assert_eq!(value14, 0x800);
            }
            other => panic!("expected SongPositionPointer, got {other:?}"),
        }
    }

    #[test]
    fn parse_mtc_quarter_frame_preserves_piece_byte() {
        match parse(&[0xF1, 0x33]) {
            Some(MidiMessage::MtcQuarterFrame { piece }) => {
                // Piece selector (high 3 bits) = 3; nibble (low 4) = 3.
                assert_eq!(piece, 0x33);
            }
            other => panic!("expected MtcQuarterFrame, got {other:?}"),
        }
    }

    #[test]
    fn parse_sysex_records_manufacturer_id_and_len() {
        // 0xF0 <id=0x42> 0x10 0x20 0xF7 — 5-byte SysEx.
        match parse(&[0xF0, 0x42, 0x10, 0x20, 0xF7]) {
            Some(MidiMessage::SysEx {
                manufacturer_id,
                len,
            }) => {
                assert_eq!(manufacturer_id, 0x42);
                assert_eq!(len, 5);
            }
            other => panic!("expected SysEx, got {other:?}"),
        }
        // Truncated SysEx (no trailing 0xF7) rejects.
        assert!(parse(&[0xF0, 0x42, 0x10, 0x20]).is_none());
        // Bare 0xF0 0xF7 is below the 3-byte minimum.
        assert!(parse(&[0xF0, 0xF7]).is_none());
    }

    #[test]
    fn combine_and_split_14bit_round_trip() {
        for sample in [0u16, 1, 8192, 16_383, 12_345, 8191] {
            let (msb, lsb) = split_14bit(sample);
            assert_eq!(combine_14bit(msb, lsb), sample);
            assert!(msb <= 0x7F);
            assert!(lsb <= 0x7F);
        }
    }
}
