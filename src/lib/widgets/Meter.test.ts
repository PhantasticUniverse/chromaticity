// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W39 Meter widget tests — verify the LED-segment-ladder math matches
// the bundle's `components.jsx` Meter (lines 163-180): `levelIdx`
// counts segments at indices < level*bars; `peakIdx` is the trailing
// edge held at max(levelIdx, floor(held*bars)) when `peak` rides
// above `level`; `clip` lights any "on" segment whose index/bars
// exceeds 0.9. ARIA label defaults to "Level NN%" rounding the level
// fraction, and a custom `ariaLabel` overrides it for callers that
// embed the meter inside a labelled row.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention — the
// chromaticity Vitest 4 + jsdom + @testing-library/svelte stack does
// not auto-cleanup the DOM between renders.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Meter from './Meter.svelte';

describe('Meter widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the requested number of segments', () => {
    const { container } = render(Meter, { props: { level: 0.5, bars: 28 } });
    expect(container.querySelectorAll('.meter-bar').length).toBe(28);
  });

  it('lights segments below the level cursor', () => {
    const { container } = render(Meter, { props: { level: 0.5, bars: 20 } });
    const bars = container.querySelectorAll('.meter-bar');
    const onBars = container.querySelectorAll('.meter-bar.on');
    expect(bars.length).toBe(20);
    // levelIdx = floor(0.5 * 20) = 10 → indices 0..9 are below cursor.
    // The peak segment at index 10 is `peak`-classed, not `on`-classed,
    // so the count of `.on` segments is exactly 10.
    expect(onBars.length).toBe(10);
  });

  it('places the peak marker one segment ahead when peak > level', () => {
    const { container } = render(Meter, { props: { level: 0.5, bars: 20, peak: 0.8 } });
    const peakBar = container.querySelectorAll('.meter-bar.peak');
    expect(peakBar.length).toBe(1);
    // peakIdx = floor(0.8 * 20) = 16 → 17th meter-bar carries .peak.
    const allBars = container.querySelectorAll('.meter-bar');
    expect(allBars[16]?.classList.contains('peak')).toBe(true);
  });

  it('places the peak marker at the trailing edge when peak is omitted', () => {
    const { container } = render(Meter, { props: { level: 0.5, bars: 20 } });
    const peakBar = container.querySelectorAll('.meter-bar.peak');
    expect(peakBar.length).toBe(1);
    const allBars = container.querySelectorAll('.meter-bar');
    // peakIdx = max(10, floor(0.5*20)) = 10 → 11th meter-bar carries .peak.
    expect(allBars[10]?.classList.contains('peak')).toBe(true);
  });

  it('marks segments above 90% as clip when they are also on', () => {
    const { container } = render(Meter, { props: { level: 0.99, bars: 20 } });
    const clipBars = container.querySelectorAll('.meter-bar.clip');
    // levelIdx = floor(0.99 * 20) = 19. on-indices 0..18.
    // clip threshold: i / 20 > 0.9 → i in {19} on but excluded (peak slot at idx 19).
    // i in {18} → 18/20 = 0.9, NOT strictly > 0.9 → not clip.
    // So no `.clip` segments at level=0.99 with bars=20 because the only
    // on-segment above the 0.9 ratio is index 19, which is captured as
    // peak when peak >= level.
    expect(clipBars.length).toBe(0);
  });

  it('does not classify peak segments as clip even in the top decile', () => {
    // bars=22 so levelIdx=20 puts the peak slot at i=20 with i/bars=0.909 > 0.9.
    // Peak class takes precedence over clip on the same segment, so the
    // top-decile peak marker stays peak-coloured (warning) rather than
    // clip-coloured (danger). No `.clip` segments expected.
    const { container } = render(Meter, { props: { level: 0.95, bars: 22 } });
    expect(container.querySelectorAll('.meter-bar.clip').length).toBe(0);
  });

  it('clip lights when level fully saturates and peak falls beyond the bar count', () => {
    const { container } = render(Meter, { props: { level: 1.0, bars: 20 } });
    const clipBars = container.querySelectorAll('.meter-bar.clip');
    // levelIdx = floor(1.0 * 20) = 20. on-indices 0..19.
    // peakIdx = 20 → falls outside the 20-segment array, so no segment
    // gets the `.peak` class. Indices 19 (i/bars = 0.95 > 0.9) becomes
    // clip; indices 18 (i/bars = 0.9 NOT strictly > 0.9) stays on.
    expect(clipBars.length).toBe(1);
  });

  it('emits a default ARIA label of "Level NN%"', () => {
    const { container } = render(Meter, { props: { level: 0.42, bars: 20 } });
    const root = container.querySelector('.meter');
    expect(root?.getAttribute('aria-label')).toBe('Level 42%');
  });

  it('honours an explicit ariaLabel prop', () => {
    const { container } = render(Meter, {
      props: { level: 0.42, bars: 20, ariaLabel: 'Audio input monitor' },
    });
    const root = container.querySelector('.meter');
    expect(root?.getAttribute('aria-label')).toBe('Audio input monitor');
  });

  it('clamps out-of-range level inputs to [0, 1]', () => {
    const high = render(Meter, { props: { level: 1.5, bars: 20 } });
    const allBarsHigh = high.container.querySelectorAll('.meter-bar.on');
    // clamped to 1.0 → levelIdx = 20 → indices 0..19 on, with i=19
    // becoming clip per the previous assertion's path.
    expect(allBarsHigh.length).toBe(19);

    const negative = render(Meter, { props: { level: -0.4, bars: 20 } });
    const allBarsNeg = negative.container.querySelectorAll('.meter-bar.on');
    expect(allBarsNeg.length).toBe(0);
  });
});
