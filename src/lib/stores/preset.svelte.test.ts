// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane A.4 preset store tests. Validates the
// `recall_preset` + `active_preset` invocation paths, reactive state
// propagation, and the error-surface path: an `invoke()` rejection
// (real engine `Result<_, String>` failure) is recorded into
// `presetStore.error` and cleared by the next successful call. The
// no-Tauri-runtime fallback (import-level rejection) stays silent
// and is not reachable here — `vi.mock` makes the module import
// always succeed.
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

  it('refresh() records invoke rejection in error and leaves activeId unchanged', async () => {
    invokeMock.mockRejectedValueOnce(new Error('state mutex poisoned: boom'));
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();

    await expect(presetStore.refresh()).resolves.toBeUndefined();
    expect(presetStore.activeId).toBeNull();
    expect(presetStore.error).toBe('state mutex poisoned: boom');
  });

  it('recall() records invoke rejection in error and leaves activeId unchanged', async () => {
    invokeMock.mockRejectedValueOnce(new Error('state mutex poisoned: boom'));
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();

    await expect(presetStore.recall(5)).resolves.toBeUndefined();
    expect(presetStore.activeId).toBeNull();
    expect(presetStore.error).toBe('state mutex poisoned: boom');
  });

  it('records a non-Error rejection (engine Result<_, String> payload) via String()', async () => {
    invokeMock.mockRejectedValueOnce('command bus backpressure');
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();

    await presetStore.refresh();

    expect(presetStore.error).toBe('command bus backpressure');
  });

  it('a successful call clears a previously recorded error', async () => {
    invokeMock.mockRejectedValueOnce(new Error('state mutex poisoned: boom'));
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();

    await presetStore.refresh();
    expect(presetStore.error).toBe('state mutex poisoned: boom');

    invokeMock.mockResolvedValueOnce(2);
    await presetStore.refresh();

    expect(presetStore.error).toBeNull();
    expect(presetStore.activeId).toBe(2);
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

  it('save() records invoke rejection in error and leaves activeId unchanged', async () => {
    invokeMock.mockRejectedValueOnce(new Error('state mutex poisoned: boom'));
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();

    await expect(presetStore.save(5)).resolves.toBeUndefined();
    expect(presetStore.activeId).toBeNull();
    expect(presetStore.error).toBe('state mutex poisoned: boom');
  });

  it('reset() returns activeId and error to the baseline', async () => {
    invokeMock.mockResolvedValueOnce(9);
    const { presetStore } = await import('./preset.svelte');
    presetStore.reset();
    await presetStore.refresh();
    expect(presetStore.activeId).toBe(9);
    invokeMock.mockRejectedValueOnce(new Error('boom'));
    await presetStore.refresh();
    expect(presetStore.error).toBe('boom');

    presetStore.reset();

    expect(presetStore.activeId).toBeNull();
    expect(presetStore.error).toBeNull();
  });
});
