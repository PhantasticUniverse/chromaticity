// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane B.2 — transport clock store.
//
// Module-singleton `$state`-backed store wrapping `performance.now()` to
// drive the top-bar transport HUD timecode + beat-position math. Mirrors
// the `performVariantStore` Lane B.1 pattern: class instance exported as
// a module-level singleton so any number of components (or stores) can
// read `transportStore.elapsedMs` and re-render reactively on tick.
//
// **Why a store rather than per-widget rAF.** The existing `Timecode`
// and `BeatDots` widgets each run their own rAF tick — fine for the
// frozen-frame bundle port, but Lane B.2 needs a single shared clock
// that the TopBar timecode chip, the TopBar beat dots, and any future
// Wave 2.2 transport consumers (record-button arming countdown, GO LIVE
// activation timestamp, playlist cue cross-fade timer) all read from.
// A shared clock also keeps the beat-dot pulse phase-locked to the
// timecode readout the operator sees — running two independent rAF
// loops can drift fractionally over long sessions.
//
// **Tick cadence.** The Wave 2.1 plan specifies a 30 Hz drive. We tick
// on rAF (~60 Hz on a healthy display, throttled by the browser) and
// derive both `elapsedMs` and the HH:MM:SS:FF formatted helper directly
// from the rAF timestamp delta — the FF subfield rounds to a 30-fps
// frame index regardless of the underlying tick rate, matching the
// bundle's `(s * 30) % 30` derivation. When the engine emits real
// transport timecode events (Wave 2.2+), the producer swaps; the API
// (`start()` / `stop()` / `reset()` / `timecode()` / `elapsedMs`) stays.
//
// **Reduced motion.** Tests under jsdom + `prefers-reduced-motion:
// reduce` short-circuit the tick to leave `elapsedMs` at its current
// value. The visible-UI consumers (`Timecode`-style format, beat-dot
// pulse) each apply their own reduced-motion gate where appropriate.
//
// **`.svelte.ts` extension** is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention.

const FRAMES_PER_SECOND = 30;

class TransportStore {
  /** Milliseconds since `start()` was first called (or since the last
   * `reset()`). Drives all derived timecode displays + beat-position
   * math in the top bar. */
  elapsedMs: number = $state(0);

  /** Whether the rAF tick is currently attached. Idempotent gate for
   * `start()` so multiple components can call `start()` on mount
   * without leaking rAF handles. */
  started: boolean = $state(false);

  /** rAF handle for the tick loop; tracked so `stop()` can cancel. */
  #raf: number | null = null;

  /** `performance.now()` reading from the most-recent `start()` call,
   * minus the previously-accumulated `elapsedMs` so a pause/resume
   * cycle preserves the count without resetting. */
  #startTime: number = 0;

  /**
   * Begin ticking the transport clock. Idempotent — subsequent calls
   * while `started` is true are no-ops. Tick runs via
   * `requestAnimationFrame`; under jsdom without rAF stubs the tick
   * never fires and `elapsedMs` stays at its prior value.
   */
  start(): void {
    if (this.started) return;
    if (typeof window === 'undefined' || typeof requestAnimationFrame === 'undefined') {
      // Headless / SSR — flag started so subsequent stop()/start() pairs
      // converge, but skip the rAF wiring.
      this.started = true;
      return;
    }
    this.started = true;
    // Anchor against `performance.now() - elapsedMs` so a start after a
    // stop resumes from where the clock paused rather than re-zeroing.
    this.#startTime = performance.now() - this.elapsedMs;
    const tick = (now: number): void => {
      this.elapsedMs = now - this.#startTime;
      this.#raf = requestAnimationFrame(tick);
    };
    this.#raf = requestAnimationFrame(tick);
  }

  /**
   * Halt the rAF tick. Idempotent — calls when `started` is false are
   * no-ops. `elapsedMs` retains its last value so a subsequent
   * `start()` resumes from the paused position.
   */
  stop(): void {
    if (!this.started) return;
    this.started = false;
    if (this.#raf !== null && typeof cancelAnimationFrame !== 'undefined') {
      cancelAnimationFrame(this.#raf);
    }
    this.#raf = null;
  }

  /**
   * Zero the accumulated `elapsedMs`. Safe to call while running — the
   * next tick re-anchors against the current `performance.now()`. Also
   * safe to call while stopped — the next `start()` begins from zero.
   */
  reset(): void {
    this.elapsedMs = 0;
    if (this.started && typeof performance !== 'undefined') {
      this.#startTime = performance.now();
    }
  }

  /**
   * Format the current `elapsedMs` as `HH:MM:SS:FF` (FF = frame index
   * within the current second at 30 fps). Mirrors the bundle's
   * `Timecode` widget formatter (`components.jsx` lines 371-388):
   *   pad(s / 3600) : pad((s / 60) % 60) : pad(s % 60) : pad((s * 30) % 30)
   * Returns a freshly-computed string each call; callers that want
   * reactive updates can wrap in `$derived(transportStore.timecode())`.
   */
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
