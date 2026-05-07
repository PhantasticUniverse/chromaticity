// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 1c-β Phase 4 Lane B Syphon-store tests. Validates the
// `list_syphon_servers` / `publish_syphon_output` /
// `unpublish_syphon_output` Tauri command invocations, servers + output
// state propagation, input selection + localStorage persistence
// round-trip, loading-flag lifecycle, default-off output posture, and
// the empty-vec stub posture documented in the module-level prose.
//
// `vi.hoisted` invoke-mock pattern per the
// `feedback_vitest_invoke_mock_hoisted.md` user-memory convention —
// hoisted `vi.mock` factories cannot reference module-scope `const`s.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { SyphonOutputStatusPayload } from '../ipc/bindings/SyphonOutputStatusPayload';
import type { SyphonServerPayload } from '../ipc/bindings/SyphonServerPayload';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const SAMPLE_SERVER_A: SyphonServerPayload = {
  uuid: 'syphon-uuid-aaaa',
  name: 'Composition 1',
  app_name: 'Resolume Arena',
  dictionary_version: 0,
};

const SAMPLE_SERVER_B: SyphonServerPayload = {
  uuid: 'syphon-uuid-bbbb',
  name: 'Master Out',
  app_name: 'MadMapper',
  dictionary_version: 0,
};

const ACTIVE_OUTPUT: SyphonOutputStatusPayload = {
  active: true,
  server_name: 'Chromaticity Performance',
};

const INACTIVE_OUTPUT: SyphonOutputStatusPayload = {
  active: false,
  server_name: null,
};

describe('syphonStore', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.resetModules();
    localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    localStorage.clear();
  });

  it('initial output state is inactive (default-off per ADR-013/ADR-016)', async () => {
    const { syphonStore } = await import('./syphonStore.svelte');
    syphonStore.reset();
    expect(syphonStore.output.active).toBe(false);
    expect(syphonStore.output.server_name).toBeNull();
  });

  it('refreshServers() invokes list_syphon_servers and stores the returned vec', async () => {
    invokeMock.mockResolvedValue([SAMPLE_SERVER_A, SAMPLE_SERVER_B]);
    const { syphonStore } = await import('./syphonStore.svelte');
    syphonStore.reset();
    await syphonStore.refreshServers();
    expect(invokeMock).toHaveBeenCalledWith('list_syphon_servers');
    expect(syphonStore.servers).toEqual([SAMPLE_SERVER_A, SAMPLE_SERVER_B]);
    expect(syphonStore.error).toBeNull();
    expect(syphonStore.isLoading).toBe(false);
  });

  it('refreshServers() surfaces an empty vec verbatim under the scaffold stub', async () => {
    // Phase 4 Lane B documented stub: SyphonServerDirectory's
    // current_servers() returns an empty list until the engine-side
    // NSDistributedNotificationCenter observer wires end-to-end.
    invokeMock.mockResolvedValue([]);
    const { syphonStore } = await import('./syphonStore.svelte');
    syphonStore.reset();
    await syphonStore.refreshServers();
    expect(syphonStore.servers).toEqual([]);
    expect(syphonStore.error).toBeNull();
  });

  it('refreshServers() captures errors and clears the server list on failure', async () => {
    invokeMock.mockRejectedValue(new Error('syphon: macOS-only feature'));
    const { syphonStore } = await import('./syphonStore.svelte');
    syphonStore.reset();
    syphonStore.servers = [SAMPLE_SERVER_A];
    await syphonStore.refreshServers();
    expect(syphonStore.servers).toEqual([]);
    expect(syphonStore.error).toBe('syphon: macOS-only feature');
    expect(syphonStore.isLoading).toBe(false);
  });

  it('publishOutput(name) invokes publish_syphon_output with the supplied name and updates state', async () => {
    invokeMock.mockResolvedValue(ACTIVE_OUTPUT);
    const { syphonStore } = await import('./syphonStore.svelte');
    syphonStore.reset();
    await syphonStore.publishOutput('Chromaticity Performance');
    expect(invokeMock).toHaveBeenCalledWith('publish_syphon_output', {
      name: 'Chromaticity Performance',
    });
    expect(syphonStore.output).toEqual(ACTIVE_OUTPUT);
    expect(syphonStore.error).toBeNull();
  });

  it('publishOutput() captures errors and leaves prior output state intact on failure', async () => {
    invokeMock.mockRejectedValue(new Error('not running in tauri'));
    const { syphonStore } = await import('./syphonStore.svelte');
    syphonStore.reset();
    await syphonStore.publishOutput('Test');
    // Prior state remains (still inactive): the publish attempt did
    // not transition the framework state.
    expect(syphonStore.output.active).toBe(false);
    expect(syphonStore.error).toBe('not running in tauri');
  });

  it('unpublishOutput() invokes unpublish_syphon_output and updates state to inactive', async () => {
    invokeMock.mockResolvedValueOnce(ACTIVE_OUTPUT).mockResolvedValueOnce(INACTIVE_OUTPUT);
    const { syphonStore } = await import('./syphonStore.svelte');
    syphonStore.reset();
    await syphonStore.publishOutput('Chromaticity Performance');
    expect(syphonStore.output.active).toBe(true);
    await syphonStore.unpublishOutput();
    expect(invokeMock).toHaveBeenLastCalledWith('unpublish_syphon_output');
    expect(syphonStore.output).toEqual(INACTIVE_OUTPUT);
    expect(syphonStore.error).toBeNull();
  });

  it('selectInput(uuid) updates `selectedInput` and persists to localStorage', async () => {
    const { syphonStore } = await import('./syphonStore.svelte');
    syphonStore.reset();
    syphonStore.selectInput('syphon-uuid-aaaa');
    expect(syphonStore.selectedInput).toBe('syphon-uuid-aaaa');
    expect(localStorage.getItem('chromaticity.syphon.selectedInput')).toBe('syphon-uuid-aaaa');
  });

  it('selectInput(null) clears `selectedInput` and removes the persisted key', async () => {
    const { syphonStore } = await import('./syphonStore.svelte');
    syphonStore.reset();
    syphonStore.selectInput('syphon-uuid-bbbb');
    syphonStore.selectInput(null);
    expect(syphonStore.selectedInput).toBeNull();
    expect(localStorage.getItem('chromaticity.syphon.selectedInput')).toBeNull();
  });

  it('loadPersistedSelection() reads the prior input selection on init', async () => {
    localStorage.setItem('chromaticity.syphon.selectedInput', 'syphon-uuid-bbbb');
    const { syphonStore } = await import('./syphonStore.svelte');
    syphonStore.reset();
    syphonStore.loadPersistedSelection();
    expect(syphonStore.selectedInput).toBe('syphon-uuid-bbbb');
  });

  it('loadPersistedSelection() leaves `selectedInput` at null when nothing is persisted', async () => {
    const { syphonStore } = await import('./syphonStore.svelte');
    syphonStore.reset();
    syphonStore.loadPersistedSelection();
    expect(syphonStore.selectedInput).toBeNull();
  });

  it('isLoading toggles around the in-flight refreshServers promise', async () => {
    let resolveInvoke: (value: SyphonServerPayload[]) => void = () => {};
    invokeMock.mockImplementation(
      () =>
        new Promise<SyphonServerPayload[]>((resolve) => {
          resolveInvoke = resolve;
        }),
    );
    const { syphonStore } = await import('./syphonStore.svelte');
    syphonStore.reset();
    const refreshPromise = syphonStore.refreshServers();
    expect(syphonStore.isLoading).toBe(true);
    resolveInvoke([SAMPLE_SERVER_A]);
    await refreshPromise;
    expect(syphonStore.isLoading).toBe(false);
    expect(syphonStore.servers).toEqual([SAMPLE_SERVER_A]);
  });
});
