// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * DJ-style BPM Auto-Listen & Phase Scrub / Beatmatch store.
 *
 * Explicit discriminated state machine:
 * - `auto`: tracks detected live BPM from `audioSnapshotStore` while still
 *   applying the performer's `phaseOffsetBeats` scrub (`-0.5..+0.5`) so
 *   visuals can be nudged dead-on to the kick drum.
 * - `manual`: locks tempo to `manualBpm` (ignoring detector drift during
 *   breakdowns) with tap-tempo, half/double (`0.5×` / `2×`), phase scrub,
 *   momentary nudge (`±0.05` beats), and instant downbeat resync.
 */

import { untrack } from 'svelte';

import { registerStateMachine } from '../state/diagnostics';

export type BpmControlMachine =
  | { readonly kind: 'auto' }
  | { readonly kind: 'manual'; readonly bpm: number };

const MIN_BPM = 40;
const MAX_BPM = 240;
const MIN_PHASE = -0.5;
const MAX_PHASE = 0.5;

function clampBpm(bpm: number): number {
  if (!Number.isFinite(bpm)) return 120;
  return Math.round(Math.max(MIN_BPM, Math.min(MAX_BPM, bpm)) * 10) / 10;
}

function wrapPhase(beats: number): number {
  if (!Number.isFinite(beats)) return 0;
  let v = beats;
  while (v > MAX_PHASE) v -= 1;
  while (v < MIN_PHASE) v += 1;
  return Math.round(v * 1000) / 1000;
}

class BpmControlStore {
  private machine = $state<BpmControlMachine>({ kind: 'auto' });
  private _phaseOffsetBeats = $state(0);
  private _resyncEpochMs = $state(0);
  private tapTimestamps: number[] = [];

  constructor() {
    registerStateMachine('bpmControl', () =>
      untrack(() => ({
        kind: this.machine.kind,
        bpm: this.machine.kind === 'manual' ? this.machine.bpm : null,
        phaseOffsetBeats: this._phaseOffsetBeats,
      })),
    );
  }

  get mode(): 'auto' | 'manual' {
    return this.machine.kind;
  }

  get isListening(): boolean {
    return this.machine.kind === 'auto';
  }

  get manualBpm(): number | null {
    return this.machine.kind === 'manual' ? this.machine.bpm : null;
  }

  get phaseOffsetBeats(): number {
    return this._phaseOffsetBeats;
  }

  /** Toggle BPM auto-listening ON (`auto`) or OFF (`manual`, locking `currentBpm`). */
  toggleListening(currentDetectedBpm = 120): void {
    const current = untrack(() => this.machine);
    if (current.kind === 'auto') {
      this.machine = { kind: 'manual', bpm: clampBpm(currentDetectedBpm) };
    } else {
      this.machine = { kind: 'auto' };
    }
  }

  setAuto(): void {
    this.machine = { kind: 'auto' };
  }

  setManualBpm(bpm: number): void {
    this.machine = { kind: 'manual', bpm: clampBpm(bpm) };
  }

  /** Set continuous beatmatch phase scrub in `[-0.5, +0.5]` beats. */
  setPhaseOffset(beats: number): void {
    this._phaseOffsetBeats = Math.max(
      MIN_PHASE,
      Math.min(MAX_PHASE, Math.round(beats * 1000) / 1000),
    );
  }

  /** Momentary CDJ-style platter nudge (`deltaBeats`, e.g. `-0.05` or `+0.05`). */
  nudgePhase(deltaBeats: number): void {
    const current = untrack(() => this._phaseOffsetBeats);
    this._phaseOffsetBeats = wrapPhase(current + deltaBeats);
  }

  /** Snap downbeat phase to `nowMs` immediately (resets visual beat 1 to now). */
  resyncDownbeat(nowMs = Date.now()): void {
    this._resyncEpochMs = nowMs;
    this._phaseOffsetBeats = 0;
  }

  /** Record a manual tap-tempo hit; switches to `manual` once >= 2 taps are recorded. */
  tap(nowMs = Date.now()): number | null {
    const prev = this.tapTimestamps[this.tapTimestamps.length - 1];
    if (prev !== undefined && nowMs - prev > 2500) {
      this.tapTimestamps = [];
    }
    this.tapTimestamps.push(nowMs);
    if (this.tapTimestamps.length > 6) {
      this.tapTimestamps.shift();
    }
    if (this.tapTimestamps.length < 2) {
      return null;
    }
    let sum = 0;
    for (let i = 1; i < this.tapTimestamps.length; i++) {
      sum += this.tapTimestamps[i]! - this.tapTimestamps[i - 1]!;
    }
    const avgMs = sum / (this.tapTimestamps.length - 1);
    if (avgMs <= 0) return null;
    const bpm = clampBpm(60000 / avgMs);
    this.machine = { kind: 'manual', bpm };
    return bpm;
  }

  /** Resolve the effective BPM for visuals & transport HUD. */
  effectiveBpm(detectedBpm: number): number {
    return this.machine.kind === 'manual' ? this.machine.bpm : clampBpm(detectedBpm);
  }

  /** Compute current continuous beat position `[0, ∞)` including phase scrub & downbeat resync. */
  beatPosition(nowMs: number, detectedBpm: number): number {
    const bpm = this.effectiveBpm(detectedBpm);
    const elapsedBeats = ((nowMs - this._resyncEpochMs) / 60000) * bpm;
    return elapsedBeats + this._phaseOffsetBeats;
  }

  reset(): void {
    this.machine = { kind: 'auto' };
    this._phaseOffsetBeats = 0;
    this._resyncEpochMs = 0;
    this.tapTimestamps = [];
  }
}

export const bpmControlStore = new BpmControlStore();
