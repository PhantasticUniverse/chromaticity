// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! DMX512-A / Art-Net 4 / sACN (ANSI E1.31) stage lighting synchronization.
//!
//! Drives venue stage fixtures and LED walls directly from Chromaticity's
//! spectral energy bands and beat detector so visuals and stage lighting
//! breathe as one unified instrument:
//!
//! 1. [`DmxUniverse`] — 512-channel DMX512-A frame with safe 1-based channel
//!    addressing (`1..=512`) and instant [`DmxUniverse::blackout`].
//! 2. **Art-Net 4 `OpDmx` (`0x5000`) codec** — [`encode_artnet_dmx`] emits an
//!    exact 530-byte UDP datagram (18-byte header + 512 DMX slots) and
//!    [`decode_artnet_dmx`] performs bounds-checked validation and decoding.
//! 3. **sACN / ANSI E1.31-2018 Data Packet encoder** — [`encode_sacn_dmx`]
//!    emits a standards-compliant 638-byte E1.31 Data Packet (ACN Root Layer,
//!    Framing Layer, and DMP Layer with start code `0x00` + 512 slots).
//! 4. **Spectral-to-DMX fixture mapping** — [`SpectralDmxMapper`],
//!    [`DmxFixture`], [`FixtureKind`], [`SpectralBandSource`], and
//!    [`map_audio_to_universe`] map normalized bass/mid/high/beat signals onto
//!    dimmer, RGB, RGBW, and strobe fixtures with master dimmer scaling,
//!    blackout override, and strict 512-channel bounds safety.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Number of DMX512-A slots in a single universe.
pub const DMX_CHANNEL_COUNT: usize = 512;

/// Standard UDP port for Art-Net 4 (`0x1936`).
pub const ARTNET_PORT: u16 = 6454;

/// 8-byte null-terminated ASCII identifier for all Art-Net packets.
pub const ARTNET_MAGIC: &[u8; 8] = b"Art-Net\0";

/// Art-Net `OpDmx` (`OpOutput`) opcode (`0x5000`, wire little-endian).
pub const ARTNET_OPCODE_DMX: u16 = 0x5000;

/// Art-Net 4 protocol revision (`14`, wire big-endian).
pub const ARTNET_PROTOCOL_VERSION: u16 = 14;

/// Byte length of the Art-Net 4 `OpDmx` header before DMX payload bytes.
pub const ARTNET_DMX_HEADER_LEN: usize = 18;

/// Exact byte length of a full 512-channel Art-Net 4 `OpDmx` packet (`18 + 512`).
pub const ARTNET_DMX_PACKET_LEN: usize = ARTNET_DMX_HEADER_LEN + DMX_CHANNEL_COUNT;

/// Standard UDP port for sACN / ANSI E1.31 (`0x15C0`).
pub const SACN_PORT: u16 = 5568;

/// 12-byte ACN Packet Identifier (`ASC-E1.17\0\0\0`).
pub const SACN_ACN_PACKET_ID: &[u8; 12] = b"ASC-E1.17\0\0\0";

/// ANSI E1.31 Root Layer vector for E1.31 Data Packets (`0x0000_0004`).
pub const SACN_VECTOR_ROOT_E131_DATA: u32 = 0x0000_0004;

/// ANSI E1.31 Framing Layer vector for E1.31 Data Packets (`0x0000_0002`).
pub const SACN_VECTOR_E131_DATA_PACKET: u32 = 0x0000_0002;

/// ANSI E1.31 DMP Layer vector (`VECTOR_DMP_SET_PROPERTY` = `0x02`).
pub const SACN_VECTOR_DMP_SET_PROPERTY: u8 = 0x02;

/// Exact byte length of a full 512-channel sACN / ANSI E1.31 Data Packet.
///
/// Root Layer (38) + Framing Layer (77) + DMP Layer (11 + 512) = 638 bytes.
pub const SACN_DMX_PACKET_LEN: usize = 638;

/// Maximum valid sACN priority per ANSI E1.31 (`0..=200`).
pub const SACN_MAX_PRIORITY: u8 = 200;

/// Maximum byte length of the null-terminated sACN source name field.
const SACN_SOURCE_NAME_LEN: usize = 64;

/// Flags nibble (`0x7000`) ORed with the 12-bit PDU length in E1.31 layers.
const SACN_PDU_FLAGS: u16 = 0x7000;

/// DMX512-A null start code (`0x00`) for standard dimmer/fixture slot data.
pub const DMX_NULL_START_CODE: u8 = 0x00;

/// A single 512-channel DMX512-A universe frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DmxUniverse {
    channels: [u8; DMX_CHANNEL_COUNT],
}

impl Default for DmxUniverse {
    fn default() -> Self {
        Self::new()
    }
}

impl DmxUniverse {
    /// Creates a zeroed 512-channel DMX universe.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            channels: [0u8; DMX_CHANNEL_COUNT],
        }
    }

    /// Creates a DMX universe initialized from an existing 512-byte array.
    #[must_use]
    pub const fn from_channels(channels: [u8; DMX_CHANNEL_COUNT]) -> Self {
        Self { channels }
    }

    /// Sets a single DMX channel using 1-based addressing (`1..=512`).
    ///
    /// Returns `true` if `channel` is within `1..=512` and updated, or `false`
    /// if out of bounds (never panics or overflows).
    pub fn set_channel(&mut self, channel: u16, value: u8) -> bool {
        if let Some(index) = channel_to_index(channel) {
            self.channels[index] = value;
            true
        } else {
            false
        }
    }

    /// Reads a single DMX channel using 1-based addressing (`1..=512`).
    ///
    /// Returns `None` if `channel` is `0` or greater than `512`.
    #[must_use]
    pub fn get_channel(&self, channel: u16) -> Option<u8> {
        channel_to_index(channel).map(|index| self.channels[index])
    }

    /// Forces all 512 DMX channels in this universe to `0`.
    pub fn blackout(&mut self) {
        self.channels.fill(0);
    }

    /// Returns a read-only reference to the raw 512-channel array.
    #[must_use]
    pub const fn channels(&self) -> &[u8; DMX_CHANNEL_COUNT] {
        &self.channels
    }

    /// Returns a mutable reference to the raw 512-channel array.
    #[must_use]
    pub fn channels_mut(&mut self) -> &mut [u8; DMX_CHANNEL_COUNT] {
        &mut self.channels
    }
}

/// Converts a 1-based DMX channel (`1..=512`) into a 0-based array index.
#[inline]
const fn channel_to_index(channel: u16) -> Option<usize> {
    if channel >= 1 && (channel as usize) <= DMX_CHANNEL_COUNT {
        Some((channel - 1) as usize)
    } else {
        None
    }
}

/// Decoded Art-Net 4 `OpDmx` (`0x5000`) packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtNetDmxPacket {
    /// Art-Net protocol version decoded from bytes `10..12` (big-endian).
    pub protocol_version: u16,
    /// Packet sequence counter (`0` disables sequence checking per Art-Net 4).
    pub sequence: u8,
    /// Physical input port index (`0` for software-originated frames).
    pub physical: u8,
    /// 15-bit Port-Address (`SubUni` low byte + 7-bit `Net` high byte).
    pub universe: u16,
    /// Number of DMX data bytes declared in the packet header (`2..=512`).
    pub data_len: u16,
    /// 512-channel DMX payload (bytes beyond `data_len` are zero-padded).
    pub channels: [u8; DMX_CHANNEL_COUNT],
}

/// Encodes a full 512-channel Art-Net 4 `OpDmx` (`0x5000`) UDP packet.
///
/// The returned buffer is always exactly [`ARTNET_DMX_PACKET_LEN`] (530) bytes:
/// - Bytes `0..8`: `"Art-Net\0"`
/// - Bytes `8..10`: `OpDmx` opcode (`0x5000`, little-endian)
/// - Bytes `10..12`: Protocol version (`14`, big-endian)
/// - Byte `12`: `sequence`
/// - Byte `13`: `physical` (`0`)
/// - Bytes `14..16`: 15-bit `SubUni` / `Net` universe (`universe & 0x7FFF`, little-endian)
/// - Bytes `16..18`: DMX length (`512`, big-endian)
/// - Bytes `18..530`: 512 DMX slot bytes
#[must_use]
pub fn encode_artnet_dmx(
    universe: u16,
    sequence: u8,
    channels: &[u8; DMX_CHANNEL_COUNT],
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(ARTNET_DMX_PACKET_LEN);
    packet.extend_from_slice(ARTNET_MAGIC);
    packet.extend_from_slice(&ARTNET_OPCODE_DMX.to_le_bytes());
    packet.extend_from_slice(&ARTNET_PROTOCOL_VERSION.to_be_bytes());
    packet.push(sequence);
    packet.push(0u8); // Physical port = 0
    let port_address = universe & 0x7FFF;
    packet.extend_from_slice(&port_address.to_le_bytes());
    #[allow(clippy::cast_possible_truncation)]
    let dmx_len = DMX_CHANNEL_COUNT as u16;
    packet.extend_from_slice(&dmx_len.to_be_bytes());
    packet.extend_from_slice(channels);
    packet
}

/// Decodes and validates an Art-Net 4 `OpDmx` (`0x5000`) UDP packet.
///
/// Returns `None` if the slice is truncated, has an invalid magic header,
/// non-`OpDmx` opcode, protocol version below 14, invalid 15-bit universe
/// high bit, or an out-of-range / odd DMX payload length.
#[must_use]
pub fn decode_artnet_dmx(packet: &[u8]) -> Option<ArtNetDmxPacket> {
    if packet.len() < ARTNET_DMX_HEADER_LEN {
        return None;
    }

    if packet.get(0..8)? != ARTNET_MAGIC {
        return None;
    }

    let opcode = u16::from_le_bytes([*packet.get(8)?, *packet.get(9)?]);
    if opcode != ARTNET_OPCODE_DMX {
        return None;
    }

    let protocol_version = u16::from_be_bytes([*packet.get(10)?, *packet.get(11)?]);
    if protocol_version < ARTNET_PROTOCOL_VERSION {
        return None;
    }

    let sequence = *packet.get(12)?;
    let physical = *packet.get(13)?;
    let sub_uni = *packet.get(14)?;
    let net = *packet.get(15)?;
    if net & 0x80 != 0 {
        return None;
    }
    let universe = u16::from_le_bytes([sub_uni, net]);

    let data_len = u16::from_be_bytes([*packet.get(16)?, *packet.get(17)?]);
    let data_len_usize = usize::from(data_len);
    if !(2..=DMX_CHANNEL_COUNT).contains(&data_len_usize) || data_len_usize % 2 != 0 {
        return None;
    }

    let payload = packet.get(ARTNET_DMX_HEADER_LEN..ARTNET_DMX_HEADER_LEN + data_len_usize)?;
    let mut channels = [0u8; DMX_CHANNEL_COUNT];
    channels[..data_len_usize].copy_from_slice(payload);

    Some(ArtNetDmxPacket {
        protocol_version,
        sequence,
        physical,
        universe,
        data_len,
        channels,
    })
}

/// Encodes a standards-compliant 638-byte sACN (ANSI E1.31-2018) Data Packet.
///
/// Layout:
/// - **ACN Root Layer** (bytes `0..38`): preamble `0x0010`, post-amble `0x0000`,
///   packet ID `"ASC-E1.17\0\0\0"`, flags+length (`0x726E`), vector `0x0000_0004`,
///   and 16-byte `cid`.
/// - **E1.31 Framing Layer** (bytes `38..115`): flags+length (`0x7258`),
///   vector `0x0000_0002`, 64-byte UTF-8 null-terminated `source_name`,
///   `priority` (clamped to `0..=200`), sync address `0`, `sequence`,
///   options `0`, and 16-bit big-endian `universe`.
/// - **DMP Layer** (bytes `115..638`): flags+length (`0x720B`), vector `0x02`,
///   address/data type `0xA1`, first property address `0x0000`, address
///   increment `0x0001`, property value count `513` (`0x0201`), DMX start code
///   `0x00`, and 512 DMX slot bytes.
#[must_use]
pub fn encode_sacn_dmx(
    universe: u16,
    priority: u8,
    sequence: u8,
    source_name: &str,
    cid: [u8; 16],
    channels: &[u8; DMX_CHANNEL_COUNT],
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(SACN_DMX_PACKET_LEN);

    // --- ACN Root Layer (bytes 0..38, 38 bytes) ---
    packet.extend_from_slice(&0x0010u16.to_be_bytes()); // Preamble size = 16
    packet.extend_from_slice(&0x0000u16.to_be_bytes()); // Post-amble size = 0
    packet.extend_from_slice(SACN_ACN_PACKET_ID); // "ASC-E1.17\0\0\0"
    #[allow(clippy::cast_possible_truncation)]
    let root_pdu_len = (SACN_DMX_PACKET_LEN - 16) as u16; // 622 = 0x26E
    packet.extend_from_slice(&(SACN_PDU_FLAGS | root_pdu_len).to_be_bytes());
    packet.extend_from_slice(&SACN_VECTOR_ROOT_E131_DATA.to_be_bytes());
    packet.extend_from_slice(&cid);

    // --- E1.31 Framing Layer (bytes 38..115, 77 bytes) ---
    #[allow(clippy::cast_possible_truncation)]
    let framing_pdu_len = (SACN_DMX_PACKET_LEN - 38) as u16; // 600 = 0x258
    packet.extend_from_slice(&(SACN_PDU_FLAGS | framing_pdu_len).to_be_bytes());
    packet.extend_from_slice(&SACN_VECTOR_E131_DATA_PACKET.to_be_bytes());
    packet.extend_from_slice(&encode_sacn_source_name(source_name));
    packet.push(priority.min(SACN_MAX_PRIORITY));
    packet.extend_from_slice(&0x0000u16.to_be_bytes()); // Synchronization address = 0
    packet.push(sequence);
    packet.push(0x00u8); // Options = 0
    packet.extend_from_slice(&universe.to_be_bytes());

    // --- DMP Layer (bytes 115..638, 523 bytes) ---
    #[allow(clippy::cast_possible_truncation)]
    let dmp_pdu_len = (SACN_DMX_PACKET_LEN - 115) as u16; // 523 = 0x20B
    packet.extend_from_slice(&(SACN_PDU_FLAGS | dmp_pdu_len).to_be_bytes());
    packet.push(SACN_VECTOR_DMP_SET_PROPERTY);
    packet.push(0xA1u8); // Address Type & Data Type
    packet.extend_from_slice(&0x0000u16.to_be_bytes()); // First Property Address = 0
    packet.extend_from_slice(&0x0001u16.to_be_bytes()); // Address Increment = 1
    #[allow(clippy::cast_possible_truncation)]
    let property_value_count = (DMX_CHANNEL_COUNT + 1) as u16; // 513 (start code + 512 slots)
    packet.extend_from_slice(&property_value_count.to_be_bytes());
    packet.push(DMX_NULL_START_CODE);
    packet.extend_from_slice(channels);

    packet
}

/// Encodes a UTF-8 source name into a 64-byte null-terminated buffer without
/// splitting multi-byte UTF-8 code points.
fn encode_sacn_source_name(source_name: &str) -> [u8; SACN_SOURCE_NAME_LEN] {
    let mut buf = [0u8; SACN_SOURCE_NAME_LEN];
    let max_bytes = SACN_SOURCE_NAME_LEN - 1; // Reserve final byte for '\0'
    let mut end = source_name.len().min(max_bytes);
    while end > 0 && !source_name.is_char_boundary(end) {
        end -= 1;
    }
    buf[..end].copy_from_slice(&source_name.as_bytes()[..end]);
    buf
}

/// Stage lighting fixture channel profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FixtureKind {
    /// 1-channel intensity dimmer (`[Dimmer]`).
    Dimmer,
    /// 3-channel RGB color wash (`[R, G, B]`).
    Rgb,
    /// 4-channel RGBW color wash with dedicated white channel (`[R, G, B, W]`).
    Rgbw,
    /// 1-channel strobe shutter / flash intensity (`[Strobe]`).
    Strobe,
}

impl FixtureKind {
    /// Number of consecutive DMX channels occupied by this fixture kind.
    #[must_use]
    pub const fn channel_footprint(self) -> u16 {
        match self {
            Self::Dimmer | Self::Strobe => 1,
            Self::Rgb => 3,
            Self::Rgbw => 4,
        }
    }
}

/// Audio spectral band or beat trigger driving a DMX fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpectralBandSource {
    /// Low-frequency bass energy (`0.0..=1.0`).
    Bass,
    /// Mid-frequency energy (`0.0..=1.0`).
    Mid,
    /// High-frequency treble energy (`0.0..=1.0`).
    High,
    /// Full-spectrum composite energy (`(bass + mid + high) / 3.0` for scalar
    /// fixtures; per-band `R=bass, G=mid, B=high` for RGB/RGBW fixtures).
    Overall,
    /// Beat-onset trigger (`1.0` on beat, `0.0` off beat).
    OnBeat,
}

/// A single DMX fixture patched into a universe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmxFixture {
    /// Human-readable fixture label (e.g., `"Stage Left Wash 1"`).
    pub name: String,
    /// 1-based DMX start address (`1..=512`).
    pub start_address: u16,
    /// Fixture channel layout.
    pub kind: FixtureKind,
    /// Audio band or beat source driving this fixture.
    pub source: SpectralBandSource,
}

impl DmxFixture {
    /// Creates a new DMX fixture patch entry.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        start_address: u16,
        kind: FixtureKind,
        source: SpectralBandSource,
    ) -> Self {
        Self {
            name: name.into(),
            start_address,
            kind,
            source,
        }
    }

    /// Number of consecutive DMX channels consumed by this fixture.
    #[must_use]
    pub const fn channel_footprint(&self) -> u16 {
        self.kind.channel_footprint()
    }
}

/// Stateful spectral-to-DMX patchbay that maps audio analysis into a [`DmxUniverse`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectralDmxMapper {
    fixtures: Vec<DmxFixture>,
    master_dimmer: f32,
    blackout: bool,
}

impl Default for SpectralDmxMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl SpectralDmxMapper {
    /// Creates an empty mapper with full master dimmer (`1.0`) and blackout off.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fixtures: Vec::new(),
            master_dimmer: 1.0,
            blackout: false,
        }
    }

    /// Creates a mapper pre-populated with a fixture patch list.
    #[must_use]
    pub fn with_fixtures(fixtures: Vec<DmxFixture>) -> Self {
        Self {
            fixtures,
            master_dimmer: 1.0,
            blackout: false,
        }
    }

    /// Appends a fixture to the patch list.
    pub fn add_fixture(&mut self, fixture: DmxFixture) {
        self.fixtures.push(fixture);
    }

    /// Returns the patched fixtures.
    #[must_use]
    pub fn fixtures(&self) -> &[DmxFixture] {
        &self.fixtures
    }

    /// Sets the global master dimmer (`0.0..=1.0`, sanitized and clamped).
    pub fn set_master_dimmer(&mut self, master_dimmer: f32) {
        self.master_dimmer = sanitize_unit(master_dimmer);
    }

    /// Returns the current master dimmer (`0.0..=1.0`).
    #[must_use]
    pub const fn master_dimmer(&self) -> f32 {
        self.master_dimmer
    }

    /// Enables or disables full-universe blackout override.
    pub fn set_blackout(&mut self, blackout: bool) {
        self.blackout = blackout;
    }

    /// Returns whether blackout override is currently active.
    #[must_use]
    pub const fn is_blackout(&self) -> bool {
        self.blackout
    }

    /// Evaluates all patched fixtures against the current audio snapshot and
    /// writes the resulting channel values into `universe`.
    pub fn apply(&self, bass: f32, mid: f32, high: f32, on_beat: bool, universe: &mut DmxUniverse) {
        map_audio_to_universe(
            &self.fixtures,
            bass,
            mid,
            high,
            on_beat,
            self.master_dimmer,
            self.blackout,
            universe,
        );
    }
}

/// Maps normalized audio spectral bands and beat state onto patched DMX fixtures.
///
/// - All floating-point inputs (`bass`, `mid`, `high`, `master_dimmer`) are
///   sanitized (non-finite values treated as `0.0`) and clamped to `0.0..=1.0`,
///   producing DMX bytes cleanly clamped to `0..=255`.
/// - When `blackout` is `true`, all 512 channels in `universe` are forced to `0`.
/// - Fixture addresses are 1-based (`1..=512`); any channel offset beyond `512`
///   or at `0` is silently ignored and never overflows.
#[allow(clippy::too_many_arguments)]
pub fn map_audio_to_universe(
    fixtures: &[DmxFixture],
    bass: f32,
    mid: f32,
    high: f32,
    on_beat: bool,
    master_dimmer: f32,
    blackout: bool,
    universe: &mut DmxUniverse,
) {
    if blackout {
        universe.blackout();
        return;
    }

    let bass = sanitize_unit(bass);
    let mid = sanitize_unit(mid);
    let high = sanitize_unit(high);
    let master = sanitize_unit(master_dimmer);
    let overall = sanitize_unit((bass + mid + high) / 3.0);
    let beat = if on_beat { 1.0 } else { 0.0 };

    for fixture in fixtures {
        if fixture.start_address == 0 || usize::from(fixture.start_address) > DMX_CHANNEL_COUNT {
            continue;
        }

        let scalar = match fixture.source {
            SpectralBandSource::Bass => bass,
            SpectralBandSource::Mid => mid,
            SpectralBandSource::High => high,
            SpectralBandSource::Overall => overall,
            SpectralBandSource::OnBeat => beat,
        };

        match fixture.kind {
            FixtureKind::Dimmer | FixtureKind::Strobe => {
                let value = to_dmx_byte(scalar * master);
                universe.set_channel(fixture.start_address, value);
            }
            FixtureKind::Rgb => {
                let (r, g, b) = rgb_for_source(fixture.source, bass, mid, high, beat);
                write_offset(universe, fixture.start_address, 0, to_dmx_byte(r * master));
                write_offset(universe, fixture.start_address, 1, to_dmx_byte(g * master));
                write_offset(universe, fixture.start_address, 2, to_dmx_byte(b * master));
            }
            FixtureKind::Rgbw => {
                let (r, g, b) = rgb_for_source(fixture.source, bass, mid, high, beat);
                write_offset(universe, fixture.start_address, 0, to_dmx_byte(r * master));
                write_offset(universe, fixture.start_address, 1, to_dmx_byte(g * master));
                write_offset(universe, fixture.start_address, 2, to_dmx_byte(b * master));
                write_offset(
                    universe,
                    fixture.start_address,
                    3,
                    to_dmx_byte(beat * master),
                );
            }
        }
    }
}

/// Computes normalized `(R, G, B)` channels for an RGB/RGBW fixture.
#[inline]
fn rgb_for_source(
    source: SpectralBandSource,
    bass: f32,
    mid: f32,
    high: f32,
    beat: f32,
) -> (f32, f32, f32) {
    match source {
        SpectralBandSource::Bass => (bass, 0.0, 0.0),
        SpectralBandSource::Mid => (0.0, mid, 0.0),
        SpectralBandSource::High => (0.0, 0.0, high),
        SpectralBandSource::Overall => (bass, mid, high),
        SpectralBandSource::OnBeat => (beat, beat, beat),
    }
}

/// Safely writes `start_address + offset` without u16 overflow or >512 overrun.
#[inline]
fn write_offset(universe: &mut DmxUniverse, start_address: u16, offset: u16, value: u8) {
    if let Some(channel) = start_address.checked_add(offset) {
        universe.set_channel(channel, value);
    }
}

/// Sanitizes a floating-point unit scalar into `0.0..=1.0` (non-finite -> `0.0`).
#[inline]
fn sanitize_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Converts a normalized `0.0..=1.0` intensity into a `0..=255` DMX byte.
#[inline]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_dmx_byte(normalized: f32) -> u8 {
    (sanitize_unit(normalized) * 255.0).round() as u8
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — Art-Net 4 OpDmx, sACN ANSI E1.31, and SpectralDmxMapper protocol and bounds invariants.

    use super::*;

    #[test]
    fn dmx_universe_safe_one_based_indexing_and_blackout() {
        let mut universe = DmxUniverse::new();
        assert_eq!(universe.get_channel(0), None);
        assert_eq!(universe.get_channel(513), None);
        assert!(!universe.set_channel(0, 255));
        assert!(!universe.set_channel(513, 255));

        assert!(universe.set_channel(1, 42));
        assert!(universe.set_channel(512, 255));
        assert_eq!(universe.get_channel(1), Some(42));
        assert_eq!(universe.get_channel(512), Some(255));
        assert_eq!(universe.channels()[0], 42);
        assert_eq!(universe.channels()[511], 255);

        universe.blackout();
        assert!(universe.channels().iter().all(|&b| b == 0));
    }

    #[test]
    fn artnet_dmx_exact_530_byte_encode_decode_roundtrip() {
        let mut channels = [0u8; DMX_CHANNEL_COUNT];
        for (i, slot) in channels.iter_mut().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            {
                *slot = (i % 256) as u8;
            }
        }
        channels[0] = 0xAA;
        channels[511] = 0x55;

        // 15-bit Port-Address: Net = 0x12, SubUni = 0x34 -> 0x1234
        let universe = 0x1234u16;
        let sequence = 0x7Bu8;
        let packet = encode_artnet_dmx(universe, sequence, &channels);

        assert_eq!(packet.len(), 530);
        assert_eq!(&packet[0..8], b"Art-Net\0");
        assert_eq!(&packet[8..10], &[0x00, 0x50]); // OpDmx 0x5000 little-endian
        assert_eq!(&packet[10..12], &[0x00, 0x0E]); // ProtVer 14 big-endian
        assert_eq!(packet[12], sequence);
        assert_eq!(packet[13], 0x00); // Physical = 0
        assert_eq!(&packet[14..16], &[0x34, 0x12]); // SubUni=0x34, Net=0x12 little-endian
        assert_eq!(&packet[16..18], &[0x02, 0x00]); // Length = 512 big-endian
        assert_eq!(&packet[18..530], &channels[..]);

        let decoded = decode_artnet_dmx(&packet).expect("valid Art-Net 4 OpDmx packet");
        assert_eq!(decoded.protocol_version, ARTNET_PROTOCOL_VERSION);
        assert_eq!(decoded.sequence, sequence);
        assert_eq!(decoded.physical, 0);
        assert_eq!(decoded.universe, universe);
        assert_eq!(decoded.data_len, 512);
        assert_eq!(decoded.channels, channels);
    }

    #[test]
    fn artnet_dmx_masks_15_bit_universe_and_rejects_malformed_packets() {
        let channels = [0u8; DMX_CHANNEL_COUNT];
        // Bit 15 must be masked off per Art-Net 4 15-bit Port-Address spec.
        let packet = encode_artnet_dmx(0xFFFF, 1, &channels);
        assert_eq!(&packet[14..16], &[0xFF, 0x7F]);
        let decoded = decode_artnet_dmx(&packet).expect("masked universe must decode");
        assert_eq!(decoded.universe, 0x7FFF);

        // Truncated header
        assert!(decode_artnet_dmx(&packet[..17]).is_none());
        // Truncated payload
        assert!(decode_artnet_dmx(&packet[..529]).is_none());

        // Bad magic
        let mut bad_magic = packet.clone();
        bad_magic[0] = b'X';
        assert!(decode_artnet_dmx(&bad_magic).is_none());

        // Bad opcode (e.g., OpPoll 0x2000)
        let mut bad_opcode = packet.clone();
        bad_opcode[8] = 0x00;
        bad_opcode[9] = 0x20;
        assert!(decode_artnet_dmx(&bad_opcode).is_none());

        // Bad length (> 512)
        let mut bad_len = packet.clone();
        bad_len[16] = 0x02;
        bad_len[17] = 0x02; // 514
        assert!(decode_artnet_dmx(&bad_len).is_none());

        // Odd length (511)
        let mut odd_len = packet.clone();
        odd_len[16] = 0x01;
        odd_len[17] = 0xFF;
        assert!(decode_artnet_dmx(&odd_len).is_none());
    }

    #[test]
    fn sacn_dmx_exact_638_byte_e131_header_and_payload() {
        let mut channels = [0u8; DMX_CHANNEL_COUNT];
        channels[0] = 0x11;
        channels[255] = 0x88;
        channels[511] = 0xEE;

        let cid: [u8; 16] = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ];
        let packet = encode_sacn_dmx(42, 150, 99, "Chromaticity VJ Rig", cid, &channels);

        assert_eq!(packet.len(), 638);

        // ACN Root Layer (bytes 0..38)
        assert_eq!(&packet[0..2], &[0x00, 0x10]); // Preamble size = 16
        assert_eq!(&packet[2..4], &[0x00, 0x00]); // Post-amble size = 0
        assert_eq!(&packet[4..16], b"ASC-E1.17\0\0\0");
        assert_eq!(&packet[16..18], &0x726Eu16.to_be_bytes()); // Flags 0x7 + length 622
        assert_eq!(&packet[18..22], &0x0000_0004u32.to_be_bytes()); // VECTOR_ROOT_E131_DATA
        assert_eq!(&packet[22..38], &cid);

        // E1.31 Framing Layer (bytes 38..115)
        assert_eq!(&packet[38..40], &0x7258u16.to_be_bytes()); // Flags 0x7 + length 600
        assert_eq!(&packet[40..44], &0x0000_0002u32.to_be_bytes()); // VECTOR_E131_DATA_PACKET
        assert_eq!(&packet[44..63], b"Chromaticity VJ Rig");
        assert!(packet[63..108].iter().all(|&b| b == 0)); // Null-terminated + zero-padded
        assert_eq!(packet[108], 150); // Priority
        assert_eq!(&packet[109..111], &[0x00, 0x00]); // Sync address = 0
        assert_eq!(packet[111], 99); // Sequence
        assert_eq!(packet[112], 0x00); // Options = 0
        assert_eq!(&packet[113..115], &42u16.to_be_bytes()); // Universe = 42

        // DMP Layer (bytes 115..638)
        assert_eq!(&packet[115..117], &0x720Bu16.to_be_bytes()); // Flags 0x7 + length 523
        assert_eq!(packet[117], 0x02); // VECTOR_DMP_SET_PROPERTY
        assert_eq!(packet[118], 0xA1); // Address & Data Type
        assert_eq!(&packet[119..121], &[0x00, 0x00]); // First property address
        assert_eq!(&packet[121..123], &[0x00, 0x01]); // Address increment
        assert_eq!(&packet[123..125], &513u16.to_be_bytes()); // Property value count (1 + 512)
        assert_eq!(packet[125], 0x00); // DMX512-A Start Code
        assert_eq!(&packet[126..638], &channels[..]);

        // Priority above 200 clamps cleanly to SACN_MAX_PRIORITY (200)
        let clamped = encode_sacn_dmx(1, 255, 0, "Long".repeat(30).as_str(), cid, &channels);
        assert_eq!(clamped[108], SACN_MAX_PRIORITY);
        assert_eq!(clamped[107], 0x00); // Final byte of 64-byte source_name is always '\0'
    }

    #[test]
    fn spectral_dmx_mapper_rgbw_dimmer_strobe_blackout_and_bounds_clamping() {
        let fixtures = vec![
            DmxFixture::new(
                "Sub Dimmer",
                1,
                FixtureKind::Dimmer,
                SpectralBandSource::Bass,
            ),
            DmxFixture::new(
                "Main Wash",
                10,
                FixtureKind::Rgbw,
                SpectralBandSource::Overall,
            ),
            DmxFixture::new(
                "Beat Strobe",
                20,
                FixtureKind::Strobe,
                SpectralBandSource::OnBeat,
            ),
            // Starts at channel 511 with 4-channel RGBW: channels 511 & 512 write, 513 & 514 clamped safely.
            DmxFixture::new(
                "Edge Wash",
                511,
                FixtureKind::Rgbw,
                SpectralBandSource::Overall,
            ),
            // Out-of-bounds fixtures (0 and 513) must be ignored without panic.
            DmxFixture::new(
                "Invalid Zero",
                0,
                FixtureKind::Dimmer,
                SpectralBandSource::Bass,
            ),
            DmxFixture::new(
                "Invalid High",
                513,
                FixtureKind::Rgb,
                SpectralBandSource::Overall,
            ),
        ];

        let mut mapper = SpectralDmxMapper::with_fixtures(fixtures);
        mapper.set_master_dimmer(0.5);

        let mut universe = DmxUniverse::new();
        // Values > 1.0 or < 0.0 / NaN clamp safely into 0..=255.
        mapper.apply(1.5, 0.5, -0.2, true, &mut universe);

        // Sub Dimmer (ch 1): bass clamped to 1.0 * master 0.5 -> 128
        assert_eq!(universe.get_channel(1), Some(128));

        // Main Wash RGBW (ch 10..=13): R=1.0*0.5=128, G=0.5*0.5=64, B=0.0, W=beat(1.0)*0.5=128
        assert_eq!(universe.get_channel(10), Some(128));
        assert_eq!(universe.get_channel(11), Some(64));
        assert_eq!(universe.get_channel(12), Some(0));
        assert_eq!(universe.get_channel(13), Some(128));

        // Beat Strobe (ch 20): on_beat=true * master 0.5 -> 128
        assert_eq!(universe.get_channel(20), Some(128));

        // Edge Wash at 511: ch 511 (R) = 128, ch 512 (G) = 64, ch 513/514 ignored
        assert_eq!(universe.get_channel(511), Some(128));
        assert_eq!(universe.get_channel(512), Some(64));

        // Off-beat turns strobe & RGBW white off
        mapper.apply(0.8, 0.4, 0.2, false, &mut universe);
        assert_eq!(universe.get_channel(13), Some(0));
        assert_eq!(universe.get_channel(20), Some(0));

        // NaN inputs sanitize cleanly to 0
        mapper.apply(
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            false,
            &mut universe,
        );
        assert_eq!(universe.get_channel(1), Some(0));
        assert_eq!(universe.get_channel(10), Some(0));
        assert_eq!(universe.get_channel(11), Some(0));
        assert_eq!(universe.get_channel(12), Some(0));

        // Blackout forces all 512 channels to 0 (even previously set channels)
        universe.set_channel(100, 255);
        mapper.set_blackout(true);
        mapper.apply(1.0, 1.0, 1.0, true, &mut universe);
        assert!(universe.channels().iter().all(|&b| b == 0));
    }
}
