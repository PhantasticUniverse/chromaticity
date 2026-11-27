// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Audio device-enumeration IPC handlers — Wave 2.6 Lane AUD.A
//! (input) + Wave 2.7 Lane SET.C (output).
//!
//! Surfaces the [`chromaticity_audio::enumerate_input_devices`] +
//! [`chromaticity_audio::enumerate_output_devices`] results across
//! the Tauri command boundary so the operator-facing
//! Settings → Audio Device → Input Device / Output Device dropdowns
//! in `src/lib/views/Settings.svelte` can render the host's real
//! devices (rather than the W21 PR1 fixture-only `AUDIO_INPUT_DEVICES`
//! / `AUDIO_OUTPUT_DEVICES` sets defined in
//! `src/lib/data/settings-fixtures.ts`).
//!
//! ## Error model
//!
//! Both underlying engine surfaces collapse the
//! [`cpal::Host::input_devices`] / [`cpal::Host::output_devices`]
//! failure (the only error class today: CI / headless / no-audio-
//! subsystem hosts) into `Ok(Vec::new())` so the operator-facing
//! fixture-fallback branch fires identically for "audio available but
//! no devices connected" and "audio subsystem unavailable". Both
//! handlers therefore return `Err(String)` only for the genuinely
//! unrecoverable error class — there are no producers of that branch
//! today, but the return type stays `Result<_, String>` to match the
//! project's existing Tauri command shape (see
//! `commands::midi::list_midi_devices` for the parallel precedent).
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (audio arm, Wave 2.6 Lane
//! AUD.A + Wave 2.7 Lane SET.C) — input + output device-listing
//! surfaces widened from the W21 PR1 fixture-only posture documented
//! in `src/lib/data/settings-fixtures.ts`.

use crate::state::AppState;
use chromaticity_audio::{enumerate_input_devices, enumerate_output_devices};
use chromaticity_runtime::rekordbox_bridge::{
    ChannelPairMeter, ChannelPairSelection, DjDeviceKind, DjDeviceProfile, MultiChannelExtractor,
};
use chromaticity_ui_bridge::{AudioDeviceKindPayload, AudioDevicePayload};
use serde::{Deserialize, Serialize};
use tauri::State;

/// Enumerate the cpal input devices currently visible to the host.
///
/// Returns one [`AudioDevicePayload`] per device surfaced by a fresh
/// [`chromaticity_audio::enumerate_input_devices`] call. On CI /
/// headless / no-audio-subsystem hosts the adapter returns an empty
/// vec (rather than erroring) so the frontend's fixture-fallback
/// branch fires uniformly across the two "nothing-to-show" causes.
///
/// Sync (not `async`) per the project's existing Tauri command shape
/// — the underlying `enumerate_input_devices` call is a fast cpal host
/// enumeration (CoreAudio / WASAPI / ALSA), not a per-frame poll path.
/// Tauri 2 dispatches sync commands on a worker pool, so this does
/// not block the UI thread.
#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<AudioDevicePayload>, String> {
    let infos = enumerate_input_devices().map_err(|e| format!("audio enumeration failed: {e}"))?;
    Ok(infos.into_iter().map(AudioDevicePayload::from).collect())
}

/// Enumerate the cpal output devices currently visible to the host.
///
/// Returns one [`AudioDevicePayload`] per device surfaced by a fresh
/// [`chromaticity_audio::enumerate_output_devices`] call, with the
/// payload `kind` field set to [`AudioDeviceKindPayload::Output`]
/// (the engine [`chromaticity_audio::AudioDeviceInfo`] is direction-
/// agnostic; the bridge re-attaches the discriminant at this command
/// callsite — the [`From<AudioDeviceInfo>`] impl defaults to `Input`,
/// so this command constructs the payload by hand).
///
/// Mirrors [`list_audio_devices`]: on CI / headless / no-audio-
/// subsystem hosts the engine adapter returns an empty vec (rather
/// than erroring) so the frontend's fixture-fallback branch fires
/// uniformly across the two "nothing-to-show" causes.
///
/// Sync (not `async`) per the project's existing Tauri command shape
/// — the underlying call is a fast cpal host enumeration (CoreAudio /
/// WASAPI / ALSA), not a per-frame poll path.
///
/// [`From<AudioDeviceInfo>`]: chromaticity_ui_bridge::AudioDevicePayload
#[tauri::command]
pub fn list_audio_output_devices() -> Result<Vec<AudioDevicePayload>, String> {
    let infos =
        enumerate_output_devices().map_err(|e| format!("audio output enumeration failed: {e}"))?;
    Ok(infos
        .into_iter()
        .map(|info| AudioDevicePayload {
            id: info.id,
            name: info.name,
            kind: AudioDeviceKindPayload::Output,
        })
        .collect())
}

/// Serializable stereo channel-pair DTO surfaced across the Tauri IPC boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioChannelPairDto {
    /// Zero-based left channel index (`0` = Ch 1).
    pub left_channel: u16,
    /// Zero-based right channel index (`1` = Ch 2).
    pub right_channel: u16,
    /// Human-readable 1-based channel label (e.g. `"Ch 1-2"`).
    pub label: String,
}

impl From<ChannelPairSelection> for AudioChannelPairDto {
    fn from(selection: ChannelPairSelection) -> Self {
        Self {
            left_channel: selection.left_channel.min(usize::from(u16::MAX)) as u16,
            right_channel: selection.right_channel.min(usize::from(u16::MAX)) as u16,
            label: selection.one_based_label(),
        }
    }
}

/// Serializable DJ hardware routing profile DTO returned by [`get_dj_device_profile`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DjDeviceProfileDto {
    /// Stable snake_case device family identifier (e.g. `"ddj_flx10"`, `"ddj_flx4"`,
    /// `"djm_a9"`, `"rekordbox_pc_master_out"`, `"generic_multi_channel"`).
    pub kind: &'static str,
    /// Canonical human-readable device name (e.g. `"Pioneer DJ DDJ-FLX10"`).
    pub display_name: String,
    /// Typical USB / Core Audio channel count for this hardware.
    pub expected_channels: u16,
    /// Recommended Master / REC OUT stereo channel pair.
    pub master_pair: AudioChannelPairDto,
    /// Human-readable Master pair label (e.g. `"Ch 1-2 (Master / REC OUT)"`).
    pub master_label: String,
    /// Headphone Cue / monitor pair to isolate and exclude (if exposed).
    pub cue_pair: Option<AudioChannelPairDto>,
    /// Human-readable Cue pair label (if present).
    pub cue_label: Option<String>,
    /// Whether Rekordbox `PC MASTER OUT` + macOS Core Audio Process Tap is supported.
    pub supports_pc_master_out: bool,
    /// Operator setup guidance for routing without Headphone Cue leakage.
    pub notes: String,
    /// `true` if `device_name` matched a known DJ controller/mixer/Rekordbox profile.
    pub recognized: bool,
}

const fn dj_device_kind_str(kind: DjDeviceKind) -> &'static str {
    match kind {
        DjDeviceKind::DdjFlx10 => "ddj_flx10",
        DjDeviceKind::DdjFlx4 => "ddj_flx4",
        DjDeviceKind::Ddj1000 => "ddj_1000",
        DjDeviceKind::OpusQuad => "opus_quad",
        DjDeviceKind::XdjXz => "xdj_xz",
        DjDeviceKind::DjmA9 => "djm_a9",
        DjDeviceKind::Djm900Nxs2 => "djm_900nxs2",
        DjDeviceKind::RekordboxPcMasterOut => "rekordbox_pc_master_out",
        DjDeviceKind::GenericMultiChannel => "generic_multi_channel",
    }
}

impl From<DjDeviceProfile> for DjDeviceProfileDto {
    fn from(profile: DjDeviceProfile) -> Self {
        let recognized = profile.kind != DjDeviceKind::GenericMultiChannel;
        Self {
            kind: dj_device_kind_str(profile.kind),
            display_name: profile.display_name,
            expected_channels: profile.expected_channels.min(usize::from(u16::MAX)) as u16,
            master_pair: AudioChannelPairDto::from(profile.master_pair),
            master_label: profile.master_label,
            cue_pair: profile.cue_pair.map(AudioChannelPairDto::from),
            cue_label: profile.cue_label,
            supports_pc_master_out: profile.supports_pc_master_out,
            notes: profile.notes,
            recognized,
        }
    }
}

/// Detect the Rekordbox / Pioneer DJ / AlphaTheta hardware routing profile for
/// `device_name` and `channel_count`, surfacing recommended Master vs. Headphone
/// Cue stereo pairs and operator setup guidance.
#[tauri::command]
pub fn get_dj_device_profile(device_name: String, channel_count: u16) -> DjDeviceProfileDto {
    let profile = DjDeviceProfile::recognize_or_default(&device_name, usize::from(channel_count));
    DjDeviceProfileDto::from(profile)
}

/// Inner implementation for [`set_audio_channel_pair`].
pub fn set_audio_channel_pair_impl(
    app: &AppState,
    left_channel: u16,
    right_channel: u16,
) -> Result<AudioChannelPairDto, String> {
    let selection =
        ChannelPairSelection::new(usize::from(left_channel), usize::from(right_channel));
    app.store_audio_channel_pair(selection);
    Ok(AudioChannelPairDto::from(selection))
}

/// Inner implementation for [`get_audio_channel_pair`].
pub fn get_audio_channel_pair_impl(app: &AppState) -> Result<AudioChannelPairDto, String> {
    Ok(AudioChannelPairDto::from(app.current_audio_channel_pair()))
}

/// Select the active stereo channel pair (0-based indices, default `(0, 1)` =
/// Ch 1–2 Master) for multi-channel DJ / Rekordbox audio capture so Headphone
/// Cue (e.g. Ch 3–4 on DDJ-FLX10) is isolated from visuals.
#[tauri::command]
pub fn set_audio_channel_pair(
    app_state: State<'_, AppState>,
    left_channel: u16,
    right_channel: u16,
) -> Result<AudioChannelPairDto, String> {
    set_audio_channel_pair_impl(app_state.inner(), left_channel, right_channel)
}

/// Read the currently selected stereo channel pair (0-based indices + 1-based label).
#[tauri::command]
pub fn get_audio_channel_pair(
    app_state: State<'_, AppState>,
) -> Result<AudioChannelPairDto, String> {
    get_audio_channel_pair_impl(app_state.inner())
}

/// Extract only the managed stereo channel pair from an interleaved multi-channel
/// PCM buffer using [`MultiChannelExtractor`], falling back safely to mono `(0, 0)`
/// or Master `(0, 1)` if the stream exposes fewer channels than the configured pair.
///
/// Guarantees Headphone Cue channels (e.g. Ch 3–4 on DDJ-FLX10) are never mixed
/// into the extracted stereo analysis buffer.
pub fn extract_selected_channel_pair(
    app: &AppState,
    channel_count: u16,
    interleaved: &[f32],
) -> Result<(Vec<f32>, Vec<ChannelPairMeter>), String> {
    let channels = usize::from(channel_count);
    if channels == 0 {
        return Err("audio stream must have at least one channel".into());
    }
    let configured = app.current_audio_channel_pair();
    let effective = if configured.is_valid_for_channels(channels) {
        configured
    } else if channels >= 2 {
        ChannelPairSelection::master_default()
    } else {
        ChannelPairSelection::new(0, 0)
    };
    let extractor = MultiChannelExtractor::new(channels, effective).map_err(|e| e.to_string())?;
    let mut stereo = Vec::new();
    let meters = extractor
        .extract_into(interleaved, &mut stereo)
        .map_err(|e| e.to_string())?;
    Ok((stereo, meters))
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — pins the empty-vec
    //! graceful-fallback contract documented in the module-level prose
    //! for the Wave 2.6 Lane AUD.A audio device-listing handler. On
    //! CI / headless / no-audio-subsystem hosts the underlying adapter
    //! MUST collapse the cpal `input_devices()` failure to
    //! `Ok(Vec::new())`; the handler MUST propagate that without
    //! erroring so the frontend's fixture-fallback branch fires
    //! uniformly.

    use super::*;

    #[test]
    fn list_audio_devices_does_not_error_on_any_host() {
        // The IPC handler MUST NOT propagate the cpal host-init
        // failure to the caller — CI runners typically have no audio
        // subsystem available. The empty vec under that condition is
        // the contract; on a developer host with audio subsystem
        // access AND zero connected input devices, the result is also
        // empty; on a host WITH connected input devices the result is
        // non-empty. All three cases pass this assertion.
        let result =
            list_audio_devices().expect("list_audio_devices must not error at MVP surface");
        for d in &result {
            assert!(
                d.id.starts_with("input-"),
                "expected id to start with `input-`, got {:?}",
                d.id,
            );
            assert!(!d.name.is_empty(), "cpal device name must not be empty");
        }
    }

    #[test]
    fn list_audio_output_devices_does_not_error_on_any_host() {
        // Wave 2.7 Lane SET.C: mirrors `list_audio_devices_does_not_error_on_any_host`
        // for the output direction. Same three-case coverage (no
        // audio subsystem / no devices connected / devices present).
        // Each payload MUST carry the `output-` id prefix +
        // `AudioDeviceKindPayload::Output` discriminant.
        let result = list_audio_output_devices()
            .expect("list_audio_output_devices must not error at MVP surface");
        for d in &result {
            assert!(
                d.id.starts_with("output-"),
                "expected id to start with `output-`, got {:?}",
                d.id,
            );
            assert!(!d.name.is_empty(), "cpal device name must not be empty");
            assert_eq!(
                d.kind,
                chromaticity_ui_bridge::AudioDeviceKindPayload::Output,
                "expected Output kind on every payload, got {:?} for id {:?}",
                d.kind,
                d.id,
            );
        }
    }

    #[test]
    fn get_dj_device_profile_recognizes_ddj_flx10_and_rekordbox_devices() {
        let flx10 = get_dj_device_profile("Pioneer DJ DDJ-FLX10".into(), 4);
        assert!(flx10.recognized);
        assert_eq!(flx10.kind, "ddj_flx10");
        assert_eq!(flx10.display_name, "Pioneer DJ DDJ-FLX10");
        assert_eq!(flx10.expected_channels, 4);
        assert_eq!(flx10.master_pair.left_channel, 0);
        assert_eq!(flx10.master_pair.right_channel, 1);
        assert_eq!(flx10.master_pair.label, "Ch 1-2");
        assert!(flx10.master_label.contains("Master"));
        let cue = flx10.cue_pair.expect("DDJ-FLX10 must report Cue pair");
        assert_eq!(cue.left_channel, 2);
        assert_eq!(cue.right_channel, 3);
        assert_eq!(cue.label, "Ch 3-4");
        assert!(flx10.supports_pc_master_out);
        assert!(!flx10.notes.is_empty());

        let flx4 = get_dj_device_profile("DDJ-FLX4".into(), 4);
        assert!(flx4.recognized);
        assert_eq!(flx4.kind, "ddj_flx4");

        let djm_a9 = get_dj_device_profile("DJM-A9".into(), 10);
        assert!(djm_a9.recognized);
        assert_eq!(djm_a9.kind, "djm_a9");
        assert_eq!(djm_a9.expected_channels, 10);

        let rb_pc = get_dj_device_profile("com.pioneerdj.rekordboxdj".into(), 2);
        assert!(rb_pc.recognized);
        assert_eq!(rb_pc.kind, "rekordbox_pc_master_out");
        assert!(rb_pc.cue_pair.is_none());

        let generic = get_dj_device_profile("Scarlett 4i4 USB".into(), 4);
        assert!(!generic.recognized);
        assert_eq!(generic.kind, "generic_multi_channel");
        assert_eq!(generic.master_pair.left_channel, 0);
        assert_eq!(generic.master_pair.right_channel, 1);
        assert!(generic.cue_pair.is_some());
    }

    #[test]
    fn audio_channel_pair_state_defaults_to_master_and_isolates_cue_channels() {
        let app = AppState::new();
        let initial = get_audio_channel_pair_impl(&app).expect("initial channel pair");
        assert_eq!(initial.left_channel, 0);
        assert_eq!(initial.right_channel, 1);
        assert_eq!(initial.label, "Ch 1-2");

        // 4-channel DDJ-FLX10 interleaved frames: Ch 1-2 Master = 0.25/-0.25, Ch 3-4 Cue = 0.95/-0.95.
        let interleaved = [0.25_f32, -0.25, 0.95, -0.95, 0.5, -0.5, 0.9, -0.9];
        let (stereo_master, meters) =
            extract_selected_channel_pair(&app, 4, &interleaved).expect("extract master pair");
        assert_eq!(stereo_master, vec![0.25, -0.25, 0.5, -0.5]);
        assert_eq!(meters.len(), 2);
        assert!(meters[0].active);
        assert!(meters[1].active);

        // Switch to Ch 3-4 and verify state transition + extraction.
        let updated = set_audio_channel_pair_impl(&app, 2, 3).expect("set Ch 3-4");
        assert_eq!(updated.left_channel, 2);
        assert_eq!(updated.right_channel, 3);
        assert_eq!(updated.label, "Ch 3-4");
        assert_eq!(
            get_audio_channel_pair_impl(&app).expect("get Ch 3-4"),
            updated
        );

        let (stereo_cue, _) =
            extract_selected_channel_pair(&app, 4, &interleaved).expect("extract Ch 3-4");
        assert_eq!(stereo_cue, vec![0.95, -0.95, 0.9, -0.9]);

        // If stream drops to 2-channel stereo, out-of-range (2, 3) safely falls back to (0, 1).
        let stereo_only = [0.4_f32, -0.4, 0.2, -0.2];
        let (fallback, _) =
            extract_selected_channel_pair(&app, 2, &stereo_only).expect("safe fallback");
        assert_eq!(fallback, vec![0.4, -0.4, 0.2, -0.2]);
    }
}
