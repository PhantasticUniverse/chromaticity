// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { describe, expect, it } from 'vitest';

import { buildLayerCssFilter } from './liveVideoFx';
import {
  linearToMeterRatio,
  METER_PEAK_HOLD_MS,
  METER_RELEASE_RATIO_PER_SEC,
  stepMeterBallistics,
} from './meterBallistics';
import type { MixLayer } from '../stores/videoMixer.svelte';

describe('meterBallistics', () => {
  it('maps linear amplitudes to OBS-style logarithmic dBFS ratios (-60..0 dBFS)', () => {
    expect(linearToMeterRatio(0)).toBe(0);
    expect(linearToMeterRatio(-0.5)).toBe(0);
    expect(linearToMeterRatio(Number.NaN)).toBe(0);
    expect(linearToMeterRatio(0.001)).toBe(0);
    // -20 dBFS (0.1 linear) => (-20 + 60) / 60 = 0.6667
    expect(linearToMeterRatio(0.1)).toBeCloseTo(2 / 3, 3);
    // -6 dBFS (~0.5012 linear) => ~0.90
    expect(linearToMeterRatio(0.5012)).toBeCloseTo(0.9, 2);
    // 0 dBFS (1.0 linear) and overshoot clamp to 1.0
    expect(linearToMeterRatio(1.0)).toBe(1);
    expect(linearToMeterRatio(2.5)).toBe(1);
  });

  it('attacks quickly, holds peak for 800ms, and decays smoothly on silence', () => {
    const t0 = 1000;
    const attacked = stepMeterBallistics(undefined, 0.9, 0.016, t0);
    expect(attacked.level).toBeCloseTo(0.9 * 0.85, 5);
    expect(attacked.peak).toBeCloseTo(attacked.level, 5);
    expect(attacked.peakHoldUntilMs).toBe(t0 + METER_PEAK_HOLD_MS);

    // During peak hold (t0 + 100ms), level decays at 0.42/s while peak stays pinned
    const dt = 0.1;
    const held = stepMeterBallistics(attacked, 0, dt, t0 + 100);
    expect(held.level).toBeCloseTo(attacked.level - METER_RELEASE_RATIO_PER_SEC * dt, 5);
    expect(held.peak).toBe(attacked.peak);

    // After peak hold expires (t0 + 900ms), peak begins decaying
    const released = stepMeterBallistics(held, 0, dt, t0 + 900);
    expect(released.peak).toBeLessThan(held.peak);
    expect(released.level).toBeLessThan(held.level);
  });
});

describe('buildLayerCssFilter', () => {
  const baseLayer: MixLayer = {
    id: 'layer-1',
    name: 'Test Layer',
    kind: 'webcam',
    visible: true,
    locked: false,
    solo: false,
    opacity: 1,
    blendMode: 'normal',
    transform: { x: 0, y: 0, scaleX: 1, scaleY: 1, rotationDeg: 0 },
    crop: { top: 0, right: 0, bottom: 0, left: 0 },
    filters: [],
  };

  it('returns none when no enabled visual filters are attached', () => {
    expect(buildLayerCssFilter(baseLayer, 0)).toBe('none');
  });

  it('combines color grade, glow, and audio-reactive glow into valid CSS filters', () => {
    const fxLayer: MixLayer = {
      ...baseLayer,
      filters: [
        {
          kind: 'colorGrade',
          enabled: true,
          brightness: 1.2,
          contrast: 1.1,
          saturation: 1.4,
          hue: 45,
          gamma: 1,
          invert: 1,
          sepia: 0,
          grayscale: 0,
        },
        {
          kind: 'stylize',
          enabled: true,
          blur: 2,
          bloom: 0.5,
          vignette: 0,
          scanlines: 0,
          chromaticAberration: 0,
          mirror: 'none',
        },
        {
          kind: 'audioLink',
          enabled: true,
          source: 'bass',
          target: 'glow',
          amount: 1,
        },
      ],
    };

    const quiet = buildLayerCssFilter(fxLayer, 0);
    const loud = buildLayerCssFilter(fxLayer, 1);
    expect(quiet).toContain('brightness(1.2)');
    expect(quiet).toContain('contrast(1.1)');
    expect(quiet).toContain('saturate(1.4)');
    expect(quiet).toContain('hue-rotate(45deg)');
    expect(quiet).toContain('blur(2px)');
    expect(quiet).toContain('invert(1)');
    expect(loud).not.toBe(quiet);
  });
});
