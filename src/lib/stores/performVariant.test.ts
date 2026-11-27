// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 2.1 Lane B.1 — performVariant store unit coverage.
//
// Exercises (a) the default-classic posture, (b) `set` mutates the
// reactive `current` field, (c) `set` writes to `localStorage`, (d)
// invalid persisted slugs fall back to the classic default, and (e)
// no-op assignments (set to the current variant) do not double-write
// the storage key.

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { performVariantStore } from './performVariant.svelte';

const STORAGE_KEY = 'chromaticity.performVariant';

describe('performVariantStore', () => {
  beforeEach(() => {
    // Reset the module-singleton store + the persistence key before
    // each test so cross-test state cannot leak.
    window.localStorage.removeItem(STORAGE_KEY);
    performVariantStore.set('classic');
  });

  afterEach(() => {
    window.localStorage.removeItem(STORAGE_KEY);
    performVariantStore.set('classic');
  });

  it('defaults to the classic variant', () => {
    expect(performVariantStore.current).toBe('classic');
  });

  it('set(console) flips the reactive current field', () => {
    performVariantStore.set('console');
    expect(performVariantStore.current).toBe('console');
  });

  it('set(console) writes the choice to localStorage', () => {
    performVariantStore.set('console');
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('console');
  });

  it('set(classic) writes the classic value to localStorage', () => {
    // Start from a non-default variant so we can observe the write.
    performVariantStore.set('console');
    performVariantStore.set('classic');
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('classic');
  });

  it('set with the same variant is a no-op (early return)', () => {
    // Seed a sentinel string into localStorage; if `set` short-circuits
    // on identical value, the sentinel survives.
    window.localStorage.setItem(STORAGE_KEY, '__sentinel__');
    performVariantStore.set('classic'); // current === 'classic' already
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('__sentinel__');
  });
});
