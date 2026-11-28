// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Live FX Output State Machine Store (`blackout` & momentary `strobe`).
 *
 * Replaces `Perform.svelte`'s split module-level `persistedBlackoutActive` +
 * component `$state` booleans (`blackoutActive`, `strobeActive`) with a single
 * explicit discriminated union:
 *
 * - `normal`: standard scene output.
 * - `strobe`: momentary white strobe active (released on pointerup/keyup/destroy).
 * - `blackout`: latched blackout active (visually overrides strobe while tracking
 *   `strobeHeld` so releasing/toggling blackout never leaves a stuck strobe).
 *
 * Registered via `registerStateMachine('liveFx', ...)` so diagnostics and agents
 * can always inspect whether live output is blacked out or strobing.
 */

import { untrack } from 'svelte';

import { registerStateMachine } from '../state/diagnostics';

export type LiveFxMachine =
  | { readonly kind: 'normal' }
  | { readonly kind: 'strobe' }
  | { readonly kind: 'blackout'; readonly strobeHeld: boolean };

class LiveFxStore {
  private _machine = $state<LiveFxMachine>({ kind: 'normal' });

  constructor() {
    registerStateMachine('liveFx', () =>
      untrack(() => ({
        kind: this._machine.kind,
        blackoutActive: this.blackoutActive,
        strobeActive: this.strobeActive,
      })),
    );
  }

  get state(): LiveFxMachine {
    return this._machine;
  }

  get blackoutActive(): boolean {
    return this._machine.kind === 'blackout';
  }

  get strobeActive(): boolean {
    return (
      this._machine.kind === 'strobe' ||
      (this._machine.kind === 'blackout' && this._machine.strobeHeld)
    );
  }

  /** Whether the visual strobe effect is currently visible (suppressed during blackout). */
  get effectiveStrobeVisible(): boolean {
    return this._machine.kind === 'strobe';
  }

  /** Activate momentary strobe. Returns `true` iff strobe transitioned from inactive -> active. */
  activateStrobe(): boolean {
    if (this._machine.kind === 'strobe') return false;
    if (this._machine.kind === 'blackout') {
      if (this._machine.strobeHeld) return false;
      this._machine = { kind: 'blackout', strobeHeld: true };
      return true;
    }
    this._machine = { kind: 'strobe' };
    return true;
  }

  /** Release momentary strobe. Returns `true` iff strobe transitioned from active -> inactive. */
  releaseStrobe(): boolean {
    if (this._machine.kind === 'normal') return false;
    if (this._machine.kind === 'blackout') {
      if (!this._machine.strobeHeld) return false;
      this._machine = { kind: 'blackout', strobeHeld: false };
      return true;
    }
    this._machine = { kind: 'normal' };
    return true;
  }

  /** Toggle latched blackout. Returns the new `blackoutActive` state. */
  toggleBlackout(): boolean {
    if (this._machine.kind === 'blackout') {
      this._machine = this._machine.strobeHeld ? { kind: 'strobe' } : { kind: 'normal' };
      return false;
    }
    const strobeHeld = this._machine.kind === 'strobe';
    this._machine = { kind: 'blackout', strobeHeld };
    return true;
  }

  /**
   * Reset both strobe and blackout to `normal`.
   * Returns flags indicating which effects were active so callers can dispatch
   * corresponding zeroed meta-control IPC values cleanly.
   */
  reset(): { readonly releasedStrobe: boolean; readonly clearedBlackout: boolean } {
    const releasedStrobe = this.strobeActive;
    const clearedBlackout = this.blackoutActive;
    this._machine = { kind: 'normal' };
    return { releasedStrobe, clearedBlackout };
  }
}

export const liveFxStore = new LiveFxStore();
