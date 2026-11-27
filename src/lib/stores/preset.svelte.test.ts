// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 2.1 Lane A.4 preset store tests. Validates the
// `recall_preset` + `active_preset` invocation paths, reactive state
// propagation, and the no-Tauri-runtime swallow path.
//
// `vi.hoisted` invoke-mock pattern per the
// `feedback_vitest_invoke_mock_hoisted.md` user-memory convention —
// hoisted `vi.mock` factories cannot reference module-scope `const`s.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

describe('presetStore', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.resetModules();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('refresh() invokes active_preset and stores the returned id', async () => {
    invokeMock.mockResolvedValueOnce(7);
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();

    await presetStore.refresh();

    expect(invokeMock).toHaveBeenCalledWith('active_preset');
    expect(presetStore.activeId).toBe(7);
  });

  it('refresh() stores null when the engine reports no active preset', async () => {
    invokeMock.mockResolvedValueOnce(null);
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();

    await presetStore.refresh();

    expect(presetStore.activeId).toBeNull();
  });

  it('recall() invokes recall_preset with id then refreshes activeId', async () => {
    const calls: Array<[string, unknown]> = [];
    invokeMock.mockImplementation(async (cmd: string, args?: unknown) => {
      calls.push([cmd, args]);
      if (cmd === 'recall_preset') return undefined;
      if (cmd === 'active_preset') return 12;
      throw new Error(`unhandled-${cmd}`);
    });
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();

    await presetStore.recall(12);

    expect(calls.length).toBe(2);
    expect(calls[0]).toEqual(['recall_preset', { id: 12 }]);
    expect(calls[1]?.[0]).toBe('active_preset');
    expect(presetStore.activeId).toBe(12);
  });

  it('recall() reflects an engine clamp by storing the read-back id, not the input', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'recall_preset') return undefined;
      if (cmd === 'active_preset') return 3; // engine reports a different id
      throw new Error(`unhandled-${cmd}`);
    });
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();

    await presetStore.recall(99);

    expect(presetStore.activeId).toBe(3);
  });

  it('refresh() swallows invoke rejection (no Tauri runtime) and leaves activeId unchanged', async () => {
    invokeMock.mockRejectedValueOnce(new Error('not running in tauri'));
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();

    await expect(presetStore.refresh()).resolves.toBeUndefined();
    expect(presetStore.activeId).toBeNull();
  });

  it('recall() swallows invoke rejection and leaves activeId unchanged', async () => {
    invokeMock.mockRejectedValueOnce(new Error('not running in tauri'));
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();

    await expect(presetStore.recall(5)).resolves.toBeUndefined();
    expect(presetStore.activeId).toBeNull();
  });

  it('save() invokes save_preset with id then refreshes activeId (Wave 2.4 PERF.C)', async () => {
    const calls: Array<[string, unknown]> = [];
    invokeMock.mockImplementation(async (cmd: string, args?: unknown) => {
      calls.push([cmd, args]);
      if (cmd === 'save_preset') return undefined;
      if (cmd === 'active_preset') return 4;
      throw new Error(`unhandled-${cmd}`);
    });
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();

    await presetStore.save(4);

    expect(calls.length).toBe(2);
    expect(calls[0]).toEqual(['save_preset', { id: 4 }]);
    expect(calls[1]?.[0]).toBe('active_preset');
    expect(presetStore.activeId).toBe(4);
  });

  it('save() swallows invoke rejection and leaves activeId unchanged', async () => {
    invokeMock.mockRejectedValueOnce(new Error('not running in tauri'));
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();

    await expect(presetStore.save(5)).resolves.toBeUndefined();
    expect(presetStore.activeId).toBeNull();
  });

  it('reset() returns activeId to the baseline null', async () => {
    invokeMock.mockResolvedValueOnce(9);
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();
    await presetStore.refresh();
    expect(presetStore.activeId).toBe(9);

    presetStore.reset();

    expect(presetStore.activeId).toBeNull();
  });
});
