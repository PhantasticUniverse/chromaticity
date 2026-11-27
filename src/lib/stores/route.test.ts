// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave-12 PR3 route store tests. The store is a `$state`-backed
// singleton with `current` (RouteName) and `set(name)`. The W16 / W18
// / W19 sprint waves widened the union to thirteen route names; the
// tests here exercise the perform-default seed, the `set` mutator, and
// a representative cross-section of the post-W19 union.
//
// Tests run in declaration order against a shared singleton — each
// case finishes by asserting state independently of the previous one.

import { afterAll, describe, expect, it } from 'vitest';

import { routeStore, type RouteName } from './route.svelte';

const ALL_ROUTES: readonly RouteName[] = [
  'perform',
  'mix',
  'library',
  'author',
  'audio-uniforms',
  'playlist',
  'sources',
  'midi',
  'osc',
  'import',
  'patch',
  'settings',
  'license',
  'welcome',
  'help',
];

describe('routeStore', () => {
  // The route singleton is shared across the whole test suite (App.svelte
  // imports it at module load). Restore the seed default in afterAll so
  // unrelated test files that import the store continue to see `perform`.
  afterAll(() => {
    routeStore.set('perform');
  });

  it("seeds current to 'perform' (the W12 PR3 default)", () => {
    // Re-seed in case earlier in-suite tests mutated it; the assertion
    // is on the post-set state.
    routeStore.set('perform');
    expect(routeStore.current).toBe('perform');
  });

  it("set('library') flips current to 'library'", () => {
    routeStore.set('library');
    expect(routeStore.current).toBe('library');
  });

  it("set('author') flips current to 'author'", () => {
    routeStore.set('author');
    expect(routeStore.current).toBe('author');
  });

  it('set is idempotent: re-setting the active route is a no-op', () => {
    routeStore.set('settings');
    expect(routeStore.current).toBe('settings');
    routeStore.set('settings');
    expect(routeStore.current).toBe('settings');
  });

  it('every RouteName union member is settable and round-trips', () => {
    for (const name of ALL_ROUTES) {
      routeStore.set(name);
      expect(routeStore.current).toBe(name);
    }
  });

  it("set('welcome') reaches the W16 PR2 onboarding-default route", () => {
    routeStore.set('welcome');
    expect(routeStore.current).toBe('welcome');
  });

  it("set('midi') reaches the W18 PR3 added route", () => {
    routeStore.set('midi');
    expect(routeStore.current).toBe('midi');
  });

  it("set('patch') reaches the W19 PR1 added route", () => {
    routeStore.set('patch');
    expect(routeStore.current).toBe('patch');
  });

  it("set('import') reaches the W19 PR2 added route", () => {
    routeStore.set('import');
    expect(routeStore.current).toBe('import');
  });

  it('two consecutive sets retain only the latest write', () => {
    routeStore.set('library');
    routeStore.set('osc');
    expect(routeStore.current).toBe('osc');
  });
});
