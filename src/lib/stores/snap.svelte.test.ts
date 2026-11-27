// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 2.1 Lane A.3 snap rune-store tests. Validates the
// `recall_snap` / `save_snap` / `clear_snap` / `active_snap_slot` /
// `snap_slot_saved` Tauri invocation paths, the SnapBar-domain
// (1-indexed → 0-indexed engine) slot mapping, the saved[] mask
// updates, and the no-Tauri-runtime swallow path.
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

import { SNAP_SLOT_COUNT, snapStore } from './snap.svelte';

describe('snapStore', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    snapStore.reset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('refresh() invokes active_snap_slot + 8x snap_slot_saved and populates state', async () => {
    // Engine reports slot 2 active (0-indexed); slots 0, 2, 5 saved.
    const savedMask = [true, false, true, false, false, true, false, false];
    invokeMock.mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd === 'active_snap_slot') return 2;
      if (cmd === 'snap_slot_saved') {
        const slot = (args as { slot: number }).slot;
        return savedMask[slot] ?? false;
      }
      throw new Error(`unhandled-${cmd}`);
    });

    await snapStore.refresh();

    // active_snap_slot called exactly once + 8 snap_slot_saved (one per slot).
    expect(invokeMock).toHaveBeenCalledTimes(1 + SNAP_SLOT_COUNT);
    const savedCalls = invokeMock.mock.calls.filter((c) => c[0] === 'snap_slot_saved');
    const slots = savedCalls.map((c) => (c[1] as { slot: number }).slot).sort((a, b) => a - b);
    expect(slots).toEqual([0, 1, 2, 3, 4, 5, 6, 7]);
    // Engine slot 2 → SnapBar slot 3 (1-indexed).
    expect(snapStore.activeSlot).toBe(3);
    expect(snapStore.saved).toEqual(savedMask);
    expect(snapStore.error).toBeNull();
  });

  it('refresh() leaves activeSlot null when the engine returns null', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'active_snap_slot') return null;
      if (cmd === 'snap_slot_saved') return false;
      throw new Error(`unhandled-${cmd}`);
    });

    await snapStore.refresh();

    expect(snapStore.activeSlot).toBeNull();
    expect(snapStore.saved.every((s) => s === false)).toBe(true);
  });

  it('recall(slot) invokes recall_snap with the engine slot id and updates activeSlot', async () => {
    invokeMock.mockResolvedValue(undefined);

    await snapStore.recall(5);

    expect(invokeMock).toHaveBeenCalledTimes(1);
    // SnapBar slot 5 → engine slot 4.
    expect(invokeMock).toHaveBeenCalledWith('recall_snap', { slot: 4 });
    expect(snapStore.activeSlot).toBe(5);
    expect(snapStore.error).toBeNull();
  });

  it('save(slot) invokes save_snap with the engine slot id and marks saved[slot-1] true', async () => {
    invokeMock.mockResolvedValue(undefined);

    await snapStore.save(3);

    expect(invokeMock).toHaveBeenCalledWith('save_snap', { slot: 2 });
    expect(snapStore.saved[2]).toBe(true);
    // Other slots stay false.
    expect(snapStore.saved.filter((s) => s).length).toBe(1);
    expect(snapStore.error).toBeNull();
  });

  it('clear(slot) invokes clear_snap, clears saved[slot-1], and clears activeSlot when matched', async () => {
    invokeMock.mockResolvedValue(undefined);
    // Seed state: slot 4 is saved and active.
    snapStore.saved = [false, false, false, true, false, false, false, false];
    snapStore.activeSlot = 4;

    await snapStore.clear(4);

    expect(invokeMock).toHaveBeenCalledWith('clear_snap', { slot: 3 });
    expect(snapStore.saved[3]).toBe(false);
    expect(snapStore.activeSlot).toBeNull();
  });

  it('clear(slot) leaves activeSlot unchanged when clearing a different slot', async () => {
    invokeMock.mockResolvedValue(undefined);
    snapStore.saved = [true, false, false, true, false, false, false, false];
    snapStore.activeSlot = 1;

    await snapStore.clear(4);

    expect(snapStore.saved[3]).toBe(false);
    expect(snapStore.saved[0]).toBe(true);
    expect(snapStore.activeSlot).toBe(1);
  });

  it('captures error message on recall rejection without mutating activeSlot', async () => {
    invokeMock.mockRejectedValueOnce(new Error('runtime offline'));
    snapStore.activeSlot = 2;

    await snapStore.recall(7);

    expect(snapStore.error).toBe('runtime offline');
    expect(snapStore.activeSlot).toBe(2);
  });

  it('refresh() swallows invoke rejection (no Tauri runtime) and leaves state at baseline', async () => {
    invokeMock.mockRejectedValue(new Error('not running in tauri'));

    await expect(snapStore.refresh()).resolves.toBeUndefined();

    expect(snapStore.activeSlot).toBeNull();
    expect(snapStore.saved.every((s) => s === false)).toBe(true);
  });

  it('reset() returns the store to the no-snap baseline', async () => {
    invokeMock.mockResolvedValue(undefined);
    await snapStore.save(2);
    await snapStore.recall(2);
    expect(snapStore.activeSlot).not.toBeNull();

    snapStore.reset();

    expect(snapStore.activeSlot).toBeNull();
    expect(snapStore.saved.every((s) => s === false)).toBe(true);
    expect(snapStore.error).toBeNull();
  });
});
