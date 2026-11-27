// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane A.1 loaded-scene IPC consumer store, backed by the
// explicit `AsyncResource<LoadedScenePayload | null>` state machine so
// rapid scene double-clicks cannot resolve out of order.

import { untrack } from 'svelte';

import type { LoadedScenePayload } from '../ipc/bindings';
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

class LoadedSceneStore {
  /** Explicit discriminated async state machine. */
  machine: AsyncResource<LoadedScenePayload | null> = $state(idleResource(null));

  /** Monotonic request sequence id to drop stale out-of-order scene loads. */
  #nextRequestId = 0;

  /** Back-compat projection: most recently loaded scene payload, or `null`. */
  get snapshot(): LoadedScenePayload | null {
    return this.machine.data;
  }

  set snapshot(value: LoadedScenePayload | null) {
    this.machine =
      value === null
        ? idleResource(null)
        : { kind: 'ready', requestId: ++this.#nextRequestId, data: value };
  }

  /** Back-compat projection: most recent load-attempt error message, or `null`. */
  get error(): string | null {
    return resourceError(this.machine);
  }

  set error(value: string | null) {
    this.machine =
      value === null
        ? { kind: 'ready', requestId: ++this.#nextRequestId, data: this.machine.data }
        : {
            kind: 'error',
            requestId: ++this.#nextRequestId,
            data: this.machine.data,
            error: value,
          };
  }

  /** Back-compat projection: `true` while a `load()` call is in flight. */
  get loading(): boolean {
    return isResourceLoading(this.machine);
  }

  /**
   * Load the `.chroma` package at `path` into the runtime and update
   * `snapshot`. Out-of-order responses from superseded loads are dropped.
   */
  async load(path: string): Promise<LoadedScenePayload | null> {
    const reqId = ++this.#nextRequestId;
    this.machine = beginRequest(
      untrack(() => this.machine),
      reqId,
    );
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const payload = await invoke<LoadedScenePayload>('load_scene', { path });
      const next = succeedRequest(
        untrack(() => this.machine),
        reqId,
        payload,
      );
      this.machine = next;
      return next.kind === 'ready' && next.requestId === reqId ? payload : null;
    } catch (err) {
      this.machine = failRequest(
        untrack(() => this.machine),
        reqId,
        formatErrorMessage(err),
      );
      return null;
    }
  }

  /**
   * Read the most recently loaded scene from the engine and hydrate
   * the store. Ignored if a `load()` is already in flight.
   */
  async hydrate(): Promise<void> {
    if (untrack(() => this.machine.kind) === 'loading') return;
    const reqId = ++this.#nextRequestId;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const payload = await invoke<LoadedScenePayload | null>('loaded_scene');
      // Only apply if no newer load/reset started while hydrate was awaiting.
      if (reqId === this.#nextRequestId && payload) {
        this.machine = { kind: 'ready', requestId: reqId, data: payload };
      }
    } catch {
      // Swallow — no Tauri runtime available; consumers fall back to fixtures.
    }
  }

  /** Reset the store to the no-scene-loaded baseline (test harness). */
  reset(): void {
    this.#nextRequestId += 1;
    this.machine = idleResource(null);
  }
}

export const loadedSceneStore = new LoadedSceneStore();

registerStateMachine('loadedScene', () => ({
  kind: loadedSceneStore.machine.kind,
  sceneId: loadedSceneStore.snapshot?.id ?? null,
  error: loadedSceneStore.error,
}));
