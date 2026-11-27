// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { describe, expect, it } from 'vitest';

import { sceneKindFromMetadata, sceneSeedFromMetadata } from './scene-kind-from-metadata';

describe('sceneKindFromMetadata', () => {
  it('returns the default ring kind for null or empty input', () => {
    expect(sceneKindFromMetadata(null)).toBe('rings');
    expect(sceneKindFromMetadata(undefined)).toBe('rings');
    expect(sceneKindFromMetadata({})).toBe('rings');
  });

  it('maps the 12 canonical bundle scene titles to their kinds', () => {
    // Mirrors the `setlist-fixtures.ts` rows + `snap-bar.fixtures.ts` defaults.
    const canonical: ReadonlyArray<readonly [string, string]> = [
      ['Quicksand', 'dunes'],
      ['Glob', 'fluid'],
      ['Sun_Set', 'rings'],
      ['Glitch', 'glitch'],
      ['Glassier', 'kaleido'],
      ['Ink Shot', 'ink'],
      ['Hex Array', 'lattice'],
      ['SquareShoals', 'grid'],
      ['Churning', 'plasma'],
      ['Myoplasm', 'cells'],
      ['GPU_Melter', 'filaments'],
      ['Test Card', 'bars'],
    ];
    for (const [title, kind] of canonical) {
      expect(sceneKindFromMetadata({ title })).toBe(kind);
    }
  });

  it('matches when the title contains a SceneKind name directly', () => {
    expect(sceneKindFromMetadata({ title: 'demo-kaleido' })).toBe('kaleido');
    expect(sceneKindFromMetadata({ title: 'Smooth Kaleidoscope 2' })).toBe('kaleido');
    expect(sceneKindFromMetadata({ title: 'Plasma Showcase' })).toBe('plasma');
    // First-match-wins against the SCENE_KINDS declaration order
    // (rings comes before orbit). The brief documents this behaviour
    // as deterministic; the user-visible takeaway is that direct
    // SceneKind names in the title always resolve to themselves
    // when they are the only match.
    expect(sceneKindFromMetadata({ title: 'orbit-demo' })).toBe('orbit');
    expect(sceneKindFromMetadata({ title: 'just-rings' })).toBe('rings');
  });

  it('uses tag keywords when the title does not match', () => {
    expect(
      sceneKindFromMetadata({
        title: 'Untitled',
        tags: ['audio reactive', 'bpm'],
      }),
    ).toBe('bars');
    expect(
      sceneKindFromMetadata({
        title: 'Untitled',
        tags: ['hex'],
      }),
    ).toBe('lattice');
  });

  it('falls back to the category mapping when neither title nor tags match', () => {
    expect(
      sceneKindFromMetadata({
        title: 'Untitled scene',
        categories: ['GEOMETRIC'],
      }),
    ).toBe('lattice');
    expect(
      sceneKindFromMetadata({
        title: 'Untitled scene',
        categories: ['3D'],
      }),
    ).toBe('tunnel');
    expect(
      sceneKindFromMetadata({
        title: 'Untitled scene',
        categories: ['ORGANIC'],
      }),
    ).toBe('fluid');
  });

  it('defaults to rings when no signal at all', () => {
    expect(
      sceneKindFromMetadata({
        title: 'Untitled scene',
        description: '',
        tags: [],
        categories: [],
      }),
    ).toBe('rings');
  });
});

describe('sceneSeedFromMetadata', () => {
  it('returns 0 for empty input', () => {
    expect(sceneSeedFromMetadata(null)).toBe(0);
    expect(sceneSeedFromMetadata({})).toBe(0);
  });

  it('is deterministic across calls for the same id', () => {
    const a = sceneSeedFromMetadata({ id: 'demo-kaleido' });
    const b = sceneSeedFromMetadata({ id: 'demo-kaleido' });
    expect(a).toBe(b);
  });

  it('differs across distinct ids', () => {
    const a = sceneSeedFromMetadata({ id: 'demo-kaleido' });
    const b = sceneSeedFromMetadata({ id: 'demo-glitch' });
    expect(a).not.toBe(b);
  });

  it('folds into the [0, 64) range', () => {
    for (const id of [
      'demo-kaleido',
      'a',
      'Quicksand',
      'very-long-scene-id-with-many-characters',
    ]) {
      const s = sceneSeedFromMetadata({ id });
      expect(s).toBeGreaterThanOrEqual(0);
      expect(s).toBeLessThan(64);
    }
  });
});
