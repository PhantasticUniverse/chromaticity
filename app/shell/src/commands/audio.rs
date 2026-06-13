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

use chromaticity_audio::{enumerate_input_devices, enumerate_output_devices};
use chromaticity_ui_bridge::{AudioDeviceKindPayload, AudioDevicePayload};

/// Diagnostic-only IPC sink (gated by `CHROMATICITY_AUDIO_DIAG`). The
/// audio-snapshot store calls this with each received payload's `rms`/`peak`
/// so the engine-side capture log shows what the WEBVIEW actually received vs
/// what the producer emitted — bisecting a "static meter despite a live
/// engine snapshot" without driving the webview's DevTools. No-op when
/// diagnostics are off, so it is free in normal operation.
#[tauri::command]
pub fn audio_diag_rx(rms: f32, peak: f32, received: u64) {
    if chromaticity_audio::diagnostics_enabled() {
        eprintln!("[audio-diag] webview rx #{received} rms={rms:.4} peak={peak:.4}");
    }
}

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
}
