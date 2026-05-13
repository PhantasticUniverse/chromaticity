// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.6 Lane AUD.A (input) + Wave 2.7 Lane SET.C (output) cpal
// device-listing singleton store. Backs the operator-facing Settings →
// Audio Device → Input Device / Output Device dropdowns in
// `src/lib/views/Settings.svelte` — `refresh()` invokes the
// `list_audio_devices` Tauri command (`app/shell/src/commands/audio.rs`)
// for inputs; `refreshOutput()` invokes the parallel
// `list_audio_output_devices` command for outputs.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention; the
// Wave 2.1 Lane D.1 `midiStore.svelte.ts` is the load-bearing precedent
// for this shape (class-instance singleton + module-level export).
//
// **Phase posture.** Both engine adapters
// (`engine/audio/src/cpal_source.rs::enumerate_input_devices` +
// `enumerate_output_devices`) collapse the `cpal::Host::input_devices`
// / `cpal::Host::output_devices` failure (CI / headless / no-audio-
// subsystem hosts) into `Ok(Vec::new())`, so the consumer view falls
// back to the W21 PR1 fixture set uniformly for both "audio available
// but no devices connected" and "audio subsystem unavailable".
//
// `refresh()` / `refreshOutput()` swallow the `invoke` rejection that
// fires when running outside Tauri (vite-only `npm run dev`, Vitest
// jsdom with no IPC mock) and record the error onto `error` /
// `outputError` so callers can render an operator-facing banner if
// they choose. The device lists are reset to empty vecs in that case
// so the consumer view's fixture-fallback branch fires.
//
// `subscribe`-style listener detach is intentionally absent: the
// command surface is a pull-model (`refresh()` is on demand), unlike
// the audio snapshot store that listens to a push-model Tauri event
// channel.

import { invoke } from '@tauri-apps/api/core';

import type { AudioDevicePayload } from '../ipc/bindings/AudioDevicePayload';

/** Tauri command name surfaced by `app/shell/src/commands/audio.rs`. */
const LIST_AUDIO_DEVICES_COMMAND = 'list_audio_devices';
/** Wave 2.7 Lane SET.C output-side counterpart. */
const LIST_AUDIO_OUTPUT_DEVICES_COMMAND = 'list_audio_output_devices';

class AudioDevicesStore {
  /** Latest input-enumeration result. Empty vec under the no-devices /
   * no-audio-subsystem branch; non-empty when the host has connected
   * cpal input devices. */
  devices = $state<readonly AudioDevicePayload[]>([]);

  /** Wave 2.7 Lane SET.C — latest output-enumeration result. Same
   * empty-vec graceful-fallback posture as `devices`. */
  outputDevices = $state<readonly AudioDevicePayload[]>([]);

  /** Tracks the in-flight `refresh()` promise so the consumer view can
   * render a loading spinner without dispatching duplicate enumeration
   * calls. */
  isLoading = $state(false);

  /** Wave 2.7 Lane SET.C — output-side counterpart of `isLoading`.
   * Tracked separately so the Input + Output dropdowns can render
   * independent loading states (a slow output enumeration MUST NOT
   * block the input dropdown's spinner from clearing). */
  isLoadingOutput = $state(false);

  /** Last input-refresh error surface, or `null` on success / fresh
   * state. Drives the operator-facing error banner when the input
   * command fails (e.g. running outside Tauri). */
  error = $state<string | null>(null);

  /** Wave 2.7 Lane SET.C — output-side counterpart of `error`. */
  outputError = $state<string | null>(null);

  /** Invoke `list_audio_devices` and replace the input-device list.
   * Sets `isLoading` for the duration; on failure, clears the device
   * list and records the error. Idempotent on the failure path —
   * callers can retry without first clearing state. */
  async refresh(): Promise<void> {
    this.isLoading = true;
    this.error = null;
    try {
      const fresh = await invoke<AudioDevicePayload[]>(LIST_AUDIO_DEVICES_COMMAND);
      // Defensive coerce — a test harness that mocks `invoke` without
      // a return value (or a degenerate runtime path) hands back
      // `undefined`; the consumer view's `audioDevicesStore.devices.map`
      // call would explode in that case. Collapsing to an empty vec
      // routes those cases through the same fixture-fallback branch
      // as the no-devices / no-Tauri-context paths.
      this.devices = Array.isArray(fresh) ? fresh : [];
    } catch (e) {
      this.devices = [];
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.isLoading = false;
    }
  }

  /** Wave 2.7 Lane SET.C — invoke `list_audio_output_devices` and
   * replace the output-device list. Same lifecycle contract as
   * `refresh()`: `isLoadingOutput` toggles around the in-flight
   * promise; on failure, clears the device list and records the
   * error. Idempotent on the failure path. */
  async refreshOutput(): Promise<void> {
    this.isLoadingOutput = true;
    this.outputError = null;
    try {
      const fresh = await invoke<AudioDevicePayload[]>(LIST_AUDIO_OUTPUT_DEVICES_COMMAND);
      this.outputDevices = Array.isArray(fresh) ? fresh : [];
    } catch (e) {
      this.outputDevices = [];
      this.outputError = e instanceof Error ? e.message : String(e);
    } finally {
      this.isLoadingOutput = false;
    }
  }

  /** Test-harness reset. Clears both device lists, both loading
   * flags, and both error fields. */
  reset(): void {
    this.devices = [];
    this.outputDevices = [];
    this.isLoading = false;
    this.isLoadingOutput = false;
    this.error = null;
    this.outputError = null;
  }
}

export const audioDevicesStore = new AudioDevicesStore();
