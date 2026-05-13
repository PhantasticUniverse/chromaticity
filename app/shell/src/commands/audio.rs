// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Audio device-enumeration IPC handler — Wave 2.6 Lane AUD.A.
//!
//! Surfaces the [`chromaticity_audio::enumerate_input_devices`] result
//! across the Tauri command boundary so the operator-facing
//! Settings → Audio Device → Input Device dropdown in
//! `src/lib/views/Settings.svelte` can render the host's real input
//! devices (rather than the W21 PR1 fixture-only `AUDIO_INPUT_DEVICES`
//! set defined in `src/lib/data/settings-fixtures.ts`).
//!
//! ## Lane AUD.A scope narrowing — input-only
//!
//! Wave 2.6 Lane AUD.A ships INPUT device enumeration only — output
//! enumeration is reserved for a follow-up wave alongside the
//! per-device routing surface. The engine adapter's
//! [`chromaticity_audio::AudioDeviceKind`] enum is `#[non_exhaustive]`
//! with `Input` as its sole variant today; output enumeration widens
//! both the engine enum and the bridge
//! [`AudioDeviceKindPayload`] mirror in lockstep when it lands. The
//! bridge already scaffolds an `Output` variant on
//! [`AudioDeviceKindPayload`] so the wire shape doesn't churn at that
//! point.
//!
//! ## Error model
//!
//! The underlying [`chromaticity_audio::enumerate_input_devices`]
//! collapses the [`cpal::Host::input_devices`] failure (the only error
//! class today: CI / headless / no-audio-subsystem hosts) into
//! `Ok(Vec::new())` so the operator-facing fixture-fallback branch
//! fires identically for "audio available but no devices connected"
//! and "audio subsystem unavailable". This handler therefore returns
//! `Err(String)` only for the genuinely unrecoverable error class —
//! there are no producers of that branch in Wave 2.6 Lane AUD.A, but
//! the return type stays `Result<_, String>` to match the project's
//! existing Tauri command shape (see `commands::midi::list_midi_devices`
//! for the parallel precedent) and so the follow-up output-enumeration
//! wave can introduce real error producers without churning the IPC
//! contract.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (audio arm, Wave 2.6 Lane
//! AUD.A) — device-listing surface widened from the W21 PR1
//! fixture-only posture documented in
//! `src/lib/data/settings-fixtures.ts`.

use chromaticity_audio::enumerate_input_devices;
use chromaticity_ui_bridge::AudioDevicePayload;

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
}
