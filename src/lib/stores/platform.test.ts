// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Unit coverage for the W22 PR2 platform detector. `currentPlatform` is
// evaluated once at module load, so each test stubs `navigator` BEFORE
// `vi.resetModules()` + dynamic import to force a fresh detection pass.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

describe('detectPlatform', () => {
  beforeEach(() => {
    vi.resetModules();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns "mac" when navigator is undefined (SSR / non-browser)', async () => {
    vi.stubGlobal('navigator', undefined);
    const { currentPlatform } = await import('./platform');
    expect(currentPlatform).toBe('mac');
  });

  it('returns "mac" for the jsdom UA (contains darwin substring)', async () => {
    vi.stubGlobal('navigator', {
      userAgent: 'Mozilla/5.0 (darwin) AppleWebKit/537.36 jsdom/29.0.2',
    });
    const { currentPlatform } = await import('./platform');
    expect(currentPlatform).toBe('mac');
  });

  it('returns "win" for a real Windows Chrome UA', async () => {
    vi.stubGlobal('navigator', {
      userAgent:
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
    });
    const { currentPlatform } = await import('./platform');
    expect(currentPlatform).toBe('win');
  });

  it('returns "mac" for a real macOS Safari UA', async () => {
    vi.stubGlobal('navigator', {
      userAgent:
        'Mozilla/5.0 (Macintosh; Intel Mac OS X 11_0_0) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15',
    });
    const { currentPlatform } = await import('./platform');
    expect(currentPlatform).toBe('mac');
  });

  it('returns "mac" for a UA that mentions darwin in a kernel-info segment', async () => {
    vi.stubGlobal('navigator', {
      userAgent: 'CustomAgent/1.0 (darwin 23.0.0)',
    });
    const { currentPlatform } = await import('./platform');
    expect(currentPlatform).toBe('mac');
  });
});
