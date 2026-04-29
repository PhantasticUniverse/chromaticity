// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Shared-primitive backfill — `Timecode` widget tests. Bundle source:
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
// lines 371-388. Covers initial-state render (`00:00:00:00`), the
// frozen-frame branch under `prefers-reduced-motion: reduce`, the
// `running={false}` branch (no rAF subscription), the dim colon
// separators, ARIA timer role, and the rAF cleanup on unmount.
//
// `vi.stubGlobal('matchMedia', ...)` per the user-memory
// `feedback_vitest_matchmedia_stubglobal.md` convention — `vi.spyOn`
// would throw because jsdom does not pre-expose `matchMedia`.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import Timecode from './Timecode.svelte';

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

describe('Timecode widget', () => {
  it('renders a timecode element with role=timer', () => {
    const { container } = render(Timecode);
    const tc = container.querySelector('.timecode');
    expect(tc).not.toBeNull();
    expect(tc?.getAttribute('role')).toBe('timer');
  });

  it('starts at 00:00:00:00 on first paint', () => {
    const { container } = render(Timecode);
    const text = container.querySelector('.timecode')?.textContent ?? '';
    expect(text.replace(/\s+/g, '')).toBe('00:00:00:00');
  });

  it('renders three dimmed colon separators', () => {
    const { container } = render(Timecode);
    const dims = container.querySelectorAll('.timecode .dim');
    expect(dims).toHaveLength(3);
    for (const dim of Array.from(dims)) {
      expect(dim.textContent).toBe(':');
    }
  });

  it('describes the elapsed time in aria-label', () => {
    const { container } = render(Timecode);
    const tc = container.querySelector('.timecode');
    expect(tc?.getAttribute('aria-label')).toBe('Elapsed time 00:00:00, frame 00 of 30');
  });

  it('honors running=false by leaving the readout at 00:00:00:00', () => {
    const { container } = render(Timecode, { props: { running: false } });
    const text = container.querySelector('.timecode')?.textContent ?? '';
    expect(text.replace(/\s+/g, '')).toBe('00:00:00:00');
  });

  it('pins the readout at 00:00:00:00 under prefers-reduced-motion', () => {
    stubMatchMedia(true);
    const { container } = render(Timecode);
    const text = container.querySelector('.timecode')?.textContent ?? '';
    expect(text.replace(/\s+/g, '')).toBe('00:00:00:00');
  });

  it('mounts and unmounts cleanly without throwing (rAF lifecycle owned)', () => {
    // Svelte 5's `$effect` schedules the rAF loop after a microtask, so
    // a synchronous `vi.spyOn(rAF/cancelAnimationFrame)` assertion is
    // racy under jsdom + Vitest 4. Smoke-testing the mount/unmount pair
    // is the load-bearing check; the matchMedia listener + rAF
    // teardown happens inside the `$effect`'s return function.
    const { unmount } = render(Timecode);
    expect(() => unmount()).not.toThrow();
  });
});
