// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * 1-click DJ Autopilot state machine store.
 *
 * Models autonomous phrase-synced visual progression as an explicit
 * discriminated union registered with the state-machine diagnostics registry:
 * - `off`: manual operator control only.
 * - `active`: phrase-interval transitions (`8 | 16 | 32` bars), optional
 *   subtle parameter drift, and configurable crossfade duration in milliseconds.
 */

import { untrack } from 'svelte';

import { registerStateMachine } from '../state/diagnostics';

export type DjAutopilotPhraseBars = 8 | 16 | 32;

export type DjAutopilotMachine =
  | { readonly kind: 'off' }
  | {
      readonly kind: 'active';
      readonly phraseIntervalBars: DjAutopilotPhraseBars;
      readonly subtleDrift: boolean;
      readonly crossfadeMs: number;
    };

export interface DjAutopilotEnableOptions {
  readonly phraseIntervalBars?: DjAutopilotPhraseBars;
  readonly subtleDrift?: boolean;
  readonly crossfadeMs?: number;
}

const DEFAULT_PHRASE_INTERVAL_BARS: DjAutopilotPhraseBars = 16;
const DEFAULT_SUBTLE_DRIFT = true;
const DEFAULT_CROSSFADE_MS = 1200;
const MIN_CROSSFADE_MS = 0;
const MAX_CROSSFADE_MS = 10_000;

function clampCrossfadeMs(ms: number): number {
  if (!Number.isFinite(ms)) return DEFAULT_CROSSFADE_MS;
  return Math.round(Math.max(MIN_CROSSFADE_MS, Math.min(MAX_CROSSFADE_MS, ms)));
}

class DjAutopilotStore {
  private machine = $state<DjAutopilotMachine>({ kind: 'off' });
  private preferredPhraseIntervalBars: DjAutopilotPhraseBars = DEFAULT_PHRASE_INTERVAL_BARS;
  private preferredSubtleDrift = DEFAULT_SUBTLE_DRIFT;
  private preferredCrossfadeMs = DEFAULT_CROSSFADE_MS;

  constructor() {
    registerStateMachine('djAutopilot', () =>
      untrack(() => ({
        kind: this.machine.kind,
        phraseIntervalBars: this.machine.kind === 'active' ? this.machine.phraseIntervalBars : null,
        subtleDrift: this.machine.kind === 'active' ? this.machine.subtleDrift : false,
        crossfadeMs: this.machine.kind === 'active' ? this.machine.crossfadeMs : 0,
      })),
    );
  }

  get state(): DjAutopilotMachine {
    return this.machine;
  }

  get isActive(): boolean {
    return this.machine.kind === 'active';
  }

  get phraseIntervalBars(): DjAutopilotPhraseBars {
    return this.machine.kind === 'active'
      ? this.machine.phraseIntervalBars
      : this.preferredPhraseIntervalBars;
  }

  get subtleDrift(): boolean {
    return this.machine.kind === 'active' ? this.machine.subtleDrift : this.preferredSubtleDrift;
  }

  get crossfadeMs(): number {
    return this.machine.kind === 'active' ? this.machine.crossfadeMs : this.preferredCrossfadeMs;
  }

  /** Toggle DJ Autopilot between `off` and `active`. */
  toggle(): void {
    const current = untrack(() => this.machine);
    if (current.kind === 'active') {
      this.disable();
    } else {
      this.enable();
    }
  }

  /** Enable DJ Autopilot with optional phrase interval, drift, and crossfade overrides. */
  enable(opts?: DjAutopilotEnableOptions): void {
    const phraseIntervalBars = opts?.phraseIntervalBars ?? this.preferredPhraseIntervalBars;
    const subtleDrift = opts?.subtleDrift ?? this.preferredSubtleDrift;
    const crossfadeMs =
      opts?.crossfadeMs !== undefined
        ? clampCrossfadeMs(opts.crossfadeMs)
        : this.preferredCrossfadeMs;

    this.preferredPhraseIntervalBars = phraseIntervalBars;
    this.preferredSubtleDrift = subtleDrift;
    this.preferredCrossfadeMs = crossfadeMs;
    this.machine = {
      kind: 'active',
      phraseIntervalBars,
      subtleDrift,
      crossfadeMs,
    };
  }

  /** Disable DJ Autopilot and return to manual control (`off`). */
  disable(): void {
    this.machine = { kind: 'off' };
  }

  /** Update the phrase interval in bars (`8 | 16 | 32`). */
  setPhraseInterval(bars: DjAutopilotPhraseBars): void {
    this.preferredPhraseIntervalBars = bars;
    const current = untrack(() => this.machine);
    if (current.kind === 'active') {
      this.machine = {
        ...current,
        phraseIntervalBars: bars,
      };
    }
  }

  /** Enable or disable subtle continuous parameter drift while autopilot is active. */
  setSubtleDrift(enabled: boolean): void {
    this.preferredSubtleDrift = enabled;
    const current = untrack(() => this.machine);
    if (current.kind === 'active') {
      this.machine = {
        ...current,
        subtleDrift: enabled,
      };
    }
  }

  /** Reset DJ Autopilot state and preferences to factory defaults. */
  reset(): void {
    this.preferredPhraseIntervalBars = DEFAULT_PHRASE_INTERVAL_BARS;
    this.preferredSubtleDrift = DEFAULT_SUBTLE_DRIFT;
    this.preferredCrossfadeMs = DEFAULT_CROSSFADE_MS;
    this.machine = { kind: 'off' };
  }
}

export const djAutopilotStore = new DjAutopilotStore();
