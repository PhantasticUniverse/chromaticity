// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 1c-β Phase 4 Lane B Syphon source-picker + output-publish
// singleton store, backed by `AsyncResource<readonly SyphonServerPayload[]>`.

import { untrack } from 'svelte';

import type { SyphonOutputStatusPayload } from '../ipc/bindings/SyphonOutputStatusPayload';
import type { SyphonServerPayload } from '../ipc/bindings/SyphonServerPayload';
import { listSyphonServers, publishSyphonOutput, unpublishSyphonOutput } from '../ipc/syphon';
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

const PERSISTED_INPUT_KEY = 'chromaticity.syphon.selectedInput';
export const DEFAULT_OUTPUT_NAME = 'Chromaticity Performance';

class SyphonStore {
  /** Explicit discriminated state machine for server enumeration. */
  serversMachine: AsyncResource<readonly SyphonServerPayload[]> = $state(idleResource([]));

  /** Operator-selected input server UUID, or `null`. */
  selectedInput = $state<string | null>(null);

  /** Output-publish status (default-off per ADR-013 / ADR-016 / ADR-021 §10). */
  output = $state<SyphonOutputStatusPayload>({ active: false, server_name: null });

  /** Last publish/unpublish error when distinct from server-list errors. */
  #outputError = $state<string | null>(null);

  #reqId = 0;

  get servers(): readonly SyphonServerPayload[] {
    return this.serversMachine.data;
  }

  set servers(value: readonly SyphonServerPayload[]) {
    this.serversMachine = { kind: 'ready', requestId: ++this.#reqId, data: value };
  }

  get isLoading(): boolean {
    return isResourceLoading(this.serversMachine);
  }

  get error(): string | null {
    return this.#outputError ?? resourceError(this.serversMachine);
  }

  set error(value: string | null) {
    this.#outputError = value;
  }

  async refreshServers(): Promise<void> {
    const reqId = ++this.#reqId;
    this.#outputError = null;
    this.serversMachine = beginRequest(
      untrack(() => this.serversMachine),
      reqId,
    );
    try {
      const fresh = await listSyphonServers();
      this.serversMachine = succeedRequest(
        untrack(() => this.serversMachine),
        reqId,
        fresh,
      );
    } catch (e) {
      this.serversMachine = failRequest(
        untrack(() => this.serversMachine),
        reqId,
        formatErrorMessage(e),
        [],
      );
    }
  }

  async publishOutput(name: string): Promise<void> {
    this.#outputError = null;
    try {
      this.output = await publishSyphonOutput(name);
    } catch (e) {
      this.#outputError = formatErrorMessage(e);
    }
  }

  async unpublishOutput(): Promise<void> {
    this.#outputError = null;
    try {
      this.output = await unpublishSyphonOutput();
    } catch (e) {
      this.#outputError = formatErrorMessage(e);
    }
  }

  selectInput(uuid: string | null): void {
    this.selectedInput = uuid;
    if (typeof localStorage === 'undefined') return;
    try {
      if (uuid) {
        localStorage.setItem(PERSISTED_INPUT_KEY, uuid);
      } else {
        localStorage.removeItem(PERSISTED_INPUT_KEY);
      }
    } catch {
      // Swallow storage exceptions — in-memory selection stays intact.
    }
  }

  loadPersistedSelection(): void {
    if (typeof localStorage === 'undefined') return;
    try {
      const persisted = localStorage.getItem(PERSISTED_INPUT_KEY);
      if (persisted) this.selectedInput = persisted;
    } catch {
      // Swallow — no persisted selection is the safe default.
    }
  }

  reset(): void {
    this.#reqId += 1;
    this.serversMachine = idleResource([]);
    this.selectedInput = null;
    this.output = { active: false, server_name: null };
    this.#outputError = null;
  }
}

export const syphonStore = new SyphonStore();

registerStateMachine('syphon', () => ({
  kind: syphonStore.serversMachine.kind,
  serverCount: syphonStore.servers.length,
  outputActive: syphonStore.output.active,
  error: syphonStore.error,
}));
