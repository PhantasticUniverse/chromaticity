// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! MIDI device-enumeration IPC handler — Wave 2.1 Lane D.1.
//!
//! Surfaces the [`chromaticity_midi_adapter::list_devices`] result
//! across the Tauri command boundary so the operator-facing MIDI view
//! Devices tab in `src/lib/views/MIDI.svelte` can render the host's
//! real input ports (rather than the W18 PR3 fixture-only set defined
//! in `src/lib/data/midi-fixtures.ts`).
//!
//! ## Lane D.1 scope narrowing — input-only
//!
//! Wave 2.1 Lane D.1 ships INPUT device enumeration only — output
//! enumeration is reserved for a follow-up wave alongside the per-device
//! allowlist-persistence surface. The engine adapter's
//! [`chromaticity_midi_adapter::MidiDeviceKind`] enum is
//! `#[non_exhaustive]` with `Input` as its sole variant today; output
//! enumeration widens both the engine enum and the bridge
//! [`MidiDeviceKindPayload`] mirror in lockstep when it lands.
//!
//! ## Error model
//!
//! The underlying [`chromaticity_midi_adapter::list_devices`] collapses
//! the `MidiInput::new` client-construction failure (the only error
//! class today: CI / headless / no-MIDI-subsystem hosts) into
//! `Ok(Vec::new())` so the operator-facing "No MIDI devices detected"
//! empty-state branch fires identically for "MIDI available but no
//! devices connected" and "MIDI subsystem unavailable". This handler
//! therefore returns `Err(String)` only for the genuinely unrecoverable
//! error class — there are no producers of that branch in Wave 2.1
//! Lane D.1, but the return type stays `Result<_, String>` to match
//! the project's existing Tauri command shape (see `commands::camera`
//! for the parallel precedent) and so the follow-up output-enumeration
//! wave can introduce real error producers without churning the IPC
//! contract.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (MIDI arm, Wave 2.1 Lane D.1)
//! — device-listing surface widened from the W18 PR3 fixture-only
//! posture documented in the `EV-INTEGRATION-MATRIX` Wave 1c-β
//! closure section of `docs/EV-GATE-MODEL-AUDIT-checklist.md`.

use chromaticity_midi_adapter::list_devices;
use chromaticity_ui_bridge::MidiDevicePayload;

/// Enumerate the MIDI input ports currently visible to the host.
///
/// Returns one [`MidiDevicePayload`] per port surfaced by a fresh
/// midir [`chromaticity_midi_adapter::list_devices`] call. On CI /
/// headless / no-MIDI-subsystem hosts the adapter returns an empty
/// vec (rather than erroring) so the frontend's "No MIDI devices
/// detected" empty-state branch fires uniformly across the two
/// "nothing-to-show" causes.
///
/// Sync (not `async`) per the project's existing Tauri command shape
/// — the underlying `list_devices` call is a fast OS-MIDI-subsystem
/// enumeration (CoreMIDI / WinMM / ALSA), not a per-frame poll path.
/// Tauri 2 dispatches sync commands on a worker pool, so this does
/// not block the UI thread.
#[tauri::command]
pub fn list_midi_devices() -> Result<Vec<MidiDevicePayload>, String> {
    let descriptors = list_devices().map_err(|e| format!("midi enumeration failed: {e}"))?;
    Ok(descriptors
        .into_iter()
        .map(MidiDevicePayload::from)
        .collect())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — pins the empty-vec
    //! graceful-fallback contract documented in the module-level prose
    //! for the Wave 2.1 Lane D.1 MIDI device-listing handler. On
    //! CI / headless /
    //! no-MIDI-subsystem hosts the underlying adapter MUST collapse
    //! the client-construction failure to `Ok(Vec::new())`; the
    //! handler MUST propagate that without erroring so the frontend's
    //! "no MIDI devices" empty-state branch fires uniformly.

    use super::*;

    #[test]
    fn list_midi_devices_does_not_error_on_any_host() {
        // The IPC handler MUST NOT propagate the midir client-init
        // failure to the caller — CI runners typically have no MIDI
        // subsystem available. The empty vec under that condition is
        // the contract; on a developer host with MIDI subsystem
        // access AND zero connected devices, the result is also
        // empty; on a host WITH connected devices the result is
        // non-empty. All three cases pass this assertion.
        let result = list_midi_devices().expect("list_midi_devices must not error at MVP surface");
        for d in &result {
            assert!(
                d.id.starts_with("input-"),
                "expected id to start with `input-`, got {:?}",
                d.id,
            );
            assert!(!d.name.is_empty(), "midir port name must not be empty");
        }
    }
}
