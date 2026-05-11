// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane D.1 MIDI device-listing singleton store. Backs the
// operator-facing MIDI view Devices tab in `src/lib/views/MIDI.svelte`
// — `refresh()` invokes the new `list_midi_devices` Tauri command
// (`app/shell/src/commands/midi.rs`) and stores the returned vec.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention; the
// Wave 1c-β `cameraStore.svelte.ts` + Wave 6 `audioSnapshot.svelte.ts`
// siblings are the live precedents for this shape (class-instance
// singleton + module-level export).
//
// **Phase posture.** Wave 2.1 Lane D.1 surfaces input-device enumeration
// only — output enumeration is reserved for a follow-up wave alongside
// the per-device allowlist-persistence surface. The engine adapter
// (`integrations/midi-adapter/src/adapter.rs::list_devices`) collapses
// the `MidiInput::new` client-construction failure (CI / headless /
// no-MIDI-subsystem hosts) into `Ok(Vec::new())`, so the consumer view
// renders the "No MIDI devices detected" empty-state branch uniformly
// for both "MIDI available but no devices connected" and "MIDI
// subsystem unavailable".
//
// `refresh()` swallows the `invoke` rejection that fires when running
// outside Tauri (vite-only `npm run dev`, Vitest jsdom with no IPC
// mock) and records the error onto `error` so the consumer view can
// render an operator-facing banner. The device list is reset to an
// empty vec in that case so the empty-state branch fires.
//
// `subscribe`-style listener detach is intentionally absent: the
// command surface is a pull-model (`refresh()` is on demand), unlike
// the audio snapshot store that listens to a push-model Tauri event
// channel.

import { invoke } from '@tauri-apps/api/core';

import type { MidiDevicePayload } from '../ipc/bindings/MidiDevicePayload';

/** Tauri command name surfaced by `app/shell/src/commands/midi.rs`. */
const LIST_MIDI_DEVICES_COMMAND = 'list_midi_devices';

class MidiStore {
  /** Latest enumeration result. Empty vec under the no-devices /
   * no-MIDI-subsystem branch; non-empty when the host has connected
   * MIDI input ports. */
  devices = $state<readonly MidiDevicePayload[]>([]);

  /** Tracks the in-flight `refresh()` promise so the consumer view can
   * render a loading spinner without dispatching duplicate enumeration
   * calls. */
  isLoading = $state(false);

  /** Last refresh error surface, or `null` on success / fresh state.
   * Drives the operator-facing error banner when the command fails
   * (e.g. running outside Tauri). */
  error = $state<string | null>(null);

  /** Invoke `list_midi_devices` and replace the device list. Sets
   * `isLoading` for the duration; on failure, clears the device list
   * and records the error. Idempotent on the failure path — callers
   * can retry without first clearing state. */
  async refresh(): Promise<void> {
    this.isLoading = true;
    this.error = null;
    try {
      const fresh = await invoke<MidiDevicePayload[]>(LIST_MIDI_DEVICES_COMMAND);
      this.devices = fresh;
    } catch (e) {
      this.devices = [];
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.isLoading = false;
    }
  }

  /** Test-harness reset. Clears the device list, loading, and error
   * state. */
  reset(): void {
    this.devices = [];
    this.isLoading = false;
    this.error = null;
  }
}

export const midiStore = new MidiStore();
