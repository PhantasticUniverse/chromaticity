// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W45 AudioReactivity widget tests — rAF-driven primitive backfill. Bundle
// source: `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/perform.jsx`
// lines 198-261. Tests the band-tile + 48-bar FFT + tempo-strip composite
// against the prop -> render contract; reduced-motion freezes the FFT bars.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import AudioReactivity from './AudioReactivity.svelte';

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

describe('AudioReactivity widget', () => {
  it('renders the wrapper with role=img and aria-label', () => {
    const { container } = render(AudioReactivity);
    const root = container.querySelector('.audio-reactivity');
    expect(root).not.toBeNull();
    expect(root?.getAttribute('role')).toBe('img');
    expect(root?.getAttribute('aria-label')).toBe('Audio reactivity preview');
  });

  it('renders the four default band tiles', () => {
    const { container, getByText } = render(AudioReactivity);
    expect(container.querySelectorAll('.audio-reactivity__band')).toHaveLength(4);
    expect(getByText('BASS')).toBeInTheDocument();
    expect(getByText('LOW MID')).toBeInTheDocument();
    expect(getByText('HIGH MID')).toBeInTheDocument();
    expect(getByText('HIGH')).toBeInTheDocument();
  });

  it('renders the 48-bar FFT visualiser as inline SVG rects', () => {
    const { container } = render(AudioReactivity);
    expect(container.querySelectorAll('rect.audio-reactivity__fft-bar')).toHaveLength(48);
  });

  it('reflects bpm and linkPeerCount props in the tempo strip', () => {
    const { container, getByText } = render(AudioReactivity, {
      props: { bpm: 96.5, linkPeerCount: 7 },
    });
    expect(container.querySelector('.audio-reactivity__tempo-bpm')?.textContent).toBe('96.50 BPM');
    expect(getByText(/Ableton Link · 7 peers/)).toBeInTheDocument();
  });

  it('honors a caller-provided bands fixture (override default tiles)', () => {
    const customBands = [
      { name: 'SUB', value: 0.1, tint: 'red' },
      { name: 'MID', value: 0.5, tint: 'green' },
      { name: 'AIR', value: 0.9, tint: 'blue' },
    ] as const;
    const { container, getByText, queryByText } = render(AudioReactivity, {
      props: { bands: customBands },
    });
    expect(container.querySelectorAll('.audio-reactivity__band')).toHaveLength(3);
    expect(getByText('SUB')).toBeInTheDocument();
    expect(getByText('MID')).toBeInTheDocument();
    expect(getByText('AIR')).toBeInTheDocument();
    expect(queryByText('BASS')).toBeNull();
    // Percent readouts: 0.1 -> 10%, 0.5 -> 50%, 0.9 -> 90%.
    expect(getByText('10%')).toBeInTheDocument();
    expect(getByText('50%')).toBeInTheDocument();
    expect(getByText('90%')).toBeInTheDocument();
  });

  it('renders the disabled TAP button (Phase-3+ stub)', () => {
    const { getByRole } = render(AudioReactivity);
    const tap = getByRole('button', { name: 'TAP' });
    expect(tap).toBeDisabled();
  });

  it('still renders 48 FFT bars under prefers-reduced-motion (frozen-frame contract)', () => {
    stubMatchMedia(true);
    const { container } = render(AudioReactivity);
    expect(container.querySelectorAll('rect.audio-reactivity__fft-bar')).toHaveLength(48);
  });
});
