// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Shape pins for the per-view empty-state copy constants. Pure-data
// fixtures; the consumers in `Library.test.ts` / `Playlist.test.ts` /
// `MIDI.test.ts` / `OSC.test.ts` test rendering separately. This file
// pins the contract so the bundle-alignment three-action pattern stays
// observable to reviewers (DESIGN.md "Bundle alignment (2026-05-18)"
// §Trail closeout).

import { describe, expect, it } from 'vitest';

import {
  EMPTY_LIBRARY,
  EMPTY_LIBRARY_FILTERED,
  EMPTY_MIDI_DEVICES,
  EMPTY_OSC,
  EMPTY_PLAYLIST,
  EMPTY_PLAYLIST_CUES,
} from './empty-states';

describe('empty-states copy contract', () => {
  it('every EMPTY_* carries a non-empty title + body', () => {
    for (const copy of [
      EMPTY_LIBRARY,
      EMPTY_LIBRARY_FILTERED,
      EMPTY_PLAYLIST,
      EMPTY_PLAYLIST_CUES,
      EMPTY_MIDI_DEVICES,
      EMPTY_OSC,
    ]) {
      expect(copy.title.length).toBeGreaterThan(0);
      expect(copy.body.length).toBeGreaterThan(0);
    }
  });

  it('EMPTY_LIBRARY carries the bundle 3-action pattern (primary + secondary + tertiary)', () => {
    // DESIGN.md "Bundle alignment (2026-05-18)" §Trail closeout — the
    // bundle's `EmptyLibrary` ships three actions (Install Built-Ins / New
    // Scene / Import). The prior 2-action cap clipped this; tertiaryAction
    // restores parity. Consumers iterate
    // `[primaryAction, secondaryAction, tertiaryAction].filter(Boolean)`.
    expect(EMPTY_LIBRARY.primaryAction?.label).toBe('Install Built-Ins');
    expect(EMPTY_LIBRARY.secondaryAction?.label).toBe('New Scene');
    expect(EMPTY_LIBRARY.tertiaryAction?.label).toBe('Import…');
  });

  it('EMPTY_LIBRARY meta carries the supported-formats footnote', () => {
    expect(EMPTY_LIBRARY.meta).toContain('.chroma');
    expect(EMPTY_LIBRARY.meta).toContain('Shadertoy');
  });

  it('EMPTY_LIBRARY_FILTERED reuses the supported-formats meta line', () => {
    expect(EMPTY_LIBRARY_FILTERED.meta).toEqual(EMPTY_LIBRARY.meta);
  });
});
