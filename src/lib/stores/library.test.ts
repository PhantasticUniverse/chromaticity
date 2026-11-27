// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W9 / W11 PR2 scene-library helper tests. The module wraps three
// IPC reads (`default_scene_root`, `list_scene_packages`,
// `list_bundled_scene_packages`) plus a per-root in-memory cache map
// keyed on the user-root path or the `__bundled__` sentinel.
//
// Tests cover:
//   - getDefaultRoot pass-through
//   - fetchSceneCards happy-path + cached-hit short-circuit
//   - fetchBundledSceneCards happy-path + cached-hit short-circuit
//   - per-root cache isolation across distinct paths
//   - clearCache invalidates bundled AND user entries (both refetch)
//   - rejected fetches do not poison the cache (next call retries)
//
// `clearCache()` runs in `beforeEach` so module-level cache state
// from prior tests does not leak into the next assertion.
//
// `vi.hoisted` invoke mock per the project convention.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import { clearCache, fetchBundledSceneCards, fetchSceneCards, getDefaultRoot } from './library';

const USER_CARDS_A = [
  {
    id: 'pkg.user.a',
    title: 'A',
    credit: 'tester',
    description: '',
    tags: [],
    categories: [],
    path: '/root-a/a.chroma',
    format_tag: 'native-chroma-scene',
    source: 'User' as const,
  },
];

const USER_CARDS_B = [
  {
    id: 'pkg.user.b',
    title: 'B',
    credit: 'tester',
    description: '',
    tags: [],
    categories: [],
    path: '/root-b/b.chroma',
    format_tag: 'native-chroma-scene',
    source: 'User' as const,
  },
];

const BUNDLED_CARDS = [
  {
    id: 'pkg.builtin.welcome',
    title: 'Welcome',
    credit: 'Chromaticity',
    description: '',
    tags: [],
    categories: ['Built-In'],
    path: '/bundle/welcome.chroma',
    format_tag: 'native-chroma-scene',
    source: 'Bundled' as const,
  },
];

describe('library helpers', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    clearCache();
  });

  afterEach(() => {
    clearCache();
    vi.restoreAllMocks();
  });

  describe('getDefaultRoot', () => {
    it('returns the runtime-provided default scene root path', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'default_scene_root') return '/Users/op/Library/Scenes';
        throw new Error(`unhandled-${cmd}`);
      });
      const root = await getDefaultRoot();
      expect(root).toBe('/Users/op/Library/Scenes');
      expect(invokeMock).toHaveBeenCalledWith('default_scene_root');
    });
  });

  describe('fetchSceneCards (per-user-root cache)', () => {
    it('issues list_scene_packages with { root } on first call', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'list_scene_packages') return USER_CARDS_A;
        throw new Error(`unhandled-${cmd}`);
      });
      const cards = await fetchSceneCards('/root-a');
      expect(cards).toEqual(USER_CARDS_A);
      expect(invokeMock).toHaveBeenCalledTimes(1);
      expect(invokeMock).toHaveBeenCalledWith('list_scene_packages', { root: '/root-a' });
    });

    it('short-circuits the IPC on a repeat call for the same root (cache hit)', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'list_scene_packages') return USER_CARDS_A;
        throw new Error(`unhandled-${cmd}`);
      });
      const first = await fetchSceneCards('/root-a');
      const second = await fetchSceneCards('/root-a');
      expect(first).toBe(second); // same array reference (cached)
      expect(invokeMock).toHaveBeenCalledTimes(1);
    });

    it('caches per-root: distinct roots refetch independently', async () => {
      invokeMock.mockImplementation(async (cmd: string, args?: unknown) => {
        if (cmd === 'list_scene_packages') {
          const root = (args as { root: string }).root;
          if (root === '/root-a') return USER_CARDS_A;
          if (root === '/root-b') return USER_CARDS_B;
        }
        throw new Error(`unhandled-${cmd}`);
      });
      const a1 = await fetchSceneCards('/root-a');
      const b1 = await fetchSceneCards('/root-b');
      expect(a1).toEqual(USER_CARDS_A);
      expect(b1).toEqual(USER_CARDS_B);
      expect(invokeMock).toHaveBeenCalledTimes(2);
      // Repeat both — neither hits the IPC.
      await fetchSceneCards('/root-a');
      await fetchSceneCards('/root-b');
      expect(invokeMock).toHaveBeenCalledTimes(2);
    });
  });

  describe('fetchBundledSceneCards (sentinel-keyed cache)', () => {
    it('issues list_bundled_scene_packages on first call', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'list_bundled_scene_packages') return BUNDLED_CARDS;
        throw new Error(`unhandled-${cmd}`);
      });
      const cards = await fetchBundledSceneCards();
      expect(cards).toEqual(BUNDLED_CARDS);
      expect(invokeMock).toHaveBeenCalledWith('list_bundled_scene_packages');
    });

    it('short-circuits the IPC on a repeat call (sentinel-key cache hit)', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'list_bundled_scene_packages') return BUNDLED_CARDS;
        throw new Error(`unhandled-${cmd}`);
      });
      const first = await fetchBundledSceneCards();
      const second = await fetchBundledSceneCards();
      expect(first).toBe(second);
      expect(invokeMock).toHaveBeenCalledTimes(1);
    });

    it('keeps the bundled cache distinct from any user-root entry', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'list_bundled_scene_packages') return BUNDLED_CARDS;
        if (cmd === 'list_scene_packages') return USER_CARDS_A;
        throw new Error(`unhandled-${cmd}`);
      });
      await fetchBundledSceneCards();
      await fetchSceneCards('/root-a');
      // Neither call should service the other's cache key.
      expect(invokeMock).toHaveBeenCalledTimes(2);
      // Repeats: both hit the cache.
      await fetchBundledSceneCards();
      await fetchSceneCards('/root-a');
      expect(invokeMock).toHaveBeenCalledTimes(2);
    });
  });

  describe('clearCache', () => {
    it('forces both bundled and user-root entries to refetch on next call', async () => {
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'list_bundled_scene_packages') return BUNDLED_CARDS;
        if (cmd === 'list_scene_packages') return USER_CARDS_A;
        throw new Error(`unhandled-${cmd}`);
      });
      await fetchBundledSceneCards();
      await fetchSceneCards('/root-a');
      expect(invokeMock).toHaveBeenCalledTimes(2);
      clearCache();
      await fetchBundledSceneCards();
      await fetchSceneCards('/root-a');
      expect(invokeMock).toHaveBeenCalledTimes(4);
    });

    it('is safe to call against an empty cache (no-op)', () => {
      expect(() => clearCache()).not.toThrow();
    });
  });

  describe('rejection handling', () => {
    it('does NOT poison the user-root cache on a rejected fetch (retry replays the IPC)', async () => {
      let attempt = 0;
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === 'list_scene_packages') {
          attempt += 1;
          if (attempt === 1) throw new Error('fs offline');
          return USER_CARDS_A;
        }
        throw new Error(`unhandled-${cmd}`);
      });
      await expect(fetchSceneCards('/root-a')).rejects.toThrow('fs offline');
      // Second call should re-issue the IPC since the cache was never
      // populated by the failed fetch.
      const cards = await fetchSceneCards('/root-a');
      expect(cards).toEqual(USER_CARDS_A);
      expect(attempt).toBe(2);
    });
  });
});
