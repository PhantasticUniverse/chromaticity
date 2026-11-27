// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Crash-safe Fragmented MP4 (fMP4) recorder and rolling replay buffer (Phase 7a).
//!
//! Provides a zero-GPL, pure-Rust ISO/IEC 14496-12 (ISO BMFF) Fragmented MP4
//! serializer ([`Fmp4Muxer`]) paired with an explicit lifecycle state machine
//! ([`RecorderState`]) and a keyframe-aligned in-memory rolling ring buffer
//! ([`ReplayBuffer`]).
//!
//! # Crash Safety
//!
//! Standard MP4 files write their `moov` index at recording stop, so a crash
//! or laptop sleep corrupts the entire file. [`Fmp4Muxer`] writes an init
//! segment (`ftyp` + `moov` with `mvex`/`trex`) upfront, then emits
//! self-contained `moof` (`mfhd` + `traf`/`tfhd`/`tfdt`/`trun`) + `mdat`
//! fragments on every GOP/keyframe boundary. Every flushed fragment is
//! independently playable if power is lost mid-recording.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

/// Standard MPEG / ISO BMFF video clock timescale (90 kHz).
pub const TIMESCALE_90KHZ: u32 = 90_000;

/// Default rolling replay buffer duration in seconds.
pub const DEFAULT_REPLAY_DURATION_SECS: u32 = 60;

/// Minimum allowed rolling replay buffer duration in seconds.
pub const MIN_REPLAY_DURATION_SECS: u32 = 5;

/// Maximum allowed rolling replay buffer duration in seconds.
pub const MAX_REPLAY_DURATION_SECS: u32 = 300;

/// Default rolling replay buffer memory cap (256 MiB).
pub const DEFAULT_REPLAY_MAX_BYTES: usize = 256 * 1024 * 1024;

/// Default sample duration for 60 fps at 90 kHz (`90_000 / 60 = 1_500`).
pub const DEFAULT_SAMPLE_DURATION_90KHZ: u32 = 1_500;

/// ISO BMFF sample flags for a sync / keyframe sample (`sample_depends_on = 2`).
const SAMPLE_FLAGS_KEYFRAME: u32 = 0x0200_0000;

/// ISO BMFF sample flags for a non-sync / inter-predicted sample
/// (`sample_depends_on = 1`, `sample_is_non_sync_sample = 1`).
const SAMPLE_FLAGS_NON_KEYFRAME: u32 = 0x0101_0000;

/// `tfhd` flags: `default-base-is-moof` (`0x02_0000`).
const TFHD_FLAG_DEFAULT_BASE_IS_MOOF: u32 = 0x0002_0000;

/// `trun` flags: `data-offset-present` | `sample-duration-present` |
/// `sample-size-present` | `sample-flags-present` |
/// `sample-composition-time-offsets-present`.
const TRUN_FLAGS_FULL: u32 = 0x0000_0F01;

/// ISO BMFF unity transformation matrix (3x3 fixed-point).
const UNITY_MATRIX: [u32; 9] = [0x0001_0000, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000];

/// Explicit lifecycle state machine for [`Fmp4Muxer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RecorderState {
    /// No active recording session.
    #[default]
    Idle,
    /// Actively ingesting encoded packets and flushing fMP4 fragments.
    Recording {
        /// Monotonic session start timestamp in milliseconds.
        started_ms: u64,
        /// Number of complete `moof` + `mdat` fragments emitted so far.
        fragments_written: u64,
        /// Total serialized bytes emitted so far (init segment + fragments).
        bytes_written: u64,
    },
    /// Stop requested; awaiting final fragment flush before returning to `Idle`.
    Stopping,
}

impl RecorderState {
    /// Returns a stable diagnostic label for the current state.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Recording { .. } => "recording",
            Self::Stopping => "stopping",
        }
    }
}

/// Errors produced by [`Fmp4Muxer`] or ISO BMFF inspection helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecorderError {
    /// An operation was attempted in a [`RecorderState`] that does not permit it.
    InvalidStateTransition {
        /// Label of the current state.
        from: &'static str,
        /// Name of the attempted operation.
        operation: &'static str,
    },
    /// Muxer configuration failed validation.
    InvalidConfig {
        /// Human-readable validation failure reason.
        reason: &'static str,
    },
    /// A packet had an empty payload or exceeded ISO BMFF 32-bit size limits.
    InvalidPacket {
        /// Human-readable validation failure reason.
        reason: &'static str,
    },
    /// Serialized ISO BMFF box or fragment exceeded 32-bit size limits.
    BoxSizeOverflow {
        /// FourCC of the overflowing box.
        box_type: [u8; 4],
    },
    /// A byte slice could not be parsed as a valid sequence of ISO BMFF boxes.
    MalformedBox {
        /// Byte offset where parsing failed.
        offset: usize,
        /// Human-readable diagnostic reason.
        reason: &'static str,
    },
}

impl fmt::Display for RecorderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStateTransition { from, operation } => {
                write!(
                    f,
                    "invalid recorder state transition: cannot `{operation}` from `{from}`"
                )
            }
            Self::InvalidConfig { reason } => write!(f, "invalid fMP4 muxer config: {reason}"),
            Self::InvalidPacket { reason } => write!(f, "invalid encoded packet: {reason}"),
            Self::BoxSizeOverflow { box_type } => {
                let tag = String::from_utf8_lossy(box_type);
                write!(f, "ISO BMFF box `{tag}` exceeded 32-bit size limit")
            }
            Self::MalformedBox { offset, reason } => {
                write!(f, "malformed ISO BMFF box at byte {offset}: {reason}")
            }
        }
    }
}

impl std::error::Error for RecorderError {}

/// Single encoded video access unit fed into [`Fmp4Muxer`] or [`ReplayBuffer`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncodedPacket {
    /// Presentation timestamp in 90 kHz ticks.
    pub pts_90khz: u64,
    /// Decode timestamp in 90 kHz ticks.
    pub dts_90khz: u64,
    /// Sample duration in 90 kHz ticks.
    pub duration_90khz: u32,
    /// `true` when this packet begins a new GOP / IDR random-access point.
    pub is_keyframe: bool,
    /// Compressed bitstream bytes (e.g. AVCC-formatted H.264/HEVC NALUs).
    pub data: Vec<u8>,
}

/// Configuration for [`Fmp4Muxer`] and [`ReplayBuffer::flush_to_fmp4`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fmp4MuxerConfig {
    /// Coded video width in pixels (must be > 0).
    pub width: u16,
    /// Coded video height in pixels (must be > 0).
    pub height: u16,
    /// Media timescale in ticks per second (default: `90_000`).
    pub timescale: u32,
    /// ISO BMFF track identifier (default: `1`, must be > 0).
    pub track_id: u32,
    /// Visual sample-entry FourCC (default: `*b"avc1"`).
    pub codec_fourcc: [u8; 4],
    /// Default sample duration in timescale ticks (default: `1_500` for 60 fps @ 90 kHz).
    pub default_sample_duration: u32,
    /// Optional codec-specific decoder configuration record (e.g. `avcC` body).
    pub codec_config: Vec<u8>,
}

impl Default for Fmp4MuxerConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            timescale: TIMESCALE_90KHZ,
            track_id: 1,
            codec_fourcc: *b"avc1",
            default_sample_duration: DEFAULT_SAMPLE_DURATION_90KHZ,
            codec_config: Vec::new(),
        }
    }
}

impl Fmp4MuxerConfig {
    /// Validates configuration invariants.
    ///
    /// # Errors
    ///
    /// Returns [`RecorderError::InvalidConfig`] when dimensions, timescale,
    /// or `track_id` are zero.
    pub fn validate(&self) -> Result<(), RecorderError> {
        if self.width == 0 || self.height == 0 {
            return Err(RecorderError::InvalidConfig {
                reason: "video width and height must be non-zero",
            });
        }
        if self.timescale == 0 {
            return Err(RecorderError::InvalidConfig {
                reason: "timescale must be non-zero",
            });
        }
        if self.track_id == 0 {
            return Err(RecorderError::InvalidConfig {
                reason: "track_id must be non-zero",
            });
        }
        Ok(())
    }
}

/// Pure-Rust crash-safe Fragmented MP4 (ISO BMFF) muxer.
///
/// Lifecycle:
/// `Idle` --`start(started_ms)`--> `Recording` --`begin_stop()`--> `Stopping`
/// --`finish_stop()`--> `Idle`.
#[derive(Debug, Clone)]
pub struct Fmp4Muxer {
    config: Fmp4MuxerConfig,
    state: RecorderState,
    next_sequence_number: u32,
    pending_gop: Vec<EncodedPacket>,
    dropped_pre_keyframe_packets: u64,
}

impl Fmp4Muxer {
    /// Creates a new muxer in [`RecorderState::Idle`].
    ///
    /// # Errors
    ///
    /// Returns [`RecorderError::InvalidConfig`] if `config` fails validation.
    pub fn new(config: Fmp4MuxerConfig) -> Result<Self, RecorderError> {
        config.validate()?;
        Ok(Self {
            config,
            state: RecorderState::Idle,
            next_sequence_number: 1,
            pending_gop: Vec::new(),
            dropped_pre_keyframe_packets: 0,
        })
    }

    /// Returns the current explicit [`RecorderState`].
    #[must_use]
    pub const fn state(&self) -> RecorderState {
        self.state
    }

    /// Returns the immutable muxer configuration.
    #[must_use]
    pub const fn config(&self) -> &Fmp4MuxerConfig {
        &self.config
    }

    /// Returns the number of leading non-keyframe packets discarded while
    /// waiting for the first keyframe of the current/last session.
    #[must_use]
    pub const fn dropped_pre_keyframe_packets(&self) -> u64 {
        self.dropped_pre_keyframe_packets
    }

    /// Transitions `Idle -> Recording` and returns the serialized fMP4 init
    /// segment (`ftyp` + `moov` containing `mvex`/`trex`).
    ///
    /// # Errors
    ///
    /// Returns [`RecorderError::InvalidStateTransition`] if not currently
    /// [`RecorderState::Idle`].
    pub fn start(&mut self, started_ms: u64) -> Result<Vec<u8>, RecorderError> {
        if !matches!(self.state, RecorderState::Idle) {
            return Err(RecorderError::InvalidStateTransition {
                from: self.state.label(),
                operation: "start",
            });
        }

        let init_segment = build_init_segment(&self.config)?;
        self.next_sequence_number = 1;
        self.pending_gop.clear();
        self.dropped_pre_keyframe_packets = 0;
        self.state = RecorderState::Recording {
            started_ms,
            fragments_written: 0,
            bytes_written: init_segment.len() as u64,
        };
        Ok(init_segment)
    }

    /// Ingests an [`EncodedPacket`].
    ///
    /// Leading non-keyframe packets before the first keyframe are dropped so
    /// every emitted fragment begins cleanly on a keyframe. Whenever a new
    /// keyframe arrives while a prior GOP is buffered, the completed GOP is
    /// serialized as a crash-safe `moof` + `mdat` fragment and returned for
    /// immediate disk flush.
    ///
    /// # Errors
    ///
    /// - [`RecorderError::InvalidStateTransition`] if not in [`RecorderState::Recording`].
    /// - [`RecorderError::InvalidPacket`] if `packet.data` is empty.
    pub fn push_packet(&mut self, packet: EncodedPacket) -> Result<Option<Vec<u8>>, RecorderError> {
        let (started_ms, fragments_written, bytes_written) = match self.state {
            RecorderState::Recording {
                started_ms,
                fragments_written,
                bytes_written,
            } => (started_ms, fragments_written, bytes_written),
            other => {
                return Err(RecorderError::InvalidStateTransition {
                    from: other.label(),
                    operation: "push_packet",
                });
            }
        };

        if packet.data.is_empty() {
            return Err(RecorderError::InvalidPacket {
                reason: "encoded packet payload must not be empty",
            });
        }

        if self.pending_gop.is_empty() && !packet.is_keyframe {
            self.dropped_pre_keyframe_packets = self.dropped_pre_keyframe_packets.saturating_add(1);
            return Ok(None);
        }

        if packet.is_keyframe && !self.pending_gop.is_empty() {
            let seq = self.next_sequence_number;
            let fragment = build_fragment(&self.config, seq, &self.pending_gop, 0)?;
            self.next_sequence_number = seq.saturating_add(1);
            self.pending_gop.clear();
            self.pending_gop.push(packet);
            self.state = RecorderState::Recording {
                started_ms,
                fragments_written: fragments_written.saturating_add(1),
                bytes_written: bytes_written.saturating_add(fragment.len() as u64),
            };
            return Ok(Some(fragment));
        }

        self.pending_gop.push(packet);
        Ok(None)
    }

    /// Transitions `Recording -> Stopping`.
    ///
    /// Call [`Self::finish_stop`] immediately afterward to flush the final
    /// buffered GOP fragment and return to [`RecorderState::Idle`].
    ///
    /// # Errors
    ///
    /// Returns [`RecorderError::InvalidStateTransition`] if not currently
    /// [`RecorderState::Recording`].
    pub fn begin_stop(&mut self) -> Result<(), RecorderError> {
        if !matches!(self.state, RecorderState::Recording { .. }) {
            return Err(RecorderError::InvalidStateTransition {
                from: self.state.label(),
                operation: "begin_stop",
            });
        }
        self.state = RecorderState::Stopping;
        Ok(())
    }

    /// Transitions `Stopping -> Idle`, flushing any remaining buffered GOP as
    /// a final `moof` + `mdat` fragment.
    ///
    /// # Errors
    ///
    /// Returns [`RecorderError::InvalidStateTransition`] if not currently
    /// [`RecorderState::Stopping`].
    pub fn finish_stop(&mut self) -> Result<Option<Vec<u8>>, RecorderError> {
        if !matches!(self.state, RecorderState::Stopping) {
            return Err(RecorderError::InvalidStateTransition {
                from: self.state.label(),
                operation: "finish_stop",
            });
        }

        let final_fragment = if self.pending_gop.is_empty() {
            None
        } else {
            let seq = self.next_sequence_number;
            let fragment = build_fragment(&self.config, seq, &self.pending_gop, 0)?;
            self.next_sequence_number = seq.saturating_add(1);
            self.pending_gop.clear();
            Some(fragment)
        };

        self.state = RecorderState::Idle;
        Ok(final_fragment)
    }
}

/// GOP unit stored inside [`ReplayBuffer`]. Always starts on a keyframe.
#[derive(Debug, Clone)]
struct GopSegment {
    packets: Vec<EncodedPacket>,
    duration_90khz: u64,
    payload_bytes: usize,
}

/// Keyframe-aligned rolling in-memory replay buffer ("save last 15–120 s").
///
/// Stores complete GOPs aligned on keyframe boundaries, evicting the oldest
/// whole GOP whenever the configured duration ceiling (`5..=300` s) or memory
/// ceiling (`max_bytes`) is exceeded. Calling [`Self::flush_to_fmp4`]
/// materializes a standalone, playable Fragmented MP4 starting cleanly on the
/// oldest retained keyframe.
#[derive(Debug, Clone)]
pub struct ReplayBuffer {
    max_duration_secs: u32,
    max_bytes: usize,
    gops: VecDeque<GopSegment>,
    total_duration_90khz: u64,
    total_bytes: usize,
}

impl Default for ReplayBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_REPLAY_DURATION_SECS, DEFAULT_REPLAY_MAX_BYTES)
    }
}

impl ReplayBuffer {
    /// Creates a new rolling replay buffer.
    ///
    /// `max_duration_secs` is clamped to `5..=300` seconds.
    #[must_use]
    pub fn new(max_duration_secs: u32, max_bytes: usize) -> Self {
        Self {
            max_duration_secs: max_duration_secs
                .clamp(MIN_REPLAY_DURATION_SECS, MAX_REPLAY_DURATION_SECS),
            max_bytes,
            gops: VecDeque::new(),
            total_duration_90khz: 0,
            total_bytes: 0,
        }
    }

    /// Returns the clamped maximum duration in seconds (`5..=300`).
    #[must_use]
    pub const fn max_duration_secs(&self) -> u32 {
        self.max_duration_secs
    }

    /// Returns the configured memory cap in bytes.
    #[must_use]
    pub const fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    /// Returns the number of retained GOP segments.
    #[must_use]
    pub fn gop_count(&self) -> usize {
        self.gops.len()
    }

    /// Returns the total number of retained encoded packets across all GOPs.
    #[must_use]
    pub fn packet_count(&self) -> usize {
        self.gops.iter().map(|gop| gop.packets.len()).sum()
    }

    /// Returns total retained payload bytes across all GOPs.
    #[must_use]
    pub const fn retained_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Returns total retained duration in 90 kHz ticks.
    #[must_use]
    pub const fn retained_duration_90khz(&self) -> u64 {
        self.total_duration_90khz
    }

    /// Returns the oldest retained packet, if any (guaranteed to be a keyframe).
    #[must_use]
    pub fn oldest_packet(&self) -> Option<&EncodedPacket> {
        self.gops.front().and_then(|gop| gop.packets.first())
    }

    /// Clears all retained GOPs.
    pub fn clear(&mut self) {
        self.gops.clear();
        self.total_duration_90khz = 0;
        self.total_bytes = 0;
    }

    /// Appends an encoded packet to the rolling buffer, evicting oldest whole
    /// GOPs when duration or byte caps are exceeded.
    ///
    /// Empty packets and leading non-keyframe packets received before any
    /// keyframe are ignored so the buffer always begins cleanly on a keyframe.
    pub fn push_packet(&mut self, pkt: EncodedPacket) {
        if pkt.data.is_empty() {
            return;
        }

        let pkt_bytes = pkt.data.len();
        let pkt_duration = u64::from(pkt.duration_90khz);

        if pkt.is_keyframe {
            self.gops.push_back(GopSegment {
                packets: vec![pkt],
                duration_90khz: pkt_duration,
                payload_bytes: pkt_bytes,
            });
            self.total_duration_90khz = self.total_duration_90khz.saturating_add(pkt_duration);
            self.total_bytes = self.total_bytes.saturating_add(pkt_bytes);
        } else if let Some(current_gop) = self.gops.back_mut() {
            current_gop.packets.push(pkt);
            current_gop.duration_90khz = current_gop.duration_90khz.saturating_add(pkt_duration);
            current_gop.payload_bytes = current_gop.payload_bytes.saturating_add(pkt_bytes);
            self.total_duration_90khz = self.total_duration_90khz.saturating_add(pkt_duration);
            self.total_bytes = self.total_bytes.saturating_add(pkt_bytes);
        } else {
            // Ignore orphan delta frames prior to the first keyframe.
            return;
        }

        self.evict_excess();
    }

    /// Evicts oldest whole GOPs to maintain both duration and memory bounds.
    fn evict_excess(&mut self) {
        let max_duration_90khz =
            u64::from(self.max_duration_secs).saturating_mul(u64::from(TIMESCALE_90KHZ));

        // Evict whole GOPs while duration exceeds cap (keep at least the active
        // GOP if a single GOP is longer than the duration cap, unless memory
        // cap requires dropping it below).
        while self.gops.len() > 1 && self.total_duration_90khz > max_duration_90khz {
            self.pop_oldest_gop();
        }

        // Strictly enforce memory cap at GOP boundaries.
        while self.total_bytes > self.max_bytes {
            if self.gops.is_empty() {
                break;
            }
            self.pop_oldest_gop();
        }
    }

    fn pop_oldest_gop(&mut self) {
        if let Some(evicted) = self.gops.pop_front() {
            self.total_duration_90khz = self
                .total_duration_90khz
                .saturating_sub(evicted.duration_90khz);
            self.total_bytes = self.total_bytes.saturating_sub(evicted.payload_bytes);
        }
    }

    /// Serializes all retained GOPs into a complete, standalone, playable fMP4
    /// byte buffer (`ftyp` + `moov` + one `moof`/`mdat` fragment per retained
    /// GOP), starting cleanly on the oldest retained keyframe with decode
    /// timestamps rebased to zero.
    ///
    /// Returns an empty `Vec` if no keyframes are currently retained or if
    /// `muxer_config` is invalid.
    #[must_use]
    pub fn flush_to_fmp4(&self, muxer_config: &Fmp4MuxerConfig) -> Vec<u8> {
        let Some(first_pkt) = self.oldest_packet() else {
            return Vec::new();
        };
        if muxer_config.validate().is_err() {
            return Vec::new();
        }

        let Ok(mut out) = build_init_segment(muxer_config) else {
            return Vec::new();
        };

        let base_dts = first_pkt.dts_90khz;
        let mut seq: u32 = 1;
        for gop in &self.gops {
            if gop.packets.is_empty() {
                continue;
            }
            let Ok(fragment) = build_fragment(muxer_config, seq, &gop.packets, base_dts) else {
                return Vec::new();
            };
            out.extend_from_slice(&fragment);
            seq = seq.saturating_add(1);
        }

        out
    }
}

/// Parsed header of an ISO BMFF box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsoBoxHeader {
    /// Four-character box type (e.g. `*b"ftyp"`, `*b"moov"`, `*b"moof"`, `*b"mdat"`).
    pub box_type: [u8; 4],
    /// Byte offset of the box within its parent slice.
    pub offset: usize,
    /// Total box size in bytes, including the 8-byte header.
    pub size: u32,
}

/// Parses sequential top-level (or child) ISO BMFF box headers from `bytes`,
/// verifying exact 4-byte big-endian box size integrity with no trailing
/// garbage or truncated boxes.
///
/// # Errors
///
/// Returns [`RecorderError::MalformedBox`] if any box header is truncated,
/// has size `< 8`, or extends past the end of `bytes`.
pub fn parse_box_headers(bytes: &[u8]) -> Result<Vec<IsoBoxHeader>, RecorderError> {
    let mut headers = Vec::new();
    let mut offset = 0usize;

    while offset < bytes.len() {
        let remaining = bytes.len().saturating_sub(offset);
        if remaining < 8 {
            return Err(RecorderError::MalformedBox {
                offset,
                reason: "truncated ISO BMFF box header (< 8 bytes)",
            });
        }

        let size = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        let box_type = [
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ];

        if size < 8 {
            return Err(RecorderError::MalformedBox {
                offset,
                reason: "ISO BMFF box size must be at least 8 bytes",
            });
        }

        let size_usize = size as usize;
        if size_usize > remaining {
            return Err(RecorderError::MalformedBox {
                offset,
                reason: "ISO BMFF box size extends past buffer end",
            });
        }

        headers.push(IsoBoxHeader {
            box_type,
            offset,
            size,
        });
        offset = offset.saturating_add(size_usize);
    }

    Ok(headers)
}

/// Builds a standalone ISO BMFF init segment (`ftyp` + `moov` with `mvex`/`trex`).
///
/// # Errors
///
/// Returns [`RecorderError::InvalidConfig`] if `config` is invalid or
/// [`RecorderError::BoxSizeOverflow`] if a box exceeds 32-bit size limits.
pub fn build_init_segment(config: &Fmp4MuxerConfig) -> Result<Vec<u8>, RecorderError> {
    config.validate()?;
    let mut out = Vec::with_capacity(512);
    append_ftyp(&mut out)?;
    append_moov(&mut out, config)?;
    Ok(out)
}

/// Builds a self-contained ISO BMFF media fragment (`moof` + `mdat`).
///
/// `base_dts_offset` is subtracted (saturating) from the first packet's
/// `dts_90khz` when writing `tfdt` so replay clips can rebase timestamps to 0.
///
/// # Errors
///
/// Returns [`RecorderError::InvalidPacket`] if `packets` is empty or any
/// packet exceeds 32-bit ISO BMFF size limits.
pub fn build_fragment(
    config: &Fmp4MuxerConfig,
    sequence_number: u32,
    packets: &[EncodedPacket],
    base_dts_offset: u64,
) -> Result<Vec<u8>, RecorderError> {
    config.validate()?;
    let Some(first) = packets.first() else {
        return Err(RecorderError::InvalidPacket {
            reason: "cannot build media fragment from empty packet list",
        });
    };

    let sample_count = u32::try_from(packets.len()).map_err(|_| RecorderError::InvalidPacket {
        reason: "fragment sample count exceeds u32::MAX",
    })?;

    let mut total_payload_len: usize = 0;
    for pkt in packets {
        if pkt.data.is_empty() {
            return Err(RecorderError::InvalidPacket {
                reason: "encoded packet payload must not be empty",
            });
        }
        let _ = u32::try_from(pkt.data.len()).map_err(|_| RecorderError::InvalidPacket {
            reason: "encoded packet payload exceeds u32::MAX",
        })?;
        total_payload_len = total_payload_len
            .checked_add(pkt.data.len())
            .ok_or(RecorderError::BoxSizeOverflow { box_type: *b"mdat" })?;
    }

    // Build `mfhd` (16 bytes).
    let mut mfhd = Vec::with_capacity(16);
    write_full_box(&mut mfhd, *b"mfhd", 0, 0, |buf| {
        buf.extend_from_slice(&sequence_number.to_be_bytes());
    })?;

    // Build `tfhd` (16 bytes: 8 header + 4 ver/flags + 4 track_id).
    let mut tfhd = Vec::with_capacity(16);
    write_full_box(
        &mut tfhd,
        *b"tfhd",
        0,
        TFHD_FLAG_DEFAULT_BASE_IS_MOOF,
        |buf| {
            buf.extend_from_slice(&config.track_id.to_be_bytes());
        },
    )?;

    // Build `tfdt` (version 1, 20 bytes: 8 header + 4 ver/flags + 8 base_media_decode_time).
    let base_decode_time = first.dts_90khz.saturating_sub(base_dts_offset);
    let mut tfdt = Vec::with_capacity(20);
    write_full_box(&mut tfdt, *b"tfdt", 1, 0, |buf| {
        buf.extend_from_slice(&base_decode_time.to_be_bytes());
    })?;

    // Compute exact `trun` size upfront so `data_offset` (from start of `moof`
    // to start of `mdat` payload) is exact in a single pass.
    // `trun` = 8 (header) + 4 (ver/flags) + 4 (sample_count) + 4 (data_offset)
    //        + sample_count * 16 (duration + size + flags + cto).
    let trun_entries_len = packets
        .len()
        .checked_mul(16)
        .ok_or(RecorderError::BoxSizeOverflow { box_type: *b"trun" })?;
    let trun_size = 20usize
        .checked_add(trun_entries_len)
        .ok_or(RecorderError::BoxSizeOverflow { box_type: *b"trun" })?;
    let traf_size = 8usize
        .checked_add(tfhd.len())
        .and_then(|v| v.checked_add(tfdt.len()))
        .and_then(|v| v.checked_add(trun_size))
        .ok_or(RecorderError::BoxSizeOverflow { box_type: *b"traf" })?;
    let moof_size = 8usize
        .checked_add(mfhd.len())
        .and_then(|v| v.checked_add(traf_size))
        .ok_or(RecorderError::BoxSizeOverflow { box_type: *b"moof" })?;
    let mdat_payload_offset = moof_size
        .checked_add(8)
        .ok_or(RecorderError::BoxSizeOverflow { box_type: *b"moof" })?;
    let data_offset = i32::try_from(mdat_payload_offset)
        .map_err(|_| RecorderError::BoxSizeOverflow { box_type: *b"trun" })?;

    // Build `trun` (version 1 supports signed composition time offsets).
    let mut trun = Vec::with_capacity(trun_size);
    write_full_box(&mut trun, *b"trun", 1, TRUN_FLAGS_FULL, |buf| {
        buf.extend_from_slice(&sample_count.to_be_bytes());
        buf.extend_from_slice(&data_offset.to_be_bytes());
        for pkt in packets {
            let pkt_size = pkt.data.len() as u32;
            let sample_flags = if pkt.is_keyframe {
                SAMPLE_FLAGS_KEYFRAME
            } else {
                SAMPLE_FLAGS_NON_KEYFRAME
            };
            let cto_i128 = i128::from(pkt.pts_90khz) - i128::from(pkt.dts_90khz);
            let cto = cto_i128.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32;

            buf.extend_from_slice(&pkt.duration_90khz.to_be_bytes());
            buf.extend_from_slice(&pkt_size.to_be_bytes());
            buf.extend_from_slice(&sample_flags.to_be_bytes());
            buf.extend_from_slice(&cto.to_be_bytes());
        }
    })?;

    // Assemble `traf`, `moof`, and `mdat`.
    let mdat_size = 8usize
        .checked_add(total_payload_len)
        .ok_or(RecorderError::BoxSizeOverflow { box_type: *b"mdat" })?;
    let mdat_size_u32 = u32::try_from(mdat_size)
        .map_err(|_| RecorderError::BoxSizeOverflow { box_type: *b"mdat" })?;

    let mut out = Vec::with_capacity(moof_size.saturating_add(mdat_size));
    write_box(&mut out, *b"moof", |moof_buf| {
        moof_buf.extend_from_slice(&mfhd);
        let _ = write_box(moof_buf, *b"traf", |traf_buf| {
            traf_buf.extend_from_slice(&tfhd);
            traf_buf.extend_from_slice(&tfdt);
            traf_buf.extend_from_slice(&trun);
        });
    })?;

    // `mdat` box.
    out.extend_from_slice(&mdat_size_u32.to_be_bytes());
    out.extend_from_slice(b"mdat");
    for pkt in packets {
        out.extend_from_slice(&pkt.data);
    }

    Ok(out)
}

fn append_ftyp(out: &mut Vec<u8>) -> Result<(), RecorderError> {
    write_box(out, *b"ftyp", |buf| {
        buf.extend_from_slice(b"isom"); // major_brand
        buf.extend_from_slice(&0x0000_0200u32.to_be_bytes()); // minor_version
        // compatible_brands
        buf.extend_from_slice(b"isom");
        buf.extend_from_slice(b"iso6");
        buf.extend_from_slice(b"avc1");
        buf.extend_from_slice(b"mp41");
        buf.extend_from_slice(b"dash");
    })
}

fn append_moov(out: &mut Vec<u8>, config: &Fmp4MuxerConfig) -> Result<(), RecorderError> {
    let mut mvhd = Vec::with_capacity(108);
    write_full_box(&mut mvhd, *b"mvhd", 0, 0, |buf| {
        buf.extend_from_slice(&0u32.to_be_bytes()); // creation_time
        buf.extend_from_slice(&0u32.to_be_bytes()); // modification_time
        buf.extend_from_slice(&config.timescale.to_be_bytes());
        buf.extend_from_slice(&0u32.to_be_bytes()); // duration (0 for fragmented)
        buf.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // rate (1.0)
        buf.extend_from_slice(&0x0100u16.to_be_bytes()); // volume (1.0)
        buf.extend_from_slice(&[0u8; 10]); // reserved
        for cell in UNITY_MATRIX {
            buf.extend_from_slice(&cell.to_be_bytes());
        }
        buf.extend_from_slice(&[0u8; 24]); // pre_defined
        buf.extend_from_slice(&config.track_id.saturating_add(1).to_be_bytes());
    })?;

    let mut trak = Vec::with_capacity(320);
    append_trak(&mut trak, config)?;

    let mut mvex = Vec::with_capacity(40);
    append_mvex(&mut mvex, config)?;

    write_box(out, *b"moov", |buf| {
        buf.extend_from_slice(&mvhd);
        buf.extend_from_slice(&trak);
        buf.extend_from_slice(&mvex);
    })
}

fn append_trak(out: &mut Vec<u8>, config: &Fmp4MuxerConfig) -> Result<(), RecorderError> {
    let mut tkhd = Vec::with_capacity(92);
    // flags = 0x000003 (track_enabled | track_in_movie)
    write_full_box(&mut tkhd, *b"tkhd", 0, 0x0000_0003, |buf| {
        buf.extend_from_slice(&0u32.to_be_bytes()); // creation_time
        buf.extend_from_slice(&0u32.to_be_bytes()); // modification_time
        buf.extend_from_slice(&config.track_id.to_be_bytes());
        buf.extend_from_slice(&0u32.to_be_bytes()); // reserved
        buf.extend_from_slice(&0u32.to_be_bytes()); // duration (0 for fragmented)
        buf.extend_from_slice(&[0u8; 8]); // reserved
        buf.extend_from_slice(&0u16.to_be_bytes()); // layer
        buf.extend_from_slice(&0u16.to_be_bytes()); // alternate_group
        buf.extend_from_slice(&0u16.to_be_bytes()); // volume (0 for video)
        buf.extend_from_slice(&0u16.to_be_bytes()); // reserved
        for cell in UNITY_MATRIX {
            buf.extend_from_slice(&cell.to_be_bytes());
        }
        buf.extend_from_slice(&(u32::from(config.width) << 16).to_be_bytes());
        buf.extend_from_slice(&(u32::from(config.height) << 16).to_be_bytes());
    })?;

    let mut mdia = Vec::with_capacity(240);
    append_mdia(&mut mdia, config)?;

    write_box(out, *b"trak", |buf| {
        buf.extend_from_slice(&tkhd);
        buf.extend_from_slice(&mdia);
    })
}

fn append_mdia(out: &mut Vec<u8>, config: &Fmp4MuxerConfig) -> Result<(), RecorderError> {
    let mut mdhd = Vec::with_capacity(32);
    write_full_box(&mut mdhd, *b"mdhd", 0, 0, |buf| {
        buf.extend_from_slice(&0u32.to_be_bytes()); // creation_time
        buf.extend_from_slice(&0u32.to_be_bytes()); // modification_time
        buf.extend_from_slice(&config.timescale.to_be_bytes());
        buf.extend_from_slice(&0u32.to_be_bytes()); // duration
        buf.extend_from_slice(&0x55C4u16.to_be_bytes()); // language: "und"
        buf.extend_from_slice(&0u16.to_be_bytes()); // pre_defined
    })?;

    let mut hdlr = Vec::with_capacity(50);
    write_full_box(&mut hdlr, *b"hdlr", 0, 0, |buf| {
        buf.extend_from_slice(&0u32.to_be_bytes()); // pre_defined
        buf.extend_from_slice(b"vide"); // handler_type
        buf.extend_from_slice(&[0u8; 12]); // reserved
        buf.extend_from_slice(b"ChromaticityVideo\0");
    })?;

    let mut minf = Vec::with_capacity(180);
    append_minf(&mut minf, config)?;

    write_box(out, *b"mdia", |buf| {
        buf.extend_from_slice(&mdhd);
        buf.extend_from_slice(&hdlr);
        buf.extend_from_slice(&minf);
    })
}

fn append_minf(out: &mut Vec<u8>, config: &Fmp4MuxerConfig) -> Result<(), RecorderError> {
    let mut vmhd = Vec::with_capacity(20);
    write_full_box(&mut vmhd, *b"vmhd", 0, 1, |buf| {
        buf.extend_from_slice(&0u16.to_be_bytes()); // graphicsmode
        buf.extend_from_slice(&[0u8; 6]); // opcolor
    })?;

    let mut url_box = Vec::with_capacity(12);
    write_full_box(&mut url_box, *b"url ", 0, 1, |_| {})?;

    let mut dref = Vec::with_capacity(28);
    write_full_box(&mut dref, *b"dref", 0, 0, |buf| {
        buf.extend_from_slice(&1u32.to_be_bytes()); // entry_count
        buf.extend_from_slice(&url_box);
    })?;

    let mut dinf = Vec::with_capacity(36);
    write_box(&mut dinf, *b"dinf", |buf| {
        buf.extend_from_slice(&dref);
    })?;

    let mut stbl = Vec::with_capacity(160);
    append_stbl(&mut stbl, config)?;

    write_box(out, *b"minf", |buf| {
        buf.extend_from_slice(&vmhd);
        buf.extend_from_slice(&dinf);
        buf.extend_from_slice(&stbl);
    })
}

fn append_stbl(out: &mut Vec<u8>, config: &Fmp4MuxerConfig) -> Result<(), RecorderError> {
    let mut sample_entry = Vec::with_capacity(96 + config.codec_config.len());
    write_box(&mut sample_entry, config.codec_fourcc, |buf| {
        buf.extend_from_slice(&[0u8; 6]); // reserved
        buf.extend_from_slice(&1u16.to_be_bytes()); // data_reference_index
        buf.extend_from_slice(&0u16.to_be_bytes()); // pre_defined
        buf.extend_from_slice(&0u16.to_be_bytes()); // reserved
        buf.extend_from_slice(&[0u8; 12]); // pre_defined
        buf.extend_from_slice(&config.width.to_be_bytes());
        buf.extend_from_slice(&config.height.to_be_bytes());
        buf.extend_from_slice(&0x0048_0000u32.to_be_bytes()); // horizresolution (72 dpi)
        buf.extend_from_slice(&0x0048_0000u32.to_be_bytes()); // vertresolution (72 dpi)
        buf.extend_from_slice(&0u32.to_be_bytes()); // reserved
        buf.extend_from_slice(&1u16.to_be_bytes()); // frame_count
        buf.extend_from_slice(&[0u8; 32]); // compressorname
        buf.extend_from_slice(&0x0018u16.to_be_bytes()); // depth (24-bit)
        buf.extend_from_slice(&0xFFFFu16.to_be_bytes()); // pre_defined (-1)
        if !config.codec_config.is_empty() {
            let _ = write_box(buf, *b"avcC", |cfg_buf| {
                cfg_buf.extend_from_slice(&config.codec_config);
            });
        }
    })?;

    let mut stsd = Vec::with_capacity(16 + sample_entry.len());
    write_full_box(&mut stsd, *b"stsd", 0, 0, |buf| {
        buf.extend_from_slice(&1u32.to_be_bytes()); // entry_count
        buf.extend_from_slice(&sample_entry);
    })?;

    let mut stts = Vec::with_capacity(16);
    write_full_box(&mut stts, *b"stts", 0, 0, |buf| {
        buf.extend_from_slice(&0u32.to_be_bytes());
    })?;

    let mut stsc = Vec::with_capacity(16);
    write_full_box(&mut stsc, *b"stsc", 0, 0, |buf| {
        buf.extend_from_slice(&0u32.to_be_bytes());
    })?;

    let mut stsz = Vec::with_capacity(20);
    write_full_box(&mut stsz, *b"stsz", 0, 0, |buf| {
        buf.extend_from_slice(&0u32.to_be_bytes()); // sample_size
        buf.extend_from_slice(&0u32.to_be_bytes()); // sample_count
    })?;

    let mut stco = Vec::with_capacity(16);
    write_full_box(&mut stco, *b"stco", 0, 0, |buf| {
        buf.extend_from_slice(&0u32.to_be_bytes());
    })?;

    write_box(out, *b"stbl", |buf| {
        buf.extend_from_slice(&stsd);
        buf.extend_from_slice(&stts);
        buf.extend_from_slice(&stsc);
        buf.extend_from_slice(&stsz);
        buf.extend_from_slice(&stco);
    })
}

fn append_mvex(out: &mut Vec<u8>, config: &Fmp4MuxerConfig) -> Result<(), RecorderError> {
    let mut trex = Vec::with_capacity(32);
    write_full_box(&mut trex, *b"trex", 0, 0, |buf| {
        buf.extend_from_slice(&config.track_id.to_be_bytes());
        buf.extend_from_slice(&1u32.to_be_bytes()); // default_sample_description_index
        buf.extend_from_slice(&config.default_sample_duration.to_be_bytes());
        buf.extend_from_slice(&0u32.to_be_bytes()); // default_sample_size
        buf.extend_from_slice(&SAMPLE_FLAGS_NON_KEYFRAME.to_be_bytes());
    })?;

    write_box(out, *b"mvex", |buf| {
        buf.extend_from_slice(&trex);
    })
}

/// Writes an ISO BMFF box with an exact 4-byte big-endian size header.
fn write_box(
    out: &mut Vec<u8>,
    box_type: [u8; 4],
    write_body: impl FnOnce(&mut Vec<u8>),
) -> Result<(), RecorderError> {
    let start = out.len();
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&box_type);
    write_body(out);
    let len = out.len().saturating_sub(start);
    let size = u32::try_from(len).map_err(|_| RecorderError::BoxSizeOverflow { box_type })?;
    let size_bytes = size.to_be_bytes();
    out[start..start + 4].copy_from_slice(&size_bytes);
    Ok(())
}

/// Writes an ISO BMFF FullBox (`version` + 24-bit `flags`) with an exact
/// 4-byte big-endian size header.
fn write_full_box(
    out: &mut Vec<u8>,
    box_type: [u8; 4],
    version: u8,
    flags: u32,
    write_body: impl FnOnce(&mut Vec<u8>),
) -> Result<(), RecorderError> {
    write_box(out, box_type, |buf| {
        let ver_flags = (u32::from(version) << 24) | (flags & 0x00FF_FFFF);
        buf.extend_from_slice(&ver_flags.to_be_bytes());
        write_body(buf);
    })
}

#[cfg(test)]
mod tests {
    //! Evidence: EV-DETERMINISM (T1) — Deterministic fMP4 box serialization,
    //! explicit recorder state transitions, and keyframe-aligned replay buffer
    //! eviction invariants.

    use super::*;

    fn make_packet(
        dts_90khz: u64,
        pts_offset: i64,
        duration_90khz: u32,
        is_keyframe: bool,
        payload_len: usize,
        fill_byte: u8,
    ) -> EncodedPacket {
        let pts_90khz = if pts_offset >= 0 {
            dts_90khz.saturating_add(pts_offset as u64)
        } else {
            dts_90khz.saturating_sub(pts_offset.unsigned_abs())
        };
        EncodedPacket {
            pts_90khz,
            dts_90khz,
            duration_90khz,
            is_keyframe,
            data: vec![fill_byte; payload_len],
        }
    }

    /// Helper to extract child box slices from a container box.
    fn child_boxes(bytes: &[u8], parent: IsoBoxHeader) -> Result<Vec<IsoBoxHeader>, RecorderError> {
        let start = parent.offset + 8;
        let end = parent.offset + parent.size as usize;
        parse_box_headers(&bytes[start..end])
    }

    #[test]
    fn state_machine_lifecycle_and_double_start_rejection() -> Result<(), RecorderError> {
        let mut muxer = Fmp4Muxer::new(Fmp4MuxerConfig::default())?;
        assert_eq!(muxer.state(), RecorderState::Idle);

        // Cannot stop or push packets from Idle.
        assert!(matches!(
            muxer.begin_stop(),
            Err(RecorderError::InvalidStateTransition {
                from: "idle",
                operation: "begin_stop",
            })
        ));
        assert!(matches!(
            muxer.finish_stop(),
            Err(RecorderError::InvalidStateTransition {
                from: "idle",
                operation: "finish_stop",
            })
        ));
        assert!(matches!(
            muxer.push_packet(make_packet(0, 0, 1500, true, 16, 0xAA)),
            Err(RecorderError::InvalidStateTransition {
                from: "idle",
                operation: "push_packet",
            })
        ));

        // Idle -> Recording.
        let init = muxer.start(1_000)?;
        assert!(!init.is_empty());
        assert_eq!(
            muxer.state(),
            RecorderState::Recording {
                started_ms: 1_000,
                fragments_written: 0,
                bytes_written: init.len() as u64,
            }
        );

        // Reject invalid double-start while Recording.
        assert!(matches!(
            muxer.start(2_000),
            Err(RecorderError::InvalidStateTransition {
                from: "recording",
                operation: "start",
            })
        ));

        // Cannot finish_stop directly from Recording.
        assert!(matches!(
            muxer.finish_stop(),
            Err(RecorderError::InvalidStateTransition {
                from: "recording",
                operation: "finish_stop",
            })
        ));

        // Push GOP 1 + keyframe of GOP 2 -> emits fragment 1.
        assert!(
            muxer
                .push_packet(make_packet(0, 0, 1500, true, 32, 0x11))?
                .is_none()
        );
        assert!(
            muxer
                .push_packet(make_packet(1500, 0, 1500, false, 16, 0x12))?
                .is_none()
        );
        let frag1 = muxer.push_packet(make_packet(3000, 0, 1500, true, 24, 0x21))?;
        let Some(frag1_bytes) = frag1 else {
            panic!("expected fragment flush on second keyframe");
        };
        assert_eq!(
            muxer.state(),
            RecorderState::Recording {
                started_ms: 1_000,
                fragments_written: 1,
                bytes_written: (init.len() + frag1_bytes.len()) as u64,
            }
        );

        // Recording -> Stopping.
        muxer.begin_stop()?;
        assert_eq!(muxer.state(), RecorderState::Stopping);

        // Reject double-start and double-stop from Stopping.
        assert!(matches!(
            muxer.start(3_000),
            Err(RecorderError::InvalidStateTransition {
                from: "stopping",
                operation: "start",
            })
        ));
        assert!(matches!(
            muxer.begin_stop(),
            Err(RecorderError::InvalidStateTransition {
                from: "stopping",
                operation: "begin_stop",
            })
        ));

        // Stopping -> Idle (flushes remaining GOP 2).
        let final_frag = muxer.finish_stop()?;
        assert!(final_frag.is_some());
        assert_eq!(muxer.state(), RecorderState::Idle);

        Ok(())
    }

    #[test]
    fn fmp4_box_headers_and_multi_fragment_size_integrity() -> Result<(), RecorderError> {
        let config = Fmp4MuxerConfig {
            codec_config: vec![0x01, 0x64, 0x00, 0x2A, 0xFF],
            ..Default::default()
        };
        let mut muxer = Fmp4Muxer::new(config)?;

        let mut stream = muxer.start(500)?;

        // Leading non-keyframe before first keyframe is safely dropped.
        assert!(
            muxer
                .push_packet(make_packet(0, 0, 1500, false, 12, 0x00))?
                .is_none()
        );
        assert_eq!(muxer.dropped_pre_keyframe_packets(), 1);

        // GOP 1 (2 packets, including B-frame style PTS offset).
        assert!(
            muxer
                .push_packet(make_packet(90_000, 3_000, 1500, true, 64, 0xA1))?
                .is_none()
        );
        assert!(
            muxer
                .push_packet(make_packet(91_500, -1_500, 1500, false, 20, 0xA2))?
                .is_none()
        );

        // GOP 2 keyframe flushes GOP 1.
        let Some(frag1) = muxer.push_packet(make_packet(93_000, 0, 1500, true, 48, 0xB1))? else {
            panic!("expected fragment 1");
        };
        stream.extend_from_slice(&frag1);

        // GOP 3 keyframe flushes GOP 2.
        let Some(frag2) = muxer.push_packet(make_packet(94_500, 0, 1500, true, 40, 0xC1))? else {
            panic!("expected fragment 2");
        };
        stream.extend_from_slice(&frag2);

        muxer.begin_stop()?;
        let Some(frag3) = muxer.finish_stop()? else {
            panic!("expected final fragment 3");
        };
        stream.extend_from_slice(&frag3);

        // Verify exact top-level ISO BMFF box sequence and 4-byte big-endian sizes.
        let boxes = parse_box_headers(&stream)?;
        let types: Vec<[u8; 4]> = boxes.iter().map(|b| b.box_type).collect();
        assert_eq!(
            types,
            vec![
                *b"ftyp", *b"moov", *b"moof", *b"mdat", *b"moof", *b"mdat", *b"moof", *b"mdat",
            ]
        );

        let sum_sizes: usize = boxes.iter().map(|b| b.size as usize).sum();
        assert_eq!(sum_sizes, stream.len());

        // Verify `moov` contains `mvhd`, `trak`, and `mvex` -> `trex`.
        let moov_children = child_boxes(&stream, boxes[1])?;
        let moov_types: Vec<[u8; 4]> = moov_children.iter().map(|b| b.box_type).collect();
        assert_eq!(moov_types, vec![*b"mvhd", *b"trak", *b"mvex"]);

        let mvex_children = child_boxes(
            &stream[boxes[1].offset + 8..boxes[1].offset + boxes[1].size as usize],
            moov_children[2],
        )?;
        assert_eq!(mvex_children.len(), 1);
        assert_eq!(mvex_children[0].box_type, *b"trex");

        // Verify every `moof` contains `mfhd` + `traf` (`tfhd`, `tfdt`, `trun`)
        // and that `trun.data_offset` points exactly to the matching `mdat` payload.
        for (frag_idx, pair) in boxes[2..].chunks_exact(2).enumerate() {
            let moof = pair[0];
            let mdat = pair[1];
            assert_eq!(moof.box_type, *b"moof");
            assert_eq!(mdat.box_type, *b"mdat");

            let moof_slice = &stream[moof.offset..moof.offset + moof.size as usize];
            let moof_children = parse_box_headers(&moof_slice[8..])?;
            let moof_child_types: Vec<[u8; 4]> = moof_children.iter().map(|b| b.box_type).collect();
            assert_eq!(moof_child_types, vec![*b"mfhd", *b"traf"]);

            // Check monotonic 1-based sequence_number inside `mfhd`.
            let mfhd_start = 8 + moof_children[0].offset;
            let seq = u32::from_be_bytes([
                moof_slice[mfhd_start + 12],
                moof_slice[mfhd_start + 13],
                moof_slice[mfhd_start + 14],
                moof_slice[mfhd_start + 15],
            ]);
            assert_eq!(seq, (frag_idx as u32) + 1);

            // Check `traf` children: `tfhd`, `tfdt`, `trun`.
            let traf_start = 8 + moof_children[1].offset;
            let traf_end = traf_start + moof_children[1].size as usize;
            let traf_children = parse_box_headers(&moof_slice[traf_start + 8..traf_end])?;
            let traf_types: Vec<[u8; 4]> = traf_children.iter().map(|b| b.box_type).collect();
            assert_eq!(traf_types, vec![*b"tfhd", *b"tfdt", *b"trun"]);

            // Check `trun.data_offset == moof.size + 8` (exact start of `mdat` payload).
            let trun_start = traf_start + 8 + traf_children[2].offset;
            let data_offset = i32::from_be_bytes([
                moof_slice[trun_start + 16],
                moof_slice[trun_start + 17],
                moof_slice[trun_start + 18],
                moof_slice[trun_start + 19],
            ]);
            assert_eq!(data_offset, (moof.size + 8) as i32);
        }

        // Crash-safety check: truncating after fragment 1 still yields a
        // structurally valid, playable fMP4 (`ftyp` + `moov` + `moof` + `mdat`).
        let truncated_end = boxes[3].offset + boxes[3].size as usize;
        let truncated_boxes = parse_box_headers(&stream[..truncated_end])?;
        let truncated_types: Vec<[u8; 4]> = truncated_boxes.iter().map(|b| b.box_type).collect();
        assert_eq!(
            truncated_types,
            vec![*b"ftyp", *b"moov", *b"moof", *b"mdat"]
        );

        Ok(())
    }

    #[test]
    fn replay_buffer_clamps_duration_and_evicts_at_gop_boundaries() -> Result<(), RecorderError> {
        // Duration clamping: 1 -> 5, 999 -> 300.
        assert_eq!(
            ReplayBuffer::new(1, DEFAULT_REPLAY_MAX_BYTES).max_duration_secs(),
            MIN_REPLAY_DURATION_SECS
        );
        assert_eq!(
            ReplayBuffer::new(999, DEFAULT_REPLAY_MAX_BYTES).max_duration_secs(),
            MAX_REPLAY_DURATION_SECS
        );
        assert_eq!(
            ReplayBuffer::default().max_duration_secs(),
            DEFAULT_REPLAY_DURATION_SECS
        );

        // 5-second rolling buffer, 2-second GOPs (180_000 ticks each).
        let mut buf = ReplayBuffer::new(5, 1024 * 1024);

        // Leading orphan delta frame is ignored.
        buf.push_packet(make_packet(0, 0, 90_000, false, 100, 0x01));
        assert_eq!(buf.gop_count(), 0);
        assert!(buf.flush_to_fmp4(&Fmp4MuxerConfig::default()).is_empty());

        // Push 4 two-second GOPs (total 8 seconds).
        for gop_idx in 0u64..4 {
            let base_dts = gop_idx * 180_000;
            buf.push_packet(make_packet(base_dts, 0, 90_000, true, 128, gop_idx as u8));
            buf.push_packet(make_packet(
                base_dts + 90_000,
                0,
                90_000,
                false,
                64,
                gop_idx as u8,
            ));
        }

        // Only the last 2 GOPs (4 seconds <= 5 seconds) should remain.
        assert_eq!(buf.gop_count(), 2);
        assert_eq!(buf.packet_count(), 4);
        assert_eq!(buf.retained_duration_90khz(), 360_000);

        let Some(oldest) = buf.oldest_packet() else {
            panic!("expected retained keyframe");
        };
        assert!(oldest.is_keyframe);
        assert_eq!(oldest.dts_90khz, 2 * 180_000);

        // Flush to standalone fMP4 and validate ISO BMFF box integrity.
        let fmp4 = buf.flush_to_fmp4(&Fmp4MuxerConfig::default());
        let boxes = parse_box_headers(&fmp4)?;
        let types: Vec<[u8; 4]> = boxes.iter().map(|b| b.box_type).collect();
        assert_eq!(
            types,
            vec![*b"ftyp", *b"moov", *b"moof", *b"mdat", *b"moof", *b"mdat"]
        );

        // Verify oldest retained fragment's `tfdt` is rebased cleanly to 0.
        let first_moof = &fmp4[boxes[2].offset..boxes[2].offset + boxes[2].size as usize];
        let moof_children = parse_box_headers(&first_moof[8..])?;
        let traf_start = 8 + moof_children[1].offset;
        let traf_end = traf_start + moof_children[1].size as usize;
        let traf_children = parse_box_headers(&first_moof[traf_start + 8..traf_end])?;
        let tfdt_start = traf_start + 8 + traf_children[1].offset;
        let base_decode_time = u64::from_be_bytes([
            first_moof[tfdt_start + 12],
            first_moof[tfdt_start + 13],
            first_moof[tfdt_start + 14],
            first_moof[tfdt_start + 15],
            first_moof[tfdt_start + 16],
            first_moof[tfdt_start + 17],
            first_moof[tfdt_start + 18],
            first_moof[tfdt_start + 19],
        ]);
        assert_eq!(base_decode_time, 0);

        Ok(())
    }

    #[test]
    fn replay_buffer_enforces_byte_cap_at_gop_boundaries() -> Result<(), RecorderError> {
        // Memory cap of 250 bytes; each GOP is 100 bytes (60 keyframe + 40 delta).
        let mut buf = ReplayBuffer::new(60, 250);

        for gop_idx in 0u64..5 {
            let base_dts = gop_idx * 90_000;
            buf.push_packet(make_packet(base_dts, 0, 45_000, true, 60, gop_idx as u8));
            buf.push_packet(make_packet(
                base_dts + 45_000,
                0,
                45_000,
                false,
                40,
                gop_idx as u8,
            ));
        }

        // At most 2 GOPs (200 bytes <= 250 bytes) fit; oldest retained is GOP 3.
        assert_eq!(buf.gop_count(), 2);
        assert_eq!(buf.retained_bytes(), 200);
        let Some(oldest) = buf.oldest_packet() else {
            panic!("expected oldest packet");
        };
        assert!(oldest.is_keyframe);
        assert_eq!(oldest.dts_90khz, 3 * 90_000);

        let fmp4 = buf.flush_to_fmp4(&Fmp4MuxerConfig::default());
        let boxes = parse_box_headers(&fmp4)?;
        assert_eq!(boxes.len(), 6); // ftyp + moov + 2 * (moof + mdat)
        Ok(())
    }

    #[test]
    fn deterministic_fmp4_output_and_config_validation() -> Result<(), RecorderError> {
        // Reject invalid configs.
        let bad_dim = Fmp4MuxerConfig {
            width: 0,
            ..Default::default()
        };
        assert!(matches!(
            Fmp4Muxer::new(bad_dim),
            Err(RecorderError::InvalidConfig { .. })
        ));

        let bad_scale = Fmp4MuxerConfig {
            timescale: 0,
            ..Default::default()
        };
        assert!(matches!(
            Fmp4Muxer::new(bad_scale),
            Err(RecorderError::InvalidConfig { .. })
        ));

        // Identical inputs produce byte-identical fMP4 output.
        let mut buf_a = ReplayBuffer::new(30, 64 * 1024);
        let mut buf_b = ReplayBuffer::new(30, 64 * 1024);
        let packets = vec![
            make_packet(0, 0, 1500, true, 32, 0x5A),
            make_packet(1500, 0, 1500, false, 16, 0x5B),
            make_packet(3000, 0, 1500, true, 32, 0x5C),
        ];
        for pkt in &packets {
            buf_a.push_packet(pkt.clone());
            buf_b.push_packet(pkt.clone());
        }

        let cfg = Fmp4MuxerConfig::default();
        assert_eq!(buf_a.flush_to_fmp4(&cfg), buf_b.flush_to_fmp4(&cfg));
        Ok(())
    }
}
