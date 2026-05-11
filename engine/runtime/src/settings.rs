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
//! TOML; the shell-side handler logs the error and falls back to
//! [`Settings::default`] so a corrupted settings file never blocks
//! the app from launching. A missing file (initial launch) is NOT
//! an error — callers test [`std::path::Path::exists`] first or
//! treat [`std::io::ErrorKind::NotFound`] as the default-fallback
//! sentinel; the helpers below ([`Settings::load_or_default`]) do
//! exactly that.

use serde::{Deserialize, Serialize};
use std::path::Path;

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
/// blocks the app from launching — the operator's next save
/// overwrites the corrupted file with a fresh document.
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

    /// Serialise this [`Settings`] to `path` as TOML.
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
        std::fs::write(path, serialised)?;
        Ok(())
    }

    /// Convenience helper: load from `path` if it exists, otherwise
    /// return [`Self::default`]. A missing file is the initial-launch
    /// case and is NOT an error; a malformed file IS an error and
    /// surfaces verbatim so the caller can choose between "log +
    /// fall back to default" or "refuse to launch".
    pub fn load_or_default(path: &Path) -> Result<Self, SettingsError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                let parsed: Self = toml::from_str(&contents)?;
                Ok(parsed)
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(SettingsError::Io(err)),
        }
    }
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
}
