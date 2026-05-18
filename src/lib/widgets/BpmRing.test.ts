// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W45 BpmRing widget tests — rAF-driven primitive backfill. Bundle source:
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx` lines
// 343-358. Tests the structural render, ARIA label, BPM/size prop flow, and
// the reduced-motion frozen-frame branch; rAF tick math is implementation
// detail (the dashoffset only animates at >0 frames).
//
// Wave 6 PR 2 adds the live-mode dual-mode coverage: when a non-null
// `snapshot` prop is passed the widget reads BPM from `snapshot.bpm`;
// `null`/missing falls back to the existing `bpm` prop / synthetic
// rAF behaviour exactly as before.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import BpmRing from './BpmRing.svelte';
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

describe('BpmRing widget', () => {
  it('renders the bpm-ring wrapper with role=img', () => {
    const { container } = render(BpmRing);
    const root = container.querySelector('.bpm-ring');
    expect(root).not.toBeNull();
    expect(root?.getAttribute('role')).toBe('img');
  });

  it('renders both ring tracks (bg + fg) and the BPM readout', () => {
    const { container } = render(BpmRing, { props: { bpm: 140 } });
    expect(container.querySelector('.bpm-ring__bg')).not.toBeNull();
    expect(container.querySelector('.bpm-ring__fg')).not.toBeNull();
    expect(container.querySelector('.bpm-ring__val')?.textContent).toBe('140');
  });

  it('reflects the bpm prop in the aria-label', () => {
    const { container } = render(BpmRing, { props: { bpm: 100 } });
    const root = container.querySelector('.bpm-ring');
    expect(root?.getAttribute('aria-label')).toContain('100 BPM');
  });

  it('applies the size prop to the root style (width + height)', () => {
    const { container } = render(BpmRing, { props: { size: 64 } });
    const root = container.querySelector('.bpm-ring') as HTMLElement | null;
    expect(root?.style.width).toBe('64px');
    expect(root?.style.height).toBe('64px');
  });

  it('pins beat 1 / measure 0 under prefers-reduced-motion', () => {
    stubMatchMedia(true);
    const { container } = render(BpmRing, { props: { bpm: 128 } });
    const root = container.querySelector('.bpm-ring');
    expect(root?.getAttribute('aria-label')).toBe('BPM ring at 128 BPM, beat 1 of 4');
    // Reduced-motion -> now=0 -> measureFrac=0 -> dashoffset = full circumference.
    const fg = container.querySelector('.bpm-ring__fg');
    const dashArray = fg?.getAttribute('stroke-dasharray');
    const dashOffset = fg?.getAttribute('stroke-dashoffset');
    expect(dashArray).toBe(dashOffset);
  });

  it('renders an aria-hidden inner SVG so the wrapper label is the sole a11y node', () => {
    const { container } = render(BpmRing);
    const svg = container.querySelector('.bpm-ring svg');
    expect(svg?.getAttribute('aria-hidden')).toBe('true');
  });

  it('renders bpm from snapshot prop when supplied (Wave 6 PR 2 live mode)', () => {
    const { container } = render(BpmRing, { props: { bpm: 100, snapshot: SAMPLE_SNAPSHOT } });
    expect(container.querySelector('.bpm-ring__val')?.textContent).toBe('145');
    expect(container.querySelector('.bpm-ring')?.getAttribute('aria-label')).toContain('145 BPM');
  });

  it('renders a dim placeholder while snapshot BPM is warming', () => {
    const { container } = render(BpmRing, {
      props: {
        bpm: 100,
        snapshot: { ...SAMPLE_SNAPSHOT, bpm: 145, bpm_availability: 'Warming' },
      },
    });
    expect(container.querySelector('.bpm-ring__val')?.textContent).toBe('--');
    expect(container.querySelector('.bpm-ring')).toHaveClass('bpm-ring--warming');
    expect(container.querySelector('.bpm-ring')?.getAttribute('aria-label')).toContain('warming');
  });

  it('falls back to the bpm prop when snapshot is null', () => {
    const { container } = render(BpmRing, { props: { bpm: 100, snapshot: null } });
    expect(container.querySelector('.bpm-ring__val')?.textContent).toBe('100');
    expect(container.querySelector('.bpm-ring')?.getAttribute('aria-label')).toContain('100 BPM');
  });
});
