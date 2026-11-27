// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane D.1 MIDI device-listing singleton store,
// backed by `AsyncResource<readonly MidiDevicePayload[]>`.

import { invoke } from '@tauri-apps/api/core';
import { untrack } from 'svelte';

import type { MidiDevicePayload } from '../ipc/bindings/MidiDevicePayload';
import {
  beginRequest,
  failRequest,
  formatErrorMessage,
  idleResource,
  isResourceLoading,
  registerStateMachine,
  resourceError,
  succeedRequest,
  type AsyncResource,
} from '../state';

const LIST_MIDI_DEVICES_COMMAND = 'list_midi_devices';

class MidiStore {
  /** Explicit discriminated state machine for MIDI device enumeration. */
  machine: AsyncResource<readonly MidiDevicePayload[]> = $state(idleResource([]));

  #reqId = 0;

  get devices(): readonly MidiDevicePayload[] {
    return this.machine.data;
  }

  set devices(value: readonly MidiDevicePayload[]) {
    this.machine = { kind: 'ready', requestId: ++this.#reqId, data: value };
  }

  get isLoading(): boolean {
    return isResourceLoading(this.machine);
  }

  get error(): string | null {
    return resourceError(this.machine);
  }

  set error(value: string | null) {
    this.machine =
      value === null
        ? { kind: 'ready', requestId: ++this.#reqId, data: this.machine.data }
        : { kind: 'error', requestId: ++this.#reqId, data: this.machine.data, error: value };
  }

  async refresh(): Promise<void> {
    const reqId = ++this.#reqId;
    this.machine = beginRequest(
      untrack(() => this.machine),
      reqId,
    );
    try {
      const fresh = await invoke<MidiDevicePayload[]>(LIST_MIDI_DEVICES_COMMAND);
      this.machine = succeedRequest(
        untrack(() => this.machine),
        reqId,
        fresh,
      );
    } catch (e) {
      this.machine = failRequest(
        untrack(() => this.machine),
        reqId,
        formatErrorMessage(e),
        [],
      );
    }
  }

  reset(): void {
    this.#reqId += 1;
    this.machine = idleResource([]);
  }
}

export const midiStore = new MidiStore();

registerStateMachine('midiDevices', () => ({
  kind: midiStore.machine.kind,
  deviceCount: midiStore.devices.length,
  error: midiStore.error,
}));
