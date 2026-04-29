// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Phase-1 D4 playlist round-trip helper tests. The module exposes
// `dispatchPlaylistAdvanceAndRead` (advance + read-back) and
// `readPlaylistPosition` (read-only). Engine-side cursor saturation at
// `i64::MAX` / `i64::MIN` is the runtime crate's contract; these tests
// stay on the wrapper surface — strict ordering + payload shape.
//
// `vi.hoisted` invoke mock per the project convention.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import { dispatchPlaylistAdvanceAndRead, readPlaylistPosition } from './playlist';

describe('playlist round-trip helpers', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('dispatchPlaylistAdvanceAndRead', () => {
    it('issues advance_playlist before playlist_position', async () => {
      const calls: string[] = [];
      invokeMock.mockImplementation(async (cmd: string) => {
        calls.push(cmd);
        if (cmd === 'advance_playlist') return undefined;
        if (cmd === 'playlist_position') return 1;
        throw new Error(`unhandled-${cmd}`);
      });
      await dispatchPlaylistAdvanceAndRead(1);
      expect(calls).toEqual(['advance_playlist', 'playlist_position']);
    });

    it('forwards the step payload to advance_playlist', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'advance_playlist') return undefined;
        if (cmd === 'playlist_position') return 5;
        throw new Error(`unhandled-${cmd}`);
      });
      await dispatchPlaylistAdvanceAndRead(3);
      expect(invokeMock).toHaveBeenNthCalledWith(1, 'advance_playlist', { step: 3 });
    });

    it('returns the read-back cursor position verbatim', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'advance_playlist') return undefined;
        if (cmd === 'playlist_position') return 12;
        throw new Error(`unhandled-${cmd}`);
      });
      const pos = await dispatchPlaylistAdvanceAndRead(2);
      expect(pos).toBe(12);
    });

    it('forwards negative step values for backward movement', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'advance_playlist') return undefined;
        if (cmd === 'playlist_position') return -1;
        throw new Error(`unhandled-${cmd}`);
      });
      const pos = await dispatchPlaylistAdvanceAndRead(-1);
      expect(invokeMock).toHaveBeenNthCalledWith(1, 'advance_playlist', { step: -1 });
      expect(pos).toBe(-1);
    });

    it('forwards a zero step (no-op advance) and reads position back', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'advance_playlist') return undefined;
        if (cmd === 'playlist_position') return 0;
        throw new Error(`unhandled-${cmd}`);
      });
      const pos = await dispatchPlaylistAdvanceAndRead(0);
      expect(invokeMock).toHaveBeenNthCalledWith(1, 'advance_playlist', { step: 0 });
      expect(pos).toBe(0);
    });

    it('propagates an advance_playlist rejection without reading back', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'advance_playlist') throw new Error('engine busy');
        if (cmd === 'playlist_position') return 0;
        throw new Error(`unhandled-${cmd}`);
      });
      await expect(dispatchPlaylistAdvanceAndRead(1)).rejects.toThrow('engine busy');
      expect(invokeMock).toHaveBeenCalledTimes(1);
    });
  });

  describe('readPlaylistPosition', () => {
    it('issues exactly one playlist_position call with no payload', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'playlist_position') return 7;
        throw new Error(`unhandled-${cmd}`);
      });
      const pos = await readPlaylistPosition();
      expect(invokeMock).toHaveBeenCalledTimes(1);
      // playlist_position takes no args (a single positional arg in
      // Tauri lands as `undefined`).
      expect(invokeMock).toHaveBeenCalledWith('playlist_position');
      expect(pos).toBe(7);
    });

    it('returns 0 for the pristine pre-advance cursor', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'playlist_position') return 0;
        throw new Error(`unhandled-${cmd}`);
      });
      const pos = await readPlaylistPosition();
      expect(pos).toBe(0);
    });

    it('propagates a runtime read failure', async () => {
      invokeMock.mockImplementation(async () => {
        throw new Error('cursor unreadable');
      });
      await expect(readPlaylistPosition()).rejects.toThrow('cursor unreadable');
    });
  });
});
