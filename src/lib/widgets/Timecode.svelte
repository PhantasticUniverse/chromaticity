<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Shared-primitive backfill — `Timecode` from
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
  // lines 371-388. Counts up from zero in `HH:MM:SS:FF` format (FF =
  // frames at 30 fps, mirroring the bundle's `s * 30 % 30` derivation),
  // dimming the colon separators for readability.
  //
  // Bundle parity:
  //   const pad = (n) => String(Math.floor(n)).padStart(2, "0");
  //   const hh = pad(s / 3600);
  //   const mm = pad((s / 60) % 60);
  //   const ss = pad(s % 60);
  //   const ff = pad((s * 30) % 30);
  //
  // Frozen-frame contract: synthetic timecode driven by
  // `requestAnimationFrame` against `performance.now()`, no IPC, no
  // engine-pacing dependency. `running={false}` freezes the readout
  // without resetting it (per the bundle's `if (!running) return`
  // early-out — `s` keeps its previous value but the rAF loop is
  // detached). `prefers-reduced-motion: reduce` short-circuits the
  // tick to a static `00:00:00:00` so the display does not silently
  // animate when the user has expressed motion preferences.
  //
  // Consumed by Author's preview-tools fps strip (W63 sees the
  // bundle's `<Timecode />` riding alongside the FPS readout) and by
  // any future Perform / Playlist surface needing wall-clock counter
  // chrome.

  import { untrack } from 'svelte';

  type Props = {
    /** When false, the rAF tick is detached and the display freezes
     * at its current value. */
    running?: boolean;
  };

  let { running = true }: Props = $props();

  /** Elapsed seconds (float). Drives the four formatted segments. */
  let seconds = $state(0);
  let reduceMotion = $state(false);

  $effect(() => {
    if (typeof window === 'undefined') return;
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    reduceMotion = mq.matches;
    const onChange = (e: MediaQueryListEvent): void => {
      reduceMotion = e.matches;
    };
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  });

  $effect(() => {
    if (!running) return;
    if (reduceMotion) {
      seconds = 0;
      return;
    }
    let raf = 0;
    // Mirrors the bundle's `start = performance.now() - s * 1000` so
    // pause/resume preserves accumulated time.
    const start = performance.now() - untrack(() => seconds) * 1000;
    const tick = (t: number): void => {
      seconds = (t - start) / 1000;
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });

  function pad(n: number): string {
    return String(Math.floor(n)).padStart(2, '0');
  }

  const hh = $derived(pad(seconds / 3600));
  const mm = $derived(pad((seconds / 60) % 60));
  const ss = $derived(pad(seconds % 60));
  const ff = $derived(pad((seconds * 30) % 30));
</script>

<span class="timecode mono" role="timer" aria-label="Elapsed time {hh}:{mm}:{ss}, frame {ff} of 30">
  {hh}<span class="dim">:</span>{mm}<span class="dim">:</span>{ss}<span class="dim">:</span>{ff}
</span>

<style>
  .timecode {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-md);
    font-variant-numeric: tabular-nums;
    color: var(--color-text-1);
    letter-spacing: 0.04em;
  }
  .dim {
    color: var(--color-text-3);
    margin: 0 1px;
  }
  .mono {
    font-family: var(--font-family-mono);
  }
</style>
