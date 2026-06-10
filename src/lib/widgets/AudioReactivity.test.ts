// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W45 AudioReactivity widget tests — rAF-driven primitive backfill. Bundle
// source: `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/perform.jsx`
// lines 198-261. Tests the band-tile + 48-bar FFT + tempo-strip composite
// against the prop -> render contract; reduced-motion freezes the FFT bars.
//
// Wave 6 PR 2 adds the live-mode dual-mode coverage: when a non-null
// `snapshot` prop is passed the widget reduces it into the four named
// tiles and reads BPM from `snapshot.bpm`. An OMITTED prop reads the
// singleton store; an explicit `snapshot={null}` pins the default tile
// fixtures + `bpm` prop with no store fallthrough.

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

// ADR-024 + FIX 3 — the named band tiles read the volume-independent
// AGC reactive bands (`bass` / `mid` / `treb`), mapped onto a 0..1 bar
// fill by `clamp((value - 1) / 3, 0, 1)` — a REST-AT-FLOOR reactivity
// ratio (neutral 1.0 → 0% dark, ~4.0 transient → 100%). The two upper
// tiles (HIGH MID / HIGH) share the treble band. `fft_bands` no longer
// drive the named tiles. Sample bands are chosen above neutral so the
// tiles read the same percentages the prior `value/2` mapping produced:
//   BASS    ← bass 1.75 -> clamp((0.75)/3)  -> 25%
//   LOW MID ← mid  2.50 -> clamp((1.50)/3)  -> 50%
//   HIGH MID← treb 3.10 -> clamp((2.10)/3)  -> 70%
//   HIGH    ← treb 3.10 -> clamp((2.10)/3)  -> 70%
const SAMPLE_SNAPSHOT: AudioUniformSnapshotPayload = {
  bpm: 145,
  bpm_availability: 'Ready',
  rms: 0.5,
  peak: 0.9,
  fft_bands: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
  fft_availability: 'Ready',
  bass: 1.75,
  mid: 2.5,
  treb: 3.1,
  bass_att: 1.75,
  mid_att: 2.5,
  treb_att: 3.1,
  degraded: false,
};

// jsdom does not auto-run rAF; capture scheduled callbacks and pump them
// manually. The band smoother is wall-clock-dt-aware
// (`coeff = 1 - exp(-dt/tau)`), so the pumped timestamps advance a
// synthetic clock (default 16.7 ms = one 60 Hz frame) rather than
// reusing `performance.now()` (whose near-zero inter-pump deltas would
// freeze a dt-aware follower).
function stubRafQueue(): { pump: (n: number, dtMs?: number) => Promise<void> } {
  const rafCbs: FrameRequestCallback[] = [];
  vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback): number => {
    rafCbs.push(cb);
    return rafCbs.length;
  });
  vi.stubGlobal('cancelAnimationFrame', () => {});
  let frameTs = 0;
  const pump = async (n: number, dtMs = 16.7): Promise<void> => {
    for (let i = 0; i < n; i++) {
      const cb = rafCbs.shift();
      frameTs += dtMs;
      if (cb) cb(frameTs);
      await Promise.resolve();
    }
  };
  return { pump };
}

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

  it('band fill width tracks the input value with no CSS transition on width (FIX 2)', () => {
    const customBands = [
      { name: 'SUB', value: 0.1, tint: 'red' },
      { name: 'MID', value: 0.5, tint: 'green' },
      { name: 'AIR', value: 0.9, tint: 'blue' },
    ] as const;
    const { container } = render(AudioReactivity, { props: { bands: customBands } });
    const fills = Array.from(
      container.querySelectorAll<HTMLElement>('.audio-reactivity__band-fill'),
    );
    expect(fills.length).toBe(3);
    // The fill width reflects the input value directly (the DSP/rAF layer
    // is the only smoothing — the bar geometry is not CSS-tweened).
    expect(fills[0]?.style.width).toBe('10%');
    expect(fills[1]?.style.width).toBe('50%');
    expect(fills[2]?.style.width).toBe('90%');
    // No `transition` is set on the moving bar geometry (FIX 2 removed
    // the `transition: width 120ms ease` rule; nothing re-adds it inline).
    for (const fill of fills) {
      expect(fill.style.transition).toBe('');
      expect(fill.getAttribute('style') ?? '').not.toContain('transition');
    }
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

  it('explicit snapshot={null} pins the no-live-data defaults even when the store is live', () => {
    // `null` means "explicitly no data": the widget must NOT fall through
    // to the live store. (The smoothing $effect and `useHystPct` gate on
    // `snapshot !== undefined`, so a `??` fallthrough would select the
    // live path with the smoothing loop disabled — bars stuck at 0%.)
    audioSnapshotStore.snapshot = SAMPLE_SNAPSHOT;
    const { container, getByText } = render(AudioReactivity, {
      props: { bpm: 100, snapshot: null },
    });
    // Prop bpm renders — not the store's 145.
    expect(container.querySelector('.audio-reactivity__tempo-bpm')?.textContent?.trim()).toBe(
      '100.00 BPM',
    );
    // Default fixture tiles render — not the store-driven path.
    expect(getByText('82%')).toBeInTheDocument();
    expect(getByText('64%')).toBeInTheDocument();
    expect(getByText('41%')).toBeInTheDocument();
    expect(getByText('58%')).toBeInTheDocument();
  });

  it('an omitted snapshot prop still reads the live store (undefined-only fallback)', () => {
    audioSnapshotStore.snapshot = SAMPLE_SNAPSHOT;
    const { container, queryByText } = render(AudioReactivity, { props: { bpm: 100 } });
    // Store BPM wins over the prop when the prop is omitted.
    expect(container.querySelector('.audio-reactivity__tempo-bpm')?.textContent?.trim()).toBe(
      '145.00 BPM',
    );
    // The live rAF-smoothed path is selected (no rAF pumped here, so the
    // smoothed fills sit at 0%) — the default fixture percentages must
    // NOT render.
    expect(queryByText('82%')).toBeNull();
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

  it('smoothed band fill reaches ~90% of a step within ~3 rAF frames (FIX 4)', async () => {
    // FIX 4 trusts the engine's already-smoothed AGC ballistic: the UI
    // follower is tau = 21 ms (the former per-frame alpha 0.55 at 60 Hz,
    // ~3 frames to 90%, vs ~8 at the old alpha 0.25). Freeze the
    // FFT-visualiser rAF loop (reduced-motion) so the ONLY rAF producer
    // is the band-smoother — otherwise the FFT-bar ticks interleave into
    // the captured queue and skew the frame count.
    stubMatchMedia(true);
    const { pump } = stubRafQueue();
    const bassFillPct = (container: HTMLElement): number => {
      const fill = container.querySelectorAll<HTMLElement>('.audio-reactivity__band-fill')[0];
      return parseFloat(fill?.style.width ?? '0');
    };

    // Seed at neutral (bass = 1.0 → target fill 0%) and let it settle.
    audioSnapshotStore.snapshot = { ...NEUTRAL_SNAPSHOT };
    const { container } = render(AudioReactivity);
    await Promise.resolve();
    await pump(10);
    expect(bassFillPct(container)).toBeLessThan(5);

    // Step to a full transient: bass = 4.0 → target fill = clamp((4-1)/3)
    // = 1.0 (100%). At 16.7 ms frames the tau-21 ms follower reaches ≥90%
    // in ~3 advances (0.55, 0.80, 0.91); with a 1-frame DOM-render-commit
    // lag in the harness that is observable by ~4 pumped frames. The OLD
    // alpha=0.25 would still be at ~68% here (needing ~8 frames), so this
    // cleanly pins the FIX 4 speed-up rather than the prior sluggish
    // response.
    audioSnapshotStore.snapshot = { ...NEUTRAL_SNAPSHOT, bass: 4.0 };
    await Promise.resolve();
    await pump(4);
    expect(bassFillPct(container)).toBeGreaterThanOrEqual(90);
  });

  it('band smoothing is dt-aware: small frame deltas advance the EMA proportionally', async () => {
    // Frame-rate independence (docs/dev/ui-smoothness-and-latency.md
    // principle 4): five 1 ms frames advance the follower by only
    // ~1 - exp(-5/21) ≈ 21% of a step, whereas the old fixed per-frame
    // alpha 0.55 would already sit at ~98% (running 16x faster than at
    // 60 Hz). Pins `coeff = 1 - exp(-dt/tau)` against a per-frame
    // constant regression.
    stubMatchMedia(true); // freeze the FFT-visualiser rAF producer
    const { pump } = stubRafQueue();
    const bassFillPct = (container: HTMLElement): number => {
      const fill = container.querySelectorAll<HTMLElement>('.audio-reactivity__band-fill')[0];
      return parseFloat(fill?.style.width ?? '0');
    };

    audioSnapshotStore.snapshot = { ...NEUTRAL_SNAPSHOT };
    const { container } = render(AudioReactivity);
    await Promise.resolve();
    await pump(3); // settle at the 0% neutral target
    expect(bassFillPct(container)).toBeLessThan(1);

    audioSnapshotStore.snapshot = { ...NEUTRAL_SNAPSHOT, bass: 4.0 };
    await Promise.resolve();
    await pump(5, 1); // 5 frames, 1 ms apart (~1000 fps burst)
    const pct = bassFillPct(container);
    expect(pct).toBeGreaterThan(4); // it does move…
    expect(pct).toBeLessThan(50); // …but proportionally to elapsed time
  });

  it('the live-store band fill changes when the AGC bass band swings (FIX B)', async () => {
    // No `bands` / `snapshot` prop → the rAF-smoothing path observes
    // `audioSnapshotStore.snapshot` directly. jsdom does NOT auto-run
    // rAF, so capture the scheduled callbacks and pump them manually
    // on the synthetic 60 Hz clock.
    const { pump } = stubRafQueue();

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
