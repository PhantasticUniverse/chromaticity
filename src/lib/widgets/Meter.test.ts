// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Meter widget tests — the world-class rebuild renders a LOGARITHMIC
// dBFS ladder (src/lib/audio/metering.ts): `levelDb` maps to a segment
// count via `dbToFraction`, each "on" segment is colored by the dB it
// represents (green/amber/red zones), a floating peak-hold cap tracks
// `peakDb` above the fill, and `silent` renders the no-signal state.
// Expectations are computed from the metering helpers so they stay
// correct if the scale/zone breakpoints are retuned.
//
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import Meter from './Meter.svelte';
import { dbToFraction, formatDb, fractionToDb, zoneForDb } from '../audio/metering';

describe('Meter widget (dBFS)', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the requested number of segments', () => {
    const { container } = render(Meter, { props: { levelDb: -30, bars: 28 } });
    expect(container.querySelectorAll('.meter-bar').length).toBe(28);
  });

  it('lights segments up to the dB-scaled level cursor', () => {
    // -30 dBFS sits at the scale midpoint → 10 of 20 segments lit.
    const { container } = render(Meter, { props: { levelDb: -30, bars: 20 } });
    const levelIdx = Math.round(dbToFraction(-30) * 20);
    expect(levelIdx).toBe(10);
    // Segments 0..9 are "on"; index 10 carries the (level-tracking) peak cap.
    expect(container.querySelectorAll('.meter-bar.on').length).toBe(10);
  });

  it('colors each on-segment by the dB zone it represents', () => {
    const bars = 20;
    const { container } = render(Meter, { props: { levelDb: 0, bars } });
    // Full scale → every segment on, colored by its own dB position.
    let green = 0;
    let amber = 0;
    let red = 0;
    for (let i = 0; i < bars; i++) {
      const z = zoneForDb(fractionToDb((i + 1) / bars));
      if (z === 'green') green++;
      else if (z === 'amber') amber++;
      else red++;
    }
    expect(green).toBeGreaterThan(0);
    expect(red).toBeGreaterThan(0);
    expect(container.querySelectorAll('.meter-bar.green').length).toBe(green);
    expect(container.querySelectorAll('.meter-bar.amber').length).toBe(amber);
    expect(container.querySelectorAll('.meter-bar.red').length).toBe(red);
  });

  it('floats the peak-hold cap at the held dB position above the fill', () => {
    const { container } = render(Meter, { props: { levelDb: -42, peakDb: -12, bars: 20 } });
    const peakBars = container.querySelectorAll('.meter-bar.peak');
    expect(peakBars.length).toBe(1);
    const peakIdx = Math.round(dbToFraction(-12) * 20);
    const allBars = container.querySelectorAll('.meter-bar');
    expect(allBars[peakIdx]?.classList.contains('peak')).toBe(true);
  });

  it('caps at the level when peakDb is omitted', () => {
    const { container } = render(Meter, { props: { levelDb: -30, bars: 20 } });
    const levelIdx = Math.round(dbToFraction(-30) * 20);
    const allBars = container.querySelectorAll('.meter-bar');
    expect(allBars[levelIdx]?.classList.contains('peak')).toBe(true);
  });

  it('renders the no-signal state when silent', () => {
    const { container } = render(Meter, { props: { levelDb: -80, silent: true, bars: 20 } });
    expect(container.querySelector('.meter')?.classList.contains('silent')).toBe(true);
    expect(container.querySelector('.meter__no-signal')?.textContent?.trim()).toBe('no signal');
    expect(container.querySelector('.meter')?.getAttribute('aria-label')).toBe('No signal');
  });

  it('shows the numeric dB readout when requested', () => {
    const { container } = render(Meter, {
      props: { levelDb: -12.3, peakDb: -6, bars: 20, showReadout: true },
    });
    const readout = container.querySelector('.meter__readout');
    expect(readout).not.toBeNull();
    expect(container.querySelector('.meter__db')?.textContent?.trim()).toBe(formatDb(-12.3));
  });

  it('invokes onResetPeak when the reset affordance is clicked', async () => {
    const onResetPeak = vi.fn();
    const { container } = render(Meter, {
      props: { levelDb: -6, peakDb: -3, bars: 20, showReadout: true, onResetPeak },
    });
    const reset = container.querySelector('.meter__reset') as HTMLButtonElement | null;
    expect(reset).not.toBeNull();
    if (reset) await fireEvent.click(reset);
    expect(onResetPeak).toHaveBeenCalledOnce();
  });

  it('defaults the ARIA label to the dBFS level', () => {
    const { container } = render(Meter, { props: { levelDb: -12.3, bars: 20 } });
    expect(container.querySelector('.meter')?.getAttribute('aria-label')).toBe(
      `Level ${formatDb(-12.3)} dBFS`,
    );
  });

  it('honours an explicit ariaLabel prop', () => {
    const { container } = render(Meter, {
      props: { levelDb: -12, bars: 20, ariaLabel: 'Audio input monitor' },
    });
    expect(container.querySelector('.meter')?.getAttribute('aria-label')).toBe(
      'Audio input monitor',
    );
  });
});
