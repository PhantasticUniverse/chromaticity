// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane A.4 playlist store tests. Validates the
// `advance_playlist` + `playlist_position` invocation paths,
// reactive state propagation, and the no-Tauri-runtime swallow
// path. Engine-side cursor saturation at `i64::MAX` / `i64::MIN` is
// pinned in `engine/runtime/src/state.rs`; these tests stay on the
// store surface.
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

describe('playlistStore', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.resetModules();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('refresh() invokes playlist_position and stores the returned cursor', async () => {
    invokeMock.mockResolvedValueOnce(5);
    const { playlistStore } = await import('./playlist.svelte');
    playlistStore.reset();

    await playlistStore.refresh();

    expect(invokeMock).toHaveBeenCalledWith('playlist_position');
    expect(playlistStore.position).toBe(5);
  });

  it('advance() invokes advance_playlist with step then refreshes position', async () => {
    const calls: Array<[string, unknown]> = [];
    invokeMock.mockImplementation(async (cmd: string, args?: unknown) => {
      calls.push([cmd, args]);
      if (cmd === 'advance_playlist') return undefined;
      if (cmd === 'playlist_position') return 4;
      throw new Error(`unhandled-${cmd}`);
    });
    const { playlistStore } = await import('./playlist.svelte');
    playlistStore.reset();

    await playlistStore.advance(3);

    expect(calls.length).toBe(2);
    expect(calls[0]).toEqual(['advance_playlist', { step: 3 }]);
    expect(calls[1]?.[0]).toBe('playlist_position');
    expect(playlistStore.position).toBe(4);
  });

  it('next() advances by +1 and reflects the engine read-back', async () => {
    const calls: Array<[string, unknown]> = [];
    invokeMock.mockImplementation(async (cmd: string, args?: unknown) => {
      calls.push([cmd, args]);
      if (cmd === 'advance_playlist') return undefined;
      if (cmd === 'playlist_position') return 1;
      throw new Error(`unhandled-${cmd}`);
    });
    const { playlistStore } = await import('./playlist.svelte');
    playlistStore.reset();

    await playlistStore.next();

    expect(calls[0]).toEqual(['advance_playlist', { step: 1 }]);
    expect(playlistStore.position).toBe(1);
  });

  it('previous() advances by -1 and reflects the engine read-back', async () => {
    const calls: Array<[string, unknown]> = [];
    invokeMock.mockImplementation(async (cmd: string, args?: unknown) => {
      calls.push([cmd, args]);
      if (cmd === 'advance_playlist') return undefined;
      if (cmd === 'playlist_position') return -1;
      throw new Error(`unhandled-${cmd}`);
    });
    const { playlistStore } = await import('./playlist.svelte');
    playlistStore.reset();

    await playlistStore.previous();

    expect(calls[0]).toEqual(['advance_playlist', { step: -1 }]);
    expect(playlistStore.position).toBe(-1);
  });

  it('refresh() swallows invoke rejection (no Tauri runtime) and leaves position unchanged', async () => {
    invokeMock.mockRejectedValueOnce(new Error('not running in tauri'));
    const { playlistStore } = await import('./playlist.svelte');
    playlistStore.reset();

    await expect(playlistStore.refresh()).resolves.toBeUndefined();
    expect(playlistStore.position).toBe(0);
  });

  it('advance() swallows invoke rejection and leaves position unchanged', async () => {
    invokeMock.mockRejectedValueOnce(new Error('not running in tauri'));
    const { playlistStore } = await import('./playlist.svelte');
    playlistStore.reset();

    await expect(playlistStore.advance(2)).resolves.toBeUndefined();
    expect(playlistStore.position).toBe(0);
  });

  it('reset() returns position to the baseline zero', async () => {
    invokeMock.mockResolvedValueOnce(8);
    const { playlistStore } = await import('./playlist.svelte');
    playlistStore.reset();
    await playlistStore.refresh();
    expect(playlistStore.position).toBe(8);

    playlistStore.reset();

    expect(playlistStore.position).toBe(0);
  });
});
