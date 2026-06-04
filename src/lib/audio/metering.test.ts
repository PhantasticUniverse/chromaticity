// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

import { describe, expect, it } from 'vitest';

import {
  ampToDb,
  ampToFraction,
  AttackReleaseEnvelope,
  dbToFraction,
  ExpEnvelope,
  formatDb,
  METER_CEIL_DB,
  METER_FLOOR_DB,
  PeakBallistics,
  PeakHold,
  PPM_RELEASE_DB_PER_S,
  SilenceGate,
  zoneForDb,
  ZONE_AMBER_MAX_DB,
  ZONE_GREEN_MAX_DB,
  ZONE_ORANGE_MAX_DB,
} from './metering';

describe('dBFS conversion', () => {
  it('maps full scale, half amplitude, and a decade correctly', () => {
    expect(ampToDb(1.0)).toBeCloseTo(0, 6);
    expect(ampToDb(0.5)).toBeCloseTo(-6.0206, 3);
    expect(ampToDb(0.1)).toBeCloseTo(-20, 6);
  });

  it('clamps silence to a finite floor instead of -Infinity', () => {
    const db = ampToDb(0);
    expect(Number.isFinite(db)).toBe(true);
    expect(db).toBeLessThanOrEqual(METER_FLOOR_DB);
  });

  it('treats negative amplitude by magnitude', () => {
    expect(ampToDb(-0.5)).toBeCloseTo(ampToDb(0.5), 6);
  });
});

describe('dbToFraction', () => {
  it('maps the fixed -60..0 dBFS range onto [0,1]', () => {
    expect(dbToFraction(METER_FLOOR_DB)).toBeCloseTo(0, 6);
    expect(dbToFraction(METER_CEIL_DB)).toBeCloseTo(1, 6);
    expect(dbToFraction(-30)).toBeCloseTo(0.5, 6);
  });

  it('clamps out-of-range values', () => {
    expect(dbToFraction(-120)).toBe(0);
    expect(dbToFraction(6)).toBe(1);
  });

  it('ampToFraction composes conversion + scale (a quiet signal is still readable)', () => {
    // -20 dBFS (amp 0.1) sits at 2/3 up the bar, NOT crushed into the
    // bottom tenth the way a linear [0,1] fill would render it.
    expect(ampToFraction(0.1)).toBeCloseTo(40 / 60, 5);
  });
});

describe('zoneForDb breakpoints', () => {
  it('places zones at the DAW reference grid (green/amber/orange/red), not linear positions', () => {
    // green ≤ -12, amber -12..-6, orange -6..-3, red ≥ -3.
    expect(zoneForDb(-20)).toBe('green'); // -18 dBFS nominal sits in green
    expect(zoneForDb(ZONE_GREEN_MAX_DB - 0.01)).toBe('green');
    expect(zoneForDb(ZONE_GREEN_MAX_DB)).toBe('amber');
    expect(zoneForDb(ZONE_AMBER_MAX_DB - 0.01)).toBe('amber');
    expect(zoneForDb(ZONE_AMBER_MAX_DB)).toBe('orange');
    expect(zoneForDb(ZONE_ORANGE_MAX_DB - 0.01)).toBe('orange');
    expect(zoneForDb(ZONE_ORANGE_MAX_DB)).toBe('red');
    expect(zoneForDb(0)).toBe('red');
  });
});

describe('ExpEnvelope', () => {
  it('converges toward the target', () => {
    const env = new ExpEnvelope(100);
    let v = METER_FLOOR_DB;
    for (let i = 0; i < 50; i++) v = env.process(-6, 16);
    expect(v).toBeCloseTo(-6, 1);
  });

  it('is dt-invariant: one big step equals many small steps over the same span', () => {
    const big = new ExpEnvelope(300);
    const small = new ExpEnvelope(300);
    big.process(0, 100);
    for (let i = 0; i < 10; i++) small.process(0, 10);
    expect(big.value()).toBeCloseTo(small.value(), 6);
  });

  it('resets to the floor', () => {
    const env = new ExpEnvelope(50);
    env.process(0, 100);
    env.reset();
    expect(env.value()).toBe(METER_FLOOR_DB);
  });
});

describe('AttackReleaseEnvelope', () => {
  // Helper: percent of the way from the seeded floor to the target.
  const pctToTarget = (current: number, from: number, target: number): number =>
    (current - from) / (target - from);

  it('rises to >90% of a step-up within ~30 ms (fast attack)', () => {
    const env = new AttackReleaseEnvelope(10, 250);
    const from = env.value();
    const target = 0; // 0 dBFS step-up from the -60 dBFS floor
    // 30 ms in ~2 frames at 60 Hz.
    let v = env.process(target, 16.7);
    v = env.process(target, 13.3); // 30 ms total elapsed
    expect(pctToTarget(v, from, target)).toBeGreaterThan(0.9);
  });

  it('falls slowly on a step-down (~250 ms tau, not <100 ms)', () => {
    const env = new AttackReleaseEnvelope(10, 250);
    // Seed at the top with a quick attack.
    for (let i = 0; i < 10; i++) env.process(0, 16.7);
    const start = env.value();
    expect(start).toBeCloseTo(0, 1);
    // Step the target down to the floor; after ~100 ms it must NOT yet
    // have crossed the halfway point (a <100 ms tau would have).
    const target = METER_FLOOR_DB;
    let v = start;
    for (let i = 0; i < 6; i++) v = env.process(target, 16.7); // ~100 ms
    const fallen = pctToTarget(v, start, target);
    // 1 - exp(-100/250) ≈ 0.33 < 0.5 (a 100 ms tau would give ≈ 0.63).
    expect(fallen).toBeLessThan(0.5);
    expect(fallen).toBeGreaterThan(0.2);
  });

  it('is dt-invariant: same wall-clock → same level across 16.7 vs 8.3 ms steps', () => {
    const coarse = new AttackReleaseEnvelope(10, 250);
    const fine = new AttackReleaseEnvelope(10, 250);
    // ~100 ms of attack toward the same target on both, different dt.
    for (let i = 0; i < 6; i++) coarse.process(0, 16.7);
    for (let i = 0; i < 12; i++) fine.process(0, 8.35);
    expect(coarse.value()).toBeCloseTo(fine.value(), 3);
  });

  it('resets to the floor', () => {
    const env = new AttackReleaseEnvelope();
    env.process(0, 100);
    env.reset();
    expect(env.value()).toBe(METER_FLOOR_DB);
  });
});

describe('PeakBallistics', () => {
  it('attacks fast onto a transient', () => {
    const p = new PeakBallistics();
    // A single ~frame step should already be essentially at the target
    // (1.7 ms attack tau vs a 16 ms frame).
    const v = p.process(-3, 16);
    expect(v).toBeGreaterThan(-3.5);
  });

  it('releases linearly at the PPM dB/s rate, dt-invariant', () => {
    const oneStep = new PeakBallistics();
    const manySteps = new PeakBallistics();
    oneStep.process(0, 100); // attack to ~0
    manySteps.process(0, 100);
    // release for 1 s
    oneStep.process(METER_FLOOR_DB, 1000);
    for (let i = 0; i < 100; i++) manySteps.process(METER_FLOOR_DB, 10);
    expect(oneStep.value()).toBeCloseTo(-PPM_RELEASE_DB_PER_S, 1);
    expect(oneStep.value()).toBeCloseTo(manySteps.value(), 4);
  });

  it('does not release past the target', () => {
    const p = new PeakBallistics();
    p.process(-10, 100);
    const v = p.process(-12, 10000); // huge dt, but target is only -12
    expect(v).toBeCloseTo(-12, 6);
  });
});

describe('PeakHold', () => {
  it('latches the loudest value and holds it stationary during the hold window', () => {
    const h = new PeakHold(1500, PPM_RELEASE_DB_PER_S);
    h.process(-10, 16);
    h.process(-40, 16); // within hold window
    expect(h.value()).toBeCloseTo(-10, 6);
  });

  it('decays only over the post-hold overshoot at the configured dB/s', () => {
    const h = new PeakHold(1500, PPM_RELEASE_DB_PER_S);
    h.process(-10, 16); // latch -10
    // A 1600 ms step crosses the 1500 ms hold by 100 ms; only that 100 ms
    // of overshoot decays (the boundary frame is dt-invariant).
    const v = h.process(-40, 1600);
    expect(v).toBeCloseTo(-10 - PPM_RELEASE_DB_PER_S * 0.1, 2);
  });

  it('decay is dt-invariant across the hold boundary', () => {
    const big = new PeakHold(1500, PPM_RELEASE_DB_PER_S);
    const small = new PeakHold(1500, PPM_RELEASE_DB_PER_S);
    big.process(-10, 16);
    small.process(-10, 16);
    // 2 s of silence after the latch (500 ms past the hold) — one coarse
    // step must equal 200 fine steps over the same span.
    big.process(-60, 2000);
    for (let i = 0; i < 200; i++) small.process(-60, 10);
    expect(big.value()).toBeCloseTo(small.value(), 2);
  });

  it('click-to-reset clears the held marker', () => {
    const h = new PeakHold();
    h.process(-3, 16);
    h.reset();
    expect(h.value()).toBe(METER_FLOOR_DB);
  });
});

describe('SilenceGate', () => {
  it('latches no-signal only after the dwell elapses', () => {
    const g = new SilenceGate(-60, 500);
    expect(g.update(-70, 400)).toBe(false);
    expect(g.update(-70, 200)).toBe(true); // 600 ms total below
  });

  it('resets the dwell the instant signal returns', () => {
    const g = new SilenceGate(-60, 500);
    g.update(-70, 600);
    expect(g.isSilent()).toBe(true);
    expect(g.update(-10, 16)).toBe(false);
  });
});

describe('formatDb', () => {
  it('renders the floor as -∞ and otherwise one decimal place', () => {
    expect(formatDb(METER_FLOOR_DB)).toBe('-∞');
    expect(formatDb(-120)).toBe('-∞');
    expect(formatDb(-12.34)).toBe('-12.3');
    expect(formatDb(0)).toBe('0.0');
  });
});
