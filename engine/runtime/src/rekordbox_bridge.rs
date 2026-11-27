// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Rekordbox Audio & Pro DJ Link bridge.
//!
//! Solves two recurring live-DJ integration problems for Chromaticity:
//!
//! 1. **Master vs. Headphone Cue multi-channel isolation** — Pioneer DJ /
//!    AlphaTheta USB controllers and club mixers expose 4 to 12 interleaved
//!    PCM channels where Master / REC OUT and Headphone Cue occupy separate
//!    stereo pairs (typically Ch 1–2 = Master, Ch 3–4 = Headphone Cue).
//!    [`MultiChannelExtractor`] extracts only the chosen
//!    [`ChannelPairSelection`] into the downstream stereo analysis buffer
//!    while computing live per-pair peak/RMS [`ChannelPairMeter`] readouts
//!    so the operator can visually verify Cue never leaks into visuals or
//!    stream audio. [`DjDeviceProfile`] provides heuristic device-name
//!    recognition (including first-class Pioneer DJ DDJ-FLX10 and Rekordbox
//!    PC Master Out profiles).
//! 2. **Pro DJ Link beat / BPM / phase synchronization** —
//!    [`parse_pro_dj_link_beat_packet`] performs strict, bounds-checked
//!    parsing of Pro DJ Link UDP port 50001 beat packets (`Qspt1WmJOL`,
//!    type `0x28`), feeding [`RekordboxDeckState`] and deterministic
//!    ADR-017 `chroma_Data_Deck*` uniforms ([`RekordboxUniforms`]).

#![forbid(unsafe_code)]

use crate::data_ingest::DataValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Verified macOS bundle identifier for Rekordbox 7 (`Contents/Info.plist`).
pub const REKORDBOX_MACOS_BUNDLE_ID: &str = "com.pioneerdj.rekordboxdj";

/// Verified macOS executable/process name for Rekordbox 7.
pub const REKORDBOX_MACOS_PROCESS_NAME: &str = "rekordbox";

/// Pro DJ Link UDP discovery / keep-alive port.
pub const PRO_DJ_LINK_KEEPALIVE_PORT: u16 = 50_000;

/// Pro DJ Link UDP beat / sync broadcast port.
pub const PRO_DJ_LINK_BEAT_PORT: u16 = 50_001;

/// 10-byte ASCII magic header present on all Pro DJ Link UDP packets.
pub const PRO_DJ_LINK_MAGIC: &[u8; 10] = b"Qspt1WmJOL";

/// Packet type byte for Pro DJ Link beat packets (port 50001).
pub const PRO_DJ_LINK_BEAT_PACKET_TYPE: u8 = 0x28;

/// Minimum valid byte length of a Pro DJ Link beat packet (`0x60` = 96 bytes).
pub const PRO_DJ_LINK_BEAT_PACKET_MIN_LEN: usize = 96;

/// Neutral pitch denominator (`0x0010_0000` = 1.0x playback speed / 0% pitch).
const PRO_DJ_LINK_NEUTRAL_PITCH: f32 = 1_048_576.0;

/// Reactive uniform for the active deck's effective BPM.
pub const UNIFORM_DECK_BPM: &str = "chroma_Data_DeckBpm";

/// Reactive uniform for the active deck's beat phase (`0.0..1.0`).
pub const UNIFORM_DECK_BEAT_PHASE: &str = "chroma_Data_DeckBeatPhase";

/// Reactive uniform for the active deck's beat within the 4/4 bar (`1.0..=4.0`, or `0.0` idle).
pub const UNIFORM_DECK_BAR_BEAT: &str = "chroma_Data_DeckBarBeat";

/// Reactive uniform indicating whether the deck is tempo master (`0.0` or `1.0`).
pub const UNIFORM_DECK_IS_MASTER: &str = "chroma_Data_DeckIsMaster";

/// Reactive uniform indicating whether the deck is currently playing (`0.0` or `1.0`).
pub const UNIFORM_DECK_PLAYING: &str = "chroma_Data_DeckPlaying";

/// Reactive uniform for normalized phrase energy (`0.0..=1.0`).
pub const UNIFORM_DECK_PHRASE_ENERGY: &str = "chroma_Data_DeckPhraseEnergy";

/// Reactive uniform for exponentially decaying cue / downbeat pulse (`0.0..=1.0`).
pub const UNIFORM_DECK_CUE_PULSE: &str = "chroma_Data_DeckCuePulse";

/// Linear amplitude threshold above which a channel pair meter reports `active = true` (~ -80 dBFS).
const METER_ACTIVE_THRESHOLD: f32 = 1e-4;

/// Minimum dBFS floor returned by [`ChannelPairMeter::peak_dbfs`] and [`ChannelPairMeter::rms_dbfs`].
const METER_MIN_DBFS: f32 = -120.0;

/// Default exponential decay time constant (seconds) for [`RekordboxDeckState::cue_pulse`].
const DEFAULT_CUE_PULSE_DECAY_SECS: f32 = 0.35;

/// Floor below which decayed cue pulse values snap cleanly to `0.0`.
const PULSE_ZERO_EPSILON: f32 = 1e-6;

/// Errors returned by Rekordbox multi-channel extraction and configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RekordboxBridgeError {
    /// Device or stream reported zero channels.
    ZeroChannels,
    /// Selected stereo channel indices exceed the stream's channel count.
    ChannelOutOfRange {
        /// Selected left channel (0-based).
        left_channel: usize,
        /// Selected right channel (0-based).
        right_channel: usize,
        /// Total available channels in the stream.
        channel_count: usize,
    },
    /// Interleaved PCM slice length is not a whole multiple of `channel_count`.
    RaggedFrameBuffer {
        /// Number of float samples provided.
        sample_count: usize,
        /// Configured channels per frame.
        channel_count: usize,
    },
}

impl fmt::Display for RekordboxBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroChannels => write!(f, "audio stream must have at least one channel"),
            Self::ChannelOutOfRange {
                left_channel,
                right_channel,
                channel_count,
            } => write!(
                f,
                "selected channel pair ({left_channel}, {right_channel}) exceeds stream channel count {channel_count}"
            ),
            Self::RaggedFrameBuffer {
                sample_count,
                channel_count,
            } => write!(
                f,
                "interleaved buffer length {sample_count} is not a multiple of channel count {channel_count}"
            ),
        }
    }
}

impl std::error::Error for RekordboxBridgeError {}

/// Selected stereo channel pair (0-based indices) within an interleaved N-channel stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelPairSelection {
    /// Zero-based index of the left channel (e.g. `0` for Ch 1).
    pub left_channel: usize,
    /// Zero-based index of the right channel (e.g. `1` for Ch 2).
    pub right_channel: usize,
}

impl ChannelPairSelection {
    /// Constructs a selection from 0-based channel indices.
    #[must_use]
    pub const fn new(left_channel: usize, right_channel: usize) -> Self {
        Self {
            left_channel,
            right_channel,
        }
    }

    /// Constructs a contiguous stereo pair by 0-based pair index (`0` → `(0, 1)`, `1` → `(2, 3)`).
    #[must_use]
    pub const fn stereo_pair(pair_index: usize) -> Self {
        let base = pair_index.saturating_mul(2);
        Self {
            left_channel: base,
            right_channel: base.saturating_add(1),
        }
    }

    /// Default Master output stereo pair: Ch 1–2 (`(0, 1)`).
    #[must_use]
    pub const fn master_default() -> Self {
        Self::new(0, 1)
    }

    /// Constructs a selection from 1-based human channel numbers (e.g. `(1, 2)` → `(0, 1)`).
    /// Returns `None` if either channel number is `0`.
    #[must_use]
    pub const fn from_one_based(left_one_based: usize, right_one_based: usize) -> Option<Self> {
        match (
            left_one_based.checked_sub(1),
            right_one_based.checked_sub(1),
        ) {
            (Some(left_channel), Some(right_channel)) => Some(Self {
                left_channel,
                right_channel,
            }),
            _ => None,
        }
    }

    /// Returns `true` if both selected channels exist in a stream of `channel_count` channels.
    #[must_use]
    pub const fn is_valid_for_channels(self, channel_count: usize) -> bool {
        self.left_channel < channel_count && self.right_channel < channel_count
    }

    /// Returns a 1-based human label such as `"Ch 1-2"` or `"Ch 1"` (for mono).
    #[must_use]
    pub fn one_based_label(self) -> String {
        let left = self.left_channel.saturating_add(1);
        let right = self.right_channel.saturating_add(1);
        if left == right {
            format!("Ch {left}")
        } else {
            format!("Ch {left}-{right}")
        }
    }
}

impl Default for ChannelPairSelection {
    fn default() -> Self {
        Self::master_default()
    }
}

/// Live peak/RMS VU readout for a single stereo channel pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelPairMeter {
    /// Zero-based pair index (`0` = Ch 1–2, `1` = Ch 3–4, ...).
    pub pair_index: usize,
    /// Zero-based left channel index.
    pub left_channel: usize,
    /// Zero-based right channel index.
    pub right_channel: usize,
    /// Human-readable label for UI display (e.g. `"Ch 1-2 (Master / REC OUT)"`).
    pub label: String,
    /// Peak absolute linear sample amplitude on the left channel (`0.0..`).
    pub peak_left: f32,
    /// Peak absolute linear sample amplitude on the right channel (`0.0..`).
    pub peak_right: f32,
    /// Combined stereo peak (`max(peak_left, peak_right)`).
    pub peak: f32,
    /// Root-mean-square amplitude on the left channel.
    pub rms_left: f32,
    /// Root-mean-square amplitude on the right channel.
    pub rms_right: f32,
    /// Combined stereo RMS amplitude.
    pub rms: f32,
    /// `true` when `peak` exceeds the noise-floor activity threshold.
    pub active: bool,
}

impl ChannelPairMeter {
    /// Combined stereo peak level in dBFS, clamped to `[-120.0, 0.0]`.
    #[must_use]
    pub fn peak_dbfs(&self) -> f32 {
        linear_to_dbfs(self.peak)
    }

    /// Combined stereo RMS level in dBFS, clamped to `[-120.0, 0.0]`.
    #[must_use]
    pub fn rms_dbfs(&self) -> f32 {
        linear_to_dbfs(self.rms)
    }
}

/// Converts a linear amplitude to dBFS, clamped to `[METER_MIN_DBFS, 0.0]`.
#[must_use]
fn linear_to_dbfs(amplitude: f32) -> f32 {
    if !amplitude.is_finite() || amplitude <= 0.0 {
        return METER_MIN_DBFS;
    }
    let db = 20.0 * amplitude.log10();
    db.clamp(METER_MIN_DBFS, 0.0)
}

/// Sanitizes an incoming PCM float sample, replacing `NaN` / `±Inf` with `0.0`.
#[inline]
#[must_use]
fn sanitize_sample(sample: f32) -> f32 {
    if sample.is_finite() { sample } else { 0.0 }
}

/// Known Pioneer DJ / AlphaTheta / Rekordbox hardware and routing families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DjDeviceKind {
    /// Pioneer DJ DDJ-FLX10 4-channel controller (USB REC OUT / Master Ch 1–2, Cue Ch 3–4).
    DdjFlx10,
    /// Pioneer DJ DDJ-FLX4 2-channel class-compliant USB controller.
    DdjFlx4,
    /// Pioneer DJ DDJ-1000 4-channel controller.
    Ddj1000,
    /// AlphaTheta / Pioneer DJ OPUS-QUAD all-in-one DJ system.
    OpusQuad,
    /// Pioneer DJ XDJ-XZ all-in-one DJ system.
    XdjXz,
    /// Pioneer DJ DJM-A9 4-channel club mixer (10ch/12ch USB interface).
    DjmA9,
    /// Pioneer DJ DJM-900NXS2 4-channel club mixer (10ch/12ch USB interface).
    Djm900Nxs2,
    /// Rekordbox PC Master Out / Core Audio Process Tap (`com.pioneerdj.rekordboxdj`).
    RekordboxPcMasterOut,
    /// Unrecognized multi-channel or stereo audio device.
    GenericMultiChannel,
}

/// Heuristic channel-routing profile for a recognized DJ controller, mixer, or Rekordbox tap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DjDeviceProfile {
    /// Classified device family.
    pub kind: DjDeviceKind,
    /// Canonical display name for the UI.
    pub display_name: String,
    /// Typical USB / Core Audio channel count for this hardware.
    pub expected_channels: usize,
    /// Recommended stereo channel pair for clean Master / REC OUT capture.
    pub master_pair: ChannelPairSelection,
    /// Human-readable label for the Master pair.
    pub master_label: String,
    /// Headphone Cue / monitor pair to isolate and exclude (if exposed by the device).
    pub cue_pair: Option<ChannelPairSelection>,
    /// Human-readable label for the Cue pair (if present).
    pub cue_label: Option<String>,
    /// Whether Rekordbox `PC MASTER OUT` + ADR-022 process tap is supported alongside direct USB.
    pub supports_pc_master_out: bool,
    /// Operator setup guidance for routing without Headphone Cue leakage.
    pub notes: String,
}

impl DjDeviceProfile {
    /// Recognizes common Pioneer DJ / AlphaTheta / Rekordbox audio devices from a
    /// Core Audio / WASAPI device name or macOS bundle identifier.
    ///
    /// Matching is case-insensitive and ignores hyphens, underscores, and spaces so
    /// `"DDJ-FLX10"`, `"Pioneer DJ DDJ FLX10"`, and `"FLX10"` all resolve cleanly.
    #[must_use]
    pub fn recognize(device_name: &str) -> Option<Self> {
        let norm = normalize_device_token(device_name);
        if norm.is_empty() {
            return None;
        }

        if norm.contains("ddjflx10") || norm.contains("flx10") {
            return Some(Self::ddj_flx10());
        }
        if norm.contains("ddjflx4") || norm.contains("flx4") {
            return Some(Self::ddj_flx4());
        }
        if norm.contains("ddj1000") {
            return Some(Self::ddj_1000());
        }
        if norm.contains("opusquad") {
            return Some(Self::opus_quad());
        }
        if norm.contains("xdjxz") {
            return Some(Self::xdj_xz());
        }
        if norm.contains("djma9") {
            return Some(Self::djm_a9());
        }
        if norm.contains("djm900nxs2") || norm.contains("900nxs2") {
            return Some(Self::djm_900nxs2());
        }
        if norm.contains("pcmasterout")
            || norm.contains("rekordbox")
            || norm.contains("compioneerdjrekordboxdj")
        {
            return Some(Self::rekordbox_pc_master_out());
        }

        None
    }

    /// Recognizes a known DJ device or returns a safe generic multi-channel profile
    /// tailored to `channel_count`.
    #[must_use]
    pub fn recognize_or_default(device_name: &str, channel_count: usize) -> Self {
        if let Some(profile) = Self::recognize(device_name) {
            return profile;
        }
        let channels = channel_count.max(1);
        let master_pair = if channels == 1 {
            ChannelPairSelection::new(0, 0)
        } else {
            ChannelPairSelection::master_default()
        };
        let (cue_pair, cue_label) = if channels >= 4 {
            (
                Some(ChannelPairSelection::stereo_pair(1)),
                Some("Ch 3-4 (Headphone Cue)".to_owned()),
            )
        } else {
            (None, None)
        };
        Self {
            kind: DjDeviceKind::GenericMultiChannel,
            display_name: if device_name.trim().is_empty() {
                "Generic Audio Device".to_owned()
            } else {
                device_name.trim().to_owned()
            },
            expected_channels: channels,
            master_pair,
            master_label: if channels == 1 {
                "Ch 1 (Mono Master)".to_owned()
            } else {
                "Ch 1-2 (Master)".to_owned()
            },
            cue_pair,
            cue_label,
            supports_pc_master_out: true,
            notes: "Select the Master stereo pair and confirm Headphone Cue stays silent on the selected pair's VU meter."
                .to_owned(),
        }
    }

    /// Returns a human-readable label for a stereo pair index on this device.
    #[must_use]
    pub fn label_for_pair(&self, left_channel: usize, right_channel: usize) -> String {
        let pair = ChannelPairSelection::new(left_channel, right_channel);
        if pair == self.master_pair {
            return self.master_label.clone();
        }
        if let Some(cue) = self.cue_pair {
            if pair == cue {
                if let Some(ref label) = self.cue_label {
                    return label.clone();
                }
            }
        }
        pair.one_based_label()
    }

    /// First-class profile for the **Pioneer DJ DDJ-FLX10**.
    ///
    /// Covers both:
    /// - Direct USB Core Audio input (`DDJ-FLX10` Master / REC OUT on Ch 1–2,
    ///   isolating Headphone Cue and Deck/Line pairs on Ch 3–4+), and
    /// - Rekordbox `PC MASTER OUT` via macOS Core Audio Process Tap (ADR-022),
    ///   which already strips Headphone Cue before macOS audio output.
    #[must_use]
    pub fn ddj_flx10() -> Self {
        Self {
            kind: DjDeviceKind::DdjFlx10,
            display_name: "Pioneer DJ DDJ-FLX10".to_owned(),
            expected_channels: 4,
            master_pair: ChannelPairSelection::new(0, 1),
            master_label: "Ch 1-2 (Master / REC OUT)".to_owned(),
            cue_pair: Some(ChannelPairSelection::new(2, 3)),
            cue_label: Some("Ch 3-4 (Headphone Cue / Deck)".to_owned()),
            supports_pc_master_out: true,
            notes: "Direct USB: tap Ch 1-2 (Master / REC OUT) and ignore Ch 3-4+ (Cue/Deck/Line). Or enable Rekordbox PC MASTER OUT and tap via macOS Core Audio Process Tap (ADR-022), which already excludes Headphone Cue."
                .to_owned(),
        }
    }

    /// Profile for the Pioneer DJ DDJ-FLX4 (4ch class-compliant USB).
    #[must_use]
    pub fn ddj_flx4() -> Self {
        Self {
            kind: DjDeviceKind::DdjFlx4,
            display_name: "Pioneer DJ DDJ-FLX4".to_owned(),
            expected_channels: 4,
            master_pair: ChannelPairSelection::new(0, 1),
            master_label: "Ch 1-2 (Master Out)".to_owned(),
            cue_pair: Some(ChannelPairSelection::new(2, 3)),
            cue_label: Some("Ch 3-4 (Headphone Cue)".to_owned()),
            supports_pc_master_out: true,
            notes: "Class-compliant 4ch USB audio: Ch 1-2 carries Master Out, Ch 3-4 carries Headphone Cue."
                .to_owned(),
        }
    }

    /// Profile for the Pioneer DJ DDJ-1000.
    #[must_use]
    pub fn ddj_1000() -> Self {
        Self {
            kind: DjDeviceKind::Ddj1000,
            display_name: "Pioneer DJ DDJ-1000".to_owned(),
            expected_channels: 4,
            master_pair: ChannelPairSelection::new(0, 1),
            master_label: "Ch 1-2 (Master / REC OUT)".to_owned(),
            cue_pair: Some(ChannelPairSelection::new(2, 3)),
            cue_label: Some("Ch 3-4 (Headphone Cue)".to_owned()),
            supports_pc_master_out: true,
            notes: "Tap Ch 1-2 for Master / REC OUT; Ch 3-4 carries Headphone Cue.".to_owned(),
        }
    }

    /// Profile for the AlphaTheta / Pioneer DJ OPUS-QUAD.
    #[must_use]
    pub fn opus_quad() -> Self {
        Self {
            kind: DjDeviceKind::OpusQuad,
            display_name: "AlphaTheta OPUS-QUAD".to_owned(),
            expected_channels: 6,
            master_pair: ChannelPairSelection::new(0, 1),
            master_label: "Ch 1-2 (Master Out)".to_owned(),
            cue_pair: Some(ChannelPairSelection::new(2, 3)),
            cue_label: Some("Ch 3-4 (Headphone Cue)".to_owned()),
            supports_pc_master_out: true,
            notes: "Tap Ch 1-2 for Master Out; Ch 3-4 is Headphone Cue and Ch 5-6 is Zone Out."
                .to_owned(),
        }
    }

    /// Profile for the Pioneer DJ XDJ-XZ.
    #[must_use]
    pub fn xdj_xz() -> Self {
        Self {
            kind: DjDeviceKind::XdjXz,
            display_name: "Pioneer DJ XDJ-XZ".to_owned(),
            expected_channels: 6,
            master_pair: ChannelPairSelection::new(0, 1),
            master_label: "Ch 1-2 (Master / REC OUT)".to_owned(),
            cue_pair: Some(ChannelPairSelection::new(2, 3)),
            cue_label: Some("Ch 3-4 (Headphone Cue)".to_owned()),
            supports_pc_master_out: true,
            notes: "Tap Ch 1-2 for Master / REC OUT; isolate Ch 3-4 Headphone Cue.".to_owned(),
        }
    }

    /// Profile for the Pioneer DJ DJM-A9 club mixer (10ch USB interface).
    #[must_use]
    pub fn djm_a9() -> Self {
        Self {
            kind: DjDeviceKind::DjmA9,
            display_name: "Pioneer DJ DJM-A9".to_owned(),
            expected_channels: 10,
            master_pair: ChannelPairSelection::new(0, 1),
            master_label: "Ch 1-2 (USB REC OUT / Master)".to_owned(),
            cue_pair: Some(ChannelPairSelection::new(8, 9)),
            cue_label: Some("Ch 9-10 (Aux / Monitor)".to_owned()),
            supports_pc_master_out: false,
            notes: "In DJM-A9 Setting Utility, assign USB Out 1-2 (or 9-10) to REC OUT; match that pair in Chromaticity."
                .to_owned(),
        }
    }

    /// Profile for the Pioneer DJ DJM-900NXS2 club mixer (10ch USB interface).
    #[must_use]
    pub fn djm_900nxs2() -> Self {
        Self {
            kind: DjDeviceKind::Djm900Nxs2,
            display_name: "Pioneer DJ DJM-900NXS2".to_owned(),
            expected_channels: 10,
            master_pair: ChannelPairSelection::new(0, 1),
            master_label: "Ch 1-2 (USB REC OUT / Master)".to_owned(),
            cue_pair: Some(ChannelPairSelection::new(8, 9)),
            cue_label: Some("Ch 9-10 (Crossfader / Monitor)".to_owned()),
            supports_pc_master_out: false,
            notes: "In DJM-900NXS2 Setting Utility, route REC OUT to USB 1-2 (or 9-10) and select the matching stereo pair."
                .to_owned(),
        }
    }

    /// Profile for Rekordbox `PC MASTER OUT` via macOS Core Audio Process Tap (ADR-022).
    #[must_use]
    pub fn rekordbox_pc_master_out() -> Self {
        Self {
            kind: DjDeviceKind::RekordboxPcMasterOut,
            display_name: "Rekordbox PC Master Out".to_owned(),
            expected_channels: 2,
            master_pair: ChannelPairSelection::new(0, 1),
            master_label: "Ch 1-2 (PC Master Out)".to_owned(),
            cue_pair: None,
            cue_label: None,
            supports_pc_master_out: true,
            notes: "Rekordbox PC MASTER OUT mirrors Master-only audio to macOS output and already strips Headphone Cue; tap via ADR-022 Core Audio Process Tap targeting com.pioneerdj.rekordboxdj."
                .to_owned(),
        }
    }
}

/// Lowercases and strips non-alphanumeric separators for robust device-name matching.
#[must_use]
fn normalize_device_token(input: &str) -> String {
    input
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Extracts a chosen stereo channel pair from interleaved N-channel PCM frames
/// while computing live peak/RMS meters for every channel pair in the stream.
#[derive(Debug, Clone)]
pub struct MultiChannelExtractor {
    channel_count: usize,
    selection: ChannelPairSelection,
    device_profile: Option<DjDeviceProfile>,
}

impl MultiChannelExtractor {
    /// Creates a new extractor for an interleaved `channel_count`-channel stream.
    ///
    /// Fails fast if `channel_count == 0` or `selection` references a channel outside
    /// `0..channel_count`.
    pub fn new(
        channel_count: usize,
        selection: ChannelPairSelection,
    ) -> Result<Self, RekordboxBridgeError> {
        if channel_count == 0 {
            return Err(RekordboxBridgeError::ZeroChannels);
        }
        if !selection.is_valid_for_channels(channel_count) {
            return Err(RekordboxBridgeError::ChannelOutOfRange {
                left_channel: selection.left_channel,
                right_channel: selection.right_channel,
                channel_count,
            });
        }
        Ok(Self {
            channel_count,
            selection,
            device_profile: None,
        })
    }

    /// Creates an extractor configured from a [`DjDeviceProfile`]'s recommended Master pair.
    ///
    /// If `channel_count == 1`, falls back cleanly to mono `(0, 0)`.
    pub fn for_device(
        device_name: &str,
        channel_count: usize,
    ) -> Result<Self, RekordboxBridgeError> {
        if channel_count == 0 {
            return Err(RekordboxBridgeError::ZeroChannels);
        }
        let profile = DjDeviceProfile::recognize_or_default(device_name, channel_count);
        let selection = if profile.master_pair.is_valid_for_channels(channel_count) {
            profile.master_pair
        } else if channel_count >= 2 {
            ChannelPairSelection::master_default()
        } else {
            ChannelPairSelection::new(0, 0)
        };
        Ok(Self {
            channel_count,
            selection,
            device_profile: Some(profile),
        })
    }

    /// Attaches a [`DjDeviceProfile`] used to annotate [`ChannelPairMeter::label`] values.
    #[must_use]
    pub fn with_device_profile(mut self, profile: DjDeviceProfile) -> Self {
        self.device_profile = Some(profile);
        self
    }

    /// Returns the configured stream channel count.
    #[must_use]
    pub const fn channel_count(&self) -> usize {
        self.channel_count
    }

    /// Returns the currently active stereo channel pair selection.
    #[must_use]
    pub const fn selection(&self) -> ChannelPairSelection {
        self.selection
    }

    /// Returns the optional attached device profile.
    #[must_use]
    pub const fn device_profile(&self) -> Option<&DjDeviceProfile> {
        self.device_profile.as_ref()
    }

    /// Updates the active channel pair selection after validating bounds.
    pub fn set_selection(
        &mut self,
        selection: ChannelPairSelection,
    ) -> Result<(), RekordboxBridgeError> {
        if !selection.is_valid_for_channels(self.channel_count) {
            return Err(RekordboxBridgeError::ChannelOutOfRange {
                left_channel: selection.left_channel,
                right_channel: selection.right_channel,
                channel_count: self.channel_count,
            });
        }
        self.selection = selection;
        Ok(())
    }

    /// Extracts the selected stereo channel pair into `out_stereo` (clearing any prior
    /// contents) and returns per-pair peak/RMS [`ChannelPairMeter`] readouts for every
    /// stereo pair in the stream.
    ///
    /// - Interleaved input layout: `[ch0_f0, ch1_f0, ..., ch{N-1}_f0, ch0_f1, ...]`.
    /// - Interleaved stereo output layout: `[L_f0, R_f0, L_f1, R_f1, ...]`.
    /// - Non-finite input samples (`NaN` / `±Inf`) are sanitized to `0.0`.
    pub fn extract_into(
        &self,
        interleaved: &[f32],
        out_stereo: &mut Vec<f32>,
    ) -> Result<Vec<ChannelPairMeter>, RekordboxBridgeError> {
        if interleaved.len() % self.channel_count != 0 {
            return Err(RekordboxBridgeError::RaggedFrameBuffer {
                sample_count: interleaved.len(),
                channel_count: self.channel_count,
            });
        }

        let frame_count = interleaved.len() / self.channel_count;
        out_stereo.clear();
        out_stereo.reserve(frame_count.saturating_mul(2));

        let pair_count = self.channel_count.div_ceil(2);
        let mut peaks_l = vec![0.0_f32; pair_count];
        let mut peaks_r = vec![0.0_f32; pair_count];
        let mut sum_sq_l = vec![0.0_f64; pair_count];
        let mut sum_sq_r = vec![0.0_f64; pair_count];

        for frame in interleaved.chunks_exact(self.channel_count) {
            let sel_l = frame
                .get(self.selection.left_channel)
                .copied()
                .map_or(0.0, sanitize_sample);
            let sel_r = frame
                .get(self.selection.right_channel)
                .copied()
                .map_or(0.0, sanitize_sample);
            out_stereo.push(sel_l);
            out_stereo.push(sel_r);

            for pair_idx in 0..pair_count {
                let left_ch = pair_idx.saturating_mul(2);
                let right_ch = (left_ch.saturating_add(1)).min(self.channel_count - 1);

                let s_l = frame.get(left_ch).copied().map_or(0.0, sanitize_sample);
                let s_r = frame.get(right_ch).copied().map_or(0.0, sanitize_sample);

                let abs_l = s_l.abs();
                let abs_r = s_r.abs();

                if let Some(p_l) = peaks_l.get_mut(pair_idx) {
                    if abs_l > *p_l {
                        *p_l = abs_l;
                    }
                }
                if let Some(p_r) = peaks_r.get_mut(pair_idx) {
                    if abs_r > *p_r {
                        *p_r = abs_r;
                    }
                }
                if let Some(sq_l) = sum_sq_l.get_mut(pair_idx) {
                    let v = f64::from(s_l);
                    *sq_l += v * v;
                }
                if let Some(sq_r) = sum_sq_r.get_mut(pair_idx) {
                    let v = f64::from(s_r);
                    *sq_r += v * v;
                }
            }
        }

        let mut meters = Vec::with_capacity(pair_count);
        let denom = if frame_count > 0 {
            frame_count as f64
        } else {
            1.0
        };

        for pair_idx in 0..pair_count {
            let left_ch = pair_idx.saturating_mul(2);
            let right_ch = (left_ch.saturating_add(1)).min(self.channel_count - 1);

            let peak_left = peaks_l.get(pair_idx).copied().unwrap_or(0.0);
            let peak_right = peaks_r.get(pair_idx).copied().unwrap_or(0.0);
            let peak = peak_left.max(peak_right);

            let sq_l = sum_sq_l.get(pair_idx).copied().unwrap_or(0.0);
            let sq_r = sum_sq_r.get(pair_idx).copied().unwrap_or(0.0);

            let rms_left = if frame_count > 0 {
                (sq_l / denom).sqrt() as f32
            } else {
                0.0
            };
            let rms_right = if frame_count > 0 {
                (sq_r / denom).sqrt() as f32
            } else {
                0.0
            };
            let rms = if frame_count > 0 {
                ((sq_l + sq_r) / (2.0 * denom)).sqrt() as f32
            } else {
                0.0
            };

            let label = match self.device_profile {
                Some(ref profile) => profile.label_for_pair(left_ch, right_ch),
                None => ChannelPairSelection::new(left_ch, right_ch).one_based_label(),
            };

            meters.push(ChannelPairMeter {
                pair_index: pair_idx,
                left_channel: left_ch,
                right_channel: right_ch,
                label,
                peak_left,
                peak_right,
                peak,
                rms_left,
                rms_right,
                rms,
                active: peak > METER_ACTIVE_THRESHOLD,
            });
        }

        Ok(meters)
    }

    /// Convenience wrapper that allocates and returns `(stereo_pcm, per_pair_meters)`.
    pub fn extract(
        &self,
        interleaved: &[f32],
    ) -> Result<(Vec<f32>, Vec<ChannelPairMeter>), RekordboxBridgeError> {
        let mut stereo = Vec::new();
        let meters = self.extract_into(interleaved, &mut stereo)?;
        Ok((stereo, meters))
    }
}

/// Decoded Pro DJ Link UDP beat packet (port 50001, packet type `0x28`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProDjLinkBeat {
    /// Device name reported by the player (e.g. `"CDJ-3000"`, `"rekordbox"`).
    pub device_name: String,
    /// Player / deck number (`1..=4` for CDJs, `0x11+` for Rekordbox).
    pub player_number: u8,
    /// Milliseconds until the next beat.
    pub next_beat_ms: u32,
    /// Milliseconds until the next bar downbeat.
    pub next_bar_ms: u32,
    /// Raw pitch integer (`0x0010_0000` = 1.0x / 0% pitch).
    pub pitch_raw: u32,
    /// Playback speed multiplier derived from `pitch_raw`.
    pub pitch_multiplier: f32,
    /// Unpitched track BPM (decoded from BPM × 100).
    pub track_bpm: f32,
    /// Effective live BPM (`track_bpm * pitch_multiplier`).
    pub effective_bpm: f32,
    /// Beat index within the current 4/4 bar (`1..=4`, where `1` is the downbeat).
    pub beat_within_bar: u8,
}

/// Safely parses a Pro DJ Link UDP beat packet (port 50001, type `0x28`).
///
/// Returns `None` if the slice is shorter than [`PRO_DJ_LINK_BEAT_PACKET_MIN_LEN`] (96 bytes),
/// lacks the [`PRO_DJ_LINK_MAGIC`] header (`Qspt1WmJOL`), is not packet type `0x28`,
/// or carries an out-of-range `beat_within_bar` (outside `1..=4`).
#[must_use]
pub fn parse_pro_dj_link_beat_packet(packet: &[u8]) -> Option<ProDjLinkBeat> {
    if packet.len() < PRO_DJ_LINK_BEAT_PACKET_MIN_LEN {
        return None;
    }
    if packet.get(0..10)? != PRO_DJ_LINK_MAGIC {
        return None;
    }
    if *packet.get(0x0a)? != PRO_DJ_LINK_BEAT_PACKET_TYPE {
        return None;
    }

    let name_bytes = packet.get(0x0b..0x1f)?;
    let nul_pos = name_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(name_bytes.len());
    let device_name = String::from_utf8_lossy(name_bytes.get(..nul_pos)?)
        .trim()
        .to_owned();

    let player_number = *packet.get(0x21)?;
    let next_beat_ms = read_be_u32(packet, 0x24)?;
    let next_bar_ms = read_be_u32(packet, 0x2c)?;
    let pitch_raw = read_be_u32(packet, 0x54)?;
    let bpm_centi = read_be_u16(packet, 0x58)?;
    let beat_within_bar = *packet.get(0x5b)?;

    if !(1..=4).contains(&beat_within_bar) {
        return None;
    }

    let pitch_multiplier = (pitch_raw as f32) / PRO_DJ_LINK_NEUTRAL_PITCH;
    let track_bpm = f32::from(bpm_centi) / 100.0;
    let effective_bpm = track_bpm * pitch_multiplier;

    if !pitch_multiplier.is_finite() || !effective_bpm.is_finite() {
        return None;
    }

    Some(ProDjLinkBeat {
        device_name,
        player_number,
        next_beat_ms,
        next_bar_ms,
        pitch_raw,
        pitch_multiplier,
        track_bpm,
        effective_bpm,
        beat_within_bar,
    })
}

/// Bounds-checked big-endian `u32` reader.
#[inline]
#[must_use]
fn read_be_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset.checked_add(4)?)?;
    let arr: [u8; 4] = slice.try_into().ok()?;
    Some(u32::from_be_bytes(arr))
}

/// Bounds-checked big-endian `u16` reader.
#[inline]
#[must_use]
fn read_be_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset.checked_add(2)?)?;
    let arr: [u8; 2] = slice.try_into().ok()?;
    Some(u16::from_be_bytes(arr))
}

/// Deterministic ADR-017 shader uniform snapshot emitted from [`RekordboxDeckState`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RekordboxUniforms {
    /// `chroma_Data_DeckBpm`: effective deck BPM (`0.0` when idle).
    pub deck_bpm: f32,
    /// `chroma_Data_DeckBeatPhase`: continuous beat phase in `[0.0, 1.0)`.
    pub deck_beat_phase: f32,
    /// `chroma_Data_DeckBarBeat`: beat position within bar (`1.0..=4.0`, or `0.0` when idle).
    pub deck_bar_beat: f32,
    /// `chroma_Data_DeckIsMaster`: `1.0` if deck is tempo master, else `0.0`.
    pub deck_is_master: f32,
    /// `chroma_Data_DeckPlaying`: `1.0` if deck is playing, else `0.0`.
    pub deck_playing: f32,
    /// `chroma_Data_DeckPhraseEnergy`: normalized phrase energy in `[0.0, 1.0]`.
    pub deck_phrase_energy: f32,
    /// `chroma_Data_DeckCuePulse`: exponentially decaying cue / downbeat pulse in `[0.0, 1.0]`.
    pub deck_cue_pulse: f32,
}

impl Default for RekordboxUniforms {
    fn default() -> Self {
        Self {
            deck_bpm: 0.0,
            deck_beat_phase: 0.0,
            deck_bar_beat: 0.0,
            deck_is_master: 0.0,
            deck_playing: 0.0,
            deck_phrase_energy: 0.0,
            deck_cue_pulse: 0.0,
        }
    }
}

impl RekordboxUniforms {
    /// Exports all seven `chroma_Data_Deck*` uniforms into an ordered ADR-017 data map.
    #[must_use]
    pub fn to_data_map(&self) -> BTreeMap<String, DataValue> {
        let mut map = BTreeMap::new();
        map.insert(
            UNIFORM_DECK_BPM.to_owned(),
            DataValue::Scalar(self.deck_bpm),
        );
        map.insert(
            UNIFORM_DECK_BEAT_PHASE.to_owned(),
            DataValue::Scalar(self.deck_beat_phase),
        );
        map.insert(
            UNIFORM_DECK_BAR_BEAT.to_owned(),
            DataValue::Scalar(self.deck_bar_beat),
        );
        map.insert(
            UNIFORM_DECK_IS_MASTER.to_owned(),
            DataValue::Scalar(self.deck_is_master),
        );
        map.insert(
            UNIFORM_DECK_PLAYING.to_owned(),
            DataValue::Scalar(self.deck_playing),
        );
        map.insert(
            UNIFORM_DECK_PHRASE_ENERGY.to_owned(),
            DataValue::Scalar(self.deck_phrase_energy),
        );
        map.insert(
            UNIFORM_DECK_CUE_PULSE.to_owned(),
            DataValue::Scalar(self.deck_cue_pulse),
        );
        map
    }
}

/// Live deck synchronization state fed by Pro DJ Link beat packets or Ableton Link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RekordboxDeckState {
    /// Deck / player identifier (`1..=4` or `0x11+` for Rekordbox).
    pub deck_id: u8,
    /// Last reported device name.
    pub device_name: String,
    /// Current effective BPM.
    pub bpm: f32,
    /// Continuous phase within the current beat (`[0.0, 1.0)`).
    pub beat_phase: f32,
    /// Current beat within the 4/4 bar (`1..=4`, or `0` when no beat has arrived).
    pub bar_beat: u8,
    /// Whether this deck is currently the tempo master.
    pub is_master: bool,
    /// Whether this deck is currently playing.
    pub playing: bool,
    /// Normalized phrase energy (`0.0..=1.0`).
    pub phrase_energy: f32,
    /// Exponentially decaying cue / downbeat pulse (`0.0..=1.0`).
    pub cue_pulse: f32,
    /// Milliseconds to next beat from the last Pro DJ Link packet.
    pub next_beat_ms: u32,
    /// Last reported playback pitch multiplier (`1.0` = 0% pitch).
    pub pitch_multiplier: f32,
    /// Exponential decay time constant (seconds) for `cue_pulse`.
    cue_pulse_decay_secs: f32,
}

impl RekordboxDeckState {
    /// Creates an idle deck state for `deck_id`.
    #[must_use]
    pub fn new(deck_id: u8) -> Self {
        Self {
            deck_id,
            device_name: String::new(),
            bpm: 0.0,
            beat_phase: 0.0,
            bar_beat: 0,
            is_master: false,
            playing: false,
            phrase_energy: 0.0,
            cue_pulse: 0.0,
            next_beat_ms: 0,
            pitch_multiplier: 1.0,
            cue_pulse_decay_secs: DEFAULT_CUE_PULSE_DECAY_SECS,
        }
    }

    /// Applies a decoded [`ProDjLinkBeat`] packet, snapping `beat_phase` to `0.0`
    /// and pulsing `cue_pulse` on bar downbeats (`beat_within_bar == 1`).
    pub fn apply_beat_packet(&mut self, beat: &ProDjLinkBeat) {
        self.deck_id = beat.player_number;
        self.device_name.clone_from(&beat.device_name);
        self.bpm = if beat.effective_bpm.is_finite() && beat.effective_bpm >= 0.0 {
            beat.effective_bpm
        } else {
            0.0
        };
        self.beat_phase = 0.0;
        self.bar_beat = beat.beat_within_bar;
        self.playing = self.bpm > 0.0;
        self.next_beat_ms = beat.next_beat_ms;
        self.pitch_multiplier = if beat.pitch_multiplier.is_finite() {
            beat.pitch_multiplier
        } else {
            1.0
        };
        if beat.beat_within_bar == 1 {
            self.cue_pulse = 1.0;
        }
    }

    /// Marks whether this deck is the active tempo master.
    pub const fn set_master(&mut self, is_master: bool) {
        self.is_master = is_master;
    }

    /// Updates normalized phrase energy, clamping to `[0.0, 1.0]`.
    pub fn set_phrase_energy(&mut self, energy: f32) {
        self.phrase_energy = if energy.is_finite() {
            energy.clamp(0.0, 1.0)
        } else {
            0.0
        };
    }

    /// Triggers a cue / hot-cue visual pulse (`0.0..=1.0`), taking the max of
    /// the current pulse and `intensity`.
    pub fn trigger_cue_pulse(&mut self, intensity: f32) {
        if intensity.is_finite() {
            let clamped = intensity.clamp(0.0, 1.0);
            if clamped > self.cue_pulse {
                self.cue_pulse = clamped;
            }
        }
    }

    /// Deterministically advances beat phase and decays `cue_pulse` by `dt_secs`.
    ///
    /// Non-positive or non-finite `dt_secs` values are ignored safely.
    pub fn advance(&mut self, dt_secs: f32) {
        if !dt_secs.is_finite() || dt_secs <= 0.0 {
            return;
        }

        if self.playing && self.bpm > 0.0 && self.bpm.is_finite() {
            let beats_delta = dt_secs * (self.bpm / 60.0);
            let total = self.beat_phase + beats_delta;
            let whole_beats = total.floor();
            let wrapped = total - whole_beats;
            self.beat_phase = if wrapped.is_finite() {
                wrapped.clamp(0.0, 1.0 - f32::EPSILON)
            } else {
                0.0
            };

            if whole_beats >= 1.0 && self.bar_beat >= 1 {
                let steps = (whole_beats as u64) % 4;
                let zero_based = (u64::from(self.bar_beat - 1) + steps) % 4;
                self.bar_beat = (zero_based as u8) + 1;
            }
        }

        if self.cue_pulse > 0.0 && self.cue_pulse_decay_secs > 0.0 {
            let decayed = self.cue_pulse * (-dt_secs / self.cue_pulse_decay_secs).exp();
            self.cue_pulse = if decayed < PULSE_ZERO_EPSILON || !decayed.is_finite() {
                0.0
            } else {
                decayed
            };
        }
    }

    /// Produces the current [`RekordboxUniforms`] snapshot for shader binding.
    #[must_use]
    pub fn to_uniforms(&self) -> RekordboxUniforms {
        RekordboxUniforms {
            deck_bpm: if self.bpm.is_finite() && self.bpm >= 0.0 {
                self.bpm
            } else {
                0.0
            },
            deck_beat_phase: if self.beat_phase.is_finite() {
                self.beat_phase.clamp(0.0, 1.0)
            } else {
                0.0
            },
            deck_bar_beat: f32::from(self.bar_beat),
            deck_is_master: if self.is_master { 1.0 } else { 0.0 },
            deck_playing: if self.playing { 1.0 } else { 0.0 },
            deck_phrase_energy: if self.phrase_energy.is_finite() {
                self.phrase_energy.clamp(0.0, 1.0)
            } else {
                0.0
            },
            deck_cue_pulse: if self.cue_pulse.is_finite() {
                self.cue_pulse.clamp(0.0, 1.0)
            } else {
                0.0
            },
        }
    }
}

#[cfg(test)]
mod tests {
    //! Evidence: EV-REKORDBOX-001 (T1) — Rekordbox multi-channel Master/Cue isolation, DJ device profiles, and Pro DJ Link beat packet parser pins.

    use super::*;

    /// Helper that builds a valid 96-byte Pro DJ Link beat packet for testing.
    fn build_beat_packet(
        device_name: &str,
        player_number: u8,
        bpm_centi: u16,
        pitch_raw: u32,
        beat_within_bar: u8,
        next_beat_ms: u32,
        next_bar_ms: u32,
    ) -> [u8; PRO_DJ_LINK_BEAT_PACKET_MIN_LEN] {
        let mut pkt = [0_u8; PRO_DJ_LINK_BEAT_PACKET_MIN_LEN];
        pkt[0..10].copy_from_slice(PRO_DJ_LINK_MAGIC);
        pkt[0x0a] = PRO_DJ_LINK_BEAT_PACKET_TYPE;
        let name_bytes = device_name.as_bytes();
        let copy_len = name_bytes.len().min(20);
        pkt[0x0b..0x0b + copy_len].copy_from_slice(&name_bytes[..copy_len]);
        pkt[0x1f] = 0x01;
        pkt[0x21] = player_number;
        pkt[0x22..0x24].copy_from_slice(&60_u16.to_be_bytes());
        pkt[0x24..0x28].copy_from_slice(&next_beat_ms.to_be_bytes());
        pkt[0x2c..0x30].copy_from_slice(&next_bar_ms.to_be_bytes());
        pkt[0x54..0x58].copy_from_slice(&pitch_raw.to_be_bytes());
        pkt[0x58..0x5a].copy_from_slice(&bpm_centi.to_be_bytes());
        pkt[0x5b] = beat_within_bar;
        pkt[0x5f] = player_number;
        pkt
    }

    #[test]
    fn extractor_isolates_master_from_headphone_cue_on_4ch_controller() {
        let extractor = MultiChannelExtractor::for_device("Pioneer DJ DDJ-FLX10", 4).ok();
        assert!(extractor.is_some());
        let Some(extractor) = extractor else {
            return;
        };
        assert_eq!(extractor.selection(), ChannelPairSelection::new(0, 1));

        // 4 interleaved frames: Master (Ch 1-2) is silent; Headphone Cue (Ch 3-4) is full scale!
        let interleaved: [f32; 16] = [
            0.0, 0.0, 1.0, -1.0, // frame 0
            0.0, 0.0, 1.0, -1.0, // frame 1
            0.0, 0.0, 1.0, -1.0, // frame 2
            0.0, 0.0, 1.0, -1.0, // frame 3
        ];

        let result = extractor.extract(&interleaved);
        assert!(result.is_ok());
        let Ok((stereo, meters)) = result else {
            return;
        };

        // Extracted stereo Master output MUST be completely silent — zero Cue leakage.
        assert_eq!(stereo.len(), 8);
        assert!(stereo.iter().all(|&s| s == 0.0));

        // Live per-pair meters show Ch 1-2 silent and Ch 3-4 active at 0 dBFS.
        assert_eq!(meters.len(), 2);
        assert_eq!(meters[0].label, "Ch 1-2 (Master / REC OUT)");
        assert!(!meters[0].active);
        assert_eq!(meters[0].peak, 0.0);
        assert_eq!(meters[0].peak_dbfs(), METER_MIN_DBFS);

        assert_eq!(meters[1].label, "Ch 3-4 (Headphone Cue / Deck)");
        assert!(meters[1].active);
        assert!((meters[1].peak - 1.0).abs() < 1e-6);
        assert!((meters[1].rms - 1.0).abs() < 1e-6);
        assert!(meters[1].peak_dbfs().abs() < 1e-4);
    }

    #[test]
    fn extractor_supports_10ch_club_mixer_arbitrary_pair_and_sanitizes_non_finite() {
        let mut extractor = match MultiChannelExtractor::for_device("Pioneer DJ DJM-A9", 10) {
            Ok(ex) => ex,
            Err(_) => panic!("valid 10ch extractor must succeed"),
        };

        // Switch selection to Ch 9-10 (0-based 8, 9).
        assert!(
            extractor
                .set_selection(ChannelPairSelection::new(8, 9))
                .is_ok()
        );

        // 2 frames of 10 channels: inject NaN/+Inf on Ch 1-2, real signal on Ch 9-10.
        let mut interleaved = vec![0.0_f32; 20];
        interleaved[0] = f32::NAN;
        interleaved[1] = f32::INFINITY;
        interleaved[8] = 0.5;
        interleaved[9] = -0.25;
        interleaved[18] = 0.5;
        interleaved[19] = -0.25;

        let (stereo, meters) = match extractor.extract(&interleaved) {
            Ok(v) => v,
            Err(_) => panic!("valid 10ch buffer must extract"),
        };

        assert_eq!(stereo, vec![0.5, -0.25, 0.5, -0.25]);
        assert_eq!(meters.len(), 5);
        // Sanitized NaN/Inf pair reports clean zero, never NaN.
        assert_eq!(meters[0].peak, 0.0);
        assert!(meters[0].rms.is_finite());
        // Ch 9-10 meter reflects exact peak & RMS.
        assert!((meters[4].peak_left - 0.5).abs() < 1e-6);
        assert!((meters[4].peak_right - 0.25).abs() < 1e-6);
        assert!(meters[4].active);
        assert_eq!(meters[4].label, "Ch 9-10 (Aux / Monitor)");
    }

    #[test]
    fn extractor_rejects_invalid_channel_config_and_ragged_buffers() {
        assert_eq!(
            MultiChannelExtractor::new(0, ChannelPairSelection::master_default()).err(),
            Some(RekordboxBridgeError::ZeroChannels)
        );
        assert_eq!(
            MultiChannelExtractor::new(2, ChannelPairSelection::new(2, 3)).err(),
            Some(RekordboxBridgeError::ChannelOutOfRange {
                left_channel: 2,
                right_channel: 3,
                channel_count: 2,
            })
        );

        let extractor = match MultiChannelExtractor::new(4, ChannelPairSelection::master_default())
        {
            Ok(ex) => ex,
            Err(_) => panic!("valid 4ch extractor"),
        };
        assert_eq!(
            extractor.extract(&[0.1, 0.2, 0.3]).err(),
            Some(RekordboxBridgeError::RaggedFrameBuffer {
                sample_count: 3,
                channel_count: 4,
            })
        );
    }

    #[test]
    fn device_profile_recognizes_ddj_flx10_and_major_pioneer_hardware() {
        // First-class DDJ-FLX10 matching (case-insensitive, full & short names).
        for name in ["DDJ-FLX10", "ddj-flx10", "Pioneer DJ DDJ FLX10", "FLX10"] {
            let profile = DjDeviceProfile::recognize(name);
            assert!(profile.is_some(), "expected DDJ-FLX10 match for {name}");
            if let Some(p) = profile {
                assert_eq!(p.kind, DjDeviceKind::DdjFlx10);
                assert_eq!(p.master_pair, ChannelPairSelection::new(0, 1));
                assert_eq!(p.cue_pair, Some(ChannelPairSelection::new(2, 3)));
                assert!(p.supports_pc_master_out);
                assert!(p.notes.contains("PC MASTER OUT"));
            }
        }

        let cases = [
            ("DDJ-FLX4", DjDeviceKind::DdjFlx4),
            ("Pioneer DJ DDJ-1000", DjDeviceKind::Ddj1000),
            ("AlphaTheta OPUS-QUAD", DjDeviceKind::OpusQuad),
            ("Opus Quad", DjDeviceKind::OpusQuad),
            ("XDJ-XZ", DjDeviceKind::XdjXz),
            ("DJM-A9", DjDeviceKind::DjmA9),
            ("DJM-900NXS2", DjDeviceKind::Djm900Nxs2),
            (
                "Rekordbox PC Master Out",
                DjDeviceKind::RekordboxPcMasterOut,
            ),
            (
                REKORDBOX_MACOS_BUNDLE_ID,
                DjDeviceKind::RekordboxPcMasterOut,
            ),
        ];

        for (name, expected_kind) in cases {
            let profile = DjDeviceProfile::recognize(name);
            assert_eq!(profile.map(|p| p.kind), Some(expected_kind), "for {name}");
        }

        // Unknown device returns None from recognize() and safe fallback from recognize_or_default().
        assert!(DjDeviceProfile::recognize("Scarlett 2i2 USB").is_none());
        let fallback = DjDeviceProfile::recognize_or_default("Scarlett 4i4", 4);
        assert_eq!(fallback.kind, DjDeviceKind::GenericMultiChannel);
        assert_eq!(fallback.master_pair, ChannelPairSelection::new(0, 1));
        assert_eq!(fallback.cue_pair, Some(ChannelPairSelection::new(2, 3)));
    }

    #[test]
    fn pro_dj_link_beat_packet_parser_decodes_valid_and_rejects_malformed() {
        // 128.00 BPM at +5% pitch (0x0010_0000 * 1.05 = 1_101_005)
        let pitch_plus_5pct: u32 = 1_101_005;
        let pkt = build_beat_packet("CDJ-3000", 2, 12_800, pitch_plus_5pct, 1, 446, 1785);

        let parsed = parse_pro_dj_link_beat_packet(&pkt);
        assert!(parsed.is_some());
        if let Some(beat) = parsed {
            assert_eq!(beat.device_name, "CDJ-3000");
            assert_eq!(beat.player_number, 2);
            assert_eq!(beat.next_beat_ms, 446);
            assert_eq!(beat.next_bar_ms, 1785);
            assert!((beat.track_bpm - 128.0).abs() < 1e-4);
            assert!((beat.pitch_multiplier - 1.05).abs() < 1e-4);
            assert!((beat.effective_bpm - 134.4).abs() < 0.05);
            assert_eq!(beat.beat_within_bar, 1);
        }

        // Truncated packet must return None.
        assert!(parse_pro_dj_link_beat_packet(&pkt[..95]).is_none());

        // Bad magic header must return None.
        let mut bad_magic = pkt;
        bad_magic[0] = b'X';
        assert!(parse_pro_dj_link_beat_packet(&bad_magic).is_none());

        // Wrong packet type (e.g. keep-alive 0x06) must return None.
        let mut keepalive = pkt;
        keepalive[0x0a] = 0x06;
        assert!(parse_pro_dj_link_beat_packet(&keepalive).is_none());

        // Invalid beat_within_bar (0 or 5) must return None.
        let mut bad_beat_0 = pkt;
        bad_beat_0[0x5b] = 0;
        assert!(parse_pro_dj_link_beat_packet(&bad_beat_0).is_none());

        let mut bad_beat_5 = pkt;
        bad_beat_5[0x5b] = 5;
        assert!(parse_pro_dj_link_beat_packet(&bad_beat_5).is_none());
    }

    #[test]
    fn deck_state_advances_phase_decays_cue_pulse_and_emits_all_seven_uniforms() {
        let pkt = build_beat_packet("rekordbox", 1, 12_000, 0x0010_0000, 1, 500, 2000);
        let beat = match parse_pro_dj_link_beat_packet(&pkt) {
            Some(b) => b,
            None => panic!("valid beat packet"),
        };

        let mut deck = RekordboxDeckState::new(1);
        deck.set_master(true);
        deck.set_phrase_energy(0.75);
        deck.apply_beat_packet(&beat);

        // Immediately after downbeat packet: BPM=120, phase=0, bar_beat=1, cue_pulse=1.0.
        let u0 = deck.to_uniforms();
        assert!((u0.deck_bpm - 120.0).abs() < 1e-4);
        assert_eq!(u0.deck_beat_phase, 0.0);
        assert_eq!(u0.deck_bar_beat, 1.0);
        assert_eq!(u0.deck_is_master, 1.0);
        assert_eq!(u0.deck_playing, 1.0);
        assert_eq!(u0.deck_phrase_energy, 0.75);
        assert_eq!(u0.deck_cue_pulse, 1.0);

        // Advance 0.75 seconds at 120 BPM (2 beats/sec) = 1.5 beats -> bar_beat=2, phase=0.5.
        deck.advance(0.75);
        let u1 = deck.to_uniforms();
        assert!((u1.deck_beat_phase - 0.5).abs() < 1e-4);
        assert_eq!(u1.deck_bar_beat, 2.0);
        assert!(u1.deck_cue_pulse < 1.0 && u1.deck_cue_pulse > 0.0);

        // Verify exact ADR-017 uniform key export.
        let map = u1.to_data_map();
        assert_eq!(map.len(), 7);
        assert_eq!(
            map.get(UNIFORM_DECK_BPM),
            Some(&DataValue::Scalar(u1.deck_bpm))
        );
        assert_eq!(
            map.get(UNIFORM_DECK_BEAT_PHASE),
            Some(&DataValue::Scalar(u1.deck_beat_phase))
        );
        assert_eq!(
            map.get(UNIFORM_DECK_BAR_BEAT),
            Some(&DataValue::Scalar(u1.deck_bar_beat))
        );
        assert_eq!(
            map.get(UNIFORM_DECK_IS_MASTER),
            Some(&DataValue::Scalar(u1.deck_is_master))
        );
        assert_eq!(
            map.get(UNIFORM_DECK_PLAYING),
            Some(&DataValue::Scalar(u1.deck_playing))
        );
        assert_eq!(
            map.get(UNIFORM_DECK_PHRASE_ENERGY),
            Some(&DataValue::Scalar(u1.deck_phrase_energy))
        );
        assert_eq!(
            map.get(UNIFORM_DECK_CUE_PULSE),
            Some(&DataValue::Scalar(u1.deck_cue_pulse))
        );
    }

    #[test]
    fn channel_pair_selection_one_based_helpers() {
        assert_eq!(
            ChannelPairSelection::from_one_based(1, 2),
            Some(ChannelPairSelection::new(0, 1))
        );
        assert_eq!(ChannelPairSelection::from_one_based(0, 2), None);
        assert_eq!(ChannelPairSelection::new(0, 1).one_based_label(), "Ch 1-2");
        assert_eq!(ChannelPairSelection::new(2, 3).one_based_label(), "Ch 3-4");
        assert_eq!(ChannelPairSelection::new(0, 0).one_based_label(), "Ch 1");
    }
}
