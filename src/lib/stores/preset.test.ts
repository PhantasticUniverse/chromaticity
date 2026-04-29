// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Phase-1 D4 preset round-trip helper tests. The module exposes
// `dispatchPresetRecallAndRead` (recall + read-back) and
// `readActivePreset` (read-only mount-time hydration). Tests cover
// strict IPC ordering, payload shape, the `null` sentinel for "no
// active preset", and rejection propagation.
//
// `vi.hoisted` invoke mock per the project convention.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import { dispatchPresetRecallAndRead, readActivePreset } from './preset';

describe('preset round-trip helpers', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('dispatchPresetRecallAndRead', () => {
    it('issues recall_preset before active_preset', async () => {
      const calls: string[] = [];
      invokeMock.mockImplementation(async (cmd: string) => {
        calls.push(cmd);
        if (cmd === 'recall_preset') return undefined;
        if (cmd === 'active_preset') return 4;
        throw new Error(`unhandled-${cmd}`);
      });
      await dispatchPresetRecallAndRead(4);
      expect(calls).toEqual(['recall_preset', 'active_preset']);
    });

    it('forwards the id payload to recall_preset', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'recall_preset') return undefined;
        if (cmd === 'active_preset') return 12;
        throw new Error(`unhandled-${cmd}`);
      });
      await dispatchPresetRecallAndRead(12);
      expect(invokeMock).toHaveBeenNthCalledWith(1, 'recall_preset', { id: 12 });
    });

    it('returns the read-back active preset id verbatim', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'recall_preset') return undefined;
        if (cmd === 'active_preset') return 9;
        throw new Error(`unhandled-${cmd}`);
      });
      const id = await dispatchPresetRecallAndRead(9);
      expect(id).toBe(9);
    });

    it('returns null when the runtime reports no active preset', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'recall_preset') return undefined;
        if (cmd === 'active_preset') return null;
        throw new Error(`unhandled-${cmd}`);
      });
      const id = await dispatchPresetRecallAndRead(0);
      expect(id).toBeNull();
    });

    it('propagates a recall_preset rejection without reading back', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'recall_preset') throw new Error('preset id rejected');
        if (cmd === 'active_preset') return null;
        throw new Error(`unhandled-${cmd}`);
      });
      await expect(dispatchPresetRecallAndRead(7)).rejects.toThrow('preset id rejected');
      expect(invokeMock).toHaveBeenCalledTimes(1);
    });

    it('propagates an active_preset rejection after successful recall', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'recall_preset') return undefined;
        if (cmd === 'active_preset') throw new Error('read-back failed');
        throw new Error(`unhandled-${cmd}`);
      });
      await expect(dispatchPresetRecallAndRead(7)).rejects.toThrow('read-back failed');
      expect(invokeMock).toHaveBeenCalledTimes(2);
    });
  });

  describe('readActivePreset', () => {
    it('issues exactly one active_preset call with no payload', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'active_preset') return 3;
        throw new Error(`unhandled-${cmd}`);
      });
      const id = await readActivePreset();
      expect(invokeMock).toHaveBeenCalledTimes(1);
      expect(invokeMock).toHaveBeenCalledWith('active_preset');
      expect(id).toBe(3);
    });

    it('returns null on a fresh-runtime read with no recall yet', async () => {
      invokeMock.mockImplementation(async () => null);
      const id = await readActivePreset();
      expect(id).toBeNull();
    });

    it('propagates a runtime read failure', async () => {
      invokeMock.mockImplementation(async () => {
        throw new Error('preset registry unreadable');
      });
      await expect(readActivePreset()).rejects.toThrow('preset registry unreadable');
    });
  });
});
