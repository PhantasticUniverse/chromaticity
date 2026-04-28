// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W45 VuStereo widget tests — rAF-driven primitive backfill. Bundle source:
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/perform.jsx`. Tests
// the prop -> render contract, ARIA, the `live` rAF gate, and the
// reduced-motion frozen-frame branch; rAF tick math is implementation detail.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import VuStereo from './VuStereo.svelte';

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
  // jsdom does not implement matchMedia; default to "no preference" so the
  // rAF code path runs without throwing. Reduced-motion tests override.
  stubMatchMedia(false);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  cleanup();
});

describe('VuStereo widget', () => {
  it('renders the stereo wrapper with the aria-label', () => {
    const { container } = render(VuStereo, { props: { live: false } });
    const root = container.querySelector('.vu-stereo');
    expect(root).not.toBeNull();
    expect(root?.getAttribute('aria-label')).toBe('Stereo output levels');
  });

  it('renders two .vu-row level meters', () => {
    const { container } = render(VuStereo, { props: { live: false } });
    expect(container.querySelectorAll('.vu-row')).toHaveLength(2);
  });

  it('applies the width prop to the root style', () => {
    const { container } = render(VuStereo, { props: { live: false, width: 160 } });
    const root = container.querySelector('.vu-stereo') as HTMLElement | null;
    expect(root?.style.width).toBe('160px');
  });

  it('renders frozen levels when live=false', () => {
    const { container } = render(VuStereo, { props: { live: false } });
    const rows = container.querySelectorAll<HTMLElement>('.vu-row');
    // Frozen contract: FROZEN_L = 0.6 -> 60%, FROZEN_R = 0.55 -> ~55%
    // (FP-tolerant match: `0.55 * 100` produces 55.000000000000014).
    expect(rows[0]?.style.getPropertyValue('--lvl')).toMatch(/^60(\.0+)?%$/);
    expect(rows[1]?.style.getPropertyValue('--lvl')).toMatch(/^55(\.0+\d*)?%$/);
  });

  it('renders frozen levels under prefers-reduced-motion even when live=true', () => {
    stubMatchMedia(true);
    const { container } = render(VuStereo, { props: { live: true } });
    const rows = container.querySelectorAll<HTMLElement>('.vu-row');
    expect(rows[0]?.style.getPropertyValue('--lvl')).toMatch(/^60(\.0+)?%$/);
    expect(rows[1]?.style.getPropertyValue('--lvl')).toMatch(/^55(\.0+\d*)?%$/);
  });

  it('marks the meter rows aria-hidden so the wrapper label is the only a11y node', () => {
    const { container } = render(VuStereo, { props: { live: false } });
    const rows = container.querySelectorAll('.vu-row');
    rows.forEach((row) => {
      expect(row.getAttribute('aria-hidden')).toBe('true');
    });
  });
});
