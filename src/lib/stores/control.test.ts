// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Phase-1 D2 control round-trip helper tests. `dispatchAndRead`
// dispatches a `set_control` IPC, then reads the committed value back
// via `get_control`; the contract under test is the strict ordering
// (set BEFORE get), single-call-per-side semantics, and the read-back
// pass-through (including the `null` sentinel for unset controls).
//
// Per `feedback_vitest_invoke_mock_hoisted.md`: the `invoke` mock is
// lifted via `vi.hoisted` so the `vi.mock` factory can reference it
// without a TDZ access on hoisted module init. Mirrors the pattern
// landed in W55 / W57 (`views/Osc.test.ts`, `views/Library.test.ts`).

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import { dispatchAndRead } from './control';

describe('dispatchAndRead (control round-trip)', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('issues set_control before get_control', async () => {
    const calls: string[] = [];
    invokeMock.mockImplementation(async (cmd: string) => {
      calls.push(cmd);
      if (cmd === 'set_control') return undefined;
      if (cmd === 'get_control') return 0.5;
      throw new Error(`unhandled-${cmd}`);
    });
    await dispatchAndRead(7, 0.5);
    expect(calls).toEqual(['set_control', 'get_control']);
  });

  it('forwards the (id, value) payload to set_control', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'set_control') return undefined;
      if (cmd === 'get_control') return 0.25;
      throw new Error(`unhandled-${cmd}`);
    });
    await dispatchAndRead(42, 0.25);
    expect(invokeMock).toHaveBeenNthCalledWith(1, 'set_control', { id: 42, value: 0.25 });
  });

  it('forwards the id to get_control on the read-back', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'set_control') return undefined;
      if (cmd === 'get_control') return 1.0;
      throw new Error(`unhandled-${cmd}`);
    });
    await dispatchAndRead(7, 1.0);
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'get_control', { id: 7 });
  });

  it('returns the get_control read-back value verbatim', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'set_control') return undefined;
      if (cmd === 'get_control') return 0.875;
      throw new Error(`unhandled-${cmd}`);
    });
    const result = await dispatchAndRead(3, 0.5);
    expect(result).toBe(0.875);
  });

  it('returns null when the runtime reports the control as unset', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'set_control') return undefined;
      if (cmd === 'get_control') return null;
      throw new Error(`unhandled-${cmd}`);
    });
    const result = await dispatchAndRead(99, 0.0);
    expect(result).toBeNull();
  });

  it('propagates a set_control rejection without issuing get_control', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'set_control') throw new Error('runtime offline');
      if (cmd === 'get_control') return 0.0;
      throw new Error(`unhandled-${cmd}`);
    });
    await expect(dispatchAndRead(0, 0.0)).rejects.toThrow('runtime offline');
    // get_control must not fire when the dispatch leg fails.
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('set_control', { id: 0, value: 0.0 });
  });

  it('propagates a get_control rejection after a successful set_control', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'set_control') return undefined;
      if (cmd === 'get_control') throw new Error('read-back failed');
      throw new Error(`unhandled-${cmd}`);
    });
    await expect(dispatchAndRead(0, 0.0)).rejects.toThrow('read-back failed');
    expect(invokeMock).toHaveBeenCalledTimes(2);
  });
});
