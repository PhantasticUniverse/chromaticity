// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Persisted operator settings — TOML-serialised preferences in the
//! per-user app data dir.
//!
//! Wave 2.1 Lane C.3 (`/Users/xaviermac/.claude/plans/inherited-percolating-quill.md`)
//! introduces this module so the Settings view's Audio / Graphics /
//! Storage / General form state survives restart. Sits OUTSIDE the
//! ADR-004 canonical [`crate::state::State`] core because operator
//! preferences are not part of the event-sourced state model — they
//! live alongside the canonical state as a separate
//! `Arc<Mutex<Settings>>` on the shell-side [`AppState`], persisted
//! through file I/O rather than committed-event mutation.
//!
//! Field set mirrors `src/lib/data/settings-fixtures.ts` (the
//! frontend's bundle-faithful fixture corpus) so the Settings view's
//! existing controls map 1:1 onto the persisted shape. The bridge
//! type [`chromaticity_ui_bridge::SettingsPayload`] (Wave 2.1 Lane
//! C.3) carries the ts-rs newtype + bidirectional `From` impls so the
//! IPC seam stays free of engine-only concerns.
//!
//! ## Format choice
//!
//! TOML over JSON for operator-editable persistence: humans can
//! correct a corrupted file by hand and the format tolerates
//! comments + trailing commas more gracefully than `serde_json`.
//! Forward-compat: new fields land with `#[serde(default)]` so a
//! settings file written under an older app version deserialises
//! cleanly under a newer one.
//!
//! ## Error model
//!
//! [`Settings::load_from_path`] returns [`SettingsError::Io`] for
//! filesystem failures and [`SettingsError::Parse`] for malformed
//! TOML. [`Settings::load_or_default`] is the launch helper: a
//! missing file (initial launch) returns [`Settings::default`], and
//! a malformed file is first quarantined to
//! `settings.toml.malformed-<timestamp>` before falling back to
//! defaults so the next write cannot silently destroy the damaged
//! operator file.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chromaticity_osc_adapter::defaults::{DEFAULT_OSC_IN_PORT, DEFAULT_OSC_OUT_PORT};

/// Three-theme cascade matching the existing frontend
/// `ThemeSwitcher` (`src/lib/ThemeSwitcher.svelte`). Pulse is the
/// bundle default; Atelier and Console are the v0.2-beta sweep's
/// alternative aesthetics per `feedback_chromaticity_themes_for_fun.md`.
///
/// Serialised as a flat string ("pulse" / "atelier" / "console") via
/// `#[serde(rename_all = "kebab-case")]` to keep the TOML wire shape
/// human-readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    Pulse,
    Atelier,
    Console,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Pulse
    }
}

/// Audio capture / monitoring preferences.
///
/// `input_device` / `output_device` carry the operator's chosen cpal
/// device identifier (free-text string at Wave 2.1 Lane C.3 — live
/// cpal enumeration lands in a follow-up wave). `sample_rate_hz` +
/// `latency_hint` + `channel_layout` mirror the bundle-faithful
/// fixture options in `src/lib/data/settings-fixtures.ts`.
/// `channel_left` / `channel_right` realise the W39 routing pickers.
/// `input_gain` is `[0, 1]` (0.5 == unity; ±18 dB symmetric mapping
/// per the fixture-side `AUDIO_INPUT_GAIN_DB_RANGE_DB`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSettings {
    #[serde(default = "AudioSettings::default_input_device")]
    pub input_device: String,
    #[serde(default = "AudioSettings::default_output_device")]
    pub output_device: String,
    #[serde(default = "AudioSettings::default_sample_rate_hz")]
    pub sample_rate_hz: u32,
    #[serde(default = "AudioSettings::default_latency_hint")]
    pub latency_hint: String,
    #[serde(default = "AudioSettings::default_channel_layout")]
    pub channel_layout: String,
    #[serde(default = "AudioSettings::default_channel_left")]
    pub channel_left: String,
    #[serde(default = "AudioSettings::default_channel_right")]
    pub channel_right: String,
    #[serde(default = "AudioSettings::default_input_gain")]
    pub input_gain: f32,
}

impl AudioSettings {
    fn default_input_device() -> String {
        "default".to_string()
    }
    fn default_output_device() -> String {
        "default".to_string()
    }
    fn default_sample_rate_hz() -> u32 {
        48_000
    }
    fn default_latency_hint() -> String {
        "balanced".to_string()
    }
    fn default_channel_layout() -> String {
        "stereo".to_string()
    }
    fn default_channel_left() -> String {
        "L1".to_string()
    }
    fn default_channel_right() -> String {
        "R2".to_string()
    }
    fn default_input_gain() -> f32 {
        0.59
    }
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            input_device: Self::default_input_device(),
            output_device: Self::default_output_device(),
            sample_rate_hz: Self::default_sample_rate_hz(),
            latency_hint: Self::default_latency_hint(),
            channel_layout: Self::default_channel_layout(),
            channel_left: Self::default_channel_left(),
            channel_right: Self::default_channel_right(),
            input_gain: Self::default_input_gain(),
        }
    }
}

/// Graphics / output preferences.
///
/// `resolution_preset` carries the fixture id (`720p` / `1080p` /
/// `1440p` / `4k`); `custom_resolution_enabled` + `custom_width` /
/// `custom_height` realise the bundle's mutually-exclusive override.
/// `fps_target` ids match the frontend fixture (`30` / `60` / `120` /
/// `144` / `unlimited`). `color_space` / `multi_display_mode` /
/// `screen_fitting` mirror the W41 expansion fixture ids.
/// `render_scale` is `[RENDER_SCALE_MIN, RENDER_SCALE_MAX]` from
/// the fixture (0.5 .. 2.0).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphicsSettings {
    #[serde(default = "GraphicsSettings::default_resolution_preset")]
    pub resolution_preset: String,
    #[serde(default)]
    pub custom_resolution_enabled: bool,
    #[serde(default = "GraphicsSettings::default_custom_width")]
    pub custom_width: u32,
    #[serde(default = "GraphicsSettings::default_custom_height")]
    pub custom_height: u32,
    #[serde(default = "GraphicsSettings::default_fps_target")]
    pub fps_target: String,
    #[serde(default = "GraphicsSettings::default_vsync")]
    pub vsync: bool,
    #[serde(default = "GraphicsSettings::default_hdr_target")]
    pub hdr_target: String,
    #[serde(default = "GraphicsSettings::default_display")]
    pub display: String,
    #[serde(default = "GraphicsSettings::default_color_space")]
    pub color_space: String,
    #[serde(default = "GraphicsSettings::default_multi_display_mode")]
    pub multi_display_mode: String,
    #[serde(default = "GraphicsSettings::default_render_scale")]
    pub render_scale: f32,
    #[serde(default = "GraphicsSettings::default_screen_fitting")]
    pub screen_fitting: String,
    #[serde(default)]
    pub fps_slider_enabled: bool,
}

impl GraphicsSettings {
    fn default_resolution_preset() -> String {
        "1080p".to_string()
    }
    fn default_custom_width() -> u32 {
        1920
    }
    fn default_custom_height() -> u32 {
        1080
    }
    fn default_fps_target() -> String {
        "60".to_string()
    }
    fn default_vsync() -> bool {
        true
    }
    fn default_hdr_target() -> String {
        "sdr".to_string()
    }
    fn default_display() -> String {
        "display-1".to_string()
    }
    fn default_color_space() -> String {
        "srgb".to_string()
    }
    fn default_multi_display_mode() -> String {
        "single".to_string()
    }
    fn default_render_scale() -> f32 {
        1.0
    }
    fn default_screen_fitting() -> String {
        "fit".to_string()
    }
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            resolution_preset: Self::default_resolution_preset(),
            custom_resolution_enabled: false,
            custom_width: Self::default_custom_width(),
            custom_height: Self::default_custom_height(),
            fps_target: Self::default_fps_target(),
            vsync: Self::default_vsync(),
            hdr_target: Self::default_hdr_target(),
            display: Self::default_display(),
            color_space: Self::default_color_space(),
            multi_display_mode: Self::default_multi_display_mode(),
            render_scale: Self::default_render_scale(),
            screen_fitting: Self::default_screen_fitting(),
            fps_slider_enabled: false,
        }
    }
}

/// Storage / file-layout preferences.
///
/// `media_folders` + `scene_folders` carry the operator's added
/// folder paths (the frontend manages the row ids; the persisted
/// shape is just the path list). `backup_path` + `backup_retention`
/// realise the Storage section's Backups group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageSettings {
    #[serde(default)]
    pub media_folders: Vec<String>,
    #[serde(default)]
    pub scene_folders: Vec<String>,
    #[serde(default = "StorageSettings::default_backup_path")]
    pub backup_path: String,
    #[serde(default = "StorageSettings::default_backup_retention")]
    pub backup_retention: String,
}

impl StorageSettings {
    fn default_backup_path() -> String {
        // Empty string == "use the OS default" sentinel; the shell-side
        // handler can substitute the resolved app data dir's
        // `Backups/` subdirectory when the operator has not chosen
        // a custom path.
        String::new()
    }
    fn default_backup_retention() -> String {
        "retain-10".to_string()
    }
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            media_folders: Vec::new(),
            scene_folders: Vec::new(),
            backup_path: Self::default_backup_path(),
            backup_retention: Self::default_backup_retention(),
        }
    }
}

/// General operator preferences — theme + miscellaneous toggles
/// that don't fit the Audio / Graphics / Storage sections.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GeneralSettings {
    #[serde(default)]
    pub theme: Theme,
}

/// Live Sources / Spout-Syphon / NDI operator preferences.
///
/// Wave 2.7 SET.B: the engine-side capability gating for Spout / Syphon
/// / NDI input + output is a Wave 1c-β follow-up (per-platform binding
/// stubs + Syphon `Send`-trait reconciliation, per
/// `docs/EV-GATE-MODEL-AUDIT-checklist.md` Phase-4a Wave 1c-β closure
/// section); the UI toggles + sender name need to persist NOW so
/// operators don't keep re-flipping the same six prefs across restarts.
/// This struct carries the persisted preferences only — the cross-seam
/// IPC that actually publishes / subscribes to the sources rides in
/// alongside the deferred follow-ups.
///
/// Field set mirrors the Live Sources section in
/// `src/lib/views/Settings.svelte` (six fields total — five booleans +
/// one short string): Spout / Syphon Input + Output, NDI Input +
/// Output, Constant Output Resolution, Sender Name. The
/// `data-cap=...` attributes on the view rows (`io.syphon.input`,
/// `io.syphon.output`, `io.ndi.input`, `io.ndi.output`,
/// `render.custom-resolution`) carry forward unchanged; entitlement-
/// gated enforcement attaches at the future SKU rollout per W15+ work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSourcesSettings {
    #[serde(default = "LiveSourcesSettings::default_syphon_input_enabled")]
    pub syphon_input_enabled: bool,
    #[serde(default = "LiveSourcesSettings::default_syphon_output_enabled")]
    pub syphon_output_enabled: bool,
    #[serde(default)]
    pub ndi_input_enabled: bool,
    #[serde(default = "LiveSourcesSettings::default_ndi_output_enabled")]
    pub ndi_output_enabled: bool,
    #[serde(default)]
    pub constant_output_resolution: bool,
    #[serde(default = "LiveSourcesSettings::default_sender_name")]
    pub sender_name: String,
}

impl LiveSourcesSettings {
    fn default_syphon_input_enabled() -> bool {
        true
    }
    fn default_syphon_output_enabled() -> bool {
        true
    }
    fn default_ndi_output_enabled() -> bool {
        true
    }
    fn default_sender_name() -> String {
        // Mirrors `SENDER_NAME_DEFAULT` in
        // `src/lib/data/settings-fixtures.ts` so the engine-side
        // default and the bundle-faithful fixture default agree.
        "chrm".to_string()
    }
}

impl Default for LiveSourcesSettings {
    fn default() -> Self {
        Self {
            syphon_input_enabled: Self::default_syphon_input_enabled(),
            syphon_output_enabled: Self::default_syphon_output_enabled(),
            ndi_input_enabled: false,
            ndi_output_enabled: Self::default_ndi_output_enabled(),
            constant_output_resolution: false,
            sender_name: Self::default_sender_name(),
        }
    }
}

/// OSC adapter operator-facing network defaults — Wave 2.1 Lane D.3.
///
/// `local_ip` carries the bind-side interface (ADR-013 §1 default is
/// `127.0.0.1` loopback; non-loopback values are an explicit operator
/// opt-in for routable inbound). `port` carries the bind-side inbound
/// UDP port and `out_port` carries the default outbound target port;
/// both values come from `chromaticity-osc-adapter` so fresh-install
/// settings and adapter defaults cannot drift. `tx_enabled` gates
/// outbound transmission; `rx_enabled` gates inbound packet processing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OscSettings {
    #[serde(default = "OscSettings::default_local_ip")]
    pub local_ip: String,
    #[serde(default = "OscSettings::default_port")]
    pub port: u16,
    #[serde(default = "OscSettings::default_out_port")]
    pub out_port: u16,
    #[serde(default = "OscSettings::default_tx_enabled")]
    pub tx_enabled: bool,
    #[serde(default = "OscSettings::default_rx_enabled")]
    pub rx_enabled: bool,
}

impl OscSettings {
    fn default_local_ip() -> String {
        "127.0.0.1".to_string()
    }
    fn default_port() -> u16 {
        DEFAULT_OSC_IN_PORT
    }
    fn default_out_port() -> u16 {
        DEFAULT_OSC_OUT_PORT
    }
    fn default_tx_enabled() -> bool {
        true
    }
    fn default_rx_enabled() -> bool {
        true
    }
}

impl Default for OscSettings {
    fn default() -> Self {
        Self {
            local_ip: Self::default_local_ip(),
            port: Self::default_port(),
            out_port: Self::default_out_port(),
            tx_enabled: Self::default_tx_enabled(),
            rx_enabled: Self::default_rx_enabled(),
        }
    }
}

/// Tempo & Sync operator preferences — Wave 2.7 Lane SET.A.
///
/// Mirrors the Settings view's Tempo & Sync section (W22 PR1 + W40
/// expansion). Carries the field set the operator wires through the
/// frontend's Tempo controls: Ableton Link toggle + Start/Stop Sync,
/// MIDI Clock input + output (each toggle + device pick), Quantize
/// Transitions, Manual BPM, Link offset (ms), Audio BPM sensitivity
/// (%), and the BPM source priority chain.
///
/// `bpm_source_priority` is a ranked list of source-id strings
/// matching the frontend's `BpmSourceId` union (`"audio"` /
/// `"midiclock"` / `"link"` / `"manual"`). The fixed W22 order today
/// is `["audio", "midiclock", "link", "manual"]`; the operator-
/// reorder + per-source health surfaces ride the Phase-3+ M-10 BPM
/// resolver (per `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/midi.jsx`).
/// Live runtime arbitration (Audio estimator, MIDI Clock, Ableton
/// Link peer discovery) lands alongside that resolver — Wave 2.7
/// closes only the persistence seam so operator-tuned values
/// survive restart.
///
/// `quantize` carries the QuantizeTransitionOption id ("off" /
/// "beat" / "bar" / "phrase") matching the frontend's tab-segment
/// fixture. `manual_bpm` is the operator-entered tap-tempo /
/// numeric BPM (clamped 20..=300 on the frontend). `link_offset_ms`
/// is signed (downstream-link latency compensation per the W40
/// hint copy). `audio_bpm_sensitivity` is the audio onset
/// estimator's tuning weight as a 0..=100 integer percent — the
/// frontend's BPM-sense fader maps `[0, 1]` to `[0, 100]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TempoSettings {
    #[serde(default = "TempoSettings::default_ableton_link_enabled")]
    pub ableton_link_enabled: bool,
    #[serde(default = "TempoSettings::default_start_stop_sync")]
    pub start_stop_sync: bool,
    #[serde(default)]
    pub midi_clock_in_enabled: bool,
    #[serde(default = "TempoSettings::default_midi_clock_in_device")]
    pub midi_clock_in_device: String,
    #[serde(default = "TempoSettings::default_midi_clock_out_enabled")]
    pub midi_clock_out_enabled: bool,
    #[serde(default = "TempoSettings::default_midi_clock_out_device")]
    pub midi_clock_out_device: String,
    #[serde(default = "TempoSettings::default_quantize")]
    pub quantize: String,
    #[serde(default = "TempoSettings::default_manual_bpm")]
    pub manual_bpm: f32,
    #[serde(default)]
    pub link_offset_ms: f32,
    #[serde(default = "TempoSettings::default_audio_bpm_sensitivity")]
    pub audio_bpm_sensitivity: f32,
    #[serde(default = "TempoSettings::default_bpm_source_priority")]
    pub bpm_source_priority: Vec<String>,
}

impl TempoSettings {
    fn default_ableton_link_enabled() -> bool {
        true
    }
    fn default_start_stop_sync() -> bool {
        true
    }
    fn default_midi_clock_in_device() -> String {
        "any".to_string()
    }
    fn default_midi_clock_out_enabled() -> bool {
        true
    }
    fn default_midi_clock_out_device() -> String {
        "iac-1".to_string()
    }
    fn default_quantize() -> String {
        "bar".to_string()
    }
    fn default_manual_bpm() -> f32 {
        120.0
    }
    fn default_audio_bpm_sensitivity() -> f32 {
        60.0
    }
    fn default_bpm_source_priority() -> Vec<String> {
        vec![
            "audio".to_string(),
            "midiclock".to_string(),
            "link".to_string(),
            "manual".to_string(),
        ]
    }
}

impl Default for TempoSettings {
    fn default() -> Self {
        Self {
            ableton_link_enabled: Self::default_ableton_link_enabled(),
            start_stop_sync: Self::default_start_stop_sync(),
            midi_clock_in_enabled: false,
            midi_clock_in_device: Self::default_midi_clock_in_device(),
            midi_clock_out_enabled: Self::default_midi_clock_out_enabled(),
            midi_clock_out_device: Self::default_midi_clock_out_device(),
            quantize: Self::default_quantize(),
            manual_bpm: Self::default_manual_bpm(),
            link_offset_ms: 0.0,
            audio_bpm_sensitivity: Self::default_audio_bpm_sensitivity(),
            bpm_source_priority: Self::default_bpm_source_priority(),
        }
    }
}

/// Top-level persisted settings document.
///
/// Per-section grouping mirrors the Settings view's article layout
/// (Audio Device / Graphics / Output / Storage / General) so a
/// future hand-edit of the TOML file matches what the operator
/// sees in the UI. Every field defaults to a bundle-faithful value
/// via the nested [`Default`] impls so a freshly-installed app
/// renders the same shape the bundle wireframe shows before the
/// operator touches anything.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub general: GeneralSettings,
    #[serde(default)]
    pub audio: AudioSettings,
    #[serde(default)]
    pub graphics: GraphicsSettings,
    #[serde(default)]
    pub storage: StorageSettings,
    #[serde(default)]
    pub live_sources: LiveSourcesSettings,
    #[serde(default)]
    pub osc: OscSettings,
    #[serde(default)]
    pub tempo: TempoSettings,
}

/// Failure surfaces for [`Settings::load_from_path`] and
/// [`Settings::save_to_path`].
///
/// `Io` carries filesystem failures (permission denied, disk full,
/// path collision with a directory, etc.); `Parse` carries malformed
/// TOML (corrupted file, partial write from a prior crash, manual
/// edit syntax error). `Serialize` carries the (rare) toml-emit
/// failure surfaced by [`Settings::save_to_path`].
///
/// The shell-side handler matches on the error and falls back to
/// [`Settings::default`] on `Parse` so a corrupted file never
/// blocks the app from launching — [`Settings::load_or_default`]
/// quarantines malformed files before returning defaults.
#[derive(Debug)]
pub enum SettingsError {
    /// Filesystem I/O failure.
    Io(std::io::Error),
    /// TOML parse failure (malformed file).
    Parse(toml::de::Error),
    /// TOML serialise failure (encoder rejected the value).
    Serialize(toml::ser::Error),
}

impl core::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "settings io error: {err}"),
            Self::Parse(err) => write!(f, "settings parse error: {err}"),
            Self::Serialize(err) => write!(f, "settings serialise error: {err}"),
        }
    }
}

impl std::error::Error for SettingsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Parse(err) => Some(err),
            Self::Serialize(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for SettingsError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<toml::de::Error> for SettingsError {
    fn from(err: toml::de::Error) -> Self {
        Self::Parse(err)
    }
}

impl From<toml::ser::Error> for SettingsError {
    fn from(err: toml::ser::Error) -> Self {
        Self::Serialize(err)
    }
}

impl Settings {
    /// Read a TOML-serialised [`Settings`] from `path`.
    ///
    /// Returns [`SettingsError::Io`] on filesystem failure (caller
    /// should check `std::io::ErrorKind::NotFound` and fall back to
    /// [`Self::default`] for the missing-file / initial-launch case;
    /// [`Self::load_or_default`] does this automatically) and
    /// [`SettingsError::Parse`] on malformed TOML.
    pub fn load_from_path(path: &Path) -> Result<Self, SettingsError> {
        let contents = std::fs::read_to_string(path)?;
        let parsed: Self = toml::from_str(&contents)?;
        Ok(parsed)
    }

    /// Serialise this [`Settings`] to `path` as TOML via a same-dir
    /// temporary file, `flush` / `sync_all`, then atomic rename.
    ///
    /// Creates any missing parent directories so the first-launch
    /// save path doesn't fail when `~/Library/Application Support/...`
    /// doesn't yet contain the chromaticity subdirectory.
    pub fn save_to_path(&self, path: &Path) -> Result<(), SettingsError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let serialised = toml::to_string_pretty(self)?;
        atomic_write(path, serialised.as_bytes())?;
        Ok(())
    }

    /// Convenience helper: load from `path` if it exists, otherwise
    /// return [`Self::default`].
    ///
    /// A missing file is the initial-launch case and is NOT an
    /// error. A malformed file is quarantined next to the original as
    /// `settings.toml.malformed-<timestamp>` before returning
    /// defaults, so the next save cannot silently overwrite the only
    /// copy of the operator's hand-edited or partially-written file.
    pub fn load_or_default(path: &Path) -> Result<Self, SettingsError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                let parsed = match toml::from_str(&contents) {
                    Ok(parsed) => parsed,
                    Err(_err) => {
                        quarantine_malformed(path)?;
                        return Ok(Self::default());
                    }
                };
                Ok(parsed)
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(SettingsError::Io(err)),
        }
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), SettingsError> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    let tmp_path = temporary_sibling_path(path, "tmp");
    let mut tmp_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)?;
    let write_result = (|| -> Result<(), std::io::Error> {
        tmp_file.write_all(bytes)?;
        tmp_file.flush()?;
        tmp_file.sync_all()?;
        Ok(())
    })();
    if let Err(err) = write_result {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(SettingsError::Io(err));
    }
    drop(tmp_file);
    if let Err(err) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(SettingsError::Io(err));
    }
    if let Some(parent) = parent {
        let _ = std::fs::File::open(parent).and_then(|dir| dir.sync_all());
    }
    Ok(())
}

fn quarantine_malformed(path: &Path) -> Result<PathBuf, SettingsError> {
    let quarantine_path = unique_malformed_path(path);
    std::fs::rename(path, &quarantine_path)?;
    Ok(quarantine_path)
}

fn unique_malformed_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "settings.toml".to_string());
    let timestamp = timestamp_nanos();
    path.with_file_name(format!("{file_name}.malformed-{timestamp}"))
}

fn temporary_sibling_path(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "settings.toml".to_string());
    let pid = std::process::id();
    let timestamp = timestamp_nanos();
    path.with_file_name(format!("{file_name}.{suffix}-{pid}-{timestamp}"))
}

fn timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — Wave 2.1 Lane C.3
    //! settings persistence seam load/save round-trip drift pins.
    //!
    //! T1 per `docs/43 §15.5` covers in-source state-machine
    //! invariants the [`Settings`] type must honour for the
    //! shell-side [`crate::commands::settings`] handlers to compose
    //! correctly:
    //!
    //! - `default_round_trips_through_toml` — the bundle-faithful
    //!   defaults serialise + parse back to the same structural
    //!   value, so a fresh-install settings file written by the
    //!   shell deserialises cleanly on next launch.
    //! - `load_or_default_returns_default_when_file_missing` — the
    //!   initial-launch case (no settings file yet) is NOT an error
    //!   and resolves to [`Settings::default`].
    //! - `load_from_missing_path_returns_io_not_found` — the
    //!   `load_from_path` variant (no fallback) surfaces the missing-
    //!   file case as an `Io` error with `NotFound` kind so the
    //!   shell handler can choose its own fallback policy.
    //! - `malformed_toml_surfaces_parse_error` — a corrupted /
    //!   hand-edited file with invalid TOML syntax surfaces as
    //!   [`SettingsError::Parse`] without panicking.
    //! - `partial_toml_uses_serde_defaults_for_missing_fields` —
    //!   forward-compat: a settings file missing fields a future
    //!   version added (or a hand-edited subset) deserialises with
    //!   the missing fields filled by the per-field
    //!   `#[serde(default)]` annotations.
    //! - `save_to_path_creates_missing_parent_directories` — the
    //!   first-launch save path doesn't fail when the parent
    //!   directory (e.g. `~/Library/Application Support/<bundle id>/`)
    //!   doesn't yet exist.
    //! - `save_then_load_round_trip_preserves_non_default_values` —
    //!   a settings document mutated through the public field API
    //!   serialises + parses back to the same value, pinning the
    //!   on-disk wire shape against silent serde-rename drift.

    use super::{Settings, SettingsError, Theme};
    use chromaticity_osc_adapter::defaults::{DEFAULT_OSC_IN_PORT, DEFAULT_OSC_OUT_PORT};
    use std::io::Write;

    fn tmp_path(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        // Disambiguate test runs against parallel test workers. The
        // PID + a per-test name is sufficient; cleanup is best-effort
        // via the platform tmp dir cycle.
        path.push(format!(
            "chromaticity-settings-test-{}-{}-{}.toml",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        path
    }

    #[test]
    fn default_round_trips_through_toml() {
        let original = Settings::default();
        let serialised = toml::to_string_pretty(&original).unwrap();
        let parsed: Settings = toml::from_str(&serialised).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn load_or_default_returns_default_when_file_missing() {
        let path = tmp_path("missing");
        assert!(!path.exists(), "test precondition: path must not exist");
        let loaded = Settings::load_or_default(&path).unwrap();
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn load_from_missing_path_returns_io_not_found() {
        let path = tmp_path("missing-strict");
        let err = Settings::load_from_path(&path).unwrap_err();
        match err {
            SettingsError::Io(io) => {
                assert_eq!(io.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("expected SettingsError::Io with NotFound, got {other:?}"),
        }
    }

    #[test]
    fn malformed_toml_surfaces_parse_error() {
        let path = tmp_path("malformed");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"this is not valid toml = = =\n").unwrap();
        }
        let err = Settings::load_from_path(&path).unwrap_err();
        match err {
            SettingsError::Parse(_) => {}
            other => panic!("expected SettingsError::Parse, got {other:?}"),
        }
        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_or_default_quarantines_malformed_toml_before_defaulting() {
        let path = tmp_path("malformed-load-default");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"this is not valid toml = = =\n").unwrap();
        }

        let loaded = Settings::load_or_default(&path).unwrap();

        assert_eq!(loaded, Settings::default());
        assert!(
            !path.exists(),
            "malformed source should be moved aside before default fallback",
        );
        let parent = path.parent().unwrap_or_else(|| std::path::Path::new(""));
        let prefix = format!(
            "{}.malformed-",
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "settings.toml".to_string()),
        );
        let quarantined: Vec<std::path::PathBuf> = std::fs::read_dir(parent)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|candidate| {
                candidate
                    .file_name()
                    .map(|name| name.to_string_lossy().starts_with(&prefix))
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(quarantined.len(), 1, "expected one quarantined file");
        let quarantined_contents = std::fs::read_to_string(&quarantined[0]).unwrap();
        assert_eq!(quarantined_contents, "this is not valid toml = = =\n");
        let _ = std::fs::remove_file(&quarantined[0]);
    }

    #[test]
    fn partial_toml_uses_serde_defaults_for_missing_fields() {
        // Forward-compat: a partial document (e.g. hand-edited to
        // override one field, or written by an older app version)
        // deserialises with the missing fields filled by
        // `#[serde(default)]`. Pin this so a future field addition
        // doesn't accidentally break the migration path.
        let path = tmp_path("partial");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"[general]\ntheme = \"console\"\n\n[audio]\nsample_rate_hz = 96000\n")
                .unwrap();
        }
        let loaded = Settings::load_from_path(&path).unwrap();
        assert_eq!(loaded.general.theme, Theme::Console);
        assert_eq!(loaded.audio.sample_rate_hz, 96_000);
        // Missing fields fall through to per-field defaults.
        assert_eq!(loaded.audio.input_device, "default");
        assert_eq!(loaded.audio.latency_hint, "balanced");
        assert_eq!(loaded.graphics.resolution_preset, "1080p");
        assert_eq!(loaded.storage.backup_retention, "retain-10");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_to_path_creates_missing_parent_directories() {
        // The first-launch save target lives under the bundle's app
        // data dir which may not yet exist. Pin that `save_to_path`
        // creates the chain rather than failing with NotFound on the
        // parent. Avoid colliding with other tests by nesting under
        // a per-test name.
        let mut path = std::env::temp_dir();
        path.push(format!(
            "chromaticity-settings-mkdir-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        path.push("nested");
        path.push("settings.toml");
        assert!(
            !path.parent().unwrap().exists(),
            "test precondition: parent must not exist"
        );
        let settings = Settings::default();
        settings.save_to_path(&path).unwrap();
        assert!(path.exists(), "save_to_path must create the file");
        // Reload pins the round trip.
        let loaded = Settings::load_from_path(&path).unwrap();
        assert_eq!(loaded, settings);
        // Cleanup the test tree.
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(path.parent().unwrap());
        let _ = std::fs::remove_dir(path.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn save_then_load_round_trip_preserves_non_default_values() {
        // Mutate a representative field per section so a future
        // field-rename or serde-attribute drift surfaces here.
        let path = tmp_path("round-trip");
        let mut settings = Settings::default();
        settings.general.theme = Theme::Atelier;
        settings.audio.input_device = "BlackHole 16ch".to_string();
        settings.audio.sample_rate_hz = 96_000;
        settings.audio.input_gain = 0.75;
        settings.graphics.resolution_preset = "4k".to_string();
        settings.graphics.fps_target = "120".to_string();
        settings.graphics.vsync = false;
        settings.graphics.render_scale = 1.5;
        settings.storage.media_folders = vec![
            "/Users/operator/Movies/A".to_string(),
            "/Users/operator/Movies/B".to_string(),
        ];
        settings.storage.scene_folders = vec!["/Users/operator/Scenes".to_string()];
        settings.storage.backup_path = "/Users/operator/Backups".to_string();
        settings.storage.backup_retention = "retain-25".to_string();

        settings.save_to_path(&path).unwrap();
        let loaded = Settings::load_from_path(&path).unwrap();
        assert_eq!(loaded, settings);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn theme_serialises_as_kebab_case() {
        // Pin the wire shape for the Theme enum so a future
        // `#[serde(rename = "...")]` change doesn't silently
        // break operator-edited config files. The frontend
        // ThemeSwitcher reads "pulse" / "atelier" / "console"
        // strings from its select; the bridge mirrors them
        // verbatim.
        //
        // toml 0.9 requires a top-level struct (a bare enum isn't a
        // valid TOML document), so route through a full `Settings`
        // round-trip and inspect the serialised form for the
        // expected kebab-case discriminant.
        let mut settings = Settings::default();
        settings.general.theme = Theme::Console;
        let serialised = toml::to_string(&settings).unwrap();
        assert!(
            serialised.contains("theme = \"console\""),
            "expected theme to serialise as \"console\": got {serialised}"
        );
        // Parse-back from a hand-crafted partial doc verifies the
        // kebab-case discriminants accepted at the deserialise edge.
        let parsed: Settings = toml::from_str("[general]\ntheme = \"atelier\"\n").unwrap();
        assert_eq!(parsed.general.theme, Theme::Atelier);
    }

    #[test]
    fn osc_settings_default_matches_bundle_baseline() {
        // ADR-013 §1: the runtime/UI OSC default is loopback-only;
        // binding a routable interface is an explicit operator
        // opt-in. The frontend reads back these values from the
        // settings store on view mount and renders them in the OSC
        // view's listening / send-to readouts.
        let defaults = Settings::default();
        assert_eq!(defaults.osc.local_ip, "127.0.0.1");
        assert_eq!(defaults.osc.port, DEFAULT_OSC_IN_PORT);
        assert_eq!(defaults.osc.out_port, DEFAULT_OSC_OUT_PORT);
        assert!(defaults.osc.tx_enabled);
        assert!(defaults.osc.rx_enabled);
    }

    #[test]
    fn osc_settings_round_trip_preserves_operator_values() {
        // Mutating each OSC field and round-tripping through TOML
        // pins the field-name + serde-attribute stability of the
        // [osc] section against future drift.
        let path = tmp_path("osc-round-trip");
        let mut settings = Settings::default();
        settings.osc.local_ip = "192.168.1.42".to_string();
        settings.osc.port = 9100;
        settings.osc.out_port = 9101;
        settings.osc.tx_enabled = false;
        settings.osc.rx_enabled = true;
        settings.save_to_path(&path).unwrap();
        let loaded = Settings::load_from_path(&path).unwrap();
        assert_eq!(loaded.osc.local_ip, "192.168.1.42");
        assert_eq!(loaded.osc.port, 9100);
        assert_eq!(loaded.osc.out_port, 9101);
        assert!(!loaded.osc.tx_enabled);
        assert!(loaded.osc.rx_enabled);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tempo_settings_default_matches_bundle_baseline() {
        // Wave 2.7 Lane SET.A: pin the bundle-faithful Tempo & Sync
        // defaults. The frontend reads back these values from the
        // settings store on view mount and renders them in the
        // Settings view's Tempo & Sync controls.
        let defaults = Settings::default();
        assert!(defaults.tempo.ableton_link_enabled);
        assert!(defaults.tempo.start_stop_sync);
        assert!(!defaults.tempo.midi_clock_in_enabled);
        assert_eq!(defaults.tempo.midi_clock_in_device, "any");
        assert!(defaults.tempo.midi_clock_out_enabled);
        assert_eq!(defaults.tempo.midi_clock_out_device, "iac-1");
        assert_eq!(defaults.tempo.quantize, "bar");
        assert_eq!(defaults.tempo.manual_bpm, 120.0);
        assert_eq!(defaults.tempo.link_offset_ms, 0.0);
        assert_eq!(defaults.tempo.audio_bpm_sensitivity, 60.0);
        assert_eq!(
            defaults.tempo.bpm_source_priority,
            vec![
                "audio".to_string(),
                "midiclock".to_string(),
                "link".to_string(),
                "manual".to_string(),
            ]
        );
    }

    #[test]
    fn tempo_settings_round_trip_preserves_operator_values() {
        // Mutating each Tempo field and round-tripping through TOML
        // pins the field-name + serde-attribute stability of the
        // [tempo] section against future drift.
        let path = tmp_path("tempo-round-trip");
        let mut settings = Settings::default();
        settings.tempo.ableton_link_enabled = false;
        settings.tempo.start_stop_sync = false;
        settings.tempo.midi_clock_in_enabled = true;
        settings.tempo.midi_clock_in_device = "midi-port-2".to_string();
        settings.tempo.midi_clock_out_enabled = false;
        settings.tempo.midi_clock_out_device = "midi-port-3".to_string();
        settings.tempo.quantize = "phrase".to_string();
        settings.tempo.manual_bpm = 128.5;
        settings.tempo.link_offset_ms = -25.0;
        settings.tempo.audio_bpm_sensitivity = 80.0;
        settings.tempo.bpm_source_priority = vec![
            "manual".to_string(),
            "midiclock".to_string(),
            "audio".to_string(),
            "link".to_string(),
        ];

        settings.save_to_path(&path).unwrap();
        let loaded = Settings::load_from_path(&path).unwrap();
        assert!(!loaded.tempo.ableton_link_enabled);
        assert!(!loaded.tempo.start_stop_sync);
        assert!(loaded.tempo.midi_clock_in_enabled);
        assert_eq!(loaded.tempo.midi_clock_in_device, "midi-port-2");
        assert!(!loaded.tempo.midi_clock_out_enabled);
        assert_eq!(loaded.tempo.midi_clock_out_device, "midi-port-3");
        assert_eq!(loaded.tempo.quantize, "phrase");
        assert_eq!(loaded.tempo.manual_bpm, 128.5);
        assert_eq!(loaded.tempo.link_offset_ms, -25.0);
        assert_eq!(loaded.tempo.audio_bpm_sensitivity, 80.0);
        assert_eq!(
            loaded.tempo.bpm_source_priority,
            vec![
                "manual".to_string(),
                "midiclock".to_string(),
                "audio".to_string(),
                "link".to_string(),
            ]
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn live_sources_default_matches_bundle_baseline() {
        // Wave 2.7 SET.B: pin the bundle-faithful Live Sources
        // defaults. The frontend Settings view reads back these
        // values on view mount and renders the wireframe-faithful
        // toggle states (Syphon in/out + NDI out on; NDI in +
        // Constant Output Resolution off; Sender Name = "chrm").
        let defaults = Settings::default();
        assert!(defaults.live_sources.syphon_input_enabled);
        assert!(defaults.live_sources.syphon_output_enabled);
        assert!(!defaults.live_sources.ndi_input_enabled);
        assert!(defaults.live_sources.ndi_output_enabled);
        assert!(!defaults.live_sources.constant_output_resolution);
        assert_eq!(defaults.live_sources.sender_name, "chrm");
    }

    #[test]
    fn live_sources_round_trip_preserves_operator_values() {
        // Mutating each Live Sources field and round-tripping through
        // TOML pins the field-name + serde-attribute stability of the
        // [live_sources] section against future drift.
        let path = tmp_path("live-sources-round-trip");
        let mut settings = Settings::default();
        settings.live_sources.syphon_input_enabled = false;
        settings.live_sources.syphon_output_enabled = false;
        settings.live_sources.ndi_input_enabled = true;
        settings.live_sources.ndi_output_enabled = false;
        settings.live_sources.constant_output_resolution = true;
        settings.live_sources.sender_name = "studio-A".to_string();
        settings.save_to_path(&path).unwrap();
        let loaded = Settings::load_from_path(&path).unwrap();
        assert!(!loaded.live_sources.syphon_input_enabled);
        assert!(!loaded.live_sources.syphon_output_enabled);
        assert!(loaded.live_sources.ndi_input_enabled);
        assert!(!loaded.live_sources.ndi_output_enabled);
        assert!(loaded.live_sources.constant_output_resolution);
        assert_eq!(loaded.live_sources.sender_name, "studio-A");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn legacy_toml_without_tempo_section_loads_via_serde_default() {
        // Forward-compat: a settings file written before Wave 2.7
        // Lane SET.A added the [tempo] section MUST deserialise
        // cleanly. Missing fields fill from the per-field
        // `#[serde(default)]` annotations.
        let path = tmp_path("pre-tempo-section");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"[general]\ntheme = \"pulse\"\n\n[audio]\nsample_rate_hz = 48000\n")
                .unwrap();
        }
        let loaded = Settings::load_from_path(&path).unwrap();
        // The tempo section materialises with the per-field defaults.
        assert!(loaded.tempo.ableton_link_enabled);
        assert_eq!(loaded.tempo.quantize, "bar");
        assert_eq!(loaded.tempo.manual_bpm, 120.0);
        assert_eq!(
            loaded.tempo.bpm_source_priority,
            vec![
                "audio".to_string(),
                "midiclock".to_string(),
                "link".to_string(),
                "manual".to_string(),
            ]
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn legacy_toml_without_live_sources_section_loads_via_serde_default() {
        // Forward-compat: a settings file written by the Wave 2.1
        // C.3 release (which had no [live_sources] section) MUST
        // deserialise cleanly under the Wave 2.7 SET.B release. The
        // missing fields fill via the per-field `#[serde(default)]`
        // annotations + the section-level `#[serde(default)]` on
        // `Settings::live_sources`.
        let path = tmp_path("pre-live-sources-section");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"[general]\ntheme = \"pulse\"\n\n[audio]\nsample_rate_hz = 48000\n")
                .unwrap();
        }
        let loaded = Settings::load_from_path(&path).unwrap();
        // The Live Sources section materialises with the per-field
        // defaults.
        assert!(loaded.live_sources.syphon_input_enabled);
        assert!(loaded.live_sources.syphon_output_enabled);
        assert!(!loaded.live_sources.ndi_input_enabled);
        assert!(loaded.live_sources.ndi_output_enabled);
        assert!(!loaded.live_sources.constant_output_resolution);
        assert_eq!(loaded.live_sources.sender_name, "chrm");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn legacy_toml_without_osc_section_loads_via_serde_default() {
        // Forward-compat: a settings file written by the W34 C.3
        // landing (which had no [osc] section) MUST deserialise
        // cleanly under the Wave 2.1 Lane D.3 release that adds the
        // section. The missing fields fill via the per-field
        // `#[serde(default)]` annotations.
        let path = tmp_path("pre-osc-section");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"[general]\ntheme = \"pulse\"\n\n[audio]\nsample_rate_hz = 48000\n")
                .unwrap();
        }
        let loaded = Settings::load_from_path(&path).unwrap();
        // The OSC section materialises with the per-field defaults.
        assert_eq!(loaded.osc.local_ip, "127.0.0.1");
        assert_eq!(loaded.osc.port, DEFAULT_OSC_IN_PORT);
        assert_eq!(loaded.osc.out_port, DEFAULT_OSC_OUT_PORT);
        assert!(loaded.osc.tx_enabled);
        assert!(loaded.osc.rx_enabled);
        // Other sections still resolve as expected.
        assert_eq!(loaded.general.theme, Theme::Pulse);
        assert_eq!(loaded.audio.sample_rate_hz, 48_000);
        let _ = std::fs::remove_file(&path);
    }
}
