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
import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';

const NEUTRAL_SNAPSHOT: AudioUniformSnapshotPayload = {
  bpm: 120,
  bpm_availability: 'Ready',
  rms: 0.01,
  peak: 0.02,
  fft_bands: [0, 0, 0, 0, 0, 0, 0, 0],
  fft_availability: 'Ready',
  bass: 1,
  mid: 1,
  treb: 1,
  bass_att: 1,
  mid_att: 1,
  treb_att: 1,
  degraded: false,
};

// ADR-024 — the named band tiles read the volume-independent AGC
// reactive bands (`bass` / `mid` / `treb`), mapped onto a 0..1 bar
// fill by `clamp(value / 2, 0, 1)` (neutral 1.0 → 50%, ~4.2 transient
// → 100%). The two upper tiles (HIGH MID / HIGH) share the treble
// band. `fft_bands` no longer drive the named tiles.
//   BASS    ← bass 0.5  -> clamp(0.25)  -> 25%
//   LOW MID ← mid  1.0  -> clamp(0.50)  -> 50%
//   HIGH MID← treb 1.4  -> clamp(0.70)  -> 70%
//   HIGH    ← treb 1.4  -> clamp(0.70)  -> 70%
const SAMPLE_SNAPSHOT: AudioUniformSnapshotPayload = {
  bpm: 145,
  bpm_availability: 'Ready',
  rms: 0.5,
  peak: 0.9,
  fft_bands: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
  fft_availability: 'Ready',
  bass: 0.5,
  mid: 1,
  treb: 1.4,
  bass_att: 0.5,
  mid_att: 1,
  treb_att: 1.4,
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
  audioSnapshotStore.snapshot = null;
  audioSnapshotStore.unsubscribe();
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

  it('still shows the BPM placeholder while the snapshot BPM axis is warming', () => {
    // ADR-024: the named tiles read the AGC bands (which carry a
    // neutral 1.0 default and never report `Warming`), so the
    // FFT-warming dim treatment no longer blanks them. BPM warming
    // still surfaces the `-- BPM` placeholder.
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
    // Tiles render their AGC-derived percentages, NOT the `--` warming
    // placeholder, and carry no warming modifier class.
    const pct = Array.from(container.querySelectorAll('.audio-reactivity__band-pct')).map((el) =>
      el.textContent?.trim(),
    );
    expect(pct).toEqual(['25%', '50%', '70%', '70%']);
    expect(container.querySelectorAll('.audio-reactivity__band--warming')).toHaveLength(0);
  });

  it('maps the AGC bands (bass/mid/treb) onto the four named tile fills', () => {
    const { container, getByText } = render(AudioReactivity, {
      props: { snapshot: SAMPLE_SNAPSHOT },
    });
    expect(container.querySelectorAll('.audio-reactivity__band')).toHaveLength(4);
    expect(getByText('BASS')).toBeInTheDocument();
    expect(getByText('LOW MID')).toBeInTheDocument();
    expect(getByText('HIGH MID')).toBeInTheDocument();
    expect(getByText('HIGH')).toBeInTheDocument();
    // clamp(value/2): bass 0.5 -> 25%, mid 1.0 -> 50%, treb 1.4 -> 70%
    // (HIGH MID and HIGH both read treb).
    expect(getByText('25%')).toBeInTheDocument();
    expect(getByText('50%')).toBeInTheDocument();
    const seventy = Array.from(container.querySelectorAll('.audio-reactivity__band-pct')).filter(
      (el) => el.textContent?.trim() === '70%',
    );
    expect(seventy).toHaveLength(2);
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

  it('the live-store band fill changes when the AGC bass band swings (FIX B)', async () => {
    // No `bands` / `snapshot` prop → the rAF-smoothing path observes
    // `audioSnapshotStore.snapshot` directly. jsdom does NOT auto-run
    // rAF, so capture the scheduled callback and pump it manually.
    const rafCbs: FrameRequestCallback[] = [];
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback): number => {
      rafCbs.push(cb);
      return rafCbs.length;
    });
    vi.stubGlobal('cancelAnimationFrame', () => {});
    const pump = async (n: number): Promise<void> => {
      for (let i = 0; i < n; i++) {
        const cb = rafCbs.shift();
        if (cb) cb(performance.now());
        await Promise.resolve();
      }
    };

    const bassFillWidth = (container: HTMLElement): string => {
      const fill = container.querySelectorAll<HTMLElement>('.audio-reactivity__band-fill')[0];
      // Read the inline `width: NN%` literal off the first (BASS) tile.
      return fill?.style.width ?? '';
    };

    // Neutral snapshot: bass = 1.0 → fill drifts toward 50%.
    audioSnapshotStore.snapshot = { ...NEUTRAL_SNAPSHOT };
    const { container } = render(AudioReactivity);
    await Promise.resolve();
    await pump(30);
    const neutralWidth = bassFillWidth(container);
    expect(neutralWidth).not.toBe('');

    // Transient: bass = 4.2 → fill drifts toward 100%. The smoothed
    // value MUST move, so the inline width string changes.
    audioSnapshotStore.snapshot = { ...NEUTRAL_SNAPSHOT, bass: 4.2 };
    await Promise.resolve();
    await pump(30);
    const loudWidth = bassFillWidth(container);
    expect(loudWidth).not.toBe(neutralWidth);
    // Sanity: the loud transient saturates the bar (clamped to 100%).
    expect(parseFloat(loudWidth)).toBeGreaterThan(parseFloat(neutralWidth));
  });
});
