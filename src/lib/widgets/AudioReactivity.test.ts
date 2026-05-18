// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W45 AudioReactivity widget tests — rAF-driven primitive backfill. Bundle
// source: `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/perform.jsx`
// lines 198-261. Tests the band-tile + 48-bar FFT + tempo-strip composite
// against the prop -> render contract; reduced-motion freezes the FFT bars.
//
// Wave 6 PR 2 adds the live-mode dual-mode coverage: when a non-null
// `snapshot` prop is passed the widget reduces `snapshot.fft_bands`
// (8 entries) into the four named tiles by averaging adjacent pairs
// and reads BPM from `snapshot.bpm`. `null`/missing falls back to the
// existing default tile fixtures and `bpm` prop.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import AudioReactivity from './AudioReactivity.svelte';
import type { AudioUniformSnapshotPayload } from '../ipc/bindings/AudioUniformSnapshotPayload';

const SAMPLE_SNAPSHOT: AudioUniformSnapshotPayload = {
  bpm: 145,
  bpm_availability: 'Ready',
  rms: 0.5,
  peak: 0.9,
  // BASS = avg(0.10, 0.20) = 0.15 -> 15%
  // LOW MID = avg(0.30, 0.40) = 0.35 -> 35%
  // HIGH MID = avg(0.50, 0.60) = 0.55 -> 55%
  // HIGH = avg(0.70, 0.80) = 0.75 -> 75%
  fft_bands: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
  fft_availability: 'Ready',
  degraded: false,
};

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
    expect(container.querySelector('.audio-reactivity__tempo-bpm')?.textContent?.trim()).toBe(
      '96.50 BPM',
    );
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

  it('renders bpm from snapshot when supplied (Wave 6 PR 2 live mode)', () => {
    const { container } = render(AudioReactivity, {
      props: { bpm: 100, snapshot: SAMPLE_SNAPSHOT },
    });
    expect(container.querySelector('.audio-reactivity__tempo-bpm')?.textContent?.trim()).toBe(
      '145.00 BPM',
    );
  });

  it('renders placeholders while snapshot BPM and FFT are warming', () => {
    const { container } = render(AudioReactivity, {
      props: {
        bpm: 100,
        snapshot: {
          ...SAMPLE_SNAPSHOT,
          bpm_availability: 'Warming',
          fft_availability: 'Warming',
        },
      },
    });
    expect(container.querySelector('.audio-reactivity__tempo-bpm')?.textContent?.trim()).toBe(
      '-- BPM',
    );
    const pct = Array.from(container.querySelectorAll('.audio-reactivity__band-pct')).map((el) =>
      el.textContent?.trim(),
    );
    expect(pct).toEqual(['--', '--', '--', '--']);
    expect(container.querySelectorAll('.audio-reactivity__band--warming')).toHaveLength(4);
  });

  it('reduces snapshot.fft_bands (8) into the four named band tiles by averaging adjacent pairs', () => {
    const { container, getByText } = render(AudioReactivity, {
      props: { snapshot: SAMPLE_SNAPSHOT },
    });
    expect(container.querySelectorAll('.audio-reactivity__band')).toHaveLength(4);
    expect(getByText('BASS')).toBeInTheDocument();
    expect(getByText('LOW MID')).toBeInTheDocument();
    expect(getByText('HIGH MID')).toBeInTheDocument();
    expect(getByText('HIGH')).toBeInTheDocument();
    // Reduced values: 0.15 / 0.35 / 0.55 / 0.75 -> 15% / 35% / 55% / 75%
    expect(getByText('15%')).toBeInTheDocument();
    expect(getByText('35%')).toBeInTheDocument();
    expect(getByText('55%')).toBeInTheDocument();
    expect(getByText('75%')).toBeInTheDocument();
  });

  it('falls back to default band fixtures when snapshot is null', () => {
    const { getByText } = render(AudioReactivity, { props: { bpm: 100, snapshot: null } });
    // Default values from DEFAULT_BANDS: 0.82 / 0.64 / 0.41 / 0.58
    expect(getByText('82%')).toBeInTheDocument();
    expect(getByText('64%')).toBeInTheDocument();
    expect(getByText('41%')).toBeInTheDocument();
    expect(getByText('58%')).toBeInTheDocument();
  });

  it('caller-supplied bands prop overrides snapshot reduction', () => {
    const customBands = [
      { name: 'SUB', value: 0.1, tint: 'red' },
      { name: 'AIR', value: 0.9, tint: 'blue' },
    ] as const;
    const { container, getByText, queryByText } = render(AudioReactivity, {
      props: { bands: customBands, snapshot: SAMPLE_SNAPSHOT },
    });
    expect(container.querySelectorAll('.audio-reactivity__band')).toHaveLength(2);
    expect(getByText('SUB')).toBeInTheDocument();
    expect(getByText('AIR')).toBeInTheDocument();
    // Snapshot-reduced names must NOT appear when bands prop wins.
    expect(queryByText('BASS')).toBeNull();
    expect(queryByText('LOW MID')).toBeNull();
  });
});
