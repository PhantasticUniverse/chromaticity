// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 2.5 Lane LIB.C — libraryRecent store unit coverage.
//
// Exercises (a) the empty-default posture, (b) `push` prepends, (c)
// `push` of an existing id moves it to the front (no duplicates),
// (d) the MAX_ENTRIES (20) cap, (e) localStorage persistence on push,
// (f) `hydrate` restores from localStorage, (g) corrupt JSON resets
// to empty, (h) `isRecent` predicate, (i) `reset` clears in-memory
// state without touching localStorage.

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { libraryRecentStore } from './libraryRecent.svelte';

const STORAGE_KEY = 'library-recent';

describe('libraryRecentStore', () => {
  beforeEach(() => {
    window.localStorage.removeItem(STORAGE_KEY);
    libraryRecentStore.reset();
  });

  afterEach(() => {
    window.localStorage.removeItem(STORAGE_KEY);
    libraryRecentStore.reset();
  });

  it('defaults to an empty ids list', () => {
    expect(libraryRecentStore.ids).toEqual([]);
  });

  it('push(id) prepends to the front of the list', () => {
    libraryRecentStore.push('scene-a');
    libraryRecentStore.push('scene-b');
    expect(libraryRecentStore.ids).toEqual(['scene-b', 'scene-a']);
  });

  it('push(id) of an existing id moves it to the front without duplicating', () => {
    libraryRecentStore.push('scene-a');
    libraryRecentStore.push('scene-b');
    libraryRecentStore.push('scene-c');
    libraryRecentStore.push('scene-a');
    expect(libraryRecentStore.ids).toEqual(['scene-a', 'scene-c', 'scene-b']);
  });

  it('caps the recent list at 20 entries (oldest evicted)', () => {
    for (let i = 0; i < 25; i++) {
      libraryRecentStore.push(`scene-${i}`);
    }
    expect(libraryRecentStore.ids.length).toBe(20);
    // Most recent push is at the front; the first five pushes were
    // evicted (scene-0 through scene-4).
    expect(libraryRecentStore.ids[0]).toBe('scene-24');
    expect(libraryRecentStore.ids[19]).toBe('scene-5');
    expect(libraryRecentStore.ids).not.toContain('scene-0');
    expect(libraryRecentStore.ids).not.toContain('scene-4');
  });

  it('push(id) writes the updated list to localStorage', () => {
    libraryRecentStore.push('scene-a');
    libraryRecentStore.push('scene-b');
    const raw = window.localStorage.getItem(STORAGE_KEY);
    expect(raw).not.toBeNull();
    expect(JSON.parse(raw as string)).toEqual(['scene-b', 'scene-a']);
  });

  it('push of an empty id is a no-op', () => {
    libraryRecentStore.push('');
    expect(libraryRecentStore.ids).toEqual([]);
    expect(window.localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it('hydrate() restores the list from localStorage', () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(['scene-x', 'scene-y']));
    libraryRecentStore.hydrate();
    expect(libraryRecentStore.ids).toEqual(['scene-x', 'scene-y']);
  });

  it('hydrate() silently resets to empty when localStorage holds corrupt JSON', () => {
    window.localStorage.setItem(STORAGE_KEY, '{not-json');
    libraryRecentStore.hydrate();
    expect(libraryRecentStore.ids).toEqual([]);
  });

  it('hydrate() silently resets when localStorage holds a non-array payload', () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ scene: 'a' }));
    libraryRecentStore.hydrate();
    expect(libraryRecentStore.ids).toEqual([]);
  });

  it('hydrate() filters out non-string entries', () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(['scene-a', 42, null, 'scene-b', '']));
    libraryRecentStore.hydrate();
    expect(libraryRecentStore.ids).toEqual(['scene-a', 'scene-b']);
  });

  it('hydrate() caps the restored list at 20 entries', () => {
    const stored = Array.from({ length: 30 }, (_, i) => `scene-${i}`);
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(stored));
    libraryRecentStore.hydrate();
    expect(libraryRecentStore.ids.length).toBe(20);
    expect(libraryRecentStore.ids[0]).toBe('scene-0');
    expect(libraryRecentStore.ids[19]).toBe('scene-19');
  });

  it('isRecent(id) returns true for tracked ids and false otherwise', () => {
    libraryRecentStore.push('scene-a');
    expect(libraryRecentStore.isRecent('scene-a')).toBe(true);
    expect(libraryRecentStore.isRecent('scene-b')).toBe(false);
  });

  it('reset() clears in-memory state without touching localStorage', () => {
    libraryRecentStore.push('scene-a');
    expect(window.localStorage.getItem(STORAGE_KEY)).not.toBeNull();
    libraryRecentStore.reset();
    expect(libraryRecentStore.ids).toEqual([]);
    // localStorage value still present — the test harness clears it
    // in afterEach.
    expect(window.localStorage.getItem(STORAGE_KEY)).not.toBeNull();
  });
});
