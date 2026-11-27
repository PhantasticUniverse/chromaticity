// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 2.1 Lane A.1 loaded-scene store tests. Validates the
// `load_scene` + `loaded_scene` invocation paths, snapshot state
// propagation, error capture, the `hydrate()` re-bind path, and the
// no-Tauri-runtime swallow path.
//
// `vi.hoisted` invoke-mock pattern per the
// `feedback_vitest_invoke_mock_hoisted.md` user-memory convention —
// hoisted `vi.mock` factories cannot reference module-scope `const`s.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { LoadedScenePayload } from '../ipc/bindings';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const SAMPLE_PAYLOAD: LoadedScenePayload = {
  id: 'demo-kaleido',
  path: '/tmp/demo-kaleido.chroma',
  name: 'Demo Kaleido',
  credit: 'chromaticity demo',
  description: 'Six-fold radial mirror',
  controls: [
    {
      name: 'segments',
      kind: 'Slider',
      default: 6.0,
      min: 2.0,
      max: 24.0,
      ui_group: null,
      description: null,
      label: null,
    },
  ],
  schema_version: 1,
};

describe('loadedSceneStore', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.resetModules();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('load() invokes load_scene with the path and stores the returned payload', async () => {
    invokeMock.mockResolvedValueOnce(SAMPLE_PAYLOAD);
    const { loadedSceneStore } = await import('./loadedScene.svelte');
    loadedSceneStore.reset();

    const result = await loadedSceneStore.load('/tmp/demo-kaleido.chroma');

    expect(invokeMock).toHaveBeenCalledWith('load_scene', {
      path: '/tmp/demo-kaleido.chroma',
    });
    expect(result).toEqual(SAMPLE_PAYLOAD);
    expect(loadedSceneStore.snapshot).toEqual(SAMPLE_PAYLOAD);
    expect(loadedSceneStore.error).toBeNull();
    expect(loadedSceneStore.loading).toBe(false);
  });

  it('load() captures the rejection message on `error` and leaves snapshot unchanged', async () => {
    invokeMock.mockRejectedValueOnce(new Error('scene.json missing'));
    const { loadedSceneStore } = await import('./loadedScene.svelte');
    loadedSceneStore.reset();

    const result = await loadedSceneStore.load('/tmp/missing.chroma');

    expect(result).toBeNull();
    expect(loadedSceneStore.snapshot).toBeNull();
    expect(loadedSceneStore.error).toBe('scene.json missing');
    expect(loadedSceneStore.loading).toBe(false);
  });

  it('load() sets `loading` true during in-flight invocation', async () => {
    let resolveInvoke!: (value: LoadedScenePayload) => void;
    invokeMock.mockImplementation(
      () =>
        new Promise<LoadedScenePayload>((resolve) => {
          resolveInvoke = resolve;
        }),
    );
    const { loadedSceneStore } = await import('./loadedScene.svelte');
    loadedSceneStore.reset();

    const promise = loadedSceneStore.load('/tmp/x.chroma');
    // `loading` is set synchronously inside `load()` before the
    // dynamic-import await, so it is `true` immediately. Wait for
    // the dynamic import to settle so `invokeMock` is actually
    // invoked and `resolveInvoke` is captured.
    expect(loadedSceneStore.loading).toBe(true);
    await new Promise((r) => setTimeout(r, 0));
    expect(loadedSceneStore.loading).toBe(true);
    resolveInvoke(SAMPLE_PAYLOAD);
    await promise;
    expect(loadedSceneStore.loading).toBe(false);
  });

  it('hydrate() invokes loaded_scene and updates snapshot when non-null', async () => {
    invokeMock.mockResolvedValueOnce(SAMPLE_PAYLOAD);
    const { loadedSceneStore } = await import('./loadedScene.svelte');
    loadedSceneStore.reset();

    await loadedSceneStore.hydrate();

    expect(invokeMock).toHaveBeenCalledWith('loaded_scene');
    expect(loadedSceneStore.snapshot).toEqual(SAMPLE_PAYLOAD);
  });

  it('hydrate() leaves snapshot unchanged when the engine returns null', async () => {
    invokeMock.mockResolvedValueOnce(null);
    const { loadedSceneStore } = await import('./loadedScene.svelte');
    loadedSceneStore.reset();

    await loadedSceneStore.hydrate();

    expect(invokeMock).toHaveBeenCalledWith('loaded_scene');
    expect(loadedSceneStore.snapshot).toBeNull();
  });

  it('hydrate() swallows invoke rejection (no Tauri runtime) and leaves snapshot null', async () => {
    invokeMock.mockRejectedValueOnce(new Error('not running in tauri'));
    const { loadedSceneStore } = await import('./loadedScene.svelte');
    loadedSceneStore.reset();

    await expect(loadedSceneStore.hydrate()).resolves.toBeUndefined();
    expect(loadedSceneStore.snapshot).toBeNull();
  });

  it('reset() returns the store to the no-scene baseline', async () => {
    invokeMock.mockResolvedValueOnce(SAMPLE_PAYLOAD);
    const { loadedSceneStore } = await import('./loadedScene.svelte');
    loadedSceneStore.reset();
    await loadedSceneStore.load('/tmp/x.chroma');
    expect(loadedSceneStore.snapshot).not.toBeNull();

    loadedSceneStore.reset();

    expect(loadedSceneStore.snapshot).toBeNull();
    expect(loadedSceneStore.error).toBeNull();
    expect(loadedSceneStore.loading).toBe(false);
  });
});
