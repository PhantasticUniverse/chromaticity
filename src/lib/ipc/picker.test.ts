// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// pickFolder resolution-order tests. The load-bearing case is the
// real-shell failure path: when `@tauri-apps/plugin-dialog` `open()`
// throws inside a Tauri runtime (`window.__TAURI_INTERNALS__`
// present), pickFolder must RETHROW instead of falling through to the
// legacy `pick_folder` stub — the stub's deterministic placeholder
// (`/Users/operator/Movies/Chromaticity/Media`) would masquerade as a
// successful operator pick and get persisted into settings.storage.
// Outside a Tauri runtime (vite-only dev / jsdom) the legacy fallback
// chain is preserved: stub first, then `null` when no invoke exists.
//
// `vi.hoisted` invoke-mock pattern per
// `feedback_vitest_invoke_mock_hoisted.md`. The `__TAURI_INTERNALS__`
// marker is injected/removed per test the same way
// `views/Library.test.ts` exercises its browser-mode gate.

import { afterEach, describe, expect, it, vi } from 'vitest';

const { invokeMock, dialogOpenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  dialogOpenMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: dialogOpenMock,
}));

import { pickFolder } from './picker';

const w = window as unknown as { __TAURI_INTERNALS__?: unknown };

describe('pickFolder', () => {
  afterEach(() => {
    invokeMock.mockReset();
    dialogOpenMock.mockReset();
    delete w.__TAURI_INTERNALS__;
  });

  it('resolves the picked path from the dialog plugin', async () => {
    dialogOpenMock.mockResolvedValue('/tmp/picked');
    await expect(pickFolder('Pick a folder')).resolves.toBe('/tmp/picked');
    expect(dialogOpenMock).toHaveBeenCalledWith({
      directory: true,
      multiple: false,
      title: 'Pick a folder',
    });
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('resolves null on user dismiss without consulting the legacy stub', async () => {
    dialogOpenMock.mockResolvedValue(null);
    await expect(pickFolder('Pick a folder')).resolves.toBeNull();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('rethrows a dialog failure in a real shell instead of fabricating via the stub', async () => {
    w.__TAURI_INTERNALS__ = {};
    dialogOpenMock.mockRejectedValue(new Error('dialog capability missing'));
    invokeMock.mockResolvedValue('/Users/operator/Movies/Chromaticity/Media');
    await expect(pickFolder('Pick a folder')).rejects.toThrow('dialog capability missing');
    // The legacy stub must never be consulted in a real shell — its
    // placeholder path is fabricated success data on this error path.
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('falls back to the legacy pick_folder stub outside a Tauri runtime', async () => {
    dialogOpenMock.mockRejectedValue(new Error('plugin unavailable'));
    invokeMock.mockResolvedValue('/tmp/legacy');
    await expect(pickFolder('Pick a folder')).resolves.toBe('/tmp/legacy');
    expect(invokeMock).toHaveBeenCalledWith('pick_folder', { title: 'Pick a folder' });
  });

  it('resolves null when both the plugin and the legacy invoke are unavailable', async () => {
    dialogOpenMock.mockRejectedValue(new Error('plugin unavailable'));
    invokeMock.mockRejectedValue(new Error('no tauri runtime'));
    await expect(pickFolder('Pick a folder')).resolves.toBeNull();
  });

  it('treats a defensive empty-array result as no pick (no stub fall-through)', async () => {
    dialogOpenMock.mockResolvedValue([]);
    await expect(pickFolder('Pick a folder')).resolves.toBeNull();
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
