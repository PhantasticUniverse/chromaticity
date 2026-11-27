// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane B.2 — transport clock store, upgraded to an explicit
// `TransportState` (`'stopped' | 'running'`).

import { untrack } from 'svelte';

import { registerStateMachine } from '../state';

const FRAMES_PER_SECOND = 30;

export type TransportState = 'stopped' | 'running';

class TransportStore {
  /** Milliseconds since `start()` was first called (or since the last `reset()`). */
  elapsedMs: number = $state(0);

  /** Explicit discriminated clock state. */
  machine: TransportState = $state('stopped');

  #raf: number | null = null;
  #startTime: number = 0;

  /** Back-compat projection: `true` iff the transport clock is running. */
  get started(): boolean {
    return this.machine === 'running';
  }

  start(): void {
    if (untrack(() => this.machine) === 'running') return;
    this.machine = 'running';
    if (typeof window === 'undefined' || typeof requestAnimationFrame === 'undefined') {
      return;
    }
    this.#startTime = performance.now() - untrack(() => this.elapsedMs);
    const tick = (now: number): void => {
      if (untrack(() => this.machine) !== 'running') return;
      this.elapsedMs = now - this.#startTime;
      this.#raf = requestAnimationFrame(tick);
    };
    this.#raf = requestAnimationFrame(tick);
  }

  stop(): void {
    if (untrack(() => this.machine) === 'stopped') return;
    this.machine = 'stopped';
    if (this.#raf !== null && typeof cancelAnimationFrame !== 'undefined') {
      cancelAnimationFrame(this.#raf);
    }
    this.#raf = null;
  }

  reset(): void {
    this.elapsedMs = 0;
    if (this.machine === 'running' && typeof performance !== 'undefined') {
      this.#startTime = performance.now();
    }
  }

  timecode(): string {
    const seconds = this.elapsedMs / 1000;
    const hh = pad(Math.floor(seconds / 3600));
    const mm = pad(Math.floor((seconds / 60) % 60));
    const ss = pad(Math.floor(seconds % 60));
    const ff = pad(Math.floor((seconds * FRAMES_PER_SECOND) % FRAMES_PER_SECOND));
    return `${hh}:${mm}:${ss}:${ff}`;
  }
}

function pad(n: number): string {
  return String(n).padStart(2, '0');
}

export const transportStore = new TransportStore();

registerStateMachine('transport', () => ({
  kind: transportStore.machine,
  elapsedMs: Math.round(transportStore.elapsedMs),
}));
