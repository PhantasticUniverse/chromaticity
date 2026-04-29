// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Phase-1 D3 Snap round-trip helper tests. The module exposes thin
// `saveSnap` / `recallSnap` / `clearSnap` dispatchers plus
// `readSnapState` which aggregates `active_snap_slot` with sixteen
// `snap_slot_saved` reads into a single snapshot. Tests cover the
// per-slot dispatcher pass-through and the aggregate read shape: 16
// boolean cells indexed by slot, plus the active-slot accessor.
//
// `vi.hoisted` invoke mock per the project convention.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import { clearSnap, readSnapState, recallSnap, saveSnap } from './snap';

describe('snap dispatcher helpers', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('saveSnap forwards { slot } to save_snap exactly once', async () => {
    invokeMock.mockResolvedValue(undefined);
    await saveSnap(3);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('save_snap', { slot: 3 });
  });

  it('recallSnap forwards { slot } to recall_snap exactly once', async () => {
    invokeMock.mockResolvedValue(undefined);
    await recallSnap(7);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('recall_snap', { slot: 7 });
  });

  it('clearSnap forwards { slot } to clear_snap exactly once', async () => {
    invokeMock.mockResolvedValue(undefined);
    await clearSnap(15);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('clear_snap', { slot: 15 });
  });

  it('saveSnap propagates an engine-side rejection (e.g. SnapSlotId::new failure)', async () => {
    invokeMock.mockRejectedValue(new Error('slot id out of range'));
    await expect(saveSnap(99)).rejects.toThrow('slot id out of range');
  });
});

describe('readSnapState (aggregated read)', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('issues active_snap_slot and 16 snap_slot_saved calls (one per slot)', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'active_snap_slot') return null;
      if (cmd === 'snap_slot_saved') return false;
      throw new Error(`unhandled-${cmd}`);
    });
    await readSnapState();
    expect(invokeMock).toHaveBeenCalledTimes(17); // 1 active + 16 saved
    const savedCalls = invokeMock.mock.calls.filter((c) => c[0] === 'snap_slot_saved');
    expect(savedCalls.length).toBe(16);
    // Each slot 0..15 is queried exactly once.
    const slots = savedCalls.map((c) => (c[1] as { slot: number }).slot).sort((a, b) => a - b);
    expect(slots).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
  });

  it('returns active=null when no slot is active', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'active_snap_slot') return null;
      if (cmd === 'snap_slot_saved') return false;
      throw new Error(`unhandled-${cmd}`);
    });
    const state = await readSnapState();
    expect(state.active).toBeNull();
  });

  it('returns active=<slot> when the runtime reports an active slot', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'active_snap_slot') return 5;
      if (cmd === 'snap_slot_saved') return false;
      throw new Error(`unhandled-${cmd}`);
    });
    const state = await readSnapState();
    expect(state.active).toBe(5);
  });

  it('projects the saved-bitmask into a 16-cell boolean array indexed by slot', async () => {
    // Slots 2, 5, and 11 are saved; rest are empty.
    const SAVED_SLOTS = new Set([2, 5, 11]);
    invokeMock.mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd === 'active_snap_slot') return null;
      if (cmd === 'snap_slot_saved') {
        const slot = (args as { slot: number }).slot;
        return SAVED_SLOTS.has(slot);
      }
      throw new Error(`unhandled-${cmd}`);
    });
    const state = await readSnapState();
    expect(state.saved.length).toBe(16);
    for (let i = 0; i < 16; i += 1) {
      expect(state.saved[i]).toBe(SAVED_SLOTS.has(i));
    }
  });

  it('returns all-false saved array when no slots are populated', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'active_snap_slot') return null;
      if (cmd === 'snap_slot_saved') return false;
      throw new Error(`unhandled-${cmd}`);
    });
    const state = await readSnapState();
    expect(state.saved.every((s) => s === false)).toBe(true);
  });

  it('returns all-true saved array when every slot is populated', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'active_snap_slot') return 0;
      if (cmd === 'snap_slot_saved') return true;
      throw new Error(`unhandled-${cmd}`);
    });
    const state = await readSnapState();
    expect(state.saved.every((s) => s === true)).toBe(true);
    expect(state.saved.length).toBe(16);
  });

  it('propagates an active_snap_slot rejection', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'active_snap_slot') throw new Error('runtime offline');
      if (cmd === 'snap_slot_saved') return false;
      throw new Error(`unhandled-${cmd}`);
    });
    await expect(readSnapState()).rejects.toThrow('runtime offline');
  });
});
