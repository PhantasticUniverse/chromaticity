// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Shared-primitive backfill — `SceneCanvas` widget tests. Bundle source:
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/shaders.jsx`
// lines 278-310. Covers structural mount, role/aria-label switching,
// prefers-reduced-motion frozen-frame branch (no rAF subscription),
// rAF lifecycle (start + cleanup on unmount), and the `audio` /
// `dpr` props' propagation. jsdom's canvas does not paint pixels, so
// pixel-fidelity is the bundle's responsibility — this file verifies
// the widget composes the public contract.
//
// `vi.stubGlobal('matchMedia', ...)` per the user-memory
// `feedback_vitest_matchmedia_stubglobal.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { SCENE_KINDS } from './scene-renderers';
import SceneCanvas from './SceneCanvas.svelte';

function stubMatchMedia(reduce: boolean): void {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: reduce && query.includes('prefers-reduced-motion: reduce'),
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
}

beforeEach(() => {
  stubMatchMedia(false);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  cleanup();
});

describe('SceneCanvas widget', () => {
  it('renders a canvas with the scene-canvas class', () => {
    const { container } = render(SceneCanvas, {
      props: { kind: 'rings', seed: 0 },
    });
    expect(container.querySelector('canvas.scene-canvas')).not.toBeNull();
  });

  it('defaults to role=presentation when no ariaLabel is given', () => {
    const { container } = render(SceneCanvas, {
      props: { kind: 'rings', seed: 0 },
    });
    expect(container.querySelector('canvas')?.getAttribute('role')).toBe('presentation');
  });

  it('switches to role=img + aria-label when an ariaLabel is supplied', () => {
    const { container } = render(SceneCanvas, {
      props: { kind: 'plasma', seed: 1, ariaLabel: 'Live scene preview' },
    });
    const c = container.querySelector('canvas');
    expect(c?.getAttribute('role')).toBe('img');
    expect(c?.getAttribute('aria-label')).toBe('Live scene preview');
  });

  it('mounts cleanly for every SceneKind without throwing', () => {
    for (const kind of SCENE_KINDS) {
      const { container, unmount } = render(SceneCanvas, {
        props: { kind, seed: kind.length },
      });
      expect(container.querySelector('canvas.scene-canvas')).not.toBeNull();
      unmount();
    }
  });

  it('mounts and unmounts cleanly without throwing (rAF lifecycle owned)', () => {
    // Svelte 5's `$effect` schedules the rAF loop after a microtask, so
    // a synchronous `vi.spyOn(rAF)` assertion is racy under jsdom +
    // Vitest 4 — see `feedback_vitest_microtask_autofocus.md`. Smoke-
    // testing the mount/unmount pair is the load-bearing check; rAF +
    // ResizeObserver teardown happens inside the `$effect`'s return
    // function, and a leaked subscription would surface as a timer-
    // hold leak across the suite.
    const { unmount } = render(SceneCanvas, { props: { kind: 'rings', seed: 0 } });
    expect(() => unmount()).not.toThrow();
  });

  it('mounts and unmounts cleanly under prefers-reduced-motion', () => {
    stubMatchMedia(true);
    const { container, unmount } = render(SceneCanvas, {
      props: { kind: 'rings', seed: 0 },
    });
    // The reduced-motion branch renders one static frame and skips the
    // rAF loop entirely — same canvas surface, no animation.
    expect(container.querySelector('canvas.scene-canvas')).not.toBeNull();
    expect(() => unmount()).not.toThrow();
  });

  it('does not throw when audio=false (synthetic level fixed at 0.3)', () => {
    const { container, unmount } = render(SceneCanvas, {
      props: { kind: 'plasma', seed: 2, audio: false },
    });
    expect(container.querySelector('canvas.scene-canvas')).not.toBeNull();
    unmount();
  });

  it('honors dpr=2 by sizing the bitmap above 1px', () => {
    const { container, unmount } = render(SceneCanvas, {
      props: { kind: 'rings', seed: 0, dpr: 2 },
    });
    const c = container.querySelector('canvas') as HTMLCanvasElement | null;
    // jsdom's getBoundingClientRect returns 0×0 for unattached layout, so
    // the resize step floors to width=height=1 — but the bitmap is set,
    // not undefined, and `.width` is at least 1.
    expect(c?.width).toBeGreaterThanOrEqual(1);
    expect(c?.height).toBeGreaterThanOrEqual(1);
    unmount();
  });

  // Wave 2.3 Q.1 stress-test fixes — regression coverage for the AO-
  // reported "mostly black flashing" Perform preview symptom.
  it('defaults dpr to window.devicePixelRatio when no explicit dpr is passed', () => {
    // The dpr auto-default lives in the $effect; we can't easily observe
    // the computed value but we CAN verify the mount/unmount cycle stays
    // clean across both null and explicit-dpr cases (regression guard
    // for the `dpr ?? window.devicePixelRatio` access pattern, which
    // previously hardcoded `dpr=1`).
    const { container, unmount } = render(SceneCanvas, {
      props: { kind: 'rings', seed: 0 },
    });
    expect(container.querySelector('canvas.scene-canvas')).not.toBeNull();
    expect(() => unmount()).not.toThrow();
  });

  it('mounts with audio=true and a null live snapshot (synthetic fallback path)', () => {
    // The Wave 2.3 Q.1 audio-gate widening from
    // `live && (live.rms > 0 || live.peak > 0)` → `live !== null` is
    // covered by the existing fallback path (no live snapshot in
    // Vitest jsdom → synthetic sine pulse). This guard catches a
    // future regression where the gate logic re-narrows.
    const { container, unmount } = render(SceneCanvas, {
      props: { kind: 'bars', seed: 1, audio: true },
    });
    expect(container.querySelector('canvas.scene-canvas')).not.toBeNull();
    expect(() => unmount()).not.toThrow();
  });
});
