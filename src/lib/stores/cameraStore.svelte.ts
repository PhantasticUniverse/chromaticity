// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 1c-β Phase 4 Lane A camera device-picker singleton store,
// backed by `AsyncResource<readonly CameraDevicePayload[]>`.

import { invoke } from '@tauri-apps/api/core';
import { untrack } from 'svelte';

import type { CameraDevicePayload } from '../ipc/bindings/CameraDevicePayload';
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

const PERSISTED_DEVICE_KEY = 'chromaticity.camera.selectedDevice';
const LIST_CAMERA_DEVICES_COMMAND = 'list_camera_devices';

class CameraStore {
  /** Explicit discriminated state machine for camera enumeration. */
  machine: AsyncResource<readonly CameraDevicePayload[]> = $state(idleResource([]));

  /** Operator-selected device id, or `null`. */
  selected = $state<string | null>(null);

  #reqId = 0;

  get devices(): readonly CameraDevicePayload[] {
    return this.machine.data;
  }

  set devices(value: readonly CameraDevicePayload[]) {
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
      const fresh = await invoke<CameraDevicePayload[]>(LIST_CAMERA_DEVICES_COMMAND);
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

  select(deviceId: string | null): void {
    this.selected = deviceId;
    if (typeof localStorage === 'undefined') return;
    try {
      if (deviceId) {
        localStorage.setItem(PERSISTED_DEVICE_KEY, deviceId);
      } else {
        localStorage.removeItem(PERSISTED_DEVICE_KEY);
      }
    } catch {
      // Swallow storage exceptions — in-memory selection remains intact.
    }
  }

  loadPersistedSelection(): void {
    if (typeof localStorage === 'undefined') return;
    try {
      const persisted = localStorage.getItem(PERSISTED_DEVICE_KEY);
      if (persisted) this.selected = persisted;
    } catch {
      // Swallow — no persisted selection is the safe default.
    }
  }

  reset(): void {
    this.#reqId += 1;
    this.machine = idleResource([]);
    this.selected = null;
  }
}

export const cameraStore = new CameraStore();

registerStateMachine('camera', () => ({
  kind: cameraStore.machine.kind,
  deviceCount: cameraStore.devices.length,
  selected: cameraStore.selected,
  error: cameraStore.error,
}));
