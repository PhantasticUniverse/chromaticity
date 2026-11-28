<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-25 PR2 widget primitive: Audio Reactivity panel for the
  // Perform view's middle column. Bundle source:
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/perform.jsx`
  // lines 198-261 (`AudioReactivity` component). Three rows, top to
  // bottom: 4 frequency-band tiles, animated 48-bar FFT visualiser,
  // tempo strip (BPM + Ableton Link pill + TAP button).
  //
  // The bundle uses a `<canvas>` for the FFT bars; the Svelte port
  // keeps the pre-ADR-011 no-canvas constraint by rendering the bars
  // as inline `<rect>` elements driven by `$effect + rAF` writes to a
  // `liveBars` `$state` array. Reduced-motion guard short-circuits the
  // animation to a static fixture.
  //
  // **Live IPC OR synthetic fallback (Wave 6 PR 2).** Per
  // `docs/adr/ADR-019-audio-uniform-ipc-bridge.md` §Rollout PR 2 this
  // widget consumes the engine's `audio_uniform_snapshot_changed`
  // Tauri event via `audioSnapshotStore` (producer side: Wave 6 PR 1,
  // merge `c1d93e8`). When a non-null snapshot is available the four
  // band-tile values are reduced from the engine's 8 FFT bands (BASS
  // = avg(fft_bands[0..2]), LOW MID = avg(fft_bands[2..4]), HIGH MID
  // = avg(fft_bands[4..6]), HIGH = avg(fft_bands[6..8])) once
  // `fft_availability === 'Ready'`, and the BPM readout reads
  // `liveSnapshot.bpm` once `bpm_availability === 'Ready'`; warming
  // axes render dim `--` placeholders. Otherwise the existing
  // synthetic-time defaults render. The 48-bar FFT visualiser stays
  // synthetic-driven — the 8 → 48 interpolation is a stretch goal,
  // not in PR 2 scope (deferred as a post-MVP enhancement). Tap-tempo
  // and Ableton Link peer count remain visual stubs.
  //
  // The optional `snapshot` prop lets tests inject a fixed payload
  // without going through the singleton store; default behaviour reads
  // from the store. The `bands` prop (caller-supplied per-tile values)
  // overrides both store and synthetic defaults so tests / future
  // device-driven values keep working.

  import { untrack } from 'svelte';

  import type { AudioUniformSnapshotPayload } from '../ipc/bindings/AudioUniformSnapshotPayload';
  import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';
  import { bpmControlStore } from '../stores/bpmControl.svelte';

  let {
    bpm = undefined,
    linkPeerCount = 0,
    bands,
    snapshot = undefined,
  }: {
    bpm?: number;
    linkPeerCount?: number;
    bands?: ReadonlyArray<{ readonly name: string; readonly value: number; readonly tint: string }>;
    snapshot?: AudioUniformSnapshotPayload | null;
  } = $props();

  $effect(() => {
    void audioSnapshotStore.subscribe();
  });

  const liveSnapshot = $derived(snapshot ?? audioSnapshotStore.snapshot);
  const bpmReady = $derived(liveSnapshot === null || liveSnapshot.bpm_availability === 'Ready');
  const liveBpm = $derived(
    liveSnapshot && bpmReady ? liveSnapshot.bpm : (bpm ?? bpmControlStore.manualBpm ?? 120),
  );
  const bpmDisplay = $derived(liveSnapshot && !bpmReady ? '-- BPM' : `${liveBpm.toFixed(2)} BPM`);
  const fftWarming = $derived(
    !bands && liveSnapshot !== null && liveSnapshot.fft_availability !== 'Ready',
  );

  // Wave 2.8 Q.1 — smooth the per-band FFT values before they drive
  // the 4 named tiles. The engine producer emits a fresh snapshot
  // ~60 Hz with bin-level jitter (background mic noise reports
  // multi-millisecond bursts even when the room is quiet), and the
  // band-fill `transition: width 120ms ease` on top of every-frame %
  // writes amplifies that into a perceptible flicker. A short EMA on
  // each band damps the noise without erasing real audio onsets.
  // Skipped entirely when `bands` is caller-supplied (tests / custom
  // device-driven values) and when there's no snapshot.
  //
  // Wave 2.9 SC.C — the smoothed bar *fill width* was stable but the
  // integer-percent text label still flickered ±1% across consecutive
  // frames because `Math.round(value * 100)` quantizes the smooth
  // 21.0-23.4% drift into discrete `46→47→46→45→46` integers. Add a
  // small hysteresis on the *displayed* integer percent (separate from
  // the smoothed value driving the bar width) so the label only flips
  // when the smoothed value crosses ±0.6 past the previous integer
  // boundary. The fill width stays unchanged so visual responsiveness
  // is preserved.
  let smoothedBands = $state<readonly number[]>([0, 0, 0, 0]);
  let displayedBandPct = $state<readonly number[]>([0, 0, 0, 0]);
  $effect(() => {
    if (bands || snapshot !== undefined) return;
    let raf = 0;
    const s = [0, 0, 0, 0];
    const displayed = [-1, -1, -1, -1]; // -1 sentinel = first observation
    const EMA = 0.25;
    // Hysteresis margin in percent points. Once a label shows N%, the
    // smoothed value must move below (N - 0.6) or to or above (N + 1.6)
    // to flip. With smoothed-frame deltas of ~1% under typical jitter,
    // this eliminates frame-to-frame label oscillation.
    const HYST = 0.6;
    const tick = (): void => {
      const snap = untrack(() => audioSnapshotStore.snapshot);
      if (snap && snap.fft_availability === 'Ready') {
        const f = snap.fft_bands;
        const tiles = [(f[0] + f[1]) / 2, (f[2] + f[3]) / 2, (f[4] + f[5]) / 2, (f[6] + f[7]) / 2];
        for (let i = 0; i < 4; i++) {
          s[i] = (s[i] ?? 0) + ((tiles[i] ?? 0) - (s[i] ?? 0)) * EMA;
          const pct = (s[i] ?? 0) * 100;
          const cur = displayed[i] ?? -1;
          if (cur < 0) {
            displayed[i] = Math.round(pct);
          } else if (pct < cur - HYST || pct >= cur + 1 + HYST) {
            displayed[i] = Math.round(pct);
          }
        }
        smoothedBands = [s[0] ?? 0, s[1] ?? 0, s[2] ?? 0, s[3] ?? 0];
        displayedBandPct = [
          displayed[0] ?? 0,
          displayed[1] ?? 0,
          displayed[2] ?? 0,
          displayed[3] ?? 0,
        ];
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });

  // Default band tiles match the bundle's `AudioReactivity` JSX values.
  // Pass `bands` to override (tests / future device-driven values).
  const DEFAULT_BANDS: ReadonlyArray<{
    readonly name: string;
    readonly value: number;
    readonly tint: string;
  }> = [
    { name: 'BASS', value: 0, tint: 'var(--color-band-bass)' },
    { name: 'LOW MID', value: 0, tint: 'var(--color-band-mid)' },
    { name: 'HIGH MID', value: 0, tint: 'var(--color-band-midhi)' },
    { name: 'HIGH', value: 0, tint: 'var(--color-band-high)' },
  ];

  const WARMING_BANDS: ReadonlyArray<{
    readonly name: string;
    readonly value: number;
    readonly tint: string;
  }> = DEFAULT_BANDS.map((band) => ({ ...band, value: 0 }));

  /**
   * Reduce the engine's 8-band FFT array into the bundle's 4 named
   * tiles by averaging adjacent pairs (8 → 4). Preserves the bundle's
   * tile name + tint mapping so the visual layout never drifts; only
   * the per-tile `value` switches to live data.
   *
   * Wave 2.8 Q.1: callers now pass a pre-smoothed values array
   * (`smoothedBands` rAF state) instead of the raw snapshot, so the
   * tile values are stable across engine ticks. The bundle's tile
   * `name + tint` mapping is preserved verbatim.
   */
  function tilesFromValues(
    values: readonly number[],
  ): ReadonlyArray<{ readonly name: string; readonly value: number; readonly tint: string }> {
    return [
      { name: 'BASS', value: values[0] ?? 0, tint: 'var(--color-band-bass)' },
      { name: 'LOW MID', value: values[1] ?? 0, tint: 'var(--color-band-mid)' },
      { name: 'HIGH MID', value: values[2] ?? 0, tint: 'var(--color-band-midhi)' },
      { name: 'HIGH', value: values[3] ?? 0, tint: 'var(--color-band-high)' },
    ];
  }

  // When a caller supplies `snapshot` directly (tests / parametric
  // mounts), bypass the rAF-smoothing path and pull values straight
  // off the supplied payload so deterministic test fixtures stay
  // exact. Otherwise consume the locally-smoothed values driven by
  // the rAF $effect above.
  const renderBands = $derived(
    bands ??
      (fftWarming
        ? WARMING_BANDS
        : snapshot
          ? tilesFromValues([
              (snapshot.fft_bands[0] + snapshot.fft_bands[1]) / 2,
              (snapshot.fft_bands[2] + snapshot.fft_bands[3]) / 2,
              (snapshot.fft_bands[4] + snapshot.fft_bands[5]) / 2,
              (snapshot.fft_bands[6] + snapshot.fft_bands[7]) / 2,
            ])
          : liveSnapshot
            ? tilesFromValues(smoothedBands)
            : DEFAULT_BANDS),
  );

  // True iff the rAF-smoothing path is active (no caller-supplied
  // `bands` / `snapshot`, and a live snapshot exists). The hysteresis-
  // stabilized integer percent in `displayedBandPct` is the label
  // source only in this path; all other paths keep the synchronous
  // `Math.round(value * 100)` behaviour so the test contract (the
  // existing 7 percent-text assertions in `AudioReactivity.test.ts`)
  // stays exact.
  const useHystPct = $derived(!bands && snapshot === undefined && liveSnapshot !== null);

  const FFT_BAR_COUNT = 48;
  const FFT_VIEW_W = 100;
  const FFT_VIEW_H = 32;
  const FFT_BAR_GAP = 0.2;
  const FFT_BAR_W = (FFT_VIEW_W - FFT_BAR_GAP * (FFT_BAR_COUNT - 1)) / FFT_BAR_COUNT;

  /**
   * Bundle's per-bar formula (perform.jsx line 214):
   * `0.3 + 0.5 * |sin(t·(1 + f·4) + f·10)| * (1 - f·0.5)`.
   */
  function fftBarValue(f: number, t: number): number {
    return 0.3 + 0.5 * Math.abs(Math.sin(t * (1 + f * 4) + f * 10)) * (1 - f * 0.5);
  }

  function frozenBars(): readonly number[] {
    return new Array<number>(FFT_BAR_COUNT).fill(0.06);
  }

  let liveBars = $state<readonly number[]>(frozenBars());
  let reducedMotion = $state<boolean>(false);

  $effect(() => {
    if (typeof window === 'undefined') return;
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    reducedMotion = mq.matches;
    const handler = (e: MediaQueryListEvent): void => {
      reducedMotion = e.matches;
    };
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  });

  $effect(() => {
    if (reducedMotion || liveSnapshot === null) {
      liveBars = frozenBars();
      return;
    }
    let raf = 0;
    let last = 0;
    const start = performance.now();
    const next = new Array<number>(FFT_BAR_COUNT).fill(0);
    const tick = (now: number): void => {
      if (now - last > 50) {
        last = now;
        const t = (now - start) / 1000;
        for (let i = 0; i < FFT_BAR_COUNT; i++) {
          next[i] = fftBarValue(i / FFT_BAR_COUNT, t);
        }
        liveBars = [...next];
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });

  function fftBarX(index: number): number {
    return index * (FFT_BAR_W + FFT_BAR_GAP);
  }
  function fftBarH(value: number): number {
    return Math.max(0.4, value * (FFT_VIEW_H - 0.4));
  }
  function fftBarY(value: number): number {
    return FFT_VIEW_H - fftBarH(value);
  }

  function fftBarHueOpen(index: number): number {
    return 80 + (index / FFT_BAR_COUNT) * 40;
  }
</script>

<div class="audio-reactivity" role="img" aria-label="Audio reactivity preview">
  <div class="audio-reactivity__bands">
    {#each renderBands as band, i (band.name)}
      <div class="audio-reactivity__band" class:audio-reactivity__band--warming={fftWarming}>
        <div class="audio-reactivity__band-name">{band.name}</div>
        <div class="audio-reactivity__band-track">
          <div
            class="audio-reactivity__band-fill"
            style="width: {Math.max(0, Math.min(1, band.value)) * 100}%; background: {band.tint};"
          ></div>
        </div>
        <div class="audio-reactivity__band-pct">
          {#if fftWarming}
            --
          {:else}
            {useHystPct ? (displayedBandPct[i] ?? 0) : Math.round(band.value * 100)}%
          {/if}
        </div>
      </div>
    {/each}
  </div>

  <div class="audio-reactivity__fft">
    <svg
      class="audio-reactivity__fft-svg"
      viewBox="0 0 {FFT_VIEW_W} {FFT_VIEW_H}"
      preserveAspectRatio="none"
      aria-hidden="true"
    >
      {#each liveBars as bar, i (i)}
        <rect
          class="audio-reactivity__fft-bar"
          x={fftBarX(i)}
          y={fftBarY(bar)}
          width={FFT_BAR_W}
          height={fftBarH(bar)}
          rx="0.2"
          style="fill: hsl({fftBarHueOpen(i)} 90% 55%);"
        />
      {/each}
    </svg>
  </div>

  <div class="audio-reactivity__tempo">
    <span class="audio-reactivity__tempo-label">tempo</span>
    <span
      class="audio-reactivity__tempo-bpm"
      class:audio-reactivity__tempo-bpm--warming={liveSnapshot !== null && !bpmReady}
    >
      {bpmDisplay}
    </span>
    <span class="audio-reactivity__link-pill">
      <span class="audio-reactivity__link-dot" aria-hidden="true"></span>
      Ableton Link · {linkPeerCount} peers
    </span>
    <span class="audio-reactivity__tempo-spacer" aria-hidden="true"></span>
    <button type="button" class="audio-reactivity__tap-btn" disabled title="Tap tempo — Phase-3+">
      TAP
    </button>
  </div>
</div>

<style>
  .audio-reactivity {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    gap: var(--space-sm);
    flex: 1 1 auto;
    min-height: 0;
  }
  .audio-reactivity__bands {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--space-xs);
  }
  .audio-reactivity__band {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: var(--space-xs) var(--space-sm);
  }
  .audio-reactivity__band-name {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
  }
  .audio-reactivity__band-track {
    height: 4px;
    background: var(--color-bg-0);
    border-radius: var(--radius-xs);
    overflow: hidden;
    margin-top: var(--space-xxs);
  }
  .audio-reactivity__band-fill {
    height: 100%;
    border-radius: inherit;
    transition: width 120ms ease;
  }
  .audio-reactivity__band--warming .audio-reactivity__band-fill {
    opacity: 0.35;
  }
  .audio-reactivity__band-pct {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
    margin-top: var(--space-xxs);
    font-variant-numeric: tabular-nums;
  }
  .audio-reactivity__fft {
    position: relative;
    background: var(--color-bg-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    overflow: hidden;
    min-height: 0;
    padding: 4px;
  }
  .audio-reactivity__fft-svg {
    display: block;
    width: 100%;
    height: 100%;
  }
  .audio-reactivity__tempo {
    display: flex;
    gap: var(--space-sm);
    align-items: center;
    flex-wrap: wrap;
  }
  .audio-reactivity__tempo-label {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
  }
  .audio-reactivity__tempo-bpm {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-md);
    color: var(--color-primary);
    font-variant-numeric: tabular-nums;
  }
  .audio-reactivity__tempo-bpm--warming {
    color: var(--color-text-3);
  }
  .audio-reactivity__link-pill {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xxs);
    padding: 2px var(--space-sm);
    border: 1px solid var(--color-line-2);
    border-radius: 999px;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-2);
  }
  .audio-reactivity__link-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-primary);
  }
  .audio-reactivity__tempo-spacer {
    flex: 1;
  }
  .audio-reactivity__tap-btn {
    background: var(--color-bg-2);
    color: var(--color-text-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: 2px var(--space-md);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    cursor: not-allowed;
    opacity: 0.6;
  }
</style>
