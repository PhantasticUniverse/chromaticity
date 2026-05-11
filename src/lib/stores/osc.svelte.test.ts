// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane D.3 OSC route store tests. Validates the
// `osc_routes_snapshot` / `bind_osc_route` / `unbind_osc_route` Tauri
// command invocations, routes state propagation, loading flag
// lifecycle, error capture, and bind/unbind-followed-by-refresh
// composition.
//
// `vi.hoisted` invoke-mock pattern per the
// `feedback_vitest_invoke_mock_hoisted.md` user-memory convention.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { OscRoutePayload } from '../ipc/bindings/OscRoutePayload';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const ROUTE_A: OscRoutePayload = {
  canonical: '/controls/{bank}/{name}',
  compat_alias: '/controls/{addr}',
  type_tag: 'f',
  range: '0.0 – 1.0',
  direction: 'Both',
  family: 'Control',
  bind_target: null,
};

const ROUTE_B: OscRoutePayload = {
  canonical: '/scenes/{sceneName}',
  compat_alias: '/scene/{sceneName}',
  type_tag: '—',
  range: 'announce / trigger',
  direction: 'Both',
  family: 'Scene',
  bind_target: null,
};

describe('oscStore', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.resetModules();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('refresh() invokes osc_routes_snapshot and stores the returned vec', async () => {
    invokeMock.mockResolvedValue([ROUTE_A, ROUTE_B]);
    const { oscStore } = await import('./osc.svelte');
    oscStore.reset();
    await oscStore.refresh();
    expect(invokeMock).toHaveBeenCalledWith('osc_routes_snapshot');
    expect(oscStore.routes).toEqual([ROUTE_A, ROUTE_B]);
    expect(oscStore.error).toBeNull();
    expect(oscStore.isLoading).toBe(false);
  });

  it('refresh() captures errors and clears the route list on failure', async () => {
    invokeMock.mockRejectedValue(new Error('not running in tauri'));
    const { oscStore } = await import('./osc.svelte');
    oscStore.reset();
    oscStore.routes = [ROUTE_A];
    await oscStore.refresh();
    expect(oscStore.routes).toEqual([]);
    expect(oscStore.error).toBe('not running in tauri');
    expect(oscStore.isLoading).toBe(false);
  });

  it('refresh() coerces non-Error rejections to a string', async () => {
    invokeMock.mockRejectedValue('plain-string-error');
    const { oscStore } = await import('./osc.svelte');
    oscStore.reset();
    await oscStore.refresh();
    expect(oscStore.error).toBe('plain-string-error');
  });

  it('isLoading toggles around the in-flight refresh promise', async () => {
    let resolveInvoke: (value: OscRoutePayload[]) => void = () => {};
    invokeMock.mockImplementation(
      () =>
        new Promise<OscRoutePayload[]>((resolve) => {
          resolveInvoke = resolve;
        }),
    );
    const { oscStore } = await import('./osc.svelte');
    oscStore.reset();
    const refreshPromise = oscStore.refresh();
    expect(oscStore.isLoading).toBe(true);
    resolveInvoke([ROUTE_A]);
    await refreshPromise;
    expect(oscStore.isLoading).toBe(false);
    expect(oscStore.routes).toEqual([ROUTE_A]);
  });

  it('bind() invokes bind_osc_route and refreshes the snapshot', async () => {
    // Bind succeeds, then the implicit refresh returns the updated
    // route with bind_target set.
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'bind_osc_route') return Promise.resolve();
      if (cmd === 'osc_routes_snapshot')
        return Promise.resolve([{ ...ROUTE_A, bind_target: 'fader-1' }]);
      return Promise.reject(new Error(`unexpected command ${cmd}`));
    });
    const { oscStore } = await import('./osc.svelte');
    oscStore.reset();
    await oscStore.bind('/controls/{bank}/{name}', 'fader-1');
    expect(invokeMock).toHaveBeenCalledWith('bind_osc_route', {
      route: '/controls/{bank}/{name}',
      target: 'fader-1',
    });
    expect(invokeMock).toHaveBeenCalledWith('osc_routes_snapshot');
    expect(oscStore.routes[0]?.bind_target).toBe('fader-1');
    expect(oscStore.error).toBeNull();
  });

  it('bind() captures errors without mutating the route list', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'bind_osc_route')
        return Promise.reject(new Error('bind target must not be empty'));
      return Promise.reject(new Error(`unexpected command ${cmd}`));
    });
    const { oscStore } = await import('./osc.svelte');
    oscStore.reset();
    oscStore.routes = [ROUTE_A];
    await oscStore.bind('/controls/{bank}/{name}', '');
    expect(oscStore.error).toBe('bind target must not be empty');
    // The pre-bind routes vec is untouched (no implicit refresh on
    // failure).
    expect(oscStore.routes).toEqual([ROUTE_A]);
  });

  it('unbind() invokes unbind_osc_route and refreshes the snapshot', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'unbind_osc_route') return Promise.resolve();
      if (cmd === 'osc_routes_snapshot') return Promise.resolve([ROUTE_A]);
      return Promise.reject(new Error(`unexpected command ${cmd}`));
    });
    const { oscStore } = await import('./osc.svelte');
    oscStore.reset();
    await oscStore.unbind('/controls/{bank}/{name}');
    expect(invokeMock).toHaveBeenCalledWith('unbind_osc_route', {
      route: '/controls/{bank}/{name}',
    });
    expect(invokeMock).toHaveBeenCalledWith('osc_routes_snapshot');
    expect(oscStore.routes[0]?.bind_target).toBeNull();
  });

  it('unbind() captures errors without mutating the route list', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'unbind_osc_route')
        return Promise.reject(new Error('osc route not registered: /unknown'));
      return Promise.reject(new Error(`unexpected command ${cmd}`));
    });
    const { oscStore } = await import('./osc.svelte');
    oscStore.reset();
    oscStore.routes = [ROUTE_A];
    await oscStore.unbind('/unknown');
    expect(oscStore.error).toBe('osc route not registered: /unknown');
    expect(oscStore.routes).toEqual([ROUTE_A]);
  });

  it('reset() clears routes, loading, and error state', async () => {
    invokeMock.mockResolvedValue([ROUTE_A]);
    const { oscStore } = await import('./osc.svelte');
    await oscStore.refresh();
    oscStore.error = 'old error';
    oscStore.reset();
    expect(oscStore.routes).toEqual([]);
    expect(oscStore.error).toBeNull();
    expect(oscStore.isLoading).toBe(false);
  });
});
