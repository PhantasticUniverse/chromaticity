// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.4 Lane PERF.D OSC health polling store, upgraded to an
// explicit polling lifecycle + `AsyncResource<OscHealthSnapshot | null>`.

import { untrack } from 'svelte';

import type { OscHealthSnapshot } from '../ipc/bindings';
import {
  beginRequest,
  failRequest,
  formatErrorMessage,
  idleResource,
  registerStateMachine,
  resourceError,
  succeedRequest,
  type AsyncResource,
} from '../state';

export type OscHealthPollingState = 'stopped' | 'polling';

class OscHealthStore {
  /** Explicit discriminated async state for health snapshots. */
  machine: AsyncResource<OscHealthSnapshot | null> = $state(idleResource(null));

  /** Explicit polling lifecycle state. */
  pollingState: OscHealthPollingState = $state('stopped');

  #reqId = 0;
  #interval: ReturnType<typeof setInterval> | null = null;

  get snapshot(): OscHealthSnapshot | null {
    return this.machine.data;
  }

  set snapshot(value: OscHealthSnapshot | null) {
    this.machine =
      value === null
        ? idleResource(null)
        : { kind: 'ready', requestId: ++this.#reqId, data: value };
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
      const { invoke } = await import('@tauri-apps/api/core');
      const fresh = await invoke<OscHealthSnapshot>('osc_health_snapshot');
      this.machine = succeedRequest(
        untrack(() => this.machine),
        reqId,
        fresh,
      );
    } catch (err) {
      this.machine = failRequest(
        untrack(() => this.machine),
        reqId,
        formatErrorMessage(err),
      );
    }
  }

  start(intervalMs = 5000): void {
    if (untrack(() => this.pollingState) === 'polling') return;
    this.pollingState = 'polling';
    void this.refresh();
    this.#interval = setInterval(() => {
      void this.refresh();
    }, intervalMs);
  }

  stop(): void {
    if (this.#interval) {
      clearInterval(this.#interval);
      this.#interval = null;
    }
    this.pollingState = 'stopped';
  }

  reset(): void {
    this.stop();
    this.#reqId += 1;
    this.machine = idleResource(null);
  }
}

export const oscHealthStore = new OscHealthStore();

registerStateMachine('oscHealth', () => ({
  polling: oscHealthStore.pollingState,
  kind: oscHealthStore.machine.kind,
  hasSnapshot: oscHealthStore.snapshot !== null,
  error: oscHealthStore.error,
}));
