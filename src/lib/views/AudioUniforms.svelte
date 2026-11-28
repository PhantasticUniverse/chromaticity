<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-17 PR3 Audio Uniforms view full port — replaces the W16 PR2
  // `EmptyState` stub with the spectrum + frequency-band tiles + time
  // uniforms graph + BPM uniform cards layout per the canonical bundle
  // JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/audio.jsx`,
  // the `AudioUniformsView` component mapped to `id: "audio"` in the
  // bundle's VIEWS array). The W16 PR2 sidebar redesign added Audio
  // Uniforms as an enabled stub; this PR fills in the operator-facing
  // content. Bundle JSX is the canonical structural reference per the
  // auto-memory `feedback_wireframes_normative_for_ui_ux.md`; the
  // `v0.2-ui-wireframe-screenshots/pulse/audio-uniforms.png` rendering
  // is the visual confirmation (it renders the same JSX).
  //
  // Locked W17 PR3 deviations from the bundle:
  //   - The bundle's `requestAnimationFrame` driver and signal generators
  //     are gone; live snapshots arrive via the Wave 1c-α-2 audio IPC seam
  //     (`audio_uniform_snapshot_changed`) and drive the band tiles + BPM
  //     strip; the 64-bar Spectrum + 48-bar FFT visualisers remain
  //     synthetic-driven per the `EV-AUDIO-UNIFORM-IPC` post-MVP narrowing.
  //   - The bundle's `<Spectrum>`, `<Preview>`, and `<Sparkline>` shared
  //     primitives are not yet ported as Svelte widgets; this view inlines
  //     SVG renderings of each. No new icon / canvas primitives are
  //     introduced — the brief constraint and the project's "three similar
  //     lines beats a premature abstraction" anti-pattern both apply.
  //   - Streaming / Pause buttons toggle the local `streaming` flag so the
  //     band tiles can freeze on a captured frame; the underlying IPC
  //     subscription stays live (no engine-side pause command is wired).
  //   - The `Live Visualizer` preview placeholder is a static gradient blob
  //     field — same approach the W16 PR3 Welcome view took for its
  //     `<Preview>` substitution, for the same reason (runtime / scene
  //     crates are not wired through to the UI yet).
  //   - W20 view-internal icon sweep: Snapshot button, "How to use"
  //     collapse caret, and the docs deep-link arrow render via the W19
  //     PR3 vendored `Icon.svelte` primitive (`camera` / `caret` /
  //     `chevron` / `arrow`) per `views/audio.jsx`. Inline `●` / `○`
  //     hit-state markers in the band-bar value column have no clean
  //     bundle Icon mapping and stay as Unicode (font-driven; the
  //     filled-circle `bang` icon is a different metaphor).
  import {
    AUDIO_UNIFORMS_FIXTURE,
    HOW_TO_USE_BLOCKS,
    SPARKLINE_GENERATORS,
    SPECTRUM_BAR_COUNT,
    buildSparkline,
    spectrumBarValue,
    type FrequencyBand,
    type FrequencyBandBar,
  } from '../data/audio-uniform-fixtures';
  import Icon from '../Icon.svelte';
  import type { AudioUniformSnapshotPayload } from '../ipc/bindings/AudioUniformSnapshotPayload';
  import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';
  import { bpmControlStore } from '../stores/bpmControl.svelte';
  import { settingsStore } from '../stores/settings.svelte';
  import { BpmRing, BeatDots } from '../widgets';

  const heroBpm = $derived(
    audioSnapshotStore.snapshot?.bpm_availability === 'Ready'
      ? audioSnapshotStore.snapshot.bpm
      : (bpmControlStore.manualBpm ?? 120),
  );

  let streaming = $state<boolean>(AUDIO_UNIFORMS_FIXTURE.streamingActive);
  let simulateSignal = $state<boolean>(false);
  let docsOpen = $state<boolean>(true);

  function toggleStreaming(): void {
    streaming = !streaming;
  }

  function toggleSimulateSignal(): void {
    simulateSignal = !simulateSignal;
  }

  function toggleDocs(): void {
    docsOpen = !docsOpen;
  }

  // Spectrum SVG geometry — 64 bars across a 100-unit viewBox so the SVG
  // scales fluidly to whatever pixel width the panel grants it.
  const SPECTRUM_VIEW_W = 100;
  const SPECTRUM_VIEW_H = 36;
  const SPECTRUM_BAR_GAP = 0.15; // viewBox units
  const SPECTRUM_BAR_W =
    (SPECTRUM_VIEW_W - SPECTRUM_BAR_GAP * (SPECTRUM_BAR_COUNT - 1)) / SPECTRUM_BAR_COUNT;

  function spectrumBarX(index: number): number {
    return index * (SPECTRUM_BAR_W + SPECTRUM_BAR_GAP);
  }
  function spectrumBarH(value: number): number {
    return Math.max(0.6, value * (SPECTRUM_VIEW_H - 1));
  }
  function spectrumBarY(value: number): number {
    return SPECTRUM_VIEW_H - spectrumBarH(value);
  }

  // Live animation pipeline — W25 PR1 closes the W17 frozen-frame
  // deferral. The bundle's `<Spectrum>` and `<Sparkline>` primitives run
  // a deterministic synthetic generator at ~50 ms cadence; under the
  // bundle's interpretation of the frozen-frame contract that animation
  // is in scope (no IPC dispatch / no real audio data — the formula is
  // purely time-dependent).
  let liveT = $state<number>(17);
  let livePeaks = $state<readonly number[]>(new Array(SPECTRUM_BAR_COUNT).fill(0));
  let audioReducedMotion = $state<boolean>(false);

  $effect(() => {
    if (typeof window === 'undefined') return;
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    audioReducedMotion = mq.matches;
    const handler = (e: MediaQueryListEvent): void => {
      audioReducedMotion = e.matches;
    };
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  });

  $effect(() => {
    const activeSignal = audioSnapshotStore.snapshot !== null || simulateSignal;
    if (!streaming || !activeSignal) return;
    if (audioReducedMotion) {
      liveT = 17;
      livePeaks = new Array(SPECTRUM_BAR_COUNT).fill(0);
      return;
    }
    // Closure-local peak ring — written each frame, mirrored to the
    // `livePeaks` $state so the SVG re-renders without the $effect
    // self-triggering on its own writes.
    const peaks = new Array<number>(SPECTRUM_BAR_COUNT).fill(0);
    let raf = 0;
    let last = 0;
    const start = performance.now();
    const tick = (now: number): void => {
      if (now - last > 50) {
        last = now;
        const t = 17 + (now - start) / 1000;
        for (let i = 0; i < SPECTRUM_BAR_COUNT; i++) {
          const v = spectrumBarValue(i / SPECTRUM_BAR_COUNT, t);
          peaks[i] = Math.max((peaks[i] ?? 0) * 0.95, v);
        }
        liveT = t;
        livePeaks = [...peaks];
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });

  const IDLE_SPECTRUM_BARS: readonly number[] = new Array(SPECTRUM_BAR_COUNT).fill(0.03);

  const liveSpectrumBars = $derived.by<readonly number[]>(() => {
    const activeSignal = audioSnapshotStore.snapshot !== null || simulateSignal;
    if (!activeSignal) return IDLE_SPECTRUM_BARS;
    if (audioReducedMotion || !streaming) return AUDIO_UNIFORMS_FIXTURE.spectrumBars;
    const out: number[] = [];
    for (let i = 0; i < SPECTRUM_BAR_COUNT; i++) {
      out.push(spectrumBarValue(i / SPECTRUM_BAR_COUNT, liveT));
    }
    return out;
  });

  // Sparkline samples by uniform key. Flat baseline when offline (unless Test Signal is enabled).
  const liveSparklines = $derived.by<Readonly<Record<string, readonly number[]>>>(() => {
    const activeSignal = audioSnapshotStore.snapshot !== null || simulateSignal;
    if (!activeSignal) {
      const flat: Record<string, readonly number[]> = {};
      for (const key of Object.keys(SPARKLINE_GENERATORS)) {
        flat[key] = new Array(60).fill(0.5);
      }
      return flat;
    }
    if (audioReducedMotion || !streaming) return {};
    const out: Record<string, readonly number[]> = {};
    for (const [key, gen] of Object.entries(SPARKLINE_GENERATORS)) {
      out[key] = buildSparkline(gen, liveT);
    }
    return out;
  });

  function spectrumPeakY(peak: number): number {
    return SPECTRUM_VIEW_H - peak * (SPECTRUM_VIEW_H - 1);
  }

  // ---------------------------------------------------------------------
  // Wave 2.6 AUD.D — live band-tile wiring. The five fixture band tiles
  // (Bass / Mid / Mid-High / High / Full) read live values reduced from
  // the engine's 8 log-spaced FFT bands carried over the
  // `audio_uniform_snapshot_changed` Tauri event per
  // `docs/adr/ADR-019-audio-uniform-ipc-bridge.md`. Mirrors the
  // `AudioReactivity` widget's 8 → 4 idiom from Wave 1c-α (PR `#340`),
  // extended to 5 tiles by pairing adjacent fft bins for the four
  // frequency-specific tiles and averaging all eight for the broadband
  // `Full` tile:
  //   bass    ← avg(fft_bands[0], fft_bands[1])
  //   mid     ← avg(fft_bands[2], fft_bands[3])
  //   mid-hi  ← avg(fft_bands[4], fft_bands[5])
  //   high    ← avg(fft_bands[6], fft_bands[7])
  //   full    ← avg(fft_bands[0..7])
  //
  // Each tile renders three bars (level / hit / presence). The reduced
  // value drives `level` directly; `hit` is a simple above-threshold
  // flash so the `●`/`○` indicator still reacts to onsets; `presence`
  // mirrors level (the visual just needs to react to the engine value
  // — proper peak-hold smoothing is post-MVP UX). Dual-mode fallback:
  // when `audioSnapshotStore.snapshot === null` (no Tauri runtime, or
  // pre-bootstrap startup window) the frozen-frame fixture values
  // render, matching the Wave 1c-α-2 posture for the visualizer widgets.

  $effect(() => {
    void audioSnapshotStore.subscribe();
  });

  // Wave 2.6 AUD.C — settings hydration for the hero stats source-label
  // fallback (see derivation block below).
  $effect(() => {
    void settingsStore.load();
  });

  const HIT_THRESHOLD = 0.5;

  /**
   * Reduce the engine's 8-band FFT array into the bundle's 5 named
   * tiles (Bass / Mid / Mid-High / High / Full). The four
   * frequency-specific tiles average adjacent pairs (8 → 4); the
   * broadband `Full` tile averages all eight bins. Per-tile bar
   * derivations are kept simple: level = reduced value, hit = step
   * flash above `HIT_THRESHOLD`, presence = reduced value (mirrors
   * level — proper peak-hold is post-MVP UX). Preserves the fixture's
   * tile id / label / sub-label / tint / bar-key shape so the visual
   * layout never drifts; only the per-bar `value` switches to live data.
   */
  function reduceFftToBands(payload: AudioUniformSnapshotPayload): readonly FrequencyBand[] {
    const f = payload.fft_bands;
    const avg2 = (a: number, b: number): number => (a + b) / 2;
    const avgAll = (f[0] + f[1] + f[2] + f[3] + f[4] + f[5] + f[6] + f[7]) / 8;
    const reducedByBandId: Readonly<Record<string, number>> = {
      bass: avg2(f[0], f[1]),
      mid: avg2(f[2], f[3]),
      midhi: avg2(f[4], f[5]),
      high: avg2(f[6], f[7]),
      full: avgAll,
    };
    return AUDIO_UNIFORMS_FIXTURE.bands.map((band) => {
      const v = reducedByBandId[band.id] ?? 0;
      const hit = v > HIT_THRESHOLD ? Math.min(1, (v - HIT_THRESHOLD) * 2) : 0;
      const bars: readonly FrequencyBandBar[] = band.bars.map((bar) => {
        if (bar.kind === 'level') return { key: bar.key, kind: bar.kind, value: v };
        if (bar.kind === 'hit') return { key: bar.key, kind: bar.kind, value: hit };
        return { key: bar.key, kind: bar.kind, value: v };
      });
      return { ...band, bars };
    });
  }

  // Wave 2.8 Q.1 — the engine producer emits a fresh snapshot ~60 Hz
  // and its `peak` / `rms` / `fft_bands` values carry bin-level jitter
  // even when the room is quiet (background mic noise reports ~0.005
  // RMS with several-millisecond bursts). Re-deriving the band tiles +
  // hero dB readout off the raw snapshot once per frame made the band
  // values and the dB text flicker visibly. Smooth them with a short
  // EMA + peak-hold so the displayed values track the audible signal
  // instead of the noise floor.
  //
  // The smoother runs in a `$effect` that observes the snapshot store
  // directly — every snapshot write advances the smoother synchronously
  // before the next paint, so deterministic tests that mutate
  // `audioSnapshotStore.snapshot` then read the DOM still see the
  // raw values applied (the first observation seeds the smoother, no
  // EMA blending on frame 1). The smoother state is mirrored to a
  // separate `smoothedSnapshot` `$state` so downstream `$derived`
  // tile / dB readout consumers re-evaluate on every advance.
  // Initial value seeded synchronously off the current store snapshot
  // so first-paint reads it BEFORE the `$effect` flush. This preserves
  // the deterministic test contract that `audioSnapshotStore.snapshot
  // = {...}; render(...); querySelector(...)` sees the raw values
  // without an awaited tick.
  const _initialSnap = audioSnapshotStore.snapshot;
  let smoothedSnapshot = $state<AudioUniformSnapshotPayload | null>(
    _initialSnap ? { ..._initialSnap, fft_bands: [..._initialSnap.fft_bands] } : null,
  );
  // EMA factor 0.25 means ~5-tick half-life (~80 ms at 60 Hz) — fast
  // enough to track real onsets, slow enough to discard noise-floor
  // bursts. Peak holds with 0.004 decay to mirror the VuStereo path.
  const LEVEL_EMA = 0.25;
  const PEAK_DECAY = 0.004;
  let _smInit = _initialSnap !== null;
  let _smRms = _initialSnap?.rms ?? 0;
  let _smPeak = _initialSnap?.peak ?? 0;
  const _smFft: number[] = _initialSnap
    ? [..._initialSnap.fft_bands]
    : new Array<number>(8).fill(0);
  $effect(() => {
    const snap = audioSnapshotStore.snapshot;
    if (!snap) {
      _smInit = false;
      smoothedSnapshot = null;
      return;
    }
    if (!_smInit) {
      _smInit = true;
      _smRms = snap.rms;
      _smPeak = snap.peak;
      for (let i = 0; i < 8; i++) {
        _smFft[i] = snap.fft_bands[i] ?? 0;
      }
    } else {
      _smRms = _smRms + (snap.rms - _smRms) * LEVEL_EMA;
      _smPeak = Math.max(snap.peak, _smPeak - PEAK_DECAY);
      for (let i = 0; i < 8; i++) {
        const raw = snap.fft_bands[i] ?? 0;
        _smFft[i] = (_smFft[i] ?? 0) + (raw - (_smFft[i] ?? 0)) * LEVEL_EMA;
      }
    }
    smoothedSnapshot = {
      bpm: snap.bpm,
      bpm_availability: snap.bpm_availability,
      rms: _smRms,
      peak: _smPeak,
      fft_bands: [
        _smFft[0] ?? 0,
        _smFft[1] ?? 0,
        _smFft[2] ?? 0,
        _smFft[3] ?? 0,
        _smFft[4] ?? 0,
        _smFft[5] ?? 0,
        _smFft[6] ?? 0,
        _smFft[7] ?? 0,
      ],
      fft_availability: snap.fft_availability,
      degraded: snap.degraded,
    };
  });

  const ZERO_BANDS: readonly FrequencyBand[] = AUDIO_UNIFORMS_FIXTURE.bands.map((band) => ({
    ...band,
    bars: band.bars.map((bar) => ({ ...bar, value: 0 })),
  }));

  const liveBands = $derived<readonly FrequencyBand[]>(
    smoothedSnapshot?.fft_availability === 'Ready'
      ? reduceFftToBands(smoothedSnapshot)
      : simulateSignal
        ? AUDIO_UNIFORMS_FIXTURE.bands
        : ZERO_BANDS,
  );

  // Wave 2.6 AUD.C — live peak / RMS dB readout in the hero stats strip.
  // The bundle's static `peakDb` / `rmsDb` fixture is dropped in favour
  // of a derivation off `audioSnapshotStore.snapshot.peak` /
  // `.rms` (linear amplitude, 0..1) converted to dBFS via
  // `20 * log10(x)`. A floor at 1e-3 (-60 dBFS) avoids -Infinity when
  // the producer reports digital silence (e.g. the `SilentSource`
  // fallback on hosts without a default cpal input device). Source
  // label reads `settingsStore.snapshot.audio.input_device` when it
  // resolves to a non-default device string; the literal sentinel
  // 'default' (seeded in `DEFAULT_SETTINGS`) falls back to fixture so
  // the wireframe-faithful "Mic · Yeti" placeholder survives until
  // Lane AUD.A's live device enumeration populates an authoritative
  // name. Same dual-mode posture as `AudioReactivity.svelte`.
  //
  // Wave 2.8 Q.1: dB readout reads the smoothed snapshot above, not
  // the raw store, so the displayed text doesn't churn 2-3 dB per
  // frame on tiny noise-floor variations.
  const DB_FLOOR = 1e-3;

  function toDbFs(linear: number): number {
    return 20 * Math.log10(Math.max(linear, DB_FLOOR));
  }

  const livePeakDb = $derived<number>(
    smoothedSnapshot ? toDbFs(smoothedSnapshot.peak) : AUDIO_UNIFORMS_FIXTURE.peakDb,
  );
  const liveRmsDb = $derived<number>(
    smoothedSnapshot ? toDbFs(smoothedSnapshot.rms) : AUDIO_UNIFORMS_FIXTURE.rmsDb,
  );
  const liveSource = $derived<string>(
    settingsStore.snapshot.audio.input_device &&
      settingsStore.snapshot.audio.input_device !== 'default'
      ? settingsStore.snapshot.audio.input_device
      : 'System Default',
  );

  // Sparkline SVG geometry — matches the bundle's 100×32 viewBox / 60-point
  // window so the existing fixture sample arrays drop straight in.
  const SPARK_W = 100;
  const SPARK_H = 32;

  function sparklinePath(samples: readonly number[]): string {
    if (samples.length === 0) return '';
    return samples
      .map((sample, i) => {
        const x = (i / (samples.length - 1)) * SPARK_W;
        const y = SPARK_H - sample * (SPARK_H - 4) - 2;
        return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
  }

  function sparklineHeadX(samples: readonly number[]): number {
    if (samples.length === 0) return 0;
    return ((samples.length - 1) / (samples.length - 1)) * SPARK_W;
  }
  function sparklineHeadY(samples: readonly number[]): number {
    if (samples.length === 0) return SPARK_H / 2;
    const last = samples[samples.length - 1] ?? 0;
    return SPARK_H - last * (SPARK_H - 4) - 2;
  }

  function shortLabel(kind: FrequencyBandBar['kind']): string {
    if (kind === 'level') return 'LVL';
    if (kind === 'hit') return 'HIT';
    return 'PRES';
  }

  // Wave 2.6 AUD.E — frontend-only capture of the current live audio
  // snapshot to a JSON file. Reads `audioSnapshotStore.snapshot`; no
  // engine round-trip. The button is ARIA-disabled while the store is
  // `null` (no live IPC events yet) so this handler's null-guard is a
  // belt-and-braces no-op for the keyboard path.
  //
  // Download path: Blob + URL.createObjectURL + temp anchor with the
  // `download` attribute. Works equivalently in the vite dev / vitest
  // browser context and inside Tauri's WebView (no plugin deps required).
  const hasSnapshot = $derived<boolean>(audioSnapshotStore.snapshot !== null);

  function captureSnapshot(): void {
    const snap = audioSnapshotStore.snapshot;
    if (snap === null) return;
    const capturedAt = new Date().toISOString();
    const payload = {
      bpm: snap.bpm,
      bpm_availability: snap.bpm_availability,
      rms: snap.rms,
      peak: snap.peak,
      fft_bands: snap.fft_bands,
      fft_availability: snap.fft_availability,
      degraded: snap.degraded,
      captured_at: capturedAt,
    };
    const json = JSON.stringify(payload, null, 2);
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    // Strip the `:` characters from the ISO timestamp so the filename is
    // portable across Windows + macOS + Linux.
    const stamp = capturedAt.replace(/[:.]/g, '-');
    const filename = `audio-snapshot-${stamp}.json`;
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.rel = 'noopener';
    a.style.display = 'none';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }
</script>

<section class="au">
  <div class="au__main">
    <header class="au__header">
      <h2 class="au__title">
        Audio Uniforms <span class="au__title-sep" aria-hidden="true">·</span>
        <span class="au__title-mode">{hasSnapshot ? 'Live' : 'Preview'}</span>
      </h2>
      <div class="au__header-actions">
        <span class="au__pill" class:au__pill--on={hasSnapshot && streaming}>
          <span class="au__pill-dot" aria-hidden="true"></span>
          {!hasSnapshot
            ? simulateSignal
              ? 'test signal'
              : 'offline · preview'
            : streaming
              ? 'streaming'
              : 'paused'}
        </span>
        {#if !hasSnapshot}
          <button
            type="button"
            class="au__btn"
            onclick={toggleSimulateSignal}
            aria-pressed={simulateSignal ? 'true' : 'false'}
            title="Drive spectrum and band meters with a synthetic test signal while audio is offline"
          >
            {simulateSignal ? 'Stop Test' : 'Test Signal'}
          </button>
        {/if}
        <button
          type="button"
          class="au__btn"
          onclick={toggleStreaming}
          aria-pressed={streaming ? 'false' : 'true'}
        >
          {streaming ? 'Freeze' : 'Resume'}
        </button>
        <button
          type="button"
          class="au__btn"
          onclick={captureSnapshot}
          disabled={!hasSnapshot}
          aria-disabled={hasSnapshot ? 'false' : 'true'}
          aria-label="Snapshot"
          title={hasSnapshot
            ? 'Capture the current audio snapshot as JSON'
            : 'Snapshot — waiting for live audio data'}
        >
          <Icon name="camera" size={11} /> Snapshot
        </button>
      </div>
    </header>

    <div class="au__body">
      <!-- Hero: spectrum -->
      <section class="au__hero" aria-label="Spectrum">
        <div class="au__hero-head">
          <span class="au__section-label">waveform · spectrum</span>
          <div class="au__hero-stats">
            <span>peak <b>{hasSnapshot ? `${livePeakDb.toFixed(1)} dB` : '— dB'}</b></span>
            <span>rms <b>{hasSnapshot ? `${liveRmsDb.toFixed(1)} dB` : '— dB'}</b></span>
            <span>src <b>{liveSource}</b></span>
          </div>
        </div>
        <div
          class="au__hero-canvas"
          role="img"
          aria-label={streaming ? 'Audio spectrum (live synthetic)' : 'Audio spectrum (paused)'}
        >
          <svg
            class="au__spectrum-svg"
            viewBox="0 0 {SPECTRUM_VIEW_W} {SPECTRUM_VIEW_H}"
            preserveAspectRatio="none"
            aria-hidden="true"
          >
            {#each liveSpectrumBars as bar, i (i)}
              <rect
                class="au__spectrum-bar"
                x={spectrumBarX(i)}
                y={spectrumBarY(bar)}
                width={SPECTRUM_BAR_W}
                height={spectrumBarH(bar)}
                rx="0.2"
              />
            {/each}
            {#if !audioReducedMotion && streaming}
              {#each livePeaks as peak, i (i)}
                {#if peak > 0.04}
                  <rect
                    class="au__spectrum-peak"
                    x={spectrumBarX(i)}
                    y={spectrumPeakY(peak)}
                    width={SPECTRUM_BAR_W}
                    height="0.4"
                  />
                {/if}
              {/each}
            {/if}
          </svg>
        </div>
        <div class="au__hero-tempo">
          <BpmRing bpm={heroBpm} size={72} />
          <div class="au__hero-tempo-meta">
            <span class="au__section-label">tempo · 4-count</span>
            <BeatDots bpm={heroBpm} />
          </div>
        </div>
      </section>

      <!-- Frequency bands -->
      <section class="au__section" aria-label="Frequency bands">
        <header class="au__section-head">
          <span class="au__section-title">frequency bands</span>
          <span class="au__section-sub">smoothed · hits · presence per band</span>
        </header>
        <div class="au__bands">
          {#each liveBands as band (band.id)}
            <div class="au__band" style="--band-tint: {band.tint};">
              <div class="au__band-head">
                <span class="au__band-name">{band.label}</span>
                <span class="au__band-sub">{band.subLabel}</span>
              </div>
              <div class="au__band-bars">
                {#each band.bars as bar (bar.key)}
                  <div class="au__bar">
                    <div class="au__bar-track">
                      {#if bar.kind === 'hit'}
                        <div
                          class="au__bar-flash"
                          style="opacity: {Math.max(0, Math.min(1, bar.value))};"
                        ></div>
                      {:else}
                        <div
                          class="au__bar-fill"
                          class:au__bar-fill--presence={bar.kind === 'presence'}
                          style="height: {Math.max(0.02, bar.value) * 100}%;"
                        ></div>
                      {/if}
                    </div>
                    <div
                      class="au__bar-value"
                      class:au__bar-value--hit={bar.kind === 'hit'}
                      class:au__bar-value--lit={bar.kind === 'hit' && bar.value > 0.05}
                    >
                      {#if bar.kind === 'hit'}
                        {bar.value > 0.05 ? '●' : '○'}
                      {:else}
                        {bar.value.toFixed(2)}
                      {/if}
                    </div>
                    <div class="au__bar-label">{shortLabel(bar.kind)}</div>
                  </div>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      </section>

      <!-- Time uniforms -->
      <section class="au__section" aria-label="Time uniforms">
        <header class="au__section-head">
          <span class="au__section-title">time uniforms</span>
          <span class="au__section-sub">per-band accumulators · seconds since session start</span>
        </header>
        <div class="au__time-rows">
          {#each AUDIO_UNIFORMS_FIXTURE.timeUniforms as uniform (uniform.key)}
            <div class="au__time-row" style="--time-tint: {uniform.tint};">
              <span class="au__time-key mono">{uniform.key}</span>
              <div class="au__time-track">
                <div class="au__time-fill" style="width: {uniform.phase * 100}%;"></div>
              </div>
              <span class="au__time-val mono">{uniform.value.toFixed(2)}</span>
            </div>
          {/each}
        </div>
      </section>

      <!-- BPM uniforms -->
      <section class="au__section" aria-label="BPM uniforms">
        <header class="au__section-head">
          <span class="au__section-title">bpm uniforms</span>
          <span class="au__section-sub"
            >tempo-locked oscillators · live preview of last 3 seconds</span
          >
        </header>
        <div class="au__bpm-grid">
          {#each AUDIO_UNIFORMS_FIXTURE.bpmUniforms as bpm (bpm.key)}
            {#if bpm.kind === 'scalar'}
              <article class="au__bpm-card au__bpm-card--accent">
                <header class="au__bpm-head">
                  <span class="au__bpm-name mono">{bpm.key}</span>
                  <span class="au__bpm-val mono">{bpm.displayValue}</span>
                </header>
                <span class="au__bpm-label">{bpm.label}</span>
              </article>
            {:else if bpm.kind === 'bool'}
              <article class="au__bpm-card au__bpm-card--bool">
                <header class="au__bpm-head">
                  <span class="au__bpm-name mono">{bpm.key}</span>
                  <span
                    class="au__bpm-led"
                    class:au__bpm-led--on={bpm.ledOn ?? false}
                    aria-hidden="true"
                  ></span>
                </header>
                <span class="au__bpm-label">{bpm.label}</span>
              </article>
            {:else}
              <article class="au__bpm-card">
                <header class="au__bpm-head">
                  <span class="au__bpm-name mono">{bpm.key}</span>
                  <span class="au__bpm-val mono">
                    {bpm.key === 'syn_BPM' ? heroBpm.toFixed(1) : bpm.displayValue}
                  </span>
                </header>
                {#if bpm.sparkline && bpm.sparkline.length > 0}
                  {@const samples = liveSparklines[bpm.key] ?? bpm.sparkline}
                  <div class="au__spark" aria-hidden="true">
                    <div class="au__spark-mid"></div>
                    <svg
                      class="au__spark-svg"
                      viewBox="0 0 {SPARK_W} {SPARK_H}"
                      preserveAspectRatio="none"
                    >
                      <path
                        class="au__spark-path"
                        d={sparklinePath(samples)}
                        vector-effect="non-scaling-stroke"
                      />
                      <circle
                        class="au__spark-head"
                        cx={sparklineHeadX(samples)}
                        cy={sparklineHeadY(samples)}
                        r="1.6"
                      />
                    </svg>
                  </div>
                {/if}
                <span class="au__bpm-label">{bpm.label}</span>
              </article>
            {/if}
          {/each}
        </div>
      </section>
    </div>
  </div>

  <aside class="au__side" aria-label="Audio uniforms reference">
    <div class="au__preview" role="img" aria-label="Live visualizer preview">
      <span class="au__preview-blob au__preview-blob--bass"></span>
      <span class="au__preview-blob au__preview-blob--mid"></span>
      <span class="au__preview-blob au__preview-blob--high"></span>
      <span class="au__preview-label">live visualizer</span>
    </div>

    <section class="au__docs">
      <button
        type="button"
        class="au__docs-toggle"
        aria-expanded={docsOpen ? 'true' : 'false'}
        onclick={toggleDocs}
      >
        <span>how to use</span>
        <span class="au__docs-chevron" class:au__docs-chevron--collapsed={!docsOpen}>
          <Icon name={docsOpen ? 'caret' : 'chevron'} size={11} />
        </span>
      </button>
      {#if docsOpen}
        <div class="au__docs-body">
          {#each HOW_TO_USE_BLOCKS as block, i (i)}
            <div class="au__docs-block">
              {#if block.title}
                <h3 class="au__docs-subhead">{block.title}</h3>
              {/if}
              <p class="au__docs-prose">
                {#if block.intro}{block.intro}{/if}
                {#if block.tokens}
                  {#each block.tokens as token, ti (ti)}
                    <span class="au__docs-token mono" style="color: {token.tint};"
                      >{token.text}</span
                    >{#if block.tokens && ti < block.tokens.length - 1}<span>,&nbsp;</span>{/if}
                  {/each}
                {/if}
                {#if block.trailing}{block.trailing}{/if}
              </p>
            </div>
          {/each}
          <a class="au__docs-link mono" href="#audio-uniforms-docs">
            <Icon name="arrow" size={11} /> Read the Chromaticity Audio Uniforms docs
          </a>
        </div>
      {/if}
    </section>
  </aside>
</section>

<style>
  /* ---------------- Outer grid ----------------------------------------- */
  .au {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 320px;
    gap: var(--space-md);
    align-items: stretch;
    min-height: 0;
  }

  @media (max-width: 980px) {
    .au {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  /* ---------------- Main panel ----------------------------------------- */
  .au__main {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-panel);
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  .au__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
    padding: var(--space-md) var(--space-lg);
    border-bottom: 1px solid var(--color-line-2);
    background: var(--color-bg-3);
  }

  .au__title {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-1);
  }
  .au__title-sep {
    color: var(--color-text-3);
    margin: 0 4px;
  }
  .au__title-mode {
    color: var(--color-primary);
  }

  .au__header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .au__pill {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    padding: 2px 8px;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-full);
    background: var(--color-bg-2);
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
  }
  .au__pill--on {
    color: var(--color-success);
    border-color: color-mix(in oklab, var(--color-success) 50%, var(--color-line-2));
  }
  .au__pill-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-text-3);
  }
  .au__pill--on .au__pill-dot {
    background: var(--color-success);
    box-shadow: 0 0 calc(6px * var(--glow, 1)) var(--color-success);
  }

  .au__btn {
    padding: var(--space-xs) var(--space-md);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-label);
    cursor: pointer;
    transition:
      background-color 120ms ease,
      border-color 120ms ease,
      color 120ms ease;
  }
  .au__btn:hover:not(:disabled) {
    background: var(--color-bg-4);
    border-color: var(--color-line-3);
    color: var(--color-text-0);
  }
  .au__btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .au__btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  /* ---------------- Body / sections ------------------------------------ */
  .au__body {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
    padding: var(--space-lg);
    overflow: auto;
    min-height: 0;
  }

  .au__section {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .au__section-head {
    display: flex;
    align-items: baseline;
    gap: var(--space-md);
  }

  .au__section-label,
  .au__section-title {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-2);
  }
  .au__section-sub {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }

  /* ---------------- Spectrum hero -------------------------------------- */
  .au__hero {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: var(--space-md);
  }
  .au__hero-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-md);
  }
  .au__hero-stats {
    display: flex;
    gap: var(--space-md);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }
  .au__hero-stats b {
    color: var(--color-text-1);
    font-weight: var(--font-weight-label);
  }
  .au__hero-canvas {
    height: 76px;
    background: var(--color-bg-0);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    overflow: hidden;
    padding: 4px;
  }
  .au__spectrum-svg {
    display: block;
    width: 100%;
    height: 100%;
  }
  .au__spectrum-bar {
    fill: var(--color-band-midhi);
    opacity: 0.85;
  }
  .au__spectrum-peak {
    fill: var(--color-text-1);
    opacity: 0.55;
  }
  .au__hero-tempo {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    padding-top: var(--space-xs);
  }
  .au__hero-tempo-meta {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    min-width: 0;
  }

  /* ---------------- Band tiles ----------------------------------------- */
  .au__bands {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: var(--space-sm);
  }

  @media (max-width: 1100px) {
    .au__bands {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  .au__band {
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: var(--space-xs) var(--space-sm);
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    min-width: 0;
  }
  .au__band-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-xs);
  }
  .au__band-name {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--band-tint);
  }
  .au__band-sub {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
  }
  .au__band-bars {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-sm);
  }
  .au__bar {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    min-width: 0;
  }
  .au__bar-track {
    position: relative;
    width: 100%;
    height: 44px;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-xs);
    overflow: hidden;
  }
  .au__bar-fill {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--band-tint);
    opacity: 0.95;
  }
  .au__bar-fill--presence {
    opacity: 0.55;
  }
  .au__bar-flash {
    position: absolute;
    inset: 0;
    background: var(--band-tint);
  }
  .au__bar-value {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }
  .au__bar-value--hit {
    color: var(--color-text-3);
  }
  .au__bar-value--lit {
    color: var(--band-tint);
  }
  .au__bar-label {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
    text-transform: uppercase;
  }

  /* ---------------- Time uniforms -------------------------------------- */
  .au__time-rows {
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: var(--space-sm) var(--space-md);
  }
  .au__time-row {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr) 60px;
    gap: var(--space-md);
    align-items: center;
    padding: 4px 0;
  }
  .au__time-key {
    color: var(--color-text-1);
    font-size: var(--font-size-mono-sm);
  }
  .au__time-track {
    height: 6px;
    background: var(--color-bg-3);
    border-radius: var(--radius-full);
    overflow: hidden;
  }
  .au__time-fill {
    height: 100%;
    background: var(--time-tint);
    border-radius: var(--radius-full);
    box-shadow: 0 0 calc(8px * var(--glow, 1)) var(--time-tint);
  }
  .au__time-val {
    text-align: right;
    color: var(--color-text-1);
    font-size: var(--font-size-mono-sm);
  }

  /* ---------------- BPM cards ------------------------------------------ */
  .au__bpm-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--space-sm);
  }

  .au__bpm-card {
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: var(--space-sm) var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    min-width: 0;
  }
  .au__bpm-card--accent {
    border-color: color-mix(in oklab, var(--color-primary) 35%, var(--color-line-1));
  }
  .au__bpm-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
  }
  .au__bpm-name {
    color: var(--color-text-1);
    font-size: var(--font-size-mono-sm);
  }
  .au__bpm-val {
    color: var(--color-primary);
    font-size: var(--font-size-mono-md);
    font-weight: var(--font-weight-label);
  }
  .au__bpm-label {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
  }
  .au__bpm-led {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-text-4);
    border: 1px solid var(--color-line-2);
  }
  .au__bpm-led--on {
    background: var(--color-success);
    border-color: var(--color-success);
    box-shadow: 0 0 calc(6px * var(--glow, 1)) var(--color-success);
  }

  /* ---------------- Sparkline ------------------------------------------ */
  .au__spark {
    position: relative;
    width: 100%;
    height: 32px;
    background: var(--color-bg-0);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-xs);
    overflow: hidden;
  }
  .au__spark-mid {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: 1px;
    background: var(--color-line-1);
    opacity: 0.7;
  }
  .au__spark-svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .au__spark-path {
    fill: none;
    stroke: var(--color-primary);
    stroke-width: 1.2;
    opacity: 0.95;
  }
  .au__spark-head {
    fill: var(--color-primary);
  }

  /* ---------------- Right sidebar -------------------------------------- */
  .au__side {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    min-width: 0;
  }

  .au__preview {
    position: relative;
    width: 100%;
    max-width: 100%;
    min-width: 0;
    box-sizing: border-box;
    aspect-ratio: 16 / 9;
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    overflow: hidden;
    contain: layout paint;
    box-shadow: var(--shadow-panel);
  }
  .au__preview-blob {
    position: absolute;
    width: 60%;
    height: 60%;
    border-radius: 50%;
    filter: blur(32px);
    opacity: calc(0.65 * var(--glow, 1));
  }
  .au__preview-blob--bass {
    top: 0;
    left: 0;
    background: var(--color-band-bass);
  }
  .au__preview-blob--mid {
    top: 15%;
    right: 0;
    background: var(--color-band-mid);
  }
  .au__preview-blob--high {
    bottom: 0;
    left: 20%;
    background: var(--color-band-high);
  }
  .au__preview-label {
    position: absolute;
    bottom: var(--space-sm);
    left: var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-1);
    background: color-mix(in oklab, var(--color-bg-0) 65%, transparent);
    padding: 2px 6px;
    border-radius: var(--radius-xs);
  }

  .au__docs {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-panel);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .au__docs-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--space-md) var(--space-lg);
    background: var(--color-bg-3);
    border: 0;
    border-bottom: 1px solid var(--color-line-2);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    cursor: pointer;
  }
  .au__docs-toggle:hover {
    color: var(--color-text-0);
  }
  .au__docs-toggle:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }
  .au__docs-chevron {
    color: var(--color-text-3);
  }
  .au__docs-body {
    padding: var(--space-md) var(--space-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    overflow: auto;
  }
  .au__docs-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .au__docs-subhead {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-2);
  }
  .au__docs-prose {
    margin: 0;
    font-size: var(--font-size-body-sm);
    line-height: var(--line-height-body);
    color: var(--color-text-1);
  }
  .au__docs-token {
    color: var(--color-primary);
    font-size: var(--font-size-mono-sm);
  }
  .au__docs-link {
    color: var(--color-primary);
    font-size: var(--font-size-mono-sm);
    text-decoration: none;
  }
  .au__docs-link:hover {
    text-decoration: underline;
  }

  /* ---------------- Shared utility ------------------------------------- */
  .mono {
    font-family: var(--font-family-mono);
  }
</style>
