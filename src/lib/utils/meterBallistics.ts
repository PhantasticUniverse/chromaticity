// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Pure OBS-grade logarithmic dBFS meter conversion and fast-attack /
 * calibrated-release + peak-hold ballistics for live broadcast metering.
 */

export interface MeterBallisticsState {
  level: number;
  peak: number;
  peakHoldUntilMs: number;
}

/** OBS Studio default falloff: ~24 dB/s over a 60 dB dynamic range. */
export const METER_RELEASE_RATIO_PER_SEC = 0.42;

/** Calibrated peak-hold indicator falloff after hold expires. */
export const METER_PEAK_RELEASE_RATIO_PER_SEC = 0.28;

/** Peak-hold dwell duration in milliseconds. */
export const METER_PEAK_HOLD_MS = 800;

/**
 * Convert linear amplitude `[0, 1]` to OBS-style normalized dBFS meter ratio
 * `[0, 1]` spanning `-60 dBFS..0 dBFS`. Non-finite or sub-floor values return `0`.
 */
export function linearToMeterRatio(linear: number): number {
  if (!Number.isFinite(linear) || linear <= 0.001) return 0;
  const db = 20 * Math.log10(Math.min(1, linear));
  return Math.max(0, Math.min(1, (db + 60) / 60));
}

/**
 * Advance one meter channel's ballistics state by `dtSec` toward `targetRatio`
 * (`[0, 1]`) at timestamp `nowMs`. Pure state update returning a new snapshot.
 */
export function stepMeterBallistics(
  prev: MeterBallisticsState | undefined,
  targetRatio: number,
  dtSec: number,
  nowMs: number,
): MeterBallisticsState {
  const safeTarget = Number.isFinite(targetRatio) ? Math.max(0, Math.min(1, targetRatio)) : 0;
  const safeDt = Number.isFinite(dtSec) ? Math.min(0.1, Math.max(0.001, dtSec)) : 0.016;
  const safeNowMs = Number.isFinite(nowMs) ? nowMs : 0;
  const current = prev ?? { level: 0, peak: 0, peakHoldUntilMs: 0 };

  const releaseStep = METER_RELEASE_RATIO_PER_SEC * safeDt;
  const peakReleaseStep = METER_PEAK_RELEASE_RATIO_PER_SEC * safeDt;

  let nextLevel: number;
  if (safeTarget >= current.level) {
    // Frame-rate-independent fast attack (~12 ms time constant) so transients punch immediately.
    const attackAlpha = 1 - Math.exp(-safeDt / 0.012);
    nextLevel = current.level + (safeTarget - current.level) * attackAlpha;
  } else {
    nextLevel = Math.max(safeTarget, current.level - releaseStep);
  }

  let nextPeak = current.peak;
  let nextHoldUntil = current.peakHoldUntilMs;
  if (nextLevel >= nextPeak) {
    nextPeak = nextLevel;
    nextHoldUntil = safeNowMs + METER_PEAK_HOLD_MS;
  } else if (safeNowMs >= nextHoldUntil) {
    nextPeak = Math.max(nextLevel, nextPeak - peakReleaseStep);
  }

  return {
    level: nextLevel,
    peak: nextPeak,
    peakHoldUntilMs: nextHoldUntil,
  };
}
