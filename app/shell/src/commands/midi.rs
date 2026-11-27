// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

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

use crate::state::AppState;
use chromaticity_midi_adapter::{list_devices, start_input_listen};
use chromaticity_ui_bridge::{MidiDevicePayload, MidiLearnEventPayload};
use tauri::{AppHandle, Emitter, State};

/// Tauri event name emitted on each MIDI learn-mode capture.
///
/// Wave 2.1 Lane D.2 — the Svelte consumer in
/// `src/lib/stores/midiLearn.svelte.ts` calls
/// `listen('midi_learn_event', ...)` against this literal. Pinning
/// the constant in one place keeps the wire-name contract co-located
/// with its producer (the precedent: the
/// `AUDIO_UNIFORM_SNAPSHOT_EVENT` constant in
/// `app/shell/src/state/ipc_emit.rs`).
pub const MIDI_LEARN_EVENT: &str = "midi_learn_event";

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

/// Open a transient MIDI learn-mode listener on `port_id` for
/// Wave 2.1 Lane D.2 minimal-viable learn capture.
///
/// `port_id` is the `"input-<n>"` identifier surfaced by
/// [`list_midi_devices`] (the same id rendered as the Devices-tab
/// keyed-each iteration key). The handler parses the suffix back into
/// the midir port-index space, opens a fresh midir input listener via
/// [`chromaticity_midi_adapter::start_input_listen`], and stores the
/// returned [`chromaticity_midi_adapter::MidiListenHandle`] onto
/// [`AppState::midi_learn_handle`]. Each parsed
/// [`chromaticity_midi_adapter::MidiMessage`] is forwarded via
/// [`tauri::Emitter::emit`] as [`MIDI_LEARN_EVENT`] with a
/// [`MidiLearnEventPayload`] carrying the port id + decode fields.
///
/// **Lifecycle**: replacing an existing learn session is intentional.
/// Calling `start_midi_learn` while a prior handle is active drops
/// the prior handle (midir's
/// [`chromaticity_midi_adapter::MidiListenHandle::drop`] closes the
/// prior listener) and installs a fresh one. This matches the
/// minimal-viable single-capture posture: re-entering learn mode on
/// a different port (or on the same port to re-arm) cleans up
/// deterministically.
///
/// **AppState shutdown**: when the Tauri runtime tears down the
/// managed [`AppState`], the `Drop` impl on the
/// `Option<MidiListenHandle>` fires and releases the OS midir port
/// handle. No `MidiInputConnection` leaks across app exit; see the
/// `midi_learn_handle` field prose in `app/shell/src/state/mod.rs`.
///
/// Errors:
/// - `Err("invalid port_id …")` — the `port_id` doesn't match the
///   `"input-<n>"` shape [`list_midi_devices`] emits.
/// - `Err("midi enumeration failed: …")` — midir client init failed
///   or the port index is out of range.
///
/// Sync (not `async`) per the project's existing Tauri command shape
/// — none of the body awaits, so the sync signature is the right
/// surface and avoids the `'static`-bound friction `tauri::State`
/// imposes on `async fn` handlers. Tauri 2 dispatches sync commands
/// on a worker pool, so this does not block the UI thread.
#[tauri::command]
pub fn start_midi_learn(
    app: AppHandle,
    state: State<'_, AppState>,
    port_id: String,
) -> Result<(), String> {
    let port_index = parse_port_id(&port_id)?;
    // Clone the AppHandle (cheap — internally `Arc`) for the midir
    // callback. The callback runs on midir's delivery thread; it
    // must own its AppHandle clone rather than borrowing because
    // the closure outlives this function scope.
    let app_for_callback = app.clone();
    let port_id_for_callback = port_id.clone();
    let handle = start_input_listen(port_index, move |message| {
        let payload = MidiLearnEventPayload::from_message(port_id_for_callback.clone(), &message);
        // Silent drop on emit failure — Tauri's emit can fail if the
        // event queue is full or the app is shutting down; neither
        // failure mode warrants a panic and the midir callback must
        // continue accepting messages. The frontend store will simply
        // miss this one message; subsequent messages flow normally.
        // Mirrors the `TauriIpcEmitter::emit_snapshot` posture in
        // `app/shell/src/state/ipc_emit.rs`.
        let _ = app_for_callback.emit(MIDI_LEARN_EVENT, &payload);
    })
    .map_err(|e| format!("midi learn start failed: {e}"))?;

    // Install the handle. The replaced prior handle (if any) drops
    // here, which closes the prior midir connection.
    let mut guard = match state.midi_learn_handle.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    *guard = Some(handle);
    Ok(())
}

/// Close the active MIDI learn-mode listener installed by
/// [`start_midi_learn`].
///
/// Idempotent: calling `stop_midi_learn` with no active session is a
/// no-op (the slot stays `None`). Dropping the held
/// [`chromaticity_midi_adapter::MidiListenHandle`] closes the
/// underlying midir port.
#[tauri::command]
pub fn stop_midi_learn(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = match state.midi_learn_handle.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    *guard = None;
    Ok(())
}

/// Parse the `"input-<n>"` form [`MidiDevicePayload::id`] uses back
/// into the midir port-index space.
///
/// Wave 2.1 Lane D.2 only opens input ports; an `"output-<n>"` id
/// surfaces an error because there's no output-listen path. A future
/// wave that adds output enumeration may widen this surface.
fn parse_port_id(port_id: &str) -> Result<usize, String> {
    let Some(suffix) = port_id.strip_prefix("input-") else {
        return Err(format!(
            "invalid port_id {port_id:?}: expected \"input-<n>\""
        ));
    };
    suffix
        .parse::<usize>()
        .map_err(|e| format!("invalid port_id {port_id:?}: index parse failed: {e}"))
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

    #[test]
    fn parse_port_id_accepts_valid_input_form() {
        // Pin the `"input-<n>"` parse contract — drift here breaks the
        // round-trip from `list_midi_devices` (which emits this form)
        // back to `start_midi_learn` (which parses it).
        assert_eq!(super::parse_port_id("input-0"), Ok(0));
        assert_eq!(super::parse_port_id("input-3"), Ok(3));
        assert_eq!(super::parse_port_id("input-42"), Ok(42));
    }

    #[test]
    fn parse_port_id_rejects_malformed_inputs() {
        // Missing prefix.
        assert!(super::parse_port_id("0").is_err());
        // Wrong prefix (output enumeration is a follow-up wave).
        assert!(super::parse_port_id("output-0").is_err());
        // Empty suffix.
        assert!(super::parse_port_id("input-").is_err());
        // Negative — usize parse rejects.
        assert!(super::parse_port_id("input--1").is_err());
        // Non-numeric suffix.
        assert!(super::parse_port_id("input-abc").is_err());
    }

    #[test]
    fn midi_learn_event_constant_matches_consumer_listener() {
        // The Svelte consumer in `src/lib/stores/midiLearn.svelte.ts`
        // calls `listen('midi_learn_event', ...)`. A drift in the
        // event name is a silent break that the e2e harness would
        // catch downstream; this T1 pin catches it at unit-test time.
        // Mirrors the `event_name_constant_matches_consumer_listener`
        // pin in `app/shell/src/state/ipc_emit.rs`.
        assert_eq!(super::MIDI_LEARN_EVENT, "midi_learn_event");
    }
}
