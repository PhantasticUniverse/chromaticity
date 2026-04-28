// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W36 SceneStill widget tests — exercise the static-thumbnail surface
// at the structural level. jsdom's canvas implementation does not paint
// pixels (getContext('2d') may return null or a noop-stub), so the
// tests cover (a) the canvas element renders with the right HTML attrs,
// (b) the SCENE_KINDS family is complete and RENDERERS dispatch is
// keyed by every kind, and (c) `renderStill` does not throw for any
// kind given a real or stub 2d context. Pixel-level fidelity is the
// bundle's responsibility; these tests verify the port preserves the
// public contract.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import { RENDERERS, SCENE_KINDS, renderStill, type SceneKind } from './scene-renderers';
import SceneStill from './SceneStill.svelte';

describe('SCENE_KINDS family', () => {
  it('declares all 16 bundle-faithful kind names', () => {
    expect(SCENE_KINDS).toHaveLength(16);
    expect(SCENE_KINDS).toEqual([
      'rings',
      'fluid',
      'grid',
      'kaleido',
      'noise',
      'cells',
      'rain',
      'plasma',
      'orbit',
      'tunnel',
      'filaments',
      'glitch',
      'ink',
      'bars',
      'dunes',
      'lattice',
    ]);
  });

  it('RENDERERS is keyed by every SceneKind', () => {
    for (const kind of SCENE_KINDS) {
      expect(RENDERERS[kind]).toBeTypeOf('function');
    }
  });
});

describe('renderStill', () => {
  function makeCanvas(width = 100, height = 44): HTMLCanvasElement {
    const c = document.createElement('canvas');
    c.width = width;
    c.height = height;
    return c;
  }

  it('does not throw for any kind', () => {
    for (const kind of SCENE_KINDS) {
      const c = makeCanvas();
      expect(() => renderStill(c, kind, 7)).not.toThrow();
    }
  });

  it('honors a tOverride argument', () => {
    const c = makeCanvas();
    expect(() => renderStill(c, 'rings', 3, 0)).not.toThrow();
    expect(() => renderStill(c, 'rings', 3, 999)).not.toThrow();
  });

  it('falls back to rings for an unknown kind', () => {
    const c = makeCanvas();
    // Cast through unknown to bypass the SceneKind constraint — the
    // runtime fallback lives at `RENDERERS[kind] ?? RENDERERS.rings`
    // and must not throw on a missing key.
    expect(() => renderStill(c, 'not-a-real-kind' as unknown as SceneKind, 0)).not.toThrow();
  });
});

describe('SceneStill widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders a canvas element with the requested width and height', () => {
    const { container } = render(SceneStill, {
      props: { kind: 'rings', seed: 4, width: 100, height: 44 },
    });
    const canvas = container.querySelector('canvas');
    expect(canvas).not.toBeNull();
    expect(canvas?.getAttribute('width')).toBe('100');
    expect(canvas?.getAttribute('height')).toBe('44');
  });

  it('defaults to role="presentation" when no ariaLabel is given', () => {
    const { container } = render(SceneStill, {
      props: { kind: 'rings', seed: 0 },
    });
    const canvas = container.querySelector('canvas');
    expect(canvas?.getAttribute('role')).toBe('presentation');
  });

  it('switches to role="img" + aria-label when an ariaLabel is provided', () => {
    const { container } = render(SceneStill, {
      props: { kind: 'plasma', seed: 1, ariaLabel: 'Festival opener thumbnail' },
    });
    const canvas = container.querySelector('canvas');
    expect(canvas?.getAttribute('role')).toBe('img');
    expect(canvas?.getAttribute('aria-label')).toBe('Festival opener thumbnail');
  });

  it('mounts cleanly for every SceneKind without throwing', () => {
    for (const kind of SCENE_KINDS) {
      const { container, unmount } = render(SceneStill, {
        props: { kind, seed: kind.length, width: 64, height: 36 },
      });
      expect(container.querySelector('canvas')).not.toBeNull();
      unmount();
    }
  });
});
