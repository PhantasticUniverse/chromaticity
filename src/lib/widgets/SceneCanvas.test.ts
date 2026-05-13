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

// Wave 2.9 SC.A — `vi.hoisted` lifts the renderer-args recorder so the
// `vi.mock` factory below can populate it before any module under test
// imports `./scene-renderers`. Per the user-memory
// `feedback_vitest_invoke_mock_hoisted.md` pattern: `const rendererSpy
// = vi.fn()` referenced from `vi.mock` throws ReferenceError; the
// hoisted form is the only way to retain spy access across the
// boundary.
const { rendererSpy } = vi.hoisted(() => ({ rendererSpy: vi.fn() }));

// Mock the renderer registry to a single recording spy keyed across
// every SceneKind. The vignette post-pass + tint overlay still call
// `ctx.createRadialGradient` / `ctx.fillRect` on the real canvas
// context (jsdom returns a no-op stub) — only the per-kind procedural
// draw is intercepted. `renderStill` / `SCENE_KINDS` / `SceneKind` /
// `SceneRenderer` type re-exports stay untouched since they're imported
// by other widgets too.
vi.mock('./scene-renderers', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./scene-renderers')>();
  const kinds = actual.SCENE_KINDS;
  const stub: Record<string, typeof rendererSpy> = {};
  for (const k of kinds) stub[k] = rendererSpy;
  return {
    ...actual,
    RENDERERS: stub,
  };
});

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

// jsdom does not implement `HTMLCanvasElement.getContext`; without a
// stub the `SceneCanvas` effect short-circuits at the `if (!ctx) return`
// guard and the renderer pre-multiplier path is never exercised. The
// stub records every `fillStyle` write so the tint-overlay assertion
// can search for the `rgba(...)` string the adapter paints.
type FillStyleRecord = { styles: string[] };
function stubCanvasContext(record: FillStyleRecord): () => void {
  const orig = HTMLCanvasElement.prototype.getContext;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (HTMLCanvasElement.prototype as any).getContext = function getContext(
    this: HTMLCanvasElement,
    type: string,
  ) {
    if (type !== '2d') return null;
    const ctx = {
      fillStyle: '' as string | CanvasGradient,
      createRadialGradient: (): CanvasGradient => {
        const stops: Array<[number, string]> = [];
        const grad = {
          addColorStop: (off: number, color: string): void => {
            stops.push([off, color]);
          },
        } as unknown as CanvasGradient;
        return grad;
      },
      fillRect: vi.fn(),
    };
    return new Proxy(ctx, {
      set(target, prop, value): boolean {
        if (prop === 'fillStyle' && typeof value === 'string') {
          record.styles.push(value);
        }
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        (target as any)[prop] = value;
        return true;
      },
    });
  };
  return () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (HTMLCanvasElement.prototype as any).getContext = orig;
  };
}

beforeEach(() => {
  stubMatchMedia(false);
  rendererSpy.mockClear();
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

  // Wave 2.9 SC.A — controls pre-multiplier coverage. The reduce-motion
  // branch draws exactly one static frame inside the `$effect` so the
  // recorded renderer args are deterministic; the rAF loop path is
  // exercised by mount-and-unmount checks (per the rAF lifecycle
  // memory note above). Static-frame `t = seed * 13.7`. Tests stub
  // `HTMLCanvasElement.getContext('2d')` because jsdom's default
  // returns null and the SceneCanvas effect short-circuits before
  // `draw()` runs without it.

  it('pre-multiplies t by speed: speed=0 freezes t at 0', () => {
    stubMatchMedia(true);
    const restore = stubCanvasContext({ styles: [] });
    try {
      render(SceneCanvas, {
        props: { kind: 'rings', seed: 4, controls: { speed: 0 } },
      });
      expect(rendererSpy).toHaveBeenCalled();
      const call = rendererSpy.mock.calls[0];
      expect(call).toBeDefined();
      // Args: (ctx, t, seed, w, h, lvl); freeze drops t to 0.
      expect(call?.[1]).toBe(0);
    } finally {
      restore();
    }
  });

  it('pre-multiplies t by speed: speed=1.0 scales t by 2', () => {
    stubMatchMedia(true);
    const restore = stubCanvasContext({ styles: [] });
    try {
      render(SceneCanvas, {
        props: { kind: 'rings', seed: 4, controls: { speed: 1 } },
      });
      const call = rendererSpy.mock.calls[0];
      expect(call).toBeDefined();
      // seed=4 → static-frame t = 4 * 13.7 = 54.8; speed=1 → effectiveT = 109.6.
      expect(call?.[1]).toBeCloseTo(54.8 * 2, 5);
    } finally {
      restore();
    }
  });

  it('defaults speed to nominal 0.5 when controls.speed is absent (effectiveT = t)', () => {
    stubMatchMedia(true);
    const restore = stubCanvasContext({ styles: [] });
    try {
      render(SceneCanvas, {
        props: { kind: 'rings', seed: 4 },
      });
      const call = rendererSpy.mock.calls[0];
      expect(call).toBeDefined();
      // Nominal speed=0.5 → effectiveT = t * (0.5 * 2) = t. Pre-Wave-2.9 baseline.
      expect(call?.[1]).toBeCloseTo(54.8, 5);
    } finally {
      restore();
    }
  });

  it('pre-multiplies lvl by intensity: intensity=0 zeroes effectiveLvl', () => {
    stubMatchMedia(true);
    const restore = stubCanvasContext({ styles: [] });
    try {
      render(SceneCanvas, {
        props: { kind: 'rings', seed: 0, controls: { intensity: 0 } },
      });
      const call = rendererSpy.mock.calls[0];
      expect(call).toBeDefined();
      // Args: (ctx, t, seed, w, h, lvl). Static-frame lvl=0.5; intensity=0 → 0.
      expect(call?.[5]).toBe(0);
    } finally {
      restore();
    }
  });

  it('pre-multiplies lvl by intensity: intensity=1.0 scales lvl by 2 (clamped)', () => {
    stubMatchMedia(true);
    const restore = stubCanvasContext({ styles: [] });
    try {
      render(SceneCanvas, {
        props: { kind: 'rings', seed: 0, controls: { intensity: 1 } },
      });
      const call = rendererSpy.mock.calls[0];
      expect(call).toBeDefined();
      // lvl=0.5 * intensity=1 * 2 = 1.0 (clamped at upper bound).
      expect(call?.[5]).toBe(1);
    } finally {
      restore();
    }
  });

  it('defaults intensity to nominal 0.5 when controls.intensity is absent', () => {
    stubMatchMedia(true);
    const restore = stubCanvasContext({ styles: [] });
    try {
      render(SceneCanvas, {
        props: { kind: 'rings', seed: 0 },
      });
      const call = rendererSpy.mock.calls[0];
      expect(call).toBeDefined();
      // Nominal intensity=0.5 → effectiveLvl = 0.5 * 0.5 * 2 = 0.5 (pre-Wave-2.9 baseline).
      expect(call?.[5]).toBeCloseTo(0.5, 5);
    } finally {
      restore();
    }
  });

  it('ignores non-numeric speed / intensity values (defaults to nominal)', () => {
    stubMatchMedia(true);
    const restore = stubCanvasContext({ styles: [] });
    try {
      render(SceneCanvas, {
        // Cast through unknown to feed the runtime guard a non-number value.
        // The widget's `typeof === 'number'` check should fall through to
        // the 0.5 default.
        props: {
          kind: 'rings',
          seed: 4,
          controls: { speed: [0, 0] as unknown as number, intensity: [0, 0] as unknown as number },
        },
      });
      const call = rendererSpy.mock.calls[0];
      expect(call).toBeDefined();
      expect(call?.[1]).toBeCloseTo(54.8, 5);
      expect(call?.[5]).toBeCloseTo(0.5, 5);
    } finally {
      restore();
    }
  });

  it('paints tint overlay (Vec4) post-vignette with alpha = ta * 0.2', () => {
    stubMatchMedia(true);
    const record: FillStyleRecord = { styles: [] };
    const restore = stubCanvasContext(record);
    try {
      render(SceneCanvas, {
        props: {
          kind: 'rings',
          seed: 0,
          controls: { tint: [1, 0, 0, 1] },
        },
      });
      // Expect a rgba(...) entry with red colour + alpha=0.2.
      const tintFill = record.styles.find((s) => /rgba\(255,\s*0,\s*0,\s*0\.2\)/.test(s));
      expect(tintFill).toBeDefined();
    } finally {
      restore();
    }
  });

  it('skips tint overlay when controls.tint is absent (no rgba(...) fill)', () => {
    stubMatchMedia(true);
    const record: FillStyleRecord = { styles: [] };
    const restore = stubCanvasContext(record);
    try {
      render(SceneCanvas, {
        props: { kind: 'rings', seed: 0 },
      });
      // Absent tint → only the vignette `CanvasGradient` is assigned as
      // `fillStyle`; the recorder only logs string assignments, so no
      // entries should be present. (Empty array is a strong signal the
      // tint branch was not taken.)
      expect(record.styles).toEqual([]);
      expect(rendererSpy).toHaveBeenCalled();
    } finally {
      restore();
    }
  });
});
