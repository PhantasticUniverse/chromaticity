// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Pure, deterministic visual modulation graph evaluator (Phase 6).
// Zero DOM or Svelte dependencies so it can be unit-tested directly
// and stepped deterministically from any clock source.

export type ModSourceId =
  | 'audio.rms'
  | 'audio.peak'
  | 'audio.bass'
  | 'audio.mid'
  | 'audio.high'
  | 'audio.bpm'
  | 'lfo.sine'
  | 'lfo.triangle'
  | 'lfo.saw';

export type ModOp = 'passthrough' | 'gain' | 'invert' | 'smooth' | 'quantize';

export interface ModRoute {
  readonly id: string;
  readonly source: ModSourceId;
  /** Target shader control slot index (`0..7`). */
  readonly targetControlIndex: number;
  readonly op: ModOp;
  readonly amount: number;
  readonly bias: number;
  /** EMA smoothing factor in `[0, 0.99]`; higher values retain more history. */
  readonly smoothing: number;
  readonly enabled: boolean;
}

export interface ModAudioInputs {
  readonly rms: number;
  readonly peak: number;
  readonly bass: number;
  readonly mid: number;
  readonly high: number;
  /** Raw tempo in BPM (normalized against `BPM_NORM_CEILING` when sampled). */
  readonly bpm: number;
}

export interface ModFrameInputs {
  readonly elapsedSecs: number;
  /** LFO frequency in Hz (defaults to `DEFAULT_LFO_RATE_HZ`). */
  readonly lfoRateHz?: number;
  readonly audio?: Partial<ModAudioInputs>;
}

export interface ModulationFrameResult {
  /** Final `[0, 1]`-clamped output per targeted control index (`0..7`). */
  readonly outputs: Map<number, number>;
  /** Updated per-route EMA state to feed into the next frame. */
  readonly nextState: Map<string, number>;
  /** Per-route `[0, 1]`-clamped contribution for live UI readouts. */
  readonly routeValues: Map<string, number>;
}

/** Structural subset of `AudioUniformSnapshotPayload` to avoid IPC coupling. */
export interface AudioSnapshotLike {
  readonly bpm: number;
  readonly rms: number;
  readonly peak: number;
  readonly fft_bands: readonly number[];
}

export const MOD_SOURCES: readonly ModSourceId[] = [
  'audio.rms',
  'audio.peak',
  'audio.bass',
  'audio.mid',
  'audio.high',
  'audio.bpm',
  'lfo.sine',
  'lfo.triangle',
  'lfo.saw',
] as const;

export const MOD_OPS: readonly ModOp[] = [
  'passthrough',
  'gain',
  'invert',
  'smooth',
  'quantize',
] as const;

export const CONTROL_INDEX_MIN = 0;
export const CONTROL_INDEX_MAX = 7;
export const DEFAULT_LFO_RATE_HZ = 0.5;
export const BPM_NORM_CEILING = 240;
export const QUANTIZE_STEPS = 4;
const DEFAULT_SMOOTH_FACTOR = 0.5;
const MAX_SMOOTH_FACTOR = 0.99;

export function clamp01(value: number): number {
  if (!Number.isFinite(value)) return 0;
  if (value <= 0) return 0;
  if (value >= 1) return 1;
  return value;
}

/** Extract normalized band + energy inputs from an engine audio snapshot. */
export function extractAudioInputs(snapshot: AudioSnapshotLike | null): ModAudioInputs {
  if (snapshot === null) {
    return { rms: 0, peak: 0, bass: 0, mid: 0, high: 0, bpm: 0 };
  }
  const bands = snapshot.fft_bands;
  const band = (i: number): number => clamp01(bands[i] ?? 0);
  const bass = (band(0) + band(1)) / 2;
  const mid = (band(2) + band(3) + band(4)) / 3;
  const high = (band(5) + band(6) + band(7)) / 3;
  return {
    rms: clamp01(snapshot.rms),
    peak: clamp01(snapshot.peak),
    bass: clamp01(bass),
    mid: clamp01(mid),
    high: clamp01(high),
    bpm: Number.isFinite(snapshot.bpm) && snapshot.bpm > 0 ? snapshot.bpm : 0,
  };
}

/** Wrap a phase value deterministically into `[0, 1)`. */
function wrapPhase(cycles: number): number {
  if (!Number.isFinite(cycles)) return 0;
  const mod = cycles % 1;
  return mod < 0 ? mod + 1 : mod;
}

/** Sample a single modulation source into `[0, 1]`. */
export function sampleModSource(source: ModSourceId, inputs: ModFrameInputs): number {
  const audio = inputs.audio;
  const rateHz =
    inputs.lfoRateHz !== undefined && Number.isFinite(inputs.lfoRateHz) && inputs.lfoRateHz > 0
      ? inputs.lfoRateHz
      : DEFAULT_LFO_RATE_HZ;
  const phase = wrapPhase(inputs.elapsedSecs * rateHz);

  switch (source) {
    case 'audio.rms':
      return clamp01(audio?.rms ?? 0);
    case 'audio.peak':
      return clamp01(audio?.peak ?? 0);
    case 'audio.bass':
      return clamp01(audio?.bass ?? 0);
    case 'audio.mid':
      return clamp01(audio?.mid ?? 0);
    case 'audio.high':
      return clamp01(audio?.high ?? 0);
    case 'audio.bpm':
      return clamp01((audio?.bpm ?? 0) / BPM_NORM_CEILING);
    case 'lfo.sine':
      return clamp01(0.5 + 0.5 * Math.sin(phase * 2 * Math.PI));
    case 'lfo.triangle':
      return clamp01(1 - Math.abs(2 * phase - 1));
    case 'lfo.saw':
      return clamp01(phase);
  }
}

function isValidControlIndex(index: number): boolean {
  return Number.isInteger(index) && index >= CONTROL_INDEX_MIN && index <= CONTROL_INDEX_MAX;
}

/**
 * Deterministically evaluate all enabled modulation routes for one frame.
 *
 * - Multiple routes targeting the same `targetControlIndex` are summed and
 *   clamped to `[0, 1]`.
 * - EMA smoothing state is threaded purely via `prevState -> nextState`.
 */
export function evaluateModulationGraph(
  routes: readonly ModRoute[],
  inputs: ModFrameInputs,
  prevState: ReadonlyMap<string, number>,
): ModulationFrameResult {
  const rawSums = new Map<number, number>();
  const nextState = new Map<string, number>();
  const routeValues = new Map<string, number>();

  for (const route of routes) {
    if (!route.enabled) continue;
    if (!isValidControlIndex(route.targetControlIndex)) continue;

    const raw = sampleModSource(route.source, inputs);
    const prev = prevState.get(route.id);

    // Apply EMA smoothing when the route uses the `smooth` op or specifies a
    // positive smoothing factor.
    let signal = raw;
    const smoothFactor =
      route.op === 'smooth'
        ? route.smoothing > 0
          ? Math.min(route.smoothing, MAX_SMOOTH_FACTOR)
          : DEFAULT_SMOOTH_FACTOR
        : route.smoothing > 0
          ? Math.min(route.smoothing, MAX_SMOOTH_FACTOR)
          : 0;

    if (smoothFactor > 0) {
      signal = prev === undefined ? raw : prev * smoothFactor + raw * (1 - smoothFactor);
      nextState.set(route.id, signal);
    }

    let evaluated: number;
    switch (route.op) {
      case 'passthrough':
        evaluated = signal + route.bias;
        break;
      case 'gain':
        evaluated = signal * route.amount + route.bias;
        break;
      case 'invert':
        evaluated = (1 - signal) * route.amount + route.bias;
        break;
      case 'smooth':
        evaluated = signal * route.amount + route.bias;
        break;
      case 'quantize': {
        const quantized = Math.round(clamp01(signal) * QUANTIZE_STEPS) / QUANTIZE_STEPS;
        evaluated = quantized * route.amount + route.bias;
        break;
      }
    }

    routeValues.set(route.id, clamp01(evaluated));
    const currentSum = rawSums.get(route.targetControlIndex) ?? 0;
    rawSums.set(route.targetControlIndex, currentSum + evaluated);
  }

  const outputs = new Map<number, number>();
  for (const [controlIndex, sum] of rawSums.entries()) {
    outputs.set(controlIndex, clamp01(sum));
  }

  return { outputs, nextState, routeValues };
}
