// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W45 BeatDots widget tests — rAF-driven primitive backfill. Bundle source:
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx` lines
// 360-369. Tests the structural render, ARIA label, BPM-prop flow, and the
// reduced-motion frozen-frame branch; rAF tick math is implementation detail.
//
// Wave 6 PR 2 adds the live-mode dual-mode coverage: when a non-null
// `snapshot` prop is passed the widget reads BPM from `snapshot.bpm`;
// `null`/missing falls back to the existing `bpm` prop / synthetic
// rAF behaviour exactly as before.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import BeatDots from './BeatDots.svelte';
import type { AudioUniformSnapshotPayload } from '../ipc/bindings/AudioUniformSnapshotPayload';

const SAMPLE_SNAPSHOT: AudioUniformSnapshotPayload = {
  bpm: 145,
  bpm_availability: 'Ready',
  rms: 0.5,
  peak: 0.9,
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

describe('BeatDots widget', () => {
  it('renders four .beat-dots__dot elements', () => {
    const { container } = render(BeatDots);
    expect(container.querySelectorAll('.beat-dots__dot')).toHaveLength(4);
  });

  it('renders the beat-dots wrapper with role=img', () => {
    const { container } = render(BeatDots);
    const root = container.querySelector('.beat-dots');
    expect(root).not.toBeNull();
    expect(root?.getAttribute('role')).toBe('img');
  });

  it('reflects the bpm prop in the aria-label', () => {
    const { container } = render(BeatDots, { props: { bpm: 96 } });
    const root = container.querySelector('.beat-dots');
    expect(root?.getAttribute('aria-label')).toContain('96 BPM');
  });

  it('defaults to 128 BPM in the aria-label when no prop is given', () => {
    const { container } = render(BeatDots);
    const root = container.querySelector('.beat-dots');
    expect(root?.getAttribute('aria-label')).toContain('128 BPM');
  });

  it('pins the downbeat (beat 1 of 4) under prefers-reduced-motion', () => {
    stubMatchMedia(true);
    const { container } = render(BeatDots, { props: { bpm: 128 } });
    const root = container.querySelector('.beat-dots');
    expect(root?.getAttribute('aria-label')).toBe('Beat 1 of 4 at 128 BPM');
    // Reduced-motion -> now=0 -> cur=0 -> first dot is the downbeat
    // (the only --down dot, no --on dots).
    expect(container.querySelectorAll('.beat-dots__dot--down')).toHaveLength(1);
    expect(container.querySelectorAll('.beat-dots__dot--on')).toHaveLength(0);
  });

  it('marks each dot aria-hidden so the wrapper aria-label is the sole a11y node', () => {
    const { container } = render(BeatDots);
    const dots = container.querySelectorAll('.beat-dots__dot');
    dots.forEach((dot) => {
      expect(dot.getAttribute('aria-hidden')).toBe('true');
    });
  });

  it('reflects snapshot.bpm in the aria-label when supplied (Wave 6 PR 2 live mode)', () => {
    const { container } = render(BeatDots, { props: { bpm: 100, snapshot: SAMPLE_SNAPSHOT } });
    const root = container.querySelector('.beat-dots');
    expect(root?.getAttribute('aria-label')).toContain('145 BPM');
  });

  it('falls back to the bpm prop when snapshot is null', () => {
    const { container } = render(BeatDots, { props: { bpm: 100, snapshot: null } });
    const root = container.querySelector('.beat-dots');
    expect(root?.getAttribute('aria-label')).toContain('100 BPM');
  });
});
