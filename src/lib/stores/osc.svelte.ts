// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 2.1 Lane D.3 OSC route binding singleton store,
// backed by `AsyncResource<readonly OscRoutePayload[]>`.

import { invoke } from '@tauri-apps/api/core';
import { untrack } from 'svelte';

import type { OscRoutePayload } from '../ipc/bindings/OscRoutePayload';
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

const OSC_ROUTES_SNAPSHOT_COMMAND = 'osc_routes_snapshot';
const BIND_OSC_ROUTE_COMMAND = 'bind_osc_route';
const UNBIND_OSC_ROUTE_COMMAND = 'unbind_osc_route';

class OscStore {
  /** Explicit discriminated state machine for OSC route enumeration. */
  machine: AsyncResource<readonly OscRoutePayload[]> = $state(idleResource([]));

  /** Last bind/unbind mutation error when distinct from refresh errors. */
  #mutationError = $state<string | null>(null);

  #reqId = 0;

  get routes(): readonly OscRoutePayload[] {
    return this.machine.data;
  }

  set routes(value: readonly OscRoutePayload[]) {
    this.machine = { kind: 'ready', requestId: ++this.#reqId, data: value };
  }

  get isLoading(): boolean {
    return isResourceLoading(this.machine);
  }

  get error(): string | null {
    return this.#mutationError ?? resourceError(this.machine);
  }

  set error(value: string | null) {
    this.#mutationError = value;
  }

  async refresh(): Promise<void> {
    const reqId = ++this.#reqId;
    this.#mutationError = null;
    this.machine = beginRequest(
      untrack(() => this.machine),
      reqId,
    );
    try {
      const fresh = await invoke<OscRoutePayload[]>(OSC_ROUTES_SNAPSHOT_COMMAND);
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

  async bind(route: string, target: string): Promise<void> {
    this.#mutationError = null;
    try {
      await invoke(BIND_OSC_ROUTE_COMMAND, { route, target });
      await this.refresh();
    } catch (e) {
      this.#mutationError = formatErrorMessage(e);
    }
  }

  async unbind(route: string): Promise<void> {
    this.#mutationError = null;
    try {
      await invoke(UNBIND_OSC_ROUTE_COMMAND, { route });
      await this.refresh();
    } catch (e) {
      this.#mutationError = formatErrorMessage(e);
    }
  }

  reset(): void {
    this.#reqId += 1;
    this.machine = idleResource([]);
    this.#mutationError = null;
  }
}

export const oscStore = new OscStore();

registerStateMachine('oscRoutes', () => ({
  kind: oscStore.machine.kind,
  routeCount: oscStore.routes.length,
  error: oscStore.error,
}));
