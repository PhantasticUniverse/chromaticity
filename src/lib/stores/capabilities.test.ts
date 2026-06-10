// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W33b capabilities-store tests — exercises the seed snapshot derived
// from `LICENSE_FIXTURE`, the `has(capabilityId)` lookup contract,
// the `licensed` / `platform` convenience accessors, and the
// `setSnapshot` Phase-3+/4a swap surface.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { capabilitiesStore } from './capabilities.svelte';
import { LICENSE_FIXTURE } from '../data/license-fixtures';

describe('capabilities store', () => {
  it('seeds licensed=true from the activated LICENSE_FIXTURE', () => {
    expect(capabilitiesStore.licensed).toBe(true);
  });

  it('seeds the capability map verbatim from LICENSE_FIXTURE.capabilities', () => {
    for (const cap of LICENSE_FIXTURE.capabilities) {
      expect(capabilitiesStore.has(cap.id)).toBe(cap.enabled);
    }
  });

  it('returns false for unknown capability ids (fail-closed semantics)', () => {
    expect(capabilitiesStore.has('made.up.capability.id')).toBe(false);
  });

  it('reports a non-empty platform string', () => {
    const p = capabilitiesStore.platform;
    expect(['macos', 'windows', 'linux']).toContain(p);
  });

  it('setSnapshot replaces the live snapshot atomically', () => {
    const before = capabilitiesStore.snapshot;
    capabilitiesStore.setSnapshot({
      licensed: false,
      capabilities: {},
      platform: 'linux',
    });
    expect(capabilitiesStore.licensed).toBe(false);
    expect(capabilitiesStore.has('render.watermark.off')).toBe(false);
    expect(capabilitiesStore.platform).toBe('linux');
    // Restore so subsequent tests in the same suite (and any other
    // module that imports the singleton) keep their seed assumptions.
    capabilitiesStore.setSnapshot(before);
    expect(capabilitiesStore.licensed).toBe(true);
  });

  it('the activated seed grants render.watermark.off', () => {
    expect(capabilitiesStore.has('render.watermark.off')).toBe(true);
  });
});

// The seed platform is detected once at module load, so each case stubs
// `navigator` BEFORE `vi.resetModules()` + dynamic import to force a
// fresh detection pass (mirrors `platform.test.ts` for the W22 helper).
describe('capabilities store platform detection', () => {
  beforeEach(() => {
    vi.resetModules();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('reports macos for the jsdom UA (darwin substring must not match windows)', async () => {
    vi.stubGlobal('navigator', {
      userAgent: 'Mozilla/5.0 (darwin) AppleWebKit/537.36 jsdom/29.0.2',
    });
    const { capabilitiesStore: store } = await import('./capabilities.svelte');
    expect(store.platform).toBe('macos');
  });

  it('reports windows for a real Windows Chrome UA', async () => {
    vi.stubGlobal('navigator', {
      userAgent:
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
    });
    const { capabilitiesStore: store } = await import('./capabilities.svelte');
    expect(store.platform).toBe('windows');
  });

  it('reports linux for an X11 Linux UA', async () => {
    vi.stubGlobal('navigator', {
      userAgent:
        'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
    });
    const { capabilitiesStore: store } = await import('./capabilities.svelte');
    expect(store.platform).toBe('linux');
  });

  it('reports macos for a real macOS Safari UA', async () => {
    vi.stubGlobal('navigator', {
      userAgent:
        'Mozilla/5.0 (Macintosh; Intel Mac OS X 11_0_0) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15',
    });
    const { capabilitiesStore: store } = await import('./capabilities.svelte');
    expect(store.platform).toBe('macos');
  });

  it('reports macos when navigator is undefined (SSR / non-browser fallback)', async () => {
    vi.stubGlobal('navigator', undefined);
    const { capabilitiesStore: store } = await import('./capabilities.svelte');
    expect(store.platform).toBe('macos');
  });
});
