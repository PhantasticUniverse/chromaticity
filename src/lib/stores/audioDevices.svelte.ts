// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 2.6 Lane AUD.A (input) + Wave 2.7 Lane SET.C (output) cpal
// device-listing singleton store, backed by two independent
// `AsyncResource<readonly AudioDevicePayload[]>` state machines.

import { untrack } from 'svelte';

import { listAudioDevices, listAudioOutputDevices } from '../ipc/audio';
import type { AudioDevicePayload } from '../ipc/bindings/AudioDevicePayload';
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

class AudioDevicesStore {
  /** Explicit discriminated state machine for input device enumeration. */
  inputMachine: AsyncResource<readonly AudioDevicePayload[]> = $state(idleResource([]));

  /** Explicit discriminated state machine for output device enumeration. */
  outputMachine: AsyncResource<readonly AudioDevicePayload[]> = $state(idleResource([]));

  #inputReqId = 0;
  #outputReqId = 0;

  get devices(): readonly AudioDevicePayload[] {
    return this.inputMachine.data;
  }

  set devices(value: readonly AudioDevicePayload[]) {
    this.inputMachine = { kind: 'ready', requestId: ++this.#inputReqId, data: value };
  }

  get outputDevices(): readonly AudioDevicePayload[] {
    return this.outputMachine.data;
  }

  set outputDevices(value: readonly AudioDevicePayload[]) {
    this.outputMachine = { kind: 'ready', requestId: ++this.#outputReqId, data: value };
  }

  get isLoading(): boolean {
    return isResourceLoading(this.inputMachine);
  }

  get isLoadingOutput(): boolean {
    return isResourceLoading(this.outputMachine);
  }

  get error(): string | null {
    return resourceError(this.inputMachine);
  }

  set error(value: string | null) {
    this.inputMachine =
      value === null
        ? { kind: 'ready', requestId: ++this.#inputReqId, data: this.inputMachine.data }
        : {
            kind: 'error',
            requestId: ++this.#inputReqId,
            data: this.inputMachine.data,
            error: value,
          };
  }

  get outputError(): string | null {
    return resourceError(this.outputMachine);
  }

  set outputError(value: string | null) {
    this.outputMachine =
      value === null
        ? { kind: 'ready', requestId: ++this.#outputReqId, data: this.outputMachine.data }
        : {
            kind: 'error',
            requestId: ++this.#outputReqId,
            data: this.outputMachine.data,
            error: value,
          };
  }

  async refresh(): Promise<void> {
    const reqId = ++this.#inputReqId;
    this.inputMachine = beginRequest(
      untrack(() => this.inputMachine),
      reqId,
    );
    try {
      const fresh = await listAudioDevices();
      this.inputMachine = succeedRequest(
        untrack(() => this.inputMachine),
        reqId,
        Array.isArray(fresh) ? fresh : [],
      );
    } catch (e) {
      this.inputMachine = failRequest(
        untrack(() => this.inputMachine),
        reqId,
        formatErrorMessage(e),
        [],
      );
    }
  }

  async refreshOutput(): Promise<void> {
    const reqId = ++this.#outputReqId;
    this.outputMachine = beginRequest(
      untrack(() => this.outputMachine),
      reqId,
    );
    try {
      const fresh = await listAudioOutputDevices();
      this.outputMachine = succeedRequest(
        untrack(() => this.outputMachine),
        reqId,
        Array.isArray(fresh) ? fresh : [],
      );
    } catch (e) {
      this.outputMachine = failRequest(
        untrack(() => this.outputMachine),
        reqId,
        formatErrorMessage(e),
        [],
      );
    }
  }

  reset(): void {
    this.#inputReqId += 1;
    this.#outputReqId += 1;
    this.inputMachine = idleResource([]);
    this.outputMachine = idleResource([]);
  }
}

export const audioDevicesStore = new AudioDevicesStore();

registerStateMachine('audioDevices', () => ({
  inputKind: audioDevicesStore.inputMachine.kind,
  inputCount: audioDevicesStore.devices.length,
  outputKind: audioDevicesStore.outputMachine.kind,
  outputCount: audioDevicesStore.outputDevices.length,
}));
