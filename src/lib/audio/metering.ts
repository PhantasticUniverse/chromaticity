// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Shared audio-metering DSP primitive — the single source of truth for
 * level→display conversion + ballistics. Consumed today by the Meter
 * widget and the Settings monitoring meter; VuStereo and AudioUniforms
 * are tracked to migrate onto it (each still runs its own legacy EMA
 * until then).
 *
 * Replaces the ad-hoc, frame-rate-dependent `EMA 0.25 + 0.004/frame`
 * smoothing (previously duplicated per-widget) with one calibrated,
 * **wall-clock-dt-aware** implementation derived from named pro-audio time
 * constants, plus a logarithmic **dBFS** scale (human hearing is
 * logarithmic; a linear `[0,1]` fill crushes everything below ~-20 dBFS
 * into the bottom tenth of the bar and looks dead even with healthy
 * signal — the single highest-leverage metering fix per the world-class
 * metering design).
 *
 * This module is pure DSP — no Svelte runes, no DOM — so it is fully
 * unit-testable and reusable. Widgets instantiate the follower/peak-hold
 * classes once and call them per animation frame with the real elapsed
 * `dtMs`, so behaviour is identical on 60 / 120 / 144 Hz displays.
 *
 * # Scope (what stays out)
 *
 * `true_peak` (≥4× oversampled, dBTP), clip detection above 0 dBFS, real
 * stereo, AGC/threshold, and BPM-confidence all require an engine-side
 * `AudioUniformSnapshot` widening (the engine currently clamps a single
 * mono `rms`/`peak` to `[0,1]`, destroying over-0-dBFS information) and
 * are tracked as a separate ADR-gated wave. This module operates on the
 * `[0,1]` amplitudes available today; every dB value below 0 dBFS is
 * already correct, and the clip/over path is intentionally left to the
 * engine widening rather than faked here.
 */

/** Bottom of the meter scale (dBFS). Signal at/below this reads as empty. */
export const METER_FLOOR_DB = -60;
/** Top of the meter scale (dBFS / digital full scale). */
export const METER_CEIL_DB = 0;

/**
 * EBU R68 alignment level — the nominal "0 VU" reference, drawn as a
 * printed tick so operators have a calibrated anchor (an exceed over the
 * reference, which ships no scale legend).
 */
export const REFERENCE_TICK_DB = -18;

/** Top of the GREEN working zone. At/below this is the healthy nominal band. */
export const ZONE_GREEN_MAX_DB = -9;
/** Top of the AMBER caution zone. Above this is RED (clip/over territory). */
export const ZONE_AMBER_MAX_DB = -1;

/**
 * Floor amplitude for the dB conversion. `20·log10(0)` is `-Infinity`;
 * clamping the input to this floor keeps the math finite and maps true
 * silence to {@link METER_FLOOR_DB}. `1e-5` ≈ -100 dBFS, comfortably below
 * the -60 dBFS scale floor.
 */
const AMP_FLOOR = 1e-5;

// --- Ballistics time constants (named, from pro-audio standards) ---

/**
 * Fast-peak (Digital / Type-I PPM family) attack time constant.
 * ~1.7 ms is effectively instantaneous at any display refresh, so a
 * transient lights the peak immediately; modelled as an exponential
 * attack so it stays correct (not over-fast) on high-refresh displays.
 */
export const PPM_ATTACK_TAU_MS = 1.7;
/**
 * Fast-peak release: a LINEAR fall of 20 dB over 1.7 s (Digital/Type-I
 * PPM return time) ≈ 11.76 dB/s. Linear-in-dB, not exponential, per the
 * PPM standard.
 */
export const PPM_RELEASE_DB_PER_S = 20 / 1.7;

/**
 * RMS/loudness integration time constant for the perceived-level bar fill
 * (symmetric attack and fall-back). 300 ms gives a deliberately slow,
 * smooth loudness bar in the spirit of a VU meter — note the classic VU
 * "300 ms" spec is the ~99% RISE TIME (τ ≈ 65 ms), so this is a slower
 * loudness average by choice, not a VU-faithful τ.
 */
export const VU_INTEGRATION_TAU_MS = 300;

/** Peak-hold: how long the held marker stays stationary before decaying. */
export const PEAK_HOLD_MS = 1500;
/** Peak-hold decay once the hold expires: 20 dB over 1.7 s (matches PPM). */
export const PEAK_HOLD_DECAY_DB_PER_S = 20 / 1.7;

/** No-signal threshold: sustained level below this reads as "no signal". */
export const NO_SIGNAL_DB = METER_FLOOR_DB;
/** No-signal dwell: level must stay below {@link NO_SIGNAL_DB} this long. */
export const NO_SIGNAL_MS = 500;

/** Meter color zone, placed at real dB breakpoints (not linear position). */
export type MeterZone = 'green' | 'amber' | 'red';

/**
 * Convert a linear amplitude (`0..1`, or above for over-0-dBFS once the
 * engine stops clamping) to dBFS, clamped at the {@link AMP_FLOOR} so
 * silence maps to a finite floor rather than `-Infinity`.
 */
export function ampToDb(amp: number): number {
  const a = Math.max(Math.abs(amp), AMP_FLOOR);
  return 20 * Math.log10(a);
}

/**
 * Map a dBFS value onto the `[0,1]` meter travel using the fixed
 * {@link METER_FLOOR_DB}..{@link METER_CEIL_DB} range. `-60 dBFS → 0`,
 * `0 dBFS → 1`, clamped at both ends.
 */
export function dbToFraction(db: number): number {
  const f = (db - METER_FLOOR_DB) / (METER_CEIL_DB - METER_FLOOR_DB);
  return Math.min(1, Math.max(0, f));
}

/** Convenience: linear amplitude → `[0,1]` meter travel on the dB scale. */
export function ampToFraction(amp: number): number {
  return dbToFraction(ampToDb(amp));
}

/**
 * Inverse of {@link dbToFraction}: map a `[0,1]` position on the meter
 * travel back to its dBFS value. Used to color each segment by the dB it
 * represents (green at the bottom, amber, red at the top) rather than by
 * a linear position heuristic.
 */
export function fractionToDb(fraction: number): number {
  const f = Math.min(1, Math.max(0, fraction));
  return METER_FLOOR_DB + f * (METER_CEIL_DB - METER_FLOOR_DB);
}

/** Classify a dBFS value into its color zone at the dB breakpoints. */
export function zoneForDb(db: number): MeterZone {
  if (db >= ZONE_AMBER_MAX_DB) return 'red';
  if (db >= ZONE_GREEN_MAX_DB) return 'amber';
  return 'green';
}

/**
 * Exponential one-pole follower with a wall-clock-dt-aware coefficient
 * (`1 - exp(-dt/τ)`), so the smoothing is identical regardless of frame
 * rate. Symmetric (single τ) — used for the VU/RMS perceived-level bar.
 *
 * Values are tracked in the dB domain; the initial value is the scale
 * floor so a freshly-constructed follower reads as silence.
 */
export class ExpEnvelope {
  #current = METER_FLOOR_DB;
  readonly #tauMs: number;

  constructor(tauMs: number = VU_INTEGRATION_TAU_MS) {
    this.#tauMs = Math.max(tauMs, 1e-3);
  }

  /** Smooth toward `targetDb` over `dtMs` and return the new value (dB). */
  process(targetDb: number, dtMs: number): number {
    const dt = Math.max(dtMs, 0);
    const coeff = 1 - Math.exp(-dt / this.#tauMs);
    this.#current += (targetDb - this.#current) * coeff;
    return this.#current;
  }

  /** Current smoothed value (dB) without advancing. */
  value(): number {
    return this.#current;
  }

  /** Reset to the scale floor (silence). */
  reset(): void {
    this.#current = METER_FLOOR_DB;
  }
}

/**
 * PPM-style fast-peak ballistics: a fast exponential attack toward a
 * rising target and a LINEAR dB/s release on a falling target (the
 * Digital/Type-I PPM behaviour). dt-aware on both paths.
 */
export class PeakBallistics {
  #current = METER_FLOOR_DB;
  readonly #attackTauMs: number;
  readonly #releaseDbPerS: number;

  constructor(
    attackTauMs: number = PPM_ATTACK_TAU_MS,
    releaseDbPerS: number = PPM_RELEASE_DB_PER_S,
  ) {
    this.#attackTauMs = Math.max(attackTauMs, 1e-3);
    this.#releaseDbPerS = Math.max(releaseDbPerS, 0);
  }

  /** Advance by `dtMs` toward `targetDb`; returns the new peak value (dB). */
  process(targetDb: number, dtMs: number): number {
    const dt = Math.max(dtMs, 0);
    if (targetDb >= this.#current) {
      const coeff = 1 - Math.exp(-dt / this.#attackTauMs);
      this.#current += (targetDb - this.#current) * coeff;
    } else {
      const drop = this.#releaseDbPerS * (dt / 1000);
      this.#current = Math.max(targetDb, this.#current - drop);
    }
    return this.#current;
  }

  value(): number {
    return this.#current;
  }

  reset(): void {
    this.#current = METER_FLOOR_DB;
  }
}

/**
 * Peak-hold marker: latches the loudest recent value, holds it stationary
 * for {@link PEAK_HOLD_MS}, then decays linearly at
 * {@link PEAK_HOLD_DECAY_DB_PER_S} dB/s. Rendered as the thin cap line
 * floating above the live fill.
 */
export class PeakHold {
  #heldDb = METER_FLOOR_DB;
  #sinceUpdateMs = 0;
  readonly #holdMs: number;
  readonly #decayDbPerS: number;

  constructor(holdMs: number = PEAK_HOLD_MS, decayDbPerS: number = PEAK_HOLD_DECAY_DB_PER_S) {
    this.#holdMs = Math.max(holdMs, 0);
    this.#decayDbPerS = Math.max(decayDbPerS, 0);
  }

  /** Feed the current value; returns the held marker value (dB). */
  process(currentDb: number, dtMs: number): number {
    const dt = Math.max(dtMs, 0);
    if (currentDb >= this.#heldDb) {
      this.#heldDb = currentDb;
      this.#sinceUpdateMs = 0;
    } else {
      this.#sinceUpdateMs += dt;
      if (this.#sinceUpdateMs > this.#holdMs) {
        // Decay only the portion of this step PAST the hold window, so
        // the boundary frame stays dt-invariant (one coarse step and
        // many fine steps over the same span land identically).
        const decayMs = Math.min(dt, this.#sinceUpdateMs - this.#holdMs);
        const drop = this.#decayDbPerS * (decayMs / 1000);
        this.#heldDb = Math.max(currentDb, this.#heldDb - drop);
      }
    }
    return this.#heldDb;
  }

  value(): number {
    return this.#heldDb;
  }

  /** Operator click-to-reset clears the held marker to the floor. */
  reset(): void {
    this.#heldDb = METER_FLOOR_DB;
    this.#sinceUpdateMs = 0;
  }
}

/**
 * No-signal detector: reports `true` once the level has stayed below
 * {@link NO_SIGNAL_DB} continuously for {@link NO_SIGNAL_MS}, so the meter
 * can render the reference's whole-bar no-signal state (distinct from our
 * richer warming/ready/degraded chain pill). Any sample at/above the
 * threshold resets the dwell immediately.
 */
export class SilenceGate {
  #belowMs = 0;
  readonly #thresholdDb: number;
  readonly #durationMs: number;

  constructor(thresholdDb: number = NO_SIGNAL_DB, durationMs: number = NO_SIGNAL_MS) {
    this.#thresholdDb = thresholdDb;
    this.#durationMs = Math.max(durationMs, 0);
  }

  /** Advance by `dtMs` with the current dB level; returns whether silent. */
  update(db: number, dtMs: number): boolean {
    if (db < this.#thresholdDb) {
      this.#belowMs += Math.max(dtMs, 0);
    } else {
      this.#belowMs = 0;
    }
    return this.isSilent();
  }

  isSilent(): boolean {
    return this.#belowMs >= this.#durationMs;
  }

  reset(): void {
    this.#belowMs = 0;
  }
}

/** Format a dBFS value for the numeric readout (e.g. `-12.3` / `-∞`). */
export function formatDb(db: number): string {
  if (db <= METER_FLOOR_DB) return '-∞';
  return db.toFixed(1);
}
